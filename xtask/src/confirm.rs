//! **The gate `rollback` is the mirror of**: an upgrade that comes up is confirmed by the running
//! system, and the machine keeps it.
//!
//! `cargo xtask rollback-boot` proves nife gives up on an image that never comes up. That is half
//! a mechanism, and on its own it is the *dangerous* half: a machine that rolls back correctly and
//! confirms nothing reverts every upgrade a few boots after it succeeds, which fails days later
//! and looks like something else. **A rollback mechanism nobody has watched NOT roll back is as
//! unproven as one nobody has watched roll back.**
//!
//! # What the three boots are, and why the second one is allowed to live
//!
//! | | |
//! |---|---|
//! | 1 | the stick installs itself, exactly as `cargo xtask install-boot`'s first boot does |
//! | | *the host writes an upgrade into slot 1 and marks it on trial, above slot 0, with one try* |
//! | 2 | the machine picks slot 1, spends its one try, **comes up**, and says so on the disk |
//! | 3 | the machine picks slot 1 again, on a slot with no tries left |
//!
//! Boot 3 is the whole assertion. Slot 1 reaches it with `tries = 0`, which is a state
//! `boot_slot::State::is_bootable` refuses unless the successful bit is set, so a machine that
//! chooses it has read a bit that something on boot 2 wrote. Nothing about that can be faked by a
//! transcript: the two boots are separate QEMU processes and the only thing between them is the
//! disk.
//!
//! **One try rather than three, deliberately, and it is the opposite reason from `rollback`'s.**
//! There the count was about how many boots the gate costs. Here it is about strength: a slot
//! confirmed with tries to spare would still be chosen by priority alone, and the test would pass
//! on a machine whose confirmation did nothing at all.
//!
//! The upgrade in slot 1 is a byte-for-byte copy of the working image, the same staging
//! `rollback` does and through the same crates the installer uses.
//!
//! # What it asserts, and what a failure of each one would mean
//!
//! - Boot 2 says `starting boot slot 1` and `started from boot slot 1`. **The chooser chose the
//!   upgrade and told it which slot it was**, which is the token `boot_slot::cmdline` carries.
//! - Boot 2 reaches a shell and reads the file `mkfs` wrote. The criterion
//!   `install_service::confirm` states was actually met, rather than the confirmation being a line
//!   printed by a machine that had not come up.
//! - Boot 2 says `slot 1 confirmed`. **The write happened**, and a failure here is the whole
//!   feature missing.
//! - Boot 3 says `starting boot slot 1` and never mentions slot 0. **The bit reached the disk and
//!   survived a power cycle**, on a slot with no tries left, which is the property this gate
//!   exists for.
//! - Boot 3 says `already confirmed; nothing written`. **The write is idempotent and an installed
//!   machine does not rewrite its partition table on every boot of its life.**
//! - The disk, read afterwards by this gate rather than by nife, shows slot 1 at `tries = 0,
//!   successful = 1` with its priority untouched, and slot 0 exactly as the install left it.
//!
//! # EXAMPLES
//!
//! ```console
//! $ cargo xtask confirm-boot
//! --- the stick installs itself onto an empty NVMe disk ---
//!   install     : DONE. Remove the installation medium and reboot.
//! rollback-boot: slot 1 written from target/esp-install/EFI/BOOT/BOOTX64.EFI, on trial
//!                priority 3, 1 try; slot 0 stays at priority 2, confirmed
//! --- boot 2 of 3: the machine tries the upgrade, and the upgrade comes up ---
//! uefi_loader: starting boot slot 1
//! uefi_loader: started from boot slot 1
//!   boot slot   : slot 1 confirmed. This upgrade will not roll back.
//! $ wc made-on-target
//!   1 10 57
//! --- boot 3 of 3: the upgrade has no tries left, and is chosen anyway ---
//! uefi_loader: starting boot slot 1
//!   boot slot   : slot 1 was already confirmed; nothing written.
//! confirm-boot: slot 1 is priority 3, 0 tries, successful
//! confirm-boot: PASS
//! ```
//!
//! # BUGS
//!
//! - **The upgrade is staged by this gate and not by nife**, `rollback`'s own caveat and the same
//!   reason: nothing in this tree upgrades a running machine yet. The bytes and the attribute bits
//!   go through the crates the installer uses, so the format is exercised end to end; the program
//!   that will one day stage this is not.
//! - **It does not cut power during the confirming write.** The interrupted-confirmation case is
//!   argued in `install_service::confirm` and is as untested here as the interrupted decrement is
//!   in `rollback`: nothing in this tree cuts power to a QEMU between two block writes.
//! - **Nothing here checks that the two disk clients did not collide**, because on the boot path
//!   they cannot: the filesystem server is blocked in receive while this runs. That is an ordering
//!   rather than a mechanism (`install_service::confirm` names it as a foot gun), and a gate that
//!   passed would not be evidence the ordering is enforced, only that it held this time.
//! - **It proves nothing about a real firmware**, the same caveat `install-boot` and `rollback`
//!   carry. OVMF is one implementation and a generous one.
//! - **x86_64 only** (DECISIONS §19 (architectural parity is a tenet; the targets are aarch64, riscv64, and x86_64)), because
//!   the chooser is. The device-tree architectures have no installed disk to choose slots on yet.
//! - **It takes several minutes under TCG**, three boots of it, so it is not in `script/test`'s
//!   default legs any more than `install-boot` and `rollback-boot` are.

use boot_slot::State;
use globally_unique_identifier_partition_table::GloballyUniqueIdentifierPartitionTable as Table;

