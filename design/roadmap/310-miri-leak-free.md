# 310. The Miri leak check stays on, and the test pool it caught cannot leak again

**Status: BUILT 2026-09-17.** Built by a lane on `milestone/310-miri-leak-free`.
*(Number provisional until the merge queue lands it.)*

`.github/workflows/undefined-behavior-check.yml` had **never once succeeded**. Five scheduled runs
since 2026-08-11, all red, with `script/cadence-check` (milestone 238) reporting it DEAD. The cause
was not undefined behaviour:

```
error: memory leaked: alloc78416 (Rust heap, size: 4096, align: 4096), allocated here:
   --> crates/paging/src/domain.rs:425:26
error: aborting due to 5 previous errors
error: test failed, to rerun pass `-p paging --lib`
```

`crates/paging`'s domain tests back synthetic physical addresses with real zeroed host frames, so
the page-table walker has memory to walk. Those frames were never freed. `cargo test` does not care,
because the process exits; Miri checks for leaks by default and does.

## The part worth reading: the guard was not missing

The obvious story is "a fixture nobody taught to free". It is wrong, and the correct story is the
reason this milestone changed a shape instead of adding a line.

`PoolGuard` was added on **2026-08-03** (`d0f74db9`, *"paging: the test harnesses free what they
allocate"*) for exactly this leak, with a `Drop` that drains the thread-local map and deallocates
every frame. It worked. What it required was that each test *remember* to write
`let _pool = PoolGuard;` as its first line.

`a_multi_page_grant_maps_its_whole_pages_and_not_the_partial_tail`, written afterwards, did not.
One test out of four, five frames, five weeks of red.

That is `AGENTS.md`'s ladder, rung four, failing the way rung four fails, and the whole list of
sites confirms it: `page_frame_at` was the only function allocating, `page_frame` the only other
caller of it, and the thread-local `PHYS` had no third writer anywhere in the crate. Nothing was
subtle. **Binding the missing line would have restored the identical defect for the next test
author**, and the next author is exactly who this file says cannot be relied on to remember.

So the fix goes up the ladder to rung one, *make the wrong state unrepresentable*: `frame` and
`frame_at` are now methods on `FramePool`, the free functions are gone, and **a test that allocates
a frame without holding a pool does not compile.** The receiver is otherwise unused, and that is the
point of it.

Three smaller things came with it, each closing a way the fix itself could be wrong:

- **The layout is named once** (`frame_layout()`), because `alloc` and `dealloc` must agree exactly
  and a `dealloc` with a mismatched layout is itself undefined behaviour. Failing a UB check with UB
  introduced by the UB fix is an available outcome and it is now unreachable by construction.
- **`FramePool::new` refuses a second live pool on one thread.** Nesting would make the inner drop
  free the outer's frames, leaving live entries in `PHYS` pointing at freed memory: a use-after-free
  discovered later instead of a panic discovered now. `PHYS` is thread-local, which is what makes
  one-pool-per-thread the right invariant; the test harness runs each test on its own thread and no
  test in the crate spawns one.
- **A `BUGS` section on `FramePool` itself**, where a reader meets the fixture, saying why the frames
  are freed and what breaks if they are not.

## The two options that were refused, and why

calef ruled on 2026-09-17. Recording the refusals rather than only the choice, per §75:

**`-Zmiri-ignore-leaks`, workspace-wide. Refused.** It trades a permanent reduction in what Miri
checks for a one-time cost, which is the wrong direction on a check that had already gone five weeks
without anybody noticing it was checking nothing. This project's `no_std` crates are host-testable on
purpose, and a future host test of an allocator is precisely the thing leak checking would earn its
keep on; turning it off now spends that in advance to save an afternoon.

**A `paging`-scoped ignore. Refused.** Narrower, and still an ignore, but it also does not exist:
`cargo xtask undefined-behavior-check` runs one `cargo miri test --workspace` with a static exclusion
list and `MIRIFLAGS` is a process-wide environment variable, so per-crate flags would need a
mechanism built first. Paying for machinery whose purpose is to check less is the losing side of both
arguments at once.

The honest cost of what was done instead: **one afternoon, one crate, 60 lines changed in one test
module, no production code touched, and no test covering less than it did.** That comparison is why
the decision was not close.

## Freeing changed nothing about what the tests cover

Worth stating explicitly, because "make the leak go away" has a bad version where the frames stop
being allocated and the walker stops walking real memory. Every test allocates exactly the frames it
did before, at the same synthetic addresses, and asserts the same things. The only difference is that
the allocations are reachable from a value that outlives them and frees them. All four domain tests
and the crate's other thirty pass unchanged.

## BUGS

- **Nothing checks that a new fixture in some *other* crate does not do the same thing.** This
  milestone made one fixture's leak unrepresentable; the general shape (a test that allocates host
  memory and lets the process clean up) is caught only by the weekly Miri run, which is exactly the
  detection latency that let this sit for five weeks. `script/cadence-check` shortens the latency on
  the *job going quiet*, not on the job going red.
- **The weekly run's cost is still unanswered.** The last CI run took **141 minutes** and the
  workflow's own header says the honest current cost "is not yet known" and asks for the budget to be
  tightened once `compositor` is sampled. Turning the job green is what finally makes that question
  answerable, and it is not answered here. See the follow-on below.
- **`FramePool` is a provisional name** (a private type inside `#[cfg(test)] mod tests`, not a public
  surface). It replaces `PoolGuard`, which named the `Drop` half of a thing that is now mostly an
  allocator.

## Follow-on

- **Is a 141-minute weekly Miri run worth what it costs?** The workflow's header raises it and nobody
  has answered. Now that a full run can actually finish, the inputs exist: what the run found in five
  weeks of working (nothing, because it never ran), what it found in its working life before that,
  and which crates dominate the wall clock. `compositor`'s six full-screen sweeps at 317,856 pixels
  each are named in the header as a milestone of their own, and sampling them under `cfg(miri)` the
  way `gpt`, `calendar` and `network_time_protocol` already do is the obvious first cut. A lane.
