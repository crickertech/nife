# 257. Boot radon over the network, so the microSD card stops being the tax on every experiment

**Status: NOT-STARTED.** Minted 2026-09-04 by calef, the same evening the path was proved by hand at
radon's U-Boot prompt. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The board side is proved and needs nothing built. What remains is a boot script, a
TFTP server on patagonia, and the `xtask` arm that writes the script.

## The card is the bottleneck, and the tree predicted the day this would matter

`notes/visionfive2.md` has carried this since 2026-09-01, under **TFTP alternative (not built)**:

> the cordoba side (dnsmasq or tftpd serving the image) is its own small piece of work and turns the
> flash-a-card loop into a rebuild-and-reset loop. **Worth building the moment the card loop gets
> annoying, which history says is the second bench session.**

The 2026-09-04 E3 session wrote the card **six times**, once per boot, each a walk to the board and
back, because E3 is a comparison of two builds and the interleaving that makes it valid requires
alternating them. That session also produced the other half of the argument: the numbers it took
were good, so the cost was entirely in handling rather than in anything the experiment needed.

**One correction the note carries and this block does not inherit: the server belongs on patagonia,
not cordoba.** radon's UART goes into patagonia (corrected by calef 2026-09-04; the rig note said
cordoba and was wrong), and patagonia is where the images are built. Serving from patagonia means
there is no copy step at all: build, power cycle, watch, on one machine.

## What was proved by hand, 2026-09-04, and what it cost

Driven from patagonia over the serial line, with the board sitting at its `StarFive #` prompt.

**The vendor firmware has a network stack**, which was the one real unknown and was checked rather
than assumed:

```
U-Boot 2021.10 (Feb 12 2023 - 18:15:33 +0800), Build: jenkins-VF2_515_Branch_SDK_Release-24
dhcp     - boot image via network using DHCP/TFTP protocol
tftpboot - boot image via network using TFTP protocol
ethernet@16030000 Waiting for PHY auto negotiation to complete...... done
DHCP client bound to address 192.168.8.200 (3378 ms)
```

**Then nife booted, entirely over the wire, with the card ignored.** The full bench suite ran to
`bench: done`:

```
tftpboot ${kernel_addr_r} nife-vf2.img        282,624 bytes,   1.4 s
tftpboot 0x90000000 nife-initrd.img         9,044,480 bytes,  20.6 s  (428 KiB/s)
setenv nife_archive_size ${filesize}
fdt addr ${fdtcontroladdr}
fdt move ${fdtcontroladdr} 0x86000000
booti ${kernel_addr_r} 0x90000000:${nife_archive_size} 0x86000000
```

Only the two `load` lines of `target/board/boot.cmd` changed. `fdt move` and `booti` are byte for
byte what milestone 218's script already emits, which is why this is a small change rather than a
new boot path.

### It also produced a control nobody asked for, and it is the reason to trust the workflow

The image served was the **padded** E3 build, so that boot is a fourth reading of the padded
condition, taken through a completely different load path: DHCP, ARP, and about 6,200 TFTP round
trips instead of a FAT read.

| row | card-booted, 3 boots | TFTP-booted |
|---|---|---|
| `ipc_rtt` | 4311 · 4310 · 4310 | **4311** |
| `call_reply` | 5088 · 5089 · 5088 | **5089** |
| `ipc_rtt_el0` | 124958 · 124391 · 124903 | **124917** |

All three land inside the card-booted cluster. **How the kernel arrives does not perturb what it
measures**, which is the one thing that could have made this workflow useless for the bench work it
exists to serve, and it is now checked rather than assumed.

## What to build

1. **A `--tftp` mode for `cargo xtask board-script`**, emitting `dhcp` and two `tftpboot` lines in
   place of the two `load` lines. `script/board-image` grows the matching flag.
2. **A fallback to the card in the same script.** U-Boot can branch on command status, so the script
   should try the network and fall back to `load` from the card's own payload. This is what lets the
   card be written once and left in radon forever: no cable, no hub, no DHCP, and the board still
   boots something. **A card that can be bricked by an unplugged cable is a worse rig than the one
   being replaced.**
3. **A TFTP server on patagonia.** A read-only server with `blksize` (RFC 2348) is about a hundred
   lines and was written for the 2026-09-04 session; whether that becomes a `script/` entry point or
   a Homebrew `dnsmasq` in TFTP-only mode is the lane's call. Note that port 69 bound without root
   on this machine, so neither needs `sudo`.
4. **A recorded address expectation.** `serverip` is patagonia's, `192.168.8.216` on 2026-09-04, and
   a DHCP lease can move it. Whatever the script does about that, it should say so where a reader
   meets it rather than leaving a stale constant to be discovered at the bench. This is the same
   defect class milestone 256 is about, one layer out.

## The measure

**A bench session that never touches the card.** Concretely: write the card once with the new
script, then take two boots of different builds with no physical access between them.

## What this does not do

**It is not remote power.** A hung board still needs a person, because radon's software reset path is
closed: `notes/soak.md` records SBI SRST resetting the SoC and U-Boot SPL then failing to re-init the
PMIC, so the board does not come back. **calef accepted manual power on 2026-09-04**; milestone 224
holds that decision and the options for revisiting it.

**And it is not unattended CI.** Three things stand between here and that, and this is one:
network boot (this block), an on-board test-suite exit so a machine rather than a person reads the
result (milestone 16's remaining piece), and remote power (224, accepted as manual). Two of the
three are lanes; the third is a decision that has been made the other way, on purpose.

## BUGS

- **428 KiB/s is slow**, and the 9 MB archive is 20 seconds of it. That is fine against a two-minute
  walk and it is worse than a card read, so a session that boots many times pays it many times.
  Nobody has looked at whether U-Boot's `blksize` or the TFTP window is the limit.
- **Everything here was proved on one evening, on one board, with one image.** The fallback path in
  item 2 has never been exercised at all, because the card was ignored rather than tested.
- **The card still has to be written once**, and a change to the boot script means writing it again.
  The loop is shorter, not gone.
- **`ethernet@16030000` is one of radon's two ports**, and it is the one that was plugged in. Nothing
  here says what happens on the other, or whether U-Boot would find it.
