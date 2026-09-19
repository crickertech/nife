# Leaving UEFI on riscv64: the last thing the loader does, after ExitBootServices.
#
# THE CONTRACT (the standard RISC-V calling convention, `extern "C"`):
#
#   a0 = the kernel's entry point, physical.
#   a1 = this hart's id.
#   a2 = the device tree the kernel is to read, physical.
#
# It does not return. WHAT IT LEAVES is the state OpenSBI's jump to an S-mode payload produces and
# kernel/src/arch/riscv64/boot.s expects: S-mode, supervisor interrupts off, paging off
# (`satp` = 0), a0 = hart id, a1 = device tree.
#
# WHY TURNING PAGING OFF HERE IS SAFE: UEFI runs with an identity map when it runs with one at all
# (UEFI 2.10, 2.3.7), so the instruction after `csrw satp` is fetched from the same physical address
# either way. Nothing below touches memory.

.text
.global riscv64_leave_uefi
riscv64_leave_uefi:
    csrci   sstatus, 2              # SIE off: no interrupt may arrive at firmware's vectors now
    csrw    sie, zero
    csrw    satp, zero              # paging off
    sfence.vma
    # `fence.i`, so the hart fetches the kernel the loader just copied rather than anything an
    # instruction cache held for those addresses. Emitted as its encoding because it belongs to the
    # Zifencei extension, which the `riscv64imac` assembler does not enable by name.
    .word   0x0000100f
    mv      t0, a0
    mv      a0, a1
    mv      a1, a2
    jr      t0
