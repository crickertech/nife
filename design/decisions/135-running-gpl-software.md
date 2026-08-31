# 135. Running GPL software is aggregation, the capability boundary is what makes it so, and packages are how it arrives

**Status: PROPOSED.** Raised 2026-08-30 by calef, from the question of whether to delete the SMB
implementation: *"Isn't delivering GPL software on nife as stand alone programs a worth
demonstration in itself? There is a lot of GPL software we would want to run."* And then the
sharpening that changed the answer: *"does GPL and LGPL need to be shipped in an image as
aggregation? Can't it be installed through our to be delivered package manager?"* *(Section number
provisional until the merge queue lands it.)*

## What is being decided

Whether nife may run GPL and LGPL programs, what that does to nife's own licence, **through which
channel such programs reach a machine**, and whether the ability to do it is a claim worth making
rather than a compliance detail.

## Why now, and it is not the SMB question

The SMB question is answered on other grounds (Samba is unrunnable here for POSIX reasons, not
licence ones; see the refusal at the bottom). What forces this section is that **the roadmap already
plans to run GPL programs and nobody noticed.** Milestone 99 / milestone 171 is `git`, GPLv2.
Milestone 170 is `nano`, GPLv3. Those lanes will meet this question with no decision behind them.

A licence posture is in the *move fast on what can be undone* tenet's irreversible category, a fact
that leaves the machine: it is quoted, relied on, and cannot be unpublished. It should be decided
before a lane needs it rather than by a lane that needed it.

## The insight, which is architectural rather than legal

**The same boundary that confines a compromise confines a licence.**

In a monolith, GPL code in the kernel makes the kernel GPL. That is why Linux's ext4, ksmbd, and its
drivers are GPL, and why a permissively licensed kernel cannot take any of them. In a capability
microkernel every service is a separate program communicating over IPC, so a GPL filesystem server, a
GPL network service or a GPL editor is **aggregation rather than derivation**, and the licence stops
at the process boundary the same way a fault does.

That belongs beside the confinement claim rather than in a compliance appendix, and it is the
demonstration calef is pointing at: not *"we can run GPL software"*, which is unremarkable, but
*"somebody else's large, memory-unsafe, ambient-authority-assuming program runs here holding only
what it was granted, and its licence reaches nothing."* `caps <program>` prints the first half. This
section is what makes the second half sayable.

## The pattern this corrects, and every instance of it was right

This tree has routed around the GPL three times and been correct every time:

- Milestone 149 wrote STREAM clean rather than take the one Rust implementation, which is GPL.
- §100 refused Linux's `lib/fonts/font_8x16.c`, GPL-2.0 on line one.
- Milestone 190 refused `lwext4`, whose extents and xattr files are GPLv2.

**Every one of those was a linked dependency. Not one was a separate program.** The tree learned
"avoid GPL" from cases where avoidance was right and has no recorded rule saying where it stops being
right, which is exactly the condition §46 was written to fix: a practice unanimous in effect and
written down nowhere.

**§87 (MIT OR Apache-2.0, and why the GPL's lesson does not transfer) does not cover this.** It
decides nife's own licence and why copyleft's strategy is not ours. It says nothing about running
GPL programs, and nothing in it should be read as refusing them.

## The delivery channel, which is calef's correction and the better answer

The first draft of this section said GPL programs ship in the image as aggregation. **That is legal
and it is not the right default**, because aggregation still makes nife the distributor, with every
obligation that follows: offering source for what we ship, and under GPLv3 supplying installation
information as well.

**A package manager moves the obligation to whoever runs the repository.** If nife does not
distribute the binary, nife has no GPL obligation for it, and the user obtains it from upstream by
their own act.

**The prior art is FreeBSD's, and it is both decisive and current.** FreeBSD spent years moving GPL
software out of the base system, replacing GCC with Clang and finally retiring `dialog` for
`bsddialog`, and **FreeBSD 16 completed the removal of all GPL code from its base system in July
2026**. Users did not lose anything: thousands of GPL applications remain available through ports and
binary packages. That is precisely the split proposed here, from the project whose documentation
standard this tree already follows.

So the rule is a **base and packages split**:

- **The image carries no GPL or LGPL.** Everything nife ships is permissive, which keeps `deny.toml`
  meaningful and keeps a downstream free to vendor the whole image.
- **GPL and LGPL programs arrive as packages**, installed by the user.
- **Milestone 47 already makes that shape natural**: a program namespace is an endowment, so
  installing a program is granting it into a namespace. The licence boundary and the capability
  boundary turn out to be the same boundary, which is worth noticing rather than arranging.

**The honest gap: the package manager does not exist.** Milestone 39 owns repository structure with
four options and no decision, `design/haiku-bfs-and-packages.md` owns the activation shape, and
`design/what-a-distribution-packages.md` is explicitly speculation. **Until one exists, the image is
the only channel there is**, so the near-term answer for `git` and `nano` is either aggregation with
its obligations accepted and recorded, or those milestones wait. This section does not decide which,
and that is the first thing calef should settle after the principle.

## GPLv3's replaceability clause, and the mechanism this system already has

The clause that makes GPLv3 painful for embedded vendors is the requirement that users be able to
replace the software on the device. It is why Samba is avoided by appliance makers, and it is the
reason a base-and-packages split matters more here than on a desktop.

**Where nife does ship a GPLv3 program, §41 is the mechanism that satisfies it by construction.**
Live component replacement makes the stable name an endpoint, and a swap is a change in who is parked
on it, with no forwarding process and zero steady-state cost. What GPLv3 asks a device vendor to
permit is a thing this architecture does as a feature. That is worth stating in the same breath,
because it converts a compliance burden into a demonstration.

## What this permits, what it does not change, and what it requires

**Permits.** GPL and LGPL programs as confined nife programs, holding only granted capabilities,
delivered by package.

**Does not change.** Linked dependencies stay permissive-only. `deny.toml`'s allow-list is untouched
and stays an allow-list, §83's prefer-Rust rule still governs crates, and §46 still governs whether a
dependency is taken at all. **A GPL crate in the shipping graph is refused exactly as firmly as
before**, and this section must never be cited to relax that. Milestone 190's `lwext4` refusal stands
for the same reason.

**Requires.**

1. **The image carries no copyleft**, and something should check it rather than a person remembering.
2. **Each packaged program's licence is recorded where a reader meets the program**, not in a registry.
3. **A GPLv3 program is shipped only where the replaceability answer is written down**, per above.
4. **The claim is stated with its limits**, the way this tree states benchmark ties: the boundary is
   the process, not the machine, and aggregation is a legal conclusion this project is not qualified
   to give as advice.

## What was refused

**Shipping GPL in the image as the default.** Legal, and it makes nife the distributor for no gain
once a package manager exists. Kept as the *interim* answer only because there is no package manager
yet, and marked as interim rather than allowed to become the design.

**Refusing GPL software outright**, which is where the tree's instinct was drifting. It would forfeit
`git`, `nano`, and most of the corpus milestone 123 (the demonstration: somebody else's software,
running narrow) needs, in exchange for nothing, since the process boundary already provides the
isolation the refusal would be buying.

**Using this to rescue Samba.** Samba's blocker is POSIX, not licence: `fork`, threads and a full
libc, which is tier three plus §105's declined shared-address-space threads. ksmbd is in-kernel and
so is the LKL problem. Licence isolation makes GPL software permissible; it does not make Samba
runnable, and nife's own SMB server remains the permissive one a downstream who cannot take GPL would
use.

## BUGS

- **This is not legal advice and nobody here is qualified to give it.** "Aggregation" is a conclusion
  drawn from how the FSF and every major distribution have treated separate programs for decades, not
  from counsel. Anyone shipping a product on nife should get their own.
- **The package manager does not exist**, so the rule's preferred channel is unavailable today and
  the section's own recommendation cannot yet be followed.
- **Nothing enforces requirement 1.** "The image carries no copyleft" is prose until a gate reads the
  manifest of what an image packs, which is rung three describing a rung two that should exist.
- **LGPL is treated as GPL throughout and they differ**, materially, for linking. A future case that
  wants to *link* LGPL rather than run it is not covered here and should not be decided by analogy.
- **The demonstration claim is untested.** No GPL program runs on nife today. Milestone 121
  (`ripgrep`, MIT/Unlicense) is not even a test of it, and the first real one is `git` or `nano`.
