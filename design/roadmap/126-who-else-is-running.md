# 126. The `procps` package: who else is running, and who is allowed to ask

**Status: PARTIAL.** Minted 2026-08-14 by calef, from a design conversation about what ambient
authority utilities become on this system. **Scoped to the whole package by calef the same day**, for
consistency with milestone 123's approach to popular packages: the corpus is chosen by an external
ordering and taken in the units that ordering uses, which is packages rather than programs we like.

**Gate: NONE.** As of 2026-08-23, both forks this gate pointed at are decided: `pmap`'s
`ENUMERATE`-on-address-space extension is **yes** (DECISIONS §114), and `sysctl` is **declined**
(DECISIONS §115), each subsystem's own service carrying its own tuning instead. What remains is
real unbuilt work rather than anything waiting on calef, except where this lane (2026-08-26)
found the opposite is true: `top` on per-thread CPU accounting that does not exist turns out to
need a real decision about what "CPU accounting" means here and how it crosses the wire (**Fork:
`top`'s per-thread CPU accounting**, below), and `pwdx` and `w` on a process display name this
system has no design for turn out to have a clean answer on the authority half and none yet on
the mechanism half (**Fork: a process display name**, below). `uptime` is **built, 2026-08-26**
(see below) and needed neither a design decision nor calef's time.

## Built: `uptime`, 2026-08-26, needing no capability at all

`uptime` works on both ISAs and needed no new capability, no manifest field beyond what `worker`
already declares, and no kernel change. It reads `user_rt::monotonic_nanos`, the same ambient
counter `date` already reads to compute the wall clock, and formats the elapsed time as
`up [D day[s], ]HH:MM:SS\n`. The formatting is `crates/uptime`, host-tested (five tests: the zero
case, second/minute/hour rendering, the day rollover with its singular/plural, and the
sub-second-truncates-rather-than-rounds case); `user/src/uptime.rs` is the syscall and nothing
else, `wc`'s shape (a sink write, no input).

**Why this one member of "machine-wide statistics" turned out to be pure wiring.** The BUGS entry
that grouped `free`, `uptime` and `vmstat` together as needing "machine statistics rather than
process enumeration" was right about the first and third and wrong about the second, and the
difference is worth stating because it is not obvious from the grouping: `monotonic_nanos` is
granted to **every** EL0 process unconditionally, by `kernel/src/arch/*/timer.rs`'s `init`
(`CNTKCTL_EL1`'s `EL0VCTEN` bit on aarch64, the RISC-V and `x86_64` equivalents), which documents
the grant as **a deliberate, eyes-open exception to DECISIONS §10's no-ambient-authority rule**: a
monotonic counter grants no authority to *affect* anything, only to observe the passage of time,
and every OS that offers userspace self-timing accepts the same side channel. That exception
already existed and already covered every process before this lane started, so there was nothing
here to design: `uptime` is `worker`'s manifest with the arithmetic swapped out. `free` and
`vmstat` read physical-memory accounting the kernel keeps for itself
(`kernel/src/memory.rs::stats`) with no path to userspace today, which is a different body of work
and a real fork; see below.

**BUGS**, in full in `crates/uptime`'s and `user/src/uptime.rs`'s own module docs: no load average
(no decaying figure this scheduler maintains) and no logged-in-user count (no login registry
exists); the counter's own zero predates this kernel's init by an unmeasured amount, the same
caveat `date` already carries for the same counter; one-second resolution, for the same reason
`date` has it. **Provisional name**: `uptime` is upstream `procps`'s own name for this program,
which the naming tenet calls the best name available for a standard term a reader already knows,
flagged anyway because this program prints only elapsed time, none of upstream's load average or
user count.

## Built: `watch`, 2026-08-24, and the package file list is now verified rather than remembered

`watch` works on both ISAs: `ps`'s own domain walk (`abi::rendezvous::SURVEY`, `crates/ps`'s
`collect`), redrawn a bounded number of times instead of printed once, with `CSI 2J`/`CSI H` ahead
of each frame (`crates/watch`'s `REDRAW`) so the terminal shows the latest snapshot in place rather
than the whole run scrolling past. **It is not upstream `watch`'s "re-run an arbitrary command"**:
that needs a program to hold spawn authority, which in this system belongs to the shell alone
(`grant_plan::spawnproto`) and is granted to nothing the shell spawns -- an interruptible
(`^C`-stoppable) child is built with no capabilities in its cspace at all
(`crates/system_initializer`'s `spawn_service`), so there is no route today from "a program is
running" to "that program can start a second one". That is the same category of gap `top`, `pwdx`
and `w` are blocked on, and building spawn-delegation machinery to close it is a decision for
whoever needs it generally, not for this milestone's `watch`.

**So `watch` redraws the one thing it can already reach without any of that**: the supervision
domain it was spawned into, exactly `ps`'s own listing, and it is granted exactly `ps`'s three
capabilities plus a required argument (the redraw count). `watch ps` is also real `watch`'s single
most common invocation, so the narrowing is a real demonstration and not a consolation prize.
**Bounded rather than interruptible, and that is a scope decision stated plainly rather than an
oversight**: since an interruptible spawn gets no capabilities, `watch` cannot be both `^C`-stoppable
and hold the domain it needs for its whole run, so a bare `watch N` redraws `N` times (clamped to
`[1, watch::MAX_ITERATIONS]`) and exits on its own. The interval is fixed (`watch::INTERVAL_NANOS`,
half a second) and is a **yield-spin, not a sleep**: this kernel has neither, and `watch` is the
fifth named consumer of milestone 106's timed-wait fork (`user/src/timetable.rs`'s module docs name
the first four). Proven with a real terminal-output test, `kernel::user::watch_tests`, on both ISAs:
a domain member dies and is reaped through a capability `watch` itself is never granted (`READ`,
not `ENUMERATE` alone), and the test asserts the dead member's tid is gone from the **whole**
`video_terminal::Vt` grid after the second frame, not merely absent from wherever that frame wrote,
which is what would still be true of a `watch` that only overwrote instead of erasing.

**The real `dpkg -L procps` file list, verified rather than remembered.** Source: `podman run --rm
ubuntu:24.04`, `apt-get update` then `dpkg -L procps` and `dpkg -s procps`, run 2026-08-24 against
the live Ubuntu 24.04 archive (image reports `PRETTY_NAME="Ubuntu 24.04.4 LTS"`; package
`procps 2:4.0.4-4ubuntu3.2`, architecture `arm64`, matching this project's own `ubuntu-24.04-arm`
CI runners). The package installs eighteen names under `/usr/bin` and `/usr/sbin`, two of them
symlinks to a third program:

| name | what it is |
|---|---|
| `free`, `kill`, `pgrep`, `pmap`, `ps`, `pwdx`, `skill`, `slabtop`, `tload`, `top`, `uptime`, `vmstat`, `w`, `watch` | distinct binaries |
| `pidwait` | a **distinct binary**, not a symlink (same file size as `pgrep`, evidently the same source built in a second mode) |
| `pkill` | a symlink to `pgrep` |
| `snice` | a symlink to `skill` |
| `sysctl` | under `/usr/sbin`, not `/usr/bin` |

**The delta against this document's memory-sourced table (the one below, until this edit): `pidwait`
was missing.** Every other name the table already listed (`ps`, `top`, `pgrep`, `pmap`, `pwdx`, `w`,
`kill`, `pkill`, `skill`, `snice`, `free`, `uptime`, `vmstat`, `slabtop`, `tload`, `sysctl`, `watch`)
is confirmed present; nothing was listed that should not have been. `pidwait` waits for the matching
processes to terminate rather than printing them once, which puts it in the same **read the process
namespace** stratum as `pgrep` in spirit (no signal, no write), though its blocking-until-death
behaviour is closer to a wait/reap than a snapshot and would want its own design conversation before
anyone builds it; it is recorded here as the corrected membership fact and is **not** attempted by
this lane, which is scoped to `watch` and the verification itself. `dpkg -s procps`'s own long
description text ("It contains free, kill, pkill, pgrep, pmap, ps, pwdx, skill, slabtop, snice,
sysctl, tload, top, uptime, vmstat, w, and watch") omits `pidwait` too, which is presumably why the
earlier memory-sourced table missed it: the package's own prose is stale against its own file list.

## Built: `pmap`, 2026-08-23

`pmap` works on both ISAs, over `abi::aspace::LIST`: **a new method on the address-space object,
no new syscall number**, `Endpoint::SURVEY`'s shape one object type over (DECISIONS §114). Gated
by `Rights::ENUMERATE`, pointedly not `WRITE`, which is what `MAP_INTO` takes; a capability holding
`ENUMERATE` alone lists every mapping and is refused `MAP_INTO`, proved in both directions by
`kernel::user::pmap_tests`. The listing reads the space's own revocation log (`kernel/src/revoke.rs`,
already maintained for reclamation) rather than walking page tables for a member list, and turns
each `va` into a permission (`abi::aspace::MAP_RO`/`MAP_RW`/`MAP_CODE`, reused rather than a second
vocabulary invented) via `arch::mmu::translate_at`, present on both architectures already and
previously reachable only from revocation's own tests.

**The delegation audit §114 required, done rather than deferred.** Every site that mints an
`Object::Aspace` capability was checked (`user/src/builder.rs`, `crates/supervision_proto`,
`user/src/hello.rs`, `user/src/os_primitives_benchmarker.rs`, plus `kernel/src/bench.rs`'s
benchmark harness): **none delegates one to a program other than its own builder.** Every path is
retype -> map -> `Tcb::CONFIGURE` (which consumes the capability), all inside one thread. So there
was nothing to narrow: the audit's answer is that the caveat's feared case (a delegated holder
gaining ENUMERATE nobody assessed for it) does not exist in the shipped tree today, and the finding
is recorded rather than assumed.

**What building it actually found, and it is a larger gap than the fork's wording suggested.**
`Tcb::CONFIGURE` does not just consume the caller's `Object::Aspace` capability, it removes the
space's entry from the registry `abi::aspace::LIST` reads (`take_user_aspace`). So the instant a
space is bound to a thread and starts running, **every capability that ever named it, including
ones already sitting in a third party's cspace, reads as an empty listing rather than a live one.**
Combined with the delegation audit's finding that nothing delegates an `Object::Aspace` capability
at all, the practical result is that **there is no address space anywhere in this system today that
a second program can be handed a live view of.** `pmap`'s kernel mechanism and program are real and
proven end to end (`kernel::user::pmap_tests`, the same discipline `ps`'s `survey_tests` uses), but
unlike `ps` there is no `Manifest` field and no `system_initializer` wiring to reach it from the
shell, because there is nothing alive to wire: the shell's own address space was already consumed
by init's `CONFIGURE` call before the shell ever typed a command. This is a real, load-bearing
finding rather than an oversight in this lane's build; it is not this lane's decision to make, and
it is recorded here and in `crates/pmap`'s and notes/process-view.md's `BUGS` for whoever takes it
next. The shape a fix would need -- a builder handing a narrowed, still-registered view to a third
party *before* `CONFIGURE` consumes its own copy -- is a spawn-protocol change, not a `pmap` change.

**Still to build:** `top`, `pwdx` and `w`, each blocked on a fork proposed 2026-08-26 (see the
`Fork:` sections below) rather than on unbuilt-but-clear work; and `free`/`vmstat`, blocked on a
third fork of the same kind. `uptime` is **built, 2026-08-26** (see above): the one member of
"machine-wide statistics" that turned out to need no fork at all. `sysctl` is declined (DECISIONS
§115) rather than blocked on effort. `watch` is **built, 2026-08-24** (see above); the
package-membership confirmation is **done, 2026-08-24** (see above), and found one name the
memory-sourced table below had missed (`pidwait`).

## Built: the first stratum, 2026-08-16

`ps` works on both ISAs, and the view it reads is `abi::endpoint::SURVEY`: **a new method on the
supervision endpoint, no new syscall number**. Membership is `capability::survey_includes`, the same
relationship §32 authorizes a reap with, so the domain a monitor sees and the domain a supervisor may
collect from cannot diverge; three Kani harnesses hold that. Written up in notes/process-view.md,
with the semantics of the new method stated there for the integrator.

What the demonstration actually shows, and it is the negative control rather than the listing: a
viewer holding the endpoint **send-only** is refused (`NotPermitted`), a viewer holding nothing is
refused (`NoSuchSlot`), and a domain that is genuinely empty **answers**. Three distinct outcomes
where `/proc` has one, proved in one cross-ISA kernel test whose walk is `ps::collect`, the real
program's real loop.

`Manifest::domain` is the declaration that gets the grant to a `ps` at the prompt, `clock`'s twin,
and `caps ps` prints the scope before anything is spawned.

**The finding this turned up, and it is now fixed rather than recorded.** A view riding on `READ`
was wider than looking needs, because `READ` on a supervision endpoint is also what `RECV` and
`endpoint::REAP` take: a `ps` could have collected the children it listed. The lane left it on the
reasoning that the signalling stratum needed the same decision, and that reasoning did not survive
the ruling below. `capability::Rights::ENUMERATE` is the answer, the kernel-level twin of
`fs_proto`'s directory `ENUMERATE` and the same argument one layer down; `SURVEY` takes it, `RECV`
and `REAP` still take `READ`, and a viewer is granted `ENUMERATE` alone. notes/process-view.md
carries the whole argument.

**Only `Endpoint` consults the new right today.** The address-space object will when `pmap` is
built (observe a mapping without being able to map; **decided, DECISIONS §114**) and the
memory-region object (`Untyped`, renamed by DECISIONS §113) wants it when `free` is built (ask
what is committed without being able to `SPLIT` or `DESTROY`, still undecided). Both are named
here rather than pre-wired, because a right defined for hypothetical callers is the speculative
abstraction this tree declines; the consumer that can say what it needs is the one that should
extend it.

## Built: `pgrep`, 2026-08-17

**`pgrep` works on both ISAs, and it is granted exactly what `ps` is: one supervision endpoint with
`ENUMERATE`, and not one right more.** The two manifests in `grant_plan` are identical field for
field, which is the readable form of the ruling below rather than a redundancy to factor out. It
filters the survey `ps` already walks (`crates/pgrep` takes a finished `ps::Survey` and never touches
a cursor), and the selector is the run state, matched by `crates/glob` against the four state names
`ps` prints.

**What the demonstration shows, and it is one answer more than `ps` had.** A selector that **matched
nothing** in a domain that really has members is a distinct outcome from an empty domain and from a
refusal, and upstream `pgrep` collapses all three into printing nothing and exiting 1, so a monitor
there cannot tell an idle machine from a closed door. Four answers, asserted in twelve host tests and
again on a real kernel domain on both ISAs
(`kernel::user::survey_tests::a_filter_names_members_and_tells_its_four_answers_apart`), which also
asserts the claim the whole pair exists to make: **the capability that printed a corpse's tid is
refused the reap.**

**The limitation that shaped it, and it is a property of the boundary rather than of the program.**
Nothing in this system delivers *bytes* from a command line to a program: `Endowment::arg` is one
`u64`, `spawnproto` carries three words, and every string-shaped designation a person types arrives at
the child as a **capability**, which is what `rm logs/old` and `doc notes/glob.md` both are. So a
pattern cannot be typed at this prompt, and the shipped `pgrep` names every member, which is
`Prog::Date`'s deliberate under-declaration verbatim. It lifts when `ArgSpec` grows milestone 47's
deferred positional arity. Written up in notes/process-view.md, with the reason recorded where a
reader meets the feature.

**Still to build:** the rest of the view stratum (`top`, `pwdx`, `w`), each now blocked on a
2026-08-26 fork rather than on unbuilt-but-clear work (see the `Fork:` sections below). The
signalling stratum is no longer on this list; see the ruling below. `pmap` is **built, 2026-08-23**
(DECISIONS §114; see above), though not reachable from the interactive shell -- a real finding
that build turned up, not a caveat this block is glossing over. `sysctl` itself is **declined
(DECISIONS §115)** rather than blocked on effort. `watch` is **built, 2026-08-24** (see above),
redrawing `ps`'s own domain walk rather than an arbitrary command. `uptime` is **built,
2026-08-26** (see above), needing no capability at all. The package file list is **verified,
2026-08-24** (see above), against a real `dpkg -L procps` on Ubuntu 24.04; the one correction it
found (`pidwait`, missing from the table
below until this edit) is recorded and not attempted by this lane.

## Why this package, and why the package rather than the program

**What these programs want is enumeration of the process namespace, and enumeration is the authority
this system is built to refuse.** Milestone 121 makes the same argument for directories: a program
that can list learns what exists, which is a larger power than reading something it was handed.

On Linux the answer comes from `/proc`, which is **ambient**. Any process reads it with no grant from
anyone, so `ps aux` prints every command line on the machine, including the ones with secrets in
`argv`. Nobody defends that design; they live with it, and `hidepid` exists because enough people
stopped wanting to.

That makes this a better first demonstration than `ripgrep` on one axis: **the reader already knows
the Unix behaviour is wrong.** The claim needs no setup.

`procps` (upstream `procps-ng`) is Priority: important, so it is on essentially every Ubuntu install
and high on any popcon ordering. **Taking the package whole is the point rather than a burden**: it
is the unit the distribution ships, so it is the unit that tests whether the approach generalises. A
port that cherry-picked the two programs with the tidiest capability story would prove nothing about
typical software.

**Confirmed, 2026-08-24 (see above): a real `dpkg -L procps` / `dpkg -s procps` against Ubuntu 24.04**
(`podman run --rm ubuntu:24.04`, package version `2:4.0.4-4ubuntu3.2`, architecture `arm64`). The
table below was from memory when this section was first written and undercounted by one:
`pidwait` ships in the package and was not in it. Every other name was confirmed correct.

## The strata, which are the build order

**The package is the unit of ambient authority, not the program.** All of these exist because
`/proc` is readable by anyone. Once that is replaced by a held capability, they stop being one thing:

| what it actually needs | programs | state here |
|---|---|---|
| **read the process namespace** | `ps`, `top`, `pgrep`, `pidwait`, `pmap`, `pwdx`, `w` | `ps`, `pgrep` **built**; `pidwait` found by the 2026-08-24 verification and not yet designed (it blocks until a match terminates, closer to a wait/reap than a snapshot); the rest each blocked on something named above |
| **signal a process** (control, not view) | `kill`, `pkill`, `skill`, `snice` | **mostly abolished 2026-08-17**: a domain names, never acts, and a tid is not a capability. Killing stays with whoever holds the child's region |
| **machine-wide statistics**, no process namespace | `free`, `uptime`, `vmstat`, `slabtop`, `tload` | `uptime` **built, 2026-08-26**, needing no capability at all (an existing ambient exception, not a new one); `free`/`vmstat` blocked on a 2026-08-26 fork, see below |
| **write kernel tunables** | `sysctl` | no design, and see the fork below |
| **none of the above** | `watch` | **built, 2026-08-24**: `line_editor` and the compositor turned out not to be what it needed (its output travels the same sink-and-terminal path `ps` and `date` already use), but the "needs nothing new" call was right |

Build in that order. `ps` first, because it is a snapshot of the domain and needs no clock and no
accounting, so it is the whole capability argument with none of the scheduler work. Then the rest of
the view stratum, then signalling, then statistics, then `sysctl`, then `watch` whenever.

**`pgrep` came second rather than `top`, and the order turned out to matter more than expected.** It
needs nothing that does not exist, so it was the cheapest way to find out whether the capability
argument survives being *filtered* rather than merely listed. It did, and it also surfaced the
boundary limitation above, which is a fact about every future program that wants an operand and not a
fact about `pgrep`.

## The design: a view over a supervision domain

**The scope is the supervision subtree, because the kernel already maintains it.** A shell holds a
domain; the programs it spawns are in that domain; a `ps` launched from that shell sees exactly those
and nothing else. Same move `rm -r` makes with a directory subtree and `ripgrep` will make with
`ENUMERATE`: **authority is a subtree, not a global.** A scope the system already keeps cannot drift
out of agreement with reality.

**A wide grant is fine and must be nameable.** An operator's `top` genuinely wants the whole machine.
The point is not to forbid it but to make it visible: `caps top` should print the difference between
a `top` that sees one shell's children and a `top` that sees everything. On Linux there is no such
distinction to print, which is the whole difference.

## The demonstration was `pgrep` beside `pkill`, and the ruling took `pkill` away

**A domain names its members and does not act on them** (calef, 2026-08-17). That answers the
signalling stratum before it was built, and it mostly abolishes it.

The reason is one the ABI already made and nobody had read back: **a survey returns a tid, which is
a name and not a capability**, and there is no path from a tid to authority over that thread. `Tcb`
has no `DESTROY`; killing a live child is `Untyped::DESTROY` on the region it was built from (§24's
forcible `^C`), held by whoever spawned it. So `pkill` cannot be assembled out of a view, and making
a domain confer control would be the one place this system copied the thing it exists to refuse,
which is Unix turning `kill(pid)` from a name into an action.

What replaces the demonstration: **`caps pgrep` prints a scope, and there is no `caps pkill` to print
beside it**, because the program does not exist here. That is a weaker side-by-side and a stronger
claim, and the write-up has to make the trade explicit rather than quietly dropping a promised
comparison. Killing stays with the shell that spawned the thing, which already holds the region.

**Done, 2026-08-17.** `pgrep` is built and the trade is stated in three places a reader might arrive
at: `crates/pgrep`'s module docs, `user/src/pgrep.rs`'s, and notes/process-view.md's own section. The
claim is also asserted rather than argued: the kernel test filters a domain down to its corpse and
then shows that the same capability which named the tid is refused the reap.

The cost, stated plainly: `procps` gets ported without its signalling stratum, and a reader who
expects `kill` to be a program will not find one.

The negative control keeps milestone 108's shape: a viewer run against a domain it was not granted is
**refused loudly** rather than shown an empty list. A monitor that silently reports nothing because
it could not look is the worst failure available to this tool, and `fs_proto` already chose `EPERM`
over an empty listing for exactly that reason.

## `sysctl` is declined (DECISIONS §115)

It writes machine-global kernel tunables, and **it ships in the same package as `ps`**, which is a
striking illustration of what Unix packaging bundles: `apt install procps` gets you process listing
and the ability to retune the kernel.

There is no ambient tunables namespace here to write to, and inventing one would import exactly the
thing this system exists to avoid. Two shapes were named: a capability-per-subsystem `sysctl`
program holding a bag of them, honest but a different program wearing the same name; or no
`sysctl` at all, each subsystem's tuning reached through that subsystem's own service, which
breaks the package's coverage claim and says so.

**Decided (calef, 2026-08-23): no `sysctl`.** Same ruling as `pkill`'s decline above, one layer
over -- authority stays with whoever already holds a resource, never centralized into a generic
tool -- and the same shape `notes/net.md` already built favorably (`announce 80` written to
`/net/tcp/clone`, Plan 9's per-resource `ctl` file over a global panel). `procps` ships without
`sysctl`; the coverage gap is recorded rather than glossed over, same as `pkill`'s absence.

## Fork: `top`'s per-thread CPU accounting

**Status: PROPOSED, 2026-08-26 (this lane).** Investigated rather than built, because the
mechanism it needs crosses the syscall surface, which AGENTS.md reserves for calef.

**The premise this milestone's own BUGS section stated turned out to be false, and the correction
matters before the design question does.** The entry read: *"`top` needs per-thread CPU
accounting that does not exist at all: `QuotaToken` is dead code whose own comment says
`spawn_with_quota` 'has no caller of its own today'."* `QuotaToken` and `spawn_with_quota` are
real and are exactly as dead as quoted (`kernel/src/thread.rs:301-324`,
`kernel/src/sched.rs:1122-1187`), but they have nothing to do with CPU time: a `QuotaToken` is a
reserved slot in a **spawn-count budget** ("at most `budget` of these may be alive at once"),
returned to the counter when the thread is reaped. It bounds how many *children* a spawner may
have alive, the same job §28/milestone 41 moved to per-process untyped retyping (the budget *is*
the quota now, enforced by retyping rather than a counter), and it has never measured a single
tick of anyone's CPU time. Wiring `top` through it would not produce CPU accounting; it would
produce a child-count limiter wearing `top`'s name.

**So the honest finding is stronger than the BUGS entry: there is no per-thread CPU accounting
anywhere in this kernel, dead or live, and no partial mechanism to wire.** Checked directly:
`Thread` (`kernel/src/thread.rs`) carries no time-on-CPU field of any kind. The timer IRQ handler,
`sched::on_tick()`, does exactly one thing: `cpu::current().need_resched.store(true, ...)`, plus
the corruption-canary check, and touches no per-thread state. `sched.rs` has a **global**
`preemptions()` counter (how many preemptions have happened machine-wide, used by the
`a_thread_that_never_yields_is_preempted_anyway` test) and nothing per-thread. `abi::rendezvous::SURVEY`,
the mechanism `ps`/`pgrep`/`watch` already use, reports a state (`READY`/`RUNNING`/`BLOCKED`/`DEAD`)
per snapshot and nothing cumulative. Building `top` therefore means adding new kernel state, not
wiring existing state, and the state has to cross the syscall boundary to reach userspace, which
is why this is a fork rather than a build.

**What "CPU accounting" could mean here, and the tree does not already answer this the way it
answers most of the other five questions.**

1. **Wall-clock age (time since spawn).** Cheapest possible: one `spawn_instant: u64` field on
   `Thread`, set once at `START` from the ambient counter (`uptime`'s own primitive, see above),
   read back as `now - spawn_instant`. Trivial to implement, but it is not what `top`'s `%CPU`
   column means anywhere else: a thread that has been alive five minutes and a thread that has
   run continuously for five minutes look identical. Naming this "CPU accounting" would mislead a
   reader who knows Unix `top`.
2. **Scheduled (on-CPU) time, accumulated.** What Linux's `utime`/`stime` and Fuchsia's
   `zx_object_get_info` runtime both are: a counter incremented by the scheduler itself at every
   switch-out (`schedule()`'s dispatch point already touches the outgoing and incoming `Thread`,
   so the hook point exists) or, cheaper and coarser, sampled once per tick by charging
   `on_tick()`'s currently-running thread one tick (the 100 Hz `TICK_HZ` this kernel already
   runs). This is real CPU accounting and is the one a reader expecting `top` would recognize.
   It is also genuinely new kernel state (a `u64` per `Thread`, written on the hottest paths in
   the scheduler) and needs cross-core correctness worth stating plainly: each core's tick and
   switch touch only the thread on that core, so no cross-core synchronization is needed for the
   write, but a **reader** on one core observing a counter another core is actively incrementing
   is an ordinary relaxed-load race, the same shape `TICKS`'s per-CPU array already accepts.
3. **Sampling over `SURVEY`, no new kernel state at all.** A monitor could poll
   `abi::rendezvous::SURVEY` repeatedly (as `watch` already does) and estimate `%CPU` statistically
   from how often a tid's `state` reads `RUNNING` versus `READY`/`BLOCKED` across samples. This is
   the only option that needs **zero** new kernel state and zero new syscall surface: it is pure
   wiring against what `ps`/`watch` already ship. It is also not accounting in any real sense: a
   thread that runs in the gaps between samples is invisible, short bursts are systematically
   under- or over-counted depending on phase, and the number a reader sees would not match what
   the kernel itself could report exactly if asked. Named for completeness and because option 5 of
   the six questions (measure, don't assert) requires naming the zero-cost option even when it is
   not the recommendation.

**What every option that produces a real number needs, regardless of which semantics is chosen:
a way to get it out of the kernel, and that is itself a wire decision.** `abi::rendezvous::SURVEY`
already returns three words (`next_cursor`, `tid`, `state`) and a fourth (a CPU figure) would
widen an existing method's return shape rather than add a new syscall number, mirroring how
`pmap` added a method to a different object type instead of a syscall (DECISIONS §114). Whether
that fourth word rides on `SURVEY` itself, or arrives through a second method the same
`ENUMERATE` right gates, is exactly the kind of thing AGENTS.md calls out by name: "Anything two
programs agree on... The syscall surface... which every future program is written against."
Cheap to prototype, expensive to un-ship.

**Recommendation: option 2 (scheduled time, tick-sampled rather than switch-accumulated), exposed
as a widened `SURVEY` return, per-thread rather than per-process.** Reasons, briefly: tick-sampling
costs one branch and one increment inside code that already runs every tick on every core, versus
touching two `Thread`s on every voluntary yield in `schedule()`'s hot path; per-thread rather than
per-process because **this kernel's native unit is the thread**, exactly as `ps` and `pgrep`
already report it: this system has no process/thread-group construct to aggregate into, and
inventing one only for `top` would be new state with no other consumer, the same speculative
abstraction §46 and this milestone's own ENUMERATE rustdoc already decline elsewhere. Prior art
for the recommendation, not just the alternatives it rejects: Linux's `jiffies`-based `utime`
accumulates at the scheduler tick for exactly this reason (cheap, coarse, good enough), and no
mainstream `top` implementation ships the pure-sampling approach (option 3) as its primary source,
which is some evidence it does not hold up as a real number even where it is available for free.

**This is a fork rather than a recommendation-only decision** because unlike milestone 54's demo
share (recommended and moved on), this one touches the syscall surface and a wire format two
programs (kernel and every SURVEY reader) would agree on forever; AGENTS.md's own rule is
"recommend on reversible forks; give options only on irreversible ones," and this is the second kind.
**What is blocked on this:** `top` itself, entirely; nothing else in the milestone depends on it.

## Fork: a process display name (`pwdx`, `w`)

**Status: PROPOSED, 2026-08-26 (this lane).** Split into two questions on purpose, because they
turned out to have different answers.

**The authority question has a clean answer, and it falls out of work this milestone already
did.** The BUGS entry worried that "a name is information rather than authority, but a confined
viewer may still not be entitled to it." But `Rights::ENUMERATE` was defined, in this same
milestone, as exactly "the right to **learn what exists**, as distinct from acting on it"
(`crates/capability/src/lib.rs`), and its own rustdoc already names the two objects expected to
grow it (`AddressSpace`, built, `pmap`; `MemoryRegion`, not yet, `free`), on the same "ask
what exists, not what it is doing" argument a name would need. A display name is *more
information about a thing already named*, not a new kind of access to it: a viewer that already
holds `ENUMERATE` on a supervision endpoint can already learn a member's tid and run state, both
of which are more operationally sensitive than a static program name (a state transition can leak
timing; a tid is already the handle every other SURVEY-gated operation keys on). There is no
principled reason a name would need a *stronger* right than the one that already unlocks
"everything else about this member except acting on it." **Recommendation: a display name, if
built, is gated by the same `ENUMERATE` right `SURVEY` already checks: no new right, no widened
manifest field beyond what `ps` already declares.**

**The mechanism question does not have a clean answer, because the thing being gated does not
exist anywhere in the tree to be gated.** Checked directly: `Spawn::arg0` (`kernel/src/user.rs`)
is one `u64` register, used today for role selection and integer arguments (`worker`'s multiplier,
`date`'s format selector), never a string. `grant_plan::Prog` *does* know a program's name at the
moment the shell resolves it (`Prog::from_name`/`Prog::name`), and `system_initializer`'s
`spawn_service` *does* read that `Prog` to find the ELF to load, but nothing persists the
association past that one spawn call. No `Thread` field, no per-tid table in init, no table in
any supervisor (checked `root_supervisor.rs`, `sub_server_supervisor.rs`,
`crates/component_plan`, none of which keep a live tid-to-name map either; `component_plan`'s
"role name" is a build-time label on a static declaration, not a runtime lookup). So even granting
every viewer `ENUMERATE` today would answer nothing, because there is nothing on the other end of
the lookup.

**Two shapes close that gap, and they are not equally invasive:**

1. **Kernel-resident name.** `Thread` grows a fixed-size byte field (bounded, `nifefs`'s
   `NAME_LEN = 32` is the precedent for what this tree considers a reasonable cap), set once at
   `START` from bytes the spawner supplies. This is symmetric with how `arg0` already works and
   would let a widened `SURVEY` (or a new method, same question `top`'s fork raises) report a name
   the same way it reports state, but it is a **wire commitment**: `START`'s signature changes
   (today `invoke(cap, START, _, _, _)` takes nothing), or a new `CAP_INSERT`-shaped call carries
   the bytes before `START`, and either way it is new syscall surface two programs agree on
   forever, the same category `top`'s fork is in.
2. **Userspace-resident name, kept by whoever already knows it.** `system_initializer` already
   knows the `Prog` at spawn time and already is the one process every supervised child's fault
   endpoint (`deaths`) routes through. It could keep its own tid-to-name table and answer a lookup
   itself, but "answer a lookup" means a new RPC surface *to init*, since `deaths` today is a
   fault endpoint with a kernel-fixed method set (`RECV`/`REAP`/`SURVEY`), not a channel init reads
   requests from. This avoids touching the syscall surface but invents a new userspace protocol
   with its own authority question (is holding `ENUMERATE` on `deaths` sufficient to also query
   init's table, or does init need to re-derive the same check SURVEY's kernel code already does)
   that this tree has no precedent for solving cleanly, since every other cross-process query here
   (`SURVEY`, `aspace::LIST`) is a kernel-adjudicated method on the object itself, not a request to
   a third process holding a side table.

**No recommendation between the two**, per AGENTS.md's own limit ("recommend on reversible forks;
give options only on irreversible ones"): both are syscall-surface-adjacent decisions (one
literally is one; the other invents a new inter-process protocol shape this tree has not used
before), and a wrong pick here is expensive to unwind either way. **What is blocked on this:**
`pwdx` and `w`, both of which are otherwise small (`pwdx` is "print a name for a tid"; `w` is
`ps` with an idle-time column, itself downstream of the CPU-accounting fork above). Once a
mechanism is chosen, the authority answer above already applies without further discussion.

## Fork: `free` and `vmstat`'s machine-wide memory statistics

**Status: PROPOSED, 2026-08-26 (this lane).** The third member of "machine-wide statistics," and
the one that confirms `uptime` (built, see above) was the exception rather than the rule for that
row: `free` and `vmstat` both want physical-memory accounting, and unlike the monotonic counter,
nothing makes that accounting ambient today.

**What the kernel already tracks, checked directly in `kernel/src/memory.rs`.** A page-frame
bitmap allocator with `stats() -> Option<Stats>`, `free_page_frames() -> usize`, and
`largest_free_run() -> usize`, used today only for the boot-time `print_summary()` diagnostic
printed to the kernel console before any userspace exists. This is real, already-maintained data,
the same "already keeps it for its own reasons" property that made `ps`'s supervision-subtree
scope and `pmap`'s revocation-log read free rides, but it is **kernel-internal state with no
existing path to userspace**, unlike the monotonic counter (`uptime`'s fork), which is a hardware
register the kernel explicitly opened to EL0 (`CNTKCTL_EL1`'s `EL0VCTEN`, a documented exception
to DECISIONS §10). There is no analogous exception for physical-memory statistics, and inventing
one is squarely a new-syscall-surface question, the same category as both forks above: the number
lives only in kernel data structures, and the only way out is a trap.

**This is also where `capability::Rights::ENUMERATE`'s own rustdoc already puts the next question,
which this lane did not have to invent.** It names `MemoryRegion` (DECISIONS §113's rename of
`Untyped`) as the second of the two objects "expected to grow it," for `free`'s benefit: "ask what
is committed without being able to `SPLIT` or `DESTROY`." That is a **per-capability** query (how
much of *this* region, which someone already holds, is committed), not a **machine-wide** one (how
much RAM does the whole box have free), and `free`/`vmstat` upstream ask the second question, not
the first. The two are genuinely different asks with different confinement stories:

1. **Per-region `ENUMERATE`, following the pattern already named.** Exactly the `pmap` shape: a
   new method on `MemoryRegion`, gated by `ENUMERATE`, answering "how much of the region behind
   *this capability* is committed." No ambient machine total, consistent with this system's
   region-ownership model (DECISIONS §13: memory belongs to whoever's budget it came from, not to
   a global pool anyone can ask about). This is *not* what upstream `free` reports, and a `free`
   built this way would need to say so plainly, the same honesty `pmap`'s BUGS section already
   models for a narrower-than-upstream program.
2. **A machine-wide figure, ambient or capability-gated.** Closer to what a reader expects from
   `free`, and further from this system's design: nothing today lets a program learn a fact about
   memory it does not hold a capability to, and a global free-memory total is exactly that. If
   built, the clock page's shape is the closest existing precedent worth reusing rather than
   inventing a new one from scratch: a service publishes into a page it owns, readers hold a
   capability to a read-only mapping of it, but *unlike* the clock page, nothing today computes
   this figure in userspace: the data starts out kernel-internal, so some new kernel-to-userspace
   channel (a syscall, or the kernel writing into a page it hands to a stats-publishing service)
   is unavoidable regardless of which shape the read side takes.

**No recommendation**, for the same reason as the name fork above: this is a confinement-model
question (does "how much memory is free" carry the same region-scoped authority everything else
in this system carries, or is it the one machine-wide ambient fact besides monotonic time), not
an implementation-cost one, and it is calef's to make. **What is blocked on this:** `free` and
`vmstat` entirely. Nothing else in the milestone depends on either.

## The other fork: where the process view comes from

**Derive it from the supervision tree** (recommended), or **give processes a separate namespace with
its own capability**. The first is elegant, already exists, and cannot disagree with reality. The
second is more flexible: it can express a monitor watching two unrelated services, which a subtree
view can never say. Taking the first forecloses that, and the honest workaround is a supervisor whose
only purpose is to be their common parent.

## Prior art

**A design to copy: Fuchsia's job handles**, which is this milestone's design already shipped. A
Fuchsia process lives in a *job*, jobs nest, and listing processes requires a handle to the job whose
children you want; their `ps` needs a handle to the root job to see everything. That is exactly the
"wide grant, explicitly held" shape proposed here. Worth reading for how they handle a process that
dies mid-enumeration, which this block has no answer for.

**A mistake to avoid: `/proc` as an ambient filesystem.** Plan 9 made `/proc` cleaner than Linux did
and still put process state in a namespace a program reaches by *naming* rather than by *holding*.
Getting this wrong looks like `ps` working beautifully while the confinement is decorative.

**Code to use:** none for the capability half. The rendering (columns, sorting, a redrawing terminal)
is ordinary, and `line_editor` and the compositor already exist beneath it.

## BUGS

- **`pmap` cannot be run from the interactive prompt against any real process, and this is a gap
  in the address-space object's lifecycle rather than in `pmap`.** Every `Object::Aspace`
  capability in this tree is minted and consumed within the thread that builds it
  (`RETYPE_OBJ(ASPACE)` -> `MAP_INTO`* -> `Tcb::CONFIGURE`, which removes the space from the
  registry the instant it binds to a thread), and the 2026-08-23 delegation audit DECISIONS §114
  required found that nothing shipped here delegates one to a different program. So there is no
  address space anywhere in the system, held by anyone other than its own builder, that a second
  program could be handed a live view of; `ps`'s `Manifest::domain` has no analogue because there
  is nothing alive to wire it to. The kernel method (`abi::aspace::LIST`, `Rights::ENUMERATE`) and
  the program are real and proven end to end against a genuine `Object::Aspace`
  (`kernel::user::pmap_tests`, `ps`'s `survey_tests` discipline exactly). Fixing this is a
  spawn-protocol question -- a builder handing a narrowed, still-registered view to a third party
  before `CONFIGURE` consumes its own copy -- not a `pmap` change, and it is named here for whoever
  takes it next rather than decided by the lane that found it.
- **`pmap` shows one row per mapped page with no VA-range coalescing or size column**, because
  `abi::aspace::LIST` reads the space's revocation log, which records one entry per page and
  nothing about adjacency. Upstream `pmap` coalesces contiguous same-permission pages into ranges;
  doing that here would need an ordering guarantee the log does not make.
- **`pmap` cannot tell a device mapping from ordinary read/write memory.** `kind` comes from
  `paging::Flags`, which carries no "this is device memory" bit as far as the syscall handler can
  see; a `DeviceFrame` mapping reads as `rw-`, indistinguishable from a heap page.
- **`procps` ships without `sysctl` (DECISIONS §115), the same gap `pkill`'s absence already
  states.** A reader who expects to retune the kernel through one program, the way `apt install
  procps` provides on Linux, will not find one here. Each subsystem that grows a runtime tunable
  carries its own control surface instead; there is no unifying admin tool by design.
- **Estimating the package from the `ps` half gets it wrong twice, and one of the two ways was
  itself a misreading, corrected 2026-08-26.** `top` needs per-thread CPU accounting that does not
  exist at all, but not because of `QuotaToken`: that dead code (its own comment says
  `spawn_with_quota` "has no caller of its own today") is a spawn-**count** budget, unrelated to
  CPU time, and wiring `top` through it would produce a child-count limiter rather than an
  accounting mechanism. There is no partial CPU-accounting machinery anywhere in this kernel to
  wire, dead or otherwise; see **Fork: `top`'s per-thread CPU accounting**, above, which also
  found that a real answer needs new syscall surface, not just new kernel state. And `free`,
  `uptime` and `vmstat` want machine statistics rather than process enumeration, so **building
  `top` does not give you `free`**, except `uptime` turned out not to need machine statistics at
  all (**built, 2026-08-26**, see above): it reads the same ambient monotonic counter `date`
  already does, needing no capability and no design decision. `free` and `vmstat` still do; see
  **Fork: `free` and `vmstat`'s machine-wide memory statistics**, above. Three separate bodies of
  work wear one package name, and now one of the three is finished.
- **Aggregate statistics are a side channel, and capabilities do not close it.** CPU time per process
  leaks information about work the viewer was never shown, even with names withheld. A capability
  bounds *who* may ask; it says nothing about what the numbers reveal to whoever may. A real limit of
  the model, recorded next to the feature rather than in a threat model nobody reads.
- **A process has no name here.** `ps` shows command lines; this system has `arg0` in `Spawn` and no
  display name. **The authority half of this is now answered (2026-08-26): a confined viewer that
  already holds `ENUMERATE` is entitled to a name for the same reason it is entitled to a tid and a
  state, since a name is more information about a member already named rather than a new kind of
  access.** What has no design is the mechanism: nothing in this tree persists a tid-to-name
  association past the one spawn call that briefly has both, so before an authority question can
  even apply, something has to decide where a name would live and how it crosses a process
  boundary, which is squarely the syscall-surface question the "move fast on what can be undone"
  tenet reserves for calef. See **Fork: a process display name**, above.
- **A supervision-derived view cannot express a non-subtree set**, if that fork lands that way. The
  workaround is a supervisor existing only to be a common parent, and it should be recorded when the
  fork is decided rather than found by whoever first needs it.
- **The comparison against Linux is not apples to apples and the write-up must say so.** Ours lists a
  domain; theirs lists a machine. That is the entire point, and a table putting them side by side
  without stating it would be dishonest in the way §14's map "tie" caveat exists to prevent.
- **The package membership is verified, 2026-08-24** (`podman run --rm ubuntu:24.04`,
  `dpkg -L procps` / `dpkg -s procps`, package `2:4.0.4-4ubuntu3.2` arm64). One name was missing
  from the memory-sourced table this replaced: `pidwait`. It is recorded above in the strata table
  and not designed or built here.
- **`watch` is not upstream `watch`.** It redraws one fixed built-in view (`ps`'s own domain walk)
  rather than re-running an arbitrary command line, because re-running a named command needs spawn
  authority this system grants to the shell alone, and nothing here delegates that authority onward
  to a spawned program (the same gap `top`, `pwdx` and `w` are blocked on). A reader who expects
  `watch <any command>` will not find one; `watch <count>` is what exists. See `crates/watch`'s and
  `user/src/watch.rs`'s own module docs for the full argument.
- **`watch` cannot be interrupted with `^C` mid-run.** It is not spawned as an interruptible job (an
  interruptible child in this system is built with no capabilities in its cspace at all, and this
  program needs the domain and the output sink for its whole run), so a bare `watch N` runs its full
  count before the prompt returns. It always terminates on its own, so this is a wait rather than a
  hang, but there is no way to cut one short today.
- **`watch`'s interval is fixed and is a yield-spin, not a sleep**, because this kernel has neither.
  It is milestone 106's fifth named consumer (`user/src/timetable.rs`'s module docs name the first
  four); a five-frame `watch` burns a core for roughly two seconds to do what a real timer would do
  for nothing. The interval is also not settable from the command line: `ArgSpec` carries one
  integer and it is spent on the redraw count.
- **A domain that becomes refused partway through a `watch` run stops the loop silently**, with no
  further complaint, because DECISIONS §67's diagnostics-before-output rule closes that stream
  before the loop starts. In practice this needs the domain's own endpoint to be destroyed mid-run,
  which nothing in this tree does to a live supervision endpoint today.
