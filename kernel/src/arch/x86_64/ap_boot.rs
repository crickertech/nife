//! **Starting a second logical CPU: INIT-SIPI-SIPI and the real-mode trampoline it needs**
//! (milestone 161's SMP item).
//!
//! aarch64 asks firmware to start a core (PSCI `CPU_ON`) and RISC-V asks SBI (`hart_start`); x86
//! has no firmware call for this at all. Starting a second CPU here means sending it two kinds of
//! inter-processor interrupt through the local APIC: an INIT (reset it into a wait-for-SIPI state)
//! and two STARTUP IPIs ("SIPI"), which name a *physical page below 1 MiB* the target begins
//! executing at, in 16-bit real mode. See `irq::send_init`/`irq::send_startup` for the messages
//! themselves and `boot.s`'s `secondary_boot` (the trampoline that page holds) for what runs there.
//!
//! **This module's whole job is the one thing INIT-SIPI-SIPI cannot do for itself**: getting the
//! trampoline's bytes onto that low page and handing it the one value that varies per core (the
//! stack top), before the messages are sent.
//!
//! # BUGS
//!
//! **Two separate, unresolved failures were found bringing this up, and neither is root-caused.**
//! Both are why `scripts/qemu-runner-x86_64.sh` still defaults `NIFE_SMP` to 1 rather than to a
//! count that actually starts anything: the mechanism in this file is built and does what it
//! says, but nothing downstream of "a second core exists" has been shown safe yet.
//!
//! **1. A third or later secondary, brought up while an earlier one is already online and
//! running, fails intermittently and non-deterministically** (measured extensively on QEMU TCG).
//! At `-smp 3` and above, exactly one secondary typically fails to reach `secondary_main`'s online
//! mark, and *which* one varies run to run (not always the last-attempted, not always a fixed id);
//! the total online count then falls one short and stays there. Instrumented with raw port-I/O
//! checkpoints inside the trampoline (bypassing the console lock entirely), the failing core is
//! seen to reach 64-bit long mode (`ap_long_mode_entry`, past the GDT loads and the
//! `CR3`/`EFER`/`CR0` sequence) but never reach the checkpoint immediately before
//! `jmp secondary_main`, a five-instruction gap (`mov rsp, [rip+...]`, `cpuid`, `shr`, `movzx`)
//! that does nothing unusual and is identical to what the succeeding core(s) just executed.
//! Neither of this port's two working hypotheses survived a direct test: routing `cpu_start`'s
//! wait through `hlt` instead of a tight spin (in case CPU 0's own busy-loop was starving the
//! target vCPU thread of host time under TCG) changed the failure from an occasional full hang to
//! a reliable "gives up cleanly, boot continues", but did not stop the underlying core from
//! failing to start; and copying the trampoline's code bytes only once instead of once per
//! `STARTUP` IPI (in case QEMU's self-modifying-code detection was mishandling a
//! rewrite-and-re-execute of a page another vCPU might be concurrently running from) made no
//! measurable difference either.
//!
//! **2. Two cores crashed under the kernel's own test suite's real scheduler workload. FIXED
//! 2026-08-25 (milestone 161's SMP-crash lane); root cause was a missing cross-core TLB
//! shootdown**, recorded here because this is where the symptom was found and where a reader
//! meets it.
//!
//! The symptom was a `script/test` run at `NIFE_SMP=2` faulting with `rip` = `stack::PAINT`
//! (`0x5afe57ac5afe57ac`), the pattern this kernel writes into a fresh kernel stack, or with
//! `rip` = 0. It reproduced on 10 of 10 runs.
//!
//! The cause was not in this file, and not in the portable scheduler either. `arch::x86_64::mmu`'s
//! `flush_tlb` was **local to the calling CPU**: `invlpg` invalidates one core's TLB and says
//! nothing about any other, and its own doc comment said so and predicted this
//! (*"a multi-CPU kernel needs a software shootdown protocol (an IPI) ... this is the line that
//! will need company"*). SMP arrived and the line never got its company. aarch64 needs none
//! (`tlbi ..., is` is broadcast by the hardware) and RISC-V already had one (an SBI RFENCE), which
//! is why the same portable `sched`/`thread` code runs clean at `-smp 4` on both.
//!
//! The failure that produced is exact rather than vague. `thread::KernelStack::drop` unmaps a dead
//! thread's stack and hands the address range back for reuse; a core that had cached a translation
//! for that range kept it, so when the range was remapped onto **different** physical frames the
//! stale core read the old frame instead. A `Context` read back that way is whatever the old frame
//! now holds, and since a freshly recycled stack page is painted, the `ret` at the end of
//! `switch_to` jumped to the paint. Confirmed by making a stale entry harmless (never reusing stack
//! address space, so no VA is ever remapped onto a new frame): the fault vanished, 8 runs of 8,
//! leaving only that test's own reuse assertion.
//!
//! The fix is `mmu::shoot_down_others`, and the one part of it worth knowing here is **why it is an
//! NMI**: `unmap_page` runs inside `KERNEL_MMU`, an `IrqSafeMutex`, so both the sender and every
//! other core running the same code have interrupts masked, and a maskable IPI would deadlock the
//! first time two cores spawned and reaped concurrently. notes/riscv-tlb-shootdown.md already
//! names that property as load-bearing for RISC-V, which gets it from M-mode; on x86 the NMI is the
//! only delivery `cli` cannot suppress. See that function's own doc comment.
//!
//! **3. A secondary is brought up and idles, but the suite cannot agree on which core booted**
//! (found by this lane while verifying #2, and **not fixed**: it is a separate bug in a different
//! subsystem). `arch::x86_64::boot_cpu_id` reads CPUID leaf 1's initial local APIC id, which is
//! *"which core am I"* and not *"which core booted"*. aarch64 answers a constant `0` and RISC-V
//! returns the hart id recorded at boot; only this port recomputes it per caller. So any test body
//! that §28's placement has migrated onto a secondary gets that secondary's id as "the boot core",
//! and `smp::tests::every_secondary_runs_scheduled_work` then waits for a `RAN_ON` mark the real
//! boot core never sets ("secondary core 0 never ran scheduled work"). `stack.rs`'s high-water
//! report skips a slot chosen the same way, so it scans a never-painted slot and reports a
//! secondary stack at 65536/65536. One cause, both symptoms, roughly half of runs at `NIFE_SMP=2`.
//! It reproduces with **no shootdown code present at all** (checked against the pre-fix tree), so
//! it predates #2's fix rather than following from it. The likely shape of the answer is a boot-time
//! record, as RISC-V's `boot_hartid` already is.
//!
//! **`NIFE_SMP` still defaults to 1**, because #1 and #3 are both open and either can fail a run.
//! #2's fix is verified rather than gated: `user::tests::an_asid_flush_reaches_the_other_cores`,
//! the portable test milestone 58 wrote for exactly this property, **fails on this port without
//! the shootdown and passes with it**, and the whole suite reaches `test result: ok. 177 passed` at
//! two cores once #3 is stepped over. It cannot be a CI gate until the default moves, and the
//! default cannot move until #1 and #3 are answered.
//!
//! Whether #1 is specific to QEMU TCG's emulation or a real bug in this port's own code is exactly
//! the kind of question milestone 87's real hardware would settle. #3 wants no hardware and is
//! small; #1 is the one that wants a lane.

