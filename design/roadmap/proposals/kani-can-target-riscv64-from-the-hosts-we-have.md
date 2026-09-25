# Kani can prove riscv64 from the hosts we already have, with a 46-line patch and no riscv64 machine

**Status: PROPOSED 2026-09-24.** Raised by the research lane `lane/kani-riscv64-host` (pull request
#1280). calef asked whether Kani could be fixed for riscv64, perhaps with `radon` as a native host
once bench runs are done. The maintainer added a second question mid-lane: what would containing
every `asm!` in thin wrapper functions buy? This file is a sibling of pull request #1276's proposal,
which found the `Unsupported architecture` panic and is not yet merged. *(Slug provisional; naming is
calef's.)*

**Gate: DECISION.** Every option that reaches riscv64 changes the prover, either as a patch this
tree carries or as a contribution upstream. DECISIONS §46 (thin primitives or whole subsystems)
makes that a dependency decision. Nothing was added to the tree: the patch was built and run in a
scratch checkout of Kani, and the probe harnesses were removed from the kernel after the run.

## The answer

**A riscv64 host is not needed, and would not help.** CBMC does not care which machine it runs on.
It checks a goto program, and the machine model is data written into that program. So what Kani
lacks is a riscv64 *target*, not a riscv64 *host*. A patched Kani 0.67.0 on patagonia (aarch64
macOS, stock CBMC 6.8.0) compiled the whole nife kernel for `riscv64gc-unknown-linux-gnu` and ran
its harnesses:

```console
$ KANI_TARGET=riscv64gc-unknown-linux-gnu cargo-kani -p kernel -Z unstable-options \
    --ignore-global-asm --output-format=terse
Checking harness arch::riscv64::mmu::riscv64_reach_probe::reaches_sfence...
Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani. Please post your example at https://github.com/model-checking/kani/issues/2
 File: "kernel/src/arch/riscv64/mmu.rs", line 681, in arch::riscv64::mmu::flush_asid
VERIFICATION:- FAILED
Checking harness arch::riscv64::mmu::riscv64_reach_probe::satp_asid_round_trips...
VERIFICATION:- SUCCESSFUL
Checking harness syscall::proofs::every_page_between_the_checked_ends_is_itself_a_user_page...
VERIFICATION:- SUCCESSFUL
Checking harness syscall::proofs::the_run_end_is_exact_and_refuses_exactly_what_does_not_fit...
VERIFICATION:- SUCCESSFUL
Complete - 3 successfully verified harnesses, 1 failures, 4 total.
        7.27 real         2.27 user         0.68 sys
```

The failure is the point of the second probe. It calls `flush_asid`, which runs `sfence.vma`, and
Kani reports the reachable `asm!` as unsupported rather than refusing to compile. That is the same
behaviour a native aarch64 `asm!` gets today. The first probe proves that `ttbr0_value` and `asid_of`
round-trip the ASID and the root for every page-aligned root below 2^56. It sits inside
`arch/riscv64/mmu.rs`, the largest riscv64 file, with its real `cfg` dispatch and its real
`crate::arch`. Nothing here is a fiction about which architecture is underneath.

A third probe stubbed `read_satp` (a one-instruction `csrr` wrapper) with a model and proved
`current_root_pa` extracts the root it was given:

```console
$ KANI_TARGET=riscv64gc-unknown-linux-gnu cargo-kani -p kernel -Z unstable-options -Z stubbing \
    --ignore-global-asm --harness current_root_is_the_ppn_field
Checking harness arch::riscv64::mmu::riscv64_stub_probe::current_root_is_the_ppn_field...
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

So the target flag and the maintainer's containment idea combine. The flag makes every riscv64 line
compile; stubs on thin wrappers make the logic around `asm!` reachable.

## 1. What Kani needs per architecture, read in the source

Read at the `kani-0.67.0` tag (the version this tree runs locally) and at `cbmc-6.8.0`, the CBMC that
tag pins in `kani-dependencies`.

Kani, four places, all keyed to the host:

| where | what it does | host-bound how |
|---|---|---|
| `compiler_interface.rs` `new_machine_model` | builds CBMC's machine model | matches `Arch::X86_64` and `Arch::AArch64`, otherwise panics |
| `compiler_interface.rs` `check_target` | allowlist | `llvm_target` must be x86_64 or aarch64, Linux or Apple |
| `kani-driver/src/call_cargo.rs` | the `--target` Kani passes to cargo | `env!("TARGET")`, the triple Kani itself was built for |
| `tools/build-kani/src/sysroot.rs` | builds Kani's `std` and `kani` rlibs | `-Z build-std` for `env!("TARGET")` only |

The machine model is 20 fields: pointer width, endianness and alignment come from the rustc session,
and the C widths (`char` signedness, `long double`, `wchar_t`) are hard-coded per arch.
`cprover_bindings/src/env.rs` writes them into the goto program as `__CPROVER_architecture_*`
symbols.

CBMC already supports riscv64. `src/util/config.cpp` has `set_arch_spec_riscv64()`: LP64,
little-endian, 128-bit `long double`, unsigned `char`. `set_arch("riscv64")` dispatches to it, and
`set_from_symbol_table` reads the architecture string from Kani's symbols before overriding each width
from them. So CBMC needs no change and no rebuild. The stock macOS binary checked the riscv64 program
above.

The patch, measured: 46 lines added and 5 removed across the four files. The model arm is the
aarch64 arm with the RISC-V psABI's `wchar_t` (signed) and `long double` (binary128). The prototype
reads the target from a `KANI_TARGET` environment variable, which is a lab convenience, not a design.
It built in 244 seconds wall at four jobs on patagonia, 1.09 GB peak resident, 1.5 GB of `target/`.
It applies to Kani 0.68.0 (released 2026-09-16) with 9 of 10 hunks at an offset. The one rejected
hunk appends a function at end of file, a one-line fix.

One loose end: every riscv64 compile warns `target feature 'd' must be enabled`. Adding riscv64
features in Kani's `target_config` did not silence it. It is a warning today, and rustc says it will
become an error, so an upstream version has to find the right place to set it.

## 2. Two shapes: a target flag, or a riscv64 host

| | (a) target flag on today's hosts | (b) native Kani on a riscv64 host |
|---|---|---|
| Kani change | the four places above | three of them: the model, `check_target` and a bundle; only the driver's `--target` goes away |
| CBMC | stock, any host | built from source: 6.8.0 ships Windows, x86_64 and arm64 Ubuntu packages only |
| Kani install | build from source | build from source: 0.67.0's four release bundles are x86_64 and aarch64, and `cargo kani setup` looks up a bundle named for the host triple |
| rustc | the nightly Kani pins, on the host we have | `nightly-2025-11-21` does ship `rustc`, `rustc-dev` and `llvm-tools` for `riscv64gc-unknown-linux-gnu`, checked in its channel manifest |
| removes the architectural gap | yes: real `cfg`s, real `crate::arch`, the 95 riscv64 `cfg` lines outside `arch/` | yes, identically, since the goto program is the same |
| runs on every PR | yes, on the existing `ubuntu-24.04-arm` runner | only with a self-hosted runner; GitHub's hosted runners are x86_64 and arm64 (recalled, not checked) |
| machine | none | `radon` or the Scaleway RV1, as a Linux box |

(b) is (a) plus a machine. It still needs the compiler patch, because `new_machine_model` panics
on a riscv64 host exactly as on any other. It saves only the driver's four lines, and pays with a
from-source CBMC, a Linux host and a self-hosted runner.

Why `radon` is the wrong host. `radon` runs nife by netboot on the bench: UART into patagonia,
smart plug 2. Milestone 225 (run the soak on radon, argon and xenon) and pull request #1275's bench
rehearsal both need it as a nife board. As a Linux runner it would boot Linux from local storage, so every bench run first has
to take it off CI duty and back again. A soak cannot share it at all. A self-hosted runner on a
public repository also executes pull request code on a machine on calef's LAN, next to patagonia.
The Scaleway RV1 avoids the LAN problem but pull request #1278 is preparing it as a nife port
target, so it has the same conflict.

## 3. Would upstream take (a)?

Read, not recalled:

- model-checking/kani#2402, "Command-line flag to change model target or environment": open since
  2023-04-23, labelled `[C] Feature / Enhancement`, last touched 2024-10-02. A zerocopy maintainer
  added two use cases there (big-endian proofs, and 16-bit `usize` to speed up proofs). The one Kani
  contributor reply points at the supported-platforms page. Nobody has objected, and nobody has sent a
  pull request.
- #2886, "Support custom build target" for embedded, closed the same day as a duplicate of #2402.
- #2086, a user's non-host target (32-bit armv7). #1276 records that it "then hit a CBMC crash".
  The thread goes on: a Kani contributor found the cause, `goto-cc` needing `-m32` for a 32-bit
  model, and the user confirmed the fix worked. The crash was a missing flag, not a dead end.
  riscv64 is 64-bit like both linking hosts, and needed no such flag above.
- #2197 (open) says Kani's own platform documentation should warn that building from source "is
  not guaranteed to work on most platforms since their machine models may not be included".
- Kani's RFC process asks for an RFC for a "one way door". A flag behind `-Z unstable-options` is
  not one, which is the shape to propose.

So the demand is on record from other users, the maintainers have helped people do this by hand, and
nothing says no. What the tree cannot know is review latency. The source moves weekly, too: the
compiler file changed three times in the last month for toolchain bumps.

What an upstream-quality version adds beyond the prototype, estimated and not measured: a real flag,
a sysroot holding more than one target's `std` (today `lib/libstd.rlib` has one slot), bundle and
`setup` support, a regression test, and the `d` warning. Two to four lane-days, plus review.

## 4. What stays unreachable even with riscv64 support

Measured on base `334804c8e`. The counting command for `asm!` excludes `global_asm!` and comment
lines:

```console
$ grep -rnE --include='*.rs' 'asm!' kernel/src/arch/$ARCH | grep -v global_asm \
    | grep -vE '^[^:]+:[0-9]+:\s*(//|\*)'
```

| arch | inline `asm!` | `global_asm!` | Rust lines in `arch/` |
|---|---:|---:|---:|
| aarch64 | 45 | 6 | 6,214 |
| riscv64 | 56 | 5 | 5,873 (6,663 with the four `.s` files) |
| x86_64 | 38 | 4 | 10,043 |

(#1276's "61 sites" is these 56 plus the 5 `global_asm!`. Its 11,746-line total for
`arch/riscv64/` is exactly twice 5,873, so its "6%" for option 1's 736 lines is really 12.5%.)

With the target flag, every riscv64 Rust line compiles, as the run above shows. What a harness
can then *prove* is bounded by three things:

- Reachable `asm!` fails the harness unless it is stubbed. 56 sites in 9 of 13 files.
- Fixed-address MMIO reads as a dereference of an invalid pointer. model-checking/kani#1304 is
  still open, waiting on CBMC's MMIO regions. 22 volatile sites: `iommu.rs` 13, `mmu.rs` 6,
  `exceptions.rs` 2, `semihosting.rs` 1. Also a stub.
- The four `.s` files (boot, context switch, trap entry, FP save) are 790 lines no Kani on any host
  will read. `--ignore-global-asm` drops them.

Reach today without stubs: the asm-free, MMIO-light logic. That is `context.rs`, `irq.rs` and the
logic of `iommu.rs` (736 lines), plus the pure functions inside asm-bearing files, like `satp`
composition above. Reach with stubs is most of the rest, which is where containment comes in.

## Containment, priced (the maintainer's addition)

The idea: each arch keeps all `asm!` in one-instruction wrapper functions, logic calls only those,
and a proof stubs them.

The sites classify cleanly. Of 139 inline sites, 127 are single instructions or fixed fences
(a CSR or system-register access, a barrier, a TLB or cache op, `wfi`, `ecall`, port I/O). The
per-site table is in pull request #1280's description, and the command above reproduces the counts.

| arch | single-instruction | must stay asm | logic inside asm |
|---|---:|---:|---:|
| aarch64 | 37 | 3 | 5 |
| riscv64 | 53 | 2 | 1 |
| x86_64 | 37 | 1 | 0 |

The six "must stay" are two register-preservation tests, the `at`+`par_el1` translation probes, the
`satp` ASID probe and the x86_64 GDT reload. Context switch and trap entry are already in `.s`. The
"logic" are four aarch64 read-modify-writes of CPACR and ICC_SRE (movable to Rust around two
wrappers) and two calibration spin loops that are asm on purpose, to fix the instruction count.

riscv64 is nearly contained already. By a crude count (asm inside a function of 8 or fewer code
lines), 34 of its 56 sites already sit in small functions. Most of the rest are SBI `ecall` helpers
padded with comments, copied per call site with no shared helper. About six functions carry asm in
real logic: `mmu::install`, `probe_asid_bits`, `permit_kernel_access_to_user_pages`,
`timer::set_cycle_counter_grant`, and two in `exceptions.rs` init and tests. aarch64 has 30 sites
inline in larger functions and x86_64 12, by the same count.

Does it answer #1276's refusal of its option 2? Only partly. #1276 refused compiling riscv64
files on the aarch64 host because `crate::arch` resolves to aarch64. Containment plus a model module
swapped in by `cfg(kani)` would let riscv64 logic compile without riscv64 register names, and
`super::` stays inside the riscv64 tree. But the riscv64 files hold 40 `crate::arch` references and
at least 58 into portable modules (`crate::memory`, `crate::smp`, `crate::cpu` and others). On the aarch64
host those run aarch64's architecture underneath. Rewriting the 40 to `super::` narrows the fiction;
the 58 remain, and the 95 riscv64 `cfg` sites in portable code still take their aarch64 branch. The
target flag removes all of it. So the host-swap half of containment is dominated by the flag, and
is only worth doing if calef refuses the flag.

The stub half stands on its own, on every architecture and host. The tree has zero `kani::stub`
today. It is also what turns option (a)'s compile reach into proof reach.

## The options

1. Carry the patch, and upstream it at the same time. The patch goes in `patches/` in `git
   format-patch` form, which is the tree's precedent for fixes it carries until upstream releases
   them (the two redoxfs patches). A new job, `prove-kernel-riscv64`, builds the patched Kani from
   source on the `ubuntu-24.04-arm` runner, cached on the patch's hash, and runs `script/verify
   --only kernel`. The same diff, reworked into a `-Z` flag, goes upstream against #2402. The carried
   patch leaves when a Kani release contains it. Cost: the patch as measured, and a CI job whose cold
   time is unmeasured (patagonia built Kani in about 4 minutes, before toolchain download). Then a
   rebase at each Kani release (one hunk at 0.68.0), and 2 to 4 lane-days for the upstream version.
2. Upstream only, and wait. No carried patch. Cost is the upstream work alone. Risk: #2402 has sat
   for three years, so riscv64 stays unproved for an unknown time.
3. A native riscv64 host. Refused: it needs option 1's compiler patch anyway, plus a
   from-source CBMC, Linux on a board whose job is running nife, and a self-hosted runner for
   pull request code.
4. #1276's option 1 alone (compile asm-free riscv64 files as a proof-only module on the aarch64
   host). Superseded by options 1 and 2, which reach the same files with the real `crate::arch`.
   It is the fallback if calef refuses to change the prover.
5. Containment, as priced above. Not an alternative to 1 or 2 for riscv64; it is the next step
   after either, and useful on aarch64 and x86_64 now.
6. Nothing.

## Recommendation

**Option 1, then containment's stub half, riscv64 first.** Carry the patch so riscv64 is proved on
every pull request now. Send the flag upstream so the patch has an exit. Then give riscv64's SBI calls
one shared helper and move the six logic-bearing sites behind wrappers. That wrapper set is what the
first riscv64 `kani::stub` harnesses need. aarch64 and x86_64 follow the same pattern afterwards.

## The seven questions

1. Considered and lost. Native host lost because it is option 1 plus a machine: same compiler
   patch, more cost, and it takes `radon` from the bench. Upstream-only lost on unknown latency, not
   merit. #1276's option 1 lost on the `crate::arch` fiction the flag removes. Containment's host
   swap lost to the flag for the same reason; its stub half is kept.
2. What the tree does in the analogous case. `patches/README.md`: carry a patch in format-patch
   form, submit it, drop it when the pin passes a release with the fix. And milestone 304 (`cargo kani
   -p kernel` only ever compiled one architecture) answered the x86_64 gap with a second CI row, which
   is the shape of `prove-kernel-riscv64`.
3. Prior art outside the tree. Kani and CBMC source at their pinned tags, and #2402, #2886, #2086,
   #2197 and #1304, all read through `gh` for this proposal. The GitHub hosted-runner architectures
   are recalled.
4. Is the premise true? The question's premise, that riscv64 support needs a riscv64 host, is not.
   CBMC is host-independent, and a patched Kani on an aarch64 Mac proved riscv64 code. #1276's
   reading of #2086 as ending in a crash is also incomplete: the crash had a one-flag fix.
5. Cost. Patch size, build time, memory, rebase fit and the kernel run are measured above. Kani's
   upstream-quality rework and CI cold time are estimates, marked.
6. Reversibility. A carried patch and one CI job come out cleanly, and nobody downstream acts on
   them. An upstream contribution is a public commitment to maintain, in a small way, a flag other
   people will use; that is the irreversible part, and it is an architect's.
7. Same cost, same choice? Yes. If the native host cost what the flag does, the flag still wins:
   it runs on hosted runners, on every pull request, and leaves `radon` on the bench. This
   recommendation is not about effort. Carrying versus waiting is about time, and says so.

## What it does not change

It does not turn fatal risk 2 (the proofs prove trivia) green. The survivorship half of that amber
is untouched, and `design/fatal-risks.md` is an architect's file. What changes is the sentence
`notes/kernel-proofs.md` and milestone 304's block carry, that riscv64 is unreachable and no one here
can fix it. Once option 1 or 2 lands, that sentence is false, and milestone 536 (two records still say the prover cannot see
`kernel/src`) should correct it.
