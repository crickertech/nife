//! The disks a boot is given: the virtio image, the RedoxFS images the FS server opens, the
//! GPT and NVMe fixtures, and the blank disk milestone 57 writes a table onto.
//!
//! Every one is remade per boot, so a run meets a fresh fixture rather than whatever the last
//! one wrote.

use crate::host::{run, workspace_root};
use crate::manual::doc_store;

/// Where the nifefs disk image is written.
pub(crate) fn disk_path() -> String {
    workspace_root()
        .join("target/nifefs.img")
        .display()
        .to_string()
}

/// The PCIe transport's copy of the disk image, a sibling of [`disk_path`]. Two files because
/// both transports are now attached **writable** (milestone 32's write path) and QEMU's image
/// locking refuses to attach one file to two devices once either attachment can write. The
/// runner derives this name from `NIFE_DISK`, so the two stay in lockstep.
fn disk_pci_path() -> String {
    workspace_root()
        .join("target/nifefs-pci.img")
        .display()
        .to_string()
}

/// Build the nifefs disk images the virtio-blk driver will read and write.
///
/// **The disk is generated, not checked in**, the same way the flat kernel image is: a binary
/// blob in git is a blob nobody can review. The contents are a couple of tiny files, written
/// through the same `nifefs::write_image` the userspace filesystem server reads back, so the
/// format has exactly one definition.
///
/// `scratch` is the write-path tests' one-block playground: the driver writes a pattern into its
/// block and reads it back, so nothing else on the disk is ever a write target. Regenerating the
/// images here is also what makes test runs independent: whatever a previous run wrote to
/// scratch is rebuilt to zeros.
pub(crate) fn mkdisk() -> bool {
    let files: [(&str, &[u8]); 3] = [
        (
            "motd",
            b"nife: read from a virtio disk, by a driver at EL0.\n",
        ),
        (
            "readme",
            b"this file came off a real block device through a userspace driver.\n",
        ),
        ("scratch", &[0u8; 512]),
    ];
    let size = nifefs::image_size(&files).max(64 * 1024); // pad to a friendly size
    let mut img = std::vec![0u8; size];
    if nifefs::write_image(&files, &mut img).is_err() {
        eprintln!("mkdisk: could not build the image");
        return false;
    }
    // One identical image per transport; see disk_pci_path for why they cannot share a file.
    for path in [disk_path(), disk_pci_path()] {
        if let Err(e) = std::fs::write(&path, &img) {
            eprintln!("mkdisk: could not write {path}: {e}");
            return false;
        }
    }
    true
}

// ===========================================================================================
// The RedoxFS FS server and its test image (milestone 32 phase 2).
//
// The FS-server binary is out-of-workspace (it links the vendored engine), built for the bare
// targets with the pure no_std core (`--no-default-features`) plus the EL0 runtime (`el0`),
// release so the initrd stays small. The test image is made HOST-side by the redoxfs_host tool,
// the same engine the server opens it with; the server never creates. See notes/fs-server.md.
// ===========================================================================================

/// Build the FS-server ELF for `triple`. Its own workspace, so it takes `--manifest-path` and its
/// artifacts land under `redoxfs_server/target/`.
pub(crate) fn redoxfs_server_build(triple: &str) -> bool {
    run(
        "cargo",
        &[
            "build",
            "--manifest-path",
            "redoxfs_server/Cargo.toml",
            // Both binaries out of the one package: the server that opens an image and never
            // creates, and `mkfs` (milestone 57), which creates one and never serves.
            "--bin",
            "redoxfs_server",
            "--bin",
            "mkfs",
            "--no-default-features",
            "--features",
            "el0",
            "--release",
            "--target",
            triple,
        ],
    )
}

/// The FS-server ELF path for a target triple (always the release profile; see `redoxfs_server_build`).
pub(crate) fn redoxfs_server_elf(triple: &str) -> String {
    workspace_root()
        .join(format!(
            "redoxfs_server/target/{triple}/release/redoxfs_server"
        ))
        .display()
        .to_string()
}

/// The `mkfs` ELF path for a target triple. Same package, same profile, same build.
pub(crate) fn mkfs_elf(triple: &str) -> String {
    workspace_root()
        .join(format!("redoxfs_server/target/{triple}/release/mkfs"))
        .display()
        .to_string()
}

/// Where the RedoxFS test image is written. The runners derive exactly this name from
/// `NIFE_DISK` (`${NIFE_DISK%.img}-redoxfs.img`), so the two stay in lockstep.
pub(crate) fn redoxfs_disk_path() -> String {
    workspace_root()
        .join("target/nifefs-redoxfs.img")
        .display()
        .to_string()
}

