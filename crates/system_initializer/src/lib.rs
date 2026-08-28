#![no_std]
// `Result<_, ()>` throughout the build path, for `supervision_proto`'s reason: every failure here is
// a syscall that already returned its own error through the ABI, and a second, richer error would be
// inventing detail the kernel did not provide. See that crate's head comment.
#![allow(clippy::result_unit_err)]
//! **The interactive system, built once** (milestone 96).
//!
//! There are two inits, because the two boards' kernels hand off differently: `user::initrd()` loads
//! the archive entry `init`, which is `user/src/hello.rs`'s `init_boot` role on aarch64 and
//! `user/src/system_initializer.rs` on riscv64. There is **one** system they build, and this crate
//! is it. What each init still holds is the table of slot numbers its own kernel granted, which is a
//! fact about that boot path and nothing else; everything from parsing the archive to serving the
//! shell's last `run` is here.
//!
//! Before this crate the construction and the spawn service were written twice, about three hundred
//! near-identical lines, and the failure that caused is the reason the milestone exists: a fix that
//! lands in one init and not the other is **a boot that reaches userspace and prints nothing at
//! all**, with no fault and no message. That shape cost three separate lanes an evening each.
//! `script/shell-check` boots both ISAs and types at the prompt, which is what makes it the gate
//! that proves this: it is the only thing in the tree that runs a real init.
//!
//! # Examples
//!
//! **This one cannot run either, and the reason is structural.** [`boot`] returns `!` and every step
//! it takes is a syscall on a capability the kernel granted at spawn, so there is nothing to assert
//! and nowhere to assert it: `script/test`'s host pass excludes this crate (it depends on `user_rt`'s
//! EL0 `asm!`, an exclusion `script/lint` derives and checks), and the thing that actually proves this
//! code is `script/shell-check`, which boots both ISAs and types at the prompt. So the example below
//! is `no_run`: type-checked against the real signatures, and executed by that gate.
//!
//! An init's whole source, near enough. Everything a board contributes is the **table of slot numbers
//! its own kernel granted**, which is a fact about that boot path and nothing else:
//!
//! ```no_run
//! use system_initializer::{BootEndowment, boot};
//!
//! /// riscv64's init. The slot numbers come from `kernel::user::riscv_shell_boot` and are the only
//! /// thing this file knows that the other board's init does not.
//! fn init(initrd_len: u64, fs_rights: u64) -> ! {
//!     let endowment = BootEndowment {
//!         untyped: 2,
//!         uart_dev: 3,
//!         uart_irq: 4,
//!         clock_page: 5,
//!         config_page: 6,
//!         fs_ep: 7,
//!         fs_page: 8,
//!         // Always these three slots, whether or not this boot has a disk (`grant_at`, not
//!         // first-free numbering, on the kernel side): a boot with no virtio-rng device leaves
//!         // them empty, and `boot`'s own probe is what tells it apart from a granted one. Fixed
//!         // past the filesystem pair's own max reach (slot 8), not slot 7, because the
//!         // inert-configuration page (slot 6) shifted that pair down by one.
//!         virtio_rng: 9,
//!         virtio_rng_irq: 10,
//!         virtio_rng_dma: 11,
//!         // The graphical terminal stack (milestone 177, option A): empty on a boot with no GPU
//!         // or no keyboard attached, the same "absence, not failure" shape as the virtio-rng trio.
//!         disp_term_ep: 12,
//!         disp_term_page: 13,
//!         kbd_ep: 14,
//!         // Empty here. On aarch64 this holds the kernel's report endpoint and a test SGI, because
//!         // that boot path is shared with milestone 19d's test roles; init deletes them with the
//!         // device authority once the drivers exist, rather than keeping delegable authority for
//!         // nothing.
//!         for_test_roles: &[],
//!     };
//!
//!     // `fs_rights` of 0 means this boot attached no RedoxFS disk, in which case `fs_ep` and
//!     // `fs_page` hold nothing at all and the system comes up without a filesystem.
//!     //
//!     // The fourth argument is milestone 154's second directory grant: `None` here, as at every
//!     // real entry point today, because what the second subtree should *be* is a boot-time
//!     // policy decision, not something this call site decides for itself.
//!     boot(&endowment, initrd_len, fs_rights, None)
//! }
//! ```
//!
//! Reading that struct literal is meant to tell you the complete authority of the system about to
//! exist, which is why the fields are named for what they *are* rather than numbered. Note what is
//! not in it: no network and no second budget.
//!
//! # What the kernel hands over, and what this builds from it
//!
//! [`BootEndowment`] names the capabilities the kernel granted: a construction budget, the UART's registers
//! as a device capability, the UART receive interrupt, a read-only view of the wall clock, when
//! this boot attached a RedoxFS disk the file service and the page its clients share with it, and,
//! when a GPU and a keyboard are both attached (milestone 177, option A), `display_terminal`'s own
//! served endpoint and output page, plus the endpoint the keyboard driver already `CALL`s. From
//! those, and nothing else, [`boot`] builds the whole interactive system out of its own budget:
//!
//! 1. **output**: the **console** server, reading text from a shared page and writing it to the
//!    UART, when this boot has no graphical terminal stack. When it does, there is no console
//!    server at all: `display_terminal` was already built kernel-side, before this process existed,
//!    and `line_editor` prints through it directly instead.
//! 2. **keystrokes**: the **input** driver, waiting on the UART receive interrupt and forwarding
//!    bytes, in the same "no graphical stack" case. When there is one, `kbd` plays this role
//!    instead, also built kernel-side, `CALL`ing `line_editor`'s own endpoint directly rather than
//!    going through this process at all (option A's whole point: no compositor in this path).
//! 3. the **line discipline** (`line_editor`, milestone 28): editing, echo, history, between
//!    whichever pair of the above this boot has. The same server either way; only its output-side
//!    wire shape changes (`mode`, `user/src/line_editor.rs`'s own `MODE_CONSOLE`/`MODE_DISPLAY`).
//! 4. the **shell**: prints and reads lines through the terminal endpoint, runs commands, and since
//!    milestone 86 holds a `READ` view of the wall clock so `time <command>` can measure one;
//! 5. the **terminal's sink adapter** (`terminal_sink_caretaker`, milestone 50), when the archive
//!    carries one: it holds the terminal and serves the sink contract, so a declared second stream
//!    can be pointed at the screen without handing anyone the endpoint that also reads the keyboard;
//! 6. the **undertaker** (`job_undertaker`), which collects a finished job's corpse so its region
//!    comes back to the pool.
//!
//! They are wired together with endpoints and shared pages this code creates. The kernel wires none
//! of it. Then init stays alive as the spawn service (milestone 31): the shell resolves a `run` into
//! a grant expression and directs init to load the named program and endow it with exactly what the
//! command named. Nothing here names an architecture: the console and input drivers hold the one
//! device-specific fact (the UART register layout), and the kernel grants the right device.
//!
//! # What it gives away once the system is up (milestone 22, the interactive increment)
//!
//! It used to hold the kernel's whole construction budget for life, which made every process in the
//! system one bug in init away from being built wrong. It no longer does. Once the boot servers
//! above are built it carves two bounded budgets off that root and **deletes the root**:
//!
//! - [`INIT_OWN_PAGES`] for its own scratch page tables, which is all it spends on itself; and
//! - [`JOBS_BUDGET_PAGES`] for the jobs the prompt asks for, one reclaimable region per job.
//!
//! It also gives up the UART device capability and the UART interrupt as soon as the drivers that
//! need them are built, the file service as soon as the shell holds it, and everything in
//! [`BootEndowment::for_test_roles`] with them. The proof is a negative control taken from inside the process and
//! printed at the prompt, exactly the shape `root_supervisor` uses: after the delete, `RETYPE` and
//! `RETYPE_OBJ` on that slot must answer `NoSuchSlot` (there is nothing there) rather than
//! `NotPermitted` (there is, and you may not).
//!
//! The job budget is **renewable**, which is what makes bounding it cheap. Every job is built in its
//! own region split off [`JOBS_BUDGET_PAGES`] and is born supervised: `job_undertaker`, a process
//! holding one endpoint capability and nothing else, collects each corpse through `Rendezvous::REAP`
//! (DECISIONS §32) and the region's pages come back here (§13: a reclaimed region returns to its
//! owner, which is whoever split it). Before that, a spawned job's memory was spent for the life of
//! the boot.
//!
//! # What it refuses to load (milestone 104)
//!
//! The kernel measures the one program *it* loads, which is this one. Everything else in the
//! archive is loaded here, and those bytes used to be unchecked, so the chain of trust stopped at
//! init's entry. It does not now. The build packs a table of digests into the archive
//! ([`measured_boot::PROGRAM_MEASUREMENTS`]), the kernel's trust root vouches for that table exactly
//! as it vouches for this program's own bytes, and [`boot`] looks every program up in it before
//! loading it.
//!
//! **One rule: init runs nothing it cannot vouch for.** A digest that does not match is a refusal,
//! and so is a name the table does not mention, for the reason the kernel's empty trust root is
//! refused: a check that passes when there is nothing to check against is not a check.
//!
//! What a refusal *costs* is not a second policy. It falls out of what the program was for, which is
//! a question this crate already had to answer for an archive entry that is simply missing, so a
//! refused program is treated exactly as a missing one:
//!
//! - **console, input, `line_editor`, swish, `job_undertaker`.** The system is made of these, so not
//!   running one and not having a system are the same outcome. init prints which one it refused and
//!   traps, which is `kernel::trust::require`'s decision one link down.
//! - **`terminal_sink_caretaker`.** Already optional: a boot without an adapter comes up and a
//!   declared second stream finds an empty slot. A refused adapter costs that same feature.
//! - **The spawnable programs.** Left out of the table `spawn_service` indexes, so the prompt
//!   answers "could not spawn" for them and everything else still works. Halting a running machine
//!   because `wc` changed would turn a build defect into an unbootable system, and it buys nothing:
//!   the guarantee is that nothing unvouched-for runs, and not spawning it is that guarantee.
//!
//! Recording a mismatch and loading anyway was considered and refused. There is no audit log to put
//! a record in, and a measurement that changes nothing about what runs is theatre; a chain whose
//! second link is advisory is not a chain.
//!
//! # BUGS
//!
//! **A refused `console` or `line_editor` stops in silence.** Those two are what carry init's
//! output, so a refusal of either has no route to a person: init traps and the operator sees the
//! kernel's fault line for init and nothing else, indistinguishable from any other early init
//! failure. Everything refused after them is named on the console. There is no debug-print syscall
//! (the kernel-served `Console` object went away at milestone 8) and driving the UART from here
//! would be a second copy of the console driver, per ISA, inside the process the drivers exist to
//! keep small.
//!
//! **An absent required component still traps with no message**, unchanged from before this
//! existed. That is a build that did not pack it rather than bytes somebody swapped, and it has
//! never had one.
//!
//! **The measurement is of the archive, not of memory over time.** It is checked once, when the
//! program is loaded. Nothing re-measures a running process, and nothing measures the pages init
//! wrote into a child after `build_child` copied them.
//!
//! The return of pages is **LIFO** (§16, `crates/regions`): a job region that is not at the top of
//! the budget's watermark when it is reclaimed returns nothing, and its run is a hole until this
//! process dies, which it never does. Sequential commands at a prompt are exactly LIFO and recover
//! fully; two jobs alive at once (a pipeline stage that outlives its producer) permanently costs one
//! region. A long enough session of concurrent pipelines still ends at "could not spawn".
//!
//! The loader's scratch window is never unmapped, so init keeps a **writable mapping of every page
//! it ever laid down for a child**. Reaping a job undoes that (region reclaim revokes every mapping
//! of the pages first, §13), but the boot servers are never reclaimed, so init can still read and
//! write the console's, the line editor's, the input driver's, the shell's and the sink adapter's
//! memory. Giving the construction budget away does not reach that, and nothing in the ABI unmaps a
//! page.
//!
//! Printing the negative control costs one more of those: the shell's output frame stays mapped here
//! for life, because there is no unmap and `PageFrame::REVOKE` would take it from the shell too.
//!
//! **Init's capability table has sixteen slots, and running out of them prints nothing at all.** Every
//! capability held across a `build_child` is one the child's address space, frames and TCB cannot
//! have, and `build_child` answering `Err(())` is a silent halt. Two of the three evenings this file
//! has cost were that, once when the kernel grew two grants and once when a boot component was built
//! one step too early. The order below is load-bearing and the comments say where.
//!
//! **It is now nine at rest and fifteen at peak, which is one slot from the wall** (milestone 31
//! phase 3, 2026-08-17). Keeping the file service and its shared page for the life of the boot took
//! the resting endowment from seven to nine, and a directory-granted spawn adds the job region, the
//! narrowed endpoint, the readiness endpoint, and a `build_child` retyping an address space and a
//! TCB. Anything that wants a tenth permanent capability here has to buy it from something, and the
//! honest candidates are the readiness endpoint (it could be retyped after the caretaker's build
//! rather than before, if the caretaker learned to take it another way) and the file page (nothing
//! but a second frame per grant retires it, which is `notes/shared-page-audit.md`'s proposed lane).
//!
//! Name: ratified 2026-08-04 (calef, milestone 96), and it is the ratification that raised
//! milestone 115. Refused `system_builder` (milestone 63 had already refused it, for a reason still
//! true: `builder.rs` calls itself "a minimal init: the system builder", so two programs would
//! claim one phrase) and `system_bootloader` (it claims a position in the boot sequence it does not
//! occupy, and milestone 88 will need the real one). A lane proposed `system_builder` anyway and
//! the maintainer endorsed it, because that refusal lived in one table cell inside one milestone
//! block and neither of them found it. The type this crate exports as `BootEndowment` was ratified
//! the same day, replacing `Grants`.

use grant_plan::{Prog, spawnproto};
use line_editor::proto;
// The loader, and the tree's only one since milestone 96. Named here rather than qualified at every
// call, because the point of the crate is that there is one of these.
use supervision_proto::{
    ChildEndowment, build_child, retype_obj_from as retype_obj,
    retype_page_frame_from as retype_page_frame, thread_control_block_start,
};
use user_rt::{call, cap_delete, granted, invoke, recv, recv_cap, send};

