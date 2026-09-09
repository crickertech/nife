//! The narrator: the milestone narrative, printed by a program at EL0 (milestone 267).
//!
//! **What this is.** The nine-line story the boot used to tell about itself, lifted out of
//! `kernel_main`. *"milestone 1: we are running our own code on a CPU with nothing underneath
//! it."* It is a demonstration rather than a diagnostic, which is the distinction milestone 267
//! drew: the machine description (paging, the ISA, the cores, the memory map) is how a port is
//! verified on a board with no serial console and prints on every boot, and this is not that.
//!
//! **Why it is a program and not a `println!`.** Two reasons, and the second is the better one.
//!
//! It does not need to be in the kernel. Nothing here reads a kernel counter, spawns a kernel
//! thread, or takes the address of a kernel static; it is text, and text at EL1 is text that costs
//! kernel bytes on every boot including the ones that never print it.
//!
//! And the last claim it makes is about itself. These lines travel through the console server,
//! which is a driver at EL0 holding the UART's registers, sent from a second program at EL0 that
//! holds two endpoints and one shared page and cannot reach the device at all. The sentence *"a
//! userspace program printed to the screen"* used to be printed by the kernel, on the program's
//! behalf, which is the one claim in the tour that was not being demonstrated by the thing making
//! it. Now the program says it.
//!
//! **What it holds.** Slot 0: `SEND` on the console server's request endpoint. Slot 1: `RECV` on
//! its reply endpoint. One page at [`SHARED_VA`], read/write, that the server has mapped read-only.
//! No device, no memory budget, no way to name a second console. The kernel hands it exactly this
//! and nothing else (`kernel/src/user/console_service.rs`, `spawn_client`).
//!
//! Name: provisional (milestone 267's lane, 2026-09-09). Every name in this tree is calef's;
//! this one is shipped so the program can exist and is expected to change. The case for it: it is
//! an agent noun, which is milestone 63's family and what `spinner`, `heeder` and `budgeter`
//! already are, and it names what the program does rather than what it holds, which is the scheme
//! `console` and `input` follow and the one `dwarden` is on record as breaking. The case against:
//! "narrator" says nothing about *what* is narrated, and a reader meeting it in `ls` learns less
//! than they would from `tour`. `tour` was refused because this tree spends the word on the boot
//! path itself (`board_console`'s `Stage::Tour`, `script/board-console`'s reports), and reusing it
//! for one program inside that boot would make the existing term ambiguous.
//!
//! BUGS
//!
//! **You cannot yet type it at the prompt, which is half of what milestone 267 set out to
//! deliver.** It speaks the console server's raw protocol (a shared page plus two rendezvous
//! endpoints, `user/src/hello.rs`'s `printing_client` shape), and `swish` starts a program with an
//! output *sink* (`crates/byte_sink_proto`) instead. So the kernel's tour can run it and the shell
//! cannot. Nothing here is hard to fix and it is a second protocol's worth of work rather than a
//! line: see design/roadmap/267-the-tour-is-three-things-wearing-one-name.md, which proposes it.
//!
//! **It prints a fixed list that nothing checks against the roadmap.** The nine lines name
//! milestones 1 through 11 and were written when those were the newest thing in the tree; there are
//! 267 milestones now and no gate compares this text with `design/roadmap/`. It was equally
//! unchecked as a `println!` in the kernel, so this is a limitation inherited rather than
//! introduced, but it is worth a reader knowing that the story stops early on purpose: it is the
//! argument for the design, not a changelog.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and the count of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_rt::{exit, recv, send};

/// The page this program and the console server share. The server has it read-only and this
/// program has it read/write, so the bytes are written once and never copied. It must match
/// `SHARED_VA` in `kernel/src/user/console_service.rs`, which is where the kernel maps it, and
/// `user/src/console.rs`, which is the other end.
const SHARED_VA: u64 = 0x0000_0000_0060_0000;

/// Slot 0: `SEND` the length of what we just wrote into the shared page.
const REQUEST: u64 = 0;
/// Slot 1: `RECV` the server's ack, which is what says the page is ours to write again.
const REPLY: u64 = 1;

/// The narrative itself.
///
/// One `&str` per line, printed in order. The trailing newlines are part of the text because the
/// console server writes bytes and adds nothing; there is no line discipline underneath this.
const NARRATIVE: &[&str] = &[
    "\n",
    "milestone 1: we are running our own code on a CPU with nothing underneath it.\n",
    "milestone 2: and when it goes wrong, we get told.\n",
    "           : and the machine now tells us what it is, instead of us guessing.\n",
    "milestone 3: and we know which parts of it are ours to give away.\n",
    "milestone 4: and nothing writable is executable, and Vec works again.\n",
    "milestone 5: and the machine can now interrupt us. we are preemptible.\n",
    "milestone 6: and a thread that refuses to yield gets preempted anyway.\n",
    "milestone 7: and now it runs a binary it did not compile, unprivileged.\n",
    "           : and that binary can talk to a server it can only name, not reach.\n",
    "\n",
    "  every line above was printed by a program at EL0, through a console driver that is\n",
    "  also a program at EL0, and the kernel does not contain a line of code that puts a\n",
    "  user's bytes on the wire.\n",
    "\n",
];

#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, _x1: u64, _x2: u64) -> ! {
    for line in NARRATIVE {
        print(line.as_bytes());
    }
    // Exit rather than spin. A user thread that never exits sits on a core for the rest of the
    // boot, which is what `no_leaked_threads` is about and what `hello`'s printing client learned.
    exit();
}

/// Hand `bytes` to the console server: write them into the shared page, send the length, wait for
/// the ack that says the server is done reading.
///
/// The length is the message; the data never crosses the endpoint. A failure is ignored rather than
/// reported, because there is nowhere to report it to: this program's only output is the console it
/// just failed to reach, and a `brk` here would fault the boot over a missing demonstration.
fn print(bytes: &[u8]) {
    let n = bytes.len().min(4096);

    // SAFETY: the shared page is mapped read/write in this address space by the kernel's `Spawn`
    // literal. It is ours between the ack below and the next send, which is what the reply is for.
    let shared = SHARED_VA as *mut u8;
    for (i, &b) in bytes[..n].iter().enumerate() {
        // SAFETY: `i < n <= 4096`, and the page is a full frame mapped writable.
        unsafe { core::ptr::write_volatile(shared.add(i), b) };
    }

    if send(REQUEST, n as u64, 0, 0) < 0 {
        return; // we were not handed a console; there is nothing else to do
    }
    let (_ack, _, _) = recv(REPLY);
}

user_rt::panic_handler!();
