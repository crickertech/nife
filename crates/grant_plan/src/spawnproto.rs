//! **The spawn protocol: the wire half of the shell-to-init grant expression.**
//!
//! When the shell resolves a `run` into an [`Endowment`](crate::Endowment), it does not build the
//! child itself: init holds the initrd and is the ELF loader (the parser stays in one place, out of
//! the shell). So the shell tells init what to spawn and, crucially, *delegates the capabilities it
//! grants* over the same endpoint. This module is that contract's word layout, the capability-shell
//! analogue of `line_editor::proto`.
//!
//! It is a **userspace** protocol, not kernel ABI. The kernel routes these words the way it routes
//! any IPC (DECISIONS §10, §12, §21); it never reads them. Adding a field is a change here, not to
//! the syscall surface.
//!
//! # The exchange
//!
//! The shell owns the sequence; init serves it in a loop.
//!
//! 1. **Request.** The shell `SEND`s three words on the spawn endpoint: the program id, the
//!    integer argument, and the memory-grant page count. See [`request`] / [`prog_id`] /
//!    [`arg`] / [`mem_pages`].
//! 2. **The directory grant, if the request announced one** ([`Wiring::dir`], milestone 31 phase 3):
//!    [`GRANT_WORDS`] plain `SEND`s carrying the caretaker's `START` words and then the child's.
//!    Before the delegation rather than after, because these are **data and not capabilities** and
//!    mixing the two orders would put a `RECV` where a `RECV_CAP` belongs.
//! 3. **Delegation.** The capabilities the request announced, in a fixed order: the supervised
//!    job's pair (untyped, frame), then the **sink** (milestone 50), then the **source**, then the
//!    **diagnostic endpoint** (DECISIONS §67), then the **screen-narrowed tail's completion
//!    endpoint** (DECISIONS §106), then the `--mem` untyped. Order rather than tags, because both
//!    sides read the same [`Wiring`] out of the same word and a promise nobody receives would
//!    deadlock both.
//!
//!    If `mem_pages > 0`, the shell `SEND_CAP`s exactly one capability there: an
//!    untyped it split from *its own* budget, sized to `mem_pages`. This is the grant made real,
//!    not parsed and dropped. Programs that grant no capability (worker) skip this step, and init
//!    knows to skip the matching `RECV_CAP` from `mem_pages == 0`.
//! 4. **Outcome.** init builds the child, endows it (the shared result endpoint always; the
//!    delegated untyped when present), and starts it. The child reports its own answer on the
//!    result endpoint. If init cannot build it (its own budget is spent, or the program vanished),
//!    it sends [`SPAWN_FAILED`] on the result endpoint so the shell's read completes instead of
//!    hanging.
//!
//! The result endpoint carries both init's failure sentinel and the child's success answer, and
//! the shell reads exactly once: a well-formed spawn yields the child's word, a failed one yields
//! [`SPAWN_FAILED`]. One reader, one word, no ambiguity.
//!
//! 5. **Death** (milestone 235). A child the kernel killed sends nothing, so neither of those two
//!    words arrives and the shell's single read has nothing to complete it. `job_undertaker`, which
//!    already holds init's supervision endpoint and already collects the corpse, sends
//!    [`JOB_FAULTED`] there instead. It is a third value on the same one-word read rather than a
//!    second channel, because the shell has one thread and can be blocked in exactly one `RECV`;
//!    see [`JOB_FAULTED`] for the two couplings this refused.

/// The interruptible bit, packed into the high half of the page-count word so one `SEND` still
/// carries the whole request. `mem_pages` is a small count (budgeter's ceiling is 64), so the low
/// 32 bits hold it and this bit rides above.
const INTERRUPTIBLE_BIT: u64 = 1 << 32;

/// **A capability for the child's output slot follows** (milestone 50). Set by `>` and by every
/// stage of a `|` but the last: the shell delegates an endpoint and init puts it where the result
/// endpoint would have gone, so the child writes to a pipe or a file sink without knowing which.
const SINK_BIT: u64 = 1 << 33;

