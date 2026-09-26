# Concept notes

*Name: ratified (§75 covers this directory). `notes` predates every convention here and stays for the reason `elf` stays: it is the plain word for what the files are, and renaming it would spend a reader's recognition to buy nothing. (This said `elf` and `dtb` until 2026-09-19, when DECISIONS §154 expanded `dtb` to `device_tree_blob`; `notes` is an ordinary English word and was never an acronym, so §154 does not reach it.)*

Running glossary for nife, written as concepts come up. If something in the code or the
conversation does not make sense, it belongs here.

This page is an index: one line per note, saying what the note is. The detail lives in the note.
`script/lint` fails if a `notes/*.md` file has no row here, so a new note adds its line in the
section that fits. Keep the line short; commentary on a note belongs in the note. Naming conventions
are a rule rather than a note, and live in [design/naming.md](../design/naming.md).

## Start here

- [Acronyms](acronyms.md): every acronym expanded and linked; look here first.
- [How this is built](how-this-is-built.md): the agent-built method as a claim, with caveats.
- [The stranger test](stranger-test.md): the instrument for whether a newcomer succeeds unaided.
- [Adding a user program](adding-a-program.md): the steps to add a program, task-oriented.
- [Why this isn't a general-purpose OS](why-not-general-purpose.md).
- [Things this project has already gotten wrong](corrections.md).
- [The works this project is arguing with](bibliography.md): every outside work some page here actually reads.

## The machine

What the hardware is and how the kernel meets it: read these before any kernel code.

- [Registers](registers.md): the CPU's whole state; the most fundamental note.
- [Harts and PEs](harts-and-pes.md): precise words for one instruction stream, not "core".
- [aarch64](aarch64.md): the instruction set, privilege levels, and our target triple.
- [Reading aarch64 assembly](reading-assembly.md): five rules for decoding assembly; start here.
- [The stack, `sp`, and `x30`](stack.md): how the stack works, and our stack incidents.
- [Stack high-water](stack-high-water.md): measuring how deep each kernel stack actually goes.
- [Exceptions](exceptions.md): faults, interrupts and syscalls as one aarch64 mechanism.
- [Interrupts: the GIC and the timer](interrupts.md): the GIC, the timer, and interrupts as messages.
- [The device tree](device-tree.md): the machine's self-description, and how we parse it.
- [ISA discovery](isa-discovery.md): reading each CPU's features and core list at boot.
- [The UART](uart.md): the serial port, and our PL011 driver line by line.
- [The PMU, and the two clocks in a core](pmu.md): the cycle counter versus the virtualization-proof generic timer.
- [QEMU](qemu.md): the emulator we develop on, and its flags.
- [Semihosting](semihosting.md): how the kernel reports a test exit status to QEMU.
- [Call-frame information in hand-written assembly](cfi-unwind.md): unwind tables for the hand-written `.s` files. Name provisional.

## Rust without std

- [`no_std`](no-std.md): why the kernel cannot use `std`, and what `core` gives.
- [Vec, Box, String, BTreeMap](collections.md): the four `alloc` types and why each exists.
- [The heap and the slab](heap.md): the retired kernel allocator, and the userspace heap.

## Memory

- [Physical memory](physical-memory.md): the frame allocator, bitmap over free list.
- [The MMU](mmu.md): virtual addresses, page tables, the TLB and page faults.
- [aarch64 page tables](page-tables.md): the descriptor format the MMU walks, and its traps.
- [The higher-half kernel](higher-half.md): why the kernel lives in TTBR1, and how it boots.
- [Tearing down an address space](teardown.md): how the kernel reclaims a dead address space's frames.
- [Memory regions: the kernel stops allocating](memory-regions.md): processes spend pages from their own memory capability.
- [PageFrame capabilities](frames.md): shared memory a process owns, maps and delegates.
- [ASIDs: tagged address spaces](address-space-identifiers.md): per-space TLB tags so context switches flush nothing.
- [The RISC-V TLB shootdown](riscv-tlb-shootdown.md): cross-hart ASID flush via SBI RFENCE, replacing full flushes.
- [The x86_64 TLB shootdown](x86-tlb-shootdown.md): cross-core TLB invalidation on x86, done by NMI.
- [The kernel's own budget](kernel-budget.md): kernel stacks drawn from one fixed boot-carved region.

## The kernel

Threads, capabilities, IPC, and how authority ends.

- [Userspace](userspace.md): the three hardware walls that make userspace.
- [The console driver leaves the kernel](userspace-drivers.md): the console as a userspace process reached by IPC.
- [The native ABI](abi.md): the syscall convention, object surface and entry contract.
- [Threads, the context switch, and preemption](threads.md): threads, the fifteen-instruction switch, and preemption.
- [The scheduler: placement, stealing, wakes](scheduler.md): per-core run queues, placement, stealing and wakes.
- [The TCB](tcb.md): the Thread Control Block, and where it lives.
- [Generational names](generational-names.md): thread ids that fail safely after slot reuse.
- [Intrusive queues](intrusive-queues.md): run queues whose link lives inside the thread.
- [Locking](locking.md): spinlocks with interrupts, and the orderings that avoid deadlock.
- [Deadlock](deadlock.md): the four Coffman conditions and how to break one.
- [Memory ordering, and the fences with no partner](memory-ordering.md): every fence and ordered atomic, adjudicated.
- [The IPC_TABLES lock inventory](ipc-tables-lock-inventory.md): what the one remaining IPC lock protects, by heat.
- [Capabilities, and why the kernel has no `open()`](capabilities.md): capabilities, the confused deputy, first syscalls and IPC.
- [Who does IPC name?](ipc-naming.md): IPC names a rendezvous, never the peer.
- [How authority moves, narrows, and ends](capability-lifecycle.md): how capabilities are copied, narrowed and revoked.
- [Delegating a capability](delegation.md): passing a narrowed capability between processes over IPC.
- [Object revocation: tearing a process back down](object-revocation.md): reclaiming the kernel objects a process built.
- [Ending a permanently blocked thread](blocked-thread-teardown.md): research and proposals for ending a blocked thread.
- [Supervision: a thread's death becomes a message](supervision.md): the fault endpoint, and reaping without building.
- [Per-process resource quotas](quotas.md): a live-children cap on spawners, kept but unused.
- [What a timed wait costs](timed-wait.md): pricing a deadline on a blocked thread.
- [Can a userspace process hold a timer?](timer-capability.md).
- [Trusted init: measuring the boot program, and then everything the progenitor loads](trusted-init.md).
- [The progenitor, and loading a program from userspace](progenitor-and-loading.md).
- [Auditing the hand-written arch assembly](arch-audit.md): a by-hand audit of the least-verified TCB code.
- [The L4 lessons, audited against this kernel](l4-lessons.md): the kernel checked against L4's twenty-year retrospective.

## Drivers and devices

- [virtio-blk, driven from userspace](virtio.md): a block device driven from EL0 with DMA.
- [PCIe, and driving a disk over it](pcie.md): the PCIe transport, with the kernel as firmware.
- [Scoping a PCIe transport](pcie-transport-scope.md): the pre-build scope for PCIe and virtio-pci.
- [NVMe: the first non-virtio disk](non-volatile-memory-express.md): an NVMe driver confined by the IOMMU alone.
- [Fatal risk 6's bench evening on xenon](risk-6-bench-evening.md): the confined NVMe driver's preflight, throughput boot and outcomes.
- [Confining DMA without an IOMMU](dma.md): kernel validation of every descriptor a driver submits.
- [Confining DMA with an IOMMU](iommu.md): hardware DMA confinement with SMMUv3 and the RISC-V IOMMU.
- [Block devices: what is attached, and what holding one means](block-devices.md).
- [A machine with no serial port](serial-less-output.md): screen output for machines without a UART.
- [The framebuffer contract](framebuffer-contract.md): how a confined client gets pixels onto a screen.
- [The compositor](compositor.md): one screen shared among clients that distrust each other.
- [Glyphs, the VT engine, and input](glyphs.md): the font, VT engine and keyboard behind on-screen text.

## Programs, std and services

What runs at EL0: the std port, the shell, components, and the services they call.

- [Rust `std` on the native ABI](std.md): std's platform layer implemented on the capability ABI.
- [Somebody else's crate on nife](crates-io-on-nife.md): fifty crates.io crates built against nife's `std`.
- [`ripgrep` on nife](ripgrep-on-nife.md): unmodified ripgrep builds and runs, and what stops it.
- [What one shim costs](foreign-program-arguments.md): priced per program and as a library.
- [A TLS crypto provider on nife](cryptography-provider.md): building a `rustls` crypto provider for all three targets.
- [The `thread::spawn` fork](thread-spawn-fork.md): what a std thread would cost, and why declined.
- [Running a foreign language: the C seam](c-seam.md): a confined, restartable C component under a Rust shell.
- [The program manifest](program-manifest.md): a program's declared endowment, checked at spawn.
- [A shell at EL0](shell.md): an interactive shell, console input and spawned workers.
- [The line discipline as a userspace component](line-discipline.md): the tty line editor as a userspace process.
- [The terminal contract](terminal-contract.md): the IPC protocol a terminal presents to programs.
- [The sink protocol](sink-protocol.md): one register-only protocol for writing bytes anywhere.
- [Pipes and redirection](pipes.md): `>`, `<` and `|` as one capability substitution.
- [The tail-stage output fork](tail-output-narrowing.md): where a tail stage's output goes, decided.
- [`swish` the language](swish-language.md): quoting, sequencing, and refusal as its own exit status.
- [The command line as a grant expression](grant-expression.md): naming a resource at the prompt grants it.
- [The glob matcher](glob.md): a pure byte glob matcher with a bounded cost.
- [Globbing, and the expansion you see is the grant](glob-grant.md).
- [Navigating with no global namespace](shell-navigation.md): `cd`, `pwd`, `ls`, `mkdir` and `rm` as capability builtins.
- [The inert-configuration page](env-config.md): validated read-only `TZ`, `LANG` and `TERM` for programs.
- [The documentation crate](documentation.md): streaming markdown renderer, manual viewer and search index.
- [The component manifest](component-manifest.md): what a supervisor must route before a component serves.
- [Live component replacement](live-replacement.md): swapping a running component under a live client.
- [The hung component](hung-component.md): a component that stops answering without dying.
- [Dependency-aware orchestration](dependency-orchestration.md): which components to warn before swapping a dependency.
- [The process view](process-view.md): `ps` and `pgrep` over a supervision subtree.
- [Scheduled execution](scheduled-execution.md): a cron whose every entry is a grant.
- [Wall-clock time](clock.md): wall clock as counter plus offset, three authorities.
- [`date`](date.md): prints the wall clock and cannot set it.
- [`time`](time-command.md): times a command on the shell's clock.
- [The calendar crate](calendar.md): Unix seconds to civil dates and back, and formats.
- [Entropy](entropy.md): where randomness comes from, and who may reach it.
- [Credentials](credentials.md): checking a secret the service can never read back.
- [NTLM](ntlm.md): the NTLMv2 key half of the secrets store, removed.
- [Login](login.md): authentication that returns capabilities instead of changing identity.
- [The network stack as a confined component](net.md): the confined NIC driver, smoltcp server and socket contract.
- [NTP: the wire format, and the client that carries it](ntp.md): the NTPv4 codec and a one-shot time client.
- [SMB: the network file service a Mac mounted, and why it is no longer here](smb.md).
- [mDNS/DNS-SD: the Time Machine advertisement, and why it is no longer here](mdns.md).

## Storage

- [The RedoxFS filesystem server](fs-server.md): RedoxFS confined behind a capability-shaped file contract.
- [RedoxFS std-footprint audit](redoxfs-audit.md): costing the RedoxFS port to no_std by building it.
- [The directory capability](dir-capability.md): a directory split into separable, attenuable rights.
- [Removal needs a directory](rm.md): why `rm` gets a directory and `-r` widens it.
- [`touch`: create if absent](touch.md): create-if-absent and mtime setting, with separate rights.
- [Extended attributes](xattr.md): named byte strings on files, kept above RedoxFS.
- [Reading the backup from a MacBook or a Linux host](host-recovery.md).
- [The GUID Partition Table](globally-unique-identifier-partition-table.md): reading, writing and validating GPT, host-tested and proved.
- [nifefs](nifefs.md): the boot archive format and its 32-byte names.

## Verification and security

- [Machine-checked proofs (Kani)](verification.md): how the Kani proofs work, and what they prove.
- [Proving things about `kernel/src`](kernel-proofs.md): proving kernel code, and the stub boundary. Name provisional.
- [Proving things about `user/`](user-proofs.md): proving the EL0 programs, and what it found. Name provisional.
- [Verus, and whether it reaches the code Kani stops at](verus.md). Name provisional.
- [Did the proofs catch the bugs?](proof-retrospective.md). Name provisional.
- [Falsification records](falsification.md): recording that each proof harness can fail.
- [Fuzzing the parse surface](fuzzing.md): coverage-guided fuzzing of the parsers that read outside bytes.
- [Dynamic undefined-behavior checking (Miri)](undefined-behavior.md): Miri over the host crates, and what "clean" means.
- [Interleavings, model-checked (loom)](interleaving.md): loom over the hand-rolled concurrency protocols, and its finds.
- [Mutation testing](mutation-testing.md): the cargo-mutants triage rule, the current census, and per-crate triage in 17 appendices.
- [The mutation census record](mutation-census.md): per-crate mutation scores for every census, comparable. Names provisional.
- [Where an unsafe obligation is written, and where it is only implied](unsafe-obligations.md).
- [What nife claims a confined component cannot do](confinement-claims.md).
- [A security audit](security.md): the first adversarial review of the whole kernel.
- [Auditing the shared pages](shared-page-audit.md): the second security audit, reading for double fetches.
- [Auditing untrusted counterparty input](untrusted-input-audit.md): network and device input read as hostile.
- [What each system makes you trust, measured](trusted-base.md).
- [The incremental path to a safer kernel, and why nife is not on it](incremental-path.md). Name provisional.
- [RedLeaf, and the opposite bet about where isolation comes from](redleaf.md).

## Performance

- [Benchmarks with teeth](benchmarks.md): the current numbers and their gates, with the dated history in `notes/benchmarks/` appendices.
- [The instruction clock](instruction-clock.md): timing claims measured in instructions retired, not wall time.
- [The bench runbook: which machine, in what order, and what an evening buys](bench-runbook.md).
- [Taking a benchmark on radon](footprint-perturbation.md): running the cache-footprint experiments on the small-cache board.
- [The workload that does not stop](soak.md): a sustained multicore workload whose threads never migrate.
- [A kernel-initiated reboot on every board](board-reboot.md).
- [The multicore defect-discovery curve](multicore-defect-curve.md): milestone 201 (is multicore reliability converging)'s data, the format a soak appends to, and every multicore defect so far. Name provisional.
- [The multi-tasking workload benchmark](job-mix.md): an AIM7-style workload for the process-versus-event kernel question.
- [Cycle counters on RISC-V, and why nothing here has measured one](riscv-cycle-counters.md).
- [Does the TSC tick at a constant rate under TCG?](tsc-under-tcg.md). Name provisional.
- [Running under virtualization on Apple Silicon](virtualization.md): running the kernel on the Mac's core via HVF.
- [The HVF leg](hvf-leg.md): the aarch64 suite on the physical Apple Silicon core.
- [The CPU-model matrix](cpu-models.md): the RISC-V suite run across five QEMU CPU models.

## Ports and boards

- [How portable kernels are written](portability.md): what belongs in `arch/`, and why port early.
- [Porting to RISC-V](riscv-port.md): the second-architecture port and the `arch/` boundary it tested.
- [Scoping RISC-V / aarch64 parity](riscv-parity-scope.md): the RISC-V parity gaps, all now closed, with corrections.
- [The RISC-V arch tests](riscv-arch-tests.md): RISC-V twins of the aarch64 arch unit tests.
- [RISC-V Summit Europe 2026, read for what it changes here](riscv-summit-2026.md).
- [Porting to x86_64](x86-port.md): the third-architecture port, and what did not fit `arch/`.
- [Booting x86_64 from real firmware](x86-uefi-boot.md): the UEFI loader, and the bench procedure for xenon.
- [xenon's firmware, page by page](xenon-firmware.md): the OptiPlex 7050 setup UI, transcribed setting by setting. Name provisional.
- [Where nife could actually run, and what the three bench machines are named](target-hardware.md).
- [The aarch64 board for the seL4 comparison](aarch64-board-survey.md): choosing a board sel4bench really runs on.
- [The VisionFive 2: first silicon](visionfive2.md): radon's board facts, boot paths and bench runbook.
- [Programming a clock and a reset line, for the first time](jh7110-clock-and-reset.md).
- [Reading a board, without a person watching it](board-console.md): how `script/board-console` reads a board's serial boot unattended.
- [The boot ladder](boot-ladder.md): the boot markers every architecture prints, in order.
- [What rented metal costs](rented-metal.md): priced rented hardware for each architecture. Name provisional.

## Build, boot and install

- [LLVM](llvm.md): how rustc and LLVM turn Rust into aarch64.
- [Linker scripts](linker-scripts.md): who places code, zeroes `.bss`, and sets the stack.
- [ELF](elf.md): the file format the kernel ships in.
- [The boot protocol](boot-protocol.md): the arm64 Image header that marks a kernel.
- [The boot stick, and the program that makes it](boot-stick.md).
- [What a nife package is, and what still cannot be done with one](packages.md).
- [Who may write the activation set: a proposal](who-may-write-the-activation-set.md).
- [Two boot slots, so a bad upgrade cannot brick the machine](boot-slots.md).
- [Installing nife onto a disk from a boot stick](installing.md).

## How the tree is run

Gates, records and the merge queue: the machinery that keeps many lanes honest.

- [The `script/` entry points](scripts.md): the normalized front-door commands and what each does.
- [Every check in this repository](check-inventory.md): audit of what runs, blocks, and asserts. Name provisional.
- [Selectors that can select nothing](empty-selectors.md): gates that pass when their pattern matches nothing. Name provisional.
- [What to do when `main` goes red](main-is-red.md). Names provisional.
- [The merge queue, and the three things that watch it](merge-queue.md): the scripts that land, watch, and flag queue work. Names provisional.
- [Working from a cloud session](working-from-a-cloud-session.md): what past cloud sessions hit, how to set up, claim and gate in CI, and what needs patagonia. Name provisional.
- [The automation's own identity](automation-identity.md): the `smelter` GitHub App that replaces a personal token. Name provisional.
- [Hardening the repository itself](repo-hardening.md): the GitHub settings that cannot be committed.
- [The roadmap](roadmap.md): how to add a milestone, and its vocabularies.
- [Follow-on work, and what happened to it](follow-on-work.md): the Follow-on section every finished block must answer. Name provisional.
- [The untracked-work sweep, and what each finding became](untracked-work-sweep.md).
- [The dependency census](dependency-census.md): real prerequisite edges between milestones, measured against declared ones.
- [Citations that name what they cite](citations.md).
- [Counted claims](counted-claims.md): numbers in prose that a gate re-derives. Name provisional.
- [The register of measures](register-of-measures.md): the numbers this kernel holds itself to. Name provisional.
- [Project metrics: what moved, week by week](project-metrics.md): weekly charts of the project's measures, from git history. Script and data names provisional.
- [The violation ledger](rule-violations.md): counting how often each written rule is broken. Name provisional.
- [Load-sensitive assertions](load-sensitive-assertions.md): the register of assertions that fail under host load, how to fix one, and each site's status. Appendix names provisional.
- [The CI log baseline](ci-log-baseline.md): which check failed each CI job, from expiring logs. Names provisional.
- [Every place that enumerates architectures, and whether the list is complete](architecture-list-sweep.md).
- [Rustdoc coverage](doc-coverage.md): the doc-example floor and the `missing_docs` ratchet.
- [The documentation sweep](documentation-audit.md): how to run a documentation sweep, and what counts.
- [Prior art and reuse](prior-art.md): where to look before building, and the build-versus-reuse rule.
- [Handing a session over](session-handoff.md): superseded 2026-07-29 restart point, kept as history.
- [Cobble, the mascot](mascot.md): the project's mascot, drawn by Clay.

## Delegation and the method

How agents do the work here, and what it costs.

- [What a session carries, and why delegation is about context](what-a-session-carries.md).
- [What `claude --effort` buys, measured against a real repository query](effort-levels.md). Name provisional.
- [Does a delegated AI review catch what the gates and the maintainer miss](delegated-review/README.md).
- [Renting an open-weight model for the mechanical lanes](open-model-lanes.md).
- [Reviewing the work of one model](model-attribution-review.md): a plan for reviewing one model's commits fairly. Name provisional.
- [Comparing models as lanes and maintainer](model-comparison.md): a pre-registered pilot, Opus 5 against Opus 5.5. Name provisional.
