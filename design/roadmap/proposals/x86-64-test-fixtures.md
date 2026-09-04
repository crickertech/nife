# Most x86_64 tests take a "no RedoxFS disk attached" arm, because the runner attaches little

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 215's block.

**Gate: NONE.** The blocker is gone. Milestone 215 made a PCI function's interrupt reach a userspace
driver on x86_64, and it did the whole thing on patagonia under QEMU's `q35`, so nothing here waits
on xenon.

**In brief.** `scripts/qemu-runner-x86_64.sh` starts a much barer machine than the aarch64 and
riscv64 runners do. The RedoxFS image, the GPT and blank disks, the NIC, the GPU, the keyboard and
the RNG each need a line in that script plus the wiring behind it. The first piece is making the FS
server's disk lookup transport-blind, since it currently assumes the transport the other two
architectures use. The measure of done is the count of tests taking a "no RedoxFS disk attached"
arm, which is 36 today.

## Why this matters

Architectural parity is a gate rather than an aspiration (DECISIONS §19): a kernel capability ships
on every supported architecture, proven by the same suite, or a scope note records the gap.
Thirty-six tests that pass by taking an early-exit arm are the worst version of that, because they
are green. The suite reports a passing x86_64 leg while a third of the interesting surface is
untested there, and nothing in the output distinguishes "this works on x86_64" from "this was
skipped on x86_64".

The specific risk is a defect that only x86_64 has and that only the fixtures would find. Milestone
215 is the existence proof: PCI interrupt routing reached nothing on x86_64, no userspace driver
could run there, and the suite was green the entire time because nothing on that leg ever asked a
device for an interrupt. Every fixture still unattached is another arm of that same blind spot.

## Where it came from

Milestone 215 (a PCI function's interrupt reaches nothing on x86_64) named it as the natural next
step from what it built: *"Attach the rest of the x86_64 test fixtures now that a function's
interrupt works: the RedoxFS image, the GPT and blank disks, the NIC, the GPU, the keyboard and the
RNG, each a line in `scripts/qemu-runner-x86_64.sh` plus its wiring, starting with making the FS
server's disk lookup transport-blind. The measure is the 36 tests taking a 'no RedoxFS disk
attached' arm."*
