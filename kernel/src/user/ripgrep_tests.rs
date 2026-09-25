//! **`ripgrep`, unmodified, from crates.io** (milestone 121; `design/fatal-risks.md` risk 1).
//!
//! Risk 1 is *"only software written for nife runs on nife"*, and it is the most dangerous of the
//! nine because it is structural: optimization cannot fix "nothing runs here". `ripgrep` is the
//! decisive experiment because it is not a toy. It has forty transitive crates, it walks a
//! filesystem, and it is written for a world with threads, a command line, and `mmap`.
//!
//! **The program here is not ours and is not patched.** `helpers/build-ripgrep.sh` downloads the
//! published `ripgrep` crate and builds it with a target spec and three link arguments; there is no
//! overlay, no vendored copy and no fork. Everything this test observes is therefore a fact about
//! the platform rather than about our port of a program.
//!
//! **It skips when the archive has no `rg`**, which is every ordinary build and all of CI, because
//! making the gate fetch a crates.io dependency tree is DECISIONS §46's decision and calef's rather
//! than a lane's. See notes/ripgrep-on-nife.md.
//!
//! **All three ISAs run it**, which is DECISIONS §19 rather than thoroughness: a capability ships on
//! every supported architecture or a scope note records the gap and the plan. `helpers/build-ripgrep.sh`
//! builds for `aarch64-unknown-nife`, `riscv64-unknown-nife` and (since milestone 184)
//! `x86_64-unknown-nife` in one pass, and one test body serves all three because nothing it asserts
//! is architecture-specific. **It runs on all three since milestone 303**, which gave `q35` a RedoxFS
//! image the FS service can find (the block lookup spans virtio-mmio and virtio-pci now), and the
//! x86_64 transcript is byte-identical to the other two.

use super::*;

/// The reason this test gives when nobody built `rg`.
const NO_RIPGREP: &str = "no rg in this archive: build it with helpers/build-ripgrep.sh, which \
                          fetches the published ripgrep crate from crates.io (milestone 121)";

/// **The block server's ELF**, one program in every archive since milestone 291. This was two
/// `cfg` arms (a role of `hello` on aarch64, the dedicated `block_driver` elsewhere) until that
/// milestone packed `block_driver` on aarch64 too; `fs_service::blk_server_image` carries the
/// reason.
fn block_server_image() -> &'static [u8] {
    program("block_driver").expect("no block_driver program in the initrd archive")
}

/// **Somebody else's forty-crate application loads, runs, reaches a real filesystem through a
/// capability it was handed, and then cannot be told what to search for.**
///
/// Every layer below the last clause works, and none of it was written for `ripgrep`. A multi-megabyte
/// ELF the loader maps (4.7 MB on aarch64, 10.7 MB on riscv64); a heap std grows one page at a time under `regex`'s and `ignore`'s allocation
/// patterns; `std::env::current_dir` answering `/` because this process holds a directory; output
/// through the one endpoint it was granted. `ripgrep` did all of that without a line of nife in it.
///
/// **What stops it is that the nife ABI has no argument vector.** `std::env::args()` resolves to
/// std's `sys/args/unsupported.rs`, which yields an empty iterator, because a program is entered
/// with three registers and a capability table (notes/abi.md) and there is nowhere for a command
/// line to live. So `ripgrep` parses zero arguments, finds no pattern, and prints its own usage
/// text. It is not refusing and it has not failed: it was never asked anything.
///
/// **It does not hit DECISIONS §105 at all**, which is the result this test was expected to
/// produce and did not. `ripgrep` asks `std::thread::available_parallelism()` how much parallelism
/// it has, nife's PAL answers `1` honestly, and `ripgrep` picks its own single-threaded walker and
/// searcher. `thread::spawn` is never reached. See notes/ripgrep-on-nife.md.
#[test_case]
fn unmodified_ripgrep_runs_and_has_no_arguments_to_run_on() {
    if program("rg").is_none() {
        crate::testing::skip!(NO_RIPGREP);
    }
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let image = program("rg").expect("no rg program in the initrd archive");
    let faults_before = USER_FAULTS.load(Ordering::Relaxed);
    let Some(rg) = fs_service::start_std_full(
        block_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        image,
    ) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    super::std_tests::assert_fs_service_ready(rg.readiness);

    let mut got = [0u8; 8192];
    let len = super::std_tests::drain_sink(rg.report, &mut got, "rg");
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    rg printed {len} bytes:\n{text}");

    // `ripgrep`'s own usage text, which is what it prints when it is given nothing. Asserting on
    // its words rather than on the whole block, because the block is a stranger's copy and pinning
    // it byte for byte would make a `ripgrep` release a failure here.
    assert!(
        text.contains("ripgrep"),
        "rg printed something that is not ripgrep's own output",
    );
    assert!(
        !text.contains("current working directory"),
        "rg could not name its own directory: the FS grant did not reach it",
    );

    // The exit, on `std_tests`' reasoning: `ripgrep`'s `main` ends in `std::process::exit`, and a
    // program that printed a perfect transcript and then trapped would look identical from here
    // without this.
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(rg.thread)),
        "rg never left: it is neither exited nor faulted",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults_before,
        "rg trapped instead of exiting",
    );

    // **Give the 256-page heap back** (`user::holding`'s reasoning). This program is in the archive
    // only when somebody ran `helpers/build-ripgrep.sh`, so a permanent charge here would make the
    // suite's frame ledger fail for exactly the person running the experiment and pass for everyone
    // else. The thread is already gone, so one call is enough.
    let _ = crate::sched::reclaim_region(rg.heap);
}
