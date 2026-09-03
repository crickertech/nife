# What nife claims a confined component cannot do, and which of those claims is tested

Milestone 202, building `design/fatal-risks.md`'s risk 7. The enumeration is the first
deliverable and this note is it. What follows the table is what happened when each claim's test
was broken on purpose.

Risk 7 is *"the confinement claim is false."* The evidence against it is a set of tests this
project wrote about attacks this project chose, and **a passing confinement test is consistent
with two very different worlds**: the component was stopped, or it never reached the address and
the assertion is decorative. Milestone 194 built the mechanism that tells those apart for a Kani
harness (`Falsification:`, `script/falsifications`, a recorded patch that must turn one harness
red). This milestone points it at the security claims.

**Nothing here supports "the confinement holds."** What it supports is narrower and is the
sentence to quote instead: *these named claims are tested, and each test has been shown to fail
when the claim is broken.* It cannot reach a claim nobody made, and that is where real escapes
live.

## The claims

Assembled from DECISIONS §14, §20, §31, §32, `notes/untrusted-input-audit.md`, and the tests
themselves. The last column is this milestone's result.

| # | The claim | Stated in | Tested by | Falsified |
|---|---|---|---|---|
| 1 | A component cannot widen its own rights: a derive holds no more than its source | §14, `crates/capability` | `capability::derive_never_widens_rights` | milestone 194 |
| 2 | Userspace cannot forge a right out of a syscall register | §14 | `capability::from_bits_cannot_forge_a_right` | milestone 194 |
| 3 | A budget that cannot delegate cannot split itself a child that can | §16, `Cap::mint_child` | `capability::split_never_widens_rights` | **yes** |
| 4 | A consumed capability cannot be used again | §12 | `capability::a_deleted_capability_stays_deleted` | **yes** |
| 5 | Dropping one capability does not disturb the others | §12 | `capability::delete_touches_only_its_slot` | **yes** |
| 6 | A supervisor cannot collect a corpse it does not supervise | §32 | `capability::reap_is_permitted_only_to_the_supervising_rendezvous` | **yes** |
| 7 | A refusal about a stranger's thread discloses nothing about it | §32 | `capability::a_stranger_reveals_nothing_about_its_liveness` | **yes** |
| 8 | A process view shows exactly the rendezvous's own children | milestone 126 | `capability::a_survey_shows_exactly_the_endpoints_own_children` | **yes** |
| 9 | What a supervisor may see and what it may reap are one domain | milestone 126 | `capability::the_view_and_the_reap_have_the_same_scope` | **yes** |
| 10 | A user virtual address is in the low half and page-aligned, on every ISA | §19 | `paging::{aarch64,sv39,x86_64}::the_user_va_gate_admits_only_the_aligned_low_half` | **yes, three times** |
| 11 | No page is both writable and executable | §19 | `paging::x86_64::no_encoded_leaf_is_both_writable_and_executable` | **yes** |
| 12 | An IOMMU entry sets no bit the hardware treats as reserved | §20 | `paging::x86_64::no_vtd_entry_ever_sets_a_reserved_bit` | **yes** |
| 13 | A device cannot touch memory outside its driver's granted region | §20 | `dma_validator::in_region_is_sound`, `an_accepted_descriptor_is_confined`, `validate_and_shadow_confines_every_chain` | **yes, three** |
| 14 | A driver cannot send its device to descriptors nothing validated | §20 | `dma_validator::an_accepted_descriptor_is_confined` (the indirect refusal) | **yes** |
| 15 | A driver cannot make the validator walk outside the rings, or forever | §20 | `dma_validator::the_outer_walk_stays_inside_the_rings_and_terminates`, `an_oversized_batch_is_refused` | **yes, two** |
| 16 | One queue's validation cannot touch another queue's rings | §20 | `dma_validator::distinct_queues_occupy_disjoint_blocks` | **yes** |
| 17 | A descriptor changed after validation cannot reach the device | §20 | `dma_validator::a_descriptor_mutated_after_validation_cannot_reach_the_device` | **no, and see below** |
| 18 | A wiring plan never grants a right the declaration did not ask for | §41 | `component_plan::a_plan_never_grants_a_right_the_declaration_did_not_ask_for` | **yes** |
| 19 | A directory capability reaches its subtree and nothing above it | §50 | `filesystem_proto::attenuate_never_widens`, `a_grandchild_is_bounded_by_the_root`; `kernel::user::dir_capability_tests` | milestone 194 (the proofs) |
| 20 | A memory-unsafe C component faults on an out-of-bounds write and changes nothing outside its grant | §31 | `kernel::user::c_seam_tests::a_c_out_of_bounds_write_faults_and_changes_nothing_outside_its_grant` | **yes, by hand** |
| 21 | A user program cannot read a kernel address, on every ISA | §19 | `kernel::user::tests::a_user_program_cannot_read_a_kernel_address`, `the_hardware_says_el0_cannot_read_the_kernels_memory`, `riscv_virtio_tests::the_page_tables_say_u_mode_cannot_read_the_kernels_memory` | **no** |
| 22 | An ELF cannot ask to be loaded over the kernel, or for a writable executable page | §15 | `kernel::user::tests::an_elf_that_asks_to_be_loaded_over_the_kernel_is_refused`, `..._for_a_writable_executable_page_is_refused` | **no** |
| 23 | init cannot rebuild after dropping its construction authority | §26 | `kernel::user::authority_tests::init_drops_its_construction_authority_and_cannot_build_again` | **no** |
| 24 | Two shells with different roots cannot name each other's files | §50 | `kernel::user::shell_navigation_tests::two_shells_with_different_roots_cannot_name_each_others_files` | **no** |
| 25 | A client cannot reach its neighbour's pixels or read the screen | §66 | `kernel::user::compositor_tests::a_client_holds_no_capability_for_its_neighbours_pixels_or_the_screen` | **no** |
| 26 | A client of a rendezvous cannot become its server | §41 | `kernel::user::live_swap_tests::a_client_of_the_stable_rendezvous_cannot_become_its_server` | **no** |

