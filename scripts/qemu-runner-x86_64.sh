#!/bin/sh
#
# The x86_64 QEMU runner (milestone 161). Cargo invokes this for `cargo run` and `cargo test` on the
# x86_64 target, appending the path to the ELF it just built.
#
# The simplest of the three runners, and deliberately so while the port is partial. There is no
# flat-Image objcopy step (that is aarch64's arm64 boot protocol) and no `-bios` handoff (that is
# RISC-V's OpenSBI): QEMU's `q35` reads the PVH note in our ELF, loads the segments at their
# physical addresses and enters the 32-bit trampoline directly. See kernel/src/arch/x86_64/boot.s.
#
# WHAT IS NOT HERE YET, and it is most of what the other two runners do: no initrd, no virtio
# disks, no NIC, no GPU, no RNG, no NVMe, no IOMMU. Those are wired one at a time as the port
# reaches them, and adding a device to this file before the kernel can drive it only produces a
# boot that looks richer than it is. See design/roadmap/161-x86-64-kernel-port.md.
#
# The kernel halts with `hlt` (arch::halt), so QEMU does not exit on its own. Bound any interactive
# run with scripts/qemu-bounded.sh (see CLAUDE.md, "Never leave QEMU running").

set -e

ELF="$1"
shift

# One CPU for now: SMP bring-up on x86 is INIT-SIPI-SIPI through the local APIC, which this port has
# not reached. NIFE_SMP moves it once it has, matching the other two runners' knob.
SMP="${NIFE_SMP:-1}"

# `q35` rather than the older `pc` because it is what the physical target looks like: a PCIe root
# complex with an ECAM window, an AHCI controller, and the legacy 16550 COM1 at port 0x3f8 that
# milestone 87's Dell C4PDJ module also presents. One machine model, both paths.
#
# `-cpu max` gives us every feature QEMU models. It is the permissive choice, and the same caveat
# applies as on the RISC-V side (notes/cpu-models.md): a kernel that has only ever run here has
# never been told no. NIFE_CPU narrows it; `qemu64` is the conservative baseline.
CPU="${NIFE_CPU:-max}"

# isa-debug-exit is x86's answer to the semihosting exit the other two use: a write to this port
# terminates QEMU with status (value << 1) | 1, so the guest can report pass/fail to the harness.
# Always present, because a test build that cannot exit leaves an emulator running forever.
# See kernel/src/arch/x86_64/semihosting.rs.
DEBUG_EXIT="-device isa-debug-exit,iobase=0xf4,iosize=0x04"

# `-no-reboot` turns a triple fault into an exit instead of a silent reset loop, which is the
# difference between seeing that early boot died and watching a blank terminal. Every failure in
# this port's bring-up so far has been a triple fault; add `-d int,cpu_reset` to see the state.
#
# NOT `exec`, and that is the one thing in this file that is not like the other two runners. See the
# status translation below.
set +e
qemu-system-x86_64 \
    -machine q35 \
    -cpu "$CPU" \
    -smp "$SMP" \
    -m 256M \
    -display none \
    -serial stdio \
    -no-reboot \
    $DEBUG_EXIT \
    -kernel "$ELF" \
    "$@"
STATUS=$?

# **isa-debug-exit cannot produce exit status 0.** It terminates QEMU with `(value << 1) | 1`, so
# every status it can report is odd and "the suite passed" has to be some other agreed number. The
# guest writes 1, which lands here as 3; this turns that back into the 0 the harness and every other
# architecture's runner mean by success. The matching half is EXIT_SUCCESS in
# kernel/src/arch/x86_64/semihosting.rs, and the two files name the same number on purpose: getting
# this backwards produces a suite that passes when it fails.
if [ "$STATUS" -eq 3 ]; then
    exit 0
fi
exit "$STATUS"
