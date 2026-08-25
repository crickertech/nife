//! `watch_tests`: `crates/watch`'s `frame`, driven by the real `endpoint::SURVEY` dispatcher against
//! a real, changing domain, and fed into a real `video_terminal::Vt`. `survey_tests`'s discipline,
//! one program over: every survey here is `ps::collect` (the same walk `crates/watch`'s `frame`
//! wraps), never a description of one, and the terminal it renders into is the same engine
//! `display_terminal.rs` puts a real screen behind.

use super::supervision_tests::{FAULT_STUB, REPORT_STUB, build_child_in};
use crate::arch::exceptions::TrapFrame;
use crate::cap::Rights;
use crate::sched;
use crate::syscall::invoke;

/// The builder's whole budget: two instances plus slack. `survey_tests::BUILDER_BUDGET_PAGES`'s
/// reasoning, sized down because this module's widest domain is two.
const BUILDER_BUDGET_PAGES: u64 = 64;

/// Pages per instance region: `survey_tests::INSTANCE_PAGES`, the same carve.
const INSTANCE_PAGES: u64 = 16;

/// Pages for this test's own rendezvous points (the domain and the parking rendezvous).
const RENDEZVOUS_PAGES: u64 = 4;

/// The widest domain any test here builds is two; `ps::collect`'s buffer just needs to hold that
/// plus slack, `survey_tests::TEST_ROWS`'s reasoning.
const TEST_ROWS: usize = 8;

/// One test's world: a budget the builder owns, and a region the rendezvous points come out of.
/// `survey_tests::arena`, verbatim.
fn arena() -> (u64, u64) {
    let budget = crate::memory_region::create(BUILDER_BUDGET_PAGES).expect("no builder budget");
    let rendezvous_region =
        crate::memory_region::create(RENDEZVOUS_PAGES).expect("no rendezvous region");
    (budget, rendezvous_region)
}

fn rendezvous(region: u64) -> sched::RendezvousId {
    sched::create_rendezvous_from(region).expect("no rendezvous")
}

/// Hold a domain **the way `watch` holds it**: `ENUMERATE` and nothing else. `survey_tests::hold_view`,
/// verbatim, renamed for this module's own reader: `watch` is exactly `ps`'s viewer, one program over.
fn hold_view(ep: sched::RendezvousId) -> u64 {
    sched::grant(crate::cap::rendezvous_cap(ep, Rights::ENUMERATE)).expect("grant the rendezvous")
}

/// Hold a domain **the way its supervisor holds it**: `READ` (which carries `RECV` and `REAP`) plus
/// `ENUMERATE`. Used only by this test, to collect the corpse `watch`'s own `ENUMERATE`-only capability
/// could never reap; `watch` never touches this capability, which is the whole point of the milestone.
fn hold_supervisor(ep: sched::RendezvousId) -> u64 {
    sched::grant(crate::cap::rendezvous_cap(
        ep,
        Rights::READ.union(Rights::ENUMERATE),
    ))
    .expect("grant the rendezvous")
}

fn child_in(
    budget: u64,
    stub: &[u32],
    report: Option<sched::RendezvousId>,
    fault_ep: sched::RendezvousId,
) -> u64 {
    let region = crate::memory_region::split(budget, INSTANCE_PAGES).expect("no instance region");
    build_child_in(region, stub, report, Some(fault_ep))
}

/// Let a parked [`REPORT_STUB`] finish: it is blocked in a send on `parking`, so taking its message
/// off releases it to exit (and become a corpse this test must still reap, `collect_all`'s point).
/// `survey_tests::drain`, verbatim.
fn drain(parking: sched::RendezvousId, n: usize) {
    for _ in 0..n {
        sched::ipc_recv(parking);
    }
}

/// `invoke(cap, REAP, tid, _, _)`, through the real dispatcher. `survey_tests::reap`, verbatim.
fn reap(slot: u64, tid: u64) -> Result<i64, abi::Error> {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    invoke(&mut frame, slot, abi::rendezvous::REAP, tid, 0, 0)
}

