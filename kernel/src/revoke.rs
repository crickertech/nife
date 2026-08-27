//! Capability revocation and untyped reclamation (milestone 13, DECISIONS §13; storage reworked
//! at milestone 14 phase C).
//!
//! Until now a granted capability could not be retracted and a spent page could not be reclaimed.
//! That was safe only by a structural accident: retyped frames are **spend-only, never reused**
//! (untyped.rs), so a peer that still mapped a shared frame after the granter left was mapping
//! valid, non-reused memory. `memory_region::destroy` carried a tripwire saying exactly this: wiring up
//! any reclamation before revocation exists turns those "harmless" dangling mappings into a
//! use-after-free.
//!
//! This module is that revocation. It keeps a **mapping database, lite**: every mapping of an
//! untyped-derived page. To revoke a page it unmaps it from *every* address space that held it and
//! deletes every `PageFrame` capability to it, after which no holder maps it and no capability names
//! it, so the page is safe to return to the allocator. seL4 keeps a full capability-derivation
//! tree and revokes a *subtree*; this keeps only the unmap side and revokes *all* derivatives of a
//! page, which is precisely what reclamation wants.
//!
//! # Who pays for the records (phase C, the heap's last customer)
//!
//! The database used to be a global `Vec`: one entry per user mapping, growing without bound, on
//! the kernel's heap. **Now the mapper pays.** Each address space's records live in log pages
//! retyped from *its own* untyped region, reached through a fixed registry of live spaces (root,
//! region, log head). A process that maps a thousand shared pages spends its own budget recording
//! them; a process that cannot afford the record cannot make the mapping. And teardown got
//! simpler, not more complex: the log pages are region pages, so `memory_region::destroy` reclaims the
//! records with the process, and "forget this root" is one registry slot going empty.

use crate::arch::mmu;
use crate::sync::{IrqSafeMutex, rank};

/// One recorded mapping: `va` in the owning space maps `phys`. `phys == 0` is a tombstone (RAM
/// starts at `0x4000_0000` on this board, so no real frame is 0).
#[repr(C)]
#[derive(Clone, Copy)]
struct LogEntry {
    phys: u64,
    va: u64,
}

/// How many entries fit a log page after its header.
const LOG_ENTRIES: usize = 255;

/// One page of mapping records, retyped from the owning space's region. Exactly one frame.
#[repr(C)]
struct LogPage {
    /// Physical address of the next (older) log page in this space's chain; 0 ends it.
    next: u64,
    /// High-water mark of entries ever written here. Slots below it may be tombstones.
    used: u64,
    entries: [LogEntry; LOG_ENTRIES],
}

const _: () = assert!(size_of::<LogPage>() == page_frames::FRAME_SIZE as usize);

/// A live address space, as revocation sees it: where its tables root, which region pays for its
/// records, and its newest log page.
struct SpaceLog {
    root: u64,
    region: u64,
    /// Physical address of the newest log page; 0 until the first record needs one.
    head: u64,
}

/// The most concurrently-live address spaces the registry can track: every user thread has one
/// (bounded by `MAX_THREADS` = 128), plus headroom for the tests' bare `AddressSpace`s.
const MAX_SPACES: usize = 160;

/// **The registry of live address spaces.** Fixed (milestone 14 phase C): the records themselves
/// live in the spaces' own regions, so this is just the index that finds them, bounded by how
/// many spaces can exist at once.
static SPACES: IrqSafeMutex<[Option<SpaceLog>; MAX_SPACES]> =
    IrqSafeMutex::new(rank::MAPPINGS, [const { None }; MAX_SPACES]);

/// A log page, by physical address, through the direct map.
///
/// # Safety
/// `phys` must be a page this module linked into some space's chain (retyped exclusively for the
/// log), and the caller must hold the `SPACES` lock, which serializes every touch of every chain.
unsafe fn log_page(phys: u64) -> &'static mut LogPage {
    // SAFETY: per the function's contract; the direct map names every RAM page.
    unsafe { &mut *(mmu::phys_to_virt(phys) as *mut LogPage) }
}

/// Enter a newly created address space into the registry. `false` (and the caller should fail
/// creation) if the registry is full.
pub fn register_space(root: u64, region: u64) -> bool {
    let mut spaces = SPACES.lock();
    let Some(slot) = spaces.iter_mut().find(|s| s.is_none()) else {
        return false;
    };
    *slot = Some(SpaceLog {
        root,
        region,
        head: 0,
    });
    true
}

