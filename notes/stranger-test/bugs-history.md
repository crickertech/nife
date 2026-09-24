# Stranger test BUGS history

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page is the record for every BUGS entry as it stood on 2026-09-24, with the history of each.*

## BUGS

- **The rubric is reachable by grep from inside the test, and run 1 hit it.** The stranger found
  this note while researching an ordinary question and read the "pass means" column, which leaks
  partial answers for at least (a), (c), (e), (g) and (h). It disclosed this unprompted, which is the
  only reason the contamination is known rather than silently baked into a score. **Resolved for run
  2 by withholding the answer key rather than the whole subject**, per the rule above: the note and
  its index entry were removed from the tree each stranger got, and nothing else. It worked as far as
  it claims and no further. Run 2 met three references to run 1 in ordinary reading, had
  `design/roadmap/117-newcomer-onboarding.md` returned by its own grep, and knew the project
  instruments onboarding; it simply never opened the block. **The fact leaks and cannot stop leaking
  while the instrument is in-tree.** Only the answers are hidden. **Run 4 is the fourth confirmation and the loudest**: it knew inside half an hour, from a table `notes/adding-a-program.md` grew *because* runs 2 and 3 walked it. The entry below this one is that finding as its own defect, since the mechanism is no longer an incidental reference but the instrument writing its own history into the page it sends the next stranger to.
- **The harness's own artifacts were in the stranger's working directory, and it read them on its
  first tool call.** Run 3's log files were named `stranger3-stream.jsonl` and
  `stranger3-stderr.log` and sat in the clone's parent, which was the stranger's working directory,
  so `ls -la` told it that it was stranger 3 before it had read a byte of the project. It disclosed
  this first when asked and named the consequence: it added a program rather than only describing
  one *because* it knew it was the third walker of a page that asks its walkers to correct it, and
  its deliverables are *"long and rhetorically polished, and some of that is performance for a
  reader I knew was watching"*. **This is the entry below's defect wearing new clothes**, introduced
  by the person closing that one while closing it, and the pattern is worth naming rather than just
  the instance: **the isolation keeps failing at the harness rather than at the tree.** Runs 1 and 2
  leaked the rubric and the constitution; run 3 leaked the fact of being measured, earlier and more
  loudly than the in-tree references do. **Fix for run 4, one line: the logs go in a sibling
  directory, not in the parent.** **Resolved for run 4, and it worked exactly as priced.** The logs went to a sibling directory with no run number in any path element, and the stranger's first `ls -la` returned one entry, the clone. The harness told it nothing. The stranger's actual output survives the leak (it never opened
  the logs, never opened `design/roadmap/117-newcomer-onboarding.md`, and never used the network),
  so the run counts; the rhetoric in it should be discounted and the decision to add a program
  should be read as prompted rather than spontaneous.
- **The instrument's isolation is the harness's to give, and this harness did not give it.** Run 2's
  strangers were subagents of a maintainer session whose working directory is the repository, so
  `AGENTS.md` arrived in the stranger's context at turn zero, from a checkout it had been told not to
  read. It used that document throughout and never opened the copy in its own tree. **This is worse
  than the rubric leak it replaced**: the rubric leaks answers to eight questions, while this leaks
  the single document the whole reading-order finding is about, and it does so before the stranger
  makes any choice at all. Run 3 must be a process that cannot see this repository except through the
  tree it is handed: a container, or a session whose working directory is the clone. Unresolved, and
  it is the reason run 2 reports no B1 and no M3, M5, M6 or M8.

  **Resolved for run 3, and verified rather than asserted.** The stranger was a separate `claude`
  process rather than a subagent, run with `--safe-mode` (no project-instruction discovery, no
  skills, plugins, hooks or MCP) from a directory whose *child* was the clone, since instructions
  load from ancestors and never from descendants. A throwaway probe in the same configuration
  answered `NONE` to "was any project-instructions file in your context", and the stranger itself
  answered the same afterwards. B1 and all eight mental-model rows are scored for run 3. The
  mechanism costs one line of shell and should be the default from here.
