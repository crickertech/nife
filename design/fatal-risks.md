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

**Status: RUN, 2026-08-31. GREEN on all three architectures since 2026-09-16, and the blocker is
not what anyone predicted.**
notes/ripgrep-on-nife.md has it; PR #600 for the first two, milestone 303 for x86_64.

- **Unmodified `ripgrep` 14.1.1 from crates.io, forty transitive crates, builds for
  `aarch64-unknown-nife`, `riscv64-unknown-nife` and `x86_64-unknown-nife` with zero source
  changes**, loads, runs, resolves its working directory through a granted directory capability, and
  exits through `std::process::exit`. **Zero patches**, and the three transcripts are byte for byte
  identical from three separately built binaries. Everything that differs from a Linux build is on
  the command line.
- **x86_64 took two more milestones and the second was a disk.** Milestone 184 (extend the `std` port
  to x86_64) made `std_exerciser` pass there on 2026-09-14 and `ripgrep` build, and **a build is not
  a transcript**: the run needed a RedoxFS disk the FS service could find, which `q35` could not
  offer because the lookup walked the virtio-mmio bus that machine does not have. Milestone 303
  (x86_64's FS service has a server and no disk it can find) closed that on 2026-09-16, and the
  transcript is the same 62 bytes. So the honest sentence is now *unmodified third-party software
  runs on nife*, with no architecture qualifier, which is what DECISIONS §19 (architectural parity is
  a tenet) asks before the qualifier comes off. `notes/ripgrep-on-nife.md` has the parity table.
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

**What it changes.** The structural fear behind this risk is retired **on every architecture this
kernel supports**: this system runs software it did not write, unmodified, with a real dependency
tree. What remains is an ABI gap with a name, which is a design question rather than a wall. The
parity gap that qualified this paragraph narrowed on 2026-09-14 from a missing port to a missing
disk, and closed on 2026-09-16 when the disk arrived.

**This qualifier was written on 2026-08-31 and did not land for two weeks.** calef caught the first
draft omitting the architecture the day the result came in; the correction was committed to a
maintainer branch that was never pushed, and survived only because a branch cleanup on 2026-09-14
checked for unique commits before deleting. By then half of it was stale (it said riscv64 was
unbuilt, and riscv64 had been run), which is why it was rewritten here rather than merged. AGENTS.md
already names the failure: *nobody reads branches.*

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

**Milestone 197 (`user/` and `xtask` are out of reach of the prover) closed the second half on
2026-08-31, and half-refuted its own premise.** `user/` was argued to deserve the prover more than
the kernel did, because it holds parsers over bytes this system did not produce. **It mostly does not
any more**: rule 7 and the host-testability discipline already lifted the initrd, ELF, GPT, mDNS,
directory-entry, terminal-escape, shell and glob parsers into crates, **every one already in
`script/verify`'s table**. The prize had been collected under other numbers, which is the tree
working as designed rather than a disappointment.

**It still found a live defect, and found it before any harness ran.** `rmle`'s save buffer was sized
for the document's text while `save` writes rows `\n`-joined, so a full document staged 3,231 bytes
into a 3,200-byte buffer and **panicked the editor on `^S`**. Eight months old and invisible to any
test that does not hit both limits at once. It came out of *writing the property down*, which was the
first time the two constants were compared, and the fix is a `const` assertion beside the buffer
rather than a harness: rung one, because the claim is a relationship between compile-time constants.

**And it measured a trap worth more than the proof.** Three properties were tried and abandoned with
numbers: a sum over 32 symbolic values (no answer in 20 minutes on two solvers), a symbolic index into
a 3.5 KB struct (CBMC out of memory in 3m23s), and any claim downstream of 20 divisions (no answer in
10 minutes). The last is the lesson: **the same harness asserting only a length bound returns in 0.3
seconds, because `--slice-formula` discards the divisions.** A fast harness can be evidence that the
assertion asked nothing, and nothing in this tree currently distinguishes those two cases.

**And the amber moved the same day.** Milestone 193 (put `kernel/src` within reach of the prover) was
minted from this finding and built hours later: two properties proved over `kernel/src/syscall.rs`
with nothing moved into a crate first, **both falsified before being believed** by re-introducing
milestone 142's real wrapping-multiply defect and watching them turn red. That is the counterfactual
this study said the tree did not have, and it now exists. It cost about 10 seconds of
`script/verify`. `kernel/src/arch/`, `user/` and `xtask` are still out of reach, so the amber stands;
what changed is that the reason is now a worklist rather than a wall.

**And on 2026-09-16 a proof caught a real kernel defect, on an architecture the prover had never
compiled.** Milestone 304 found that `cargo kani -p kernel` selects its `arch/` subtree by
`#[cfg(target_arch)]`, which under Kani is the **host**, so every CI job and the dev Mac had been
proving `arch/aarch64/` and nothing of the other two. The premise was measured rather than assumed:
two `assert!(false)` probes placed in `arch/riscv64/` and `arch/x86_64/` produced *"4 successfully
verified harnesses, 0 failures"*, because neither subtree was compiled.

The first proof ever pointed at `arch/x86_64/irq.rs` went red. `gsi_vector` is a flat
`GSI_VECTOR_BASE.wrapping_add(gsi)`, and `MAX_REDIRECTION_ENTRIES` is documented as the reason it
"cannot silently wrap onto an exception vector" while actually bounding the IO APIC's **entry
count**, not the GSI. A second IO APIC based at global interrupt 200 with the 24 entries every real
part has admits GSI 210, and `gsi_vector(210)` wraps onto **vector 2, the NMI**. Nothing has hit it
because every single-socket PC gives its one IO APIC `gsi_base` 0; a nonzero base is legal ACPI and
exists on multi-socket servers.

**That is a latent defect on hardware this project does not own, found by a model checker, which no
test in this tree could have reached.** It is the class this risk was written to ask about, and it
is the first time this tree has an instance of it. The survivorship caveat above still applies with
full force: the harness caught it *while being written*, like `dtb::be32` and `pci::intx_irq` before
it, so it is evidence that pointing the prover somewhere new pays, not yet evidence that a standing
proof catches regressions. **riscv64 remains unreachable to the prover and nobody here can change
that**: no GitHub image, no Kani cross-target flag, and CBMC needs a goto-binary for its own host.
The fix was deliberately not made in that lane, because it changes a public signature and a
documented policy; it was raised as a proposal with gate `DECISION`, calef chose to route by
redirection index on 2026-09-16, and it was built the same day as
[milestone 308](roadmap/308-route-gsi-by-index.md). **The fix does not add to this risk's
evidence and slightly complicates it**: the harness's `kani::assume(base == 0)` is gone, so a
standing proof now covers the case, but no machine here can execute the path, which
`kernel/src/arch/x86_64/irq.rs`'s module `BUGS` records where a reader meets the feature. A proof
that catches a regression on hardware nobody owns is still the honest shape of what this risk asks
about.

## 3. The tests do not test anything, and the quality is illusory

**The claim:** AGENTS.md's principle 2 says the method works because of the gates, the proofs and the
review discipline. If the suite would not notice the code being wrong, that sentence is decoration.

**Status: STALE, 2026-09-13, and a census now exists that this entry does not yet read.** Ruled by
calef: the headline this entry carried, *"MEASURED, and it came back green"*, was true of a run from
2026-08-03 and nothing has refreshed it since, so it read as a verdict where the evidence underneath
had become a history. The measurement itself is not in doubt and is kept below; what changed is that
this entry no longer presents it as current.

**The refresh arrived on 2026-09-14 and it is not what the entry below predicts.** The weekly
workflow completed for the first time, all eight shards, once milestone 277's memory bound stopped
the runaway mutant: **10,012 mutants over 64 crates, 91.7% of viable mutants killed**. Against the
38 crates the baseline covers, like for like, **93.6% against 92.4%**: the score went *up*.
`notes/mutation-testing.md` has the tables.

**So the "fall to 85.3%" was an artifact, and the entry below is kept as the account it is.** That
reading came from a one-eighth sample taken while two crates were being scored against suites that
could not run, and milestone 280 fixed both: `uefi_loader` now scores 100% and `documentation` 95.4%,
the two crates the drop had been blamed on. The 1.9-point gap between the like-for-like 93.6% and
the corpus 91.7% is the 26 crates that did not exist at baseline, which is a worklist rather than a
verdict.

**The verdict stays calef's and this entry is not marked settled.**
`design/roadmap/proposals/fatal-risk-3-against-the-new-number.md` is the proposal that owns the
re-read, gate `DECISION`, waiting since 2026-09-03; what changed is that it now has its number. Two
things a reader should weigh before that call, both of which a census shows and a sample cannot.
**Three of the baseline's five perfect crates lost their perfect score** (`memory_regions` 100% to
88.9%, `elf` 100% to 94.2%, `capability` 97.4% to 88.2%), which are regressions in properties that
used to hold. And the tree's worst crate on this measure is `timetable` at 73.6% with 48 survivors,
which is the crate holding `next_after`, the property risk 2 below names as its strongest
counterfactual.

