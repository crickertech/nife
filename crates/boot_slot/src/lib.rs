//! **Two boot slots, and the rule that stops a bad upgrade bricking the machine.**
//!
//! An installed nife machine keeps two copies of its boot image. An upgrade writes the one that is
//! not running and marks it *on trial*. If the trial image fails to come up, **the machine goes
//! back to the previous one by itself**, with nobody at the console. This crate is the state that
//! makes that true and the policy that reads it; it touches no disk, so the program that writes the
//! state (`components/src/installer.rs`) and the program that acts on it (`uefi_loader`'s chooser)
//! share one implementation rather than two readings of a paragraph.
//!
//! calef ruled on 2026-09-21: *"Yes, write the tries and priority attributes in 2b."*
//!
//! Name: provisional, minted 2026-09-21 by the rung 2b lane. A noun for the thing on the disk,
//! singular like `page_frames`' members and `grant_plan`, rather than for the mechanism over it.
//! Refused `ab_boot`, which is the industry's word for this (A/B partitions) and which says
//! *two* when nothing in here does; refused `boot_slot_attributes`, which names the byte layout
//! rather than the concept and would have to change the day the state moved. `slot` alone was
//! refused as ambiguous in a capability system, where a slot is a place in a cspace. calef names
//! crates; expect this to change.
//!
//! # Where the state lives, and why it is allowed to live there
//!
//! In the **GPT partition entry attribute bits of the slot partitions themselves**. UEFI 2.11
//! section 5.3.3's attribute table says of bits 48 to 63, verbatim:
//!
//! > Reserved for GUID specific use. The use of these bits will vary depending on the
//! > `PartitionTypeGUID`. Only the owner of the `PartitionTypeGUID` is allowed to modify these
//! > bits. They must be preserved if Bits 0-47 are modified.
//!
//! A boot slot's partition type is
//! [`globally_unique_identifier_partition_table::guid::types::NIFE_BOOT`], which is ours, so these
//! sixteen bits are ours by specification and **no other operating system's partition tool will
//! touch them**. That is the whole reason to prefer them to a file: a file in a filesystem is
//! something anything that can mount the volume may rewrite, and the state that decides whether a
//! machine boots should not be.
//!
//! ## The correction that shapes the rest of this crate
//!
//! It is easy to say, and was said on this project, that *firmware* does the selecting. **It does
//! not, and cannot.** ChromeOS is where this design comes from and it gets firmware-level selection
//! because depthcharge is a custom coreboot payload that **replaces** UEFI; its BIOS is a program
//! Google wrote. Generic UEFI firmware, OVMF included, reads bits 48 to 63 of a type it does not
//! own for exactly no purpose: the specification just told it not to. So **the selector has to be
//! ours**, and on a UEFI machine the only code of ours that runs before the kernel is the image at
//! `\EFI\BOOT\BOOTX64.EFI`. That image is therefore the chooser, and the slots are what it chooses
//! between.
//!
//! The other mechanism a reader will reach for is UEFI `Boot####` variables with `BootOrder` and
//! `BootNext`, which firmware genuinely does read, and it was evaluated and refused.
//! Rung 2a of milestone 198 (a package manager, and the trivial install that makes a second customer possible) proved its boot **with the firmware variable store deleted**, on purpose,
//! so that "the firmware found the file on its own" was a claim about the disk rather than about
//! leftovers from a previous run. A rollback design that depends on those variables surviving
//! contradicts the one property that boot was built to demonstrate, and it would also make the
//! state unreadable and unwritable from anywhere but firmware runtime services, which this kernel
//! does not map.
//!
//! # The bit layout, which is a format two programs agree on
//!
//! **Provisional, pending calef's ratification**, and written here because this is where a reader
//! meets it. The positions are ChromeOS's own, verified against the ChromiumOS disk-format
//! reference rather than recalled, so that anyone who has run `cgpt show` can read a nife disk:
//!
//! | bits | field | meaning |
//! |---|---|---|
//! | 48-51 | [`PRIORITY_SHIFT`] | 0 to 15. **0 means never select this slot.** Higher wins |
//! | 52-55 | [`TRIES_SHIFT`] | 0 to 15. Boot attempts left before the slot is given up on |
//! | 56 | [`SUCCESSFUL_BIT`] | this image has booted and been confirmed; [`State::tries`] stops mattering |
//! | 57-63 | | reserved, written zero, and [`State::into_attributes`] preserves whatever is there |
//!
//! Bits 0 to 47 are the specification's own (`ATTR_REQUIRED` and friends) and are preserved
//! untouched, which is what that table row asks for in its last sentence.
//!
//! # The policy
//!
//! A slot is bootable when its priority is above zero **and** it has either been confirmed or has
//! tries left ([`State::bootable`]). [`select`] returns the bootable slot of highest priority,
//! breaking a tie toward the lower index so that two slots written with the same priority still
//! produce the same boot twice running.
//!
//! **The chooser decrements `tries` before it hands off, not after**, and that is the crux of the
//! whole design rather than a detail. A chooser that only read, leaving the running system to mark
//! itself good once it was up, would protect against an image that *crashes* and not against one
//! that **hangs**: a machine that wedges before userspace would retry the same bad image forever,
//! and the console nobody is standing at would show nothing. Writing first costs a disk write on
//! every trial boot and buys the failure mode that actually strands people. ChromeOS decrements in
//! firmware before the launch for the same reason.
//!
//! # EXAMPLES
//!
//! The life of one upgrade, in the states the disk holds between reboots:
//!
//! ```
//! use boot_slot::{State, select};
//!
//! // After an install: slot 0 holds the image that was just written and is known good.
//! let mut slots = [State::installed(), State::EMPTY];
//! assert_eq!(select(&slots), Some(0));
//!
//! // An upgrade writes slot 1 and puts it on trial, above slot 0.
//! slots[1] = State::on_trial(State::installed().priority + 1, 3);
//! assert_eq!(select(&slots), Some(1));
//!
//! // Three boots that never come up. Each one spends a try BEFORE handing off.
//! for _ in 0..3 {
//!     let chosen = select(&slots).expect("something is always bootable here");
//!     slots[chosen] = slots[chosen].attempted();
//! }
//!
//! // Nobody had to do anything: the machine is back on the image it came with.
//! assert_eq!(select(&slots), Some(0));
//! ```
//!
//! And the same upgrade when it works: something in the booted system calls [`State::confirmed`],
//! and the trial slot stops being a trial.
//!
//! ```
//! # use boot_slot::{State, select};
//! let mut slots = [State::installed(), State::on_trial(3, 3)];
//! slots[1] = select(&slots).map(|i| slots[i].attempted()).unwrap();
//! slots[1] = slots[1].confirmed();
//! assert!(slots[1].successful);
//! assert_eq!(select(&slots), Some(1));
//! ```
//!
//! # BUGS
//!
//! - **The bit layout and [`SlotHeader`]'s bytes are provisional.** They are a format two programs
//!   agree on, which `AGENTS.md` puts in the irreversible category, and calef has not ratified
//!   either. A disk written by a version of nife before that ratification may not be readable by
//!   one after it.
//! - **Nothing in this crate marks a slot successful on its own.** [`State::confirmed`] exists and
//!   is exercised by the tests; the program that calls it on a running machine does not, so today
//!   only `installer` sets the bit, at install time, on the slot it just wrote. The consequence is
//!   precise and it is a real limitation rather than a theoretical one: **an upgrade that is never
//!   confirmed rolls back after its tries are spent, even though it was working.** That fails
//!   safe (the machine keeps running the previous image) and it means upgrades do not stick. See
//!   `design/roadmap/proposals/nothing-marks-a-trial-boot-successful.md`.
//! - **Two slots is not a number this crate enforces.** [`select`] takes a slice of any length and
//!   the policy is the same for three; `installer` lays out two because a third costs a partition
//!   and buys nothing until something can use it.
//! - **Priority is not rotated on exhaustion.** ChromeOS drops an exhausted slot's priority to zero
//!   so that its firmware's plain highest-priority scan moves on. [`select`]'s predicate already
//!   excludes an exhausted slot, so nife does not need the write, and not doing it keeps the
//!   record of what the priorities *were* when the machine last chose. The one place nife does
//!   spend priority is [`State::unreadable`], for a slot whose image will not load at all.
//! - **A slot is 64 MiB of disk that is usually a duplicate.** Two copies of a ten-megabyte boot
//!   image is the price of the property, and it is paid on every installed machine whether or not
//!   it is ever upgraded.

