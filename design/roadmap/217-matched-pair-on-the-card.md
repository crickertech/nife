# 217. The card carries a kernel and an archive from different builds, and the gate is the only thing that noticed

**Status: NOT-STARTED.** Minted 2026-09-01 by the maintainer, from a boot driven over the serial
console the same hour. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The fix is in `script/board-image`, and the evidence is already captured.

**In brief.** The VisionFive 2 boots, starts a user program at U-mode, and then halts:

```
userspace  : a program ran at U-mode and made 0 syscalls (yield/yield/exit via ecall)

MEASURED BOOT REFUSED: 'init' is not what this kernel image was built against
  expected sha256 d63054330191625f54db33110bdf3882d6f81bc5c97877f32f574c2e8cc798b1
  measured sha256 dfd6a05381ab613cfc6cbcdb007ed4c67316e60229347f87e8b05be7643dbc11
halting rather than handing the archive to init.
```

**That is the gate working and it should be read as a success**, in the same way DECISIONS §14's
verified core is a success when it refuses. The kernel on the card was built against a different
userspace archive than the one on the card, so it declines to hand the archive to `init`. Nothing is
wrong with either file; they are from different builds.

## Why the pair drifted, which is the part worth fixing

`script/board-image` builds a matched pair correctly, and its own comment records the scar that
taught it to: on 2026-08-15 it built kernel-first, produced a kernel vouching for the *previous*
archive, and the board refused the pair at the bench (`MEASURED BOOT REFUSED`, boot 12). The build
order was fixed then.

**What was not fixed is the card-prep instructions the script prints**, and they are how the pair
comes apart afterwards. Step 3 copies two files:

```
cp target/board/nife-vf2.img /Volumes/NIFE/
mkdir -p /Volumes/NIFE/extlinux
cp target/board/extlinux/extlinux.conf /Volumes/NIFE/extlinux/
```

The archive is mentioned only in the build output, as *"the userspace archive; optional third
file on the card"*. **It is not optional and calling it optional is the defect.** A person who
follows the printed steps exactly copies a new kernel over an old archive, which is precisely the
2026-08-15 failure reintroduced one layer out: the script stopped producing a mismatched pair and
started instructing one.

## What it needs

- **The printed card-prep steps copy all three files**, with the archive no longer described as
  optional.
- **A word about why**, at the place the reader meets it: the kernel measures the archive and
  refuses a mismatch, so the two files travel together or the board halts.
- **Consider making the mismatch impossible to express rather than documented.** The strongest form
  available here is that the script copies the files itself when given a target volume, so a human
  cannot copy one without the other. That is rung one of AGENTS.md's ladder against this block's
  rung three. It is in tension with the script's deliberate refusal to run anything destructive
  ("`dd` to a device is a decision the person at the bench makes"), and **that tension is the design
  question this milestone should answer rather than assume**: copying files onto a mounted
  filesystem is not `dd` to a raw device, and the two may not deserve the same caution.

## BUGS

- **This block does not fix any card.** Re-flashing is a bench action and the pair on the card is
  stale until somebody does it.
- **Nothing here checks a card after the fact.** A tool that reads a mounted volume and reports
  whether its kernel and archive match would catch this before a power cycle rather than after, and
  is not in scope.
- **The measured-boot refusal is only as good as the manifest.** This milestone treats the gate as
  correct because it fired on a real mismatch; it makes no claim about what the gate would miss.
