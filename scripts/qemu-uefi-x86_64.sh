#!/bin/sh
#
# Name: ratified 2026-08-30 (calef, in session, on milestone 87's lane report). Hyphens because
# shell commands are hyphenated everywhere, per AGENTS.md's per-domain naming table, and the ISA
# suffix matches its sibling scripts/qemu-runner-x86_64.sh. NOTE: script/names scans script/ and
# not scripts/, so nothing reads this block today; it is written where a reader meets the thing.
#
# Boot the x86_64 kernel under REAL FIRMWARE (milestone 87): OVMF, the open-source UEFI
# implementation that ships with QEMU, loading `uefi_loader` from a FAT filesystem exactly the way
# the Dell OptiPlex 7050's firmware loads it from a USB stick.
#
# WHY THIS IS NOT scripts/qemu-runner-x86_64.sh WITH A FLAG. That runner is a cargo `runner`: cargo
# appends a kernel ELF and QEMU's PVH loader reads it with `-kernel`. This path has no `-kernel` at
# all. The firmware finds `\EFI\BOOT\BOOTX64.EFI` on a disk, starts it, and the loader places the
# kernel itself. Nothing is shared but the machine model, and merging them would mean a runner that
# ignores the one argument cargo exists to pass it.
#
#   scripts/qemu-uefi-x86_64.sh <esp-directory> [extra qemu args...]
#
# `cargo xtask uefi-image` populates the directory; see notes/x86-uefi-boot.md for the whole picture
# and for the bench procedure on the real machine.
#
# THE FAT FILESYSTEM IS QEMU'S OWN, and that is what makes this need no new host tooling. `file=fat:`
# is QEMU's vvfat block driver: it synthesises a FAT filesystem out of a host DIRECTORY, live, so
# there is no image to build and no `mtools`/`mformat` to install (neither is available on this
# machine, and `grub-mkrescue` is not installable at all here; see notes/x86-uefi-boot.md's fork).
# `fat:rw:` rather than read-only because OVMF writes its own boot-order bookkeeping.
#
# BOUNDED, ALWAYS. The kernel halts with `hlt` rather than exiting, so QEMU never exits on its own.
# `timeout(1)` does not exist on macOS and `perl -e 'alarm N'` does not work on QEMU (it installs its
# own SIGALRM handler and swallows it, which once leaked eleven emulators). NIFE_UEFI_TIMEOUT moves
# the bound; it is a bound rather than a choice.

set -e
cd "$(dirname "$0")/.."

ESP="$1"
if [ -z "$ESP" ] || [ ! -d "$ESP" ]; then
    echo "qemu-uefi-x86_64: usage: $0 <esp-directory> [qemu args...]" >&2
    echo "                 run \`cargo xtask uefi-image\` to populate one" >&2
    exit 1
fi
shift

# OVMF, wherever this machine keeps it. On macOS it ships WITH QEMU rather than beside it, which is
# the fact that decided milestone 87's fork: nothing had to be installed for this to work.
# **That is a Homebrew fact and not a universal one**, learned when this gate first ran in CI:
# Debian packages the firmware separately, so `script/bootstrap` installs `ovmf` on Linux, and
# Ubuntu 24.04 names the file `OVMF_CODE_4M.fd` rather than `OVMF_CODE.fd`, which is why both
# spellings are searched. NIFE_OVMF_CODE names it explicitly on a machine that puts it elsewhere.
#
# **The variable store is `edk2-i386-vars.fd` even for x86_64**, which is QEMU's own naming and not
# a mistake: the vars template is architecture-shared across the i386/x86_64 pair. Same prefix-first
# search as the code image below, for the same reason.
#
# **Ask QEMU where it lives, first.** The header above says the firmware ships WITH QEMU, and that is
# the fact to act on rather than a list of places QEMU has been seen. CI builds QEMU from source into
# a cached prefix (`script/ci-qemu`, `$HOME/.cache/nife-qemu`), so no absolute path in a list can
# ever name it, and the hardcoded list is exactly why this gate failed on its first CI run while
# passing on every developer machine. `<prefix>/share/qemu/edk2-x86_64-code.fd` is where QEMU's own
# build puts it, which is also what makes the Homebrew entry below work.
qemu_bin="$(command -v qemu-system-x86_64 2>/dev/null || true)"
qemu_prefix=""
[ -n "$qemu_bin" ] && qemu_prefix="$(dirname "$(dirname "$qemu_bin")")"
if [ -z "$NIFE_OVMF_CODE" ]; then
    if [ -n "$qemu_prefix" ]; then
        for candidate in \
            "$qemu_prefix/share/qemu/edk2-x86_64-code.fd" \
            "$qemu_prefix/share/edk2-x86_64-code.fd"
        do
            [ -f "$candidate" ] && NIFE_OVMF_CODE="$candidate" && break
        done
    fi
