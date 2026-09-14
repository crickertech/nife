//! **Building another address space from EL0**, milestone 19b.
//!
//! Holds a memory region (slot 0) and a report line (slot 1). It retypes part of its own memory
//! into an address space, retypes a frame, maps the frame into the space it built, and proves the
//! kernel keeps the rules there too: the same virtual address twice is refused.
//!
//! Nothing can run in the built space (threads are 19c's object). What this witnesses is that a
//! process can *construct* one at all, out of memory it was handed, with the kernel allocating
//! nothing.
//!
//! The verdict is three bits: bit 0 the space was retyped, bit 1 the frame mapped into it, bit 2
//! the double map was refused. `kernel::user::tests` asserts `0b111` and prints the bit meanings
//! on failure.
//!
//! Name: provisional (milestone 291). This was `hello`'s `ADDRESS_SPACE_BUILDER` role, number 19,
//! and the lowercase of the role constant is the name. It is deliberately **not** `builder`, which
//! is already a program in every archive (milestone 20's richer-initrd demo); the qualifier says
//! which thing is built.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, map_into, retype_object, retype_page_frame, send};

const MEMORY_REGION: u64 = 0;
const REPORT: u64 = 1;
const VA: u64 = 0x0040_0000;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    let aspace = retype_object(MEMORY_REGION, abi::objtype::ADDRESS_SPACE);
    let mut verdict = 0u64;
    if aspace >= 0 {
        verdict |= 1; // built a space out of our own pages
        let frame = retype_page_frame(MEMORY_REGION);
        if frame >= 0 {
            let mapped = map_into(aspace as u64, VA, frame as u64, 1);
            if mapped == 0 {
                verdict |= 2; // mapped our frame into the space we built
            }
            let again = map_into(aspace as u64, VA, frame as u64, 1);
            if again < 0 {
                verdict |= 4; // the same va twice was refused: break-before-make holds there too
            }
        }
    }
    send(REPORT, verdict, 0, 0);
    exit()
}

user_mode_runtime::panic_handler!();
