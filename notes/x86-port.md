# Porting to x86_64

Milestone 161, the third architecture, and the one milestone 20 named as the real test of whether
`arch/` is a hardware abstraction layer or an accident of two similar RISC machines. aarch64 and
RISC-V share a device tree, weak memory ordering, a boot handoff in the width the kernel runs in,
and a page-table shape. x86_64 shares none of those.

This note is what the port learned, in the order a reader needs it: how the machine is entered, what
had to change above `arch/` (very little, and the exceptions are listed), the three things that
genuinely do not fit the existing seam, and an honest account of what is built.

## Status, and it is a partial port

**Built, running, and gated:** the boot path, the console, the GDT/TSS, the IDT and trap frame, the
page-table format, the boot-handoff parser, the ACPI tables (the RSDP scan, the root-table walk with
checksums, the MADT and the MCFG), **the local APIC and a calibrated periodic timer**, the frame
allocator, **the fine-grained W^X kernel page tables**, **the IO APIC and a routed device line**,
**user address spaces, the `syscall` pair and ring 3**, **the scheduler, preemption, kernel threads
and real ring-3 processes**, **the kernel's own test suite and a `script/test` leg**, the address
arithmetic, interrupt masking, the context switch, and the test exit. A boot under QEMU's `q35`
prints a tour, takes real hardware interrupts from the CPU's own timer *and* from a device, builds
and installs its own page tables, brings up the scheduler, runs a kernel thread, builds two
processes out of untyped memory and runs them at CPL 3 (one invokes a capability and exits, one
faults and is delivered to its supervisor), and halts.

**And since 2026-08-24, userspace:** every program in `user/` compiles for `x86_64-unknown-none`,
`xtask` packs the same archive RISC-V's leg does, QEMU's PVH loader hands it over as a module, and
`cfg(initrd)` is on. `script/test --arch x86_64` runs **170 tests and skips 67**, where it ran 97
and skipped 7 the day before.

**Not built:** VT-d and SMP bring-up, both loud `unimplemented!()`s in `arch/x86_64/` that name
themselves and why. What bounds this architecture now is not userspace but **devices**: no PCI bus
is enumerated, so no virtio function of any kind is found, and the console UART is in an I/O port
space with no capability shape yet (DECISIONS §121). See "What a userspace still does not have here"
below, and design/roadmap/161-x86-64-kernel-port.md for the order the rest comes in.

## How the machine is entered, and why it is not multiboot

`-kernel` on x86 wants a Multiboot 1 image, and **Multiboot 1 cannot boot a 64-bit kernel**: the
specification says the OS image is ELF32 and QEMU enforces it, refusing the image with

```
qemu-system-x86_64: Cannot load x86-64 image, give a 32bit one.
```

Multiboot 2 lifts that restriction and QEMU 11 does not implement it (its x86 loader is one file,
`hw/i386/multiboot.c`, version 1 only). The header is *fatal* rather than ignored, so an image
cannot carry one and fall back to something else.

So this kernel boots by **PVH**, the direct-boot protocol Xen defined and which QEMU, Firecracker
and cloud-hypervisor all implement. Its whole interface is one ELF note:

| Field | Value |
|---|---|
| name | `Xen` |
| type | 18 (`XEN_ELFNOTE_PHYS32_ENTRY`) |
| descriptor | the physical address to enter at |

QEMU walks the **PT_NOTE program headers** to find it, which is why `link-x86_64.ld` gives
`.note.Xen` an output section of its own: lld only emits a PT_NOTE for an output section whose type
is SHT_NOTE, and folding the note into `.boot` would make that section PROGBITS, produce no PT_NOTE,
and leave QEMU reporting nothing more informative than a guest that never runs.

PVH turns out to be a better fit than Multiboot quite apart from the ELF64 question, because of what
rides in `ebx`. `hvm_start_info` carries the memory map **and the ACPI RSDP address**, which is the
root of every table x86 uses in place of a device tree. One pointer in, everything discoverable from
it: exactly the shape `kernel_main(dtb)` already has.

### And real firmware, since 2026-08-30

**PVH is a hypervisor protocol and no real firmware speaks it**, which is what milestone 87 was for.
The answer turned out to need no change to this file's subject at all: `uefi_loader` is a UEFI
application that places this kernel at its `p_paddr`, builds an `hvm_start_info` out of what the
firmware knows, leaves long mode, and enters **the same `_start`** with the same register contract
QEMU's PVH loader delivers. So there is one entry point and one handoff structure rather than two,
which is why the suite above cannot regress under it.

What that buys, beyond a bootable stick: four code paths on *this* side of the boundary had never
executed, because a hypervisor never took them. `rsdp` arrives non-zero, so `find_rsdp`'s BIOS-area
scan is skipped; the RSDP is revision 2 with an **XSDT** root rather than revision 0 with an RSDT;
the MCFG's ECAM window is `0xe0000000` rather than the `0xb0000000` that `arch::mmu::PCI_ECAM_PHYS`
also happens to say, so "read the table" is finally distinguishable from "used the constant"; and
the memory map is **118 regions** rather than nine. See notes/x86-uefi-boot.md, which also carries
the bench procedure for the OptiPlex.

### BUGS

- **Multiboot is still not an option, and nothing here added a header.** The refusal above is a
  property of QEMU's loader rather than of this kernel, so it stands. GRUB Multiboot 2 remains the
  path for a BIOS-only machine and would cost a header plus a second handoff decoder; it was priced
  against UEFI and lost on testability (`brew info grub`: no formula on this machine at all). See
  notes/x86-uefi-boot.md's fork.

## The trampoline, and why this file is three times its RISC-V twin

PVH (and Multiboot, and GRUB) enter in **32-bit protected mode**. A 32-bit instruction stream cannot
name a 64-bit address at all, so unlike the other two architectures the image cannot simply be
linked high and use PC-relative addressing until the MMU comes on. The image has two worlds:

1. `.boot`, linked at its **physical** address (VA == PA, low), holding the PVH note, the 32-bit
   trampoline, the boot GDT and the boot page tables.
2. Everything else, linked **high** at `KERNEL_VA_BASE + PA`, with `AT()` telling the loader where
   the bytes go.

The trampoline zeroes the boot page tables (they are `NOLOAD`, so nothing else has), fills them,
turns on PAE then CR3 then EFER.LME then CR0.PG in that exact order, far-jumps through a code
descriptor whose `L` bit is set, and only then jumps to the high alias of itself.

**`KERNEL_VA_BASE = 0xffffffff80000000` is not a free choice.** `x86_64-unknown-none` has
`code-model: kernel`, which promises LLVM that every symbol is in the top 2 GiB of the address space
so it may use the sign-extended 32-bit relocations that make kernel code compact. Moving the base
breaks every relocation in the image, silently, in a way that looks like random corruption.

The boot map is 4 GiB identity-mapped with 2 MiB pages, the first gigabyte aliased at
`KERNEL_VA_BASE` (where the image is linked), and the same 4 GiB aliased a third time at
`DIRECT_MAP_BASE` (where `phys_to_virt` points; see "Two bases" below). 4 GiB rather than the 1 GiB
the image needs, because everything x86 talks to early is above 1 GiB and below 4: the local APIC at
`0xfee00000`, the IO APIC at `0xfec00000`, and q35's PCIe ECAM window at `0xb0000000`. The identity
map is dropped by `mmu::init`; the other two survive it, and the direct-map alias costs eight bytes
because it points at the page directories the identity map already built.

**When it fails, it fails silently.** A wrong page table means the instruction after `mov cr0, eax`
is fetched through a broken mapping, the CPU takes a page fault with no IDT, escalates to a double
fault with no IDT, and triple-faults, which on QEMU is a machine reset with no output. The
diagnostic is `-d int,cpu_reset -no-reboot`, which prints the full register state at each
escalation.

## What had to change above `arch/`

This is the part that matters for the milestone-20 claim, so it is stated as a list rather than as a
conclusion. Making the entire kernel compile for a third architecture took **42 compiler errors**,
every one of them "this `arch::` name does not exist yet", and:

- **`crates/paging` did not change at all.** `paging::x86_64::Ia32e` is sixty lines of bit encoding
  behind the existing `PageFormat` trait; `LEVELS = 4` and `SPLIT_SHIFT = 47` were the whole of the
  geometry, and the shared `Mapper` walk needed nothing.
- **`drivers/ns16550.rs` gained a type parameter and no second driver.** The same 16550 QEMU's
  RISC-V `virt` puts at physical `0x1000_0000` is, on every x86 machine, at **I/O port** `0x3f8`: a
  separate address space reached only by `in`/`out`. That is a difference in how eight registers are
  *reached* and in nothing else, so it is a `RegisterSpace` implementation (defaulting to `Mmio`, so
  every existing use means what it always did) with the port-space half under `arch/x86_64/`.
- **`console.rs`, `user.rs`, `drivers/mod.rs` and `user/fs_service.rs` gained `cfg` arms**, in the
  same places they already had two.

That is the whole diff above `arch/`. A new ISA was a new directory.

## What the loader hands over, and what it does not

`machine_discovery::x86_64` decodes `hvm_start_info`, host-tested, for the same reason `crates/dtb`
exists rather than a device-tree reader living in `arch/aarch64/`: a parser proved only inside a
booting kernel is proved by nothing that runs in milliseconds. The kernel side
(`arch/x86_64/machine.rs`) does nothing but turn a physical address into bytes through the direct
map.

