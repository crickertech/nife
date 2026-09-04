# 159. A real hardware entropy source: the JH7110's TRNG

**Status: NOT-STARTED.** Minted 2026-08-23, surfaced while investigating milestone 49's boot-wiring
fork (DECISIONS §120): the entropy service (milestone 56, `BUILT`) only has a virtio-rng backend,
which exists in QEMU and not on the VisionFive 2 the tree already boots (milestone 16a). Checked
before minting: the StarFive JH7110's TRNG is already named as the real-hardware candidate in two
places (`notes/entropy.md`, milestone 56's own doc) and tracked by no milestone in either.

**Gate: HARDWARE.** In milestone 53's sense: the board is on the desk and this needs hands on it.
The driver's own logic can be written and host-tested without silicon; whether it actually produces
usable entropy, and at what rate, can only be verified by reading real bits off a real TRNG.

**Status deliberately did not move on 2026-08-24, and again on 2026-09-01, and this section is
both lanes' report of why**, per `design/roadmap/README.md`'s own rule that a branch touching
nothing else in this file must at least say so.

The first lane (2026-08-24) confirmed the TRNG from documentation, wrote and host-tested its
register and DTB-discovery logic (`crates/jh7110_trng`), and wrote an unwired driver program
(`user/src/jh7110_trng.rs`). The second lane (2026-09-01) wired that program to a spawner, gave it
a boot-tour step that says something falsifiable, and moved the one remaining piece of untested
logic into the host-tested crate. **Neither clears this tree's bar.** Milestone 53's `PARTIAL`
names a phase that runs end to end, proven in QEMU; the phase here is "read real bits off a real
TRNG", and there is no JH7110 TRNG in QEMU to run it against, so what runs in QEMU is the
*absence* path. `NOT-STARTED` ("specified, nothing built") is not right either, and it is less
right than it was; it is still the closer of the two available tokens, and the honest gap is
recorded here rather than folded into a status word this vocabulary does not have.

## What the second lane built (2026-09-01), and what it proves

The device is on **radon**, the `StarFive` VisionFive 2 (JH7110, riscv64). radon booted nife with
userspace on 2026-09-01, reaching `init/build`, `device IRQ` and four cores online, so a userspace
process holding a capability runs there; what has never happened is one talking to a real
non-virtio device.

- **A spawner.** `kernel/src/user/entropy_service.rs` grew `Bus::Jh7110`, beside `Mmio`, `Pci` and
  `Instruction`. **The fork the first lane held open turned out not to be one**: it declined to add
  the variant because "adding one needs a real decision about how the service that wires it locates
  this binary in an initrd", and `entropy_service::ensure` already takes the program's bytes from
  its caller, exactly as it does for `entropy`. The caller reads `user::program("jh7110_trng")`.
  Nothing about the interactive boot changed, so DECISIONS §120's stopgap question is untouched.
- **The authority, which is the risk-6 claim.** The driver is granted two rendezvous capabilities
  (a request endpoint it RECVs on, a readiness endpoint it SENDs once) and **one page of device
  memory**: the TRNG's register block, device-typed. No DMA page (the device writes nothing to
  memory), no `Irq` capability (it polls), no `Virtio` capability (there is no transport). The
  binding's `reg` window is `0x4000` and the spawner maps `0x1000` of it, because
  `jh7110_trng::regs` reaches only `0x68`. An earlier draft of the program named a third slot for a
  `DeviceFrame` capability; the spawner that exists grants the page as a `Mapping`, so the slot was
  describing something nobody hands over, and it is gone.
- **A falsifiable boot-tour line.** The riscv64 tour's new `hw entropy` step plays a client over
  the same request endpoint any client would hold, draws 32 bytes twice, and prints the success
  line **only** when the bring-up report says `READY`, both draws come back full, the first is not
  all zeros, and the two differ. Anything else prints `FAILED` with the numbers. The kernel never
  reads a `RAND` register.