`script/mutation` (milestone 85) ran 5,551 mutants over
38 host crates on 2026-08-03: 4,654 caught, 391 missed, 96 timed out, 410 unviable, which is **92.4%
of viable mutants killed**, with every survivor triaged into a test, an exclusion with a reason, or a
recorded gap. Five crates scored 100%.

**What that does not settle**, recorded because a green number is where inflation starts: the run is
from 2026-08-03 and the tree has grown since; it covers **host** crates only, so the kernel and the
arch trees, where risks 5 and 9 live, are not in it at all; and mutation testing measures the test
suite, not the code.

**The remaining experiment was cheap:** re-run it and compare against `.cargo/mutants-baseline.txt`.
No new milestone; milestone 85 already owned it, and it ran on 2026-09-14.

**Correction, 2026-09-11, and its second half closed three days later.** That paragraph used to close
"and the weekly workflow already publishes the report", and the workflow had published nothing.
`mutation.yml`'s own `BUGS` section records it: the workflow **had never once succeeded**, four
scheduled runs red from 2026-08-10, found by milestone 232's audit on 2026-09-03. Milestone 238
repaired one of the two causes (shard indices counted from one, so a job died in twenty seconds every
run and shard 0 was never tested). **The other was repaired by milestone 277 on 2026-09-12** (a
runaway mutant exhausting the runner's memory inside the timeout meant to catch it, which had taken
the 2026-09-07 run), and the next scheduled run, 2026-09-14, was the workflow's first success.
**The cadence is alive**; `script/cadence-check` is what reported it dead, and one success is not yet
a cadence.

