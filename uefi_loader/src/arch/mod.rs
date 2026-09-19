//! **What differs between architectures**, and only that (AGENTS.md rule 1, applied to the loader).
//!
//! Each module provides the same five things, and `main.rs` calls nothing else architecture-
//! specific:
//!
//! - `ALLOCATION_CEILING`: the highest physical address the kernel can read before its own page
//!   tables are up, under which the archive and every handoff structure are allocated.
//! - `Found` and `discover`: what is read from the firmware before anything is placed.
//! - `hand_over`: exit boot services and enter the kernel in its entry contract.
//! - `park`: halt a core without burning it.

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

#[cfg(target_arch = "riscv64")]
mod riscv64;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;