use super::mmu::phys_to_virt;

unsafe extern "C" {
    /// The trampoline's own low VMA (`AP_TRAMPOLINE_PHYS`, link-x86_64.ld): where it executes from,
    /// and so also where its bytes must be copied to before a `STARTUP` IPI can name it.
    static __ap_trampoline_start: core::ffi::c_void;
    /// One past the trampoline's last byte, at the same low VMA. `__ap_trampoline_end -
    /// __ap_trampoline_start` is exactly how much [`prepare`] copies.
    static __ap_trampoline_end: core::ffi::c_void;
    /// Where the trampoline's bytes actually sit in the *loaded* image: link-x86_64.ld gives
    /// `.ap_trampoline` a low VMA but leaves its load address (LMA) an ordinary spot beside
    /// `.rodata`, which is where the loader actually put the bytes.
    static __ap_trampoline_lma: core::ffi::c_void;
    /// The one value the trampoline cannot know at link time: the target core's stack top.
    /// [`prepare`] writes it; `boot.s`'s 64-bit tail reads it back RIP-relative.
    static ap_trampoline_stack_top: u64;
}

/// The physical page a `STARTUP` IPI names: `secondary_boot`'s own address, which is also
/// `__ap_trampoline_start`'s, since the trampoline's VMA *is* where it runs from.
///
/// Page-aligned by construction (`link-x86_64.ld`'s `.ap_trampoline` names `AP_TRAMPOLINE_PHYS`,
/// itself a multiple of 4096), which is what lets [`super::cpu_start`] derive a `STARTUP` vector
/// from it with a plain shift.
pub fn trampoline_phys() -> u64 {
    (&raw const __ap_trampoline_start) as u64
}

