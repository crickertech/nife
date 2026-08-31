# 202. Every confinement test is a ritual until somebody breaks the confinement and watches it fail

**Status: NOT-STARTED.** Minted 2026-08-31 by calef, scoping `design/fatal-risks.md`'s risk 7, which
its own `BUGS` recorded as unowned and needing framing before a lane. *(Number provisional until the
merge queue lands it.)*

**Gate: NONE.** The claims and the tests both exist; what is missing is the evidence that the tests
would notice a break, and nothing has to be decided to go and find out.

**In brief.** Risk 7 is *"the confinement claim is false."* The evidence against it is a set of tests
this project wrote about attacks this project chose, and **a passing confinement test is consistent
with two very different worlds.**

DECISIONS §31's (the foreign-language seam) C component performs a deliberate out-of-bounds write,
and the test asserts its two witness pages are unchanged. That passes if the component reached the
page and the MMU stopped it. **It also passes if the component never reached that address at all**,
in which case the assertion is decorative. Nothing in this tree distinguishes those cases.

## The mechanism, which DECISIONS §134 already decided

§134 (a harness carries a machine-replayable falsification record) settled that a proof which cannot
be made to fail is not evidence, and chose a recorded, replayable diff as the way to show it can.
**The same discipline applies to a confinement test with no change of shape.**

Worked example on §31's own setup: grant the C component write access to `WITNESS_RO`, rebuild, run.
**The test must go red.** If it still passes, the test was never checking what it claims.

## The work

1. **Enumerate the claims.** What does this system say a confined component cannot do? Reach memory
   outside its grant; make a syscall without a capability; widen its own rights; DMA outside its
   IOMMU domain; name another process's objects. The enumeration is the first deliverable and is
   worth review on its own, because a claim nobody wrote down is a claim nobody is testing.
2. **Record, per claim, the change that would break it**: remove the check, widen the grant, drop the
   domain.
3. **Show the corresponding test goes red** under that change.
4. **Store it replayably**, in whatever shape milestone 194 (build §134) lands, so the two do not
   invent separate conventions for one idea.

## What this is not, and the block exists partly to say so

**It cannot find the claim nobody made.** Falsifying the claims we enumerated will never reach an
escape route we never thought of, and that is where real escapes live.

So this is a **floor**. It converts existing confinement tests from rituals into evidence, and it
says nothing about the attacks nobody imagined. A genuine adversarial pass by someone who did not
build this system is a different and better thing; it wants outside eyes, and calef's position that
no third party sees nife until there is a package manager and a trivial install gates it behind
milestone 198 (a package manager, and the trivial install that makes a second customer possible).

**No result from this milestone may be quoted as "the confinement holds."** What it can support is
narrower and worth having: *these named claims are tested, and each test has been shown to fail when
the claim is broken.*

## BUGS

- **Breaking confinement deliberately means writing code that must never ship.** The falsification
  diffs are, by construction, patches that disable security checks, and they will live in the tree
  next to the checks they disable.
- **A test can go red for the wrong reason.** Widening a grant may break a test through an unrelated
  assertion, which looks like success and proves nothing, so each falsification has to name which
  assertion it expects to fail.
- **It duplicates milestone 194's machinery if the two are built separately**, which is why item 4
  points at whatever 194 lands rather than specifying a format here.
- **The enumeration in step 1 is the hard part and is not a lane's to finish alone.** What this
  system claims about confinement is spread across DECISIONS §31, §14, the DMA validator and
  `notes/untrusted-input-audit.md`, and assembling it will surface claims nobody has stated plainly.
