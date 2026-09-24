# Load-sensitive assertions

*(Milestone 78 (load-sensitive assertions) and milestone 62 (time-sensitive tests). This page is
the register: how to recognise one of these assertions, the rule for fixing one, and every known
site with its status. The history of each fix, with its runs and injections, is in the appendices
under [`notes/load-sensitive-assertions/`](load-sensitive-assertions/), listed at the end.)*

A load-sensitive assertion is a test that goes red because the host was busy, not because the
kernel was wrong. The guest runs under QEMU on a shared machine. When other emulators, lanes or CI
jobs compete for the cores, the guest's vCPUs are descheduled for milliseconds at a time. A test
that measured the wrong thing then fails on a pull request that could not have reached it.

The family was named on 2026-08-03, when five assertions had failed pull requests that changed no
executable code. It has grown since, and it has cost real time: a flaky red trains people to re-run
rather than read. On 2026-08-17 a loaded run produced nine reds. Eight were two known timer
assertions, and the ninth was a real double free in the kernel, wearing the same colour
([the first loaded acceptance run](load-sensitive-assertions/first-loaded-acceptance-run.md)).

## How to recognise one

Five questions sort the family. Ask them in order; each was learned from a site the earlier ones
missed.

### 1. Which direction did it fail?

A slow machine produces a deficit, never a surplus. Host contention makes work happen late or not
yet, so it can only make a count lower than the test expected.

- A positive failure ("not yet": a thread has not run, a tick has not arrived) is honest load
  sensitivity. The fix is to wait on the property, with a clock or tick budget as the bound.
- A negative failure (more free frames than at the start, fewer threads than the baseline) is not
  a timeout. The assertion was written against something wider than the property, and state from
  outside the measured window tripped it. The usual source is a neighbouring test's teardown
  landing late. `notes/riscv-parity-scope.md` named the shape: "a wait written against something
  wider than the property".

A negative failure cannot be fixed by a margin. Widening the bound only hides the defect, which is
§61's reasoning about the dropped lints, applied to assertions.

### 2. Is a yield count standing in for a duration?

A yield count is not a duration. Since §28 scatters threads across cores, a yield on a core with an
empty run queue returns in microseconds. Fifty of them can elapse before the awaited thread has been
scheduled at all. The error runs both ways: a loaded host burns yields while another vCPU is
descheduled, and a native host under Hypervisor.framework burns them in nanoseconds
([notes/hvf-leg.md](hvf-leg.md)). The same holds in a user program. A retry loop over a refusal that
a timer tick has to clear cannot buy the tick with syscalls
([the caretaker teardown wait](load-sensitive-assertions/caretaker-teardown-wait.md)).

### 3. Does the window include instructions outside the property?

Some failures are positive and still wrong. The test measures across instructions that are not part
of the claim. Examples: a tick counted between the read and the lock, a probe's execution that
placement never promised, a spinner sampled before it had a turn. Contention does not cause these
races. It stretches the window in wall-clock terms, so a race that never lost on a quiet machine
loses on a shared runner
([the second round](load-sensitive-assertions/measurement-windows-and-the-load-recipe.md)).

A per-core counter read on both sides of a wait must name its core. Otherwise a migration silently
changes the subject. That is why `ticks_on(core)` and `missed_ticks_on(core)` exist on both ISAs.

### 4. What else lands in the band that makes it fire?

An assertion is worth keeping only if its firing band contains the defect and nothing else. Where a
defect and the host produce the same band, no threshold inside it separates them. The choice is then
between a flake and a false pass. From inside the guest, a 30 ms handler and a 30 ms deschedule are
the same observation. That sentence is why `the_handler_keeps_up_when_no_lock_is_held` was deleted
rather than re-cut ([the timer disposition](load-sensitive-assertions/timer-assertion-disposition.md)).

### 5. Is a count standing in for a mechanism?

The first four questions find assertions that fail when they should not. This one finds assertions
that pass when they should not. A frame count is a weak detector of a page-table leak: eight thread
stacks are 224 KiB against a 2 MiB table span. The reaper test missed milestone 6's leak entirely,
and an injection proved it
([stack reuse and proxy detectors](load-sensitive-assertions/stack-reuse-and-proxy-detectors.md)).

### What the harness tells you

A guest knows it was late. It cannot know that eleven other emulators shared its eight cores. So
`xtask` samples the host's load average during each emulated leg and prints it when a leg goes red,
with the core count and the oversubscription factor
([the host-load line](load-sensitive-assertions/host-load-line.md)). Read that line before
theorising. Three host-side checks (`inbound`, `multicast`, `smb`) also fail together when the host
is saturated, so all three red at once is a load gauge, not three regressions.

