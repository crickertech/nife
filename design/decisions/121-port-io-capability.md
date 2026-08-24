# 121. What a device capability is when the device has no page: x86 port I/O

**Status: PROPOSED.** Raised 2026-08-23 by milestone 161's lane, which found it while wiring the
x86_64 console and could not decide it: what a capability *is* is the centre of this system's claim,
not an implementation choice a lane makes on the way past. The number is **provisional**, minted by
a lane against the current README rather than by an integrator.

**What is blocked: nothing today, and one thing soon.** Milestone 161's boot is entirely in ring 0,
so the kernel drives COM1 directly and no grant is needed. What is blocked is a **userspace console
or input driver on x86**, which is the arrangement both other architectures already have and one of
the things "parity" means (DECISIONS §19). `user::UART_PHYS` is **zero** on x86_64 today, and that
zero is a marker for this file rather than an address.

## The rule this collides with

On aarch64 and RISC-V, **a device is a page**. A driver holds a mapping with
`paging::Flags::user_device()`, its user-mode stores go straight to the hardware, and the MMU is what
enforces that it can touch that device and no other. That is the mechanism behind the whole
userspace-driver claim: the console server holds the UART's registers as a capability, and nothing
else in the system can reach them.

**x86's legacy devices are not in memory at all.** The 16550 COM ports, the PIT, the 8259 PICs and
the CMOS clock live in a separate 16-bit I/O address space reached only by the `in` and `out`
instructions. There is no page table in front of it, so there is nothing for a mapping to be *of*,
and `Flags::user_device()` has no meaning there.

This is not an x86 quirk to route around. It is the first case in this tree where **the object a
capability names is not memory**, and how it is answered decides the shape of every future
capability over something that is not a frame.

## What x86 actually provides

Two mechanisms, and only one has usable granularity.

- **`RFLAGS.IOPL`**, two bits in the flags register: ring 3 may use `in`/`out` on *every* port, or on
  none. All-or-nothing. Granting it is granting the whole machine, including the interrupt
  controller and the CMOS clock, which is not a capability in any sense this project means.
- **The TSS I/O permission bitmap**: one bit per port, 8 KiB for the full 64 Ki ports, at an offset
  the TSS names. The CPU consults it on every `in`/`out` from ring 3 and faults when the bit is set.
  This has exactly the right granularity, and it is the only thing that does.

The bitmap's awkwardness, and the reason this is a decision rather than a task, is that it is
**per-task state, not per-address-space state**. A page mapping lives in the address space and
travels with it; the I/O bitmap lives in the TSS, and the TSS is **per-CPU**. So a thread's port
rights have to be re-established on every context switch that changes which thread is running, on
whichever CPU it lands on. That is a cost and a synchronisation problem the mapping model does not
have.

## The options

**Option 1: a port-range capability, enforced by the TSS bitmap.** A new capability type naming a
`(base, count)` range of ports. Granting it sets bits; revoking clears them; the scheduler writes the
holder's bitmap into the current CPU's TSS on switch-in.

- *For*: it is the honest analogue of the mapping model. The same "hold it or you cannot touch it"
  property, enforced by hardware, at the granularity a real driver needs (a 16550 is eight
  consecutive ports).
- *Against*: a new object type on the capability surface, which is the expensive kind of decision
  (§10, §16). It puts an 8 KiB copy per address space, or a shared bitmap plus a rewrite on switch,
  on the context-switch path, which is the path milestone 95 spent a lot of effort making fast.
  Revocation has to reach every CPU that might hold a stale bitmap, which is a shootdown protocol of
  its own.

**Option 2: keep legacy devices in the kernel on x86, and grant only MMIO devices.** The console
stays a kernel driver on this architecture; PCI devices, whose BARs *are* memory, get the existing
mapping-based capability and work identically on all three architectures.

- *For*: no new capability type, no context-switch cost, no shootdown. Everything modern is
  memory-mapped anyway, so the userspace-driver claim still holds for every device that matters on a
  machine built after about 1995, including everything on milestone 87's OptiPlex except the serial
  port.
