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
# THE `swapgs` PAIR, AND WHY IT IS GUARDED RATHER THAN UNCONDITIONAL (milestone 161, roadmap
# item 3). A trap from ring 3 arrives with the USER's GS base, and this kernel keeps its per-CPU
# pointer in the kernel's, so the first thing that takes a lock would read whatever the user left in
# `IA32_GS_BASE`. `swapgs` exchanges `IA32_GS_BASE` with `IA32_KERNEL_GS_BASE`, which is what makes
# the kernel's pointer unforgeable: the value the kernel needs is in a register the user cannot
# write, exactly as RISC-V's `sscratch` is.
#
# It is an EXCHANGE, not a load, so it must run exactly once per privilege change in each direction.
# A trap from ring 0 that swapped would install the user's base while running kernel code; a nested
# trap that swapped again would put the kernel's back and then hand it to the user on the way out.
# So both sites test the saved CS's low two bits, which ARE the interrupted CPL, and swap only for a
# 3. Note what is deliberately not tested: `SS`, which a `syscall` return leaves unreliable, and the
# frame's RPL alone, which is the same two bits read a longer way round.
#
# `swapgs` writes an MSR pair and does NOT load a segment register, so it does not trip the hazard
# `segments.rs` documents at length (loading a segment register in long mode zeroes that segment's
# base MSR). Verified against the SDM's description of the instruction rather than assumed, because
# that hazard has already cost this port an afternoon once.
#
# THE WINDOW ON THE WAY OUT is the one place this is delicate. Between the exit `swapgs` and the
# `iretq` the CPU is in ring 0 holding the USER's GS base, so an interrupt taken there would see
# CPL 0, decline to swap, and dereference the user's value. `RFLAGS.IF` is clear at every such site
# (an interrupt gate cleared it on entry; `IA32_FMASK` clears it for `syscall`; the two entry points
# below `cli` before they touch anything), which closes it for everything except an NMI or a machine
# check. Those need a paranoid entry path that reads `IA32_GS_BASE` and decides, which this port
# does not have; it is recorded in notes/x86-port.md rather than pretended away.

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

    # Recover the kernel's per-CPU pointer if this trap crossed a privilege boundary. `cs` is field
    # 18 of the TrapFrame (offsets are asserted against `offset_of!` in exceptions.rs), and its low
    # two bits are the interrupted CPL. See this file's header for why the test is here and not a
    # bare `swapgs`.
    test byte ptr [rsp + 18*8], 3
    jz 1f
    swapgs
1:

    # The System V ABI requires DF clear on entry to a C function, and an interrupt can land while
    # a `std`-using routine holds it set. Nothing in this kernel sets DF, but the handler is not the
    # place to be relying on that.
    cld

    # The frame is 22 quadwords = 176 bytes, and the CPU aligned rsp to 16 before pushing its own
    # part, so rsp is 16-byte aligned here and `call` leaves it at the 8-mod-16 the ABI expects.
    mov rdi, rsp
    call x86_trap_dispatch

    # Restore a TrapFrame at rsp and return from the trap. Shared by the IDT path above, by the
    # `syscall` path below, and by the first-entry-to-ring-3 path after that, which is what keeps
    # "how a frame becomes running registers" a single piece of code with a single swapgs rule. The
    # RISC-V twin of this label is `trap_return`.
isr_restore:
    # ON A RETURN TO RING 3, RECORD WHERE THE NEXT TRAP FROM THIS THREAD SHOULD LAND (milestone
    # 161, roadmap item 4). Until the scheduler existed there was one user program and one kernel
    # stack, and `ring3_self_test` set both by hand; with threads there is one kernel stack per
    # thread and the two doors into the kernel have to be re-pointed every time the thread that
    # would come through them changes.
    #
    # THE FRAME'S OWN ADDRESS IS THE ANSWER, which is what makes this a rule rather than a
    # bookkeeping duty somebody has to remember at each switch. Every thread's TrapFrame lives at
    # `stack_top - 176` for the life of the thread (kernel/src/user.rs `enter_frame`, milestone 71),
    # so the top is `rsp + 176` at this instant, computed from the frame we are about to load rather
    # than from any record of who is running. RISC-V does exactly this, at exactly this point, in
    # `trap_return`.
    #
    # Two writes, because x86 has two doors and they find their stack differently: a trap reads
    # `TSS.rsp0` and `syscall` reads nothing at all. `segments::set_kernel_stack` keeps the same pair
    # in step for the boot-thread case; this is the per-trap half.
    #
    # rax and rcx are free here: every general register is still in the frame below and is about to
    # be popped over.
    test byte ptr [rsp + 18*8], 3   # cs, whose low two bits are the ring we are returning to
    jz 3f
    lea rax, [rsp + 176]            # this thread's kernel-stack top
    mov [rip + X86_SYSCALL_KERNEL_RSP], rax
    mov rcx, [rip + X86_TSS_RSP0]
    mov [rcx], rax