- **The skip, which is what CI actually exercises.** QEMU's riscv64 `virt` board has no
  `starfive,jh7110-trng` node, so on every machine this repository boots the step prints
  `hw entropy  : skipped`. `kernel::user::entropy_tests`'
  `the_jh7110_backend_refuses_to_wire_where_there_is_no_jh7110` pins that the wiring *refuses*
  rather than spawning a driver holding a device mapping of an address nobody named.
- **The buffer, moved somewhere testable.** `jh7110_trng::Pool` is the one part of the driver that
  can serve a byte twice, lose the seam between two generations, or pad a short answer with zeros a
  client would mistake for entropy, and none of that is visible in the register decode. It is now
  in the crate with five host tests (no byte served twice across ten generations, a request
  straddling the seam, a dry device shortening the count rather than padding, a served byte zeroed
  behind the cursor, an oversized request clamped). Writing the first of them found `take` shifting
  left by `8 * n` for unclamped `n`: unreachable through the wire format, because
  `entropy_proto::want` clamps to 8 first, but a real overflow at the API. `take` clamps, and a
  `const` assert in the program ties the two 8s together.

**What none of that establishes.** Nothing here has read a bit off a TRNG. Every register offset is
still transcribed from Linux's driver rather than observed, the polling bounds are still guesses
with no board measurement behind them, and the rate question this milestone opens ("a rate and
quality argument, measured rather than assumed") is untouched, because a rate cannot be measured
against a device that has not run.

## The bench procedure, in order

Written to be followed without rederiving anything. It needs radon powered on (the Kasa plug), a
serial console attached before power, and a microSD card. See `notes/visionfive2.md` for the board
facts this leans on and its failure-triage ladder for everything that goes wrong before the tour.

1. On the dev machine, from a checkout with this milestone's work on it:

   ```
   script/board-image
   ```

   It builds the userspace archive first and the kernel second, in that order, because packing the
   archive regenerates the measured-boot manifest the kernel compiles in.

2. **Copy all three files to the card.** The archive is **not** optional, whatever the script's own
   printed instructions say (that wording is milestone 217's, and a stale pair already cost a boot
   with `MEASURED BOOT REFUSED`):

   ```
   target/board/nife-vf2.img          -> /Volumes/NIFE/nife-vf2.img
   target/board/nife-initrd.img       -> /Volumes/NIFE/nife-initrd.img
   target/board/extlinux/extlinux.conf -> /Volumes/NIFE/extlinux/extlinux.conf
   ```

   The kernel and the archive must be **from the same `script/board-image` run**. A mismatched pair
   halts at `MEASURED BOOT REFUSED` before any of this milestone's code runs.

3. Insert the card, DIP switches to QSPI, serial at 115200 8N1, then power. Interrupt U-Boot's
   countdown (the extlinux path does not work; that is milestone 218) and type these five, which
   are what `script/board-image` prints:

   ```
   StarFive # load mmc 1:1 ${kernel_addr_r} /nife-vf2.img
   StarFive # load mmc 1:1 0x90000000 /nife-initrd.img
   StarFive # fdt addr ${fdtcontroladdr}
   StarFive # fdt move ${fdtcontroladdr} 0x86000000
   StarFive # booti ${kernel_addr_r} 0x90000000:${filesize} 0x86000000
   ```

4. **Read the `hw entropy` line, which is the last line of the tour before the banner.** It is one
   of five, and each one means a different thing:

   | Line | What it means |
   |---|---|
   | `hw entropy  : JH7110 TRNG at 0x1600c000 served 32+32 bytes to a client through a capability that names no device; first draw ...., second differs` | **The milestone.** A confined userspace process drove a real non-virtio device and a client got bytes through a capability naming no device. Record the whole line; it is the first such number this project has. |
   | `hw entropy  : skipped (this machine's tree describes no starfive,jh7110-trng; ...)` | The board's own device tree has no TRNG node where mainline's binding says it should be. **Capture the tree** (see step 6) rather than guessing; this is the one fact the first lane flagged as unconfirmed. |
   | `hw entropy  : JH7110 TRNG at 0x..., but no 'jh7110_trng' in the initrd` | The card has a stale archive. Redo step 2 with a matched pair. |
   | `hw entropy  : FAILED: ... bring-up diagnostic 0x0000000000000000 ...` | The register window read as nothing. Most likely the block's clocks are gated or its reset is not deasserted (see the driver's `BUGS`), and next most likely the base address is not the TRNG. |
   | `hw entropy  : FAILED: ... bring-up diagnostic 0x<nonzero> ...` | The device answered and the sequence is wrong. The high 32 bits are `STAT` and the low 32 `ISTAT`; `STAT` bit 9 is `SEEDED`, bits 30/31 are generate/seed in flight, `ISTAT` bit 0 is `RAND_RDY`, bit 1 `SEED_DONE`, bit 4 `LFSR_LOCKUP`. Record the raw word. |