fi
if [ -z "$NIFE_OVMF_CODE" ]; then
    for candidate in \
        /opt/homebrew/share/qemu/edk2-x86_64-code.fd \
        /usr/local/share/qemu/edk2-x86_64-code.fd \
        /usr/share/OVMF/OVMF_CODE_4M.fd \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/edk2/x64/OVMF_CODE.fd
    do
        [ -f "$candidate" ] && NIFE_OVMF_CODE="$candidate" && break
    done
fi
if [ ! -f "$NIFE_OVMF_CODE" ]; then
    echo "qemu-uefi-x86_64: no OVMF firmware found. Set NIFE_OVMF_CODE to an edk2 x86_64 CODE image." >&2
    exit 1
fi

# The variable store has to be WRITABLE (the firmware records its boot order in it), so the pristine
# one is copied rather than used in place. It is `edk2-i386-vars.fd` even for an x86_64 build,
# because upstream edk2 builds one variable store for both and Homebrew keeps that name.
VARS="target/ovmf-vars.fd"
if [ ! -f "$VARS" ]; then
    for candidate in \
        "$NIFE_OVMF_VARS" \
        "$qemu_prefix/share/qemu/edk2-i386-vars.fd" \
        "$qemu_prefix/share/edk2-i386-vars.fd" \
        /opt/homebrew/share/qemu/edk2-i386-vars.fd \
        /usr/local/share/qemu/edk2-i386-vars.fd \
        /usr/share/OVMF/OVMF_VARS.fd \
        /usr/share/edk2/x64/OVMF_VARS.fd
    do
        [ -n "$candidate" ] && [ -f "$candidate" ] && cp "$candidate" "$VARS" && break
    done
fi
if [ ! -f "$VARS" ]; then
    echo "qemu-uefi-x86_64: no OVMF variable store found. Set NIFE_OVMF_VARS." >&2
    exit 1
fi

# `q35`, `-cpu max`, `-m 256M` and `isa-debug-exit` are deliberately the same as
# scripts/qemu-runner-x86_64.sh's, so a difference between the two boots is the FIRMWARE and not the
# machine. One core, because AP bring-up under UEFI has never been exercised (src/main.rs's BUGS).
SMP="${NIFE_SMP:-1}"
CPU="${NIFE_CPU:-max}"
TIMEOUT="${NIFE_UEFI_TIMEOUT:-90}"

exec scripts/qemu-bounded.sh "$TIMEOUT" qemu-system-x86_64 \
    -machine q35 \
    -cpu "$CPU" \
    -smp "$SMP" \
    -m 256M \
    -display none \
    -serial stdio \
    -no-reboot \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -drive "if=pflash,format=raw,unit=0,readonly=on,file=$NIFE_OVMF_CODE" \
    -drive "if=pflash,format=raw,unit=1,file=$VARS" \
    -drive "format=raw,file=fat:rw:$ESP" \
    "$@"
