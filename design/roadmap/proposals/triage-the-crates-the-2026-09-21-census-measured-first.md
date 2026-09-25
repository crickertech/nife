# Triage the crates the 2026-09-21 mutation census measured for the first time

**Status: PROPOSED 2026-09-24.** Raised by the lane that condensed `notes/mutation-testing.md`
(#1209), while recording the 2026-09-21 census, whose run no one had captured.

**Gate: NONE.** The census is recorded in `notes/project-metrics/mutation-census.csv`, and the work
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

## What to check first

Re-derive each crate with `script/mutation -p <crate>`; do not trust the census rows. Before
triaging, rule out the instrument. Three crates have read low because of it: `uefi_loader`'s unbuilt
`[[bin]]` modules (#1232), `documentation`'s feature-gated tests, and `timetable`'s proof file. Ask
whether any of `stick_maker`'s or `portable_executable`'s files are compiled only behind a feature or
a target, with `cargo mutants --list -p <crate>` and the crate's `Cargo.toml`.

## Done when

The four crates' survivors are each triaged, the appendix exists, and the main page's appendix
table links it.
