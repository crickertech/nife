# The record-level sweep

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds milestone 138's sweep of the RedoxFS record size and the two-term model it found, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## The record level, swept: what a 128 KiB record actually costs (milestone 138, 2026-08-18)

Milestone 38 ended on one term: every 4 KiB file request moves 128 KiB, because RedoxFS stores a file
in 128 KiB records and reads a record whole. Milestone 138 lists three ways to fix that and says none
of them can be argued until somebody measures throughput against the record level, because nobody
had. This is that sweep.

**The headline.** Taking the record from 128 KiB down to one block makes a 4 KiB read **5.6 times
faster** and a 4 KiB write **3.0 to 3.8 times faster**. It does not deliver the 32x, and why it does
not is the useful part: **the 32x was never all of the cost.** A request also pays a fixed ~208 us
that no record level touches, and once the record is one block that fixed cost is **80% of what is
left**.

### How it was measured, and what to do about a machine that was not quiet

`bench/record-level-sweep.sh`, on milestone 38's existing harness rather than a new one: the same six
phases of `fs_test_client`'s throughput role, the same 256 transfers of 4 KiB per phase, the same
fixed-seed offsets, the same fresh incompressible payload per write. Nothing about the benchmark
changed, so every figure here is comparable to the milestone 38 table above.

**How a level gets set, since nothing can ask for one.** The script edits `RECORD_LEVEL` in
`vendor/redoxfs/src/lib.rs`, rebuilds, runs, and puts the constant back; it refuses to start if that
file is already dirty, so it cannot restore over an edit it did not make. `cargo xtask bench
--release --real --smp` regenerates the RedoxFS image on every run, so each point is a whole
filesystem built at that level rather than a mixed one. **The tree's committed value is unchanged at
5.** This sweep measures; it does not decide.

**The machine was loaded, so the control does more here than select rounds.** Twenty passes ran over
the six levels, interleaved (one pass sweeps 0 through 5, then the next), on a host at load averages
of 5.5 to 21 with three other lanes gating. That is outside the load 4 to 9 milestone 38 took its own
figures at, and keeping only the rounds that pass a 2% control would have left one or two per level.

So the primary figure below is a **ratio**: each phase divided by the **same round's** `fs_read`.
`fs_read` is the right denominator rather than an arbitrary one, because it reads `motd`, which is 69
bytes and lives inline in its node, so it fetches no record at all and its cost is **independent of
the level being swept**. That is a property of the code (`read_node_inner` returns from the inline
branch before it reads `record_level()`) and it is visible in the data: `fs_read` measures 203 to
208 us at every level on every quiet round. Dividing by it cancels whatever the host was doing to the
guest that second.

**Two runs taken at an ordinary load are the evidence that the normalisation invents nothing.** Before
the sweep, at load 6.9 and 7.3, single runs at level 5 and level 0 gave a sequential read of
1,466,327 ns and 257,893 ns. The ratio method over twenty passes at loads from 5.5 to 21 puts them at 1,453,963 and
258,582: **0.8% and 0.3% apart**. The raw minimums are printed beside the normalised figures, and
where the two disagree the disagreement is the noise rather than a finding.

### The sweep

ns per 4 KiB, with MiB/s in brackets. Level 5 is the tree's shipped value, so that row is milestone
38's table measured again by a different method on the same day; it agrees with it to within 6%.

| record level | record | seq read | rand read | record read | seq write | rand write |
|---|---|---|---|---|---|---|
| **0** | 4 KiB | 258,582 (15.1) | 261,918 (14.9) | 262,883 (14.9) | 790,317 (4.9) | 881,395 (4.4) |
| **1** | 8 KiB | 280,262 (13.9) | 281,607 (13.9) | 279,648 (14.0) | 769,266 (5.1) | 911,009 (4.3) |
| **2** | 16 KiB | 355,002 (11.0) | 359,628 (10.9) | 359,342 (10.9) | 870,776 (4.5) | 1,079,208 (3.6) |
| **3** | 32 KiB | 517,314 (7.6) | 521,923 (7.5) | 518,700 (7.5) | 1,058,267 (3.7) | 1,398,535 (2.8) |
| **4** | 64 KiB | 836,690 (4.7) | 839,899 (4.7) | 840,427 (4.6) | 1,520,484 (2.6) | 2,056,411 (1.9) |
| **5** | 128 KiB | 1,453,963 (2.7) | 1,458,916 (2.7) | 1,458,735 (2.7) | 2,408,470 (1.6) | 3,331,724 (1.2) |

