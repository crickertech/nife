# The kernel's real entry on x86_64, and the 32-bit -> 64-bit -> higher-half transition.
#
# This file is written in INTEL syntax, because that is what Rust's `global_asm!` selects by default
# on x86 and the alternative is an `options(att_syntax)` this kernel would then have to remember
# everywhere. The other two architectures' boot.s have no such choice to make.
#
# HOW WE GET HERE, AND WHY IT IS NOT MULTIBOOT. The obvious answer for `-kernel` on x86 is
# Multiboot 1, and it does not work for a 64-bit kernel: QEMU's loader recognises the header, then
# refuses the image with "Cannot load x86-64 image, give a 32bit one", because the Multiboot 1
# specification says the OS image is ELF32 and QEMU enforces it. Multiboot 2 lifts that restriction
# and QEMU 11 does not implement Multiboot 2 (its x86 loader is one file, hw/i386/multiboot.c, and
# it is version 1 only; checked by inspecting the binary for a version-2 code path, 2026-08-23).
# The header is fatal rather than ignored, so an image cannot carry both and fall back.
#
# So this kernel boots by **PVH**, the direct-boot protocol Xen defined and QEMU, Firecracker and
# cloud-hypervisor all implement. Its entire interface is one ELF note: name "Xen", type
# XEN_ELFNOTE_PHYS32_ENTRY (18), whose descriptor is the physical address to enter at. QEMU finds it
# by walking the PT_NOTE program headers, loads the ELF by p_paddr as usual, and jumps to that
# address **in 32-bit protected mode with paging off**: eax = 0x336EC578, ebx = the physical address
# of a `struct hvm_start_info`, CS/DS flat 4 GiB, interrupts disabled, no valid stack.
#
# PVH is a better fit than Multiboot for this kernel quite apart from the ELF64 question, and the
# reason is what rides in ebx. `hvm_start_info` carries the memory map **and the RSDP address**,
# which is the root of the ACPI tables: on the other two architectures the equivalent single pointer
# is the device tree, and ACPI is what x86 has instead. One pointer in, everything discoverable from
# it, exactly the shape `kernel_main(dtb)` already has.
#
# WHAT THIS DOES NOT BOOT. PVH is a hypervisor protocol. A real machine's firmware does not speak it,
# so milestone 87's OptiPlex will need either a UEFI stub or GRUB's Multiboot (which enters in the
# same 32-bit protected mode this trampoline already expects, so the trampoline itself carries over
# unchanged; only the header and the ebx contract differ). See notes/x86-port.md's BUGS.
#
# That entry mode is the whole reason this file is three times the size of its RISC-V twin. RISC-V
# and aarch64 are entered in the width they run in, so their boot code can be linked high and use
# PC-relative addressing until the MMU comes on. A 32-bit instruction stream cannot name a 64-bit
# address at all, so the trampoline below is linked LOW (VA == PA, the `.boot` output section in
# link-x86_64.ld) and only the second half of this file lives at the high addresses the rest of the
# kernel is linked for.
#
# THE ORDER, AND WHY EACH STEP CANNOT MOVE:
#
#   1. Zero the boot page tables. They are NOLOAD, so nothing has zeroed them, and PML4/PDPT entries
#      we do not write must read as not-present or the CPU walks into garbage.
#   2. Fill them: 4 GiB identity-mapped with 2 MiB pages, the first GiB aliased at KERNEL_VA_BASE
#      (where the image is linked), and the same 4 GiB aliased again at DIRECT_MAP_BASE (where
#      `mmu::phys_to_virt` points). The identity half is not a convenience, it is what keeps
#      `mov cr0, eax` from being the last instruction ever fetched: paging comes on between one
#      instruction and the next, and the next one's address is still low. The direct map is not a
#      convenience either; see step 2e for why it has to exist before Rust runs.
#   3. CR4.PAE, then CR3, then EFER.LME, then CR0.PG. The CPU only enters long mode when PG is set
#      *while* LME is set and PAE is on; any other order either faults or silently enters 32-bit
#      paging, which then walks our 4-level table as a 3-level one and triple-faults.
#   4. A far jump through a 64-bit code descriptor. Setting LME+PG puts the CPU in COMPATIBILITY
#      mode, not 64-bit mode; the L bit in the code segment is what actually widens it, and the only
#      way to load CS is a far transfer.
#   5. Only then jump to the high alias, and only then touch an absolute symbol from the rest of the
#      image.
#
# This is the single sketchiest moment in the x86_64 port, and it fails silently: a wrong page table
# means the instruction after `mov cr0, eax` is fetched through a broken mapping, the CPU takes a
# page fault with no IDT, escalates to a double fault with no IDT, and triple-faults, which on QEMU
# is a machine reset with no output. `-d int,cpu_reset -no-reboot` is how you see it. See
# notes/x86-port.md.

