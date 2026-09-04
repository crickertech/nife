//! The two spellings of "what extensions does this hart have", and the corners of the older one.
//!
//! `riscv,isa-extensions` is a stringlist and needs no cleverness. The deprecated `riscv,isa` is a
//! small grammar with one genuine trap in it, and the trap is on the board this milestone exists
//! for: `rv64gc` and `rv64imafdc_zicsr_zifencei` name the same machine.

use machine_discovery::riscv64::*;

#[test]
fn extension_lists_are_nul_separated() {
    let value = b"i\0m\0a\0c\0zicsr\0zifencei\0";
    let e = parse_extension_list(value);

    assert!(e.contains(I.union(M).union(A).union(C).union(ZICSR).union(ZIFENCEI)));
    assert!(!e.contains(F));
}

/// Names outside [`TABLE`] are ignored rather than being an error. QEMU's default `rv64` declares
/// about thirty of them; a kernel that failed on an extension it had never heard of would be a
/// kernel that stops booting every time the ecosystem ratifies something.
#[test]
fn unknown_extension_names_are_ignored() {
    let e = parse_extension_list(b"i\0m\0a\0c\0zicond\0smaia\0ssaia\0xtheadba\0");

    assert!(e.contains(I.union(M).union(A).union(C)));
    assert_eq!(e, parse_extension_list(b"i\0m\0a\0c\0"));
}

/// **`g` is an abbreviation, not an extension.** A tree that says `rv64gc` is saying `imafdc` plus
/// `zicsr` and `zifencei`, and a parser that looks `g` up in a table finds nothing and silently
/// reports a machine with no multiplier.
#[test]
fn g_expands() {
    let g = parse_isa_string(b"rv64gc");
    let spelled_out = parse_isa_string(b"rv64imafdc_zicsr_zifencei");

    assert_eq!(g, spelled_out);
    assert!(g.contains(M), "the failure this test exists for");
    assert!(g.contains(D));
    assert!(g.contains(ZIFENCEI));
}

/// The shape a pre-2019 vendor tree has: single letters, no separators, nothing after them. This is
/// the string the VisionFive 2's own firmware is expected to carry.
#[test]
fn a_bare_letter_run_parses() {
    let e = parse_isa_string(b"rv64imafdc");

    assert!(e.contains(I.union(M).union(A).union(C).union(F).union(D)));
    assert!(
        !e.contains(ZICSR),
        "and it says nothing about zicsr, which is why zicsr is not required"
    );
    assert!(REQUIRED.contains(M.union(A).union(C)));
    assert!(
        !REQUIRED.contains(ZICSR),
        "see the module BUGS in riscv64.rs"
    );
}

#[test]
fn the_base_prefix_is_decoded_and_skipped() {
    assert_eq!(Base::from_isa_string(b"rv64imafdc"), Base::Rv64);
    assert_eq!(Base::from_isa_string(b"rv32imac"), Base::Rv32);
    assert_eq!(Base::from_isa_string(b"rv128i"), Base::Rv128);
    assert_eq!(Base::from_isa_string(b"nonsense"), Base::Unknown);

    // The `c` in `rv32imac` is the compressed extension, and the `3`/`2` of the prefix must not be
    // mistaken for anything. Nothing in the letter run comes from the prefix.
    assert_eq!(parse_isa_string(b"rv32imac"), parse_isa_string(b"rv64imac"));
}

/// **The privilege letters, both generations of spelling.** The vendor VisionFive 2 tree, read at
/// the bench 2026-08-14, is the old generation: cpu@0 says `rv64imacu` (user mode spelled,
/// supervisor omitted, the one truthful property on a node whose `status` and `mmu-type` both
/// lie), and cpu@1 says `rv64imafdcbsux`, where `bsux` is four single letters and `s` is the one
/// that matters.
#[test]
fn the_vendor_s7_disclaims_supervisor_mode_and_its_u74_claims_it() {
    assert_eq!(supervisor_mode_claim(b"rv64imacu"), Some(false));
    assert_eq!(supervisor_mode_claim(b"rv64imafdcbsux"), Some(true));
    // The spelling QEMU used before 5.1, still the clearest form of the claim.
    assert_eq!(supervisor_mode_claim(b"rv64imafdcsu"), Some(true));
}