/// Forget an address space. Called from `AddressSpace::drop` **before** its region is destroyed:
/// its page tables and its log pages are about to be freed, and a stale registry entry would send
/// a later revoke walking memory that belongs to someone else. The records need no cleanup of
/// their own; they are region pages, and the region is about to come back whole.
pub fn forget_root(root: u64) {
    let mut spaces = SPACES.lock();
    for slot in spaces.iter_mut() {
        if slot.as_ref().is_some_and(|s| s.root == root) {
            *slot = None;
        }
    }
}

/// Record that the address space rooted at `root` mapped `phys` at `va`, **paid for by that
/// space's own region**: the record goes in an existing log slot, or a fresh log page is retyped
/// from the region (rank MAPPINGS > `MEMORY_REGION` makes that legal under this lock). Returns `false` if
/// the space is unknown or its budget is exhausted, and the caller must then unmap what it just
/// mapped: an unrecorded mapping is invisible to revocation, which is the §13 use-after-free.
#[must_use]
pub fn record_mapping(phys: u64, root: u64, va: u64) -> bool {
    let mut spaces = SPACES.lock();
    let Some(space) = spaces.iter_mut().flatten().find(|s| s.root == root) else {
        return false;
    };

    // A free slot in the chain: the first tombstone, or headroom in any page.
    let mut page_phys = space.head;
    while page_phys != 0 {
        // SAFETY: pages in the chain are the log's own; SPACES is held.
        let page = unsafe { log_page(page_phys) };
        for e in page.entries.iter_mut().take(page.used as usize) {
            if e.phys == 0 {
                *e = LogEntry { phys, va };
                return true;
            }
        }
        if (page.used as usize) < LOG_ENTRIES {
            page.entries[page.used as usize] = LogEntry { phys, va };
            page.used += 1;
            return true;
        }
        page_phys = page.next;
    }

    // No room anywhere: a fresh page from the space's own budget becomes the new head. Retyped
    // zeroed, so `used = 0` and `next = 0` need no separate scrub.
    let Some(fresh) = crate::memory_region::retype_page(space.region) else {
        return false; // out of budget: the caller unmaps, the process pays for its own limit
    };
    // SAFETY: just retyped exclusively for the log; SPACES is held.
    let page = unsafe { log_page(fresh) };
    page.next = space.head;
    page.entries[0] = LogEntry { phys, va };
    page.used = 1;
    space.head = fresh;
    true
}

/// **Undo one [`record_mapping`]**: tombstone the record that `root` maps `phys` at `va`, without
/// touching any other space's view of `phys`.
///
/// The counterpart the rollback paths needed and did not have. [`unmap_everywhere`] is the wrong
/// tool for undoing a half-finished `PageFrame::MAP`, because it is space-blind by design: it pulls
/// the physical page out of *every* address space that maps it, which for a shared frame would
/// punish the peers for the mapper's failure. This removes exactly the one record the caller just
/// wrote, and the caller unmaps exactly the one page it just mapped.
///
/// Silent when the space or the record is unknown, because both mean the same thing to a rollback:
/// there is nothing left to undo.
pub fn forget_mapping(phys: u64, root: u64, va: u64) {
    let mut spaces = SPACES.lock();
    let Some(space) = spaces.iter_mut().flatten().find(|s| s.root == root) else {
        return;
    };
    let mut page_phys = space.head;
    while page_phys != 0 {
        // SAFETY: pages in the chain are the log's own; SPACES is held.
        let page = unsafe { log_page(page_phys) };
        for e in page.entries.iter_mut().take(page.used as usize) {
            if e.phys == phys && e.va == va {
                e.phys = 0; // tombstone: reusable by the next record, exactly as a revoke leaves it
                return;
            }
        }
        page_phys = page.next;
    }
}

