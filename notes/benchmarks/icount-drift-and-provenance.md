# icount drift, and what a baseline was read against

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds why the tripwire is 10%, the toolchain and emulator stamps, and the 2026-07-28 `-smp 4` artefact, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## What the icount instrument cannot see

Cache misses, TLB behavior, branch prediction: TCG models none of them, so a change that is
count-neutral but cache-hostile passes `--check` silently. That is the known limit, stated in
the roadmap block too; the `--real` numbers are the net that catches what counts cannot, read by
a human rather than a gate.

## A correction: the counts drift across builds, so "attributable to the commit" was too strong

The original milestone-21 note said a count change "is a change in a code path, attributable to the
commit that made it." Building the EL0 primitive suite disproved the attribution half, and the
machine's verdict is worth writing down (milestone 25 folds in the fix).

icount is deterministic **per binary** (byte-identical runs, verified twice). It is **not** stable
across different binaries. Adding the `null_syscall_el0` bench (which touches no other benchmark's
code) moved `yield_switch` -7% and `ipc_rtt` +1.8% at the same time. Two controlled facts pin the
mechanism:

- A **dead** function added to `bench.rs` moved nothing. So it is not raw code *layout* (addresses
  don't change instruction counts anyway).
- The shifts are **non-uniform and opposite in sign** across benchmarks. So it is not a common-mode
  offset that could be subtracted out.

What is left is the compiler's **whole-crate decisions**: adding live code that calls into shared
functions (`sched::spawn`, `user::run`) changes inlining and monomorphization elsewhere, so *other*
functions' executed-instruction counts move, each its own way. Mixed into the session's drift was
also a **real** increase from the 19f object-capability refactor (the scheduler and thread hot paths
genuinely grew); the point is that the instrument cannot separate that from the codegen churn.

**The fix (milestone 25):** demote `--check` from a 2% gate to a **coarse 10% tripwire**. It still
catches a gross regression ("you 3x'd IPC"), which is real value, but it no longer pretends to
attribute a 3% wiggle to the commit in front of it. The **`--real` medians, read by a human, are the
fine signal**, and a few-percent codegen shuffle is already in their noise. Ideas we did *not* take,
and why: pinning hot-path layout (fragile, and layout was not even the cause); per-operation deltas
that cancel fixed overhead (the shift is in the measured body, not fixed overhead, so it would not
cancel); common-mode subtraction (the shifts are not common-mode). Recorded here rather than quietly
re-baselined, because the machine overruled the claim.

### The floors name the nightly they were read against (2026-09-21)

**A toolchain bump revalues every number in `bench/baseline-*.txt` and no commit is responsible for
it.** icount counts guest instructions and a new compiler emits different ones for the same source,
so the tripwire's headroom moves for a reason nothing in the tree explains. That happened: on
2026-09-15 the pin went to `nightly-2026-09-15` with the floors left alone, most of the headroom
went with it, and the tree was bumped again to `nightly-2026-09-20` before anybody acted. It was
luck that the drift then evaporated. Measured 2026-09-21 the worst margin was +0.02% on aarch64 and
+2.27% on x86_64, with riscv64's `rfence_self` **7.49% faster** than its floor.

So each baseline carries a `# toolchain: nightly-YYYY-MM-DD` line, written by `cargo xtask bench
--save` from the **pin** rather than from whichever compiler happens to be running, and
`script/lint` fails when that line and `rust-toolchain.toml` disagree. **It can only fire on a
branch that raises the pin**, which is the point: the bump is where the delta is caused, where a
human is already deciding something, and where re-recording is attributable. A general staleness
check would have failed branches that caused nothing.

**Auto-re-saving on a bump is refused** (calef, 2026-09-21), and the refusal is written at both
places somebody would reach for it, `script/toolchain-bump` and `.github/workflows/toolchain-bump.yml`.
A floor that tracks the compiler by construction moves by exactly as much as a nightly moved the
kernel, so a nightly that genuinely made this kernel slower would report nothing, and catching that
is most of what the tripwire is for.

### The emulator is the other half of the same fact (2026-09-21)

**An icount count is a function of two things: the compiler that emitted the instructions and the
emulator that counted them.** The section above made the first a record. The second was worse off,
and the `rfence` lane found it while chasing a benchmark that had apparently got 7.49% faster:
**`.qemu-version` pins 11.0.2, this machine has had 11.1.1 installed since 2026-08-28, and
`script/bench` never checked.** Every committed baseline in this tree was measured against an
emulator nobody recorded and nothing verified. `script/qemu-check`'s own warning text asserted the
baselines "were recorded against" the pin, which is the assumption wearing a fact's clothes in the
one place a reader goes to ask.

**The stamp records the emulator that RAN, not the pin, and the asymmetry with the toolchain stamp
is the argument rather than an inconsistency.** `rustup` *resolves* the compiler from
`rust-toolchain.toml`, so there the pin and the thing that ran are one fact by construction (the one
escape, `RUSTUP_TOOLCHAIN`, is named in the code). **Nothing resolves QEMU from `.qemu-version`.** It
is a wish about the machine, and on 2026-08-28 the machine stopped granting it. Writing the pin into
the baseline would file intent under the heading of provenance, which is precisely the defect being
closed. So `cargo xtask bench --save` asks the binary, and when the answer differs from the pin the
line says both, because that disagreement is a fact about the numbers rather than something to tidy.

**The check compares the stamp against the emulator in front of it, and lives in `--check` rather
than in `script/lint`.** A static lint cannot ask this: there is no emulator in a repository. Two
comparisons were available and only one is honest.

| compared against | what it would do |
|---|---|
| `.qemu-version` | fails any baseline truthfully recorded off-pin, so it forbids the file from stating the truth, and still passes a comparison run on a third version |
| **the emulator this run used** | asks the only apples-to-apples question there is: is the counter reading this floor the counter that produced it |

**`unrecorded` is a truthful answer and does not fail**, the posture
milestone 115 (the names that were refused) already takes for names. Every baseline carries it today, because
nobody wrote down which QEMU produced those counts and inventing a version now would be worse than
the gap. The check therefore fires at full strength from the first honest `--save` onward and never
on a number nobody stamped.

**What a failure means is not "upgrade something".** A re-record and the pin have to be settled in
the same commit, because CI builds the pinned emulator (`script/ci-qemu`) and will then read these
floors on it. **Which version this project should run is calef's, and is deliberately not decided
here**: the divergence has already reached published figures, so it wants stating rather than
tidying.

## 2026-07-28: the day `--check` failed on every primitive, and why it was the harness

Roughly eight merges landed on `main` in one day (milestone 32 phase 1 block writes, 16b IOMMU, 28
line discipline, 27 std, 30 net in three stages, 31 capability shell, and 22 phase A's fault
endpoint with the DESTROY force-kill amendment). None ran `bench --check`, because bench is not in
the `script/test` gate. When the dust settled, `--check` failed IMPROVED on four primitives and
REGRESSED on two, all far past the 10% tripwire:

| primitive | old baseline (smp=4) | HEAD (smp=4) | reported delta |
|---|---|---|---|
| call_reply | 1,876,614 | 965,679 | **-49%** |
| spawn_reap | 1,769,595 | 175,185 | **-90%** |
| ctx_switch | 6,308,105 | 3,648,857 | **-42%** |
| ipc_rtt_el0 | 21,364,834 | 11,950,853 | **-44%** |
| map_el0 | 408,897 | 575,752 | **+41%** |
| spawn_el0 | 3,132,783 | 3,633,897 | **+16%** |

Improvements that large and that uniform are suspicious on their face; a real -90% on spawn is not
something eight unrelated merges hand you for free. Bisecting the merge points (one icount run each,
deterministic) turned up the tell straight away: **coremark, which is pure compute and touches no OS
primitive, moved +63%** at the capability-shell merge (20.9M to 34.1M ticks), and `spawn_reap` did
not creep, it *teleported*, reading 1.77M at the base, 172k three merges later, 3.5M one merge after
that. A compute loop cannot legitimately move 63% because a kernel merged a socket API. The numbers
were not measuring what they claimed to.

### Root cause: the aarch64 icount bench ran `-smp 4`, and CNTVCT is global under icount

The bench reads `CNTVCT_EL0` (`arch::timer::now()`) around each loop. Under `-icount shift=0` all
vCPUs share **one** deterministic virtual-instruction clock, so that counter advances with the
*global* instruction stream across every hart, not just the core running the benchmark. The aarch64
runner defaults to `-smp 4` (`NIFE_SMP:-4`, matching the SMP tests), and the bench never overrode
it. So each measured window silently counted three other harts: their idle loops, and, worse, under
`-icount` an idle secondary hart parked in `wfi` **jumps virtual time forward to the next timer
tick**, dumping a large quantized lump of ticks into whatever window happened to be open. Add the
load-balanced spawner (a thread that spreads children across cores) and the count for `spawn_reap`
or `ipc_rtt_el0` becomes a function of how four harts happened to interleave, which any code change
perturbs. The result is deterministic per binary (so `--check` "worked" and the old baseline looked
stable), but it is not the path length of the primitive. It is the machine's four-hart idle pattern,
sampled.

The proof is a re-run at `-smp 1`. Single hart, the counter advances only with the bench thread, and
the same four commits that swung wildly at `-smp 4` go flat:

| primitive | baseline | iommu(16b) | cap-shell(31) | HEAD | smp=1 spread |
|---|---|---|---|---|---|
| coremark | 20,914,947 | 20,913,678 | 20,913,678 | 20,913,678 | ~0.006% |
| spawn_reap | 166,860 | 170,952 | 170,952 | 175,890 | +5.4% |
| ctx_switch | 2,664,204 | 2,679,734 | 2,680,270 | 2,685,272 | +0.8% |
| ipc_rtt_el0 | 9,603,751 | 9,694,986 | 10,005,385 | 10,101,111 | +5.2% |
| call_reply | 956,768 | 956,769 | 956,893 | 963,080 | +0.6% |
| spawn_el0 | 1,574,632 | 1,609,770 | 1,750,084 | 1,754,438 | +11% |
| ipc_rtt | 861,095 | 864,935 | 861,346 | 922,720 | +7.2% |
| null_syscall | 427,706 | 457,705 | 457,705 | 457,705 | +7.0% |

coremark is invariant to five decimal places, which is the sanity check the smp=4 run failed. The
`-42%` to `-90%` improvements and the `+41%` regression were **entirely the smp=4 artifact**; they do
not attribute to any merge's code because they are not code, they are the four-hart interleaving that
the old baseline froze one sample of and today's merges reshuffled.

### This was a known bug on one ISA and an unfixed one on the other

The riscv bench path already pins `NIFE_SMP=1`, with a comment describing this exact failure
("a `wfi` jumps virtual time to the next timer tick, inflating the spawn primitives to
timer-quantized nonsense"). That fix landed with the riscv icount bench (commit 494514b) and was
never mirrored to aarch64. So the aarch64 icount instrument has been measuring four-hart noise since
milestone 21; the old baseline was noise too, internally consistent enough to pass `--check` until a
day of merges moved the interleaving far enough to trip it. **The fix is one line**, the aarch64
icount path now sets `NIFE_SMP=1` like riscv, and the baseline is re-saved at single hart. Real
per-core magnitudes still come from `--real` (HVF), where each core keeps its own counter and
parallel harts do not inflate elapsed time, so SMP there is not a confound.

### What actually moved, once the noise is gone

At `-smp 1` every primitive is within ~11% of the old (contaminated) baseline's *intent*, and the
movement that is real is small and mostly explained:

| primitive | true delta (smp=1) | cause | assessment |
|---|---|---|---|
| ipc_rtt | +7.2% | the IPC mailbox widened 3 words to 5 (milestone 22 §26, the fault-message carrier); every `ipc_send`/`ipc_recv` now copies five words via `wide()` | real, small, expected; the step lands exactly at the M22 merge (cap-shell 861k to HEAD 923k) |
| ipc_rtt_el0 | +5.2% | same mailbox widening, on the EL0 path | real, small |
| spawn_el0 | +11% | the M31 SPLIT rights-inheritance change (child budget gets full delegable rights); spawn_el0 does a SPLIT + retype per iteration | real, small; the step lands at the cap-shell merge (1.61M to 1.75M) |
| null_syscall | +7.0% | one-step at the blk-write/iommu merges, then flat; kernel layout/codegen drift in the syscall entry path, not a redesign | codegen drift, in the noise the note below already documents |
| spawn_reap, map_new, map_el0, call_reply, ctx_switch | +0.6% to +5% | whole-crate codegen churn across eight merges | codegen drift, expected and sub-tripwire |
| coremark, yield_switch | ~0% | pure compute / tight kernel yield, no structural change | invariant, as they should be |

None of these needed a merge reverted or a path investigated. The only defect the episode exposed was
the harness itself. No bench was measuring fiction in the sense of an elided loop or an early exit,
the loops all still do their work; the fiction was the *counter*, reading four harts where the
benchmark meant one. The new baseline (smp=1) is the first aarch64 icount baseline that measures the
primitive rather than the machine, and it now agrees in shape with the riscv one.
