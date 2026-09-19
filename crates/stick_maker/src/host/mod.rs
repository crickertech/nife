//! **The half that runs the host's own tools**: `diskutil` on macOS, `sfdisk`, `mkfs.vfat` and
//! `mount` on Linux, four `kernel32` calls on Windows; and the real terminal.
//!
//! Everything that decides anything is elsewhere and tested: which disks may be written
//! (`crate::disk`), what each host's description of a disk means (`crate::macos`, `crate::linux`,
//! `crate::windows`), and the whole conversation with the person (`crate::cli`, against a fake
//! host). What is left here only runs a program or makes a system call and reports what came back.
//!
//! **Why this directory is outside `script/coverage`'s floor, recorded there too**: the coverage
//! run is an unprivileged Linux process, and nothing here can run in it. There is no `diskutil`
//! on Linux and no `kernel32`; the Linux arm erases a whole block device as root. Each arm is
//! instead run for real on its own host: macOS by `scripts/stick-maker-proof.sh` against
//! hdiutil-attached files, Linux and Windows by `.github/workflows/stick-maker-hosts.yml` (a loop
//! device through both paths; Windows discovery). That is a stronger signal than a line count, and
//! it is where a regression here would show.

use std::io::{self, BufRead as _, Write as _};
use std::path::{Path, PathBuf};

use crate::cli::{Console, Host};
use crate::disk::Disk;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// This machine's disks.
pub struct Machine;

/// Standard input and output.
pub struct Terminal;

impl Console for Terminal {
    fn say(&mut self, line: &str) {
        println!("{line}");
    }

    fn ask(&mut self, prompt: &str) -> Result<String, String> {
        print!("{prompt}");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        let n = io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("no answer (end of input); nothing was written".to_owned());
        }
        Ok(line.trim().to_owned())
    }
}

#[cfg(target_os = "macos")]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        macos::discover()
    }

    fn can_erase(&self, _disk: &Disk) -> Result<(), String> {
        Ok(())
    }

    /// `diskutil eraseDisk`, then find the new volume where macOS mounted it, mounting it if the
    /// disk was attached without mounting (as a file-backed disk from `hdiutil -nomount` is).
    fn erase(&self, disk: &Disk) -> Result<PathBuf, String> {
        macos::erase(disk)?;
        for _ in 0..20 {
            let disks = macos::discover()?;
            if let Some(found) = disks.iter().find(|d| d.id == disk.id)
                && let Some(v) = found
                    .volumes
                    .iter()
                    .find(|v| v.filesystem == crate::disk::Filesystem::Fat)
            {
                if let Some(mount) = &v.mount {
                    return Ok(mount.clone());
                }
                macos::mount(&v.id)?;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        Err(format!(
            "{} was erased but its new volume never appeared mounted",
            disk.id
        ))
    }

    fn finish(&self, disk: &Disk, _root: &Path) -> Result<String, String> {
        macos::eject(disk)?;
        Ok(format!("Ejected {}; it can be pulled out.", disk.id))
    }
}

#[cfg(target_os = "linux")]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        linux::discover()
    }

    fn can_erase(&self, _disk: &Disk) -> Result<(), String> {
        Ok(())
    }

    fn erase(&self, disk: &Disk) -> Result<PathBuf, String> {
        let partition = linux::erase(disk)?;
        linux::mount(&partition)
    }

    fn finish(&self, _disk: &Disk, root: &Path) -> Result<String, String> {
        linux::unmount(root)?;
        Ok(format!(
            "Unmounted {}; it can be pulled out.",
            root.display()
        ))
    }
}

#[cfg(windows)]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        windows::discover()
    }

    /// Not automated on Windows; see `crate::windows`'s BUGS. The steps by hand are the answer.
    fn can_erase(&self, disk: &Disk) -> Result<(), String> {
        Err(crate::windows::erase_instructions(disk))
    }

    fn erase(&self, disk: &Disk) -> Result<PathBuf, String> {
        Err(crate::windows::erase_instructions(disk))
    }

    fn finish(&self, disk: &Disk, _root: &Path) -> Result<String, String> {
        Ok(format!(
            "Use \"Safely Remove Hardware\" on {} before pulling it out.",
            disk.id
        ))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        Err("this program does not know how to find disks on this operating system".to_owned())
    }

    fn can_erase(&self, _disk: &Disk) -> Result<(), String> {
        Err("erasing is not supported on this operating system".to_owned())
    }

    fn erase(&self, _disk: &Disk) -> Result<PathBuf, String> {
        Err("erasing is not supported on this operating system".to_owned())
    }

    fn finish(&self, _disk: &Disk, _root: &Path) -> Result<String, String> {
        Ok(String::new())
    }
}
