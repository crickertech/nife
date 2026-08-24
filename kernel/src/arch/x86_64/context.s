# The x86_64 context switch and the first-run trampolines (the asm half of context.rs).
#
# Intel syntax, like the rest of this architecture's assembly.
#
# WHAT A CONTEXT IS HERE. The System V AMD64 ABI's callee-saved set is rbx, rbp, r12, r13, r14 and
# r15: six registers, against RISC-V's thirteen and aarch64's twelve. Everything else is the
# compiler's to spill, so a switch that saves those six and the return address has saved everything
# that can outlive a call. The saved stack pointer is the context pointer itself, so it is not a
# field, exactly as on the other two.

.section .text
.code64

# void switch_to(Context **prev_context, Context *next_context)
#
# rdi = where to store OUR context pointer, rsi = the context to resume.
#
# The last instruction returns to a DIFFERENT thread: `ret` pops the return address that thread
# pushed when it was switched away from (or, for a thread that has never run, the trampoline address
# context.rs put there).
.global switch_to
switch_to:
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov [rdi], rsp                  # our context pointer is our stack pointer
    mov rsp, rsi                    # adopt theirs
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret

# The first-run landing pad for a KERNEL thread.
#
# `Context::for_kernel_thread` put the closure pointer in rbx and the monomorphized call shim in
# rbp, because those are two of the six registers the switch restores. The closure's concrete type
# was erased, so the address says *where* and the shim says *how*.
#
# rsp is 16-byte aligned here by construction (see context.rs's alignment note), which is what the
# `call` below requires.
.global thread_trampoline
thread_trampoline:
    mov rdi, rbx                    # closure_at
    mov rsi, rbp                    # call_shim
    xor rbp, rbp                    # the bottom of the backtrace
    call thread_entry
    # thread_entry is `-> !`. If it ever returns, stop rather than run on.
1:  hlt
    jmp 1b

# The first-run landing pad for a USER thread. rbx = entry, rbp = user stack pointer,
# r12..r14 = the child's first three arguments.
.global user_entry_trampoline
user_entry_trampoline:
    mov rdi, rbx
    mov rsi, rbp
    mov rdx, r12
    mov rcx, r13
    mov r8, r14
    xor rbp, rbp
    call user_thread_entry
1:  hlt
    jmp 1b
