# The baseline ledger: every survivor's disposition

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## The ledger: every survivor's disposition

The triage rule says nothing stays untriaged, so this is the accounting that backs the claim. Rows
are the run's survivors (missed plus timeout); "killed" means a test was written and **verified by
applying the mutation and watching that named test fail**, "equivalent" means the mutant provably
cannot differ from the original, and "hang" means a timeout that was confirmed to be an infinite
loop rather than an undetected bug.

| crate | survivors | killed | equivalent | hang | deferred |
|---|---|---|---|---|---|
| video_terminal | 79 | 64 | 15 | 0 | 0 |
| line_editor | 48 | 45 | 3 | 0 | 0 |
| fs_proto | 46 | 1 | 36 | 9 | 0 |
| grant_plan | 46 | 25 | 1 | 20 | 0 |
| paging | 39 | 17 | 22 | 0 | 0 |
| pci | 31 | 20 | 7 | 4 | 0 |
| isa | 25 | 12 | 10 | 3 | 0 |
| glob | 21 | 14 | 0 | 7 | 0 |
| dtb | 21 | 0 | 1 | 20 | 0 |
| network_time_protocol | 16 | 4 | 12 | 0 | 0 |
| compositor | 14 | 0 | 14 | 0 | 0 |
| measured_boot | 13 | 1 | 8 | 4 | 0 |
| swish | 12 | 2 | 1 | 9 | 0 |
| calendar | 10 | 2 | 5 | 3 | 0 |
| clock_protocol | 9 | 0 | 2 | 7 | 0 |
| user_mode_heap | 8 | 3 | 0 | 5 | 0 |
| gpt | 6 | 1 | 4 | 1 | 0 |
| graphics_protocol | 6 | 0 | 6 | 0 | 0 |
| slots | 5 | 4 | 1 | 0 | 0 |
| ipc | 5 | 4 | 1 | 0 | 0 |
| frames | 4 | 1 | 2 | 1 | 0 |
| cred | 4 | 1 | 2 | 1 | 0 |
| intrusive | 3 | 3 | 0 | 0 | 0 |
| coremark | 3 | 0 | 2 | 1 | 0 |
| the 2-survivor crates | 12 | 1 | 11 | 0 | 0 |
| the 1-survivor crates | 4 | 0 | 4 | 0 | 0 |
| **total** | **487** | **225** | **170** | **95** | **0** |

The 2-survivor crates are `asid`, `credential_protocol`, `entropy_protocol`, `byte_sink_protocol`, `socket_protocol` and
`generational_table`' siblings; the 1-survivor crates are `abi`, `block_roster`, `c_seam` and `capability`. Their
survivors are the recurring patterns named at the top of this section, one or two each.

**Nothing is deferred, and that is a claim worth being suspicious of**, so here is what it rests on.
Every "equivalent" in the table was argued from the code, and in the crates a later pass audited
(compositor, frames, calendar, cred, clock_protocol, gpt, fs_proto, dtb, glob) every one was also
**re-run under its mutation**. That audit changed six verdicts: five mutants called equivalent were
real gaps (`frames::index_of`'s upper bound, `calendar::from_hm`'s sign guard and its offset-length
check, `cred`'s memory ceiling, `gpt::check_partitions`' one-block partition), and glob's entire
first pass turned out to have written its tests where `cargo test` could not see them. **A verdict
reached by reading is wrong about ten percent of the time; a verdict reached by running is not.**
The crates that were not re-audited (`grant_plan`, `machine_discovery`, `measured_boot`, `network_time_protocol`, `ipc`,
`intrusive_fifo`) had their kills verified the same way when they were written, but their *equivalence*
claims rest on argument alone, and the weekly run is what will check them.

**The alarming survivors, named.** A survivor in a security boundary is worth more attention than
fifty in a display crate, so: `capability`, `memory_regions`, `dma_validator`, `nifefs` and `elf` have
**zero real survivors** between them, and the three trust-boundary parsers score 100%. The one
security-relevant survivor the run found anywhere was `filesystem_protocol::xattr::store::write_record`, whose
value limit stopped being enforced under a single `||` to `&&`, on a path that re-emits records
whose lengths come off the blob rather than from a bounds-checked caller. It is closed. The
next-most-serious were `paging`'s user-VA gate and `Mapper::root` (a constant there installs the
wrong table in silicon), `machine_discovery`'s widest-wins fold (an rv32 hart booting an rv64 answer), and
`generational_table::get_mut` (a `None` the kernel `unwrap()`s on the switch path). All closed.
