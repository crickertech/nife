# The nine things that would kill nife

calef, 2026-08-30: *"something that would kill nife for me as a project is a fatal characteristic
that would demonstrate the approach isn't viable... We should then try to prove or disprove those
things."*

This file is the falsification list. Not a risk register in the project-management sense, which
tracks things that might go badly; **a list of claims that, if false, mean the project should stop.**
The difference is the point. A risk you can mitigate belongs in a milestone. A risk you can only
answer belongs here.

It was written the same week the project's first customer left. The family's backups moved to borg
over SSH on cordoba, with Immich for images, because the data problem was pressing and nife was not
ready; journey 2 was retired, and milestone 55's premise went with it. That is not what this file is
about, but it is why it exists now: **with no customer, the ranking function has nothing to rank by**,
and the honest substitute is not "pick interesting work" but "find out whether this can work at all."

## The rule an entry has to meet

Three properties, and an entry that lacks one is a worry rather than a risk:

1. **It can come back red.** An experiment that can only confirm is not a test. Where an experiment
   is structurally confirmation-biased, the entry says so and names the second pass that fixes it
   (risk 2 is the worked example).
2. **The experiment is cheap relative to the project.** A test that costs a year answers a question
   the year would have answered anyway.
3. **It does not wait on more of the project being built.** Otherwise it is a schedule, not a test.

**And the ranking is chance-of-fatal times cheapness-of-test**, which is why the running order at
the bottom is not the numbering. The numbers are identity, like a milestone's.

## 1. Only software written for nife runs on nife

**The claim, stated so it can fail:** the platform can run hand-written Rust and nothing else, so
every piece of software anyone wants has to be rewritten.

**Why it is the most dangerous one:** it is structural. Optimization cannot fix "nothing runs here",
and no amount of kernel work changes it. A system in this state is a research demonstrator forever,
which is a legitimate thing to be and is not what DECISIONS §14 (a verified-Rust capability microkernel that runs real workloads) claims.

**Evidence today, both directions.** For: milestone 27 (Rust `std` on the native ABI) works, and
milestone 64 sorted crates.io by build status. `kilo` runs. Against: DECISIONS §105
(`std::thread::spawn` stays declined) means `rayon`, `tokio` and `crossbeam` compile and link but
cannot spawn; `std::process` refuses everything; there is no `fork`, no POSIX, no libc tier three.

**The experiment:** milestone 121 (`ripgrep`: enumeration as a capability), chosen because `ripgrep`
has a real dependency tree, walks a filesystem, and uses threads.

**Status: RUN, 2026-08-31. GREEN, and the blocker is not what anyone predicted.**
notes/ripgrep-on-nife.md has it; PR #600.

- **Unmodified `ripgrep` 14.1.1 from crates.io, forty transitive crates, builds for
  `aarch64-unknown-nife` with zero source changes**, loads, runs, resolves its working directory
  through a granted directory capability, and exits through `std::process::exit`. **Zero patches.**
  Everything that differs from a Linux build is on the command line.
- **What stops it is that the ABI has no argument vector.** `std::env::args()` compiles std's
  `unsupported` backend and yields nothing, so `ripgrep` parses no arguments and prints its own
  *"requires at least one pattern to execute a search"*. **Somebody else's application reached its
  own error path on this kernel**, which is a far better result than the build failing.
- **§105 was never reached, and that is the finding that reverses the premise.** `ripgrep` does not
  assume parallelism, it **asks**: `available_parallelism()`, to which nife's PAL answers `Ok(1)`
  honestly, so it selects `search_serial` and never calls `thread::spawn`. **A platform answering
  `Unsupported` there would have failed this program.** The declined threads cost nothing here, and
  answering honestly rather than refusing is what made it work.
- **The capability model is visible from inside a stranger's program.** Without slot 4 the same
  binary prints `failed to get current working directory: operation not supported on this platform`.

**What it changes.** The structural fear behind this risk is retired: this system runs software it
did not write, unmodified, with a real dependency tree. What remains is an ABI gap with a name, which
is a design question rather than a wall.

