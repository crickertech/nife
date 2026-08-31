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

### The RAM that is deliberately left on the table

206684 KiB against PVH's 261627 KiB, so **about 54 MiB of a 256 MiB machine is reported as
reserved that a Linux-style loader would reclaim.** That is a choice, not a defect, and the
reasoning is in `uefi_loader::handoff`'s `BUGS`: UEFI's `EfiBootServicesCode`/`Data` become free the
moment `ExitBootServices` returns, but this loader's own handoff structure, memory map, module list
and trampoline live in `EfiLoaderData`, and the kernel reads three of those *after* the frame
allocator is up. Reporting the whole class as reserved is the conservative direction, and the
direction matters: claiming less RAM than exists costs megabytes, claiming more corrupts something,
on hardware nobody can attach a debugger to. Splitting the loader's four known allocations back out
would recover the rest and is the fix if it ever matters.

## Running it

```console
$ cargo xtask uefi-image                       # kernel + archive + loader, staged at target/esp
$ scripts/qemu-uefi-x86_64.sh target/esp       # boot it under OVMF
$ cargo xtask uefi-boot                        # both of the above, plus the assertions
```

`cargo xtask uefi-boot` also runs inside `script/test --arch x86_64`, after the PVH suite. It boots
the **tour** rather than the suite, and that is a cost decision stated where it is paid: the tour is
about ten seconds and covers the whole boot path, where re-running two hundred tests under a second
firmware would buy coverage of the tests rather than of the firmware.

What it asserts, chosen so it cannot pass for the wrong reason:

- `(xsdt)` in the ACPI line, which only the firmware path can print.
- no `rsdp 0x0`, which is PVH's own tell.
- the tour's completion line, which is everything in between: the fine W^X page tables, the APIC,
  the timer, the scheduler, and two ring-3 processes, on a memory map from firmware.

`NIFE_OVMF_CODE` and `NIFE_OVMF_VARS` name the firmware images on a machine that keeps them
somewhere other than Homebrew's QEMU share; `NIFE_UEFI_TIMEOUT` moves the bound.

## The bench: booting nife on the OptiPlex 7050

**This has not been done.** Everything above is QEMU with real firmware in the loop, which is as
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

## BUGS

- **The bench procedure is written and untested.** Every firmware-menu path, key and setting name
  above is from the 7050's documented behaviour rather than from this machine, and the first person
  to follow it should expect at least one of them to be worded differently on the screen.
- **`cargo xtask uefi-boot` boots the tour, not the suite.** So the two hundred kernel tests have
  never run under firmware. Nothing suggests they would behave differently (the firmware is gone by
  the time the kernel's first instruction runs), but it is an untested claim rather than a checked
  one. Embedding the test ELF instead of the tour's is a two-line change to `uefi_image` if it
  becomes worth the time.
- **SMP under UEFI has never been exercised.** `arch::x86_64::ap_boot` copies its real-mode
  trampoline to physical `0x8000`, which is memory this loader never asks the firmware for. Both
  the runner and the bench procedure boot one core. Asking for that page in the loader is the fix if
  it turns out to matter.
- **Nothing verifies what the loader hands over.** The kernel and the archive are bytes the loader
  was compiled with, so the trust boundary is the build; `measured_boot`'s manifest is not consulted
  and the image is not signed. That is also why Secure Boot has to be off.
- **A stale `.efi` on a stick is silent.** The loader embeds the kernel, so a stick that was written
  last week boots last week's kernel with nothing to say so. `cargo xtask uefi-image` rebuilds both
  every time, which moves the hazard to the copy step rather than removing it.
