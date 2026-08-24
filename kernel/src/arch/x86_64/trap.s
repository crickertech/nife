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
    call x86_trap_handler

    # Restore a TrapFrame at rsp and return from the trap. Shared by the IDT path above, by the
    # `syscall` path below, and by the two first-entry-to-ring-3 paths after that, which is what
    # keeps "how a frame becomes running registers" a single piece of code with a single swapgs
    # rule. The RISC-V twin of this label is `trap_return`.
isr_restore:
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

# ---------------------------------------------------------------------------------------------
# The first entry to ring 3. Two doors into `isr_restore`, because a user program is entered from
# two different places and only one of them ever comes back.
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

# u64 x86_enter_user_and_wait(TrapFrame *frame, u64 *resume_slot)
#
# **Bring-up scaffolding, and named so it reads as scaffolding** (provisional; milestone 161). Enter
# ring 3 like `user_return`, but leave this caller a way back: park the six callee-saved registers
# and this call's own return address on the current stack, record that block's address in
# `*resume_slot`, and enter. `x86_leave_user` below resumes it, and this function then returns
# normally with whatever value that call passed.
#
# It is `switch_to`'s two halves with a ring change in the middle, which is not an accident: what a
# scheduler does with two threads is what this does with one, and when the scheduler comes up on
# this architecture (roadmap item 4) this pair is what it replaces. Until then it is the only way a
# boot tour can run a ring-3 program and still print what happened.
.global x86_enter_user_and_wait
x86_enter_user_and_wait:
    cli
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov [rsi], rsp                  # *resume_slot = the saved block, with our return address on top
    mov rsp, rdi
    jmp isr_restore

# void x86_leave_user(u64 resume_rsp, u64 value) -> !
#
# Abandon the kernel stack we are on and resume whoever called `x86_enter_user_and_wait`, handing
# back `value` as its return value. The pop order is the mirror of the push order above, and the
# `ret` lands on the return address the original `call` left underneath them.
.global x86_leave_user
x86_leave_user:
    mov rax, rsi
    mov rsp, rdi
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret

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