## 2. The proofs prove trivia, and the real bugs live where Kani cannot reach

**The claim:** the verification half of DECISIONS §14 is real but narrow, and narrow in the direction
that does not matter.

**Evidence today:** 112+ Kani harnesses and `notes/verification.md`. Against: the VisionFive 2's
undelivered-wake bug was found by a bench on three harts, invisible in QEMU, and no proof was
positioned to see it.

**The experiment:** milestone 191 (did the proofs catch the bugs?), against this project's own defect
history, with a second pass over the harnesses asking which prove a property that could plausibly
have been false.

**Status: RUN, 2026-08-30. AMBER, and the red half is structural.** notes/proof-retrospective.md has
the study; PR #589.

- **No Kani harness in this tree has ever caught a defect after the day it was written.** All
  eighteen defects in the corpus were found by something else: a flaky suite, a boot on real
  silicon, a fuzzer, the mutation sweep, loom, a code read, or a CI lint. No red `script/verify` run
  appears anywhere in the record.
- **The cause is one line of `script/verify`'s own header**, verified rather than inferred:
  *"`cargo kani -p <crate>` never compiles the kernel, the user programs, or xtask."* So **64,818
  lines of `kernel/src` are out of reach by construction**, and that is exactly where every
  concurrency, hardware-contract and resource-accounting defect lived. The proofs are not failing to
  catch bugs in the code they cover; they do not cover the code the bugs are in.
