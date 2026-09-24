# Milestone 138, steps 4 and 2, and the comparisons not yet run

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the 16-block blk contract, the metadata cache, and the two controlled comparisons, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## Step 4 taken: the blk contract carries 16 blocks, and the win is smaller than the block count
## predicts (milestone 138, 2026-08-19)

Step 3's own residual pointed here: after it, `fs_seq_read` was 74% single-block trips through
`filesystem_protocol::blk`, one per filesystem block, against a ~100 MiB/s ceiling notes/fs-server.md's `BUGS`
section had already named. `filesystem_protocol::blk::TRANSFER_BLOCKS` goes from an unwritten 1 to 16, so the
region the FS server and the block server share is 64 KiB of contiguous pages, `IpcDisk` batches
contiguous whole-block runs into one blk `CALL` (up to 16 blocks), and the block server issues one
virtio descriptor for the whole batch instead of one per block. The crash injector and the
write-verify diagnostic keep the pre-step-4, one-block-per-`CALL` path unconditionally, because
neither tolerates a request the device completes as one indivisible unit; see `redoxfs_server`'s
`IpcDisk::write_at` for the argument.

### Before and after, ten interleaved rounds each, on a shared and noisy machine

`sh bench/blk-transfer-sweep.sh 10 1 16`. This machine was not quiet: `uptime`'s one-minute load sat
15 to 21 throughout the run (several other lanes building and testing concurrently, the tree's normal
condition per `AGENTS.md`), well above every earlier sweep's 3.6 to 9. **The `fs_read` control is the
signal that says the numbers are still usable**: 8 of 10 rounds at each point landed within 6% of the
203,000 to 208,000 ns quiet baseline the earlier sweeps established, and the analysis below is the
**median** of all 10 rounds at each point, which is what the discipline milestone 38 set (discard
rather than average a loaded round) becomes when almost every round is close and one or two are not.
The file transfer size is unchanged at `fs::TRANSFER_PAGES = 16` (64 KiB, step 3's setting) for both
points; only `blk::TRANSFER_BLOCKS` varies.

| phase | 1 block per blk CALL | 16 blocks per blk CALL | speedup |
|---|---|---|---|
| `fs_seq_write` | 1,544,228 ns | **1,335,376 ns** | **1.16x** |
| `fs_rand_write` | 2,101,313 | **1,539,488** | **1.37x** |
| `fs_seq_read` | 811,724 | **527,420** | **1.54x** |
| `fs_rand_read` | 848,706 | **546,541** | **1.55x** |
| `fs_record_read` | 843,448 | **545,040** | **1.55x** |

### Why this is nowhere near 16x, and it is a finding about steps 1 and 3, not a flaw in step 4

The naive model says "sixteen round trips become one, so this should be close to 16x." It is not,
and the reason is that steps 1 and 3 already changed what is inside the batch. `fs::TRANSFER_PAGES`
is 16 (64 KiB per file-level request), but `RECORD_LEVEL` is 1 (step 1): an 8 KiB record, two
blocks. RedoxFS's engine (`Transaction::read_node`/`read_node_inner`, `vendor/redoxfs/src/transaction.rs`)
walks the tree **once per file-level `Server::read`/`write` call** (`read_tree`, five single-block
reads: the L3/L2/L1/L0 spine and the target node), then loops over however many records that one
call's transfer spans, reading each record's body with its own `Disk::read_at` call sized to the
record. A 64 KiB request therefore spans **8 records**, and step 4 batches each record's own body
(2 blocks, one call instead of two) but cannot batch **across** records, because each record's body
arrives through a separate `Disk::read_at` the engine issues on its own. Per record: 5 metadata
reads (unbatchable, one call each, unaffected by step 4) + 1 data call (was 2). Read call count per
64 KiB request: 8 x 6 = 48, down from 8 x 7 = 56, a call-count ratio of 1.17x, which lines up with
`fs_seq_write`'s measured 1.16x almost exactly. Reads show a larger ratio (~1.55x) than writes
(~1.16 to 1.37x) because a read's total cost is almost entirely blk calls, so the same eight
eliminated round trips are a larger fraction of it; a write pays extra, unbatched cost in the
transaction commit that dilutes the same absolute saving. The absolute savings agree with the
per-block marginal cost this page already measured twice (35.9 to 39.1 us): eliminating 8 round
trips at ~37 us each predicts **~296 us** saved, and `fs_seq_read`/`fs_rand_read`/`fs_record_read`
each saved 284,304 to 302,165 ns, matching to within 6%.

**The finding, stated as a sentence a reader can act on**: step 4's batching is bounded by how big
a record is, not by how big the file-level request is, because RedoxFS only ever asks its `Disk`
for one record's worth of bytes at a time. Step 1 chose an 8 KiB record specifically to keep lz4
compression, and that choice caps what step 4 alone can batch per record at two blocks. **The
majority of what remains (5 of 6 to 7 calls per record) is the tree walk**, which is exactly what
step 2, next, targets.

### Crash consistency, re-run at the new geometry

`redoxfs_server/tests/crash_consistency.rs`, unchanged pass: 0 silently wrong. It is honest to say why it
could not have changed, the same reason step 3's re-run gave: the host-side crash model drives
`BlockDisk<Recording>` directly, below `IpcDisk`'s batching, so nothing this step touches is on that
model's path. The **device-level** crash injector (`redoxfs_server/src/bin/redoxfs_server.rs`'s `inject`
module) is on `IpcDisk`'s own path and is the reason `write_at` keeps an unconditional one-block-per-
`CALL` fallback whenever it might be armed; that fallback is byte-for-byte the code this milestone's
earlier steps already exercised, so the crash test's own coverage of the real device is unchanged by
this step, not merely re-run.