**One number published in between, and it read worse: 83.4%, corrected to 85.3%.** It came from the
single shard that survived, a uniform one-eighth sample across all 60 crates rather than the 38 host
crates the 92.4% figure covers, so it was never a like-for-like reading. Two crates carried most of
the apparent fall and neither was explained at the time: `uefi_loader` at 15% and `manual` at 52%.
**Both turned out to be measurement rather than quality** (milestone 280, built 2026-09-13), as did a
third, `system_initializer`, before them (milestone 244). The census of 2026-09-14 above supersedes
this number; it is kept here because it is what this entry was ranked on for eleven days.

**Both repairs landed and the reading arrived.** The runaway mutant was milestone 277, built
2026-09-12, which made the clean full run possible for the first time since 2026-08-03; the run
happened two days later. What it means for this entry's verdict remains calef's:
`design/roadmap/proposals/fatal-risk-3-against-the-new-number.md` is the proposal waiting on it, and
this entry should not be marked settled again until that one is.

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

**Status: RUN, and as of 2026-09-16 all three of its parts are measured on silicon.** The two
2026-09-04 halves are below; the third was taken on 2026-09-16 and is the bullet that used to read
*unmeasured*. **This does not retire the risk**, and the reason is in the third bullet and repeated
at the foot of this entry: a TRNG is the smallest real device on the board, and throughput is what a
TRNG cannot test.

The risk names three things and they were never one claim. Measured on radon, transcripts at
`target/board/radon-2026-09-04-trng-success.log` and `bench/radon-2026-09-16/tour-083200.log`:

- **Confined: yes**, 2026-09-03. Milestone 159's driver is an EL0 process started from the archive,
  reaching the JH7110's TRNG through a capability that names no device. This is the tree's only
  confined driver for a real, non-virtio device.
- **Drives real hardware: yes**, 2026-09-04, reproducibly. `served 32+32 bytes`, two boots, first
  draws `3faa07e1` and `731191ba`, each boot's two draws differing from each other. Reseeded per
  boot rather than a constant in silicon or a stale register file.