use crate::install;
use crate::rollback::{
    LBA, PRIMARY_BYTES, TRIAL_PRIORITY, read_table, slot_states, stage_an_upgrade,
};

/// What boot 2's transcript must contain: the chooser's two lines, the confirmation, and a shell
/// that read a file off the installed disk.
const BOOT_TWO_WANTS: [&str; 5] = [
    "uefi_loader: starting boot slot 1",
    "uefi_loader: started from boot slot 1",
    "slot 1 confirmed",
    "nife capability shell.",
    "1 10 57",
];

/// And boot 3's: the same slot chosen with no tries left, and no second write to the table.
const BOOT_THREE_WANTS: [&str; 3] = [
    "uefi_loader: starting boot slot 1",
    "already confirmed; nothing written",
    "1 10 57",
];

/// What a shell is asked to do on a boot that is allowed to come all the way up, and the marker
/// each answer is keyed on. `rollback`'s boot 3 does the same thing for the same reason: reaching a
/// prompt is the claim, and reading a file `mkfs` wrote is what makes it one.
const AT_THE_PROMPT: [(&str, &str); 2] = [
    ("the progenitor is running at ring 3", "ls\r"),
    ("made-on-target", "wc made-on-target\r"),
];

/// **The gate.** `true` only if a good upgrade was confirmed and then chosen again with no tries
/// left.
pub(crate) fn confirm_boot() -> bool {
    if !install::prepare() {
        return false;
    }
    if install::install_once().is_none() {
        return false;
    }

    let disk = install::install_disk_path();
    let boot_file = install::install_esp_dir().join("EFI/BOOT/BOOTX64.EFI");
    if !stage_an_upgrade(&disk, &boot_file) {
        return false;
    }

    eprintln!();
    eprintln!("--- boot 2 of 3: the machine tries the upgrade, and the upgrade comes up ---");
    let Some(trial) = install::boot(
        &install::empty_esp_dir(),
        true,
        &AT_THE_PROMPT,
        "1 10 57",
        300,
    ) else {
        eprintln!("confirm-boot: boot 2 never reached a prompt, so nothing was tested");
        return false;
    };

    let mut ok = true;
    for wanted in BOOT_TWO_WANTS {
        if !trial.contains(wanted) {
            eprintln!("confirm-boot: boot 2's transcript is missing {wanted:?}");
            ok = false;
        }
    }
    // **A confirmation is not a wish.** The one refusal worth naming on its own, because its
    // symptom without this line is a passing gate on a machine that decided the slot number it was
    // handed named nothing.
    if trial.contains("NOT confirmed") {
        eprintln!("confirm-boot: boot 2 refused to confirm the slot it booted from");
        ok = false;
    }

    eprintln!();
    eprintln!("--- boot 3 of 3: the upgrade has no tries left, and is chosen anyway ---");
    let Some(kept) = install::boot(
        &install::empty_esp_dir(),
        true,
        &AT_THE_PROMPT,
        "1 10 57",
        300,
    ) else {
        return false;
    };
    for wanted in BOOT_THREE_WANTS {
        if !kept.contains(wanted) {
            eprintln!("confirm-boot: boot 3's transcript is missing {wanted:?}");
            ok = false;
        }
    }
    // **The whole promise in one assertion**, and it is the exact negative of `rollback`'s. A
    // machine that fell back to slot 0 here read a slot with no tries left as unbootable, which is
    // what it would have done if nothing had written the bit.
    if kept.contains("uefi_loader: starting boot slot 0") {
        eprintln!("confirm-boot: boot 3 fell back to slot 0; the confirmation did not stick");
        ok = false;
    }

    ok &= the_disk_agrees(&disk);
    if ok {
        eprintln!("confirm-boot: PASS");
    }
    ok
}

/// **Read the bits back off the disk and say what they are**, which is the assertion a transcript
/// cannot make: the console says what the machine decided, and this says what it wrote down.
fn the_disk_agrees(disk: &std::path::Path) -> bool {
    let mut head = [0u8; PRIMARY_BYTES];
    if !read_table(disk, &mut head) {
        return false;
    }
    let Ok(table) = Table::parse(
        &head[LBA as usize..2 * LBA as usize],
        &head[2 * LBA as usize..],
    ) else {
        // The crash-consistency case showing up: a table this gate wrote and the machine rewrote
        // that no longer parses. Worth its own sentence rather than a generic failure, because the
        // confirming write is the second thing in this tree that rewrites a live partition table.
        eprintln!("confirm-boot: the disk's table no longer parses after the machine rewrote it");
        return false;
    };

    let states = slot_states(&table);
    let [zero, one] = states[..] else {
        eprintln!("confirm-boot: the disk no longer has two boot slots");
        return false;
    };
    eprintln!(
        "confirm-boot: slot 1 is priority {}, {} tries, {}successful",
        one.priority,
        one.tries,
        if one.successful { "" } else { "not " }
    );

    let mut ok = true;
    // Confirmed, with the try still spent and the priority still saying what was chosen and why.
    // `State::confirmed` zeroes tries rather than restoring them, so this is the same value the
    // rollback gate asserts with one bit different, which is the point.
    if one != State::on_trial(TRIAL_PRIORITY, 0).confirmed() {
        eprintln!("confirm-boot: slot 1 should be priority {TRIAL_PRIORITY}, 0 tries, successful");
        ok = false;
    }
    // And the image the machine no longer needs was not touched on the way past. A confirmation
    // that demoted the previous slot would leave a machine with nothing to fall back to.
    if zero != State::installed() {
        eprintln!("confirm-boot: slot 0 is no longer the state the install left it in");
        ok = false;
    }
    ok
}
