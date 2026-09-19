//! `stick_maker`: write nife's boot files to a USB stick or SD card.
//!
//! ```text
//! stick_maker                      # find the removable disks, ask which, confirm, write
//! stick_maker --list               # what would be offered, and nothing else
//! stick_maker --list --all         # ...and every disk that is not, with the reason
//! stick_maker --disk disk7 --yes   # no questions: copy onto disk7's FAT volume
//! stick_maker --disk disk7 --erase # no questions: erase disk7 if it is not FAT, then copy
//! ```
//!
//! `--include-disk-images` also offers file-backed disks (`hdiutil attach`, Linux loop devices),
//! which is how this program is tested without a real stick. See notes/boot-stick.md.

use std::io::{self, BufRead as _, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;

use stick_maker::disk::{self, Disk, Plan, Policy, human_size};
use stick_maker::embedded::{BUILD, FILES};
use stick_maker::payload::{self, architecture};
use stick_maker::write;

struct Options {
    list: bool,
    all: bool,
    disk: Option<String>,
    yes: bool,
    erase: bool,
    keep_mounted: bool,
    policy: Policy,
}

const USAGE: &str = "\
usage: stick_maker [--list [--all]] [--disk ID] [--yes] [--erase] [--keep-mounted]
                   [--include-disk-images]

Writes nife's boot files to a USB stick or SD card. With no arguments it lists the removable
disks, asks which to use, and asks again before writing anything.

  --list                 show the disks that would be offered, then stop
  --all                  with --list, also show every disk that is not offered, and why
  --disk ID              use this disk (as --list names it) instead of asking
  --yes                  with --disk, copy without asking (never erases)
  --erase                with --disk, erase the disk first if it is not FAT32
  --keep-mounted         do not eject the disk afterwards
  --include-disk-images  also offer file-backed disks, which is how this program is tested
";

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        list: false,
        all: false,
        disk: None,
        yes: false,
        erase: false,
        keep_mounted: false,
        policy: Policy::default(),
    };
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--list" => options.list = true,
            "--all" => options.all = true,
            "--disk" => options.disk = Some(args.next().ok_or("--disk needs a disk name")?),
            "--yes" => options.yes = true,
            "--erase" => options.erase = true,
            "--keep-mounted" => options.keep_mounted = true,
            "--include-disk-images" => options.policy.include_disk_images = true,
            "--help" | "-h" => return Err(String::new()),
            "--version" => {
                println!("stick_maker, nife build {BUILD}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if (options.yes || options.erase) && options.disk.is_none() {
        return Err("--yes and --erase only mean something with --disk".to_owned());
    }
    Ok(options)
}

fn discover() -> Result<Vec<Disk>, String> {
    #[cfg(target_os = "macos")]
    return stick_maker::macos::discover();
    #[cfg(target_os = "linux")]
    return stick_maker::linux::discover();
    #[cfg(windows)]
    return stick_maker::windows::discover();
    #[allow(unreachable_code)]
    Err("this program does not know how to find disks on this operating system".to_owned())
}

fn describe(disk: &Disk) -> String {
    let volumes: Vec<String> = disk
        .volumes
        .iter()
        .map(|v| {
            let fs = match &v.filesystem {
                disk::Filesystem::Fat => "FAT".to_owned(),
                disk::Filesystem::Other(name) => name.clone(),
                disk::Filesystem::Unknown => "unrecognised".to_owned(),
            };
            let label = if v.label.is_empty() {
                String::new()
            } else {
                format!(" \"{}\"", v.label)
            };
            format!("{fs}{label}")
        })
        .collect();
    let volumes = if volumes.is_empty() {
        "no volumes".to_owned()
    } else {
        volumes.join(", ")
    };
    format!(
        "{:<8} {:<28} {:>9}  {:<14} {volumes}",
        disk.id,
        if disk.description.is_empty() {
            "(no name)"
        } else {
            &disk.description
        },
        human_size(disk.size),
        disk.bus
    )
}

