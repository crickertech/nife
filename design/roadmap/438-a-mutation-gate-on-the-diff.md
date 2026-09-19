# 438. Would a diff-scoped mutation check have caught the 55, and what would it cost

**Status: NOT-STARTED.** Minted 2026-09-19 by calef, from the census that found `machine_discovery`
had gained 55 survivors two days earlier and nobody had noticed. *(Number provisional until the merge
queue lands it.)*

**Gate: NONE.** Everything it needs exists: `cargo-mutants` 27.1.0 is pinned in
`.cargo-mutants-version` and carries `-D, --in-diff <IN_DIFF>`, *"include only mutants in code touched
by this diff"*. The measurement replays history and never touches a live pull request, so it cannot
break anything and needs nobody's permission.

## What happened, and why a report was not enough

The census of 2026-09-19 found the tree's like-for-like mutation score had **fallen a point** in five
days, 93.6% to 92.6%, in a window that included 77 new tests. One crate accounted for it:
`crates/machine_discovery`, 22 survivors to **77**.

**They did not accumulate.** They arrived in one pull request, [#927](https://github.com/crickertech/nife/pull/927)
(merge `aa6a50b`, milestone 319, 2026-09-17), in the firmware-parsing code that milestone added. They
sat unnoticed for two days until a scheduled job happened to look. `design/fatal-risks.md`'s risk 3
is AMBER on exactly this shape: **the tree adds untested code faster than triage removes it**, which
is a rate problem, and a weekly report is the wrong instrument for a rate.

## Why this is not the gate milestone 85 refused

**Milestone 85 refused a full-corpus pull-request gate and its reason still holds**: *"a survivor is a
worklist entry, not a defect in whatever commit happened to precede the weekly cron."* A full run
blames whoever's pull request happens to be underneath it, and costs 52 minutes across eight runners.

**`--in-diff` has neither property.** It mutates only lines the diff touched, so every survivor it
reports **is in code that pull request wrote**, and the attribution is correct by construction rather
than by luck. The cost scales with the diff instead of the corpus.

**And 85's stated trigger is the wrong test for this gate**, which is worth saying because it would
otherwise read as the blocker. 85 says wait *"until the weekly numbers prove stable enough that a new
survivor deserves to fail something."* That is a condition about the **corpus rate**, and the two
censuses that exist disagree by a point, so by that test the wait never ends. A diff-scoped check does
not depend on corpus stability at all. This is the same defect risk 3's own green condition had: a
condition written for one quantity being applied to another.

## The three measurements, and the first one can kill the idea

1. **Does it catch the case we know?** Run `cargo mutants --in-diff` against **#927's own diff**
   (`git show aa6a50b`) and see whether it reports the `machine_discovery` survivors the census found.
   **This is the decisive measurement and it goes first.** If it surfaces them, the mechanism is
   proven against the exact failure it exists for rather than against a hypothetical. **If it does
   not, this milestone is over and the answer is no**, which is the result worth stating up front so
   the work cannot quietly become a search for a reason to ship.
2. **What does it cost?** Wall clock for `--in-diff` across a sample of recently merged pull requests,
   chosen to span the range: a documentation-only change, a single-crate change, and a parser or
   kernel change. Nobody has this number and the whole question turns on it, because a check that
   costs minutes on an ordinary diff cannot sit on a pull request whatever it catches.
3. **What is the friction, in cases rather than adjectives?** Over that same sample: how many pull
   requests would have failed, and for each survivor, whether it is a **real missing test** or an
   **equivalent mutant** whose author would have had to write an exclusion with a reason. If most
   failures are real gaps the gate is doing its job; if most are equivalences it is a toll booth, and
   the ratio is the number that decides which.

## The constraint, stated so it is not rediscovered

**It ships as a blocking gate or not at all.** calef, 2026-09-19: *"Advisory gates don't seem to work
for us."* [§97](../decisions/97-advisory-checks.md) decided this on 2026-08-25 and its own `BUGS`
predicted the failure in as many words, *"a check added to CI is advisory by default, so the list
grows silently"*; `image-permissions` is the seventh check sitting in that state and milestone 340 is
the block about it. So an advisory period is not the safe default here, it is the known failure mode,
and this milestone does not propose one.

**This milestone does not switch anything on.** It produces three numbers and a recommendation. The
ruling is calef's, because a gate on every pull request changes what contributing costs, which is the
irreversible half.

## BUGS

- **A replay is not a live run.** `--in-diff` against a merged diff sees the code as it landed, where
  a real gate would see it as proposed, and a pull request that was amended during review differs
  from its merge. The sample should note where that applies rather than pretend it does not.
- **The sample is small by construction**, because each run costs real time and the point is to
  decide, not to publish a rate. Three or four pull requests spanning the range is the intent; a
  number computed from them is an order of magnitude, not a statistic.
- **`--in-diff` inherits every exclusion in `.cargo/mutants.toml`**, including the ones added on
  2026-09-19 for `**/src/proofs.rs` and `interleavings::`. A pull request that adds a new Kani
  harness in a shape those globs do not match would fail the gate on its own proofs, which is the
  defect that inflated `timetable` arriving from the other side.
- **It cannot catch an untested line the diff does not touch.** A pull request that adds a caller for
  existing untested code leaves that code exactly as untested and passes. The weekly census stays the
  instrument for the standing corpus; this is only about the derivative.

## Index row

The census of 2026-09-19 found `machine_discovery` had gained 55 mutation survivors in a single pull
request two days earlier and nobody noticed until a scheduled job looked, which is
`design/fatal-risks.md` risk 3's amber in one instance: the tree adds untested code faster than triage
removes it. Milestone 85 refused a full-corpus gate for a reason that still holds, and the pinned
`cargo-mutants` already carries `--in-diff`, which mutates only the lines a diff touched and so blames
correctly by construction. This measures three things against history rather than on live pull
requests: whether replaying #927's diff surfaces the 55 (which goes first, because a no ends the
milestone), what the check costs on diffs spanning documentation to parser, and how many failures
would be real gaps against equivalent mutants somebody has to exclude. It ships as a blocking gate or
not at all, per §97 and calef's ruling that advisory gates do not work here.
