//! **The CMOS real-time clock**, kernel-resident by construction (DECISIONS §121, §130;
//! milestone 176's piece 2).
//!
//! Two fixed I/O ports, `0x70` (index) and `0x71` (data), is all there is: no page, so no device
//! capability could ever map it, and §121's current recommendation keeps every x86 legacy port
//! kernel-side rather than paying a port-range capability's measured ~1.5-2.7 us per-context-switch
//! cost for a device a boot reads exactly once. So unlike `user/src/clock.rs`'s PL031 and Goldfish
//! drivers, which map their device and poll it themselves, nothing in userspace ever touches this
//! one: the kernel reads it here, once per clock-service spawn, and
//! `kernel::user::clock_service::start` hands the already-converted reading across as a plain
//! `Spawn` argument (`clock_proto::rtc::CMOS`). Same shape as `arch::x86_64::timer`'s PIT
//! calibration: kernel-side, boot-time, `in`/`out` only, sub-microsecond.
//!
//! # The register map
//!
//! | Register | What it holds |
//! |---|---|
//! | `0x00` | seconds |
//! | `0x02` | minutes |
//! | `0x04` | hours, with bit 7 the PM flag in 12-hour mode |
//! | `0x07` | day of month |
//! | `0x08` | month |
//! | `0x09` | year, two digits, no century (see below) |
//! | `0x0A` | status register A; bit 7 is Update In Progress (UIP) |
//! | `0x0B` | status register B; bit 2 is binary(1)/BCD(0), bit 1 is 24-hour(1)/12-hour(0) |
//!
//! Every field but status register B may be BCD (packed decimal, one digit per nibble) rather than
//! binary; which one this machine uses is itself a CMOS field, read *after* the six date/time
//! registers precisely because it says how to interpret them.
//!
//! # Tearing: UIP is necessary and not sufficient
//!
//! The RTC's update cycle rewrites all ten time/date bytes roughly once a second, and a read that
//! lands mid-update can see some old bytes and some new ones (a minute of 59 paired with an hour
//! that already rolled over, for instance). UIP going low promises the *previous* update has
//! finished; it does not promise the *next* one will not begin between that check and this file's
//! first port read, so a single poll-then-read is not enough. This driver takes two full snapshots,
//! each preceded by its own UIP wait, and retries if they disagree: the standard shape (the
//! `OSDev` wiki's CMOS page documents the same double-read-and-compare a real BIOS RTC driver
//! uses).
//!
//! # The century, which this machine does not tell you
//!
//! Most PC firmware exposes an eleventh, non-standard byte for the century, at a port that varies by
//! chipset (`0x32` on many, and ACPI's FADT names whichever one a given machine actually uses when
//! it does at all). This driver does not read it: every machine capable of running this kernel is
//! past the year 2000, so the CMOS year is decoded as `2000 + value` rather than located and
//! trusted. Linux's own RTC driver (`drivers/rtc/rtc-cmos.c`) makes the identical assumption for the
//! identical reason.
//!
//! # BUGS
//!
//! - **No century byte, so a date before 2000 or after 2099 cannot be represented.** Not the failure
//!   mode this file is built for; see above.
//! - **Daylight saving and the alarm/periodic-interrupt registers are untouched.** This kernel reads
//!   the clock once and never again (`clock_proto`'s wall clock is counter-plus-offset from then
//!   on), so nothing here needs an alarm, a periodic tick, or status register B's DST bit.
//! - **Not proven against real hardware.** QEMU's CMOS is a straightforward MC146818A model seeded
//!   from the host clock; a real board's chipset (milestone 87's `OptiPlex`) may exercise a branch
//!   this never observed in testing, though every branch here is the documented behavior of the
//!   standard part.

use super::port::{in8, out8};

/// The index port: write a register number here before reading or writing [`DATA`].
const INDEX: u16 = 0x70;
/// The data port: [`INDEX`]'s last write names which CMOS byte this reaches.
const DATA: u16 = 0x71;

const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x02;
const REG_HOURS: u8 = 0x04;
const REG_DAY: u8 = 0x07;
const REG_MONTH: u8 = 0x08;
const REG_YEAR: u8 = 0x09;
const REG_STATUS_A: u8 = 0x0A;
const REG_STATUS_B: u8 = 0x0B;

