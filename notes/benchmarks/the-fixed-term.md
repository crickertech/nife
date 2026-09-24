# The fixed term: five blocks per request

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the 208 us identified, what option 2 costs, and why 4 KiB is the only transfer size the system had, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

### The 208 us, identified: five blocks, and they are the same five every time

The sweep above left the fixed term attributed rather than measured, as "RedoxFS re-reading its own
metadata, about 5.3 block reads". calef's question was whether that is inherent to RedoxFS's design
or the absence of a cache any store would need. It is the second, and the measurement is a count
rather than a time, so it is not close.

**Every 4 KiB read of an ordinary file makes exactly five single-block reads below the record, and
they are the same five block numbers on every request.** Not 5.3 on average, and not a different five
each time: five, and the same five, in every phase measured.

They are one call, `Transaction::read_tree_and_addr`, which `Server::read` reaches once per request
through `read_node`:

| # | block | what it is |
|---|---|---|
| 1 | `header.tree` | the node tree's L3 root, fixed for the whole filesystem until something commits |
| 2 | L2 | one block per 16.7 M node ids |
| 3 | L1 | one block per 65,536 node ids |
| 4 | L0 | one block per 256 node ids |
| 5 | the node | the file's own `Node`, one block, one per file |

`TREE_LIST_SHIFT` is 8 (`vendor/redoxfs/src/tree.rs`), so the fanout is 256 per level and the first
four are shared by every file whose node id falls in the same 256. In the fixture below, two 1 MiB
files and a `motd` share blocks 1 through 4 and differ only in block 5.

**How it was counted.** A `Disk` implementation over an in-memory image that logs every `read_at`,
built as a temporary probe in `redoxfs_server`'s host tests and reverted before this note was committed
(see the reproduction below). `BlockDisk` splits a `Disk` call into whole-block transfers, one
`filesystem_protocol::blk` request each, exactly as `IpcDisk` does on device, so `ceil(len / 4096)` per call is
the number of block-server round trips the request costs on the machine. The block *numbers* differ
on device (a different image, behind a partition offset); the counts and the repetition do not.

Fixture: a 32 MiB image, `FileSystem::create`, two 1 MiB incompressible files and one inline `motd`,
reopened through `Server::open`, then 256 requests of 4 KiB per phase, logged per request.

| phase | single-block reads per request | distinct such blocks | record read |
|---|---|---|---|
| 1 MiB file, 256 sequential 4 KiB reads | **5.00** | **5** | 1 call, 32 blocks |
| 1 MiB file, 256 random 4 KiB reads | **5.00** | **5** | 1 call, 32 blocks |
| a second 1 MiB file, 256 sequential | **5.00** | **5** | 1 call, 32 blocks |
| alternating between the two files | **5.00** | **6** (4 shared) | 1 call, 32 blocks |
| `motd`, 64 reads, inline, no record | **5.00** | **5** | none |

**99.6% of those reads were of a block already read in the same phase** (1,275 of 1,280). Zero writes
happened during any read phase, so nothing was invalidating anything.

**And it does not move with the record level**, which is what makes it the fixed term rather than
part of the slope. Re-run at `RECORD_LEVEL` 1 and 2: 5.00 per request, 5 distinct blocks, unchanged,
with the record read falling to 2 and 4 blocks as the model says.

Level 0 is the exception and it is the residual the sweep already found. There the record read is
itself one block, so the probe's classifier folds it in, and the figure is **6.50** per request
sequential and 6.53 random. That decomposes as five tree blocks, one record, and **0.5 indirect
pointer blocks**: a node holds 128 direct record pointers, a 1 MiB file at level 0 is 256 records, so
half of its reads need an indirect block first. That is the 5% level-0 residual in the fit above,
measured directly rather than inferred, and it is a property of the *small* record rather than of the
walk.

**What a perfect cache removes.** Five block reads at the measured marginal 39.0 us is **195 us
against the fitted 208 us intercept, 94% of it.** The remaining ~13 us is the file-IPC round trip and
the server's own work, which no cache touches. A second measurement says the same thing from the
other direction and was already on this page: `fs_read` reads an inline `motd`, does exactly these
five reads and nothing else, and costs 203 to 208 us.

