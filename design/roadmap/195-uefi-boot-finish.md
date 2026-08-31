# 195. Finish the UEFI boot path: the suite under firmware, the memory it gives back, and a second core

**Status: NOT-STARTED.** Minted 2026-08-30 from milestone 87's (the x86_64 bare-metal machine) lane,
whose three ordinary handoffs otherwise lived only in a pull request body. *(Number provisional until
the merge queue lands it.)*

**Gate: NONE.** Every increment is downstream of work milestone 87's lane already landed, and none
of them needs a ruling to start; item 3 needs the hardware to be *proved*, not to be built.

**In brief.** The UEFI loader boots the kernel's tour under OVMF and, once calef has run it, on the
OptiPlex. Three things it does not do yet, each small and each already scoped by the lane that found
it.

## The three

1. **Run the kernel suite under firmware, not just the tour.** `uefi-boot` gates the tour; the 200
   x86_64 tests have only ever run under PVH. Embedding the test ELF instead is described as a
   two-line change to `uefi_image`. Until it is done, "it boots under real firmware" and "it passes
   under real firmware" are different claims and only the first is made.
2. **Reclaim boot-services memory.** About 54 MiB of a 256 MiB machine is reported reserved that
   Linux would reclaim, because the loader's four known allocations are not split out of the
   reserved class. Conservative on purpose; the fix is named in the note.
3. **SMP under UEFI**, never exercised. `ap_boot` copies to physical `0x8000`, a page the loader
   never asks firmware for. This is also where milestone 161's two unresolved `ap_boot` bugs and
   `design/fatal-risks.md`'s risk 5 meet, so it is worth more than its size suggests.

## BUGS

- **None of this is provable without the OptiPlex** except under OVMF, and OVMF is not a Dell.
- **Item 3 may not be small.** It is grouped here because it is UEFI-shaped, not because anyone has
  costed it against the `ap_boot` defects it sits next to.