- **At real speed: MEASURED on silicon, 2026-09-16.** **955,223 bytes/s**, 64 bytes in 67 us over
  eight `entropy_protocol` round trips, which is about **8.4 us per round trip**; bring-up 562 us.
  One boot of radon, transcript at `bench/radon-2026-09-16/tour-083200.log`. The instrument is the
  one built on 2026-09-10 (the tour reads the timebase around the step and the `hw entropy` line
  carries three figures), and this is the boot that proposal was waiting for.

  **The QEMU denominator did not survive contact with the board, and that is the finding.** It was
  built to say that the path itself costs about 250 us per 8-byte exchange with an emulated device
  that costs nothing, so that whatever radon spent beyond that would be the JH7110's. radon spends
  **8.4 us**, which is thirty times *less* than the floor it was supposed to be read against, and
  the bring-up is 562 us against QEMU's 8069 to 13057 us. So TCG was slower than silicon in both
  halves and the subtraction the denominator was for cannot be done. The proposal had already said
  to distrust the QEMU bring-up figure; the rate figure turns out to want the same warning.

  **What the number counts** is what the tour's own line says: the round trips, the context switches
  each one costs, the driver's poll loop, and the device. It does not count the spawn or the
  bring-up, and **it is not comparable to a Linux `hwrng` throughput figure**, which is a read from
  an already-running in-kernel driver with no IPC in it. The honest comparison, against
  `jh7110-trng.c` on the same silicon, is interrupt-driven where this driver polls and remains
  unmeasured.

**What it took is worth recording, because none of it was the driver.** Milestone 239 found the
device tree spells the node with the vendor U-Boot's `starfive,trng` rather than mainline's
`starfive,jh7110-trng`. Milestone 220 found the block's clocks gated and its reset asserted, and this
kernel had never programmed either. And milestone 159's own boot tour asked for 32 bytes down a
channel that carries 8, so its success line had been **unreachable on any device, working or dead,
since the day it was written** and survived because QEMU has no TRNG node to take the other arm.

**The remaining part is the one the risk is named for.** A driver that serves entropy slowly still
refutes nothing; the claim is about cost, and cost is what has not been measured.

**The decisive experiment:** one real, non-virtio device on real silicon, confined, at throughput.
The JH7110's GMAC (milestone 53) or NVMe behind milestone 163 (the JH7110's PCIe root complex) are
the candidates.

**Journey 3 settles most of this as a side effect**, because a framebuffer and a keyboard on real
hardware are real devices.

**Two corrections, 2026-09-03, from the §86 research lane.** The evidence line above says "All of it
is virtio or emulated", and the first half went stale on 2026-08-15: milestone 53 built a real,
non-virtio NVMe driver, confined by the IOMMU on all three architectures. Still emulated, so the
sentence's conclusion holds; its reason does not.

The second correction is the one that matters. **No IOMMU data point can settle this risk while the
NVMe driver is kernel-resident**, because the driver whose confinement the risk is about would be
the kernel. That is what DECISIONS §86 (whether an NVMe driver can leave the kernel, and what
capability would let it) decides, which puts §86 on this risk's critical path rather than beside it.
And the board this risk's experiment names has no IOMMU: milestone 143 (silicon IOMMU) exists
because no board shipping the ratified RISC-V IOMMU spec exists today. So a real-silicon NVMe
experiment on radon confines nothing unless something in software does, which §86's research pass
found is possible and had been ruled out on a false premise.

**The third correction, 2026-09-04, and it is a result rather than a correction.** On radon, a
userspace program holding two rendezvous capabilities and **one page of device memory** (no IRQ
capability, no DMA page, no `Virtio` capability) brought up the JH7110's TRNG and served a client
bytes that were not zero and that changed between draws, through a capability that names no device.
Transcript: `target/board/radon-2026-09-04-clock-and-first-entropy.log`, milestone 159.

That splits this risk into three, and two of them are now answered:

- **Confined**: yes, demonstrated on silicon.
- **Drive real hardware**: yes, demonstrated on silicon. A TRNG is a small device, and that is worth
  saying plainly rather than glossing: it has no DMA, no interrupt in this driver's path, and one
  register window. It is the *smallest* real device on the board, so it settles "a confined
  userspace process can reach non-virtio silicon at all" and it settles nothing about a device with
  a ring buffer.
- **At real speed**: **measured on silicon 2026-09-16, and the small-device question is closed.**
  955,223 bytes/s, 8.4 us per round trip, bring-up 562 us. The full figures and what they do and do
  not count are at the head of this entry. **Corrected 2026-09-11:** this used to read "nothing in
  the boot tour timestamps the step, so the only available clock is a person watching a serial
  console", and that stopped being true on 2026-09-10 when the step began timing itself; the boot
  that read it came five days after that.

