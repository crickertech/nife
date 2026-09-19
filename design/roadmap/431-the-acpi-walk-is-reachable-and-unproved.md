# 431. The ACPI walk is reachable by the prover for the first time, and proved by nothing

**Status: PROPOSED 2026-09-18.** Split out of milestone 304's `## Follow-on`, which named this work
against the proposal 304 was itself promoted from. That proposal is discharged, so the work it
carried had no home; this is the home.

**Gate: NONE.** A lane can start by reading the seam and proposing where it falls. Nothing has to
build or boot first, and the decision it reaches is a lane's own: where to cut a proof boundary is
reversible, and no wire format, name or syscall is on the other side of it.

**In brief.** Milestone 304 put `kernel/src/arch/x86_64/` in front of the prover for the first time.
Its strongest target went with it and is **still unproved**: `x86_64/machine.rs`'s ACPI walk, 836
lines reasoning over firmware-supplied lengths, checksums and counts, which is the shape of input a
prover is for and the shape of input a machine will lie about.

**Why it is not simply a harness**, which is what makes it a lane rather than an afternoon. The walk
is two things wearing one name. Its parsing half already lives in `crates/machine_discovery`, which
carries no harnesses and appears in no `script/verify` row, so it is pure logic that nothing proves.
Its volatile half reads raw pointers into the direct map, which Kani cannot follow. **The work is
finding where those two separate**, and that seam does not exist yet; proposing it is most of the
job, and writing the harnesses is the smaller half that follows.

**What it is worth.** `design/fatal-risks.md`'s risk 2 is that the proofs prove trivia while the
real bugs live where Kani cannot reach. Firmware-supplied lengths and counts are the least trivial
input this kernel takes, and milestone 87's first light on xenon already turned up one machine
whose firmware map did not describe the MMIO hole. This is the risk-2 frontier with the door newly
open.
