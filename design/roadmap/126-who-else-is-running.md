# 126. The `procps` package: who else is running, and who is allowed to ask

**Status: PARTIAL.** Minted 2026-08-14 by calef, from a design conversation about what ambient
authority utilities become on this system. **Scoped to the whole package by calef the same day**, for
consistency with milestone 123's approach to popular packages: the corpus is chosen by an external
ordering and taken in the units that ordering uses, which is packages rather than programs we like.

**Gate: NONE.** As of 2026-08-23, both forks this gate pointed at are decided: `pmap`'s
`ENUMERATE`-on-address-space extension is **yes** (DECISIONS §114), and `sysctl` is **declined**
(DECISIONS §115), each subsystem's own service carrying its own tuning instead. What remains is
real unbuilt work rather than anything waiting on calef: `top` on per-thread CPU accounting that
does not exist; `pwdx` and `w` on a process display name this system has no design for.

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

**Still to build:** `top` (per-thread CPU accounting that does not exist), `pwdx` and `w` (a
process display name this system does not have), and the machine-wide statistics. `sysctl` is
declined (DECISIONS §115) rather than blocked on effort. `watch` is **built, 2026-08-24** (see
above); the package-membership confirmation is **done, 2026-08-24** (see above), and found one
name the memory-sourced table below had missed (`pidwait`).

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

**Still to build:** the rest of the view stratum (`top`, `pwdx`, `w`) and the machine-wide
statistics. The signalling stratum is no longer on this list; see the ruling below. `pmap` is
**built, 2026-08-23** (DECISIONS §114; see above), though not reachable from the interactive shell
-- a real finding that build turned up, not a caveat this block is glossing over; `top` on
per-thread CPU accounting that does not exist; `pwdx` and `w` on a process display name this system
does not have. `sysctl` itself is **declined (DECISIONS §115)** rather than blocked on effort.
`watch` is **built, 2026-08-24** (see above), redrawing `ps`'s own domain walk rather than an
arbitrary command. The package file list is **verified, 2026-08-24** (see above), against a real
`dpkg -L procps` on Ubuntu 24.04; the one correction it found (`pidwait`, missing from the table
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
| **machine-wide statistics**, no process namespace | `free`, `uptime`, `vmstat`, `slabtop`, `tload` | **a different capability entirely**, and none exists |
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
- **Estimating the package from the `ps` half gets it wrong twice.** `top` needs per-thread CPU
  accounting that does not exist at all: `QuotaToken` is dead code whose own comment says
  `spawn_with_quota` "has no caller of its own today". And `free`, `uptime` and `vmstat` want machine
  statistics rather than process enumeration, so **building `top` does not give you `free`**. Three
  separate bodies of work wear one package name.
- **Aggregate statistics are a side channel, and capabilities do not close it.** CPU time per process
  leaks information about work the viewer was never shown, even with names withheld. A capability
  bounds *who* may ask; it says nothing about what the numbers reveal to whoever may. A real limit of
  the model, recorded next to the feature rather than in a threat model nobody reads.
- **A process has no name here.** `ps` shows command lines; this system has `arg0` in `Spawn` and no
  display name. A name is information rather than authority, but a confined viewer may still not be
  entitled to it, and there is no design for that today.
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