## The rule for fixing one

1. Wait on the property, not on a proxy for it. Bound the wait with `smp.rs`'s `wait_for` (a
   clock) or with `testing::TickBudget` (delivered ticks). Assert that the wait succeeded.
2. Ask about the object, not the machine. A thread is asked about by its generational `Tid`
   (`thread_present`). A region is asked about by `memory_region::usage`, or by its own frames
   through `testing::RegionRun`. A single frame is asked about through `memory::is_page_frame_used`.
3. Denominate a budget in what the guest actually received. A descheduled emulator delivers fewer
   ticks per second, so a tick budget stretches under exactly the load that broke a counter
   deadline. A user program has no tick reading yet, so it uses a clock with a measured margin.
4. Never widen a bound to make it pass. If the claim cannot be attributed from inside the guest,
   move it to an instrument where the host is not a term (`script/icount`), or delete it.
5. Print the observation the predicate decided on. A panic that re-samples a global count can print
   an impossible quantity, and that is the message someone reads at 2 a.m.
6. Prove the fix by injection, and prove the giving-up path too. Aim the injection inside the
   test's own window: a whole-machine defect kills the suite before the target test runs. An
   injection that fires shows only that the assertion can fail, not that it fails for the right
   reason.
7. Then run it under load. Two clean matrices in a row proved nothing about the sites that failed on
   the third, loaded one.

## The instruments

- `script/repeat-under-load [-n runs] [-s spinners]` runs the suite repeatedly with one busy loop
  per core. It records elapsed time, load average and how many QEMUs were up (see
  [notes/scripts.md](scripts.md)). It surfaces a problem well and characterises one badly. Its
  "1 in 45" for the double free was a sighting, not a rate: the bug was a deterministic ownership
  defect, found by reading. A neighbouring emulator predicted reds better than the load average did.
- `script/icount` boots under `-icount shift=0,sleep=off`, where virtual time advances only when the
  guest retires instructions. It asserts the timer claims in instructions: arrival, whole-handler
  cost, zero missed ticks, and the re-arm grid law. The fourth claim was added after an injection
  showed the first three were blind to the drift bug. It is `-smp 1`, so it cannot host a cross-core
  claim. [notes/instruction-clock.md](instruction-clock.md) is its note, and
  [the icount claims](load-sensitive-assertions/the-icount-claims.md) is how it was built.
- The load recipe, for a quick reproduction on an eight-core machine: start one spinner per core,
  then run `script/cpu-matrix`. It takes the load average to about 22. Do not run it while other
  lanes are gating on the same machine; it fails their runs with the family under study.

## The register

Status is open (known and not fixed), narrowed (the exposure is smaller and a named residual
remains) or fixed. A fixed site can still carry a residual in BUGS below.

