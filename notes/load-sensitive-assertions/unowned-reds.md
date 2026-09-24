# Two unowned reds, and a third, 2026-08-27

*(An appendix of [notes/load-sensitive-assertions.md](../load-sensitive-assertions.md), which holds
the register and the rules. Moved there verbatim on 2026-09-24; "this page" and "above" in the text
below meant the single note it came from.)*

## Two unowned reds, recorded rather than chased: 2026-08-27 (milestone 185's lane)

The paragraph above named both and moved on; this is the fuller entry the diagnostic asks for, so a
reader who meets either site does not have to re-derive what the two-line version already knew.
Neither line number has moved since the paragraph above was written (checked directly against
`main`, 2026-08-27), and neither site was touched: this lane's brief was to record, not to chase.

### `kernel/src/user/live_swap_tests.rs:263`, `run_swap`: "reclaiming the operator's budget returned 277 of 224 pages" **FIXED 2026-09-22**

```rust
let recovered = memory::free_page_frames() - before_reclaim;
assert_eq!(
    recovered, SWAPPER_BUDGET_PAGES as usize,
    "reclaiming the operator's budget returned {recovered} of {SWAPPER_BUDGET_PAGES} pages",
);
```

`run_swap` is the shared teardown helper all four `#[test_case]`s in this file call, not one test's
own body, so any of the four could have been the run that hit it. `SWAPPER_BUDGET_PAGES` is 224; the
observed run recovered 277, **53 pages more than the operator's own budget held.**

**Applying this page's first diagnostic (line 14 above): a slow machine produces a deficit, never a
surplus.** Host contention can only make `reclaim_region` return late or with less than expected; it
cannot manufacture 53 extra pages out of a budget that was only ever 224. A count higher than the
property being measured is the negative-direction shape this page already has a name for, "a wait
written against something wider than the property" (`notes/riscv-parity-scope.md`'s phrase, used
throughout this file), which here reads as free frames from *outside* `budget` landing inside the
measured window, most likely a neighboring test's own teardown completing while this one's
`before_reclaim`/`reclaim_region` pair was in flight.

**That diagnosis was right, and the fix follows from it.** The assertion read
`memory::free_page_frames()`, a count of every free frame in the machine, so anything else
allocating or freeing between the two reads moved it. It now reads `memory_region::usage(budget)`,
which counts the pages retyped out of *this* region, and that is the quantity the test was always
trying to assert: that the operator's budget gave back exactly what it held. A neighbouring test's
teardown can no longer be mistaken for this one's result.

**The narrower assertion is also a stronger one.** A global delta of the right size can be reached
by the wrong frames coming back; a scoped one cannot.

**The first version of this fix was wrong, and CI caught what the local gates could not.** It asked
`memory_region::usage(budget)` *after* the reclaim and unwrapped it. But `reclaim_region` destroys
the region, so the name is stale and `usage` returns `None`: the test panicked with "the operator's
budget should still exist after reclaim" on every architecture. The fix had been written by a lane
that never committed or ran it, and `script/lint` and `script/fmt` cannot boot QEMU, so it reached a
pull request looking clean. What the assertion should say is the opposite of what it said: the
region's **absence** is the measurement, because §16 (object revocation: reclaim the objects a process built) refuses to reclaim a region whose
children are
still carved out of it, so a stale name proves every child was gone first.

***Two paragraphs below this one used to sit under the `current_cpu_tests.rs` entry, and they are
this site's: both quote `run_swap`'s own message ("N of 224 pages"), which the other test does not
print. Moved here 2026-09-23, and the stale "PR #XXX" placeholder resolved to the pull request that
actually made the change.***

**Seen a second time, 2026-09-21**, by the milestone 198 (a package manager, and the trivial install
that makes a second customer possible) rung 2b lane, as *"returned 296 of 224 pages"*: a **72**-page
surplus where the first sighting was 53. Three things the second sighting adds, and none of them
closes it.