#![no_std]

use globally_unique_identifier_partition_table::crc;

/// The lowest bit of the four-bit priority field: bits 48 to 51.
pub const PRIORITY_SHIFT: u32 = 48;
/// The lowest bit of the four-bit tries field: bits 52 to 55.
pub const TRIES_SHIFT: u32 = 52;
/// The confirmed-boot bit: bit 56.
pub const SUCCESSFUL_BIT: u32 = 56;

/// The largest value either four-bit field holds, and the ceiling both setters saturate at.
pub const MAX_NIBBLE: u8 = 15;

/// **The priority `installer` gives the slot it writes at install time.**
///
/// Not 15, deliberately. An upgrade has to be able to outrank the running image without first
/// lowering it, and a first install that took the top of the range would force every upgrader to
/// begin by rewriting a partition entry it is not otherwise touching.
pub const INSTALLED_PRIORITY: u8 = 2;

/// Mask of the bits this crate owns: 48 through 56. Everything else in an attribute word is
/// somebody else's and is preserved.
const OWNED: u64 = (0xF << PRIORITY_SHIFT) | (0xF << TRIES_SHIFT) | (1 << SUCCESSFUL_BIT);

/// **One slot's boot state**, as the three fields the attribute bits carry.
///
/// A plain struct rather than a wrapped `u64` so that a caller reading a disk in a debugger sees
/// three numbers, and so that the encode/decode round trip is something the tests can state.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct State {
    /// 0 to 15. **Zero means never select this slot**, whatever else is set.
    pub priority: u8,
    /// 0 to 15. Boot attempts left before the slot stops being bootable.
    pub tries: u8,
    /// This image has booted and something confirmed it, so [`Self::tries`] no longer matters.
    pub successful: bool,
}

