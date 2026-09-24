# The caretaker teardown wait, 2026-08-27 and 2026-08-28

*(An appendix of [notes/load-sensitive-assertions.md](../load-sensitive-assertions.md), which holds
the register and the rules. Moved there verbatim on 2026-09-24; "this page" and "above" in the text
below meant the single note it came from.)*

## The fifth round, 2026-08-27: caretaker teardown joins the family, observed rather than rescoped

### `caretaker_teardown_reclaims_a_full_session_worth_of_memory` (`user/login_tests.rs`): fails at roughly 2x oversubscription, passes quiet, passes CI

Found by milestone 142's bench-regression lane while it was gating an unrelated change, and
recorded here as an observation with its evidence; nothing has been rescoped and the mechanism is
a hypothesis, not a verdict.

**The observation, three failing runs and two passing ones on the same tree.** With the aarch64
suite sharing this host with a second lane's full suite (the harness's own load line read 14.5 to
16.2 one-minute average on 8 cores, 1.9 to 2x oversubscribed), the test failed its
`F_TEARDOWN_OK` assertion ("login N's logout ticket did not destroy the caretaker's construction
region", `left: 0, right: 16`) on login 6, login 6 again, and, at a clean checkout of the branch
head with no local changes at all, login 8. The same clean-head code passed the whole test in
CI's own QEMU leg the same day, and passed locally, twice, the moment the competing suite
finished (the passing run's load line: back under 1x by the QEMU phase). The failing iteration
moving between 6 and 8 is the tell it shares with the rest of this file's family: the code path
is fixed, the schedule is not.

**The hypothesis, unverified.** `MemoryRegion::DESTROY` refuses a region that still holds a live
thread, and `login_test_client`'s teardown role has a bounded patience for that refusal; a
session-worth of threads takes longer to finish exiting on a 2x-oversubscribed host than that
patience allows. That is the same shape as the reaper-count and address-space-frames verdicts
above, which is why it is recorded in this note rather than in the test's own file. Whoever picks
this up should start from those two sections' dispositions.

**The three network checks fail in the same runs and recover in the same runs** (`inbound`,
`multicast`, `smb`: connection refused or reset for the whole window), which is worth knowing so
the cluster is read as one host-load event and not as four independent regressions.

## The disposition, 2026-08-28: the teardown wait is a yield count, and now it is a clock

The section above ends by saying nothing has been rescoped and the mechanism is a hypothesis rather
than a verdict. This is the verdict. It is also the first one this page has had to write about a
**user program**, which matters more than "one more assertion" would suggest: every fix above
reaches for `smp.rs`'s `wait_for` or `testing::TickBudget`, and a process can call neither.

**A correction this lane owes on its own account, because it got the record wrong in the other
direction.** It was briefed from the section above and went looking for it here, found the newest
section to be the 2026-08-22 confirmation run, checked `main` as well, and wrote a paragraph saying
the observation had never been written down anywhere. That was false by about an hour:
the observation was on `milestone/142-terminal-size`, and that branch merged
(`ac41c827`, in PR #545) while this lane was running its loaded runs. Both halves are worth keeping.
The claim was checked against `main` and was right when it was checked, which is the honest part;
and it was still wrong, because **a branch is not a record**, which is AGENTS.md's own "nobody
reads branches" arriving from the reader's side rather than the writer's. A lane cannot see another
lane's branch, so "I checked and it is not in the tree" means "not yet", and the merge is what
decides.

The one trace that was on `main` the whole time is in a place a reader meets by accident:
`kernel/src/testing.rs`'s `SUITE_PAGE_FRAME_BUDGET` takes its number from CI rather than from this
laptop because "this milestone's local host reproduces an unrelated, pre-existing flake in
`caretaker_teardown_reclaims_a_full_session_worth_of_memory` that does not occur in CI". A frame
budget's comment is where that fact had been living.

### The hypothesis held in outline and was wrong about the mechanism

The hypothesis above: `DESTROY` refuses a region that still holds a live thread, and the client's
teardown role has a bounded patience for that refusal that a session's worth of threads can outrun
under load. Reproduced on the first instrumented run, so the shape is confirmed. **The mechanism is
not what the words say**, and that is the part worth keeping. The patience is not too small for the
number of threads; there is only ever one thread in that region. It is denominated in the wrong
thing:

```rust
fn destroy_with_retry(ut: u64) -> bool {
    const ATTEMPTS: usize = 64;
    for _ in 0..ATTEMPTS {
        if destroy_region(ut) == 0 { return true; }
        yield_now();
    }
    false
}
```

What the refusal is waiting on is **a timer tick**. `sched::reap_region_objects` refuses while the
caretaker is still live and *arms* DECISIONS §16's kill on it (`region_reap_verdict`'s
`RefuseAndArm`), and an armed kill lands at that thread's next preemption. So the wait is one tick
period, 10 ms at 100 Hz, and a count of syscalls is not a way to buy one. The direction is positive
("not yet"), so by this page's first diagnostic it is honest load sensitivity rather than a wait
written against something wider than the property.

### The measurement, and the number that settles it

The client was instrumented to report both what it spent, in attempts and in counter ticks, and the
first run reproduced the failure. Every logout of that run, in order (aarch64, one-minute load
average 9.4 to 10.8 on eight cores, which the harness's own host-load line reported):

| logout | attempts | elapsed | verdict |
|---|---|---|---|
| 0 | 23 | 5.97 ms | ok |
| 1 | 36 | 10.79 ms | ok |
| 2 | 2 | 8.60 ms | ok |
| 3 | **64** | **8.26 ms** | **red** |

**The failing wait is the shortest one in the table.** It gave up after 8.26 ms having spent its
whole budget, where the logout above it took 10.79 ms and passed. Attempts and elapsed time are not
the same axis at all: a `yield_now` that finds work on this core returns in about 130 us, and one
that finds none parks until the next tick, so sixty-four of the first kind elapse in 8 ms while two
of the second cover 12 ms. Which kind a logout gets is a scheduling outcome the host decides. That
is "a yield count is not a duration", the second round's own sentence, arriving in a program instead
of in the scheduler.

The A/B below produced the cleaner version of the same fact in a single run: the old form gave up on
logout 6 after **26.6 ms**, in a run where logout 3 had legitimately taken **39.3 ms** and passed.

**That also answers the question the section above left hanging**, which is why the failing
iteration wanders (login 6, login 6, login 8 there; login 3 and login 6 here). Nothing distinguishes
one logout from another. Each one draws a fresh answer to "does this client's `yield_now` park or
return", ten times per run, and a run goes red when any one of the ten draws badly. Ten draws is
also why this test is where the family surfaced first: the other login tests log out once or not at
all.

**The three network checks failing together, which that section flags as one host-load event, is
confirmed here** and is worth reading as a load gauge rather than as noise. `inbound`, `multicast`
and `smb` are host-side checks that talk to the guest over forwarded ports, and they went red in
exactly the loaded runs below (including runs whose kernel suite was entirely green), so a
transcript with all three red is a transcript to read as "this host was saturated".

### The fix, and why its bound is the second-best unit

`fixtures/src/login_test_client.rs`'s `destroy_with_retry` now waits on the property (the region
genuinely coming down) and bounds itself with a clock rather than with a count. The ceiling is
`DESTROY_WAIT_SECS`, five seconds of counter time, and it is a watchdog rather than a rate: only a
run that is going to fail ever pays it, because the loop returns the moment the region comes down.

The ceiling has an argument rather than a feel, and the argument is the tail, measured here:

| condition | waits | worst single wait |
|---|---|---|
| aarch64, no induced load (one-minute load average 7 to 15) | 30 | 12.8 ms |
| aarch64, six spinners (13 to 29) | 40 | 44.3 ms |
| riscv64, six spinners (27 to 47) | 30 | **126.4 ms** |

Five seconds is 40x the worst of those, and the cost of being wrong in the generous direction is
that a genuinely wedged caretaker takes ten seconds to report (this ceiling twice, the budget and
the region) inside a 90 s per-test harness ceiling, instead of reporting in eight milliseconds and
being wrong.

**The unit is wall clock, which is the unit the migration-drain section above argues against, and
that is a limitation rather than a disagreement.** Milestone 62 re-denominated `smp.rs`'s drain in
*delivered timer ticks* precisely because a counter deadline keeps running while the guest is
descheduled. A process cannot do that: `user_mode_runtime::now` is the raw counter, and nothing publishes the
kernel's per-core tick count to userspace. So what makes this safe is the margin and not the unit,
and the margin is what would have to be re-measured if the wait ever grew. Giving a process a
delivered-tick reading is an ABI addition, which is calef's call rather than a lane's; it is written
in that function's own BUGS section so the next reader meets it there and not only here.

**The failure message now carries the observation the wait decided on.** The client reports its wait
in the third report word (where every other behaviour puts an identity hint, and `LOGOUT` has no
identity to report), and the assertion prints it beside the ceiling: a wait near the ceiling means
the caretaker never died, which is a kernel bug, and one far under it means the client stopped
waiting early, which is the defect this ceiling replaced. That is the fifth round's panic-message
lesson applied before anybody had to read a red run at 2 a.m.

### The injection, because a green run proves nothing about a branch it never took

The fourth round's caveat is the one this page keeps having to repeat: twelve green runs said
nothing about a branch that was never exercised. So the giving-up path was proven directly, by
pointing the wait at a capability whose `DESTROY` can never succeed (the caretaker's *endpoint*
rather than its region, a one-word edit, reverted):

```
assertion `left == right` failed: login 0's logout ticket did not destroy the caretaker's
construction region; the client waited 5000100 us for the refusal to clear, against its own ceiling
of 5000000 us. A number near that ceiling means the caretaker never died, which is a kernel bug and
not this host being slow; a number far under it means the client stopped waiting early, which is
the defect that ceiling replaced.
```

The ceiling fires, it fires at the value the constant names, and the message is the one a reader
needs. (An earlier run of the same injection, at the 2 s ceiling this lane started from, printed
2000037 us against 2000000; the ceiling was raised afterwards on the riscv64 tail above.)

### The runs, all of them

The second round's rule: reporting the interesting ones would be the dishonesty this milestone
exists to remove. Six spinners rather than the recipe's eight, because other lanes were gating on
this laptop and the fourth round's choice not to fail somebody else's run still applies; the machine
supplied the rest of the contention itself.

| tree | leg | runs | reached the test | logouts | red at this assertion | worst wait |
|---|---|---|---|---|---|---|
| old form (64 attempts) | aarch64 | 5 | 4 | 31 | **2** | 94.4 ms, and truncated by the give-up |
| new form (5 s clock) | aarch64 | 8 | 8 | 80 | 0 | 44.3 ms |
| new form (5 s clock) | riscv64 | 3 | 3 | 30 | 0 | 126.4 ms |

Five of those eleven new-form runs carried a 2 s ceiling rather than the 5 s this lane shipped,
which is recorded because it is the sort of detail a later reader would otherwise have to trust: the
raise happened after the riscv64 tail was measured, and it changes nothing about a run that passes,
since the loop returns when the region comes down and never reaches its bound. The tree as it stands
was then run once through all three architectures at one-minute load averages of 13 to 25 (aarch64
300 passed, riscv64 303, x86_64 189 with this test skipped for the reason
`fs_service::NO_FS_SERVER` gives, which is the vendored RedoxFS `aes` dependency rather than
anything here).

The two old-form reds were at one-minute load averages of about 10 and about 37, which is the range
this laptop sits in whenever two lanes are gating: 2x to 5x oversubscribed on eight cores, not a
pathological host. Of the runs that did not reach the test, one died at `live_swap_tests.rs:263` and
one at `holding_a_lock_masks_the_timer`; both are below.

**What eleven green runs do not prove** is that the rate is zero. By the rule of three, 0 failures
in 110 waits bounds it near 3%, and this is one host, one QEMU build, one load shape, which is the
same caveat the 2026-08-22 confirmation run attaches to its own 45 runs.

### Three things found on the way, none of them fixed here

**`crates/system_initializer::reclaim` is the same loop, in a shipping program.** Sixty-four
attempts over the same §16 refusal, and its own comment makes the claim this lane's measurement
refutes: "one preemption is enough" is true, and sixty-four yields is not a way to buy one. It was
left alone deliberately, because the two failures are different sizes (a test client that gives up
early fails a run; that one gives up quietly and strands `DIR_JOB_REGION_PAGES` until the machine
stops) and because changing the boot path was not this lane's brief. The measurement is recorded in
a BUGS section at `RECLAIM_ATTEMPTS`, where the next reader of that constant meets it.

**And it is not alone: there are five of these, and two of them trap.** Every one waits on the same
refusal, and none has a clock in it:

| site | on exhaustion |
|---|---|
| `crates/system_initializer::reclaim` (`RECLAIM_ATTEMPTS`) | returns; a stranded region |
| `components/src/login.rs`'s `reclaim` (`RECLAIM_ATTEMPTS`) | returns; a stranded region |
| `components/src/swish.rs`'s `await_screen` (`SCREEN_REAP_ATTEMPTS`) | returns; leaks one job's pool |
| `components/src/job_undertaker.rs`'s `collect` (`MAX_ATTEMPTS`) | **`user_mode_runtime::trap()`** |
| `components/src/timetable.rs`'s `collect` (`REAP_ATTEMPTS`) | **`user_mode_runtime::trap()`** |

That is the reading order's fourth grep, and it is the first one that leaves this repository's
kernel: **a bounded retry count in a user program, over a syscall that a timer tick has to clear.**
The scope note's 39 sites were all kernel (`wait_for`, `free_frames`, `thread_count`, `used()`), so
userspace was never swept at all. Proposed as a milestone of its own in this lane's report rather
than taken here.

**Two more kernel sites went red under this lane's load and are nobody's yet.** Neither is in the
scope note. `user/live_swap_tests.rs:263`, "reclaiming the operator's budget returned 277 of 224
pages", is a **negative-direction** failure against a global count, which is this page's first
diagnostic pointing straight at a wait written against something wider than the property.
`arch/aarch64/timer.rs:680`, "the timer is not ticking at all", killed a run before it reached the
suite's userspace half; it is a sibling of the assertions milestone 62 dispositioned and it was not
one of them.