/// **One entry of what `root` has mapped, resuming from `cursor`** (`abi::address_space::LIST`,
/// milestone 126's `pmap`, DECISIONS §114). `(0, 0)` means done, the same `abi::survey::DONE`
/// convention `SURVEY` uses on the endpoint side: start with `cursor = 0`, feed each returned
/// cursor back, stop when it comes back 0.
///
/// **Reads the space's own revocation log rather than walking page tables**, which is the same
/// move `ps` makes over `/proc`: the kernel already keeps this record for reclamation, so
/// answering "what is mapped" costs nothing the space did not already pay for, and the answer
/// cannot drift out of agreement with what `revoke_page_frame`/`revoke_region` would find. The caller
/// (`kernel::syscall`) turns each `va` this hands back into a `(phys, Flags)` with
/// `arch::mmu::translate_at`, which is where the permission bits `pmap` prints come from; this
/// function knows nothing about flags, on purpose, because the log does not record them.
///
/// **A tombstoned entry (`phys == 0`, an unmapped or revoked slot) is skipped silently**, and a
/// slot a later `record_mapping` reused for an unrelated mapping is not detected: unlike
/// `SURVEY`'s slot table, a log entry carries no generation, so a resumed cursor that outlives a
/// tombstone-then-reuse in the same slot can read the wrong mapping there. Recorded in
/// `crates/pmap`'s `BUGS`, because nothing in this module can tell the difference.
///
/// **Cursor encoding**: a log page's own physical address (always page-aligned, so its low 12
/// bits are free and never legitimately 0 -- RAM starts at `0x4000_0000` on this board, the same
/// fact [`LogEntry`]'s tombstone convention leans on) OR'd with the index into that page. Pages
/// are only ever *prepended* to a space's chain, never freed or reordered until the whole space
/// dies (`forget_root`), so a cursor this function handed back stays valid regardless of what
/// `record_mapping` does to the chain in between: a page prepended after a walk starts is simply
/// never reached by it (`SURVEY`'s "can miss a member born into an already-passed slot," one
/// object type over), and a page already visited is never revisited because pages are singly
/// linked toward *older* entries and a cursor only ever advances that way.
pub fn list_mapping(root: u64, cursor: u64) -> (u64, u64) {
    let spaces = SPACES.lock();
    let Some(space) = spaces.iter().flatten().find(|s| s.root == root) else {
        // The space is gone (a race with teardown, or a stale cursor from a caller that kept one
        // past the space's life): nothing to report. Not a refusal; the syscall layer already
        // checked the capability before calling here.
        return (0, 0);
    };

    const PAGE_MASK: u64 = !(page_frames::FRAME_SIZE - 1);
    let (mut page_phys, mut index) = if cursor == 0 {
        (space.head, 0usize)
    } else {
        (cursor & PAGE_MASK, (cursor & !PAGE_MASK) as usize)
    };

    while page_phys != 0 {
        // SAFETY: `page_phys` is either this space's own `head` or a cursor this function minted
        // from a page in this space's chain; SPACES is held.
        let page = unsafe { log_page(page_phys) };
        while index < page.used as usize {
            let entry = page.entries[index];
            index += 1;
            if entry.phys != 0 {
                // `page_phys` is always nonzero (RAM starts at 0x4000_0000), so `page_phys |
                // index` can never collide with the `(0, 0)` DONE sentinel below, even for the
                // very last real entry in a space, where `index` has just walked off the end of
                // this page. The bug this replaced returned a bare `0` for exactly that case
                // (nothing left to point at), which a caller cannot tell apart from "this call
                // found nothing": the last real mapping in every space was silently dropped.
                // Pointing at the (now out-of-range) position instead costs one extra call --
                // the next one finds `index == page.used`, falls through to `page.next`, and
                // returns genuine `(0, 0)` if there is none -- and it is what keeps a hit's
                // `next` and the DONE sentinel from ever being the same value.
                return (page_phys | index as u64, entry.va);
            }
        }
        page_phys = page.next;
        index = 0;
    }
    (0, 0)
}

/// Unmap `phys` from every address space whose log records it, tombstoning the records.
///
/// The unmapping (TLB broadcast included) happens under the registry lock. The old database
/// lifted victims out first to keep the §9 critical section short; without a heap there is
/// nowhere to lift them to, and the honest accounting is: revocation is rare, the lock is
/// contended only by `record_mapping` (a syscall path that can afford to wait), and a `tlbi`
/// completes in hardware regardless of who spins on what.
///
/// `spare` is an address-space root to leave alone (0 spares none). Only the device take-back
/// below passes one: reclamation must unmap everywhere or the page is not safe to reuse, but
/// transferring a device wants the invoker to keep what it is about to hand on. See
/// [`revoke_device_from_others`].
fn unmap_everywhere(phys: u64, spare: u64) {
    let spaces = SPACES.lock();
    for space in spaces.iter().flatten() {
        if space.root == spare {
            continue;
        }
        let mut page_phys = space.head;
        while page_phys != 0 {
            // SAFETY: chain pages under the held SPACES lock.
            let page = unsafe { log_page(page_phys) };
            for e in page.entries.iter_mut().take(page.used as usize) {
                if e.phys == phys {
                    mmu::unmap_user_at(space.root, e.va);
                    e.phys = 0; // tombstone: reusable by the next record
                }
            }
            page_phys = page.next;
        }
    }
}