/// **A capability for the child's input slot follows** (milestone 50). Set by `<` and by every
/// stage of a `|` but the first.
const SOURCE_BIT: u64 = 1 << 34;

/// **A capability for the child's declared second output follows** (DECISIONS §67). Set for a
/// program whose manifest declares one, whether or not the line has a `2>` on it: the stream exists
/// because the program says so, and the operator only names where it goes.
///
/// Unlike [`SINK_BIT`] this does **not** say which slot: init reads that from the manifest, because
/// the slot is the program's declaration and not the shell's choice. What the wire says is only
/// "expect one more capability", which is what keeps the two sides in lockstep.
const DIAG_BIT: u64 = 1 << 35;

/// **A capability for a narrowed tail stage's completion signal follows** (DECISIONS §106). Set
/// when the shell decided this stage both writes and reads, and the line named neither `>` nor `|`
/// for its output (`Wiring::sink` is false and the plan says [`crate::line::Sink::Report`]): the
/// shell cannot be both this stage's feeder and its reader, so its primary output defaults to
/// `terminal_sink_caretaker` instead of the shell's own result endpoint, the same adapter a
/// declared second stream already reaches by default under DECISIONS §67.
///
/// What follows is not a sink capability (init already knows to build that default from its own
/// `term_sink`, unprompted, the same way it builds a diagnostic default). It is a **fresh
/// endpoint the shell minted and kept a copy of**, delegated so init can install it as this child's
/// DECISIONS §26 fault target in place of its own domain channel. The kernel then delivers the
/// child's exit there instead of to init's reaper, and the shell `RECV`s it as its completion
/// signal instead of draining the child's bytes, which it no longer sees.
const SCREEN_BIT: u64 = 1 << 37;

/// **A directory grant follows, and init is to build a caretaker for it** (milestone 31 phase 3).
///
/// The odd one out on this word, because it announces **data rather than a capability**. Every other
/// bit here says "expect one more `SEND_CAP`"; this one says "expect two more `SEND`s", and the
/// reason is that the shell has nothing to delegate. A directory grant is delivered by a
/// `fs_subtree_caretaker`, the caretaker has to hold the file service to attenuate it, and **the
/// shell's file-service endpoint carries no `GRANT`**, so the shell could not hand one over if it
/// wanted to. What it can do is say what the grant *is*; init holds the endpoint and builds the rest.
///
/// See [`GRANT_WORDS`] for what the two messages carry and why they are opaque to this module.
const DIR_BIT: u64 = 1 << 36;

/// **A second directory grant follows, for the same confined program** (milestone 154,
/// design/roadmap/154-multi-directory-namespace.md). Set only alongside [`DIR_BIT`]: a second
/// grant is meaningless without a first, the same way the kernel's `fs_service::TwoDirGrant`
/// (slot 0 grant A, slot 1 grant B) only exists in pairs.
///
/// **Following [`DIR_BIT`]'s own precedent rather than inventing a new shape**, as milestone
/// 154's roadmap block names as the open question this closes: another two [`GRANT_WORDS`]
/// messages follow the first pair, carrying the second caretaker's `START` words. Unlike
/// [`DIR_BIT`], the confined program's own `START` words are **not** repeated a second time: one
/// program is still being started, holding two narrowed endpoints (slot 0 grant A, slot 1 grant
/// B), the same delivery `fs_service::start_granted_two_dirs` already proved.
///
/// **Nothing on the shell side sets this bit yet.** No verb in `grant_plan` constructs a
/// two-directory `Endowment`: that is milestone 47's `bind`, still unbuilt. This is the wire
/// format and init's decode side, built ahead of an emitter the way [`DIR_BIT`] itself once
/// stated a grant nothing could construct yet.
const DIR2_BIT: u64 = 1 << 38;

