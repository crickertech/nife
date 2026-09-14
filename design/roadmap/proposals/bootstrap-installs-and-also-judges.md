# `script/bootstrap` installs what is missing, and also judges whether the machine is good enough

**Status: PROPOSED 2026-09-13.** Written by milestone 286's lane, out of a failure that milestone
measured on its own container.

**Gate: DECISION.** Whether the two jobs split, and if so how the caller asks for one without the
other, is a change to a canonical "Scripts to Rule Them All" entry point that `script/setup`,
`script/update` and `script/ci-build` all call. The measuring and the wiring are a lane's; whether
`bootstrap` is allowed to return "installed everything I could, and this machine is still not good
enough" as a *non-fatal* answer is not.

**In brief.** `script/bootstrap`'s own header promises one thing: *"it checks for what is already
present and installs only what is missing, so it is cheap to re-run and safe to call."* It does a
second thing the header does not mention. After installing, it calls `script/qemu-check`, which
asks whether the QEMU on `PATH` is *new enough* to boot this kernel, and exits non-zero when it is
not. Installing what is missing and judging whether what is present is adequate are two different
questions, and they fail in two different ways.

## Why now

Milestone 286 put `script/bootstrap` at the head of the command a developer runs before pushing.
That was calef's ruling and it is right. It also made this conflation expensive for the first time,
because a bootstrap that exits non-zero now ends the whole local tier.

**Measured on that lane's container**: packaged QEMU is 8.2.2, which lacks `riscv-iommu-pci`
(milestone 16b, `DECISIONS §20`). `script/bootstrap` installed nothing, broke nothing, found
everything it looks for present, and exited 1 on the adequacy check. On that machine
`script/ci-build fmt`, `script/ci-build lint` and `script/ci-build image-permissions` all pass
perfectly well; the no-argument path runs none of them.

Milestone 286 made that loud rather than silent, which is the half a lane could fix without a
ruling: the exit now says NO CHECKS RAN, lists the skipped tier out of `script/ci-build`'s table,
and names what is still runnable by hand. The half it could not fix is that the developer still has
to type those by hand on a machine that is merely out of date.

## The options

| | what | cost |
|---|---|---|
| **A** | leave it; the loud message is enough | zero, and the developer on a stale machine keeps typing three commands by hand |
| **B** | `script/bootstrap --no-verify`, and `script/ci-build` passes it | one flag; the caller now decides adequacy, which is a judgement moving to the wrong place |
| **C** | split: `bootstrap` provisions and returns zero when it installed everything it could; a separate adequacy check is its own row in `script/ci-build`'s table | the honest shape, and it changes what a canonical entry point's exit code means, which four callers read |
| **D** | `bootstrap` keeps both jobs but distinguishes the exit codes (say 1 for "could not install", 2 for "installed, still inadequate") | small; callers that do not look still see non-zero, and one that cares can |

**C is the shape this tree already reaches for**, and it is worth saying why rather than asserting
it. An adequacy check is a check: it renders a verdict about the machine, it is cheap, and
`script/ci-build`'s table is now the place a check lives. `script/qemu-check` already exists as its
own entry point, so most of C is wiring rather than writing. Against it: `script/setup` and
`script/update` currently get the adequacy verdict for free, and C must not quietly take that away
from a newcomer running `script/setup` on a fresh machine, which is exactly the reader the check was
written for.

**D is the cheap one and should be priced honestly as cheap.** It does not fix the conflation; it
makes it legible to one caller. If the answer is "not worth a split", D is better than A.

**B is refused unless calef wants it**, because it puts the decision "is this machine good enough"
in the caller, and the caller is a table of checks that has no opinion about QEMU versions.

## What is blocked

Nothing. Milestone 286 shipped, the failure is loud, and the workaround is three commands a
developer can type. This is a sharpness problem rather than a correctness one.
