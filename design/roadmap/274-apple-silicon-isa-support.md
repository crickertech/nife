# 274. What Apple Silicon's own core actually implements, measured rather than assumed

**Status: NOT-STARTED.** Minted 2026-09-10 by calef, after the maintainer ran the aarch64 suite
under HVF (`scripts/qemu-bounded.sh 300 cargo xtask test --arch aarch64`, this dev machine) to
answer a question about `neoverse-n2`, and got back a firmer and smaller fact than expected.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Measurement, not a design fork. What this finds may feed DECISIONS §149's or a
later section's inputs, but deciding anything is out of scope here.

## What was actually measured, 2026-09-10, on this development machine

`cargo xtask test --arch aarch64` under HVF (`-cpu host`, the physical Apple core, no `NIFE_CPU`
override) reported **321 passed, 3 skipped**:

- `a_client_obtains_unpredictable_bytes_from_rndrrs_with_no_device_at_all` — **skipped**.
  `entropy_tests.rs`'s own comment names `--cpu neoverse-n2` as the fix under TCG. **Under HVF
  there is no CPU flag to pass**: the guest runs the physical core (`scripts/qemu-runner-aarch64.sh`
  refuses `NIFE_CPU` under HVF for exactly this reason), so this result is a direct measurement of
  the Mac's own silicon. **Apple Silicon does not implement `FEAT_RNG` (`RNDR`/`RNDRRS`), on this
  machine, confirmed rather than inferred.**
- `the_jh7110_backend_refuses_to_wire_where_there_is_no_jh7110` — skipped, correctly and as
  documented (`design/roadmap/159-jh7110-trng-driver.md`): this is riscv64-only hardware and the
  test's own comment already says so.
- `ripgrep_tests::unmodified_ripgrep_runs_and_has_no_arguments_to_run_on` — skipped, unrelated to
  architecture (`scripts/build-ripgrep` was not run before this suite).

**PMUv3 did not skip.** `cycle_counter_grantable()` read true and
`the_cycle_counter_grant_moves_cr_and_nothing_else` ran and passed, so **Apple Silicon's PMU is
visible to a QEMU/HVF guest**, at least the counter DECISIONS §139 grants.

## Why this earns its own milestone rather than a line in `entropy_tests.rs`

**The dev machine is not an edge case here, it is the primary aarch64 test target.** Most of this
tree's aarch64 work is run and reviewed on exactly this hardware, under exactly this hypervisor.
"RNDR needs a CPU flag" was the honest belief in `entropy_tests.rs` and it undersold the gap: there
is no flag that fixes HVF, so **instruction-mode entropy on aarch64 has never actually been
exercised on the machine most of this project's aarch64 testing runs on.** TCG with `--cpu
neoverse-n2` is the only aarch64 leg that has ever passed this test, and CI would need to be checked
for whether it runs that leg at all.

## The second finding, and it is bigger than the first

**Checked directly: CI never exercises the passing leg either.** `.github/workflows/ci.yml`'s `test`
job runs `script/ci-build` on `ubuntu-24.04-arm` over QEMU/TCG, with no `NIFE_CPU` set anywhere in
that script, so it takes the suite's default `cortex-a72`. The `cpu-matrix` job that does sweep CPU
models is named, in its own workflow, `cpu matrix (riscv64 across QEMU CPU models)` — riscv64 only.

**Milestone 162, which built this test, already knew the default would skip it.** Its own text: *"The
suite's default `cortex-a72` predates `FEAT_RNG`, so the test skips"* under the default, and the
correctness claim rests on one recorded manual run: `script/test --arch aarch64 --cpu neoverse-n2`.
That run happened once, to prove the code, and nothing was built afterward to make it happen again.

**So this test has passed exactly once in this project's history, by hand, on 2026-08-25 or
whenever 162 landed, and has skipped on every run since**, including every merge to `main` and
(per today's measurement) every run on the machine most aarch64 work is actually done on. That is
milestone 214's own concern (*"a test that measures nothing is not a pass"*) except this one has
never even been a pass to lose: it has always been a skip that a correct-looking green suite
absorbed silently.

## What this needs

1. **Give aarch64 the same thing riscv64 has**: either fold a `neoverse-n2` leg into `cpu-matrix`
   (renaming it to cover both architectures, or adding an aarch64-specific matrix job beside it), or
   change the suite's own default CPU model if `cortex-a72` no longer has a reason to be it.
2. **Correct `entropy_tests.rs`'s comment**, which currently reads as though a flag reliably fixes
   this. It fixes the emulated-CPU-model case; it says nothing about HVF, and a reader running this
   suite on their own Mac will hit the same skip and reach for a flag that cannot help them.
3. **Check radon**, once it has serial connected: does the JH7110's own core implement `FEAT_RNG`,
   independent of the software TRNG driver milestone 159 already built? A second, unrelated way
   aarch64 hardware might or might not have this instruction.
4. **Decide whether `PMU_PRESENT`'s HVF result should be written down as a positive claim** ("Apple
   Silicon's PMU is visible under HVF, measured 2026-09-10") the way the RNDR absence now is, rather
   than only living in this block.

## BUGS

- **The HVF measurement is one machine, one run.** It has not been repeated, and Apple's own
  hardware varies by generation; this says nothing about older or newer Apple Silicon, only this dev
  machine.
- **A skip that has stood in for a pass since 2026-08-25 was not caught by any gate.** `script/lint`
  and CI's own required-check machinery both consider `321 passed, 3 skipped` a clean run, and
  nothing compares the skip count against a baseline the way other counted-claims checks in this
  tree do. This is the same shape as milestone 268's finding 4 (the tour's checks report failure by
  printing a word and continuing): a real assertion existed, and nothing was reading its result.

## Follow-on

- **Milestone 268.** The same class of defect: a real check whose verdict nothing enforces. Worth
  reading together rather than as coincidence.
