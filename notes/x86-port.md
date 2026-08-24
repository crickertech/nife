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
the address arithmetic, interrupt masking, the context switch, and the test exit. A boot under
QEMU's `q35` prints a tour, takes real hardware interrupts from the CPU's own timer *and* from a
device, builds and installs its own page tables, and halts.

**Not built:** VT-d, SMP bring-up, ring 3, and the kernel's own test suite. Every one of those is a
loud `unimplemented!()` in `arch/x86_64/` that names itself and why. See
design/roadmap/161-x86-64-kernel-port.md for the order they come in.

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

### BUGS

- **PVH is a hypervisor protocol and no real firmware speaks it.** Milestone 87's OptiPlex will need
  a UEFI stub or GRUB's Multiboot. The trampoline itself carries over unchanged, because GRUB also
  enters in 32-bit protected mode with paging off; only the header and the `ebx` contract differ.

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
which is now confirmed twice over (the PVH memory map reports the same range as reserved). The
constant should come from here rather than being checked against here, and doing that is a line of
code once something consumes it.

**`8259s present (must be masked)`** is the MADT's `PCAT_COMPAT` flag, and it is a real obligation
rather than trivia: the legacy PICs are still wired and still raise interrupts, so whoever brings the
APIC up has to mask both of them first or a spurious interrupt arrives through a controller nothing
is driving.

**What is deliberately not decoded is AML**, the bytecode in the DSDT that describes everything with
no fixed table, including PCI interrupt routing (`_PRT`). AML needs an interpreter, which is a
project rather than a parser. That is the reason `arch::mmu::PCI_IRQ_BASE` is zero and honest about
it: a PCI function's legacy interrupt goes through a router the `_PRT` describes, and MSI bypasses
the routing entirely by writing a vector straight to the local APIC. The MSI path is the one worth
building here, precisely because it needs no AML.

## The discovery seam milestone 20 promised and did not build

Milestone 20's deliverable had two abstraction shapes in it. The first, "a generic level-walk plus a
per-arch entry codec", **was built and holds**: `crates/paging` needed no change for a third format.
The second, "put device discovery behind a 'here is the hardware' interface (device tree today,
ACPI/PCI later)", **was not built**, and this port is where that shows.

`kernel/src/memory.rs`'s `init` took a device-tree pointer and read `Dtb` directly for the RAM
regions, the reservations, the interrupt controller, the RTC, the UART's interrupt and the PCIe
window. Nothing about that is wrong on two architectures that both have a device tree. On x86 there
is no tree, so **the frame allocator could not come up at all**.

**The narrow half is now split out** (`memory::bring_up_frames`), because without it nothing below
the allocator can exist on this architecture and the port would have stopped there. `init` is now
explicitly a device-tree *front end*: it reads the tree, assembles a RAM slice and a forbidden slice,
and hands them to a function that does not care where they came from.
`arch::x86_64::machine::bring_up_memory` is the x86 front end doing the same job from the PVH map.

**The wide half is still owed, and it is the milestone.** What crosses the seam today is RAM and
reservations only. The device *windows* the tree front end also discovers stay in `memory.rs`'s
statics and stay `None` on x86, so `memory::pci_regions()` reports nothing even though the ACPI MCFG
answered a few lines earlier. Two facts now have two sources that do not know about each other,
which is the shape this should not be left in:

| Fact | Device-tree machines | x86 |
|---|---|---|
| RAM, reservations | `memory::init` | `machine::bring_up_memory` (**shared consumer**) |
| Interrupt controller | `memory::init` -> `GIC_REGIONS`/`PLIC_REGION` | ACPI MADT -> `machine::Acpi` -> `irq::init_{local,io}_apic` **directly**, bypassing the statics |
| PCIe ECAM window | `memory::init` -> `PCI_REGIONS` | ACPI MCFG -> `machine::Acpi`, unwired |
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
the direct map in different places. That is not hypothetical: `memory::bring_up_frames` turns the
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

Nothing here uses the bitmap yet, because nothing runs in ring 3. When something does, that bitmap is
where a port grant has to be recorded, and it is a genuinely different shape of capability from the
one the tree has. `user::UART_PHYS` is **zero** on this architecture, and that zero is the marker for
this question rather than a value. Written up as **§121 (PROPOSED)**, because it is the first case in
this tree where the object a capability names is not memory, and how that is answered decides the
shape of every future capability over something that is not a frame.

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

- **No `--all-targets`**: the kernel's test suite does not compile for x86_64, because its
  userspace-exec tests hand-assemble machine code and its supervision fixtures are per-ISA stubs.
  Bringing the suite up is downstream of ring 3 existing here.
- **No feature loop**: all eight boot-mode features fail on x86_64, measured, because each selects a
  boot path needing an arch layer this port has not written.
- **`-A dead_code`**: the boot tour halts before the scheduler, so ~85 items live on the other two
  ISAs are unreferenced here. Code dead on *all three* architectures is still caught by the other two
  passes; what can hide is code dead on x86_64 alone.

There is **no `script/test` leg for x86_64**, for the same reason there is no `--all-targets`.

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
  frames      : allocator up over 1 ram region(s) (first frame 0x1fc000)
  memory          : 254 MiB total, 253 MiB free (1012 KiB in use)
  mmu         : fine W^X 4-level map installed (cr3 0x1fd000), image 0xffffffff80000000, direct map 0xffff888000000000
                556 KiB of page tables, no identity map, guard pages are holes
  image       : text 0xffffffff80109000..0xffffffff80126000, stack 0xffffffff8013a000..0xffffffff8014a000

  next        : the IO APIC, then ring 3.
nife x86_64: early boot complete, halting.
```

**The lines after the `mmu` one are the proof, not the `mmu` line itself.** They are printed after
the `mov cr3`, through a console the new tables do not describe (COM1 is a port) but from code and a
stack they do.

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