- **Why it is amber and not red.** Two real defects were caught *while harnesses were being written*
  (`dtb::be32`'s unchecked `at + 4`, reachable from a corrupt device tree on the boot path;
  `pci::intx_irq`'s pin-0 underflow). That is the survivorship asymmetry this file's rule 1 warned
  about, showing up as evidence rather than as an excuse.
- **The strongest counterfactual is nearly a measurement.** The milestone 6 timer re-arm drift (100
  Hz configured, ~70 Hz delivered) has its property **already proved in this tree**, over
  already-written code, in `crates/timetable`'s `next_after`. The timer does not call it.
- **The numbers were wrong and are now counted:** **145** harnesses, not the roadmap's "112+";
  `script/verify` runs **140**; 31,725 of 206,728 source lines are reachable, though **both sides of
  that ratio count comments**, and `kernel/src` is 40% comment by measurement (25,762 of 64,818
  lines), so any published figure should be in code lines rather than raw ones; 19 `kani::cover!`
  vacuity guards exist, in 4 of 24 harness crates, and a vacuous harness reports `SUCCESSFUL`.
- **The reverse pass found real chaff**, which is what makes the green half credible:
  `capability::subset_is_reflexive` proves `a & !a == 0`, a tautology no plausible implementation
  error breaks, and twelve of the 26 `paging` harnesses are per-ISA restatements of six properties.

**What it changes.** The verification claim should be stated as what it is: proofs over the pure
crates, with the kernel itself largely unverified.

**And the amber moved the same day.** Milestone 193 (put `kernel/src` within reach of the prover) was
minted from this finding and built hours later: two properties proved over `kernel/src/syscall.rs`
with nothing moved into a crate first, **both falsified before being believed** by re-introducing
milestone 142's real wrapping-multiply defect and watching them turn red. That is the counterfactual
this study said the tree did not have, and it now exists. It cost about 10 seconds of
`script/verify`. `kernel/src/arch/`, `user/` and `xtask` are still out of reach, so the amber stands;
what changed is that the reason is now a worklist rather than a wall.

## 3. The tests do not test anything, and the quality is illusory

**The claim:** AGENTS.md's principle 2 says the method works because of the gates, the proofs and the
review discipline. If the suite would not notice the code being wrong, that sentence is decoration.

**Status: MEASURED, and it came back green.** `script/mutation` (milestone 85) ran 5,551 mutants over
38 host crates on 2026-08-03: 4,654 caught, 391 missed, 96 timed out, 410 unviable, which is **92.4%
of viable mutants killed**, with every survivor triaged into a test, an exclusion with a reason, or a
recorded gap. Five crates scored 100%.

**What that does not settle**, recorded because a green number is where inflation starts: the run is
from 2026-08-03 and the tree has grown since; it covers **host** crates only, so the kernel and the
arch trees, where risks 5 and 9 live, are not in it at all; and mutation testing measures the test
suite, not the code.

**The remaining experiment is cheap:** re-run it and compare against `.cargo/mutants-baseline.txt`.
No new milestone; milestone 85 already owns it and the weekly workflow already publishes the report.

## 4. The architecture imposes a per-crossing cost that cannot be engineered away

**The claim, and calef named this one first:** a capability microkernel pays on every boundary
crossing, and on workloads that cross constantly the cost is architectural rather than a matter of
tuning.

**Evidence today, and this is the best-covered risk on the list.** Milestone 138 (close the read gap) measured 16x against where it started, including 5.67x on a read and 8.02x on a write
from one wire change. `call_reply`'s steady state is measured and the live-replacement mechanism
costs zero in it (DECISIONS §41). Milestone 25 (cross-OS performance comparison) has numbers against
Linux and macOS with the honest ties recorded.

**The decisive experiment that has not been run:** milestone 168 (a multi-tasking workload
benchmark), whose own block calls it *"the number that would decide the event-kernel question."*
Milestone 188 (the IPC fastpath) is the follow-on if the number is bad.

**Ranked fourth on purpose.** This is where a skeptic expects the project to die and it is where the
project has the most evidence that it will not.

## 5. It cannot be made reliable on multicore, and the bugs appear only on silicon

**The claim:** the concurrency is wrong in ways that QEMU cannot show and that arrive one at a time,
forever.

**Why it is under-weighted:** it already fired once. The VisionFive 2 produced a receiver woken with
nothing delivered, on three harts, that no emulator run had ever shown, and the fix
(`wake_load_aware` refusing to make a waiting thread Ready unless the waker delivered) came out of a
bench session rather than a test. This is the class where OS projects lose years without ever getting
a clean red or green, which makes it the hardest entry on this list to run properly.

**Evidence for the defence:** DECISIONS §4 rule 4 (assume weak memory ordering) is a deliberate bet
that ARM-first development prevents hidden strong-ordering assumptions, and the tree has
`script/repeat-under-load`, `script/interleaving-check`, the loom work (milestone 135) and
`notes/load-sensitive-assertions.md`.

**The decisive experiment:** sustained multi-core stress on all three boards with the load-sensitive
assertions live. Expensive, hardware-bound, and it produces a confidence rather than a verdict, which
is honest about what this class of question can return.

## 6. A capability-confined userspace driver cannot drive real hardware at real speed

**The claim:** the thing that makes the thesis interesting, drivers outside the kernel behind an
IOMMU, does not survive contact with a real device.

**Evidence today:** milestone 16b proved IOMMU-backed DMA isolation against QEMU's emulation of the
ratified RISC-V IOMMU, over the §18 PCIe transport, and milestone 35 built the DMA validator. All of
it is virtio or emulated. The VisionFive 2 boots and its ratified-IOMMU silicon does not exist
(milestone 143).

**The decisive experiment:** one real, non-virtio device on real silicon, confined, at throughput.
The JH7110's GMAC (milestone 53) or NVMe behind milestone 163 (the JH7110's PCIe root complex) are
the candidates.

**Journey 3 settles most of this as a side effect**, because a framebuffer and a keyboard on real
hardware are real devices.

## 7. The confinement claim is false

**The claim:** a confined component escapes, and the property the whole system is built to provide
does not hold.

**Evidence today:** DECISIONS §31 (the foreign-language seam) proves a C component faulting on a
deliberate out-of-bounds write, restarted by its supervisor, with two witness pages answering two
different questions. `notes/untrusted-input-audit.md` surveys the attack surface, and there are fuzz
targets.

**What is missing:** every one of those is a test written by the same people who wrote the thing
being tested.

**Status: RUN, 2026-08-31, and it found the thing this risk exists to find.**
notes/confinement-claims.md; PR #614.