fn ask(prompt: &str) -> Result<String, String> {
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

fn replaced(volume: &disk::Volume) -> u64 {
    volume
        .mount
        .as_deref()
        .map_or(0, |root| write::replaced_bytes(root, FILES))
}

/// Find the disk again after erasing it, and return where its FAT volume is mounted.
fn mounted_fat_volume(id: &str) -> Result<PathBuf, String> {
    for _ in 0..20 {
        let disks = discover()?;
        if let Some(disk) = disks.iter().find(|d| d.id == id)
            && let Some(v) = disk
                .volumes
                .iter()
                .find(|v| v.filesystem == disk::Filesystem::Fat)
        {
            if let Some(mount) = &v.mount {
                return Ok(mount.clone());
            }
            #[cfg(target_os = "macos")]
            stick_maker::macos::mount(&v.id)?;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Err(format!(
        "{id} was erased but its new volume never appeared mounted"
    ))
}

fn run(options: Options) -> Result<(), String> {
    println!("nife stick maker, build {BUILD}");
    if FILES.is_empty() {
        return Err(
            "this copy of the program carries no boot files. It was built without them; \
             `cargo xtask stick` builds one that has them."
                .to_owned(),
        );
    }
    let needed = payload::total(FILES);
    for file in FILES {
        println!(
            "  {:<8} {:<24} {}",
            architecture(file.path).unwrap_or("?"),
            file.path,
            human_size(file.bytes.len() as u64)
        );
    }
    println!();

    let disks = discover()?;
    let offered: Vec<&Disk> = disks
        .iter()
        .filter(|d| disk::refusal(d, options.policy).is_none())
        .collect();

    if options.list {
        if offered.is_empty() {
            println!("No removable disk found.");
        }
        for d in &offered {
            println!("  {}", describe(d));
        }
        if options.all {
            for d in &disks {
                if let Some(refusal) = disk::refusal(d, options.policy) {
                    println!(
                        "  {}\n           not offered: {}",
                        describe(d),
                        refusal.reason()
                    );
                }
            }
        }
        return Ok(());
    }

    let chosen: &Disk = match &options.disk {
        Some(id) => {
            let Some(found) = disks.iter().find(|d| &d.id == id) else {
                return Err(format!(
                    "there is no disk called {id}; `--list --all` shows them"
                ));
            };
            if let Some(refusal) = disk::refusal(found, options.policy) {
                return Err(format!("{id} is not offered: it is {}", refusal.reason()));
            }
            found
        }
        None => {
            if offered.is_empty() {
                return Err(
                    "no USB stick or SD card found. Insert one and run this again \
                     (`--list --all` shows every disk and why it was not offered)."
                        .to_owned(),
                );
            }
            println!("Removable disks:");
            for (i, d) in offered.iter().enumerate() {
                println!("  {}) {}", i + 1, describe(d));
            }
            let answer = ask(&format!(
                "Which one? (1-{}, or Enter to stop) ",
                offered.len()
            ))?;
            let Some(d) = answer
                .parse::<usize>()
                .ok()
                .and_then(|n| n.checked_sub(1))
                .and_then(|n| offered.get(n))
            else {
                return Err("nothing chosen; nothing was written".to_owned());
            };
            d
        }
    };

    let root = match disk::plan(chosen, needed, replaced) {
        Plan::Copy { volume } => {
            let v = &chosen.volumes[volume];
            let mount = v
                .mount
                .clone()
                .expect("the plan only copies to a mounted volume");
            println!(
                "Will copy {} of boot files to {} on {} ({}). Nothing on it is erased.",
                human_size(needed),
                mount.display(),
                chosen.id,
                chosen.description
            );
            if !options.yes {
                let answer = ask("Type y to copy: ")?;
                if !answer.eq_ignore_ascii_case("y") && !answer.eq_ignore_ascii_case("yes") {
                    return Err("not confirmed; nothing was written".to_owned());
                }
            }
            mount
        }
        Plan::Erase { because } => {
            println!(
                "{} ({}, {}) has to be ERASED first: {because}.",
                chosen.id,
                chosen.description,
                human_size(chosen.size)
            );
            for v in &chosen.volumes {
                println!(
                    "  everything on {} {} will be lost",
                    v.id,
                    if v.label.is_empty() {
                        String::new()
                    } else {
                        format!("(\"{}\")", v.label)
                    }
                );
            }
            #[cfg(windows)]
            return Err(stick_maker::windows::erase_instructions(chosen));
            #[allow(unreachable_code)]
            if options.disk.is_some() {
                if !options.erase {
                    return Err("pass --erase to allow it; nothing was written".to_owned());
                }
            } else {
                let answer = ask(&format!(
                    "Type the disk's name, {}, to erase it (anything else stops): ",
                    chosen.id
                ))?;
                if answer != chosen.id {
                    return Err("not confirmed; nothing was written".to_owned());
                }
            }
            erase(chosen)?
        }
    };

    let note = payload::note(FILES, BUILD);
    let written = write::write_set(&root, FILES, &note)
        .map_err(|e| format!("writing to {} failed: {e}", root.display()))?;
    for w in &written {
        println!(
            "  wrote {} ({}{})",
            w.path.display(),
            human_size(w.bytes),
            if w.replaced {
                ", replacing the old one"
            } else {
                ""
            }
        );
    }

    finish(chosen, &root, options.keep_mounted)?;
    println!();
    println!("Done. To boot from it:");
    for file in FILES {
        match architecture(file.path) {
            Some("x86_64") => println!(
                "  a PC: turn Secure Boot off in its firmware settings, then choose the stick \
                 from its boot menu (F12 on Dell, F11 or Esc on many others)"
            ),
            Some("aarch64") => println!(
                "  an aarch64 board or VM with UEFI firmware (U-Boot's bootefi, or EDK2): boot the \
                 stick's removable-media entry"
            ),
            Some("riscv64") => println!(
                "  a riscv64 board or VM with UEFI firmware (U-Boot's bootefi, or EDK2): boot the \
                 stick's removable-media entry"
            ),
            _ => {}
        }
    }
    Ok(())
}

fn erase(disk: &Disk) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        stick_maker::macos::erase(disk)?;
        return mounted_fat_volume(&disk.id);
    }
    #[cfg(target_os = "linux")]
    {
        let partition = stick_maker::linux::erase(disk)?;
        return stick_maker::linux::mount(&partition);
    }
    #[allow(unreachable_code)]
    {
        let _ = (disk, mounted_fat_volume);
        Err("erasing is not supported on this operating system".to_owned())
    }
}

fn finish(disk: &Disk, root: &std::path::Path, keep_mounted: bool) -> Result<(), String> {
    if keep_mounted {
        println!("{} is still mounted at {}.", disk.id, root.display());
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let _ = root;
        stick_maker::macos::eject(disk)?;
        println!("Ejected {}; it can be pulled out.", disk.id);
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        stick_maker::linux::unmount(root)?;
        println!("Unmounted {}; it can be pulled out.", root.display());
        return Ok(());
    }
    #[allow(unreachable_code)]
    {
        let _ = root;
        println!(
            "Use \"Safely Remove Hardware\" on {} before pulling it out.",
            disk.id
        );
        Ok(())
    }
}

fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("stick_maker: {message}");
            }
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("stick_maker: {message}");
            ExitCode::FAILURE
        }
    }
}
