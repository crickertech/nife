//! `std::env`'s variables on nife (milestone 64, rank 4 of the measured gap list).
//!
//! # There is no ambient environment, and that is the design
//!
//! A Unix process inherits `environ` from whoever spawned it, and every program on the machine can
//! read it. This system has no such thing: what a process holds is what it was granted, and an
//! environment variable is not a capability. So a nife process starts with an **empty** environment,
//! seeded only with what its own grants say (see [`seed`], below).
//!
//! What that buys, stated plainly so nobody reads it as an omission: `env::var("HOME")` is `None`
//! because nobody gave this program a home, not because the lookup failed. A crate reading
//! `RUST_LOG` or `NO_COLOR` gets the same `None` it would get on a Unix box where the variable
//! is unset, which is a case every one of them already handles.
//!
//! # Why this exists at all, given that `getenv` could just answer `None`
//!
//! Because [`env()`] could not. Without a backend, nife fell through to `sys::env::unsupported`,
//! whose `env()` is `panic!("not supported on this platform")`. So `std::env::vars()` **aborted the
//! process**, and so did anything built on it: `Command::envs`, a logger dumping its configuration,
//! `dotenvy`, any crate that filters the environment rather than asking for one name. Nothing in a
//! build failure list showed that, which is the measurement's own sting (notes/crates-io-on-nife.md)
//! in a second place: `env::vars()` compiled fine and killed the program.
//!
//! An empty iterator is the truthful answer to "what variables does this process have", and it is
//! the one answer that is never a lie here.
//!
//! # The table is real, because `set_var` is
//!
//! `set_var` and `remove_var` operate on a process-local table, and `var` reads it back. That is not
//! a courtesy: it is what `set_var` means on every platform (it changes *this* process), and a
//! program that sets a variable for a library it is about to call is doing something entirely
//! ordinary that this system has no reason to refuse. Nothing leaves the process, because there is
//! nowhere for it to go: nife has no `execve` carrying an environment, and spawn is by capability.
//!
//! # `TZ`, `LANG` and `TERM` are seeded from a grant, not invented (milestone 47, DECISIONS §111)
//!
//! [`seed`] is called once, from `pal::nife::init`, before `main` runs. It reads the
//! inert-configuration page this process was granted (`rt::CONFIG_SLOT`, `rt::CONFIG_PAGE`), if
//! any, and pushes whatever keys it carries into [`ENV`] before the program's own code has had a
//! chance to touch it. A program granted no such page is seeded with nothing, so
//! `env::var("TZ")` still answers `Err` for a program nobody handed a timezone to, the same
//! honest-absence shape [`env()`] already has for the general case. See `env_proto` for why this
//! needs no seqlock (one writer, and it finishes before the page has a second reader) and for the
//! closed domains `TZ`/`LANG`/`TERM` are validated against before they are ever assembled onto
//! the page.
//!
//! # BUGS
//!
//! - **Only the inert-configuration third is seeded.** `PATH` and `HOME` are **names**
//!   (directory capabilities wearing a string costume, per the roadmap's own framing) and wait on
//!   `bind` (milestone 154's two-directory endowment, not yet built); secrets are answered
//!   elsewhere, by an endpoint (§41), and are never meant to arrive as a string on this table at
//!   all. A program cannot yet be *handed* an arbitrary variable the way it can be handed `TZ`.
//! - **No shipped program declares wanting the config page.** `grant_plan::Manifest` carries no
//!   `config` field the way it carries `clock` for `date`, so the seeding above is real and
//!   tested (`std_exerciser`, milestone 47) but has no shell-facing customer yet. The `caps`
//!   preview extension DECISIONS §111 also asks for (showing a program's inert-config values
//!   before it runs) waits on the same customer.
//! - **The table is per-process and dies with it.** There is no `/etc/environment`, no shell export
//!   that survives, and no way for a parent to hand one down.
//! - **`env::temp_dir` and `env::current_dir` are not this module**, and they are still refused.
//!   Both are namespace questions rather than variable ones (ranks 16 and 18), and they wait on the
//!   same `File::open` resolution fork milestone 47 owns.

use core::sync::atomic::{AtomicU8, Ordering};

use crate::ffi::{OsStr, OsString};
use crate::sync::Mutex;
use crate::sys::pal::nife::envproto::ConfigPage;
use crate::sys::pal::nife::rt;
use crate::{fmt, io, vec};

/// The process's variables. `Vec` rather than `HashMap` on purpose: an environment here holds what
/// the program itself put in it, which is a handful of entries, and a map would drag `RandomState`
/// (and therefore the entropy PAL) into the first `env::var` any program makes.
static ENV: Mutex<Vec<(OsString, OsString)>> = Mutex::new(Vec::new());