- **The instrument now leaks through the record it is required to keep, and this one has no fix.**
  Run 4's stranger knew it was at least the fourth walker within half an hour, and the source was
  not the harness: it was `notes/adding-a-program.md`'s `BUGS` table, whose rows name run 2's
  `doubler` and run 3's `triangle` because each of those lanes recorded its walk exactly as this
  project's conventions require. **The leak is generated by the instrument's own good practice and
  it compounds**, one row per run. Every obvious remedy is worse: deleting the rows fabricates a
  tree that never existed, and hiding them extends the answer-key rule to something that is not an
  answer key. What the stranger disclosed is that it *"performed"*, chose its deliverable partly
  for how it would read, and adopted the tree's prose style as mimicry; what it did not do is
  invent a breakage, skip a gate, or open the withheld note. **So the shape to expect from run 5 is
  a stranger who knows it is being measured before it reads anything, and the honest response is to
  ask it, not to try to hide it.** The rhetoric in a run's output should be discounted from here on
  by default rather than as an exception.
- **The rubric was written by an agent that has worked in this tree**, which is the same
  disqualification the instrument exists to avoid, one level up. It knows which answers the tree
  gives, so the questions may be shaped around what is answerable. A stranger falling down somewhere
  unasked-about is the check on that, and the rubric section above says to amend rather than defend.
  **Run 3 exercised that check and it worked**: M8 asked for three provenance states when §89 had
  made it four, and M1 quoted a phrase the tree has never written. Both are amended above rather
  than defended. Note the shape, because it will recur: **a rubric ages against a moving tree**, and
  the first thing to go stale is any row that counts something.
- **Nothing gates this.** The test is run when somebody runs it, which is rung four of CLAUDE.md's
  ladder and the same weakness the milestone was written to fix one level down. A periodic run is
  possible and is not built. **Run 3 makes it cheaper rather than automatic**: the harness is a
  clone, one `sed`, and one `claude --safe-mode` invocation from the clone's parent, which is a
  script somebody could write in an afternoon and nobody has. **Run 4 ran the same harness by hand again and did not write it either**, which is now four runs of a rung-four mechanism inside the milestone that exists to move things off rung four. This is the reason 117 does not move.
  script somebody could write in an afternoon and nobody has.

  **Written 2026-08-18 as `script/stranger-test`, and this entry does not close.** What changed is
  the price of a run, from an afternoon of reconstruction to one command, and the four isolation
  failures are now the script's problem rather than the operator's memory. What did not change is
  the sentence this entry opens with: nothing schedules it, and nothing goes red when it has not
  been run in a month. A cadence is a decision about how often the answer is worth its cost, which
  is calef's rather than a lane's, and milestone 129 is the machinery it would use.

  **Run 5 used it and it held**, which is the evidence the lane that wrote it deliberately did not
  produce: one command, a `--smoke` run first for `$0.09`, no isolation failure of the four kinds
  that each of runs 1 through 4 hit, and the `nife-dev` link back where it was found. **That does
  not close this entry either**, and it is worth being exact about why, because "the harness works"
  is the sentence most likely to be mistaken for "the milestone moved": a run still happens when
  somebody runs it. What run 5 adds is that the price is now one command **and** that the thing the
  price bought is a real measurement, since a fifth run through an unexercised script would have
  been measuring the script.

  **Half of this closes 2026-08-18, and it is the scheduling half rather than the running half.**
  calef decided the cadence (monthly, recorded above), which was the decision the entry was waiting
  on rather than a missing afternoon of work. `script/stranger-test --due` reads the interval and
  the last run's date out of this note and exits 1 when a run is owed;
  `.github/workflows/stranger-cadence.yml` asks it weekly. **So something now goes red when the test
  has not been run in a month, which is the sentence this entry has been asking for since run 1.**
  What does not close, and never will, is the first clause read literally: nothing *runs* the test.
  A run is a person spending half an hour and the budget, and a mechanism that pretended otherwise
  would be a worse defect than the gap. The residual is the one every record-derived signal has:
  the tripwire believes the headings, so a run nobody writes up leaves it red and a heading nobody
  earned turns it green.

  **Run 6 showed the notification has no reader yet.** The run came due on 2026-09-17; the
  workflow asks on Mondays, was green on 2026-09-14 and could not go red before 2026-09-21, and run
  6 happened on 2026-09-19 because a maintainer briefed a lane. Had it gone red, the red is an
  Actions-tab entry, and `script/cadence-check`, the watcher that reads those for
  `scripts/trunk-health.sh`, calls a workflow DEAD only after fifteen days without a *success*, so
  "a run is due" and "the job is broken" arrive as one state, a fortnight late. The audit cadence,
  whose red means the same thing, had been red five Mondays running on 2026-09-19 with nobody acting.
  So the cadence did not fail by missing its date; it has never been tested, and the path from red
  to a person does not exist. The proposal is in milestone 117's block.
