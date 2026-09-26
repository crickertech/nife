---
status: PROPOSED
raised: 2026-09-24
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Triage the crates the 2026-09-21 mutation census measured for the first time

Raised by the lane that condensed `notes/mutation-testing.md`
(#1209), while recording the 2026-09-21 census, whose run no one had captured.

The census is recorded in `notes/project-metrics/mutation-census.csv`, and the work
is milestone 326 (nobody has been assigned to turn a mutation score upward)'s method applied to new
crates.

## What is owed

Four crates appear for the first time in the 2026-09-21 census (run 35589550926). Two of them carry
most of its new survivors:

| crate | viable | killed | missed | timeout |
|---|---|---|---|---|
| `stick_maker` | 288 | 64.9% | 101 | 2 |
| `portable_executable` | 277 | 72.6% | 76 | 0 |
| `firmware_configuration` | 32 | 84.4% | 5 | 0 |
| `sealed_pair` | 63 | 95.2% | 3 | 0 |

Every survivor becomes a test, an exclusion carrying its reason, or a recorded gap, per the triage
rule in `notes/mutation-testing.md`. The accounting goes in a new appendix under
`notes/mutation-testing/`, in the shape the 2026-09-20 appendices use.

## Update 2026-09-24: two of the four are done, and the rest of the census joins them

Milestone 326's classification lane (`milestone/326-survivor-classification`) took `stick_maker`
and `portable_executable` and sorted all 771 of the census's missed survivors by kind
(`notes/mutation-testing/census-2026-09-21-triage.md`). `stick_maker`'s answer to the question below
was yes: 61 of its 101 were `src/host/` and `main.rs`, `#[cfg]`-selected per host OS, now excluded.
`portable_executable` went 76 to 5, all equivalent.

**What that classification left untriaged is this proposal's natural scope**, so it is widened here
rather than filed twice. 89 survivors, plus 14 that arrived after the census:

| crate | missed at 2026-09-21 | note |
|---|---|---|
| `documentation` | 47 | a lane of its own; see its appendix's feature-gate history |
| `component_plan` | 11 | new since the baseline, never triaged |
| `ps`, `pgrep`, `pmap` | 6, 6, 5 | the same |
| `firmware_configuration`, `sealed_pair` | 5, 3 | this proposal's original two |
| `loaded_image_check`, `uptime` | 2, 1 | new since the baseline |
| `globally_unique_identifier_partition_table`, `entropy_protocol`, `socket_protocol` | 1 each | above the August triaged count |
| `uefi_loader/src/device_tree_from_acpi.rs` | 14 (current tree) | merged 2026-09-23, after the census: inflow |

## What to check first

Re-derive each crate with `script/mutation -p <crate>`; do not trust the census rows. Before
triaging, rule out the instrument. Three crates have read low because of it: `uefi_loader`'s unbuilt
`[[bin]]` modules (#1232), `documentation`'s feature-gated tests, and `timetable`'s proof file. Ask
whether any of `stick_maker`'s or `portable_executable`'s files are compiled only behind a feature or
a target, with `cargo mutants --list -p <crate>` and the crate's `Cargo.toml`.

## Done when

Every crate in the update's table is triaged, with its accounting in an appendix the main page's
table links.
