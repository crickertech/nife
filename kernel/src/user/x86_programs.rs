//! **Hand-assembled `x86_64` user programs**, because no compiled one exists yet.
//!
//! Milestone 161, roadmap item 4. Every other architecture in this tree runs real ELF binaries out
//! of an initrd; `x86_64-unknown-none` has none, because `crates/user_rt` has no arms for this ISA
//! and `user/build.rs` cannot compile its C components for it (notes/x86-port.md). So the programs
//! the kernel needs in order to *have* a userspace at all are written here as machine code, exactly
//! as the other two ports' test fixtures still are (`user/supervision_tests.rs`,
//! `user/force_kill_tests.rs`), and for the same reason: a four-instruction program that faults on
//! purpose is not worth a compiler.
//!
//! **This module is not test-only, and that is the difference from those two files.** The boot tour
//! needs a real ring-3 process to show, and a `#[cfg(test)]` fixture cannot be it. One copy lives
//! here and the test modules alias it, so the ABI these programs speak is written down once.
//!
//! # Why the programs are `&[u32]` when x86 instructions are not four bytes
//!
//! `sched`'s child builders take a stub as `&[u32]`, one word per instruction, which is exactly
//! right on two fixed-width ISAs and means nothing here. Changing that type would have made the
//! aarch64 and RISC-V programs byte soup, losing the one-instruction-per-line-with-a-comment shape
//! that makes them readable; so instead each program below is written as a **byte listing** (what an
//! assembler actually produced, verified against `clang -target x86_64-unknown-none` rather than
//! recalled) and packed into little-endian words at compile time. The bytes reach memory in the same
//! order either way, which is the only thing the CPU cares about.
//!
//! # The ABI these speak
//!
//! `rax` carries the syscall number and `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9` the arguments
//! (`arch::x86_64::exceptions::TrapFrame::syscall_nr`), ratified as **DECISIONS §124**. These
//! programs are the first things in the tree to speak it from ring 3 through a scheduler.
//!
//! # BUGS
//!
//! - **Every immediate is a 32-bit one**, so a report word or a syscall number above `u32::MAX`
//!   would be silently truncated. Nothing here is near that, and the `debug_assert`s below say so;
//!   the sixty-four-bit forms are five bytes longer and would buy nothing today.
//! - **Nothing here checks a syscall's return value.** These are fixtures, not programs: what each
//!   one proves is that it reached the kernel and came back, and the kernel's own side of the
//!   transaction is what the caller asserts on.

/// Pack a byte listing into the little-endian words the child builders take. See this module's
/// header for why the type is `u32` at all.
macro_rules! packer {
    ($name:ident, $bytes:literal, $words:literal) => {
        const fn $name(b: [u8; $bytes]) -> [u32; $words] {
            let mut out = [0u32; $words];
            let mut i = 0;
            while i < $words {
                out[i] = u32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
                i += 1;
            }
            out
        }
    };
}

packer!(pack_1, 4, 1);
packer!(pack_2, 8, 2);
packer!(pack_8, 32, 8);

/// `0x90`, the one-byte `nop`. Programs are padded up to a word boundary with it rather than with
/// zero, because `00 00` is `add [rax], al` and would fault rather than fall off the end quietly.
const NOP: u8 = 0x90;

/// **A one-instruction runaway**: `jmp .`, forever.
///
/// It never yields, never syscalls and never touches a rendezvous, so nothing cooperative can end
/// it and the forcible tier (DECISIONS §16) is the only thing that can. The twin of aarch64's
/// `b .` and RISC-V's `j .`.
///
/// ```text
///   eb fe        jmp $-2          (to itself)
/// ```
pub const SPIN: &[u32] = &pack_1([0xEB, 0xFE, NOP, NOP]);

/// **A child that faults on its very first memory access**: load from `addr`, which nothing maps.
///
/// The faulting instruction is the second, so the reported pc is the entry plus five (the length of
/// the `mov` that precedes it). That offset is x86's own and differs from the other two ports'
/// four, which is why a test asserting on the faulting pc has to ask this rather than add a
/// constant.
///
/// ```text
///   b8 xx xx xx xx    mov eax, addr    (32-bit form: writing eax zeroes the top half of rax)
///   48 8b 18          mov rbx, [rax]   (page fault: nothing maps addr)
/// ```
pub const fn fault(addr: u32) -> [u32; 2] {
    let a = addr.to_le_bytes();
    pack_2([0xB8, a[0], a[1], a[2], a[3], 0x48, 0x8B, 0x18])
}

/// How far past the entry the faulting instruction in [`fault`] begins. See its doc comment.
pub const FAULT_PC_OFFSET: u64 = 5;

