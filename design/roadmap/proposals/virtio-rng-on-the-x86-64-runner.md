# The x86_64 runner has no RNG, so four of six NTP tests skip there

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 176's block.

**Gate: NONE.** It is a line in `scripts/qemu-runner-x86_64.sh` plus its wiring, and both the device
and the client exist on the other architectures already.

**In brief.** Attach a virtio-rng function to the x86_64 test runner and wire it through, so the
NTP client has a nonce source on that architecture. Without one, four of the six tests in
`ntp_tests.rs` skip on x86_64.

**This is one item inside a larger piece of work, and it should be folded into it.** Milestone 176's
own bullet says so: milestone 215 proposes attaching the rest of the x86_64 test fixtures (the
RedoxFS image, the GPT and blank disks, the NIC, the GPU, the keyboard and the RNG) as one lane, and
the RNG is one line of it. That larger lane now has a proposal of its own,
`design/roadmap/proposals/x86-64-test-fixtures.md`, written by the same sweep. This file exists so
the item is not lost while that larger proposal is written; whoever promotes either should merge the
two rather than run two lanes at the same fixture file.

## Why this matters

A skipped test reads as a passing suite. Parity is a gate in this project, not an aspiration, and
the specific failure it guards against is a feature that works on one instruction set architecture
and silently not another. Four skipped NTP tests on x86_64 is that state today, and the reason is
not that the NTP client is wrong on x86_64, it is that the runner never gave it entropy. Nobody can
tell those two apart from the outside, which is what makes the gap worth closing rather than
recording.

The cost is small and the payoff is proportional: one device on a command line, and four tests
change from skipped to proving something.

## Where it came from

Milestone 176's Follow-on: *"Attach a virtio-rng function to the x86_64 test runner and wire it, so
the NTP client has a nonce source there; four of `ntp_tests.rs`'s six tests skip on x86_64 without
one. Milestone 215's block proposes this as one item in a larger x86_64 fixture lane, so take it
there rather than as a second piece of work."*

Milestone 215's Follow-on names the larger lane: *"Attach the rest of the x86_64 test fixtures now
that a function's interrupt works ... each a line in `scripts/qemu-runner-x86_64.sh` plus its
wiring, starting with making the FS server's disk lookup transport-blind. The measure is the 36
tests taking a 'no RedoxFS disk attached' arm."*
