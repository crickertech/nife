# 243. A machine with no serial port has no way to say anything, and no gate can read it

**Status: PARTIAL.** Minted 2026-09-03 by calef, from asking how nife reaches commodity hardware.
Built 2026-09-04: **the human half is answered on x86_64/UEFI and the gate half is answered only
under QEMU.** See `notes/serial-less-output.md`, and the `## Follow-on` section for what is left.

**Gate: NONE.** Everything it needs is a design question rather than a dependency.

## What was built, and what it does not cover

**The mechanism chosen is the firmware's own linear framebuffer.** UEFI's
`EFI_GRAPHICS_OUTPUT_PROTOCOL` reports one, and **its address survives `ExitBootServices`**, because
the aperture is a BAR on the display adapter rather than firmware memory: what ends is the firmware's
*console*, not the *display*. That single fact is why this milestone is a week rather than a quarter.

| Piece | Where |
|---|---|
| The sentence one boot stage says to the next | `machine_discovery::framebuffer`, riding PVH's own `cmdline_paddr` (a field the format has always had and nothing ever read) |
| Asking the firmware | `uefi_loader::find_screen`, one `LocateProtocol` call |
| Painting text | `crates/screen_console` (**name provisional**), sharing `bitmap_font` with the graphical terminal and sharing nothing else with `video_terminal` |
| The kernel seam | `console::attach_screen`, teed under the lock `print!` already takes; `arch::x86_64::mmu` carries the aperture into the fine map |
| **A gate reading a screen** | `board_console::screen`: a screendump decoded back into text by exact glyph match, judged by `board_console::progress` unchanged |

**Measured under OVMF, 2026-09-04:** 1280x800 bgrx at `0x80000000`, 182x100 cells, and
`cargo xtask uefi-boot` reads **96 non-blank rows of the boot tour back off the framebuffer**. That
assertion is the one the serial transcript could not make: the transcript would be identical if the
screen were black.

**Three things it does not cover**, and they map onto the block's own three problems:

- **Problem 2 on real hardware.** A gate can read a *virtual* machine's screen. Nobody can ask
  Graeme's laptop for a screendump, so the fleet's record is still a photograph and a person. The
  answer is postmortem to the boot medium and it is blocked on USB mass storage; see `## Follow-on`.
- **Problem 3 entirely.** The screen is armed by the first statement of the x86 boot tour, which is
  as early as a kernel can manage, and everything before `kernel_main` still writes nothing anywhere.
- **The other two architectures.** This is x86_64/UEFI only. The crate halves were written
  arch-neutral on purpose: what milestone 157 needs to add on aarch64 is the *discovery*, not the
  console.

**In brief.** Every word nife has ever said, it said down a UART.