5. If the success line appears, do the two things that make it a measurement rather than an
   anecdote. **Boot twice** and confirm the two first-draw prefixes differ across boots (a device
   reseeded per boot, rather than a constant baked into silicon or a stale register file). And
   **time it**: the tour prints nothing between `pcie` and `hw entropy`, so the wall time between
   those two lines is roughly one bring-up plus two 32-byte draws, which is the first datum for
   this milestone's open rate question.

6. Whatever happened, **capture the board's device tree** while you have it: at the `StarFive #`
   prompt, `fdt addr ${fdtcontroladdr}` then `fdt print /soc/rng@1600c000` (and `fdt list /soc` if
   that finds nothing). That answers the first lane's one unconfirmed fact, and a blob dumped off
   the board is a drop-in fixture for `crates/jh7110_trng`'s existing discovery test rather than a
   new code path.

## Why this is the same shape of gap as 53 and 157

Milestones 53 (network/storage on real silicon) and 157 (display via U-Boot's framebuffer handoff)
both exist because virtio only carries an emulator's paravirtual devices, and real hardware has
none. Entropy is the same story one subsystem over: `entropy_service` (milestone 56) speaks to a
`Virtio` capability today, proven end to end in QEMU, and has never run against real hardware
because nothing here has a driver for the JH7110's TRNG.

## What it needs

- **Confirm the TRNG actually exists and is reachable on this board**, **done from documentation,
  2026-08-24, not from hardware.** The JH7110 datasheet (v1.67) §2.8.2 documents a TRNG module
  ("Ring-oscillator based entropy source... LFSR based digital post process... self re-seeding...
  256-bit random number generation") and Linux carries a shipped, mainline driver
  (`drivers/char/hw_random/jh7110-trng.c`) and device-tree binding
  (`starfive,jh7110-trng`, `reg = <0x1600C000 0x4000>`, `interrupts = <30>`, clocks `hclk`/`ahb`,
  one reset line) for it. `crates/jh7110_trng` transcribes the register layout from the Linux driver
  and proves, on the host, that its DTB-discovery query finds a tree shaped like the binding's own
  example and correctly finds nothing on QEMU's riscv64 `virt` board (which has no such node). What
  remains unconfirmed is whether the VisionFive 2's own shipped device tree actually carries this
  node the way mainline's does; nobody has captured one from the board to check.
- **A driver, not a new protocol.** `entropy_service`'s own contract with its clients does not
  change; this is a new backend behind the existing service, the same relationship milestone 157's
  framebuffer driver has to rung one's existing `gfx_proto` contract. Rule 2 applies: it takes a
  base address and knows nothing else. `user/src/jh7110_trng.rs` is that backend, speaking
  `entropy_proto` unchanged. **Wired as of 2026-09-01**: `entropy_service`'s `Bus` enum has a
  `Jh7110` variant and the riscv64 boot tour spawns it when the machine's device tree describes the
  device. Still never run against one.
- **A rate and quality argument, measured rather than assumed.** Still open, and now sharper rather
  than answered. The datasheet documents exactly one hardware fault signal
  (`ISTAT.LFSR_LOCKUP`, an SEU in the post-processing stage), which is cheap to read and this
  driver reads it; **neither the datasheet nor the Linux driver document anything resembling a NIST
  SP 800-90B health test or a compliance claim (no FIPS, no AIS-31) over the raw bitstream.**
  Whether this tree needs one before trusting these bytes for anything security-shaped is a real
  design question the documentation does not resolve. It is **not decided by this lane** (a
  developer does not edit `design/decisions/`); see `crates/jh7110_trng/src/lib.rs`'s "Health
  testing" section for the full argument, and treat this as a candidate for a PROPOSED entry in
  `design/decisions/` if calef wants the question tracked formally rather than left in this
  paragraph and the crate's own doc.

## The bench ran it, 2026-09-04, and the failure is cleanly attributed

**The confined driver ran on silicon for the first time.** Two boots of radon, byte-identical:

```
hw entropy  : FAILED: JH7110 TRNG at 0x1600c000 (tree says starfive,trng, status disabled):
              report 0x524e475550, bring-up diagnostic 0x0000000000000000,
              draws 0/0 bytes, first-all-zero true, draws-differ false
```

Transcript: `target/board/radon-2026-09-04-trng-bringup.log`.

**Milestone 239's fix works.** `tree says starfive,trng` is the vendor U-Boot spelling, matched by the
second arm 239 added on 2026-09-03. Every boot before that read `skipped`. The device-tree half of
this milestone is done.

**The diagnostic is the all-zero case**, which this block's own outcome table routes to **milestone
220** (this kernel drives no clock or reset controller, and the first real device will need one)
rather than to a defect here: the register window read as nothing, most likely because the block's
clocks are gated or its reset is not deasserted.

**Two independent signals agree**, which is what makes this a diagnosis rather than a guess. The
device tree marks the node `status disabled`, and the register window reads zero. Milestone 239
deliberately *reports* `status` without acting on it, because that same tree lies about the S7 core;
here the tree and the hardware say the same thing.

### What it establishes for fatal risk 6, which is more than "FAILED" suggests

`report 0x524e475550` is **`entropy_proto::READY`**, ASCII `RNGUP`, and that crate's own comment says
a bring-up failure reports `0xDEAD_0000_0000_0000 | step` **instead**. So the service did not report a
failure. It reported ready.

So on real silicon, for the first time: a userspace driver **started** from the archive as an EL0
process, **reached the device through a capability that names no device**, and **completed its
bring-up far enough to send `READY`**. What it could not do is get a non-zero byte out of a block
nothing has powered.

Risk 6 is *"a capability-confined userspace driver cannot drive real hardware at real speed."* This
splits it: **confined** is demonstrated on silicon, **drives real hardware** is blocked on milestone
220, and **at real speed** stays unmeasured and unmeasurable until the block is on.

### A defect in this milestone, found by the same line

**The service sent `READY` while holding 32 bytes of zeros.** `entropy_proto::READY`'s own doc says
it is sent *"once the device is up **and its first bytes are in hand**"*, and two paragraphs later the
same file argues that a caller who cannot be given randomness **must find out**, because the
alternative is *"the exact silent-degradation failure"* it exists to prevent.

A service that reports ready on a dead device is that failure. The tour caught it only because it
prints the draws beside the report and checks `first-all-zero`; a client that trusted `READY` would
have consumed zeros believing them random. **That is this milestone's to fix, not milestone 220's**,
and it is worth fixing before 220 lands rather than after, because once the clock works the symptom
disappears and the defect does not.

## What this does not decide

**DECISIONS §120's stopgap question is separate and not reopened by this milestone.** Whether the
interactive boot should grant *any* entropy source (virtio-rng or this driver) before there is a
real customer for interactive login is §120's call, declined for now; building this driver does not
require revisiting that, and landing this driver does not by itself grant it to the boot path.

## What it unblocks

The real-hardware half of milestone 56's own claim: an entropy service this tree can trust on the
board it actually ships to, not only in QEMU. Downstream of that, whenever §120's stopgap question
is revisited with a real customer, the answer can be "real hardware entropy," not only "QEMU's
virtio-rng."
