# The workload that does not stop, and what a clean run of it is worth

*(Milestones 219 and 221. `kernel/src/soak.rs`, `user/src/soaker.rs`, `crates/soak_page`,
`script/soak`, and the `Stage::Soak` half of `crates/board_console`.)*

`design/fatal-risks.md`'s fifth entry, *it cannot be made reliable on multicore, and the bugs appear
only on silicon*, names its decisive experiment as **sustained multi-core stress on the boards with
the load-sensitive assertions live**. Until this milestone the tree could not sustain anything: the
boot tour ran its checks, printed its last line, and called `arch::halt()`. Captured on radon on
2026-09-01, that is the last thing the board says before it sits in `wfi` indefinitely.

This note is what the workload is, what number it produces, and, more usefully, **what it was
measured to be unable to do**.

## The shape

One kernel feature (`--features soak`) replaces the halt at the end of the boot tour with a pool of
user-mode workers and a supervisor that watches them forever.

- **The workload is a user program** (`user/src/soaker.rs`), so the pressure goes through the real
  syscall boundary. Groups of one responder, three callers, one pure-compute grinder and one tick
  waiter, one group per online core.
- **The detection is in the kernel** (`kernel/src/soak.rs`), because a user program cannot assert
  about kernel internals and a workload that could reach its own tripwire is not a tripwire.
- **The two share one page** (`crates/soak_page`), three `u64` per worker with exactly one writer
  each, so the supervisor reads progress without asking for it.
- **The tick waiter is milestone 221's** and has its own section below. It is the one worker that
  completes no IPC: it blocks on a rendezvous the kernel signals from `sched::on_tick`, which is
  what makes anything on this machine cross cores at all.

A round trip is `CALL` -> `RECV_CAP` -> `REPLY` -> the caller waking: two block/wake handshakes, the
protocol `crates/thread_wake_handshake` models and the one the risk's only real defect was in
(`sched::wake_load_aware` making a receiver `Ready` without a delivery).

Each worker spins a small pseudo-random number of iterations between round trips. That is not
decoration: a soak that repeats one interleaving for eight hours has explored one interleaving, and
the jitter keeps the pairs' phase drifting instead of locking.

## The number, and the only three things it is for

Every five seconds the supervisor prints one line:

```
soak: t=25s beat=5 rounds=1151772 rate=43031/s wakes=10032 wakerate=401/s workers=24 refused=0 mismatch=0 stalled=0 crossings=2252 remote=3584 steals=3 deferred=99
```

`rounds` is **the** figure: cumulative IPC round trips completed by every worker. It exists so that
a run can be compared, and it has three honest uses:

1. **Between architectures**, so a rate an order of magnitude off on one of them is a question.
2. **Between QEMU and silicon**, which is the comparison risk 5 is actually about.
3. **Against the same machine later**, where a large drop is an IPC-path regression no functional
   test would fail on.

`wakes` is **not** part of `rounds` and never will be: a tick-route wake is not a round trip, and
folding the two together would make the one comparable figure mean something different depending on
which build produced it. Its own rate is pinned to the machine (`TICK_HZ` times the online cores, so
about 400 a second on a four-core QEMU), which makes it a useful liveness check in its own right: a
`wakerate` well under that is the timer or the wake path falling behind, not the workload.

`refused`, `mismatch` and `stalled` must all be zero, and any of them nonzero fails the run: the
supervisor prints `soak: FAILED`, dumps the threads (so the per-core event rings are on the log) and
panics.

### First measurements, 2026-09-01, patagonia, QEMU

Taken with `script/soak --for 30s`, four groups per machine except x86, whose runner defaults to one
core. **These are QEMU numbers on a loaded laptop and are a baseline for comparison, not a
benchmark**; `script/bench` is the instrument for cost.

| Architecture | Cores | Workers | Round trips/s | Cross-core handoffs in 25s |
|---|---|---|---|---|
| aarch64 | 4 | 20 | ~58,000 | 17 |
| riscv64 | 4 | 20 | ~24,000 | 21 |
| x86_64 | 1 | 10 | ~3,900 | 0 |

### With the tick route, 2026-09-02, patagonia, QEMU (milestone 221)

Same command, same host, one day later, and **these rows are not comparable with the rows above**
for a reason larger than the date: the build changed. Each pair below was measured back to back, the
"before" leg from the exact commit this work branched from, on an otherwise idle machine, and read
at the 25-second beat (20 seconds on x86, whose beat count is lower).

| Architecture | Cores | Round trips before | after | Crossings before | after |
|---|---|---|---|---|---|
| aarch64 | 4 | 1,623,764 and 1,630,605 | 1,632,746 and 1,632,803 | 15, frozen from beat 1 | 1,452 and 3,779, both rising linearly |
| riscv64 | 4 | 871,047 and 886,428 | 662,787 and 823,783 | 10 and 14, frozen | 2,573 and 4,358, rising |
| x86_64 | 1 | 77,372 | 51,749 | 0 | 0, and one core is the whole reason |

**Two runs of each leg on the multicore architectures, because one would have been misleading**, and
the first pass of these measurements *was* misleading: it was taken while another lane's test suite
was running on the same laptop, and the numbers it produced (aarch64 47,864 against 43,031 a second)
were the host's load rather than this change. Everything above is from an idle machine.

**What the numbers support:**

- **aarch64 pays nothing measurable.** 0.6% more round trips after than before, in the direction of
  faster, which is noise. The tick waiters complete no round trips and the set of workers that does
  is identical in both legs, so the totals are directly comparable.
