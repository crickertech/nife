# **A ring-3 probe program**, hand-assembled, for `exceptions::ring3_self_test` (milestone 161,
# roadmap item 3). Intel syntax, like the rest of this architecture's assembly.
#
# WHY THIS EXISTS AND WHAT IT IS NOT. The other two architectures prove ring 3 by loading a compiled
# ELF out of the initrd and running it under the scheduler. Neither is available here yet: no user
# program is built for `x86_64-unknown-none`, and the scheduler has not been brought up on this
# architecture (roadmap item 4). So this is the shape the aarch64 and RISC-V ports both shipped on
# their first day and then deleted: a few instructions in `.rodata`, copied into a user frame and
# entered directly. It proves the privilege boundary and the syscall ABI, and it proves nothing
# about the loader, argument passing, or the process model, which is what item 4 is for.
#
# IT MUST BE POSITION-INDEPENDENT, because it is linked in the kernel's `.rodata` and executed at a
# user virtual address. Every instruction below uses immediates and registers only; there is no data
# reference, no `rip`-relative operand and no absolute address in it.
#
# WHAT IT IS HANDED. `TrapFrame::for_user_entry` puts the kernel's three arguments in rdi/rsi/rdx.
# Argument 0 is a **kernel** virtual address, which this program is meant to be unable to read.

.section .rodata
.align 16

.global x86_ring3_probe_start
x86_ring3_probe_start:
    mov r12, rdi                    # keep the kernel address; rdi is about to be an argument

    # ---- 1. The shared dispatcher, asked something it knows how to refuse. -------------------
    # {ABI} is a syscall number `crate::syscall::dispatch` does not implement, so its answer is
    # Error::BadSyscall, written back through `TrapFrame::set_arg(0)`, which on this architecture is
    # rdi. Asking for a refusal rather than a service is deliberate: it is the one syscall the
    # portable dispatcher can serve on a kernel with no scheduler, so this round trip exercises the
    # REAL dispatcher and the real ABI accessors rather than a stand-in.
    xor rdi, rdi
    xor rsi, rsi
    mov rax, {ABI}
    syscall
    mov rbx, rdi                    # the dispatcher's answer, kept for the report below

    # ---- 2. Report what privilege we are at. -------------------------------------------------
    # Reaching this instruction at all is half the proof: it means the kernel took the syscall above
    # and put us back in ring 3. CS's low two bits ARE the current privilege level, maintained by the
    # hardware, so reporting the register is reporting the CPU's own answer rather than ours.
    mov ax, cs
    movzx rdi, ax
    mov ax, ss
    movzx rsi, ax
    mov rdx, rbx
    mov rax, {REPORT}
    syscall

    # ---- 3. The boundary, from the other side. -----------------------------------------------
    # r12 is mapped -- a process root carries the kernel's high half, so the page is THERE -- and its
    # leaf is supervisor-only. At CPL 3 this is a page fault whose error code has the U/S bit set,
    # which is the hardware saying "the walk found it and refused you". At CPL 0 it would simply
    # load, which is why this is a stronger statement than the CS report above rather than a repeat
    # of it.
    mov rax, [r12]

    # ---- Only reached if that load SUCCEEDED, which would mean we are not confined. -----------
    mov rdi, rax
    mov rax, {ESCAPED}
    syscall
1:  jmp 1b

.global x86_ring3_probe_end
x86_ring3_probe_end:
