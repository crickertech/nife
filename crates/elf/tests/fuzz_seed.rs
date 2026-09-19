//! The committed fuzz seeds must still be valid ELFs.
//!
//! **A seed that no longer parses is not a seed.** `fuzz/fuzz_targets/elf_parse` returns immediately
//! on anything `Elf::parse` rejects, so the moment the seed for this build stops being accepted, the
//! fuzzer is starting from an empty corpus and has to rediscover five constants and a program header
//! table before it reaches a single interesting branch. Nothing about the fuzz run would say so: it
//! would report the same "no crashes" it reports when it is working.
//!
//! That is the silent degradation this test exists to make loud, and it costs a millisecond. The
//! other three targets seed from fixtures that already have tests of their own
//! (`crates/device_tree_blob/tests/qemu_aarch64_virt.rs`,
//! `crates/globally_unique_identifier_partition_table/tests/real_disks.rs`); these seeds had none,
//! because they have no other reason to exist.
//!
//! **There is one seed per machine nife runs, and that is milestone 288's doing.** There used to be
//! one, `EM_AARCH64`, with the honest note that a `riscv64` build would reject it. An `x86_64` build
//! rejects it too, which nobody wrote down because `x86_64` became a target after that sentence, and
//! on an `x86_64` host both tests below went red and the fuzz corpus really was empty. A single seed
//! cannot be right for three machines; three seeds can, and the two this build refuses cost the
//! fuzzer one rejected input each. They differ from each other at exactly one byte, offset 18.
//!
//! Regenerate them with the script in [fuzz/seeds/README.md](../../../fuzz/seeds/README.md) if a
//! deliberate tightening of `parse` makes them invalid.

use elf::Elf;

/// Every seed under `fuzz/seeds/elf_parse/`. Everything else the fuzz targets start from is a real
/// fixture already committed for another purpose.
///
/// **No `#[cfg(target_arch)]` here on purpose.** Picking the file with a `cfg` chain would put a
/// default arm naming one architecture back into the tree, which is exactly the shape that made the
/// x86 kernel accept `aarch64` binaries in milestone 161. All three are compiled in and the right one
/// is chosen by the machine number in its own header, so the selection cannot disagree with the
/// bytes.
const SEEDS: [&[u8]; 3] = [
    include_bytes!("../../../fuzz/seeds/elf_parse/minimal_rx_aarch64.elf"),
    include_bytes!("../../../fuzz/seeds/elf_parse/minimal_rx_riscv64.elf"),
    include_bytes!("../../../fuzz/seeds/elf_parse/minimal_rx_x86_64.elf"),
];

/// The `e_machine` a seed's header claims.
fn machine_of(seed: &[u8]) -> u16 {
    u16::from_le_bytes([seed[18], seed[19]])
}

/// The seed this build's fuzzer will actually get past `Elf::parse`.
fn seed_for_this_build() -> &'static [u8] {
    SEEDS
        .iter()
        .copied()
        .find(|seed| machine_of(seed) == elf::NATIVE_MACHINE)
        .expect("fuzz/seeds/elf_parse/ must hold a seed for the machine this build accepts")
}

#[test]
fn the_fuzz_seed_is_a_valid_executable() {
    let seed = seed_for_this_build();
    let elf = Elf::parse(seed).expect("the seed must parse, or the fuzzer starts from nothing");

    assert_eq!(elf.entry(), 0x4000_0000);

    let segments: Vec<_> = elf.segments().collect();
    assert_eq!(
        segments.len(),
        1,
        "one PT_LOAD, which is the point of minimal"
    );

    // Read and execute, not write: the seed has to get past the W^X refusal as well as the header
    // checks, or it teaches the fuzzer nothing about the paths after validation.
    let seg = segments[0];
    assert!(seg.is_readable());
    assert!(seg.is_executable());
    assert!(!seg.is_writable());
    assert!(
        seg.vaddr <= elf.entry() && elf.entry() < seg.vaddr + seg.memsz,
        "the entry point must land inside the executable segment"
    );
}

/// **Every machine nife runs has a seed, checked from whichever host is running.**
///
/// The machine number is the one thing about these seeds that is host-dependent, and the previous
/// version of this test stated the fact rather than removing the dependence: it asserted that *the*
/// seed matched `NATIVE_MACHINE`, which is a true and useful thing to assert right up until the
/// seed's machine is not yours, at which point it reports a defect nobody can fix without a second
/// seed (milestone 288).
///
/// Comparing against `elf::KNOWN_MACHINES` instead means the `aarch64` laptop that adds a fourth
/// architecture fails here, rather than the stranger who first tries to fuzz on it. A gate that
/// fires only where nobody is standing is not a gate.
#[test]
fn every_machine_nife_runs_has_a_seed() {
    for machine in elf::KNOWN_MACHINES {
        assert!(
            SEEDS.iter().any(|seed| machine_of(seed) == machine),
            "no seed in fuzz/seeds/elf_parse/ carries e_machine {machine}; \
             see fuzz/seeds/README.md for the generator (milestone 288)"
        );
    }
    assert_eq!(
        SEEDS.len(),
        elf::KNOWN_MACHINES.len(),
        "a seed for a machine no nife build accepts is dead weight in every fuzz run"
    );
}
