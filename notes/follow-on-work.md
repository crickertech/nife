# Follow-on work, and what happened to it

*Name: provisional (milestone 247). It names the thing it tracks, per §75's noun rule, but calef
names notes as he names everything else in the tree.*

A milestone finishes and, on its way out, names work it is not doing. A hazard it noticed, a design
fork it could not settle, a second phase somebody should take. **That work is what this project keeps
losing**, and this note is where its fate is written down.

## The failure, three times

- **Milestone 90** exists only because calef happened to be at his desk the day a lane report named
  its finding. Nothing else would have caught it.
- **Milestone 94** swept the tree for exactly this category, and then left **its own inventory in a
  pull request body for twelve days**. By the time anyone came back, the item-level list was gone and
  had to be re-derived (notes/untracked-work-sweep.md).
- **Milestone 244**, on 2026-09-03, named an unvouched-binary hazard in a `BUGS` section and a design
  fork in a handoff paragraph. Both surfaced because calef asked *"any work to follow up on 244?"* by
  hand, which is the mechanism today and is rung zero: somebody has to remember to notice.

AGENTS.md already carries the rule. *"Identified work leaves the lane in a tracked form, or the merge
waits."* What it does not carry is anything that fires when it is not followed, and the rule as
written is rung four: a lane report is read once, by one person, on the day it is written.

## The mechanism: a section a finished block has to answer

**Every BUILT or REMOVED milestone block carries a `## Follow-on` section**, checked by
`script/roadmap --check` and therefore by `script/lint`. Four dispositions, three of them one line:

| Bullet opens | Means | Resolves to |
|---|---|---|
| `**None.**` | Nothing was identified. | Stands alone; it is the whole answer |
| `**Milestone N.**` | It became milestone N. | A block under `design/roadmap/` |
| `**Recorded.**` | It is a limitation and it stays one. | A file, in backticks, that exists |
| `**Refused.**` | Considered and deliberately not taken. | A reason, in prose |
| `**Decision.**` | It is calef's call, written up as one. | A file under `design/decisions/` |

```markdown
## Follow-on

- **Milestone 244.** The `os_primitives_benchmarker` crate is proved by nothing.
- **Recorded.** `notes/nifefs.md` BUGS: `NAME_LEN` is 32 bytes and raising it costs directory
  entries per block. It is a constraint, not a defect.
- **Refused.** A per-call audit hook was considered and dropped: it would cost a branch on the
  fast path to record what the existing counters already show at the boundary.
```

**`Decision.` is not a fifth tracked form.** It is the tree's existing second one: AGENTS.md says
open decisions live in a file rather than in a conversation, in `design/decisions/` with
`**Status: PROPOSED.**`. Without this bullet a decision owed to calef would have to be spelled as a
refusal, which would be a lie in the one place this gate exists to stop lying.

**Why it hangs on the status rather than on a marker in prose.** The moment a block turns BUILT is
the last time anyone reads it on purpose, so it is exactly the moment the work gets buried. It is
also a state a script can see, which prose intent is not: AGENTS.md priced the greppable alternative
at `git grep -w TODO`'s 82% false-positive rate, and a check that cannot tell an observation from an
intention gets disabled within a week. **Nothing here reads a block's prose looking for intent.** It
checks that a finished block was asked the question, and that whatever answer its author wrote
resolves to something.

This is the third instance of a shape the tree already knows, which is most of the argument for it:
`script/lint` fails a `TODO` that does not name a milestone, `script/citations` checks that a glossed
citation is grounded in the document it names, and this checks that a disposition resolves to a
block, a file, or a reason.

**`None.` and `Recorded.` are deliberately the cheapest things to write.** An over-strict version of
this gate is worse than the burial it prevents: if every observation in a `BUGS` section had to
resolve to a milestone, the honest thing to write becomes expensive, people write less, and the tree
gets worse. `Recorded.` points **at** a `BUGS` entry and never replaces one. The FreeBSD posture is
upstream of this check and is not touched by it.

**An explicit refusal is a success.** The defect this attacks is silence, not the absence of a
milestone.

## The unswept list, which is an exception and says so

137 blocks were already BUILT the day this gate was written, holding roughly 159,000 words of prose.
Sweeping all of them at once is not something one lane can do honestly, and a sweep that claims
coverage it does not have would be this milestone failing in its own image. So a block that finished
**before** the sweep may sit on the list below instead of carrying a section.

**This is the exception, and per AGENTS.md's ladder it is named as one rather than left to read as a
design.** The foot gun is that a list of blocks exempt from a gate is a place to hide from the gate.

**Two things close it.** The list **may only shrink**: an entry comes off when its block gets a
section, and `script/roadmap --check` fails on an entry whose block already has one. And the parking
is closed **by date rather than by good intentions**: an entry must name a block whose `Built` column
is on or before **2026-09-03**, so nothing that finishes after this gate exists can be parked here.
A block that turns BUILT tomorrow answers the question or fails the build.

`script/roadmap --unswept` prints what is left. Read the number as a debt.