The cache is not a large object. Four of the five blocks are the tree spine and are shared by every
file; a filesystem with 65,536 nodes has a spine of 1 + 1 + 1 + 256 = **259 blocks, about 1 MiB**,
and the fifth block is the node, one per open handle, which a server holding handles could keep
without a cache at all.

**So option 3 gets nothing here either, and now for a measured reason rather than an argued one.**
The question was whether the 208 us is structural. The walk is structural in one narrow sense, that
the format fixes the depth at four levels plus the node, and a store with a shallower id-to-node map
would do fewer reads. That is not what makes it cost 195 us. It costs 195 us because **the same five
blocks are fetched off the device 256 times in a row**, and every store that maps an id to a node has
a path from a root to that node which it would also fetch. Replacing RedoxFS buys a rewrite and
arrives needing the identical cache. **Nothing measured here is evidence that RedoxFS is the
problem**, which is what the sweep said and this now says with the block numbers in hand.

**What this measurement does not settle**, stated because it is the half a count cannot reach: it
says a cache removes 94% of the fixed term on this workload, not that a cache is cheap to build. A
cache in this server has coherency and confinement questions of its own, and milestone 138 puts it
out of scope on purpose. This is an argument about which milestone owns the 208 us, not a design for
one.

**Reproducing it.** The probe was a `Disk` recorder in `redoxfs_server`'s `mod tests` plus a driver that
clears the log per request and histograms block number against read count; it is not in the tree,
because it is a one-question instrument and the tree already carries the two facts it produced. The
whole of it is `read_tree_and_addr`'s five `read_block` calls, so a reader who wants the result
without the probe can read `vendor/redoxfs/src/transaction.rs:498` and count. `git log` for this
section has the probe in its message.

**Conditions.** Host measurement only, no emulator, so no QEMU ran and nothing about it competes with
another lane. Load average 2.4 at the start. That matters less here than anywhere else on this page:
every number in this section is a count of block reads, and a count does not move with load. The one
time in it, 39.0 us per block, comes from the sweep above and carries that sweep's conditions.

### What option 2 costs, and why level 1 is the interesting answer rather than level 0

Two of the three costs milestone 138 named are avoided by not going all the way down.

**Compression is given up at level 0 and only at level 0.** RedoxFS compresses a record when its
stored level is above zero (`write_node_inner_records`: `if decomp_level.0 > 0`), so a one-block
record is never compressed and an 8 KiB record still is. Level 1 keeps lz4 and reads **8% slower than
level 0**, which is nothing against the 5.6x either of them buys, and it writes sequentially
*faster* (769,266 ns against 790,317).

**More records means more block pointers, and the sweep shows it** in the level 0 residual above. It
grows with the file rather than staying put: an 8 MiB Time Machine band file is 64 records at level 5
and every one of them direct, against 1,024 records at level 1 of which 87% need an indirect block
read, which is one more 39.0 us round trip on a 280 us request, about 14%. Level 1 halves the number
of records against level 0 for the same reason it keeps compression.

**Copy-on-write means a write reads its record first**, and that cost *falls* with the level rather
than rising: it is the 80.0 us per block slope, and at level 0 there is one block to read instead of
32. It is a cost of the large record, not of the small one.

**And the space cost, which is the one this sweep can put a number on.** The same 560 KiB of
documentation imported into a fresh 16 MiB image, counted as non-zero 4 KiB blocks:

| record level | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| blocks used | 200 | 172 | 165 | 162 | 160 | 145 |
| against level 5 | **+38%** | +19% | +14% | +12% | +10% | 1x |

That is compression and metadata together, on text, which is the payload most favourable to lz4.
Level 0 gives up both and pays 38%; level 1 keeps compression and pays 19%, and the 19% is the
pointers rather than the entropy. **An incompressible payload would show only the pointer half**, so
a backup workload should expect something closer to the 10% at level 4 than to the 38%, and this
sweep did not measure that case.