/// `invoke(cap, SURVEY, cursor, _, _)`, through the real dispatcher. `survey_tests::survey`, verbatim.
fn survey(slot: u64, cursor: u64) -> (i64, u64, u64) {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    match invoke(&mut frame, slot, abi::rendezvous::SURVEY, cursor, 0, 0) {
        Ok(next) => (next, frame.arg(1), frame.arg(2)),
        Err(e) => (e as i64, 0, 0),
    }
}

/// The whole domain, walked by `watch`'s own logic: `ps::collect` driving real syscalls, exactly what
/// `crates/watch`'s `frame` is handed in `user/src/watch.rs`.
fn walk(slot: u64, rows: &mut [ps::Row; TEST_ROWS]) -> ps::Survey<'_> {
    let s = ps::collect(rows, &mut |cursor| survey(slot, cursor));
    assert!(
        s.complete() || s.refused(),
        "the survey outgrew this test's row buffer, so what it reported is not the domain",
    );
    s
}

/// **Everything currently on the grid, in one flat buffer**, so a check can ask "does this number
/// appear anywhere on screen" without assuming which row or column a line landed at. Needed because
/// this engine's `LF` does not return the carriage (`video_terminal`'s own tests pin `LF alone must
/// not return the carriage`), so a multi-line table's rows do not all start at column 0 the way a
/// naive reader would expect; a check keyed to exact row/column position would be testing that
/// quirk, not `watch`. `[u8; 256]` is `MAX_COLS * MAX_ROWS_TALL` below, on the stack: this runs on a
/// kernel thread stack under a 4,096-byte guard page, the same reason `TEST_ROWS` above is 8 and not
/// `ps::MAX_ROWS`.
const ROWS_TALL: u32 = 8;
const GRID_BYTES: usize = video_terminal::MAX_COLS * ROWS_TALL as usize;

fn grid_text(vt: &video_terminal::Vt, out: &mut [u8; GRID_BYTES]) {
    let mut row_buf = [0u8; video_terminal::MAX_COLS];
    for r in 0..vt.rows() {
        vt.row_bytes(r, &mut row_buf);
        let at = r as usize * video_terminal::MAX_COLS;
        out[at..at + video_terminal::MAX_COLS].copy_from_slice(&row_buf);
    }
}