It **recurs** rather than being one run's accident, and the surplus is a different number each time,
which is what a neighbouring teardown landing in the window would look like and is not what a fixed
accounting error would look like. It is **intermittent**: the same binary, on the same machine, the
same afternoon, passed the assertion twice and failed it once. And the failing run was the loaded
one, load 7.0 of 8 cores with another lane's two runaway doctests pegged at 99% each, while both
passing runs were quieter; that does not contradict the diagnostic above, because load does not
manufacture pages, but it fits the "a neighbour's teardown completes inside my measured window"
reading, since contention is what stretches the window.

So the second sighting **strengthens the negative-direction diagnosis and still does not establish
whether the frames come from a neighbour or from a leak**. It remains un-investigated and it remains
worth a lane.

**Fixed 2026-09-22**: The assertion now measures a scoped quantity (the specific budget region's
page count) rather than a global one (the total free frame count). This eliminates the sensitivity
to concurrent activity on the machine while preserving the test's ability to detect actual leaks.
See pull request #1101.

### `kernel/src/user/current_cpu_tests.rs:134`, `the_page_is_returned_when_the_space_is_dropped` **FIXED 2026-09-23**

```rust
let before = crate::memory::free_page_frames();
{ /* load a space, check it has a current-cpu page */ }
assert_eq!(crate::memory::free_page_frames(), before,
           "dropping an address space did not return its current-cpu frame");
```

**Same global counter, same failure, seen 2026-09-22** on pull request #1094, whose diff touches
only a maintainer script and so cannot have caused it. It failed on the riscv64 CPU matrix while
`main` was green, which is the signature of a load-sensitive assertion rather than a regression.

**It cannot take the fix above, and that is worth saying where a reader meets it.** The frame this
test watches is the one an address space owns that *its region does not pay for*: the whole reason
the test exists is that `memory_region::destroy` does not cover it and `Drop` must free it by hand.
So there is no region whose usage could be queried instead, and a scoped counter is not available
the way it was for `run_swap`. Fixing this one needs a different mechanism, and it has not been
chased.

***The paragraph above is the record of what was not available, and it is still true: there is no
region to scope to. What was missed is that a scoped quantity does not have to be a region.***

**The fix asks about the frame, not about a count.** The test holds the page's kernel virtual
address while the space is alive (`current_cpu_page_kernel_va`), that address is the frame through
the direct map, and `mmu::virt_to_phys` inverts the map, so the test can name the physical frame the
allocator handed out. After the drop it asks `memory::is_page_frame_used(frame)`, which reads that
one frame's bit in the bitmap:

```rust
let frame = {
    let space = load(current_cpu_reader_image(), 0).expect("load failed").0;
    let va = space.current_cpu_page_kernel_va().expect("...");
    let frame = PageFrame::containing(mmu::virt_to_phys(va));
    assert_eq!(memory::is_page_frame_used(frame), Some(true), "...");
    frame
};
assert_eq!(memory::is_page_frame_used(frame), Some(false), "...");
```

**Why a global count could never work here, in one sentence: the quantity it measures is not the
property.** `free_page_frames()` is a fact about the whole machine, so bracketing it asserts that
nothing else in the kernel allocated or freed for the length of the window, which is a claim this
test has no business making and cannot keep on any loaded run. The frame's own bit is the property
itself: one bit, owned by this space, set by `attach_current_cpu_page` and cleared only by `Drop`.

