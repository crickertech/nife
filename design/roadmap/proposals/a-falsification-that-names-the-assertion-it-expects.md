# A falsification that names the assertion it expects, and a gate that checks the transcript

**Status: PROPOSED 2026-09-16.** Found by milestone 307, which swept all 26 rows of
`notes/confinement-claims.md` asking which assertion actually fires when a claim is broken.

**Gate: DECISION.** It adds a field to the `Falsification:` block, whose spelling DECISIONS §134
ratified, so the field's name and its placement are calef's before they are anyone's.

**Promoted:** minted as **milestone 323** on 2026-09-18, as one of its parts rather than on its own.
calef promoted the cluster: this proposal and its siblings were each filed by a different lane
against the same surface, and answering them one at a time would have produced one brief per face of
a single finding. The record is
[design/roadmap/323-falsification-record-completeness.md](../323-falsification-record-completeness.md);
the status line above keeps its original date, because that is what makes the pile measurable, and
this file keeps its own argument, because the proposal is the argument as it stood and the milestone
is the account.
## In brief

`script/falsifications` checks that a recorded defect turns its harness or test **red**. Its own
`BUGS` is honest that it checks the red's *shape* rather than its sentence: it requires the kernel to
have booted and selected exactly one test before a non-zero exit counts, which stops a patch that
does not compile from reading as a successful falsification, and then it prints the panic line and
leaves the rest to a reader.

That gap has a cost with two recorded instances. Milestone 202's break of DECISIONS §31 surfaced as a
**234-second watchdog timeout reading "a livelock, not a lost wakeup"**: the right answer with a
diagnostic containing no word about confinement. Milestone 305 hit it again when a faithful kernel
defect for row 23 went red inside a helper at *"the supervision tree could not be built: stage 3"*,
and the lane swapped the patch rather than record a red for the wrong reason.

**Four patches in the tree already solve this in prose, correctly**, by stating which assertion the
defect is expected to fail and, in two cases, which nearby assertions stay green and why
(`an_oversized_batch_is_refused`, `a_deleted_capability_stays_deleted`, `split_never_widens_rights`,
`validate_and_shadow_confines_every_chain`). A reader meets that only by choosing to open a patch
file, which is rung four by `AGENTS.md`'s own ladder, and none of it is checked against what the run
actually printed.

## The shape

A line in the patch's prose head, beside `Falsifies` and `Architecture:`, naming the expected failure
site. The sweep then compares it against the transcript and fails a record whose red arrived
somewhere else. Provisional spelling only; the field name is the decision.

## The honest case against doing it first

**It would have caught none of milestone 307's findings**, and saying so is the point. An unreachable
assertion is invisible to this check: the patch's prose and the transcript simply agree on some
*other* line, which is exactly what row 24's record does today and exactly what row 12's tautology
did for as long as it existed. This closes the wrong-reason-red hole, which is a different hole from
the one 307 was looking down.

What it buys is that the next 202 is a gate failure rather than something somebody happens to notice,
and that the four patches already doing this by hand stop being the only ones.

## What is blocked until it is answered

Nothing. `script/falsifications`' `BUGS` carries the limitation where a reader meets the tool, and the
per-record prose convention works for whoever writes it. This is a promotion from record to
mechanism, in §71's sense, not a repair.