/// **A missing `s` alone proves nothing.** Modern strings spell no privilege letters at all
/// (Linux rejects them), so QEMU `virt` today and the mainline JH7110 dtsi both say nothing about
/// supervisor mode on machines that certainly have it. A rule that read absence as denial would
/// refuse every hart of every modern machine, boot hart included.
#[test]
fn modern_strings_are_silent_about_privilege_modes() {
    assert_eq!(supervisor_mode_claim(b"rv64imac_zba_zbb"), None); // mainline JH7110 S7
    assert_eq!(supervisor_mode_claim(b"rv64imafdc_zba_zbb"), None); // mainline JH7110 U74
    assert_eq!(supervisor_mode_claim(b"rv64imafdch_zicsr_zifencei"), None); // QEMU virt's shape
}

/// **`_s`-prefixed multi-letter extensions can never read as a bare `s`.** QEMU's real string
/// carries `sstc`, `svadu` and `sdtrig` in the multi-letter run; only the run before the first
/// `_` is scanned. Same for a hypothetical `u`-leading multi-letter name.
#[test]
fn multi_letter_extensions_cannot_fake_a_privilege_letter() {
    assert_eq!(supervisor_mode_claim(b"rv64imac_sstc_svadu_sdtrig"), None);
    assert_eq!(supervisor_mode_claim(b"rv64imac_smstateen_ssaia"), None);
    // NUL-terminated, as a real property value arrives.
    assert_eq!(supervisor_mode_claim(b"rv64imacu_sstc\0"), Some(false));
}

/// **What the boot line prints is the device tree's own spelling, and it parses back.**
///
/// `Base::name`'s doc says it is "deliberately spelled as the device tree spells it", and nothing
/// checked that. It matters because the whole deliverable of this milestone is a line a reader
/// trusts: someone who sees `rv64i` on the console and greps a device tree for it should find the
/// same string, and someone who sees `sv39` should be able to write it into a `mmu-type`.
///
/// A round trip is the honest way to check a name, because it holds the printer and the parser to
/// each other rather than to a copy of the answer written into a test.
#[test]
fn every_printed_name_parses_back_to_itself() {
    for base in [Base::Rv32, Base::Rv64, Base::Rv128] {
        assert_eq!(
            Base::from_isa_string(base.name().as_bytes()),
            base,
            "{} does not parse back",
            base.name()
        );
    }
    // The exception, and it has to be one: there is no string a device tree could carry that means
    // "this tree did not say", so `Unknown` prints an English phrase rather than a spelling.
    assert_eq!(Base::Unknown.name(), "unknown");
    assert_eq!(Base::from_isa_string(b"unknown"), Base::Unknown);

    for mmu in [
        MmuType::None,
        MmuType::Sv32,
        MmuType::Sv39,
        MmuType::Sv48,
        MmuType::Sv57,
    ] {
        assert_eq!(
            MmuType::from_property(mmu.name().as_bytes()),
            mmu,
            "{} does not parse back",
            mmu.name()
        );
    }
    assert_eq!(MmuType::Unknown.name(), "not declared");
    assert_eq!(MmuType::from_property(b"not declared"), MmuType::Unknown);
}