### BUGS

- **The machine was loaded for this measurement**, `uptime` load 15 to 21 throughout, several other
  lanes building and testing concurrently. The `fs_read` control and the two-term model's own
  internal consistency (the predicted ~296 us saving matching the measured 284 to 302 us across three
  independent phases) are the evidence this is a real signal and not noise, but it is not the quiet
  single-tenant machine the record-level and transfer-size sweeps had.
- **`blk::TRANSFER_BLOCKS` was chosen equal to `fs::TRANSFER_PAGES` (16) for symmetry with step 3**,
  not because 16 is where step 4's own curve bends. Nobody has swept it independently the way
  `bench/record-level-sweep.sh` swept the record level; `sh bench/blk-transfer-sweep.sh` takes any
  list of block counts and would answer this in one run.
- **A larger record level would let step 4 batch more per record**, and nobody has re-measured that
  trade now that step 4 exists. `RECORD_LEVEL` is still 1 for lz4's sake (step 1's reasoning); this
  step does not revisit it.

## Step 2 taken: a 64-block metadata cache, and it is the biggest single number in this milestone
## (milestone 138, 2026-08-19)

Step 4's own finding said where the residual now lives: 5 of every 6 to 7 blk calls per record are
`Transaction::read_tree_and_addr`'s tree walk, issued fresh on **every** `Server::read`/`write`/...
call even when the immediately preceding call resolved the identical node
(notes/fs-server.md, "the same five blocks every time", first identified 2026-08-18 and never
addressed until now). `redoxfs_server::CachedDisk` (`redoxfs_server/src/lib.rs`) wraps `IpcDisk` in a small
direct-mapped, write-through cache of single-block reads, 64 slots (`CACHE_SLOTS`,
`redoxfs_server/src/bin/redoxfs_server.rs`), about 257 KiB. Only a `buffer.len() == BLOCK` `Disk::read_at`
consults it; a record body (already the batched call step 4 built) bypasses it entirely. A write
updates or invalidates the written block's slot, and only after the inner disk confirms the write
landed, never before: RedoxFS's copy-on-write allocator never rewrites a live address in place, so
the only way a cached address's content can change is through that same write path, which is what
makes a bare write-through cache correct here with no generation counter. Six host tests
(`redoxfs_server/src/lib.rs`) check hit/miss, write-through freshness, short-write invalidation,
slot-collision safety and multi-block bypass in milliseconds, no emulator.

### Before and after, eight interleaved rounds each