/// **The capabilities the kernel granted this init, by slot.** The one thing the two boards do not
/// agree on, so it is data each init states rather than code this crate repeats.
///
/// The two orders come from `kernel::user::spawn_init` (aarch64) and `kernel::user::riscv_shell_boot`
/// (riscv64). They differ because the aarch64 path is shared with milestone 19d's test roles, which
/// were granted a report endpoint and a test interrupt this system has no use for; see
/// [`for_test_roles`](BootEndowment::for_test_roles).
pub struct BootEndowment {
    /// The construction budget, held `WRITE | GRANT`: everything this system is made of.
    pub untyped: u64,
    /// The UART's registers, a device capability to map into the console and input drivers.
    pub uart_dev: u64,
    /// The UART receive interrupt, an `Irq` capability to delegate into the input driver.
    pub uart_irq: u64,
    /// **The wall clock** (milestone 51's wiring): a `PageFrame` capability with `READ` and nothing
    /// else, granted ahead of the filesystem pair so its slot is the same on every boot, whether or
    /// not a disk was attached, and granted **unconditionally**: a boot with no clock service hands
    /// us a zeroed page, which reads as `clock_proto::state::UNKNOWN` and is the honest answer for a
    /// machine that does not know the time. init hands it on only to a child whose manifest declares
    /// a clock, and hands on `READ`, so nothing spawned from this prompt can set the time
    /// (DECISIONS §43).
    ///
    /// **The shell holds a narrowed view of this same frame** (milestone 86), mapped at
    /// `SH_CLOCK_VA`, which is what `time <command>` measures with. There is deliberately no
    /// second field for it: the shell's clock is not a separate kernel grant but this one handed on,
    /// so a field would ask each board to state the same slot number twice with nothing checking
    /// that the two agree. `READ` and no `GRANT` there too, so the shell can read the wall clock and
    /// can hand one to nothing it spawns.
    pub clock_page: u64,
    /// **The inert-configuration page** (milestone 47's environment-variable fork, DECISIONS §111):
    /// a `PageFrame` capability with `READ` and nothing else, granted ahead of the filesystem pair
    /// for [`clock_page`](BootEndowment::clock_page)'s own reason (so its slot is the same whether
    /// or not a disk was attached), and granted **unconditionally** for the same reason too: this
    /// boot's fixed defaults (`TZ=UTC`, `LANG=C`, `TERM=dumb`) are assembled once, before init
    /// exists, so every boot hands init a real, validated page rather than making the slot's
    /// presence depend on some other component having started (`kernel::user::boot_config_page`,
    /// the same shape as `boot_clock_page`, minus the service: nothing here runs, so there is no
    /// readiness handshake to wait on, only a page to assemble and write once).
    ///
    /// init hands it on only to a child whose manifest declares [`grant_plan::Manifest::config`],
    /// and hands on `READ`, so nothing spawned from this prompt can change what a shell hands its
    /// own children.
    pub config_page: u64,
    /// **The filesystem, when this boot has one** (milestone 50). The kernel wires the block server
    /// and the FS server before it starts us and grants the service endpoint plus the page its
    /// clients share with it. The rights that endpoint carries arrive in `fs_rights`, and **0 means
    /// this boot attached no RedoxFS disk**, in which case these two slots hold nothing at all.
    pub fs_ep: u64,
    /// The page the file service's clients share with it; see [`fs_ep`](BootEndowment::fs_ep).
    pub fs_page: u64,
    /// **A virtio-rng device, when this boot has one** (DECISIONS §120's 2026-08-26 amendment:
    /// "grant the QEMU-only virtio-rng stopgap"). The confined transport; `WRITE | GRANT`, so this
    /// process can delegate it to an entropy service it builds. **Absent** on real hardware
    /// (milestone 55's actual target has none) or a run with `NIFE_RNG` unset, in which case this
    /// slot holds nothing at all, the same "0/empty means absent" shape [`fs_ep`](BootEndowment::fs_ep)
    /// already carries; [`boot`] probes for it (`invoke`'s own `NoSuchSlot` on an ungranted slot)
    /// rather than being told, because there is no fourth `START` argument word left to tell it
    /// with (`role`, `initrd_len`, `fs_rights` already spend all three).
    pub virtio_rng: u64,
    /// The device's completion interrupt; see [`virtio_rng`](BootEndowment::virtio_rng). `READ |
    /// GRANT`, routed by the kernel but left unenabled on riscv64 (`kernel::user::VirtioRngGrant`'s
    /// own doc: the board's hart-lottery hazard means only the kernel, which knows the true boot
    /// hart, may enable it, and it already did before granting this).
    pub virtio_rng_irq: u64,
    /// **The DMA page the kernel wrote `dma_phys` into**, at its own last eight bytes; see
    /// [`virtio_rng`](BootEndowment::virtio_rng). `READ | WRITE | GRANT`: this process maps it to
    /// read that value back out (entropy needs its own DMA region's physical base as a plain
    /// value; no capability exposes one), then delegates the same frame to the entropy service it
    /// builds, unread by anything else in between.
    pub virtio_rng_dma: u64,
    /// **`display_terminal`'s own served endpoint, when this boot has a graphical terminal stack**
    /// (milestone 177, option A). The kernel builds the virtio-gpu driver and `display_terminal`
    /// itself, before this process exists (`kernel::user::boot_graphical_terminal`, mirroring
    /// [`fs_ep`](BootEndowment::fs_ep)'s own shape: a virtio-gpu device alone needs eleven
    /// capability-table slots, which does not fit this process's own budget). `WRITE | GRANT`, so
    /// this process can hand it to `line_editor` as the endpoint it prints through in place of the
    /// console. **Absent** (holds nothing) on a boot with no GPU, no keyboard, or no
    /// `display`/`display_terminal`/`kbd` program in the archive, the same "0/empty means absent"
    /// shape [`fs_ep`](BootEndowment::fs_ep) already carries; [`boot`] probes for it the same way.
    pub disp_term_ep: u64,
    /// The physical page shared with `display_terminal`, written before an `OP_WRITE` on
    /// [`disp_term_ep`](BootEndowment::disp_term_ep); see that field's own doc for when this is
    /// absent. `READ | WRITE | GRANT`.
    pub disp_term_page: u64,
    /// **The endpoint the keyboard driver already holds `WRITE` (`CALL`) on**, when this boot has
    /// one (milestone 177, option A). The kernel spawns the keyboard driver kernel-side too, wired
    /// to this endpoint at its own spawn time, for a reason distinct from the GPU's: its target is
    /// `line_editor`'s own served endpoint, which does not exist until this process builds it, and
    /// a driver this process spawns can only be wired to capabilities it already holds. Granting
    /// `READ | WRITE | GRANT` here lets this process delegate `READ` to `line_editor` (as its own
    /// terminal endpoint, in place of a self-created one) and `WRITE` to `swish`, exactly the two
    /// views the plain-console boot already carves out of a self-created endpoint of the same
    /// shape. Absent in the same sense as
    /// [`disp_term_ep`](BootEndowment::disp_term_ep); the two are granted together or not at all.
    pub kbd_ep: u64,
    /// **Capabilities the kernel granted that the interactive system never uses**, deleted with the
    /// device authority once the drivers exist.
    ///
    /// Empty on riscv64. On aarch64 it is the kernel's report endpoint and the milestone-19d.2b test
    /// SGI, both of which exist because that boot path is shared with the test roles: nothing
    /// receives on the report here, and no interactive component waits on that interrupt. An init
    /// that kept them would be keeping delegable authority for no reason, which is the same kind of
    /// thing the construction budget is.
    pub for_test_roles: &'static [u64],
}

/// **A second, disjoint directory capability for the shell** (milestone 154's "wiring a second
/// grant into the real boot", design/roadmap/154-multi-directory-namespace.md), passed to [`boot`]
/// alongside [`BootEndowment`] rather than folded into it: this is not a kernel grant like every
/// field above, it is something [`boot`] itself constructs (a second `fs_subtree_caretaker`,
/// narrowing the same file service [`BootEndowment::fs_ep`] already names) out of capabilities the
/// kernel already granted.
///
/// **`None` at every real entry point today.** The mechanism here is real and reachable through
/// the one function both boards' real inits call (`user/src/system_initializer.rs`,
/// `user/src/hello.rs`'s `init_boot` role), not a synthetic kernel-side test harness. What it does
/// not decide is *what* the second subtree should be: [DECISIONS
/// §126](../../../design/decisions/126-two-directory-cwd.md) named that a boot-time policy
/// question reserved for calef, so no shipped boot enables it. A second, separate gap: nothing
/// yet tells the shell process *which* label and cspace slot this landed at (the `START` ABI's
/// three words are already spoken for by the role, the argument, and the clock slot), so a shell
/// built with one of these today would hold a capability its own `Nav` has no way to learn about.
/// Provisional shape.
#[derive(Clone, Copy)]
pub struct SecondDirGrant {
    /// One component under the image root, the same shape a `DirGrant`'s `name` already takes.
    pub name: &'static str,
    /// The `filesystem_proto::dir` rights the caretaker asks for on its descent.
    pub rights: u64,
}

/// Where the kernel maps the initrd archive, read-only. Must match `kernel::user::INITRD_VA`.
const INITRD_VA: u64 = 0x2000_0000;

/// Stack pages every child init builds gets, mapped down from `supervision_proto::CHILD_STACK_VA`.
///
/// **Twelve since DECISIONS §67**, and every step of that number was measured rather than chosen.
/// Four overflowed at the first `ls > out.txt`; eight held until `2>` put a **second** `FileOut` on
/// `run_pipeline`'s frame, each carrying a 256-byte staging buffer by value, and the scripted wiring
/// faulted twenty-four bytes below its lowest page. Four extra rather than one, because every
/// previous instance bought exactly enough and the next change found the wall again. The cost is
/// 48 KiB of address space per child, which is nothing next to a page table.
///
/// `kernel::user::pipeline_service`'s `SHELL_EXTRA_STACK` must stay level with this: a test wiring
/// with less headroom than the boot wiring finds faults the boot does not have (notes/pipes.md).
/// The kernel cannot depend on this crate (it would drag `user_rt`'s EL0 syscall stubs in), so that
/// one is still a number in two places, and this is the one it follows.
pub const CHILD_STACK_PAGES: u64 = 12;

/// Where a child that declares a clock maps it, read-only. Must match `user/src/date.rs`'s
/// `CLOCK_VA` and `kernel/src/user/clock_service.rs`.
const CHILD_CLOCK_VA: u64 = 0x00c0_0000;

/// Where a child that declares the inert-configuration page maps it, read-only (DECISIONS §111).
/// Must match `user/src/printenv.rs`'s `CONFIG_VA`. A different address from `std_service.rs`'s
/// `CONFIG_PAGE_STD`: that std program is spawned by a different wiring entirely (the `-Zbuild-std`
/// farm's own harness), in its own address space, so there is no collision to avoid, only two
/// numbers that happen not to need to agree.
const CHILD_CONFIG_VA: u64 = 0x00e0_0000;

/// Where a supervised (interruptible) child maps its shared job frame (DECISIONS §24). Below the ELF
/// load address (`0x40_0000`) and the stack; must match heeder.rs / spinner.rs's `JOB_PAGE_FRAME_VA`.
const CHILD_JOB_PAGE_FRAME_VA: u64 = 0x0030_0000;

/// Pages of untyped split off our own budget and handed the shell (milestone 31), so the shell can
/// in turn endow the programs it spawns (`run --mem N`) out of a budget that is genuinely *its own*.
/// The shell shrinks this by N pages per grant; the pages a spawned child pins are not reclaimed in
/// phase 1, so this is a session budget, not a renewable one. Must match swish.rs's
/// `SH_BUDGET_PAGES`.
const SH_BUDGET_PAGES: u64 = 128;

/// **What init keeps for itself after the boot servers are up** (milestone 22, the interactive
/// increment). It pays for one thing: the page tables reaching the loader's scratch window, which
/// are init's own mappings and must never come out of a child's region (tearing that region down
/// would free init's tables under a window it never unmaps). One L3 covers 512 scratch pages and a
/// job maps at most a couple of dozen, so this is thousands of commands' worth.
pub const INIT_OWN_PAGES: u64 = 128;

/// **One job's region**: everything a spawned program is made of, so a single reclaim frees all of
/// it. The biggest program the prompt can spawn is `date` at seven pages, plus
/// [`CHILD_STACK_PAGES`], a TCB, an address-space root, the intermediate tables for the four windows
/// a child touches, and the §13 mapping records. Forty is that with room to spare, and it is spent
/// per *live* job rather than per job ever run.
const JOB_REGION_PAGES: u64 = 40;

/// **One directory-granted job's region**: the program *and* the `fs_subtree_caretaker` that carries
/// its grant, plus the two endpoints between them, all out of one carve.
///
/// One region rather than two is DECISIONS §92 read through §40's mechanism. A caretaker's serve loop
/// never returns, so it never dies of its own accord; built in a region of its own, that region never
/// comes home and §16's LIFO rule then pins the region above it too, and six `rm`s would end the
/// prompt. Built out of the region it serves, it is inside the client's subtree, and the one reclaim
/// `job_undertaker` already performs ends both. **The two endpoints are retyped from this region too,
/// and that is load-bearing rather than tidy**: `sched::reap_region_objects` sweeps a region's
/// endpoints before it looks at its threads, and that sweep is what wakes a caretaker parked in
/// `RECV` so it can be collected. An endpoint carved from init's own budget would leave it blocked on
/// something the teardown never touches, and a blocked thread never reaches the `schedule()` that
/// spends §16's kill.
///
/// **Ninety-six rather than eighty**, and the extra is not slack for its own sake: the caretaker is
/// a second address space with its own tables and its own stack, and the failure mode of getting it
/// wrong is `build_child` answering `Err(())` mid-boot-command, which reads at the prompt as "could
/// not spawn" with no way to tell a small region from an empty pool.
const DIR_JOB_REGION_PAGES: u64 = 96;

/// The stack a `fs_subtree_caretaker` gets, beyond the one page `build_child` maps for it.
///
/// Four rather than [`CHILD_STACK_PAGES`]'s twelve, and it is a measurement of the program rather
/// than a guess: it has no allocator, no recursion, and one frame that matters, a
/// `[Option<u64>; 16]` handle table plus a `grant::MAX_NAME` name buffer. Twelve pages per caretaker
/// would be 32 KiB of a region this crate is already sizing carefully.
const CARETAKER_STACK_PAGES: u64 = 4;

/// Where a `fs_subtree_caretaker` and the program it serves both map the FS contract's shared page.
/// Must match `user/src/fs_subtree_caretaker.rs`'s `PAGE_VA` and `user/src/rm.rs`'s.
///
/// One address for both because they are two ends of one contract and neither is the other's parent:
/// a request travels caretaker-to-server and program-to-caretaker through the same frame, so a
/// second VA would only be a second name for the same page.
const FS_CLIENT_PAGE_VA: u64 = 0x0060_0000;

/// **One shell-boot second-directory caretaker's region** (milestone 154's "wiring a second
/// grant into the real boot"). Sized for one caretaker alone, the way [`CARETAKER_STACK_PAGES`]
/// already is: unlike [`DIR_JOB_REGION_PAGES`], nothing else is built out of this region, because
/// the confined program here is the shell itself, already built directly out of `ut` rather than
/// out of a region of its own. Conservative rather than tight (the same headroom
/// [`JOB_REGION_PAGES`] carries for one child), since this is spent once per boot rather than
/// once per command.
const SECOND_DIR_CARETAKER_PAGES: u64 = JOB_REGION_PAGES;

/// **The job pool.** Six live jobs at once, which is far more than a prompt has ever needed and is
/// deliberately small: the whole claim of this increment is that a *bounded* budget is enough once
/// the regions come back, so a budget nobody could exhaust would prove nothing. `script/shell-check`
/// runs thirteen jobs through it, so widening this silently retires that gate.
pub const JOBS_BUDGET_PAGES: u64 = JOB_REGION_PAGES * 6;

/// Where init maps the shell's output frame in **its own** address space, to print the one line it
/// ever prints (the dropped-authority negative control). Well clear of init's segments, its stack,
/// and the loader's scratch window at `0x1000_0000`.
const INIT_OUT_VA: u64 = 0x0f00_0000;

/// Where init briefly maps the virtio-rng DMA page, in **its own** address space, to read
/// [`RNG_DMA_PHYS_OFFSET`] back out before handing the same frame on to entropy. Distinct from
/// [`INIT_OUT_VA`] and never unmapped (this file's own BUGS: there is no unmap in the ABI), the
/// same permanent-scratch cost that address already carries.
const RNG_DMA_PEEK_VA: u64 = 0x0f10_0000;

/// The DMA region's own physical base, written inside the page itself at its last eight bytes
/// (`kernel::user::VIRTIO_RNG_DMA_PHYS_OFFSET`; the two constants must agree, and the kernel-side
/// one carries the reasoning for exactly this offset). Named separately here because reading it
/// happens in this crate and writing it happens in the kernel; there is no crate the two could
/// share it through (rule 7's own carve-out: this is a kernel/init boot convention, one program's
/// bytes handed to another it spawned, not a contract between two peer user programs).
const RNG_DMA_PHYS_OFFSET: u64 = 4096 - 8;

/// Where entropy maps its own DMA page. Must match `user/src/entropy.rs`'s `DMA_VA`.
const RNG_DMA_VA: u64 = 0x0000_0000_0090_0000;

/// `entropy.rs`'s own spawn-argument convention (`user/src/entropy.rs`'s `MODE_VIRTIO`): the
/// kernel's (and now this crate's) shared understanding with the one program it spawns, not a wire
/// contract between two user programs, so rule 7 does not apply the way it does to `RNG_DMA_VA`.
const RNG_MODE_VIRTIO: u64 = 0;

