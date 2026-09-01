# The workload that does not stop, and what a clean run of it is worth

*(Milestone 219. `kernel/src/soak.rs`, `user/src/soaker.rs`, `crates/soak_page`, `script/soak`, and
the `Stage::Soak` half of `crates/board_console`.)*

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
  syscall boundary. Groups of one responder, three callers and one pure-compute grinder, one group
  per online core.
- **The detection is in the kernel** (`kernel/src/soak.rs`), because a user program cannot assert
  about kernel internals and a workload that could reach its own tripwire is not a tripwire.
- **The two share one page** (`crates/soak_page`), one `u64` per worker with exactly one writer, so
  the supervisor reads progress without asking for it.

A round trip is `CALL` -> `RECV_CAP` -> `REPLY` -> the caller waking: two block/wake handshakes, the
protocol `crates/thread_wake_handshake` models and the one the risk's only real defect was in
(`sched::wake_load_aware` making a receiver `Ready` without a delivery).

Each worker spins a small pseudo-random number of iterations between round trips. That is not
decoration: a soak that repeats one interleaving for eight hours has explored one interleaving, and
the jitter keeps the pairs' phase drifting instead of locking.

## The number, and the only three things it is for

Every five seconds the supervisor prints one line:

```
soak: t=25s beat=5 rounds=595432 rate=24160/s workers=20 refused=0 mismatch=0 stalled=0 crossings=21 remote=20 steals=4 deferred=0
```

`rounds` is **the** figure: cumulative IPC round trips completed by every worker. It exists so that
a run can be compared, and it has three honest uses:

1. **Between architectures**, so a rate an order of magnitude off on one of them is a question.
2. **Between QEMU and silicon**, which is the comparison risk 5 is actually about.
3. **Against the same machine later**, where a large drop is an IPC-path regression no functional
   test would fail on.

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

## The finding: a saturated workload does not migrate under this scheduler

This is the part worth reading, and it is the reason the milestone was worth running rather than
merely worth building.

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

Closing the second half needs a decision that is not a lane's: thread affinity (so a responder can
be forced to answer callers on other cores), a periodic rebalancer, or a device-interrupt source the
workload can drive at rate. Each is a scheduler-policy or syscall-surface change. See the milestone
block's handoff.

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

5. **Watch for `soak: started`.** If it does not appear, the kernel was built without the feature or
   the archive has no `soaker` entry; the tour's last line will be there either way.

6. **Leave it.** The watcher stops at the deadline, or the moment the board announces a failure, or
   after three missed beats. The log is the artifact; the last `soak:` line in it is the number.

7. **Record the number in this note's table**, beside the QEMU rows, with the date and the duration.
   That is what makes the next run comparable, and it is the only thing that makes an eight-hour
   vigil worth having sat through.

To confirm a build soaks at all without waiting: `script/board-console --for 3m --until soak`
returns as soon as the workload announces itself.

The same procedure works on **argon** and **xenon**, with their own architectures'
`script/board-image` equivalents. Neither has been run at a bench yet.

## BUGS

- **A soak that finds nothing is weak evidence, and this is the sentence to repeat.** A clean eight
  hours licenses exactly one claim: *this machine did N cross-core IPC round trips without the wake
  gate refusing one, without a wrong reply, and without a worker stalling.* It licenses nothing about
  the interleavings that did not occur, and the ones that did not occur are where the remaining bugs
  are. `script/soak` prints this on every green run because a number quoted without it is a number
  quoted wrongly.
- **No duration is prescribed, because nobody knows what duration would be persuasive.** The risk's
  own text says this class "produces a confidence rather than a verdict". Eight hours is a night;
  it is not an argument.
- **The heartbeat is guest time and the watcher's deadline is host time.** Under heavy host load a
  QEMU guest's clock runs slower than the wall, so beats arrive later in host seconds than the
  kernel thinks it printed them. The three-beat margin absorbs the ordinary case; a machine running
  a mutation sweep beside a soak can produce a false `WentQuiet`. `--quiet-after` is the knob, and
  not running a soak beside other heavy work is the better answer (`AGENTS.md`'s memory ceiling).
- **`--arch x86_64` soaks one core** unless `--smp` says otherwise, because that runner defaults to
  one and its SMP bring-up has two open bugs (`arch::x86_64::ap_boot`'s BUGS #1 and #3). Its
  `crossings=0` says so out loud, and a single-core soak is not a multicore soak.
- **The supervisor yields in a loop rather than sleeping**, because this kernel has no
  sleep-until primitive a kernel thread can use. It is one more thread contending, which is not
  entirely a cost, and it is why these round-trip rates are not comparable with `script/bench`'s IPC
  numbers.
- **A worker that dies looks exactly like a worker that wedged** from the shared page. Both fail the
  run; the thread dump the supervisor prints before panicking is what separates them.
- **Nothing runs a soak in `script/test`.** A twenty-second leg per architecture would gate the
  build against bitrot, and it is not there: the soak is exercised by `script/soak` and by
  `board_console`'s host tests over a real capture. If the feature stops compiling, nothing will say
  so until someone runs the script.
