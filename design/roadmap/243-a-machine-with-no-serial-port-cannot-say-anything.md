# 243. A machine with no serial port has no way to say anything, and no gate can read it

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from asking how nife reaches commodity hardware.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Everything it needs is a design question rather than a dependency.

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

## Why it is worth one block rather than three

Because the answer might be one mechanism. A kernel that can write its diagnostics somewhere other
than a UART serves the human, the gate and the panic at once, and choosing three separate answers
would be the expensive way to discover that. **This block does not pick the mechanism**, and the
choice is the milestone.

The constraint that should drive it: **whatever it is has to work when the thing being reported is
the reason the machine is broken.** A network console needs a working stack; a filesystem log needs
a working filesystem; the UART needed neither, which is why it was the right first answer and why
replacing it is harder than it looks.

## BUGS

- **This block names no mechanism**, which is deliberate and also means it cannot be scheduled until
  somebody does the choosing.
- **It does not cover the interleaving defect** milestone 230 found, where the kernel and the console
  server both drive one UART with nothing arbitrating. That is a live bug on the machines that *do*
  have a serial port and it has its own home.
- **A machine that can only report over a network is a machine whose failures you cannot see when
  the network is the failure**, and no answer here escapes that entirely.
