# Name a customer, or admit the ranking function has nothing to rank

**Status: PROPOSED 2026-09-21.** Raised by the maintainer, from calef's question about what nife
would have to be to change anyone else's behaviour, and from the fact that every honest answer began
with "a customer".

**Gate: DECISION.** Nobody but calef can name one, and no lane can substitute for it.

## The hole this names

`AGENTS.md`'s first principle ranks work by the shortest path to a system a customer runs, and then
says plainly that **the path is vacant**. It has been since 2026-08-30, when the family's backups
went to borg over SSH on cordoba and the premise of milestone 55 (Time Machine over SMB3) went with it. The file is clear that
this is the principle working rather than failing: a customer with a real deadline went elsewhere,
early, which is exactly what the principle exists to surface.

**But a ranking function with nothing to rank stops ranking.** The tie now breaks toward
`design/fatal-risks.md`, which the file itself calls a stand-in rather than a replacement. Ten weeks
on, the visible effect is real: this project is very good at the work that can be justified without a
user, and the roadmap has grown to 515 milestones.

## What the first customer has to be, from what the last one taught

The recorded lesson is precise and it is about size, not enthusiasm: *"A first customer should be
something nife can plausibly be adequate at within a milestone or two."* The family backup server
was among the largest things a home system can be asked to be: somebody else's filesystem, a
network protocol, crash consistency, and the only copy of irreplaceable data.

So a candidate is worth naming only if:

- **It is adequate-able within a milestone or two**, on evidence rather than optimism.
- **Somebody notices when it breaks**, which is the only test that cannot be gamed.
- **Failure is survivable.** Not the only copy of anything, not anyone's livelihood.
- **It exercises something the demonstrator claims.** A workload that avoids isolation, drivers and
  real hardware proves nothing about this thesis.

## The candidates already in the tree

Named so the decision starts from what exists rather than from a blank page. None is recommended
here; the ordering is the maintainer's reading of adequacy distance, closest first.

- **A machine that runs one thing on real silicon.** xenon boots nife from firmware today; radon
  boots and has been a bench target for weeks. A single long-running service on a board that nobody
  needs back is the smallest honest customer this project could have.
- **The package manager's own path.** DECISIONS §157 (a trivial install is a web page, a USB drive
  and packages) defines an install story, and §203 (capacity is rented rather than bought) has just
  made rented machines part of how this project works. A nife machine that serves this project's own
  builds is a customer whose complaints arrive as our own broken builds.
- **A measurement appliance.** The benchmarking work already runs real workloads on three
  architectures; a box whose job is to produce numbers on a schedule is a workload nife is close to
  adequate at, and its output is evidence the demonstrator needs anyway.
- **Something outside this project entirely**, which is the only candidate that tests principle 1's
  distinction between "the architect runs it" and "a customer runs it". `AGENTS.md` records the
  precondition calef set for that: a package manager and a trivial install must exist before a
  second customer can be accepted at all.

## What is being asked

**Name one, or record that the path stays vacant and why.** Both are answers. What the tree should
not keep doing is carrying a ranking function whose top entry is blank, because that quietly hands
the ordering to whatever is most interesting that week.

If one is named, the roadmap owes it a milestone and the fatal-risks file owes risk 8 an update,
since its current text ends with the path vacant.