/// Drive the `redoxfs_host` tool (its own workspace) by `--manifest-path`, quietly. Returns success.
fn redoxfs_host(args: &[&str]) -> bool {
    let mut v = vec![
        "run",
        "--quiet",
        "--manifest-path",
        "tools/redoxfs_host/Cargo.toml",
        "--",
    ];
    v.extend_from_slice(args);
    run("cargo", &v)
}

/// Build the RedoxFS test image the FS server serves: an empty filesystem with the two fixture
/// files (`motd`, `scratch`) the client reads and writes, plus milestone 47's **subtree**
/// (`sub/` with a file and a grandchild, and the sibling `other/` a directory capability must not
/// reach). Made host-side with the pinned engine, so an image the server opens is proven against
/// exactly the code that opens it. Arch-neutral (the on-disk format does not depend on the CPU), so
/// one image serves both ISA test legs.
pub(crate) fn mkredoxfs() -> bool {
    let img = redoxfs_disk_path();
    // **`NIFE_KEEP_REDOXFS=1` keeps an existing image instead of rebuilding it.** This is the
    // deliberate way to run the second-boot case: run the suite once normally, then again with this
    // set, and every mount in the second run is a mount of an image a previous *boot* wrote. That is
    // the condition the cross-boot write failure needs, and doing it this way keeps it independent of
    // which ISA leg happens to run first (the order-coupling that hid the bug for three rounds).
    // Absent the variable, each leg gets a fresh fixture, which is what makes the legs reproducible.
    if std::env::var_os("NIFE_KEEP_REDOXFS").is_some() && std::path::Path::new(&img).exists() {
        eprintln!("mkredoxfs: keeping the existing image (NIFE_KEEP_REDOXFS)");
        return true;
    }
    // Stage the fixture contents in temp files (the host tool's `put` takes a host file), then load
    // them. The contents live in filesystem_protocol::fixture, shared with the client and the fixture's readers.
    let motd = workspace_root().join("target/redoxfs-motd.tmp");
    let scratch = workspace_root().join("target/redoxfs-scratch.tmp");
    if std::fs::write(&motd, filesystem_protocol::fixture::MOTD).is_err()
        || std::fs::write(&scratch, filesystem_protocol::fixture::SCRATCH_INIT).is_err()
    {
        eprintln!("mkredoxfs: cannot stage the fixture files");
        return false;
    }
    let motd = motd.display().to_string();
    let scratch = scratch.display().to_string();
    let Some(tree) = stage_subtree() else {
        return false;
    };
    redoxfs_host(&["mkfs", &img, "16"])
        && redoxfs_host(&["put", &img, filesystem_protocol::fixture::MOTD_NAME, &motd])
        && redoxfs_host(&[
            "put",
            &img,
            filesystem_protocol::fixture::SCRATCH_NAME,
            &scratch,
        ])
        && doc_store().is_some()
        && redoxfs_host(&["import", &img, &tree])
}

