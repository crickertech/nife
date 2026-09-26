# A nightly bump restamps the icount floors when it proves it did not move them

**Status: PROPOSED 2026-09-26.** Raised by the lane `lane/bump-baselines`, briefed to stop the
daily toolchain bump from needing a person every day. The brief allowed a build; this is a proposal
instead, because every version of the fix that removes the person changes a ruling calef made on
2026-09-21. The title is provisional.

**Gate: DECISION.** calef's call: whether `# toolchain:` may mean "vetted against" rather than
"measured on", and the two numbers in option 4.

## The problem, measured

Since the stamp check landed (commit `99f13dad`, 2026-09-21), every bump that built has failed
`script/lint` until someone ran `script/bench --save` on three ISAs on the branch: #1112, #1250,
#1308, #1335. The branch is force-pushed daily, so a day the person misses loses the work. The
save itself admits what it cannot say: each `# why:` line on those saves reads "this mixes the
compiler with drift on main since the last save, not separated".

What #1335 (nightly-2026-09-26) showed, measured on patagonia on the bump branch:

- The compiler term is small and separable. The same tree built with nightly-2026-09-25 and
  nightly-2026-09-26: x86_64 identical on all 11 rows; aarch64 differs on 8 of 14, largest
  `spawn_el0` 0.09%; riscv64 differs on 9 of 15, largest `ipc_rtt` 0.21%. Repeated runs on one
  nightly are identical to the tick, so these are the compiler's.
- CI's host is a different instrument. CI's bench job on the same tree and nightly differs from
  patagonia on 5 aarch64 rows and 7 riscv64 rows, largest riscv64 `ipc_rtt` 170436 against 171468
  (0.6%). x86_64 matched exactly. The host term is larger than the compiler term.
- Milestone 300 (decompose the icount baseline drift, and re-baseline only what is proven) found the toolchain term across nightly-2026-08-27 to nightly-2026-09-15 was ~0.

## Options

1. Status quo. A person re-records daily. Keeps the ruling. Loses: the daily person, and the
   save blesses whatever drift sat on `main` into the floor, which is milestone 415 (sub-tripwire drift accumulates across baseline saves)
   happening on a schedule.
2. The bump workflow re-saves the floors in CI (the brief's suggestion). Refused on two counts.
   It is exactly what the 2026-09-21 ruling refuses: the floors would track the compiler, so a slower
   nightly could never be seen. And a CI save would record the floors on a host that reads up to
   0.6% differently from the one every other save used, so each bump would swap instruments.
3. Restamp only when the compiler term is zero. The workflow builds the tree twice (old pin, new
   pin) on one runner, and when every row is identical it rewrites `# toolchain:` and nothing else.
   Within the ruling, since no number moves and none could have. Loses: it would have handled only
   x86_64 today, so the person is still needed daily.
4. **Recommended: restamp when the compiler term is proven small, never re-save.** Same A/B as
   option 3. If every row's compiler term is within ε and the cumulative term since the last
   person-made save stays under a cap, the workflow rewrites `# toolchain:` and appends a `# why:`
   line with the A/B's largest moves. The numbers never change. Otherwise it leaves the branch red
   as today, with the A/B table in the pull request body so the person starts from the decomposition.
   Proposed ε 0.5% per row, cap 2% (a fifth of the 10% tripwire). Today's bump passes both.
5. Bump weekly. Composes with any of the above; cuts the person's cost sevenfold and makes each
   compiler step larger. The drift workflow still reports upstream breakage daily.

Option 4 keeps what the refusal protects. A slower nightly still uses up tripwire headroom against
numbers a person recorded, and the cap stops a run of small moves from being absorbed. Each restamp
records the term it proved, so the file tells a reader what the compiler has cost since the last
save. It also stops the bump from blessing `main`'s drift, which options 1 and 2 both do.

## What it changes, which is why it is a decision

- What `# toolchain:` means. Today: the nightly these counts were produced by (the comment in
  `xtask/src/bench.rs` says so). Under option 4: the nightly these counts were last proven valid
  for. The `# why:` line is what keeps that honest.
- Two numbers, ε and the cap. They are drift policy, which the brief reserved.
- `toolchain-bump.yml`'s header and `script/lint`'s comment both say re-saving stays a person's act.
  Option 4 does not re-save, but both texts would need rewording to say restamping is not re-saving.

## The seven questions

1. Considered: the five options above.
2. In the tree: `metrics/weekly` already commits generated numbers from a bot (#1333), with a person
   merging. Milestone 300 did the same A/B by hand.
3. Prior art: not researched. From memory only, rust-lang's rustc-perf compares one benchmark
   across two compiler builds, which is option 3's A/B.
4. Premise: true. Four consecutive bumps needed the person.
5. Cost: CI's bench step runs all three ISAs in about a minute (#1335, 12:47 to 12:48 UTC). Option 4
   adds a second toolchain build and a second bench run to the bump job, plus one cold QEMU build on
   its arm runner (CI's cache is keyed per arch, 10 to 20 minutes once). Not built, so not measured.
6. Reversible: a workflow, an `xtask bench` restamp mode and a lint comment. No floor number is
   written by the machine, so reverting leaves nothing to undo. Nobody has acted on it.
7. Equal cost: still option 4 over option 1, because it separates the compiler from `main`'s drift
   and today's saves say they cannot.

## What to build, if option 4 is taken

- `cargo xtask bench --restamp --why`: rewrite the header's `# toolchain:` and `# date:`, append the
  reason, keep every row.
- In `toolchain-bump.yml`, after the overlay builds: bench on the old pin, bench on the new pin,
  compare per row, restamp or not, commit on the bump branch as its own commit.
- The cap needs the cumulative term. The simplest record is the restamp's own `# why:` lines, summed
  since the last `--save`.