/// A snapshot of the process's variables, taken while the lock was held.
///
/// Snapshot rather than a live view, because std's `Env` outlives the borrow and a program is free
/// to `set_var` while iterating. Unix has the same hazard and answers it the same way.
pub struct Env {
    iter: vec::IntoIter<(OsString, OsString)>,
}

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter.as_slice()).finish()
    }
}

impl Iterator for Env {
    type Item = (OsString, OsString);

    fn next(&mut self) -> Option<(OsString, OsString)> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// Every variable this process holds. **Empty until the program sets one**, and never a panic; see
/// the module docs for why that distinction is the whole reason this file exists.
pub fn env() -> Env {
    let snapshot = ENV.lock().unwrap_or_else(|e| e.into_inner()).clone();
    Env { iter: snapshot.into_iter() }
}

pub fn getenv(k: &OsStr) -> Option<OsString> {
    let guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    guard.iter().find(|(name, _)| name == k).map(|(_, value)| value.clone())
}

/// # Safety
///
/// Same contract as every other platform's: the caller must not be racing another thread that is
/// reading the environment. Trivially satisfied today (nife programs are single-threaded, see
/// `sys/thread/nife.rs`) and stated anyway, because the day `thread::spawn` lands is the day a
/// silent assumption here would become a data race.
pub unsafe fn setenv(k: &OsStr, v: &OsStr) -> io::Result<()> {
    let mut guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    match guard.iter_mut().find(|(name, _)| name == k) {
        Some(entry) => entry.1 = v.to_owned(),
        None => guard.push((k.to_owned(), v.to_owned())),
    }
    Ok(())
}

/// # Safety
///
/// See [`setenv`].
pub unsafe fn unsetenv(k: &OsStr) -> io::Result<()> {
    let mut guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    guard.retain(|(name, _)| name != k);
    Ok(())
}

// --- Is an inert-configuration page reachable at all? -----------------------------------------
//
// The same probe `sys/time/nife.rs` runs for the clock, and for the same reason: this has to be
// answerable WITHOUT touching the config page, because a program not granted one has no page
// mapped and a read would fault instead of returning an answer. The probe is an invocation of the
// capability in the slot, with a method number no object defines:
//
//   - no capability in the slot: the kernel answers `NoSuchSlot` (-1).
//   - the config page's Frame capability: the kernel answers `BadMethod` (-5), a refusal from a
//     real object and therefore proof one is there.
//
// Cached, because the answer cannot change: a cspace slot's contents are fixed at spawn on this
// ABI (0 = not yet asked, 1 = granted, 2 = not granted).

static CONFIG_GRANTED: AtomicU8 = AtomicU8::new(0);

/// A method number no object type defines, so the invocation can only ever be refused. Same
/// constant `sys/time/nife.rs` uses for its own probe; not shared between the two files because
/// each PAL module is meant to be readable with no other file open.
const NO_SUCH_METHOD: u64 = 0xffff;

fn config_granted() -> bool {
    match CONFIG_GRANTED.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            // SAFETY: a plain syscall that cannot succeed; the kernel validates the slot.
            let r = unsafe { rt::invoke(rt::CONFIG_SLOT, NO_SUCH_METHOD, 0, 0, 0) };
            let ok = r != -1; // anything but "the slot is empty" means a capability is there
            CONFIG_GRANTED.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            ok
        }
    }
}

/// Seed [`ENV`] from the inert-configuration page this process was granted, if any (milestone
/// 47's environment-variable fork, DECISIONS §111). Called once, from `pal::nife::init`, before
/// `main` runs; see the module docs.
///
/// A process granted no config page calls [`config_granted`] once, gets `false`, and returns
/// having touched nothing: [`ENV`] stays exactly as empty as it was before this milestone
/// existed, which is what makes this addition safe to call unconditionally at startup.
pub fn seed() {
    if !config_granted() {
        return;
    }
    // SAFETY: the loader maps the config page read-only at `rt::CONFIG_PAGE` alongside the
    // capability the probe just found in `rt::CONFIG_SLOT`, and nothing unmaps or writes it
    // (the page has exactly one writer, and it is not this process; see `env_proto`'s docs).
    let page = unsafe { ConfigPage::new(rt::CONFIG_PAGE) };
    let mut guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    for (key, value) in [("TZ", page.tz()), ("LANG", page.lang()), ("TERM", page.term())] {
        if let Some(v) = value {
            guard.push((OsString::from(key), OsString::from(v)));
        }
    }
}