- **26 claims enumerated**, each with where it is stated, which test checks it, and whether that test
  has been shown to fail when the claim is broken. **Three were stated nowhere**, including one the
  system deliberately does *not* make: a confined device's **values** are not confined, only its
  reach. The IOMMU and the DMA validator constrain placement, never content.
- **25 harnesses now carry a replayable falsification**, up from 6. The sweep is 25 swept, 0
  survivors, and every patch names the assertion it expects to fail.
- **DECISIONS §31's headline assertion never runs.** Mapping `WITNESS_RO` read/write does turn the C
  seam test red, but `assert_eq!(v[2], CONFINED)`, the line that prints *"read-only witness intact"*,
  is not what catches it: **a component that is not confined does not fault, no fault means no death
  report, and the witness check is reachable only by an escape that faults anyway.** The sentence the
  seam is quoted for is not the sentence doing the work.
- **And the first attempt failed for the wrong reason**, which is the hazard this milestone's own
  block warned about, on the day it was written: the break surfaced as a 234-second watchdog timeout
  reading *"a livelock, not a lost wakeup"*, a correct red with nothing in it about confinement.

**What it does not say.** Nothing here says the confinement holds. What it supports is narrower and
was the point: these named claims are tested, and each has been shown to fail when the claim is
broken. **Six kernel confinement rows still have no mechanism at all**, and the adversarial exercise
this entry originally called for is still unbuilt: an outsider trying to escape, rather than us
demonstrating that a planned escape fails. That wants outside eyes and is gated behind milestone 198
by calef's no-third-parties position.

## 8. Nobody needs it

**The claim:** everything works and no one has a reason to run it.

**This one already fired**, which is the most useful thing about it. AGENTS.md's principle 1 ranks
work by the shortest path to a system a customer runs, and in August 2026 the customer had a real
deadline, nife could not meet it, and the customer solved the problem with Linux. That is the
principle working exactly as designed, and it is evidence rather than failure.

**What it changed:** the first customer was a family backup server, which is one of the largest
things a home system can be asked to be. **A first customer should be something nife can plausibly be
adequate at within a milestone or two.** The customer path is currently vacant, and it should be
recorded as vacant rather than implied by a roadmap that still names one.

**There is no experiment here**, which is why it is last in the numbering and not in the running
order at all. It is the question the other eight are in service of.

## 9. The HAL is a fiction, and an architecture costs a restructure rather than a port

**The claim, calef's, 2026-08-30:** *"another proof/disproof of the nife thesis is actual
functioning on the three silicons. If we can't get it to run on one, that would also likely kill the
effort."*

**Sharpened, because the ISA count is not the fatal part.** An OS that runs on two of three
architectures is still an OS. What would be fatal is what a failure would reveal: that adding an
architecture requires changing the kernel rather than adding a directory under `arch/`, which is
exactly what DECISIONS §4 rule 1 and §19 (architectural parity is a tenet) claim it does not.

**The status is asymmetric, and that is the useful part.** riscv64 already disproves the strong form:
the VisionFive 2 booted the full tour on three harts on 2026-08-14, which is the single strongest
piece of evidence in the tree that the HAL is real. aarch64 is the development ISA and its board (the
Jetson TX1, milestone 127) is well documented. **x86_64 is where the risk actually lives**, and not
because x86 is hard, but because it is newest: milestone 161 is `PARTIAL`, milestone 177's text says
x86_64 has no real interactive boot entry point at all, milestone 164 says its userspace cannot build
`aes` and therefore has no `fs_server`, and 165, 166 and 167 are each a piece of the same unfinished
edge.

**The decisive experiment is milestone 87 (the x86_64 bare-metal machine)**, which completes when the
OptiPlex prints a byte over serial. The machine, the serial module and the RS-232 chain have been
installed since 2026-08-23 and nothing has ever been booted on it. Then boot the tour. If it needs
driver work, that is schedule. If it needs the kernel restructured, that is the red result.

