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
page-table format, the boot-handoff parser (the memory map and the ACPI root pointer), the address
arithmetic, interrupt masking, the context switch, and the test exit. A boot under QEMU's `q35`
prints a tour and halts.

**Not built:** the fine-grained page tables, the APIC, any clock, VT-d, SMP bring-up, ring 3, and the
kernel's own test suite. Every one of those is a loud `unimplemented!()` in `arch/x86_64/` that names
itself and why. See design/roadmap/161-x86-64-kernel-port.md for the order they come in.

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

The boot map is 4 GiB identity-mapped with 2 MiB pages, plus the first gigabyte aliased at
`KERNEL_VA_BASE`. 4 GiB rather than the 1 GiB the image needs, because everything x86 talks to early
is above 1 GiB and below 4: the local APIC at `0xfee00000`, the IO APIC at `0xfec00000`, and q35's
PCIe ECAM window at `0xb0000000`.

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
zero, which means the RSDP has to be found the older way, by scanning `0xe0000..0x100000` for the
`"RSD PTR "` signature. That range is exactly the third reserved region above. This matters because
everything x86 needs a device tree for (the APIC in the MADT, the ECAM window in the MCFG, the IOMMU
in the DMAR) hangs off the RSDP, so the scan is a prerequisite for the whole next step and cannot be
skipped by trusting the handoff.

## The discovery seam milestone 20 promised and did not build

Milestone 20's deliverable had two abstraction shapes in it. The first, "a generic level-walk plus a
per-arch entry codec", **was built and holds**: `crates/paging` needed no change for a third format.
The second, "put device discovery behind a 'here is the hardware' interface (device tree today,
ACPI/PCI later)", **was not built**, and this port is where that shows.

`kernel/src/memory.rs`'s `init` takes a device-tree pointer and reads `Dtb` directly for the RAM
regions, the reservations, the interrupt controller, the RTC, the UART's interrupt and the PCIe
window. Nothing about that is wrong on two architectures that both have a device tree. On x86 there
is no tree, so **the frame allocator cannot come up at all** until the discovery half is separated
from the consumption half.

The seam is visible and small: everything in `memory::init` after "the whole span we have to be able
to describe" needs only two slices of `Region`, the RAM map and the forbidden list, and none of it
cares where they came from. Splitting it is what the fine-map step needs first, and it is the
largest single piece of portable-code work this port implies. It is not done here because it touches
a file every lane edits and it is a milestone-sized change rather than a step in this one.

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

### 3. Two names in the arch contract are aarch64's and do not stretch

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

Expected output, 2026-08-23:

```
nife on x86_64 (long mode, ring 0, 4-level paging)
  cpu 0 booted: high-half kernel, .bss, and the 16550 console are up.
  running at  : 0xffffffff8010a2f0  (high half: the long-mode jump landed)
  boot info   : 0x0000000000001580  (PVH hvm_start_info, not a device tree)
  cpu         : x86_64, vendor AuthenticAMD, cpuid leaves 0..0xd
  traps       : idt installed; a breakpoint was caught and stepped over (1)
  mmu         : 4-level paging on (cr3 0x102000), boot map: 4 GiB identity + 1 GiB high
  image       : text 0xffffffff80109000..0xffffffff80119000, stack 0xffffffff8012a000..0xffffffff8013a000

  next        : fine-grained page tables, the APIC, a clock, then ring 3.
nife x86_64: early boot complete, halting.
```

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
