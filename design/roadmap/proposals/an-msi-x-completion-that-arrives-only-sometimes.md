# An `x86_64` MSI-X completion is delivered in isolation and sometimes not in the full suite

**Status: PROPOSED 2026-09-04.** Found by the milestone 133 lane, which was gating an unrelated
change and lost one `script/test` leg to it.

**Gate: NONE.** Nothing is owed. It wants a lane because the first question (is the interrupt lost,
or is the counter read before it lands?) needs the QEMU trace rather than a guess, and because a
flake in the gate everything else is judged by is worth somebody's whole attention rather than a
retry.

**In brief.** `user::tests::a_userspace_driver_reads_a_file_over_the_pcie_transport` failed once on
the `x86_64` leg of a full `script/test` on `2026-09-04`, at its second assertion:

```
[PANIC] panicked at kernel/src/user/tests.rs:2061:5:
the read completed but the device's interrupt was never delivered to this kernel
```

The **first** assertion passed, so the driver read the right bytes off the virtio-pci disk and
reported them. What did not happen is `ROUTED_IRQS` rising. Re-run alone on the same tree, twice,
it passes. Find out whether the interrupt is lost or merely late, and make the test say which.

## Why it is worth a lane rather than a retry

A test that passes alone and fails in company is either a real ordering bug or a test that reads a
global at the wrong moment, and **the two have opposite fixes**. This tree has learned that
distinction expensively more than once: `user::tests::reclaim_frees_a_started_then_exited_childs_regions`
was waiting on a whole-table headcount that the previous test's teardown was still moving, and
`user::force_kill_tests::an_address_space_never_frees_a_region_it_was_lent` opens by waiting for the
free-frame count to stop moving for exactly that reason. `ROUTED_IRQS` is a process-wide counter
read straight after a blocking `ipc_recv`, and the completion the driver waited on is what unblocked
it, so a completion delivered by a *different* path (a polled queue, a coalesced MSI-X message) would
produce this shape without anything being broken.

Against that, the honest alternative is that an MSI-X message really was dropped, which on a
`q35` machine with the IOMMU in the picture is a fact worth having. The same leg prints
`vtd_iommu_translate: detected translation failure` on every run, which is expected and documented
elsewhere, but it means this leg is the one where interrupt remapping is doing the most work.

## What doing it looks like

- Establish which of the two it is: bracket the counter read in a bounded wait (`super::wait_for`, the
  in-tree idiom) and see whether the flake becomes a pass. If it does, it was a read-too-early and the
  wait is the fix, with a comment saying what the counter is racing.
- If it does not, the interrupt is genuinely lost and the question moves to the arch layer: MSI-X
  masking around the driver's bind, or a message that lands while the vector is masked.
- Either way the test should end up asserting the property it means (*this device's completion
  reached this kernel*) rather than a monotonic global that anything on the machine can move.

**Do not fix it by deleting the assertion.** It is the half of the test that proves the interrupt
path at all; the first assertion alone would pass on a polled driver.