What q35 actually produces, read back out of the guest on 2026-08-23 with `-m 256M`:

```
  memory      : 9 regions from the PVH handoff, rsdp 0x0
                0x000000000000..0x00000009fc00  ram
                0x00000009fc00..0x0000000a0000  reserved
                0x0000000f0000..0x000000100000  reserved
                0x000000100000..0x00000ffdf000  ram
                0x00000ffdf000..0x000010000000  reserved
                0x0000b0000000..0x0000c0000000  reserved
                0x0000fed1c000..0x0000fed20000  reserved
                0x0000fffc0000..0x000100000000  reserved
                0x00fd00000000..0x010000000000  reserved
                usable ram: 261627 KiB
```

Two things worth taking from that beyond the RAM.

**`0xb0000000..0xc0000000` is the PCIe ECAM window**, reported as reserved, which independently
confirms the constant `arch::mmu::PCI_ECAM_PHYS` currently hardcodes. That constant should
eventually come from ACPI's MCFG table; until then the memory map is a second witness for it, which
is better than the constant standing alone.

**`rsdp 0x0`: QEMU's PVH loader does not fill the ACPI root pointer in.** The field exists and is
zero, which means the RSDP has to be found the older way, by scanning for the `"RSD PTR "`
signature. That is what `arch::x86_64::machine::find_rsdp` does, and it finds one at `0xf52e0`, in
the third reserved region above.

## ACPI, which is where the rest of the machine is described

`machine_discovery::acpi` decodes the RSDP, the root table, the SDT header, the MADT and the MCFG,
host-tested with thirteen tests. It sits beside the arch records rather than inside the `x86_64`
module for the reason `cpu_list` does: **ACPI is not an x86 standard**, and milestone 20's own text
expects the machine after the VisionFive 2 to be a UEFI/ACPI one.

**The checksum is the whole defence and it is worth being explicit about why.** The RSDP is found by
scanning memory for an eight-byte string, so without the checksum any sixteen bytes that happen to
spell `RSD PTR ` would be believed and the kernel would follow a pointer into somebody's data. Every
table is checksummed on the way in, and one that fails is reported as absent rather than used: a
corrupt MADT would hand out an APIC address and there is nothing downstream that could notice it was
wrong.

What q35 actually has, read on 2026-08-23:

```
  acpi        : rsdp at 0xf52e0 (revision 0), root table 0xffe2344 (rsdt)
                0x000ffe213c  FACP (244 bytes)
                0x000ffe2230  APIC (120 bytes)
                0x000ffe22a8  HPET (56 bytes)
                0x000ffe22e0  MCFG (60 bytes)
                0x000ffe231c  WAET (40 bytes)
                local apic 0xfee00000, 1 cpu(s) enabled, 0 disabled, 8259s present (must be masked)
                io apic 0 at 0xfec00000, gsi base 0
                pcie ecam 0xb0000000, buses 0..=255 (mmu::PCI_ECAM_PHYS says 0xb0000000)
```

Three things to take from that.

**The ECAM window the MCFG describes is exactly the constant `arch::mmu::PCI_ECAM_PHYS` hardcodes**,
which is now confirmed twice over (the PVH memory map reports the same range as reserved). **Milestone
165 wired the consumer**: `memory::record_pci_regions` fills `memory::pci_regions()` from this table's
answer, the same static `kernel/src/pci.rs`'s probes already read on the other two architectures, so
`PCI_ECAM_PHYS` is now only the print-time reference value above rather than what gets mapped. Reading
the table alone was not enough: QEMU's ECAM decode is off until the host bridge's `PCIEXBAR` register
is programmed, which real firmware does and PVH does not, so `arch::x86_64::machine::enable_pcie_ecam`
does it through the legacy `0xcf8`/`0xcfc` ports before the window above is trusted. See milestone
165's own text for the measurement that found this (a monitor `xp` on the address faults before the
register is written, and reads the host bridge's real id after).

**`8259s present (must be masked)`** is the MADT's `PCAT_COMPAT` flag, and it is a real obligation
rather than trivia: the legacy PICs are still wired and still raise interrupts, so whoever brings the
APIC up has to mask both of them first or a spurious interrupt arrives through a controller nothing
is driving.

**What is deliberately not decoded is AML**, the bytecode in the DSDT that describes everything with
no fixed table, including PCI interrupt routing (`_PRT`). AML needs an interpreter, which is a
project rather than a parser. That is the reason `arch::mmu::PCI_IRQ_BASE` is zero and honest about
it: a PCI function's legacy interrupt goes through a router the `_PRT` describes, and MSI bypasses
the routing entirely by writing a vector straight to the local APIC.

**The MSI path is the one that got built** (milestone 215), and the sentence above predicted it
before there was a device on the bus to need it. See "How a PCI function's interrupt reaches a
driver" below.

## The discovery seam milestone 20 promised and did not build

Milestone 20's deliverable had two abstraction shapes in it. The first, "a generic level-walk plus a
per-arch entry codec", **was built and holds**: `crates/paging` needed no change for a third format.
The second, "put device discovery behind a 'here is the hardware' interface (device tree today,
ACPI/PCI later)", **was not built**, and this port is where that shows.

`kernel/src/memory.rs`'s `init` took a device-tree pointer and read `Dtb` directly for the RAM
regions, the reservations, the interrupt controller, the RTC, the UART's interrupt and the PCIe
window. Nothing about that is wrong on two architectures that both have a device tree. On x86 there
is no tree, so **the frame allocator could not come up at all**.

**The narrow half is now split out** (`memory::bring_up_page_frames`), because without it nothing below
the allocator can exist on this architecture and the port would have stopped there. `init` is now
explicitly a device-tree *front end*: it reads the tree, assembles a RAM slice and a forbidden slice,
and hands them to a function that does not care where they came from.
`arch::x86_64::machine::bring_up_memory` is the x86 front end doing the same job from the PVH map.

**The wide half is still partly owed.** What crosses the seam today is RAM, reservations, and (as of
milestone 165) the PCIe ECAM window. The interrupt controller and the console UART's interrupt still
bypass `memory.rs`'s statics or stay unwired. Three facts still have two sources that do not know
about each other, which is the shape this should not be left in:

| Fact | Device-tree machines | x86 |
|---|---|---|
| RAM, reservations | `memory::init` | `machine::bring_up_memory` (**shared consumer**) |
| Interrupt controller | `memory::init` -> `GIC_REGIONS`/`PLIC_REGION` | ACPI MADT -> `machine::Acpi` -> `irq::init_{local,io}_apic` **directly**, bypassing the statics |
| PCIe ECAM window | `memory::init` -> `PCI_REGIONS` | ACPI MCFG -> `machine::Acpi` -> `memory::record_pci_regions` -> `PCI_REGIONS` (**shared consumer**, milestone 165). The BAR/mem32 half of the same static has no ACPI or AML source and stays a hardcoded constant (`arch::mmu::PCI_BAR_PHYS`); see that milestone for why. |
| Console UART interrupt | `memory::init` -> `UART_IRQ` | discoverable now (`Acpi::isa_irqs[4]` is COM1's), unwired |

The type at the seam is another loose end worth naming: `Region` is `dtb::Region`, which is a plain
`{ start, size }` pair and means nothing device-tree-specific, but a machine with no device tree
naming a device-tree type is a smell rather than a design.

## The clock, and why it needed a calibration loop

The other two architectures read their timer's rate out of a register or a device-tree property.
x86 has at least four clocks and no architected way to ask any of them, on the parts this has to run
on: `CPUID` leaf 0x15 gives the TSC's ratio to a crystal whose frequency leaf 0x16 may not report,
and the local APIC timer counts a bus clock nothing reports at all.

So the rate is **measured**, against the one device on a PC whose frequency is a fixed number: the
8254 PIT at 1193182 Hz, a number that has not changed since 1981 because it came from dividing the
NTSC colour-burst crystal and every clone copied it. One ten-millisecond window, polled on channel 2
(the only channel whose gate is under software control and whose output can be read), with both the
TSC and the APIC timer sampled across it, so one wait produces both numbers and they cannot disagree
with each other.

Measured on QEMU TCG, 2026-08-23:

```
  apic        : local apic 0xfee00000 up, id 0, version 0x14, 8259s masked
  clocks      : tsc 1001 MHz, apic timer 62 MHz (both measured against the PIT)
  timer       : 20 ticks in ~0.2s at 100 Hz (20 routed, 0 spurious)
```

**Twenty ticks in a fifth of a second at 100 Hz is the number that proves the whole interrupt path**,
not just the clock: an interrupt the CPU did not ask for arrived, the IDT dispatched it by vector,
and the handler acknowledged it so a second could follow. A missed EOI is a hang rather than an
error on this architecture, so exactly one tick would have been the failure to expect. The 62 MHz
APIC timer is 1 GHz divided by 16, which is the divider `irq.rs` programs, so the two measurements
agree with each other as well as with the emulator's nominal rate.

Two obligations the ACPI tables state and this code honours, both of which would otherwise show up
much later as a hang:

- **The 8259 PICs are masked before the local APIC is enabled.** Their power-on vector base overlaps
  the CPU's own exception vectors, so an interrupt from one arrives as, for instance, the double
  fault. The order matters: the reverse leaves a window in which the IDT is live and they are not
  masked.
- **The Task Priority Register is zeroed.** Anything else silently drops interrupts below that
  priority class, which looks exactly like a controller that was never wired up.

## The IO APIC, and the number that is not the number

Milestone 161's roadmap item 2, built 2026-08-24. The local APIC's timer proves the CPU accepts an
interrupt it did not raise. A **device** line is a different claim, and the IO APIC is what makes
it: it takes physical interrupt inputs and turns each into a vector delivered to some local APIC.

### The register interface is two words, and that is the first surprise

The whole device is a 4 KiB page with two 32-bit registers in it. `IOREGSEL` at offset 0 takes the
*number* of the register you want; `IOWIN` at offset 0x10 is a window onto whatever `IOREGSEL` last
named. So every access is a pair, the device is stateful, and two CPUs doing this concurrently would
interleave and each read the other's register. Nothing here is concurrent yet (one CPU,
single-threaded boot), and `irq.rs` says out loud that a lock belongs there the day SMP lands.

