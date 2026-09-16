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
| 19 | A directory capability reaches its subtree and nothing above it | §50 | `filesystem_protocol::attenuate_never_widens`, `a_grandchild_is_bounded_by_the_root`; `kernel::user::dir_capability_tests` | milestone 194 (the proofs) |
| 20 | A memory-unsafe C component faults on an out-of-bounds write and changes nothing outside its grant | §31 | `kernel::user::c_seam_tests::a_c_out_of_bounds_write_faults_and_changes_nothing_outside_its_grant` | **yes, by hand** |
| 21 | A user program cannot read a kernel address, on every ISA | §19 | `kernel::user::tests::a_user_program_cannot_read_a_kernel_address`, `the_hardware_says_el0_cannot_read_the_kernels_memory`, `riscv_virtio_tests::the_page_tables_say_u_mode_cannot_read_the_kernels_memory` | **yes, three, and see below on the ISA** |
| 22 | An ELF cannot ask to be loaded over the kernel, or for a writable executable page | §15 | `kernel::user::tests::an_elf_that_asks_to_be_loaded_over_the_kernel_is_refused`, `..._for_a_writable_executable_page_is_refused` | **yes, two** |
| 23 | The progenitor cannot rebuild after dropping its construction authority | §26 | `kernel::user::authority_tests::init_drops_its_construction_authority_and_cannot_build_again` | **yes, and see below** |
| 24 | Two shells with different roots cannot name each other's files | §50 | `kernel::user::shell_navigation_tests::two_shells_with_different_roots_cannot_name_each_others_files` | **yes, and see below** |
| 25 | A client cannot reach its neighbour's pixels or read the screen | §66 | `kernel::user::compositor_tests::a_client_holds_no_capability_for_its_neighbours_pixels_or_the_screen` | **yes, one of its four** |
| 26 | A client of a rendezvous cannot become its server | §41 | `kernel::user::live_swap_tests::a_client_of_the_stable_rendezvous_cannot_become_its_server` | **no, and see below** |

## Five claims that are stated nowhere, which is what step 1 was for

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

**A confined component's *interrupt target* is not confined, and nothing here has ever asked.**
Added 2026-09-03 by DECISIONS §86's research pass, which went looking for what a userspace NVMe
driver would need and found this underneath every option it was pricing. An MSI or MSI-X message is
a memory write to an architecturally special address, so an IOMMU doing DMA remapping alone does not
confine it: a component that can write a device's MSI-X table can aim an interrupt at a vector it
was never given. Linux refuses to hand a device to an untrusted userspace driver on a machine
without interrupt remapping for this reason, and names its escape hatch `allow_unsafe_interrupts`.

**The absence is the finding, and it is three-deep.** Row 12 and row 13 are about where a device may
*read and write*, and neither covers where it may *interrupt*. No boot this tree runs could exercise
the question even if a claim existed: `scripts/qemu-runner-x86_64.sh` attaches `-device intel-iommu`
with no `intremap=on`, and `scripts/qemu-runner-aarch64.sh` uses `gic-version=2`, which has no ITS.
And no driver touches an MSI-X table (notes/nvme.md's `BUGS`: the NVMe controller is brought up with
`IEN=0` and no MSI-X table is touched), so nothing has ever come near it.

**It is latent rather than false**, and it stays latent exactly as long as every component that can
reach a BAR is the kernel. It goes live the first time a driver leaves the kernel and wants
interrupts instead of polling, which is what §86 decides. Whatever §86 settles has to say who owns
the page holding the MSI-X table; the cheap first move is two runner flags
(`-device intel-iommu,intremap=on` with `kernel-irqchip=split`, and `gic-version=3`) to find out
whether this boot path survives the hardware being present at all. The hazard is the one milestone
202 (every confinement test is a ritual until somebody breaks the confinement) already caught in
§31's headline assertion: green after turning the flags on proves nothing by itself, and the
falsification has to be a driver aiming an interrupt where it was not given one, coming back red.

**The progenitor's bytes are unsigned.**
§14 says so plainly in its own honest caveat and it is not in the table because it is not a
confinement claim; it is the reason the confinement has an unverified component inside it.

## What breaking them found

Twenty-five Kani harnesses now carry a recorded patch that turns them red, up from six.
`script/falsifications --sweep` runs all twenty-five in about 30 seconds and every one goes red.
Three results are worth more than the count.

