# 239. radon's device tree does not describe the TRNG, so a working driver never runs

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from the first bench session that ran milestone
159's (a real hardware entropy source: the JH7110's TRNG) driver on the board it was written for.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The diagnosis needs two commands at a U-Boot prompt, and the fix is a boot-path
change rather than hardware.

**In brief.** radon booted nife hands-free on 2026-09-03 (milestone 218, every boot of the
VisionFive 2 needs a human typing four commands into U-Boot, confirmed on the board that day) and
the tour reported:

```
hw entropy  : skipped (this machine's tree describes no starfive,jh7110-trng; QEMU virt has none)
```

**The silicon has a TRNG and the device tree does not mention it.** Milestone 159's driver looks for
`compatible = "starfive,jh7110-trng"` at `reg = <0x1600C000 0x4000>`, transcribed from Linux's own
binding (`Documentation/devicetree/bindings/rng/starfive,jh7110-trng.yaml`) and its driver
(`drivers/char/hw_random/jh7110-trng.c`). The node is in Linux's `jh7110.dtsi`. It is apparently not
in the tree radon actually hands us.

**So this is not milestone 159's driver and not milestone 220** (this kernel drives no clock or
reset controller, and the first real device will need one). 159's block anticipated exactly this as
one of its five possible outcomes, and the outcome it names routes here rather than to either of
those.

## Where the tree comes from, which is the whole question

The boot script does `fdt addr ${fdtcontroladdr}` and `fdt move ${fdtcontroladdr} 0x86000000`, so
**nife is handed U-Boot's own control device tree**: a vendor U-Boot dated February 2023, describing
what U-Boot needs rather than what the SoC has. That was the right choice when milestone 16a made it,
and `notes/visionfive2.md` records why: the control DTB describes the board correctly, and the
alternative addresses landed outside the kernel's boot page table.

**Nobody has read what is actually in it.** Two commands at the prompt settle it, and milestone 159's
block already asks for them:

```
StarFive # fdt addr ${fdtcontroladdr}
StarFive # fdt print /soc/rng@1600c000
```

## The shapes a fix could take, none chosen

- **Load a fuller DTB from the card** and pass that instead of `${fdtcontroladdr}`. A `board.dtb`
  has been sitting on radon's card since 2026-09-01 and nobody has checked what it contains; it was
  deliberately left there when the stale test images were removed, precisely because it might be
  this.
- **Carry a minimal overlay** adding the node Linux's `jh7110.dtsi` already has. Smaller, and it
  makes this project the author of a hardware description, which is a different kind of claim.
- **Accept the vendor tree's limits** and record that the TRNG is out of reach until the boot path
  supplies a fuller description. Honest, and it leaves fatal risk 6 (a capability-confined userspace
  driver cannot drive real hardware at real speed) without the device milestone 159 was built to
  give it.

## Why it matters beyond entropy

**A device tree that describes less than the chip has is a limit on every future driver**, not just
this one. The same question will be asked by the GMAC (milestone 53), by the PCIe root complex
(milestone 163), and by anything else that needs a node to be discovered. Answering it once, with a
recorded reason, is worth more than answering it for the TRNG.

**And it already changed a second number.** The same boot reported `capability slots: 4 of 24 at
peak`, against 21 in QEMU. That is not a different measurement, it is the same finding: milestone 230
(`script/shell-check` is red on `main`, on both architectures, and nothing says so) established that
`have_login_stack` requires an entropy client, so no TRNG node means no entropy, which means no login
stack, which means a much smaller peak. **The 24-slot ceiling was sized against a QEMU boot richer
than the real board's.**

## BUGS

- **The premise is one line of tour output**, not a device-tree dump. It is consistent with the
  driver's discovery logic and with `capability slots: 4 of 24`, and it has not been confirmed by
  reading the tree.
- **This block assumes the node's absence is the vendor U-Boot's doing.** It could equally be a
  build-time choice in that firmware, in which case the fix might be a firmware update rather than a
  tree we supply, and that is a different piece of work.
- **Nothing here says what other nodes are missing.** The TRNG is the one a driver happened to look
  for. Whatever else the vendor tree omits is unknown and unexamined.