What is behind the window:

| Register | What it is |
|---|---|
| `0x00` | this IO APIC's id, bits 27:24 |
| `0x01` | version in bits 7:0, and **the entry count minus one** in bits 23:16 |
| `0x10 + 2n` | redirection entry `n`, low word: vector, delivery mode, polarity (bit 13), trigger (bit 15), mask (bit 16) |
| `0x11 + 2n` | redirection entry `n`, high word: the destination local APIC id in bits 63:56 |

The "minus one" in the version register is the field's definition rather than an off-by-one to
correct for: a 24-entry part reports 23. And the two words are written **high first**, so the
destination is in place before the low word's mask bit clears and the line goes live.

### A legacy IRQ number is not a pin number, and this is the whole trap

The PIT's IRQ 0 does not arrive on IO APIC input 0. On q35, and on essentially every PC:

```
                io apic 0 at 0xfec00000, gsi base 0
                isa irq 0 -> gsi 2 (active high, edge)
                isa irq 5 -> gsi 5 (active high, level)
                isa irq 9 -> gsi 9 (active high, level)
                isa irq 10 -> gsi 10 (active high, level)
                isa irq 11 -> gsi 11 (active high, level)
```

The PIT is wired to pin 2 because pin 0 carries the 8259 cascade. A kernel that armed redirection
entry 0 for "the timer" would have armed a line nothing drives, and the failure is the quiet kind:
no interrupts, no error, nothing anywhere to say why.

Resolving that is pure logic over table bytes, so it is in the crate rather than in the kernel:
`machine_discovery::acpi::isa_irq_table` walks the MADT's interrupt source overrides once and
returns all sixteen legacy IRQs resolved, with six host tests holding it. `irq::record_isa_routing`
copies the answer into a static, and `irq::enable(intid)` takes a *legacy IRQ number* the way the
arch contract's other two implementations take an INTID or a PLIC source.

**The flags word is two two-bit fields, not two bits**, and reading it as two bits gets both wrong.
Bits 1:0 are polarity (`00` conforms to the bus, `01` active high, `11` active low) and bits 3:2 are
trigger mode (`00` conforms, `01` edge, `11` level). QEMU emits `0x000d` for the PCI-link IRQs,
which is `0b1101`: active **high**, level. A decoder that tested bit 1 for polarity would answer
"active low" and arm a line that never asserts. There is a test named for exactly that misreading.

`00` meaning "conforms to the bus" is also not a synonym for "active high, edge" even though the two
coincide on the ISA bus; the default lives in one place (`IsaIrqRouting::isa_default`) so a future
reader cannot change it in only one of the two.

### What it was proved with, and why the PIT

The PIT twice over: `timer.rs` already drives it for calibration, and IRQ 0 is the one line every PC
rewires, so routing it is simultaneously the easiest device to reach and the strongest test of the
override table. Channel **0** is the one whose output goes to the interrupt controller, which is
exactly why calibration cannot use it and why this can use nothing else. Mode 2 (rate generator)
pulses the line once per reload and reloads itself.

Measured on QEMU TCG, 2026-08-24, with the local APIC timer masked for the window so the count is
the PIT's alone:

```
  io apic     : id 0 at 0xfec00000 up, version 0x20, 24 redirection entries, gsi base 0
  device irq  : pit irq 0 -> gsi 2 on vector 0x32: 20 interrupts in ~0.2s at 100 Hz
```

Twenty is the same number the local APIC timer produces in the same window against the same TSC,
which is what makes it a measurement rather than a nonzero.

### The choices, and what each one costs

**The vector map is flat**: `GSI_VECTOR_BASE + gsi`, with the base at 0x30 so that 0x20..0x2f stays
free for the local APIC's own LVT sources (the timer, and later thermal, performance, error and
inter-processor vectors). Flat means a stray vector in a fault report names its line by subtraction.
It costs the ability to express a priority policy, since x86 priority is the vector's top four bits
and a flat map fixes which line lands in which class. There is no policy to express yet.

**Physical destination mode, fixed delivery, at the boot CPU.** Not lowest-priority delivery and not
a logical group: the simplest thing that is correct on one CPU and stays correct on several.
Distributing interrupts is a policy too.

**Every entry is masked during bring-up.** They power on masked, so this changes nothing on a cold
boot. It matters on a warm one, where firmware may have armed a line for its own use and left it
armed, and an inherited interrupt arriving on a vector this kernel never assigned is a puzzle with
no clue in it.

**The 8259s stay masked rather than being remapped.** The same device line reaches both controllers,
so an unmasked 8259 would deliver a second copy of every interrupt the redirection table routes, on
a vector that is an exception number.

### What this did not touch

**PCI interrupt routing**, which is blocked on AML rather than on tables: a PCI function's legacy
interrupt goes through a router the DSDT's `_PRT` describes, and there is no interpreter here.
**MSI**, which bypasses the redirection table entirely by writing a vector straight to the local
APIC, is the path worth building for that reason and is its own piece of work.

## The fine map, and the hazard that was designed out instead of sequenced around

Milestone 161's roadmap item 1, built 2026-08-24. What `boot.s` leaves behind is enough to run and
not enough to be a kernel: 2 MiB pages, everything present, writable *and* executable, in both
halves, plus an identity map of the low 4 GiB sitting in the half ring 3 will get. `mmu::init`
replaces it with a four-level map built through the shared `paging::Mapper`, exactly as the other
two architectures do.

What the new map says, in the order `map_everything` builds it:

| What | Where | Flags |
|---|---|---|
| All of RAM, minus the kernel image's own frames | `DIRECT_MAP_BASE + pa` | `kernel_data` (RW, never X) |
| The first megabyte, and reserved entries below the top of RAM | `DIRECT_MAP_BASE + pa` | `kernel_data` |
| `.text` | `KERNEL_VA_BASE + pa` | `kernel_code` (X, never W) |
| `.rodata` | `KERNEL_VA_BASE + pa` | `kernel_rodata` (neither) |
| `.data` + `.bss`, the boot stack, the per-CPU secondary and interrupt stacks | `KERNEL_VA_BASE + pa` | `kernel_data` |
| Every guard page | -- | **not mapped**, and `verify` asserts it |
| The local APIC (from the MADT), the IO APIC, bus 0 of the PCIe ECAM window | `DIRECT_MAP_BASE + pa` | `device` (uncacheable) |
| Physical page 0, and the identity map | -- | **not mapped** |

**The image's own frames are skipped in the direct map**, which is a difference from the other two
ports rather than a copy of them. There, the image and the direct map share a base, so a direct-map
entry for those frames would collide with the section mappings and the mapper's overwrite refusal
catches the ordering mistake. Here the bases differ, so it would not collide: it would quietly be a
second, **writable** alias of `.text`. W^X that a second mapping undoes is not W^X.

### The hazard the roadmap flagged, and why there is no sequencing step

