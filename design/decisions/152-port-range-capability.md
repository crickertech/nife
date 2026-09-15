# 152. The port-range capability: object and method semantics on the syscall surface

**Status: PROPOSED.** 2026-09-15, milestone 299. **Number provisional**, minted by a lane against the
current README; the integrator renumbers if the merge queue collides it. This section records the one
thing milestone 299 puts on the capability surface, because a new object type there is calef's to
ratify (§10, §16) and the *move fast on what can be undone* tenet files the syscall surface under the
irreversible category. DECISIONS §121 decided *that* x86 gets a port capability and *why* (the
reversal at the end of that file); this decides *what the object is and what invoking it does*, the
part §121 named but left for the milestone.

## What it names

`Object::PortRange(u16, u16)` names a contiguous run of x86 I/O ports, `(base, count)`. It is the
device-capability kind for the one class of device with no page: x86's legacy port-I/O hardware,
reached only by `in`/`out`, which the MMU cannot grant or deny because there is no page table in
front of the port space. It is the honest analogue of `Object::DeviceFrame`, which names a device's
MMIO page and is enforced by the MMU; `PortRange` names a device's ports and is enforced by the
CPU's **TSS I/O permission bitmap**.

- **The granularity is a range**, because a device occupies a run of consecutive ports (a 16550 UART
  is eight: COM1 is `0x3F8..=0x3FF`). A capability names what the hardware names, the port-space twin
  of a `DeviceFrame` naming a page, or a `PageFrame(base, count)` naming a run of frames.
- **`(u16, u16)`**, because a port number is 16 bits (`in`/`out` address exactly 64 Ki ports) and a
  count never exceeds that. The pair fits the capability enum's existing 16-byte payload, so the
  object does not grow a capability-table slot.
- **It is minted by the kernel**, once, at boot, the way a `DeviceFrame` is: only the kernel knows a
  machine's device ports. The progenitor is handed COM1's `PortRange(0x3F8, 8)` with `GRANT` and
  delegates it to the console and input drivers it builds.
- **The object exists on every architecture** for a uniform capability surface and a uniform syscall
  dispatch (§19), the same way `DeviceFrame` does; it is constructed and enforced only on `x86_64`,
  where the hardware it names exists.

## What invoking it does

Almost nothing, on purpose: enforcement is at the context switch, not on the syscall path, so a
holder never invokes the capability to *use* the ports. It executes `in`/`out` directly, and the
kernel, on switch-in, writes the holder's ports into the current core's TSS bitmap so those `in`/`out`
are permitted from ring 3 and no others (the lazy write; see §121's 2026-08-25 refinement and
`notes/benchmarks.md`). A thread that holds no `PortRange` may touch no port at all.

The one method is **`PortRange::REVOKE`** (`abi::port_range::REVOKE`, method number 1), the take-back a
live driver replacement needs, with exactly `DeviceFrame`'s asymmetric semantics and for the same
reason: it deletes every `PortRange` capability naming this range from every **other** thread and
clears the cached grant, so those holders fault on their next `in`/`out`, while the caller keeps its
own. It needs `GRANT` (you were trusted to lend the ports on, so you may take them back). The
asymmetry is forced: the kernel mints a port capability once, at boot, so a symmetric revoke would
strand the device forever. No other method exists; `MAP` has no meaning for an object with no page.

## What it does not do

- **It does not replace `DeviceFrame`.** This is a *second* device-capability kind, for the one class
  with no page; PCI/PCIe devices, whose registers are memory, are unchanged and keep the mapping-based
  capability on all three architectures.
- **It does not add a syscall number.** `REVOKE` is a method on the existing `SYS_INVOKE` surface,
  within the established capability model, which is the bar §10/§16 set for a new method.
- **It does not give a way to *acquire* a port from userspace.** A `PortRange` is minted by the kernel
  and delegated (`CAP_INSERT`), narrowing rights, exactly like `DeviceFrame`; there is no retype-into-
  a-port, because a program cannot conjure hardware it was not handed.

## BUGS

- **A thread caches one port range, not a set.** The grant the context switch reads
  (`Thread::port_grant`) is set at the one choke point a `PortRange` enters a thread
  (`thread_control_block_insert_cap`) and holds the *last* range inserted. Every real consumer holds
  exactly one (a console driver holds COM1), so this is within the design's pinned consumer, but a
  thread handed two disjoint ranges would reach only the second. Widening it to a small set is a
  future change with an obvious shape; it is not built because nothing needs it.
- **A `PortRange` delegated to an already-running thread by `SEND_CAP` is not cached.** The grant is a
  creation-time fact, set when an embryo is endowed, the same posture `cycle_counter_grant` takes.
  Runtime delegation to a live thread would leave the switch installing the old grant until the next
  insert-through-the-choke-point. No consumer does this.
- **Revocation reaches one core.** x86 runs a single core today (`smp::bring_up_secondaries` refuses
  on `x86_64`), so revocation's TSS reset is core-local and the stale-bitmap window a multi-core
  machine would have does not exist. The core-local reset is written as the step a future SMP x86
  would broadcast by IPI, the shape of the TLB shootdown this tree already has, so the generalization
  is a broadcast rather than a redesign.