# ---------------------------------------------------------------------------------------------
# The PVH entry note. This is the whole boot header: four words that tell the loader where to jump.
#
# It has to end up in a **PT_NOTE program header**, because that is what QEMU walks; a note sitting
# in a PT_LOAD segment is invisible to it. The link script therefore gives `.note.Xen` its own
# output section (SHT_NOTE, allocated), which is what makes lld emit the PT_NOTE, and does NOT sweep
# it into the `/DISCARD/` rule that drops every other `.note*`.
# ---------------------------------------------------------------------------------------------
.section .note.Xen, "a", @note
.align 4
    .long 2f - 1f                       # namesz
    .long 4f - 3f                       # descsz
    .long 18                            # XEN_ELFNOTE_PHYS32_ENTRY
1:  .asciz "Xen"
2:  .align 4
3:  .long _start                        # the 32-bit entry, a physical address (`.boot` is linked low)
4:  .align 4

# ---------------------------------------------------------------------------------------------
# The 32-bit trampoline. Linked low; every `offset` below is therefore a physical address.
# ---------------------------------------------------------------------------------------------
.section .text.boot, "ax"
.code32
.global _start
_start:
    cli                                 # the loader already masked, but say so
    cld                                 # `rep stos` below depends on DF=0, and nothing else set it

    # ebx holds the `hvm_start_info` pointer and must survive to `kernel_main`. Park it in ebp:
    # a 32-bit write zeroes the upper half of rbp, so the value is already a correct 64-bit physical
    # address by the time long mode reads it. (eax holds PVH's 0x336EC578 magic and is checked in
    # Rust rather than here, where a mismatch could not be reported.)
    mov ebp, ebx

    # --- 1. zero the boot page tables (.boot_scratch is NOLOAD; nothing else has touched it) ---
    mov edi, offset __boot_scratch_start
    mov ecx, offset __boot_scratch_end
    sub ecx, edi
    shr ecx, 2                          # bytes -> dwords
    xor eax, eax
    rep stosd

    # --- 2a. the page directories: 2048 entries of 2 MiB = the low 4 GiB, identity ---
    # 4 GiB rather than the 1 GiB the kernel image needs, because everything x86 talks to early is
    # above 1 GiB and below 4: the local APIC at 0xfee00000, the IO APIC at 0xfec00000, and q35's
    # PCIe ECAM window at 0xb0000000. A boot map that stopped at 1 GiB would mean every one of those
    # had to wait for the real page tables, which is a worse ordering constraint than 16 KiB of
    # page directories.
    mov edi, offset boot_pd
    mov eax, 0x00000083                 # present | writable | PS (this PDE maps a 2 MiB page)
    mov ecx, 2048
1:
    mov [edi], eax
    mov dword ptr [edi + 4], 0          # bits 63:32 of the entry: physical address < 4 GiB, no NX
    add eax, 0x00200000                 # next 2 MiB frame
    add edi, 8
    loop 1b

    # --- 2b. the low PDPT: four entries, one per page directory, covering 0 .. 4 GiB ---
    mov edi, offset boot_pdpt_low
    mov eax, offset boot_pd
    or eax, 0x03                        # present | writable
    mov ecx, 4