- **riscv64 pays about 7%** on the closest-matched pair (886,428 against 823,783) and more on the
  looser one. It is the architecture where a migration costs the most under TCG, and it is the one
  crossing most often, so a cost showing up here and not on aarch64 is consistent rather than
  puzzling.
- **x86_64 pays about a third, and that is arithmetic rather than a finding.** Its runner is
  single-core, so the two extra waiter threads are two more shares of the one core in a round-robin
  scheduler, and `crossings=0` is what one core means.

**The round-trip rate fell far less than DECISIONS 138's spike saw**, which reported about 30% on
aarch64 and about 55% on riscv64. That difference is recorded rather than explained away: the spike
was thrown away and cannot be re-measured, so why it was slower is not recoverable, and the worker
mix is the obvious candidate and is a guess.

**`wakes` and `crossings` are the two figures milestone 221 added, and neither is a throughput
number.** The tick route wakes at `TICK_HZ` times the core count, which is a property of the machine
rather than of the workload, and the crossings are however many of those wakes `wake_load_aware`
chose to place on another core. Somewhere between a seventh and a half of them, varying by run more
than by architecture. That ratio is a fact about the placement policy under this load and nothing in
this tree yet says what it should be.

**Which build these came from, and it is not the one that ships.** Every figure above is from a
`--features soak` kernel, which is the only build in which the counters and `Thread::last_cpu`
exist at all. That is not free, and the size of it is measured rather than assumed:

| Architecture | `ipc_fastpath`, production | with `--features soak` | |
|---|---|---|---|
| aarch64 | 5,788 bytes | 6,120 | 1.06x |
| riscv64 | 5,106 bytes | 5,344 | 1.05x |
| x86_64 | 6,639 bytes | 6,995 | 1.05x |

So **a soak build is not a production build**: its IPC path is five to six per cent larger, and its
round-trip rates are therefore soak-build rates. Compare a soak number with another soak number,
which is what the three comparisons above are; never with `script/bench`, and never as a statement
about how fast this kernel does IPC.

**Milestone 221 added nothing to that table, and it was checked rather than assumed.** Its kernel
change is a `#[cfg(feature = "soak")]` call in `sched::on_tick` and a module that is not compiled
otherwise, so a production build should be untouched; "should be" is what this tree does not accept.
Built at the base commit and at the merge candidate, on all three architectures, without the
feature: every symbol has the same size, every section has the same size except `.strtab`, which is
not loaded, and `ipc_fastpath` and `syscall_entry` are unchanged at 6,687 and 1,637 bytes.

The loadable image (`llvm-objcopy -O binary`) differs by 45 bytes on aarch64, and all of them are
`core::panic::Location` line numbers below the insertion point, each larger by exactly the ten lines
added to that file. The proof is that rebuilding the base commit with **ten comment lines** at the
same point gives an image that is byte-for-byte identical to the merge candidate's, on all three
architectures. Any comment added to `sched.rs` would move those bytes, and a hash comparison that
called that a change would be measuring the file's line count.

That the instrumentation is behind a feature at all is a thing this milestone got wrong first and
was caught by a gate. Shipping the counters and the `last_cpu` write unconditionally put
`ipc_fastpath` **5.7% over milestone 132's 5% bound on aarch64** (5,788 -> 6,120), with riscv64 and
x86_64 growing 4.7% and 4.6% behind it: one cause, three effects, and aarch64 merely the one that
tipped. The `last_cpu` write sits in `schedule()`'s switch, the hottest line of the hottest
function. `script/lint` now clippies `--features soak` on both ISAs, because a `cfg`-gated
instrument that nothing lints is one that rots (and the first run of that check found two real
warnings in `kernel/src/soak.rs`, which had never been linted).

## The finding: a saturated workload does not migrate under this scheduler

This is the part worth reading, and it is the reason the milestone was worth running rather than
merely worth building. **It is still true**, and milestone 221 did not repeal it: what that
milestone added is a thread that is *not* part of the saturated workload, precisely because nothing
inside the workload can be made to move.

**The cross-core handoff count freezes within the first second and never moves again.** Measured
across three topologies (one caller per responder, three callers per responder, twice as many groups
as cores), on both multicore architectures, at up to 65,000 round trips a second. The workload runs
on every core and contends on every shared scheduler structure; the threads themselves stay exactly
where `pick_spawn_target` put them.

The mechanism, and every clause of it is in the tree already:

- **A rendezvous wake is local on purpose.** `sched::wake` pushes the woken peer onto the *waker's*
  own run queue (DECISIONS §28.2: the message is in registers and the cache is warm). So a
  communicating set converges onto one core within a few exchanges and stays there.
- **`wake_load_aware`, the load-aware placement, is for device interrupts only.** It is the function
  the one real defect was in, and **no user workload can reach it**: it takes an IRQ to get there.
- **A work steal needs an idle core and a queued thread elsewhere.** A rendezvous keeps at most two
  threads runnable per group, so run queues are almost always empty and there is nothing to give;
  add compute threads to fill the queues and no core is idle to ask. Both ends of the condition are
  hard to hold at once, and a steady-state workload holds neither.
- **Nothing rebalances periodically.** There is no such thing in this scheduler.

### The instrument that found it was the second one

The first version counted `trace::Event::PlaceRemote` and reported 23, frozen, which was read as
"the threads are not moving". That was true, but the counter could not have shown it: a rendezvous
wake queues its peer **locally**, so the placement is local *even when the thread has moved between
cores*, and a placement counter is structurally blind to the migration this workload performs.