`phys_to_virt` changes meaning the instant a new `CR3` is installed, if the old and new tables put
the direct map in different places. That is not hypothetical: `memory::bring_up_page_frames` turns the
frame bitmap's physical address into a `&'static mut [u8]` and **stores it**, and the PVH structure
and the ACPI tables are read the same way, all before any fine map exists.

The roadmap's prescribed answer was to carry both aliases across the switch and drop the old one
afterwards. The answer taken instead was to make the arithmetic never change: **`boot.s` writes
PML4[273] itself**, pointing at the same low PDPT the identity map already uses, so the direct map
exists from before the first Rust instruction and `mmu::init` widens it rather than introducing it.
Eight bytes, no extra memory, no second step, and nothing to remember. The two constants are kept in
agreement by a `const` assertion in `mmu.rs` that recomputes PML4[273] from `DIRECT_MAP_BASE`, so
changing one without the other fails the build rather than the boot.

The one thing that *does* change across the switch is the local APIC page's **memory type**, from
the boot map's cacheable to device. `irq.rs` keeps its base as a direct-map address and needs no
re-derivation.

### What made it debuggable

`verify` walks the finished tables in software and asserts the five things that would kill the
machine, before `mov cr3` bets on them: this function's own code is mapped, executable and not
writable; the current stack is mapped; the frame bitmap is reachable through the direct map; PML4[0]
is zero (the identity map is gone); and every guard page is a hole. **The identity-map check has to
read the PML4 entry rather than call `translate`**, because the kernel mapper serves the high half
and would answer `None` for a low address whatever the tables said. That is the shape of assertion
this file exists to warn about: one that passes for a reason unrelated to the thing it names.

The console is the other reason this step was less frightening here than on the other two
architectures: COM1 is an I/O **port**, so nothing the page tables do can make the machine go silent.

### What it costs, measured

`556 KiB of page tables for 254 MiB of RAM` on q35 with `-m 256M`, printed on every boot. That 0.2%
is the price of 4 KiB leaves, which is all `crates/paging` maps; Linux uses 2 MiB and 1 GiB leaves
for this map and would pay a few kilobytes. Extrapolated, a 32 GiB machine would spend ~64 MiB of
RAM describing its own RAM. Adding larger leaves is a change to the shared format trait that all
three architectures would want, so it is recorded in `arch/x86_64/mmu.rs`'s `BUGS` rather than
patched here.

Two other honest limits live in that same `BUGS` section. `mmu::init` must draw its table frames
from **below 4 GiB**, because that is all the boot map's direct map reaches and the mapper writes
every table through it (the frame allocator hands out the lowest free frame first, so this holds
today). And **`CR4.PGE` is off**, so the `G` bit the kernel's `Flags` set is ignored and every
`mov cr3` flushes the whole TLB: correct and slow, and worth revisiting with ring 3, when a context
switch starts happening often enough to measure.

## Ring 3, and the four pieces that have to work in order

Milestone 161's roadmap item 3, built 2026-08-24. Everything above this heading is the kernel
talking to the machine. This is the kernel refusing a program, which is the thing the rest of it
exists for.

### The address spaces are RISC-V's model, not aarch64's

`arch/x86_64/mmu.rs`'s header used to say the x86 shape was "aarch64's rather than RISC-V's". That
is the wrong way round and it was corrected while implementing it. What matters is how many roots
the hardware has: aarch64 has two (`TTBR0` for the user, `TTBR1` for the kernel), so switching a
process leaves the kernel mapped for free. RISC-V has one `satp` and x86 has one `CR3`, so **a
process's own root must carry the kernel's entries** or the `mov cr3` unmaps the instruction after
it.

`share_kernel_half` is therefore one `copy_from_slice` of PML4 entries 256..512, exactly the RISC-V
twin, and it is worth noticing how much it buys on this architecture specifically: both kernel bases
are up there (the image at PML4[511], the direct map at PML4[273]) along with every device window,
so one copy shares all of it. The entries point at the kernel's own intermediate tables, so this
shares the map rather than a snapshot: a page the kernel maps afterwards appears in every process,
which is what a shared half has to mean.

**Where x86 is neither**: the ASID. PCID lives in `CR3[11:0]` and is honoured only with `CR4.PCIDE`
set, and it is not set here. So `ttbr0_value` drops the tag `crates/asid` hands it, because with
PCIDE clear those bits are reserved-zero and `root | asid` would `#GP` rather than tag anything; and
`flush_asid` flushes the **whole** TLB, because there is no tag for it to select on. Both say so
where a reader meets them. Over-flushing is correct and slow; under-flushing would be one process
reading another's memory with nothing to announce it, which is the failure that function exists to
prevent.

### `swapgs` is an exchange, which is the whole reason it is guarded

A trap from ring 3 arrives with the *user's* `GS` base, and this kernel keeps its per-CPU pointer in
`IA32_GS_BASE`, so the first thing that takes a lock would read whatever the program left there.
`swapgs` exchanges `IA32_GS_BASE` with `IA32_KERNEL_GS_BASE`, and that is what makes the pointer
unforgeable: the value the kernel needs sits in a register ring 3 cannot write, which is the problem
RISC-V solves with `sscratch`.

Because it is an exchange rather than a load it must run **exactly once per privilege change in each
direction**. A trap from ring 0 that swapped would install the user's base while running kernel
code; a nested trap that swapped again would put the kernel's back and then hand it to the user on
the way out. So both sites test the saved `CS`'s low two bits, which *are* the interrupted CPL.

**It does not trip the bug this port already has a section about.** "Loading a segment register in
long mode destroys that segment's base MSR" applies to loading a *selector*; `swapgs` writes the MSR
pair and loads nothing. That was checked against the instruction's definition rather than assumed,
precisely because the earlier bug cost an afternoon and presented as something else entirely.

**The one delicate window is on the way out.** Between the exit `swapgs` and the `iretq` the CPU is
in ring 0 holding the user's GS base, so an interrupt taken there would see CPL 0, decline to swap,
and dereference the user's value. `RFLAGS.IF` is clear at every such site (an interrupt gate cleared
it; `IA32_FMASK` clears it for `syscall`; the two ring-3 entry points `cli` first), which closes it
for everything except an NMI or a machine check. Closing it for those needs a paranoid entry path
that reads `IA32_GS_BASE` and decides, which this port does not have. Recorded rather than pretended
away.

### `syscall` shares almost nothing with the IDT

Four MSRs are its entire configuration and none has a useful default:

| MSR | What it carries |
|---|---|
| `IA32_STAR[47:32]` | the kernel CS; the kernel SS is that **plus 8**, which is why the GDT's order is arithmetic |
| `IA32_STAR[63:48]` | the base `sysret` derives the user pair from: SS = base + 8, CS = base + 16 |
| `IA32_LSTAR` | where a 64-bit `syscall` jumps. A raw address, so a zero here is a jump to zero |
| `IA32_FMASK` | the `RFLAGS` bits cleared on entry, `IF` among them |
| `IA32_EFER.SCE` | whether `syscall` is a legal instruction at all, rather than `#UD` |

`SCE` is enabled **last**, so the instruction becomes legal only after the address it jumps to and
the flags it clears are already in place. Three `const` assertions in `exceptions.rs` tie
`SYSRET_SELECTOR_BASE` to the selectors in `segments.rs`, so changing one file without the other
stops the build rather than landing a program on the kernel's data segment.

The entry path is where the difference from a trap gate is felt: **`syscall` does not switch stacks
and pushes nothing**. `rsp` still names the user's stack and every register still holds a user value,
so the first three instructions are `swapgs`, park the user's `rsp` in a static, and load the
kernel's. `segments::set_kernel_stack` writes `TSS.RSP0` and that static together, so the two doors
into the kernel cannot come to name different stacks; both are one CPU's, which is the same
single-TSS limitation SMP bring-up already has to fix.

**The return is an `iretq`, not a `sysretq`, and that is a decision.** `sysret` returns to whatever
`rcx` holds without checking it is canonical, and a non-canonical `rcx` faults *in ring 0 on the
user's stack*: the shape of CVE-2012-0217. Using it safely needs an explicit canonicality check plus
an `iretq` fallback, which is Linux's "opportunistic sysret" dance. Sharing one restore path costs
some tens of cycles per syscall and buys one `swapgs` rule with one place to get it wrong. It is in
the handler's `BUGS` as worth revisiting **with a benchmark**, once there is a syscall-heavy
workload here to measure.

### What it was proved with, and the honest size of the claim

There was no ELF built for `x86_64-unknown-none` and no scheduler on this architecture, so the proof
was a hand-assembled probe (`arch/x86_64/ring3_probe.s`) entered straight from the boot thread. That
is the shape both other ports shipped on their first day and then deleted, and it was deleted here
for the same reason by roadmap item 4; the transcript below is what it printed while it existed.

```
  ring 3      : a program ran at cpl 3 (cs 0x0023, ss 0x001b) and made 2 syscalls
                the portable dispatcher answered BadSyscall (-6) to an unknown number
                reading the kernel's .text at 0xffffffff80109000 from ring 3: Permission(Read)
```

Three facts, and they get stronger down the list.

**`cs 0x0023`** is the hardware's own answer to what ring this is, because CPL is literally
`CS[1:0]`. A program that had somehow stayed in ring 0 would have reported `0x08`.

**`BadSyscall (-6)`** came out of the *portable* `crate::syscall::dispatch`, through
`TrapFrame::{syscall_nr, arg, set_arg}`, and reached the program in `rdi`. Asking for a refusal is
deliberate rather than lazy: an unimplemented number is the one thing that dispatcher can answer on a
kernel with no scheduler, so it is the only round trip available through the real thing rather than a
stand-in. Getting the answer back proves the entry, the ABI accessors and the return all work, and
it is checked in code rather than printed and left to a reader, because the interesting failure is
not "no answer" but "an answer from somewhere else": a return register the restore path never wrote
would most likely still hold the zero the entry frame put there.

**`Permission(Read)` rather than `Translation(Read)`** is the strongest of the three. The page *was*
found, because a process root carries the kernel's high half, and the walk refused it on the `U/S`
bit. A test that only asserted "something faulted" could not tell those apart, and the sloppier
of the two would have passed it. x86 is the most forthcoming of the three architectures here: it
pushes an error code saying so outright, where aarch64 reads it out of `ESR_EL1` and RISC-V has to
re-walk the tables to find out.

### The way back, which was scaffolding and is gone

`enter_user` never returns, and the boot tour had to print what happened, so `trap.s` grew
`x86_enter_user_and_wait` and `x86_leave_user`: park the six callee-saved registers and the call's
own return address on the current stack, record that block's address, enter ring 3, and resume it
from the trap handler when the probe was done. It was `switch_to`'s two halves with a ring change in
the middle, which was not a coincidence.

**Both, and `ring3_probe.s` with them, were deleted by roadmap item 4**, which is what the paragraph
above predicted: a scheduler does with two threads what that pair did with one. What replaced them
as the tour's proof is a pair of real processes; see "The scheduler" below.

### Two latent bugs on a path nothing had ever executed

Found by reading `context.s` against `context.rs` while wiring this up, and worth recording because
neither could have been caught earlier and both were in code that looked finished:

