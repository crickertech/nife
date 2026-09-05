# Booting x86_64 from real firmware

Milestone 87. The x86_64 port boots under QEMU by **PVH**, and `notes/x86-port.md`'s `BUGS`
already said what that costs:

> PVH is a hypervisor protocol and no real firmware speaks it. Milestone 87's OptiPlex will need a
> UEFI stub or GRUB's Multiboot.

This note is the answer: what was chosen, what the alternative actually cost when it was priced
rather than argued about, how the thing works, and **the exact procedure for the bench**, which is
the part of this milestone that a person has to carry out by hand.

## The fork, and it was decided by two commands

Both options were real, both were priced, and this is what the pricing found. The rule the tenets
give here is *recommend on reversible forks*: the two paths can coexist (a GRUB Multiboot 2 header
is thirty lines and does not disturb what is below), so this is a recommendation acted on rather
than a question sent up.

### What UEFI costs

```console
$ ls /opt/homebrew/share/qemu/ | grep edk2-x86_64
edk2-x86_64-code.fd
edk2-x86_64-secure-code.fd
```

**OVMF, the open-source UEFI implementation, ships with the QEMU this project already pins.**
Nothing to install. And the FAT filesystem the firmware reads is QEMU's own `vvfat` block driver,
which synthesises one out of a host *directory*, so there is no image-building step and no
`mtools`:

```console
$ command -v mformat xorriso
$                                   # neither is installed
```

On the machine end, the OptiPlex 7050 is UEFI-native, so the operational cost is **copying one
file to a FAT32 stick** as `/EFI/BOOT/BOOTX64.EFI`. That path is the removable-media fallback every
UEFI implementation looks for with no configuration at all.

The engineering cost is what §46 would have to weigh if a dependency were involved, and there is
none: this loader speaks six firmware functions and two GUIDs, hand-written in
`uefi_loader/src/efi.rs`. The `uefi` crate was not taken, and the reason is that crate's size
against this need rather than anything about its quality.

### What GRUB costs

```console
$ brew info grub
Error: No available formula with the name "grub".
```

**GRUB cannot be installed on this development machine at all.** Homebrew has no `grub` formula on
macOS, so `grub-mkrescue` cannot be run here, and neither can `xorriso`, which it needs. The GRUB
path could still be *written* on patagonia, but it could not be **proved** on patagonia: gating it
would mean a Linux container in the loop for every run, or building GRUB for the `x86_64-efi`
target from source.

On the machine end GRUB also costs calef more, not less: a bootloader installed on the box, or a
rescue ISO written to a stick with a tool he does not have.

### The decision

**UEFI.** It is testable today with what is installed, it is the shorter path to the machine, and
it is what the target hardware natively is. GRUB Multiboot 2 stays available and cheap to add if a
BIOS-only machine ever turns up: the 32-bit trampoline in `kernel/src/arch/x86_64/boot.s` is
already the entry state GRUB delivers, so the delta would be a header and a second handoff decoder.

**And the Multiboot hazard was checked rather than assumed.** `notes/x86-port.md` records QEMU
refusing an image over a Multiboot **1** header, fatally. Nothing in this milestone adds a
Multiboot header of any version, so that hazard is untouched: the PVH note is still the only boot
header in the image, and `script/test --arch x86_64` still boots through it.

## The design, and the one decision the rest follows from

**The kernel is not modified.** It is entered through its existing `_start`, in 32-bit protected
mode, with PVH's register contract: `eax` = `0x336EC578`, `ebx` = the physical address of an
`hvm_start_info`. The kernel cannot tell which loader started it.

That was chosen against the obvious alternative, which was a second 64-bit entry point in the
kernel that rebuilt the boot page tables in long mode. Two entries would have meant two contracts,
two page-table builders, and a real chance of breaking the PVH path that every
`script/test --arch x86_64` run rides. **Thirty-two instructions in the loader buy one entry point
in the kernel**, and that is the whole trade.

Everything else falls out of it:

| The loader does | Because |
|---|---|
| synthesises an `hvm_start_info` | `machine_discovery::x86_64` already decodes it, host-tested, and `arch::x86_64::machine` already consumes it |
| places the kernel at `p_paddr` | that is what a boot loader does, and this kernel's `p_vaddr` and `p_paddr` are unrelated (see below) |
| leaves long mode as its last act | that is the *only* state difference between what UEFI hands over and what QEMU's PVH loader hands the kernel |
| embeds the kernel and the archive | one file on the stick, and no `SimpleFileSystem` protocol to speak |