`thread::Thread::last_cpu` and `trace::Event::Migrated` answer the question where it cannot be
dodged, at `schedule()`'s `switch_in`, which is the one place every path to a CPU passes through
whatever moved the thread. The finding survived the better instrument, which is the only reason it
is written here as a finding rather than as a guess.

**Take the lesson, not just the number**: a counter that is *near* the question is not the same as
one that answers it, and the two agree right up until they matter.

### What this means for risk 5

The decisive experiment as the risk states it, "sustained multi-core stress", is **not one
experiment**. It is at least two, and this milestone delivers the first:

- **Concurrent contention on shared kernel state.** Four harts entering `IPC_TABLES` tens of
  thousands of times a second, preempting each other, writing their own trace rings, retiring
  rendezvous. This is real weak-memory pressure and it is what the soak sustains.
- **Cross-core handoff.** Threads actually moving between cores under load, which is where the
  observed defect lived. **The soak does not sustain this, and cannot**, for the reasons above.

Saying so is the point. A run that quietly covered one and was quoted as covering both would be
exactly the misuse `design/roadmap/219-a-workload-that-does-not-stop.md`'s BUGS section warns about,
and `script/soak` prints the gap on every run so that nobody has to have read this note to know.

**The second half is now runnable, which is a different claim from "has been run".** See the next
section.

## Where the threads are, and why a rate moves without the machine changing (milestone 240)

Two soak runs on **radon**, same card, same build, twenty minutes apart, differed **eightfold** in
round-trip rate: 183,662/s against 22,592/s. The machine was proven identical by the boot tour's own
pure-compute check, which ran 6.9M and 7.3M iterations in the first against 6.8M and 7.3M in the
second, with 82 preemptions both times. So the difference was in the workload and not in the
silicon, and the soak printed six counters and **not one thread's location**, which left placement
as an inference rather than a reading.

The kernel knew the answer the whole time and threw it away: `sched::spawn` calls
`pick_spawn_target`, places the thread, and returns only a thread id.

### What it prints

Three things, all under a `soak-census:` prefix of their own. That prefix is not `soak:` on purpose:
`crates/board_console`'s recogniser matches two substrings on that one (`soak: started` and
`soak: t=`), and a census is neither, so giving it its own word means a block of census lines never
has to be proven harmless against a recogniser it has nothing to do with.

**One block at soak start**, from the placement `pick_spawn_target` actually made, one line per
online core:

```
soak-census: where the kernel placed each worker at spawn: R=responder, C=caller, G=grinder, W=tick waiter, and the number after each letter is its group
soak-census: core=0 threads=6 C0 C2 C2 G2 C3 W3
soak-census: core=1 threads=6 R0 R1 C1 C1 C2 G3
soak-census: core=2 threads=7 C0 C0 C1 G1 W2 R3 C3
soak-census: core=3 threads=5 G0 W0 W1 R2 C3
```

A token is a role letter and a group number, so `G0 G3` on one line is that core drawing two
grinders, read off a log by someone who never saw the board. **A core with no workers gets a line
too**, because four cores online and one of them empty is an explanation and a census that printed
only the occupied cores would hide it.

**One field on every beat**, `drifted=`: how many responders, callers and grinders are no longer on
the core the last printed census put them on. While it reads zero, that block describes the machine
right now, and the reader is told so rather than assuming it.

**A fresh block whenever `drifted` is nonzero**, and one more before the thread dump on a failure.
Printing a census every beat would double an already dense log; printing one only at the start would
leave a stale block standing, which is exactly what happens (below). Printing on the change carries
a current census whenever one exists and is quiet otherwise.

### The first thing it measured was that the start census goes stale in five seconds

**Nine to eleven of the twenty non-waiter threads are off their spawn core by the first beat**, on
every QEMU run of it, with `steals=` at three to five. So it is not work stealing, and this file
already said what it is, one section up: *a rendezvous wake is local on purpose, so a communicating
set converges onto one core within a few exchanges and stays there.* DECISIONS 138 says it in the
same words. The census measured what the tree had already written down and what the first draft of
this instrument's own comments had got backwards.

That settles the question milestone 240's block left open, which was whether the census should also
be reported after the start. **It must be**, and not because threads might move: because they
provably do, immediately, every time, and a start-only census would have misattributed every run
tonight. The spawn placement is a lottery *result*, not a resting place.

### Four QEMU runs, and what the census does and does not support

aarch64, `script/soak --for 40s`, same host, same build (the third differs only in which reference
`drifted` compares against, which cannot affect scheduling). The arrangement is the settled one
from the first re-census; the rate is the mean of beats 2 through 7, after convergence.

| settled arrangement | rate |
|---|---|
| three IPC groups on core 1, one core holding only waiters | **21,700/s** |
| two IPC groups on core 0, alongside two grinders | 38,600/s |
| two IPC groups on core 2 | 33,500/s |
| one IPC group per core | 32,300/s |

**The arrangement varies run to run under QEMU exactly as radon's rate did**, which is the first
thing worth knowing: the lottery is real on emulation too, and it is now visible.

**The widest spread coincides with the most crowded arrangement**, 1.8x between the run that put
three of the four IPC groups on one core and the best of the others. That points the same direction
as radon's eightfold and does not prove it.

**And the census partly refuses the inference the block was minted with.** That block named the
starvation shape as *a core drawing two grinders*, and the run that did exactly that was the
**fastest** of the four. What tracks the rate in this small sample is the number of IPC groups
sharing a core, not the number of grinders. Four runs on an emulator settle neither, and saying so
is the point: this is an instrument, and the result it makes possible is a series of boots on
silicon rather than an argument.

### What it cost