**Leak sensitivity is not merely preserved, it is sharper.** A frame `Drop` never returned stays
marked used forever, so the defect fails this assertion on every run rather than on the runs where
the arithmetic happens to be visible; and unlike the old form, a leak here cannot be masked by
somebody else freeing the same number of frames in the same window (the coincidence caveat this
page's BUGS section records against the `<=` sites). The vacuity guard before the drop is what keeps
the inversion honest: `Some(true)` proves the address names a frame this allocator owns, so a wrong
direct-map inversion fails loudly instead of returning `None` and reading as "not used".

**The exposure that replaces the old one is far smaller and is recorded rather than hidden.** A
neighbour that allocated *this exact frame* in the microseconds between the drop and the check would
fail it falsely. That needs an allocation, where every observed failure of the old form was a
*freeing* neighbour's late teardown, and it needs the allocator's linear scan to land on this one
frame out of the machine's free set.

**Proved by injection, 2026-09-23**, because this page's fifth round is emphatic that a clean run
proves nothing about an assertion. Deleting the `crate::memory::free(frame)` call from
`AddressSpace::drop`, which is this test's defect exactly, turns the leg red:

```
test kernel::user::current_cpu_tests::the_page_is_returned_when_the_space_is_dropped ...
[PANIC] assertion `left == right` failed: dropping an address space did not return its
current-cpu frame at 0x4002d000
  left: Some(true)
 right: Some(false)
```

The frame's address is in the message, which the old form could not print: a global delta names a
quantity, not an object. Reverted; the same filtered run on the clean tree reports `test result: ok.
1 passed`, and the full aarch64 leg is green.

**The first sighting's open question is answered by construction rather than by investigation.** The
paragraph below asked whether the surplus was a neighbour or a leak, and said it was worth a lane.
It no longer is: the assertion can no longer be moved by a neighbour at all, so if it ever goes red
the answer is "a leak" with nothing else to rule out first.

**The original wording, kept because it is the question the fix retires:** this is not explained by
load, and it may be a real
bug. Every negative-direction case this page has actually chased turned out to be a test written
against a global counter that something else could also move (the reaper count, the address-space
frame count, both rescoped to a narrow `Tid`-scoped wait in the fourth round), never a kernel defect
undiscovered underneath. Whether this one is the same shape or something a swap-system regression
put there is not established, because it was not investigated here per this lane's brief. Proposed
in this lane's report as a milestone of its own (provisional; the integrator mints the number),
rather than chased in place. *(Retired by the fix above: there is nothing left for that lane to
investigate.)*

### `kernel/src/arch/aarch64/timer.rs:680`, `holding_a_lock_masks_the_timer`: "the timer is not ticking at all"

```rust
let alive_on = crate::cpu::id();
let t0 = timer::ticks_on(alive_on);
timer::spin_for(timer::interval() * 2);
assert!(
    timer::ticks_on(alive_on) > t0,
    "the timer is not ticking at all"
);
```

This is the liveness check at the top of `holding_a_lock_masks_the_timer`, ahead of the
lock-masking assertion the second round (2026-08-04) already core-scoped ("the window moved inside
the lock", above). That earlier fix touched the two reads bracketing the critical section; this
read pair, `t0` before `spin_for` and the assertion after it, is a different window in the same
function and was not part of that round's audit.

**Applying the diagnostic: the failure direction is positive** ("not yet": `ticks_on(alive_on)`
did not exceed `t0` after spinning across two tick intervals), which by line 19 above is honest load
sensitivity rather than a wait written against something wider than the property. `ticks_on` is
already core-scoped (`ticks_arrive_at_the_configured_rate`'s fix, reused here), so this is not the
migration hazard a bare `ticks()` would have. What it needs to be a genuine failure of this kernel
rather than of the host is a vCPU denied the core for the whole two-interval spin, which an
oversubscribed host can do and which killing a run before it even reaches the suite's userspace half
is consistent with: this leg never got far enough to log a host-load line (`HostLoad` samples while
the leg is *running*, and a leg that dies output-less here has nothing for it to have sampled).

**Not chased, per this page's own rule that the second kind of failure ("not yet") is the one a
clock-bounded wait or a wider margin actually fixes**, and per this lane's brief, which was to
record rather than to change kernel timer tests. Whether it wants a `wait_for`-shaped rescope, or is
rare enough not to, is unmeasured; the site is a sibling of the family milestone 62 and this page's
earlier rounds already dispositioned, and it was not one of the ones they reached.
