//! **Try to reach the network without declaring it, and print what the kernel answered**
//! (milestone 590 (the booted system starts its network stack)).
//!
//! The negative control for `network_echo_client`. Its manifest
//! (`grant_plan::Prog::UnreachableNetworkWitness`) declares nothing, so the progenitor's spawn
//! service must leave [`grant_plan::NETWORK_SLOT`] empty. It `CALL`s that slot anyway, with a real
//! socket-contract request (`OP_OPEN_TCP`), and prints the outcome:
//!
//! ```text
//! nife> unreachable_network_witness
//! network: refused (no capability at slot 10)
//! ```
//!
//! A progenitor that endowed the network to every child, rather than to the one that declared it,
//! turns that line into `network: REACHED`, and `script/swish-check` fails on it. That is the whole
//! reason this is a program rather than a sentence in a doc: the claim is about what the spawn
//! service does, and only a child it spawned can report what it was given.
//!
//! `unwritable_clock_witness`'s shape, one authority over.
//!
//! # What this program holds
//!
//! - slot 0: its output, the sink contract (`crates/byte_sink_protocol`).
//!
//! Name: provisional. Introduced 2026-09-24 for milestone 590, after `unwritable_clock_witness`.

#![no_std]
#![allow(missing_docs)]
#![no_main]

use socket_protocol::{OP_OPEN_TCP, req};
use user_mode_runtime::{call, exit, send};

/// The output slot: the sink contract.
const OUT: u64 = 0;

// The lines below name the slot in prose; this keeps the prose honest if the constant moves.
const _: () = assert!(grant_plan::NETWORK_SLOT == 10);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    // A kernel refusal comes back in the first word as a negative `abi::Error`; a stack's reply
    // is `REP_OK` or `REP_ERR`, both of which are non-negative. So the sign alone says whether a
    // service answered, and any service answering is the failure this program exists to catch.
    let (r0, _) = call(grant_plan::NETWORK_SLOT, req(OP_OPEN_TCP, 0), 0);
    let line: &[u8] = if r0 as i64 == abi::Error::NoSuchSlot as i64 {
        b"network: refused (no capability at slot 10)\n"
    } else if (r0 as i64) < 0 {
        b"network: refused, but not for want of a capability (slot 10 holds something)\n"
    } else {
        b"network: REACHED. a program that declared no network was handed one\n"
    };
    let mut rest = line;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(OUT, w0, w1, w2);
        rest = &rest[n..];
    }
    send(OUT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

user_mode_runtime::panic_handler!();