/// `line_editor.rs`'s own spawn-argument convention (`user/src/line_editor.rs`'s `MODE_CONSOLE`):
/// [`RNG_MODE_VIRTIO`]'s own reasoning, one program over. The pre-milestone-177 wiring: prints
/// through the console's bespoke two-endpoint protocol.
const LINE_EDITOR_MODE_CONSOLE: u64 = 0;
/// `line_editor.rs`'s own `MODE_DISPLAY`: milestone 177's wiring, prints through
/// `display_terminal`'s `OP_WRITE`/one-`CALL` contract instead.
const LINE_EDITOR_MODE_DISPLAY: u64 = 1;

// The VAs each program hardcodes; they must match console.rs / input.rs / line_editor.rs / swish.rs.
const CON_SHARED_VA: u64 = 0x0060_0000; // console reads text here; line_editor writes it
const CON_UART_VA: u64 = 0x0070_0000; // console's UART mapping
const TERM_OUT_VA: u64 = 0x0080_0000; // line_editor reads the shell's text/prompts here
const TERM_IN_VA: u64 = 0x0090_0000; // line_editor delivers completed lines here
const IN_UART_VA: u64 = 0x00a0_0000; // input driver's UART mapping
const SH_OUT_VA: u64 = 0x00c0_0000; // the shell's view of the TERM_OUT frame (swish.rs OUT_VA)
const LINE_VA: u64 = 0x00b0_0000; // the shell's view of the TERM_IN frame
const SH_FS_VA: u64 = 0x0060_0000; // the shell's half of the FS contract (swish.rs FS_VA)

/// Where the **shell** maps its own read-only clock (milestone 86). Must match swish.rs's
/// `SH_CLOCK_VA`. A different address from [`CHILD_CLOCK_VA`], because that is where a *child* maps
/// its clock and the shell already maps the terminal's output frame there; two address spaces may
/// agree on an address, one may not.
const SH_CLOCK_VA: u64 = 0x00d0_0000;

// -------------------------------------------------------------------------------------------
// Milestone 49's login stack: credentialer, identity_provisioner, login, audit_sink.
// -------------------------------------------------------------------------------------------

/// Where `credentialer` maps its own provision page. Must match `user/src/credentialer.rs`'s own
/// `PROV_VA`.
const CRED_SVC_PROV_VA: u64 = 0x0000_0000_00e0_0000;
/// Where `credentialer` maps its own verify page. Must match the same file's `VERIFY_VA`.
const CRED_SVC_VERIFY_VA: u64 = 0x0000_0000_00e1_0000;
/// Where `login` maps its relay of the verify page (the same physical frame as
/// [`CRED_SVC_VERIFY_VA`], mapped into a different address space). Must match `user/src/login.rs`'s
/// own `CRED_VA`.
const LOGIN_CRED_VA: u64 = 0x0000_0000_00e3_0000;
/// Where `identity_provisioner` maps the identity/secret this boot stages for it. Must match
/// `user/src/identity_provisioner.rs`'s own `REQ_VA`.
const IDP_REQ_VA: u64 = 0x0000_0000_00e4_0000;
/// Where `identity_provisioner` maps `credentialer`'s provision page (the same physical frame as
/// [`CRED_SVC_PROV_VA`]). Must match the same file's own `PROV_VA`.
const IDP_PROV_VA: u64 = 0x0000_0000_00e0_0000;
/// Where `identity_provisioner` maps the file service's shared page. Must match the same file's own
/// `FS_VA`.
const IDP_FS_VA: u64 = 0x0000_0000_00e5_0000;
/// Where this process briefly maps the page it stages `identity_provisioner`'s request into, in its
/// own address space, before delegating the same physical frame on. Distinct from every VA above
/// (those are addresses inside a *child's* address space); in the same scratch family as
/// [`RNG_DMA_PEEK_VA`] and [`INIT_OUT_VA`].
const PROVISION_SCRATCH_VA: u64 = 0x0f20_0000;

/// `credentialer`'s own construction budget, in pages: matches
/// `kernel::user::credential_service::CRED_BUDGET_PAGES` (6 MiB, sized from `cred::Cost::DEFAULT`'s
/// own scratch requirement, not guessed).
const CRED_BUDGET_PAGES: u64 = 1536;
/// Extra stack pages `credentialer` needs, beyond the one page `build_child` maps: matches
/// `kernel::user::credential_service::CRED_STACK_PAGES`, itself a measurement (Argon2id's inner
/// loop overflows one page) rather than a guess.
const CRED_STACK_PAGES: u64 = 16;
/// `login`'s own construction budget under a real boot, in pages. Sized for a handful of real
/// sessions across the boot's whole life (`OWN_UT_PAGES` 128 + `CHANNEL_UT_PAGES` 32, both
/// `user/src/login.rs`'s own one-time costs, plus `CARETAKER_REGION_PAGES` 64 + `CLIENT_BUDGET_PAGES`
/// 64 per session that never logs out), not the much larger figure
/// `kernel::user::login_tests::CONSTRUCTION_PAGES` carries for a whole guest-test suite's worth of
/// logins against one shared instance.
const LOGIN_CONSTRUCTION_PAGES: u64 = 768;
/// Extra stack pages `login` needs, beyond the one page `build_child` maps: matches
/// `kernel::user::login_service::LOGIN_STACK_PAGES`, "sized against `credentialer.rs`'s own lesson"
/// (that file's own comment).
const LOGIN_STACK_PAGES: u64 = 16;

/// `credentialer.rs`'s own readiness sentinel, duplicated here the same way its `PROV_VA`/`VERIFY_VA`
/// already are: a binary crate cannot be imported, so every wiring site that needs to recognise this
/// word states it again (`kernel::user::credential_service`'s own `RPT_READY` is the same
/// duplication one level over). Must match `user/src/credentialer.rs`'s own `RPT_READY`.
const CRED_RPT_READY: u64 = 0x_c2ed_0000_0000_0001;
/// `identity_provisioner.rs`'s own success report code, duplicated for the same reason
/// (`kernel::user::identity_provisioner_service`'s own `RPT_OK` is the identical duplication).
/// Must match `user/src/identity_provisioner.rs`'s own `RPT_OK`.
const IDP_RPT_OK: u64 = 1;

/// **The demo identity this boot provisions**, once per boot, with a freshly generated password
/// (see `have_login_stack`'s own block for the reasoning): a role name rather than a specific
/// family member's, deliberately, because the whole point of a boot-generated credential is that
/// nothing here has to decide *whose* account this is (`design/roadmap/49-users-and-attribution.md`'s
/// own BUGS names this as the reason the generated shape was recommended over a baked-in one).
const DEMO_IDENTITY: &[u8] = b"operator";
/// How many raw bytes of entropy the generated password draws. 12 bytes (96 bits) hex-encoded to
/// [`PASSWORD_HEX_LEN`] printable characters: comfortably past what an online guesser could work
/// through before the boot that generated it is long gone, and short enough to type at a keyboard.
const PASSWORD_BYTES: usize = 12;
/// [`PASSWORD_BYTES`] hex-encoded, two characters per byte.
const PASSWORD_HEX_LEN: usize = PASSWORD_BYTES * 2;

/// Draw `out.len()` bytes from the entropy service through `request`
/// ([`entropy_proto::MAX_BYTES`] at a time), the identical loop `user/src/credentialer.rs`'s own
/// `fill` performs as a *client* of that same service (this process is, briefly, one too: it draws
/// the generated password's own raw bytes before `credentialer` ever exists). `false` when the
/// service could not supply them, which the caller treats as fatal to generating a password at all
/// -- the same "never invent a weak one" posture `credentialer.rs`'s own `fill` documents for a
/// salt.
fn fill_entropy(request: u64, out: &mut [u8]) -> bool {
    let mut done = 0;
    while done < out.len() {
        let want = (out.len() - done).min(entropy_proto::MAX_BYTES as usize);
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method
        // before acting.
        let (r0, r1) = call(
            request,
            entropy_proto::req(entropy_proto::GET, want as u64),
            0,
        );
        let Some(n) = entropy_proto::delivered(r0) else {
            return false;
        };
        if n < want {
            return false;
        }
        done += entropy_proto::take(n, r1, &mut out[done..]);
    }
    true
}

/// Hex-encode `bytes` into `out` (`out.len() == 2 * bytes.len()`), lowercase: the shape a cloud
/// image's generated first-boot password already takes, printable and typeable at a keyboard
/// without a decoder.
fn hex_password(bytes: &[u8], out: &mut [u8]) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for (i, &b) in bytes.iter().enumerate() {
        out[i * 2] = DIGITS[(b >> 4) as usize];
        out[i * 2 + 1] = DIGITS[(b & 0xf) as usize];
    }
}