Nothing that ships. `sched::spawn_reporting_placement` and `sched::last_cpus` are both
`#[cfg(feature = "soak")]`, so a production build has neither, and `script/fastpath-footprint` reads
the same 6,687 bytes over eight symbols it read before. Within a soak build the census is one lock
acquisition and twenty-four comparisons every five seconds, against a workload doing tens of
thousands of round trips a second in the same window.

## The tick route: how the soak was made to cross cores (milestone 221)

`design/decisions/138-cross-core-handoff-under-load.md` (*how a saturated workload is made to hand
threads across cores*) put four options in front of calef and he approved option D on 2026-09-02.

**The mechanism, and it is short.** Under `--features soak` and nowhere else, `sched::on_tick`
signals a rendezvous, and one worker per group blocks on that rendezvous through the `Irq::WAIT` a
device driver already uses. `on_tick` is called by all three architectures' timer dispatchers in
real interrupt context on every core, so a tick runs the identical sequence a device interrupt runs:

```
soak::signal_waiters -> sched::irq_notify -> Rendezvous::signal -> handshake.serve
                     -> sched::wake_load_aware -> pick_wake_target -> place_on -> the reschedule IPI
```

Each group has its own route and each tick signals one of them, round-robin across the machine.
Both halves of that are fixes rather than flourishes, and the section below says what they fix.

That last chain is why this was worth building rather than the alternatives. `wake_load_aware` is
where risk 5's one observed defect lived, on radon, and it had **exactly one caller**
(`sched::irq_notify`) that no user workload could reach.

**Four properties, each of which was a requirement rather than a bonus:**

- **No syscall is added.** The userspace half already existed: `abi::irq::WAIT` is a method on an
  `Irq` capability and `user_rt::irq_wait` calls it. Only the *raise* was missing, and the kernel is
  already the thing that raises interrupts.
- **Nothing exists in a production build.** Proved above, not asserted.
- **It is architecture-neutral, and that is load-bearing.** riscv64 has no software-raisable line
  that reaches `irq_route` at all, so an aarch64 `send_sgi` or an x86 self-IPI would have left
  **radon** out, and radon is the machine that produced the defect. The timer is the one source all
  three share, through a function that is already portable.
- **The timer is the one event a saturated workload cannot starve**, which is the whole reason this
  works where three existing balancing moments do not.

**What crosses is the waiters, not the pairs, and this must not be misquoted.** Rendezvous wakes are
local by design whatever else is happening, so the callers and responders are as pinned as they ever
were. This sustains the **wake protocol** across cores under load; it does not make the IPC workload
migrate, and only a periodic rebalancer would, which DECISIONS 138 declines on
DECISIONS §28's own reopening trigger (*a real workload where fairness visibly fails*), which has
not fired. The kernel says this in words at the start of every run and `script/soak` says it again
in its summary, because the flattering reading is available and a summary gets quoted.

**The soak-only interrupt numbers.** Group `g`'s route is bound to intid `255 - g`, and none of
those names hardware or can be delivered on any of the three architectures: on aarch64 and riscv64 a
routed interrupt arrives only if something enabled it at the controller and nothing enables these,
and on x86_64 the top of the band is the local APIC's spurious vector (answered in its own arm
before `irq_route` is asked) with the rest at the far end of an MSI band allocated upward from 0xc0.
**None of that is what makes it safe**: `soak::bind_tick_routes` asks `sched::irq_route` about every
number before it takes any of them, and refuses to start a soak whose routes would steal somebody
else's interrupt. A soak boot runs the whole tour first, so every device has already claimed what it
is going to claim by the time that check runs.

### Two bugs this mechanism had, both found by running it, both about ordering

Worth writing down because neither was visible in review and both produced the same symptom, which
is a soak reporting workers as wedged when the defect was in the instrument.

**One rendezvous for every waiter starves all but one, on a loaded host.** The first version had a
single tick route and four waiters blocked on it. `crates/ipc`'s `Rendezvous::recv` takes a
**pending** signal before it looks at the receiver queue, which is right for a driver (an interrupt
that already happened must not be missed), and wrong for four peers sharing a source: when ticks
arrive in a burst, whichever waiter is already running drains the whole backlog through the pending
path and never queues, while the others sit at the head of a queue nothing pops. Three of four
stalled, and the run failed. The fix is a rendezvous per group, so a backlog can only ever belong to
the waiter it accumulated for. The shared version passed several idle-machine runs first, which is
the part worth remembering: the bug needed a busy host to appear at all.

**Binding the routes after spawning the waiters is a race, and the reasoning that put it there was
right about the wrong thing.** Arming last is correct for the *signalling*, because a route signalled
before anyone waits on it hands the first waiter a backlog and makes the first beat measure setup.
It is wrong for the *routing*: a waiter that reached `Irq::WAIT` before its route existed got
`WrongObject`, and a waiter has no channel to report a refusal on, so it stopped counting and the
run failed a beat later with four workers apparently wedged. aarch64 got away with it and riscv64 did
not, which is the ordinary shape of this class. The two halves are now separate: routes are bound
before the first waiter is spawned, and the signalling is switched on last.

### What it establishes about risk 5, and what it does not

- **It makes the second experiment runnable. It does not run it.** The run needs an evening at a
  bench on radon, argon or xenon. QEMU cannot show the defects this risk is about; that is the
  risk's premise, not a limitation of the tooling.
- **It says nothing about what a crossing rate should be.** The numbers above are a shape. There is
  no baseline to compare a board against until a board has produced one, and the first board run is
  what creates it.
