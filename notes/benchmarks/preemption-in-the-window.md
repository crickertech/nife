# Preemption inside a timed window

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the x86_64 `map_new` +26.4% lump and milestone 541's masked window, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## 2026-09-21: the x86_64 `map_new` "+26.4%" is a fixed 47,752-tick lump, not a cost per map

`script/bench --x86 --check` was reported failing on `map_new` (228,380 against a baseline of
180,604, +26.4%) by the lane building a thread's own CPU page, which A/B'd it and concluded it was
somebody else's and predated them. **Both halves of that conclusion are wrong, and the way they are
wrong is the useful part.** This is the attribution.

### The regression is not on `main`, and never was

On `origin/main` at `8ea8e2f3`, `map_new` measures **180,604**, which is the committed baseline to
the digit, and `script/bench --x86 --check` exits 0. It also measures 180,604 at `8cda8b53`, the
merge base that branch was cut from. Every number quoted in that pull request is a number from its
own branch.

### One commit, and one hunk inside it

Walking the branch commit by commit:

| commit | `map_new` |
|---|---|
| `99875a7b` the crate alone | 180,604 |
| `d5b28887` the kernel writes each thread's core | **228,380** |
| `32ac6bdd`, `d2489e26` | 228,380 |
| `31061717` the page moves into the gigabyte the process already maps | 224,332 |

So `228,380` and `224,332` are two commits of one branch, not a with-and-without pair. The A/B that
reported them "identical with the change compiled out" had compared the branch against itself.

### It is not the page, and it is not `cpu::id()`

Three A/Bs on the branch tip, each rebuilt and re-measured:

| what was removed | `map_new` |
|---|---|
| nothing (the tip) | 224,332 |
| `attach_current_cpu_page` in `AddressSpace::new` | 228,380 |
| `attach_current_cpu_page` made a no-op everywhere, so no space has a page at all | 228,380 |
| `cpu::id()` replaced by the constant `0` at the call site | 224,332 |
| `schedule()`'s `next_root` restored to its old expression | **176,556** |

The whole of it is one hunk in `schedule()`, and that hunk's semantic content is not what costs.
Applying **only** the shape change to a clean `origin/main`, with no current-CPU page, no
`cpu::id()` call and no new crate, reproduces the failure exactly:

```
let next_root = match sched.threads.get(next).unwrap().space.as_ref() {
    Some(space) => space.ttbr0(),
    None => crate::arch::mmu::reserved_root(),
};
```
in place of the `.map(|s| s.ttbr0()).unwrap_or_else(...)` chain gives `map_new` **228,356**. Two
spellings of the same value, and a 26.4% benchmark failure between them.

The map path's object code is byte-identical in both builds: `AddressSpace::map_at` at `0x8e`,
`AddressSpace::map_new` at `0x148`, the monomorphised `Mapper::map` at `0x331`,
`memory_region::retype_page` at `0x144`. Nothing on the path that `map_new` times got larger.

### What it actually is: a fixed lump inside a window that is too short

Sweeping the iteration count against both spellings settles it.

| `MAP_ITERS` | old chain | the `match` | delta | reads as |
|---|---|---|---|---|
| 64 (what ships) | 180,604 | 228,356 | **47,752** | **+26.4%** |
| 192 | 529,532 | 577,284 | **47,752** | +9.0% |

**The delta is the same 47,752 ticks at both sizes.** It is not work per map: the marginal cost of
one map is `(529,532 - 180,604) / 128 = 2,726` ticks in the old build and
`(577,284 - 228,356) / 128 = 2,726` in the new one, identical. It is a fixed lump that lands inside
the timed window, and `47,752 / 9,594` is **4.98 `yield_switch` iterations**, so the reading that
fits the arithmetic is roughly five context switches falling inside the window in one build and not
the other, as the periodic timer's phase moves against a `schedule()` of a different length.

Two consequences, and the second is the one that matters:

- **Every other row moves 0.3% to 0.6% and in the opposite direction**, `coremark` by 0.01%. The
  `match` form is marginally *cheaper* on the switch path, which is the honest reading of the
  change. `map_new` is the lone outlier by two orders of magnitude.