/// **The two messages a [`Wiring::dir`] request is followed by**, in order, each three words:
///
/// 1. **the caretaker's `START` words**, which init passes to `fs_subtree_caretaker` verbatim: the
///    granted directory's name and the `filesystem_proto::dir` rights the subtree capability is to carry;
/// 2. **the confined program's `START` words**, which init passes to the program verbatim: for `rm`,
///    the operand's name and the options that were typed.
///
/// **This module does not decode either, deliberately.** They are `filesystem_proto::grant`'s packing, and
/// `grant_plan` has no non-dev dependency on `filesystem_proto` on purpose (its own manifest says why: the
/// shell must be able to check a command line without linking the filesystem contract). Passing them
/// through as opaque triples keeps that true, and it means a change to how a grant is packed is a
/// change in one crate rather than in the wire this one owns. The shell packs them; init forwards
/// them; nothing in between reads them.
///
/// Two messages rather than one because the two processes are started with different names: the
/// caretaker with the *directory*, the program with the *operand inside it*. Six words do not fit in
/// three.
pub const GRANT_WORDS: usize = 2;

/// **Where the shell's operators end up on the wire**, alongside the parts of the endowment that
/// were always here. `sink` and `source` are booleans rather than capabilities because the
/// capability travels separately, over `SEND_CAP`: this word only says whether to expect it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Wiring {
    /// A foreground job the shell will supervise (DECISIONS §24). Two capabilities lead the
    /// delegation: a job untyped the child is built from so the shell can tear it down, then a
    /// shared job frame.
    pub interruptible: bool,
    /// The child's output slot is substituted (`>` or the left of a `|`).
    pub sink: bool,
    /// The child's input slot is filled (`<` or the right of a `|`).
    pub source: bool,
    /// The child declares a second output stream, so one more endpoint follows (DECISIONS §67).
    pub diagnostics: bool,
    /// **A directory grant follows as two data messages** ([`GRANT_WORDS`]), and init is to build a
    /// `fs_subtree_caretaker` for it before it builds the child. The only entry here that announces
    /// data instead of a capability; see `DIR_BIT`.
    pub dir: bool,
    /// **A second directory grant follows, as two more data messages** (milestone 154). See
    /// `DIR2_BIT`. Meaningless unless `dir` is also set; nothing here enforces that on its own
    /// (the wire is one word, and a caller that set this without `dir` sent a request that was
    /// wrong before it left the shell).
    pub dir2: bool,
    /// **This stage's primary output defaults to `terminal_sink_caretaker`, and a fresh completion
    /// endpoint follows** (DECISIONS §106). See `SCREEN_BIT`. Mutually exclusive with `sink` in
    /// practice (a stage the shell delegated an explicit sink for has somewhere else to write), but
    /// nothing here enforces that; the two ride independent bits because both sides read one word.
    pub screen: bool,
}

/// Build the three request words from a resolved endowment's parts.
pub fn request(prog_id: u64, arg: u64, mem_pages: u64, w: Wiring) -> (u64, u64, u64) {
    let mut w2 = mem_pages & 0xffff_ffff;
    if w.interruptible {
        w2 |= INTERRUPTIBLE_BIT;
    }
    if w.sink {
        w2 |= SINK_BIT;
    }
    if w.source {
        w2 |= SOURCE_BIT;
    }
    if w.diagnostics {
        w2 |= DIAG_BIT;
    }
    if w.dir {
        w2 |= DIR_BIT;
    }
    if w.dir2 {
        w2 |= DIR2_BIT;
    }
    if w.screen {
        w2 |= SCREEN_BIT;
    }
    (prog_id, arg, w2)
}

/// The whole wiring of a received request (word 2), so init reads it once rather than asking three
/// separate questions of the same word.
pub fn wiring(w2: u64) -> Wiring {
    Wiring {
        interruptible: interruptible(w2),
        sink: w2 & SINK_BIT != 0,
        source: w2 & SOURCE_BIT != 0,
        diagnostics: w2 & DIAG_BIT != 0,
        dir: w2 & DIR_BIT != 0,
        dir2: w2 & DIR2_BIT != 0,
        screen: w2 & SCREEN_BIT != 0,
    }
}