The decisive experiment above is unchanged, because throughput is what a TRNG cannot test.

**And the machine for it exists, which nobody had established until 2026-09-05.** The decisive
experiment is one real non-virtio device on real silicon, confined, at throughput.
[§86](decisions/86-el0-nvme-driver.md) was decided on 2026-09-03 and its own research recorded that
**no board this project owns has an IOMMU in front of a real NVMe controller**. xenon has both, and
its firmware transcription says so precisely: a `Micron 2450 NVMe 256GB` on M.2 PCIe SSD-0 with SATA
in AHCI rather than RAID, *"so the NVMe is a plain PCIe function rather than hidden behind Intel
RST"*, on a machine milestone 87 selected partly for VT-d.

**What stood in the way was not hardware, it was that the disk held somebody else's Windows**, and a
disk this project must not write to is not a disk it can drive. calef confirmed on 2026-09-05 that
the installation is a freshly wiped image from the seller rather than anyone's data, and that the
machine's own firmware can clear it (Maintenance, Data Wipe, `Wipe on Next Boot`, which covers M.2
PCIe SSD; `notes/xenon-firmware.md`, IMG_4091).

So the remaining distance to this risk's decisive experiment is an EL0 NVMe driver under §86's
option 2a, and a bench evening. **That is a long way, and it is a known road rather than a missing
machine**, which is a different position from the one this entry was in a week ago.

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
broken. The adversarial exercise this entry originally called for is still unbuilt: an outsider
trying to escape, rather than us demonstrating that a planned escape fails. That wants outside eyes
and is gated behind milestone 198 by calef's no-third-parties position.

**The six kernel rows got their mechanism on 2026-09-16, and one of them was not testing its own
claim.** This entry said until that day that those rows had none; milestone 305 built it, on top of
milestone 210's `cargo xtask test --test <substring>` (built 2026-08-31, and the note claiming the
mechanism "does not exist" had been stale for sixteen days). Seven of the eight tests behind rows 21
to 26 now carry a replayable falsification. The sweep is **48 swept, 0 survivors, 2 min 55 s warm**.

**The finding is the one this risk exists to produce, and it is worse than a missing test.**
`the_page_tables_say_u_mode_cannot_read_the_kernels_memory` was patched to remove the `U`-bit check
from `mmu::user_can_read` **outright**, and the test still passed. `user_can_read` went through
`translate_user`, whose `Mapper` is built with `Half::Low` *always*, so a high-half kernel address
returned `None` before any leaf was read. **The assertion answered "U-mode cannot read the kernel"
by refusing to look**, and had done so since milestone 41, with every gate in this tree green
throughout. `is_mapped_in_current_space` exists forty lines away for exactly this case and says so
in its own doc comment. Fixed in 305 (`translate_in_either_half`) and measured both ways.

That is the shape of failure this file's rule 1 is about: **a test that cannot come back red is
indistinguishable from a test that passes**, and nothing but a falsification can tell them apart.
Risk 3's mutation census measures the same property over host crates and cannot see kernel tests at
all, so this class was invisible to every instrument the project owned.

**Two further things were recorded rather than smoothed over.** Row 26 (a client of a rendezvous
cannot become its server) is **`unfalsified`**, honestly: the complete break produces a 60-second
lost-wakeup watchdog rather than a claim-shaped red, because `RECV_CAP` blocks, so an attacker the
kernel fails to refuse takes the honest server's message instead of reporting an escape. Filling it
with an easier defect would have fired an assertion while leaving the claim untested, which is
precisely what the row above shows costs eight months. And **§31's assertion-order hazard has a
second independent instance**: row 24's quotable crossing assertion sits below two per-shell bitmap
equalities that catch any crossing one call earlier, so it cannot run. Two instances found the same
way in one sweep is a reason to expect more.

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
x86_64 has no real interactive boot entry point at all, and 166 and 167 are each a piece of the same
unfinished edge.

**Two of those closed, 2026-09-01 and 2026-09-02, and this entry did not notice for ten days.** It
used to cite milestone 164 as the reason x86_64 has no `fs_server`. That milestone is `BUILT`: the
whole of it turned out to be one build flag (`--cfg aes_force_soft`, which selects `aes`'s portable
software backend), and x86_64 userspace now builds `aes`, `redoxfs_server` and `mkfs`, with the last
two in the x86_64 archive. Milestone 165 (x86_64 PCI enumeration) is `BUILT` too. **The risk is not
weakened by that so much as re-sited**: what is left on this edge is the boot entry point and the
orchestrator, not the toolchain, which is a shorter list and a different kind of work.