2:
    mov [edi], eax
    mov dword ptr [edi + 4], 0
    add eax, 4096                       # the next page directory
    add edi, 8
    loop 2b

    # --- 2c. the high PDPT: one entry, so KERNEL_VA_BASE aliases physical 0 .. 1 GiB ---
    # KERNEL_VA_BASE = 0xffffffff80000000 decomposes as PML4[511], PDPT[510], PD[0], which is why
    # the high alias reuses the FIRST page directory rather than needing one of its own.
    mov edi, offset boot_pdpt_high
    mov eax, offset boot_pd
    or eax, 0x03
    mov [edi + 510 * 8], eax
    mov dword ptr [edi + 510 * 8 + 4], 0

    # --- 2d. the PML4: entry 0 (identity) and entry 511 (the high half) ---
    mov edi, offset boot_pml4
    mov eax, offset boot_pdpt_low
    or eax, 0x03
    mov [edi], eax
    mov dword ptr [edi + 4], 0
    mov eax, offset boot_pdpt_high
    or eax, 0x03
    mov [edi + 511 * 8], eax
    mov dword ptr [edi + 511 * 8 + 4], 0

    # --- 2e. the direct map, at PML4[273], reusing the SAME low PDPT ---
    #
    # THIS IS THE ENTRY THAT MAKES `mmu::phys_to_virt` MEAN ONE THING FOR THE MACHINE'S WHOLE LIFE,
    # and it costs eight bytes because the low 4 GiB are already described by boot_pdpt_low.
    #
    # x86_64 cannot do what the other two architectures do, which is put the kernel image and the
    # direct map at one base: `code-model: kernel` pins every kernel symbol to the top 2 GiB, so
    # KERNEL_VA_BASE is 0xffffffff80000000 and there is no room above it for a map of physical
    # memory. The kernel image therefore keeps that base and the direct map gets its own,
    # DIRECT_MAP_BASE = 0xffff888000000000, which is Linux's and which decomposes as PML4[273].
    #
    # Establishing it HERE rather than when mmu::init builds the fine tables is what removes the
    # sequencing hazard the roadmap warned about: the frame allocator's bitmap, the PVH structure
    # and the ACPI tables are all read through phys_to_virt before the fine map exists, and if that
    # arithmetic changed meaning at the `mov cr3` those pointers would silently start naming
    # somewhere else. It does not change meaning, because this entry and the fine map's direct map
    # are the same base. See arch/x86_64/mmu.rs, which asserts the index below at compile time.
    mov eax, offset boot_pdpt_low
    or eax, 0x03
    mov [edi + 273 * 8], eax
    mov dword ptr [edi + 273 * 8 + 4], 0

    # --- 3. PAE, CR3, LME, PG, in that order ---
    mov eax, cr4
    or eax, 1 << 5                      # CR4.PAE: 4-level paging needs it, and long mode needs it
    mov cr4, eax

    mov eax, offset boot_pml4
    mov cr3, eax

    mov ecx, 0xC0000080                 # IA32_EFER
    rdmsr
    or eax, 1 << 8                      # LME: long mode enable
    or eax, 1 << 11                     # NXE: bit 63 of a page entry means no-execute rather than
                                        # reserved. Set here, before any table exists that uses it,
                                        # because a page table built while NXE is clear and walked
                                        # while it is set changes meaning under the walker.
    wrmsr

    mov eax, cr0
    or eax, 1 << 31                     # PG: paging on. Long mode begins at this instruction's
                                        # retirement, and the next fetch goes through boot_pml4.
    or eax, 1 << 16                     # WP: supervisor writes honour the read-only bit. Without it
                                        # the kernel can write user read-only pages, which is a
                                        # confinement hole the other two architectures do not have
                                        # an equivalent switch for.
    mov cr0, eax

    # --- 4. widen to 64 bits by loading a code segment whose L bit is set ---
    lgdt [boot_gdt_pointer]
    # A far jump, hand-encoded. LLVM's Intel-syntax parser spells this several mutually incompatible
    # ways depending on version, and getting it wrong here produces a triple fault with no output
    # rather than an assembler error, so the three bytes are written out: EA = jmp far ptr16:32,
    # then the 32-bit offset, then the 16-bit selector.
    .byte 0xEA
    .long long_mode_entry
    .word 0x08                          # boot_gdt entry 1: 64-bit code, DPL 0

.code64
long_mode_entry:
    # Load the flat data descriptor everywhere. In 64-bit mode the base and limit of these are
    # ignored, but the segment registers still have to hold something loadable, and a stale 32-bit
    # descriptor from the loader's own GDT is not it once we have replaced the GDT.
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    # Leave the identity map behind. `movabs` is the only way to get a 64-bit immediate into a
    # register, and this is the first instruction in the kernel that names a high address.
    movabs rax, offset _start_high
    jmp rax