impl State {
    /// A slot that holds nothing and will never be chosen. What `installer` writes into the spare
    /// slot, so that an upgrader finds a partition already laid out and does not have to repartition
    /// a running machine to install an update.
    pub const EMPTY: State = State {
        priority: 0,
        tries: 0,
        successful: false,
    };

    /// **The state of the slot an install just wrote**: confirmed, because the image in it is the
    /// one that is running at that moment and has therefore already booted this machine once.
    ///
    /// That claim is worth being precise about, because it is the strongest thing said anywhere in
    /// this crate. The bytes written into slot 0 by an install are a copy of the file the firmware
    /// started a few seconds earlier, on this machine, with this firmware. Marking it good is a
    /// record of an observation rather than an assumption.
    pub const fn installed() -> State {
        State {
            priority: INSTALLED_PRIORITY,
            tries: 0,
            successful: true,
        }
    }

    /// A slot holding a freshly written image that has never booted: outranking whatever is there,
    /// and with `tries` attempts to prove itself before the machine gives up on it.
    ///
    /// Both arguments saturate at [`MAX_NIBBLE`] rather than wrapping, because a wrapped priority
    /// of 16 is a priority of 0, which is the one value that means "never boot this".
    pub const fn on_trial(priority: u8, tries: u8) -> State {
        State {
            priority: if priority > MAX_NIBBLE {
                MAX_NIBBLE
            } else {
                priority
            },
            tries: if tries > MAX_NIBBLE {
                MAX_NIBBLE
            } else {
                tries
            },
            successful: false,
        }
    }

    /// **Can this slot be chosen?** Priority above zero, and either confirmed or with tries left.
    /// The same two clauses ChromeOS's firmware checks, in the same order.
    pub const fn bootable(&self) -> bool {
        self.priority > 0 && (self.successful || self.tries > 0)
    }

    /// **The state to write before handing off to this slot**: one try spent.
    ///
    /// A confirmed slot is returned unchanged, so a machine running a good image writes nothing to
    /// its disk on an ordinary boot. That is not only tidiness: a write on every boot is a write
    /// that can be interrupted on every boot.
    pub const fn attempted(&self) -> State {
        if self.successful || self.tries == 0 {
            *self
        } else {
            State {
                tries: self.tries - 1,
                ..*self
            }
        }
    }