/// **The base ordering exists for one comparison, and this is it.**
///
/// `Isa::from_device_tree` keeps the *narrowest* base any hart declares, because a machine whose
/// harts disagree can only run code every hart can execute. The ordering is by address width, which
/// is not the order the variants happen to be declared in, so the `Ord` impl is hand-written and
/// therefore capable of being wrong.
///
/// `Unknown` at the bottom is the load-bearing part: it is what makes "the tree did not say"
/// lose to any hart that did say, instead of winning and reporting an unknown machine.
#[test]
fn bases_order_by_address_width() {
    assert!(Base::Unknown < Base::Rv32);
    assert!(Base::Rv32 < Base::Rv64);
    assert!(Base::Rv64 < Base::Rv128);
    assert_eq!(Base::Rv64.cmp(&Base::Rv64), core::cmp::Ordering::Equal);
    assert_eq!(
        [Base::Rv128, Base::Rv32, Base::Rv64].iter().min(),
        Some(&Base::Rv32),
        "narrowest wins, which is what the hart loop asks for"
    );
}

#[test]
fn mmu_type_tolerates_a_missing_vendor_prefix() {
    assert_eq!(MmuType::from_property(b"riscv,sv39\0"), MmuType::Sv39);
    assert_eq!(MmuType::from_property(b"sv48\0"), MmuType::Sv48);
    assert_eq!(MmuType::from_property(b"riscv,none\0"), MmuType::None);
    assert_eq!(MmuType::from_property(b"riscv,sv57\0"), MmuType::Sv57);
    assert_eq!(
        MmuType::from_property(b"something-else\0"),
        MmuType::Unknown
    );
}

/// The ordering exists for exactly one comparison, so state what it has to mean: an undeclared MMU
/// must not read as "wide enough", and `none` must not read as "wider than sv32".
#[test]
fn mmu_types_order_by_how_much_address_space_they_reach() {
    assert!(MmuType::Unknown < MmuType::Sv39);
    assert!(MmuType::None < MmuType::Sv32);
    assert!(MmuType::Sv39 < MmuType::Sv48);
    assert!(MmuType::Sv48 < MmuType::Sv57);
}

/// Every row of the table is reachable by the name a device tree would spell, in both properties.
/// A row whose name is misspelled is a fact the kernel would report as absent forever.
#[test]
fn every_row_round_trips_through_both_properties() {
    for row in &TABLE {
        let mut listed = row.name.as_bytes().to_vec();
        listed.push(0);
        assert_eq!(
            parse_extension_list(&listed),
            row.bit,
            "row {:?} is unreachable from riscv,isa-extensions",
            row.name
        );

        // The legacy string spells single letters in the run and multi-letter names after a `_`.
        let string = if row.name.len() == 1 {
            format!("rv64{}", row.name)
        } else {
            format!("rv64i_{}", row.name)
        };
        assert!(
            parse_isa_string(string.as_bytes()).contains(row.bit),
            "row {:?} is unreachable from riscv,isa",
            row.name
        );
    }
}

/// **The extension ids, against the specification, written out in hex once.**
///
/// The crate derives these from their tags so that no hand-written hex can be wrong; this is the
/// one place the hex appears, so a reader with the SBI specification open can check all five
/// without reading a `const fn`. Three of them are also spelled at the kernel's `ecall` sites,
/// which now take them from here.
///
/// Two of the five tags are not the extension's human name, which is the whole reason this is
/// worth a test: `IPI` is assigned `sPI` and `RFENCE` is assigned `RFNC`.
#[test]
fn sbi_extension_ids_match_the_specification() {
    assert_eq!(EID_TIME, 0x5449_4D45, "\"TIME\"");
    assert_eq!(EID_IPI, 0x0073_5049, "\"sPI\", not \"IPI\"");
    assert_eq!(EID_RFENCE, 0x5246_4E43, "\"RFNC\", not \"RFENCE\"");
    assert_eq!(EID_HSM, 0x0048_534D, "\"HSM\"");
    assert_eq!(EID_PMU, 0x0050_4D55, "\"PMU\"");
    assert_eq!(EID_BASE, 0x10, "the base extension is a number, not a tag");

    // The same four again through `eid` at **runtime**, which is the only way to exercise it: every
    // caller in the crate is a `const` initializer, so the compiler folds it and nothing ever runs
    // the loop. Const evaluation and runtime evaluation are two implementations of one function in
    // this language, and this is the cheapest place to hold them to each other.
    assert_eq!(eid("TIME"), EID_TIME);
    assert_eq!(eid("sPI"), EID_IPI);
    assert_eq!(eid("RFNC"), EID_RFENCE);
    assert_eq!(eid("HSM"), EID_HSM);
    assert_eq!(eid("PMU"), EID_PMU);

    for row in &SBI_TABLE {
        assert!(row.eid != 0, "{} has no extension id", row.name);
        assert!(!row.why.is_empty(), "{} has no call site", row.name);
    }
}