### How a level is chosen, and the correction the code forced

**The premise milestone 138's block rests on is true and incomplete.** `record_level` is a per-node
field in the on-disk format (`vendor/redoxfs/src/node.rs`), `Node::new` sets it once at creation, and
both data paths honour the node's value rather than the crate constant (`transaction.rs`,
`read_node_inner` and `write_node_inner_records`). Directories get 0 already. All of that reads
exactly as the block says.

**What the block gets wrong is "not a fork of the vendored crate".** Three things in the engine put a
per-file level out of reach today:

- `Node::new` takes no level and there is no setter. The constant is the only source.
- `RecordRaw::empty` and `HTreeNode::empty` both refuse a level **above** `RECORD_LEVEL`, and
  `read_block` allocates its buffer through `T::empty(ptr.addr().level())`. So **lowering the constant
  makes every record already stored at a higher level unreadable**, with `ENOENT`, on an image that
  was perfectly good before.
- Nothing in `filesystem_protocol` can name a level, so the FS server would have nothing to pass down even if the
  engine took one.

So option 2 has two shapes and they are not the same decision. **Lowering the default** is one line
and a one-way door for every existing image: no migration exists, and it costs nothing today only
because every image in this tree is regenerated from source. **A genuine per-file choice** is what the
format supports and the crate does not: `Node::new` would have to take a level, the two `empty` guards
would have to compare against a maximum rather than against the default, and a creating client would
need a way to say which level it wants. That is a divergence carried in `patches/` plus a contract
change, which is a larger thing than the block priced.

### The workload question, answered by reading rather than by guessing

**The code quoted below was changed on 2026-08-19 (milestone 55) and the section is left standing,
because the reasoning is what makes the change legible.** `smb_server`'s two `min`s now read
`fs::TRANSFER_MAX` rather than `filesystem_protocol::PAGE`, so a Mac writing a megabyte arrives as 16 requests
rather than 256. Measured through a real SMB client: **write 4.8x, read 2.4x**, against the 8.02x
and 5.67x step 3 measured on the contract itself, with the residual now owned by the socket
contract's own 4080-byte chunking. The table and the reasoning are in notes/smb.md's throughput
section. What follows is the finding as it stood, which is what made that milestone exist.


Milestone 138 asks whether 4 KiB is the atypical case, since a Time Machine backup writes band files,
which are large and sequential, and a 128 KiB record is plausibly right for those.

**It is not the atypical case. It is the only case this system has.** `user/src/smb_server.rs` chunks
every SMB read and every SMB write into `filesystem_protocol::PAGE`-sized requests, in a loop, because that is
what a `filesystem_protocol` request carries:

```rust
let want = (out.len() - done).min(filesystem_protocol::PAGE);      // read
let chunk = (data.len() - done).min(filesystem_protocol::PAGE);    // write
```

A Mac writing a megabyte into a band file therefore arrives at the store as **256 separate 4 KiB
writes**, each a read-modify-write of a whole 128 KiB record, with nothing between the two to coalesce
them: there is no cache in the FS server, and a RedoxFS transaction's `write_cache` lives and dies
inside one request.

**So the reframing inverts.** The large record is not right for the customer path and wrong only for
the benchmark. It is wrong for both, for the same reason, which is that the transfer unit is 4 KiB
everywhere in this system while the record is 32 times that. A 128 KiB record starts to make sense the
day a request can carry one, which is option 1, and not before.

One honest qualification: **a band file is written sequentially and grown**, and that is the cheapest
thing a 128 KiB record does, because a growing record doubles its stored level instead of rewriting
128 KiB from the first page. It is already in the numbers, as the gap between the sequential and
random write columns at every level. It is a discount on a bad price rather than a case for the price.

What that costs a real backup, at today's setting and at the two that are one decision away: a 100 GB
first backup is **17.6 hours** of sequential writing at 1.62 MiB/s, **5.8 hours** at option 2's 4.94, and
**42 minutes** at the 41 MiB/s of options 1 and 2 together. Those are the write path alone, with no
network, no SMB, and no second copy of anything.
