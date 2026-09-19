# Refusals written where `script/names` cannot read them

**Status: PROPOSED 2026-09-18.** Found by the maintainer while recording calef's rulings, by
noticing that the refusal count went **down** by three after a commit that added one.

**Gate: NONE.** It is a gate and a sweep, and needs no hardware and no decision to start.

## In brief

`script/names`' own header states the rule: **"A block runs from its `Name:` line to the next empty
comment line, so it may wrap."** So a refusal written after a blank `//!` line is in the file, is
read by a human, and is **invisible to the tool**. Milestone 115's whole purpose is that a refused
name is findable by the person about to propose it again, and a refusal the tool cannot see does not
serve that purpose.

**Verified, not inferred:**

- `crates/screen_console` records four refusals (`framebuffer_console`, `pixel_console`,
  `early_console`, `video_terminal`) in paragraphs after a blank `//!` line.
  `script/names --refused | grep -c screen_console` answers **0**.
- On 2026-09-18 a maintainer edit to `crates/capability_demo_protocol` moved three previously
  visible refusals into a later paragraph. The tree-wide count fell from 171 to 168, exactly three,
  which is how the problem was found at all. (That crate is `crates/capability_witness_protocol`
  now, renamed later the same day; the old name stands here and in Scope below because both
  passages are dated accounts of measurements taken under it.)
- Restoring those three and pulling seventeen more of that day's rulings into the parsed paragraph
  took the count from 168 to **188**.

**This was found by arithmetic, not by a gate**, and only because the number happened to move in the
wrong direction in a session that was watching it. Nothing would have caught the same mistake in a
lane's commit.

## Why it matters more than a count

The failure is silent in the direction that hurts. A block with hidden refusals still **looks**
complete to a reader opening the file, and `script/names <name>` still answers for the name itself,
so nothing about the record looks wrong. What breaks is precisely the query milestone 115 exists to
serve: *has this name been refused before?* AGENTS.md's worked example for that is milestone 63
having already refused `system_builder` for a reason nobody could find.

There is a second, sharper case: **a name that is refused and still live**. `script/names` reports
those as a NOTE, which is how a reader learns that `pci` is refused somewhere and used anyway, or
that `ipc` is queued for a rename. A hidden refusal drops the name out of that list, so the queue of
outstanding renames silently under-reports.

## The options

1. **Fix the blocks, leave the parser.** Put every refusal in the first paragraph. It is what the
   header already documents, and the 2026-09-18 pass did this for ten blocks. Cheapest, and it keeps
   the parser's simple rule. Costs: nothing prevents the next block from doing it again, and long
   first paragraphs are harder to read than a paragraph per argument.
2. **Widen the parser** to read to the end of the doc comment rather than to the first blank `//!`
   line. Fixes every existing block at once with no sweep. Costs: the blank-line rule exists so a
   `Name:` block does not swallow unrelated prose that follows it, and several files put ordinary
   documentation after the name block. This needs checking before it is chosen, not assumed.
3. **A `script/lint` check**, and this is the one that holds either way: fail when the word
   `Refused` appears in a file **outside** the block `script/names` parses. It is a mechanical
   question with a mechanical answer, it is rung two where the header's sentence is rung four, and
   it would have caught this on the commit that introduced it.

**Recommendation: 3, plus whichever of 1 or 2 the check's first run argues for.** The gate is the
part that matters, because the bug is not that ten blocks were wrong; it is that nothing noticed for
as long as the convention has existed. Option 2 is attractive and may well be right, but it is a
change to how every name block is read and it needs the survey in option 3's first run to price it.

## Scope

Ten blocks were corrected on 2026-09-18 (`nvme`, `gpt`, `dtb`, `ipc`, `asid`, `pci`, `elf`,
`nvme_server`, `address_space_builder`, `capability_demo_protocol`). **`screen_console` and
`board_console` are known to still be wrong** and were deliberately left, because a naming sweep on
the same evening as ten rulings is how a cheap edit destroys an expensive record. A lane should
survey the whole tree rather than trust that list.

Two of those ten names have since been replaced by the rulings the corrected blocks recorded:
`capability_demo_protocol` is `capability_witness_protocol` and `address_space_builder` is
`address_space_witness`. The list above is an account of which blocks were edited that day, under
the names they carried then, so it keeps them.
