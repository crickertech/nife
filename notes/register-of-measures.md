# The register of measures: every number this kernel owes itself

*(Milestone 134. The name `register-of-measures.md` is **provisional**; naming is calef's, and a
lane ships a provisional one and says so.)*

This tree measures a great deal and remembers almost none of it. A number gets taken once, written
into a note beside the reasoning that needed it, and then sits there being true on the day it was
written. `notes/counted-claims.md` found three such numbers on 2026-08-14 and **all three were
wrong**; every one had been right when somebody typed it.

That convention fixed the class of number a `grep` can re-derive. This register is the other half:
the numbers that need an instrument, a boot, or a walk over the source. It says which ones this
kernel is holding itself to, which ones it merely knows, and which ones it has defined and cannot
yet take.

**And `notes/project-metrics.md` is the half that moves.** This register says which numbers are owed
and what each one is held to; that page plots the ones that change every week, one row per ISO week,
recomputed from git history by `script/metrics` so nothing here has to be remembered. Several
measures below appear there as a series: the `unsafe` density and its ceiling, the Kani harness
count, the markdown corpus. A reader who finds either file should find the other.

**Since 2026-09-24 that page is a deck and this file holds its prose.** Each chart there carries one
line; the definitions, the arguments, the capture deadlines and the reconciliations are in *The
weekly series* below.

## What belongs here, and the test

**A number belongs if something depends on its value and it can move without anybody editing it.**

Both halves are load-bearing and the second is the one that cuts.

- *Something depends on its value.* Not "somebody would find it interesting". A decision rests on
  it, a constant is sized against it, a claim in the documentation quotes it, or a customer notices
  when it moves. `documentation::render::LINE_MAX` is 2048 because the longest markdown line was 1841, so
  that measurement has a **consumer**; the kernel's image size, which
  `notes/benchmarks/kernel-footprint-and-caches.md` calls "the number that does not matter", has only a reader.
- *It moves on its own.* A constant somebody chose is not a measure, it is a decision, and it
  belongs in `design/decisions/`. The stack guard page is 4,096 bytes because a page is 4,096 bytes. The
  deepest chain that can reach that guard is a measure, because the compiler moves it every week
  and nobody is asked.

**A register that lists every number in the tree is worthless**, so the exclusions are as much of
this document as the rows are, and each one names the test it failed. They are in their own section
below rather than implied by absence.

## The three states, and the middle one is the finding

Every row is in exactly one of these. The state is a property of the **instrument**, not of the
measure's importance.

| state | means | what happens when the number moves |
|---|---|---|
| **gated** | an instrument re-takes it, and something fails on a bad move | a red build, at the commit that moved it |
| **dated** | a named command re-takes it; nothing fails | the recorded value goes stale, silently |
| **owed** | defined, with its instrument named; no instrument exists yet | nothing, because nothing is measured |

**The `dated` rows are the answer to the question this milestone was raised to ask.** They are the
numbers something depends on where a regression arrives as somebody's data being slow rather than
as a red check. Promoting one to `gated` is the work; recording that it is `dated` is what makes the
work visible.

`dated` is not a defect by itself. `notes/counted-claims.md` puts it plainly: *"A wall clock is not
a count... dating a measurement is the honest alternative to gating it, and the two should not be
confused."* A number that costs a forty-minute boot to re-take does not belong in a gate that runs
on every push. The defect is a `dated` row with no date, or with no command.

## Gated

Nine instruments, and it is worth seeing them in one table because **six of them are the same
shape**: a ceiling that fires when a number grows and stays silent when it falls. Four of those six
were here before milestone 134, unnamed and unconnected, which is why `count-at-most` is a name for
a pattern rather than a new idea. Row 1 is the odd one out, a two-sided drift band, and row 9 is a
floor.

