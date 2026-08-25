//! **One page this process was handed at a fixed virtual address, read and written by volatile
//! access** (milestone 139).
//!
//! Seven programs each hand-rolled the same four to six functions: `r8`/`w8`/`r16`/`w16`/`r32`,
//! sometimes `r16le`/`w16le` or `a_r8`/`a_w8`, every one of them `unsafe { core::ptr::read_volatile
//! ... }` or its `write_volatile` twin, over either a DMA page a driver's wiring maps before
//! `_start` runs or a shared IPC frame the program `PageFrame::MAP`s itself. `entropy.rs`, `kbd.rs`,
//! `net_transport.rs`, `mdns_responder.rs`, `socket_test_client.rs`, `smb_server.rs` and `ntp.rs`
//! all carried a copy; `ntp.rs`'s own comment named the duplication out loud ("the same shape
//! `net_stack` and `socket_test_client` use") without anyone lifting it out.
//!
//! **The invariant every copy asserted by hand was the same one**: the offset passed in is inside
//! the one page the kernel mapped at a known base VA. Copying that assertion N times is the §94
//! shape (DECISIONS' `design/decisions/94-what-may-live-in-a-library.md`), and nothing checked that
//! any of the N were actually kept inside the page: a wrong offset constant was a silent
//! out-of-bounds volatile access, not a caught bug.
//!
//! [`MappedWindow`] holds the one hand-written assertion, at construction, and turns the other end
//! of it (staying inside the page) into a checked `assert!` instead of a repeated comment. A caller
//! that gets an offset wrong now panics at the access instead of reading or writing past the page
//! silently: this is a real reduction in what can go wrong, not a relocation of the same risk.
//!
//! # Examples
//!
//! Bare-metal only, like the rest of this crate (see the crate-level doc for why the examples below
//! are `no_run`).
//!
//! ```no_run
//! use user_rt::mapped_window::{MappedWindow, PAGE};
//!
//! // Constructed once, at the top of the file, the same VA and length every copy hard-coded.
//! const DMA_VA: u64 = 0x0000_0000_0090_0000;
//! // SAFETY: the wiring maps one page read/write at DMA_VA before this program runs.
//! const WINDOW: MappedWindow = unsafe { MappedWindow::new(DMA_VA, PAGE) };
//!
//! // Every access below is safe: bounds-checked against `PAGE`, no unsafe at the call site.
//! WINDOW.w16(0x080, 7);
//! assert_eq!(WINDOW.r16(0x080), 7);
//! ```
//!
//! An offset that would read or write past the window panics instead of touching memory outside
//! it, which is the property no hand-written copy had:
//!
//! ```should_panic
//! # use user_rt::mapped_window::{MappedWindow, PAGE};
//! # const WINDOW: MappedWindow = unsafe { MappedWindow::new(0x0090_0000, PAGE) };
//! WINDOW.r32(PAGE); // one past the last valid u32, deliberately
//! ```

/// The page size every migrated caller was already assuming (each one's own comment said "one
/// page" or "single-page DMA region"; none of them checked it).
pub const PAGE: u64 = 4096;

/// A caller-owned window onto one page this process was handed at a fixed virtual address: a DMA
/// page a driver's wiring mapped before `_start` ran, or a shared IPC frame the program mapped
/// itself with `PageFrame::MAP`. Every read and write is bounds-checked against `len`, so the only
/// trust this type asks for is the one made at construction.
#[derive(Clone, Copy)]
pub struct MappedWindow {
    base: u64,
    len: u64,
}

impl MappedWindow {
    /// # Safety
    /// `base .. base + len` must be a range the kernel has mapped read/write-accessible into this
    /// process for as long as `self` is used. Every caller in this tree gets that from one of two
    /// places: the wiring maps it before `_start` runs (a DMA page), or the program's own
    /// `PageFrame::MAP` succeeded first (a shared IPC frame) and `self` is not read or written before
    /// that call returns. Both are the same trust every `invoke` already extends to the kernel;
    /// this type does not add a new one, it just asserts it once instead of at every access.
    pub const unsafe fn new(base: u64, len: u64) -> Self {
        Self { base, len }
    }

    /// Panics if `off .. off + size` would fall outside the window. The check every hand-written
    /// copy skipped: a wrong offset constant used to be a silent out-of-bounds volatile access,
    /// and is now a panic naming the access that caused it.
    fn check(&self, off: u64, size: u64) {
        assert!(
            off.checked_add(size).is_some_and(|end| end <= self.len),
            "MappedWindow: offset {off:#x}, size {size}, is outside the {}-byte window",
            self.len
        );
    }

    /// Read a plain integer `T` at byte offset `off`, bounds-checked against the window.
    pub fn read<T: Copy>(&self, off: u64) -> T {
        self.check(off, core::mem::size_of::<T>() as u64);
        // SAFETY: `self.base + off .. + size_of::<T>()` is inside the range `new`'s caller
        // promised was mapped, and the bounds check above keeps it inside `self.len`. Volatile:
        // a peer process or a device may write the same memory out from under an ordinary read.
        unsafe { core::ptr::read_volatile((self.base + off) as *const T) }
    }

    /// Write a plain integer `T` at byte offset `off`, bounds-checked against the window.
    pub fn write<T>(&self, off: u64, val: T) {
        self.check(off, core::mem::size_of::<T>() as u64);
        // SAFETY: as `read`'s.
        unsafe { core::ptr::write_volatile((self.base + off) as *mut T, val) }
    }

    /// Named accessors matching the vocabulary every migrated call site already used
    /// (`r8`/`w8`/`r16`/`w16`/`r32`), so migrating a file changes only where these functions are
    /// *defined*, never where they are called. All native-endian, which is little-endian on both
    /// architectures this tree targets; the `le` some copies wrote in their own name documented an
    /// assumption this makes true rather than a different behaviour.
    ///
    /// Read one byte at `off`.
    pub fn r8(&self, off: u64) -> u8 {
        self.read(off)
    }
    /// Write one byte at `off`.
    pub fn w8(&self, off: u64, v: u8) {
        self.write(off, v);
    }
    /// Read a `u16` at `off`.
    pub fn r16(&self, off: u64) -> u16 {
        self.read(off)
    }
    /// Write a `u16` at `off`.
    pub fn w16(&self, off: u64, v: u16) {
        self.write(off, v);
    }
    /// Read a `u32` at `off`.
    pub fn r32(&self, off: u64) -> u32 {
        self.read(off)
    }
    /// Write a `u32` at `off`.
    pub fn w32(&self, off: u64, v: u32) {
        self.write(off, v);
    }
}