## Four claims that are stated nowhere, which is what step 1 was for

**A confined component's *timing* is not confined.**
Added 2026-09-02 with DECISIONS 139 (how a saturated workload is made to hand threads across
cores is a different section; this is 139, who may read the cycle counter and by what authority).
The words `timing`, `side channel` and `covert` appeared zero times in this note, in
`DECISIONS.md` and in `design/fatal-risks.md` before that decision, so nothing here was falsified
by it; the absence was the finding. seL4 states its own position in one clause, that exporting the
PMU to user level "opens the possibility of timing channels", and this tree intends to publish
cycle-denominated numbers against seL4's while saying nothing.

**The reason it cannot be claimed is measured rather than assumed.** Two threads and a shared word
reconstruct a fine clock with no privileged instruction: 6.8 ns of usable resolution on cordoba
under load, matching Schwarz et al. (FC 2017). That holds on all three architectures, so gating a
cycle counter cannot deliver timing isolation on any of them. What the grant in DECISIONS 139
buys is **accountable authority**: the cheap accurate path is granted rather than ambient, and the
kernel knows which threads hold it. It belongs in this section so nobody reads the capability rows
as covering timing.


**A confined device's *values* are not confined, only its *reach*.**
`notes/untrusted-input-audit.md` says this in its own words ("the IOMMU confines placement, not
values") and no test asserts it, because it is a limit rather than a guarantee. It belongs in
this table as a claim the system does **not** make, so that nobody reads row 13 as covering it.
The audit's finding 1 is the live consequence: the NVMe driver panics on a device-written index
it does not check, which the IOMMU cannot prevent and does not claim to.

**A `SURVEY` cursor counts threads the viewer cannot name.**
`kernel::user::survey_tests::the_survey_cursor_counts_threads_the_viewer_cannot_name` is a test
that *states a limit*, found by a 2026-08-17 audit. Row 8's claim is about which threads are
shown; nothing claims the count is confined, and one is not.

**init's bytes are unsigned.**
§14 says so plainly in its own honest caveat and it is not in the table because it is not a
confinement claim; it is the reason the confinement has an unverified component inside it.

## What breaking them found

Twenty-five Kani harnesses now carry a recorded patch that turns them red, up from six.
`script/falsifications --sweep` runs all twenty-five in about 30 seconds and every one goes red.
Three results are worth more than the count.

### §31's headline sentence is not what catches a broken confinement

Row 20 is the roadmap's own worked example: map `WITNESS_RO` read/write into the C component,
rebuild, run, and the test must go red. It does. **It does not go red on the assertion anybody
would name.**

The obvious answer is the verdict equality, `assert_eq!(v[2], CONFINED, ...)`, which prints all
four bits including `read-only witness intact`. That assertion never runs. A component that is
not confined does not fault; a component that does not fault produces no death report; and
`run_seam` collects every report before the test inspects any of them, so the run stalls at the
collection. The witness check, which is the sentence §31 leads with, is reached only by an
escape that faults anyway.

**And the first run of it failed for the wrong reason**, which is the hazard milestone 202's
block names. `run_seam`'s blocking receive had nothing to take, so the break surfaced as a
watchdog timeout at 234 seconds reading `a livelock, not a lost wakeup`. Right answer, useless
diagnostic: nothing in it says the word confinement. `wait_for_report` (provisional name) now
bounds that wait at 30 seconds against a 90-second budget, and the second run fails at report 4
of 12 with a sentence about what a missing death report means.

### A proof can be blind to the predicate it is stated in, twice

`component_plan::a_plan_never_grants_a_right_the_declaration_did_not_ask_for` asserts
`p.caps()[i].1 == reqs.caps[i].direction.rights()`, which is stated *through* `rights()`, so a
`rights()` that adds `GRANT` to everything satisfies it. Only the explicit
`& abi::rights::GRANT == 0` beside it catches the defect. That is the same shape milestone 194
measured in `capability::derive_never_widens_rights`, one crate over, and it is the argument for
keeping an assertion that looks redundant.

### One claim has no falsification and the reason is structural

Row 17, the time-of-check/time-of-use property, stayed `unfalsified` on purpose. It holds
because the driver's descriptor table and the shadow are two disjoint arrays in the harness's
memory model, and no line of `dma_validator` can make them one. Aiming the copy back at the
driver's table does turn the harness red, but through `ChainMem::write64`'s address arithmetic
rather than through the post-mutation assertion, which is a red for the wrong reason and so is
not recorded as evidence. The harness proves a property of the *design* rather than of code that
could regress, and its honest denominator is that state and not a patch.

## BUGS

- **Six kernel confinement tests in the table are marked "no", and there is no mechanism to
  change that.** `script/falsifications` walks `crates/` and keys on `#[kani::proof]`. Row 20's
  patch was applied, run and reverted by hand, and it lives in `kernel/falsifications/`, a
  **provisional** path nothing sweeps. Automating it needs a way to run one kernel test by name,
  which does not exist: `kernel/src/testing.rs`'s runner takes no filter, `cargo xtask test`
  parses only `--arch`, `--cpu` and `--hvf`, and arguments after `--` reach QEMU rather than the
  kernel. Without that, one falsification costs a whole suite run. See milestone 202's block for
  the proposal.
- **This table is a floor and its own worst failure is invisible.** It cannot list the claim
  nobody made. Every row here was found by reading what this project already wrote, so the
  enumeration inherits exactly the blind spots the tests have. §31's `BUGS` and
  `design/fatal-risks.md` both say the decisive experiment is adversarial and by somebody else;
  this is not that, and calef's position gates outside eyes behind milestone 198.
- **A recorded falsification proves the harness catches *that* defect, not the class.** The
  `the_view_and_the_reap_have_the_same_scope` record carries a prediction that was measured
  false: its defect was claimed to be caught by that harness alone and it also reaches
  `reap_is_permitted_only_to_the_supervising_rendezvous`. Both the prediction and its correction
  are in the patch, which is the point of writing the prediction down.
- **The rows citing kernel tests are not evidence at the same grade as the rows citing
  harnesses.** A Kani row means a machine re-checks the falsification on every sweep. A kernel
  row means a human ran it once, or, for the six marked "no", that nobody has.
