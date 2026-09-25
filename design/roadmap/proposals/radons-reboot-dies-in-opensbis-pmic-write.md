# radon's cold reboot dies in OpenSBI's PMIC write, and the kernel may be able to set that write up

**Status: PROPOSED 2026-09-24.** Raised by the lane that brought milestone 249 (the boot lottery is sampled by a person walking to the board)'s reboot to aarch64
and `x86_64`. The maintainer asked it to price radon's reset hang. Reading the 2026-09-04 log against
OpenSBI's source showed that the hang is in OpenSBI and not in U-Boot SPL. Milestone 249's block
carries the correction.

**Gate: HARDWARE.** Every option ends in one boot of radon. Option C is also a DECISION, because it
writes radon's SPI flash.

## What happens

On radon, SBI SRST reset and shutdown are both a single I2C write to the AXP15060 PMIC at `0x36` on
I2C5: register `0x32`, bit 6 for a reset and bit 7 for a power-off. OpenSBI's `pm_system_reset` has
no other route. When the read that comes before the write fails, it prints `cannot read pmic power
register` and hangs the hart. The source is `starfive-tech/opensbi`, `JH7110_VisionFive2_devel`,
`platform/generic/starfive/jh7110.c` lines 126 to 190, read 2026-09-24. The 2026-09-04 log shows
exactly that, and no second boot.

The likely cause is in the log too. At `Starting kernel`, U-Boot prints `clk u5_dw_i2c_clk_core
already disabled` and `clk u5_dw_i2c_clk_apb already disabled`. OpenSBI re-enables only the APB gate
(syscrg `0x1302_0000 + 0x228 + 5*4`, bit 31), and only when that whole register reads zero. It
leaves the core clock off. Upstream OpenSBI commit `4d8569df7bd7` (2024-02-22) fixes a related
defect: the vendor code finds the controller by the node name `i2c5`, which U-Boot's tree does not
use. Neither point is proven on radon.

A second thing to know before any of these works: the vendor code sets bit 7 (power-off)
unconditionally before it chooses between bits 6 and 7, so even a successful write may power the
board off rather than reset it. A board that goes dark after a fix is the next row of notes/soak.md's
outcome table, not a new mystery.

## Options, cheapest first

| | what | cost | reversible | whose call |
|---|---|---|---|---|
| A | Before the `ecall`, the kernel sets the gate bit on both I2C5 clocks (APB and core) in syscrg and prints what each register held | about 20 lines in `arch/riscv64`, under `board`; the syscrg page has to be in the kernel's map, which this lane did not verify | yes | a lane, then one bench boot |
| B | The kernel resets the board itself: a minimal DesignWare I2C master, and the AXP15060 write OpenSBI makes. This resets the PMIC, so it is a real power cycle | about 250 lines (OpenSBI's `lib/utils/i2c/dw_i2c.c` is about 190), plus pinmux; nife has no I2C code today | yes | a lane |
| C | Update radon's SPL and OpenSBI to an upstream build that carries the fixes | writes the SPI flash of the only board of its kind; milestone 218 (every boot of the VisionFive 2 needs a human typing four commands into U-Boot) refused that write for the same reason | no | calef |

Recommendation: A first. It costs one bench boot to learn whether the clock is the cause. If it
is, milestone 249's series is available with no new driver and no firmware change. If A fails, B is
the kernel-side answer that needs nothing from firmware. C stays calef's call.

Would a watchdog avoid this? Probably. A JH7110 watchdog reset does not go through OpenSBI's
`pm-reset` path. Whether SPL survives a SoC-only reset with the PMIC left alone has never been tested.
See `a-wedged-kernel-resets-itself.md` beside this file.
