//! **Linux: reading the live sysfs, and running `sfdisk`, `mkfs.vfat` and `mount`.** The pure half,
//! which reads what sysfs and udev say, is `crate::linux`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::disk::Disk;
use crate::linux::{assemble, first_partition, mounted};

/// Every considered block device on this machine.
pub fn discover() -> Result<Vec<Disk>, String> {
    let mounts = fs::read_to_string("/proc/self/mounts")
        .map_err(|e| format!("cannot read /proc/self/mounts: {e}"))?;
    Ok(assemble(
        Path::new("/sys/block"),
        &mounts,
        Path::new("/run/udev/data"),
    ))
}

unsafe extern "C" {
    fn geteuid() -> u32;
}

fn is_root() -> bool {
    // SAFETY: `geteuid` takes nothing, cannot fail, and touches no memory of ours.
    unsafe { geteuid() == 0 }
}

fn run(program: &str, args: &[&str], package: &str) -> Result<(), String> {
    let status = Command::new(program).args(args).status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("{program} is not installed; it is in the {package} package")
        } else {
            format!("cannot run {program}: {e}")
        }
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed ({status})", args.join(" ")))
    }
}

/// **Erase `disk` as one FAT32 partition labelled `NIFE`** (MBR, for the reason `macos::erase`
/// gives), and return the new partition's device path. Needs root.
pub fn erase(disk: &Disk) -> Result<String, String> {
    if !is_root() {
        return Err(
            "erasing a disk needs root on Linux; run this program again with sudo".to_owned(),
        );
    }
    for volume in &disk.volumes {
        if volume.mount.is_some() {
            run("umount", &[&format!("/dev/{}", volume.id)], "util-linux")?;
        }
    }
    let device = format!("/dev/{}", disk.id);
    let mut child = Command::new("sfdisk")
        .args(["--wipe", "always", "--wipe-partitions", "always", &device])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run sfdisk (util-linux): {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write as _;
        // One partition of type 0x0c (FAT32 with LBA), the whole disk.
        stdin
            .write_all(b"label: dos\n,,c\n")
            .map_err(|e| format!("cannot talk to sfdisk: {e}"))?;
    }
    let status = child.wait().map_err(|e| format!("sfdisk: {e}"))?;
    if !status.success() {
        return Err(format!("sfdisk {device} failed ({status})"));
    }
    let partition = format!("/dev/{}", first_partition(&disk.id));
    // The kernel re-reads the table when sfdisk asks it to; the node appears shortly after.
    for _ in 0..50 {
        if Path::new(&partition).exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    run(
        "mkfs.vfat",
        &["-F", "32", "-n", "NIFE", &partition],
        "dosfstools",
    )?;
    Ok(partition)
}

/// Mount `partition` at a fresh directory and return it. Uses `udisksctl` as an ordinary user (the
/// desktop's own mechanism, no password for removable media) and `mount` as root.
pub fn mount(partition: &str) -> Result<PathBuf, String> {
    if is_root() {
        let dir = std::env::temp_dir().join(format!("nife-stick-{}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        run(
            "mount",
            &["-t", "vfat", partition, &dir.to_string_lossy()],
            "util-linux",
        )?;
        return Ok(dir);
    }
    run("udisksctl", &["mount", "-b", partition], "udisks2")?;
    let mounts = fs::read_to_string("/proc/self/mounts")
        .map_err(|e| format!("cannot read /proc/self/mounts: {e}"))?;
    mounted(&mounts, partition)
        .map(|(m, _)| m)
        .ok_or_else(|| format!("{partition} did not appear in /proc/self/mounts"))
}

/// Unmount what [`mount`] mounted, so the stick can be pulled.
pub fn unmount(path: &Path) -> Result<(), String> {
    let path = path.to_string_lossy();
    if is_root() {
        run("umount", &[&path], "util-linux")
    } else {
        run("udisksctl", &["unmount", "-p", &path], "udisks2")
            .or_else(|_| run("umount", &[&path], "util-linux"))
    }
}
