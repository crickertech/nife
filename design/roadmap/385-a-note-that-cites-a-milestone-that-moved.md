# 385. When a milestone's status flips, tell the lane which notes cite it

**Status: NOT-STARTED.** Filed 2026-09-05 as an unnumbered proposal, written after six "is there a
milestone for X" questions in one evening turned up four things wrong on `main` rather than four
things missing; numbered 2026-09-19 by milestone 433's drain of the proposal pile. **Premise re-read
against the tree on 2026-09-19 and still true**: `script/citations` still takes `--check` and
`--untracked` and nothing else, so there is still no mode that reads a pull request's base commit,
notices a milestone's status moving, and prints the notes citing it. The instances have been
corrected, which is the whole point of the block rather than a reason to close it: milestone 99's
block no longer claims a compressor among the things this tree has or is building, and the other
three were fixed before the file was written. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** `script/citations` already parses milestone citations and CI already knows a pull
request's base commit. This is an addition to machinery that exists.

**Landed 2026-09-14, nine days late.** It was committed on 2026-09-05 to a maintainer branch that was
never pushed, while milestone 259 (sweep `notes/` for claims that stopped being true), minted and
built the same day as "the other half" of this proposal, cited this file by path from `main`. So for
nine days a `BUILT` block pointed at a file that did not exist, and no gate said so (see `BUGS`). It
surfaced during a branch cleanup that checked each stale branch for unique commits before deleting.

## The failure, with its four instances

Each was found by a person asking an unrelated question, not by any gate.

- **`notes/why-not-general-purpose.md`** told newcomers there was no networking, no writable
  filesystem, no display and no SMP, and that *"you cannot drop in existing software"*. Five of six
  rows false; milestone 121 ran unmodified `ripgrep` on 2026-08-31.
- **Milestone 66's gap table** said TCP listen and accept were *"absent from the contract"*. They
  shipped with milestone 107 on 2026-08-04. Its own gate line had been corrected on 2026-08-15 and
  the table had not.
- **Milestone 99's block** counts *"a compressor"* among the things *"this tree either has or is
  building"*. There is no compression code anywhere.
- **A TLS proposal written the same evening** listed milestone 198 as needing client-side TLS, which
  is probably wrong, and was corrected an hour later.

**Where the four stand on 2026-09-14.** Three are fixed: `notes/why-not-general-purpose.md` was
rewritten and now marks each old claim false, milestone 66's table no longer says listen and accept
are absent, and the TLS proposal was corrected that evening. **The fourth was still on `main`**:
milestone 99 still counted a compressor among things the tree has or is building, when nothing in the
tree compresses and milestone 258 (archive and compression) is `NOT-STARTED`. It is corrected in the
same pull request that lands this file, which is the proposal's own point made a fifth time: fixing
the instances does not fix the class, and one of four survived a sweep aimed at exactly this.

**Every one is rung four** in AGENTS.md's ladder: a note, a table, a gloss. Every one stayed green
through every gate. `notes/why-not-general-purpose.md` even asked in writing to be updated *"when a
subsystem that would move the needle... actually gets built"*, and two of the three were built.

## What to build

**When a pull request changes a milestone's status, print the notes that cite that milestone.** Not
a failure, a worklist, in the shape `script/names --unratified` already uses: it names work without
blocking anybody's build.

The lane flipping milestone N to `BUILT` is the one person in the project who knows what N now makes
true, and it is holding that knowledge at exactly the moment the check fires. Any later reader has
to reconstruct it.

## Four measurements, because they decided the design rather than decorating it

**1. A worklist is three notes, not thirty.** Across `notes/`, 194 milestones are cited by at least
one note; the median is **3 notes per milestone**, the mean 4.4, the maximum 20. Three files is a
thing a lane reads. That is what makes this viable at all.

**2. It must fire per pull request, not over the tree.** In the fourteen days to 2026-09-05, **68
milestone rows flipped to `BUILT`**, about five a day. Tree-wide at a median of three notes each,
that is fifteen note-reads a day owned by nobody, which is how a check becomes noise and then gets
ignored. Scoped to the pull request that moved the status, it is three reads by the person
responsible.

**3. It must key on the bare citation, and this is the one that would have broken it.** Of 1,519
milestone citations in `notes/`, **only 66 carry a gloss and 1,453 do not: 4%**. `script/citations`
checks the glossed form, which is right for what that script does and would find one citation in
twenty-five here. **This check reads `milestone N` and nothing more.**