**Milestone 305 added the kernel half** (2026-09-16), which milestone 202 could not: ten kernel
`#[test_case]`s now carry a record and `--sweep` replays each by booting one architecture. Its own
results are in the section after this one. Read them first if you only read one: the headline is a
confinement test that **stayed green under a patch that broke the thing it claims**, and had been
unable to fail since milestone 41.

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

## What breaking the kernel tests found (milestone 305)

Rows 21 to 26 are six claims and nine tests, and a falsification is per test. **Eight of the nine
now carry a recorded patch; the ninth is row 26 and it cannot carry one.** Five results are worth
more than that count.

### A confinement test that could not fail, on RISC-V, since milestone 41

Row 21's RISC-V twin, `the_page_tables_say_u_mode_cannot_read_the_kernels_memory`, is the one place
this milestone found the thing it was looking for. The patch removes the `U`-bit check from
`mmu::user_can_read` outright, which is as complete a break of that test's stated property as can be
written, and the first sweep reported **SURVIVOR**: the test ran, and passed.

The reason was not a weak defect. `user_can_read` walked through `translate_user`, and
`translate_at` builds its `Mapper` with `Half::Low`, **always**, so the high-half kernel address the
test asks about came back `None` before any leaf was read. The headline assertion, `assert!(
!mmu::user_can_read(kernel_va), "the page tables say U-mode could read the kernel's own memory")`,
answered "no" by refusing to look. It could not fail, and the test's own doc comment calls that walk
"the thing under test" and calls the `U` bit RISC-V's single line of defence.

**The tree had already written the cause down and nothing connected it.**
`is_mapped_in_current_space`, forty lines away in the same file, exists for exactly this case and
says so in its doc comment: *"a user thread reaching for the kernel's memory names a high-half
address ... `translate_user` alone would say 'not mapped' for it and turn the most interesting case
into the wrong answer."* `user_can_read` went on calling `translate_user`.

Milestone 305 fixed it (`translate_in_either_half`) and the patch is recorded against the fixed
function, so the row is evidence now rather than ritual. **Two things follow.** A green confinement
test is consistent with the assertion being unable to fail, which is this note's opening sentence
arriving from a direction nobody had checked. And **the only instrument that could find it was a
falsification**: every gate in this tree was green throughout, because a vacuous assertion is a
passing assertion.

### The §31 assertion-order hazard recurs, in row 24

Milestone 202 found that §31's leading sentence, the witness-page equality, is reached only by an
escape that faults anyway. **Row 24 is the same shape in a different subsystem, and here it is
structural rather than incidental.**

`two_shells_with_different_roots_cannot_name_each_others_files` states its property twice: once as
the per-shell bitmap equalities in `assert_report`, and once as the crossing, `assert_eq!((a &
nb::REACHED_SECRET, b & nb::REACHED_INNER), (0, 0), "a shell named a file in the other shell's
root")`. The second is the sentence the milestone makes and the one a reader would quote. It sits
**below** both per-shell checks, and **it cannot run**: any defect that causes a crossing sets a
forbidden bit in one of the reports, and `assert_report`'s first direction catches that one call
earlier. No patch tried in milestone 305 made the crossing fire, and none can.

The quotable sentence is documentation; the bitmap equalities are the mechanism. **Here that costs
nothing**, because `assert_report`'s messages name the offending or missing bit, so a reader learns
as much as the crossing would have told them. §31's instance cost a 234-second watchdog timeout
reading "livelock". Two instances found the same way promotes it from an anecdote about §31 to a
thing to look for: **in a test that states its property twice, the readable statement is usually the
unreachable one.**

**And row 24's own record is weaker than the row looks**, which the patch says where a reader meets
it. The recorded defect (the caretaker serving the filesystem root instead of its narrowed handle)
turns the test red through `assert_report`'s *second* direction, the vacuity guard: the shell could
no longer reach its own files, so every refusal it reported would have proved nothing. Nothing
crossed. It proves the test is wired to the real root handle; it does not demonstrate a crossing.

### Row 21's "on every ISA" is not evidenced the same way on every ISA

