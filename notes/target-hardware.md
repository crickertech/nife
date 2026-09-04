# Where nife could actually run

> **Recast (2026-07-27):** milestone 16 is now RISC-V-first (design/roadmap/16-real-hardware-iommu.md): first
> silicon is a VisionFive 2-class board, whose OpenSBI/NS16550/PLIC/Sv39 contract the kernel
> already speaks exactly, and the IOMMU work targets QEMU's emulation of the ratified RISC-V
> IOMMU before any silicon. This note's Pi-first analysis predates the riscv port reaching
> parity and stands as the aarch64-side record; its "the ISA is almost never the constraint"
> thesis is exactly why the cheaper-contract board wins.

## The three machines have names: argon, radon, xenon

Ratified by calef, 2026-09-01, once all three existed and "the board" had stopped being an
unambiguous phrase in a conversation. One noble gas per architecture:

| name | architecture | machine | state, 2026-09-01 |
|---|---|---|---|
| **argon** | aarch64 | NVIDIA Jetson (milestone 127) | in hand |
| **radon** | riscv64 | StarFive VisionFive 2, JH7110 | boots nife, wired as a bench target |
| **xenon** | x86_64 | Dell OptiPlex, serial port and null modem (milestone 87) | in hand, no first light yet |

**Why they earn names rather than descriptions.** This project's own tenet is that a name is a
claim and a reader meets it before anything else. "The board" was serviceable while there was
one; with three it silently means whichever the speaker had in mind, and the cost lands on
whoever reads the sentence later. The names are also stable in a way the descriptions are not:
the aarch64 board may not always be a Jetson, but argon stays argon.

They are deliberately not architecture names. `riscv64` already names the ISA, and a machine
name has to survive the day a second machine of the same architecture arrives.

The development machines keep their existing names and are not part of this scheme:
**patagonia** (the Mac everything is built on) and **cordoba** (the always-on x86 box).

## The ISA is almost never the constraint