- **The hook fires on a timer, which is why it works and why it proves nothing about the machine
  without it.** A soak with the tick route live is evidence about the wake path under sustained
  cross-core traffic. It is not evidence that a workload would ever generate that traffic on its
  own; measurement says it would not.
- **The interrupt controller is not on this path.** The timer is not a controller-routed source, so
  the claim, mask and complete sequence (the GIC, the PLIC, the local APIC) is untouched. The
  experiment is about the wake protocol, and that is what it runs.

## Why this extends `board_console` and not the other two instruments

`script/repeat-under-load` and `script/interleaving-check` are the tree's existing load and
concurrency instruments, and neither was the right place for this.

- **`script/repeat-under-load`** repeats a **terminating** suite N times with the host deliberately
  loaded, and reports what the load actually was. A soak has no runs to repeat and does not
  terminate, and the contention it wants is the guest's own rather than the host's. The two are
  complements: that one asks "does the suite still pass when the machine is busy", this one asks
  "does the machine stay correct when it is busy for hours".
- **`script/interleaving-check`** is loom over the extracted protocols, on the host, searching every
  interleaving the C11 model permits. It is the strongest evidence available about those protocols
  and it says so honestly: loom models C11, not ARM and not RISC-V. A soak on silicon is the
  evidence loom cannot give, not a substitute for it.
- **`crates/board_console`** was the right one, because the thing a soak needs that did not exist is
  a judgement about *silence*, and that crate already owned it.

## How a hang is told from a slow run

One rule, and both halves of the tree implement it rather than agreeing to:

**The heartbeat is on the wall clock, not on the work.** A machine doing one round trip a second
still prints on time, with a `rate` that says it is crawling. A machine doing none still prints, and
its `stalled` count fires. So silence means the thing that prints is itself wedged, which is the only
thing silence is allowed to mean.

`crates/board_console` is the other half. Its `Stage::Soak` is reached by the kernel's own
`soak: started` line, and reaching it **re-arms the quiet check that a completed boot tour
suppresses**: a halted kernel is supposed to be quiet and a soaking one is not. That is a one-word
change (`< Stage::Tour` became `!= Stage::Tour`) and it is the whole agreement. Beat interval five
seconds against a fifteen-second default quiet window: three missed beats before a run is called a
hang, exit status 2.

`script/soak` runs the QEMU side through **the same recogniser and the same policy**, so the local
rehearsal and the bench run are one experiment with different deadlines.

## Running it

### Under QEMU, which is the rehearsal

```
script/soak                                  # aarch64, one minute
script/soak --arch riscv64 --for 10m         # radon's architecture
script/soak --arch x86_64 --smp 1            # xenon's, single core (see BUGS)
```

Exit statuses are `script/board-console`'s: `0` beat for the whole watch, `1` announced a failure,
`2` went quiet, `3` QEMU exited early or the workload never started, `4` build or arguments.

### On radon at a bench, which is the experiment

This is the procedure, in order. It assumes the runbook in `notes/visionfive2.md` for the cabling
and the U-Boot commands, and changes only two things about it.

1. **Build the payload with the soak feature.**

   ```
   script/board-image --soak
   ```

   The flag exists rather than a hand-built kernel because that script builds the archive **before**
   the kernel, and that order is load-bearing: the archive regenerates the measurement manifest the
   kernel compiles in as its trust root, and building them the other way round is what produced
   `MEASURED BOOT REFUSED` at the bench on 2026-08-15. It prints the `dd` commands; it runs
   nothing destructive itself.

2. **Copy the image to the microSD card and put it back in the board**, exactly as the runbook says.
   The archive must be the one built beside this kernel or the measured-boot gate refuses it.

3. **Start the watcher before powering the board**, so the boot itself is captured:

   ```
   script/board-console --for 8h --until none --log target/radon-soak-$(date +%s).log
   ```

   `--until none` is what makes it a sustained watch rather than a boot check. Leave
   `--quiet-after` at its default unless the console is noisy.

4. **Power the board and type the four U-Boot commands** the runbook gives (milestone 218 is about
   removing this step).

5. **Watch for `soak: started`.** Its own line names the worker mix, and on a four-hart JH7110 it
   should read four groups and 24 user threads. If it does not appear at all, the kernel was built
   without the feature or the archive has no `soaker` entry; the tour's last line will be there
   either way.

6. **Check the first heartbeat before you walk away**, which takes five seconds and is the whole of
   milestone 221's bench procedure. Two fields decide whether the cross-core experiment is actually
   running:

   - **`wakerate` should be about `100 * harts`**, so roughly 400 on radon. `TICK_HZ` is 100 and
     every online hart signals the tick route on its own timer, so a rate well under that means the
     timer or the wake path is falling behind and the run is measuring something else.
   - **`crossings` must be *rising* between beats.** Frozen is the pre-221 state and means the tick
     route is not armed: a kernel built without `--features soak` cannot get this far, so the
     realistic cause is that the intid was already routed, and the kernel says so and refuses to
     start rather than soaking silently without it.

   If either is wrong, stop and fix it. Eight hours of a soak that is not crossing cores is eight
   hours of the experiment milestone 219 already ran.

7. **Leave it.** The watcher stops at the deadline, or the moment the board announces a failure, or
   after three missed beats. The log is the artifact; the last `soak:` line in it is the number.

8. **Record the numbers in this note's table**, beside the QEMU rows, with the date and the
   duration: `rounds`, `rate`, `wakes` and `crossings`, and all four rather than the first two,
   because a later run cannot be compared on a figure this one did not write down. That is the only
   thing that makes an eight-hour vigil worth having sat through.

