# Which model wrote it

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

## Which model wrote it

**The third flow on this page**, and it answers a question the first two cannot. Velocity says how
many milestones landed, the pull-request series says how much landed at all, and this says **who
wrote it**. calef asked for it on 2026-09-24. Every column is a per-week event, like the two above
and unlike every stock on this page.

### Why the trailer, and not the richer source that was right there

The agent harness keeps session records with **tokens per model**, which is a better measure of
effort than anything below. calef rejected them as the spine of this series on 2026-09-24 and the
reasons are structural rather than a matter of taste:

- **They are not in git.** They live under `~/.claude/projects/` on one laptop and nothing promises
  to keep them. `script/effort`'s own header already records 2026W29 through 2026W33 as
  unrecoverable, which is five weeks of this project gone.
- **They cannot backfill.** Git can, and this series reaches the first commit because of it.
- **They see one machine.** That is not hypothetical: `Claude Fable 5` signs **360 commits** in this
  repository and appears in **no session record** on the machine those records live on.
- **`script/metrics` may not read anything but git**, by the rule at the top of the script, because
  `.github/workflows/metrics.yml` runs it on a GitHub runner where none of that exists. A
  trailer-based series can live in `script/metrics`. A token-based one cannot, which is why the cost
  columns further down are captured by a person on patagonia and have the gaps to show for it.

So this series is cheaper, coarser, and it will still be here in a year. The cost section below is
the richer measurement, and its "what was already gone when the capture started" subsection is the
argument for this one.

### Three buckets, and the sum is checked

Every commit in the repository falls in exactly one of three, and `script/metrics` **fails loudly**
if they do not add up to the week's whole commit count:

- **attributed**, carrying a `Co-Authored-By` trailer, counted per model.
- **unattributed**, authored with no trailer.
- **merge**, created by the platform rather than by anyone.

**The balance is the point rather than a tidiness.** A sum that must come out right is a denominator
that cannot leak: if normalization drops a model, if a new model name is swallowed, if a
classification rule is wrong, the total stops matching and the script stops. Every other miscount
this tree has written up was a count over a set nobody could see, from the status token that was
worth seven invisible milestones for five days to the naming surface that answered confidently about
a kind it did not walk. This one is auditable by arithmetic, by anyone, from the CSV.

**A merge commit is a pull request landing, and that is worth having on its own terms rather than as
leftovers.** `allow_squash_merge` is `false` on this repository and the merge queue creates the
merge commits, so the merge bucket is a throughput count. 95 merge commits do carry a trailer, from
an agent resolving a conflict, and the merge bucket still wins: what the commit records is a
landing, not a piece of writing.

**Merges carry a commit count and no line count.** A merge's diff against its first parent is the
whole of what landed on the branch, every line of which is already counted on that branch's own
commits, so adding it would roughly double the tree.

### Normalization collapses a variant, never a version

`Claude Opus 5 (1M context)` and `Claude Opus 5` are one model reached through two context windows,
so the trailing parenthetical is stripped and they share a column. `Claude Fable 5.1` keeps its own
column, because a version is a different model and folding it into `Claude Fable 5` would be exactly
the silent merge the table exists to prevent.

**A trailer value the table has never seen is counted in `other_models` and printed by name on every
run.** It is not dropped and it does not quietly become an existing model. `script/metrics`' own
`MODEL_TRAILERS` records all seven spellings this repository has ever carried, checked against all
5,487 commits on 2026-09-24, `Claude Code` included: that one signs a single commit, is the tool
rather than a model, and is in the table as a decision so it does not warn forever.

**A commit carrying two trailers is counted once, under the first, and named on stderr.** Counting
it twice would break the invariant above and splitting it in half would put a fraction in a column
of counts. No commit in this repository carries two, measured the same day, so the rule has never
fired and the warning is what would tell us it started to.

### Unattributed hides two different facts, and only one of them is a gap

A commit made before the first trailer ever appeared **could not** have carried one. A commit made
after it and carrying none is a hole in the record. They are separate columns:
`model_commits_pre_convention` and `model_commits_unattributed`.

**The convention starts 2026-07-23 UTC**, at `16a5a5c97e40`, signed `Claude Opus 4.8`, ten days
after the first commit. 79 non-merge commits precede it, 61 of them in 2026W29 and 18 in 2026W30.
**2026W29 is therefore absent rather than zero** in every model column, the same rule milestone
519's cost columns follow, and `notes/project-metrics/models.csv` carries empty cells there rather
than noughts. It has already left the ten-week chart window, so the marker is a fact about the file
today rather than something a reader will see drawn.

**The post-convention gap is real and it moves.** 832 commits since the convention began carry no
trailer, and they are not spread evenly: 2026W34 has 295 of them against 16 in 2026W37. This column
is the honest denominator under every share above it, and it is on the chart for that reason. A
reader who cannot see the unattributed band will read the attributed shares as the whole picture.

### Lines are volume, not effort

![Lines touched each week, by the model that signed them](../project-metrics/models-lines.svg)

**Added plus removed, the churn, not the net.** Net is the wrong quantity twice over: a commit that
only deletes reads negative, and a careful mechanical sweep can net out near nothing while being the
largest diff of the week. The tree's net size already has a series of its own ("Rust in the tree"
below), which is a stock; this is a flow, and what flows is how much text was touched.

**And the ranking it produces is backwards, which this project can demonstrate from its own history
one day before the chart was built.** On 2026-09-23 pull request #1168 was the hardest work of the
night, a silently dropped table cell diagnosed, the renderer separated from the note, the obvious
fix found to break `mdr`'s memory grant on x86_64 only, and the gate deliberately falsified, and it
changed about a dozen lines; pull request #1171 changed **246 files** in a careful mechanical sweep.
`AGENTS.md` says the same of its own line count and the posture is the one to carry here: **take the
number as a scale, never as a claim about correctness.**

**Commits are the better unit of the two**, and the reason is a property of this repository rather
than a preference. `AGENTS.md` requires one purpose per commit, and `allow_squash_merge` is `false`,
so a commit here is an enforced unit of intent rather than an arbitrary chunk of diff. The
lane-hygiene caveat two sections up still applies to it; it applies to the line chart as well, and
harder.

### What this series cannot tell you

**It attributes a commit to the model named in its trailer, and nothing checks that.** The trailer is
written by the harness, so it is good evidence and it is not a measurement. A commit whose work was
done by one model and committed in a session running another is attributed to the second, and
nothing here can see that.

**It says nothing about difficulty, and the models are not interchangeable in what they were given.**
A week where one model's band dominates may mean it did most of the work or that the maintainer
pointed it at most of the lanes. Lane assignment is a person's choice and it is not in git.
