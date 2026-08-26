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
//! **2. Exactly two cores (`-smp 2`, the case #1 above does not touch) start and idle correctly,
//! but crash under the kernel's own test suite's real scheduler workload**, which is a more
//! serious finding than #1: this is not AP bring-up racing, it is ordinary cross-core thread
//! placement and reaping, the same portable `sched`/`thread` machinery aarch64 and RISC-V already
//! run at `-smp 4` without issue. `script/test`, run at `NIFE_SMP=2`, reliably reaches
//! `sched::tests::a_finished_thread_is_reaped_and_its_memory_returned` (a test that spawns eight
//! bare kernel threads, lets `§28`'s placement scatter them across cores, and waits for the
//! reaper) and then faults: one run reported a page fault at `rip 0x0`, another a general
//! protection fault at `rip 0x5afe57ac5afe57ac`. **That second value is not garbage; it is
//! `stack::PAINT`, the exact bit pattern this kernel writes into a fresh kernel stack before
//! anything real occupies it** (`stack.rs`, milestone 84's high-water instrument). A `ret` (or an
//! equivalent read of a saved return address) landing on that value can only mean something read
//! a `Context` back from a stack location that was never written with a real one: a new thread's
//! first switch-to finding paint instead of `Context::for_kernel_thread`'s `thread_trampoline`
//! address, or a reaped thread's freed range being reused before whatever wrote to it synchronized
//! with whatever is about to read it. Not chased further than this characterization: it implicates
//! the interaction between real cross-core thread placement/reaping and something in this port's
//! own arch layer (most plausibly stack allocation, mapping, or the context switch itself, none of
//! which were ever exercised under genuine concurrency before this milestone, since nothing on
//! this architecture had a second core to place work on), rather than the INIT-SIPI-SIPI mechanism
//! this file owns, which had already finished its job by the time either crash occurred.
//!
//! Whether either failure is specific to QEMU TCG's emulation or a real bug in this port's own
//! code is exactly the kind of question milestone 87's real hardware would settle, and each is
//! worth a lane of its own: #2 especially, since it is a correctness question about portable
//! scheduler machinery meeting this architecture's own arch layer for the first time under real
//! concurrency, not a detail of this file's own IPI sequence.

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