The raw minimum of every round at each level, with no normalisation at all, in ns:

| record level | seq read | rand read | record read | seq write | rand write |
|---|---|---|---|---|---|
| 0 | 256,296 | 251,037 | 251,311 | 767,722 | 868,865 |
| 1 | 268,126 | 268,806 | 264,532 | 720,203 | 879,846 |
| 2 | 338,488 | 333,764 | 336,788 | 805,264 | 1,018,608 |
| 3 | 506,940 | 512,699 | 513,431 | 1,043,946 | 1,367,542 |
| 4 | 810,257 | 830,180 | 823,514 | 1,505,937 | 2,016,396 |
| 5 | 1,445,499 | 1,462,345 | 1,399,981 | 2,357,163 | 3,260,817 |

### One straight line fits all six points, and that is the result

The three read phases measure the same thing at every level, which is milestone 38's no-cache finding
holding at every record size. Fitting `cost = a + b x 2^level` by least squares:

| phase | `a`, fixed per request | `b`, per 4 KiB block | residual at levels 0..5 |
|---|---|---|---|
| `fs_seq_read` | 207,679 ns | 38,980 ns | +4.6 -1.9 -2.4 -0.4 +0.6 -0.1 % |
| `fs_rand_read` | 210,769 ns | 39,036 ns | +4.6 -2.6 -2.0 -0.2 +0.5 -0.1 % |
| `fs_record_read` | 209,832 ns | 39,059 ns | +5.3 -3.0 -1.9 -0.7 +0.7 -0.1 % |
| `fs_seq_write` | 672,199 ns | 53,720 ns | +8.1 -1.3 -1.9 -4.1 -0.7 +0.7 % |
| `fs_rand_write` | 769,202 ns | 80,049 ns | +3.6 -2.0 -0.9 -0.8 +0.3 +0.0 % |

**Read a request as two terms**: about **208 us** the record level does not touch, plus **39.0 us for
every 4 KiB block the record holds**. At level 5 the second term is 32 blocks and swamps the first; at
level 0 it is one block and the first term is 80% of the total.

**That corrects a constant this page has been quoting.** Milestone 38's **46.2 us per block** came
from dividing one measurement by 32, so it charged the per-request metadata walk to the blocks. It is
an average rather than a marginal cost. The marginal cost of a block through the confined block server
is **39.0 us**, and the per-request metadata walk is a separate 208 us, which is about 5.3 blocks at
that price. Both readings describe the same measurement; the sweep can separate them because it has
six points and a slope. The parity claim survives and gets slightly stronger: 39.0 us against Linux's
38.7 to 53.3 us for a raw 4 KiB virtio read at this tier.

**The writes decompose the same way, and they say out loud what a copy-on-write write is.** A random
write's slope is **80.0 us per block, 2.05 times the read slope**: the record is read and then
written, exactly as copy-on-write says. A sequential write's slope is lower, **53.7 us**, because a
growing record doubles its stored level rather than rewriting a full one, which is the mechanism
behind milestone 38's "a sequential write is 55 blocks and a random write is 74". Both write
intercepts are far larger than the read's, 672 us and 769 us against 208: that is the transaction,
which allocates, rewrites the node and commits to the header ring on **every request**, and no record
level touches it either.

**The one place the fit is visibly wrong is worth more than the fit.** Level 0 reads land about 5%
above the line, consistently, in all three read phases. That is the metadata cost of small records
arriving on schedule. A node carries 128 direct record pointers, so at level 0 they address the first
512 KiB of a file and the throughput file is 1 MiB: **half of its reads need an indirect block read
first**. At level 5 the whole file is eight direct pointers and there is no indirection at all.

### What this says about milestone 138's three options

| | read | seq write | rand write | against today |
|---|---|---|---|---|
| **today**: 4 KiB request, 128 KiB record | 2.7 MiB/s | 1.6 | 1.2 | 1x |
| **option 2 alone**: 4 KiB request, one-block record | **15.1** | 4.9 | 4.4 | 5.6x read, 3.0x write |
| **option 1 alone**: 64 KiB request, 128 KiB record | **43** | 26 | | 16x |
| **options 1 and 2**: 64 KiB request, 64 KiB record | **75** | 41 | | 28x |
| the block path's own ceiling | ~100 | | | 37x |
| ext4 `O_DIRECT` at a 64 KiB unit, same tier | ~940 | ~940 | ~900 | |