<!-- BEGIN UNSWEPT -->
```text
01-first-boot.md
02-exception-vectors.md
03-frame-allocator.md
04-mmu-and-heap.md
05-gic-and-timer.md
06-threads-and-preemption.md
07-user-mode.md
08-console-leaves-the-kernel.md
09-virtio-blk-at-el0.md
10-shell-and-spawn.md
11-untyped-memory.md
12-call-reply-ipc.md
13-capability-revocation.md
14-kernel-objects-from-untyped.md
15-asids.md
18-verify-capability-core.md
19-real-workload.md
20-portable-hal.md
21-benchmarks.md
22-trusted-init.md
26-object-revocation.md
27-rust-std.md
28-line-discipline.md
29-display-terminal.md
30-network-stack.md
31-capability-shell.md
32-redoxfs-fs-server.md
33-compositor.md
35-dma-confinement-proof.md
36-foreign-component.md
37-redoxfs-crash-consistency.md
38-filesystem-throughput.md
40-documentation-service.md
41-dead-code-triage.md
42-supply-chain-and-fuzzing.md
43-second-security-audit.md
44-github-hardening.md
45-codeql-triage.md
46-component-renames.md
49-users-and-attribution.md
50-pipes-and-redirection.md
51-wall-clock-time.md
54-network-file-service.md
56-secrets-and-entropy.md
57-partitioning-and-xattrs.md
58-riscv-tlb-shootdown.md
59-cpu-model-matrix.md
60-isa-discovery.md
61-caretakers.md
62-time-sensitive-tests.md
63-name-spellings.md
65-secrets-service.md
67-swish-language.md
68-code-quality-gates.md
69-split-user-rs.md
70-swish-logic-crate.md
71-thread-start-fault.md
72-lost-wakeup.md
73-aarch64-file-names.md
76-roadmap-split.md
78-load-sensitive-assertions.md
79-miri.md
80-loom.md
81-hvf-leg.md
82-unsafe-op-in-unsafe-fn.md
83-rule-1-lint.md
84-stack-high-water.md
85-mutation-testing.md
86-time-command.md
90-secondary-stack-guard.md
92-security-audit-cadence.md
93-doc-audit-cadence.md
94-untracked-work.md
96-one-init.md
97-citations-that-name.md
98-scheduler-naming.md
100-psci-and-cpu-discovery.md
104-init-measures-what-init-loads.md
107-sockets-that-accept.md
108-drivers-on-frame-capabilities.md
109-xargs-at-the-grant-bound.md
110-recovery-from-a-partition.md
112-safety-comments-that-bind.md
113-proofs-under-the-unsafe-lints.md
114-decisions-split.md
115-ratified-names.md
116-one-sided-fences.md
119-merge-throughput.md
120-nife-and-the-organization.md
122-a-directory-handle-std-can-hold.md
124-a-thread-is-born-where-it-lives.md
125-a-number-in-the-prose-is-a-claim.md
130-the-copy-that-outlived-its-reason.md
132-the-fastpath-footprint.md
135-loom-region-claim.md
136-one-decision-path.md
138-file-io-throughput.md
155-user-provisioning.md
156-syscall-entry-diet.md
158-kernel-object-rename-build.md
162-cpu-instruction-entropy.md
164-x86-64-fs-server-aes.md
165-x86-64-pci-acpi-mcfg.md
169-kilo-editor.md
176-x86-64-discovery-seam-wide-half.md
193-kernel-kani-reachable.md
194-falsification-records.md
195-uefi-boot-finish.md
196-elf-physical-address.md
197-user-and-xtask-proofs.md
202-confinement-claims-falsified.md
203-vendored-engine-upgrades.md
204-lane-claim-check.md
208-boot-section-wx.md
210-run-one-kernel-test.md
211-self-referential-harnesses.md
212-falsification-ratio-is-partial.md
213-harnesses-that-duplicate-the-implementation.md
214-print-and-return-skips.md
215-x86-64-pci-interrupt-routing.md
216-board-console.md
217-matched-pair-on-the-card.md
219-a-workload-that-does-not-stop.md
221-a-soak-that-crosses-cores.md
222-hvf-leg-fails-silently.md
226-qemu-bounded-orphans.md
228-close-what-we-claim-is-closed.md
229-the-counter-grant.md
230-shell-check-is-red.md
231-capability-slot-high-water-mark.md
232-what-the-checks-actually-check.md
233-login-never-runs.md
234-project-metrics.md
235-a-faulted-job-should-reach-the-prompt.md
236-lift-the-copied-derivations.md
237-the-cycle-grant-is-a-measurement-build.md
238-the-scheduled-checks-that-never-run.md
240-a-soak-should-say-where-its-threads-landed.md
```
<!-- END UNSWEPT -->

## BUGS

- **A disposition can be wrong and this gate cannot tell.** `**Milestone 240.**` resolves whether or
  not milestone 240 is the work the block meant, which is the same blind spot `script/decisions`
  records for `§N` citations and `script/roadmap` records for its own tree-wide citations. Check by
  content after any renumber.
- **It cannot find work that was never written down.** A hazard named only in a chat window or in a
  lane report nobody landed is not in the tree to be found. The count of what has already been lost
  that way is unknowable rather than zero.
- **It fires once, at the moment a block finishes.** Follow-on work identified *after* a block turned
  BUILT lands in a block nothing will re-check, because the section already exists and already
  passes. The `BUGS` convention and a `TODO(milestone N)` marker both still work there; this gate
  does not add to them.
- **The list's date cutoff is the index's `Built` column, which is a claim a human types.** It is
  gated for ISO shape and for presence, not for truth. A wrong date could park a block that should
  not be parked, and only a reader would notice.