/// **Every firmware in the registry is reachable by its id, and an unassigned id is not guessed at.**
///
/// The boot line names the firmware, and a wrong name there is a wrong fact in the place a reader
/// trusts most. The shadowing this guards against was possible while the lookup was a `match`: two
/// arms with the same id compile, and the second is unreachable forever with no warning.
#[test]
fn every_sbi_implementation_is_reachable_by_its_id() {
    for row in &IMPLEMENTATIONS {
        let sbi = Sbi {
            impl_id: row.id,
            ..Sbi::default()
        };
        assert_eq!(
            sbi.impl_name(),
            Some(row.name),
            "id {} does not reach {}",
            row.id,
            row.name
        );
    }

    // Past the end of the registry. Reported as a number by the boot line rather than as the last
    // row it happened to be near.
    let unassigned = Sbi {
        impl_id: IMPLEMENTATIONS.len() as u32,
        ..Sbi::default()
    };
    assert_eq!(unassigned.impl_name(), None);
    assert_eq!(
        Sbi {
            impl_id: 9999,
            ..Sbi::default()
        }
        .impl_name(),
        None
    );
}

/// **Accumulating probe answers one at a time builds the same set the requirement check wants.**
///
/// This is what `arch::isa::probe_sbi` does: start empty, `union` in each extension the firmware
/// says it has, then hand the result to `missing_requirements`. Worth its own test because the
/// kernel's version of this loop is the half of SBI discovery no host test can run, so the set
/// arithmetic under it should be proved somewhere that does not need an emulator.
#[test]
fn probing_one_extension_at_a_time_accumulates() {
    let mut acc = SbiExtensions::NONE;
    assert!(acc.is_empty());

    for row in &SBI_TABLE {
        assert!(!acc.contains(row.bit), "not yet probed");
        acc = acc.union(row.bit);
        assert!(acc.contains(row.bit));
    }

    // The accumulator now holds every row, required or not, so it is a strict superset of what the
    // boot refuses to run without. That direction is the assertion worth keeping: a firmware that
    // has everything satisfies the requirement, and a requirement set that grew past the table
    // would be a set the probe loop can never satisfy.
    assert!(acc.contains(SBI_REQUIRED));
    assert!(SBI_REQUIRED.difference(acc).is_empty());

    // And the optional rows are exactly the ones NOT in it. This is the assertion that would have
    // caught the shape this table had before milestone 74, when `SBI_REQUIRED` was every row by
    // construction and adding an instrument to the table would have made the kernel refuse to boot
    // on firmware that merely lacks a performance counter.
    let mut optional = SbiExtensions::NONE;
    for row in &SBI_TABLE {
        if !row.required {
            optional = optional.union(row.bit);
        }
        assert_eq!(
            SBI_REQUIRED.contains(row.bit),
            row.required,
            "{} is in SBI_REQUIRED iff its row says required",
            row.name
        );
    }
    assert!(
        !optional.is_empty(),
        "PMU is optional, so this test is not vacuous"
    );
    assert!(SBI_REQUIRED.difference(optional) == SBI_REQUIRED);
}

