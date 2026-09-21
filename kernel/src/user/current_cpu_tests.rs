use super::*;

/// The `current_cpu_reader` program's ELF bytes: reads its own core twice across a yield, with no
/// syscall, and reports both plus the id bound.
fn current_cpu_reader_image() -> &'static [u8] {
    program("current_cpu_reader").expect("no current_cpu_reader program in the initrd archive")
}

/// **A userspace thread learns which core it is on, by loading from memory, on all three
/// architectures.**
///
/// This is the whole of calef's 2026-09-21 ruling that a thread observing itself reads a page,
/// proved end to end: `crates/current_cpu_protocol` lays the page out, `AddressSpace::new` maps it
/// read-only, `schedule()` writes the core at switch-in, and `user_mode_runtime::current_cpu`
/// reads it. (The ruling's `design/decisions/` section is on another branch and is named here
/// rather than cited.)
///
/// **What it asserts is membership in the online set, not a particular number**, and that is the
/// assertion worth having rather than the one that looks stronger. A test that pinned the answer
/// to `0` would pass on QEMU `virt` forever and say nothing, because the scheduler is free to put
/// the thread anywhere. A test that the id names a core which is actually online is the one that
/// catches the failure this tree has already paid for: the VisionFive 2's set is `{1, 2, 3}`, and
/// treating a count as an index put `init` into a parked core's inbox and cost three boots
/// (`crates/cpu_set`). A userspace consumer indexing an array by this value inherits that hazard,
/// so the value had better be a real id.
///
/// **And that neither read is `None`**, which is the other half: a thread cannot execute an
/// instruction without having been switched in first, so by the time this program runs its page has
/// been written, and a `None` here means the page never got mapped or never got written.
#[test_case]
fn a_userspace_thread_reads_a_core_that_is_really_online() {
    let result = crate::sched::create_rendezvous();
    crate::sched::spawn(move || {
        run(
            current_cpu_reader_image(),
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[crate::cap::rendezvous_cap(
                    result,
                    crate::cap::Rights::WRITE,
                )],
                maps: &[],
            },
        )
    })
    .expect("spawn failed");

    let [first, second, bound, _, _] = crate::sched::ipc_recv(result);
    let online = crate::smp::online_harts_mask();

    for (which, cpu) in [("first", first), ("second", second)] {
        assert_ne!(
            cpu,
            current_cpu_protocol::UNSCHEDULED,
            "the {which} read came back unknown: a running thread's page was never mapped or \
             never written",
        );
        assert!(
            cpu < current_cpu_protocol::CPU_ID_BOUND as u64,
            "the {which} read is cpu {cpu}, past the bound userspace sizes its arrays by",
        );
        assert!(
            online & (1 << cpu) != 0,
            "the {which} read is cpu {cpu}, which is not in the online set {online:#b}: a \
             consumer indexing by this would reach a core that is not there",
        );
    }

    // The one number two binaries have to agree on, carried back across the boundary. Cheap here
    // and impossible anywhere else: a host test can only compare the crate against itself.
    assert_eq!(
        bound,
        crate::cpu::MAX_CPUS as u64,
        "userspace sizes its per-cpu arrays by {bound} and the kernel hands out ids under {}; one \
         of them is a stale copy of the other",
        crate::cpu::MAX_CPUS,
    );
}

/// **A space that has never run a thread reads as unknown, not as CPU 0**, and the page is a real
/// mapping rather than a promise.
///
/// The half the end-to-end test above structurally cannot reach: by the time a program can ask,
/// it has been switched in, so the unset state is unobservable from inside the thread. Read here
/// from the kernel's own side of the same frame, before anything has run in the space.
///
/// Zero would be a plausible answer and a wrong one, which is why the page carries a magic and a
/// sentinel instead of relying on a zeroed frame. calef ruled the same day, about the counter
/// frequency, that a wrong number is worse than no number.
#[test_case]
fn a_space_that_never_ran_has_no_answer_and_then_has_the_right_one() {
    let (space, _entry) = load(current_cpu_reader_image()).expect("load failed");
    let va = space
        .current_cpu_page_kernel_va()
        .expect("a freshly loaded space has no current-cpu page");

    // SAFETY: `va` is this space's own frame through the direct map, `PAGE_BYTES` wide, and this
    // is the only thread that can reach it: no thread has been bound to the space.
    let page = unsafe { current_cpu_protocol::CurrentCpuPage::new(va) };
    assert_eq!(
        page.cpu(),
        None,
        "a space whose thread has never been switched in reported a core",
    );

    // The write the context switch makes, made by hand, so the reader and the writer are proved
    // against each other on the machine rather than only on the host.
    space.publish_current_cpu(1);
    assert_eq!(page.cpu(), Some(1), "the published core did not read back");
    space.publish_current_cpu(0);
    assert_eq!(
        page.cpu(),
        Some(0),
        "cpu 0 read back as unknown: the sentinel and a real core got confused",
    );
}

/// **The page comes back when the space dies.** It is the one frame an address space owns that its
/// region does not pay for, so `memory_region::destroy` does not cover it and `Drop` has to free it
/// by hand. That is exactly the shape a leak hides in, and the suite's own frame ledger would only
/// notice it as a slow drift across an unrelated test.
#[test_case]
fn the_page_is_returned_when_the_space_is_dropped() {
    let before = crate::memory::free_page_frames();
    {
        let space = load(current_cpu_reader_image()).expect("load failed").0;
        assert!(
            space.current_cpu_page_kernel_va().is_some(),
            "a loaded space has no current-cpu page to return",
        );
    }
    assert_eq!(
        crate::memory::free_page_frames(),
        before,
        "dropping an address space did not return its current-cpu frame",
    );
}