**The third claim above is a premise rather than a status, so no gate will ever catch it.** This
entry still says milestone 177's text has x86_64 with no real interactive boot entry point at all,
and reads that as meaning the graphical stack is the only route to a shell. That does not follow.
`swish` never talks to a UART on any architecture; it talks to a console server over an endpoint, and
what actually blocks x86_64 is that §121 leaves no userspace holder for that endpoint. Whether a
kernel thread may answer there instead is `design/decisions/149-kernel-served-console-endpoint.md`,
`PROPOSED` since 2026-09-09. **If §149 is decided yes, 177 stops being a prerequisite** and milestone
182 reaches a shell over serial, which is also what a bench session needs. If it is decided no, the
sentence above stands as written. Either way this risk's decisive experiment below is unaffected,
because milestone 87 is about the boot entry and not about the shell.

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
| ~~4~~ | 1, the ecosystem | **RUN 2026-08-31: green on aarch64 and riscv64.** Unmodified `ripgrep`, zero patches, runs and reaches its own argument parsing. The blocker is a missing argv, not threads. x86_64 has `std` (milestone 184) and builds it; the run waits on a disk the FS service can find | milestone 121 | done for two ISAs |
| ~~5~~ | 3, the tests | **RUN 2026-09-14, the first census since the baseline.** 10,012 mutants, 64 crates, 91.7% killed; 93.6% against the baseline's own 38 crates, which is **up** from 92.4%. The fall to 85.3% was two crates scored against suites that could not run. **The verdict is calef's and is not yet given** | the proposal, gate `DECISION` | done; the re-read remains |
| 6 | 4, performance | the multi-tasking workload number | milestone 168 | one lane |
| 7 | 9 and 6 together | journey 3, end to end on three boards | journey 3 | months, and it is the capstone |
| -- | 5, multicore | the defect-discovery curve: a linear one is the red result | milestone 201 | weeks, hardware |
| ~~7~~ | 7, confinement | **RUN 2026-08-31, extended 2026-09-16.** 26 claims enumerated, 25 falsifications replaying red, and §31's headline assertion found unreachable in the case it exists to catch. Milestone 305 then gave the six kernel rows a mechanism and found **a confinement test that could not fail**: the U-bit check removed outright and the test still green, since milestone 41 | milestones 202, 305 | done; the adversarial half remains |
| -- | 8, nobody needs it | none. This is principle 1 | -- | -- |

## BUGS

- ~~**Nothing gates this file.**~~ Closed 2026-09-11 for the mechanical half by milestone 275:
  `script/fatal-risks --check` runs in `script/lint` and compares what this file says about a
  milestone or a decision against what the roadmap and the decision index record. It found four
  live disagreements on its first run, every one of them a case a person had already had to catch by
  asking: milestone 191 recorded `NOT-STARTED` while risk 2 and the running order both called its
  experiment run; risk 3 crediting a weekly report the workflow had never published; risk 6 calling
  the hw-entropy step untimed after the tour began timing it; and risk 9 citing milestone 164 as the
  reason x86_64 has no `fs_server` after 164 turned `BUILT`.

  **What is not closed is the larger half, and it stays named here rather than implied.** A gate can
  see a status word contradicting the record. It cannot see a premise being overtaken, which is what
  happened to this entry's second example: risk 9's cost line priced milestone 87 as bench time when
  `notes/x86-port.md` already recorded that no real firmware speaks PVH. Nothing mechanical would
  have caught that, and nothing here claims to. **A green `script/fatal-risks` means no status word
  in this file contradicts the record it names; it is not a warrant that the arguments still hold.**
- ~~**Two entries have no owner.**~~ Closed 2026-08-31: risks 5 and 7 are milestones 201 and 202,
  both scoped by calef, and both reframed in the process. Risk 5's experiment could not come back red
  as written and now can; risk 7's needed framing before a lane, and got §134's. **Neither can return
  a clean green**, and both blocks say so where a reader meets them.
- **The ranking is a judgement, not a calculation.** "Chance of fatal" is nobody's measurement, and
  two readers could order this differently on the same evidence.
- **A green result is not proof of anything.** Every experiment here can only fail to kill the
  project, which is the nature of falsification and worth saying out loud before a run comes back
  clean and gets quoted as a claim.