    /// **The state of a slot that has proved itself**: confirmed, and with its tries returned to
    /// zero because they no longer mean anything.
    ///
    /// Priority is left alone. A trial slot was written with a priority above the running one, so
    /// confirming it is enough to keep it chosen; raising it further would spend range for nothing.
    pub const fn confirmed(&self) -> State {
        State {
            priority: self.priority,
            tries: 0,
            successful: true,
        }
    }

    /// **The state of a slot whose image cannot be read at all**: given up on immediately rather
    /// than after its remaining tries.
    ///
    /// This is the one place nife writes a zero priority, and the reason is that the failure is
    /// one the chooser can see for itself. Tries exist to bound a failure that happens *after* the
    /// handoff, where nothing is watching. A header that does not decode, or an image whose
    /// checksum is wrong, is a failure the chooser observed with its own eyes, and spending three
    /// reboots to re-observe it would be three reboots of a machine somebody is waiting on.
    /// ChromeOS does the same thing for an invalid kernel header.
    pub const fn unreadable(&self) -> State {
        State {
            priority: 0,
            tries: 0,
            successful: false,
        }
    }

    /// Decode the three fields out of a GPT entry's attribute word.
    ///
    /// Total: every `u64` decodes, because every bit pattern of four bits is a number from 0 to 15
    /// and every bit is a boolean. There is no such thing as a malformed slot state, only one that
    /// is not bootable, which is the same layering
    /// [`globally_unique_identifier_partition_table::Entry::decode`] keeps.
    pub const fn from_attributes(attributes: u64) -> State {
        State {
            priority: ((attributes >> PRIORITY_SHIFT) & 0xF) as u8,
            tries: ((attributes >> TRIES_SHIFT) & 0xF) as u8,
            successful: (attributes >> SUCCESSFUL_BIT) & 1 == 1,
        }
    }

    /// The attribute word to write back, **preserving every bit this crate does not own**: the
    /// specification's bits 0 to 47 and the reserved 57 to 63.
    ///
    /// That preservation is the specification's own instruction ("They must be preserved if Bits
    /// 0-47 are modified", read in both directions) and it is why this takes the previous word
    /// rather than building one from nothing.
    pub const fn into_attributes(self, previous: u64) -> u64 {
        let priority = (self.priority & 0xF) as u64;
        let tries = (self.tries & 0xF) as u64;
        let successful = self.successful as u64;
        (previous & !OWNED)
            | (priority << PRIORITY_SHIFT)
            | (tries << TRIES_SHIFT)
            | (successful << SUCCESSFUL_BIT)
    }
}

/// **Which slot to boot**: the bootable one of highest priority, ties broken toward the lower
/// index.
///
/// `None` when nothing is bootable, and the caller's answer to that is the interesting half. On
/// this system it is not "refuse to boot": see [`select_excluding`] and `uefi_loader`'s chooser,
/// which falls back to the image in its own file.
pub fn select(slots: &[State]) -> Option<usize> {
    select_excluding(slots, 0)
}

/// [`select`], ignoring every slot whose bit is set in `tried`.
///
/// **The chooser retries within one boot**, which is the difference between a failure it can see
/// and one it cannot. An image whose header will not decode is a failure the chooser observed
/// before handing off, and making a person wait for a reboot to recover from it would be a worse
/// machine for no gain. An image that starts and then hangs is a failure nothing observes, and
/// that one is what [`State::attempted`] and the write-before-handoff are for.
///
/// Bit `n` of `tried` corresponds to slot `n`; slots past bit 63 cannot be excluded, which is not a
/// limit anything reaches with two.
pub fn select_excluding(slots: &[State], tried: u64) -> Option<usize> {
    slots
        .iter()
        .enumerate()
        .filter(|(i, s)| s.bootable() && (*i >= 64 || tried & (1 << i) == 0))
        .max_by_key(|(i, s)| (s.priority, core::cmp::Reverse(*i)))
        .map(|(i, _)| i)
}

/// The magic at the start of a slot partition: eight bytes a reader can find with `strings`.
pub const SLOT_MAGIC: [u8; 8] = *b"NIFESLOT";

/// The layout version this crate writes and the only one it reads.
pub const SLOT_VERSION: u32 = 1;