/// The program id from a received request (word 0).
pub fn prog_id(w0: u64) -> u64 {
    w0
}

/// The integer argument from a received request (word 1).
pub fn arg(w1: u64) -> u64 {
    w1
}

/// The memory-grant page count from a received request (word 2). Non-zero means one delegated
/// untyped capability follows the interrupt caps (if any) over `SEND_CAP` / `RECV_CAP`.
pub fn mem_pages(w2: u64) -> u64 {
    w2 & 0xffff_ffff
}

/// Whether this is a supervised foreground job (word 2's high bit). When set, the delegation leads
/// with two caps: a job untyped (init builds the child from it; the shell keeps it to `DESTROY`) and
/// a shared job frame (the cooperative interrupt flag and the child's status).
pub fn interruptible(w2: u64) -> bool {
    w2 & INTERRUPTIBLE_BIT != 0
}

/// The data word carried alongside the delegated untyped in the `SEND_CAP`. It is not load-bearing
/// (init identifies the cap by the protocol position, not the tag), but a fixed marker makes a
/// misrouted message obvious in a trace. Its low bits echo the page count as a cheap cross-check.
pub const CAP_TAG: u64 = 0x6361_705f; // "cap_" little-endian-ish marker

/// The sentinel init sends on the result endpoint when it could not build the child, so the
/// shell's single read completes with a legible failure rather than blocking forever. Distinct
/// from any answer a real program would report (no phase-1 program returns `u64::MAX`).
pub const SPAWN_FAILED: u64 = u64::MAX;

/// **The word for a job the kernel killed** (milestone 235,
/// design/roadmap/235-a-faulted-job-should-reach-the-prompt.md). Sent on the result endpoint by
/// `job_undertaker`, which is the process already holding init's supervision endpoint, once it has
/// collected the corpse.
///
/// It exists because a faulted job is the one outcome this protocol could not say. A child that
/// exits non-zero has answered; a child init could not build gets [`SPAWN_FAILED`]; a child the
/// kernel killed **sends nothing at all**, so the shell's single read had nothing to complete it
/// and the prompt never came back (measured 2026-09-02: `worker` patched to trap, and
/// `script/shell-check` reporting "the prompt never came back to take `worker 7`").
///
/// **Provisional name**, like everything a lane mints: a word in a protocol is exactly the kind of
/// name calef decides.
///
/// # Why the supervisor says it rather than the shell asking or the endpoint carrying it
///
/// DECISIONS §26 (the fault endpoint: thread death becomes a message a supervisor holds) delivers
/// every death to exactly **one** endpoint, so the three couplings milestone 235 named are a choice
/// of who holds that endpoint, and only one of them leaves the ordinary paths alone.
///
/// **The shell asking** loses first. A shell that asks has to decide *when* to ask, and with no
/// non-blocking receive in the ABI (`crates/system_initializer`'s own loop records that it has
/// none) that decision is a poll interval, which is a timeout wearing a different hat: it cannot
/// tell a slow job from a dead one, which is the thing the hang already could not tell.
///
/// **The endpoint carrying the death** loses on the ordinary path. Pointing a job's fault target at
/// the endpoint the shell reads would work for a fault, and §26.3 flows *exits* down the same
/// endpoint too, so every ordinary job would leave a second message on the shell's result endpoint
/// behind its answer and the next command's read would take it. It also moves collection into the
/// shell for every job, and takes every job out of init's supervision domain, which is what
/// `ps`/`pgrep` read (DECISIONS §106 already records that cost as acceptable for one narrow stage
/// and it is not acceptable for all of them).
///
/// **The supervisor telling** costs one capability and one word. `job_undertaker` already receives
/// the death, already collects the corpse, and its own `BUGS` section already recorded that it "has
/// no way to say anything". This is that sentence answered.
///
/// Distinct from [`SPAWN_FAILED`] because the two are different facts a person needs told apart:
/// nothing ran, versus something ran and died. It sits one below `u64::MAX` for the same reason
/// that one sits at it, and the same caveat applies: no program in this tree answers with either.
pub const JOB_FAULTED: u64 = u64::MAX - 1;

