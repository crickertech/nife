# The notes index is at its word cap

**Status: PROPOSED 2026-09-26.** Raised by the lane for milestone 23 (a capability-routed component
OS with live replacement), which added three notes and found `notes/README.md` could not index them
without going over §212 (a prose budget: 3,000 words of main body, with appendices under the same cap).

**Gate: NONE.** A restructuring of one index file, touching no code, wire format or dependency.

## The finding

`script/lint`'s markdown check requires every note to be linked from `notes/README.md`, and the
prose ratchet holds that file to 3,000 words because it is not in the baseline. On 2026-09-26 it was
at 2,997. Indexing three notes took trimming four unrelated entries to land at exactly 3,000, so
the next lane to add a note fails `script/lint` for a reason that has nothing to do with its work.

## What to build

Split the index by area into appendix pages, the parent-named sibling directory §212 already ratified
(`notes/README.md` beside `notes/README/`), or cut the per-entry glosses to a fixed short form. Either
way, leave headroom that a routine lane does not consume.