- **The child's arguments were in the wrong registers.** `Context::for_user_thread` wrote them to
  `r13`/`r14`/`r15`; `user_entry_trampoline` read them from `r12`/`r13`/`r14`. A child would have
  received `(0, arg0, arg1)` and `arg2` would have vanished. Both files were internally consistent
  and neither was executed.
- **The trampoline did not reserve a trap frame's worth of stack**, which milestone 71 established
  both other architectures need: the thread's `TrapFrame` lives at `top - 176` for the life of the
  thread, and the entry path's own frames start at the same top.

Both are fixed and both are **still** unexecuted: that trampoline is the *scheduler's* entry path,
and the self test enters through `enter_user` directly. They are the argument for reading the two
halves of a context switch side by side rather than one at a time.

## The scheduler, and what "a process" cost that "a program at CPL 3" did not

Milestone 161's roadmap item 4, built 2026-08-24. Item 3 ended with a hand-assembled probe entered
from the boot thread; this is the distance between that and a process, and the distance was real.

Almost none of it was new x86 code. `kmem`, `untyped`, `sched` and `user::AddressSpace` are portable
and came up on this architecture by being compiled for it. What had to be written was the arch layer
underneath them, and what had to be *found* was four places where portable code encoded an
assumption that held on two architectures and not on three.

### The trap path grew the split both other ports already had

`x86_trap_handler` became `x86_trap_dispatch` (outer) plus `x86_trap_body` (inner), with
`dispatch_on_interrupt_stack` in trap.s between them. The outer half stays on the interrupted
thread's stack and is where the deferred `schedule()` runs; the inner half may run on this CPU's
interrupt stack. That is not an optimisation: `schedule()` parks the running `rsp` in the outgoing
thread's `Context`, so calling it from a per-CPU stack would park a per-CPU address in a thread and
the thread would later resume on bytes the next interrupt had spent. See kernel/src/interrupt_stack.rs.

The timer arm now calls `sched::on_tick` and returns `true`, which is DECISIONS §9's record-and-defer
with the deferral one frame out.

### `TSS.RSP0` is recomputed from the frame on every return to ring 3

x86 has two doors into the kernel from ring 3 and they find their stack differently: a trap reads
`TSS.RSP0`, and `syscall` reads nothing at all and has to be told separately. With one user program
there was one kernel stack and `ring3_self_test` set both by hand. With a scheduler there is one per
thread, and the pair has to be re-pointed every time the thread that would come through them changes.

**The frame's own address is the answer.** Every thread's `TrapFrame` lives at `stack_top - 176` for
the life of the thread (milestone 71, `user::enter_frame`), so at the top of `isr_restore` the top is
`rsp + 176`, computed from the frame about to be loaded rather than from any record of who is
running. RISC-V does exactly this, at exactly this point, in `trap_return`. It costs four
instructions on a path that already tests the same `cs` for `swapgs`, and it makes the wrong state
unrepresentable rather than a duty somebody has to remember at each context switch.

### A self-IPI is this architecture's software-generated interrupt

`sched`'s two interrupt-delivery tests need a way to raise an interrupt by hand. aarch64 has an SGI
and needs no device; RISC-V can raise nothing at all and has to assert its console UART's transmit
line. **x86 is aarch64's case**: the local APIC's Interrupt Command Register has a "self" destination
shorthand, so any vector can be delivered to the CPU executing the write, through the real path (the
ICR, the IRR, the ISR, an EOI). `irq::raise_self_interrupt` is the whole of it, and the same ICR code
is `irq::send_reschedule`, which stopped being `unimplemented!()`.

**The intid for such a source is its vector**, and that is the naming rule this architecture needs
because `irq::enable` takes a *legacy IRQ number* instead. The two domains cannot collide: legacy
IRQs are 0..15 and the local APIC's own vectors are 0x20..0x2f.

A **device** line still cannot become a message here, and the missing piece is an inversion rather
than a mechanism: the flat vector map makes the GSI recoverable by subtraction, but a legacy IRQ is
not, because GSI 0 is the 8259 cascade and has no legacy owner, so an inversion falling back to the
GSI would answer 0 for both it and the PIT's IRQ 0. Nothing needs it until there is a userspace
driver here.

### Four things portable code got right for two architectures and wrong for three

Worth listing together, because they are one shape: a default arm, or an expression, that names one
of two answers and is silently wrong when a third exists.

- **`thread.rs`'s stack area.** `KERNEL_VA_BASE | 0x10_0000_0000`, "64 GiB above the direct map",
  which is right where the kernel base is a *half* base with room above it. `KERNEL_VA_BASE` here is
  `0xffffffff80000000` and already carries that bit, so the OR was the **identity**: every kernel
  thread stack would have been mapped at the kernel image's own base, over `.text`. It is now each
  `arch::mmu::THREAD_STACK_AREA`, and x86's is Linux's `VMALLOC_START`. Found by reading; it would
  have surfaced as `kmem: no memory to wire one`, arbitrarily far from the cause.
- **`crates/elf`'s `EXPECTED_MACHINE`.** `#[cfg(not(target_arch = "riscv64"))] EM_AARCH64`, and
  `not(riscv64)` catches x86_64: the x86 kernel was compiled to **accept aarch64 binaries and refuse
  its own**. Now a three-arm `cfg`.
- **`xtask`'s `ArchLegs`.** `fn aarch64(self) { self != Riscv64 }`, which answers `true` for every
  leg the moment there is a third variant. Now explicit `matches!`.
- **`smp::bring_up_secondaries`.** It computed the secondary entry with `virt_to_phys` *before*
  checking whether any core can be started, and on x86 that panics: `secondary_boot` is in `.boot`,
  which the linker places at its physical address because the trampoline starts in 32-bit protected
  mode and cannot name a 64-bit one. The conversion moved below the refusal. (It is not the right
  conversion for x86 SMP either, when that lands: that entry is already physical.)

The x86 tour also has to *call* `bring_up_secondaries`, even though it can start nothing. That
function marks the boot core in `ONLINE_MASK` before it refuses, and everything that broadcasts
(`online_cpus`, `nth_online`, the shootdown loops) reads that mask. Not calling it left the online
set empty while `online_count` said one, which the suite caught on its first run.

### What the boot tour shows now

```
  scheduler   : up on 1 cpu, preempting at 100 Hz (idle thread registered)
  smp: none yet (x86 uses INIT-SIPI-SIPI via the local APIC; milestone 161)
  smp: 1 core(s) online
  kernel task : a spawned thread ran and carried its captured state (0x16100004)
  userspace   : a process built from untyped ran at cpl 3 and sent 0x1610004 on a granted cap
                thread 8589934594 died at pc 0x400005 on addr 0xa50000, delivered to its supervisor
                two children cost 0 frames the first round and 0 the second (steady state)
```

Two children, because the two halves of "a process" fail differently. One is built out of a single
untyped region (address space, code page, stack page, TCB, two endpoints), granted a capability,
dispatched to ring 3 **by the scheduler**, invokes that capability, and exits; one loads from an
unmapped address and its death arrives on its supervision endpoint naming the thread, the pc and the
address. Then both regions are destroyed and the frame count is compared.

**The round runs twice, and that is what makes the frame number evidence.** A first round pays
first-use carves that are not leaks; a system in steady state charges the second round zero. The
first version of this reported sixteen frames a round going missing, which was true and was the
demo's own fault twice over: the reporting child's corpse was never collected, so its region was
still holding a live TCB when `destroy` refused it (silently, because `destroy` has nowhere to
report), and both endpoints were drawn from the kernel's shared pool rather than from the region
being reclaimed.

## The userspace, and how an archive reaches a machine with no device tree

Item 4's hand-off, 2026-08-24. Five pieces, and only one of them was interesting.

### `user_rt` needed five transliterations and two decisions

