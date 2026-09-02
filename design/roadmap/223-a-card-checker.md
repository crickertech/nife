# 223. Read a card and say whether its kernel and archive match, before the power cycle

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from milestone 218's (every boot of
the VisionFive 2 needs a human typing four commands into U-Boot) lane, which named it while closing
milestone 217 (the card carries a kernel and an archive from different builds). *(Number provisional
until the merge queue lands it.)*

**Gate: NONE.** It reads a mounted filesystem and needs no board.

**In brief.** On 2026-09-01 **radon** halted with `MEASURED BOOT REFUSED` because its card held a
kernel from one build and a userspace archive from another, six days apart. The measured-boot gate
caught it, which is the gate working, and it caught it **after** a power cycle, a serial capture and
a person watching.

Milestone 217 narrowed who can hit this: `script/board-image --card` now copies the three files as an
indivisible set, so a card written by the script is consistent by construction. **It did not remove
the need**, and 217's own `BUGS` says so. A card can be written by hand, written by an older script,
half-copied, or carried between machines.

## What it needs

**A check that reads a mounted volume and reports whether its kernel vouches for its archive.** The
kernel compiles in a manifest of the archive it was built against, which is what the refusal
compares, so the answer is derivable on the host without booting anything.

**The point is where the answer arrives, not that it exists.** Today it arrives as a halted board and
a serial log. It should arrive before somebody walks to the board, and the whole value of this
milestone is that difference.

## BUGS

- **It duplicates a check the kernel already does**, and that is deliberate rather than an oversight:
  the kernel's is the authority and this one is an early warning. If they ever disagree, the kernel
  is right and this milestone is wrong, and whatever it prints should say so.
- **It says nothing about the boot script or the extlinux config**, which are the other two ways a
  card can be wrong, and milestone 218 has just changed which of those a card should carry.
- **Nothing forces anyone to run it.** It is rung two at best, and a card written by hand by somebody
  who did not read this block is exactly the case it cannot reach.
