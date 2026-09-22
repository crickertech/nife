1. **File: notes/benchmarks.md**
   **Issue: Inconsistent boot cost calculation**
   The commit states that `script/test --arch x86_64` boots the kernel **four** times, and those four boots took 3, 3, 4 and 3 windows, concluding a **90 ms** additional cost. However, 3+3+4+3 = 13 windows, and at 10ms per window, this would be 130ms, not 90ms. This arithmetic error could mislead readers about the actual performance impact.

2. **File: design/roadmap/526-the-x86-tsc-calibration-takes-the-smallest-of-several-windows.md**
   **Issue: Contradictory calibration window count**
   This file states that the "Sixteen windows would be a 160 ms boot tax" (16 windows × 10ms = 160ms). However, the BUGS section later mentions that "nothing gates the accuracy" and that "checking the answer needs an independent clock, which is `tsc_probe` behind an off-by-default feature costing thirty-five seconds a boot." There's a mismatch between the 160ms claim (16×10ms windows) and the 35-second cost mentioned for `tsc_probe`, which suggests either the window cost is negligible compared to probe cost, or there's confusion about what constitutes the "boot tax" in different contexts.

3. **File: notes/tsc-under-tcg.md (before state)**
   **Issue: Inaccurate description of QEMU's TSC source**
   The original text claims that on ARM64 hosts, TCG falls back to `get_clock()` which returns nanoseconds, making the guest TSC exactly 1GHz. However, this is an oversimplification. QEMU's `cpu_get_host_ticks()` on ARM64 actually returns nanoseconds scaled by a factor related to the host's timer frequency, not necessarily exactly 1GHz. This could mislead readers about the precision of the measurement.