**Only the first two rows are measured**, and the difference matters. Rows three and four are the
measured cost of **one request that fetches one record of that size**, which is what a 64 KiB request
would cost if the contract could carry one. The derivation rests on something milestone 38 measured
rather than assumed: `read_record` reads the block the pointer **stores** and only then checks the
level asked for, so how many bytes a request asks for does not change what the store fetches. The
extra cost of moving 64 KiB rather than 4 KiB into the client's pages is bounded by `fs_payload_fill`,
which paints a page in 820 ns: sixteen pages is **13 us against 837**, under 2%.

- **Option 2, a record level matched to the transfer unit.** 5.6x on reads and 3.0x on writes,
  measured. It is the only one of the three that needs no agreement between two programs. Its costs
  are below, and the largest of them is not on milestone 138's list.
- **Option 1, a multi-page transfer on the file contract.** 16x on its own, which is more than option
  2 buys, because it amortises **both** terms of the model over sixteen times the payload rather than
  only the record term. It is a wire change.
- **Both.** 28x, and this is the combination worth wanting. Note what happens to the record level once
  a request carries 64 KiB: level 4 and level 0 cost the **same** 837 us for that 64 KiB, because the
  fixed cost is per request and the block count is identical either way. **With a multi-page transfer
  the record level stops mattering for aligned bulk IO**, and option 2 collapses into "do not fetch
  more than the request asked for", which every level from 0 to 4 satisfies.
- **Option 3, replace the store.** This sweep gives it nothing. The 32x it was blamed for is a
  parameter the format already carries, and what survives after that parameter is turned down is
  208 us of RedoxFS re-reading its own metadata from the device on every request, about 5.3 block
  reads. That is the absence of a cache, which milestone 138 puts out of scope on purpose, and
  swapping the store to acquire one is a strictly larger change than adding one. **Nothing measured
  here is evidence that RedoxFS is the problem.**

**And the wall behind all three, which is new.** The ceiling row is not rhetorical. `IpcDisk::read_at`
chunks every record into **one `filesystem_protocol::blk` request per 4 KiB block**, because the block contract
shares exactly one page with the block server, the same limit the file contract has. A 128 KiB record
read is therefore 32 device round trips rather than one 128 KiB transfer, and 39.0 us is the price of
a round trip rather than of the bytes. At that price **no request size and no record level can exceed
about 100 MiB/s**. Linux moves 64 KiB through one virtio request for 67 us at this same tier, which is
where the remaining order of magnitude lives, and it is one layer below the one milestone 138 is
about.

### BUGS

- **Nothing here was measured at a level other than the tree's own in a build anyone kept.** Every
  point except level 5 came from a throwaway build with the vendored constant edited, and the whole
  filesystem in that build used that level. A per-file level, which is the shape option 2 would
  actually ship, has never been built or measured.
- **The only correctness these runs prove is the harness's own.** The throughput client checks the
  bytes it read back and the file's length after each phase, and every level passed that at every
  round, but `script/test` was not run at any level below 5. A level that measures well is not
  thereby a level the confinement, crash and recovery suites pass at.
- **The record-aligned phase stays aligned by luck rather than by design.** `fs_record_read` reads at
  multiples of 128 KiB, which is a multiple of every record size at or below level 5, so it means the
  same thing at every point in this sweep. It would stop meaning it the moment anyone swept above
  level 5, and nothing checks that; `filesystem_protocol::fixture::throughput::RECORD` says so in its own
  comment.
- **The machine was not quiet and the headline figures are normalised rather than raw.** The method is
  above and the raw minimums are printed beside them. Both agree with the two runs taken at an
  ordinary load, which is the evidence that the normalisation is honest, and it is weaker evidence
  than a quiet machine would have been.
- **Option 1 is priced by derivation and not by measurement**, because it cannot be measured without
  being built: no request in this system can carry more than a page. The assumption it rests on is
  named where the price is, and it is one milestone 38 measured rather than assumed.
- **The space figures count non-zero blocks in a fresh image, which is not the allocator's own
  answer.** A block a record has vacated keeps its old bytes, so the count would drift upward on an
  image that had been rewritten; these images were made, imported into once, and never written
  again, which is the case where the count and the allocation agree. RedoxFS's own free-block count
  (`filesystem_protocol::fs::STATFS`) would be exact and needs a guest or a verb the host tool does not have.