# ---------------------------------------------------------------------------------------------
# The 64-bit, high-half world. From here every absolute symbol resolves correctly.
# ---------------------------------------------------------------------------------------------
.section .text, "ax"
.code64
_start_high:
    movabs rsp, offset __stack_top      # the high boot stack

    # Zero .bss by hand (it occupies no bytes in the ELF). Both bounds are page-aligned, so the
    # 8-byte store loop divides evenly. es and DF are already set correctly above.
    movabs rdi, offset __bss_start
    movabs rcx, offset __bss_end
    sub rcx, rdi
    shr rcx, 3
    xor eax, eax
    rep stosq

    # kernel_main(hvm_start_info_physical_address). The other two architectures pass a device-tree
    # pointer in this argument; x86 has no device tree, so what travels is PVH's `hvm_start_info`,
    # which is where the memory map and the ACPI RSDP address live. See arch/x86_64/machine.rs.
    mov rdi, rbp
    # The System V AMD64 ABI wants rsp 16-byte aligned *before* the call instruction pushes the
    # return address. __stack_top is page-aligned, so it already is; the alignment is stated here
    # because a misaligned stack only shows up much later, in SSE code that does not exist yet.
    call kernel_main

    # kernel_main is `-> !`. If it ever returns, stop rather than run off into whatever follows.
3:  hlt
    jmp 3b

# ---------------------------------------------------------------------------------------------
# The boot GDT: the minimum that makes a far jump into 64-bit mode legal.
#
# Three descriptors, because x86 requires a null one, a code one with L set, and a data one for
# ss/ds. It is deliberately NOT the GDT the kernel runs on: that one carries a TSS (the only way
# x86 knows where to put the stack on a ring-3 -> ring-0 transition) and is installed from Rust
# once there is somewhere to allocate it. See arch/x86_64/segments.rs.
# ---------------------------------------------------------------------------------------------
.section .data.boot, "a"
.align 16
boot_gdt:
    .quad 0x0000000000000000            # 0x00: the mandatory null descriptor
    .quad 0x00AF9A000000FFFF            # 0x08: code, DPL 0, L=1 (64-bit), present
    .quad 0x00CF92000000FFFF            # 0x10: data, DPL 0, present
boot_gdt_end:

boot_gdt_pointer:
    .word boot_gdt_end - boot_gdt - 1   # the limit is size MINUS ONE, and always has been
    .long boot_gdt                      # a 32-bit base, because lgdt runs in 32-bit mode

# ---------------------------------------------------------------------------------------------
# The boot page tables. NOLOAD (see link-x86_64.ld), so these are 28 KiB of reserved physical
# address space rather than 28 KiB of zeros in the image; the trampoline zeroes them itself.
#
# boot_pd is FOUR contiguous pages and the code above depends on that adjacency: it walks the low
# PDPT writing boot_pd, boot_pd+4096, boot_pd+8192, boot_pd+12288.
# ---------------------------------------------------------------------------------------------
.section .bss.boot, "aw", @nobits
.align 4096
boot_pml4:
    .space 4096
boot_pdpt_low:
    .space 4096
boot_pdpt_high:
    .space 4096
boot_pd:
    .space 4 * 4096

# ---------------------------------------------------------------------------------------------
# The secondary-CPU entry, which on this architecture does not yet exist.
#
# `smp::bring_up_secondaries` takes this symbol's address and hands it to `arch::psci_cpu_on`, so
# the symbol has to resolve or the kernel does not link. It cannot be the real thing yet, and the
# reason is worth writing down rather than leaving as an empty stub: an x86 secondary is started by
# INIT followed by two STARTUP inter-processor interrupts through the local APIC, and a STARTUP IPI
# carries an 8-bit vector naming a page **below 1 MiB** that the CPU begins executing **in 16-bit
# real mode**. So the entry point cannot be a label in this image at all: it has to be a copy of a
# real-mode trampoline placed in low memory at run time, which then repeats the whole protected-mode
# and long-mode transition above before it can reach any of this.
#
# `psci_cpu_on` returns an error on this architecture, so nothing ever jumps here. If something
# does, halting is the only safe thing: a CPU that runs on into the wrong mode corrupts memory
# rather than stopping.
# ---------------------------------------------------------------------------------------------
.section .text.boot, "ax"
.code16
.global secondary_boot
secondary_boot:
    cli
1:  hlt
    jmp 1b