The five are mechanical once DECISIONS §124 is read: `syscall` where aarch64 writes `svc` and
RISC-V writes `ecall`, the number in `rax`, the arguments in `rdi`/`rsi`/`rdx`/`r10`/`r8`/`r9`.
Three facts at those sites have no counterpart on either other architecture and all three are the
instruction rather than a choice: `syscall` clobbers `rcx` (the return address) and `r11` (the
caller's `RFLAGS`) unconditionally, so every site declares them; `r10` carries the fourth argument
because `syscall` has already taken `rcx`; and `syscall` pushes nothing, which is why the kernel's
entry path parks `rsp` by hand and why `options(nostack)` is honest here.

The two that are not transliterations are **`now()` and `cntfrq()`**. aarch64 reads `CNTVCT_EL0`
and `CNTFRQ_EL0`; RISC-V reads the `time` CSR and hardcodes the rate because nothing tells
userspace. x86 has neither register.

`now()` is `rdtsc`, and ring 3 may read it because `CR4.TSD` is clear at reset and this kernel does
not change it. That is the same shape as aarch64 needing `CNTKCTL_EL1.EL0VCTEN` and RISC-V needing
`scounteren.TM`, with the difference that here the permissive state is the default and the kernel
would have to act to *close* it. One trap: `rdtsc` answers in `edx:eax`, so reading it into a single
`out(reg)` compiles and silently returns a counter that wraps every four seconds.

#### The cycle counter is ambient here, and that was inherited rather than chosen

Recorded 2026-09-02 by milestone 228 (the cycle counters are closed by assumption, and on two
architectures the assumption is a comment), which closed the equivalent door on the other two
architectures and deliberately left this one open.

The paragraph above says the permissive state is x86's default. What it does not say is that on the
other two architectures the register userspace gets and the register a profiler would want are
**different registers**, and here they are the same one:

| | what userspace gets | what stays shut | how |
|---|---|---|---|
| aarch64 | `CNTVCT_EL0`, ~62.5 MHz under QEMU | `PMCCNTR_EL0`, the cycle counter | `CNTKCTL_EL1.EL0VCTEN` set, `PMUSERENR_EL0` written to zero |
| riscv64 | the `time` CSR, 10 MHz under QEMU | the `cycle` and `instret` CSRs | `scounteren` written to exactly `TM` |
| `x86_64` | the TSC | **nothing** | `CR4.TSD` clear at reset, never written |

So every ring-3 program on this architecture holds a sub-nanosecond instrument, roughly two orders of
magnitude finer than what the other two hand out, and no line of this kernel decided that. It is the
reset value. Milestone 228 says so out loud rather than implying that writing the other two registers
made three architectures agree.

**Why it was not closed with them.** `CR4.TSD` is one instruction away, and setting it today would
break `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps in `std_net` and the
benchmark harness simultaneously, because `user_rt`'s `now()` on this architecture **is** `rdtsc` and
there is no coarse alternative to fall back to. Closing it needs a second time source first: a coarse
monotonic value published in a page, the same move DECISIONS §43 (reading the clock is a page) already
made for the wall clock, one axis over. Nothing proposes building that here; it is named so this row
is a limitation with a price rather than an exception with no plan.

**What it means for the open decision.** `x86_64` has already answered milestone 75 (who may read the
cycle counter, and by what authority) with "everyone, always", by inheritance. Whichever way that
decision goes for aarch64 and riscv64, this architecture will not match it until the page above
exists, and §19 (architectural parity is a tenet) should read that as a scope note rather than as a
gap somebody forgot. Linux names the same asymmetry from the other side: its arm64 per-task
`PMUSERENR_EL0` work opens the counter only on request, explicitly to avoid "the information leaks
x86 has".

`cntfrq()` is **RISC-V's gap, one architecture worse**, and it is a constant with a `BUGS` section
rather than a number with a comment. There is no architected TSC rate at all: `CPUID` leaf 0x15
gives a ratio to a crystal that leaf 0x16 may not report, and neither leaf exists on every part. So
the kernel does not read the rate, it **measures** it against the PIT (`timer::init_frequency`) and
stores it in `TSC_HZ`. A ring-3 program cannot repeat that measurement, and should not be able to:
the PIT is at ports 0x40..0x43, `IOPL` is 0 and the TSS's permission bitmap is empty, so an `in`
from a process is a #GP. That is §121 not being decided yet rather than an oversight to route
around. The constant returned is QEMU's 1 GHz, which the kernel measures as 1001 MHz; on milestone
87's real Dell it will be the CPU's base frequency and this will be wrong with no way for a caller
to tell. **Both architectures want the same fix**: hand the frequency to a process at start, the
way Linux passes `AT_HWCAP` in the aux vector, so the one component that measured it is the one
that reports it.

### Four programs needed an arm, and three of them refuse rather than pretend

`os_primitives_benchmarker` hand-assembles a child stub, which had to become **bytes rather than
words** because x86 instructions are not a fixed width; one `copy_nonoverlapping` over
`size_of_val` now serves all three element types.

`console::uart_put`, `input`'s `uart` module and `swap_proto::probe_device` cannot reach a device
from ring 3 at all. Their x86 arms `trap()` (or return a sentinel that nothing observes) rather than
no-op, and that is the whole point: a silent no-op compiles into a console that acknowledges every
byte and prints none, which is a lie told in the one place an operator is looking. Those programs
are packed into the archive anyway, because an archive entry costs a directory slot and nothing
spawns a program by accident, and the tests that would spawn them skip with the reason.

### The archive arrives as a PVH module, which is the device tree's `/chosen` one machine over

`hvm_start_info` carries `nr_modules` and `modlist_paddr`, and QEMU's PVH loader turns `-initrd
FILE` into exactly one entry there. `machine_discovery::x86_64::module` decodes it (host-tested,
three tests, including the 32-byte stride that a one-module list cannot catch), and
`arch::x86_64::machine::initrd` reads entry zero through the direct map. The x86 front end then does
what `memory::init` does with the tree's answer: adds the region to the `forbidden` slice so the
allocator cannot hand out the archive it is about to read, and calls `memory::record_initrd`. The
evidence it works is arithmetic rather than a print: free memory drops from 254 MiB to 249 with a
4.4 MB archive attached.

**The first module is taken and any others ignored.** PVH permits several; this kernel wants one;
inventing a policy for a case the machine never produces would be code nobody could test.

### `xtask` grew a third packer, and one latent bug came out with it

`initrd_x86` packs the same table `initrd_riscv` does, which is now
`portable_archive_entries()`: one list, two callers, because duplicating it would have let the two
drift the first time somebody added a program to one of them. x86 builds the whole `user` package
rather than naming binaries, which is both shorter and self-maintaining now that everything
compiles for the target.

`read_stripped`'s cache tag is the latent bug. It namespaces stripped copies by the target directory
a binary came from, and an `x86_64-unknown-none` path fell through to `"host"`, sharing a filename
with every aarch64 build of the same program. Nothing had noticed because nothing ever asked for
both. The moment an x86 archive is packed in the same run as an aarch64 one, whichever ran second
would read the other's bytes back out of `target/stripped` and **measure them**: a real digest of a
real program, and the wrong one, in a trust root. The check is anchored on the full triple rather
than a substring, because a host path on an x86 CI runner contains `x86_64-unknown-linux-gnu`.

### The suite: 170 pass, 67 skip

It was **97 pass, 7 skip** the day the scheduler landed and nothing in `user/` compiled. Item 4's
hand-off (2026-08-24) built the userspace, packed an archive, and turned `cfg(initrd)` on, and the
whole of `kernel/src/user/` came into the binary at once.

Every skip prints why. Grouped by cause, largest first:

| Count | Reason | What would close it |
|---|---|---|
| 21 | no `fs_server` in the archive | it does not compile for this target; see below |
| 13 | no RTC binding | the discovery seam's wide half (item 0), or §121 for the CMOS ports |
| 11 | no virtio-rng on either bus | the runner attaches none, and PCI is not enumerated here |
| 5 | the console UART is in the I/O port space | DECISIONS §121 |
| 4 | one core online / no core roster | SMP, item 5 |
| 4 | no PCI bus enumerated, so no GPU, keyboard or NVMe | item 0 again, and the runner |
| 1 | no `std_exerciser` | an `x86_64-unknown-nife` target and a `std` farm (milestone 27) |
| 1 | no `mkfs` | `fs_server`'s cause, one binary over |
| 1 | address spaces are not tagged | `CR4.PCIDE`, which is calef's call (item 3) |
| 1 | `hvm_start_info` is not a device tree | nothing; it is a true statement about this machine |
| 1 | no instruction-mode entropy source on this build | milestone 162 |

**The `fs_server` group is the one that is not about this machine.** It is a toolchain failure and
it is worth stating exactly, because the next person to try will otherwise spend an afternoon on it:
`fs_server` links the vendored RedoxFS engine, which depends on the `aes` crate **unconditionally**
(its encrypted-volume support is not behind a feature), and building `aes` for
`x86_64-unknown-none` ends in

```
rustc-LLVM ERROR: Do not know how to split the result of this operator!
```

at **every** optimisation level, zero included. The cause is the target spec rather than the crate:
`x86_64-unknown-none` is `-mmx,-sse,+soft-float`, so LLVM has no 128-bit vector register to legalise
an AES block into and no scalar fallback for that operator. Nothing on this side fixes it. The two
routes out are a RedoxFS built without its crypto (a patch against a vendored crate, which
`patches/README.md` is the place for) or an x86 userspace target that keeps SSE. Both are their own
work, and until one of them happens x86 has no filesystem, which also means there is no point
attaching a disk to the runner.

### Four bugs userspace found, and every one had been latent since the day it was written

None of these could have been caught before there were user programs on this architecture, which is
the argument for doing item 4's hand-off rather than deferring it.

1. **`arch::x86_64::irq::enable` conflated two numbering schemes.** An intid on x86 is either a
   legacy IRQ (0..15, which needs the MADT's override table and a redirection entry) or a **local
   APIC vector** (0x20..0x2f, raised by writing the ICR, with no controller input to unmask).
   `enable` assumed the first, always. `spawn_init` enables `user::INIT_TEST_SGI`, which on this
   architecture *is* `SELF_TEST_VECTOR` = 0x22 = 34, and the kernel panicked with
   `gsi 34 is outside the IO APIC's range`. The fix is three lines and the ranges were already
   documented as disjoint in `GSI_VECTOR_BASE`'s own doc comment; nothing above the arch layer had
   ever called `enable` before.

2. **`user_rt::trap()` cannot use `int3` from ring 3.** A *software* interrupt is refused unless the
   IDT gate's DPL admits the caller's privilege, and every gate here is DPL 0, so `int3` from a
   process raises **#GP with error code 0x1a** (`(3 << 3) | 2`: the vector it was refused, tagged as
   an IDT selector) rather than #BP. The process died either way, so the first version looked like
   it worked; what it reported was a general protection fault at address zero, naming neither the
   instruction nor the reason. It is `ud2` now: a fault the CPU raises on a permanently invalid
   opcode, so no gate DPL is involved. Opening vector 3 to ring 3 is what Linux does, and Linux has
   ptrace to justify it; this kernel has no debugger, so that would widen what a process may do and
   buy nothing.

