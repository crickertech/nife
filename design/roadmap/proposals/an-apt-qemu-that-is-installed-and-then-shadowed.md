# An apt QEMU that is installed and then shadowed

**Status: PROPOSED 2026-09-13.** Found by milestone 287 while making `script/bootstrap` finish on a
stock Linux box. Not taken there because the evidence stops short of the claim.

**Gate: NONE.** Nothing here needs calef. It needs four gate runs on a Linux box with the packages
absent, listed below, and a lane can do all four.

## In brief

On a cold Linux clone, `script/bootstrap` installs five apt packages:

```
sudo apt-get install -y qemu-system-arm qemu-system-misc qemu-system-x86 ipxe-qemu ovmf
```

and then, minutes later on the same run, builds QEMU 11.0.2 from source because no Ubuntu release
ships one with `riscv-iommu-pci`. From that moment `scripts/qemu-path.sh` puts the built prefix ahead
of `/usr/bin` on PATH, so **the three emulator packages are shadowed for the life of the checkout**.
They are a few hundred megabytes downloaded to be overridden.

The two firmware packages were the reason to be careful, and they may be redundant too. `ipxe-qemu`
supplies `efi-virtio.rom`, the option ROM `-device virtio-blk-device` loads; `ovmf` supplies
milestone 87's UEFI firmware, and Ubuntu 24.04 spells it `OVMF_CODE_4M.fd`. **Measured on the prefix
QEMU's own `make install` output** (`~/.cache/nife-qemu/share/qemu`, 71 files):

```
efi-virtio.rom          pxe-e1000.rom     pxe-virtio.rom     edk2-x86_64-code.fd
edk2-aarch64-code.fd    edk2-i386-vars.fd edk2-riscv-code.fd (and nine more edk2-*)
```

So QEMU ships both of them itself, into the prefix, and `scripts/qemu-uefi-x86_64.sh` already
searches `<prefix>/share/qemu/edk2-x86_64-code.fd` **first**, ahead of the `/usr/share/OVMF` entries,
for a reason its own header records: CI builds QEMU into a cached prefix, so no absolute path in a
list can ever name the firmware.

## Why milestone 287 did not do it

The prefix *containing* the files is not the same claim as the gates *passing* without the packages.
The x86_64 UEFI boot and `script/netboot-rehearsal` are the two paths that consume this firmware, and
neither was run on a box with `ipxe-qemu` and `ovmf` absent. Removing a package on that evidence
would be the shape milestone 287 exists to correct: a change that looks right and was never run
against the configuration it changes.

There is a second-order effect worth stating, because milestone 287 introduced it. `script/bootstrap`
now sources `scripts/qemu-path.sh` before its `command -v` probes, so on a machine where the prefix
already holds the pinned QEMU **the whole apt branch is skipped**, firmware packages included. A box
that ran `script/ci-qemu` before its first `script/setup` therefore never gets `ovmf` or `ipxe-qemu`
at all. That is believed fine, on the measurement above, and it is currently believed rather than
shown.

## What would close it

1. On a Linux box, `apt-get remove --purge ipxe-qemu ovmf` and confirm nothing else on the machine
   wanted them.
2. `cargo xtask uefi-image && cargo xtask uefi-boot` green, which exercises
   `scripts/qemu-uefi-x86_64.sh`'s firmware search against the prefix alone.
3. `script/netboot-rehearsal` green, which is the other consumer of the option ROMs.
4. `script/test` green on all three ISAs, which is where `-device virtio-blk-device` loads
   `efi-virtio.rom`.

If all four pass, drop the five packages from `script/bootstrap`'s Linux branch and let
`script/ci-qemu` be the only emulator source on Linux, with the bootstrap comment rewritten to say
why (it currently explains at length why the firmware packages are needed on Debian, and that
paragraph is the thing being falsified).

If any fail, the finding is the opposite and worth just as much: name in that comment which file
comes from apt and which from the prefix, so the next person does not have to re-derive it.

## What it is worth

A few hundred megabytes and one less confusing step on a first clone. **Not correctness**: the
current arrangement works, it is merely wasteful and it reads as though apt's QEMU matters when it
does not. Priced accordingly.