/// Stage milestone 47's subtree as a host directory and return its path, for `import` to copy into
/// the image root.
///
/// **`import` rather than a new `mkdir` verb on the host tool**, deliberately: `import` is upstream
/// RedoxFS's own archiver (`redoxfs::archive`), so the directories the confinement tests attack are
/// written by the people who defined the format rather than by us. notes/host-recovery.md already
/// makes that argument for `extract`; this is the same one on the write side.
///
/// The staging directory is rebuilt from scratch each time, because a name left behind by an older
/// fixture would end up in the image and fail the post-run `ls /` check for a reason that has
/// nothing to do with the run.
fn stage_subtree() -> Option<String> {
    use filesystem_protocol::fixture::tree;
    let root = workspace_root().join("target/redoxfs-tree");
    let _ = std::fs::remove_dir_all(&root);
    let sub = root.join(tree::SUB);
    let deeper = sub.join(tree::DEEPER);
    let other = root.join(tree::OTHER);
    // Milestone 47's `rm -r` tree, a sibling of `sub` so a capability to one is provably not a
    // capability to the other. `rm-keep` sits beside the doomed tree, inside the same grant: the
    // program could have removed it and does not, because nothing named it.
    let rmtree = root.join(tree::RMTREE);
    let doomed = rmtree.join(tree::RM_DOOMED);
    let nested = doomed.join(tree::RM_NESTED);
    // Milestone 47's globbing tree, a sibling of both. Two names the pattern matches and two it does
    // not, in one directory, so "the grant is what matched" is a claim about *which* names.
    let globset = root.join(tree::GLOBSET);
    let globdir = globset.join(tree::GLOB_DIR);
    // Milestone 109's batching tree, a sibling of the globbing one: eleven names one pattern
    // matches, which is more than a single grant can carry, so `xargs` has a real directory to
    // sweep at a real prompt. Its own directory rather than more files in `globset`, because the
    // globbing lane asserts that `gl-*.txt` matches exactly two names.
    let globmany = root.join(tree::GLOBMANY);
    // Milestone 50's redirection tree, a sibling of all of them: the witness shell writes files into
    // its root, and it needs somewhere those writes cannot be confused with another test's.
    let redir = root.join(tree::REDIR);
    let mut ok = std::fs::create_dir_all(&globmany).is_ok()
        && std::fs::write(globmany.join(tree::MANY_MISS), tree::MANY_BODY).is_ok();
    for name in tree::MANY_NAMES {
        ok = ok && std::fs::write(globmany.join(name), tree::MANY_BODY).is_ok();
    }
    let ok = ok
        && std::fs::create_dir_all(&deeper).is_ok()
        && std::fs::create_dir_all(&other).is_ok()
        && std::fs::create_dir_all(&nested).is_ok()
        && std::fs::create_dir_all(&globdir).is_ok()
        && std::fs::create_dir_all(&redir).is_ok()
        && std::fs::write(redir.join(tree::REDIR_ONE), tree::REDIR_BODY).is_ok()
        && std::fs::write(redir.join(tree::REDIR_TWO), tree::REDIR_BODY).is_ok()
        && std::fs::write(globset.join(tree::GLOB_ONE), tree::GLOB_BODY).is_ok()
        && std::fs::write(globset.join(tree::GLOB_TWO), tree::GLOB_BODY).is_ok()
        && std::fs::write(globset.join(tree::GLOB_MISS), tree::GLOB_BODY).is_ok()
        && std::fs::write(globdir.join(tree::GLOB_INNER), tree::GLOB_BODY).is_ok()
        && std::fs::write(sub.join(tree::INNER), tree::INNER_BODY).is_ok()
        && std::fs::write(deeper.join(tree::LEAF), tree::LEAF_BODY).is_ok()
        && std::fs::write(other.join(tree::SECRET), tree::SECRET_BODY).is_ok()
        && std::fs::write(rmtree.join(tree::RM_KEEP), tree::RM_KEEP_BODY).is_ok()
        && std::fs::write(rmtree.join(tree::RM_SOLO), tree::RM_BODY).is_ok()
        && std::fs::write(doomed.join(tree::RM_ONE), tree::RM_BODY).is_ok()
        && std::fs::write(doomed.join(tree::RM_TWO), tree::RM_BODY).is_ok()
        && std::fs::write(nested.join(tree::RM_LEAF), tree::RM_BODY).is_ok();
    if !ok {
        eprintln!("mkredoxfs: cannot stage the milestone-47 subtree");
        return None;
    }
    Some(root.display().to_string())
}

/// Where the **crash test's** RedoxFS image is written (milestone 37). The runners derive exactly
/// this name from `NIFE_DISK`, the way they derive the shared one.
pub(crate) fn crash_disk_path() -> String {
    workspace_root()
        .join("target/nifefs-redoxfs-crash.img")
        .display()
        .to_string()
}

/// Build the crash test's image: an empty filesystem with one file, `cut`, holding a known value.
///
/// **Its own disk, and regenerated every run, both deliberately** (milestone 37, DECISIONS §34
/// condition 1). The crash test kills an FS server mid-transaction and leaves the filesystem
/// half-written on purpose. Pointing it at the shared fixture would make every other FS test's
/// result depend on whether this one had run first, and keeping the image across runs would make
/// each run's starting state a function of the last one's damage. Both are the order-coupled fixture
/// DECISIONS §27 spent a day on, so `NIFE_KEEP_REDOXFS` deliberately does **not** apply here: the
/// cross-boot case is interesting for the shared disk and is nothing but noise for this one.
pub(crate) fn mkredoxfs_crash() -> bool {
    let img = crash_disk_path();
    let initial = workspace_root().join("target/redoxfs-crash-initial.tmp");
    if std::fs::write(&initial, filesystem_protocol::fixture::crash::INITIAL).is_err() {
        eprintln!("mkredoxfs_crash: cannot stage the fixture file");
        return false;
    }
    let initial = initial.display().to_string();
    redoxfs_host(&["mkfs", &img, "16"])
        && redoxfs_host(&[
            "put",
            &img,
            filesystem_protocol::fixture::crash::NAME,
            &initial,
        ])
}

/// Where the GPT-partitioned test image is written. The runners derive exactly this name from
/// `NIFE_DISK` (`${NIFE_DISK%.img}-gpt.img`), so the two stay in lockstep.
fn gpt_disk_path() -> String {
    workspace_root()
        .join("target/nifefs-gpt.img")
        .display()
        .to_string()
}

