# 182. x86_64's own interactive-boot entry point

**Status: PARTIAL.** Split off from [milestone 177](177-graphical-interactive-boot.md),
2026-08-27, once that milestone's build lane found piece 3 (originally scoped as "build x86_64's
own interactive-boot entry point first") needs a from-scratch ELF-loading boot path, not wiring: a
substantially larger, separate undertaking than pieces 1-2's device attachment and program swap.
Built in part on 2026-09-14, inside [milestone 268](268-the-boot-ladder.md)'s lane, because 268's
top rung on x86_64 is not reachable any other way.

**Gate: NONE.** **Resolved 2026-09-15**, and milestone 299 is now BUILT, so this no longer waits on
it. The entry point is built and the progenitor runs
at ring 3 from the archive. What was left, a console `swish` can reach, was DECISIONS §149; calef
resolved it 2026-09-15 by reversing DECISIONS §121 (`AMENDED`): x86's console is a userspace driver
holding a port-range capability, not a kernel thread. Building that capability and moving the driver
is [milestone 299](299-x86-port-capability.md), which reaches the prompt this milestone and 268 both
left Outstanding. The `shell-check` leg lands with 299. The narrative below predates the ruling and
still weighs §149's options; it is kept as the record and the `## What §149 would take from here`
section is answered by 299.

**Amended 2026-09-09: this block's central premise no longer holds, and the milestone is smaller
than it reads.** It says the graphical stack is x86_64's *only possible* route to an interactive
shell, because DECISIONS §121 makes the console permanently kernel-resident and there is therefore
no userspace console server to talk to. §121 does make the driver kernel-resident, and that part
stands. What was missed is that `swish` never talks to a UART on any architecture: it talks to a
**console server over an endpoint**. So the question was never "can x86 have a userspace console
driver" (it cannot) but "can something else answer on that endpoint", and a kernel thread can:
`inter_process_communication::Rendezvous` is generic over `T: Node`, privilege-free, and kernel threads already exist.

**Two consequences.** Milestone 177 is **no longer a prerequisite** for this milestone, which
restores the agreed order of software parity before hardware parity. And the route is **serial**,
which is what a bench session needs, since `board_console` reads a wire and cannot read a monitor.

DECISIONS §149 carries that decision. This milestone waits on it, and on nothing else it did not
already wait on.

## What was missing (as of 2026-08-27)

x86_64 had no third function beside `spawn_init`/`riscv_shell_boot`. What existed instead,
`kernel::user::x86_userspace_demo`, is a fixture: it builds two children directly from a hand-built
region, each carved from a budget with every kernel object built in place, with no ELF parsed and
no archive loaded (its own doc: "the loader-shaped path minus the ELF"). It proves the scheduler and
the fault path can build and run real EL0-equivalent (ring 3) processes; it does not prove a real
`init` reached from a real archive, the thing `spawn_init`/`riscv_shell_boot` each already are for
their own architecture.

**Milestone 177 corrected its own earlier claim that this piece was independent of the others.**
x86_64 has no fallback UART path at all (DECISIONS §121, permanently kernel-resident), so this
milestone's only possible route to an interactive shell is through the graphical stack milestone
177 builds, not a plain-console alternative the way aarch64/riscv64 each have one. *(Superseded by
the 2026-09-09 amendment above; kept as the record of what was believed.)*

## What this needs

1. **A real ELF-loading boot entry**, matching `spawn_init`/`riscv_shell_boot`'s shape: parse the
   initrd archive, measure and verify `init` against the boot's own measurement table (DECISIONS
   §104's discipline, the same check the other two architectures' entries already make), build its
   address space, grant it the boot's own capability set, and dispatch it to ring 3. **Built
   2026-09-14.**
2. **The x86_64-specific capability grants** the other two boots each hand-assemble for their own
   architecture. **Built for everything but the console**, which is the §149 question.
3. **A third `script/shell-check` `--arch` leg.** Not built: there is no prompt to type at until
   §149 is decided.

## What was built (2026-09-14)

### The entry point, and the progenitor at ring 3

x86_64's default boot used to end in `nife x86_64: boot complete, halting.` after a green self-test
verdict. It now ends by handing the machine to the progenitor, through **the same function
riscv64's boot uses**, `kernel::user::riscv_shell_boot`, rather than a third copy of it. That
function parses the archive, measures `progenitor` against the trust root and the program table
against the kernel's vouching (`trust::require`, `trust::require_program_measurements`), builds the
address space with the archive mapped read-only, endows the progenitor, and starts it. Three
changes made one body serve both architectures:

- **Every grant names its slot** (`thread_control_block_insert_cap(.., Some(n))`), so an
  architecture that grants differently cannot renumber the rest.
- **The x86 timebase page is mapped**, the way `load` maps it for every process it builds; a
  hand-built address space has to do it itself (`map_x86_timebase_page`'s six earlier call sites
  each found that as a page fault).
- **The interrupt-controller arming at the end stays RISC-V's.** On x86_64 there is no input
  driver that could use COM1's line.

`main.rs`'s new `x86_hand_over` calls it and then watches, bounded, because on this architecture a
progenitor that cannot reach a console has nothing to print through, and a silent machine reads as
a hang. The transcript, QEMU `q35`, one core, the x86 archive attached:

```
nife self-test: 5 of 5 passed
  kernel task : a spawned thread ran and carried its captured state (0x16100004)
  ...
  initrd      : 4954112 bytes at 0xfb1e000, 87 programs, from the PVH module list

nife: handing the system to the userspace progenitor.
  uart irq: line 4 (machine description)

  user thread 6 killed: vector 6 (invalid opcode)
    rip 0x00000000004001c0   addr 0x0000000000000000   user rsp 0x0000000000500fa0   err 0x00000000
  the kernel is fine.

  user thread 4294967300 killed: vector 6 (invalid opcode)
    rip 0x0000000000400398   addr 0x0000000000000000   user rsp 0x0000000000500f40   err 0x00000000
  the kernel is fine.
nife x86_64: the progenitor is running at ring 3; 2 of the processes it built stopped on purpose.
  no prompt  : the console server cannot reach COM1 from ring 3 (§121); how a shell gets a console here is §149, not yet decided.
```

