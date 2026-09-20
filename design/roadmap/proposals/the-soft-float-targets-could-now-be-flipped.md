# The soft-float targets could now be flipped, and two measurements should come first

**Status: PROPOSED 2026-09-20.** Filed by the lane of milestone 447 (a thread's vector registers
are its own), which built the thing that was blocking the flip and deliberately did not take it.

**Gate: NONE.** Both measurements below run on the dev Mac and need nobody. The flip itself is
calef's ruling rather than a lane's, which is what makes this a proposal.

## Why this is a proposal and not a change

Every target in `targets/` is soft-float. Until 2026-09-20 that was **a correctness requirement**,
in the words of milestone 184 (extend the `std` port to x86_64), because `kernel/src/arch/x86_64/` saved no FPU or SSE state on a
context switch and neither did the other two architectures. Milestone 447 built the save, on all
three, so the feature strings are now a **choice**.

Taking that choice is not a lane's. Turning `+soft-float` off changes the calling convention for
every userspace binary and every `std` crate built against it, which is an ABI two programs agree
on; `AGENTS.md`'s *move fast on what can be undone* tenet puts that squarely in the irreversible
category, and §22 (Rust `std` on the native ABI, the Hermit way) picked the current
one on purpose when it specified nife's target JSON as "softfloat, and `singlethread = true`".

## What it would take

Four edits and a rebuild.

1. Drop `+soft-float` and the `-sse`/`-neon`/`-mmx` feature strings from the three files in
   `targets/`.
2. Drop `"rustc-abi": "softfloat"` from the two that carry it.
3. Rebuild the `std` farm (`xtask std-src`). `nife-dev` is one `rustup toolchain link` per user
   account, so this is machine-global and has to be sequenced against other lanes; notes/std.md has
   the 2026-08-18 cross-contamination that makes that a rule rather than a courtesy.
4. Rebuild every user program and every archive.

**The kernel stays `softfloat`, and should.** A kernel that emitted FP into its own fastpath would
pay the register-file save on every switch instead of only on the switches of threads that asked
for the unit, which is the whole of 447's cost argument.

## What it would buy, honestly accounted

- **One flag disappears.** `--cfg aes_force_soft`, in `.cargo/config.toml`'s
  `[target.x86_64-unknown-none]` block, is the only force-soft flag in this tree; `grep` finds no
  other. The block of milestone 164 (x86_64 userspace can't build `aes`: no SSE, no scalar
  fallback) and the framing this proposal inherited both suggest a family of
  them. There is one, and saying so weakens the case rather than strengthening it, which is why it
  is the first line here.
- **AES-NI instead of a bitsliced software backend**, roughly an order of magnitude by upstream
  RustCrypto's own figures. **Still unmeasured, and 164's refusal to measure it still stands**:
  nothing on x86_64 mounts an encrypted RedoxFS volume, so there is no workload and a synthetic
  number would be a fact leaving the machine with nothing behind it. What 447 changed is that the
  number is obtainable rather than blocked.
- **Possibly the second failure class of milestone 442 (a crypto provider `rustls` can use on all
  three bare-metal targets)**, and this is the one worth measuring first; see below.
- **The seam of §31 (the foreign-language seam: C holds no capabilities and makes no syscalls)
  widens.** A C component
  compiled by bare-metal clang currently cannot touch a vector register at all, and §31 says why:
  "the kernel never enables FP/SIMD for EL0, and the context switch" saves nothing, so such a
  register would be "a trap or a corruption depending on which of those two bit first". After the
  flip that sentence stops being true, which is a real gain for the vendored-component rung the
  seam exists to de-risk.

## What it would cost

Every thread that executes an FP instruction takes one trap and then saves and restores the whole
register file on **every subsequent switch for the rest of its life**, because 447's `live` flag
never clears (its first `BUGS` entry). Today that costs nothing, because no thread in this tree has
ever taken the trap and `crate::fp::ENABLES` is zero on every boot. A hard-float userspace means
`memcpy`, `std` formatting and anything LLVM feels like vectorising will take it, so **most threads
become live and 447's common case stops being common**. 512 bytes each way per switch, against a
switch that costs about 565 instructions on aarch64 today.

Nothing measures that, and nothing can until there is a hard-float userspace to run.

## The two measurements that should come first, and neither needs the decision made

1. **Re-measure 442's soft-float failures against nife's own targets.** Its block
   records that `sha2` and `polyval` "fail on soft-float x86_64", met through `embedded-tls`, and in
   the next sentence warns that the probe behind that finding "ran against stock bare targets on the
   stable host toolchain, not against nife's own target specifications and pinned nightly". 442 is
   NOT-STARTED and already owes this re-measurement as the first item on its own list. If those
   crates build against our targets, this half of the flip's case evaporates; if they do not, 442
   hands over the specific list the flip would have to fix. Cheap, and it is 442's work rather than
   a new lane's.
2. **Price a live thread's switch.** Build one user program hard-float against a scratch target
   (not the committed ones), run `script/bench`'s `yield_switch` and `ctx_switch` with it in the
   mix, and report what a switch costs when both threads are `live`. That turns the paragraph above
   from an argument into a number, and it is the number the flip is actually deciding.

## What this proposal deliberately does not do

It does not recommend. The *fork reaches calef with its questions already answered* tenet asks for
options and costs on an irreversible fork and explicitly **not** for a winner, because "a
syscall-surface decision arriving with a recommendation is already most of the way made" and an ABI
is the same shape of thing.
