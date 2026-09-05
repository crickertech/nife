# 260. Boot xenon over the network, because the stick is the tax and the router is ours

**Status: NOT-STARTED.** Minted 2026-09-05 by calef, in one sentence: *"xenon already hurts. We can
control our router since it runs OpenWRT."* *(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** Two firmware settings are calef's, named below with the photographs that show
them. Everything else is a lane's, and none of it needs the machine.

## Why this is not the same problem radon had, and why the answer just changed

Milestone 257 needed **no cooperation from the network at all.** U-Boot takes the server address
from a script we write onto the card, so `192.168.8.216` is baked in and the house router never has
to know anything.

**PXE is the opposite.** The DHCP server has to advertise the next server and the boot filename, and
that is the router. When 257's lane refused `dnsmasq`, it did so for exactly this reason, and its
own words are worth keeping because they are what made this look blocked: *"a second DHCP server on
it is an outage for everyone in the building."*

**That objection does not apply here and the difference is the whole reason this is startable.**
This is not a second DHCP server; it is one option on the existing one. OpenWRT's DHCP *is*
`dnsmasq`, and its boot options can be **scoped to a single MAC address**, so xenon gets an offer
and nothing else on the house network sees any change.

## What makes it easier than radon, which is the opposite of what anyone would guess

**xenon's payload is one file.** `cargo xtask uefi-image` stages the loader, the kernel and the
archive into a single `BOOTX64.EFI`, **9,180,160 bytes** as of 2026-09-05. So a netboot is one TFTP
transfer, where radon needs two and a boot script to sequence them. There is no fallback branch to
write, because PXE either produces the file or the firmware moves to the next boot entry on its own.

At the 400 KiB/s `board-netboot` measured against radon, that is roughly **23 seconds**.

**And `script/board-netboot` is already board-agnostic.** It serves a directory over TFTP and
knows nothing about RISC-V; milestone 257 built it, and the ratified name says netboot rather than a
board or a protocol precisely so it could serve this too.

## The two firmware settings, which are calef's

Read off `notes/xenon-firmware.md`, which transcribed 70 photographs of this machine's setup UI on
2026-09-05 so that a lane would not have to walk to it. **Both are on one page:**

> **IMG_4031, Integrated NIC.** `Enable UEFI Network Stack` **unticked**. Disabled ( );
> **Enabled (•)**; Enabled w/PXE ( ). So the LAN is visible to an OS but there is no UEFI PXE path.

So two changes: **tick `Enable UEFI Network Stack`**, and move Integrated NIC from `Enabled` to
**`Enabled w/PXE`**. They are calef's for the reason that note already gives about firmware
generally, that a setting changes this machine's behaviour for everything else it is used for.

Nothing else in the transcription is in the way. Boot List Option is already **UEFI**, Secure Boot
is **Disabled**, legacy option ROMs are **unticked**, and `UEFI Boot Path Security` has no effect
while no admin password is set, and none is.

## What a lane can do without the machine or the settings

- **Serve the ESP.** `board-netboot` takes a directory; the question is whether it serves
  `target/esp` as-is or a directory holding just the one file, and what path the DHCP option names.
- **The `dnsmasq` configuration, written down rather than typed into a router once.** A
  `dhcp-host` line tagging xenon by MAC and a tagged `dhcp-boot` line, in the tree, where a reader
  meets it. **This tree's habit is that a fact living only in one machine's configuration is the
  rung-four failure AGENTS.md names**, and a router someone reflashes takes it with them.
- **The architecture question.** DHCP option 93 tells the server what the client is, and a
  configuration that answers the same way to every client will hand a BIOS machine a UEFI binary.
  Scoping by MAC sidesteps it here and the lane should say so rather than leave it implied.
- **Everything testable under QEMU.** `scripts/qemu-uefi-x86_64.sh` already boots the image under
  real OVMF firmware, and OVMF can PXE boot, so the whole path can be exercised on patagonia before
  anyone touches the router.

## The proof that this milestone worked

**xenon boots nife with no removable media in it**, photographed, since it still has no serial
console this project can read. Anything short of that is a rehearsal, and the rehearsal under OVMF
is worth doing first because the bench session costs calef's evening.

## What this does not fix, and it is the thing that will bite next

**xenon halts at POST without a keyboard.** `notes/xenon-firmware.md` records `Warnings and Errors`
set to `Prompt on Warnings and Errors`, `Enable Keyboard Error Detection` ticked, and **eight
`Alert! Keyboard not found` entries in the machine's own event log** across a year. A netboot rig
whose point is an unattended power cycle runs straight into that, and the two settings that would
change it are calef's for the same reason as the two above. **Network boot without that is a faster
bench session, not an unattended one.**

## BUGS

- **Nobody has recorded xenon's MAC address**, and the whole `dhcp-host` approach is keyed on it.
  It is one look at the router's lease table and it is not written down anywhere in this tree.
- **`board-netboot` has never served a 9 MB single file to a UEFI client.** It served radon two
  files and its own tests used 373 bytes and 2 MB. UEFI PXE clients are not U-Boot and may negotiate
  a different `blksize` or none.
- **This block assumes OpenWRT's `dnsmasq` exposes per-host boot options**, which is true of
  `dnsmasq` and has not been checked against this router's OpenWRT version or its UI.
- **No fallback, by design, and that is a real difference from milestone 257.** radon's card can
  always boot something; a xenon that fails PXE falls to the next boot entry, and the entries on that
  list are `Windows Boot Manager` and whatever removable media is present. That is somebody's
  installed Windows, which argues for leaving a stick in the machine rather than removing it.