/// **Revoke a single-page frame from everyone.** Delete every `PageFrame(phys, 1)` capability from
/// every capability table, then unmap `phys` from every address space. Caps go **first**, so a
/// `PageFrame::MAP` that starts after this cannot re-establish a mapping we would then miss. (The
/// remaining window, an in-flight map on another core between the cap delete and the unmap, is the
/// SMP race §13 names; a full mapping-database lock is seL4's answer and this milestone's
/// deferral.)
///
/// **This is not "no capability names the page afterwards", and the earlier wording that said so
/// was wrong from the day §102 landed.** The sweep is by exact object, so a `PageFrame(p, n)` run
/// that merely *contains* `phys` survives it. That is harmless where this function is used (a
/// driver taking one of its own kernel-minted pages back, the tests below) and would not be
/// harmless on a reclamation path, which is why reclamation does not use it:
/// [`revoke_region`] sweeps capabilities by overlap over the whole range instead.
///
/// The single-page case of [`revoke_page_frame_run`]. `PageFrame::REVOKE`'s own syscall path uses
/// that one, because there the object being revoked really is the run named by the invoked
/// capability.
///
/// **Test-only in a non-`initrd` build**, and it stopped being anything else when `revoke_region`
/// took its own range sweep: its remaining callers are the two `#[cfg(all(test, initrd))]` suites
/// that revoke a single kernel-minted page (`user::disk_tests`, `user::tests`) and the suite in
/// this file. Kept rather than deleted because those are the tests that prove the property, and
/// because it is what a caller wanting exactly one page should still reach for.
#[cfg_attr(not(all(test, initrd)), allow(dead_code))]
pub fn revoke_page_frame(phys: u64) {
    crate::sched::delete_page_frame_caps(phys, 1);
    unmap_everywhere(phys, 0);
}

/// **Revoke a run of `count` frames from everyone** (DECISIONS §102). Deletes every capability
/// naming exactly the run `(phys, count)`, once, then unmaps each of the `count` physical pages from
/// every address space that mapped it. This is `PageFrame::REVOKE`'s body: the capability being
/// invoked names the whole run, so the whole run is what gets deleted and unmapped, in one syscall
/// regardless of how many pages the run holds.
///
/// The two passes are not the same granularity on purpose. Capability deletion is one exact-object
/// match against `(phys, count)`, because that is the one capability (and its narrowed derivatives)
/// this invocation could possibly be revoking. Unmapping stays per-page, because the mapping
/// database records one entry per mapped virtual page regardless of which capability produced it
/// (`PageFrame::MAP` loops over the run and records each page individually); that granularity does
/// not change here.
pub fn revoke_page_frame_run(phys: u64, count: u64) {
    crate::sched::delete_page_frame_caps(phys, count);
    for k in 0..count {
        unmap_everywhere(phys + k * page_frames::FRAME_SIZE, 0);
    }
}

/// **Take a device's registers back from everyone else** (milestone 23, DECISIONS §41). Delete
/// every `DeviceFrame` capability naming `phys` except the invoking thread's own, then unmap `phys`
/// from every address space except the invoker's. Afterwards exactly one process can reach the
/// device: the one that asked.
///
/// The asymmetry with [`revoke_page_frame`] is the point, not an oversight. Revoking a *frame* exists to
/// make reclamation safe, so the revoker's own capability and mapping must go too: the page is about
/// to be returned to the allocator and reused. A device page is never reclaimed; revoking it exists
/// to make ownership **exclusive**, which is what live replacement needs between tearing one driver
/// down and endowing the next. A take-back that also deleted the invoker's capability would leave
/// the registers unreachable forever, because only the kernel mints a `DeviceFrame` and it does so
/// once, at boot.
///
/// This is one level of the capability-derivation tree §13 deferred, and only one: the invoker is
/// the root by construction (it holds `GRANT` and it is the one asking), and every other holder is
/// treated as a derivative. Revoking one *named* holder while sparing another still wants the real
/// tree, and still is not built.
pub fn revoke_device_from_others(phys: u64) {
    crate::sched::delete_device_frame_caps_from_others(phys);
    // The invoker is the current thread, so its address space is the one installed in TTBR0 (satp
    // on RISC-V) right now: no lookup, and no way for the spare to name someone else's space.
    unmap_everywhere(phys, mmu::current_user_root());
}

