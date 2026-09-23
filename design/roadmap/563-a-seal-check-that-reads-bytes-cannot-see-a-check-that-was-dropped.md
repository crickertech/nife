# 563. A seal check that reads bytes cannot see a check that was dropped

**Status: NOT-STARTED.** The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `a-seal-check-that-reads-bytes-cannot-see-a-check-that-was-dropped` on 2026-09-22, filed 2026-09-21. Raised by the lane of milestone 523 (moving the job-mix supervisor into userspace, and the five permissions it turns out to need), which was launched because `script/board-image --job-mix --tftp` reported `NOT SEALED`
on a card whose kernel and archive were packed by one command. The cause is not the pair and not the
seal's arithmetic: the kernel image genuinely does not carry the digest, because the build that
diverts the boot tour never calls the code that would check it, and the linker removed it.

**Gate: NONE.** No hardware, no new dependency. The wording half could start today; whether the tool
should refuse such a build at all is a small judgement call, priced below.

## What was measured

In a lane worktree, alternating one cargo feature and counting the refusal message in the resulting
riscv64 release kernel:

```text
board          -> 3 occurrences of `MEASURED BOOT REFUSED`
board,job_mix  -> 0
board          -> 3
```

Since milestone 268 (every architecture boots the same way: describe the machine, test yourself, hand over) the
measured-boot refusal lives in `user::boot_progenitor`. `kernel/src/main.rs` reaches it only under
`#[cfg(not(any(feature = "soak_test", feature = "job_mix")))]` (`:1640`, `:2018`, with
`riscv_hand_over`'s own `cfg_attr` at `:2195`), and `bench` diverts at the same sites. So a `soak`,
a `job_mix` or a `bench` card **verifies nothing**, the trust root is unreferenced, and it is dead
code.

## Why the tool then says the wrong thing

`sealed_pair` decides whether a kernel vouches for an archive by scanning the image for the
archive's digest: `haystack.windows(needle.len()).any(|w| w == needle)`,
`crates/sealed_pair/src/lib.rs:445`. That is a question about **bytes present**, and the question a
reader thinks they are asking is **will this kernel check**. The two agree on every default build
and come apart on every diverted one.

The message it then prints (`:257`) asserts two things that are both false for such a card: *"they
are from different builds"*, and *"this pair halts at MEASURED BOOT REFUSED after the power cycle"*.
The pair is from one build, and the kernel halts at nothing.

**The cost is real and was paid.** An hour before a bench evening on radon, a maintainer rebuilt the
card, ran `cargo clean -p kernel`, deleted the build directory, and got the same refusal each time,
because nothing in the message pointed at the feature.

**And the inverse is the security-shaped case.** The tool reads "digest present" as "will refuse".
Any future build that carries the digest for some other reason while not reaching the check would
read as **SEALED** while verifying nothing. That is the direction worth fixing even though nothing
in the tree does it today.

## Three options

**A. Say what is actually true, in the message.** When the image carries no trust root at all,
distinguish *"this kernel and this archive are from different builds"* from *"this kernel contains
no measured-boot check; it will accept any archive"* and name the features that do that. Cheapest,
forecloses nothing, and turns an hour of rebuilding into a sentence. This is the recommendation.

**B. Have the diverted builds keep the check.** Call `boot_progenitor`'s verification before the
workload replaces the tour, so a soak, bench or job-mix card refuses a mismatched archive like any
other. It is the honest position (a card left running unattended for hours is the last one that
should skip verification) and it is a change to what three kernel features do, which is a larger
claim than a tool fix.

**C. Make the seal ask the real question.** The scan cannot know whether a symbol is reachable, so
answering *"will this kernel check"* means the build emitting a fact about itself (a section, or a
line in the manifest `board-image` already writes). More machinery, and it is the only option that
closes the inverse case in general rather than by inspection.

## What is already recorded, so this is findable without promotion

`notes/job-mix.md`'s `BUGS` and `crates/sealed_pair`'s each carry the limitation where a reader
meets it, per DECISIONS §71 (a limitation is promoted when it stops being a fact and becomes a
plan). This proposal is the plan half; the trigger for promoting it is a second person losing time
to the same message, or anyone wanting option B's guarantee for an unattended card.

## Index row

A `soak`, `job_mix` or `bench` card verifies nothing: the build that diverts the boot tour never calls the measured-boot refusal, so the linker removes it and the trust root is unreferenced. Alternating one cargo feature and counting the refusal message in the resulting kernel showed three occurrences becoming zero. `script/board-image` then reports `NOT SEALED` on a card whose kernel and archive were packed by one command, which is true about the bytes and wrong about the cause, and a check that reads bytes cannot see a check that was compiled out.