**It is not the free hour this file first called it**, and the correction is calef's, 2026-08-30,
asking why it should outrank finishing milestone 16 (real hardware + IOMMU-backed driver
isolation). `notes/x86-port.md`'s own `BUGS` says why: *"PVH is a hypervisor protocol and no real
firmware speaks it. Milestone 87's OptiPlex will need a UEFI stub or GRUB's Multiboot."* The kernel
boots under QEMU by PVH, and the OptiPlex's firmware does not speak it, so **first light needs a boot
entry path that does not exist yet.** It is bounded (the note says the 32-bit trampoline carries over
unchanged, because GRUB enters the same way; only the header and the `ebx` contract differ) and it is
a lane rather than a bench session.

**Journey 3 (the same story, on real silicon, on all three architectures) is the full-strength
version**, and it settles risk 6 along the way.

**One honest cost, recorded here rather than left implied:** parity is a multiplier on every other
risk on this list. Every driver, benchmark, proof and bring-up is three times the work. The tree's
own evidence says the multiplier is smaller than it sounds once the HAL is right, which is what
riscv64 demonstrated, but if the project ever needs to buy time, **dropping to two architectures is
the largest single lever available** and it should be a decision rather than a drift.

## The running order

Ranked by chance-of-fatal times cheapness-of-test, not by number.

| order | risk | experiment | owner | cost |
|---|---|---|---|---|
| ~~1~~ | 2, the proofs | **RUN 2026-08-30: amber.** No harness has ever caught a defect after the day it was written, because `cargo kani` never compiles the kernel | milestone 191 | done |
| 2 | 9, the HAL, on the board that already boots | the on-board test-suite exit, so silicon becomes gate-able rather than a human watching a console | milestone 16 | bench time, board proven since 2026-08-14 |
| 3 | 9, the HAL, on the architecture that carries the risk | a GRUB Multiboot or UEFI entry path, then the OptiPlex prints a byte | milestone 87 | a lane, then bench time |
| ~~4~~ | 1, the ecosystem | **RUN 2026-08-31: green.** Unmodified `ripgrep`, zero patches, runs and reaches its own argument parsing. The blocker is a missing argv, not threads | milestone 121 | done |
| 5 | 3, the tests | re-run the mutation sweep against the baseline | milestone 85 | a day, mostly waiting |
| 6 | 4, performance | the multi-tasking workload number | milestone 168 | one lane |
| 7 | 9 and 6 together | journey 3, end to end on three boards | journey 3 | months, and it is the capstone |
| -- | 5, multicore | the defect-discovery curve: a linear one is the red result | milestone 201 | weeks, hardware |
| ~~7~~ | 7, confinement | **RUN 2026-08-31.** 26 claims enumerated, 25 falsifications replaying red, and §31's headline assertion found unreachable in the case it exists to catch | milestone 202 | done; the adversarial half remains |
| -- | 8, nobody needs it | none. This is principle 1 | -- | -- |

## BUGS

- **Nothing gates this file.** No check compares it against the roadmap, so an entry can go stale the
  day a milestone lands, and a risk that was answered can sit here looking open. Risk 3 is already
  the worked example: it was on the list as unrun until someone checked and found a green number from
  three weeks earlier. Risk 9's cost line was the second, wrong on the day it was written: it priced
  milestone 87 as bench time when `notes/x86-port.md` already recorded that no real firmware speaks
  PVH. Both were caught by a person asking, which is rung zero, which is what this bullet is about.
- ~~**Two entries have no owner.**~~ Closed 2026-08-31: risks 5 and 7 are milestones 201 and 202,
  both scoped by calef, and both reframed in the process. Risk 5's experiment could not come back red
  as written and now can; risk 7's needed framing before a lane, and got §134's. **Neither can return
  a clean green**, and both blocks say so where a reader meets them.
- **The ranking is a judgement, not a calculation.** "Chance of fatal" is nobody's measurement, and
  two readers could order this differently on the same evidence.
- **A green result is not proof of anything.** Every experiment here can only fail to kill the
  project, which is the nature of falsification and worth saying out loud before a run comes back
  clean and gets quoted as a claim.