3:
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

    # The registers are gone, so `cs` is now three quadwords up: vector, error code, rip, cs. Swap
    # back if we are returning to ring 3, and do it as late as possible: everything between here and
    # the `iretq` runs in ring 0 holding the user's GS base.
    test byte ptr [rsp + 3*8], 3
    jz 2f
    swapgs
2:

    add rsp, 16                     # discard the vector number and the error code
    iretq

# ---------------------------------------------------------------------------------------------
# The `syscall` entry (milestone 161, roadmap item 3). `IA32_LSTAR` points here.
#
# THIS IS NOT AN IDT VECTOR AND THE DIFFERENCES ALL MATTER. `syscall` is a two-cycle jump, not an
# interrupt: it does not consult the IDT, does not read `TSS.RSP0`, and therefore **does not switch
# stacks** -- `rsp` still names the user's. It also does not push anything. What it does is save
# `rip` into `rcx` and `RFLAGS` into `r11` (which is why the ABI's fourth argument rides in `r10`;
# see exceptions.rs), load `rip` from `IA32_LSTAR`, take CS/SS from `IA32_STAR[47:32]`, and clear
# every `RFLAGS` bit named by `IA32_FMASK` -- including `IF`, so this arrives with interrupts
# masked exactly as an interrupt gate would.
#
# So the first three instructions are the whole of what the hardware did not do, and their order is
# forced: nothing may touch a lock (which reads the per-CPU block through `gs`) before the `swapgs`,
# and nothing may push before `rsp` names a kernel stack.
#
# THE KERNEL STACK COMES FROM A STATIC, WHICH IS A SINGLE-CPU ANSWER. `segments::set_kernel_stack`
# writes `X86_SYSCALL_KERNEL_RSP` and `TSS.RSP0` together, so the two mechanisms cannot name
# different stacks; see its BUGS for what SMP will need instead.
# ---------------------------------------------------------------------------------------------
.global x86_syscall_entry
x86_syscall_entry:
    swapgs
    mov [rip + X86_SYSCALL_USER_RSP], rsp
    mov rsp, [rip + X86_SYSCALL_KERNEL_RSP]

    # Build the same 22-quadword TrapFrame the IDT stubs build, so `isr_restore` above serves this
    # path unchanged and `crate::syscall::dispatch` reads one layout. The five words at the top are
    # the ones a real trap's hardware would have pushed, reconstructed from where `syscall` put
    # them.
    push {USER_DATA}                 # ss
    push [rip + X86_SYSCALL_USER_RSP]       # rsp, as it was in ring 3
    push r11                                # rflags, as `syscall` saved them
    push {USER_CODE}                 # cs, whose low two bits make the exit `swapgs` fire
    push rcx                                # rip: the instruction after the `syscall`
    push 0                                  # error code: there is none
    push {SYSCALL_VECTOR}                     # not an IDT vector; see exceptions.rs

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

    cld
    mov rdi, rsp
    call x86_syscall_handler
    jmp isr_restore

# RUN THE HANDLER ON THIS CPU'S INTERRUPT STACK (milestone 124, brought to this architecture by
# milestone 161's roadmap item 4).
#
#   rdi = &mut TrapFrame     rsi = the stack to run on, or 0 to stay
#
# The twin of `dispatch_on_interrupt_stack` in the other two ports' trap assembly, and the same
# contract: the frame stays where the stub built it, because a preempted thread's frame must survive
# until that thread runs again and a per-CPU stack cannot promise that; everything above it moves.
# Rust decides whether to switch, in `interrupt_stack::top_for_trap`; this only moves `rsp`.
#
# `rbp` holds the interrupted `rsp` across the call because it is callee-saved, so the handler cannot
# clobber it and it needs no slot on either stack. `x86_trap_body` returns its bool in `al`, which
# nothing here touches.
.global dispatch_on_interrupt_stack
dispatch_on_interrupt_stack:
    push rbp
    mov rbp, rsp
    test rsi, rsi
    jz 4f                           # 0: stay here (from ring 3, pre-init, or nesting)
    mov rsp, rsi
4:  call x86_trap_body
    mov rsp, rbp                    # back to the interrupted stack BEFORE anything can switch away
    pop rbp
    ret

# ---------------------------------------------------------------------------------------------
# The first entry to ring 3.
# ---------------------------------------------------------------------------------------------

# void user_return(TrapFrame *frame) -> !
#
# Load `frame` and `iretq` into it. The arch contract's first-entry-to-user path, reached only
# through `enter_user` in exceptions.rs, which is `#[inline(always)]` for the reason its RISC-V twin
# is: the frame sits at the top of this same kernel stack, so a call frame pushed here could land on
# top of it.
#
# `cli` first, for the exit-window reason in this file's header: this is the one path into
# `isr_restore` that ordinary kernel code jumps to with interrupts possibly enabled.
.global user_return
user_return:
    cli
    mov rsp, rdi
    jmp isr_restore

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