/// **Bytes of a slot partition reserved for [`SlotHeader`] before the image begins**: 4096, one
/// whole transfer block of `filesystem_protocol::blk`.
///
/// Only the first [`SLOT_HEADER_BYTES`] carry anything. The rest is padding, and it is padding
/// rather than a smaller reservation so that **the image starts on a transfer-block boundary**.
/// `installer` writes whole 4096-byte blocks, and milestone 198 rung 2a already lost ten megabytes
/// to an unaligned start once; that bug is recorded in `crates/file_allocation_table` and this
/// number is what stops it happening a second time in a different file.
pub const SLOT_IMAGE_OFFSET: u64 = 4096;

/// Bytes of [`SLOT_IMAGE_OFFSET`] that [`SlotHeader::encode`] actually fills.
pub const SLOT_HEADER_BYTES: usize = 32;

/// **What is in a boot slot**, written at the very start of the partition and read by the chooser
/// before it spends a page of memory on the image.
///
/// A partition has no length field of its own that means "how much of me is in use", so this is
/// where the image's length lives. The checksum is the other half and is what turns "the installer
/// died halfway through the copy" from a machine that starts a truncated image into a machine that
/// refuses the slot and boots the other one.
///
/// | offset | size | |
/// |---|---|---|
/// | 0 | 8 | [`SLOT_MAGIC`] |
/// | 8 | 4 | [`SLOT_VERSION`], little-endian |
/// | 12 | 4 | reserved, zero |
/// | 16 | 8 | [`Self::image_len`], little-endian |
/// | 24 | 4 | [`Self::image_crc32`], little-endian |
/// | 28 | 4 | CRC-32 of bytes 0 to 27, little-endian |
///
/// CRC-32 is the GPT's own (`globally_unique_identifier_partition_table::crc`), reused rather than
/// written again: this crate already depends on that one for the attribute bits it lives in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlotHeader {
    /// How many bytes of image follow, starting [`SLOT_IMAGE_OFFSET`] into the partition.
    pub image_len: u64,
    /// CRC-32 of exactly those bytes.
    pub image_crc32: u32,
}

impl SlotHeader {
    /// The header for an image, with its checksum computed from the bytes themselves.
    pub fn of(image: &[u8]) -> SlotHeader {
        SlotHeader {
            image_len: image.len() as u64,
            image_crc32: crc::crc32(image),
        }
    }

    /// Write the header into the start of a buffer of at least [`SLOT_HEADER_BYTES`], zeroing
    /// nothing else. `false` if the buffer is too small.
    pub fn encode(&self, out: &mut [u8]) -> bool {
        if out.len() < SLOT_HEADER_BYTES {
            return false;
        }
        out[0..8].copy_from_slice(&SLOT_MAGIC);
        out[8..12].copy_from_slice(&SLOT_VERSION.to_le_bytes());
        out[12..16].copy_from_slice(&0u32.to_le_bytes());
        out[16..24].copy_from_slice(&self.image_len.to_le_bytes());
        out[24..28].copy_from_slice(&self.image_crc32.to_le_bytes());
        let own = crc::crc32(&out[0..28]);
        out[28..32].copy_from_slice(&own.to_le_bytes());
        true
    }

    /// Read a header back. `None` for a buffer that is too short, a wrong magic, a version this
    /// build does not know, or a header whose own checksum does not hold.
    ///
    /// **An empty slot decodes as `None` rather than as a zero-length image**, because a partition
    /// that was never written is all zeros and all zeros is not the magic. That is the shape a
    /// caller wants: "there is nothing here" and "there is something broken here" both mean do not
    /// boot this slot.
    pub fn decode(bytes: &[u8]) -> Option<SlotHeader> {
        if bytes.len() < SLOT_HEADER_BYTES || bytes[0..8] != SLOT_MAGIC {
            return None;
        }
        if u32::from_le_bytes(bytes[8..12].try_into().ok()?) != SLOT_VERSION {
            return None;
        }
        if u32::from_le_bytes(bytes[28..32].try_into().ok()?) != crc::crc32(&bytes[0..28]) {
            return None;
        }
        Some(SlotHeader {
            image_len: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
            image_crc32: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
        })
    }

    /// **Are these the bytes this header describes?** The check the chooser runs after reading an
    /// image and before starting it.
    pub fn matches(&self, image: &[u8]) -> bool {
        image.len() as u64 == self.image_len && crc::crc32(image) == self.image_crc32
    }
}
