# 256. x86_64 places PCI BARs in a hardcoded window, and on xenon that window is RAM

**Status: NOT-STARTED.** Minted 2026-09-04 by calef, from
`design/roadmap/proposals/a-bar-window-the-machine-agreed-to.md` (written 2026-09-03 by the
milestone 247 sweep, out of milestone 165's Follow-on), on the evening xenon proved it from the
bench. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Both sources of truth are reachable from code that already runs: the firmware
memory map is parsed at boot, and Intel's host bridge reports `TOLUD` in its own configuration
space, which this kernel can already read. Confirming the result on xenon is `HARDWARE` of the
second kind, and the work does not start there: `pci::bar_census` already prints the number that
shows it working under QEMU.

## It stopped being a prediction on 2026-09-04

Milestone 165 wrote this on 2026-09-02, before any x86_64 kernel had met real firmware:

> That is the number to read first on xenon's first boot: **a machine whose RAM reaches above
> `0xc000_0000` would have this kernel move most of its bus on top of memory**, and the line says so
> before anything is driven.

Two days later, on xenon's second boot, with #738's local-APIC fix in:

```
[PANIC] panicked at kernel/src/arch/x86_64/mmu.rs:349:9:
failed to build the kernel page tables: AlreadyMapped mapping pci bar window
0xc0000000..0xc0200000: 0xc0000000 is also claimed by ram above the image
0xb9fbf000..0xc8940000
```

and one line above it, the census reading exactly what 165 said to read first:

```
pci : 15 function(s) on the bus, 13 with a BAR outside 0xc0000000..0xc0200000
```

**The prediction, the instrument that would show it, and the machine that confirmed it were all
written down before the confirmation arrived.** That is the shape this project is supposed to
produce, and it is worth stating plainly because the failures get recorded far more often than
this does.

**The panic is the lucky half and should not be mistaken for the risk.** `map_everything` refused
because the window collided with a range the mapper had already claimed, so the machine stopped. Had
xenon's RAM ended just below `0xc0000000` instead of at `0xc8940000`, nothing would have collided,
`place_bars` would have relocated thirteen of fifteen functions on top of memory the allocator
believes it owns, and the failure would have arrived later as corruption somewhere unrelated. The
proposal named that outcome a day before the boot: *"not a device that fails to enumerate, it is a
device writing into RAM the allocator believes it owns."*

## Why the constant was right and is not

`PCI_BAR_PHYS` is `0xc000_0000`, q35's conventional 32-bit PCI hole. Its doc comment is honest about
its evidence, and the evidence names the boundary it does not cross:

> confirmed disjoint from RAM, the ECAM window, the HPET and both APICs by reading QEMU's own
> `info mtree` on 2026-08-24 with **`-m 256M`**

On a 256 MiB machine `0xc000_0000` is empty. xenon has 16 GiB and its low DRAM runs to
`0xc894_0000`, straight through the window. The constant was checked against the only machine that
existed at the time and correct by luck anywhere else, which is the class this tree has now hit
three times on one architecture: **a constant that is right on emulated machines and has no firmware
source.** ECAM is the same class and is fine, because ACPI's MCFG names it and the kernel follows
the table (`PCI_ECAM_PHYS says 0xb0000000` while xenon's MCFG says `0xf0000000`, and the kernel took
the table's answer). The BAR window's equivalent lives in the PCI host bridge's `_CRS`, which is
AML, which milestone 165 refused and this block does not reopen.

## What to build

**Take the window from the machine, from two sources that can disagree.**

- **The firmware memory map**, already parsed at boot, which names the gaps between regions. On
  xenon the gap is visible in the photograph: the map's last entry below the hole ends at
  `0xd000_0000` and the next begins at `0xf000_0000`.
- **`TOLUD`** (top of low usable DRAM), which Intel's host bridge reports in its own configuration
  space, reachable with the config-space reads `kernel/src/pci.rs` already performs.

**Fail loudly when they disagree rather than falling back to the constant.** That is milestone 215's
posture for the analogous case, and its reason carries: a silent fallback to the old behaviour is
the original bug wearing the clothes of a graceful degradation.

**And handle the asymmetry that this boot exposed, which the proposal did not name.** QEMU hands the
kernel **unassigned** BARs and real firmware hands it **placed** ones. `place_bars` relocates
everything into the kernel's window unconditionally, which is why the census reads 13 of 15 rather
than 0. A BAR firmware has already placed in a window firmware itself chose is a BAR that wants
adopting, not moving; only genuinely unassigned BARs need placing. Whether that split belongs in
this milestone or in a second one is the lane's call to make and to say out loud.

## The measure

**`pci::bar_census`'s second number reaching zero on xenon**, or the window being one that machine
agreed to. Under QEMU the census is the same instrument and is already wired to the boot line, so
the work is testable on patagonia and only *confirmed* at the bench.

**It is also one line further into the tour.** The boot stopped at `mmu.rs:349` on 2026-09-04 and
everything past it is unseen on real firmware, so the honest expectation is that this milestone buys
the next stop rather than the end of the tour.

## BUGS

- **This block cannot say the fix is right until xenon runs it**, and xenon is a bench session that
  needs calef at the machine with a camera, because patagonia cannot be moved to it and there is no
  serial. Everything here is checkable under QEMU except the one fact that matters.
- **`TOLUD` is Intel's**, and this kernel claims three architectures. It is the right source for the
  machine we own and it is not a portable one; a non-Intel x86_64 host bridge reports the same fact
  somewhere else or not at all. Say so where the read happens.
- **The two sources may agree and both be wrong.** A gap in the firmware memory map is not a promise
  that nothing decodes there; it is only a promise that firmware did not call it RAM. The MMIO hole
  on a real machine holds things no table this kernel reads will name.
