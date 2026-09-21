# 541. Two gates that stopped meaning what they measure: a bump-time mechanism and a footprint reported against its budget

**Status: BUILT 2026-09-21.** *(Number provisional until the merge queue lands it; 524 to 540 were
claimed on in-flight branches when this was written.)*

calef, 2026-09-21: *"Do the bump-time mechanism. Do the footprint gate."* Two rulings, and they are
one milestone because they are one defect wearing two costumes. **A gate whose number is measured
against a moving reference stops making a claim about the system.** The icount floors move when the
compiler moves; the footprint gate's percentage moves when the binary moves. In both cases the gate
stayed green, said something, and the something was no longer about this kernel.

## Part 1: a toolchain bump cannot silently revalue the icount floors

`bench/baseline-{aarch64,riscv64,x86_64}.txt` are the tripwire's committed floors, and icount counts
**guest instructions**. A new nightly emits a different instruction sequence for identical source,
so every number in those files is revalued by a change no commit in the tree is responsible for.
Nothing said so and nothing checked it.

**The cost was live and it was paid.** On 2026-09-15 the pin went to `nightly-2026-09-15` with the
floors left un-resaved, most of the tripwire's headroom eroded, and the tree was bumped again to
`nightly-2026-09-20` before anybody acted on it. **The drift has since evaporated**, measured
2026-09-21: worst margin +0.02% on aarch64, +2.27% on x86_64, and riscv64's `rfence_self` is
**7.49% faster** than its floor. So this milestone is not repair. It is the mechanism that turns
"we got lucky" into "we would have been told."

**The shape, which is the loud one rather than the automatic one.** Three shapes were on the table
and calef's ruling picks the first:

- **A bump pull request that re-records in the same pull request.** Chosen. A human sees the delta
  at the moment it is caused, and it is attributable to the branch that caused it.
- **A check that fails saying the baselines are stale for this nightly.** Louder, and it fails pull
  requests that did not cause it.
- **Auto-re-saving on a bump.** **Refused**, and the refusal is written in the code at both places
  somebody would reach for it. A floor that tracks the compiler by construction moves by exactly as
  much as a nightly moved the kernel, so a nightly that genuinely made this kernel slower would
  report nothing, and catching that is most of what the tripwire is for.

**What was built.** Four small pieces, and the first two are what make the third unnecessary:

1. **The floors carry the nightly they were read against.** A `# toolchain: nightly-YYYY-MM-DD`
   line, written by `cargo xtask bench --save` from `rust-toolchain.toml`'s **pin** rather than from
   whichever compiler happens to be running, because a `RUSTUP_TOOLCHAIN` override in one shell is
   not a fact about the repository. This is the fact everything here turns on and nothing in the
   tree stated it.
2. **`script/lint` fails when that line and the pin disagree.** This is the resolution of the second
   and third shapes above rather than a compromise between them: **a branch that does not raise the
   pin cannot fail this check**, because its baseline and its pin already agree and the merge queue
   rebases it onto a `main` where they still do. The only branch it can fire on is the one that
   caused it.
3. **`script/toolchain-bump` says the re-record is the operator's**, prints the exact commands, and
   carries the refusal above in the place a reader meets the bump.
4. **`.github/workflows/toolchain-bump.yml` says the same in the pull request it opens**, including
   that its own `script/lint` will be red and that the red is a step rather than a defect. That
   workflow is what raised the pin on 2026-09-15 without touching the baselines.

**One marked exception, and it is a foot gun.** `script/toolchain-bump` sets
`NIFE_BUMP_IN_PROGRESS=1` around its own gate run, because it has just raised the pin and has not
re-recorded anything, so the check is guaranteed to fire there and firing would only trip the
script's restore trap and un-bump a tree that is otherwise fine. Nothing in CI sets it. The
enforcement was never in that script: it is `script/lint` running unexempted on the bump's pull
request, and a green lint bought with this variable means nothing. Named provisionally.

**Proved firing rather than asserted**, which is the difference between a gate and a claim. Four
cases, run against this tree:

| case | result |
|---|---|
| this tree, pin and stamps agreeing | passes, `3 file(s) at nightly-2026-09-20` |
| pin raised to `nightly-2026-09-28`, floors untouched | **fails**, naming both nightlies on all three files |
| a baseline with the stamp line removed | **fails**, `no '# toolchain: <channel>' line` |
| `NIFE_BUMP_IN_PROGRESS=1` | skips, saying why and where the real gate is |

