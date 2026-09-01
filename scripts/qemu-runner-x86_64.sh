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
# WHAT IS NOT HERE YET, and it is still most of what the other two runners do: no NIC, no GPU, no
# RNG. NVMe is wired (below, decisions §86's x86_64/VT-d data point), because the kernel-resident
# NVMe driver is arch-neutral and VT-d confinement landed this session; ONE virtio-blk is wired
# (milestone 164, below); the rest are wired one at a time as the port reaches them, and adding a
# device to this file before the kernel can drive it only produces a boot that looks richer than it
# is. See design/roadmap/161-x86-64-kernel-port.md.
#
# The kernel halts with `hlt` (arch::halt), so QEMU does not exit on its own. Bound any interactive
# run with scripts/qemu-bounded.sh (see CLAUDE.md, "Never leave QEMU running").

set -e

ELF="$1"
shift

# One by default, NOT four like the other two runners. SMP bring-up (INIT-SIPI-SIPI through the
# local APIC, milestone 161's SMP item) is built and NIFE_SMP moves this the same way it does on
# the other two runners, but it is not the default here, and the reason changed on 2026-08-25
# without the number changing.
#
# The crash that first held it at 1 (a fault partway through ordinary thread reaping, at RIP 0 or
# at `stack::PAINT`) is FIXED: it was a missing cross-core TLB shootdown, since `invlpg` is local
# to one CPU and this port had no remote half. See `arch::x86_64::mmu::shoot_down_others`.
#
# Two other failures are still open and either can fail a two-core run, so the default stays where
# it was: the AP-bring-up flakiness at three or more cores, and a boot-core-identity bug that makes
# `smp::tests::every_secondary_runs_scheduled_work` fail about half the time at two. Both are
# recorded, with what is known about each, in `arch::x86_64::ap_boot`'s own BUGS section (#1 and
# #3), which is the authoritative account; see also design/roadmap/161-x86-64-kernel-port.md item 5.
SMP="${NIFE_SMP:-1}"

# `q35` rather than the older `pc` because it is what the physical target looks like: a PCIe root
# complex with an ECAM window, an AHCI controller, and the legacy 16550 COM1 at port 0x3f8 that
# milestone 87's Dell C4PDJ module also presents. One machine model, both paths.
#
# `-cpu max` gives us every feature QEMU models. It is the permissive choice, and the same caveat
# applies as on the RISC-V side (notes/cpu-models.md): a kernel that has only ever run here has
# never been told no. NIFE_CPU narrows it; `qemu64` is the conservative baseline.
CPU="${NIFE_CPU:-max}"

# The userspace archive rides in as an initrd (milestone 161), the same knob both other runners
# take. The mechanism underneath differs and is worth one line: there is no device tree here, so
# QEMU's PVH loader puts the file in RAM and describes it in the `hvm_start_info` module list,
# which arch::x86_64::machine::initrd reads. Unset, the kernel finds no module and says so.
#
# A SET NIFE_INITRD naming a missing file is an error rather than a silent no-op, matching the
# check both other runners make about NIFE_DISK: `cargo xtask test` exports this variable
# unconditionally, so a typo or a stale path would otherwise boot a kernel with no userspace and
# report thirty test failures that name the tests rather than the cause.
INITRD=""
if [ -n "$NIFE_INITRD" ]; then
    if [ ! -f "$NIFE_INITRD" ]; then
        echo "qemu-runner-x86_64: NIFE_INITRD=$NIFE_INITRD does not exist (run cargo xtask initrd-x86)" >&2
        exit 1
    fi
    INITRD="-initrd $NIFE_INITRD"
fi

# isa-debug-exit is x86's answer to the semihosting exit the other two use: a write to this port
# terminates QEMU with status (value << 1) | 1, so the guest can report pass/fail to the harness.
# Always present, because a test build that cannot exit leaves an emulator running forever.
# See kernel/src/arch/x86_64/semihosting.rs.
DEBUG_EXIT="-device isa-debug-exit,iobase=0xf4,iosize=0x04"