3. **`user/link.ld` did not name `.got`, and x86 emits one.** An unnamed orphan section is appended
   after the last section the script mentions, which was `.bss`. `.got` has file contents and `.bss`
   does not, so the `data` segment's `p_filesz` stretched past `.bss` to cover it, `p_memsz ==
   p_filesz`, and the segment had **no zero-fill tail at all**: the loader had nothing to zero and
   `.bss` was whatever the file happened to hold. Found by `the_initrd_holds_a_native_executable`,
   which asserts `memsz > filesz` precisely so the zero-fill test below it is not vacuous. It shows
   up here and not on the other two because all three targets use the static relocation model and
   none of these programs is position-independent, but LLVM's x86-64 backend still routes a few
   references through a GOT entry where the aarch64 and RISC-V backends produce none.

4. **`fs_service::blk_server_image()`'s x86 arm was a `panic!`.** It read "x86_64 builds no user
   programs at all yet", which was true when it was written and stopped being true the same day the
   archive existed. It panicked *before* any disk check, so a dozen test files that already degrade
   gracefully on a machine with no disk aborted the suite instead.

### `cfg(initrd)`, and the prediction it made

Thirty test modules under `kernel/src/user/` were gated `#[cfg(all(test, initrd))]`, a cfg
`kernel/build.rs` emits for every target that has user programs to pack. They are portable in every
respect except that their fixture is a **real ELF binary read out of the initrd archive**.

`#[cfg(not(target_arch = "x86_64"))]` would have said the same thing thirty times and been wrong the
day this port could build user programs, in thirty places nobody would think to look. `cfg(initrd)`
reads as what it means, and unblocking it was **one match arm in the build script**, with nothing
else edited: every one of those modules came back at once. That is the prediction the cfg was
written to make, and it held. It is recorded here as well as in the build script because the
alternative spelling would have needed thirty edits and would have been found by whoever hit the
thirty-first.

Six modules under `user/` ran here before that: `force_kill_tests`, `pmap_tests`, `reap_tests`,
`supervision_tests`, `survey_tests` and `thread_leak_police`. Their fixtures are hand-assembled
programs rather than ELFs, and those live in `kernel/src/user/x86_programs.rs` rather than in a
`#[cfg(test)]` module, because the boot tour needs the same four programs. They stay: a fixture that
needs no initrd is what lets the userspace demo run on a `cargo run` with no `-initrd` at all.

### What a userspace still does not have here

The bound on everything above, listed because it is the next lane's brief rather than a caveat.
Every item is a device or a toolchain, and none is `user_rt` any more.

- **No device a ring-3 process can reach.** The console UART is in the I/O port space, so
  `user::UART_PHYS` is zero and `console`, `input`, `keyboard_driver` and `swapper` are packed but cannot run;
  their arms `trap()` rather than no-op, so a boot that reached one would say so on the first byte.
  That is DECISIONS §121, still PROPOSED. **One foot gun is marked rather than removed**:
  `spawn_init` grants slot 2 a device capability over `UART_PHYS`, which on this architecture is
  *physical page zero*. The slot is positional, so declining to grant would renumber the interrupt
  capability and every role that names it, and there is nothing better to put there until §121 is
  answered. Nothing reaches it: every fixture that would map it asks
  `user::machine_has_no_device_page_for_the_console()` first.
- **The PCI bus is enumerated, and one function is driven** (milestones 165 and 215). ACPI's MCFG
  fills `memory::pci_regions()`, and a `virtio-blk-pci` disk is attached, confined behind VT-d, and
  read and written by a driver at ring 3. What is still not attached is a NIC, a GPU, a keyboard,
  an RNG, or a second disk: each is a line in `scripts/qemu-runner-x86_64.sh` and a wiring, not a
  mechanism.
- **No RedoxFS image is attached**, so the FS server (packed since milestone 164) has nothing to
  open. The nifefs disk above is the only fixture on this bus.
- **No `std`**: there is no `x86_64-unknown-nife` target spec and no farm (milestone 27).
- **No second core** (item 5), and **no ASID tags**, because `CR4.PCIDE` is off (item 3, calef's
  call, and it wants a number rather than an argument).

### How a PCI function's interrupt reaches a driver, and why it is not the legacy pin

Milestone 215, and it is the piece that turned this port's PCI bus from a thing the kernel can
enumerate into a thing a userspace driver can operate.

**The failure it fixed was a wrong answer that looked like a right one.** `pci::intx_irq(base, dev,
pin)` is `base + ((dev + pin - 1) % 4)`, and `arch::mmu::PCI_IRQ_BASE` was `0`, so the virtio-blk
function at device 4 pin 1 resolved to intid `0`, and `irq::enable(0)` put that through
`isa_routing` to the **PIT's** line. A confined block server was armed on the timer and waited
forever. Nothing anywhere said so: the wiring succeeded, the driver blocked, and the suite wedged.

**The two candidate answers, and why one lost.**

*Legacy INTx.* On `q35` a function's pin goes through the ICH9 LPC bridge's PIRQ router to an IO
APIC input, and what states the mapping is ACPI's `_PRT`, which is AML. Two versions were
available: read `_PRT` (an AML interpreter, which is a project and one this tree should not grow
for four numbers) or hardcode what QEMU does. The hardcode is the one that fails badly rather than
loudly: it would pass every gate on this machine and be wrong on the OptiPlex, and milestone 87
would discover it at a null modem, which is the most expensive place in this project to discover
anything.

*MSI-X.* The device is handed the address to write and the value to write there, so **there is no
board-specific routing table to be wrong about**. It is more code than the hardcode and much less
than the interpreter, every device this port would ever attach has it (virtio-pci, NVMe), and it is
the direction real x86 systems went twenty years ago. It won on the OptiPlex risk, not on effort;
it happens to also be less work than the honest version of the alternative.

**The design that fell out, and it is the part worth stealing.** An x86 intid was already two
things: a *vector* for a local APIC source (there is no controller input to name) and a *legacy IRQ
number* for an IO APIC line. An MSI is a local APIC source in the only sense that matters, because
the device writes the vector straight to the APIC. So **an MSI intid is its vector**, and three
things collapse:

- `irq::enable` has nothing to do for one, which is correct rather than a stub: the message is
  edge-delivered and already over. A driver's `Irq::ACK` is correspondingly a no-op.
- The trap handler can ask `sched::irq_route(vector)` directly. The vector-to-intid **inversion**
  an IO APIC line would need (and which `exceptions.rs` still records as owed for one) never
  arises, because there is no line in between to have named it.
- Nothing above `arch/` changed shape. `kernel/src/pci.rs` asks `arch::irq::alloc_msi_vector`,
  which answers `None` on both `virt` boards and leaves their INTx swizzle exactly as it was.

**A refusal, not a fallback.** If a machine answers `Some` and the function has no MSI-X, bring-up
fails and says so. Falling back to `intx_irq(0, ..)` there is the original bug, and it is the kind
that reads as a graceful degradation.

**What only real hardware can confirm.** Three things, all in `arch::x86_64::irq`'s BUGS section:
that the OptiPlex's firmware leaves VT-d interrupt remapping off (a unit with it on rejects the
compatibility-format message this builds); that a real device's MSI-X table is where its capability
says it is once *firmware* rather than this kernel has placed the BARs; and that a machine with
more than one local APIC still delivers to the boot core's id. None of the three is a QEMU
question, and all three are cheap to check: see notes/x86-uefi-boot.md's bench procedure.

## The three things that genuinely do not fit

Recorded because the failures of an abstraction are worth more than its successes.

### 1. Permissions are inherited down the page-table walk

Both existing formats treat the leaf as the single source of truth for access rights, which is why
`PageFormat::table_entry` takes no flags. x86 ANDs every level's `U/S` and `R/W` (and ORs `XD`), so a
leaf saying "user, writable" under a PML4 entry saying "supervisor, read-only" is supervisor-only and
read-only.

`Ia32e` therefore makes intermediate entries **maximally permissive** and lets the leaf decide, which
reproduces the other two formats' meaning exactly. The cost is real and is named in the module: the
hierarchical bits are x86's mechanism for revoking a whole subtree in one store, and this gives it
up in exchange for one meaning of "what does this mapping grant" across three architectures.

### 2. Port I/O has no page, so a device capability cannot be a mapping

On aarch64 and RISC-V a device *is* a page, so a device capability is a mapping and the MMU enforces
it. The legacy x86 devices, including the console UART, live in a 16-bit I/O space with no page
tables in front of it. x86 gates that space two ways: `RFLAGS.IOPL` (all-or-nothing per privilege
level) and the TSS **I/O permission bitmap**, one bit per port, which is per-task rather than
per-page.

Nothing uses the bitmap. `TrapFrame::for_user_entry` leaves `IOPL` at 0 and the TSS's bitmap offset
points past the end of the structure, so a ring-3 program on this kernel may not touch a port at
all, which is now the *permanent* answer, not a placeholder while an open question sits above it.
`user::UART_PHYS` is **zero** on this architecture, and that zero stays the marker for the decision
below rather than for anything still open. Written up as **§121 (DECIDED)**: legacy port I/O stays
kernel-resident permanently (option 2), and the port-range-capability alternative this section once
priced as future work (a real TSS-bitmap grant, a genuinely different shape of capability from the
one the tree has) is closed, not deferred. §121 is the first case in this tree where the object a
capability names is not memory, and the answer landing on "the kernel keeps this one" is itself a
recorded finding worth reading if a later capability over something that is not a frame comes up.