| measure | instrument | what fails |
|---|---|---|
| icount ticks, 14 benchmarks, both ISAs | `script/bench --check` | drift over 10% from `bench/baseline-*.txt` |
| IPC fastpath instruction footprint | `script/fastpath-footprint --check` | growth over 5% from `bench/fastpath-*.txt` |
| the largest kernel stack frame | `script/stack-frame-check` | any frame over the 4,096-byte guard page |
| the deepest reachable kernel-thread chain | `script/stack-depth-check` | a chain over the 24,576-byte stack |
| kernel stack high-water, at runtime | `script/test`, `report_high_water` | boot 61,440, secondary 16,384, thread 18,432 |
| eleven counted claims (harnesses, syscalls, rights bits, ...) | `script/lint` | a marked number disagreeing with the tree |
| unsafe density outside `kernel/src/arch/` | `script/lint` | over 94 blocks per 10,000 lines of code |
| `unsafe impl Send`/`Sync` claims | `script/lint` | over 17, which is today's tree exactly |
| per-file line coverage | `script/coverage` | any file under the 80% floor |

## Dated

The command is the point of each row. A dated measurement whose re-taking is folklore is a `dated`
row pretending to be one.

| measure | last taken | the command that re-takes it |
|---|---|---|
| IPC round trip in nanoseconds, both planes | 2026-08-04 | `script/bench --real` |
| filesystem throughput, milestone 38's four phases | 2026-08-18 | `script/bench --real --smp`, with a RedoxFS disk attached |
| primitives against Linux and macOS on the same host | **no date recorded** | `bench/host/run_linux.sh`, then `script/bench --real` |
| `unsafe {}` blocks inside `kernel/src/arch/` | every run | `script/lint`, which prints it and asserts nothing |
| E1: IPC round trip against thread count | **2026-09-04 (radon, 6 boots)**; 2026-08-22 (dev Mac) | `cargo xtask bench --real` (`ipc_scale_*` rows); on radon, `script/board-image --bench` and notes/footprint-perturbation.md |
| E2: thread census on the customer path | 2026-08-22 | `cargo xtask test`, the "E2 thread census" line in `a_host_process_connects_to_the_guest_and_is_answered` (both ISAs) |
| E3: IPC fastpath footprint doubled, and the latency it costs | **2026-09-04 (radon, 6 boots); confounded, see below**; 2026-08-22 (dev Mac) | `script/fastpath-footprint --features fastpath_pad [--layout]` (both ISAs); `cargo xtask bench --real --extra-features fastpath_pad` against `cargo xtask bench --real`; on radon, eight images over `NIFE_FASTPATH_PAD`/`NIFE_FASTPATH_SHIFT` (the layout control, 2026-09-19) built by `script/board-image --bench --extra-features fastpath_pad`, procedure in notes/footprint-perturbation.md |
| E4: application working-set displacement under IPC traffic, at typical (8-pair) and high (48-pair, E1's-knee) background load | **2026-09-04 (radon, 6 boots)**; 2026-08-23 (dev Mac) | `cargo xtask bench --real` (`appdisp_*_ipc`/`appdisp_*_ipc96` rows); on radon, `script/board-image --bench` and notes/footprint-perturbation.md |
| per-IPC kernel stack depth, per shape and role (milestone 134) | **2026-09-19 (QEMU, all three ISAs, debug and release)** | debug: `script/test`, the `ipc-stack-depth:` lines of `one_ipc_reaches_a_measured_depth_into_its_kernel_stack`; release: a `bench,ipc_stack_depth` kernel booted on one hart (notes/stack-high-water.md, "Per-IPC depth") |
| multi-tasking throughput, jobs per minute against task count (milestone 168) | **2026-09-16 (radon, 5 boots; old instrument, `tasks=4` not a number)** | `script/board-image --job-mix --tftp`, then `script/board-console`, by `notes/job-mix.md`'s bench-evening procedure; the rehearsal is `script/job-mix` |

## Owed

Eight measures (M5 through M12, "Tier B") are defined here, and as of 2026-09-19 the blocker is no
longer the same one for all of them. E1 through E4 ("Tier A") no longer belong in this section: all
four ran 2026-08-22 and are `dated` rows above.

| measure | its instrument, checked against the tree 2026-09-19 | what is still missing |
|---|---|---|
| M5, cycles per IPC round trip | **exists on all three ISAs**: every tick row times `bench::cycles_per_tick` (milestone 74's two halves, milestone 309 for x86_64) | on riscv64, nothing: radon read `250.00` on 2026-09-16, so `call_reply` is about 1,256 cycles. On aarch64, argon's session and calef's `PMCCFILTR_EL0` ruling (the-aarch64-half-of-74, decision A) before any figure is published |
| M6, I-cache misses per IPC | none | an event-counter driver: nothing programs `PMEVTYPER<n>_EL0` or an SBI PMU cache event on any ISA (aarch64's boot line now reports six event counters visible, and none is used) |
| M7, D-cache misses per IPC in the stack region | half: the per-IPC stack depth (row above) bounds the bytes, not the misses | the same event-counter driver, and for the attribution half a data-address sampler that neither the A57 nor the U74 has; expect M7 to become "misses rise with thread count" plus the depth, rather than attribution |
| M8, TLB misses per IPC | none | the same event-counter driver |
| M9, per-phase cycles across one IPC | the counter: `arch::pmu::cycles()` is readable in-kernel on all three ISAs (riscv64 configures the boot hart only) | phase stamps at trap entry, dispatch, rendezvous, switch and exit, in a build that measures nothing else; not built |
| M10 to M12 | as milestone 134's block says | unchanged |

## Deliberately not in this register

Each of these was considered and each names the half of the test it failed. The list is here so the
next person does not add them back.

| number | why it is out |
|---|---|
| the kernel's image size (290,816 bytes on aarch64) | **no consumer.** notes/benchmarks/kernel-footprint-and-caches.md derives it and then says in its own heading that it is "the number that does not matter": `.text` that never runs during an IPC costs nothing in cache |
| `script/verify`'s wall clock (~47 minutes) | **no consumer.** It is a reader's patience, not a constraint anything is sized against, and notes/verification.md dates it honestly |
| lines of Rust, crates, user programs, commits | **no consumer.** AGENTS.md's method figures are rhetoric about scale, and that file says so; a gate on them would be measuring a paragraph |
| `nifefs`'s `NAME_LEN = 32` | **does not move on its own.** It is a decision with a cost per directory block, not a measurement |
| the number of `#[cfg(kani)]` unsafe blocks (14) | **already gated**, by milestone 113's fourteenth clippy configuration, per block rather than in aggregate |
| `unsafe {}` against `// SAFETY:` parity | **measured and refused.** `clippy::undocumented_unsafe_blocks` already enforces it per block as a hard error, and a count comparison disagrees with it in 65 places (38 after the regex is loosened), every one read a document that is right. notes/unsafe-obligations.md carries the reading |
| the CoreMark score | **already gated**, as a row in `bench/baseline-*.txt` |

The parity row is the one worth reading before proposing a new gate. A count check that fails
correct documents is not a weak gate, it is a gate that will be deleted, and `script/lint` has
already lost three checks with that signature.

## EXAMPLES

### Adding a measure to the register

Take the unsafe census, from calef's question to a gated row, because every step of it went
differently than expected.

**1. Apply the test out loud.** Does anything depend on the amount of unsafe in this tree? Yes: the
whole demonstrator claim is a verified-Rust capability microkernel, and unsafe is where verification
stops. Does it move without anybody editing it? Yes, 42 non-merge commits changed it in fourteen
days. Both halves pass.

**2. Take the number, and take it more than once.** A single measurement cannot tell a direction
from a level, and here it inverted the answer:

```sh
# blocks outside kernel/src/arch/, at four points in the tree's history
2026-07-15   171 blocks in   7,508 lines   227.8 per 10,000
2026-08-04   728 blocks in  58,351 lines   124.8 per 10,000
2026-08-16   817 blocks in  73,129 lines   111.7 per 10,000
2026-08-18   747 blocks in  80,359 lines    93.0 per 10,000
```

The count more than quadrupled and the density more than halved. A ceiling on the count would have
fired on nearly every lane; a ceiling on the density holds a trend that is already going the right
way.

**3. Choose the relation from the shape of the quantity, not from taste.** Equality for a census
somebody maintains, `count-at-least` where more is better and a deletion is the bad event,
`count-at-most` where less is better and a drift up is. See notes/counted-claims.md.

**4. Watch it fail.** This is not optional and it is where the two real bugs were:

```sh
# add one `unsafe impl Send` anywhere, then:
$ script/lint
lint: a counted claim disagrees with the tree:
  notes/unsafe-obligations.md:461: claims at most 17, the tree has 18
  (unsafe-thread-safety-claims: how many `unsafe impl Send`/`Sync` claims the tree makes, each one
  a hand-written assertion that the compiler is wrong about a type). A ceiling is only wrong when it
  stops being true, so this means the count went UP by 1 past the headroom. Take the addition back
  out, or raise the ceiling in this commit and say beside it why the addition was worth it
```

The density ceiling's first marker **did not fire when it should have**, and the reason is the sort
of thing only a deliberate failure finds. It was written as `at most 91 blocks per 10,000 lines`,
and the convention binds a marker to the **last** number on the line, so the gate was comparing
10,000 against 92 and passing every time. The marker now sits immediately after its own number.

### Re-taking a dated measure

There is no wrapper and there should not be one: each dated row's command is in its table cell
because the commands are genuinely different animals, and a `script/measures` that ran all of them
would take an hour and be run by nobody. Copy the cell.

```sh
# the filesystem row, which needs a disk attached
script/bench --real --smp

# then edit the date in this file's table, in the same commit as the numbers
```

If the number moved, **the finding is the movement**, not the new value. Say what moved and against
what, in notes/benchmarks.md where the series lives, and leave this register holding only the date.

## BUGS

- **A `dated` row goes stale silently, which is the whole point and is also the limitation.** This
  register makes the staleness visible to a reader who opens the file; it makes it visible to
  nobody else. Nothing checks that a date is recent, and a check that did would be asserting a
  policy nobody has set. If a row's staleness starts to matter, the fix is to promote it to
  `gated`, not to add a freshness gate.

- **The register is a ratchet, like the convention it extends.** A measure nobody adds is not
  tracked, and "the register is complete" is never a thing anybody can say. It grows as people
  notice numbers, which is the same honest boundary `notes/counted-claims.md` records.

- **The gated and dated rows are maintained by hand.** Nothing checks that
  `script/fastpath-footprint` still exists or that `script/bench --real --smp` is still the command,
  which makes this document exactly the class of artifact it was written to complain about, one
  level up. The mitigating fact is that `script/lint` already fails when a script in `script/` has
  no entry in notes/scripts.md, so a renamed instrument cannot vanish quietly from the tree, only
  from this table.

## Appendices

Moved verbatim from this file on 2026-09-24, to bring it under §212 (a prose budget). The stems are
provisional names.

- [gated-dated-and-owed-rows](register-of-measures/gated-dated-and-owed-rows.md)
- [cache-experiments](register-of-measures/cache-experiments.md)
- [reading-the-weekly-series](register-of-measures/reading-the-weekly-series.md)
- [records-by-status](register-of-measures/records-by-status.md)
- [code-proofs-and-coverage](register-of-measures/code-proofs-and-coverage.md)
- [unsafe-series](register-of-measures/unsafe-series.md)
- [landed-each-week](register-of-measures/landed-each-week.md)
- [which-model-wrote-it](register-of-measures/which-model-wrote-it.md)
- [project-cost](register-of-measures/project-cost.md)
- [prose-budget](register-of-measures/prose-budget.md)