/// **Build the interactive system and become its spawn service.** Never returns: the last thing it
/// does is park in `RECV` on the shell's spawn channel for the life of the boot.
///
/// `initrd_len` is the archive length the kernel passed at entry; `fs_rights` is the `filesystem_proto::dir`
/// rights the file-service endpoint carries, and 0 means this boot attached no disk. `second_dir`
/// is milestone 154's addition: `Some` hands the shell a second, disjoint directory capability
/// (see [`SecondDirGrant`] for what this does and does not decide); every real entry point passes
/// `None` today.
pub fn boot(
    g: &BootEndowment,
    initrd_len: u64,
    fs_rights: u64,
    second_dir: Option<SecondDirGrant>,
) -> ! {
    // The archive the kernel mapped read-only; its length arrived at entry.
    // SAFETY: the kernel mapped `initrd_len` bytes of reserved RAM, read-only, at INITRD_VA. It is
    // reserved memory that outlives every process, so the borrow is honest for the whole boot.
    let archive =
        unsafe { core::slice::from_raw_parts(INITRD_VA as *const u8, initrd_len as usize) };
    let Ok(fs) = nifefs::Fs::parse(archive) else {
        fail()
    };

    // **The table init measures what it loads against** (milestone 104). The kernel vouched for this
    // entry before it started us, exactly as it vouched for our own bytes
    // (`kernel::trust::require_program_measurements`), so the table is worth what this process is
    // worth and the chain extends by induction rather than by widening the kernel.
    //
    // Bytes that are not UTF-8 become the **empty** table rather than a fault, and an empty table
    // vouches for nothing: every lookup below answers `Unmeasured`, the console is refused, and the
    // boot stops. That is the same direction to be wrong in as the kernel's empty trust root, and it
    // is the failure mode a measured boot must never get backwards.
    let table = fs
        .read(measured_boot::PROGRAM_MEASUREMENTS)
        .and_then(|b| core::str::from_utf8(b).ok())
        .unwrap_or("");

    let con_elf = measured(&fs, table, "console");
    let in_elf = measured(&fs, table, "input");
    let td_elf = measured(&fs, table, "line_editor");
    let sh_elf = measured(&fs, table, "swish");
    // **The terminal's sink adapter** (milestone 50's last remainder). Optional on purpose: an
    // initrd built without it still boots, and a program that declares a second stream then finds
    // an empty slot and says what it has to say in-band. A missing component should cost a feature,
    // not a prompt. An adapter the table refuses costs exactly the same feature, which is the whole
    // policy in one line: init treats what it cannot vouch for as what is not there.
    let sink_elf = measured(&fs, table, "terminal_sink_caretaker").elf;
    // **The entropy service** (DECISIONS §120's 2026-08-26 amendment), optional in exactly the
    // adapter's own sense: a boot with no `entropy` program in its initrd, or a table that refuses
    // it, simply never builds the service, and [`g.virtio_rng`](BootEndowment::virtio_rng)'s own
    // probe finds nothing to build it *from* either way on most boots (real hardware, or a run
    // with `NIFE_RNG` unset). Neither case is a broken boot, so neither belongs with the required
    // three below.
    let ent_elf = measured(&fs, table, "entropy").elf;
    // **Milestone 49's login stack** (`credentialer`, `identity_provisioner`, `login`,
    // `audit_sink`): optional in exactly the entropy service's own sense, and gated on it too --
    // there is no salt, no password and no credential store without real entropy, so a boot with
    // no entropy service has no login path either, the same way it has no login path with no
    // filesystem (`have_login_stack`, below, checks both). Read here with the rest of this
    // pass, not built yet: building happens later, after the shell's own construction has
    // returned this table to its resting count (see that block's own comment for why the timing
    // matters).
    let cred_elf = measured(&fs, table, "credentialer").elf;
    let idp_elf = measured(&fs, table, "identity_provisioner").elf;
    let login_elf = measured(&fs, table, "login").elf;
    let audit_elf = measured(&fs, table, "audit_sink").elf;
    // The undertaker (milestone 22, the interactive increment). Read here with the rest, because
    // the archive is only readable while we hold it and every failure below is one `fail`. Required
    // rather than optional, unlike the adapter above: without it a bounded job pool fills and the
    // prompt stops spawning, which is a broken system and not a missing feature.
    let reaper_elf = measured(&fs, table, "job_undertaker");
    // **The subtree caretaker** (milestone 31 phase 3), which is not a boot component and not a
    // spawnable program: it is what a *directory grant* is made of, one per invocation, so it is
    // read here with the rest and built by the spawn service far below.
    //
    // Optional in exactly `terminal_sink_caretaker`'s sense, and the missing-component rule decides
    // it rather than a second policy: without one, a directory grant cannot be delivered and the
    // prompt says so, which costs `rm` and nothing else. A refusal by the measurement table costs
    // the same, because init treats what it cannot vouch for as what is not there.
    let care_elf = measured(&fs, table, "fs_subtree_caretaker").elf;

    // **The programs the shell can spawn** (milestone 31), measured and parsed here rather than
    // after the giveaway: the announcement further down is the only thing init ever says, so the
    // verdicts have to exist before it. One `Option<elf::Elf>` is five words, so moving the whole
    // table up the frame costs a few hundred bytes and buys a person being told at boot instead of
    // at the prompt. (It said "seven of them" while `PROG_COUNT` was nine and then ten; a count
    // written beside a constant that moves is a count that goes stale, so this one is not written.)
    //
    // The refusals are collected in the same pass, because a second pass would hash every program a
    // second time. Only entries the archive **has** count as refusals: `rm` is deliberately not
    // loadable here, and a program this initrd never packed was never spawnable, so neither is news.
    let mut progs: [Option<elf::Elf>; grant_plan::PROG_COUNT] = core::array::from_fn(|_| None);
    let mut refused = [""; grant_plan::PROG_COUNT];
    let mut refused_n = 0usize;
    for (id, slot) in progs.iter_mut().enumerate() {
        let Some(name) = Prog::from_id(id as u64).and_then(archive_name) else {
            continue;
        };
        let found = measured(&fs, table, name);
        if found.unvouched {
            refused[refused_n] = name;
            refused_n += 1;
        }
        *slot = found.elf;
    }

    let ut = g.untyped;

    // **Freed here rather than after the console, line discipline and input are all built**
    // (milestone 47, DECISIONS §111's config-page lane): `for_test_roles` is dead weight on this
    // boot from the instant `g` exists (it names capabilities milestone 19d's test roles use and
    // the interactive system never touches), and this capability table has only sixteen slots, a
    // margin this file's own comment two screens down already documents as having broken once
    // (milestone 50's two extra kernel grants left "no slot left" for the shell's `build_child`).
    // Adding a fourth permanently-held kernel grant (the config page, alongside the clock page and
    // the filesystem pair) reproduced exactly that failure during the console/line-discipline/input
    // build, silently, because nothing between here and the old deletion point ever reads a
    // `for_test_roles` capability's contents: only its *slot* matters, and freeing the slot two
    // component-builds earlier costs nothing this boot was using.
    for &c in g.for_test_roles {
        cap_delete(c);
    }

    // **Graphical, when this boot has a GPU and a keyboard both attached** (milestone 177, option
    // A). Probed the same way the virtio-rng trio is (`user_rt::granted`, since there is no
    // fourth `START` argument word left to be told with instead): `disp_term_ep` and `kbd_ep` are
    // granted together or not at all (`kernel::user::boot_graphical_terminal`'s own contract), so
    // checking one stands for both.
    //
    // **Computed and acted on here, at the very top, not where it is first needed** (found by
    // hitting the wall the same way the paragraph below describes: bisection, not reasoning).
    // `disp_term_ep`/`disp_term_page`/`kbd_ep` are kernel grants, alive from spawn, so on a
    // graphical boot they inflate the resting baseline for the **whole** function exactly the way
    // the virtio-rng trio already does (this paragraph's own next one) -- and this function's
    // baseline was already tight enough that the virtio-rng trio's own three slots forced entropy
    // to move all the way to the top once. Three more permanent slots on top of that pushed the
    // *entropy* build itself past the wall (`must(build_child(...))` for `ent_program`, which
    // retypes an `AddressSpace` and a `THREAD_CONTROL_BLOCK` from this same table, first-free,
    // with only `for_test_roles`'s two freed slots to draw from). `uart_dev`/`uart_irq` are dead
    // weight on a graphical boot from this line on (no console, no input driver ever reaches
    // them), so freeing them here, before entropy spends anything, is what buys those two slots
    // back before the peak that needed them.
    let has_graphical = granted(g.disp_term_ep);
    if has_graphical {
        cap_delete(g.uart_dev);
        cap_delete(g.uart_irq);
    }

    // **The entropy service** (DECISIONS §120's 2026-08-26 amendment), when the kernel granted a
    // virtio-rng device and the archive carries a program for it. Built **here, before anything
    // else**, and that positioning is load-bearing rather than a style choice.
    //
    // An earlier version of this built entropy after the console and line discipline, in the gap
    // their own three capabilities free ("The console's three capabilities go back now"). That
    // reads as the more generous gap, but it counts the wrong thing: the virtio-rng trio is
    // granted by the *kernel*, at spawn, so it inflates the resting baseline for the **whole**
    // function, not just the block that builds it. The earliest peak this function ever reaches
    // (retyping `request`/`reply`/`term_ep`/`con_shared`/`term_out`/`term_in`, all six before
    // console is even built) is already tight against the sixteen-slot wall on its own account
    // (`crates/system_initializer`'s own BUGS: "one slot from the wall" describes the *shell's*
    // peak, and the terminal-plumbing peak just named is close behind it); adding three more
    // permanent slots to a baseline that peak already stresses pushed it over, and the boot
    // faulted building `term_in`, in total silence, with no route to a person (this file's own
    // BUGS on why a refusal this early has none). Found by bisection, not reasoning: an isolated
    // test granted init one single harmless extra capability, at an arbitrary slot, with no code
    // anywhere naming or using it, and the identical silent fault reproduced.
    //
    // Building here instead avoids the collision rather than widening anything: this is the
    // *only* point in the function where the virtio-rng trio's three slots and the terminal
    // plumbing's six are never both live at once. Entropy is built and its three slots are
    // released before `request` is ever retyped, so every peak downstream of this block is
    // exactly what it was before this amendment existed.
    //
    // **Absence is not a failure, twice over**: no device (real hardware, or `NIFE_RNG` unset) and
    // no `entropy` program in the archive both leave the system exactly as it was before this
    // amendment, the same "a missing component costs a feature, not a boot" posture the sink
    // adapter and the subtree caretaker already take.
    //
    // **`entropy_ready`, not an inline `announce`.** This process has no terminal yet at this
    // point in the boot (that is the whole reason the peak accounting above works out), so the
    // outcome is carried forward as data and said later, at the existing, already-mapped print
    // site right before `term_ep`/`term_out` are dropped (see that site's own comment).
    //
    // # BUGS (historical: resolved below, this update)
    //
    // **Used to prove only the device chain.** This block builds the service and confirms it drew
    // real bytes from the real device; `entropy_client`, below, is what changed: `request`'s
    // init-side copy is now kept (not `cap_delete`d) exactly when a client will need it, so
    // `credentialer` can be handed a working view of it further down.
    let mut entropy_ready = false;
    // **`request`'s init-side copy, kept only when there is a real client waiting for it**
    // (milestone 49's boot-wiring update). `None` on every path that used to `cap_delete` it
    // (device absent, mapping failed, the handshake did not answer `READY`); `Some(request)`
    // exactly once entropy is proven up, so the login stack below can delegate a working view of
    // it to `credentialer` without re-probing anything. Consumed (delegated once, then dropped) by
    // the login-stack block below; `cap_delete`d there instead if that block ends up skipping
    // entirely (no filesystem, or one of the four programs missing/unvouched).
    let mut entropy_client: Option<u64> = None;
    if let Some(ent_program) = ent_elf.as_ref() {
        // `invoke`'s own `NoSuchSlot` (-1) on an ungranted slot is the probe: there is no fourth
        // `START` argument word left to be told with instead (see
        // [`BootEndowment::virtio_rng`]'s own doc). `virtio_rng`/`virtio_rng_irq`/`virtio_rng_dma`
        // are granted as one triple or not at all (`kernel::user::boot_virtio_rng_device`), so a
        // negative answer here means all three slots are empty, not just this one.
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method
        // before acting; a probe against an empty slot is exactly what that contract is built to
        // answer safely.
        let magic = unsafe { invoke(g.virtio_rng, abi::virtio::READ_REG, 0, 0, 0) };
        if magic >= 0 {
            // SAFETY: as above.
            if unsafe {
                invoke(
                    g.virtio_rng_dma,
                    abi::page_frame::MAP,
                    RNG_DMA_PEEK_VA,
                    1,
                    ut,
                )
            } == 0
            {
                // SAFETY: just mapped read/write, one page, ours alone until entropy is built and
                // holds its own copy of the same frame; `RNG_DMA_PHYS_OFFSET` is inside it and
                // outside entropy's own ring-and-buffer layout (that constant's own doc).
                let dma_phys = unsafe {
                    core::ptr::read_unaligned(
                        (RNG_DMA_PEEK_VA as *const u8)
                            .add(RNG_DMA_PHYS_OFFSET as usize)
                            .cast::<u64>(),
                    )
                };
                let request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let ready = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let entropy = must(build_child(
                    ut,
                    ut,
                    ent_program,
                    &ChildEndowment {
                        caps: &[
                            (request, abi::rights::READ),
                            (g.virtio_rng_irq, abi::rights::READ),
                            (g.virtio_rng, abi::rights::WRITE),
                            (ready, abi::rights::WRITE),
                        ],
                        maps: &[(RNG_DMA_VA, g.virtio_rng_dma, abi::address_space::MAP_RW)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new()
                    },
                ));
                must_ok(thread_control_block_start(
                    entropy,
                    RNG_MODE_VIRTIO,
                    dma_phys,
                    0,
                ));
                cap_delete(entropy);
                cap_delete(g.virtio_rng_irq);
                cap_delete(g.virtio_rng);
                cap_delete(g.virtio_rng_dma);
                // Block for the service's own proof of life: it fetches a first bufferful from the
                // real device before answering, so this means "a client that asks will be
                // answered", not merely "the handshake completed" (`user/src/entropy.rs`'s own
                // doc).
                let (verdict, _, _) = recv(ready);
                cap_delete(ready);
                entropy_ready = verdict == entropy_proto::READY;
                if entropy_ready {
                    // Kept for the login stack below (`credentialer`'s own client view); see
                    // `entropy_client`'s own doc.
                    entropy_client = Some(request);
                } else {
                    // No login stack will be built without a working entropy service (this
                    // block's own comment on `have_login_stack`), so `request` has no further use
                    // on this path.
                    cap_delete(request);
                }
            } else {
                cap_delete(g.virtio_rng_irq);
                cap_delete(g.virtio_rng);
                cap_delete(g.virtio_rng_dma);
            }
        }
    }

    // **The two components that have to exist before init can say anything.** The console writes
    // the UART and the line discipline is its only client, so a refusal of either has no route to a
    // person and this is the one case that stops in silence (see this module's BUGS). Everything
    // else is checked below, after they are running.
    let (Some(con_elf), Some(td_elf)) = (con_elf.elf, td_elf.elf) else {
        fail()
    };

    // `has_graphical` and the early `uart_dev`/`uart_irq` free both already happened, at the top
    // of this function, before the entropy block: see that comment for why the timing is
    // load-bearing rather than tidiness.

    // The endpoints and shared pages we own and hand out, each retyped with full rights so we can
    // delegate narrowed views. `term_ep` is the terminal contract's one endpoint: the discipline
    // serves it; the input driver (or `kbd`) and the shell only hold WRITE on it, and neither can
    // tell what is on the other side (notes/terminal-contract.md). In the graphical case it is not
    // ours to create: `kbd` already holds `WRITE` on `g.kbd_ep`, fixed at its own kernel-side spawn
    // time, so that is the endpoint `line_editor` has to serve instead of a fresh one.
    let term_out = must(retype_page_frame(ut)); // shell -> line_editor text and prompts
    let term_in = must(retype_page_frame(ut)); // line_editor -> shell completed lines

    let term_ep = if has_graphical {
        g.kbd_ep
    } else {
        must(retype_obj(ut, abi::objtype::RENDEZVOUS))
    };

    if has_graphical {
        // No console, no input driver, no separate request/reply pair or shared page: `kbd` and
        // `display_terminal` are already running (built kernel-side, before this process existed),
        // and `line_editor` is their only client. It prints through `display_terminal`'s own
        // `OP_WRITE`/one-`CALL` contract (`g.disp_term_ep`/`g.disp_term_page`, `LINE_EDITOR_MODE_DISPLAY`)
        // instead of the console's bespoke two-endpoint one.
        let r = build_child(
            ut,
            ut,
            &td_elf,
            &ChildEndowment {
                caps: &[
                    (term_ep, abi::rights::READ),
                    (g.disp_term_ep, abi::rights::WRITE),
                ],
                maps: &[
                    (CON_SHARED_VA, g.disp_term_page, abi::address_space::MAP_RW),
                    (TERM_OUT_VA, term_out, abi::address_space::MAP_RO),
                    (TERM_IN_VA, term_in, abi::address_space::MAP_RW),
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new()
            },
        );
        let line_editor = must(r);
        must_ok(thread_control_block_start(
            line_editor,
            LINE_EDITOR_MODE_DISPLAY,
            0,
            0,
        ));
        cap_delete(line_editor);

        // `g.disp_term_ep`/`g.disp_term_page` are ours no further: `line_editor` holds its own
        // narrowed copies, granting was a copy rather than a move (the same reason `con_shared`
        // is freed on the console side below), and nothing else in this boot ever needs to reach
        // `display_terminal` directly. `term_ep` (== `g.kbd_ep` in this branch) is not freed here:
        // it is freed with the console branch's own copy, further down, by code that does not
        // know or care which branch produced it.
        cap_delete(g.disp_term_ep);
        cap_delete(g.disp_term_page);
    } else {
        let request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
        let reply = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
        let con_shared = must(retype_page_frame(ut)); // line_editor -> console text

        // 1. Console server: reads text from the shared page, writes it to the UART.
        let con = must(build_child(
            ut,
            ut,
            &con_elf,
            &ChildEndowment {
                caps: &[(request, abi::rights::READ), (reply, abi::rights::WRITE)],
                maps: &[
                    (CON_SHARED_VA, con_shared, abi::address_space::MAP_RO),
                    (CON_UART_VA, g.uart_dev, abi::address_space::MAP_RO), // mode ignored for a DeviceFrame
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new()
            },
        ));
        must_ok(thread_control_block_start(con, 0, 0, 0));
        cap_delete(con);

        // 2. The line discipline: serves the terminal endpoint, prints through the console. It is
        // the console's only client; everyone else prints through it. `LINE_EDITOR_MODE_CONSOLE`
        // (0) is `thread_control_block_start`'s own default, so this is unchanged from before
        // milestone 177 gave `line_editor` a second mode.
        let line_editor = must(build_child(
            ut,
            ut,
            &td_elf,
            &ChildEndowment {
                caps: &[
                    (term_ep, abi::rights::READ),
                    (request, abi::rights::WRITE),
                    (reply, abi::rights::READ),
                ],
                maps: &[
                    (CON_SHARED_VA, con_shared, abi::address_space::MAP_RW), // it fills what the console reads
                    (TERM_OUT_VA, term_out, abi::address_space::MAP_RO),
                    (TERM_IN_VA, term_in, abi::address_space::MAP_RW),
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new()
            },
        ));
        must_ok(thread_control_block_start(
            line_editor,
            LINE_EDITOR_MODE_CONSOLE,
            0,
            0,
        ));
        cap_delete(line_editor);

        // `request`/`reply`/`con_shared` are ours no further: the console and `line_editor` both
        // hold their own narrowed copies (the console's own doc, one screen down, gives the
        // sixteen-slot reason this cannot wait). The graphical branch above never retypes these at
        // all, so there is nothing of theirs to free here.
        for c in [request, reply, con_shared] {
            cap_delete(c);
        }
    }

    // **Refuse the system if a required component is not the one that was measured** (milestone
    // 104), here, because this is the earliest point at which init can be read by a person and the
    // latest at which nothing unmeasured has been built. The console and the line discipline above
    // are running; nothing else is.
    //
    // Halting is not a second policy. The policy is that init runs nothing it cannot vouch for, and
    // for a component the whole system is made of, not running it and not having a system are the
    // same outcome. What it costs is decided by what the program was for, which is a question init
    // already had to answer for an archive entry that is simply missing.
    let unvouched: [&str; 3] = [
        if in_elf.unvouched { "input" } else { "" },
        if sh_elf.unvouched { "swish" } else { "" },
        if reaper_elf.unvouched {
            "job_undertaker"
        } else {
            ""
        },
    ];
    if unvouched.iter().any(|n| !n.is_empty()) {
        // The shell's output page, in our own space, so the refusal can be read. The giveaway below
        // maps it for the same reason and this path never reaches the giveaway; a refusal nobody
        // can see is most of what this milestone was written to fix.
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method before
        // acting (user_rt's contract).
        if unsafe { invoke(term_out, abi::page_frame::MAP, INIT_OUT_VA, 1, ut) } == 0 {
            let mut buf = [0u8; SENTENCE];
            announce(
                term_ep,
                sentence(
                    &mut buf,
                    b"init: cannot vouch for",
                    &unvouched,
                    b"; halting rather than building an unmeasured system\n",
                ),
            );
        }
        fail()
    }
    // Past the refusal, so an absent component still traps the way it always has: a build that did
    // not pack the console's client is a broken build, not an attack, and it has never had a
    // message.
    let (Some(in_elf), Some(sh_elf), Some(reaper_elf)) = (in_elf.elf, sh_elf.elf, reaper_elf.elf)
    else {
        fail()
    };

    // 3. Input driver: waits on the UART receive interrupt, forwards raw bytes to the terminal.
    // **Skipped entirely in the graphical case**: `kbd` already plays this role, spawned
    // kernel-side and wired directly to `term_ep` before this process existed. `g.uart_dev`/
    // `g.uart_irq` are still granted either way (their slot numbers do not move; see
    // `BootEndowment`'s own doc), simply unused, and freed below regardless of mode.
    if !has_graphical {
        let input = must(build_child(
            ut,
            ut,
            &in_elf,
            &ChildEndowment {
                caps: &[
                    (term_ep, abi::rights::WRITE),
                    (g.uart_irq, abi::rights::READ),
                ],
                maps: &[(IN_UART_VA, g.uart_dev, abi::address_space::MAP_RO)],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new()
            },
        ));
        must_ok(thread_control_block_start(input, 0, 0, 0));
        cap_delete(input);
    }

    // **The console's capabilities go back now, before the shell is built**, and that is not
    // tidiness: this capability table has sixteen slots, and milestone 50 added two more kernel grants (the
    // file service and its page). With them held, the shell's `build_child` had no slot left to
    // retype an address space into and failed silently, which presented as a boot that brought up
    // the console and then printed nothing. Nothing below needs these: line_editor is the console's
    // only client (or `display_terminal`'s, in the graphical case) and it already holds its
    // narrowed copies.
    //
    // **The device and the interrupt go with them** (milestone 22, the interactive increment). Both
    // drivers that need them exist and hold their own narrowed copies, and nothing below builds
    // another driver, so an init that kept them would be keeping the authority to hand the UART to
    // anything it later builds. Dropping them here is the same act as dropping the construction
    // budget further down, one boot stage earlier. `unused` is whatever else this board's kernel
    // granted that the interactive system never had a use for.
    //
    // **Already freed in the graphical case**, earlier and for a stronger reason than tidiness
    // (see the comment where `has_graphical` frees them, above): this table had no slot to spare
    // for `term_out`/`term_in` otherwise. Freeing an empty slot here would be harmless, but
    // skipping it is what makes the earlier comment's claim ("free the instant they are dead
    // weight") true rather than aspirational.
    if !has_graphical {
        cap_delete(g.uart_dev);
        cap_delete(g.uart_irq);
    }
    // `for_test_roles` no longer needs freeing here: it is freed at the top of this function now,
    // two component-builds earlier, for the reason recorded there.

    // **The spawn channel is retyped here, not with the rest**, and the reason is the same sixteen
    // slots: holding two more endpoints through the three builds above is what pushed this capability table
    // over. They are the shell's and the service's, so this is also where they belong.
    let spawn_ep = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    let result_ep = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    // **The supervision endpoint every job is born holding** (milestone 22, the interactive
    // increment; DECISIONS §26's spawn-slot convention). We keep it for its `GRANT`, which is all we
    // need it for: to place a `READ` view of it in each job's reserved fault slot. We never receive
    // on it. `job_undertaker` does, and collecting is the only thing that endpoint authorizes.
    let deaths = must(retype_obj(ut, abi::objtype::RENDEZVOUS));

    // 4. The shell: prints and reads lines through the terminal, holds the spawn channel, and holds
    // its own untyped budget (slot 3) so `run --mem N` grants from memory that is genuinely the
    // shell's. WRITE lets it SPLIT the budget; GRANT lets it delegate the split to init. We carve
    // that budget from our own untyped and hand it over the same way we hand any capability.
    //
    // Slot 4 is the filesystem when this boot has one, which is the whole of what `>` and `<` need
    // (milestone 50, notes/pipes.md): the shell resolves a redirection against it and writes the
    // file itself. Narrowed to WRITE, which on an endpoint is the right to CALL, and without GRANT,
    // so the shell can hand it to nobody.
    //
    // **And a read-only clock last** (milestone 86), which is what `time <command>` measures with.
    // It is [`BootEndowment::clock_page`], the same frame this init was granted and hands to a child
    // whose manifest declares a clock; the shell is simply another holder of a narrowed view. `READ`
    // and no `GRANT`, deliberately: the shell can read the wall clock and can hand one to nothing it
    // spawns, so which processes can read the time is still decided by the manifests this crate
    // reads (DECISIONS §43) rather than by anything typed at a prompt.
    //
    // Last in the list, so a boot with no filesystem takes exactly the path it took before this
    // existed. Its slot therefore moves (4 without a disk, 5 with one), which is why the shell is
    // *told* the number in `x2`/`a2` instead of assuming one; see swish.rs's `CLOCK_SLOT`.
    let sh_budget = must(memory_region_split(ut, SH_BUDGET_PAGES));
    let with_fs = fs_rights != 0;

    // **The second grant, when this boot was configured with one** (milestone 154's "wiring a
    // second grant into the real boot"). Built here, before the shell, for the same reason
    // `build_caretaker` is always called before the process that will hold its endpoint: the
    // caretaker must exist and answer ready before anything could `CALL` it.
    //
    // Meaningless without a first directory ([`SecondDirGrant`]'s own doc), so this is `None`
    // whenever the boot has no filesystem at all, and it degrades to "no second grant" rather
    // than a boot failure whenever the caretaker cannot be built, the same posture this file
    // already takes toward a missing `sink_elf` or `care_elf`: a component the archive did not
    // pack or the table would not vouch for costs a feature, not a boot.
    //
    // # BUGS
    //
    // **Unverified against a real boot.** `second_dir` is `None` at every shipped entry point
    // (DECISIONS §126: what the subtree should be is calef's call), so this branch has never run
    // under `script/shell-check`, which is the only thing in the tree that runs a real init.
    // `build_caretaker` retypes two more objects into *this process's* capability table right
    // where the comment two screens up already documents this table as tight ("the shell's
    // `build_child` had no slot left ... and failed silently"). The failure mode if this pushes
    // init over sixteen slots is exactly that one: a boot that reaches userspace and prints
    // nothing. Whoever first passes `Some` here should watch for it and run `script/shell-check`
    // before trusting this path.
    let second_dir_ep: Option<u64> = second_dir.filter(|_| with_fs).and_then(|sd| {
        assert!(
            filesystem_proto::grant::fits(sd.name.as_bytes()),
            "a granted directory's name rides in two argument words; this one does not fit",
        );
        let region = memory_region_split(ut, SECOND_DIR_CARETAKER_PAGES).ok()?;
        let care = care_elf.as_ref()?;
        let (lo, hi) = filesystem_proto::grant::pack_name(sd.name.as_bytes());
        let spec = filesystem_proto::grant::spec(sd.name.len(), sd.rights);
        build_caretaker(
            ut,
            region,
            care,
            Fs {
                ep: g.fs_ep,
                page: g.fs_page,
            },
            (lo, hi, spec),
        )
    });

    // Slot 4 is the filesystem when this boot has one, which is the whole of what `>` and `<` need
    // (milestone 50, notes/pipes.md): the shell resolves a redirection against it and writes the
    // file itself. Narrowed to WRITE, which on an endpoint is the right to CALL, and without GRANT,
    // so the shell can hand it to nobody.
    //
    // **Slot 5, when this boot also has a second grant**, is [`second_dir_ep`]: the narrowed
    // caretaker endpoint built above. It shares [`SH_FS_VA`] rather than needing a map of its own,
    // for `narrow_dir`'s own reason one level narrower: caretaker and client both map the *same*
    // physical frame the FS server shares with everything downstream of it, and the shell is one
    // thread of control with at most one `CALL` in flight, so it is never mid-request on both
    // endpoints at once.
    //
    // **And a read-only clock last, always** (milestone 86), which is what `time <command>`
    // measures with. It is [`BootEndowment::clock_page`], the same frame this init was granted and
    // hands to a child whose manifest declares a clock; the shell is simply another holder of a
    // narrowed view. `READ` and no `GRANT`, deliberately: the shell can read the wall clock and
    // can hand one to nothing it spawns, so which processes can read the time is still decided by
    // the manifests this crate reads (DECISIONS §43) rather than by anything typed at a prompt.
    //
    // Clock last in the list whatever else is granted, which is why the shell is *told* its slot
    // in `x2`/`a2` instead of assuming one; see swish.rs's `CLOCK_SLOT`. That was already true
    // before this milestone (4 without a disk, 5 with one); it now also moves to 6 when a second
    // grant lands, and the mechanism that keeps the shell honest about the number is unchanged.
    let mut sh_caps = [(0u64, 0u64); 7];
    let mut sh_maps = [(0u64, 0u64, 0u64); 4];
    let mut n_caps = 0usize;
    let mut n_maps = 0usize;
    sh_caps[n_caps] = (term_ep, abi::rights::WRITE);
    n_caps += 1;
    sh_caps[n_caps] = (spawn_ep, abi::rights::WRITE);
    n_caps += 1;
    sh_caps[n_caps] = (result_ep, abi::rights::READ);
    n_caps += 1;
    sh_caps[n_caps] = (sh_budget, abi::rights::WRITE | abi::rights::GRANT);
    n_caps += 1;
    sh_maps[n_maps] = (SH_OUT_VA, term_out, abi::address_space::MAP_RW); // text and prompts
    n_maps += 1;
    sh_maps[n_maps] = (LINE_VA, term_in, abi::address_space::MAP_RO); // completed lines
    n_maps += 1;
    if with_fs {
        sh_caps[n_caps] = (g.fs_ep, abi::rights::WRITE);
        n_caps += 1;
        sh_maps[n_maps] = (SH_FS_VA, g.fs_page, abi::address_space::MAP_RW);
        n_maps += 1;
    }
    if let Some(ep) = second_dir_ep {
        sh_caps[n_caps] = (ep, abi::rights::WRITE);
        n_caps += 1;
    }
    sh_caps[n_caps] = (g.clock_page, abi::rights::READ);
    n_caps += 1;
    sh_maps[n_maps] = (SH_CLOCK_VA, g.clock_page, abi::address_space::MAP_RO);
    n_maps += 1;
    // Which slot the clock landed in, for the shell's `x2`: the count of what went before it, the
    // same arithmetic `build_child` does when it fills the capability table from zero.
    let sh_clock_slot: u64 = n_caps as u64 - 1;
    // **Built but not started**, because the drop below has to happen while the shell's output page
    // is still ours alone to write: the negative control is printed through it, and a running shell
    // would be printing its banner into the same page.
    let shell = must(build_child(
        ut,
        ut,
        &sh_elf,
        &ChildEndowment {
            caps: &sh_caps[..n_caps],
            maps: &sh_maps[..n_maps],
            stack_pages: CHILD_STACK_PAGES,
            ..ChildEndowment::new()
        },
    ));
    cap_delete(sh_budget); // our copy; the shell holds its own now
    // The caretaker's endpoint was only ever the means of wiring: the shell holds its own copy and
    // the caretaker holds the other end, the same disposal `spawn_service`'s dynamic directory
    // grants already give their own narrowed endpoint below.
    if let Some(ep) = second_dir_ep {
        cap_delete(ep);
    }

    // Free every boot cap the spawn service does not need, so init's 16-slot capability table has room to
    // build a supervised child (which holds a job untyped and a job frame while the loader retypes
    // an address space, frames, and a TCB). The drivers and the shell hold the narrowed copies that matter.
    //
    // **Only the input frame, and the two that stay have a reason each.** `term_ep` is still ours to
    // delegate: the sink adapter below is handed `WRITE` on it, and the drop announcement is a
    // `CALL` on it. `term_out` is where that announcement stages its bytes. Both go back the moment
    // their last use is done, and after that this process holds no way to reach the terminal at all.
    cap_delete(term_in);
    // **The filesystem stays** (milestone 31 phase 3, 2026-08-17). It used to go here, with a
    // comment saying "the day `rm` is reachable from the prompt, init keeps the endpoint instead,
    // because building a `fs_subtree_caretaker` is its job and not the shell's". This is that day.
    //
    // Two slots held for the life of the boot, and it is worth being precise about what they buy and
    // what they cost. They buy the only delivery mechanism a directory grant has: the caretaker must
    // hold the file service to attenuate it, the shell's copy carries no `GRANT`, and a program
    // spawned without the capability its command line named would be the worst failure this model
    // has. They cost two of init's sixteen capability table slots, permanently, which takes the spawn service's
    // resting endowment from seven capabilities to nine and its peak from thirteen to fifteen. That
    // peak is the number to watch: it is a directory-granted spawn, and it is one slot from the wall.
    // See `spawn_dir_grant`, which counts it.
    let fs = with_fs.then_some(Fs {
        ep: g.fs_ep,
        page: g.fs_page,
    });

    // 5. **The terminal's sink adapter** (milestone 50's last remainder, notes/sink-protocol.md,
    // DECISIONS §67). It holds the terminal `WRITE` and serves the sink contract on an endpoint of
    // its own, so a child can be handed "the terminal" as a place to put bytes **without** being
    // handed the terminal endpoint, which also carries `OP_READLINE` and would be the keyboard.
    //
    // **After the shell and before the giveaway, and both halves of that are load-bearing.**
    //
    // After the shell, because of this capability table's sixteen slots: building the adapter earlier put init
    // one slot over while the loader was retyping the shell's address space, and the symptom was
    // the one this system has already seen, a boot that reaches userspace and then prints nothing at
    // all. That constraint is about the shell's build, not about being the last thing built, and
    // milestone 22 is what made the difference visible: the adapter is now the fifth of six boot
    // components rather than the last of five, and the capability table has room either way.
    //
    // Before the giveaway, because this is a **system** component and the root untyped is what the
    // system is built from. Everything below hands that budget away and proves it is gone, so an
    // adapter built after it would have to come out of [`INIT_OWN_PAGES`], the scratch budget sized
    // for page tables and nothing else. Spending a whole program out of that pool would be invisible
    // here and would surface as some later child failing to map a scratch page.
    let term_sink = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    if let Some(elf) = sink_elf.as_ref() {
        let adapter = must(build_child(
            ut,
            ut,
            elf,
            &ChildEndowment {
                caps: &[
                    (term_sink, abi::rights::READ),
                    (term_ep, abi::rights::WRITE),
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new()
            },
        ));
        // Started here even though the shell deliberately is not: the adapter owns no page and
        // prints only what a client sends it, and it has no clients until the spawn service below
        // hands one its endpoint. It cannot write into the page the announcement below stages in.
        must_ok(thread_control_block_start(adapter, 0, 0, 0));
        cap_delete(adapter);
    }

    // **Milestone 49's login stack**: `credentialer`, `identity_provisioner`, `login`,
    // `audit_sink`. Built here, after the shell (this table's own tightest peak, "one slot from
    // the wall") has already returned its own transient capabilities, and before the giveaway
    // below spends `ut` down to nothing: everything this block needs (`ut` itself, `term_ep`, the
    // file service pair, a live client view of entropy) is still ours to spend here and nowhere
    // later, the identical reasoning the entropy block's own comment gives for building *that*
    // where it does.
    //
    // Optional, in the sink adapter's own sense: a boot with no filesystem, no working entropy
    // service, or any of these four programs missing or unvouched for simply has no login path,
    // and this whole block is skipped rather than faulted on. `login_ready`/`login_password` are
    // carried forward as data (the same shape `entropy_ready` already uses) because this process
    // has no terminal mapped yet at this point in the boot; the actual announcement happens later,
    // at the existing print site, right before `term_ep` is dropped.
    let have_login_stack = with_fs
        && entropy_client.is_some()
        && cred_elf.is_some()
        && idp_elf.is_some()
        && login_elf.is_some()
        && audit_elf.is_some();
    let mut login_ready = false;
    let mut login_password = [0u8; PASSWORD_HEX_LEN];
    if !have_login_stack {
        // Nothing below will use it; give it back rather than leak a permanent slot for the rest
        // of the boot.
        if let Some(req) = entropy_client {
            cap_delete(req);
        }
    } else {
        let request = entropy_client.expect("have_login_stack checked entropy_client.is_some()");

        // **The generated password, drawn before anything else in this block**: `credentialer`'s
        // own posture ("it never invents a salt") one level up. No unpredictable bits, no
        // password, and this whole block is abandoned rather than provisioning one with something
        // weaker (DECISIONS §42).
        let mut password_bytes = [0u8; PASSWORD_BYTES];
        if !fill_entropy(request, &mut password_bytes) {
            cap_delete(request);
        } else {
            hex_password(&password_bytes, &mut login_password);

            // 1. **`credentialer`** (milestone 56/65, the secrets service): built the same way
            // entropy was, permanent for the life of the boot, holding the client view of entropy
            // this block just drew a password from.
            let prov = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let verify = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let cred_ready = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let prov_page = must(retype_page_frame(ut));
            let verify_page = must(retype_page_frame(ut));
            let cred_budget = must(memory_region_split(ut, CRED_BUDGET_PAGES));
            let cred_program = cred_elf.as_ref().expect("have_login_stack checked this");
            let cred_tcb = must(build_child(
                ut,
                ut,
                cred_program,
                &ChildEndowment {
                    caps: &[
                        (prov, abi::rights::READ),
                        (verify, abi::rights::READ),
                        (request, abi::rights::WRITE),
                        (cred_budget, abi::rights::WRITE),
                        (cred_ready, abi::rights::WRITE),
                    ],
                    maps: &[
                        (CRED_SVC_PROV_VA, prov_page, abi::address_space::MAP_RW),
                        (CRED_SVC_VERIFY_VA, verify_page, abi::address_space::MAP_RW),
                    ],
                    stack_pages: CRED_STACK_PAGES,
                    ..ChildEndowment::new()
                },
            ));
            must_ok(thread_control_block_start(cred_tcb, 0, 0, 0));
            cap_delete(cred_tcb);
            cap_delete(request); // credentialer holds its own copy now
            cap_delete(cred_budget); // ditto

            // **`cred_ready` is not read yet, and that ordering is load-bearing rather than an
            // oversight.** `credentialer.rs`'s own `_start` sends its one readiness message
            // (`RPT_READY`) only *after* `provision()` returns, which is only after this process's
            // own `SEAL` arrives (`credentialer.rs`'s own "Two phases" doc: phase one is `RECV` on
            // the provision endpoint, forever, until sealed). A `recv(cred_ready)` here, before
            // provisioning has even been attempted, is not "wait for the service to come up" the
            // way the entropy block's own `recv(ready)` is -- it is "wait for a message that
            // cannot exist until this same function seals the store a few lines further down", a
            // deadlock this process would never wake from. Found by running `script/shell-check`
            // and watching it hang rather than fault: no `[PANIC]`, nothing kept building, because
            // this process was genuinely blocked rather than trapped.
            //
            // 2. **Provision the generated credential and the demo identity's home subtree, as
            // one act, through `identity_provisioner`** (milestone 155): stage the plaintext
            // into a page this process maps briefly, hand it to the tool along with `prov`
            // (before its seal) and this boot's own `g.fs_ep`.
            let req_frame = must(retype_page_frame(ut));
            // SAFETY: `invoke` traps to the kernel, which validates the capability and the
            // method before acting.
            if unsafe { invoke(req_frame, abi::page_frame::MAP, PROVISION_SCRATCH_VA, 1, ut) } != 0
            {
                cap_delete(req_frame);
            } else {
                // SAFETY: just mapped read/write, one page, ours alone until it is delegated
                // on below.
                let req_slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        PROVISION_SCRATCH_VA as *mut u8,
                        credential_proto::PAGE,
                    )
                };
                req_slice.fill(0);
                let placed = credential_proto::place(
                    req_slice,
                    DEMO_IDENTITY,
                    &login_password,
                    credential_proto::provision::PUT,
                );
                placed.expect(
                    "DEMO_IDENTITY and login_password are both well within credential_proto's \
                     bounds by construction; a None here is a logic bug in this file, not a \
                     runtime condition",
                );

                let idp_report = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let idp_program = idp_elf.as_ref().expect("have_login_stack checked this");
                let idp_tcb = must(build_child(
                    ut,
                    ut,
                    idp_program,
                    &ChildEndowment {
                        caps: &[
                            (prov, abi::rights::WRITE),
                            (g.fs_ep, abi::rights::WRITE),
                            (idp_report, abi::rights::WRITE),
                        ],
                        maps: &[
                            (IDP_REQ_VA, req_frame, abi::address_space::MAP_RW),
                            (IDP_PROV_VA, prov_page, abi::address_space::MAP_RW),
                            (IDP_FS_VA, g.fs_page, abi::address_space::MAP_RW),
                        ],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new()
                    },
                ));
                must_ok(thread_control_block_start(
                    idp_tcb,
                    DEMO_IDENTITY.len() as u64,
                    login_password.len() as u64,
                    0,
                ));
                cap_delete(idp_tcb);
                cap_delete(req_frame);

                let (idp_code, _, _) = recv(idp_report);
                cap_delete(idp_report);
                login_ready = idp_code == IDP_RPT_OK;

                // Wipe the staged plaintext regardless of outcome: this process's own copy of
                // the generated password should not sit in memory longer than provisioning
                // needs it.
                // SAFETY: still the same page this process mapped above, and the only writer
                // between it being staged and this wipe.
                let req_slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        PROVISION_SCRATCH_VA as *mut u8,
                        credential_proto::PAGE,
                    )
                };
                credential_proto::wipe(req_slice);
            }
            // Ours no further either way: `identity_provisioner` holds its own copy when it was
            // built at all, and this process never maps this page again.
            cap_delete(prov_page);

            // **Seal the store regardless of whether provisioning succeeded.** `credentialer`
            // must leave phase one before its `VERIFY` endpoint will ever answer anything
            // (`credentialer.rs`'s own "two phases" doc), and a login against an empty or
            // partially-provisioned store should deny cleanly rather than the whole boot
            // hanging with a service that can never be asked a real question.
            call(
                prov,
                credential_proto::req(credential_proto::provision::SEAL, 0, 0),
                0,
            );
            cap_delete(prov);

            // **Now** `cred_ready` answers: the seal just sent is exactly what makes
            // `provision()` return and `credentialer.rs`'s own `_start` reach its one
            // `send(READY, RPT_READY, ...)`. A dead-on-arrival service (its own
            // `E_ENTROPY`/`E_SCRATCH` startup failure, which happens before phase one even
            // begins) still answers here too, with its own `0xDEAD_...` word instead, so this
            // `recv` is never left permanently unanswered by that path either.
            let (cred_rv, _, _) = recv(cred_ready);
            cap_delete(cred_ready);
            if cred_rv != CRED_RPT_READY {
                cap_delete(verify);
                cap_delete(verify_page);
                login_ready = false;
            } else {
                // 3. **`login`** (milestone 49): built the same way, holding narrowed views of
                // what this process already has (the file service pair, a fresh construction
                // budget) plus a client view of `credentialer`'s own verify endpoint just built,
                // and (this update) a `WRITE | GRANT` view of the terminal so it can hand the
                // single-session terminal to its first successful caller
                // (`user/src/login.rs`'s "The terminal: single-session, deny cleanly").
                //
                // `audit_sink` is built *first* and started before `login` ever runs, so the
                // receiver for `login`'s blocking `AUDIT` send exists before there is any way to
                // reach it (`user/src/audit_sink.rs`'s own doc on why this ordering matters).
                let audit = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let audit_program = audit_elf.as_ref().expect("have_login_stack checked this");
                let audit_tcb = must(build_child(
                    ut,
                    ut,
                    audit_program,
                    &ChildEndowment {
                        caps: &[(audit, abi::rights::READ)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new()
                    },
                ));
                must_ok(thread_control_block_start(audit_tcb, 0, 0, 0));
                cap_delete(audit_tcb);

                let login_request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let login_result = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let login_ut = must(memory_region_split(ut, LOGIN_CONSTRUCTION_PAGES));
                let login_program = login_elf.as_ref().expect("have_login_stack checked this");
                let login_tcb = must(build_child(
                    ut,
                    ut,
                    login_program,
                    &ChildEndowment {
                        caps: &[
                            (login_request, abi::rights::READ),
                            (login_result, abi::rights::WRITE | abi::rights::GRANT),
                            (verify, abi::rights::WRITE),
                            (g.fs_ep, abi::rights::WRITE | abi::rights::GRANT),
                            // **`WRITE` only, not `READ | WRITE`.** This process itself holds
                            // only `WRITE | GRANT` on the real file service's shared page (the
                            // kernel's own grant to init), and `WRITE` is all a writable mapping
                            // ever checks (`kernel::syscall::page_frame_map`'s own comment: "a
                            // read/write mapping needs WRITE on the frame"). See
                            // `user/src/login.rs`'s own comment on its matching delegation to its
                            // clients for the fuller account. Found here first: `must(build_child(...))`
                            // for `login` refused this exact `SEND_CAP` when it asked for `READ`
                            // too, in total silence, the same way every other capacity mismatch in
                            // this function has.
                            (g.fs_page, abi::rights::WRITE | abi::rights::GRANT),
                            (login_ut, abi::rights::WRITE | abi::rights::GRANT),
                            (audit, abi::rights::WRITE),
                            (term_ep, abi::rights::WRITE | abi::rights::GRANT),
                        ],
                        maps: &[(LOGIN_CRED_VA, verify_page, abi::address_space::MAP_RW)],
                        stack_pages: LOGIN_STACK_PAGES,
                        ..ChildEndowment::new()
                    },
                ));
                must_ok(thread_control_block_start(login_tcb, 0, 0, 0));
                cap_delete(login_tcb);
                // login holds its own copies now; ours were only ever the means of wiring.
                cap_delete(login_request);
                cap_delete(login_result);
                cap_delete(verify);
                cap_delete(verify_page);
                cap_delete(login_ut);
                cap_delete(audit);
            }
        }
    }

    // **Give the construction budget away** (milestone 22, the interactive increment). Two bounded
    // carves and then the root itself: after this line init can spend at most `INIT_OWN_PAGES` on
    // itself and `JOBS_BUDGET_PAGES` on the prompt's jobs, and it can no longer reach the rest of the
    // memory the kernel handed it or delegate the root to anything it builds.
    //
    // Two budgets rather than one, and the split is load-bearing rather than tidy: the job pool's
    // watermark must move for **jobs only**, or the LIFO return-of-pages (§16) never fires. A scratch
    // page table carved out of the same region between a job's split and its reap would sit above
    // that job's run, so the reclaim would find it is not the top and give back nothing.
    let own_ut = must(memory_region_split(ut, INIT_OWN_PAGES));
    let jobs_ut = must(memory_region_split(ut, JOBS_BUDGET_PAGES));
    // The shell's output page, in our own space, so we can say what just happened. This mapping is
    // permanent (there is no unmap, and `PageFrame::REVOKE` would take the page from the shell too); see
    // this module's BUGS.
    // SAFETY: `invoke` traps to the kernel, which validates the capability and the method before
    // acting (user_rt's contract).
    if unsafe { invoke(term_out, abi::page_frame::MAP, INIT_OUT_VA, 1, ut) } != 0 {
        fail()
    }
    cap_delete(ut);

    // And prove it from the inside, on the two primitives that build things, before anything else
    // runs. `NoSuchSlot` (-1) rather than `NotPermitted` (-3) is the whole claim: the capability is
    // *gone*, not narrowed, so there is nothing there to name. This is `root_supervisor`'s proof at
    // the interactive prompt, and `script/shell-check` reads the sentence.
    // SAFETY: as above: the kernel validates the capability and the method.
    let frame = unsafe { invoke(ut, abi::memory_region::RETYPE, 0, 0, 0) };
    // SAFETY: as above: the kernel validates the capability and the method.
    let object = unsafe {
        invoke(
            ut,
            abi::memory_region::RETYPE_OBJ,
            abi::objtype::THREAD_CONTROL_BLOCK,
            0,
            0,
        )
    };
    announce(
        term_ep,
        if frame == -1 && object == -1 {
            b"init: construction budget dropped; retype answers NoSuchSlot\n"
        } else {
            b"init: construction budget NOT dropped; it can still build\n"
        },
    );
    // **And what the measurement decided** (milestone 104), on the same terms as the line above: a
    // claim about what init refuses is worth what the check behind it is worth, and only init can
    // run that check. The affirmative line is the load-bearing one. A measured boot's natural bug is
    // for the check to evaporate when the build step does not run, and a boot that says nothing
    // looks exactly like a boot that measured everything, so `script/shell-check` reads this
    // sentence and a boot that stopped measuring fails the gate instead of passing quietly.
    let mut buf = [0u8; SENTENCE];
    announce(
        term_ep,
        if refused_n == 0 {
            b"init: every program measured against the archive table\n"
        } else {
            sentence(
                &mut buf,
                b"init: measurement refused",
                &refused[..refused_n],
                b"; they cannot be spawned\n",
            )
        },
    );
    // **The entropy service's own outcome** (DECISIONS §120's 2026-08-26 amendment), said here
    // rather than where it was decided: `entropy_ready` was set long before this process had a
    // terminal at all (the whole reason the build happens first, at the top of this function; see
    // that block's own comment), and [`INIT_OUT_VA`] is not mapped into init's own space until the
    // line above this one first used it, so nothing earlier in this function could `announce`
    // regardless. Silent when there was nothing to build (no device, or no `entropy` program in
    // the archive), the same posture the sink adapter and the subtree caretaker already take for a
    // missing component.
    if entropy_ready {
        announce(
            term_ep,
            b"init: entropy service up; drew real bytes from a virtio-rng device\n",
        );
    }
    // **The generated login credential** (milestone 49's boot-wiring update), said here for
    // [`entropy_ready`]'s own reason: `login_ready` was decided long before this process had a
    // terminal. Printed exactly once, before the prompt, the shape a cloud image's generated
    // first-boot password already takes: this process's own copy was wiped the moment
    // `identity_provisioner` used it (see that block's own comment), so this line is the only
    // place the password exists once it scrolls past. Silent when there is no login stack at all
    // (no entropy, no filesystem, or one of the four programs missing/unvouched), the same posture
    // every other optional boot component already takes.
    if login_ready {
        fn push(buf: &mut [u8; SENTENCE], n: &mut usize, src: &[u8]) {
            for &b in src {
                if *n < SENTENCE {
                    buf[*n] = b;
                    *n += 1;
                }
            }
        }
        let mut buf = [0u8; SENTENCE];
        let mut n = 0usize;
        push(
            &mut buf,
            &mut n,
            b"init: login ready -- generated credentials: identity '",
        );
        push(&mut buf, &mut n, DEMO_IDENTITY);
        push(&mut buf, &mut n, b"' password '");
        push(&mut buf, &mut n, &login_password);
        push(&mut buf, &mut n, b"' (shown once; use it now)\n");
        announce(term_ep, &buf[..n]);
    }
    cap_delete(term_ep);
    cap_delete(term_out);

    // 6. The undertaker, out of what is left of our own budget. One capability, `READ` on the
    // supervision endpoint, and nothing else: it can free a job's memory and can never spend it.
    let reaper = must(build_child(
        own_ut,
        own_ut,
        &reaper_elf,
        &ChildEndowment {
            caps: &[(deaths, abi::rights::READ)],
            stack_pages: CHILD_STACK_PAGES,
            ..ChildEndowment::new()
        },
    ));
    must_ok(thread_control_block_start(reaper, 0, 0, 0));
    cap_delete(reaper);

    // Role 0 (the prompt), and `arg1` is the rights its directory capability carries. A shell told 0
    // holds no directory and says so at every verb that would need one. `arg2` is the clock slot
    // (milestone 86), which moves with whether this boot had a disk, so it is told rather than
    // assumed.
    must_ok(thread_control_block_start(
        shell,
        0,
        fs_rights,
        sh_clock_slot,
    ));
    cap_delete(shell);

    spawn_service(
        Channels {
            spawn_ep,
            result_ep,
            deaths,
            own_ut,
            jobs_ut,
            clock_page: g.clock_page,
            config_page: g.config_page,
            // The terminal's sink, if this initrd carried an adapter to serve it. This is what a
            // declared second stream gets by default (DECISIONS §67): the shell names a file with
            // `2>` and otherwise the bytes go straight to the screen, through a process that can do
            // nothing else with them.
            term_sink: sink_elf.is_some().then_some(term_sink),
            fs,
        },
        &progs,
        care_elf,
    )
}

/// The archive entry a spawnable program is loaded from.
///
/// It answered `None` for `rm` until 2026-08-17, because `rm` is endowed a **directory** capability
/// and init had deleted the file service during the boot, so there was nothing to attenuate. Keeping
/// the slot empty was the honest answer to that: spawning `rm` with nothing to remove from would be
/// the worst failure this model has, a program told to destroy something, holding nothing, saying
/// nothing. Milestone 31 phase 3 removed the cause rather than the symptom, so the exception is gone
/// and every spawnable program is loaded the same way.
fn archive_name(p: Prog) -> Option<&'static str> {
    Some(p.name())
}

/// Turn a `recv_cap` slot into `Some(slot)`, or `None` if the message carried no capability.
fn opt_cap(slot: u64) -> Option<u64> {
    if slot == abi::rendezvous::NO_CAP {
        None
    } else {
        Some(slot)
    }
}

/// Everything the spawn service holds for its whole life, so the loop's signature says what init's
/// remaining authority *is*: two channels, one supervision endpoint it only ever delegates from, and
/// two bounded budgets. The root construction budget is deliberately not in here; it is gone.
struct Channels {
    /// READ: the shell's `run` requests arrive here.
    spawn_ep: u64,
    /// WRITE: a child's answer channel, and our own spawn-failed sentinel.
    result_ep: u64,
    /// GRANT: placed `READ` in every job's reserved fault slot, so `job_undertaker` collects it.
    deaths: u64,
    /// Our own scratch budget: page tables for the loader's scratch window, and nothing else.
    own_ut: u64,
    /// The job pool. One region per job, split off here and returned here when the job is reaped.
    jobs_ut: u64,
    /// READ on the wall clock, endowed to a child whose manifest declares one (DECISIONS §43).
    clock_page: u64,
    /// READ on the inert-configuration page, endowed to a child whose manifest declares
    /// [`grant_plan::Manifest::config`] (DECISIONS §111). [`clock_page`](Channels::clock_page)'s
    /// twin in every respect: unconditional, read-only, and handed on only where the manifest asks.
    config_page: u64,
    /// WRITE-delegable: the endpoint the terminal's sink adapter serves, which is where a declared
    /// second stream goes when the command line named no file for it (DECISIONS §67). `None` when
    /// this initrd carried no adapter, and then a declaring child simply gets no second stream.
    /// This is authority to *print*, and nothing else: the adapter holds the terminal, we do not.
    term_sink: Option<u64>,
    /// **The file service and the page its clients share with it** (milestone 31 phase 3), or `None`
    /// on a boot with no disk. `WRITE | GRANT` on the endpoint: this is the directory capability a
    /// `fs_subtree_caretaker` attenuates, and the only reason init keeps it past the boot.
    ///
    /// It used to be dropped once the shell held its narrowed copy, with a comment saying why it
    /// would have to come back. This is that day. The shell's copy carries no `GRANT`, so the shell
    /// holds nothing it could hand a caretaker; init is the only process here that can build one,
    /// which is what made `rm` a refusal at the prompt for six weeks.
    fs: Option<Fs>,
}

/// The file service, as init holds it for the life of the boot.
///
/// A struct rather than two `Option<u64>` fields because the two are one fact: a boot either has a
/// filesystem or it does not, and `Some(ep)` with `None` page is not a state that can exist.
#[derive(Clone, Copy)]
struct Fs {
    /// The service endpoint, `WRITE | GRANT`. The directory capability, rooted at the image root.
    ep: u64,
    /// The frame every client of that service maps to trade bytes with it.
    page: u64,
}

/// The spawn service loop: serve the shell's `run` requests forever. Init is the ELF loader the
/// shell directs; it inserts only what the shell endows, so a spawned program can reach nothing the
/// command line did not name.
///
/// Two shapes (`grant_plan::spawnproto`). A **normal** job: the shell sends the request and, if
/// `--mem` rode along, one delegated untyped; we split a region off the job pool, build the child in
/// it, endow it the result endpoint (and the budget) plus its supervision endpoint, and start it. A
/// **supervised** (interruptible) job: the shell leads the delegation with a job untyped and a shared
/// job frame; we build the whole child *from that untyped* (so the shell's region owns it and can
/// `DESTROY` it to tear it down, DECISIONS §24), map the job frame in, endow nothing else, start it,
/// and send `SPAWN_OK` once as the shell's go-ahead. The `progs` array is indexed by [`Prog::id`], so
/// it is [`grant_plan::PROG_COUNT`] long: a variant added to `grant_plan` without a slot here would
/// be an out-of-bounds read in init.
///
/// **Only the normal shape is supervised**, and that is not an oversight. An interruptible job's
/// region belongs to the shell, which tears it down itself on the second `^C` (§24's
/// forcible tier) and after a clean finish; endowing it a supervision endpoint here would put a
/// second party in the teardown path for memory that is not ours, racing the shell's `DESTROY`.
fn spawn_service(
    c: Channels,
    progs: &[Option<elf::Elf>; grant_plan::PROG_COUNT],
    care_elf: Option<elf::Elf>,
) -> ! {
    let Channels {
        spawn_ep,
        result_ep,
        deaths,
        own_ut,
        jobs_ut,
        clock_page,
        config_page,
        term_sink,
        fs,
    } = c;
    loop {
        let (w0, w1, w2) = recv(spawn_ep);
        let prog = Prog::from_id(spawnproto::prog_id(w0));
        let arg = spawnproto::arg(w1);
        let mem_pages = spawnproto::mem_pages(w2);
        let wiring = spawnproto::wiring(w2);
        let interruptible = wiring.interruptible;

        // **The directory grant's two data messages, before any capability** (milestone 31 phase 3).
        // They are read here rather than inside the branch that uses them because the shell has
        // already sent them: a request that announced them and an init that did not drain them would
        // leave the endpoint holding words the *next* command would read as its own.
        let grant = wiring.dir.then(|| (recv(spawn_ep), recv(spawn_ep)));

        // Receive the delegated caps in protocol order: the interrupt pair first (job untyped, job
        // frame), then the sink, then the source, then the diagnostics, then the screen-narrowed
        // tail's completion endpoint (DECISIONS §106), then any --mem untyped. No promise, no
        // receive, so both sides stay in lockstep.
        let (job_ut, job_fr) = if interruptible {
            (opt_cap(recv_cap(spawn_ep).1), opt_cap(recv_cap(spawn_ep).1))
        } else {
            (None, None)
        };
        let sink = if wiring.sink {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        let source = if wiring.source {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        let diagnostics = if wiring.diagnostics {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        // **The narrowed tail's completion endpoint** (DECISIONS §106), in the same delegation
        // order as everything else: a fresh capability the shell minted and kept a copy of, so
        // init installs it as this child's fault target and the shell can `RECV` its exit instead
        // of draining bytes it will no longer see.
        let screen = if wiring.screen {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        let budget = if mem_pages > 0 {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };

        let elf = prog.and_then(|p| progs[p.id() as usize].as_ref());
        // Read from the program's own declaration, not from the request: a clock is not something
        // the command line can designate, so there is no bit on the wire for it (`Manifest::clock`).
        let wants_clock = prog.is_some_and(|p| p.manifest().clock);
        // Same reasoning, one authority over (milestone 126): a **process domain** is not something
        // a person designates either. There is no /proc to name and no pid space to scan, so what a
        // program may see is decided here, by which supervision endpoint init puts in its capability table.
        let wants_domain = prog.is_some_and(|p| p.manifest().domain);
        // `clock`'s twin again: the inert-configuration page is init's to endow, not something a
        // command line can designate, so there is no bit on the wire for it either
        // (`Manifest::config`).
        let wants_config = prog.is_some_and(|p| p.manifest().config);

        if interruptible {
            // Build the whole child from the shell's job untyped, mapping the shared job frame; no
            // capabilities in its capability table (it reports through the frame and exits). SPAWN_OK is the
            // go-ahead the shell waits for before it starts watching the frame.
            let built = match (elf, job_ut, job_fr) {
                (Some(e), Some(job), Some(fr)) => build_child(
                    own_ut,
                    job,
                    e,
                    &ChildEndowment {
                        maps: &[(CHILD_JOB_PAGE_FRAME_VA, fr, abi::address_space::MAP_RW)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new()
                    },
                )
                .ok(),
                _ => None,
            };
            match built {
                Some(tcb) => {
                    let ok = thread_control_block_start(tcb, 0, arg, 0);
                    send(
                        result_ep,
                        if ok {
                            spawnproto::SPAWN_OK
                        } else {
                            spawnproto::SPAWN_FAILED
                        },
                        0,
                        0,
                    );
                    cap_delete(tcb);
                }
                None => {
                    send(result_ep, spawnproto::SPAWN_FAILED, 0, 0);
                }
            }
        } else {
            // **A region of its own, split first now** (milestone 31 phase 3). It used to be split
            // just before the build; a directory grant needs it earlier, because the caretaker is
            // built out of the **same** region as the program it serves. That is DECISIONS §92's
            // decision and §40's mechanism: a child's resources come from its supervisor's region,
            // so one reclaim ends both and the caretaker cannot outlive the grant it carries.
            let region = memory_region_split(
                jobs_ut,
                if wiring.dir {
                    DIR_JOB_REGION_PAGES
                } else {
                    JOB_REGION_PAGES
                },
            )
            .ok();

            // **The caretaker, built before the program it serves**, because the program's slot 0 is
            // the endpoint this returns. `None` means either that this is not a directory grant or
            // that the delivery failed, and `dir_failed` below is what keeps those apart: a grant
            // that could not be delivered must **not** spawn the program anyway.
            let narrowed = if wiring.dir {
                match (region, fs, care_elf.as_ref(), grant) {
                    (Some(r), Some(fs), Some(care), Some((care_words, _))) => {
                        build_caretaker(own_ut, r, care, fs, care_words)
                    }
                    _ => None,
                }
            } else {
                None
            };
            // A directory grant that produced no capability. The program is not built: it was told
            // to act on something and holds nothing, which is the one outcome this model must never
            // trade away.
            let dir_failed = wiring.dir && narrowed.is_none();

            // **Slot 0 is the output**, and milestone 50 is the whole of what changed here: it is
            // the shared result endpoint unless the shell delegated a sink, in which case the sink
            // goes there instead and the child never learns that anything is different. `>` and the
            // left of a `|` are this line.
            //
            // **Except behind a directory grant**, where the narrowed endpoint takes slot 0 and the
            // output moves to slot 1. That is not a second convention invented here: it is the
            // contract `user/src/rm.rs` already documents and the kernel's `start_granted_dir`
            // already wires, so one program means one thing in a guest test and at the real prompt.
            // The grant goes first because it is the authority the command line named, and the
            // output is what every program gets whether it named anything or not.
            //
            // Slot 1 is the input source when there is one, and otherwise the `--mem` untyped, which
            // is safe only because no manifest declares both today. `grant_plan` is where that stops
            // being true, and the order here is the contract; see notes/pipes.md's BUGS.
            //
            // **Except when the shell narrowed this tail to the screen** (DECISIONS §106): the same
            // default-routing shape as the clock and the diagnostic stream below, applied to slot 0.
            // The shell asked for this by delegating a `screen` completion endpoint rather than by
            // declaring anything the child can read back, so the child's own slot 0 is exactly as
            // opaque to it as a redirected one always was.
            let default_screen = term_sink.filter(|_| screen.is_some());
            let out = (
                sink.or(default_screen).unwrap_or(result_ep),
                abi::rights::WRITE,
            );
            // Six rather than five (one more than the tightest bound any manifest shipped today
            // reaches): `dir` takes two, and `clock`, `config`, `source` and `budget` are each one
            // more, so a hypothetical program declaring all of directory, clock, config, source and
            // budget at once would need six. No shipped manifest does (no program declares both
            // `clock` and `config`, or a directory grant alongside either), but the array is sized
            // for what the *fields* allow rather than for what the table happens to contain today,
            // which is `clock_map`'s own ordered-slot debt (notes/pipes.md's `BUGS`) one field over.
            let mut caps = [out; 6];
            let mut n = 1usize;
            if let Some(dir_ep) = narrowed {
                caps[0] = (dir_ep, abi::rights::WRITE);
                caps[1] = out;
                n = 2;
            }
            // **The clock, which nothing on the command line asked for** (milestone 51's wiring).
            // It comes from the manifest rather than from the request, because a person does not
            // designate a clock: `date` declares that it reads one, and init is the only process
            // here holding a page it could hand over. Before the source and the budget, so `date`'s
            // clock is slot 1, which is unambiguous only because no manifest declares a clock *and*
            // an input. That is the same ordered-slot debt notes/pipes.md's BUGS already records.
            if wants_clock {
                caps[n] = (clock_page, abi::rights::READ);
                n += 1;
            }
            // **The inert-configuration page, `clock`'s twin** (DECISIONS §111). Same reasoning,
            // same ordering rule: before the source and the budget, so `printenv`'s config page is
            // slot 1 exactly as `date`'s clock is, and for the identical reason (no manifest
            // declares `config` alongside an input either, today).
            if wants_config {
                caps[n] = (config_page, abi::rights::READ);
                n += 1;
            }
            if let Some(src) = source {
                // READ only. A pipe's reader must not be able to write back up its own input, which
                // would make a pipeline a two-way channel nobody asked for.
                caps[n] = (src, abi::rights::READ);
                n += 1;
            }
            if let Some(b) = budget {
                // Narrowed to WRITE: the child may spend it, not lend it.
                caps[n] = (b, abi::rights::WRITE);
                n += 1;
            }
            // **The declared second stream, at the slot the manifest names** (DECISIONS §67). Read
            // from the program's own declaration for the same reason the clock is: the shell knows
            // there is one (it minted the endpoint) and only the program knows where it goes. The
            // slot is high and explicit rather than next-in-line, because how many low slots this
            // child gets depends on what else the line granted, and a stream the program probes for
            // by number cannot move under it.
            let diag_slot = prog.and_then(|p| p.manifest().output.diagnostics_slot());
            // **Where the second stream goes when the line did not say.** The shell delegates an
            // endpoint only for a `2>`, because that is the case it has to back a file for. With no
            // operator on the line the destination is the **terminal's own sink**, which is init's
            // to endow exactly as the clock is: the shell holds nothing it could hand over, and a
            // person does not designate a screen.
            //
            // That is also what keeps a redirected `date`'s complaint off the redirection. The
            // shell drains the output into the file and never sees these bytes at all.
            let default_diag = term_sink.filter(|_| diag_slot.is_some());
            // Either half missing means no second stream reaches the child, and it then says what it
            // has to say in-band, which is what every program did before §67.
            //
            // **Two named slots now, and they are placed the same way for the same reason.** The
            // second is a view over the domain this child is *about to be born into*: `deaths` is
            // the endpoint every job init spawns for this shell is supervised by, so a viewer handed
            // it sees exactly this shell's jobs, including itself, and nothing else on the machine.
            // Init, the shell, the terminal and the filesystem server are all outside it, which is
            // the confinement claim and is checked by `kernel::user::survey_tests`.
            //
            // **The right is `ENUMERATE`, and it used to be `READ`** (fixed 2026-08-17). `READ` on a
            // supervision endpoint is also what `RECV` and `abi::rendezvous::REAP` take, so the old
            // grant would have let a viewer take a death message out from under `job_undertaker` or
            // collect a corpse, and only the viewer's own source code said it did not. A domain names
            // its members and does not act on them (calef, 2026-08-17); `capability::Rights::ENUMERATE`
            // is what makes that a property of the grant. notes/process-view.md carries the argument.
            let mut placed_buf = [(0u64, 0u64, 0u64); 2];
            let mut placed_n = 0usize;
            if let (Some(ep), Some(slot)) = (diagnostics.or(default_diag), diag_slot) {
                placed_buf[placed_n] = (slot, ep, abi::rights::WRITE);
                placed_n += 1;
            }
            if wants_domain {
                // `ENUMERATE` alone, deliberately. Granting `READ` here would hand a viewer
                // `RECV` and `REAP` on the supervision endpoint as well, which is authority to
                // collect a child rather than to name one. See `Rights::ENUMERATE`.
                placed_buf[placed_n] = (grant_plan::DOMAIN_SLOT, deaths, abi::rights::ENUMERATE);
                placed_n += 1;
            }
            let placed: &[(u64, u64, u64)] = &placed_buf[..placed_n];
            let clock_map = [(CHILD_CLOCK_VA, clock_page, abi::address_space::MAP_RO)];
            let config_map = [(CHILD_CONFIG_VA, config_page, abi::address_space::MAP_RO)];
            // **The FS contract's shared page, for a program behind a directory grant.** The same
            // frame the caretaker maps and the same frame the FS server maps: one page for all three
            // parties, sound because every request on both hops is a blocking `CALL`, so the client
            // is parked inside its own call for the whole time the caretaker is using it.
            let dir_map = [(
                FS_CLIENT_PAGE_VA,
                fs.map_or(0, |f| f.page),
                abi::address_space::MAP_RW,
            )];
            // The region's own comment lives at the split above, which milestone 31 phase 3 moved
            // earlier so a caretaker could be built out of it. Everything the child is made of comes
            // out of that carve, and a single reclaim frees all of it; the clock frame and the FS
            // page are ours and are only *mapped* into the child, so they are untouched when the
            // region goes.
            // **One extra mapping at most, today.** A program that declared a directory grant AND a
            // clock AND the config page would need three, and this chain only ever offers one; no
            // shipped manifest reaches that combination (the directory program, `rm`, declares
            // neither clock nor config, and no program declares both clock and config), so the gap
            // is unreached rather than closed. The same ordered-slot debt `wants_clock`'s own
            // comment above already names for `caps`, one structure over; see notes/pipes.md's
            // `BUGS`.
            let maps: &[(u64, u64, u64)] = if narrowed.is_some() {
                &dir_map
            } else if wants_clock {
                &clock_map
            } else if wants_config {
                &config_map
            } else {
                &[]
            };
            let built = match (elf.filter(|_| !dir_failed), region) {
                (Some(e), Some(r)) => {
                    // Born supervised: `deaths` goes in the reserved fault slot, where `START` reads
                    // it and clears it, so the job cannot forge messages on its own death channel.
                    // The declared second stream rides the same named-slot mechanism at the low slot
                    // its manifest picked; the two cannot collide, because the fault slot is last.
                    build_child(
                        own_ut,
                        r,
                        e,
                        &ChildEndowment {
                            caps: &caps[..n],
                            placed,
                            maps,
                            // **A screen-narrowed child is supervised by the shell's own fresh
                            // endpoint instead of `deaths`** (DECISIONS §106), so the shell can
                            // `RECV` its exit directly rather than racing init's reaper for the
                            // same message. Its memory still comes from `region` (unchanged, still
                            // this job pool), and REAP still returns it to this pool regardless of
                            // who holds the supervision endpoint (DECISIONS §26: the reclaimed
                            // region returns to its *builder*, not its supervisor). The trade is
                            // that this child is outside `deaths`'s domain for its short life, so
                            // it will not appear in a concurrent `ps`/`pgrep`; see this function's
                            // BUGS.
                            fault: Some(screen.unwrap_or(deaths)),
                            stack_pages: CHILD_STACK_PAGES,
                            ..ChildEndowment::new()
                        },
                    )
                    .ok()
                }
                _ => None,
            };
            let ok = match built {
                Some(tcb) => {
                    // **A program behind a directory grant is started with the grant's own three
                    // words** rather than with an integer, which is `rm`'s shape: a spec carrying
                    // the options and two words of name (`filesystem_proto::grant`). init forwards what the
                    // shell packed and reads none of it; see `spawnproto::GRANT_WORDS`.
                    let (a0, a1, a2) = match grant {
                        Some((_, child)) => child,
                        None => (0, arg, 0),
                    };
                    let started = thread_control_block_start(tcb, a0, a1, a2);
                    cap_delete(tcb);
                    started
                }
                None => false,
            };
            // The narrowed endpoint was only ever the means of wiring: the child holds its own copy
            // and the caretaker holds the other end. Dropped whether or not the build worked, so a
            // failed spawn does not cost this capability table a slot for the rest of the boot.
            if let Some(dir_ep) = narrowed {
                cap_delete(dir_ep);
            }
            // Our capability to the job's region goes back now. It was only ever the means of
            // building: since §32 the reap is a method on the supervision endpoint, so nothing in
            // this system holds a capability to a *live* job's memory. A build or a start that
            // failed leaves nothing running in the region, so we reclaim it here rather than wait
            // for a death that will never come.
            if let Some(r) = region {
                if !ok {
                    reclaim(r);
                }
                cap_delete(r);
            }
            // **A redirected child owes the shell no answer**, because its answer is going
            // somewhere else, so the shell has nothing to read and no way to find out that the
            // spawn failed. One ack closes that hole. An unredirected child is unchanged: the
            // child's own message is the shell's single read, and a failure is the sentinel.
            //
            // **A screen-narrowed child owes the same ack, for the same reason** (DECISIONS §106):
            // its output is not going to `result_ep` either, and its completion signal is the fault
            // endpoint above, not this one, so a build failure has to reach the shell here or not
            // at all.
            if wiring.sink || wiring.screen {
                send(
                    result_ep,
                    if ok {
                        spawnproto::SPAWN_OK
                    } else {
                        spawnproto::SPAWN_FAILED
                    },
                    0,
                    0,
                );
            } else if !ok {
                send(result_ep, spawnproto::SPAWN_FAILED, 0, 0);
            }
            // **A child that was never built cannot end its own second stream**, and the shell
            // drains that stream to `OP_EOF` before it reads anything else, so nothing would ever
            // come back. init closes it on the child's behalf. It is the same hole `SPAWN_OK`
            // closed for the output side, one stream over.
            if !ok && let Some(ep) = diagnostics {
                send(ep, byte_sink_proto::eof(), 0, 0);
            }
        }

        // Drop our copies of every delegated cap: the child holds what it needs (the job frame is
        // mapped, the budget and the streams inserted), and the shell holds the originals it kept
        // (the job untyped for teardown, the pipe it minted). This keeps init's 16-slot capability table from
        // filling across a long session.
        for s in [job_ut, job_fr, sink, source, diagnostics, screen, budget]
            .into_iter()
            .flatten()
        {
            cap_delete(s);
        }
    }
}

// -------------------------------------------------------------------------------------------
// The thin shapes over the ABI. The loader itself is `supervision_proto`'s, which is the tree's
// only one since milestone 96.
// -------------------------------------------------------------------------------------------

/// **Build a `fs_subtree_caretaker` for one directory grant and hand back the narrowed endpoint**
/// (milestone 31 phase 3, DECISIONS §92).
///
/// This is the whole of what "the command line is a grant expression" needed and did not have. The
/// FS service's unit of authority is a *directory* (§27) and `rm rmtree/rm-keep` says less than that,
/// so the narrowing is a **caretaker**: a process that holds the file service, descends once into the
/// granted directory asking for exactly the granted rights, and serves the same contract on an
/// endpoint of its own. The program then holds that endpoint and **nothing that names the FS
/// server**, so "it cannot reach a sibling directory" is a property of its capability table rather than of a
/// branch it is trusted to take.
///
/// `region` is the client's region, and everything here comes out of it; see
/// [`DIR_JOB_REGION_PAGES`] for why that is the lifetime rule rather than a convenience.
/// `care_words` are the caretaker's three `START` words exactly as the shell packed them.
///
/// # The handshake is what makes this safe to call from init
///
/// init has no second thread: it is this loop, and a `RECV` that never completes is a machine that
/// never takes another command. So the readiness endpoint is not an optimization, it is the thing
/// that bounds this call, and `fs_subtree_caretaker` answers `DESCENT_REFUSED` rather than trapping
/// precisely so that `rm nosuchdir/x` costs a refusal instead of the prompt.
///
/// # BUGS
///
/// **A caretaker that dies before it answers still parks init.** The handshake covers the refusal a
/// person can cause by typing a name that is not there; it does not cover an image whose caretaker
/// faults on its own stack, because a corpse sends nothing. Nothing in the ABI offers a receive with
/// a deadline, and giving init one would mean a second thread inside the process this system's whole
/// design keeps small. The exposure is a build defect rather than an input, and it is the same one
/// `kernel::user::fs_service::wait_for_caretaker` has carried since milestone 47.
fn build_caretaker(
    own_ut: u64,
    region: u64,
    care: &elf::Elf,
    fs: Fs,
    care_words: (u64, u64, u64),
) -> Option<u64> {
    let narrow_ep = retype_obj(region, abi::objtype::RENDEZVOUS).ok()?;
    let ready = retype_obj(region, abi::objtype::RENDEZVOUS).ok()?;
    // Its whole authority, and reading these three lines is reading it: the file service to
    // attenuate, the endpoint it will serve, and one place to say it is ready. No untyped, no clock,
    // no terminal, and nothing that could name another process.
    let built = build_child(
        own_ut,
        region,
        care,
        &ChildEndowment {
            caps: &[
                (fs.ep, abi::rights::WRITE),
                (narrow_ep, abi::rights::READ),
                (ready, abi::rights::WRITE),
            ],
            maps: &[(FS_CLIENT_PAGE_VA, fs.page, abi::address_space::MAP_RW)],
            stack_pages: CARETAKER_STACK_PAGES,
            ..ChildEndowment::new()
        },
    );
    // **Deliberately unsupervised** (`fault: None`). A caretaker built into the client's region is
    // already collected by the client's reap, and giving it a fault slot on `deaths` would put a
    // second death message on that endpoint for a thread whose region the first reap had already
    // taken away: `job_undertaker` would then be asked to collect a tid the scheduler no longer
    // knows, which it reads as the kernel contradicting itself and traps on.
    let tcb = built.ok()?;
    let started = thread_control_block_start(tcb, care_words.0, care_words.1, care_words.2);
    cap_delete(tcb);
    if !started {
        cap_delete(ready);
        cap_delete(narrow_ep);
        return None;
    }
    // The one bounded wait. `READY` means the descent succeeded and everything the client can reach
    // it will reach through the handle that one request minted.
    let (verdict, _, _) = recv(ready);
    cap_delete(ready);
    if verdict == filesystem_proto::fixture::READY {
        Some(narrow_ep)
    } else {
        // A refused descent. The caretaker has already exited; the endpoint goes back, and the
        // caller answers the prompt without building the program.
        cap_delete(narrow_ep);
        None
    }
}

/// **Give a failed job's region back, retrying while something in it can still run.**
///
/// A `DESTROY` of a region holding a live thread is refused with §16's kill armed, and one
/// preemption later the retry succeeds; that is `sched::reclaim_region`'s documented contract and it
/// is why the shell's `^C` escalation is a loop. It reaches this path when a directory grant's
/// caretaker was built and the program behind it was not: the caretaker is parked in `RECV`, the
/// endpoint sweep wakes it, and a single attempt would leave [`DIR_JOB_REGION_PAGES`] spoken for
/// until the machine stops. That is the *out of memory* path, which is exactly where a leak hurts
/// most.
///
/// Bounded and silent on failure, which is not the same call `job_undertaker` makes and the
/// difference is who is running: that program's whole job is collecting, so giving up there means a
/// leak nobody will see and trapping is the loud answer. This is the spawn service, and a trap here
/// takes the prompt down over one command's memory. The pool is bounded and renewable, so a region
/// this could not reclaim costs later commands and does not end them.
fn reclaim(region: u64) {
    for _ in 0..RECLAIM_ATTEMPTS {
        if supervision_proto::memory_region_destroy(region) {
            return;
        }
        user_rt::yield_now();
    }
}

/// How many times [`reclaim`] retries. Small, because the only resident it ever waits on is a
/// caretaker that has already been woken and doomed, and one preemption is enough.
///
/// # BUGS
///
/// **One preemption is enough and sixty-four attempts do not reliably buy one**, which is a
/// different claim from the one above and the measurement is not this crate's. `login_test_client`
/// carried this exact loop, over the same refusal, and it was measured on 2026-08-27 (see that
/// program's `destroy_with_retry` and notes/load-sensitive-assertions.md): a `yield_now` that finds
/// work on this core returns in about 130 us, so sixty-four of them can elapse in 8 ms, where the
/// preemption being waited for is a whole tick period (10 ms) away. Whether the count covers the
/// tick is a scheduling outcome the host decides, and at roughly 2x oversubscription it stopped
/// covering it in about one run in three. That client now waits on the property with a clock; this
/// loop was left as it is, deliberately, because the two failures are not the same size: a test
/// client that gives up early fails a run, and this gives up quietly and strands
/// [`DIR_JOB_REGION_PAGES`] until the machine stops. **The failure has never been observed here**,
/// and it would be invisible if it happened, which is the argument for a lane rather than for a
/// number: this wants the same clock-bounded wait and a way to notice when it expires.
const RECLAIM_ATTEMPTS: usize = 64;

/// Carve `pages` off `ut` into a new child untyped we can delegate (milestone 31). The SPLIT grants
/// full rights on the child, including GRANT, so a memory budget can be handed on. The error code
/// `supervision_proto` returns is for the dropped-authority proof, which this crate takes from the
/// raw `invoke` instead, so it is dropped here.
fn memory_region_split(ut: u64, pages: u64) -> Result<u64, ()> {
    supervision_proto::memory_region_split(ut, pages).map_err(|_| ())
}

/// **Say one sentence at the terminal**, through the line discipline, the way the shell does: stage
/// the bytes in the shell's output page (mapped here at [`INIT_OUT_VA`]) and `CALL` `OP_WRITE`.
///
/// The only thing this process ever prints, and it is called before the shell is started so nothing
/// else is writing that page. It exists for the negative control: a claim about what init can no
/// longer do is worth only as much as the check behind it, and only the holder can run that check.
fn announce(term_ep: u64, text: &[u8]) {
    let out = INIT_OUT_VA as *mut u8;
    for (i, &b) in text.iter().enumerate() {
        // SAFETY: the shell's output frame is mapped read/write here, and one line is far under a
        // page.
        unsafe { core::ptr::write_volatile(out.add(i), b) };
    }
    call(term_ep, proto::req(proto::OP_WRITE, text.len() as u64), 0);
}

// -------------------------------------------------------------------------------------------
// The measurement (milestone 104): init measures what init loads.
// -------------------------------------------------------------------------------------------

/// What init found when it asked the archive for a program.
///
/// One rule produces both fields: **init loads nothing it cannot vouch for.** So `elf` is `None`
/// whenever the entry is missing, refused, or not an ELF, and every caller below treats those the
/// same way it always treated a missing entry. `unvouched` exists only for the sentence init prints:
/// a build that did not pack a program and a program whose bytes are not the ones this system was
/// measured against are the same decision and very different news.
struct Lookup<'a> {
    elf: Option<elf::Elf<'a>>,
    /// The archive **had** this entry and the table would not vouch for it.
    unvouched: bool,
}

/// Read a program out of the archive and measure it against the table, or refuse it.
///
/// `Unmeasured` (the table says nothing about this name) and `Mismatch` (it says something else) are
/// both refusals here, which is `measured_boot`'s own rule and the kernel's: a check that passes
/// when there is nothing to check against is not a check. In particular a table that failed to
/// generate refuses **everything**, and the boot stops at the console rather than coming up
/// unmeasured.
fn measured<'a>(fs: &nifefs::Fs<'a>, table: &str, name: &str) -> Lookup<'a> {
    let Some(bytes) = fs.read(name) else {
        return Lookup {
            elf: None,
            unvouched: false,
        };
    };
    if measured_boot::verify_in_manifest(table, name, bytes).is_err() {
        return Lookup {
            elf: None,
            unvouched: true,
        };
    }
    Lookup {
        elf: elf::Elf::parse(bytes).ok(),
        unvouched: false,
    }
}

/// The buffer one printed sentence is composed in. A page would fit, but the sentences are one line
/// each and a fixed frame-local buffer is what keeps [`sentence`] free of an allocator.
const SENTENCE: usize = 192;

/// Compose `prefix name name ... suffix` into `buf`, skipping empty names.
///
/// A hand-rolled `write!` because there is no allocator and `core::fmt` would drag its machinery
/// into a program whose whole job is to be small. It **truncates** rather than failing: a diagnostic
/// cut short still names the first thing that went wrong, and the alternative is a refusal that says
/// nothing at all.
fn sentence<'b>(
    buf: &'b mut [u8; SENTENCE],
    prefix: &[u8],
    names: &[&str],
    suffix: &[u8],
) -> &'b [u8] {
    fn push(buf: &mut [u8; SENTENCE], n: &mut usize, src: &[u8]) {
        for &b in src {
            if *n < SENTENCE {
                buf[*n] = b;
                *n += 1;
            }
        }
    }
    let mut n = 0usize;
    push(buf, &mut n, prefix);
    for name in names {
        if name.is_empty() {
            continue;
        }
        push(buf, &mut n, b" ");
        push(buf, &mut n, name.as_bytes());
    }
    push(buf, &mut n, suffix);
    &buf[..n]
}

/// Unwrap a `Result<u64, ()>` or fault: a half-built system is not worth limping along.
fn must(r: Result<u64, ()>) -> u64 {
    match r {
        Ok(v) => v,
        Err(()) => fail(),
    }
}

/// Fault unless the syscall succeeded.
fn must_ok(ok: bool) {
    if !ok {
        fail();
    }
}

/// Trap. The kernel prints the pc and kills the process, which is the legible way for a builder to
/// say that the system it was asked to build cannot exist.
fn fail() -> ! {
    supervision_proto::fail()
}