/// Status register A, bit 7: the RTC is mid-update and every other register is unreliable.
const STATUS_A_UPDATE_IN_PROGRESS: u8 = 1 << 7;
/// Status register B, bit 2: fields are binary (set) rather than BCD (clear).
const STATUS_B_BINARY: u8 = 1 << 2;
/// Status register B, bit 1: the hours field is 24-hour (set) rather than 12-hour with a PM flag.
const STATUS_B_24_HOUR: u8 = 1 << 1;
/// The hours register's bit 7 in 12-hour mode: set for PM. In 24-hour mode this bit is unused and
/// reads clear.
const HOUR_PM: u8 = 1 << 7;

/// Read one CMOS register.
fn read(reg: u8) -> u8 {
    // SAFETY: 0x70/0x71 are the CMOS index/data ports on every PC-compatible machine, driven only
    // from ring 0. DECISIONS §121: no port-range capability names them under its current
    // recommendation, so nothing in userspace could reach these ports even in principle, and
    // nothing here could race a driver that does not exist.
    unsafe {
        out8(INDEX, reg);
        in8(DATA)
    }
}

fn update_in_progress() -> bool {
    read(REG_STATUS_A) & STATUS_A_UPDATE_IN_PROGRESS != 0
}

/// One look at the six date/time registers, taken together so [`read_unix_nanos`]'s tearing check
/// can compare two of them for equality.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Snapshot {
    second: u8,
    minute: u8,
    hour: u8,
    day: u8,
    month: u8,
    year: u8,
}

fn snapshot() -> Snapshot {
    Snapshot {
        second: read(REG_SECONDS),
        minute: read(REG_MINUTES),
        hour: read(REG_HOURS),
        day: read(REG_DAY),
        month: read(REG_MONTH),
        year: read(REG_YEAR),
    }
}

/// One BCD byte (two packed decimal digits) to binary: `(v & 0x0F) + 10 * (v >> 4)`.
fn bcd_to_binary(v: u8) -> u8 {
    (v & 0x0F) + 10 * (v >> 4)
}

/// Read the CMOS RTC once and return the wall clock it reports, in nanoseconds since the Unix
/// epoch.
///
/// `None` when the reading does not describe a real calendar date: every field zero, which is what
/// a CMOS with a dead battery and no host to seed it reads as, is the case this actually exists to
/// catch (`calendar::Civil::new` refuses month 0 and day 0). A machine this kernel can otherwise run
/// on has no way to make CMOS report a well-formed but implausible date; `user/src/clock.rs` still
/// runs whatever it is handed through `clock_proto::policy::plausible` before publishing it, the
/// same as every other RTC binding.
pub fn read_unix_nanos() -> Option<u64> {
    let raw = loop {
        while update_in_progress() {
            core::hint::spin_loop();
        }
        let first = snapshot();
        while update_in_progress() {
            core::hint::spin_loop();
        }
        let second = snapshot();
        if first == second {
            break first;
        }
        // The two snapshots disagree: an update began between them. Loop and take two more; see
        // the module doc's "Tearing" section for why one UIP wait alone is not enough.
    };

    let status_b = read(REG_STATUS_B);
    let binary = status_b & STATUS_B_BINARY != 0;
    let twenty_four_hour = status_b & STATUS_B_24_HOUR != 0;

    // The PM flag shares the hours byte with the BCD/binary hour in every mode, so it is peeled
    // off before either decode rather than after.
    let pm = !twenty_four_hour && raw.hour & HOUR_PM != 0;
    let hour_field = raw.hour & !HOUR_PM;

    let (second, minute, hour, day, month, year) = if binary {
        (
            raw.second, raw.minute, hour_field, raw.day, raw.month, raw.year,
        )
    } else {
        (
            bcd_to_binary(raw.second),
            bcd_to_binary(raw.minute),
            bcd_to_binary(hour_field),
            bcd_to_binary(raw.day),
            bcd_to_binary(raw.month),
            bcd_to_binary(raw.year),
        )
    };

    let hour = match (twenty_four_hour, pm, hour) {
        (true, _, h) => h,
        (false, true, 12) => 12, // 12 PM is noon, unchanged
        (false, true, h) => h + 12,
        (false, false, 12) => 0, // 12 AM is midnight
        (false, false, h) => h,
    };

    // No century byte; see the module doc's "The century" section.
    let year = 2000 + i32::from(year);

    let civil = calendar::Civil::new(year, month, day, hour, minute, second).ok()?;
    let secs = u64::try_from(civil.to_unix()).ok()?;
    Some(secs * clock_proto::NANOS_PER_SEC)
}
