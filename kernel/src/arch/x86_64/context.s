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
# r13..r15 = the child's first three arguments.
#
# THE REGISTERS HERE WERE WRONG UNTIL MILESTONE 161 AND NOTHING COULD HAVE CAUGHT IT. This read
# r12, r13 and r14 while `Context::for_user_thread` wrote r13, r14 and r15, so a child would have
# received (0, arg0, arg1) and arg2 would have vanished. Both files were internally consistent and
# neither was executed: this port had no way to enter ring 3 until item 3 of its roadmap, so the
# only witness would have been a user program reading its own arguments. It is corrected against
# context.rs, whose per-field doc comments are the more specific of the two records.
#
# It is STILL not executed: this trampoline is the scheduler's entry path, and the scheduler has not
# been brought up on this architecture (roadmap item 4). The ring-3 self test enters through
# `enter_user` directly and does not pass here.
#
# RESERVE THE TRAP FRAME BEFORE THE FIRST RUST FRAME EXISTS (milestone 71's fix, carried across for
# parity; the other two ISAs each have this line and the reasoning in full). We arrive with
# rsp = the kernel stack top, and `user::enter_frame` puts this thread's TrapFrame at top - 176,
# where every trap from ring 3 will rebuild it (`TSS.RSP0` = top). Without this the entry path's own
# frames start at the same top and overlap the region `frame.write` is about to fill. 176 is
# size_of::<TrapFrame>(), asserted in exceptions.rs, and a multiple of 16 so rsp stays aligned for
# the `call` below.
.global user_entry_trampoline
user_entry_trampoline:
    sub rsp, 176                    # reserve [top-176, top) for this thread's TrapFrame
    mov rdi, rbx
    mov rsi, rbp
    mov rdx, r13
    mov rcx, r14
    mov r8, r15
    xor rbp, rbp
    call user_thread_entry
1:  hlt
    jmp 1b
