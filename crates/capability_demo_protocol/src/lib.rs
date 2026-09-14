//! **The words the capability demonstrations agree on**, and nothing else.
//!
//! Milestones 7 through 19 each proved one property of the capability model with a pair of user
//! programs and an assertion in `kernel/src/user`. Until milestone 291 every one of those programs
//! was a *role* of `fixtures/src/hello.rs`, so a value two of them shared was a constant in one
//! file and the comment "one binary, so one constant serves both roles" was true. Splitting the
//! roles into programs made that sentence false, and AGENTS.md rule 7 is what it becomes: anything
//! two binaries must agree on is a crate.
//!
//! Everything here is a **sentinel**: a value chosen only to be recognised. None of them is a wire
//! format, and changing one costs a rebuild rather than a migration, which is why they can sit
//! together in one crate instead of one crate each.
//!
//! # Examples
//!
//! The frame-delegation pair, with the page and the two processes stood in for. The producer writes
//! the sentinel into a page it owns and delegates a read-only view; the consumer maps the same
//! physical page and checks the value it reads. Recognising it is how the consumer knows the memory
//! is genuinely *shared* rather than a second frame that happens to be zeroed, which is the whole
//! claim `kernel::user::page_frame_service` wires and asserts.
//!
//! ```
//! use capability_demo_protocol::PAGE_FRAME_SENTINEL;
//!
//! // The producer, in its own address space.
//! let mut page = [0u64; 512];
//! page[0] = PAGE_FRAME_SENTINEL;
//!
//! // The consumer, through its own read-only mapping of the same physical frame.
//! let shared: &[u64] = &page;
//! assert_eq!(shared[0], PAGE_FRAME_SENTINEL, "the page was not shared");
//!
//! // And a frame that was merely retyped, never written through, does not look like one.
//! let fresh = [0u64; 512];
//! assert_ne!(fresh[0], PAGE_FRAME_SENTINEL);
//! ```
//!
//! # Bugs
//!
//! Nothing checks that a reader of one of these constants is one of the two parties that agreed on
//! it. A third program could depend on this crate and read `USED_WORD` for an unrelated purpose,
//! and the next change to that constant would break it silently. The tree's other `*_proto` crates
//! have the same property and it has never bitten; recorded here rather than defended.
//! Name: provisional (milestone 291). The words the milestone 7-19 capability demonstrations agree
//! on, now that each demonstration is its own binary rather than a role of one. Follows the tree's
//! `*_proto` shape (`filesystem_protocol`, `supervision_protocol`, `graphics_protocol`): what two
//! compilation units must agree on has exactly one definition, which is AGENTS.md rule 7. Refused
//! `demo_words` ("words" names the representation rather than the agreement, and `_proto` is the
//! shape a reader of this tree already recognises). Refused folding these into `crates/abi` (abi is
//! the kernel's syscall surface, which every program depends on, and three fixture constants have
//! no business widening it). Refused `capability_demo` unsuffixed (it would read as the demo itself
//! rather than as what the demos agree on, and there is no single demo to be).

#![no_std]

/// The word the frame producer writes into a page it owns, and the consumer reads back through its
/// own mapping of the same physical page.
///
/// The two are separate processes with separate address spaces, so reading this value is the whole
/// evidence that one physical page is under both mappings: nothing else would put this bit pattern
/// in a freshly retyped frame.
pub const PAGE_FRAME_SENTINEL: u64 = 0xF00D_CAFE_D00D_1234;

/// The word the delegation receiver sends back through the capability it was *delegated*, so that
/// whoever holds the other end can confirm a capability minted by one process carries real
/// authority when a different process invokes it.
///
/// `kernel/src/user/delegation_service.rs` re-exports this; it kept its own copy with a "must
/// match" comment beside it until milestone 291, which is the duplicate this crate removes.
pub const USED_WORD: u64 = 0x5A;

/// The word the cycle-counter reader reports when it read the counter without being killed for it.
///
/// **The word is the whole result, and the counter value is not.** An ungranted read of
/// `PMCCNTR_EL0` or the `cycle` CSR traps, and this kernel turns that into a fault that ends the
/// thread, so a program that gets as far as sending anything is a program the grant reached. What
/// it read is uninteresting: QEMU leaves `PMCR_EL0.E` clear, so `PMCCNTR_EL0` reads zero however
/// often you ask it.
pub const CYCLE_COUNTER_WORD: u64 = 0xC1C1E;
