# The GSI-to-vector map wraps onto an exception vector on an IO APIC whose base is not zero

**Status: PROPOSED 2026-09-16.** Found by proof, in milestone 304, the first time a model checker
compiled `kernel/src/arch/x86_64/` at all. Recorded in `kernel/src/arch/x86_64/irq.rs`'s module
`BUGS` as well, which is where a reader meets the feature; this is the fork.

**Gate: DECISION.** It changes a public function's signature and a vector-assignment policy three
doc comments rest on, which AGENTS.md puts on calef's side of the line rather than a lane's.

## In brief

`arch::x86_64::irq::gsi_vector` is `GSI_VECTOR_BASE.wrapping_add(gsi as u8)`, a flat map from a
global system interrupt to the vector it is delivered on. `enable` calls it as
`route_gsi(routing.gsi, gsi_vector(routing.gsi), ...)`, where `routing.gsi` comes from the ACPI
MADT's interrupt source overrides.

`MAX_REDIRECTION_ENTRIES` (144, `MSI_VECTOR_BASE - GSI_VECTOR_BASE`) is documented as the reason
this "cannot silently wrap onto an exception vector or onto an MSI one". **It bounds the wrong
quantity.** It caps the IO APIC's *entry count*, read from the version register. It says nothing
about the GSI, and `redirection_index` admits any `gsi` in `base..base + entries`, where `base` is
`IO_APIC_GSI_BASE`, taken from the MADT and never bounded.

The proof's counterexample is `base = 127`, `entries = 129`, `gsi = 255`: owned by the IO APIC, and
`gsi_vector(255)` = `0x2f`, inside the local APIC's own band. The plausible-hardware case needs
nothing exotic: **a second IO APIC owning global interrupts from 200, with the 24 redirection
entries every real part has, admits GSI 210, and `gsi_vector(210)` wraps to vector 2, the NMI.**

## Why nothing has hit it

This kernel takes the first IO APIC the MADT lists and refuses a GSI outside its range. QEMU's q35,
and every single-socket PC, give that one IO APIC `gsi_base` 0, where the flat map is exact for
every GSI the guard admits. `route_gsi` panics for a GSI this part does not own, so the path where
a bad vector is *used* requires an owned GSI above 143, which needs a nonzero base.

**A nonzero base is legal ACPI and exists on multi-IO-APIC servers.** This is a latent defect on
hardware the project does not have, not a live one, which is precisely the class a bounded model
checker is for and the class no test in this tree would ever have found.

## Options

1. **Route by index.** `GSI_VECTOR_BASE + redirection_index(gsi)` instead of `+ gsi`. Correct for
   every base, and **a no-op on every machine this kernel boots**, because `base == 0` makes the
   index the GSI. The cost is real and is why this is a fork rather than a patch: `gsi_vector` loses
   its `const fn` (it would read two atomics), loses its total signature (it becomes fallible, since
   a GSI this part does not own has no index), and loses the "flat and reversible" property that
   `GSI_VECTOR_BASE`'s doc, `gsi_vector`'s doc, `MAX_REDIRECTION_ENTRIES`' doc and
   `exceptions.rs`'s device-vector arm all describe. Nothing in live code performs the inversion
   today (`exceptions.rs`'s own `BUGS` says the vector-to-intid map is missing and that nothing
   needs it), so the loss is to the documentation and to a future inversion, not to a caller.
2. **Refuse a nonzero base.** `init_io_apic` records the base and `read_madt` could decline an IO
   APIC that does not own global interrupt 0, turning a silent misroute into a refusal at boot with
   a named reason. Cheapest, honest, and it makes the existing flat map's precondition a checked
   fact rather than a comment. It also makes the machine unbootable rather than degraded, on
   hardware nobody here can test the alternative against.
3. **Correct the documentation and leave the code.** `MAX_REDIRECTION_ENTRIES`' claim becomes a
   statement about band disjointness (which is true, and is now proved by
   `no_vector_belongs_to_two_bands`) rather than about wrapping, and the wrap is a recorded
   limitation. This is the state the tree is in after milestone 304, so choosing it costs nothing
   more.

## Recommendation

**1, with 2 as the thing to do first if a decision is not wanted now.** Option 1 is the one that is
still right when someone plugs this kernel into a two-socket machine, and its behaviour on every
machine the project owns is bit-for-bit what happens today, which makes it about as safe a change
as a correctness fix gets. Option 2 is a strictly smaller change that removes the silent failure
without deciding the policy, and the two compose: refusing a nonzero base today does not foreclose
routing by index later.

Asked AGENTS.md's own test, *would we still choose this if both options cost the same*: yes. Option
1 is chosen for correctness on hardware, not because it is small, and it is in fact the larger of
the two.

**What happens if the answer is no:** nothing breaks. The defect stays latent, the `BUGS` entry
stands, and the first machine to hit it takes an NMI with no clue in it, which is the failure mode
worth naming out loud because it will not look like an interrupt-routing bug to whoever meets it.

## Where it came from

Milestone 304, `design/roadmap/304-prover-one-architecture.md`. The harness that found it is
`arch::x86_64::irq::proofs::an_owned_gsi_routes_inside_the_io_apic_band`, and its
`kani::assume(base == 0)` is this proposal standing in the harness until the fork is answered.