And the live case exists today: `origin/toolchain/nightly-bump` (pull request #1054) raises the pin
to `nightly-2026-09-21` and changes **nothing else**, so once this lands, that branch's CI fails
this check with exactly the second row's message.

## Part 2: the footprint gate measured drift when it should measure distance

`script/fastpath-footprint`'s own first line says why it exists: *"the IPC fastpath must stay small
enough to live in L1i"*, from Liedtke's argument that Mach's IPC was slow because of the cache
footprint of its hot path. **That is a physical budget, and the gate did not report against it.** It
printed `+1.4% against baseline (7028)`: drift from whatever the binary happened to be the last time
somebody re-recorded, a number that means something different after every compiler change.

The facts a reader needs were in `notes/benchmarks.md` and nowhere near the tool:

- the binding constraint is **radon's SiFive U74, 32 KB L1i**, which that note names as the one that
  binds among the machines this tree actually runs on;
- the tree's own stated target is **4 KiB**, about an eighth of it, a fraction derived from
  Liedtke's argument rather than from roundness;
- and the fastpath is **1.16x to 2.01x** that target depending on ISA and shape.

**What it prints now**, per architecture, with drift kept, demoted and relabelled:

```
    budget: 4096 B target, 32768 B L1i (radon's SiFive U74, the smallest we run on)
    ipc_send_recv    4734 B   1.16x target  14.4% of L1i  over 8 symbols
    ipc_call_reply   6038 B   1.47x target  18.4% of L1i  over 10 symbols  <- the shape the system runs
    ipc_fastpath     6038 B   1.47x target  18.4% of L1i  the worse of the two shapes
    syscall_entry    1914 B                  5.8% of L1i  over 5 symbols (flat, no closure)
    total            7952 B   1.94x target  24.3% of L1i  (7.77 KiB), an upper bound
    49% of the 16 KiB ceiling §144 (a delta and a ceiling) decides and does not yet enforce
    drift: ipc_send_recv +2.2% against baseline (4632), within the 5% band
```

Measured 2026-09-21 on `nightly-2026-09-20`, all three:

| | `ipc_call_reply` | x target | % of L1i | `total` | x target | % of §144's 16 KiB |
|---|---|---|---|---|---|---|
| aarch64 | 7,104 | 1.73x | 21.7% | 8,612 | 2.10x | 53% |
| riscv64 | 6,038 | 1.47x | 18.4% | 7,952 | 1.94x | 49% |
| x86_64 | **8,234** | **2.01x** | **25.1%** | **9,935** | **2.43x** | **61%** |

**`syscall_entry` gets the L1i share and no target ratio**, because the 4 KiB target is stated over
the IPC fastpath's instructions and the trap path is not part of what it bounds. Its bytes are
fetched on every syscall, so the share is still the quantity that matters.

**The 16 KiB line is the third distance and the only one the tree has already voted on.** §144 (a
delta and a ceiling) decides an absolute 16 KiB per architecture on exactly this `total`, derived as
half of the 32 KB L1i. It is DECIDED and not built, so printing the fraction is what keeps the
decision visible against the number it was made about.

**Drift still fails and the budget does not.** A 5% jump inside one pull request is a mistake
somebody just made, which is what a tripwire catches well; a budget the tree is 1.5x to 2x over
would fail on the day it was written and be turned off on the next.

## What was deliberately not done

- **The footprint baselines were not re-saved and nothing was shrunk** (calef's reasoning, recorded
  in the gate's own docs). Shrinking the fastpath today would be optimising against a target whose
  value nobody can measure, because milestone 370 (a layout control) exists precisely to say that
  the perturbation experiments cannot tell footprint from addresses. Whether 4 KiB is still right
  belongs with milestone 132 (the fast path's footprint) and milestone 188 (the IPC fastpath), and
  waits on 370.
- **No kernel code was touched.** The whole milestone is `script/`, one `xtask` function, one
  workflow, the baselines' header line, and notes.
- **§144's delta-against-`main` and its 16 KiB ceiling were not built.** The ceiling is now printed
  as a fraction, which is reporting, not enforcement.

## BUGS

- **The stamp is a verification and not a recording on its first day.** The three baselines' counts
  were produced on an earlier nightly and re-checked green against `nightly-2026-09-20` on
  2026-09-21; the line says so in the files themselves, and the next `--save` makes it ordinary.
- **`NIFE_BUMP_IN_PROGRESS` is an exemption on a gate.** Rung two with a hole in it, marked as a
  foot gun where a reader meets it, and relying on CI being the place the check is not exempt.
- **The check compares a pin to a stamp, not a measurement to a measurement.** A baseline re-saved
  under a `RUSTUP_TOOLCHAIN` override, on a compiler other than the pinned one, is stamped with the
  pin and passes. That is the honest boundary: it catches the case that has actually happened twice
  and cannot catch a lie told deliberately.
- **`bench/fastpath-*.txt` carries no such stamp**, so the footprint gate's 5% band still erodes
  under a bump with nothing to say so. Recorded in `script/fastpath-footprint`'s own `BUGS` and in
  this block's Follow-on, with the measurement that makes it urgent.

## Follow-on

- **Recorded.** The footprint baselines have the icount ones' old problem and the erosion is
  measured, not hypothetical: on `nightly-2026-09-20` every figure on every ISA sits above its
  baseline, and riscv64's `syscall_entry` is **+4.7% against a 5% band**. One more bump can fail
  that gate for a reason no commit is responsible for. Stamping it means re-saving it, and
  re-saving was refused for this milestone, so the first deliberate `--save` there is where the
  stamp belongs. In `script/fastpath-footprint`'s `BUGS`.
- **Recorded.** Whether the 4 KiB target is still the right number is not decided here and waits on
  milestone 370 (a layout control), with milestone 132 (the fast path's footprint) and
  milestone 188 (the IPC fastpath) owning the question.
- **Recorded.** §144 (a delta and a ceiling) remains DECIDED and not built. This milestone prints
  its ceiling as a fraction and enforces nothing, and the delta-against-`main` half is untouched.
- **Recorded.** `NIFE_BUMP_IN_PROGRESS` is a provisional name, like every name a lane coins.

## Index row

**Built:** 2026-09-21

Two gates had stopped making claims about this kernel. The icount floors in `bench/baseline-*.txt`
are revalued by any toolchain bump, which happened unremarked on 2026-09-15 and eroded most of the
tripwire's headroom; they now carry the nightly they were read against, `script/lint` fails when
that disagrees with the pin (which can only happen on a branch that raises it), and both
`script/toolchain-bump` and the proposing workflow say the re-record is a human's while recording
why auto-re-saving is refused. `script/fastpath-footprint` reported drift from an old binary rather
than distance to a budget; it now prints bytes against the 4 KiB target and the 32 KB L1i that
binds, plus §144's undelivered 16 KiB ceiling, with drift demoted to the secondary check that still
fails. Nothing was shrunk and no footprint baseline was re-saved.