**4. It catches one of the two real cases, and the one it misses is the worse one.** This is the
measurement worth arguing with rather than accepting:

| case | cites the milestone that moved? | caught? |
|---|---|---|
| Milestone 66's gap table | **yes**, `milestone 107` and `milestone 64`, inside the stale region | **yes** |
| `notes/why-not-general-purpose.md` | **no.** It cited milestone 7 and §4, §5, §10, §11, and none of 30, 33, 57 or 121 | **no** |

## The half it misses, stated plainly rather than glossed over

**A page that says "there is no networking" cites nothing, because there is nothing to cite.** That
is the shape of the worst instance found, and it is inherent: a negative claim has no anchor, so
there is no citation to hang a check on.

**Do not try to detect negative claims in prose.** AGENTS.md already prices that experiment: `git
grep -w TODO` ran at an 82% false-positive rate, and a lint that tried to tell an assertion from an
observation would be the same thing wearing different clothes.

**What the other half wants is a sweep, not a gate**, and there is precedent: milestone 247 swept
completed milestones for buried follow-on work, and this is the same exercise aimed at `notes/`.
That is a milestone somebody could mint and it is not this proposal.

**So this is worth building because it is cheap and it catches a real half**, and the proposal says
out loud that it is a half. A mechanism sold as covering the whole class would be worse than none,
because the next reader would trust the green.

## Where it goes

`script/citations` already parses milestone citations and has the index. The natural shape is a new
mode there rather than a new script, invoked from `script/lint` when a base commit is available, and
silent when one is not. **It never fails a build**, for the reason above: it cannot tell a stale
citation from a fine one, and a check that guesses at prose earns its own ignoring.

**The closest built relative is milestone 275** (a gate that diffs `design/fatal-risks.md` against the
roadmap it cites), which compares a status a file states about a milestone with the status the
roadmap records. That is the same comparison in its strict form, scoped to one file whose claims carry
a status word. This proposal is the loose form, over every note, and it lists rather than fails because
most citations in `notes/` carry no status word to compare.

## BUGS

- **It cannot tell staleness from correctness.** A note saying *"milestone 30 built the net stack"*
  is right forever and would be listed every time 30's row is touched. The output is a prompt to
  look, and if looking is usually wasted the check will be skipped, which is the failure mode to
  watch for after it ships.
- **It only sees `notes/`.** Milestone 66's stale table is in `design/roadmap/`, and 99's compressor
  claim is too. Both would want the same treatment and this proposal has not measured the citation
  density there.
- **A status can move without a pull request touching the index**, since the file and the row are
  two places, and §76 exists because they disagreed. This keys on whichever the implementation reads,
  and it should read both.
- **Nobody has built it**, so "cheap" is a judgment from the shape of `script/citations` rather than
  a measurement, and §92's test applies.
- **The gate that should have caught this file's absence covers only half the places a proposal is
  cited.** `script/roadmap` refuses a `**Proposed.**` entry under `## Follow-on` whose file does not
  exist. Milestone 259 cites this file in its status line instead, so that check never ran, and a
  backticked path that does not resolve passes every other gate: milestone 383,
  `design/roadmap/383-a-backticked-path-that-does-not-resolve.md`, is the general fix, and it is
  unbuilt too.

## Index row

Six "is there a milestone for X" questions in one evening turned up four things wrong on `main`
rather than four things missing, all the same shape: prose that was true when written and went
silently false when a milestone landed. `notes/why-not-general-purpose.md` told newcomers there was
no networking, no writable filesystem, no display and no SMP, five of six rows false; milestone 66's
gap table said TCP listen and accept were absent from the contract a month after milestone 107
shipped them. Every one is rung four of AGENTS.md's ladder and every one stayed green through every
gate. The mechanism is a worklist rather than a failure, in the shape `script/names --unratified`
already uses: when a pull request moves a milestone's status, print the notes that cite that
milestone, because the lane doing the flipping is the one person who knows what that milestone now
makes true and is holding that knowledge at exactly the moment the check fires. Four measurements
decided the design: the median milestone is cited by three notes, so a worklist is readable; 68
statuses flipped in fourteen days, so it must be scoped to a pull request rather than swept over the
tree; only 4% of the 1,519 milestone citations in `notes/` carry a gloss, so it keys on the bare
`milestone N` and not on what `script/citations` checks; and it catches one of the two real cases.
The half it misses is stated rather than glossed: a page saying "there is no networking" cites
nothing, so there is no anchor to hang a check on, and that half wants a sweep rather than a gate.