/// Revoke every page in `[base, base + size)`. `memory_region::destroy` calls this before
/// returning a region to the allocator, which is what turns the old "spend-only, never reused"
/// invariant into the stronger "no live mapping survives" one that makes reuse actually safe.
///
/// **Two passes, at two granularities, and the split is the point.** The capability sweep is over
/// the whole range **once**, up front, and it matches by overlap rather than by object equality
/// ([`crate::sched::delete_page_frame_caps_overlapping`], which carries the reasoning). The unmap
/// sweep stays per-page and mapping-log-driven, because the log is per-page and says nothing about
/// which capability produced an entry.
///
/// Doing capabilities first, for the whole range, is the same ordering [`revoke_page_frame`]
/// documents one function up, applied at range scale: a `PageFrame::MAP` that starts after the
/// sweep has no capability left to map with, so it cannot re-establish a mapping the unmap pass
/// would then miss.
///
/// It also fixes what the per-page version could not see. That version deleted capabilities one
/// *mapped* page at a time, so a capability naming a page nobody had mapped was never a candidate:
/// the log had no record to find it by. A range sweep does not consult the log, so a
/// retyped-but-never-mapped frame in a destroyed region loses its capability too.
///
/// The unmap pass is one page per iteration: find a recorded page in range under the registry lock,
/// release it, then unmap (unmapping retakes the registry lock, so it cannot be called while it is
/// held). Each pass tombstones every record of its page, so the scan strictly shrinks and
/// terminates.
pub fn revoke_region(base: u64, size: u64) {
    crate::sched::delete_page_frame_caps_overlapping(base, size);
    loop {
        let victim = {
            let spaces = SPACES.lock();
            let mut found = None;
            'scan: for space in spaces.iter().flatten() {
                let mut page_phys = space.head;
                while page_phys != 0 {
                    // SAFETY: chain pages under the held SPACES lock.
                    let page = unsafe { log_page(page_phys) };
                    for e in page.entries.iter().take(page.used as usize) {
                        if e.phys >= base && e.phys < base + size {
                            found = Some(e.phys);
                            break 'scan;
                        }
                    }
                    page_phys = page.next;
                }
            }
            found
        };
        match victim {
            // Unmap only: the capability sweep above already covered the whole range, and calling
            // `revoke_page_frame` here would re-run an exact-match sweep per page that by
            // construction can no longer find anything.
            Some(phys) => unmap_everywhere(phys, 0),
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use paging::Flags;

    use super::*;
    use crate::cap::{Rights, page_frame_run_cap, page_frame_run_len};
    use crate::user::AddressSpace;

    /// Three consecutive pages out of one region, or a skipped test if the region did not hand
    /// them out contiguously. A region is a contiguous span and `retype_page` bumps through it, so
    /// this holds; it is asserted rather than assumed because the run tests below are meaningless
    /// if it ever stops holding.
    fn three_in_a_row(region: u64) -> [u64; 3] {
        let pages = [
            crate::memory_region::retype_page(region).expect("retype 0"),
            crate::memory_region::retype_page(region).expect("retype 1"),
            crate::memory_region::retype_page(region).expect("retype 2"),
        ];
        assert_eq!(
            pages[1],
            pages[0] + page_frames::FRAME_SIZE,
            "a region stopped retyping contiguously; a PageFrame run has no meaning without that",
        );
        assert_eq!(pages[2], pages[1] + page_frames::FRAME_SIZE);
        pages
    }

    /// **A multi-page `REVOKE` unmaps every page of the run, in every address space, and deletes
    /// the capability that named it** (DECISIONS §102, milestone 142).
    ///
    /// The property §102 asserted and nothing tested: before this, every revocation test drove the
    /// `count: 1` path, so "the run" and "the page" were the same thing and a loop bound could have
    /// been wrong in either direction without a failure. Two spaces hold different pages of one
    /// three-page run at different virtual addresses, which is also what makes the space-blind
    /// unmap visible: the run is revoked once and both spaces lose their page.
    #[test_case]
    fn revoking_a_run_unmaps_every_page_of_it_everywhere() {
        let mut a = AddressSpace::new(2).expect("no space A");
        let mut b = AddressSpace::new(2).expect("no space B");
        let region = crate::memory_region::create(4).expect("no region");
        let run = three_in_a_row(region);

        // A maps the first two pages; B maps the third, at an unrelated VA. Nothing about the
        // revocation may depend on where a holder put the run.
        let (va_a, va_b) = (0x40_0000u64, 0x80_0000u64);
        for (k, phys) in run[..2].iter().enumerate() {
            let va = va_a + k as u64 * page_frames::FRAME_SIZE;
            a.map_physical(va, *phys, Flags::user_data())
                .expect("map A");
            assert!(record_mapping(*phys, a.root(), va), "record A");
        }
        b.map_physical(va_b, run[2], Flags::user_rodata())
            .expect("map B");
        assert!(record_mapping(run[2], b.root(), va_b), "record B");

        let slot = crate::sched::grant(page_frame_run_cap(
            run[0],
            page_frame_run_len(3),
            Rights::ALL,
        ))
        .expect("grant the run");

        revoke_page_frame_run(run[0], 3);

        for k in 0..2u64 {
            assert!(
                mmu::translate_at(a.root(), va_a + k * page_frames::FRAME_SIZE).is_none(),
                "page {k} of the run survived the revoke in A",
            );
        }
        assert!(
            mmu::translate_at(b.root(), va_b).is_none(),
            "the run's last page survived the revoke in B",
        );
        assert!(
            crate::sched::current_cap(slot).is_err(),
            "REVOKE left the capability that named the run in the revoker's own table",
        );

        crate::memory_region::destroy(region);
    }

    /// **Destroying a region deletes a run capability naming its pages, so the reclaimed memory is
    /// not nameable** (milestone 142's review, CRITICAL 1).
    ///
    /// The regression this exists for: reclamation used to delete capabilities by *exact object*,
    /// one mapped page at a time, and `PageFrame(base, 3)` is equal to no `PageFrame(p, 1)` for any
    /// `p`. So a holder kept a live capability over pages that had gone back to the allocator and
    /// could re-map them read/write once they had been handed out again as a page table or another
    /// process's stack: §13's use-after-free, straight through the door `MemoryRegion::DESTROY`
    /// exists to shut.
    ///
    /// Two capabilities, because the two halves failed for different reasons. The run is the
    /// widening's own hole. The single unmapped page is the older one the same fix closes: it was
    /// never mapped, so the mapping log had no record to find it by and the per-page sweep never
    /// looked at it at all.
    #[test_case]
    fn destroying_a_region_deletes_every_capability_naming_it() {
        let mut space = AddressSpace::new(2).expect("no space");
        let region = crate::memory_region::create(4).expect("no region");
        let run = three_in_a_row(region);
        let unmapped = crate::memory_region::retype_page(region).expect("retype 3");

        let va = 0x40_0000u64;
        space
            .map_physical(va, run[0], Flags::user_data())
            .expect("map");
        assert!(record_mapping(run[0], space.root(), va), "record");

        let run_slot = crate::sched::grant(page_frame_run_cap(
            run[0],
            page_frame_run_len(3),
            Rights::ALL,
        ))
        .expect("grant the run");
        let lone_slot = crate::sched::grant(crate::cap::page_frame_cap(unmapped, Rights::ALL))
            .expect("grant the unmapped page");

        crate::memory_region::destroy(region);

        assert!(
            crate::sched::current_cap(run_slot).is_err(),
            "a run capability outlived the reclamation of the pages it names: §13's use-after-free",
        );
        assert!(
            crate::sched::current_cap(lone_slot).is_err(),
            "a capability to a never-mapped page outlived its region: the mapping log cannot see it",
        );
        assert!(
            mmu::translate_at(space.root(), va).is_none(),
            "destroy reclaimed a page a live address space still maps",
        );
    }

    /// **Revocation unmaps a shared page from every address space that held it.** Two address
    /// spaces map one physical page; after `revoke_page_frame` neither maps it. This is the property
    /// the whole reclamation story rests on: a page may be reused only once no holder still maps
    /// it. (The records now live in the spaces' own regions; nothing else changed here.)
    #[test_case]
    fn revoke_unmaps_a_shared_page_from_every_address_space() {
        let mut a = AddressSpace::new(2).expect("no space A");
        let mut b = AddressSpace::new(2).expect("no space B");
        let shared = crate::memory::alloc().expect("no frame").addr();
        let (va_a, va_b) = (0x40_0000u64, 0x80_0000u64);

        a.map_physical(va_a, shared, Flags::user_data())
            .expect("map A");
        b.map_physical(va_b, shared, Flags::user_rodata())
            .expect("map B");
        assert!(record_mapping(shared, a.root(), va_a), "record A");
        assert!(record_mapping(shared, b.root(), va_b), "record B");

        assert!(
            mmu::translate_at(a.root(), va_a).is_some(),
            "A does not map the page"
        );
        assert!(
            mmu::translate_at(b.root(), va_b).is_some(),
            "B does not map the page"
        );

        revoke_page_frame(shared);

        assert!(
            mmu::translate_at(a.root(), va_a).is_none(),
            "A still maps the revoked page"
        );
        assert!(
            mmu::translate_at(b.root(), va_b).is_none(),
            "B still maps the revoked page"
        );

        crate::memory::free(page_frames::PageFrame::from_addr(shared));
    }

    /// **Destroying an untyped region unmaps its pages, THEN reclaims them.** A page from the
    /// region is mapped into an address space; `memory_region::destroy` must remove that mapping before
    /// the page returns to the allocator, or a later allocation hands out memory a live process
    /// still maps (the use-after-free the tripwire in untyped.rs warns of). Both halves are
    /// asserted: the mapping is gone, and the region's frames come back.
    #[test_case]
    fn destroy_unmaps_a_region_before_reclaiming_it() {
        let mut space = AddressSpace::new(2).expect("no space");
        let region = crate::memory_region::create(4).expect("no region");
        let phys = crate::memory_region::retype_page(region).expect("retype");
        let va = 0x40_0000u64;
        space
            .map_physical(va, phys, Flags::user_data())
            .expect("map");
        assert!(record_mapping(phys, space.root(), va), "record");
        assert!(
            mmu::translate_at(space.root(), va).is_some(),
            "the page was not mapped"
        );

        let free_before = crate::memory::stats().unwrap().free();
        crate::memory_region::destroy(region);
        let free_after = crate::memory::stats().unwrap().free();

        assert!(
            mmu::translate_at(space.root(), va).is_none(),
            "destroy reclaimed a page a live address space still maps: the tripwire's use-after-free",
        );
        assert_eq!(
            free_after,
            free_before + 4,
            "destroy did not return the region's 4 frames to the allocator",
        );
    }

    /// **A mapping that cannot be recorded cannot exist, and the failure is the mapper's own.**
    /// A space with a tiny region records mappings until its budget is gone; the failing record
    /// returns false rather than silently leaving a mapping revocation would miss.
    #[test_case]
    fn an_exhausted_budget_refuses_the_record_not_the_safety() {
        let mut space = AddressSpace::new(0).expect("no space");
        let shared = crate::memory::alloc().expect("no frame").addr();

        // Burn the region down to nothing by recording mappings. The arithmetic that makes
        // refusal certain: a 0-content space has ~15 spendable pages, and 4096 mappings need
        // ~16 log pages plus ~8 table pages, so the budget must run out mid-loop. (2048 was
        // tried first and fit EXACTLY: 6 tables + 9 log pages = 15. Off by nothing.)
        let mut refused = false;
        for i in 0..4096u64 {
            let va = 0x40_0000 + i * page_frames::FRAME_SIZE;
            if space
                .map_physical(va, shared, Flags::user_rodata())
                .is_err()
            {
                refused = true; // ran out mapping: also a fine way for the budget to end
                break;
            }
            if !record_mapping(shared, space.root(), va) {
                refused = true;
                break;
            }
        }
        assert!(
            refused,
            "2048 mappings recorded out of a {}-page region: records are not being paid for",
            crate::memory_region::usage(0).map(|(_, p)| p).unwrap_or(0),
        );

        crate::memory::free(page_frames::PageFrame::from_addr(shared));
    }
}