### 3. One base cannot do both jobs, so this architecture has two

**Resolved 2026-08-24** (milestone 161's roadmap item 1); left here because it is the one place the
three architectures' address arithmetic genuinely diverges, and a reader of the other two ports will
arrive expecting a single constant.

`KERNEL_VA_BASE` is `0xffffffff80000000` because the target's code model requires it, and there are
only **2 GiB of address space above it**. So `VA = PA | KERNEL_VA_BASE` can never address more than
2 GiB of physical memory, which is not enough for a real machine and not enough to reach the local
APIC at `0xfee00000` either. It is not even invertible up there: `phys_to_virt(0xfee00000)` produced
a valid distinct address whose `virt_to_phys` did not give `0xfee00000` back.

Linux separates the two jobs this one constant was doing, and so does this port now. The kernel
*image* sits at `KERNEL_VA_BASE`, where the code model needs it; the *direct map* sits at
`DIRECT_MAP_BASE = 0xffff888000000000`, which is Linux's `page_offset_base`, taken rather than
invented so that a reader who has met one x86_64 kernel has met this number. They are PML4[511] and
PML4[273], so nothing about them interferes, and both are canonically high, so the same bit-47
`Ia32e::in_half` test admits both. `crates/paging` needed no change, which is the second time this
port has been able to say that.

**`virt_to_phys` therefore has two branches**, and that is not a wart: the kernel hands it linker
symbols (`memory::image_start`) as well as pointers that came out of `phys_to_virt`
(`sched`, `kmem`). Linux's `__pa()` makes the same distinction for the same reason. `phys_to_virt`
has one branch, because everything physical is in the direct map.

`mmu::device_va`, the foot gun that reached device registers through the identity map because the
direct map could not, is **deleted**. So is the identity map itself, which was a complete alias of
physical memory sitting in the half user programs get.

### 4. Two names in the arch contract are aarch64's and do not stretch

- **`arch::psci_cpu_on`** is an ARM firmware interface's name. RISC-V already had to implement it as
  an SBI call underneath; x86 has no third mechanism to hide behind it, because SMP bring-up here is
  INIT followed by two STARTUP IPIs through the local APIC, naming a page below 1 MiB to begin
  executing at **in 16-bit real mode**. The operation is not "power on a CPU", it is "send an
  interrupt".
- **`kernel_main`'s single pointer is called `dtb`.** What arrives on x86 is `hvm_start_info`. The
  *shape* is right (one pointer to everything discoverable) and only the name is wrong.

Both are naming decisions, so both are calef's; a lane records them rather than deciding.

## The bug worth knowing about

**Loading a segment register in long mode destroys that segment's base MSR.** `gs`'s base is where
this kernel keeps its per-CPU pointer, so `segments::init`'s reload of the data selectors silently
zeroed it, and the next `println!` dereferenced null through the console lock's per-CPU rank check.

It presented as an instruction fetch from the middle of a static several frames away, with a
register dump showing a perfectly correct GDT, TSS and IDT. Nothing about the symptom pointed at the
cause. The fix saves and restores the base around the reload, inside `segments::init`, so the
ordering constraint stops existing rather than being documented for callers to remember.

## What the gates cover, and what they do not

`script/lint` runs clippy over the x86_64 kernel binary at `-D warnings`, which covers every line of
`arch/x86_64/` and every portable file compiled with `target_arch = "x86_64"`. Three narrowings, each
argued where the gate is (see `script/lint`):

- **No `--all-targets`**: written when the test suite did not compile for this target. It does now
  (milestone 161's item 4), so this narrowing is stale and is the next thing to widen; the suite is
  gated by `script/test --arch x86_64` in the meantime, which is the stronger check of the two.
- **No feature loop**: all eight boot-mode features fail on x86_64, measured, because each selects a
  boot path needing an arch layer this port has not written.
- **`-A dead_code`**: partly stale for the same reason. The tour no longer halts before the
  scheduler, so much of what was unreferenced now is not; what remains dead here is what needs a
  userspace (the services, the drivers). Code dead on *all three* architectures is still caught by
  the other two passes; what can hide is code dead on x86_64 alone.

**There is a `script/test` leg**: `--arch x86_64`, in `xtask`'s `test`, and it runs by default
alongside the other two. It builds nothing before it boots, because there is no userspace archive to
pack and `scripts/qemu-runner-x86_64.sh` attaches no disks.

## Reproducing it

```sh
cargo build -p kernel --target x86_64-unknown-none
scripts/qemu-bounded.sh 20 qemu-system-x86_64 \
    -machine q35 -cpu max -smp 1 -m 256M -display none -serial stdio -no-reboot \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -kernel target/x86_64-unknown-none/debug/kernel
```

or, equivalently, `cargo run -p kernel --target x86_64-unknown-none` (the runner in
`.cargo/config.toml` builds the same command line; bound it, because the kernel halts rather than
exiting).

Expected output, 2026-08-24 (the memory-map and ACPI blocks elided; they are quoted in full
earlier in this file):

```
nife on x86_64 (long mode, ring 0, 4-level paging)
  cpu 0 booted: high-half kernel, .bss, and the 16550 console are up.
  running at  : 0xffffffff8010b4b0  (high half: the long-mode jump landed)
  boot info   : 0x0000000000001580  (PVH hvm_start_info, not a device tree)
  cpu         : x86_64, vendor AuthenticAMD, cpuid leaves 0..0xd
  traps       : idt installed; a breakpoint was caught and stepped over (1)
  memory      : 9 regions from the PVH handoff, rsdp 0x0
                ...
  acpi        : rsdp at 0xf52e0 (revision 0), root table 0xffe2344 (rsdt)
                ...
  apic        : local apic 0xfee00000 up, id 0, version 0x14, 8259s masked
  clocks      : tsc 1001 MHz, apic timer 62 MHz (both measured against the PIT)
  timer       : 20 ticks in ~0.2s at 100 Hz (20 routed, 0 spurious)
  io apic     : id 0 at 0xfec00000 up, version 0x20, 24 redirection entries, gsi base 0
  device irq  : pit irq 0 -> gsi 2 on vector 0x32: 20 interrupts in ~0.2s at 100 Hz
  frames      : allocator up over 1 ram region(s) (first frame 0x21e000)
  memory          : 254 MiB total, 253 MiB free (1148 KiB in use)
  mmu         : fine W^X 4-level map installed (cr3 0x21f000), image 0xffffffff80000000, direct map 0xffff888000000000
                560 KiB of page tables, no identity map, guard pages are holes
  image       : text 0xffffffff80109000..0xffffffff80144000, stack 0xffffffff8015c000..0xffffffff8016c000
  entropy     : rdseed supported (cpuid leaf 7 ebx.18), drew 0x48d292e828c52899
  scheduler   : up on 1 cpu, preempting at 100 Hz (idle thread registered)
  smp: none yet (x86 uses INIT-SIPI-SIPI via the local APIC; milestone 161)
  smp: 1 core(s) online
  kernel task : a spawned thread ran and carried its captured state (0x16100004)

  user thread 8589934594 killed: vector 14 (page fault)
    rip 0x0000000000400005   addr 0x0000000000a50000   user rsp 0x0000000000501000   err 0x00000004
  the kernel is fine.

  user thread 17179869186 killed: vector 14 (page fault)
    rip 0x0000000000400005   addr 0x0000000000a50000   user rsp 0x0000000000501000   err 0x00000004
  the kernel is fine.
  userspace   : a process built from untyped ran at cpl 3 and sent 0x1610004 on a granted cap
                thread 8589934594 died at pc 0x400005 on addr 0xa50000, delivered to its supervisor
                two children cost 0 frames the first round and 0 the second (steady state)

  next        : real ELF user programs (user_rt has no x86_64 arms), then SMP.
nife x86_64: boot complete, halting.
```

The two `user thread ... killed` blocks are the *faulting* child of each round dying, printed by the
trap path on its way to `sched::fault`. They are the demo working, not a failure; the tour's
`userspace` lines are what assert on them.

To run the kernel's own suite instead of the tour:

```sh
script/test --arch x86_64
```

**The lines after the `mmu` one are the proof, not the `mmu` line itself.** They are printed after
the `mov cr3`, through a console the new tables do not describe (COM1 is a port) but from code and a
stack they do.

The frame numbers and the `rdseed` word change from boot to boot; everything else is stable.

The vendor string is whatever `-cpu` was asked for; `max` on this host reports `AuthenticAMD`.

To see a triple fault instead of a blank terminal, add `-d int,cpu_reset -D /tmp/x86.log`.

## Where TSO pays out

Rule #4 says to assume weak memory ordering because ARM is the weak one and that is a gift. This is
the port where the bet settles, and it settles in the direction it was made: code proven correct
under ARM's model and RISC-V's RVWMO is correct on x86's TSO by construction. `dma_wmb` is an
`sfence` here and is nearly free, because stores are already globally ordered; the fence is there
only for non-temporal stores and write-combining memory, which TSO does not cover.

`sync_icache` does **nothing** on x86, and that is the one place this architecture's complexity buys
something: the instruction cache is architecturally coherent with the data caches. aarch64 needs a
clean/invalidate loop and a broadcast; RISC-V needs `fence.i` locally and an SBI RFENCE remotely, and
getting that wrong is what hung init on first silicon (notes/visionfive2.md).
