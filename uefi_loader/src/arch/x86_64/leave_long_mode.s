# Leaving long mode, which is the one thing a UEFI loader has to do that a hypervisor's loader does
# not. Intel syntax, matching kernel/src/arch/x86_64/boot.s and for the same reason: that is what
# Rust's `global_asm!` selects by default on x86.
#
# WHY THIS EXISTS AT ALL. The kernel's entry contract is PVH's: 32-bit protected mode, paging off,
# eax = 0x336EC578, ebx = the physical address of an `hvm_start_info`. QEMU's PVH loader hands the
# machine over in exactly that state. UEFI hands it over in the opposite one: 64-bit long mode, with
# the firmware's identity page tables live. So the whole delta between "a hypervisor started us" and
# "real firmware started us" is this file, and the kernel is entered through the SAME `_start` in
# both cases.
#
# THAT IS THE DESIGN, NOT AN ACCIDENT. The alternative was a second 64-bit entry point in the kernel
# that rebuilt the boot page tables in long mode, which would have meant two entry contracts, two
# page-table builders, and a real chance of breaking the PVH path that every `script/test --arch
# x86_64` run rides. Thirty-two instructions in the loader buy one entry point in the kernel.
#
# POSITION-INDEPENDENT, BECAUSE IT DOES NOT EXECUTE WHERE IT WAS LINKED. The firmware loads this
# image wherever it likes, possibly above 4 GiB, and a 32-bit instruction stream cannot be fetched
# from up there. So `main.rs` allocates one page BELOW 4 GiB, copies this whole blob into it, and
# calls it there. Nothing below references a link-time address: the two addresses this code needs
# arrive in registers, and the GDT pointer's base is patched by the copier.
#
# THE CONTRACT, System V AMD64 (`extern "sysv64"`, explicitly, because the UEFI target's default
# calling convention is Windows x64 and the two disagree about every one of these registers):
#
#   rdi = the kernel's 32-bit entry point, physical. Must be < 4 GiB.
#   rsi = the `hvm_start_info` this loader built, physical. Must be < 4 GiB.
#   rdx = the physical address of `x86_leave_long_mode_gdtr` in the COPY.
#   rcx = the physical address of `x86_leave_long_mode_pmode32` in the COPY.
#
# It does not return, and it must be called with boot services already exited: after `mov cr0, eax`
# below there is no firmware left to report a failure to.

.text
.code64
.global x86_leave_long_mode
x86_leave_long_mode:
    # The firmware left interrupts enabled and its own IDT installed. Both are about to stop
    # meaning anything, so mask before rather than after.
    cli
    cld

    # Our own GDT, in the copied page. It carries a 32-bit code descriptor (L=0, D=1), which is the
    # thing the firmware's GDT does not have and the whole reason a GDT is loaded here.
    lgdt [rdx]

    # Far-return into COMPATIBILITY MODE: still long mode, but executing 32-bit code, because CS's
    # L bit is clear. This is the only way to load CS with a descriptor that is not 64-bit, and a
    # far return is the shape that works from 64-bit mode (`jmp far ptr16:32` is invalid there).
    #
    # `lretq` pops RIP first, then CS, so CS is pushed first. `push imm8` pushes eight bytes in
    # 64-bit mode, sign-extended, which is what the pop expects.
    push 0x08                           # x86_leave_long_mode_gdt entry 1: 32-bit flat code
    push rcx                            # the offset half: pmode32, in the copy
    # Hand-encoded, exactly as boot.s hand-encodes its far jumps and for the same reason: LLVM's
    # Intel-syntax parser spells the 64-bit far return several mutually incompatible ways depending
    # on version, and getting it wrong here is a triple fault with no output rather than an
    # assembler error. REX.W (0x48) + RET FAR (0xCB).
    .byte 0x48, 0xcb

.code32
.global x86_leave_long_mode_pmode32
x86_leave_long_mode_pmode32:
    # Compatibility mode, 32 bits wide. The data segment registers still hold the firmware's
    # selectors, which name descriptors in a GDT we have just replaced, so reload them all before
    # anything touches memory through one.
    mov eax, 0x10                       # x86_leave_long_mode_gdt entry 2: 32-bit flat data
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    # LEAVE LONG MODE. `IA32_EFER.LMA` is not a bit software writes: it is `LME && CR0.PG`. So
    # clearing PG here is what actually drops the machine into legacy 32-bit protected mode, and it
    # is safe only because this page is identity-mapped in the firmware's tables: the next
    # instruction is fetched at the same address with paging on or off.
    mov eax, cr0
    and eax, 0x7fffffff                 # clear CR0.PG
    mov cr0, eax

    # Tidy up behind us. Not strictly required (the kernel sets LME again in its own trampoline),
    # but a machine left claiming "long mode enabled" with paging off is a state nothing else in
    # this tree expects, and `rdmsr`/`wrmsr` here cost four instructions.
    mov ecx, 0xC0000080                 # IA32_EFER
    rdmsr
    and eax, 0xfffffeff                 # clear LME
    wrmsr

    # PVH's entry contract, which is now literally true rather than emulated: this is the state
    # QEMU's own loader leaves the machine in. eax and ebx are set last because `rdmsr` above
    # clobbers eax and edx, and `mov ds, ax` clobbered it before that.
    mov eax, 0x336ec578                 # machine_discovery::x86_64::MAGIC
    mov ebx, esi                        # the hvm_start_info, physical
    jmp edi                             # the kernel's `_start`, physical

# ---------------------------------------------------------------------------------------------
# The data half of the blob, copied along with the code above.
#
# Three descriptors, the minimum a far transfer needs: the mandatory null one, 32-bit flat code, and
# 32-bit flat data. Deliberately NOT the kernel's `boot_gdt`, which is not loaded yet (it lives in
# the image we have only just copied into place, and reaching it would mean trusting a linker symbol
# from a different binary).
# ---------------------------------------------------------------------------------------------
.align 16
.global x86_leave_long_mode_gdt
x86_leave_long_mode_gdt:
    .quad 0x0000000000000000            # 0x00: the mandatory null descriptor
    .quad 0x00CF9A000000FFFF            # 0x08: 32-bit flat code, DPL 0, D/B=1, present
    .quad 0x00CF92000000FFFF            # 0x10: 32-bit flat data, DPL 0, present

# The `lgdt` operand, and it is the 64-BIT form: a 16-bit limit followed by an EIGHT-byte base,
# because the `lgdt` above executes in 64-bit mode. (boot.s's is the four-byte form, because its
# `lgdt` runs in 32-bit mode. Getting these two swapped loads a GDT at a truncated or garbage
# address and triple-faults at the far transfer.)
#
# The base reads zero here and is PATCHED by `main.rs` after the copy, because the only correct
# value is the address the copy landed at, which nothing at link time can know.
.global x86_leave_long_mode_gdtr
x86_leave_long_mode_gdtr:
    .word 23                            # limit: three descriptors, size MINUS ONE
    .quad 0                             # base: patched to the copy's `x86_leave_long_mode_gdt`

.global x86_leave_long_mode_end
x86_leave_long_mode_end:
