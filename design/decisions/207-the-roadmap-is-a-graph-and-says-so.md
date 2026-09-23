# 207. The roadmap is a graph, and the block says so in fields a script can walk

**Status: PROPOSED.** Raised by calef, 2026-09-22, after a week in which milestone promotion kept
going wrong: *"I don't quite get why promotions are so hard. This has been killing us for weeks."*
and then, once numbering turned out not to be the cause, *"Even more important is expressing
dependencies between milestones so that we execute the graph of milestones efficiently."*
*(Section number provisional until the merge queue lands it.)*

## What is being decided

Four things, together, because each one is wrong on its own:

1. That a milestone block states its dependencies in **two fields**, on milestones and on decisions.
2. That a dependency is **never cleared when it resolves**, and blocked-ness is computed instead.
3. That a machine requirement is stated as a **capability** rather than as a host name, with a
   separate field for the rare case where one specific machine is the point.
4. That **`**Gate:**` is retired**, because the three fields above absorb every use it has.

## The measurement that prompted it

Taken 2026-09-22, across 547 milestone files:

| Gate | Blocks |
|---|---|
| `NONE` | 156 |
| `DECISION` | 62 |
| `HARDWARE` | 26 |
| `MILESTONE N` | 15 |

**Fifteen dependency edges across 547 nodes, and 288 blocks with no gate line at all.** There is no
graph to execute. There is a list with a handful of arrows, and the ordering that results is the
number a milestone happened to be minted with, which encodes allocation order and nothing else.

## Why dependencies must not be cleared when they resolve

**Today's gate clears, and the clearing is enforced.** `script/roadmap --check` fails when a
`Gate: MILESTONE N` names a milestone that has since become BUILT. That is how the promotion lane
found a stale gate on 2026-09-22, on a block whose named dependency had landed weeks earlier.

**That rule is the defect, and it is a live source of the merge pain this project has been
absorbing.** It means a milestone landing on `main` obliges an edit in every other block that
depended on it, made by somebody in a different lane who is not looking at that file. So a
dependency being satisfied turns unrelated branches red, on a line those branches never touched.
The tree already knows this hurts: milestone 385 (*when a milestone's status flips, tell the lane
which notes cite it*) exists because of exactly this coupling.

So:

- **A dependency is a fact about the work and stays true forever.** That milestone 554 (a good upgrade sticks: what marks a trial boot successful) needed
  milestone 525 (a bad upgrade cannot brick the machine: two boot slots, tries and priority) before it could exist does not stop being true once 525 is built.
- **Blocked is a question, not a stored field.** A milestone is blocked when any dependency it names
  is unresolved, computed at read time from the target's own status, which is recorded in exactly
  one place already.
- **Nothing needs editing when a dependency lands.** The only thing that changed is the target's
  status, and the lane that built it already updated that.
- **The history survives**, so "what was on the critical path to the install story" stays answerable
  after the fact, which deleting edges makes impossible.

The cost is that a block lists dependencies long since met, which reads as clutter. **That is a
display problem, not a data problem**: `script/roadmap` renders met dependencies struck through or
behind a flag. Cheap, and reversible, which is the right side of the *move fast on what can be
undone* line.

## The fields

Every milestone block carries all five. **A missing field is a defect rather than an assertion**, so
`none` is written out and silence never means anything.

    **Milestone dependencies:** 527, 543
    **Decision dependencies:** §92, §150   (a caretaker is supervised by the client it serves;
                                             how a thread's CPU time reaches userspace)
    **Machine requirements:** riscv64 silicon; PMU cycle counter
    **Specific machine:** none
    **Needs a person:** no

The first two are the graph. The last three are the machine model below.

## Why the machine model is three fields and not one

**`Gate: HARDWARE` conflates four different things**, found by reading all 26 of them, and the
dominant one is not hardware:

- **A person physically at a machine.** The most common by far, phrased as *"the board is on the
  desk and this needs hands on it."* The hardware is not missing. **calef's attention is**, which
  this project calls its scarcest resource and which a field named `HARDWARE` hides from every count.
- **A capability an emulator cannot provide**, such as *"only a core with a genuinely ASID-tagged
  TLB"* or a real cache. Any host with the property will do, which is the substitution case.
- **One specific machine**, because a comparison is only valid like-for-like. §203 (capacity is rented rather than bought) states it: renting
  gives you *a* machine, and the point is *the same* machine as the reference. argon against seL4,
  and xenon's fastpath footprint keyed to its Core i5-7500T.
- **An external account or rented resource** that is not a machine this project owns.

So a host name is the wrong thing to write down. **radon is a satisfier of "riscv64 silicon", not
the requirement itself**, and §203 already priced a Scaleway RV1 that satisfies the same
requirement on different silicon. Naming the host forecloses that substitution in prose nobody
re-reads.

`**Specific machine:**` therefore takes a host **and a reason**, and having to write the reason is
the point: it is the expensive case, and an unreasoned entry is the failure mode.

**`**Needs a person:**` is the field that earns this section.** It makes one query possible that
cannot be asked today and that §203 explicitly left open: *which milestones would rented metal
unblock, and how many bench-hours would it buy back.* That is the number that should decide §203's
undecided spend split between inference, runners and bare metal.

## What this replaces

`**Gate:**` is retired. `DECISION` becomes the decision field, `MILESTONE N` becomes the milestone
field, `HARDWARE` splits across the three machine fields, and `NONE` becomes every field saying
`none`. Nothing it expressed is lost, and three things it could not express are gained: more than
one dependency, a decision and a milestone at once, and the difference between a missing machine and
a missing person.

## Scope, so this is not a 547-file project

**Backfill the ready set first**, which is 145 milestones and is where execution order actually
bites. The remaining 400 backfill lazily, when a lane next touches the block. A block with no
dependency fields is legal during the migration and reported by `script/roadmap --unmodelled`, so
the debt is visible rather than assumed away.

## What it buys, in the order the payoff arrives

1. **A real ready queue.** `--unblocked` means every dependency resolved, rather than `NOT-STARTED`,
   which today returns 145 items and is useless as a worklist.
2. **Ranking falls out of the graph** instead of being built separately: depth toward a goal orders
   the queue, and `design/fatal-risks.md` supplies the tie-break `AGENTS.md` already names.
3. **Independent subtrees name the lanes that cannot collide**, which is the constraint `AGENTS.md`
   says actually bounds lane count: not queue depth, but *"what files will this lane touch, and who
   else is in them."*

## What is blocked until this is answered

Nothing is blocked, and nothing should be built before it is ratified. The field names are calef's
call under this tree's naming authority, and the block format is something every lane and several
scripts agree on, which puts it in *anything two programs agree on* and therefore in the expensive
category.

## What would refute it

If the backfill of the 145 ready milestones finds that most have no dependencies worth stating, the
graph is sparse by nature rather than by neglect, and this buys ordering nobody needed. The backfill
should report the edge count it found, and a figure near today's 15 refutes the section.