"Does it run aarch64" is the wrong question. **And the answer splits in two, which this note ran
together until 2026-09-04** (calef: *"nife should be able to run on hardware without a serial
console"*). Some of these are properties a machine must have to run nife at all; one is a property
*we* need to bring nife up on it, and confusing them makes the supported set look smaller than it is.

**To run nife:**

1. **Can you get code to execute at boot?** Unlocked bootloader, or no secure boot at all.
2. **Are the peripherals documented?** You need an interrupt controller, a timer, and eventually
   storage. The CPU is standardized. The stuff bolted around it is not, and that's where the work is.
3. **At least 32 KB of L1 instruction cache**, for the reason in the next section. A requirement on
   the claims rather than on the boot.

**To bring nife up on it, which is ours and not the machine's:**

4. **Can you physically reach a serial console?** Without one you are debugging a black box.

**Today these are the same list, and that is a defect this project owns rather than a fact about
hardware.** Every word nife has ever said went down a UART: the boot tour on all three machines, the
console server and the shell, kernel fault reports, and every automated gate that reads any of them.
So a machine with no serial port cannot currently *tell us* it is working, which is not the same as
being unable to work. **Milestone 243 (a machine with no serial port has no way to say anything, and
no gate can read it) is the milestone that separates them**, and until it lands, requirement 4 is
doing the job of requirements 1 to 3 by proxy.

**The reason it matters is not tidiness.** calef's fleet argument on milestone 241 names Graeme's
laptop and desktop, an Intel MacBook, cordoba and Clay's desktop as machines a USB stick could boot
(milestone 87 boots from `\EFI\BOOT\BOOTX64.EFI`, the removable-media fallback every UEFI firmware
looks for). **Not one of them has a serial port.** Read as a running requirement, item 4 excludes the
entire fleet; read correctly, it says only that we cannot yet watch them.

A device can be aarch64 and still be completely useless to us by failing any of items 1 to 3, and
useless *for development* by failing item 4.

## A fourth requirement, and this one filters silicon rather than firmware

**At least 32 KB of L1 instruction cache**, and it is a requirement on the *claims* rather than on
the boot. nife will start on less; what will not survive is the reason a microkernel is supposed to
be fast.

**Where the number comes from.** Liedtke's *On micro-Kernel Construction* (SOSP 1995) argued Mach's
IPC was slow because of the **cache footprint** of its hot path rather than anything inherent to
microkernels: a kernel touching a lot of memory per IPC evicts the *application's* working set, so
the cost appears as capacity misses spread through the workload instead of as time in the kernel.
`script/fastpath-footprint` is the gate that keeps this honest and notes/benchmarks.md carries the
argument.

**Measured 2026-09-04**, the fastpath's upper bound per architecture, against the L1i of the machine
this project runs that ISA on:

| target | fastpath | machine | L1i | fastpath as a share |
|---|---|---|---|---|
| x86_64 | **8,404 B** | xenon, Core i5-7500T | 32 KB* | **26%** |
| aarch64 | 9,156 B | argon, Cortex-A57 | 48 KB | 19% |
| riscv64 | 7,174 B | radon, SiFive U74 | 32 KB | 22% |

At 32 KB the hot path takes about a quarter of the cache and leaves the application three quarters,
which is the regime Liedtke's argument assumes. **At 16 KB it would take half**, and the thing the
gate exists to protect stops being true. That is the filter: **16 KB L1i is where nife stops being
able to claim what it claims**, and 32 KB is the floor at which the claim is comfortable.

**What this rules in and out.** Every frontier core clears it easily: notes/benchmarks.md's survey
puts Zen 5 at 32 KB, Intel's Lion Cove and Arm's Cortex-X925 and SiFive's P870 at 64 KB, and Apple
at 192 KB. What it rules out is the small end, and that is the end a capability microkernel is
otherwise attractive at: deeply embedded Cortex-M and Cortex-R parts, older in-order cores, and
microcontroller-class RISC-V. **A machine can satisfy all three requirements above and still fail
this one**, which is why it is stated separately rather than folded into "the peripherals are
documented".

**The honest caveat.** No cache is modelled by icount, and the development host's L1i is several
times the boards', so nothing in this tree has yet *observed* the effect this requirement protects
against. It is an argument from a 1995 paper plus a measured code size, not a measured miss rate.
The experiment that would settle it wants real silicon and performance counters, which milestone 74
has just made possible on radon.

## Trap: "ARM" is not "aarch64"

**Cortex-M** microcontrollers (STM32, most Arduino-adjacent parts) are 32-bit and have **no
MMU**. They cannot run the OS we are building. Ever. No virtual addresses, no isolation, no
user mode as we mean it. They can run an RTOS; that is a different thing.

We need **Cortex-A53 or newer, in 64-bit mode**. Same reason the RISC-V hardware we
considered had to be JH7110-class or better (see [mmu.md](mmu.md)).

## The realistic targets

| Device | Boot access | Peripheral docs | Verdict |
|---|---|---|---|
| **Raspberry Pi 4** (~$60) | Wide open. GPU firmware loads `kernel8.img` off a FAT32 SD card. No signing, no lock. | Excellent, plus the largest bare-metal community anywhere | **The next port.** Serial is a $10 USB-TTL cable on GPIO 14/15. |
| **Raspberry Pi 5** | Same | Worse. I/O routes through the RP1 southbridge over PCIe, less documented, less trodden for bare metal | Doable, but Pi 4 is the safer first port |
| **Rockchip / Allwinner SBCs** (Orange Pi, Radxa Rock, Pine64) | U-Boot from SD | Decent TRMs, much thinner community | Fine. Similar difficulty, less help when stuck |
| **NVIDIA Jetson** | Possible | Good docs, but the TegraBoot chain is genuinely complicated | More work than it's worth as a first port |
| **AWS Graviton bare-metal EC2** (`c7g.metal`) | UEFI. Rent by the hour; no hardware to buy or brick | SBSA-standard server ARM | **Interesting for a specific reason.** See below. |
| **Ampere Altra** workstations / dev kits | UEFI | Standards-compliant | Same category, but you'd own it |
| **Android phones** with unlockable bootloaders (Pixel, Fairphone) | fastboot | Poor. Qualcomm peripherals are barely documented, and you fight TrustZone | Painful. Possible, rarely rewarding. |
| **iPhone, iPad, Apple TV** | Locked, signed |: | No |

## The wild one: an Apple Silicon Mac

This is real, and it is not a jailbreak.

**Apple deliberately permits booting non-Apple kernels on Apple Silicon.** There is a
documented "permissive security" mode, and Asahi Linux is built entirely on it. Their
bootloader, **m1n1**, runs as an Apple-signed payload, then loads an arbitrary kernel image.
It also gives you a **serial console over USB-C** and a hypervisor mode you can use to trace
what macOS itself does to the hardware.

So an M-series Mac is genuinely, legitimately bootable with our own OS.

The catch is brutal: **Apple documents none of the peripherals.** Asahi reverse-engineered
the interrupt controller, the UART, the display, and everything else over several years.
We'd be leaning entirely on their documentation and would be a long way off the beaten path.

Filed as: not the second port. Possibly the fifth. A genuinely impressive one.

## The reframe worth taking seriously

Go back to the Alpha lesson in [portability.md](portability.md): **the second port should be
as alien as possible, because that is what forces hidden assumptions into the open.** Porting
to something *similar* teaches you very little.

Now look at what the Pi actually is:

| | Hardware discovery |
|---|---|
| QEMU `virt` | **Device Tree** |
| Raspberry Pi | **Device Tree** |
| Graviton / Ampere (UEFI + ACPI) | **ACPI tables + PCIe enumeration** |

The Pi is different peripherals inside the *same worldview*. Valuable, and it will shake out
real bugs, but it is a port within one model.

A **UEFI + ACPI server ARM machine** is a genuinely different world: a different firmware
handoff, and hardware discovered by walking ACPI tables and enumerating PCIe rather than
reading a flattened tree. *That* is the port that finds our hidden assumptions, and it is
where the `arch/` boundary either holds up or gets exposed as fiction.

Graviton bare metal costs a few dollars an hour, with no hardware to buy or brick.

## The plan

1. **Raspberry Pi 4 is the next port.** Cheap, open, enormous community, and it delivers the
   "I ran my OS on a computer I can hold" moment, which is worth more motivationally than it
   sounds. It teaches us what real hardware quirks feel like.
2. **Then a UEFI/ACPI target**, precisely because it is alien. This is the one that tests
   whether the hardware abstraction boundary is real.
3. **Apple Silicon as the trophy.** Hardest, most impressive, and we already own the machine.

---

*Add to this file as new targets come up.*