/// **`counter_info` decodes the way the specification packs it**, including the `+ 1` on the width
/// that is the field most likely to be read wrong.
///
/// The values here are the shapes real firmware returns. `mcycle` on RV64 is a 64-bit counter read
/// through CSR `0xc00` (`cycle`), which the specification's own layout encodes with a width field
/// of **63**, not 64. A decoder that skipped the `+ 1` would report a counter that wraps at 2^63.
#[test]
fn counter_info_decodes_the_way_the_specification_packs_it() {
    // A hardware counter: CSR 0xc00 (`cycle`), 64 bits wide, type bit clear.
    let cycle = CounterInfo::from_raw((63 << 12) | 0xc00);
    assert!(!cycle.firmware);
    assert_eq!(cycle.csr(), Some(0xc00));
    assert_eq!(
        cycle.bits(),
        Some(64),
        "width field is one LESS than the width"
    );

    // `hpmcounter3`, CSR 0xc03, on a part with 40-bit counters.
    let hpm3 = CounterInfo::from_raw((39 << 12) | 0xc03);
    assert_eq!(hpm3.csr(), Some(0xc03));
    assert_eq!(hpm3.bits(), Some(40));

    // A firmware counter. The specification says `csr` and `width` "should be ignored", so they are
    // not returned at all rather than returned as numbers a caller could use by accident. The raw
    // word here deliberately carries plausible-looking garbage in both fields.
    let fw = CounterInfo::from_raw((1 << 63) | (63 << 12) | 0xc00);
    assert!(fw.firmware);
    assert_eq!(fw.csr(), None);
    assert_eq!(fw.bits(), None);

    // Bit 62 is reserved and must not be mistaken for the type bit.
    let reserved_set = CounterInfo::from_raw((1 << 62) | (63 << 12) | 0xc00);
    assert!(!reserved_set.firmware);
    assert_eq!(reserved_set.csr(), Some(0xc00));

    // A zero word is a legal decode (CSR 0, one bit wide) and not a sentinel. Nothing in the kernel
    // may read it as "no counter"; absence is an SBI error code, not a value.
    let zero = CounterInfo::from_raw(0);
    assert_eq!(zero.csr(), Some(0));
    assert_eq!(zero.bits(), Some(1));
}

/// **The cycle event is type 0, code 1**, assembled from its two fields rather than written as a
/// bare `1`, so the halves can be checked separately against the specification.
#[test]
fn the_cpu_cycles_event_is_type_zero_code_one() {
    assert_eq!(EVENT_HW_CPU_CYCLES, 1);
    assert_eq!(pmu_event(0, 1), EVENT_HW_CPU_CYCLES);

    // The type is the four bits at 19:16, so a firmware event (type 15) is 0xf0000 plus its code.
    // This is what would break if anyone "simplified" `pmu_event` into an addition.
    assert_eq!(pmu_event(15, 0), 0xf_0000);
    assert_eq!(pmu_event(15, 5), 0xf_0005);
    assert_eq!(pmu_event(1, 0xffff), 0x1_ffff);

    // The config flags, against the specification's bit numbers.
    assert_eq!(PMU_CFG_CLEAR_VALUE, 0b10);
    assert_eq!(PMU_CFG_AUTO_START, 0b100);
}

/// **`union` is a set union, not a toggle.** The accumulation test above only ever unions in a bit
/// that is absent, where `|` and `^` agree, so it cannot see the difference. Overlap can: an
/// extension unioned in twice must stay, because a set that forgets TIME when TIME is mentioned
/// again reports working firmware as missing it.
#[test]
fn unioning_an_extension_already_present_keeps_it() {
    let s = SBI_TIME.union(SBI_RFENCE);

    assert_eq!(s.union(SBI_TIME), s);
    assert!(s.union(s).contains(SBI_TIME), "self-union is identity");
    assert!(s.union(s).contains(SBI_RFENCE));
}