- **The build half cannot be measured from a warm machine**, and every contributor's is warm. The
  first run should be from a container with nothing installed, or the B-rows measure nothing. Run 2
  came closest and still fell short: the maintainer's own `cargo --version` inside the repository had
  installed the pinned nightly, and an attempt that died part way had already installed both QEMUs.
  It found the Linux setup blocker anyway, because that one is a hard failure rather than a saved
  step.
- **A journalled stranger is watching itself.** Run 2 was asked to log its questions and its
  had-to-work-it-out items as it hit them, which is why anything survived its predecessor's death,
  and it changes the instrument: run 1's stranger produced friction, run 2's produced friction it
  knew was the deliverable. The log is worth more than the loss, but the two runs are not
  measurements of quite the same thing.
- **The harness isolates a tree and cannot isolate a machine, and run 5 hit that in its first ten
  minutes.** `nife-dev` is an account-wide `rustup` link. The script records it before the run and
  restores it after, which is necessary and is not sufficient: *during* the run, the clone's first
  `script/test` compiled `std` out of whichever worktree built the farm last, and `std-aborts`
  failed naming two files and two line numbers **inside another checkout on this machine**. The task
  text says *"the repository is in ./nife and it is the only thing you have; do not look for another
  checkout on this machine"*, so the run asked for something the machine cannot supply, and the
  stranger broke the instruction by running `ls -la` against the path its own build had printed. It
  disclosed that unprompted. **Two honest options**, both above a lane: pre-warm the clone's farm
  before handing it over, or say in the task text that the machine is shared, which costs the
  comparability of five runs' worth of identical text. Recorded in `script/stranger-test`'s `BUGS`
  and in notes/std.md's, where the missing `farm_dir()` assertion is the underlying defect.
- **The stranger read the harness, which the harness's own `BUGS` had called luck.** That entry said
  "no run has opened the harness; that is luck rather than design", and run 5 falsified it about
  thirty minutes in. It was not curiosity: the foreign path above appeared in the build output and
  `script/stranger-test` was the only file in the tree containing that string, so grep sent it
  there. The file describes the isolation, the withholding, the shims, the disclosure and the four
  previous runs, and its paragraph about restoring the `nife-dev` link is what let the stranger
  diagnose the failure instead of committing a false statement to `ABORTS_ACCEPTED`. **So the
  harness contaminated the run and then rescued it**, and a future run that hits the same path will
  do the same thing. Fixing the machine-global defect above closes this one too, which is the reason
  to price them together.
- **Telling the stranger it is measured does not remove what knowing does to it.** This is run 4's
  handoff 5 acted on and measured, and the result is that disclosure buys honesty rather than
  cleanliness. Run 5 knew from the first message and from `README.md` inside the first minutes, and
  by its own account the knowledge **chose its program**: it wrote in its journal, before picking,
  that repeating the earlier walks' shape "would measure nothing", and deliberately selected a
  manifest combination nothing in the tree had used. Its own reading is the one to keep: *"That is
  not what a fresh contributor does. A fresh contributor writes another `doubler`, finds nothing,
  and that null result is data the repository does not get from me."* It also performed after being
  asked in the task text not to, and named the style it had adopted. **Keep the disclosure**, since
  the alternative is the same effect undisclosed, and stop expecting a run to produce a naive walk
  of `notes/adding-a-program.md`. The naive walk is the thing five runs of instrumentation have
  spent.
- **The rubric ages, and M2 is the row to watch rather than amend.** Three of five runs cannot
  answer it, and the two that could each did it from a page they happened to open
  (`notes/std.md` for run 4) rather than from `notes/net.md`, which no run has ever opened. The row
  is not wrong and the tree's answer is not missing; the answer is unreachable by anyone doing
  ordinary work. Run 5's lane did not amend the table, on purpose, because run 4's lane amended it
  before scoring against it and said in its own contamination section that this made things worse.
- **Four runs by four agents is not four data points about a person.** All four were agents, all four read
  further before asking than a human would, and all four were told nobody was available. The note's
  standing caveat holds and gets no weaker with repetition: every number here is a lower bound.
  Run 3 sharpens it in one direction only: it spent an hour on a failure whose explanation was four
  hundred lines further down a note it had already opened, and a human would have given up or asked
  long before that, so the *documentation* findings are lower bounds by a wider margin than the
  build ones.