/// How many bytes [`prepare`] copies: `__ap_trampoline_end - __ap_trampoline_start`.
pub fn trampoline_size() -> u64 {
    (&raw const __ap_trampoline_end) as u64 - trampoline_phys()
}

/// **Where the trampoline's bytes actually live in the loaded image**, physically: link-x86_64.ld
/// places `.ap_trampoline`'s LMA in the gap it deliberately opens between `.rodata` and `.data`
/// (see that file's own comment on why not `.boot_scratch` or the secondary stacks).
///
/// **Neither the direct map nor the image's own section-by-section map covers this range on its
/// own**, and both for the same reason: it sits inside `memory::image_start()..image_end()` (so
/// `mmu::map_everything`'s direct-map step skips it, the same way it skips `.text`, to avoid a
/// second writable alias), and it is *between* two mapped sections (`.rodata`, `.data`) rather than
/// inside either one, so neither section's own `map_range` reaches it either. `mmu::map_everything`
/// direct-maps it explicitly, by name, the same way `map_firmware_regions` already covers other
/// gaps the general rules miss.
pub fn trampoline_lma() -> u64 {
    (&raw const __ap_trampoline_lma) as u64
}

/// **Copy the trampoline to the page it has to execute from, and hand it this core's stack top.**
///
/// # Safety
/// Must not run while another core might still be reading the SAME copy: there is one trampoline
/// scratch page, shared by every `STARTUP` IPI this kernel ever sends. [`super::cpu_start`] is the
/// only caller, and it does not return until the core it just started is either fully online (past
/// the point it could still be reading this page) or given up on, which is what makes calling this
/// again, for the next core, safe.
pub unsafe fn prepare(stack_top: u64) {
    let dst = trampoline_phys();
    let src = trampoline_lma();
    let len = trampoline_size();

    // SAFETY: `src..src+len` is part of the loaded image (ordinary, file-backed bytes in the gap
    // between `.rodata` and `.data`; see the linker script's comment on why NOT `.boot_scratch` or
    // the secondary stacks, both of which are mutated at runtime), and `mmu::map_everything` maps
    // exactly this range into the direct map by name ([`trampoline_lma`]'s own doc explains why it
    // needs to: neither the general direct-map rule nor either neighboring section's own mapping
    // reaches it). `dst..dst+len` is the low megabyte, which `mmu::init`'s `map_firmware_regions`
    // maps writable (`Flags::kernel_data()`) and which the frame allocator never hands out
    // (`machine::LOW_MEGABYTE`), so nothing else owns those bytes. Non-overlapping: `dst` is a
    // fixed low page (`AP_TRAMPOLINE_PHYS`) and `src` sits beside `.rodata`, comfortably above 1 MiB.
    unsafe {
        core::ptr::copy_nonoverlapping(
            phys_to_virt(src) as *const u8,
            phys_to_virt(dst) as *mut u8,
            len as usize,
        );
    }

    let stack_top_slot = (&raw const ap_trampoline_stack_top) as u64;
    // SAFETY: `stack_top_slot` is inside the page just copied, reached through the direct map, and
    // this is the one caller (see this function's own safety contract).
    unsafe {
        core::ptr::write_volatile(phys_to_virt(stack_top_slot) as *mut u64, stack_top);
    }
}