**What those two faults are, identified rather than assumed.** A temporary dump of the bytes at
each faulting `rip` and of the user stack, symbolized against the x86 ELFs: both are
`user_mode_runtime::trap`. Thread 6 is `input` (`rip` is `trap`, called from `input::uart::rx_pending`
via `input::drain`), and the other is `console` (`trap` at `0x400398` in that binary, its
`uart_put`). Both are the `x86_64` arms those programs have carried since milestone 161, which trap
on first use because a ring-3 process cannot reach port I/O (DECISIONS §121). They are the
progenitor's children, so the progenitor built the console server, the line discipline and the
input driver before they stopped, and it is still alive after the bound. Under OVMF with two cores
(`cargo xtask uefi-boot`) the same boot also prints the kernel's `capability slots: 13 of 24 at
peak`, which is the progenitor's own table gauge.

### A correction found by measurement: the empty slot

**The first version left slot 1 empty on x86_64**, because slot 1 is the UART page on RISC-V and
there is no page to grant here. The boot then failed in a way that had nothing to do with §149:
the progenitor trapped in `must(build_child(swish))` (`crates/system_initializer/src/lib.rs:1374`).
A temporary print of every syscall error named it: `MAP_INTO` of the shell's output page returned
`WrongObject`. The progenitor's first-free retype had put `term_out` into the empty slot 1, and once
the drivers existed it deleted slot 1 as `uart_dev`, deleting its own output page.

**So slot 1 holds a placeholder on x86_64**: a freshly zeroed frame, `READ | GRANT`, which nothing
else owns and which the console and input drivers' x86 arms trap before touching. It is marked in
the code as the foot gun it is. A device capability over physical page zero, which is what
`spawn_progenitor`'s aarch64-shaped path grants on x86, was refused for this: page zero is real
memory, and the console server would have written to it.

### The gates that keyed on the halt line

Three things waited for `nife x86_64: boot complete, halting.`: `cargo xtask uefi-boot`'s serial
check, its screen marker, and `script/netboot-rehearsal`. All three now wait for the self-test
verdict (`boot_ladder::SELF_TEST`), the last line every boot prints whatever it hands over to next,
and `uefi-boot` also requires the hand-over line, since that boot carries the archive. `uefi-boot`
passed under OVMF after the change.

### What §149 would take from here, per option

Priced against the tree as it stands, so the ruling can be made without reading the diff.

- **Option 1, a kernel thread answering on an endpoint (recommended in §149).** Smaller than §149
  reads, because **the progenitor already has the shape**. Its graphical branch (milestone 177,
  `has_graphical`) takes three kernel-created capabilities at slots 10 to 12 and builds no console
  and no input driver at all: `disp_term_ep`, which `line_editor` `CALL`s with `OP_WRITE` after
  filling `disp_term_page`, and `kbd_ep`, which a keystroke source `CALL`s with `OP_BYTES`. Under
  option 1 the kernel would run two kernel threads, one serving `OP_WRITE` by copying the page to
  COM1 through the direct map and one taking COM1's receive interrupt and `CALL`ing `kbd_ep`, and
  grant the three slots. The kernel already has the IPC for a kernel-side server
  (`sched::ipc_recv_cap`, `sched::ipc_reply`, `sched::ipc_call`). **No change to the progenitor,
  `line_editor` or `swish`**, and no syscall surface. What it costs is §149's own objection, a
  message parser in the kernel, and one naming wrinkle: the slots are called `disp_term_*` and would
  hold a serial terminal.
- **Option 2, a console syscall.** One or two new syscall numbers (the surface is four today), the
  `x86_64` arms of `components/src/console.rs` and `components/src/input.rs` rewritten to call
  them, and a blocking read, which needs a kernel wait queue for keystrokes. The progenitor would
  be unchanged. Every program on every architecture is then written against a surface that exists
  for one of them.
- **Option 3, the graphical stack.** `boot_graphical_terminal` is already portable and
  `pci::find_gpu_device` is PCI, which x86 has. The q35 runner would need a `NIFE_GPU` block
  (`virtio-gpu-pci` behind VT-d) and a keystroke source: the UART fallback in
  `boot_graphical_terminal` is `input`, which traps on x86, so a virtio keyboard is required too.
  xenon has neither device, and `board_console` cannot read a monitor, so the bench loses its
  serial path.

## What this does not decide

Whether x86_64's own boot path needs anything architecture-specific beyond the ELF-loading
mechanism itself (interrupt routing, device discovery specifics already covered by milestone
176's own work) is not assessed here; check that milestone's own text and `kernel/src/arch/x86_64/`
before assuming parity with aarch64/riscv64 on every point.

## What this unblocks

x86_64 joining aarch64/riscv64 as a real interactive-boot target once §149 is decided, and with it
milestone 268's top rung on all three architectures.

## BUGS

- **There is no prompt on x86_64**, and the boot says so in its last two lines rather than going
  quiet. That is §149, recorded under the gate above.
- **Slot 1 is a placeholder frame on x86_64.** Marked as a foot gun at the grant in
  `kernel::user::riscv_shell_boot`. Whatever §149 decides replaces it; until then a program that
  maps it gets a zeroed read-only page and nothing else.
- **`riscv_shell_boot` is now wrong on one of its two architectures.** It was kept rather than
  renamed, because a rename is calef's. `boot_via_progenitor` is taken by the aarch64 path, so the
  obvious noun is spoken for. Proposed: a portable name for the function both architectures enter,
  decided with `riscv_hand_over`/`x86_hand_over`, which are provisional and one shape away from
  being one function.
- **Two of the progenitor's children die at every x86_64 boot**, on purpose, and each death prints a
  kernel fault report. A reader meeting `user thread 6 killed` on a healthy boot has to read
  `x86_hand_over`'s summary line to learn it is expected.
- **`x86_hand_over` watches for ten seconds** before it prints its summary. On a boot where the
  progenitor keeps running, which is every boot now, that is ten seconds between the hand-over and
  the last line. The bound is generous for TCG and is not measured on xenon.

## Follow-on

- **Decision.** `design/decisions/149-kernel-served-console-endpoint.md`: how `swish` reaches a
  console on x86_64. The per-option cost is above.
- **Outstanding.** The third `script/shell-check` leg (item 3), and the prompt it types at, both
  waiting on §149.
- **Milestone 268.** Its top rung on x86_64 is this milestone's prompt.
- **Recorded.** The `riscv_shell_boot` naming and the slot 1 placeholder, both in BUGS above.

## Index row

Split from milestone 177 once its build lane found this piece needs a from-scratch ELF-loading
boot path (`spawn_init`/`riscv_shell_boot`'s own shape), not wiring: x86_64 has no plain-console
fallback at all (DECISIONS §121, permanently kernel-resident), so its only route to an interactive
shell is through 177's graphical stack.