`sh bench/cache-slots-sweep.sh 8 1 64`, on the same shared, noisy machine step 4 was measured on
(`uptime` load 15 to 19). Capacity 1 is not quite "off" (the same block asked for twice in a row
still hits), but the tree walk touches five *different* blocks per call, so a one-slot cache thrashes
across a single walk and rarely survives to the next one; the sweep's own doc names this. Both
`fs::TRANSFER_PAGES` (64 KiB) and `blk::TRANSFER_BLOCKS` (16) are at their step-3/step-4 settings for
both points, so this isolates the cache's marginal contribution on top of everything already shipped.

| phase | 1 slot (~off) | 64 slots | speedup |
|---|---|---|---|
| `fs_read` (control; repeated inline read) | 210,490 ns | **9,474 ns** | **22.2x** |
| `fs_seq_write` | 1,387,898 | **936,583** | **1.48x** |
| `fs_rand_write` | 1,487,736 | **1,087,904** | **1.37x** |
| `fs_seq_read` | 514,419 | **329,168** | **1.56x** |
| `fs_rand_read` | 542,605 | **331,732** | **1.64x** |
| `fs_record_read` | 560,088 | **341,504** | **1.64x** |

**`fs_read`'s 22.2x is the number that most needs its own explanation**, because it is the control
this whole milestone has used to prove there was no cache. `motd` is 69 bytes and lives *inline* in
its node, so a read of it needs no separate record read at all: the tree walk **is** the whole
request. With the cache warm, every read after the first answers from memory, so `fs_read` collapses
to close to the bare IPC/server floor this page has separately estimated at ~13 us; 9,474 ns is
better than that estimate, and the gap is plausibly the cache lookup being cheaper than a `CALL`
plus whatever margin the estimate carried. **This is also the fact that retires the "no cache
anywhere" claim** milestone 38 demonstrated and this page and notes/fs-server.md both stated as an
architectural property: it was true of the build measured then and it is not true of the build this
tree ships now. See notes/fs-server.md's own correction, in place, for what still holds (a
*different* file's first access, or any file's first access in a fresh session, is still fully
uncached) and what does not.

### The combined effect of all four steps, against milestone 38's original baseline

Multiplying the separately-measured per-step ratios would compound noise from four different days
and machine states; a single head-to-head between milestone 38's original number and this tree's
current, fully-stepped build is the honest total. `fs_seq_read`: milestone 38's 1,509,270 ns per
4 KiB request (2.68 MiB/s) against this run's 329,168 ns per 64 KiB request (189.9 MiB/s): **70.9x**,
all four steps combined, on one machine, in one comparable unit (MiB/s, since the transfer size
itself changed at step 3). That is the number for "how much of the 32x-and-then-some this milestone
set out to close has actually closed": most of it, on this metric, and the remaining gap to buffered
Linux (7,141 MiB/s) is the page-cache gap this milestone was never scoped to close (see "What is out
of scope, deliberately", below).

### Crash consistency, re-run at the new geometry

`redoxfs_server/tests/crash_consistency.rs`, unchanged pass: 0 silently wrong, for the same structural
reason step 3's and step 4's re-runs gave: the host-side model drives `BlockDisk<Recording>` below
`CachedDisk`, so the cache is not on that model's path at all. The property that matters at the
**device** level is different and is argued rather than merely re-run: milestone 37's recovery mount
is a **fresh process**, so it constructs a fresh, cold `CachedDisk` and can never observe anything
the killed process's cache held. A cache that somehow survived a process's death would be the
correctness hazard; one that cannot outlive the process that built it is not.

### What was not measured, and is the first thing to run next

**The two caches were not swept against each other.** `blk::TRANSFER_BLOCKS` and `CACHE_SLOTS` were
each swept alone, at the other's shipped value. Whether a smaller blk batch with a larger cache (or
the reverse) reaches the same total for less memory or less DMA-region size is an open question
`bench/cache-slots-sweep.sh` and `bench/blk-transfer-sweep.sh` can both answer, run together, but
nobody has run them together yet.

