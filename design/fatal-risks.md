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

**2026-09-19: milestone 64 (enough `std` to run somebody else's crate) turned BUILT, and it moves
this risk very little.** Its last pass bound file times by path (`Metadata::modified` on `GETMTIME`,
`std::fs::set_times` on `SETMTIME_AT`), which was the one item its block still called outstanding.
Nothing above depended on it: `ripgrep` stops at the missing argument vector (milestone 205), not at
`std::fs`. What it adds is one more std surface that answers rather than refuses, with a caveat a
stranger's program can trip over: a file written on nife reports an mtime in early 1970, because the
FS server stamps a per-mount counter (notes/std.md; proposed as
design/roadmap/497-a-filesystem-server-that-knows-the-time.md). Written by the milestone 64
lane, which does not normally edit this file; the status check requires the entry to know.

**The same published argument that sharpens risk 8 sharpens this one**, and it is the same paper:
Li et al.'s case for an incremental path rests on clean-slate kernels having *"significantly fewer
features than Linux ... impeding adoption"*, which is this entry's claim written by people who do
not work here. `notes/incremental-path.md` has it and the answer.

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

**Status: MEASURED, 2026-09-19. AMBER, and the ground shifted under it on 2026-09-20.** calef ruled
amber on the 2026-09-14 numbers, milestone 326 (nobody has been assigned to turn a mutation score
upward) triaged everything the amber half named, and he asked for a fresh census before deciding
whether it went green. **The verdict stands and the reason it was given does not**, which is the
useful part.

**The fall this entry was built on did not happen.** Milestone 518 (a census that cannot be attributed) captured both censuses into a committed per-crate record and recomputed them
consistently. Like-for-like reads **94.7%**, not 92.6%; the whole corpus **93.7%**, not 91.4%. The
runs themselves are unchanged: run
[35421192143](https://github.com/crickertech/nife/actions/runs/35421192143), eight shards, all green.

| | crates | viable | killed |
|---|---|---|---|
| baseline, 2026-08-03 | 38 | 5,141 | 92.4% |
| census, 2026-09-14 | 64 | 9,277 | 91.7% |
| **census, 2026-09-19, read the same way** | **62** | **8,925** | **93.7%** |
| like-for-like, 2026-09-14 | 38 | 6,552 | 93.6% |
| **like-for-like, 2026-09-19, read the same way** | **38** | **6,604** | **94.7%** |

**Two joins, each worth about a point.** The published rows disagree about what a timeout is: the
baseline and the 2026-09-14 figures follow `notes/mutation-testing.md`'s rule that a timeout is a
kill, while the 2026-09-19 figures scored that run's 205 timeouts as survivors. And `cred` was
renamed to `credentialer`, so the join dropped it: 103 viable mutants at 100%. Reproducing the
published 37 crates and 6,472 viable requires resolving two crates by hand and missing that one.

**The direction reverses under either definition read consistently**, timeouts-as-survivors giving
89.5% then 91.4%, and survivors across the corpus fell 771 to 563.

**Verified rather than taken on a lane's word.** The maintainer recomputed the corpus and
like-for-like rates from 518's committed record on 2026-09-20 and reproduces the reversal and both
causes; the 94.7% figure is 518's, since reproducing it needs that milestone's rename mapping, and an
unresolved intersection lands just under it at 94.2%. The published 92.6% is reproducible **exactly**
as a survivor-basis number, against a 93.6% that is kill-basis, which is the mixing stated above seen
from the other side.

**It stays amber, on the standard this entry actually holds.** 563 survivors are untriaged against
milestone 85 (mutation testing over the host crates)'s rule that every
survivor becomes a test, an exclusion carrying its reason, or a recorded gap. **That was the honest
ground all along.** The fall was never needed to reach amber, and leaning on it meant this entry
asserted a cause it could not attribute, which its own next paragraph admitted in the same breath.

**What green now requires, ruled 2026-09-20 and measured 2026-09-21.** calef ruled that the
condition should be **inflow**: new code cannot arrive less tested, with the corpus rate as a lagging
indicator. Milestone 517 (what fraction of survivor growth arrives on lines a pull request touched)
then measured the premise rather than assuming it, and the answer is decisive. Of the census's 771
survivors, **629 sit on lines a pull request wrote and one** is a genuine regression on a line nobody
edited (`compositor`'s `Rect::area`, caught in August, surviving in September). Old-code decay runs
about three orders of magnitude behind inflow.

> **Green when both hold.** (a) **Inflow:** the survivors a merged pull request adds on its own
> lines, measured by `cargo mutants --in-diff` on the merged diff, are zero or triaged into a test,
> an exclusion with a reason, or a recorded gap, under milestone 85 (mutation testing over the
> host crates)'s rule, for every pull request since the last census.
> (b) **Trailing:** the like-for-like census rate has not fallen between the two most recent
> censuses. **Amber if (a) holds and (b) does not**, because that is coverage decaying on code
> nobody is editing, which is a different defect wanting a different repair.

**Three things about that wording are deliberate.** It does **not** say "the blocking gate is on",
because that would make this verdict hostage to a decision milestone 479 (a blocking `--in-diff`
mutation gate) refused; the measurement is available without the gate, at four seconds on a
documentation change. It keeps clause (b) even though inflow dominates, because an inflow-only
condition cannot see `Rect::area` at all and that failure is silent by construction: nobody is
editing the code, so nothing prompts anyone to look. And it is **a floor rather than a target**,
since a percentage target can be met by excluding awkward crates, which is what milestone 85's rule
exists instead of.

**What it does not carry, stated where the verdict is read.** The kernel. `kernel` generates 7,529
mutants and `components` 3,482, and a kernel mutant costs a relink plus a full QEMU suite, about 55
seconds each: roughly 500 runner-hours per census across three architectures against 52 minutes
today. A kernel *census* is refused on that arithmetic; a kernel *diff-scoped* check is minutes on a
kernel pull request and is the affordable half. **So this condition speaks for the host-testable
corpus and not for the kernel**, which is the largest thing it does not say.

**The cost of adopting it, measured rather than estimated**: 62 of the 761 pull requests merged in
the six-week window (8.1%) would have carried untriaged survivors, median 6 each.

**What this cost, recorded because it is the second time.** Milestone 512 (the census blamed one pull
request for 55 survivors it did not write) holds the first: a delta read from `script/mutation
--report`'s `(baseline missed)` column, which diffs against `.cargo/mutants-baseline.txt` from
2026-08-03 rather than against the previous census, so six weeks of growth read as two days of
regression. Both errors are one shape, **a comparison across two records that were never made
comparable**, and both were available because the per-crate numbers were never written down. They are
now, which is what made this correction possible at all.

**2026-09-23: milestone 512, built, names the crate the "second time" paragraph above only pointed
at.** `machine_discovery` read 22 survivors in the fixed 2026-08-03 baseline and 77 in the census
that ran on 2026-09-19, two days after milestone 319 (the crate that parses firmware) landed, which
is what made "two days of regression" look plausible. Replaying `cargo mutants --in-diff` against
milestone 319's own pull request finds 4 survivors, and the crate already carried 73 on the commit
immediately before that pull request merged: 73 + 4 is 77, the census's own number, to the unit. The
other 73 predate the pull request; they accumulated while the crate grew from 212 mutants at the
baseline to 693 at the census, over six weeks the `(baseline missed)` column cannot see, because it
always diffs against 2026-08-03 rather than against the census before it. `script/mutation`'s own
comments and `notes/mutation-testing.md`'s `Scope and honest caveats` section now name that trap
where a reader of `--report` meets it.

**One convention is now load-bearing and unchecked.** Whether a timeout counts as a kill moves this
entry by about two points, and the rule rests on a hand-check of the baseline's 96 timeouts six weeks
ago. There are 205 today and none of them has been checked.

**The first amber half: seven crates regressed, and three of the baseline's five perfect crates lost
that score.** `memory_regions` 100% to 88.9%, `elf` 100% to 94.2%, `capability` 97.4% to 88.2%, with
`clock_protocol`, `swish`, `dtb` and `filesystem_protocol` behind them. Each is a property that used
to hold and no longer does, which is a different object from a crate that was never covered. A
sample cannot surface these at all, because it cannot tell an absent mutant from a killed one, so
this is the first time in the project's life that the question has been askable.

**The second: new code arrives less tested than old code, and nothing pulls it up.** The 1.9-point
gap between 93.6% and 91.7% is exactly the 26 crates that did not exist at baseline, and the eight
worst crates in the tree are all of them new, led by `work_steal_slot` at 54.2% and `timetable` at
73.6% with 48 survivors. `timetable` holds `next_after`, the property risk 2 below names as its
strongest counterfactual, which makes it the single survivor set most worth a person's afternoon.

**Why amber rather than green, which is the part worth arguing with.** 91.7% over a full census is a
good number and a green verdict would be defensible on it. It is refused because this file's job is
to be the place a green number cannot hide in: milestone 85 (mutation testing over the host crates)'s own rule is that **every survivor is
triaged into a test, an exclusion with a reason, or a recorded gap**, and that was done for the
baseline's 391 survivors and has not been done for the census's **771**. A verdict of green would
claim the discipline held when what held was the instrument.

**And why not provisional, which was the other option.** Declining a verdict until the 771 are
triaged would postpone the reading on the grounds that the evidence is good enough to want more of
it. The census *is* the experiment this entry has been waiting for; it is read here, and what it
found is recorded as owed work rather than as a reason not to read it.

**What is owed, and it now has a block rather than a sentence.** The 771 survivors of the 2026-09-14
census are untriaged, the seven regressions above have no owner, and the workflow has succeeded
exactly once, so a cadence is claimed by one data point. Nothing in `design/roadmap/` owned any of it
when this verdict was written: milestone 85 built the instrument and triaged the baseline, 238
repaired the workflow's shard indices, 277 bounded the runaway mutant, and 280 explained the two
crates that made the fall look real, so three of the four were repairs to the instrument and only one
ever turned a score. **Milestone 326 was minted the same day for exactly that gap**, ordered
`capability` first among the regressions and `timetable` next for its 48 survivors, and its own
definition of done is milestone 85's rule rather than a target percentage, because a percentage
target can be met by excluding the awkward crates.

**What would move this entry now, stated better than the condition it replaces.** Not a finished
worklist, and not a single number either. **Two consecutive censuses where the like-for-like rate
does not fall**, which is the smallest claim that distinguishes a suite keeping up from a triage pass
that happened recently. One census is a point; two is a direction, and the direction is what this
entry is about. The instrument now runs weekly and has completed twice, so this costs waiting rather
than work.

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
happened two days later, and calef read it on 2026-09-19. The proposal that had owned the re-read
since 2026-09-03 was drained with the ruling, which is what a proposal is for.

**The proposal asked the wrong question, and that is worth keeping.** It was written against a fall
from 92.4% to 83.4% and offered three options about how bad the fall was. By the time it was read the
fall had been corrected twice, first to 85.3% and then out of existence, so none of its three options
described the tree. The reading above is against the census instead. It is the fifth proposal in two
days whose premise expired between filing and reading; milestone 323 carries the argument that
promotion, not filing, is where that is cheapest to catch.

## 4. The architecture imposes a per-crossing cost that cannot be engineered away

**The claim, and calef named this one first:** a capability microkernel pays on every boundary
crossing, and on workloads that cross constantly the cost is architectural rather than a matter of
tuning.

**Status: NOT YET, 2026-09-23. No verdict, and one bench evening stands between here and one.**
This is the best-covered risk on the list by volume of measurement and it still has no answer,
because everything measured so far is a single crossing and the claim is about a cost that cannot
be **amortised**. Amortisation is a property of a workload. The instrument that produces a workload
number is built, gated and rehearsed on three architectures, and no sweep from its current form has
ever run on silicon.

**What is measured, and it is a lot.**

- **Single crossings, against Linux and macOS on one core.** `notes/benchmarks.md`'s release table
  puts nife and Linux on the same M-series core at the same HVF tier, with native macOS as the
  bare-metal ceiling: null syscall ~27 ns against Linux's ~139, IPC round trip ~337 ns against
  ~1,723, a derived context switch ~28 ns against ~415, and spawn ~7.7 us against `fork`+`exit`'s
  ~19.7 us. Provisioning a page is a **three-way tie** near ~550 ns, which the note keeps out of the
  win column because zeroing 4 KiB is bandwidth-bound and identical on all three. Four wins and one
  tie, each with its caveat stated where the number is: the tie is zeroing, the switch is derived by
  subtraction from two different mechanisms, and spawn builds a lighter object than `fork`
  duplicates.
- **A committed floor per crossing, in guest instructions.** `bench/baseline-aarch64.txt`:
  `ipc_rtt` 1,032 ticks, `call_reply` 1,059, `relay_rtt` 2,058, `ctx_switch` 612, `spawn_reap`
  3,383, `null_syscall` 20.5. Deterministic and comparable across runs, gated at a coarse 10%.
- **A bounded footprint for the crossing itself**, which is the mechanism by which a per-crossing
  cost would fail to amortise in the first place. `bench/fastpath-aarch64.txt`, written by
  `script/fastpath-footprint`: `ipc_call_reply` 7,028 bytes plus `syscall_entry` 1,508. The first
  three phases of milestone 188 (the IPC fastpath) took entry from 3,304 to 1,508 and
  `ipc_send_recv` from 5,888 to 5,356, and found that the cheap extraction method of
  milestone 156 (extract the rest and ratchet both ways) does not transfer to a closure walk, which
  is a result rather than a shortfall.
- **One real path closed against itself.** Milestone 138 (close the read gap) measured 16x against
  where it started, including 5.67x on a read and 8.02x on a write from one wire change, and
  `call_reply`'s steady state is measured with the live-replacement mechanism costing zero in it,
  which is DECISIONS §41 (the endpoint is the broker).

**Why none of that is a verdict, stated once so it is not re-litigated.** Every figure above is a
single operation or a static size. Milestone 25 (cross-OS performance comparison)'s own block says
its suite is entirely single-operation primitives, *"the same shape DECISIONS §96 (process kernel
or event kernel) means by micro-benchmark"*, and milestone 168 (a multi-tasking workload benchmark)
exists because milestone 25 has that hole. A per-crossing cost a workload can absorb and one it
cannot look identical in these numbers.

**The shape has been measured once on silicon and is not quotable.** Five radon boots on 2026-09-16:
throughput rose to about four tasks and then **plateaued through 32 without declining**, on every
boot, at 8x oversubscription, with `tasks=1` repeating to 0.0% across five cold power cycles. That
is the shape a defence of this risk wants. It does not get to be one, for two reasons milestone 168
records against itself. `tasks=4` spread 29.4% between boots and 37.3% within one, so a
best-of-three there is a coin flip rather than a number. And the mix contained no page mapping and
no process creation, which are the two jobs that go deepest into the kernel. The instrument was
rewritten on 2026-09-19 (every point the median of 21 repeats, `map` and `spawn` jobs added), and
**no sweep from it has run on a board**, so every stability claim about the current instrument is a
resampling of the old one's data.

**The decisive experiment has not been run, and it is now a bench evening rather than a lane:**
milestone 168, one radon evening with the 2026-09-19 instrument, at least five boots, by
`notes/job-mix.md`'s procedure. Its step 7 wrote down what each outcome means **before** the numbers
exist, which is what keeps the reading from being a defence afterwards: flat or rising through 32
with every point inside 10% across boots reads as no architectural per-crossing cost visible at this
scale on this silicon; a repeatable knee followed by a **decline** says a cost exists and grows with
load, with the `job-mix-kind:` lines naming which path pays; anything still wider than 10% is not a
verdict and the spread gets recorded instead. Milestone 188 (the IPC fastpath) is the follow-on if
the path that pays is `round_trip`.

**A cheaper cross-check became available on 2026-09-19 and nobody has taken it.** calef asked that
day for the sweep under HVF on patagonia's own cores, as a cross-check on the shape and never as a
result. QEMU refused it five times because `kernel/src/drivers/gic.rs` was GICv2 only.
Milestone 227 (a GICv3 driver) turned BUILT the same day, and its own block records that
`script/job-mix --hvf` was not yet on `main` when that lane ran, so the cross-check still has not
happened although its blocker is gone. It costs no board.

**What was refused here, because the refusals are half of what makes the rest readable.**

- **Every x86 `ns/iter` in this tree.** The 2026-08-24 table is marked suspect where it stands: the
  boot calibrated the TSC from one 10 ms PIT window, a poll can only notice the terminal count late,
  and 200 boots put the worst error at **+1153%**, always high. Milestone 571 (the x86 boot
  calibrates the TSC once) fixed the estimator; it did not make the published figures quotable.
- **Any nanosecond read off an `-icount` leg.** Under `-icount shift=0,sleep=off` a guest nanosecond
  is a function of the instruction stream rather than of real time, and the implied rate moved 37%
  between two workloads inside one boot (`notes/tsc-under-tcg.md`).
- **`sel4bench`.** It builds and boots and has never produced a number, because it times one
  operation through `PMCCNTR_EL0` and neither TCG nor HVF provides one. **This is the comparison the
  risk most wants and does not have**: the cross-OS table's peer is Linux, not the state of the art
  in minimal kernels. argon has been in hand since 2026-09-01, milestone 74 (cycle counters) is
  what this side of the table needs to answer it, and nobody has run either half.
- **The job mix's TCG rehearsals**, which milestone 168 already refuses to record as results, since
  TCG models no cache.

**What even a green answer would not cover**, put here rather than after the run. Nothing in the mix
touches a disk, which is milestone 493 (a disk-file job mix needs a disk)'s territory, so a flat
curve is evidence about compute, memory, trap, scheduling, IPC, mapping and process creation, and
silent about the filesystem. The mix proportions are chosen rather than derived from an AIM7
workfile, so no figure from it is quotable without naming the mix. And a flat curve on four usable harts says nothing about
a machine with thirty-two.

**The counter-thesis is published, and it is more specific than "microkernels are slow."** Two papers
argue that the per-crossing cost is avoidable by removing the crossing: *The Case for Writing a
Kernel in Rust* (Levy, Campbell, Ghena, Pannuto, Dutta, Levis, APSys '17, DOI
10.1145/3124680.3124717), which proposes to *"entirely throw away hardware protection within the
kernel and, instead, write the kernel in a memory-safe programming language"*, and RedLeaf (OSDI
'20), which builds a whole system on that bet. **If they are right, a capability crossing is a cost
this project chose rather than inherited**, and that is precisely what this risk says would be
fatal.

**Their own stated limits are the strongest thing in this entry's favour, and they are quoted rather
than asserted.** The 2017 paper evaluates Rust *"in a single-threaded setting"* on low-power
uniprocessors, leaves on-disk and in-hardware structures *"e.g. the page table"* to future work, and
says of information-flow control that *"it is not yet clear if such implementations would be
sound."* **Their open problem is risk 5 below**, which is this project's own hardest entry, so
neither side gets to treat multicore as settled.

**What this obliges when milestone 168's number arrives.** It will be read against theirs by anyone
who knows the literature, and a comparison is only honest if it says what differs: a language-
isolated crossing trusts the compiler and the absence of `unsafe` in the isolated code, where a
capability crossing trusts the hardware and a kernel small enough to prove things about. Those are
different guarantees at different prices, and the number alone does not say which was bought. This
entry carries the obligation; the reading of both papers is a lane's, and its note will be cited
here once it lands rather than promised from here.

**Ranked fourth on purpose.** This is where a skeptic expects the project to die and it is where the
project has the most evidence that it will not. **That evidence is the wrong shape**, which is this
entry's whole finding: a great deal of it, all of it about one crossing at a time, and the claim is
about many.

## 5. It cannot be made reliable on multicore, and the bugs appear only on silicon

**The claim:** the concurrency is wrong in ways that QEMU cannot show and that arrive one at a time,
forever.

**Status: UNRUN, 2026-09-23. No verdict, and the reason no verdict is available is itself the
finding.** `UNRUN` is a provisional word: this file's vocabulary is `RUN`, `MEASURED` and `AUDITED`,
and none of the three fits an experiment that has not happened. Until this date the entry carried no
status at all and cited exactly one milestone where the other eight cite between four and twelve,
and `script/fatal-risks`' own report is where that showed up. What the sweep behind this revision
found is that the single citation was the smaller half of the problem: **the sentence this entry
opened with had been retracted in the tree two weeks before this file was written.**

### The one failure on record is not a failure, and this tree overturned it itself

This entry used to say the risk had already fired, and that the VisionFive 2 had produced a receiver
woken with nothing delivered on three harts that no emulator run had ever shown. That is
`notes/visionfive2.md`'s **fourth** bench stop (boots 7 and 8, 2026-08-14), and the same note's
**fifth** bench stop overturned it on **2026-08-15**: the dumps of boots 7 through 9 are the terminal
state of a *completed* tour. The parked receivers are the UART demo's driver and its byte receiver,
the wake was the worker's real send, and `svc=20` is the choreography's exact ecall total, each
identified from the tree rather than from the dump. `notes/scheduler.md` states the consequence
plainly: "So the gate has never fired on a field failure, and `refuse:` has never appeared on a board
ring."

The `wake_load_aware` gate and the pop-own-current guard stay, and they earn their keep on their own
merits: the transition they forbid really would complete a rendezvous off a stale mailbox, they are
proven red-then-green by injection tests that drive the real wake path, and
`crates/thread_wake_handshake` models the protocol under loom. What is gone is the field failure they were built against.

**So the honest statement of this risk's evidence is that it has never fired**, and the correction
propagated badly. This file was written on 2026-08-30, milestone 201 (is multicore reliability
converging) was minted on 2026-08-31, and milestone 225 (run the soak on radon, argon and xenon) on
2026-09-02. All three repeat the retracted reading, and 201 additionally lists it as the first of the
"first three data points" its curve is to be seeded from. **Those three data points need re-deriving
before that milestone starts**: the first is retracted, and the other two are `ap_boot`'s open bugs
in milestone 161 (the x86_64 kernel port), which is `BUILT` since 2026-09-19 and with
milestone 316 (making `NIFE_SMP=2` mean something on x86_64) having fixed the boot-core-identity
defect at its root.

### Every multicore defect this project has found was found without silicon

That cuts against the claim's second clause, and it is the strongest evidence this entry has:

- **A release fence with no partner in the clock page's seqlock**, found by loom on 2026-08-04, in
  milestone 80 (the hand-rolled atomic protocols, model-checked). A reader could revalidate
  and return a state from one publish beside an offset from another. Milestone 116 (the fences with
  no partner) is the sweep it triggered, and the same mistake was found the same day by a hand audit
  that shared nothing with the harness.
- **A double free on the untyped region claim**, found by loom and gated by a falsification witness
  that passes only when the pre-fix protocol still double-frees, in
  milestone 135 (the region claim, under loom).
- **A boot-core-identity defect that failed `every_secondary_runs_scheduled_work` about half the time
  at two cores** on x86_64, found and fixed under QEMU by
  milestone 316 (which core booted).
- **A port revocation that did not reach every core**, found the same way and **fixed on 2026-09-23**
  by milestone 315 (a port revoke that reaches every core). `PortRange::REVOKE` cleared the
  capability under `IPC_TABLES` but reset the TSS I/O bitmap on the **revoker's core only**, leaving
  a window until every other core's next switch. Diagnosed from evidence rather than argument: a
  snapshot at the revoke read "revoker on cpu 0, cpu 1 holds a grant" on both captured failures,
  while 27 passing runs had no grant installed elsewhere. The fix resets this core and rides the TLB
  shootdown's NMI to the rest, and moves `install_port_grant` inside the locked region so a core
  cannot reinstall a grant the sweep just cleared. Proved by 12 of 12 full two-core suites green,
  36 boots. **Found under QEMU, not on silicon**, which is the fourth such case and bears on this
  risk's premise below.
- **A test that hung in two of three full HVF runs** and passes in every TCG run and when run alone
  (`notes/hvf-leg.md`), un-diagnosed.

None of these needed a board. That does not refute the claim, because a defect emulation can find is
not evidence about the class it cannot, but it does retire the version of this entry that treated
silicon as the only productive instrument. **The instruments that have actually produced multicore
defects here are loom and a two-core QEMU, and both are cheap.**

### What the emulator cannot show, stated precisely rather than as a worry

- **TCG's memory model is far stronger than real silicon's.** Guest accesses execute in the host's
  program order and MTTCG serialises cross-vCPU visibility through host atomics
  (`notes/visionfive2.md` says so before the runs rather than after). So an acquire that should have
  been an acquire-release passes `script/test`, `script/cpu-matrix` and every CI leg.
- **Loom models C11, not ARM and not RISC-V**, which `script/interleaving-check`'s own header states.
  It narrows the gap rather than closing it, over five extracted protocols, not the kernel.
- **The one real-silicon leg is aarch64 only and never runs in hosted CI.** `script/ci-build`'s `hvf`
  row runs the kernel suite at guest EL1 on four physical Apple cores, which is
  milestone 81 (an HVF leg: the test suite on the physical core), and GitHub's macOS arm64 runners
  are themselves virtual machines
  with no nested virtualization, so it exists on one laptop. It samples real orderings; it does not
  search them.
- **riscv64 and x86_64 have no real-silicon leg at all.** For those two architectures the whole of
  this risk is reachable only on a bench boot.
- **x86_64 had no multicore coverage by default anywhere**, and that changed on 2026-09-23 when
  milestone 315 (a port revoke that reaches every core) landed and **`NIFE_SMP` defaulted to 2**, per
  DECISIONS §153 (how a two-core x86_64 test earns its place). `script/soak-test --arch x86_64` and
  its `remote=0` report want re-checking against that default; this entry will be wrong again if they
  were not updated with it.

So the reachable fraction of this risk under CI is: five hand-extracted protocols under a C11 model,
plus whatever a round-robin TCG interleaving happens to expose on two or four emulated cores. That is
not nothing, and it is not the class the claim names.

### The two load-sensitive reds of 2026-09-22, and which way they cut

`live_swap_tests` (pull request #1101) and `current_cpu_tests` (#1120) were both chased to test
defects rather than kernel defects: each asserted on `memory::free_page_frames()`, a count of every
free frame in the machine, which any neighbouring test's teardown can move.
`notes/load-sensitive-assertions.md` has both, and its own diagnostic sorted them correctly before
either was opened, on direction alone: a slow machine produces a deficit, never a surplus, so
"returned 277 of 224 pages" was never a timeout.

**It cuts in the suite's favour in one specific way and against it in another, and both are worth
saying.** In its favour: each was closed by *narrowing* the assertion rather than by widening a
bound, which is the move milestone 62 (tests that assert on time) forbids by name, and a narrower
assertion is the stronger one, since a global delta of the right size can be reached by the wrong
frames coming back and a scoped one cannot. Against it: **a suite whose every load-sensitive red so
far has resolved to a test bug has never once caught the thing this risk names**, and that is equally
consistent with a healthy kernel and with an instrument that cannot see. The note itself refuses to
close the second one, in its own words, that this is not explained by load and may be a real bug, and
it was still un-investigated when the fix landed.

**And a third reading is the one worth acting on, because it is cheap.** The defect class is not two
sites. `memory::free_page_frames()` is read at 46 places in `kernel/src`, across
`kernel/src/user/force_kill_tests.rs`, `kernel/src/user/cpu_time_tests.rs`,
`kernel/src/user/tests.rs` and `kernel/src/testing.rs`, and many of those reads are the same
before-and-after pair that produced both of these reds. One of them already carries a comment naming
the hazard. So the same failure will recur on a different line, and every recurrence costs a lane an
afternoon deciding whether it is the kernel. A sweep of those sites is worth a milestone of its own,
and it is worth more to this risk than it looks: until a load-sensitive red can be trusted to mean
something, this suite cannot be the instrument that this entry's experiment needs.

**The decisive experiment, which has not been run:** milestone 225 (run the soak on radon, argon and
xenon), which is `NOT-STARTED` and gated `HARDWARE`. Everything it needs exists and none of it did on
2026-09-01: a workload that lasts, which is
milestone 219 (the boot tour ends and the kernel halts, so there is nothing to soak); a hook that
makes it cross cores, which is milestone 221 (the soak never crosses cores, so build the hook that
makes it); a console that watches and judges, which is milestone 216 (nothing in this tree can read a
board); and a boot that needs nobody typing, which is
milestone 218 (every boot of the VisionFive 2 needs a human typing four commands into U-Boot).
`script/soak-test` is its rehearsal under emulation, and it says on every green run that a clean run
is a number and not a verdict.
Milestone 201 (is multicore reliability converging) is the reframe that makes a red result possible at
all, and the running order carries it rather than 225.

### What would render a verdict, in the order it should be bought

1. **A bench evening on radon with milestone 225's procedure followed**, which means reading the
   first heartbeat before walking away: `wakerate` about `100 * harts`, and `crossings` **rising**
   between beats. Eight hours of a non-crossing soak is milestone 219's experiment wearing 221's
   name, and the difference is invisible afterwards.
2. **Milestone 315 landed and `NIFE_SMP` defaulted to 2**, which is the cheapest item here and the
   one that converts x86_64 from no evidence to some. It is a lane, not a bench evening.
3. **Milestone 201's three seed data points re-derived from the record**, since one is retracted and
   two have moved. A curve seeded from a retracted defect is worse than an unseeded one.
4. **A stated duration.** Neither 201 nor 225 prescribes one, both because nobody knows what would
   persuade, and a number chosen after the run is not a number.
5. **argon**, which has never booted nife at all and sits behind milestone 127 (the seL4 machine).

**What none of that would return is a green.** 201's own framing is the honest one: a flattening
defect-discovery curve is a confidence, a linear one is the red result, and there is no experiment on
this list that can come back saying the concurrency is correct.

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

**And that machine now boots nife, as of 2026-09-17** (milestone 87, `BUILT`), which removes the
last thing standing between this risk and its decisive experiment that was not code. Its first
complete boot reported `iommu : VT-d drhd at 0x00000000fed90000, root table default-deny,
translating` and `pci : 15 function(s) on the bus`, so the IOMMU this experiment needs came up on
its own hardware rather than under emulation.

**What stood in the way was not hardware, it was that the disk held somebody else's Windows**, and a
disk this project must not write to is not a disk it can drive. calef confirmed on 2026-09-05 that
the installation is a freshly wiped image from the seller rather than anyone's data, and that the
machine's own firmware can clear it (Maintenance, Data Wipe, `Wipe on Next Boot`, which covers M.2
PCIe SSD; `notes/xenon-firmware.md`, IMG_4091).

calef ran that wipe on 2026-09-17, so the disk is this project's to write to.

**The driver exists too, as of 2026-09-17** (milestone 261, §86's option 2a). An EL0 process holding
two endpoints, one page of BAR0 and a run of DMA pages brings a controller from reset through
identify to an I/O queue pair and serves the block verbs, with the IOMMU the whole of what stops it
reaching memory it was not given, and `kernel/src/user/non_volatile_memory_express_tests.rs` asserts that confinement on
every leg the runner attaches a controller to. Milestone 318 then rewrote its assertions against the
geometry each boot is handed, which is what lets the same case run on xenon's Micron rather than
only on `mknvmedisk`'s 8 MiB image.

**So the remaining distance to this risk's decisive experiment is a bench evening, and nothing
else.** Every piece is on `main` and the stick is written. Two things still have to be true on the
night, and neither is code: the DMAR's device scope must cover the NVMe function, because a
throughput number from an unconfined device answers a different question; and the controller's LBA
size must give `blocks_per` in `1..=8`, or the server never starts and the line reads `skipped`
rather than `ok`. **A skip is not a pass.**

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

**Status: AUDITED, 2026-09-17, and the answer is a qualified yes with one exception found and
fixed.** Milestone 313 read this risk's question adversarially under the userspace-confinement lens,
the first security audit since 2026-08-17. `design/audit-reports/2026-09-17-userspace-confinement.md`
has it; findings fixed 3, minted 3, accepted 1.

**One published claim was false as stated, on a path taken every boot.** DECISIONS §12 says *a
consumed capability cannot be used again*. On `x86_64` it was not: `SYS_CAP_DELETE` cleared the
capability table and **not** the cached grant the context switch installs into the TSS I/O bitmap, so
a thread that dropped its `PortRange` kept COM1 for the rest of its life. `system_initializer`
performs exactly that delete on every x86 boot. Fixed in `sched::delete_current_cap`, with a test and
a falsification replayed red.

**A second published sentence about the hardware was false and is now true.** `crates/paging`'s
decoder reports user pages as not kernel-executable, and milestone 307 wrote that the hardware makes
it so; on x86 that holds only with `CR4.SMEP`, which nothing set. The bit is now set per core where
CPUID offers it, and 307's sentence is struck through with the correction beside it rather than
edited away.

**The headline claim was not found false anywhere this audit looked**, and what it looked at is
stated rather than implied: components that took device or network authority since the last audit,
which reading every capability mint site shrank from 45 counted components to **two objects**; the
six claims milestone 307 marked quotable-but-unreachable, none of which turned out weaker than its
claim; and the two new machine classes, radon and xenon.

**Three things this does not settle.** The syscall surface and IPC model are the remaining untaken
lens and want their own audit. The adversarial half this entry has always called for, an outsider
trying to escape rather than us demonstrating a planned escape fails, is now **partly built** and
still gated behind milestone 198 (the trivial install that makes a second customer possible). An adversarial pass on 2026-09-21 asked where authority lives
outside a cspace and **found a claim false**: every revocation sweep walked a thread's capability
table and stopped, so a capability parked in `Thread::outgoing_cap` (the hand-off slot a `SEND_CAP`
writes when no receiver waits) survived the sweep and was delivered afterwards. `MemoryRegion::DESTROY`
sweeps capabilities precisely so that no capability still names a page the allocator is about to hand
out, and an in-flight one reopened that. Fixed in the three sweeps that lacked it, with a test red
first on all three architectures; `notes/confinement-claims.md` carries it and the eight attacks that
held. **The caveat is the one that keeps the gate closed**: it was us attacking our own system, which
is the thing milestone 198 exists to stop being the only kind of attack this project has seen. And one window was accepted rather than closed at the time: `PortRange::REVOKE`
reached one core, so a revoked holder on another core kept its bitmap for at most one tick
(DECISIONS §152's `BUGS`, corrected the same day, and
[milestone 315](roadmap/315-port-revoke-every-core.md), which the audit raised as finding 4 and calef
promoted out of this entry's proposal on 2026-09-17).

**Corrected 2026-09-23: that window is closed.** Milestone 315 (a port revoke that reaches every
core) is BUILT. The revoke now resets this core and rides the TLB shootdown's NMI to the rest, and
`install_port_grant` moved inside the locked region so no core can reinstall a grant the sweep just
cleared. Proved by 12 of 12 full two-core suites, 36 boots, against 3 of 12 failing before.

**What it does to this risk, which is less than it sounds.** It removes an accepted hole from the
confinement claim, so the claim is stronger than the audit left it. It does not change the caveat
above, which is the one that matters: the attacking was still us attacking our own system. A window
we closed ourselves, found by our own test, is the same category of evidence as the audit that found
it, and this entry's verdict rests on that category rather than on any single hole.

**And the audit produced a third instance of this file's recurring shape.** Milestone 299's two port
tests could not fail in the direction they exist for: a wrongly permitted `out` was followed by a
`SEND` nobody received, so the run hung instead of going red. That is row 26's shape one object over,
found only because a draft of finding 1 hung. After milestone 305's vacuous `U`-bit test and
milestone 307's six unreachable assertions, **three independent sweeps have now each found
confinement tests that could not fail**, which is the strongest evidence in this file that the
question risk 3 asks is answered differently inside the kernel than outside it.

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

**Status: UNTESTED, and untestable by this project's own policy. No verdict, 2026-09-23.** That is
the finding rather than an apology for not having one. The other eight entries can come back red;
this one cannot come back at all, because the observation that would answer it is gated behind a
precondition calef set and milestone 530 (name a customer, or admit the ranking function has nothing
to rank) ruled on. **A fatal risk that cannot be tested is the most dangerous state a fatal risk can
be in**, and this file's own rule 1 says why: an entry that cannot fail is indistinguishable from an
entry that passed. Risks 3 and 7 each found tests of exactly that shape inside the kernel, three
sweeps running. This is the same defect one level out, in the file that judges the project.

**This one already fired, which is the most useful thing about it.** AGENTS.md's principle 1 ranks
work by the shortest path to a system a customer runs, and in August 2026 the customer had a real
deadline, nife could not meet it, and the customer solved the problem with Linux: borg over SSH on
cordoba, Immich for images. Milestone 55 (Time Machine over SMB3) is `REMOVED` and journey 2 went
with it. That is the principle working exactly as designed, and it is evidence rather than failure.

**What it changed:** the first customer was a family backup server, which is one of the largest
things a home system can be asked to be. **A first customer should be something nife can plausibly
be adequate at within a milestone or two.** The customer path is vacant, and it is recorded as vacant
rather than implied by a roadmap that still names one.

**The evidence today, counted rather than characterised, and it points one way.**

- **One user, who left.** That is the entire user history of this project, and the workload he left
  for is the workload that motivated it.
- **Zero others, and zero is not the damning part.** Nobody has been offered this system, so a count
  of no users measures no demand and no supply at the same time. **The absence of evidence is the
  problem**, not the evidence of absence: an untested claim cannot be quoted in either direction, and
  this entry is worth nothing if it is read as "probably fine, nobody has complained."
- **Nothing installs, so there is nothing to count.** Milestone 576 (how many systems are out there,
  and what do they run) is `NOT-STARTED` and gated on milestone 198 (a package manager, and the
  trivial install that makes a second customer possible), which is itself `NOT-STARTED` behind a
  `DECISION` gate. The project has no instrument that could observe a user if one appeared.
- **The one published argument that addresses this says it goes badly**, and it is cited below.
- **What the green results buy is narrower than it reads.** Risk 1 is green on three architectures
  (unmodified `ripgrep`, zero patches) and risk 9 is green on three silicons. Those answer *could
  somebody run this*. This entry asks *does somebody want to*, and no amount of the first answers the
  second. Treating capability as demand is the specific error this entry exists to prevent.

**Why no verdict can be rendered, stated as the loop it is.** Nobody can be asked to run nife until
it installs (calef's precondition: no third parties until a package manager and a trivial install
exist). It will not install until milestone 198 lands, and 198 waits on decisions only calef can
make. Milestone 530 ruled on 2026-09-21 that the customer path **stays vacant, and is blocked rather
than empty**, refusing the closest candidate (a measurement appliance for this project's own
benchmarking) precisely because it would be the architect in a different hat and would let the
ranking resume without resolving what made it vacant. That ruling is correct and it is also what
seals this entry: the strongest available reading of principle 1 while the path is blocked is that
**198 inherits the ranking function's top slot**, and until it lands this risk accrues in silence
while the roadmap grows.

**What would falsify it, concretely.** One observation, in two readings that must not be confused:

1. **Somebody who is not calef installs nife on purpose and is still running it two months later.**
   The install is the weak half; a person tries anything once. **Retention is the claim**, and
   milestone 576's instrument is built to see exactly that distinction: Fedora's `countme` model
   attaches an age bucket to a repository request the system was already making, so a first check-in
   and a long-lived one are distinguishable **without any identifier being sent or stored**. An
   install curve that reaches a handful and an age bucket that never ages is the red result: people
   look, and nobody keeps it.
2. **A package fetched by a system that is not ours.** Under the same design, package popularity is
   the fetch traffic counted per package with no linkage between one system's requests, which means
   somebody chose to do something specific with this OS rather than merely booting it.

**Is 576 the right instrument? Yes, and it will not work at the scale this project is at.** It is the
first mechanism nife will have for learning whether anyone runs it, and the `countme` model is the
right one for the reason its milestone gives: counting installs without an identifier is the only
shape that is defensible when it is on by default. But its arithmetic is distinct addresses per
bucket per week, designed for a distribution with millions of systems. At three, the noise is the
whole signal: one person on a changing address counts as several, a household behind one NAT counts
several as one, and no number of weeks fixes either. **So 576 answers this risk only at a scale nife
is a long way from**, and below that scale the falsifying observation is not a number at all. It is
one named person who is not calef, which is what milestone 530 asked for and could not get. Both
instruments should be expected: 530's for the first user, 576's for the hundredth.

**And 576 cannot answer the follow-on question by construction.** Its counts are unlinkable on
purpose, so it can report how many and which packages, and never why anybody stayed. Nothing in this
tree is an instrument for that, and nothing is planned to be.

**The counter-thesis here is published too, and it is a different argument from risk 4's.** *An
Incremental Path Towards a Safer OS Kernel* (Li, Miller, Zhuo, Chen, Howell, Anderson, HotOS '21,
DOI 10.1145/3458336.3465277) argues that memory safety should be brought to the kernel people
already run, module by module, because clean-slate kernels *"have significantly fewer features than
Linux ... impeding adoption"* and *"the cost of switching from Linux to these clean-slate designs is
prohibitive due to the established Linux and Android ecosystems."* That is this entry and risk 1,
stated by strangers, with a measurement behind it: 1475 Linux CVEs bucketed, 42% reachable by type
and ownership safety and 35% more by verification, and ext4 still minting CVEs after seven years of
use. **It takes no position on capabilities or on what a crossing costs**, which is why it belongs
here and not in risk 4: no benchmark decides it, and a microkernel that wins every crossing number
and that nobody runs has lost this argument anyway. `notes/incremental-path.md` has the paper, the
answer (the incremental path is only available to an actor who can move an existing system, which
this project is not), and the check of whether Linux is walking it: at Linux 7.2.4, five years on,
0.166% of the tree is first-party Rust, all of it new leaf code, with no existing C subsystem
replaced and functional correctness not begun.

**What the project is doing in the meantime, and which half is a plan.** The plan is real and it is
narrow: milestone 198 with §157 (a trivial install is a web page, a USB drive and packages) is the
route to being installable, and milestone 530 converted this gap into a gate by writing the
precondition down where a reader meets the empty path. **That is a plan to become testable, not a
plan to be needed**, and the distinction is the whole of this paragraph. Nothing in this tree is an
attempt to find out whether anybody wants this. There is no announcement, no outside reader, nobody
asked. The order being followed is *build until it is presentable, then look*, which is defensible
and is also precisely the order that lets this risk stay unanswered longest.

**The rest is a hope, and it is recorded as one.** The unstated expectation carrying this project is
that a system running unmodified third-party software on three architectures, with a verified core
and a confinement story, will find users once it is installable. **No evidence in this tree supports
that**, the one published argument that speaks to it says otherwise, and every green result on this
list makes nife more plausible to run without making anyone more likely to.

**The experiment that has not been run, and currently cannot be:** milestone 576, which needs
milestone 198 first and then needs a population this project does not have. Until then the honest
entry is the one above. **This risk is the question the other eight are in service of, and it is the
only one with no answer and no scheduled way to get one.**

## 9. The HAL is a fiction, and an architecture costs a restructure rather than a port, and so does the next machine

**The claim, calef's, 2026-08-30:** *"another proof/disproof of the nife thesis is actual
functioning on the three silicons. If we can't get it to run on one, that would also likely kill the
effort."*

**Sharpened, because the ISA count is not the fatal part.** An OS that runs on two of three
architectures is still an OS. What would be fatal is what a failure would reveal: that adding an
architecture requires changing the kernel rather than adding a directory under `arch/`, which is
exactly what DECISIONS §4 rule 1 and §19 (architectural parity is a tenet) claim it does not.

**Widened 2026-09-23 from architectures to machines, and the ruling is calef's.** He raised it about
rented computers: *"A nife that runs on one cloud platform but not another is also its own form of
risk."* Asked whether that was a tenth entry or a wider ninth, he widened this one. So the claim
above now reads on two grains rather than one. An **architecture** is aarch64, riscv64, x86_64. An
**implementation** is a particular machine of one of them: xenon, some provider's metal, a
hypervisor's idea of a PC. (Both words are this entry's, provisional, and neither is ratified.) Two
x86_64 machines from different vendors are the same silicon with different firmware, a different way
of delivering the boot, and different tables handed to the kernel at entry. A nife that boots on
xenon and not on a rented server falls through the old wording entirely, and it would fail for
exactly the reason this entry names: the abstraction was not as real as the port made it look.

Three reasons that belongs here rather than in an entry of its own, which is what the ruling turned
on.

**It is the same claim, asked twice.** Architecture and implementation are two grains of one
question: is the seam real, or does each new machine cost a restructure? Splitting them into two
entries would put one question's two halves on two lists and let each look answered by the other.

**The implementation grain is the earlier warning.** A HAL that is a fiction shows up first as "boots
on our board, not on theirs", long before it shows up as an architecture costing a restructure. By
the time an ISA is expensive, the cheap evidence has already been available for months and nobody was
reading it.

**And it is the grain that can actually be bought.** A Jetson TX1's silicon twin is not for sale, and
a second VisionFive 2 answers nothing a first one did not. Rented x86_64 and aarch64 metal is
available by the hour, from several vendors with different firmware. So the widened claim has an
experiment the narrow one does not, which is the second property the rule at the top of this file
demands.

**The experiment is a second machine of an architecture nife already boots on**, and it is not
specified here, because a lane is pricing rented metal right now against milestone 225 (run the soak
on radon, argon and xenon) and risk 4, the per-crossing cost, both of which want the same rental.
This entry's question rides along on that for the price of the boot rather than asking for hardware
of its own, and what that pricing will conclude is not known yet. What a result would mean is worth
fixing in advance, because all three outcomes are informative and only one of them looks like news.
If a second x86_64 machine needs a change **outside** `arch/x86_64/`, this risk moves toward red at a
grain the 2026-09-17 run never touched. If it needs a change **inside** `arch/x86_64/` that xenon did
not, that is the `AlreadyMapped` shape again, a machine-specific fix behind the seam, and it is the
cost this entry calls healthy rather than fatal. **And finding no difference at all is a result**,
recorded rather than shrugged at: it would be the first evidence anyone here has that the port is a
port and not one machine's configuration.

**The status is asymmetric, and that is the useful part.** riscv64 already disproves the strong form:
the VisionFive 2 booted the full tour on three harts on 2026-08-14, which is the single strongest
piece of evidence in the tree that the HAL is real. aarch64 is the development ISA and its board (the
Jetson TX1, milestone 127) is well documented. **x86_64 is where the risk actually lives**, and not
because x86 is hard, but because it is newest: milestone 161 was unfinished when this was written
(it is `BUILT` since 2026-09-19; see the dated paragraph below), milestone 177's text says
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

**Status: RUN, 2026-09-17. GREEN, and this is the verdict this entry was missing.** The sharpened
claim is falsified: adding x86_64 did not require changing the kernel outside a new `arch/`
directory. What follows is the evidence already gathered here, read together for the first time
rather than left as a narrative with no stated conclusion.

**The decisive experiment was milestone 87 (the x86_64 bare-metal machine), and it RAN on
2026-09-17. The result is green.**

```
nife machine: x86_64, 4 processor(s), 17119 MiB, 100 Hz
nife self-test: 5 of 5 passed
```

**nife runs on all three declared architectures on real hardware.** Transcript
`bench/xenon-2026-09-17/first-light-095500.log`; milestone 87's block has the detail.

**What the experiment was actually testing, and why this answers it.** This entry's own sharpening
says the ISA count is not the fatal part: what would be fatal is a failure revealing that adding an
architecture requires changing the kernel rather than adding a directory under `arch/`. **It did
not.** The x86_64 port reached a passing self-test on its own firmware with the boot entry, the
mapper and the discovery seam living under `kernel/src/arch/x86_64/`, which is what DECISIONS §4
rule 1 and §19 claim. Two boots were needed rather than one, and the defect between them
(`AlreadyMapped`, a fill that mapped device ranges cacheably because the firmware's map does not
describe the MMIO hole) was **machine-specific and fixed inside `arch/x86_64/mmu.rs`**, which is the shape
this entry predicts for a healthy HAL rather than the shape it fears.

**The port has a measured cost, not just a passing result, and it is the number this risk asked
for.** `notes/x86-port.md`'s "What had to change above `arch/`" section counted it directly: making
the whole kernel compile for the third architecture took **42 compiler errors, every one of them
"this `arch::` name does not exist yet,"** and the diff outside `arch/` was three small, named
things: `crates/paging` needed no change at all (`Ia32e` is sixty lines behind the existing
`PageFormat` trait); `drivers/ns16550.rs` gained one type parameter, because the same 16550 is
reached by MMIO on two architectures and by I/O ports on the third, which is a `RegisterSpace`
implementation rather than a second driver; and `console.rs`, `user.rs`, `drivers/mod.rs` and
`kernel/src/user/fs_service.rs` gained `cfg` arms "in the same places they already had two." The note's own
conclusion: "That is the whole diff above `arch/`. A new ISA was a new directory." **That is the
claim this risk exists to test, quoted rather than summarized, and it is what "GREEN" above is
based on.**

**Two things this does not claim.** The completion criterion was ruled by calef on the day to be the
self-test rather than a byte over serial, because a byte was printed on 2026-09-04 while the
milestone plainly was not done; a reader should take "the self-test passes" as the claim and nothing
wider. And **userspace was not reached on that boot**: the measured-boot gate refused the handover
because `xtask::uefi_image` built the kernel before the archive, so the kernel vouched for the
previous one. That is a defect in this project's build ordering, fixed the same day and verified
under OVMF, and it is the second time that same ordering defect has reached a bench.

**A third thing it does not claim, and the verdict above did not say so when it was written hours
earlier today (2026-09-23).** Every number in it comes from **one machine per architecture**, and for
one of the three not even that. x86_64 is xenon, a Core i5-7500T OptiPlex, on one firmware. riscv64
is radon, one VisionFive 2. aarch64's silicon is patagonia's own cores under Apple's hypervisor
(notes/hvf-leg.md) plus QEMU, because argon has never booted nife at all, which the multicore entry's
own list says in as many words. The 42 compiler errors, the untouched `crates/paging` and the one
type parameter measure what a third **architecture** cost. They say nothing about what a second
**machine** of an existing architecture costs, because no second machine has been booted, and until
the widening above was written there was no wording under which anyone would have noticed the gap.
**This does not overturn the verdict.** The measurement is real, it answers the question the
sharpened claim asked, and re-reading it changes not one of the numbers. What was unstated was its
scope, and a verdict that states its scope is stronger than one that leaves it to be discovered by
the next reader.

**What remains on this edge is no longer first light.** It is the two-core defect under firmware,
the boot entry's remaining work, and the orchestrator, all of which are schedule rather than
restructure.

**Milestone 161 turned BUILT on 2026-09-19, and that does less to this risk than the status word
suggests.** The sentence above that called 161 unfinished was true when written. What closed it was
four follow-on items, none of them first light: 2 MiB and 1 GiB leaves in `crates/paging`, adopted
by all three architectures' direct maps through the same `PageFormat` seam (a fourth architecture
would implement two more trait methods, not change the walk, which is this entry's claim holding
again); `cpu_start` counting a started core as absent (the "two-core defect under firmware" above
was this counting bug, reproduced at 26 of 40 four-core QEMU boots and gone in 80 of 80 after, and
fixed in `arch/x86_64/mod.rs`, not in portable code); and `CR4.PGE`/`PCIDE`, measured and left off.
None of it touched the kernel outside `arch/` except `crates/paging`, which every architecture
shares. What remains on this edge is unchanged: the boot entry's remaining work and the
orchestrator, schedule rather than restructure. xenon has still not been asked to bring four cores
online with the fix.

**Milestones 177 and 182 turned BUILT on 2026-09-19, and the paragraph above is now settled the
way it predicted.** §149 was decided (yes, a kernel-served console endpoint) and milestone 182
reached a shell over serial on x86_64: `script/shell-check` has a third leg that boots the UEFI
image a customer's stick carries, types 60 of its 64 lines at the prompt and reads the answers.
Milestone 177 closed separately, and its defect is the interesting half for this risk: the graphical
boot hung because two userspace drivers each sent a one-time report that the boot code had stopped
receiving, so both sat in a blocking send. That is a wiring mistake in a capability protocol, not an
architecture-shaped cost, and it was identical on aarch64 and riscv64, which is this entry's claim
holding rather than bending. **So the sentence above that reads "x86_64 has no real interactive boot
entry point at all" is retired**, and what remains on this edge is what the 2026-09-19 entry for
milestone 161 says remains: the orchestrator, and xenon confirming four cores with the counting fix.

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

**A citation this entry owed and had not paid, closed today (2026-09-23).** Everything above
verifies rule 1 for the kernel. It never checked whether the rest of the tree keeps the same
discipline, and `notes/architecture-list-sweep.md` (2026-08-27) is the sweep that asked exactly
that question, tree-wide, and is milestone 186 (derive the architecture list, and close what it
does not reach)'s worklist. Milestone 186 is `NOT-STARTED`. **None of what it found is inside
`kernel/src/arch/`, and none contradicts the port's own accounting above**; every one of its eleven
silent gaps is a script, a CI leg, or a userspace driver's own hand-rolled `#[cfg(target_arch)]`
arms, which rule 1 names by its own text ("all **architecture-specific code**") but which its own
citation path (`kernel/src/arch/`) never covered. Ten of the eleven are hardcoded two-item lists in
tooling that a third architecture walked past silently (`script/bench`'s own `EXAMPLES`, a CI bench
leg, a stack-frame checker, `deny.toml`'s target array); one was a live defect, a panic handler with
no x86_64 arm, since fixed.

**The userspace half of that sweep is the shape this risk actually fears, one layer up from the
kernel, and it is worth stating precisely rather than folding into the count above.** The sweep's
finding 9 is four driver crates, `crates/virtio`, `components/src/gpu_driver.rs`,
`components/src/keyboard_driver.rs` and `components/src/net_transport.rs`, whose `barrier()`
function has an `aarch64` arm and a `riscv64` arm and no `x86_64` arm, so on x86_64 the function
compiles to an empty body: not a build failure, a silently missing compiler fence.
`components/src/entropy.rs`'s `barrier()` has the identical two-arm shape and is not in the sweep's
table, found rereading it for this entry rather than in the original sweep. The sweep's own
severity note argues the four it found are latent rather than live, because
`scripts/qemu-runner-x86_64.sh` attaches no virtio device on any x86_64 boot today; that argument is
unchanged for the fifth. `components/src/non_volatile_memory_express.rs`'s `barrier()` has all three
arms, because the NVMe driver was x86_64's own reason for existing (DECISIONS §86) and was written
arch-complete from the start, which is the control case: when a driver is built *for* the new
architecture, the third arm arrives with it; when it predates the architecture, it does not, unless
something goes and adds it. **This is a real, uncounted, non-fatal cost, and it does not overturn the
verdict above.** It is not a kernel restructure: it is five small files a fourth architecture, or a
device newly attached on x86_64, would find by nobody's list rather than by a compiler error, the
same way `pgrep`'s panic handler was found. It argues for the fix the sweep already priced (a shared
`arch::barrier()` behind a seam shaped like `crates/paging`'s `PageFormat`, or a `compile_error!`
default arm) rather than for a different reading of this risk.

**Read against the widening at the top, those five functions are the concrete instance of what it
names, and they are still open.** The failure was not a restructure and not a build error. It was
**silence**: five functions that compiled, linked, shipped and did nothing on a machine nobody had
run them on, found by somebody rereading a table rather than by any gate. A platform difference
presents the same way. A machine whose firmware leaves a bit set that xenon leaves clear, or whose
memory map is shaped differently, does not announce itself; it produces a kernel that works
everywhere anyone has tried it. Milestone 186 (derive the architecture list, and close what it does
not reach) is where the fix lives and it has not started, so the five are empty bodies on x86_64
today, latent only because no x86_64 boot attaches a virtio device.

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
| ~~3~~ | 9, the HAL, on the architecture that carries the risk | **RUN 2026-09-17: GREEN.** `nife self-test: 5 of 5 passed` on xenon; the boot entry, mapper and discovery seam it needed all landed inside `kernel/src/arch/x86_64/`, and `notes/x86-port.md` counts the diff above `arch/` at one type parameter and four files' worth of `cfg` arms | milestone 87 | done |
| 4 | 9, the HAL, at the implementation grain the entry was widened to on 2026-09-23 | a second machine of an architecture nife already boots, which is one rented boot rather than a purchase | milestone 225 (run the soak on radon, argon and xenon) | unpriced; a lane is costing rented metal for this and for risk 4 together |
| ~~4~~ | 1, the ecosystem | **RUN 2026-08-31: green on aarch64 and riscv64.** Unmodified `ripgrep`, zero patches, runs and reaches its own argument parsing. The blocker is a missing argv, not threads. x86_64 has `std` (milestone 184) and builds it; the run waits on a disk the FS service can find | milestone 121 | done for two ISAs |
| ~~5~~ | 3, the tests | **RUN 2026-09-14, the first census since the baseline.** 10,012 mutants, 64 crates, 91.7% killed; 93.6% against the baseline's own 38 crates, which is **up** from 92.4%. The fall to 85.3% was two crates scored against suites that could not run. **The verdict is calef's and is not yet given** | the proposal, gate `DECISION` | done; the re-read remains |
| 6 | 4, performance | the multi-tasking workload number, from the 2026-09-19 instrument | milestone 168 | one radon bench evening |
| 7 | 9 and 6 together | journey 3, end to end on three boards | journey 3 | months, and it is the capstone |
| -- | 5, multicore | the defect-discovery curve: a linear one is the red result. **Its three seed data points need re-deriving first (2026-09-23): the VisionFive 2's undelivered wake was retracted by that note's own fifth bench stop, and `ap_boot`'s two bugs have moved.** Milestone 315 and a two-core `NIFE_SMP` default are the cheap half and are a lane rather than bench time | milestone 201 | weeks, hardware |
| ~~7~~ | 7, confinement | **RUN 2026-08-31, extended 2026-09-16, AUDITED 2026-09-17.** 26 claims enumerated, 25 falsifications replaying red, §31's headline assertion unreachable in the case it exists to catch, and milestone 305's finding that **a confinement test could not fail**. The audit then found **DECISIONS §12 false on x86_64**: a deleted `PortRange` kept COM1 for life, on a path `system_initializer` takes every boot. Fixed | milestones 202, 305, 313 | done; the adversarial half remains |
| -- | 8, nobody needs it | **none, and none available.** The instrument is milestone 576 (how many systems are out there, and what do they run), which needs milestone 198 (a package manager, and the trivial install that makes a second customer possible) before there is anything to count | milestone 576 | blocked, not costed |

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