The pieces:

- `uefi_loader/src/lib.rs` and its three modules are the **pure half**: the firmware table layouts
  (`efi`), the `hvm_start_info` writer (`handoff`), and the physical-address ELF reading (`image`).
  All of it compiles for the host and is tested there, for the reason `crates/dtb` and
  `machine_discovery` exist rather than living inside `arch/`: a structure layout proved only by
  booting is proved by nothing that runs in milliseconds.
- `uefi_loader/src/main.rs` is the **half that cannot be**: it calls firmware and it changes CPU
  mode.
- `uefi_loader/src/leave_long_mode.s` is the mode switch.

### The handoff is the kernel's own, which is the hazard this closes

Milestone 87's brief named the risk directly: *make both entries produce the same internal
structure for `kernel_main`*, because a divergence would first show up on hardware nobody can
attach a debugger to. Synthesising an `hvm_start_info` is how that is made true rather than
promised. There is one structure, one decoder, and one set of tests, and `uefi_loader::handoff`'s
tests **decode their own output with the crate the kernel decodes with**, so the writer and the
reader cannot drift apart without a host test failing in milliseconds.

### Leaving long mode, in the order the CPU requires

UEFI hands over in 64-bit long mode with the firmware's identity page tables live. The kernel wants
32-bit protected mode with paging off. `leave_long_mode.s`:

1. `lgdt` a GDT carrying a **32-bit** code descriptor, which is the thing the firmware's GDT does
   not have and the only reason a GDT is loaded at all.
2. Far-*return* into compatibility mode. A far return is the shape that works from 64-bit mode:
   `jmp far ptr16:32` is invalid there. The instruction is hand-encoded (`48 CB`) for exactly the
   reason `boot.s` hand-encodes its far jumps: LLVM's Intel-syntax parser spells it several
   mutually incompatible ways and getting it wrong is a triple fault with no output.
3. Reload the data selectors, because they still name descriptors in the GDT we just replaced.
4. Clear `CR0.PG`. **This is what actually leaves long mode**: `IA32_EFER.LMA` is not a bit
   software writes, it is `LME && CR0.PG`. It is safe only because the trampoline is executing from
   an identity-mapped page, so the next instruction is fetched at the same address either way.
5. Clear `LME`, set `eax`/`ebx`, jump to the kernel.

**It is copied to a page below 4 GiB before it runs**, and both halves of that matter. Below 4 GiB
because the second half executes with paging off, where a linear address is a physical one and the
firmware may well have loaded this image above 4 GiB. And the page is allocated as
`EfiLoaderCode` rather than `EfiLoaderData`, because firmware with a memory-protection policy
(OVMF has one, and so does every recent vendor firmware) sets the execute-disable bit on data
allocations, and the first instruction of the copy would then fault with boot services still live
and nothing watching.

### Why the ELF reading is not `crates/elf`

That crate is the kernel's *user-program* loader, and a program is placed at its `p_vaddr`. A boot
loader places an image at its `p_paddr`, and on this kernel the two are not related by any offset a
caller could apply:

```text
.text            p_vaddr 0xffffffff80109000   p_paddr 0x109000
.ap_trampoline   p_vaddr 0x0000000000008000   p_paddr 0x165000
```

`elf::Segment` does not carry `p_paddr` at all, so there is nothing to subtract. Adding it is the
tidier answer and wants its own lane: that struct is public and the kernel's loader is its
consumer, so widening it is a change to a shared definition.

The other trap the physical span has to survive is **`NOLOAD`**. `.boot_scratch` (the boot page
tables) and the two per-CPU stack areas arrive as `PT_LOAD` segments with `p_filesz == 0` and a
real `p_memsz`. They are address space to reserve rather than bytes to copy, and a loader that
skipped them would leave the firmware free to hand that memory to something else while the kernel's
trampoline zeroes its own page tables on top of it.

## What it proves, measured

