[no text block returned; the model spent its budget on thinking]

Let me carefully review this commit. It's a documentation-only commit (design records, notes) for a capability microkernel written in Rust. The commit:

1. Adds a new roadmap milestone file `design/roadmap/526-the-x86-tsc-calibration-takes-the-smallest-of-several-windows.md`
2. Deletes the proposal `design/roadmap/proposals/the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md`
3. Modifies `notes/benchmarks.md` — adds a correction notice before the 2026-08-24 x86 table and a new dated section at the end
4. Modifies `notes/tsc-under-tcg.md` — marks the finding as fixed

The task says: "A defect may be hiding here that no automated check can see. Look for one specifically. There may be none; say so if you find none."

So I need to look for factual errors, internal inconsistencies, arithmetic errors, contradictions between files, etc.

Let me go through the content carefully.

**The milestone file (526):**

- "Twenty-two boots of one binary stored 1001 MHz to 4330 MHz. This lane widened that to 490 boots at three host loads and the shape held without a single exception: every boot high, none ever low."

- "a 4330 MHz reading is a 43 ms window reported as a 10 ms one, and the worst this lane saw, +1153%, is a 125 ms one."

Let me check: 4330 MHz vs 1000 MHz is +333%. A 43 ms window reported as 10 ms → rate = 43/10 × 1000 = 4300 MHz. OK that checks out (approximately; 4330 would be 43.3 ms).

+1153% means the rate is 1000 × (1 + 11.53) = 12,530 MHz. A 125 ms window reported as 10 ms → 125/10 × 1000 = 12,500 MHz, which is +1150%. Close enough (+1153% would be 125.3 ms). OK.

- The table:

| Windows | Median error | 99th percentile | Worst of 200 | Boots wrong by >1% |
| 1 (what shipped) | +0.36% | +884% | +1153% | 56 / 200 |
| 3 | +0.00% | +120% | +131% | 24 / 200 |
| 5 | +0.00% | +20.6% | +51.4% | 11 / 200 |
| 9 | +0.00% | +0.49% | +7.1% | 2 / 200 |
| 16 | +0.00% | +0.01% | +0.47% | 0 / 200 |

Hmm, wait. Let me check the "Boots wrong by >1%" for 1 window: 56/200 = 28%. Later in the commit, notes/benchmarks.md says "at this host's load roughly one boot in four was wrong by more than a per cent". 56/200 = 28%, which is "roughly one in four". OK.

But wait — there's an inconsistency: "Five still leaves one boot in eighteen wrong by more than a per cent". 11/200 = 5.5%, which is 1 in 18.18. OK, that checks out.

Now the milestone says "Two hundred boots on a host deliberately saturated to load 30 on eight cores, each timing sixteen windows and reporting every one, give the error of the min over the first k of them". So 200 boots at load 30. And "Measured over the same 490 boots" for the stopping rule. And "490 boots at three host loads". OK.

Hmm, but wait: "the worst this lane saw, +1153%" — and the table says worst of 200 at 1 window is +1153%. OK consistent.

- "At ordinary session load one window's worst of 30 boots was +773%, and min-of-9 got to +0.01%; at load 22 the same estimators gave +318% and +0.34%."

Hmm, wait. "min-of-9 got to +0.01%" at ordinary load, but at load 30, min-of-9's worst was +7.1% and min-of-16's 99th percentile was +0.01%. The load-30 table says min-of-16 worst is +0.47%. At ordinary load min-of-9 worst +0.01%? That's plausible for a quieter host. Hmm, but there's a subtle issue: at load 22, min-of-9 gave +0.34%, which is worse than ordinary load's +0.01%. OK, monotonic with load. Fine.

- "So the loop stops as soon as two windows agree to within one part in a thousand" — and "for two windows to land within a thousandth of each other they must both have been left alone, because two independent deschedulings agreeing to three decimal places is not a thing that happens."

Hmm, one part in a thousand is 0.1%, which is three decimal places as a fraction (0.001). OK.

- "Measured over the same 490 boots, this reaches exactly the accuracy of taking the full cap every time, boot for boot, at a mean of: Ordinary session load 3.53 windows / 35 ms; Busy (load 22) 4.77 / 48 ms; Saturated (load 30) 5.20 / 52 ms."

Wait — "reaches exactly the accuracy of taking the full cap every time, boot for boot". Hmm, is that plausible? The stopping rule stops when two windows agree to within 0.1%. If two early windows agree but are both inflated by the same descheduling... well, they argue that can't happen. But "exactly the accuracy, boot for boot" — meaning for each of the 490 boots, the early-stop answer equals the min-of-16 answer? That seems like a strong claim but it's presented as measured. OK.

But hold on: 3.53 windows × 10 ms = 35.3 ms ≈ 35 ms. 4.77 × 10 = 47.7 ≈ 48 ms. 5.20 × 10 = 52 ms. OK.

- "calef's five is what this costs; sixteen is only what it is willing to spend when the host is fighting. That the measured mean on a saturated host came out at 5.20 is a coincidence and is recorded as one."

OK.

- "A tighter tolerance (one part in 2000) moved the mean by 0.07 windows and the error distribution not at all".

OK.

- "script/test --arch x86_64 boots the kernel four times, and those four boots took 3, 3, 4 and 3 windows. So the suite pays 90 ms more than the one-window design did".

Check: 3+3+4+3 = 13 windows = 130 ms. One-window design = 4 windows = 40 ms. Difference = 90 ms. OK.

"against a 3m38s runtime: 0.04%". 3m38s = 218 s = 218,000 ms. 90/218000 = 0.0413%. OK.

- CPUID leaf 0x15/0x16 discussion. "arch::x86_64::isa::tsc_crystal_hz already reads it and init_frequency already prefers it." OK.

- RTC discussion: "Its seconds register has one-second granularity... tsc_probe gets its precision by watching for an edge and then spanning thirty-two seconds". OK, consistent with tsc-under-tcg.md.

- "What is in the diff" — mentions kernel/src/arch/x86_64/timer.rs, kernel/src/main.rs, timer_calibration_tests.rs, tsc_probe.rs, scripts/qemu-runner-x86_64.sh, notes/benchmarks.md.

Wait — the commit diff only shows changes to design/roadmap files and notes files. The milestone file describes changes to kernel source files (timer.rs, main.rs, timer_calibration_tests.rs, tsc_probe.rs, qemu-runner-x86_64.sh). But this commit doesn't include those changes! Hmm. But the task says "Below is the commit as it would appear in a pull request" — the commit message says "Record the calibration fix: milestone 526, the note, and the suspect numbers". So this commit is the *documentation* commit recording a fix that presumably landed in a separate commit. The milestone says "Status: BUILT". Hmm, but the "What is in the diff" section says the diff includes kernel/src/arch/x86_64/timer.rs etc. — but this commit's diff doesn't touch those files.

Actually, this could be a real concern: the milestone document says "What is in the diff" and lists kernel source files, but the commit that adds the milestone contains no kernel changes. However, milestones might be recorded in separate commits from the code. The commit message says "Record the calibration fix: milestone 526, the note, and the suspect numbers" — implying the fix itself was in a prior commit. The "What is in the diff" section in the milestone presumably refers to the whole change (code commit + this doc commit). It's ambiguous but probably not the planted defect.