/// **Whether `needle` (an exact number's ASCII digits) appears in `haystack` as a whole token**,
/// meaning the byte immediately before and after (if any) is not itself an ASCII digit. Plain
/// substring search is not enough: a raw kernel-assigned tid is not a small, disjoint number the way
/// this module's own fixture tids are, and tid 2 is a substring of tid 25's digits. Written without
/// `alloc`: this kernel builds no heap, so every buffer in this module is a fixed-size array.
fn contains_exact_number(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    for start in 0..=(haystack.len() - needle.len()) {
        if &haystack[start..start + needle.len()] != needle {
            continue;
        }
        let before_ok = start == 0 || !haystack[start - 1].is_ascii_digit();
        let end = start + needle.len();
        let after_ok = end == haystack.len() || !haystack[end].is_ascii_digit();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// `tid`, formatted as ASCII decimal digits into `buf`, returning the slice actually used.
/// `crates/ps`'s own `write_tid`, the digit loop, without the padding: this only needs the digits
/// themselves to search for.
fn digits(tid: u64, buf: &mut [u8; 20]) -> &[u8] {
    let mut i = buf.len();
    let mut v = tid;
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    &buf[i..]
}

/// Give everything back. `survey_tests::tidy`, verbatim.
fn tidy(budget: u64, rendezvous_region: u64, slots: &[u64]) {
    for &s in slots {
        let _ = sched::delete_current_cap(s);
    }
    sched::reclaim_region(budget).expect("the builder's own budget did not come back");
    sched::reclaim_region(rendezvous_region)
        .expect("the test's rendezvous region did not come back");
}

/// **The claim: a second frame erases the first rather than leaving it on screen**, checked against
/// two real snapshots of a domain that genuinely changed, through the real `SURVEY` syscall, the real
/// `ps::collect` walk `watch`'s own program runs, and a real `video_terminal::Vt`.
///
/// One member (`b`, a [`REPORT_STUB`]) stays parked and alive for the whole test. A second (`a`, a
/// [`FAULT_STUB`]) dies on its own and is still counted (as `DEAD`) until reaped, exactly
/// `survey_tests::a_dead_child_is_still_in_the_domain_until_it_is_reaped`'s finding. The first frame
/// is drawn while both are members; a **separate** capability with `READ` (which `watch` itself is
/// never granted; see `crates/watch`'s module docs on why a domain names and does not act) then reaps
/// `a`, and the second frame is drawn with only `b` left. A `watch` that only overwrote instead of
/// erasing would leave `a`'s tid sitting in the grid forever, since the second frame never writes over
/// that cell at all: nothing in a one-row-shorter table touches it.
#[test_case]
fn a_second_frame_erases_the_first_rather_than_leaving_it_on_screen() {
    let (budget, rendezvous_region) = arena();
    let mine = rendezvous(rendezvous_region);
    let parking = rendezvous(rendezvous_region);

    // `b` parks on `parking` and stays a live member for the whole test.
    let b = child_in(budget, REPORT_STUB, Some(parking), mine);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(parking) == 1),
        "b never reached its parking send",
    );

    // `a` faults on its own; its death message queues on `mine` itself (the domain rendezvous is
    // also its fault target), where it stays until reaped.
    let a = child_in(budget, FAULT_STUB, None, mine);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(mine) == 1),
        "a never died onto the domain rendezvous",
    );

    // `watch`'s own capability: `ENUMERATE` and nothing else. Everything from here that is not a
    // `walk()` through `viewer` is something `watch` itself could not do.
    let viewer = hold_view(mine);

    let mut buf1 = [ps::Row::default(); TEST_ROWS];
    let survey1 = walk(viewer, &mut buf1);
    assert_eq!(
        survey1.rows().len(),
        2,
        "both members should be in the first frame"
    );

    let mut vt = video_terminal::Vt::new(video_terminal::MAX_COLS as u32, ROWS_TALL);
    watch::frame(&survey1, &mut |bytes| vt.feed(bytes));

    let mut grid1 = [0u8; GRID_BYTES];
    grid_text(&vt, &mut grid1);
    let (mut da, mut db) = ([0u8; 20], [0u8; 20]);
    let da = digits(a, &mut da);
    let db = digits(b, &mut db);
    assert!(
        contains_exact_number(&grid1, da),
        "a's tid should be on screen after the first frame",
    );
    assert!(
        contains_exact_number(&grid1, db),
        "b's tid should be on screen after the first frame",
    );

    // Reap `a`, through a capability `watch` is never granted (`READ`, not `ENUMERATE` alone):
    // drain its death message, then collect it. `survey_tests`'s exact sequence.
    let supervisor = hold_supervisor(mine);
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    invoke(&mut frame, supervisor, abi::rendezvous::RECV, 0, 0, 0).expect("RECV refused");
    assert_eq!(
        invoke(&mut frame, supervisor, abi::rendezvous::REAP, a, 0, 0),
        Ok(0),
        "a's tid was not one the same rendezvous could reap",
    );

    let mut buf2 = [ps::Row::default(); TEST_ROWS];
    let survey2 = walk(viewer, &mut buf2);
    assert_eq!(
        survey2.rows().len(),
        1,
        "only b should remain in the second frame"
    );

    watch::frame(&survey2, &mut |bytes| vt.feed(bytes));

    let mut grid2 = [0u8; GRID_BYTES];
    grid_text(&vt, &mut grid2);
    assert!(
        !contains_exact_number(&grid2, da),
        "a's tid from the first frame is still on screen: watch overwrote instead of erasing",
    );
    assert!(
        contains_exact_number(&grid2, db),
        "b's tid should still be on screen in the second frame",
    );

    // Release `b` and reap it too, or its region refuses to be reclaimed: a live thread still
    // occupying part of the builder's budget is exactly what `tidy`'s own reclaim would otherwise
    // hit. `collect_all`'s pattern (poll `reap`, no explicit `RECV` first: unlike the `RECV`-then-
    // `REAP` sequence above, a corpse that has not yet queued its death message just answers
    // something other than `Ok(0)` until it has).
    drain(parking, 1);
    assert!(
        super::wait_for(|| reap(supervisor, b) == Ok(0)),
        "b was never reaped, so its region would refuse to be reclaimed",
    );

    tidy(budget, rendezvous_region, &[viewer, supervisor]);
}