**64 slots was chosen against the tree spine's own size** (five blocks, times roughly twelve for
several open handles) **and against milestone 37's smaller crash-test heap budget**, not against a
sweep of the capacity itself. `sh bench/cache-slots-sweep.sh N 1 4 16 64 256` would show where the
curve actually bends; the two points measured here are the shipped value and an approximate floor.

### BUGS

- **The machine was loaded for this measurement too**, `uptime` 15 to 19 throughout. The `fs_read`
  control's absolute value (9,474 to 15,107 ns across 8 rounds at 64 slots) is small enough that a
  scheduling hiccup could move it proportionally more than it moves a millisecond-scale phase; the
  headline 22.2x is a median of 8 rounds for that reason, not a single measurement.
- **The cache is not sized against a real deployment's node count**, only against this milestone's
  test fixtures. `notes/fs-server.md`'s own note (from the original "same five blocks" finding) says
  a 65,536-node filesystem's full spine is 259 blocks; 64 slots is comfortably enough for the working
  set *one open file* needs to stay hot, and thrashes if enough distinct files are open at once to
  collide across the tree's shared upper levels. Nobody has measured that case.
- **A collision evicts silently.** Two block numbers that hash to the same slot (`block % 64`) take
  turns being cached; correctness does not depend on the hit rate (a miss is always safe, just
  slower), but a workload that happens to alternate between two colliding blocks gets none of this
  step's benefit and there is no instrumentation that would show a reader why.

## The two controlled comparisons nobody has run (2026-08-19)

**Every filesystem comparison above is uncontrolled**, and this section exists so that is a known
limitation rather than a thing a reader works out. The published pairing is nife-on-RedoxFS against
Linux-on-ext4, where **the operating system and the filesystem differ at once**. A gap can be
attributed to either, which is why milestone 138's answer to *"does this architecture have a
disk-read liability that cannot be overcome"* is assembled from decomposition (the block server is at
parity with raw virtio, the per-request residual is ~13 us, everything else found so far is an
implementation choice) rather than read off one number.

Two comparisons would control it. Both were calef's, on 2026-08-19, and neither has been run.

### Linux-on-ext2 against nife-on-ext2

Holds the **filesystem** constant and leaves the architecture. Wanted when milestone 140's ext2
stratum exists, and argued in that block: the ext2 row isolates the operating system, the nife column
isolates the filesystem, and today's diagonal isolates neither.

Its honest ceiling: our ext2 would be new against a thirty-year-old one, so the result bounds what
the architecture can cost rather than deciding it.

### Redox-on-RedoxFS against nife-on-RedoxFS

**The sharper of the two, and the reason is how we got RedoxFS.** We vendor it, so this is not an
equivalent implementation, it is *the same code*. Every difference is ours: the IPC, the scheduler,
the block driver, the shared-page contract. There is no filesystem-maturity caveat to make, because
it is their filesystem.

It also asks a different question from the Linux comparison. **Linux tells us whether the
architecture is viable; Redox tells us whether we are a good instance of it**, against a project
about a decade older than this one.

**And it has a falsifiable prediction attached, which is what makes it worth running rather than
merely interesting.** `redoxfs::DiskCache` is std-only and is never wrapped around `IpcDisk` here
(measured while identifying the 208 us). Redox has `std`. So Redox is expected to run that cache and
this system is known not to. If Redox is faster by roughly the 195 us the metadata walk costs, that
confirms milestone 138's step 2 from an independent direction. **If it is faster by substantially
more than that, something is wrong somewhere nobody has looked**, and that is the more valuable
outcome.

**Caveats, both directions:**

- **Pin the same RedoxFS revision.** This tree carries five divergences against the vendored engine,
  including step 1's `RECORD_LEVEL_MAX`. Comparing against a different revision measures the
  divergence rather than the operating system.
- **Schemes are not capabilities.** Redox's IPC has different semantics, so "the difference is ours"
  is not the same as "the difference is implementation quality". Some of it is design, and a report
  that elides that is overclaiming in whichever direction the number happens to point.
- **The cost is the setup, not the measurement**: Redox booted at the same tier, same machine model,
  same device, same payload, with the noise control this page already uses. That is the work.