Let me keep looking for the planted defect. The prompt says "A defect may be hiding here that no automated check can see." Typical planted defects in these exercises: arithmetic errors, internal contradictions, factual contradictions between files, broken references (e.g., referencing a section that doesn't exist), etc.

Let me check the benchmarks.md changes:

The correction block before "The numbers" section (2026-08-24 x86 table):

"every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm, "the 4.2x" — I don't have the full 2026-08-24 section (the file was truncated at 1200 lines in the bundle). The truncation note says "[... truncated at 1200 lines by scripts/review-bundle.sh ...]". So I can't verify everything in benchmarks.md. But the commit diff shows the correction block inserted at line 2664 area, and the new section appended at the end (line 3781+).

The new section at the end:

"## 2026-09-21: the x86 boot calibration was wrong by up to 12x, and what that does to the numbers above"

Hmm — "wrong by up to 12x". +1153% means the rate is 12.53× the truth. So "up to 12x" is roughly right (12.5x). But wait — the correction block says "wrong by up to +1153%". And the title says "up to 12x". +1153% = 12.53x. "12x" is a slight rounding down but acceptable? Hmm, actually +1153% means 12.53 times. Saying "up to 12x" understates slightly but it's a headline rounding. Not clearly a defect.

Wait, actually, let me reconsider. Is it "12x" or "13x"? +1153% error means measured = true × (1 + 11.53) = 12.53 × true. So "up to 12x" — hmm, 12.53 rounds to 13 if rounding to nearest integer, but "up to 12x" reads as "as much as 12 times", which slightly understates. It's loose but not definitively wrong. Hmm.

Let me check the direction-of-error claim carefully, because that's a classic place for a planted defect:

"An inflated rate makes `ns/iter` come out proportionally **too small**, so these figures may read faster than the run really was."

ns/iter = ticks / freq / iters (in nanoseconds: ticks × 1e9 / freq / iters). If freq is inflated (too high), then ns/iter = ticks × 1e9 / (inflated freq) → too small. Yes, an inflated rate makes computed nanoseconds too small, so the benchmark appears faster than reality. Correct.

And: "an inflated rate makes a timeout longer in real time, never shorter". wait_for takes now() + 2 seconds. now() = ticks / freq... hmm, actually deadlines: if the kernel computes a deadline in ticks as now_ticks + 2s × freq, with inflated freq, the deadline in ticks is too far out → timeout takes longer in real time. Correct, consistent with the earlier note.

Now the correction block says: "Nothing can recover the true values: the rate each of these eleven boots stored was not written down, and it was a different number each time."

"eleven boots" — the 2026-08-24 section says "Six boots debug, five release" = 11 boots. OK consistent.

The commit message says "each of those eleven boots stored a different wrong rate and none was written down". OK.

Now the new section:

"- **The 2026-09-15 lazy-TSS section and the icount baselines are not affected.** They quote deterministic ticks under `-icount` and say so; `bench/baseline-*.txt` holds ticks."

Hmm, is there a 2026-09-15 lazy-TSS section in benchmarks.md? The truncated portion might contain it. Can't verify. Also the new section references "the 2026-09-21 section above records that CoreMark 'reports correctness, not yet a score'". Wait — "the 2026-09-21 section above"? The new section IS dated 2026-09-21. Let me re-read:

"- **`coremark`'s self-reported rate on x86** read the stored number, so any x86 CoreMark score computed from it carried the same error. No such score is published; the 2026-09-21 section above records that CoreMark 'reports correctness, not yet a score'."

Hmm, "the 2026-09-21 section above" — this new section is itself headed "## 2026-09-21: the x86 boot calibration was wrong by up to 12x...". If it refers to itself as "the 2026-09-21 section above", that's weird — the CoreMark statement "reports correctness, not yet a score" appears in the "Compute vs. OS primitives" section (milestone 19e), which I can see in the before-content: "This binary reports correctness, not yet a score; timing a run needs a userspace clock..." That's in the 19e section, not a 2026-09-21 section. Hmm, but there might be another 2026-09-21 section in the truncated part of the file (e.g., "The floors name the nightly they were read against (2026-09-21)" and "The emulator is the other half of the same fact (2026-09-21)"). Those are 2026-09-21 subsections. Do they mention CoreMark? The visible ones don't mention "reports correctness, not yet a score". The visible text with that phrase is in the milestone-19e section ("Compute vs. OS primitives"). So the reference "the 2026-09-21 section above" for the CoreMark quote looks wrong — but I can't be 100% sure given truncation. Hmm.

Actually wait, let me re-read: "No such score is published; the 2026-09-21 section above records that CoreMark 'reports correctness, not yet a score'." Since the appended section is at the END of the file, "above" means earlier in the file. The file before the change ends with "...teaches people to route around it." (the exemption paragraph at line 3781). The 19e CoreMark section is dated... The section "### The CoreMark workload" is under "## Compute vs. OS primitives: two benchmarks that measure different things (milestone 19e)". Not dated 2026-09-21.

Hmm, but maybe there's a 2026-09-21 coremark section in the truncated middle. The truncation happened at 1200 lines, and the diff shows the file has at least 3781 lines. So there's a lot I can't see. There could well be a "## 2026-09-21: coremark" section. I can't flag this definitively.

Let me look at other things.

The instrument section:

"**`script/bench --x86` and `script/bench --x86 --real` are different instruments...** Without `--real`, `bench --x86` adds `-icount shift=0,sleep=off`, and under `-icount` a guest nanosecond is a function of the instruction stream rather than of real time: the `tscdrift` lane measured the implied rate moving **37% between two workloads inside one boot**."

Consistent with tsc-under-tcg.md's 37% figure. OK.

"The calibration defect below affects the `--real` instrument only, and within it the `ns` column only. Ticks are raw `rdtsc` counts and are untouched."

OK.

Table in benchmarks.md matches the milestone table. OK.

"Every one of the 490 boots measured across three host loads was **high**, never low" — consistent.

"Which published numbers this reaches":

- "The 2026-08-24 x86 ns/iter table (yield_switch, tss_iomap_switch, debug and release) is the one body of published x86 wall-clock figures derived from the stored rate."

Hmm — the correction block is inserted before "### The numbers" in a section that mentions "per iteration (both threads resume once each), so the delta divided by two is th..." — the visible context line at 2664 is "per iteration (both threads resume once each), so the delta divided by two is th". Then "### The numbers", then the correction, then "QEMU 11.0.2 (`.qemu-version`, pinned), `-machine q35 -cpu max`, one hart, plain TCG. Six boots debug, five release; `ns/iter` computed from the guest's own calibrated TSC...".

The correction says "these eleven boots" — six + five = eleven. OK.

- "The 2026-09-15 lazy-TSS section and the icount baselines are not affected. They quote deterministic ticks under -icount and say so; bench/baseline-*.txt holds ticks."

Hmm, the correction block says the ticks are unaffected and ratios within a boot are unaffected. OK.

- "coremark's self-reported rate on x86 read the stored number, so any x86 CoreMark score computed from it carried the same error. No such score is published; the 2026-09-21 section above records that CoreMark 'reports correctness, not yet a score'."

As discussed, possibly a wrong cross-reference, but can't verify due to truncation.

- "Instant, uptime and the shell's time on x86 all read it through counter_frequency_protocol's page. Not published figures, and not therefore harmless: every process's sense of elapsed time was off by that boot's error."

OK.

- "Nothing went red over any of it, which is why it lasted. kernel::user::wait_for takes now() plus two seconds, so an inflated rate makes a timeout longer in real time, never shorter. The defect could only ever fail safe."

OK, consistent.

"### Before and after, as a controlled pair"

"Two `script/bench --x86 --real` runs of the same tree, differing in one constant (`CALIBRATION_WINDOW_CAP` at 1 and at 16), so the before case is the old estimator exactly:

| Before (one window) | 1003.7 MHz | +0.37% |
| After (min of N) | 1000.1 MHz | +0.01% |"

Check: 1003.7 vs 1000.000 → +0.37%. OK. 1000.1 → +0.01%. OK.

"**That before figure is one draw and a lucky one**... at this host's load roughly one boot in four was wrong by more than a per cent, and the worst seen was +1153%."

56/200 = 28% ≈ one in four. OK.

"**The `ticks` columns of those two runs differ by up to 4x, and that is not the calibration.** It is host load between the two runs..."

OK.

Now tsc-under-tcg.md changes:

"**FIXED, 2026-09-21, the same day**, by milestone 526... The boot now times several windows and keeps the smallest, because the error being one-sided is what makes the minimum the right estimator; it stops as soon as two windows agree to one part in a thousand, so the mean cost is 3.5 windows on a quiet host; and the boot line prints the worst window beside the chosen one..."

Milestone says 3.53 windows at ordinary session load. "3.5" OK.

"**The numbers in this section are the defect, and they got worse when measured harder.** This note recorded 4330 MHz as the worst of twenty-two boots. Milestone 526's sweep ran 490 boots at three host loads and saw **+1153%**, with 56 of 200 boots at load 30 wrong by more than one per cent. The one-sidedness held without a single exception across all 490. After the fix, 0 of those 200 boots are wrong by more than one per cent and the worst is +0.47%."

Consistent with the table.

Now, the commit message: "notes/tsc-under-tcg.md's finding is marked fixed, with the correction that measuring harder made the defect worse: 4330 MHz over twenty-two boots became +1153% over 490."

Check: 4330 MHz is +333%. +1153% is 12,530 MHz. OK.

Now let me look for the *planted* defect more carefully. Common candidates:

1. The milestone says the old `init_frequency` "armed PIT channel 2 for a 10 ms one-shot and polled port 0x61 until the output line went high, reading the TSC either side." Consistent with the proposal's description. OK.

2. The milestone says "Milestone 524 (the three x86_64 boot gates: NX, SYSCALL, and the invariant TSC) had just argued that a non-invariant TSC 'cannot be caught by measuring harder'." OK.

3. Milestone number: 526, "Number provisional until the merge queue lands it; 524 onward is contested between in-flight branches." OK.

4. The BUGS section: "0 of 200 boots at load 30 were wrong by more than 1%; that is a measured distribution and not a guarantee."

5. "The local APIC timer's rate gets the same treatment and has no CPUID escape. It is the minimum over the same windows, taken independently of the TSC's."

Hmm wait — "taken independently of the TSC's"? The proposal asked "Whether the local APIC timer's rate should be derived from the same windows." The milestone's Follow-on says "the local APIC timer is derived from the same windows". But the BUGS entry says "It is the minimum over the same windows, taken independently of the TSC's." Hmm, "the minimum over the same windows, taken independently" — meaning the APIC rate is computed as the min over the same set of windows but with its own min computation. That's consistent-ish. OK.

6. The bug about qemu-bounded.sh: "qemu-runner-x86_64.sh is the only one of the three runners that does not exec QEMU, because it has to translate isa-debug-exit's status afterwards, and it says so. But the wrapper bounds a run by sending SIGTERM to the child it started, which on aarch64 and riscv64 is QEMU and here was the runner shell. The shell died; QEMU was reparented to pid 1 and ran forever."

And: "This lane's calibration sweep orphaned an emulator on every boot until someone looked, and qemu-bounded.sh's own BUGS section named a different cause ('a SIGKILL to the killer defeats all of it') than the one operating. The runner now runs QEMU in the background, traps TERM and HUP, forwards them, and reaps. It also needed qemu-bounded.sh's own fd-3 dance, because backgrounding a command gives it /dev/null on stdin under dash and would have broken every piped-input run in CI."

Hmm, and the Follow-on: "Recorded. scripts/qemu-bounded.sh's BUGS section still describes SIGKILL as the way its bound is defeated, and does not mention that a runner which does not exec defeats it with an ordinary TERM. The x86_64 runner is fixed here and now says so at the code; the wrapper's own BUGS is the place a reader would look first and is left for whoever next touches that file."

Interesting: the tsc-under-tcg.md BUGS section (before) says: "scripts/qemu-bounded.sh's killer did not fire for this lane, repeatedly: a bounded run whose output consumer exited early left qemu-system-x86_64 orphaned to launchd past its bound, five times in one afternoon... This note is not where that gets fixed; see the lane's report and fix/qemu-bounded-stdin-under-dash, which is a different symptom of the same script."

The commit did NOT update that BUGS entry in tsc-under-tcg.md. The milestone says the x86_64 runner is fixed ("The runner now runs QEMU in the background, traps TERM and HUP, forwards them, and reaps"). But tsc-under-tcg.md's BUGS still says the killer "did not fire for this lane" and "This note is not where that gets fixed; see ... fix/qemu-bounded-stdin-under-dash". Hmm — that's a historical record of what happened during that lane, so it's arguably fine to leave. Though "This note is not where that gets fixed" remains true-ish. Not clearly a defect.

7. Now the big one to check: the correction block's claim about which numbers are affected. "every `ns/iter` in this section is suspect, and the `ns` column alone." — The 2026-08-24 section. It says the ticks are unaffected and ratios within a boot are unaffected. Wait — is that right? The ns/iter values are computed from ticks via the calibrated rate. A ratio of two ns/iter values from the same boot cancels the rate. A ratio of two ns/iter values from *different* boots does NOT cancel (different boots had different wrong rates). The correction says "every conclusion here that rests on a ratio between two rows of the same run... the debug-versus-release argument and the 4.2x are ratios within a boot".

Hmm wait — "the debug-versus-release argument" — is that a ratio within a boot? The section says "Six boots debug, five release". Debug and release are different builds, hence different boots! A debug-versus-release ratio compares a number from a debug boot against a number from a release boot — those are NOT the same boot, so the calibration does NOT cancel!

Let me re-read: "every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm. If the debug-versus-release comparison is between six debug boots and five release boots, then it's a cross-boot comparison, and the calibration does not cancel. Unless... the section describes something where debug and release were measured in the same boot? The visible text says "Six boots debug, five release" — that strongly suggests debug and release numbers come from different boots. So claiming the debug-versus-release ratio is "within a boot" where "the calibration cancels exactly" would be wrong.

But wait — I can't see the actual 2026-08-24 section because of truncation. The context line at 2664 says "per iteration (both threads resume once each), so the delta divided by two is th..." and then "### The numbers" with "Six boots debug, five release". Hmm, actually let me reconsider: maybe the section is about `tss_iomap_switch` and `yield_switch` on x86, with debug and release builds. "Six boots debug, five release" — so debug numbers from 6 boots, release from 5 boots. A debug-vs-release ratio would be across boots. Unless each "boot" ran both debug and release? No — a boot is one binary; debug and release are different binaries.

Hmm, but hold on. Maybe the "debug-versus-release argument" compares ratios: e.g., (yield_switch / tss_iomap_switch) in debug vs the same ratio in release. If the argument is "the ratio of ratios", then within each boot the calibration cancels for the inner ratio, and then comparing those dimensionless ratios across boots is fine. That's possible. The commit message says "every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them."

I can't verify without the section. This is risky to flag.

Let me look at the new section's claim again: "every conclusion in that section that rests on a ratio between two rows of one boot survives untouched, because the calibration cancels exactly in a ratio." That's a more careful statement ("that rests on"), whereas the correction block says "which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot". Hmm.

Actually, wait. Let me reconsider the correction block: "The *ticks* they were derived from are unaffected, and so is every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm, "the 4.2x" — what's that? Probably "release is 4.2x faster than debug" or similar. If the 4.2x is debug-vs-release, it's cross-boot. Unless the 2026-08-24 section's numbers were all taken in single boots that ran both? "Six boots debug, five release" — no.

Hmm, hold on. Actually, maybe the debug-vs-release argument is: within the debug boots, the ratio of two benches is X; within release boots, the ratio is similar; therefore the debug-release tax is uniform... I really can't tell. Truncation hides it. I should be careful about flagging something I can't verify. But actually — this is exactly the kind of "defect no automated check can see" that the exercise plants: a subtle statistical/metrological error. Let me keep digging for something more clear-cut.

8. Let me check the "12x" headline again: "## 2026-09-21: the x86 boot calibration was wrong by up to 12x". +1153% → 12.53×. "up to 12x" — hmm, 12.53 rounds to 13, not 12. But "12x" might be intended as "twelve and a half". Eh. It's a headline; "up to 12x" is technically an understatement of 12.53x. Not a clear defect... though it's odd. Actually wait — could it be that the defect was described as "wrong by a factor of 12.5" and "12x" is fine? Meh.

9. Let me check the claim in the milestone: "The tscdrift lane then went to check whether TCG's TSC drifts at all, built an RTC-referenced probe, and found that it does not: the counter ticks at exactly 1000.000 MHz, constant to within 42 ppm whatever the host is doing."

Consistent with tsc-under-tcg.md. OK.

10. "init_frequency armed PIT channel 2 for a 10 ms one-shot and polled port 0x61 until the output line went high" — PIT channel 2's output is readable via port 0x61 bit 5. OK, standard PC architecture.

11. Milestone: "The brief for this lane said not to take five as given... five would have been the wrong number to ship. Two hundred boots on a host deliberately saturated to load 30 on eight cores, each timing sixteen windows and reporting every one, give the error of the min over the first k of them"

So the 200 boots each timed 16 windows, and the table shows min over first k. OK.

But then: "Measured over the same 490 boots, this reaches exactly the accuracy of taking the full cap every time, boot for boot". Wait — "the same 490 boots"? The 490 boots were at three host loads (ordinary, load 22, load 30). The 200-boot table is the load-30 subset. The early-stopping rule was evaluated over all 490. For the early-stop to reach "exactly the accuracy of taking the full cap", every boot's early-stop answer must equal the min-of-16 answer. That's a measured claim. OK.

Hmm wait, actually there's something. "this reaches **exactly** the accuracy of taking the full cap every time, boot for boot, at a mean of: Ordinary session load 3.53 windows..." But at ordinary session load, the earlier text says "min-of-9 got to +0.01%" worst of 30 boots. And min-of-16 presumably also +0.01% or better. Fine.

12. Check the milestone's claim "An average would be a biased estimator for precisely the reason the minimum is an unbiased one". Hmm — the minimum is NOT unbiased! The minimum of several windows is still an upper bound on the true rate (each window is an upper bound, so the min is still ≥ truth). The minimum is a *less biased* estimator, but it is still biased upward. The text itself says "Every window is an *upper bound* on the true rate, so the minimum of several is the tightest bound taken, and it converges on the truth from above." "Converges from above" means it's still biased (consistent perhaps, but biased for finite N). Calling the minimum "an unbiased one" contradicts the very next sentence's "upper bound" logic and the table showing min-of-16 still having +0.01% median... wait, median +0.00%. Hmm, but worst +0.47%. The min is still biased upward in expectation.

Actually the milestone says: "An average would be a biased estimator for precisely the reason the minimum is an unbiased one: it carries every descheduling into the answer instead of discarding it."

This is statistically wrong: the minimum is not unbiased; it's biased toward overestimation (as the text elsewhere correctly states: "converges on the truth from above", "an upper bound"). If every window is an upper bound, the minimum of finitely many windows is still an upper bound (or equal to truth only if one window was perfectly clean, and even a clean window has polling latency ≥ 0, so still ≥ truth). So the minimum is biased (non-negative error). The proposal (deleted) said it more carefully: "the minimum of several windows converges on the truth from above. That is not an argument from taste; it is why averaging would be the wrong estimator here and the minimum is the right one." The proposal avoided "unbiased".

Hmm, is this the planted defect? It's a real conceptual error in the document: calling the minimum "unbiased" while the same section (and table) shows residual positive error (+0.47% worst, +0.01% p99 at 16 windows). The document elsewhere says "The error being one-sided is the whole of the fix. Every window is an *upper bound* on the true rate, so the minimum of several is the tightest bound taken, and it converges on the truth from above." — "converges from above" = biased for every finite sample. So "the minimum is an unbiased one" is internally contradictory with the surrounding text. That's a plausible planted defect: a subtle statistical falsehood in a design document.

Hmm, but is it "a defect that no automated check can see"? Yes, conceptual. Is it the intended one? Maybe. Let me keep looking for others, then decide which to report. The task says "Answer as a numbered list. One concern per item" — so multiple concerns are allowed.

13. Let me check the "0.04%" suite cost claim again: "the suite pays 90 ms more than the one-window design did, against a 3m38s runtime: 0.04%". 90 ms / 218 s = 0.041%. OK.

14. "an order of magnitude below the suite's own run-to-run variance" — plausible, unverifiable.

15. Milestone BUGS: "The cap was chosen against one machine. Every number here is patagonia, an eight-core Apple Silicon host running TCG." Consistent with tsc-under-tcg.md (patagonia, 8 cores, "uptime ~7 on 8 cores"). OK.

16. "Nothing gates the accuracy... checking the answer needs an independent clock, which is tsc_probe behind an off-by-default feature costing thirty-five seconds a boot."

tsc-under-tcg.md says "the question it answers takes thirty-five seconds of wall clock". OK consistent. Also the reproduce section: "qemu-bounded.sh 55" for plain TCG and "75" for icount. The 35 s figure: 32-second windows + boot. OK.

17. Now, "What is in the diff" lists `kernel/src/main.rs`: "the boot line prints `best of N windows, worst M MHz`." OK.

18. The milestone says "Milestone 524 ... had just argued that a non-invariant TSC 'cannot be caught by measuring harder'. The tscdrift lane then went to check whether TCG's TSC drifts at all..." OK.

19. Commit message: "What survives untouched is every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them."

Same claim as the correction block. The debug-vs-release concern again.

Hmm, let me think about the 2026-08-24 x86 table more. The correction is inserted at line ~2664 in a section whose visible context includes "per iteration (both threads resume once each), so the delta divided by two is th" — this looks like a section about TSS iomap switch cost, maybe "the lazy TSS" work. And the table is "QEMU 11.0.2, -machine q35 -cpu max, one hart, plain TCG. Six boots debug, five release".

Wait — "plain TCG"! The 2026-08-24 table was measured under plain TCG, not -icount? "ns/iter computed from the guest's own calibrated TSC, the same arithmetic xtask's run_bench already does for every other leg." Hmm, but script/bench --x86 adds -icount by default. The 2026-08-24 section might predate the -icount default for x86, or used --real. Whatever.

The correction block says "every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot".

Hmm wait, actually — maybe in that section, each boot produced both a debug number and a release number? No — "Six boots debug, five release" clearly indicates separate boots per build.

Actually, hold on. Let me reconsider: maybe the debug-versus-release argument is structured as: "in debug, tss_iomap_switch costs X× yield_switch; in release, it costs Y×; the ratio is preserved" — i.e., the argument rests on within-boot ratios. And "the 4.2x" might be "the TSS iomap switch costs 4.2x a plain yield switch" — a within-boot ratio. That's plausible! "the debug-versus-release argument" could be that the 4.2x holds in both debug and release. In that case the claim "ratios within a boot" could be accurate.

I can't resolve this due to truncation. Flagging it as a definite defect is risky. Hmm.

Let me look for other, more checkable defects.

20. Check the milestone's "0 of 200 boots at load 30 were wrong by more than 1%" vs the table's "16 windows ... 0 / 200". The BUGS bullet says "0 of 200 boots at load 30 were wrong by more than 1%" — but this is for the *fix as shipped* (early stopping), not min-of-16. The early-stop rule "reaches exactly the accuracy of taking the full cap every time, boot for boot" over the 490 boots, so at load 30 the shipped rule also gives 0/200. OK consistent.

21. tsc-under-tcg.md updated paragraph: "After the fix, 0 of those 200 boots are wrong by more than one per cent and the worst is +0.47%." Consistent with table (16 windows: worst +0.47%, 0/200). OK.

22. Now — the milestone says the sweep was "490 boots at three host loads". Ordinary: 30 boots ("one window's worst of 30 boots was +773%"). Load 22: ? Load 30: 200 boots. 30 + 200 = 230; so load 22 would be 260 boots? "490 boots at three host loads" — 30 + 260 + 200 = 490. Possible. But wait: "at load 22 the same estimators gave +318% and +0.34%" — same estimators = one window and min-of-9. OK. No contradiction.

Hmm wait, actually: "Measured over the same 490 boots" for the stopping rule means the stopping rule was evaluated on all 490 boots including the 200 at load 30 that timed sixteen windows. But did the ordinary-load 30 boots and load-22 260 boots also time sixteen windows each? The text says "Two hundred boots on a host deliberately saturated to load 30..., each timing sixteen windows and reporting every one". For the stopping-rule evaluation over 490 boots, all 490 must have timed up to 16 windows (or at least until stopping + verification). "Measured over the same 490 boots, this reaches exactly the accuracy of taking the full cap every time" — implies all 490 recorded all 16 windows. Plausible.

23. Let me check "+1153% is a 125 ms one": 125 ms / 10 ms = 12.5 → 12,500 MHz → +1150%. The precise +1153% → 125.3 ms. "a 125 ms one" — fine as rounding.

24. "a 4330 MHz reading is a 43 ms window reported as a 10 ms one" — 4330/1000 × 10 = 43.3 ms. OK.

25. Milestone: "notes/benchmarks.md: the 2026-08-24 x86 ns/iter table is marked in place, and a new dated section records what the defect reaches and what it does not." OK.

26. Milestone index row: "the mean cost is 3.5 windows on a quiet host and 5.2 on a saturated one against a cap of sixteen." OK.

27. Now, the proposal deletion: the milestone says "The proposal this promotes, design/roadmap/proposals/the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md, is removed by this milestone: its two open questions are both answered above (the boot does print its confidence, and the local APIC timer is derived from the same windows)."

Wait — "the local APIC timer is derived from the same windows". But the BUGS section says "The local APIC timer's rate gets the same treatment and has no CPUID escape. It is the minimum over the same windows, taken independently of the TSC's." Hmm, "derived from the same windows" vs "taken independently of the TSC's" — consistent: same windows, independent min. OK.

But wait, there's a subtlety: "Unlike the TSC there is no path that skips the measurement, so on a part that reports leaf 0x15 the windows are still timed." Hmm — if CPUID leaf 0x15 gives the TSC rate directly, does the boot still time PIT windows for the APIC timer? The BUGS entry says yes. OK, internally consistent.

28. Let me look at the "0.04%" claim again... fine.

29. The milestone title/status: "Status: BUILT 2026-09-21." OK.

30. Now the qemu-bounded bug description: "the wrapper bounds a run by sending SIGTERM to the child it started, which on aarch64 and riscv64 is QEMU and here was the runner shell. The shell died; QEMU was reparented to pid 1 and ran forever."

Hmm — "reparented to pid 1". On macOS (patagonia is an Apple Silicon host), pid 1 is launchd. tsc-under-tcg.md says "left qemu-system-x86_64 orphaned to launchd past its bound". Consistent.

"It also needed qemu-bounded.sh's own fd-3 dance, because backgrounding a command gives it /dev/null on stdin under dash and would have broken every piped-input run in CI."

Hmm — backgrounding a command in dash gives it /dev/null on stdin? Actually, POSIX says asynchronous commands (backgrounded without job control) get stdin redirected from /dev/null. dash does this. So the runner now backgrounds QEMU, which would get /dev/null stdin, breaking piped input; the fix uses fd 3 to pass the real stdin. Plausible. OK.

31. Now let me scrutinize the benchmarks.md new section's claim about which numbers are affected once more:

"- **The 2026-09-15 lazy-TSS section and the icount baselines are not affected.** They quote deterministic ticks under `-icount` and say so; `bench/baseline-*.txt` holds ticks."

Hmm wait — is the "2026-08-24 x86 ns/iter table" measured under plain TCG with --real? The visible text after the correction: "QEMU 11.0.2 (`.qemu-version`, pinned), `-machine q35 -cpu max`, one hart, plain TCG. Six boots debug, five release; `ns/iter` computed from the guest's own calibrated TSC". Plain TCG, no -icount mentioned. So those were effectively "--real" style runs (wall-clock-ish under TCG). OK, so the correction is placed correctly.

But hold on — the new section says "The calibration defect below affects the `--real` instrument only". The 2026-08-24 table ran under plain TCG without -icount — is that "--real"? For x86, `bench --x86` without --real adds -icount; plain TCG without -icount is what --real gives (no HVF on x86 on an aarch64 host; --real just omits -icount). OK.

32. Now, a potentially important one: the correction block says "See the 2026-09-21 section at the end of this file". The new section is indeed at the end. OK.

33. The correction block: "the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm, let me think about whether the "debug-versus-release" comparison can be within a boot. Actually — wait. Maybe the 2026-08-24 section measured debug and release in the SAME boots? "Six boots debug, five release" — no, that's 6 boots of a debug binary and 5 boots of a release binary. A debug-vs-release ratio is necessarily cross-boot. Different boots stored different wrong rates (the commit message itself stresses "each of those eleven boots stored a different wrong rate"). So a ratio between a debug number and a release number does NOT cancel the calibration — it cancels only if both used the same rate, which they didn't.

But is "the debug-versus-release argument" a ratio of ns/iter values? If the argument is "release is 4.2x faster than debug" — that's a cross-boot ratio of ns/iter, which does NOT cancel. The correction claims it does. That would be a real metrological error... but it hinges on what "the debug-versus-release argument" and "the 4.2x" actually are in the truncated section.

Hmm, hold on. Let me reconsider. Maybe "the 4.2x" is the debug→release tax (like the "~6.7x tax on IPC" elsewhere). If the section's headline conclusion is "release is 4.2x faster than debug on x86", that's a ratio of a release-boot ns/iter to a debug-boot ns/iter — cross-boot, calibration does not cancel. The correction block explicitly names "the debug-versus-release argument and the 4.2x" as "ratios within a boot, where the calibration cancels exactly". If the 4.2x is the debug/release ratio, the claim is wrong.

But maybe the 4.2x is something else entirely, like "tss_iomap_switch costs 4.2x yield_switch" measured within each boot. And the "debug-versus-release argument" might be "the 4.2x is the same in debug and release", which rests on within-boot ratios. I genuinely cannot tell from what's visible.

Given the truncation, I should be cautious. But actually — wait. Let me re-read the context line at the top of the diff hunk: "per iteration (both threads resume once each), so the delta divided by two is th". This is the tail of a paragraph just before "### The numbers". The section seems to be about x86 context-switch-related benches (yield_switch, tss_iomap_switch per the new section's bullet). The "debug-versus-release argument" — with "Six boots debug, five release" — the natural reading is the section compares debug and release ns/iter to argue something (maybe "the lazy TSS win is real in both builds" or "release is 4.2x faster").

Hmm, hmm. Actually, you know what — there's a subtlety that makes the claim *defensible* even for cross-boot ratios: no wait, it's not defensible. A ratio across boots with different calibration errors carries the ratio of the two errors. The commit message itself emphasizes "each of those eleven boots stored a different wrong rate". If the section's conclusions were cross-boot ratios, they would NOT survive untouched. The correction asserts all conclusions survive because they're all within-boot ratios. If the debug-vs-release argument is cross-boot, then the assertion "that is all of them" is false, and the named example "the debug-versus-release argument ... [is a] ratio within a boot" is false.

This might be the planted defect. But I can't fully verify due to truncation. Let me look at the numbers once more: is there any other place where I can find "4.2x" in the visible content? Searching the visible before-content... I see "~6.7x tax on IPC", "~5x faster than Linux", "~2.6x", "~38x", "~15x gap", "~1.5x tax", "3.5x speedup", "0.18x", "2.0x", "1.99x", "26.4%", "+1.0%, +1.4%, +3.9%"... no "4.2x" visible. So the 4.2x is in the truncated region. Can't verify.

OK here's another approach: look for defects in what I CAN fully see.

34. Milestone: "the worst this lane saw, +1153%, is a 125 ms one." — fine.

35. Milestone table row 1: "1 (what shipped) | +0.36% | +884% | +1153% | 56 / 200". And benchmarks.md's before/after: "Before (one window) | 1003.7 MHz | +0.37%". The median error at 1 window is +0.36%, and the single before-draw was +0.37%. Coincidentally close but different things (median vs single draw). OK, not a contradiction.

36. "Five still leaves one boot in eighteen wrong by more than a per cent and a worst case of +51%." 11/200 = 1/18.18. OK. Table says worst of 200 at 5 windows is +51.4%. "+51%" OK.

37. "The tail only closes at sixteen." Table: 16 → 0/200 >1%, worst +0.47%. OK.

38. "The median is +0.00% at every count from three upward" — table shows median +0.00% for 3, 5, 9, 16. OK.

39. Stopping rule: "two windows agree to within one part in a thousand" vs tsc-under-tcg.md "two windows agree to one part in a thousand". Consistent.

40. "A tighter tolerance (one part in 2000) moved the mean by 0.07 windows and the error distribution not at all, which says the choice is not delicate: clean windows agree to within the PIT's own quantisation and dirty ones are nowhere near."

PIT quantisation: PIT at 1.193182 MHz, 10 ms = 11932 ticks → quantisation ~1/11932 ≈ 0.84 parts in 10,000... wait, the quantisation of the *window* is one PIT tick ≈ 0.84 µs out of 10 ms ≈ 0.0084% ≈ 1 part in 12,000. Hmm, "clean windows agree to within the PIT's own quantisation" — but the stopping rule needs agreement to 1 part in 1000, which is much looser than PIT quantisation (1 part in 12,000). Hmm, wait: but the TSC delta is read either side of the poll; the poll granularity is a port read (~µs?). Eh, this is hand-wavy but not checkable.

41. The milestone says "notes/benchmarks.md marks the affected table where a reader meets it rather than quietly restating it" (commit message). OK.

42. Let me check the milestone's "## calef's ruling named the shape; the count was measured" — "> 'Fix the calibration with min-of-five windows.' (2026-09-21)". OK.

43. The milestone says the proposal's two open questions: "(the boot does print its confidence, and the local APIC timer is derived from the same windows)". The proposal's two questions were: (1) whether the boot should say how confident it is; (2) whether the local APIC timer's rate should be derived from the same windows. OK, matches.

44. The milestone Follow-on: "Recorded. That nothing gates the calibration's *accuracy*, only its estimator, is in kernel/src/arch/x86_64/timer_calibration_tests.rs's own BUGS". OK.

45. Now — check the tsc-under-tcg.md edit once more. The original ended the section with:

"**This is tracked**: `design/roadmap/proposals/the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md`, and `arch::x86_64::timer`'s `BUGS` section now says it where a reader meets the feature."

Replaced with the FIXED paragraph. Good — the proposal link is removed (the file is deleted), so no dangling link there. 

But wait — does anything else still reference the deleted proposal? Let me search the visible content... In the milestone file: "The proposal this promotes, `design/roadmap/proposals/the-x86-boot-calibrates-once-and-be-wrong-by-4x.md`, is removed by this milestone" — it references the path while saying it's removed. That's fine as a historical record.

tsc-under-tcg.md's "Reproducing it" section says "the calibration fix proposed above is what it exists to be measured against." — the commit didn't touch that line. "the calibration fix proposed above" — the proposal is now promoted/removed; the note's paragraph above the repro section was changed to FIXED. The line "the calibration fix proposed above" now reads slightly stale ("proposed" → now built), but it's in the Reproducing section: "This note is its owner, and the calibration fix proposed above is what it exists to be measured against." Minor staleness, arguably not a defect.

46. Now — check the milestone's statement about `wait_for`: not in the milestone, but in benchmarks.md: "kernel::user::wait_for takes now() plus two seconds". OK.

47. Check the benchmarks.md claim: "**The 2026-09-15 lazy-TSS section and the icount baselines are not affected.** They quote deterministic ticks under `-icount` and say so". Hmm wait — is the 2026-08-24 table itself under -icount? The text right after the inserted correction says "plain TCG" and "ns/iter computed from the guest's own calibrated TSC". If it were -icount, the ns would be computed from the calibrated TSC too, but the calibration defect still affects the ns conversion. Actually under -icount, does the boot calibration still run? Yes, the boot always calibrates. Under -icount, ns/iter computed from ticks/freq is "not wall time" anyway. The correction says the 2026-08-24 figures are suspect because they were computed from the calibrated TSC under plain TCG (no -icount). OK.

48. Now, one more careful check of the milestone's central statistical claim:

"**The error being one-sided is the whole of the fix.** Every window is an *upper bound* on the true rate, so the minimum of several is the tightest bound taken, and it converges on the truth from above. An average would be a biased estimator for precisely the reason the minimum is an unbiased one: it carries every descheduling into the answer instead of discarding it."

So: every window ≥ truth (upper bound). Min of windows ≥ truth. Therefore min is biased (upward). The text calls it "unbiased". Contradiction within the same paragraph: "converges on the truth from above" (i.e., remains above for finite samples — biased) vs "the minimum is an unbiased one". This is a genuine conceptual defect in the document. The minimum is a *consistent* estimator (converges as N→∞) and *less biased* than the mean, but it is not unbiased — indeed the document's own table shows min-of-16 still has a worst case of +0.47% and p99 +0.01%, i.e., residual positive bias. And the BUGS section says "This is a probabilistic filter, not a bound... produces an inflated rate however many windows are taken" — again acknowledging residual upward error.

Also the deleted proposal said it correctly: "the minimum of several windows converges on the truth from above" and never claimed unbiasedness. So the milestone introduced the wrong word. This is a solid, checkable, conceptual defect. I'm fairly confident this is a (or the) planted defect. Actually — hmm, wait. Let me steelman: could "unbiased" be defended? If clean windows exist with probability 1 eventually... no. Unbiased means E[estimate] = truth. E[min] > truth whenever there's nonzero probability all windows are inflated, which the document itself insists is always possible ("a host that never leaves the vCPU alone... produces an inflated rate however many windows are taken"). Also even a clean window is inflated by poll latency (≥ 0, strictly > 0 in expectation). So E[min] > truth: biased. The claim "the minimum is an unbiased one" is wrong, and worse, it contradicts the document's own framing ("upper bound", "converges from above", "probabilistic filter, not a bound"). This is exactly the kind of subtle conceptual error these exercises plant. 

49. Another candidate: the milestone says "Measured over the same 490 boots, this reaches **exactly** the accuracy of taking the full cap every time, boot for boot". Hmm — is that plausible? The stopping rule stops when two windows agree within 0.1%. Consider a boot where window 1 and window 2 are both inflated by nearly identical deschedulings (agree within 0.1%) but a later window would have been clean. The rule stops early with an inflated answer ≠ min-of-16. The document argues this "is not a thing that happens", and claims measured exact agreement across 490 boots. It's a measured claim; can't refute. OK.

50. Check "0.04%" once more: 90 ms / 218,000 ms = 0.04128% → "0.04%". OK.

51. Check tsc-under-tcg.md's updated claim "the mean cost is 3.5 windows on a quiet host" vs milestone "3.53". OK.

52. Now check the benchmarks.md appended section heading: "## 2026-09-21: the x86 boot calibration was wrong by up to 12x". And the correction block says "+1153%". 12.53x. Hmm, "up to 12x" — the proposal was titled "wrong by 4x" because the worst seen was 4330 MHz (4.33x). Now worst is 12.53x. "12x" is a rounding of 12.53; "up to 12x" understates by 4%. Eh. Actually, hmm, "wrong by up to 12x" — if we read "wrong by Nx" as "off by a factor of N", then 12.53x → "12x" is fine-ish as a rounded-down headline. I won't flag this strongly. Although... some might consider "up to 12x" wrong when the actual figure is 12.5x — "up to" should bound above. 12.53 > 12, so "up to 12x" is technically false; should be "up to 13x" or "12.5x" or ">12x". Hmm. It's borderline. I'd mention it only if I don't find better. Actually let me reconsider: +1153% = 12.53×. The heading says "wrong by up to 12x". Also the commit message says "+1153%". And the milestone says "+1153%". The heading's "12x" vs "12.53x" — I think this is deliberate? Hmm, "wrong by up to 12x" could also be parsed as "wrong, by up to 12x" i.e. the error factor reaches ~12x. It's a loose headline. I'll consider mentioning it as a minor item.

53. Let me check the milestone's "The measured boot cost" arithmetic again: "boots the kernel four times, and those four boots took 3, 3, 4 and 3 windows. So the suite pays 90 ms more than the one-window design did". 13 windows × 10 ms = 130 ms; old design 4 × 10 = 40 ms; delta 90 ms. OK.

54. "against a 3m38s runtime: 0.04%, an order of magnitude below the suite's own run-to-run variance." OK.

55. Milestone: "Ten milliseconds per window is real time on every boot, including every CI test boot, and the suite boots repeatedly." OK.

56. Milestone's "What is in the diff" says "kernel/src/arch/x86_64/timer.rs: ... a `Calibration` record (**provisional name**)". OK.

57. Milestone: "**x86_64 only by nature, and not a parity gap.** Under §19 ... aarch64 reads CNTFRQ_EL0 and riscv64 reads the device tree; both are architected statements of the rate, so neither has a calibration that could be wrong." Consistent with tsc-under-tcg.md's last paragraph. OK.

58. Let me double-check the correction block's placement claim: "notes/benchmarks.md marks the affected table where a reader meets it rather than quietly restating it." The block is placed right before "### The numbers" of the 2026-08-24 section. OK.

59. Hmm, wait. Let me re-read the correction block: "These were computed from the guest's calibrated TSC, and on 2026-09-21 the x86 boot calibration was found to be wrong by up to **+1153%**, always high, differently on every boot."

And the section below (2026-08-24) says "Six boots debug, five release". The correction says "these eleven boots". OK.

60. Now the "ns column alone" claim: "every `ns/iter` in this section is suspect, and the `ns` column alone." The ticks columns are unaffected. OK.

61. Now think about whether "the calibration cancels exactly in a ratio" is right even within a boot: ns/iter = ticks × 1e9 / freq. Ratio of two ns/iter from the same boot = ticks1/ticks2. Yes, freq cancels exactly. OK.

62. Let me reconsider the debug-vs-release issue once more. The correction block says: "every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm, actually — maybe in that section, each boot produced a table row, and the "debug-versus-release argument" is about the *ratio* between two benches (yield_switch vs tss_iomap_switch) being preserved across builds. I.e., "the 4.2x" = tss_iomap_switch / yield_switch ≈ 4.2, stable across debug and release. Then both named items are within-boot ratios. That's self-consistent with the claim. Plausible. Since the section is about "tss_iomap_switch" vs "yield_switch" (the new section's bullet names those two benches), "the 4.2x" is likely tss_iomap/yield. And "the debug-versus-release argument" is likely "the 4.2x holds in both debug and release, so it's structural, not codegen". Both within-boot ratios. OK — I'll not flag this; the benefit of the doubt goes to the author given truncation.

63. Now let me look again at the milestone "What is in the diff" vs the actual commit diff. The commit diff contains ONLY: the new milestone file, the proposal deletion, benchmarks.md, tsc-under-tcg.md. The milestone's "What is in the diff" lists kernel/src/arch/x86_64/timer.rs, kernel/src/main.rs, kernel/src/arch/x86_64/timer_calibration_tests.rs, kernel/src/arch/x86_64/tsc_probe.rs, scripts/qemu-runner-x86_64.sh, notes/benchmarks.md. This commit contains none of the code changes. Is that a defect? The commit message says "Record the calibration fix: milestone 526, the note, and the suspect numbers" — the word "Record" suggests the fix landed elsewhere and this commit records it. Design records "treated as part of the deliverable" — the milestone describes the whole change including code. In many repos, the roadmap milestone is committed together with the code; here it's separate. The PR would presumably include the code commit too ("Below is the commit as it would appear in a pull request" — singular commit). Hmm. If the PR contains only this commit, then the milestone claims BUILT with code changes that aren't in the PR. But the exercise says "followed by the full contents of each changed file as it stood BEFORE the change" — only the doc files. The exercise is about the docs. I think the intended scope is docs-only and the code landed in a sibling commit. Not the defect.

64. Let me look at the milestone's claim about the RTC once more: "Its seconds register has one-second granularity, so calibrating against it means either waiting for a seconds edge (up to one second of boot, a hundred times this fix's cost)". This fix costs ~35-52 ms mean; waiting up to 1 s is ~20-30x the mean cost, or vs the 90 ms suite delta ~11x. "a hundred times this fix's cost" — hmm. The fix's cost per boot is ~35 ms mean (ordinary load). 1 second / 35 ms ≈ 28x. Against the cap cost (160 ms) it's 6x. "a hundred times this fix's cost" seems exaggerated. Hmm, wait — later: "so it would need the same min-of-N treatment on top of costing a hundred times more." Let me compute more carefully. What's "this fix's cost"? The mean calibration cost is 35-52 ms. Waiting for a seconds edge costs up to 1 s = 1000 ms. 1000/35 ≈ 29; 1000/52 ≈ 19. Not 100. But "waiting for a seconds edge (up to one second of boot...)" — plus then spanning time to get precision? To calibrate to 0.1% against a 1-second-granularity reference via edge-to-edge, you'd need... an edge gives you a timestamp with ~poll-cost error (µs), so a single edge-to-edge second gives ~ppm precision actually (per tsc-under-tcg: "Over a four-second window that is 0.04 to 0.26 ppm"). Wait, the probe spans 32 seconds. But that's for drift measurement precision. For calibration to 0.1%, one second edge-to-edge suffices (error ~1 µs / 1 s = 1 ppm). Hmm, so "waiting for a seconds edge (up to one second of boot, a hundred times this fix's cost)" — 1 s vs 10 ms (one window) = 100x! Ah, "this fix's cost" might be per-window: 10 ms. 1000/10 = 100. Yes — "a hundred times this fix's cost" if the fix's cost is counted as 10 ms... but the fix costs 35-52 ms mean, 160 ms cap. Hmm, ambiguous. 1 s / 10 ms = 100. The sentence: "calibrating against it means either waiting for a seconds edge (up to one second of boot, a hundred times this fix's cost) or reading it as a time and accepting a one-second error." I think "this fix's cost" loosely = the 10 ms window, or the ~10 ms order. 1 s is 100× a 10 ms window. But the fix as a whole costs 35-52 ms mean. 1000/35 ≈ 29×. Saying "a hundred times" is loose but in the right order if compared to one window. Borderline; not flagging.

Also "tsc_probe gets its precision by watching for an edge and then spanning thirty-two seconds, which is fine for an instrument that runs once on demand and absurd for every boot." OK.

65. Now, the milestone says about the stopping rule: "for two windows to land within a thousandth of each other they must both have been left alone, because two independent deschedulings agreeing to three decimal places is not a thing that happens."

Hmm — actually there's a subtle flaw: two windows could also agree if both were left alone OR if the host is so loaded that... no. Or if both windows are inflated by the SAME amount — e.g., a periodic descheduling of exactly the same duration. "not a thing that happens" is a probabilistic claim. Fine as design reasoning.

But wait, there's another subtlety: agreement within 0.1% of two *clean* windows requires the measurement noise of clean windows to be < 0.1%. Clean-window noise sources: PIT tick quantisation (~0.0084%), poll latency variation. OK, plausible.

66. Now, "one part in a thousand" = 0.1%. The table's "Boots wrong by >1%" uses 1%. The stopping rule tolerance 0.1% vs the accuracy target 1%. OK.

67. Let me check the milestone's claim about the mean at saturated load: 5.20 windows → 52 ms. And "calef's five is what this costs" — cute. OK.

68. Now check the milestone's BUGS entry: "Nothing gates the accuracy. The two boot tests assert the estimator is the one intended, not that the answer is right; checking the answer needs an independent clock, which is tsc_probe behind an off-by-default feature costing thirty-five seconds a boot. A gate would mean a thirty-five-second suite leg to catch a defect that fails safe, which is not a trade worth making today."

OK.

69. Now, the benchmarks.md new section's bullet: "**The 2026-09-15 lazy-TSS section and the icount baselines are not affected.** They quote deterministic ticks under `-icount` and say so; `bench/baseline-*.txt` holds ticks."

Hmm — wait. Is the 2026-08-24 x86 table itself possibly an -icount table? The text after the correction says "plain TCG" — no -icount flag mentioned, and "ns/iter computed from the guest's own calibrated TSC". Under -icount the ns would also be computed from the calibrated TSC... but the new section says the calibration defect "affects the --real instrument only". The 2026-08-24 table: plain TCG without -icount = "--real" for x86. OK consistent.

But hmm — "one hart, plain TCG" — and the new section says "Every section of this file that quotes x86 numbers should say which one it used,