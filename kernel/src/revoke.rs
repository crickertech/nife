//! Capability revocation and untyped reclamation (milestone 13, DECISIONS §13; storage reworked
//! at milestone 14 phase C).
//!
//! Until now a granted capability could not be retracted and a spent page could not be reclaimed.
//! That was safe only by a structural accident: retyped frames are **spend-only, never reused**
//! (untyped.rs), so a peer that still mapped a shared frame after the granter left was mapping
//! valid, non-reused memory. `untyped::destroy` carried a tripwire saying exactly this: wiring up
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
//! simpler, not more complex: the log pages are region pages, so `untyped::destroy` reclaims the
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
/// from the region (rank MAPPINGS > UNTYPED makes that legal under this lock). Returns `false` if
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
    let Some(fresh) = crate::untyped::retype_page(space.region) else {
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

/// **Revoke a page from everyone.** Delete every `PageFrame` capability to `phys` from every capability table,
/// then unmap it from every address space. After this no capability names the page and no address
/// space maps it, so it is safe to return to the allocator. Caps go **first**, so a `PageFrame::MAP`
/// that starts after this cannot re-establish a mapping we would then miss. (The remaining window,
/// an in-flight map on another core between the cap delete and the unmap, is the SMP race §13
/// names; a full mapping-database lock is seL4's answer and this milestone's deferral.)
pub fn revoke_page_frame(phys: u64) {
    crate::sched::delete_page_frame_caps(phys);
    unmap_everywhere(phys, 0);
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

/// Revoke every mapped page in `[base, base + size)`. `untyped::destroy` calls this before
/// returning a region to the allocator, which is what turns the old "spend-only, never reused"
/// invariant into the stronger "no live mapping survives" one that makes reuse actually safe.
///
/// One page per pass: find a recorded page in range under the registry lock, release, revoke it
/// (revoking takes the scheduler lock for the capability sweep, which ranks *above* this one, so
/// it cannot be taken while the registry is held). Each pass tombstones every record of its page,
/// so the scan strictly shrinks and terminates.
pub fn revoke_region(base: u64, size: u64) {
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
            Some(phys) => revoke_page_frame(phys),
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use paging::Flags;

    use super::*;
    use crate::user::AddressSpace;

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
    /// region is mapped into an address space; `untyped::destroy` must remove that mapping before
    /// the page returns to the allocator, or a later allocation hands out memory a live process
    /// still maps (the use-after-free the tripwire in untyped.rs warns of). Both halves are
    /// asserted: the mapping is gone, and the region's frames come back.
    #[test_case]
    fn destroy_unmaps_a_region_before_reclaiming_it() {
        let mut space = AddressSpace::new(2).expect("no space");
        let region = crate::untyped::create(4).expect("no region");
        let phys = crate::untyped::retype_page(region).expect("retype");
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
        crate::untyped::destroy(region);
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
            crate::untyped::usage(0).map(|(_, p)| p).unwrap_or(0),
        );

        crate::memory::free(page_frames::PageFrame::from_addr(shared));
    }
}
