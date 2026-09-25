# The booted system has no network, so nothing at the prompt can fetch a package

**Status: PROPOSED 2026-09-24.** Raised by milestone 198 (a package manager, and the trivial install
that makes a second customer possible)'s rung 3a consumer lane, which built a package fetch through
`net_stack` and found that it can only run in the kernel's test harness. **Name provisional**: this
file's stem is a lane's coinage.

**Gate: DECISION.** Who holds the network card and the `Stack` endpoint in the booted system is an
authority question: which process the progenitor builds `net_stack` for, which capability a program
at the prompt is given to reach it, and what that costs the progenitor's sixteen-slot capability
table, whose directory-grant spawn already peaks at fifteen.

## The finding

`crates/system_initializer` builds no `net_stack` (its source never names it), and its
`BootEndowment` carries no network device. Every network test in the tree (DHCP, UDP, TCP, the
inbound listener, and rung 3a's package fetch) starts `net_stack` from the kernel test harness
(`kernel/src/user/virtio_service.rs`, `start_net_stack`). So a person at the prompt has no program
that can reach the network at all, whatever DECISIONS §215 (how the shell names an installed program
to the spawner) decides.

## The shape an answer probably has, and why it is not decided here

The tree already hands ambient services to a program by a manifest flag the progenitor reads:
`clock`, `entropy`, `domain` and `config` in `grant_plan::Manifest`. A `network` flag giving `WRITE`
on the `Stack` endpoint would be that pattern once more. Three things make it a decision rather than
a lane's call:

- **The slot budget.** A resting `Stack` capability in the progenitor is one more of sixteen. The
  directory-grant spawn's peak is fifteen, so the two together would sit at the wall.
- **The NIC is a device.** Which process holds it, and behind which IOMMU, is the confined-driver
  question fatal risk 6 is about, applied to the booted system rather than to a test.
- **Listen grants.** `net_stack` takes its inbound port authority from whoever spawns it
  (milestone 107). A booted system has to say what that is, and the answer for a package client is
  "none", but the answer for the system is not obvious.

## Exit criterion

At the prompt of a booted aarch64 system under QEMU, a program whose manifest asks for the network
fetches rung 3a's package from `scripts/package-http-peer` and prints its verdict; one without the
flag is refused the capability. riscv64 as the twin; x86_64 has no NIC under QEMU today (its runner
attaches no `-netdev`), which is a scope note until milestone 494 (a driver for the network card a
PC actually has).

## Index row

The progenitor builds no network stack, so nothing at the prompt can fetch a package; every network
test runs from the kernel's harness. Proposed: a `network` manifest flag in the `clock`/`entropy`
pattern, gated on the slot budget and on who holds the NIC.