The aarch64 falsifications map the kernel's own memory EL0-readable, one flag at one call site, and
the tests watch the hardware refuse. **That defect cannot be booted on RISC-V.** `crates/paging`'s
Sv39 encoder turns `CAP_USER` into the `U` bit, and S-mode access to a `U` page faults unless
`sstatus.SUM` is set, which this kernel sets only inside a test helper. A kernel that marked its own
`.rodata` user-readable there would not be a kernel userspace can read; it would be a kernel that
cannot read itself, dead before the first test. So the RISC-V evidence is against the software walk
instead, and the row's three "yes"es are not three of the same thing. DECISIONS §19 makes parity a
gate for the **capability**; this is a gap in the **evidence**.

### Row 26 cannot be falsified as written, because a real escape hangs the run

`a_client_of_the_stable_rendezvous_cannot_become_its_server` asserts `attack[1] ==
-NotPermitted`, and the honest defect is the one that breaks the claim: delete the kernel's
`Rights::READ` check on `RECV_CAP`, so a client really can receive on the stable rendezvous. That
patch was written and run on 2026-09-16, and the result was **a 60-second watchdog reading "no
progress ... a lost-wakeup hang"**, with a thread dump and not one word about impersonation.

The reason is structural. `RECV_CAP` is a **blocking** receive. An attacker the kernel fails to
refuse does not come back and report an escape; it takes the message the honest server was waiting
for, or parks on the rendezvous, and the run deadlocks. So the assertion that states the claim is
reachable only when the kernel *does* refuse, and the case it is written about cannot reach it.

**That is milestone 202's wrong-reason red, on a third claim, and a red for the wrong reason is not
evidence.** The row is `unfalsified` on purpose, the same disposition as row 17's, rather than being
filled with the easier defect that *does* fire the assertion: changing which error the refusal
returns makes `attack[1]` wrong while leaving the claim entirely intact, which would put a false
claim in the record whose whole job is saying what is known.

**What would close it** is the same move milestone 202 made for §31: give the attacker a bounded
wait so that "the server never answered me" is reported rather than waited out. That needs a
non-blocking or timed receive, which is the syscall surface, so it is a proposal rather than a fix.
See milestone 305's block.

### One row's falsification proves less than the row looks like it proves

Row 23 is falsified in the kernel rather than in the fixture, by making `CapabilityTable::delete`
not take the capability out of the slot, which is the right place because the claim is the kernel's.
But rows 4 and 5 are `capability::a_deleted_capability_stays_deleted` and
`capability::delete_touches_only_its_slot`, both already `replayable`, and both catch that same
defect. So the honest reading is narrower: it proves row 23's kernel test is wired to the kernel's
own delete, not that row 23's test is the only thing watching it. Rows 21, 24, 25 and 26 have no
such overlap.

## BUGS

- ~~**Six kernel confinement tests in the table are marked "no", and there is no mechanism to
  change that.**~~ **Closed by milestone 305, and the mechanism it needed had existed since
  2026-08-31 without anyone connecting the two.** The sentence this replaces said that automating a
  kernel falsification "needs a way to run one kernel test by name, which does not exist". Milestone
  210 built exactly that (`cargo xtask test --test <substring>`, with the filter baked into the test
  binary by `kernel/build.rs` and read by `kernel/src/testing.rs`'s runner) and this note went on
  saying it did not exist for a fortnight. `script/falsifications` now reads a `Falsification:` block
  above a `#[test_case]` and replays it by booting one architecture; `kernel/falsifications/` is the
  path §134 already spells, and row 20's patch is swept rather than remembered.
- **A kernel row's evidence is re-checked far less often than a harness row's, and on one
  architecture.** A Kani record costs a second, so `script/falsifications --affected-since` re-checks
  it on every pull request that can reach it. A kernel record costs a boot, so it is re-checked only
  by a full `--sweep`, which nobody runs per commit, and only on the architecture its patch names.
  Both limits are in `script/falsifications`' own `BUGS`. Read the "Falsified" column accordingly: a
  **yes** on a kernel row is a machine-replayable fact that nothing replays on a schedule.
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
- **The rows citing kernel tests are still not evidence at the same grade as the rows citing
  harnesses, and the reason changed.** It used to be that nothing could replay a kernel
  falsification at all. Since milestone 305 a machine can, so the gap is narrower and it is now
  about what the replay *proves*: a Kani harness is checked by a solver over every input in its
  bound, and a kernel test is one boot of one machine with one fixture attached, on one
  architecture. A kernel row that skipped for want of a disk or a device page is not evidence at
  all, which is why `--sweep` reports a skipped test as an error rather than as either colour.
