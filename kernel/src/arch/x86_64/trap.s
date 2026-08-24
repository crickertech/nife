# The x86_64 trap entry: 256 stubs, one common save/restore path, one Rust handler.
#
# Intel syntax, like boot.s, because that is `global_asm!`'s default on x86.
#
# WHY 256 STUBS AND NOT ONE VECTOR TABLE. aarch64 has sixteen exception vectors, each 128 bytes,
# reached by an offset the hardware computes from the exception class; RISC-V has one, and the cause
# is in `scause`. x86 has neither: the CPU jumps to the address in IDT entry N and tells the handler
# **nothing** about which N that was. The vector number is only recoverable if each entry points at
# a different piece of code that knows its own number. So the number is pushed by the stub, and the
# stubs are generated rather than written out.
#
# THE ERROR CODE IS THE SECOND ASYMMETRY, and it is worse than it looks. Ten of the 32 architectural
# exceptions push an extra word before the return frame; the other 246 vectors do not. If the common
# path did not know which, `iretq` would return to whatever the error code happened to be. The stubs
# push a dummy zero for the vectors that have none, so exactly one frame layout reaches Rust and
# exactly one `add rsp, 16` undoes it.
#
# WHAT IS NOT HERE YET: `swapgs`. A trap from ring 3 arrives with the *user's* GS base, and the
# kernel's per-CPU pointer lives in the kernel's, so the entry and exit paths will each need one
# `swapgs`, guarded by a test of the saved CS's ring bits (a trap from ring 0 must NOT swap, or a
# nested trap swaps back to the user's). Nothing here runs a ring-3 program, so the pair is absent
# rather than wrong; adding it is part of the user-mode step. See notes/x86-port.md.

.section .text
.code64

# ---------------------------------------------------------------------------------------------
# One stub per vector.
#
# `.altmacro` is what makes `%vec` expand to the counter's VALUE at the macro call, which is the
# only way to build 256 distinct labels from a `.rept`. Without it the label would literally be
# `isr_vec` 256 times over.
# ---------------------------------------------------------------------------------------------
.altmacro

.macro ISR_STUB num
.global isr_\num
isr_\num:
    # The ten vectors that push a hardware error code: #DF(8), #TS(10), #NP(11), #SS(12), #GP(13),
    # #PF(14), #AC(17), #CP(21), #VC(29), #SX(30). Everything else gets a zero so the frame the
    # common path sees is one shape.
    .if (\num != 8) && (\num != 10) && (\num != 11) && (\num != 12) && (\num != 13) && (\num != 14) && (\num != 17) && (\num != 21) && (\num != 29) && (\num != 30)
    push 0
    .endif
    push \num
    jmp isr_common
.endm

.set vec, 0
.rept 256
    ISR_STUB %vec
    .set vec, vec + 1
.endr

# ---------------------------------------------------------------------------------------------
# The common path. The push order below IS the layout of `TrapFrame` in exceptions.rs, read
# backwards: the first push lands at the highest address, so `r15` is the last field of the register
# block and `rax` is the first. Reorder one line and Rust reads a different register's value under
# the right name, silently.
# ---------------------------------------------------------------------------------------------
isr_common:
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rbp
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax

    # The System V ABI requires DF clear on entry to a C function, and an interrupt can land while
    # a `std`-using routine holds it set. Nothing in this kernel sets DF, but the handler is not the
    # place to be relying on that.
    cld

    # The frame is 22 quadwords = 176 bytes, and the CPU aligned rsp to 16 before pushing its own
    # part, so rsp is 16-byte aligned here and `call` leaves it at the 8-mod-16 the ABI expects.
    mov rdi, rsp
    call x86_trap_handler

    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop rbp
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15

    add rsp, 16                     # discard the vector number and the error code
    iretq

# ---------------------------------------------------------------------------------------------
# The table of stub addresses, so `exceptions::init` can fill the IDT from a loop in Rust rather
# than needing 256 `extern` declarations.
# ---------------------------------------------------------------------------------------------
.macro ISR_ADDR num
    .quad isr_\num
.endm

.section .rodata
.align 8
.global ISR_STUBS
ISR_STUBS:
.set vec, 0
.rept 256
    ISR_ADDR %vec
    .set vec, vec + 1
.endr