- **At 192 iterations the identical lump reads as +9.0% and passes the 10% tripwire.** The row is
  only 180,604 ticks, an order of magnitude smaller than every other non-`spawn` row, so a handful
  of preemptions is a quarter of it. `map_new` as it ships cannot tell a regression in the map path
  from the scheduler's code moving by a few hundred bytes.

### The x86_64 leg IS in CI, and the comment saying otherwise is stale

The pull request's premise, quoted from `.github/workflows/ci.yml` itself, was that this had sat
undetected because nothing pulls the x86_64 tripwire. That stopped being true on 2026-09-15:
`ba99c835` added `script/bench --x86 --check` to the `bench` row of `script/ci-build`, which is what
the `bench (icount regression tripwire)` job runs. It caught this on the first push, in 5m10s, which
is why that job is red. The `BUGS` comment beside the step is what is out of date, and it is
corrected in the same commit as this entry.

### BUGS

All three of these were closed the same day by milestone 541 (a timed window that excludes
preemption), whose entry is directly below. They are kept as written because the second one was
**wrong**, and how it was wrong is the reason the entry below exists.

- **This entry attributes the number and does not fix the benchmark.** `map_new`'s window is short
  enough to be dominated by whether a preemption lands in it, and that is a defect in the benchmark
  rather than in either spelling of `schedule()`. See
  design/roadmap/541-a-timed-window-that-excludes-preemption.md (a timed window that excludes
  preemption), which is this proposal promoted and built.
- **The five-context-switches reading is arithmetic that fits, not an instrumented count.** The
  lump is measured; its composition is inferred from `47,752 / 9,594`. Nothing here counts the
  preemptions directly, and a lane that fixes the benchmark should. **It did, and the count is
  one.**
- **No baseline was re-saved.** `map_new`'s baseline of 180,604 is the old spelling's number and
  still reproduces on `main` exactly. All three are re-saved by 541, on calef's ruling.

## 2026-09-21: one preemption, not five, and a window that can refuse it

Milestone 541, the entry above promoted and built. Two findings, and the first is a correction to
the entry above rather than an addition to it.

### The lump is ONE preemption, and a preemption does not cost what a yield costs

Counting directly, with a probe reading a per-core preemption counter across exactly the timed
window:

| build | preemptions in the window | `map_new` |
|---|---|---|
| old chain | **0** | 180,604 |
| the `match` | **1** | 228,356 |

So the whole 47,752-tick lump is **one** timer preemption. The "about five" reading above divided
it by `yield_switch`'s 9,594 ticks per iteration, and that denominator is the wrong event: a
`yield_switch` iteration is a *voluntary* round trip between two kernel threads that are both
immediately runnable, while a timer preemption is a trap, a dispatch, an EOI, a `schedule()`, and
then **however long it takes for this thread to be picked again**, which is unbounded by anything
in the benchmark.

**The spread on that last term is the substantive half of the correction.** The same single
preemption measured at 4,096 iterations costs **6,670** ticks, against 47,752 at 64. A factor of
seven, for the same event, depending only on what else was runnable when it landed. That is why
raising `MAP_ITERS` is not the fix it looks like: the disturbance is not a constant being divided
by a larger window, it is an unknown number of unknown-sized events.

### Preemption CAN be excluded under `-icount`, and this is the measurement

The open question the proposal could not answer. Masking interrupts across the window
(`arch::interrupts::disable` / `restore`, which all three architectures already implement):

| | old chain | the `match` | delta |
|---|---|---|---|
| unmasked | 180,604 | 228,356 | **47,752** |
| masked | 180,537 | 180,537 | **0** |

The row is byte-identical across the perturbation that used to move it by a quarter. The
`-icount` trade is real and is taken deliberately: all vCPUs share one virtual clock, so a masked
window excludes what the rest of the machine would have done inside it, which is exactly what a row
named `map_new` should exclude and exactly what `yield_switch` and `spawn_reap` must not.

### The window is ~2.5% of a tick period on every architecture

This is the number that says x86_64 was not special, and it is arithmetic over one measurement of
each:

