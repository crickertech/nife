# 156. `syscall_entry`'s measured size is every method combined, not the fastpath; extract the rest

**Status: NOT-STARTED.** Minted 2026-08-23, found while fixing milestone 126's `pmap` (`abi::aspace::LIST`
tripped `script/fastpath-footprint`'s 5% bound, +6.7% on riscv64). Extracting `LIST` alone into an
`#[inline(never)]` function fixed that one regression; measuring why turned up a bigger, pre-existing
gap this milestone tracks.

**Gate: NONE.** Nothing here needs a decision; it is mechanical extraction, verified per step by
re-measuring. `script/fastpath-footprint`'s own `--save` flag is the only irreversible act in this
milestone, and it is the last step, not the first.

## The finding

`kernel/src/syscall::invoke` -- the giant `match cap.object { ... match method { ... } }` handling
every capability method this kernel has -- has **no separate symbol in the compiled binary**.
Checked directly: `llvm-objdump` on `target/riscv64imac-unknown-none-elf/release/kernel` finds no
`invoke` symbol at all, only `dispatch`, at roughly 663 instructions (~3 KiB). The compiler folds
`invoke` wholesale into `dispatch`, because `dispatch` is `invoke`'s only call site and nothing marks
either `#[inline(never)]`.

`script/fastpath-footprint` measures `dispatch` **flat, with no closure** (its own header comment:
"a syscall traverses one path through the decoder, but the decoder's own bytes are on every syscall,
and its other arms are not on this path at all" -- the intent already states the property this
milestone restores). Because `invoke` is inlined, every method arm's compiled bytes count against
that flat measurement today, hot or not: `Untyped::SPLIT`/`DESTROY`/`RETYPE_OBJ` (133 source lines),
`Irq::bind`/`enable` (144 lines), `Frame::MAP` (63 lines), `Aspace::MAP_INTO`/`CONFIGURE` and the
rest, none of which run on an IPC round trip.

`Endpoint::SURVEY` (milestone 126's first stratum) never tripped this by accident, not by design: it
is two lines, one call into `sched::survey_supervised`, small enough nobody noticed the shared
problem. `abi::aspace::LIST` was the first arm big enough (a loop, two function calls) to cross the
5% line on its own commit. Every arm before it has been contributing bytes to a number labelled "the
IPC fastpath" the whole time, individually below threshold, collectively real.

## What this milestone does

Apply the extraction already proven on `LIST` (`kernel/src/syscall.rs`'s `aspace_list`, milestone
126) to every other administrative method arm still inlined in `invoke`: pull the body into its own
`#[inline(never)] fn`, called from the match arm, leaving only the arm's rights check and the call
site in `invoke` itself. Candidates, by source-line weight (measured 2026-08-23, before milestone
126 lands; re-measure after, since `LIST`'s own extraction changes this table):

| arm | lines | verbs |
|---|---|---|
| `Object::Endpoint` | 150 | `SEND`, `RECV`, `CALL` (**leave inlined -- this is the actual fastpath**), `SURVEY` (already a thin call, leave), `REAP` |
| `Object::Irq` | 144 | `bind`, `enable`, and whatever else this arm covers -- read before assuming |
| `Object::Untyped` | 133 | `SPLIT`, `DESTROY`, `RETYPE_OBJ` |
| `Object::Frame` | 63 | `MAP` |
| `Object::Tcb` | 57 | `CONFIGURE`, `START`, and others -- read before assuming |
| `Object::DeviceFrame` | 19 | probably too small to matter; measure before spending effort |
| `Object::Virtio` | 23 | same caveat |

**`Endpoint::SEND`/`RECV`/`CALL` and `Reply::REPLY` stay inlined.** They are the actual IPC round
trip this gate exists to protect; extracting them would defeat the milestone's own purpose. Read
each arm's real content before deciding it is a candidate -- the table above is source-line count,
not a verdict, and `Endpoint`'s 150 lines mix the fastpath in with `SURVEY`/`REAP`, which do not
belong in the hot measurement either.

## Verification, the same shape as `LIST`'s fix

Per arm extracted: `script/fastpath-footprint` before and after, on both `aarch64` and `riscv64`,
confirming the number drops and nothing else regresses. `script/lint` and the full `script/test`
suite after all extractions, since this touches the kernel's syscall boundary. **Do not `--save` a
new baseline until every planned extraction is done and measured together** -- saving mid-milestone
would bake in a smaller-but-still-inflated number and hide how much headroom the full extraction
actually recovers.

## What this does not decide

Whether `Endpoint`'s `REAP` (and any other genuinely-cold arm inside the 150-line block) is worth
extracting on its own is a judgment call for whoever builds this, guided by the measured delta each
extraction actually buys -- not every rare arm is large enough to be worth the diff.

## Prior art

None needed outside this tree: `LIST`'s own fix (milestone 126, `kernel/src/syscall.rs`'s
`aspace_list`) is the pattern, already proven to work and already reviewed.