The table below was measured with both boots at `-m 256M`, before milestone 195 reclaimed
boot-services memory (the `usable ram` row is the one that moved; see "The RAM that is deliberately
left on the table" below, which now records both figures). **The runner's default is now 2 GiB**
(`NIFE_MEM=256M` reproduces the table): firmware places its ACPI tables just under the top of RAM, so
the memory size decides what physical addresses the kernel is asked to read, and at 256 MiB they land
low enough that a reach bug in the ACPI walk cannot show. One did, for as long as this script matched
its PVH sibling; see notes/x86-port.md's BUGS.

The same kernel, same machine model, same `-m 256M`, booted twice. Everything that differs is the
firmware.

| | PVH (`-kernel`) | UEFI (OVMF) |
|---|---|---|
| memory map | 9 regions | **118 regions** |
| `rsdp` in the handoff | `0x0` | **`0xfb7e014`** |
| RSDP revision / root | 0, **RSDT** | **2, XSDT** |
| PCIe ECAM (from the MCFG) | `0xb0000000` | **`0xe0000000`** |
| usable RAM | 261627 KiB | 206684 KiB |
| RAM regions the allocator got | 1 | 7 (8 with an archive) |

Four of those are code paths that had **never executed**:

- **The non-zero `rsdp`.** `notes/x86-port.md` records `rsdp 0x0` under QEMU's PVH loader, so
  `arch::x86_64::machine::find_rsdp` has always fallen back to scanning the BIOS area for
  `"RSD PTR "`. Under UEFI the pointer arrives in the handoff and the scan is skipped.
- **The XSDT walk.** A scanned ACPI 1.0 RSDP has revision 0 and a 32-bit RSDT root. Firmware hands
  over a revision-2 RSDP with a 64-bit XSDT, which is a different branch in
  `machine_discovery::acpi`. This is the assertion `cargo xtask uefi-boot` gates on, because it is
  the one string that cannot be printed by the PVH path.
- **An ECAM window that is not the hardcoded constant.** Milestone 165 made
  `memory::record_pci_regions` follow the MCFG rather than `arch::mmu::PCI_ECAM_PHYS`; under PVH
  the two agreed, so nothing distinguished "read the table" from "used the constant". Under UEFI
  they disagree and the kernel follows the table.
- **A fragmented memory map.** 118 descriptors instead of 9, and the frame allocator comes up over
  seven RAM regions instead of one.

And the userspace archive arrives the same way it does under PVH, through the module list this
loader writes:

```text
  initrd      : 5229568 bytes at 0xd9bc000, 68 programs, from the PVH module list
```

### The RAM that was left on the table, and how much came back

**Before milestone 195: 206684 KiB against PVH's 261627 KiB**, so about 54 MiB of a 256 MiB machine
was reported reserved that a Linux-style loader would reclaim. That was a choice rather than a
defect, in the conservative direction on purpose: claiming less RAM than exists costs megabytes,
claiming more corrupts something, on hardware nobody can attach a debugger to.

**After it: 233148 KiB on the same 256 MiB machine, and 2068244 KiB against 2032128 on a 2 GiB one.**
Two classes moved, both of them dead by the time the kernel reads the map:

- **`EfiBootServicesCode` and `EfiBootServicesData`**, which the UEFI specification says are free the
  moment `ExitBootServices` returns. 26 MiB of the 2 GiB machine.
- **`EfiLoaderCode`**, which is the loader's own PE image and its one-shot mode-switch trampoline.
  The image is the whole embedded payload (9 MiB for the tour build, 19 MiB for the test build), and
  the kernel and the archive were copied *out* of it before boot services ended. 9 MiB more.

**`EfiLoaderData` stays reserved and must.** Every allocation this loader makes that the kernel reads
later asks for it: the `hvm_start_info`, the memory map, the module list, the archive, and the kernel
image itself. The asymmetry is not a rule anyone has to remember, which is what makes it hold: the
only thing asking for `LOADER_CODE` is the trampoline, and it asks because firmware sets the
execute-disable bit on data allocations.

What is still reported reserved on the 256 MiB machine is 28 MiB, against PVH's 0.5 MiB. That is the
firmware's own runtime services, ACPI NVS, its reserved ranges, and the loader's `EfiLoaderData`, and
none of it is ours to take.

## Running it

```console
$ cargo xtask uefi-image                       # kernel + archive + loader, staged at target/esp
$ scripts/qemu-uefi-x86_64.sh target/esp       # boot it under OVMF
$ cargo xtask uefi-boot                        # both of the above, plus the assertions
```

```console
$ cargo xtask uefi-test                        # the kernel's TEST binary under the same firmware
```

Both run inside `script/test --arch x86_64`, after the PVH suite, and they are two boots rather than
one because they carry two different kernels.

`uefi-boot` boots the **tour**, which is the build `uefi-image` stages for the USB stick and the one
calef carries to the bench. It runs at **two cores**, which no other x86_64 boot in this tree does;
see the SMP section below.

What it asserts, chosen so it cannot pass for the wrong reason:

- `(xsdt)` in the ACPI line, which only the firmware path can print.
- no `rsdp 0x0`, which is PVH's own tell.
- the tour's completion line, which is everything in between: the fine W^X page tables, the APIC,
  the timer, the scheduler, and two ring-3 processes, on a memory map from firmware.
- `smp: 2 core(s) online`, and a PIT interrupt still reaching the boot core with two local APICs on
  the machine.

`uefi-test` boots the kernel's **test binary**, which is the same kernel with `test_main()` on the
end of the same tour, so the boot prints every line above and then runs the suite. Milestone 195
added it, and until it existed *"it boots under real firmware"* and *"it passes under real firmware"*
were different claims with only the first one made. It asserts the harness's verdict **and QEMU's
exit status**, because a transcript scan alone would pass a run that printed its verdict and then
faulted on the way out.

**The two boots are the same machine except for their devices**: the suite gets the PVH runner's
`virtio-blk-pci` disk and NVMe controller, the tour gets neither. That is what makes the numbers
comparable, and on 2026-09-02 they were **identical**: 192 passed and 68 skipped under both PVH and
OVMF, with the same 68 test names skipped on each side.

### What running the suite under firmware cost, and what it found

**It was not the two-line change to `uefi_image` this was scoped as**, and the reason is the one
thing a tour boot cannot show: the test build is bigger. Its physical span reaches 10 MiB where the
tour's reaches 2.3, and OVMF keeps ACPI NVS at 8 MiB and its own boot-services allocations from 9 to
23.5, so `AllocatePages(AllocateAddress)` refused the whole range and the firmware printed nothing
more useful than `Load Error`.

Two things came out of that, and the second matters more than the first.

**`PHYS_START` moved from 1 MiB to 32 MiB** (`kernel/link-x86_64.ld`). 1 MiB is the *lowest* address
multiboot permits, never the only one, and under a hypervisor's loader nothing else is in low memory
to say so. **This is a larger gap rather than a fix**: the image is still placed at one address
chosen at link time, so a firmware that wants 32 MiB refuses the boot exactly as OVMF refused 1 MiB.
The real answer is a physically relocatable image, which is a milestone rather than a constant,
because `.boot` holds 32-bit absolute references to its own labels.

**And the loader now names what is in the way.** `AllocatePages` reports one status and no address,
so on that failure `uefi_loader::say_conflict` walks the memory map and prints every descriptor
overlapping the range that is not free RAM, with its UEFI type. On the bench that is the difference
between a `Load Error` and a sentence:

```text
uefi_loader: wanted 0x0000000000100000..0x0000000000add000
uefi_loader:   in the way: 0x0000000000800000..0x0000000000808000
uefi_loader:   memory type 0x000000000000000a
uefi_loader:   in the way: 0x0000000000900000..0x0000000001780000
uefi_loader:   memory type 0x0000000000000004
```

### SMP under firmware

`arch::x86_64::ap_boot` copies its real-mode trampoline to physical `0x8000`, because a STARTUP IPI
can only name a page below 1 MiB. Until milestone 195 the loader never mentioned that page, so
secondary cores under firmware worked or did not **by luck**: OVMF happens to leave the first 640 KiB
conventional. The loader asks the firmware for it by name now, and a refusal is a printed warning
rather than a failed boot, because a single-core boot on a machine whose firmware wants that page is
worth more than no boot at all.

Two cores come up under OVMF, five runs out of five, and `uefi-boot` gates it. **The suite stays at
one core**, and that is a known defect rather than a preference: `every_secondary_runs_scheduled_work`
fails about half the time at two cores on this architecture (`arch::x86_64::ap_boot`'s `BUGS` #3),
which is why the PVH runner defaults to one as well. The tour does not run that test.

`NIFE_OVMF_CODE` and `NIFE_OVMF_VARS` name the firmware images on a machine that keeps them
somewhere other than Homebrew's QEMU share; `NIFE_UEFI_TIMEOUT` moves the bound.

## First light on xenon, 2026-09-05

**It has now been done, and the machine talked without a serial cable.** Photograph:
`art/bench/xenon-2026-09-05-first-light.jpg`, which is the transcript, because patagonia could not be
moved to the bench and the Dell's video output was the only channel.

**Milestone 243's framebuffer console carried the whole boot tour on its first contact with real
firmware.** It had never run outside OVMF:

```
screen : 1920x1080 bgrx @ 0xd0000000, 274x135 cells (boot cmdline)
```

**How far it got**, all of it read off the screen: long mode, high-half kernel, the long-mode jump
landed, a breakpoint caught and stepped over, **148 memory regions** from the PVH handoff, ACPI
revision 2 with a real XSDT at `0xcae090b8` and the full table list, **4 cores enumerated, 4 enabled**,
PCI ECAM, TSC and PIT-measured timers at 100 Hz, and **17,119 MiB total**.

**Three things are firsts against real firmware rather than OVMF**, and each was predicted here:

- **`8259s present (must be masked)`.** Real legacy PICs.
- **`PCI_ECAM_PHYS says 0xb0000000`** while the MCFG reports `0xe0000000`. The hardcoded constant
  disagrees with the firmware table and the kernel follows the table, which is exactly what the
  "What it proves, measured" section above says the UEFI path exercises and the PVH path cannot.
- **148 regions** against OVMF's 118 and PVH's 9.

### What the boot menu documents, which is more than it looks

`art/bench/xenon-2026-09-05-boot-menu.jpg`, the F12 menu, is the first record of this machine's own
identity rather than of the model:

- **OptiPlex 7050, BIOS revision 1.27.0.**
- **Boot mode UEFI, Secure Boot OFF**, which is what steps 1 and 2 below ask for and confirms they
  were reachable as written.
- The stick enumerates as **`UEFI: SanDisk Ultra 1.26`**, so the removable-media fallback at
  `\EFI\BOOT\BOOTX64.EFI` is found with no boot entry created.
- **A `Windows Boot Manager` entry**, so this machine dual-boots and its internal disk holds
  somebody's installation. Worth knowing before anything here writes to a disk.
- **`UEFI: Micron 2450 NVMe 256GB`.**

**That last line matters beyond the bench.** DECISIONS §86 (whether an NVMe driver can leave the
kernel, and what capability would let it) was decided on 2026-09-03, and its own research recorded
that no board this project owns has an IOMMU in front of a real NVMe controller. **xenon has both**:
milestone 87's requirements list says it was selected partly for VT-d, and this photograph shows the
NVMe. So the confined-driver experiment §86 exists to enable has real hardware to run on, which
nobody had established.

### And then it panicked, in the one place a bigger machine would find

```
[PANIC] panicked at kernel/src/arch/x86_64/mmu.rs:325:33:
failed to build the kernel page tables: AlreadyMapped
```

**Diagnosed 2026-09-05 from this photograph, and the leading hypothesis was wrong.** It read the
framebuffer aperture at `0xd0000000` against 17 GB of RAM and concluded the two had met. They had
not, and the memory map on the screen is what says so: the aperture sits in the 32-bit MMIO hole,
which this firmware's map **does not describe at all**. The last entry below it ends at
`0xd0000000`, the next begins at `0xf0000000`, and the aperture is in no RAM region and never was.

**What actually collided is the local APIC**, and the mechanism is one line older than milestone
243. `map_firmware_regions` direct-mapped every loader-reserved entry **below the top of RAM**,
cacheably, and its own comment said why that bound was the load-bearing part: the reserved entries
*above* the top of RAM are the MMIO windows, and those must be device-typed. That is a true
statement about a machine whose RAM ends below the hole, which is every machine this tree had
booted. xenon's RAM ends at **`0x42e000000`**, so *every* MMIO window it has is below the bound:

| Reserved entry, from the photograph | What it is | Below `0x42e000000`? |
|---|---|---|
| `0xcbe00000..0xd0000000` | firmware's carve-out at the top of low DRAM | yes |
| `0xf0000000..0xf8000000` | PCH decode | yes |
| `0xfe000000..0xfe011000` | PCH | yes |
| `0xfec00000..0xfec01000` | **IO APIC** | yes |
| `0xfee00000..0xfee01000` | **local APIC** | yes |
| `0xff000000..0x100000000` | SPI flash | yes |

So the cacheable fill claimed `0xfee00000` a few lines before step 5 asked for the same page
device-typed, and the mapper refused, which is exactly what it is for. The framebuffer never got
that far: the local APIC is the first window step 5 maps.

**Two things are worth separating here.** The panic is the smaller half. The larger half is that on
this machine the fill was also mapping the IO APIC, the SPI flash and 128 MiB of PCH decode
**cacheably**, which is a write that can sit in a cache line and never reach the device. Nothing
had touched those yet, so nothing had failed; the panic is what made it visible.

**The fix, on `maintainer/already-mapped-on-real-ram`.** The fill's bound is now the address at
which the firmware's map stops describing memory, walked as a chain upward from the low megabyte:
DRAM and the carve-outs firmware takes out of it are contiguous, and the MMIO hole above them is a
gap. On xenon that is `0xd0000000`; on every machine whose RAM ends below the hole it is the same
number the old bound produced. The device windows are also enumerated before the fill now, so
device typing wins by construction rather than by that bound being right.

**And the panic now names both ranges.** `AlreadyMapped` on its own cost this session a diagnosis:
`map_everything` maps eleven kinds of range and the message distinguished none of them. Every
direct-map range now comes out of one enumeration that the failure path also walks, so the message
is `AlreadyMapped mapping local apic 0xfee00000..0xfee01000: 0xfee00000 is also claimed by firmware
reservation 0xfee00000..0xfee01000`. That is the difference between a bench session that ends in a
diagnosis and one that ends in a hypothesis, and it is the half worth keeping regardless of whether
the fix is right.

### Confirming the fix, which only xenon can do

**A green QEMU run is not a confirmation and should not be reported as one.** The runner boots with
2 GiB, `notes/x86-uefi-boot.md`'s own comparison table was taken at `-m 256M`, and neither reaches a
memory map with RAM above the MMIO hole. `-m 17G` on patagonia (16 GB) swaps rather than reproduces.
What the suite does prove is that the rule answers the old bound's number on a small machine, and
the `x86_64` kernel leg carries xenon's map as a fixture (`map_tests` in
`kernel/src/arch/x86_64/mmu.rs`, transcribed from the photograph) so the 17 GiB case is asserted
without the machine.

The bench session that confirms it, in the shape of the procedure below:

1. Build the stick exactly as step 3 below says, from `maintainer/already-mapped-on-real-ram` or
   from `main` once it has landed.
2. Boot it. **The line to watch for is `mmu`**, which the boot tour prints after `map_everything`
   returns:
   `mmu : fine W^X 4-level map installed (cr3 ...), image 0xffffffff80000000, direct map 0xffff888000000000`
   followed by the page-table cost in KiB. Reaching that line at all is the confirmation: it is one
   line past where the machine stopped on 2026-09-05.
3. **Photograph the whole screen anyway**, not just that line. The page-table cost on the second
   line is the number nobody has ever seen from a real machine, and this module's `BUGS` prices 4
   KiB leaves at 0.2% of RAM, so ~33 MiB is the prediction to check against.
4. **If it panics again, the message is now the deliverable.** Photograph it and stop; it names the
   two ranges, so no further bench time is needed to say what happened.
5. The tour continues past the `mmu` line into ring 3 and the scheduler. Everything after it is new
   ground on this machine and none of it has been seen on real firmware, so expect the next stop
   somewhere else and treat that as progress rather than as this fix failing.

### What it settles about the procedure below

**The procedure worked**, including its warning that firmware-menu wording would differ. calef also
photographed every page of the BIOS configuration, which is the first real record of this machine's
settings; the `BUGS` entry saying every setting name here came from Dell's documentation rather than
from the machine can be retired once those are transcribed.

**And one thing it changes:** step 1's expectation that the loader's two lines are "the only sign the
stick was read at all" is no longer true. With milestone 243 the screen carries the whole tour, so a
serial-less bring-up is a real bench session rather than a stare.

## The bench: booting nife on the OptiPlex 7050

**This section was written before first light and is kept as the procedure. It has now been run
once**; see above. Everything above is QEMU with real firmware in the loop, which is as
far as this lane could get; the machine is calef's bench. This section is the procedure, written to
be followed rather than interpreted.

### What you need

- The OptiPlex 7050 Micro, its Dell C4PDJ serial module installed, and the dev-side RS-232 chain
  (FTDI USB adapter, StarTech NM9FF null-modem barrel) already on the desk (milestone 87's own
  block).
- A USB stick, **formatted FAT32** with a GPT or MBR partition table. macOS Disk Utility: *Erase*,
  format **MS-DOS (FAT)**, scheme **GUID Partition Map**.

### Build and copy

```console
$ cd /path/to/nife
$ cargo xtask uefi-image
wrote .../target/esp/EFI/BOOT/BOOTX64.EFI (9110528 bytes: the loader, the kernel and the archive)

$ mkdir -p /Volumes/NIFE/EFI/BOOT
$ cp target/esp/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
$ diskutil eject /Volumes/NIFE
```

**The path and the capitalisation are the interface.** `\EFI\BOOT\BOOTX64.EFI` is the removable-media
fallback the firmware looks for with no configuration; anything else needs a boot entry created on
the machine.

That is the whole of it: **one file**. There is no kernel to copy separately and no configuration
file, because the loader carries both inside itself (`uefi_loader/build.rs`).

### Firmware settings, and the one that will bite

Enter setup with **F2** at the Dell splash; **F12** is the one-time boot menu.

1. **Secure Boot: off.** This image is unsigned and nothing in this tree signs it. On a 7050 that
   is *Secure Boot → Secure Boot Enable → Disabled*, and it may require *Boot List Option* to be
   **UEFI** first. **Expect to have to do this**; a Secure Boot machine will refuse the stick with a
   security-violation message and no other explanation.
2. **Boot List Option: UEFI**, not Legacy. Legacy/CSM boot would look for an MBR boot sector, which
   this stick does not have.
3. **Serial port: enabled.** The C4PDJ module presents COM1 at I/O port `0x3f8`, which is where the
   kernel's console driver looks (`arch::x86_64::port`). If the firmware exposes an address or IRQ
   choice, `0x3f8` / IRQ 4 is the one.
4. **Leave the rest alone on the first attempt.** In particular do not disable the integrated NIC
   or change the SATA mode: nothing here needs them, and a changed setting is one more variable in
   a bring-up that already has enough.

### Watch it

On the Mac, before powering the machine on:

```console
$ ls /dev/cu.usbserial-*                      # the FTDI adapter
$ screen /dev/cu.usbserial-XXXX 115200        # exit with ctrl-a k
```

115200 8N1, which is what `drivers/ns16550.rs` programs.

### What you should see, in order

1. **On the video output, before anything reaches serial:** `nife uefi_loader: milestone 87`, then
   `uefi_loader: kernel placed, exiting boot services`. The Dell's serial port does not carry
   *firmware* output, so this is the only sign the stick was read at all, which is exactly why the
   loader prints it. Attach a monitor for the first attempt.
2. **On serial, immediately after:** the kernel's own tour, beginning

   ```text
   nife on x86_64 (long mode, ring 0, 4-level paging)
     cpu 0 booted: high-half kernel, .bss, and the 16550 console are up.
   ```

   and ending `nife x86_64: boot complete, halting.` **The first line is milestone 87's own
   completion criterion**: the machine has printed a byte over serial.

### If it does not

Triage in this order, because each step rules out everything above it.

| Symptom | What it means | What to do |
|---|---|---|
| Firmware says "security violation" or silently skips the stick | Secure Boot is on | Disable it (above) |
| Firmware boots to its own shell or to the internal disk | the stick was not seen as bootable | Check the path and case: `/EFI/BOOT/BOOTX64.EFI`. Re-format FAT32, not exFAT |
| `nife uefi_loader: milestone 87` and then a message beginning `uefi_loader:` | the loader ran and refused, and the message says why | Every one of those strings is a literal in `uefi_loader/src/main.rs`; read it there |
| The loader's two lines and then nothing, ever | the handoff or the mode switch failed, or the kernel died before the console | See below |
| Video lines, but serial silent while the machine is clearly alive | the serial chain, not the software | Loop back the null-modem barrel's pins 2 and 3 and confirm `screen` echoes typing |
| The machine reboots in a loop | a triple fault | See below |

**A triple fault or a dead machine after the loader's lines is the hard case**, and the honest
answer is that this is where the QEMU work stops helping. What is available: boot with the archive
left out (the loader builds fine with no `NIFE_UEFI_INITRD`, and the tour says
`initrd: none`), which removes five megabytes of copying and the whole userspace half of the tour.
And `-cpu qemu64` under QEMU is the nearest thing to "a CPU that has told us no" (`notes/cpu-models.md`);
`-cpu max` on the dev machine has never refused this kernel anything.

### If it works

Two things are then worth doing, in this order, and neither is in this lane's scope:

1. **Record the numbers**, the way `notes/visionfive2.md` does for the VisionFive 2: the memory map
   the firmware reports, the ACPI tables it carries, the measured TSC rate against the PIT (this
   will *not* be QEMU's 1001 MHz; it is an i5-7500T, and `user_rt::cntfrq`'s hardcoded constant will
   be wrong with no way for a caller to tell, which `notes/x86-port.md` already records), and
   whether the DMAR is present so VT-d can come up.
2. **Flip milestone 87's status** and open the two follow-ups the bench will inevitably produce.

## What xenon still has to establish, and what it no longer has to

Milestone 215 listed three things only the OptiPlex could confirm. **Two of them are now answered on
patagonia**, because the suite runs under firmware with the same devices the PVH runner attaches:

- **A PCI function's MSI-X table, once *firmware* rather than this kernel placed its BARs.** OVMF
  enumerates the bus before nife exists, and `pci::bar_census` reports 5 of 8 functions with a BAR
  outside the window `mmu::map_everything` maps, so `place_bars` **moved** five rather than assigning
  them. The two milestone 215 tests that reach a `virtio-blk-pci` function through its MSI-X table
  pass on the far side of that move.
- **A machine with more than one local APIC still delivering to the boot core's id.** The tour boots
  at two cores under OVMF and its `device irq` line shows the PIT's interrupt arriving 20 times in
  0.2 s at 100 Hz, with two APICs in the MADT.

**The third is still xenon's, and so is everything below it**: whether the OptiPlex's firmware leaves
interrupt remapping off. Nothing under QEMU can answer that, because the answer is a setting in
somebody else's firmware.

And this milestone added one of its own for the bench, which is the more interesting of the two:
**whether the Dell's firmware leaves 32 MiB free.** OVMF's low-memory habits are OVMF's. If it does
not, the loader now says which ranges it wants and which descriptors are in the way, and that message
is the whole difference between a bring-up and a stare.

## BUGS

- **The bench procedure is written and untested.** Every firmware-menu path, key and setting name
  above is from the 7050's documented behaviour rather than from this machine, and the first person
  to follow it should expect at least one of them to be worded differently on the screen.
- **The kernel is placed at one address chosen at link time, and 32 MiB is not a guarantee.** It
  clears every low reservation OVMF makes and nothing more; a firmware that wants that range refuses
  the boot. The image is not physically relocatable, and making it so is a milestone rather than a
  constant: `.boot` is linked at its physical address because a 32-bit instruction stream cannot name
  a 64-bit one, and its absolute self-references are what would have to become position-independent.
- **The bench procedure still boots one core**, unlike the runner. Nothing stops two, and two work
  under OVMF; the procedure is written for a first bring-up where every variable costs.
- **The suite under firmware runs at one core**, so nothing exercises AP bring-up under UEFI *and* the
  scheduler's cross-core tests together. That is `ap_boot`'s open two-core defect rather than
  anything about firmware, and it is the PVH runner's situation too.
- **Nothing verifies what the loader hands over.** The kernel and the archive are bytes the loader
  was compiled with, so the trust boundary is the build; `measured_boot`'s manifest is not consulted
  and the image is not signed. That is also why Secure Boot has to be off.
- **A stale `.efi` on a stick is silent.** The loader embeds the kernel, so a stick that was written
  last week boots last week's kernel with nothing to say so. `cargo xtask uefi-image` rebuilds both
  every time, which moves the hazard to the copy step rather than removing it.