/// Build the milestone-57 test disk: a 64 MiB image whose partition table **`sgdisk` wrote**.
///
/// The point of this image is its provenance. `crates/globally_unique_identifier_partition_table`
/// can lay out a table, and a disk it laid out would test the reader against the writer, which is
/// the weakest test available: every mistake made on the way out is made symmetrically on the way
/// in. So the bytes come from the committed fixture in
/// `crates/globally_unique_identifier_partition_table/tests/fixtures/`, produced by `sgdisk` 1.0.10
/// (gptfdisk, C++), and the guest reads a table this project did not write. The regeneration
/// commands are at the top of
/// `crates/globally_unique_identifier_partition_table/tests/real_disks.rs`.
///
/// The fixture is the first 34 and last 33 blocks of a 64 MiB disk, which is exactly the primary
/// table and the backup table with the 64 MiB of nothing between them left out. Reconstituting it is
/// therefore the head, a run of zeros, and the tail. Nothing is put in the partitions: what is under
/// test is finding them.
pub(crate) fn mkgptdisk() -> bool {
    const BLOCK: usize = 512;
    const BLOCKS: usize = 131_072; // 64 MiB
    let dir =
        workspace_root().join("crates/globally_unique_identifier_partition_table/tests/fixtures");
    let (Ok(head), Ok(tail)) = (
        std::fs::read(dir.join("sgdisk-64m.head")),
        std::fs::read(dir.join("sgdisk-64m.tail")),
    ) else {
        eprintln!("mkgptdisk: cannot read the sgdisk fixtures");
        return false;
    };
    if head.len() != 34 * BLOCK || tail.len() != 33 * BLOCK {
        eprintln!(
            "mkgptdisk: the fixtures are {} and {} bytes; expected {} and {}",
            head.len(),
            tail.len(),
            34 * BLOCK,
            33 * BLOCK,
        );
        return false;
    }
    let mut img = std::vec![0u8; BLOCKS * BLOCK];
    img[..head.len()].copy_from_slice(&head);
    let at = BLOCKS * BLOCK - tail.len();
    img[at..].copy_from_slice(&tail);
    let path = gpt_disk_path();
    if let Err(e) = std::fs::write(&path, &img) {
        eprintln!("mkgptdisk: could not write {path}: {e}");
        return false;
    }
    true
}

/// Where the blank test disk is written. The runners derive exactly this name from `NIFE_DISK`
/// (`${NIFE_DISK%.img}-blank.img`), so the two stay in lockstep.
pub(crate) fn blank_disk_path() -> String {
    workspace_root()
        .join("target/nifefs-blank.img")
        .display()
        .to_string()
}

/// Where the NVMe test image is written; the runners take the full path in `NIFE_NVME` rather
/// than deriving it, because unlike the mmio disks it does not ride beside `NIFE_DISK`.
pub(crate) fn nvme_disk_path() -> String {
    workspace_root()
        .join("target/nife-nvme.img")
        .display()
        .to_string()
}

/// The NVMe test image (milestone 53's storage half): 8 MiB of zeros behind QEMU's `-device nvme`.
/// **Neither number is load-bearing any more** (milestone 318): the boot test compares the
/// server's size answer against the geometry the kernel read from IDENTIFY rather than against a
/// constant, and proves a write landed where it said by writing a second block rather than by
/// expecting an untouched one to read as zeros. So this file may be any size a controller will
/// take, which is what lets the same test run against xenon's 256 GB Micron. Zeros and 8 MiB
/// because a zero file is cheap to make and small is fast to attach. Regenerated per leg like the blank disk, and for the same reason: the
/// test writes it, and a leg starting from the previous leg's damage is not reproducible alone.
pub(crate) fn mknvmedisk() -> bool {
    let path = nvme_disk_path();
    if let Err(e) = std::fs::write(&path, std::vec![0u8; 8 * 1024 * 1024]) {
        eprintln!("mknvmedisk: could not write {path}: {e}");
        return false;
    }
    true
}

/// Build milestone 57's write-half disk: 64 MiB of **zeros**, and that is the whole point.
///
/// It carries no table, no filesystem and no fixture, because what the guest is going to do to it is
/// write both. Regenerated every run and never shared, for milestone 37's reason (DECISIONS §27): a
/// test that partitions a disk cannot be pointed at an image another test reads, and a test whose
/// starting state is last run's damage is not reproducible on its own. `NIFE_KEEP_REDOXFS` does
/// not apply here for the same reason it does not apply to the crash image.
pub(crate) fn mkblankdisk() -> bool {
    let path = blank_disk_path();
    let bytes = std::vec![0u8; (filesystem_protocol::fixture::blank::DISK_BLOCKS * filesystem_protocol::fixture::blank::LBA) as usize];
    if let Err(e) = std::fs::write(&path, &bytes) {
        eprintln!("mkblankdisk: could not write {path}: {e}");
        return false;
    }
    true
}
