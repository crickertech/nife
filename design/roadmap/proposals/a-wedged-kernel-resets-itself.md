# A wedged kernel cannot ask for a reset, so the soak should arm a watchdog that does not need asking

**Status: PROPOSED 2026-09-24.** Raised by the lane that brought milestone 249 (the boot lottery is sampled by a person walking to the board)'s reboot to all three
architectures. Milestone 249 resets a kernel that is *working* and chooses to reset. A kernel that
has wedged cannot call `arch::reboot`, and a wedge is exactly what the soak (milestones 219 and 225)
is looking for. Today a wedged board waits for a person.

**Gate: NONE.** xenon's half needs nothing, because q35 emulates the TCO and CI can prove it. radon's
and argon's halves are HARDWARE and would be split out at promotion; argon's also waits on its first
nife boot.

## The shape

The soak arms the board's watchdog at `soak-test: started` and pets it from the supervisor's beat,
every five seconds. A kernel that stops beating stops petting, and the board resets itself. The
timeout has to be longer than a beat and shorter than a person's patience: about 60 seconds fits
every part below. The trade-off has to be designed first, and it is 249's: a reset destroys the
wedged state, which is the best evidence a soak can produce. So the reset should follow the stall
dump the watcher already prints, never come instead of it, and a build that arms the watchdog should
say so in its banner the way `reboot_soak_test` does.

## Per board, read not recalled (sources as the lane found them, 2026-09-24)

| board | part | how it resets | cost | testable in QEMU |
|---|---|---|---|---|
| xenon | Intel TCO on the Q270 PCH (TCO v4; SMBus `0:1f.4` config `0x50` gives TCOBASE, enabled by `0x54` bit 8). RLD `+0x00` pets it, TMR `+0x12` holds the timeout in 0.6 s ticks. Linux `iTCO_wdt.c` and `i2c-i801.c` | a platform reset on the second timeout. NO_REBOOT sits behind P2SB sideband (`SBREG_BAR + 0xc6000c`, bit 1), and firmware may have locked it | about 150 lines, P2SB unlock included | yes: q35 emulates ICH9 TCO (`hw/acpi/ich9_tco.c`), and `noreboot` defaults off on current machine types |
| radon | JH7110 watchdog, `0x1307_0000`, SP805-like: LOAD `0x000`, CONTROL `0x008`, INTCLR `0x00c`, LOCK `0xc00` (unlock `0x1ACCE551`). Clocks syscrg 122 and 123, resets 109 and 110, 24 MHz. Linux `starfive-wdt.c`, `jh7110.dtsi` | two-stage: interrupt on the first expiry, reset on the second; about 357 s maximum | about 100 lines, with the clock and reset plumbing notes/jh7110-clock-and-reset.md already describes | no (no QEMU model) |
| argon | Tegra X1 timer block `0x6000_5000`, WDT0 at `+0x100` paired with timer 5. 1 MHz, reset on the fourth expiry, 1 to 255 s. Linux `tegra_wdt.c` (which binds only `tegra30-timer`, so the T210 layout is an inference to check against the TRM) | chip reset through the PMC, which re-runs BootROM and the NVIDIA boot chain | about 60 lines | no (`virt` has no watchdog; `sbsa_gwdt` exists only on `sbsa-ref`) |

Recommendation: xenon's TCO first. It is the only one that can be proven in CI before anyone
touches a board, and xenon is the board with a proven kernel-side reboot. radon's would be second.
Its watchdog reset does not go through OpenSBI's `pm-reset`, so it may come back where SRST does
not. That is untested, and the first watchdog boot on radon is the test.

## What needs calef

- xenon's POST settings. notes/xenon-firmware.md records `Warnings and Errors: Prompt on Warnings
  and Errors` and keyboard error detection on. A watchdog reset that stops at a POST prompt has not
  recovered anything. Changing these is a firmware change.
- Whether a soak should reset over its own wedge at all, rather than hold the board for a person.
  That is the same question milestone 249 answered for a *failing* soak (never reboot over evidence),
  asked again for a *silent* one. Recommend: dump, then reset, with the dump written somewhere that
  survives the reset. On these boards that means the console log the watcher already holds.