**What a green run on radon would license, stated before it happens so that nobody writes it
afterwards.** One sentence: *this board did N cross-core IPC round trips and M cross-core thread
handoffs over H hours without the wake gate refusing a wake, without a wrong reply, and without a
worker stalling.* That is the first evidence this project will have had about the wake protocol on
real silicon under sustained cross-core traffic, and it is a confidence rather than a verdict, which
is what `design/fatal-risks.md` says about this whole class.

To confirm a build soaks at all without waiting: `script/board-console --for 3m --until soak`
returns as soon as the workload announces itself.

The same procedure works on **argon** and **xenon**, with their own architectures'
`script/board-image` equivalents. Neither has been run at a bench yet.

## How long to run it, and why nobody can tell you

This note and milestone 225 (run the soak on radon, argon and xenon, which is the only place its
answer means anything) both say no duration is prescribed because nobody knows what would be
persuasive. That was written as an admission. It went unchecked until 2026-09-03, when a lane went
looking for whoever does know, and the honest result is that **the admission was correct, and it is
the field's condition rather than this project's.** Nothing found prescribes a duration for a
concurrency soak, and the one place a duration *is* derived derives it from a thermal model that has
nothing to do with interleavings.

Everything below was fetched and read on 2026-09-03. Where a thing was not found, it is written as
not found rather than as absent.

### seL4 runs nothing sustained, and its multicore tests are measured in milliseconds

This is the kernel this project measures itself against, so it is the first place to look and the
most surprising answer.

`seL4/sel4test`'s test directory (`apps/sel4test-tests/src/tests`, read at `master`) contains no
stress, soak, load or endurance file. Its multicore coverage is `multicore.c`, and the shape of
every test in it is the same: start a helper, `sel4test_sleep(env, 10 * NS_IN_MS)`, check a counter
moved or did not. Ten milliseconds is the whole observation window, and the property under test is
functional (a suspended thread stops, a resumed one runs, an affinity change takes effect) rather
than statistical.

`seL4/ci-actions` (the repository holding seL4's GitHub Actions, directory listing read at `master`)
has 40-odd actions and none of them is a soak or a stress run. The hardware ones are `sel4test-hw`,
`sel4test-hw-run` and `sel4test-hw-matrix`, which run the terminating suite above on real boards, and
`sel4bench-hw`, which is a benchmark. `sel4test-hw`'s own `action.yml` describes itself as
*"Runs sel4test builds for all hardware test platforms."*

On why there is not more, the project's own words, from Gerwin Klein on the seL4 Discourse thread
*Testing infrastructure* (2021-02-09): the CI is *"pull request checks (style, compile, licenses,
etc)"* and *"continuous integration test (either on the master branch of a specific repo, or, more
commonly on repo collections/manifests)"*, and, plainly, **"hardware tests are harder, proposals
welcome"**.

The reading to take from this is not that seL4 is careless. It is that **a project with a functional
correctness proof does not buy much from a soak**, because the thing a soak samples is the thing the
proof already covers. nife has 145 Kani harnesses and no refinement proof, so the trade is not the
same one, and copying seL4's answer here would be copying a conclusion without its premise.

### stress-ng picks a round number and says so

`stress-ng(1)` is the closest thing Linux userland has to a standard soak tool. Its `-t, --timeout T`
option reads, verbatim (Debian testing manual page, fetched 2026-09-03):

> run each stress test for at least T seconds. One can also specify the units of time in seconds,
> minutes, hours, days or years with the suffix s, m, h, d or y. [...] A 0 timeout will run stress-ng
> forever with no timeout. The default timeout is 24 hours.

**Twenty-four hours, with no stated reason.** The manual page's only account of what the tool is for
is that *"stress-ng was originally intended to make a machine work hard and trip hardware issues such
as thermal overruns as well as operating system bugs that only occur when a system is being thrashed
hard"*, and its one strongly worded caveat is about throughput rather than duration: *"it has never
been intended to be used as a precise benchmark test suite, so do NOT use it in this manner."*
Nothing in it says how long is long enough, or what a clean run licenses.

### The Linux Test Project prescribes nothing either

LTP's documentation (`setup_tests`, read 2026-09-03) treats runtime as a resource to be capped, not a
target to be reached: tests that run for more than a second or two must declare a `runtime` and check
actively how much is left, and `LTP_RUNTIME_MUL` and `-I` scale it. **The knobs are all for making
runs shorter.** No recommended soak length was found.

### Hardware is the exception, and its number is derived

Semiconductor qualification is the one practice found where "run it for N hours" is a real
requirement rather than a habit, and it is worth reading closely because **the derivation is the part
that does not transfer.**

JEDEC Standard No. 47G, *Stress-Test-Driven Qualification of Integrated Circuits* (fetched
2026-09-03), Table 1, requires High Temperature Operating Life at Tj at or above 125 C, Vcc at or
above Vccmax, 3 lots of 77 units, **"1000 hrs / 0 Fail"**. That is a hard number with a hard accept
criterion. And note 5.5(a) says where it comes from:

> with apparent activation energy of 0.7 eV, 125 °C stress temperature and 55 °C use temperature, the
> acceleration factor (Arrhenius equation) is 78.6. This means 1000h stress duration is equivalent to
> 9 years of use.

The same note is careful that the number is not self-justifying: *"The duration listed here is
generally acceptable to qualify for the given Application Level. However, it does not necessarily
imply the demonstration of the lifetime requirement for a particular use condition."*

So the hardware world has what the software world does not: **a model that converts stress hours into
a claim about the field.** Arrhenius does that for a wearout mechanism at a raised temperature. There
is no analogous model that converts soak hours into interleavings explored, and the reason is the next
section.