- *Against*: it is a **parity gap with a scope note**, not parity. The one device the tree
  demonstrates userspace drivers with is exactly the one that would stay in the kernel, so the x86
  demo would be weaker in precisely the visible place. And it puts a device driver back in the kernel
  after milestone 8 spent the effort taking one out.

**Option 3: an I/O-port *service*.** A kernel-side port broker holding the ranges, with userspace
drivers issuing reads and writes as IPC.

- *For*: no new capability type in the syscall surface; the grant is an endpoint, which the system
  already has; revocation is endpoint revocation, which already works.
- *Against*: it is a syscall per register access, on a device whose driver polls a status register in
  a loop. Measured on nothing yet, but the shape is obviously wrong for a UART, and it reintroduces
  the kernel as the thing that actually touches the hardware, which is the arrangement being argued
  against.

## The recommendation, and it is deliberately weak

**Option 2 now, with the scope note written where a reader meets the feature, and option 1 kept open.**

The reasoning is ordering rather than preference. Nothing on x86 runs in ring 3 yet, so option 1
cannot be built or measured today, and its costs (the context-switch write, the revocation
shootdown) are exactly the kind this tree measures rather than argues. Option 2 is the state the port
is already in, costs nothing to keep, and is honest as long as it is *recorded* as a gap rather than
presented as a design.

**What would change the recommendation**: a userspace console on x86 becoming a thing calef wants
demonstrated, rather than a thing the other two architectures happen to have. At that point option 1
is the only answer, and it should be built as a capability rather than bolted on.

**This is not a lane's call** because option 1 adds an object type to the capability surface and
option 2 declares a parity gap, and both are calef's under the tenets: the syscall surface is a
boundary, and a parity gap needs a recorded plan rather than a silence.

### Amended 2026-08-24: option 2 now, but not argued blind against the alternatives

calef asked whether option 2 can be built so there is something to measure against if a later call
goes to option 1 or option 3, rather than deferring the whole question to argument again when it
comes back. It can, and neither half needs the feature it is pricing.

**Option 3's cost is already on record, just never cited here.** The dominant cost of an I/O-port
broker is one IPC round trip per register access, and this kernel's own IPC round trip is measured:
**~337 ns** (release, HVF, aarch64), against a **null syscall at ~27 ns**, both from
`notes/benchmarks.md`'s cross-OS table. Neither number is x86-specific, and citing it here is not a
claim that it transfers unchanged to different silicon; it is the order-of-magnitude confirmation
this file's own "obviously wrong for a UART" already asserted without a number. A raw `in`/`out`
instruction is single-digit cycles; hundreds of nanoseconds per register access, on a device a driver
polls in a tight loop, is the gap that makes option 3 wrong regardless of which architecture supplies
the exact figure.

**Option 1's cost is not measurable yet, and the reason is narrower than "nothing runs in ring 3".**
The dominant cost is the TSS I/O bitmap write on every context switch, and that does not need the
capability type built to measure: a micro-benchmark that writes an 8 KiB bitmap into the current
CPU's TSS on every switch, timed against a switch that does not, is a far smaller thing than option 1
itself, in the same shape `script/bench`'s existing icount-based measures already take. What it needs
that does not exist yet is **real context switching between two threads on x86_64**, which is
downstream of ring 3 (this milestone's item 3, in a lane as of 2026-08-24) but not necessarily
delivered by it: that lane's own proof may run only one program in ring 3, not switch between two.
Whether it does is this decision's own next input, not something to guess now.

**So the plan, not just the recommendation:** option 2 stands, and once x86 has two threads actually
switching, a small follow-on benchmark (unscoped as its own milestone; whoever picks it up mints the
number) measures the TSS-bitmap-write cost the same way `ipc_rtt` and `null_syscall` already measure
theirs. That turns the next 1-vs-3 call into one made on both sides' numbers, not one side's.

## What is needed to answer it

One question, and it is not technical: **is a userspace console driver on x86 part of what the
demonstrator claims, or is "userspace drivers, demonstrated on two architectures, with the third's
legacy serial port a recorded exception" enough?**

If the first, option 1 and it should be scheduled. If the second, option 2 and this file becomes
`DECIDED` with the scope note as its deliverable.