- The boot tour, on all three machines.
- The console server and the shell (DECISIONS §21's line discipline).
- Kernel fault reports, which milestone 230 (`script/shell-check` is red on `main`, on both
  architectures, and nothing says so) found interleaving with the console server's own output because
  two address spaces drive the one device.
- **And every automated gate**: `script/board-console` (milestone 216), the soak's heartbeat, and
  milestone 218's (every boot of the VisionFive 2 needs a human typing four commands into U-Boot)
  boot script all read that line.

**A commodity machine does not have one.** xenon does, and that was chosen rather than lucky:
milestone 87 (the x86_64 bare-metal machine) picked a Dell with a C4PDJ serial module and a null
modem. So this was consciously deferred, and this block is where the deferral stops being implicit.

## Three problems wearing one name

**1. A human watching a machine boot.** Answered by the framebuffer chain: milestone 157 (the U-Boot
framebuffer handoff), milestone 242 (USB host and HID) and milestone 177 (wire the graphical terminal
stack into the real interactive boot). That path exists and is partly built.

**2. A gate reading a machine.** **No answer exists.** Every instrument this project has for
unattended hardware reads a serial line. The candidates each cost something:

- **A network console.** `smoltcp` works and radon's tree describes two Ethernet controllers, but it
  needs a NIC driver per board and says nothing until the stack is up.
- **Postmortem to storage**, which is what a headless panic actually needs and what nothing here
  does.
- **The firmware's own console.** UEFI `ConOut` works before `ExitBootServices` and therefore covers
  the loader and not the kernel.

**3. Early boot, which is the hardest.** **Before the framebuffer exists, a serial-less machine is
silent**, and a panic there produces nothing at all. This is not a nife peculiarity: it is why
commodity operating systems ship a firmware console, a splash, or postmortem logging. **nife has
none of the three.**

## The fleet this unlocks, which calef named on the day this was minted

> if I can boot a nife system off of a USB drive, then that opens up a lot of different hardware in
> our house that I can use for testing: Graeme's laptop, his desktop, cordoba, two MacBooks, Clay's
> desktop
>
> -- calef, 2026-09-03

**That fleet is already reachable and nobody had noticed.** Milestone 87 (the x86_64 bare-metal
machine) boots from a FAT32 stick at `\EFI\BOOT\BOOTX64.EFI`, which is the removable-media
fallback **every** UEFI firmware looks for with no configuration. Nothing about it is specific to
xenon.

**And `uefi_loader` already writes to the firmware's `ConOut`**, so on any UEFI x86_64 machine a nife
stick prints to the monitor with no driver of ours, right up to `ExitBootServices`. After that the
kernel takes over and the machine goes silent, which is exactly this milestone.

**So the blocker on using those machines is this block and not milestone 242** (USB host and HID,
because on commodity hardware the keyboard is not a UART). **A boot test needs output, not input.**

Two qualifications, both worth stating rather than discovering:

- **The Apple Silicon MacBooks are not in this fleet, and not for the reason first given.** They boot
  non-Apple kernels legitimately, and `notes/target-hardware.md` records that Asahi Linux is built on
  a documented permissive-security mode with an Apple-signed `m1n1` payload. But they have **no
  UEFI**, so a `BOOTX64.EFI` stick will not start one: that is a port, filed in that note as
  "possibly the fifth", not a boot. An Intel MacBook is in the fleet. Worth knowing for later: m1n1
  offers a serial console over USB-C, which is more than any commodity x86 machine here offers.
- **Milestone 195 (finish the UEFI boot path) created a question each machine answers for itself**:
  `PHYS_START` moved from 1 MiB to 32 MiB because OVMF holds ACPI NVS and its own allocations across
  the low range. Whether a given machine leaves 32 MiB free is that machine's answer, and the loader
  now prints which range it wanted and which descriptors are in the way rather than `Load Error`.

**The cheap experiment this makes available:** build a stick, boot one of those machines, and read
what the loader says on the screen. Even with the kernel silent afterwards, that establishes whether
the firmware finds the stick, whether Secure Boot refuses it, and what the memory map looks like away
from OVMF. **A loader that prints and a kernel that then goes quiet is this milestone confirmed by
evidence rather than by reasoning.**

**That experiment is now worth more than it was when this was written**, and the procedure for it is
`notes/serial-less-output.md`'s bench section. The kernel no longer goes quiet: it clears the screen
and prints its whole tour there. So the same stick, on the same machine, now distinguishes four
outcomes rather than two, and each is a different bug: the firmware not finding the stick, the loader
refusing and saying why, the screen clearing and staying black (the kernel armed its console and died
after), and the tour.

## Why it is worth one block rather than three

Because the answer might be one mechanism. A kernel that can write its diagnostics somewhere other
than a UART serves the human, the gate and the panic at once, and choosing three separate answers
would be the expensive way to discover that. **This block does not pick the mechanism**, and the
choice is the milestone.

The constraint that should drive it: **whatever it is has to work when the thing being reported is
the reason the machine is broken.** A network console needs a working stack; a filesystem log needs
a working filesystem; the UART needed neither, which is why it was the right first answer and why
replacing it is harder than it looks.

## Follow-on

- **Proposed.** A gate that can read a serial-less machine on real hardware, which is the half of
  this block that QEMU answered and silicon did not.
  `design/roadmap/proposals/a-gate-that-can-read-a-machine-with-no-serial-port.md`.
- **Proposed.** One screendump decoder rather than two: milestone 177's graphical `shell-check` leg
  carries `parse_ppm`/`decode_cell`/`scanout_rows` inside `xtask` and this milestone wrote a second,
  more general one in `board_console::screen`.
  `design/roadmap/proposals/one-screendump-decoder-not-two.md`.
- **Outstanding.** Problem 3, early boot, is untouched: nothing before `kernel_main` can say
  anything on a machine with no serial port, and a fault there is a black screen. Checked against
  the tree: `kernel/src/arch/x86_64/boot.s` writes to no device, and the screen cannot be armed
  before the boot handoff is readable.
- **Outstanding.** aarch64 and riscv64 have no screen. `machine_discovery::framebuffer` and
  `screen_console` are arch-neutral and unused there; milestone 157 (the U-Boot framebuffer handoff)
  is the discovery half. Checked against the tree: `console::attach_screen` has one caller and it is
  under `arch/x86_64/`.
- **Recorded.** The aperture is mapped uncacheable and scrolling reads it back, which is slow on real
  silicon and free under QEMU. Write-combining is a PAT entry this kernel does not program at all.
  `crates/screen_console/src/lib.rs`'s `BUGS`, and `notes/serial-less-output.md`'s.
- **Recorded.** Only ASCII reaches the screen, so `§` in the tour's last line is two blanks there and
  correct on the UART. `notes/serial-less-output.md`'s `BUGS`.
- **Refused.** A network console, on the block's own argument: it needs a driver per machine, says
  nothing until the stack is up, and cannot report the failure of the thing carrying it.
- **Refused.** Reading a *photograph* of a screen. `board_console::screen` works because a screendump
  is pixel-aligned with exact glyph matches and no threshold; a camera has none of those properties
  and making it work is optical character recognition, which is a different project.

## BUGS

- **The mechanism is chosen for x86_64/UEFI and nowhere else.** The block used to say it named no
  mechanism; it names one now, and the scope of that claim is one architecture and one firmware.
- **It does not cover the interleaving defect** milestone 230 found, where the kernel and the console
  server both drive one UART with nothing arbitrating. That is a live bug on the machines that *do*
  have a serial port and it has its own home.
- **A machine that can only report over a network is a machine whose failures you cannot see when
  the network is the failure**, and no answer here escapes that entirely.