/// **A child that SENDs one word on the endpoint capability in slot 0, then exits cleanly.**
///
/// The x86 twin of the nine-instruction `REPORT_STUB` both other ports carry, and it is nine
/// instructions here too. "It ran" is the SEND arriving; "it finished" is the exit.
///
/// ```text
///   31 ff             xor edi, edi      (arg0: slot 0)
///   31 f6             xor esi, esi      (arg1: rendezvous::SEND, which is 0)
///   ba xx xx xx xx    mov edx, word     (arg2: the word to send)
///   45 31 d2          xor r10d, r10d    (arg3: the second message word)
///   45 31 c0          xor r8d, r8d      (arg4: the third)
///   b8 xx xx xx xx    mov eax, SYS_INVOKE
///   0f 05             syscall           (SEND)
///   b8 xx xx xx xx    mov eax, SYS_EXIT
///   0f 05             syscall           (exit)
/// ```
///
/// # Panics
/// At compile time, through the `assert!`s below, if `rendezvous::SEND` is ever given a nonzero
/// value: the `xor esi, esi` above encodes it as a zero and would then be sending under the wrong
/// method with nothing to say so.
pub const fn report(word: u32) -> [u32; 8] {
    const {
        assert!(
            abi::rendezvous::SEND == 0,
            "the `xor esi, esi` in this program encodes SEND as zero"
        );
    }
    let w = word.to_le_bytes();
    let inv = (abi::SYS_INVOKE as u32).to_le_bytes();
    let ext = (abi::SYS_EXIT as u32).to_le_bytes();
    pack_8([
        0x31, 0xFF, // xor edi, edi
        0x31, 0xF6, // xor esi, esi
        0xBA, w[0], w[1], w[2], w[3], // mov edx, word
        0x45, 0x31, 0xD2, // xor r10d, r10d
        0x45, 0x31, 0xC0, // xor r8d, r8d
        0xB8, inv[0], inv[1], inv[2], inv[3], // mov eax, SYS_INVOKE
        0x0F, 0x05, // syscall
        0xB8, ext[0], ext[1], ext[2], ext[3], // mov eax, SYS_EXIT
        0x0F, 0x05, // syscall
        NOP, NOP, NOP,
    ])
}

/// **A child that blocks in RECV on the endpoint capability in slot 0, forever.**
///
/// A server is a thing that blocks, and DECISIONS §16's kill is armed by a refusal and spent by
/// `schedule()`, which a thread parked in RECV never reaches. That is the whole point of this
/// program: it is the shape of resident whose region used to be unreclaimable.
///
/// ```text
///   31 ff             xor edi, edi      (arg0: slot 0)
///   be xx xx xx xx    mov esi, RECV     (arg1: the method)
///   31 d2             xor edx, edx      (arg2)
///   45 31 d2          xor r10d, r10d    (arg3)
///   45 31 c0          xor r8d, r8d      (arg4)
///   b8 xx xx xx xx    mov eax, SYS_INVOKE
///   0f 05             syscall           (RECV: blocks)
///   b8 xx xx xx xx    mov eax, SYS_EXIT
///   0f 05             syscall           (exit, if it is ever woken)
/// ```
pub const fn recv() -> [u32; 8] {
    invoke_then_exit(abi::rendezvous::RECV as u32)
}

/// **A child that blocks in `CALL` on the endpoint capability in slot 0, and is never replied to.**
///
/// Milestone 133's second shape, and the harder one. A `CALL` caller whose request a server has
/// collected sits on **no queue at all**: it left the sender queue at the rendezvous and the one
/// thing that can wake it is the one-shot `Reply` capability the server now holds. So it is the
/// resident that no endpoint sweep can reach, and the one whose teardown has to sweep that `Reply`
/// out of the server's capability table or leave a forgeable answer behind.
///
/// Byte for byte [`recv`] with a different method word; see it for the encoding.
pub const fn call() -> [u32; 8] {
    invoke_then_exit(abi::rendezvous::CALL as u32)
}

/// The body [`recv`] and [`call`] share: `invoke(slot 0, method, 0, 0, 0)`, then `SYS_EXIT` if the
/// invocation ever returns. Written once because the two differ in exactly one immediate, and two
/// copies of an eight-instruction hand-assembled program are two chances to typo a syscall number.
///
/// ```text
///   31 ff             xor edi, edi      (arg0: slot 0)
///   be xx xx xx xx    mov esi, method   (arg1)
///   31 d2             xor edx, edx      (arg2)
///   45 31 d2          xor r10d, r10d    (arg3)
///   45 31 c0          xor r8d, r8d      (arg4)
///   b8 xx xx xx xx    mov eax, SYS_INVOKE
///   0f 05             syscall           (blocks)
///   b8 xx xx xx xx    mov eax, SYS_EXIT
///   0f 05             syscall           (exit, if it is ever woken)
/// ```
const fn invoke_then_exit(method: u32) -> [u32; 8] {
    let m = method.to_le_bytes();
    let inv = (abi::SYS_INVOKE as u32).to_le_bytes();
    let ext = (abi::SYS_EXIT as u32).to_le_bytes();
    pack_8([
        0x31, 0xFF, // xor edi, edi
        0xBE, m[0], m[1], m[2], m[3], // mov esi, method
        0x31, 0xD2, // xor edx, edx
        0x45, 0x31, 0xD2, // xor r10d, r10d
        0x45, 0x31, 0xC0, // xor r8d, r8d
        0xB8, inv[0], inv[1], inv[2], inv[3], // mov eax, SYS_INVOKE
        0x0F, 0x05, // syscall
        0xB8, ext[0], ext[1], ext[2], ext[3], // mov eax, SYS_EXIT
        0x0F, 0x05, // syscall
        NOP, NOP, NOP,
    ])
}
