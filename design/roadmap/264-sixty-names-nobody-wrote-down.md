# 264. Sixty names the history cannot justify, and the research that would let calef rule on them

**Status: NOT-STARTED.** Minted 2026-09-05 by calef, after measuring the naming backlog rather than
working through it. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Reading the history of names that already exist. Nothing waits on it and it blocks
nothing, which is `script/names --unratified`'s own design: a worklist rather than a wall, so an
unratified name never fails anybody's build.

## The measurement that scoped it

`script/names --unratified`, 2026-09-05: **100 unratified of 204**, and the tool already sorts them
into two kinds that are two different jobs.

| | provisional | **unrecorded** | recorded |
|---|---|---|---|
| programs | 11 | **22** | 3 |
| crates | 10 | **8** | 2 |
| packages | 0 | **5** | 1 |
| scripts | 10 | **25** | 3 |
| | 31 | **60** | 9 |

**A provisional name is prepared and a ratification is a read.** A lane coined it, argued it, and
recorded its refusals, so calef weighs an argument. Two were ratified on 2026-09-05 in one question
each (`script/board-netboot` and `bench/xenon-netboot/`), and both went the lane's way because the
argument arrived with the name.

**An unrecorded name has no argument at all.** `script/names <name>` prints what the history does and
does not say, and for these it says nothing usable. **Bringing calef a name with no argument is what
AGENTS.md refuses** in "a fork reaches calef with its questions already answered": his attention is
the scarcest thing here, and spending it on lookups anybody could run is the failure that section
exists to name.

**This milestone is those lookups.** It converts 60 unanswerable names into recorded ones plus a
small set of genuinely open questions, each arriving with its options.

## What to produce, per name

**A provenance block where a reader meets the name**, which is milestone 115's mechanism and the
shape `script/names` already checks for. `script/board-netboot`'s header is the worked example: what
the name claims, why, and **what was refused and why each lost**, because "the refusals are the
valuable half" is that script's own line.

Three outcomes, and the split should be stated per name rather than assumed:

- **`recorded`**, where the history justifies it and nobody had written the sentence down. **A large
  share of the 60 is this**, and it is nearly free: `wc`, `rm`, `date`, `login`, `clock`, `entropy`,
  `ntp`, `hello` and `doc` are terms a reader knows from outside this project, which AGENTS.md calls
  the best names available. The acronym rule settled 2026-09-05 governs them, and where an acronym
  survives, the block says why its expansion teaches nothing.
- **`provisional`**, where the history does not justify it and the lane has a case to make. `outlaw`,
  `chatty`, `flaky`, `heeder`, `spinner`, `soaker` and `budgeter` are coinages nobody argued for in
  writing. **Do not invent a justification for a name that has none**: say what the history says,
  propose one, and let calef rule.
- **A rename proposed**, where the research shows the name is wrong. Propose it; do not do it.
  Renaming is a naming decision with extra steps and is calef's.

## Two rules that govern this and were both set the same day

**An acronym is spelled out unless its expansion teaches nothing** (AGENTS.md's naming section,
2026-09-05). Several of these will turn on it, and `dma`, `dtb`, `gpt`, `ipc` and `asid` were
deratified by that rule, so a name carrying one of them needs the question asked rather than assumed.

**And the seventh question applies to a rename as much as to a fork**: would we still choose this if
both options cost the same? A name kept because renaming is work is an effort argument, and it has to
say so in those words.

## What this milestone is not

**It is not a renaming sweep.** The deliverable is the record, and a rename is an outcome the record
may recommend. Milestone 63 did the renaming; this writes down why the survivors survived.

**And it does not touch the 31 provisional ones.** Those are calef's to ratify and are already
prepared; adding to their argument is not this lane's business.

## The proof that this milestone worked

**`script/names --unratified` reports zero unrecorded**, with every one of the 60 either recorded
with its reason or moved to provisional with a case. The count is the deliverable and it is
mechanically checkable, which is unusual for a documentation milestone and worth having.

## BUGS

- **A recorded name is not a ratified one**, and this milestone deliberately does not reduce what
  calef owes. It changes 60 unanswerable questions into a smaller number of answerable ones, which
  is the whole of the claim.
- **Some of the 60 will have no discoverable reason.** A name coined in a lane that recorded nothing
  leaves a history that says only when it appeared. Saying so is the honest outcome and is what
  `unrecorded`'s own vocabulary is for; a plausible reconstruction presented as history would be
  worse than the gap.
- **`script/names` sorts by exposure and this block does not argue with that ordering.** Programs a
  person types come before scripts, and scripts before crates, which is the tool's judgment rather
  than a measured one.
- **It touches ~60 files across `user/src/`, `crates/`, `script/` and `Cargo.toml`s**, so it collides
  with almost anything. It should run when the tree is quiet, the same scheduling constraint
  milestone 91 carries.