# VT-d (milestone 161, roadmap item 6), unconditional, the same posture the other two runners take
# for their own IOMMU (`iommu=smmuv3` on aarch64, `-device riscv-iommu-pci` on riscv): idle when
# nothing attaches. `intel-iommu` is a q35-only device (it attaches to the host bridge, not to a
# PCI slot), which is one more reason this runner and the aarch64/riscv ones cannot share a code
# path. What proves the driver against real (emulated) hardware rather than only its own
# host-side unit tests is `arch::x86_64::machine::read_acpi` finding a DMAR with one DRHD, so
# `kernel_main`'s x86 tour brings VT-d up and prints so; the NVMe attachment below (§86's data
# point) is the first PCI device this runner confines behind it.
IOMMU="-device intel-iommu"

# An NVMe controller when NIFE_NVME names an image (milestone 53's storage half; decisions §86's
# x86_64/VT-d data point), the twin of the aarch64 and riscv64 runners' blocks. No
# `iommu_platform` flag, same reason as the other two: that knob is virtio's opt-in, and a real
# PCI device model's DMA always goes through the PCI address space, so with `-device intel-iommu`
# on the machine the controller sits behind VT-d with no flag to forget, and the kernel must
# confine its requester id before the controller can fetch a single command
# (kernel/src/nvme.rs). serial= is mandatory (QEMU refuses the device without one). A set
# NIFE_NVME naming a missing file is an error, the same NIFE_INITRD lesson above: a silently
# absent controller would read as a machine fact when it is a build-order mistake.
NVME=""
if [ -n "$NIFE_NVME" ]; then
    if [ ! -f "$NIFE_NVME" ]; then
        echo "qemu-runner-x86_64: NIFE_NVME=$NIFE_NVME does not exist (xtask's mknvmedisk writes it)" >&2
        exit 1
    fi
    NVME="-drive file=$NIFE_NVME,if=none,format=raw,id=nvme0 -device nvme,serial=nife-nvme,drive=nvme0"
fi

# **The RedoxFS disk, over PCIe** (milestone 164). One virtio-blk and no more, and the count is the
# contract rather than an accident: `q35` has no virtio-mmio bus at all (`VIRTIO_SLOTS` is 0 in
# `arch/x86_64/mmu.rs`), so the FS service cannot ask for "the second mmio disk" the way it does on
# the other two runners and instead takes the first virtio-blk PCI function it enumerates. Attach a
# second one here and the FS server would mount whichever QEMU happened to put first. The matching
# half is `kernel::user::fs_service::block_device`, and its comment names this file.
#
# The image path is derived from NIFE_DISK the same way both other runners derive theirs, so one
# variable still names the whole fixture set and xtask's `mkredoxfs` remains its only writer. The
# nifefs disk NIFE_DISK itself names is deliberately NOT attached, for the paragraph above.
#
# `disable-legacy=on` for the reason the riscv runner gives at its own virtio-blk-pci: the modern
# device (0x1042) rather than the transitional one (0x1001), whose legacy register layout this tree
# does not drive. `iommu_platform=on` because `-device intel-iommu` is on this machine
# unconditionally, so the device must offer VIRTIO_F_ACCESS_PLATFORM for the confined userspace
# driver to negotiate it and for its DMA to survive VT-d translation.
REDOXFS=""
if [ -n "$NIFE_DISK" ]; then
    REDOXFS_DISK="${NIFE_DISK%.img}-redoxfs.img"
    if [ ! -f "$REDOXFS_DISK" ]; then
        echo "qemu-runner-x86_64: $REDOXFS_DISK does not exist (xtask's mkredoxfs writes it)" >&2
        exit 1
    fi
    REDOXFS="-drive file=$REDOXFS_DISK,if=none,format=raw,id=hd0 -device virtio-blk-pci,drive=hd0,disable-legacy=on,iommu_platform=on"
fi

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
    $IOMMU \
    $NVME \
    $REDOXFS \
    -kernel "$ELF" \
    $INITRD \
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
