# A layout control, because the perturbation experiments cannot tell footprint from addresses

**Status: PROPOSED 2026-09-04.** Found by the E3 session on radon, which separated cleanly in three
rows and had the wrong sign in one of them. notes/footprint-perturbation.md carries the capture.

**Gate: MILESTONE 134.** E3 and E4 are milestone 134's experiments and that block owns the register
of measures. This proposes a control for them rather than a new measure.

## In brief

E3 compares two kernels that differ in one Cargo feature, `fastpath_pad`, and reads the difference
as the cost of the footprint that feature adds. **That inference does not hold**, and the
2026-09-04 session on radon proved it does not by producing an effect footprint cannot explain.

Six interleaved boots, three per condition, on a `single_hart` card where the placement lottery
cannot appear:

| row | unpadded | padded | delta |
|---|---|---|---|
| `ipc_rtt` | 4259 · 4259 · 4259 | 4311 · 4310 · 4310 | +1.21% |
| `call_reply` | 5015 · 5013 · 5013 | 5088 · 5089 · 5088 | +1.49% |
| `ipc_rtt_el0` | 128606 · 128626 · 128615 | 124958 · 124391 · 124903 | **-3.01%** |

All three are non-overlapping, with a within-condition spread of 0 to 2 units against gaps of 74
and 3,865. **The third row says the padded build is 3% faster, and the padding is never executed.**
It is reached through `core::hint::black_box` on a compare-and-branch that is never taken, so there
is no mechanism by which 5 KB of dead text makes a round trip 193 ns quicker.

What changed is the **layout**: different cache-line boundaries, different set indices in a 32 KB
2-way L1i, different branch-predictor aliasing. Mytkowicz, Diwan, Hauswirth and Sweeney measured
exactly this in *Producing Wrong Data Without Doing Anything Obviously Wrong* (ASPLOS 2009), where
changing link order or the size of a UNIX environment variable, neither of which alters an
instruction, moved measured performance by more than the optimisation being studied.

**So the +1.49% is a real difference between two binaries and is not evidence about two
footprints.** It is the sum of a footprint effect and a layout effect, and this experiment reports
the sum.

## What to build

**A layout control: several unpadded kernels that differ only in address assignment.** Each adds no
reachable instruction, so any difference between them is layout by construction. Candidates, in
increasing order of how much of the binary they disturb:

- A dead `#[used]` static of varying size in the same section.
- A dead symbol of varying size ahead of the fastpath, which is what `fastpath_pad` already is
  minus the claim that its size is the variable of interest.
- A link-order change in the linker script.

Read the same three rows across, say, five of them. That yields a **layout distribution** instead of
a single unpadded point, and E3's padded reading becomes interpretable for the first time: inside
the distribution is layout, outside it is footprint.

**The cheapest honest version is smaller than that.** `fastpath_pad` is already a size knob. Running
it at four sizes rather than two (0, 1x, 2x, 3x) gives a dose-response curve, and **footprint
predicts monotonicity where layout does not.** A row that rises with padding is footprint; one that
jumps and comes back is layout. That is one Cargo feature taking a value instead of a boolean, and
it may be a day rather than a project.

## Why it is worth doing rather than shrugging at

**It decides milestone 188 phase 4.** That block is holding a hand-written IPC fastpath, a standing
verification obligation and a permanent maintenance cost, on evidence that the footprint costs
something. The evidence currently available is a 19 ns effect with a 193 ns artifact sitting on top
of it, and nobody should buy a second IPC path with that.

**It also protects the numbers already published.** `bench/` holds stored baselines that a future
reader will diff across builds, and every one of those diffs has this exposure. The E4 displacement
percentages in the same capture (0 to 1% at ordinary load, 5 to 8% at 96 threads, peaking at 32 KiB
which is radon's L1d exactly) are the more interesting result of that session, and they are a
single-build measurement, so they are unaffected. The moment anyone compares them against another
build, they are affected.

**And it is the difference between a demonstrator and a marketing claim**, which is AGENTS.md's own
standard for benchmarks: state what a number means and where it is not apples-to-apples. An
overlapping range reported as a trend is the outcome this project's own bench note already refuses.
A confounded comparison reported as an attribution is the same failure with better arithmetic.

## What it is not

**It is not a reason to distrust E1 or E4.** Both are single-build sweeps, where the layout is
constant and the independent variable is thread count or working-set size. Nothing in this proposal
touches them, and E1's knee at 16 threads and E4's peak at 32 KiB are the session's real findings.

**It does not need the board free for long.** Each additional build is one card write and about 75
seconds of boot, measured. The cost is the interleaving discipline, which with one card means one
write per boot.

## Where it came from

The 2026-09-04 radon session. notes/footprint-perturbation.md's own BUGS carries the defect;
`design/roadmap/188-ipc-fastpath.md` is what it blocks.
