# The service path: what a userspace server costs

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds `relay_rtt`, `broker_rtt`, `fs_read`, and two small re-saves from July and August, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## The service-path benchmarks: what a userspace-server architecture costs (2026-07-29)

The microkernel bet is that filesystems, network stacks, and drivers belong in confined userspace
processes, not the kernel. The skeptic's fair question is the price: a request that a monolith
serves with one syscall now crosses into another process, maybe through a third. Two benches answer
it, and the split between them is the honest part, because the two servers this project actually
runs sit on opposite sides of a measurement line.

**`relay_rtt`: the confined-server tax, isolated and gated.** Real services fan out: the FS server
CALLs the block server (`client -> fs -> blk -> fs -> client`), net_stack CALLs the NIC driver
(`client -> net_stack -> driver -> net_stack -> client`). `relay_rtt` (kernel-side, `bench.rs`) is exactly that
two-hop topology, a client through a relay to a backend and back, and it sits on the icount baseline
next to the one-hop `ipc_rtt`:

| bench | topology | icount ticks/iter |
|---|---|---|
| `ipc_rtt` | client <-> server (one hop) | ~982 |
| `relay_rtt` | client -> relay -> backend -> relay -> client (two hops) | ~1,961 |

The two-hop path is ~2.0x the one-hop, and the **difference, ~980 ticks, is what one confined
intermediary that delegates to a backend costs**: two extra context switches and two extra
rendezvous per request. That is the architecture's per-request tax over a monolith, isolated from any
device, deterministic, and gated by `--check` so a regression in the IPC/switch path shows up against
its commit. Adding `relay_rtt` shifted the other kernel-side IPC benches a few percent (`ipc_rtt`
+6%) through whole-crate codegen, all sub-tripwire, the churn this note documents above; the baseline
was re-saved to absorb it in the commit that added the bench.

**`broker_rtt`: what the queue broker costs when both ends are up** (milestone 23, DECISIONS §41).
The same question one rung up. Milestone 23's latency ladder has two rungs built, and this is the
number that makes "opt-in per channel, never the default" a rule rather than a preference.

The **default rung has no benchmark of its own, because it has no cost of its own**, and that is the
milestone's headline rather than a dodge. A client holds a capability to a stable *endpoint*, and
whoever is parked in `RECV_CAP` on it answers; a swap changes who that is. No process stands in the
data path, so the steady state is `call_reply` exactly, and the swap adds nothing to it. The kernel's
own sender queue is what buffers the down window: requests that arrive while nobody is receiving park
there and the replacement drains them.

The **opt-in rung** is `broker`, interposed so a producer never blocks on an absent consumer. It is
the same client and the same backend as `call_reply` with a process in between, so the difference is
the whole tax:

| bench | topology | icount ticks/iter (aarch64) | HVF ns/iter |
|---|---|---|---|
| `call_reply` | client <-> server, one endpoint (**and the swap's steady state**) | ~1,007 | ~1,172 |
| `broker_rtt` | client -> broker -> backend -> broker -> client | ~2,010 | ~2,368 |

**1.99x, and about 1.2 microseconds of real time per request on this laptop.** RISC-V agrees to
within a percent (169,282 vs 338,000 ticks/1000, 2.00x). That is the price of decoupling two
components' lifecycles, and it is why the broker is wired per channel: paying it on every IPC would
trade the project's measured round-trip advantage for a feature used during swaps. It sits on the
icount baseline for both ISAs so a regression surfaces against its commit, like `relay_rtt`.

Two honest notes. `broker_rtt` and `relay_rtt` measure the same shape (one confined intermediary) in
the two different idioms the codebase actually uses, `CALL`/`Reply` and `SEND`/`RECV` pairs, and they
land within 2.5% of each other, which is a small cross-check that the Reply-capability path costs
about what a pre-wired reply endpoint does. And the broker's *pass-through* is what is measured here:
during a down window it does strictly less work per request (one rendezvous, an enqueue, an immediate
answer), which is the point, but it is not the number to quote, because the steady state is where a
channel spends its life.

Adding this bench shifted the other kernel-side numbers by a couple of percent through whole-crate
codegen (`spawn_reap` -2.3%, `ipc_rtt_el0` +0.3%), all sub-tripwire, the churn this note documents
above; both baselines were re-saved in the commit that added it.

**`fs_read`: the real RedoxFS read, whole path, and why it cannot be the isolated number.** This is
the flagship: a client opens a file through a granted **directory capability** and reads a block, over
the real confined stack (a block server driving the RedoxFS disk by DMA, the vendored RedoxFS engine
mounting it over blk IPC on its own heap). It runs on the `--real --smp` boot, where the whole stack
is proven by the redoxfs_server test, and it reports:

```
fs_read   ~9.8M ticks / 2000 reads   ~204 us/read   (HVF, --release --smp, stable across runs)
```

**204 microseconds is device latency, and saying so is the point.** A read is not served warm from a
cache; it goes to the block server, which does a DMA transfer and waits on the disk's completion
interrupt, ~200 us per block under HVF. That swamps the FS-server's own IPC-contract tax (the extra
`client -> fs` hop and the engine's dispatch), which `relay_rtt` puts at a few hundred *nanoseconds*.
So `fs_read` is the honest **whole-path** cost of a userspace file read, not an isolated server tax,
exactly the case milestone 21's rule names: when device latency swamps the isolation, measure the
whole path and say so rather than report a fictional isolated number. The clean isolation of the file
server's own cost was attempted and abandoned for this reason: a warm cache read and a raw blk-IPC
read differ by that few-hundred-ns layer sitting on top of a ~200 us block read with its own
run-to-run spread, so the delta is in the device noise. The isolated per-hop tax lives in `relay_rtt`
instead, where it is measurable; `fs_read` is what a real file read actually costs, dominated by the
disk the way it would be on any OS. And it is `--real`-only and never gated for the same reason the
number is large: the mount and every read are interrupt-driven, not deterministic under `-icount`, so
gating on `fs_read` would enshrine the non-determinism the 2026-07-28 lesson warns against. It
self-skips (the `online_count() > 1` gate) everywhere but `--real --smp`, so `bench/baseline-aarch64.txt`
never sees it.

**net_stack's socket round trip: measured, but not as a third icount bench, and here is why.** The net
path has the same shape as the FS path (a confined server the client reaches only through a granted
`Stack` capability), and its per-request IPC tax is the same `relay_rtt` topology. But a net_stack
*socket* round trip is even less gate-able than `fs_read`: net_stack only reaches its serve loop after a
DHCP handshake, and its RECV path drives smoltcp's own retransmit and delay-ACK timers (notes/net.md),
so the path is DHCP- and timer-driven, deterministic under neither `-icount` nor, at the socket level,
even a warm HVF loop. So net_stack's socket contract is proven and timed end to end by the existing net
tests (`a_client_resolves_dns_through_the_socket_contract`, `a_client_echoes_over_tcp_...`, both ISAs,
both transports), not duplicated as a bench that could only report device-and-timer latency. The bare
EL0 round trip those build on, `ipc_rtt_el0` above, is the raw baseline; the `relay_rtt` delta is the
confined-server tax net_stack pays on top of it, the same as the FS server. Recording it this way, one
gated topology tax plus the two real servers measured where each is sound, is the honest fit to what
the two instruments can and cannot see.

## 2026-07-29: both baselines re-saved for one instruction on the exception-return path

Milestone 22 phase B.2 fixed a real race in the exception-return path (notes/exceptions.md: staging
`SPSR_EL1`/`ELR_EL1`, or `sepc`/`sstatus` on RISC-V, is not atomic with respect to a nested exception).
The fix is one instruction at the top of the restore, masking interrupts, and it therefore lands on
**every return from an exception**, which is the hottest path either instrument measures.

The movement is well under 1% and both baselines were re-saved in the commit that caused it, per the
milestone-21 discipline. aarch64 `null_syscall` 457503 -> 458753 ticks over 20000 iterations (+0.3%),
`ipc_rtt_el0` +0.2%, `ctx_switch` +0.1%; RISC-V shows the same order, mixed in sign (`null_syscall`
+0.3%, `ipc_rtt` -2.2%), which is the build-to-build icount drift the 2026-07-28 attribution section
already documents rather than anything the change did.

Worth stating plainly, because it is the kind of number that invites a wrong conclusion: this is not a
regression that was traded for correctness. One masked-interrupt instruction per exception return is
what the *correct* version of that path always cost, and the previous numbers were measuring a version
that could return a new process to its entry point at the wrong exception level.

Measured-boot (phase B.1) moved nothing on either ISA, which is expected: the bench boot enters no boot
program, so the SHA-256 over the progenitor never runs there.

## 2026-08-04: the RISC-V baseline re-saved for a win this instrument cannot see

Milestone 58 removed the unconditional `sfence.vma` from every RISC-V context switch. That is a full
TLB flush per switch, gone. The numbers went **up**:

| benchmark | before | after | delta |
|---|---|---|---|
| ctx_switch | 471,827 | 477,635 | +1.2% |
| ipc_rtt_el0 | 1,738,256 | 1,766,199 | +1.6% |
| yield_switch | 179,097 | 179,217 | +0.07% |

**This is the clearest case yet of icount measuring the wrong thing, and it is worth keeping as the
worked example.** icount counts guest instructions retired. A TLB flush is *one instruction*; its
entire cost is the misses that follow, and QEMU's TCG refills its softmmu TLB with host-side work
that retires no guest instructions at all. So the instrument charged us for what we added and
credited us nothing for what we removed.

What we added, per switch: an atomic load and a branch for the ASID-width gate, and a `csrr satp`
plus a compare for the "already installed?" early return `switch_user_root` gained (aarch64 has had
it since milestone 15, and it fires on every switch between two kernel threads, which is most of them
on an idle machine). Three or four instructions, traded for not throwing the TLB away.

The baseline was re-saved in the commit that caused it, per the milestone-21 discipline. aarch64 was
left alone: the only aarch64 change in that milestone is a `dsb ishst` at the top of `flush_asid`,
which runs at address-space teardown and not on any measured path, and its numbers moved by less than
the run-to-run drift already documented above.

**The number that would settle it needs hardware with a real TLB.** `--real` runs under
Hypervisor.framework, which executes the host's own ISA, so there is no accelerated RISC-V leg to
take it on; the VisionFive 2 is where this gets measured. Recorded here rather than deferred silently,
because a milestone whose stated win is a benchmark improvement and whose benchmark got slower is
exactly the result that has to be written down.