/// The ack init sends on the result endpoint when a **supervised** (interruptible) child started
/// cleanly. An interruptible child reports its own progress and exit through the shared job frame,
/// not the result endpoint, so init sends this once as the go-ahead: the shell reads it, then begins
/// watching the job frame. `0` is distinct from [`SPAWN_FAILED`].
pub const SPAWN_OK: u64 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        // 6, not 1: word zero must come back verbatim, and an id of 1 cannot tell verbatim from a
        // hardcoded answer.
        let (w0, w1, w2) = request(6, 9, 16, Wiring::default());
        assert_eq!(prog_id(w0), 6);
        assert_eq!(arg(w1), 9);
        assert_eq!(mem_pages(w2), 16);
        assert_eq!(wiring(w2), Wiring::default());
    }

    #[test]
    fn interruptible_bit_survives_a_zero_page_count() {
        // The interrupt demonstrators take no --mem, so the flag must ride independent of the count.
        let (_, _, w2) = request(
            2,
            0,
            0,
            Wiring {
                interruptible: true,
                ..Wiring::default()
            },
        );
        assert_eq!(mem_pages(w2), 0);
        assert!(interruptible(w2));
    }

    #[test]
    fn no_grant_is_zero_pages() {
        let (_, _, w2) = request(0, 5, 0, Wiring::default());
        assert_eq!(mem_pages(w2), 0);
    }

    /// **The seven flags are independent of each other and of the page count** (milestone 50,
    /// §67's fourth, milestone 31 phase 3's fifth, DECISIONS §106's sixth, and milestone 154's
    /// seventh). They share one word, and what init reads next off the endpoint depends on all of
    /// them, so a bit that bled into another would make init take a capability for a data word
    /// (or the reverse) and hang rather than fail.
    #[test]
    fn the_wiring_flags_do_not_collide() {
        for &interruptible in &[false, true] {
            for &sink in &[false, true] {
                for &source in &[false, true] {
                    for &diagnostics in &[false, true] {
                        for &dir in &[false, true] {
                            for &dir2 in &[false, true] {
                                for &screen in &[false, true] {
                                    let w = Wiring {
                                        interruptible,
                                        sink,
                                        source,
                                        diagnostics,
                                        dir,
                                        dir2,
                                        screen,
                                    };
                                    let (_, _, w2) = request(3, 0, 64, w);
                                    assert_eq!(wiring(w2), w, "{w:?}");
                                    assert_eq!(mem_pages(w2), 64, "{w:?}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// **The three sentinels are distinct from each other** (milestone 235). They share one word
    /// on one endpoint and the shell tells them apart by value alone, so a collision would make
    /// "nothing was built", "it ran and died" and "it started" the same message.
    #[test]
    fn the_result_sentinels_do_not_collide() {
        assert_ne!(SPAWN_FAILED, JOB_FAULTED);
        assert_ne!(SPAWN_FAILED, SPAWN_OK);
        assert_ne!(JOB_FAULTED, SPAWN_OK);
    }

    /// **`DIR2_BIT` follows [`DIR_BIT`]'s own precedent**: it is a second bit, not a count, and it
    /// round-trips independent of whether `dir` itself is set (nothing here enforces the "meaningless
    /// without `dir`" rule from the wire alone; that is the emitter's obligation, stated in
    /// [`Wiring::dir2`]'s own doc).
    #[test]
    fn a_second_directory_grant_is_a_second_bit_not_a_count() {
        let (_, _, w2) = request(
            0,
            0,
            0,
            Wiring {
                dir: true,
                dir2: true,
                ..Wiring::default()
            },
        );
        let w = wiring(w2);
        assert!(w.dir);
        assert!(w.dir2);
    }
}