| assertion | file | status | what was done, or what is left | appendix |
|---|---|---|---|---|
| `holding_a_lock_masks_the_timer`, the liveness check | `arch/aarch64/timer.rs` | open | "the timer is not ticking at all", seen once on 2026-08-27. Positive direction; whether it wants a clock-bounded wait is unmeasured | [unowned reds](load-sensitive-assertions/unowned-reds.md) |
| five userspace retry loops over the §16 refusal | `system_initializer`, `login.rs`, `swish.rs`, `job_undertaker.rs`, `timetable.rs` | open | a yield count waiting on a timer tick. Two of them trap on exhaustion. Milestone 185 (sweep userspace's bounded retry loops onto a clock), not started | [caretaker teardown](load-sensitive-assertions/caretaker-teardown-wait.md) |
| `a_process_spends_memory_region_and_the_kernel_never_allocates` | `user/tests.rs` | open | a global `used()` equality across a process run. Never seen red; found by reading on 2026-09-24 | this page, BUGS |
| `a_user_program_that_never_yields_is_preempted_anyway` | `user/tests.rs` | open | a wall-clock window with no wait. Needs all four cores descheduled for the whole window; never seen red | [known residuals](load-sensitive-assertions/known-residuals.md) |
| `every_secondary_runs_scheduled_work` | `smp.rs` | open | indexes by the core a probe ran on. If it fails, take the placement probe's fix | [known residuals](load-sensitive-assertions/known-residuals.md) |
| `a_finished_thread_is_reaped_and_its_memory_returned` | `sched.rs` | narrowed | per-`Tid` reap waits and a waited `used() <= before` (08-03). Stack reuse asserted directly by a one-thread probe (08-17) | [first verdicts](load-sensitive-assertions/global-baselines-and-the-drift-law.md), [stack reuse](load-sensitive-assertions/stack-reuse-and-proxy-detectors.md) |
| `a_dead_user_thread_frees_its_whole_address_space` | `user/tests.rs` | narrowed | per-`Tid` reap waits and a waited `used() <= before` (08-03) | [first verdicts](load-sensitive-assertions/global-baselines-and-the-drift-law.md) |
| `kernel_stacks_do_not_touch_the_frame_allocator_in_steady_state` | `sched.rs` | narrowed | each spawn followed to its own reap; a waited `free() >= free_before` (08-16) | [miss taxonomy](load-sensitive-assertions/miss-taxonomy-and-clockless-loops.md) |
| `caretaker_teardown_reclaims_a_full_session_worth_of_memory` | `user/login_tests.rs`, `fixtures/src/login_test_client.rs` | narrowed | `destroy_with_retry` waits on the region with a 5 s clock ceiling, 40x the worst measured wait (08-28). The unit is wall clock | [caretaker teardown](load-sensitive-assertions/caretaker-teardown-wait.md) |
| `ticks_arrive_at_the_configured_rate` | both `timer.rs` | fixed | asserts the re-arm grid law (08-03). An exhausted retry budget prints `UNMEASURED` instead of failing (08-18) | [first verdicts](load-sensitive-assertions/global-baselines-and-the-drift-law.md), [timer disposition](load-sensitive-assertions/timer-assertion-disposition.md) |
| `the_handler_keeps_up_when_no_lock_is_held` | both `timer.rs` | fixed | deleted on 2026-08-18. Its band could not be attributed; `script/icount` makes the claim | [miss taxonomy](load-sensitive-assertions/miss-taxonomy-and-clockless-loops.md), [timer disposition](load-sensitive-assertions/timer-assertion-disposition.md) |
| `holding_a_lock_masks_the_timer`, the masking window | both `timer.rs` | fixed | both reads moved inside the critical section; `ticks_on(core)` (08-04) | [second round](load-sensitive-assertions/measurement-windows-and-the-load-recipe.md) |
| `work_can_be_placed_on_every_core` | `smp.rs` | fixed | asserts arrival at the named core through `PerCpu::adopted`, not execution (08-04). The first round's "leave it alone" was wrong | [second round](load-sensitive-assertions/measurement-windows-and-the-load-recipe.md) |
| `a_thread_that_never_yields_is_preempted_anyway` | `sched.rs` | fixed | the spinner is waited on, not sampled; the budget is 200 delivered ticks (08-04) | [second round](load-sensitive-assertions/measurement-windows-and-the-load-recipe.md) |
| `a_sender_blocks_until_a_receiver_arrives`, `other_threads_run_while_one_is_blocked` | `sched.rs` | fixed | five yield-count waits became `wait_for` (08-04) | [second round](load-sensitive-assertions/measurement-windows-and-the-load-recipe.md) |
| `threads_round_robin` | `sched.rs` | fixed | waits for every counter above zero, then for its own reaps (08-03) | [first verdicts](load-sensitive-assertions/global-baselines-and-the-drift-law.md) |
| five sites found on the physical core (milestone 81) | `sched.rs` (3), `user/reap_tests.rs`, `user/supervision_tests.rs` | fixed | yield counts became waits on the property (08-04) | [notes/hvf-leg.md](hvf-leg.md) |
| `reclaim_frees_an_embryo_tcbs_region` | `sched.rs` | fixed | `thread_present(tid)` in place of a global headcount (08-16) | [miss taxonomy](load-sensitive-assertions/miss-taxonomy-and-clockless-loops.md) |
| `a_migrated_kernel_thread_keeps_its_hart_pointer` | `smp.rs` | fixed | the drain budget is 200 delivered ticks through `testing::TickBudget` (08-18) | [migration drain](load-sensitive-assertions/migration-drain-tick-budget.md) |
| `a_userspace_driver_reads_a_file_over_the_pcie_transport`, and three siblings | `user/tests.rs`, `user/riscv_virtio_tests.rs` | fixed | the baseline moved before `start_pci`; x86_64's `ROUTED_IRQS` stopped counting timer ticks (09-04) | [PCIe interrupt counter](load-sensitive-assertions/x86-pcie-interrupt-counter.md) |
| `run_swap`, "returned 277 of 224 pages" | `user/live_swap_tests.rs` | fixed | the region's absence after reclaim is the measurement (09-22, #1101) | [unowned reds](load-sensitive-assertions/unowned-reds.md) |
| `the_page_is_returned_when_the_space_is_dropped` | `user/current_cpu_tests.rs` | fixed | asks the frame's own allocator bit (09-23) | [unowned reds](load-sensitive-assertions/unowned-reds.md) |
| 35 assertions bracketing `free_page_frames()` | `sched.rs`, `user/force_kill_tests.rs`, `user/tests.rs`, `user/cpu_time_tests.rs` | fixed | `testing::RegionRun` asks the region's own frames (09-23) | [free-page-frames sweep](load-sensitive-assertions/free-page-frames-sweep.md) |
| frame hygiene | `user/live_swap_tests.rs` | fixed | removed on 2026-08-03 (#46); the property was already asserted twelve lines above | [first verdicts](load-sensitive-assertions/global-baselines-and-the-drift-law.md) |

Milestone 62's acceptance evidence is two repeat counts on one laptop under eight spinners. On
2026-08-17, 36 of 45 loaded runs were green, and the reds were the two timer assertions plus the
double free. On 2026-08-22, after the disposition and the drain fix, 45 of 45 were green at a peak
load of 90 ([the confirmation run](load-sensitive-assertions/confirmation-run.md)).

### Where to look for the next one

Milestone 78's scope note counted 39 kernel sites of the family's shape. They were never audited as
a set. The rounds found theirs by reading, and four greps cover everything they found:

1. a global count taken as a baseline: `thread_count()`, `free_page_frames()`, `memory::stats()`;
2. a loop with no clock in it;
3. a frame count standing in for a mechanism;
4. a bounded retry count in a user program, over a syscall that a timer tick has to clear.

The 2026-09-23 sweep closed the first grep for `free_page_frames()` only. On 2026-09-24,
`memory::stats()` still had 22 matches in `kernel/src` and `thread_count()` had 36, and neither has
been read against these questions.

## BUGS

- Three frame bounds are still global and one-way: `used() <= before` in the reaper and
  address-space tests, `free() >= free_before` in the kernel-stack test. A real one-shot leak of
  `k` frames passes if a neighbour frees `k` frames in the same window. A persistent leak still
  fails essentially every run. See [known residuals](load-sensitive-assertions/known-residuals.md),
  which also records the reaper test's concurrency confound and the stack-reuse probe's two
  residuals.
- `a_process_spends_memory_region_and_the_kernel_never_allocates` (`user/tests.rs`) brackets a
  process run with a global `used()` equality. That is the swept shape in a function the sweep did
  not read, because it reads `memory::stats()` rather than `free_page_frames()`. It has never been
  seen red. Recorded here, not fixed.
- `ticks_arrive_at_the_configured_rate` can go unmeasured on a loaded host. It did on 9 of 36 legs on
  2026-08-18 and on 0 of 90 on 2026-08-22. A shorter window would be sound, since the law fails on
  the first drifting tick. It was refused for want of a rate measured on more than one host.
- `script/icount` is `-smp 1`. The tick path is lock-free on both ISAs today, which is why deleting
  the handler-latency assertion was safe. If the tick path ever takes a contended lock, neither
  instrument would notice.
- `work_can_be_placed_on_every_core` no longer proves that a specific core ran a specific thread.
  Nothing does, because placement is a hint and §28 deferred pins.
- `ROUTED_IRQS` is a global count. The PCIe test proves some interrupt became some driver's message,
  and it is sound only while no second driver runs in the window. A per-intid delivery count wants
  the three arch handlers to route through one portable call.
- `testing::RegionRun` works only for a root region. A child's pages return to its parent, so
  `assert_returned` would fail a correct reclaim, and nothing in the type stops that misuse. It also
  reads the run one frame at a time, so it is not atomic against another core.
- The migration drain loses coverage quietly under load: each worker's lifetime is still counter
  time. And a `TickBudget` that re-anchors on every check would never expire; the harness's per-test
  ceiling is the backstop ([migration drain](load-sensitive-assertions/migration-drain-tick-budget.md)).
- A user program cannot read delivered ticks, so `destroy_with_retry` is safe by margin, not by
  unit. Giving a process a tick reading is an ABI addition, which is calef's call.
- The host-side post-run checks are emulator-dependent. On one tree, apt's QEMU 8.2.2 and the pinned
  11.0.2 fail disjoint sets of them, and nothing in the output names the emulator. Before reading a
  red post-run check as this family or as a regression, check which QEMU ran it. Milestone 414 (a
  red post-run check does not say which emulator produced it) is the fix.
- The host-load line samples only while a leg runs, lands after the leg rather than beside the panic,
  and a load average counts runnable threads rather than contention for QEMU's core
  ([the host-load line](load-sensitive-assertions/host-load-line.md)).

## Appendices

Each appendix verifies a row or a rule above. The headings column lets an older citation of a dated
section find it: every heading this page used to carry is in one of these files.

| appendix | what it verifies | dated sections it holds |
|---|---|---|
| [global-baselines-and-the-drift-law](load-sensitive-assertions/global-baselines-and-the-drift-law.md) | the first five verdicts, the direction diagnostic, and the physical-core postscript | "The diagnostic that sorts the family", "The verdicts", "Postscript: a fast machine finds the same family" |
| [host-load-line](load-sensitive-assertions/host-load-line.md) | the harness's load report and its limits | "The harness now says whether the host was loaded (2026-08-18)" |
| [measurement-windows-and-the-load-recipe](load-sensitive-assertions/measurement-windows-and-the-load-recipe.md) | the window diagnostic, three fixes, the load recipe and its five runs | "The second round, 2026-08-04", "The instrument that was missing", "The handler-latency twins", "Recommended here, not built here" |
| [known-residuals](load-sensitive-assertions/known-residuals.md) | the BUGS list as it stood, with its dated corrections | the former "BUGS" section |
| [miss-taxonomy-and-clockless-loops](load-sensitive-assertions/miss-taxonomy-and-clockless-loops.md) | the handler taxonomy on both ISAs, the embryo and kernel-stack fixes, twelve loaded runs | "The fourth round, 2026-08-16", the 2026-08-15 aarch64 taxonomy |
| [stack-reuse-and-proxy-detectors](load-sensitive-assertions/stack-reuse-and-proxy-detectors.md) | the reaper test's blindness to milestone 6's leak, and nine injections | "The fifth round, 2026-08-17" |
| [the-icount-claims](load-sensitive-assertions/the-icount-claims.md) | the instruction-denominated timer claims and their injections | "The sixth round, 2026-08-17" |
| [first-loaded-acceptance-run](load-sensitive-assertions/first-loaded-acceptance-run.md) | 45 loaded runs, every one listed, and the double free | "The acceptance run, 2026-08-17" |
| [timer-assertion-disposition](load-sensitive-assertions/timer-assertion-disposition.md) | the band diagnostic, the deletion, `UNMEASURED`, claim 4, and 18 runs | "The disposition, 2026-08-18" |
| [migration-drain-tick-budget](load-sensitive-assertions/migration-drain-tick-budget.md) | the drain's budget in delivered ticks, its injections and cost | "The migration drain, 2026-08-18" |
| [confirmation-run](load-sensitive-assertions/confirmation-run.md) | 45 of 45 green and the rule-of-three bounds | "The confirmation run, 2026-08-22" |
| [caretaker-teardown-wait](load-sensitive-assertions/caretaker-teardown-wait.md) | the user-program retry loop, measured and fixed, and its five siblings | "The fifth round, 2026-08-27", "The disposition, 2026-08-28", "The measurement, and the number that settles it" |
| [unowned-reds](load-sensitive-assertions/unowned-reds.md) | `run_swap`, the current-cpu frame, and the timer liveness check | "Two unowned reds, recorded rather than chased: 2026-08-27" |
| [x86-pcie-interrupt-counter](load-sensitive-assertions/x86-pcie-interrupt-counter.md) | a counter sampled late, settled by reading and by a control | "A counter that could not have been late, only sampled late: 2026-09-04" |
| [free-page-frames-sweep](load-sensitive-assertions/free-page-frames-sweep.md) | all 46 reads of the global counter, classified | "The sweep, 2026-09-23" |

## See also

- [design/roadmap/78-load-sensitive-assertions.md](../design/roadmap/78-load-sensitive-assertions.md):
  milestone 78's spec and its evidence table
- [design/roadmap/62-time-sensitive-tests.md](../design/roadmap/62-time-sensitive-tests.md):
  milestone 62, the acceptance standard
- [notes/instruction-clock.md](instruction-clock.md): the icount instrument and its four claims
- [notes/cpu-models.md](cpu-models.md) BUGS: the load-sensitivity evidence, including the control
  model failing
- [notes/live-replacement.md](live-replacement.md) BUGS: the frame-hygiene analysis, the template
- [notes/riscv-parity-scope.md](riscv-parity-scope.md): where the shape was first named
- [notes/benchmarks.md](benchmarks.md): the two instruments, and why icount cannot host cross-core
  tests