Part of what a board soak tests genuinely is the board, and this row of the table is the one that
applies to that half: radon under sustained load is closer to an operating-life sample than to a
concurrency test. It is also the half nife is least equipped to judge, having one unit per
architecture where JEDEC wants 231.

### The academic angle exists, and its finding is that clock time is the wrong axis

There is a literature here, and it is not neutral about stress testing.

Burckhardt, Kothari, Musuvathi and Nagarakatte, *A Randomized Scheduler with Probabilistic Guarantees
of Finding Bugs*, ASPLOS 2010 (the PCT paper, PDF fetched 2026-09-03), opens by describing exactly the
practice this milestone is about:

> Popular testing methods involve various forms of stress testing where the program is run for days or
> even weeks under heavy loads with the hope of hitting buggy schedules. This is a slow and expensive
> process. Moreover, any bugs found are hard to reproduce and debug.

Two of its results bear directly on choosing a duration.

**The state space is not the thing to cover, and bug depth is.** The paper defines *"the depth of a
concurrency bug as the minimum number of scheduling constraints that are sufficient to find it"*, and
proves that a run of a program with n threads and k steps finds a bug of depth d with probability at
least `1/(n k^(d-1))`. It observes that a naive bound over schedules is useless (*"This program, to
the first-order of approximation, has n^k possible thread schedules"*) and rests on the claim that
real bugs are shallow: *"Concurrency bugs typically involve unexpected interactions among few
instructions executed by a small number of threads."* Their examples put ordering errors at depth 1
and atomicity violations and lock-cycle deadlocks at depth 2.

That claim is independently measured. Lu, Park, Seo and Zhou, *Learning from Mistakes: A Comprehensive
Study on Real World Concurrency Bug Characteristics*, ASPLOS 2008 (PDF fetched 2026-09-03), examined
105 real concurrency bugs in MySQL, Apache, Mozilla and OpenOffice, and reports as finding 3 that
**"Almost all (96%) of the examined concurrency bugs are guaranteed to manifest if certain partial
order between 2 threads is enforced"**, and as finding 8 that **"Almost all (92%) of the examined
concurrency bugs are guaranteed to manifest if certain partial order among no more than 4 memory
accesses is enforced."** Their own caveat is attached and should be carried: the findings *"are
associated with the four examined applications and the programming languages these applications use"*.

**And stress-test coverage saturates, measurably.** This is the single most useful thing found, because
it is a measurement of the exact question this note asks. PCT section 5.3.3 instrumented a work
stealing queue with twenty events, 168 possible event pairs, and compared coverage against run count:

> We restrict the horizontal axis to the 8192 runs as stress did not explore any new event pair beyond
> those already explored in the new runs after that and PCT eventually explored all the event pairs.
> [...] Fig. 11 shows that stress does not cover more than 20% of the event pairs, few of which result
> in a bug. Thus, stress's inability/ineffectiveness to detect the bug is highly correlated with the
> event pairs not covered.

Their stress infrastructure was not a strawman by their account: it inserted *"random sleeps, thread
suspensions, and thread priority changes"*, which is a superset of what this soak's jitter does. It
still stopped finding anything new, and then ran forever without improving.

**That is this note's own line, measured by somebody else.** *A soak that repeats one interleaving for
eight hours has explored one interleaving* was written here as an intuition. PCT put a number on the
shape of it: coverage climbs, flattens, and the flat part is free.

### So the field has a habit, not a standard, and here is what to do instead

Stated plainly, because it is a real finding and dressing it up would be worse than useless:
**24 hours, 48 hours and overnight are round numbers.** The one prescribed duration found anywhere
(1000 hours) is prescribed for a thermal wearout model, states its own derivation, and warns that the
number does not by itself demonstrate the requirement. For concurrency, nothing found in tooling
documentation, in seL4's practice, or in the literature converts clock time into a claim.

**The alternative is to reason from this workload's own counters, which is available and is not
available to most people asking this question.** `script/soak` already prints, every five seconds,
`rounds`, `rate`, `wakes`, `wakerate`, `crossings`, `remote`, `steals` and `deferred`. Three questions
those support, none of which is "how many hours":

1. **What is the run buying per hour, in the units that matter?** Not round trips, which saturate the
   machine by construction, but `crossings`, since the recorded defect was on the cross-core wake path
   and the crossing rate is one to two orders of magnitude below the round-trip rate. A radon run's
   crossing rate is the honest denominator: at the QEMU aarch64 figures (about 3,779 crossings in 25
   seconds on the better of two runs) an hour is a few hundred thousand crossings, and a second hour is
   another few hundred thousand of the same kind. Decide the duration against a target crossing count,
   arrived at deliberately, and then say what it was.
2. **Is the run still producing new behaviour, or is it flat?** This is PCT's saturation question and
   this tree cannot currently answer it, because nothing here counts distinct behaviour. `remote`,
   `steals` and `deferred` are the closest available and are volumes rather than varieties. **This is
   the gap worth closing before the duration argument is worth having**, and it is a milestone rather
   than a note: something like a coarse histogram over the placement decisions, so a beat can be
   compared with the beat before it and a flat run can be recognised as flat.
3. **Would the time be better spent on more starts than on longer running?** The crossing count varies
   by more than a factor of two between identical runs, which is recorded in this note's BUGS and is
   evidence that the initial conditions matter more than the tail. Under PCT's model, independent runs
   multiply the probability of finding a shallow bug and a single long run does not; ten one-hour boots
   are ten samples of the boot-time placement lottery, and one ten-hour boot is one. Nothing here
   proves that trade for this workload, and it is the question a duration decision should be made
   against rather than around.

`script/interleaving-check` is the complementary instrument and this section sharpens why: loom over
the extracted protocols searches the state space directly, which is the thing a soak samples badly and
saturates at. The two are not competitors and the soak is not the weaker one; the soak is the only one
that runs on the silicon where the defect appeared.

**What none of this decides is the number**, deliberately. It says the number is calef's and gives him
the axis to pick it on: a crossing target on real silicon, chosen and written down, rather than an
hour count inherited from a tool's default.

## BUGS

- **A soak that finds nothing is weak evidence, and this is the sentence to repeat.** A clean eight
  hours licenses exactly one claim: *this machine did N cross-core IPC round trips without the wake
  gate refusing one, without a wrong reply, and without a worker stalling.* It licenses nothing about
  the interleavings that did not occur, and the ones that did not occur are where the remaining bugs
  are. `script/soak` prints this on every green run because a number quoted without it is a number
  quoted wrongly.
- **No duration is prescribed, because nobody knows what duration would be persuasive.** The risk's
  own text says this class "produces a confidence rather than a verdict". Eight hours is a night;
  it is not an argument. Checked against the field on 2026-09-03 and the admission stands: see *How
  long to run it, and why nobody can tell you* above, which is why it is a section rather than a
  longer version of this line.
- **Nothing here counts distinct behaviour, only volumes of it**, so a soak cannot say whether it is
  still finding new interleavings or has gone flat. That is the measurement the duration question
  actually wants and this tree does not have it; the section above names it as the thing to build
  before arguing about hours.
- **The heartbeat is guest time and the watcher's deadline is host time.** Under heavy host load a
  QEMU guest's clock runs slower than the wall, so beats arrive later in host seconds than the
  kernel thinks it printed them. The three-beat margin absorbs the ordinary case; a machine running
  a mutation sweep beside a soak can produce a false `WentQuiet`. `--quiet-after` is the knob, and
  not running a soak beside other heavy work is the better answer (`AGENTS.md`'s memory ceiling).
- **`--arch x86_64` soaks one core** unless `--smp` says otherwise, because that runner defaults to
  one and its SMP bring-up has two open bugs (`arch::x86_64::ap_boot`'s BUGS #1 and #3). Its
  `crossings=0` says so out loud, and a single-core soak is not a multicore soak.
- **A soak build is not the binary that ships**, so its timing is not the shipping binary's timing.
  The numbers above quantify it. This is normal and accepted, and it is stated here because the
  round-trip figures would otherwise read as IPC benchmarks, which they are not.
- **The supervisor yields in a loop rather than sleeping**, because this kernel has no
  sleep-until primitive a kernel thread can use. It is one more thread contending, which is not
  entirely a cost, and it is why these round-trip rates are not comparable with `script/bench`'s IPC
  numbers.
- **A worker that dies looks exactly like a worker that wedged** from the shared page. Both fail the
  run; the thread dump the supervisor prints before panicking is what separates them.
- **A tick waiter's wakes are not round trips**, and mixing the two figures is the misreading this
  workload is most likely to suffer. `rounds` counts IPC round trips and `wakes` counts tick-route
  wakes; they are separate fields because they are separate quantities.
- **The crossings are the waiters, never the pairs.** Repeated here because it is the claim a reader
  most wants this tool to be making and it is not making it.
- **`wakerate` is a property of the machine, not of the workload**, so it is not a throughput number
  and a run cannot be tuned to raise it. It is `TICK_HZ` times the online cores, and its use is as a
  liveness check on the timer and the wake path.
- **The crossing count varies by more than a factor of two between otherwise identical runs**
  (1,452 and 3,779 on the same aarch64 build, same command, same host). Whether a wake goes remote is
  `wake_load_aware`'s call and it depends on where everything happened to be; nothing here is wrong,
  and it means a single run's crossing count is not a figure to compare two builds on.
- **A waiter whose `Irq::WAIT` is refused spins instead of saying so.** It has no channel to report
  on, so it stops counting and the stall check speaks for it one beat later; the report then says
  "stalled" where "refused" would be more use.
- **The census is `last_cpu`, so it is where a thread last *ran*, not where it is queued.** A thread
  that has been placed on another core's inbox and not yet switched to still reads its old core, and
  a thread that has never run at all reads as unplaced. Both are honest answers to "where did this
  thread last execute" and neither is an answer to "where will it run next"; the census says
  `not-yet-run` for the second case rather than guessing.
- **`drifted=` excludes the tick waiters**, whose movement is milestone 221's whole point and is
  already `crossings=`. Folding them in would make the number rise on a healthy run and mean
  nothing. The cost is that a waiter which stopped moving does not show up here; the crossings rate
  going flat is what says that.
- **A machine that genuinely thrashes prints a census every beat**, which is four or five extra
  lines per beat and roughly doubles the log. Nothing rate-limits it beyond the one-per-beat check,
  on the argument that a run whose arrangement changes every five seconds is a run whose arrangement
  is the finding. No such run has been seen.
- **The census counts by group and role and says nothing about priority, quota or how long a thread
  has held its core.** Two arrangements that look identical here can still differ in ways this
  cannot show, so it narrows the space of explanations rather than closing it.
- **Nothing runs a soak in `script/test`.** A twenty-second leg per architecture would gate the
  build against bitrot, and it is not there: the soak is exercised by `script/soak` and by
  `board_console`'s host tests over a real capture. If the feature stops compiling, nothing will say
  so until someone runs the script.