| arch | `map_new` window | counter | window in real time | one preemption, priced | as a fraction |
|---|---|---|---|---|---|
| aarch64 | 15,870 ticks | 62.5 MHz | 254 us | ~1,759 ticks | **11.1%** |
| riscv64 | 2,410 ticks | 10 MHz | 241 us | ~112 ticks | **4.6%** |
| x86_64 | 180,537 ticks | TSC | ~181 us | 6,670 to 47,752 | **3.7% to 26.4%** |

`TICK_HZ` is 100 on all three, so the scheduler tick period is 10 ms and the window covers about
2.5% of one. Whether a tick lands inside it is a one-in-forty coin flip on the phase the boot left
the timer in, and unrelated code-size changes move that phase. **aarch64 would fail the 10%
tripwire on a single preemption**, and the only reason it never had is that the coin had not come
up heads there yet. The per-preemption prices for aarch64 and riscv64 are lower bounds, measured at
4,096 iterations where the preempted thread resumed promptly.

### BUGS

- **`map_el0` is the same shape and is not fixed.** It times a mapping loop from EL0, and a kernel
  cannot mask interrupts around a window it does not own. It has not been measured for this.
- **The per-architecture preemption prices come from one build each**, at a different iteration
  count than the one that ships, and the x86_64 row shows the same event varying by a factor of
  seven. Read them as orders of magnitude.
- **Nothing sweeps the phase.** "One in forty" is the ratio of two measured durations, not a
  measured failure rate.

### The counter cost 150 bytes of IPC fastpath, and the increment was not why

`script/fastpath-footprint` was not on this lane's gate list and caught this after the fact:
riscv64 `ipc_send_recv` at 5.4% over a 5% bound, `syscall_entry` at 6.8%. Three plausible causes
were all wrong, which is the reason this is written down.

| what was changed | riscv64 `ipc_send_recv` |
|---|---|
| base | 4,734 |
| the milestone as first written | 4,884 |
| ...with the `fetch_add` deleted, the field kept | **4,884** |
| ...with the field moved to the end of the struct | **4,884** |
| base plus the `cpu::PerCpu` field and nothing else | **4,884** |
| the counter in its own array, `PerCpu` untouched | **4,734** |

It was `size_of::<PerCpu>()` going from **128 to 136**. `PERCPU[id]` indexes an array of that
struct, so the address is `base + id * size_of`: at 128 that is a shift and `cpu::current()`
inlines to a couple of instructions at each of its many call sites, several on the IPC fastpath,
and at 136 every one of them grows. The counter moved to its own array and the fastpath is
byte-identical to base on all three architectures.

**The instrument lesson, which is why this sits in this file.** Between the two shapes every row
of `script/bench` moved by at most **0.12%**. A tripwire on time could not see a change a tripwire
on size failed on. Neither is a substitute for the other, and `map_new`'s own defect was the
mirror image: a row that moved 26.4% while the code was byte-identical.

`kernel/src/cpu.rs` now asserts the size is a power of two, exempt on x86_64, where the struct
already carries `x86_trap`, is already 152 bytes, and whose gate is green with room (+1.0%, +1.4%,
+3.9%). The exemption is measured; asserting a property the tree does not hold is how a gate
teaches people to route around it.

## 2026-09-21: the x86_64 tripwire is 26% off, and nothing has ever pulled it

Found by the same lane, while checking whether its own change had moved the third architecture. It
had not. `script/bench --x86 --check` fails on **`map_new`: 228,380 against a baseline of 180,604,
+26.4%**, and it fails *identically with that lane's change compiled out* (224,332, marginally
faster), so this is somebody else's and predates it.

It has never been caught because **the x86_64 leg is not in CI**, which `.github/workflows/ci.yml`
already says in as many words: the tripwire "is built and recorded and simply nothing pulls it: an
x86_64 icount regression passes CI silently". This is that entry's prediction, measured. The
baseline was deliberately left red rather than re-saved: blessing a 26% regression nobody has
attributed is the opposite of what a tripwire is, and the entry above is the standard this one is
held to.
