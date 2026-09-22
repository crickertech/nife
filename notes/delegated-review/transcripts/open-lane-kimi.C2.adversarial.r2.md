[no text block returned; the model spent its budget on thinking]

Let me carefully review this commit. It's a documentation-only change to a repository that treats design records as deliverables. The commit:

1. Adds `design/roadmap/526-the-x86-tsc-calibration-takes-the-smallest-of-several-windows.md` (new milestone record)
2. Deletes `design/roadmap/proposals/the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md`
3. Modifies `notes/benchmarks.md` (adds correction notice and new dated section)
4. Modifies `notes/tsc-under-tcg.md` (marks finding as fixed)

I'm looking for a defect that no automated check can see — i.e., a factual, logical, mathematical, or internal-consistency error in the prose/numbers.

Let me check the numbers carefully.

**The +1153% figure:** The TSC ticks at 1000.000 MHz. Worst reading... 4330 MHz over 22 boots became +1153% over 490 boots. +1153% means the stored rate was 1000 × (1 + 11.53) = 12,530 MHz ≈ 12.5 GHz. The text says "the worst this lane saw, +1153%, is a 125 ms one" — a 125 ms window reported as 10 ms gives 12.5× the rate = 12,500 MHz. That's consistent: 12,500/1000 = 12.5, i.e., +1150%. So +1153% ≈ 12.53×, i.e., a 125.3 ms window. OK, consistent.

**The benchmarks.md new section title:** "the x86 boot calibration was wrong by up to 12x". +1153% means 12.53× the true value, i.e., wrong by a factor of ~12.5x. Saying "up to 12x" is roughly consistent (slightly understated but "12x" vs 12.53x... fine, it's a headline).

Wait — actually, let me check: is +1153% "12x"? An error of +1153% means the value is 12.53 times the truth. "wrong by up to 12x" — a factor of 12.5. OK.

But hold on — the commit message says "4330 MHz over twenty-two boots became +1153% over 490." And 4330 MHz is +333%. OK.

**Check the "eleven boots" claim in benchmarks.md correction note:** "Nothing can recover the true values: the rate each of these eleven boots stored was not written down, and it was a different number each time." The 2026-08-24 x86 ns/iter table: "Six boots debug, five release" — 6 + 5 = 11 boots. Consistent. Good.

**The correction note says:** "The *ticks* they were derived from are unaffected, and so is every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm, wait. Is the debug-versus-release comparison a ratio *within* a boot? Let me think. The 2026-08-24 section (which we don't see in full due to truncation, but the correction note refers to it) says "Six boots debug, five release". A debug-versus-release ratio would be a ratio between a debug boot and a release boot — i.e., *across* boots, not within one boot. If each boot stored a *different* wrong rate, then a ratio between a debug number from one boot and a release number from another boot does NOT cancel the calibration — the calibration factor only cancels if both numbers share the same stored rate, i.e., same boot.

Actually, wait. Let me re-read: "every conclusion here that rests on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them."

Hmm, the commit message says: "What survives untouched is every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them."

So the claim is that the debug-versus-release argument and the 4.2x are ratios *within a boot*. But if the table has six boots debug and five release, then the debug-vs-release comparison is inherently cross-boot. Unless... each boot ran both debug and release? No — "Six boots debug, five release" implies each boot was either a debug build or a release build.

Hmm, but wait. Actually let me think about this more carefully. The claim "the debug-versus-release argument... [is a] ratio within a boot" seems suspicious. If debug numbers come from debug boots and release numbers from release boots, then the ratio debug/release = (ticks_debug / rate_debug) / (ticks_release / rate_release). The rates differ per boot. So the calibration does not cancel.

Unless the debug-versus-release argument was made in ticks, not ns. The correction note says "The *ticks* they were derived from are unaffected" — so if the debug-vs-release argument was argued in ticks, it's fine. But the note claims the reason it's safe is that "the calibration cancels exactly in a ratio," which is the wrong reason if the ratio is across boots.

Hmm, I need to be careful here. I don't have the full 2026-08-24 section (truncated). Let me look at what's visible.

From the truncated benchmarks.md, I can't see the 2026-08-24 section directly. The visible portion ends with "The kernel's memory footprint..." section at 2026-08-17, then truncated at 1200 lines, then the tail shows a section ending at line 3781-ish with "+1.0%, +1.4%, +3.9%" about some exemption (that's the 2026-09-15 lazy-TSS section maybe?), then the new section appended.

The diff hunk at line 2664 shows the correction note inserted before "QEMU 11.0.2 ... Six boots debug, five release; ns/iter computed from the guest's own calibrated TSC".

So the section: 2026-08-24 x86 ns/iter table with yield_switch, tss_iomap_switch, debug and release. Six boots debug, five release. The new section says "every conclusion in that section that rests on a ratio between two rows of one boot survives untouched" and the in-place note says "the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm. If the debug-versus-release comparison uses the median of six debug boots vs median of five release boots, then each boot's ns figures were computed with that boot's own wrong rate. The ratio of medians does not cancel the calibration error unless the debug boots and release boots happened to have the same calibration error. The rates were "wrong by up to +1153%, always high, differently on every boot" — though under ordinary session load, the errors were small (+0.1% to +4.2% based on the 22-boot table: 1001-1005 MHz mostly, one 1042).

Actually, in practice, under ordinary load, the errors were sub-percent mostly, so a debug-vs-release ratio might be off by ~1%, which is within noise. But the *argument* presented — "the calibration cancels exactly in a ratio" — is only valid for ratios within a single boot. For a cross-boot ratio, the calibration error does not cancel; it's just (probably) small.

Hmm, but is this "the defect"? Let me consider whether the debug-versus-release argument might actually be within-boot. Consider: maybe the 2026-08-24 section measured debug and release in the same boot? "Six boots debug, five release" strongly suggests separate boots per build.

Hmm wait, actually, maybe I should reconsider. Perhaps the "4.2x" is the debug-to-release tax within... no. Let me think about what "the debug-versus-release argument and the 4.2x" refers to. In the visible text, there's a reference to "the debug-to-release tax on the IPC path is ~6.7x (measured, in the cross-OS section below)". That's a different (aarch64) section. The 2026-08-24 x86 section apparently has a "4.2x" figure — presumably the debug-to-release tax on x86 for some path (tss_iomap_switch?).

A debug-vs-release ratio computed from six debug boots and five release boots is a cross-boot ratio. The claim that "the calibration cancels exactly in a ratio" is mathematically wrong for cross-boot ratios. The commit message even states the correct principle: "every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them."

So the question: is the debug-versus-release argument a ratio between two rows of one boot? If the table's rows are (debug, release) pairs and each boot ran one build... then no. Hmm.

But wait — maybe the section compared rows within each boot? Like, maybe each boot ran both `yield_switch` and `tss_iomap_switch`, and the "4.2x" is the ratio between those two benches within one build? Actually the new section mentions "The 2026-08-24 x86 ns/iter table (`yield_switch`, `tss_iomap_switch`, debug and release)". So the table has two benchmarks × two builds.

Hmm, "the debug-versus-release argument and the 4.2x" — I think the 4.2x IS the debug-versus-release ratio. So the claim is that the debug/release ratio is safe "because the calibration cancels exactly in a ratio." That's only true within a boot. Across boots with different wrong rates, it does not cancel.

Actually, hold on. Let me reconsider. Maybe the debug-vs-release comparison was done as: for each of the 11 boots, ns/iter was computed. Then the debug median across 6 boots and release median across 5 boots. Ratio = 4.2x. The calibration errors on those 11 boots are unknown (not written down), each different. The ratio of (ticks_debug/rate_debug) to (ticks_release/rate_release) carries error rate_release/rate_debug... no wait: ns = ticks/rate × 1e9. Inflated rate → ns too small. debug_ns / release_ns = (ticks_d/ticks_r) × (rate_r/rate_d). If rate_d and rate_r differ (different boots), the ratio is off by rate_r/rate_d, which does not cancel.

So strictly, the claim "cancels exactly" is wrong for cross-boot ratios. The honest statement would be "the calibration error on those boots was, on the evidence of the 22-boot sweep at ordinary load, likely under 1%, so the 4.2x stands to within that" — but they didn't write down the rates, so they can't even assert that for those specific 11 boots. Though the 22-boot sweep at ordinary session load showed 1001-1042 MHz, i.e., +0.1% to +4.2%.

Hmm, but the commit explicitly claims the cancellation is *exact* and that this covers *all* conclusions. If the debug-vs-release ratio is cross-boot, this is a genuine analytical error in the record. But I'm not 100% sure it's cross-boot. Let me look again at the in-place note: "every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot."

"two rows of the same run" — hmm, "run" might mean a boot. If the 2026-08-24 section's table has rows for debug and release... six debug boots and five release boots produce 11 runs. A debug row and a release row are not from the same run.

Hmm, unless the table was organized differently. I genuinely can't see the original section. This is a candidate concern but I should weigh confidence.

Actually — wait. Let me reconsider. The note says "Six boots debug, five release; ns/iter computed from the guest's own calibrated TSC". If the debug-versus-release argument compared medians across boots, then the claim in the correction note is wrong. But maybe the original section's argument was e.g. "within each build, tss_iomap_switch is X times yield_switch" — that's within-boot (each boot ran both benches). And "the debug-versus-release argument" might be about... hmm.

Let me look at the phrasing again: "the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Two things named: "the debug-versus-release argument" and "the 4.2x". Maybe they're separate: the debug-vs-release argument could be a qualitative argument (e.g., "release is faster, so use release numbers for comparisons") and the 4.2x could be a within-boot ratio (tss_iomap_switch vs yield_switch?). Without the original section, I can't be sure.

Hmm, hmm. Let me look for other, more checkable defects.

**The stopping rule and the "agreement is evidence of cleanliness" argument:** "for two windows to land within a thousandth of each other they must both have been left alone, because two independent deschedulings agreeing to three decimal places is not a thing that happens."

Wait, one part in a thousand is 0.1%, which is "three decimal places"? One part in a thousand is 0.001 = 10^-3, i.e., agreement to within 0.1%. "Three decimal places" would typically mean 0.001 in absolute terms, or... for a number like 1000 MHz, one part in a thousand is 1 MHz. "Agreeing to three decimal places" is a sloppy way to say it but roughly: two windows both measuring e.g. 1000.4 and 1000.9 MHz agree to within 1 part in 1000. Deschedulings are of "essentially arbitrary size" (milliseconds), so two dirty windows agreeing within 1 MHz of 1000 MHz is unlikely. OK, the argument is heuristic and they measured it. Fine.

**Check the arithmetic in the "worst window" explanation:** "a 4330 MHz reading is a 43 ms window reported as a 10 ms one" — 4330/1000 = 4.33, × 10 ms = 43.3 ms. ✓.

"+1153% is a 125 ms one" — 12.53 × 10 = 125.3 ms ✓.

**Check the table:** "1 (what shipped) | +0.36% median | +884% 99th percentile | +1153% worst | 56/200 boots wrong by >1%".

Hmm, 99th percentile of 200 boots is the 198th-199th value. +884% 99th percentile with worst +1153% — plausible. 56/200 = 28% wrong by >1%, but median error +0.36% (under 1%) — consistent (28% < 50%).

Min-of-3: worst +131%, 24/200 > 1%. Min-of-5: worst +51.4%, 11/200. Min-of-9: worst +7.1%, 2/200. Min-of-16: worst +0.47%, 0/200. Monotonically decreasing ✓ plausible.

**Check "one boot in eighteen":** "Five still leaves one boot in eighteen wrong by more than a per cent" — 11/200 = 1 in 18.2 ✓.

**Check mean windows:** "Measured over the same 490 boots, this reaches exactly the accuracy of taking the full cap every time, boot for boot, at a mean of: Ordinary 3.53, Busy 4.77, Saturated 5.20."

Hmm wait — "reaches exactly the accuracy of taking the full cap every time, boot for boot." The stopping rule: stop as soon as two windows agree to within 1/1000. The claim is that the early-stop answer always equals the min-of-16 answer. Is that plausible? If two windows agree within 0.1%, they're both "clean" (per the argument), so the min so far is within 0.1% of truth; further windows could only lower the min slightly... Actually if two windows agree to within 0.1%, the minimum of all 16 might still be slightly lower than the current min (if a later window is even cleaner). Hmm, but if windows are clean they all measure ~the same true rate to within PIT quantization. Actually the claim "exactly... boot for boot" is suspicious as stated, but it's an empirical claim ("Measured over the same 490 boots"), so maybe the data showed that. Actually wait, would the early-stop min ever differ from the full-16 min? If the first two clean windows agree, min-so-far ≈ truth. Later windows could be even cleaner, giving a slightly lower min. So "exactly equal" would be surprising... unless the stopping rule compares to the min and the min is one of the agreeing pair. Hmm, e.g., if the rule is "stop when two windows agree within 0.1%" and the answer is the min of windows taken. If a later window would have been lower than the current min, then... the current min and that later window would have agreed within 0.1% too (since both ≈ truth). The answer differs only in tiny amounts (within 0.1%). "Exactly the accuracy" might mean "the same to the accuracy printed" or that in all 490 boots the chosen answer equaled the full-cap answer. Eh, it's an empirical claim; can't falsify from here. But "exactly... boot for boot" is a strong claim. Hmm.

Actually, wait. Let me think about whether the early-stopping rule can be *worse* than min-of-16 in a way the text misses... Suppose window 1 is inflated by a tiny amount (0.05%) and window 2 is clean — they agree within 0.1%, stop, answer = min = clean window. Fine. Suppose window 1 inflated 0.09%, window 2 clean: min = window 2 = truth. Fine. Suppose windows 1 and 2 are both inflated, by 0.1% and 0.19% — they don't agree (difference 0.09%... wait 0.19-0.1 = 0.09% < 0.1%, they DO agree). Then answer = +0.1% inflated. Min-of-16 might find a clean window = truth. So early-stop answer can be marginally worse, bounded by ~0.1%. The text says "reaches exactly the accuracy of taking the full cap every time, boot for boot" — that seems too strong. But again, it's an empirical measurement claim. Not clearly a defect I can prove.

**Now the bigger fish: the "43 ms" vs "125 ms" consistency — fine. Let me check "0.04%":** 90 ms vs 3m38s = 218 s = 218,000 ms. 90/218000 = 0.0413% ✓.

**Check "an order of magnitude below the suite's own run-to-run variance"** — plausible, unverifiable.

**Check "five boots back to back"... no that's aarch64.**

**Check the qemu-runner SIGTERM story:** plausible; the runner doesn't exec QEMU so the wrapper's SIGTERM hits the shell. The tsc-under-tcg.md BUGS section (before) said: "scripts/qemu-bounded.sh's killer did not fire for this lane, repeatedly: a bounded run whose output consumer exited early left qemu-system-x86_64 orphaned to launchd past its bound, five times in one afternoon..." — and the milestone says "This lane's calibration sweep orphaned an emulator on every boot until someone looked." Consistent-ish.

Now, here's a notable thing: the milestone's "Follow-on" section says: "**Recorded.** `scripts/qemu-bounded.sh`'s `BUGS` section still describes SIGKILL as the way its bound is defeated, and does not mention that a runner which does not `exec` defeats it with an ordinary TERM. The x86_64 runner is fixed here and now says so at the code; the wrapper's own `BUGS` is the place a reader would look first and is left for whoever next touches that file."

Hmm — they fixed the runner but deliberately left the wrapper's BUGS section wrong. That's a choice, recorded. Not a defect per se (it's documented as left-for-later), though one might argue it's bad practice. But the instructions say "look for a defect that no automated check can see" — a factual/logic error.

**Check notes/tsc-under-tcg.md after-change statement:** "This note recorded 4330 MHz as the worst of twenty-two boots. Milestone 526's sweep ran 490 boots at three host loads and saw +1153%, with 56 of 200 boots at load 30 wrong by more than one per cent. The one-sidedness held without a single exception across all 490. After the fix, 0 of those 200 boots are wrong by more than one per cent and the worst is +0.47%."

Wait — "After the fix, 0 of those 200 boots are wrong by more than one per cent and the worst is +0.47%." The table's min-of-16 row says worst of 200 = +0.47%, 0/200 > 1%. But the milestone says the shipped estimator is min-of-N with early stopping at agreement, mean ~5.2 windows at load 30 — and claims it "reaches exactly the accuracy of taking the full cap every time, boot for boot." So "after the fix" = early-stop estimator = full cap = 0/200, worst +0.47%. Consistent internally, given the "exactly" claim.

Hmm, but the early-stop and full-cap equivalence... the table row for 16 windows is "min over first k of them" per the milestone: "each timing sixteen windows and reporting every one, give the error of the min over the first k of them". So the 16-row is min-of-16. The shipped estimator stopped early (mean 5.2 windows at load 30), yet "boot for boot" matched min-of-16 in all 200 boots? For the worst boot, min-of-16 = +0.47%. If early stop triggered at, say, window 6 because two agreed within 0.1%, and the min of those 6 was within 0.1% of the min of 16... "exactly equal" is the claim. Plausible if clean windows agree to PIT quantization and the rule requires agreement with the min... eh. It's an empirical claim; fine.

**Now, a potentially significant inconsistency: the "+1153%" and "wrong by up to 12x".** +1153% = 12.53×. "wrong by up to 12x" — hmm, 12.53x is closer to 12.5x; "12x" is a rounding down. Not a defect per se. But wait — the section title says "wrong by up to 12x" while the body and commit message emphasize +1153% (12.5x). Also the deleted proposal was titled "can be wrong by 4x" (4330 MHz = 4.33x). OK.

Hmm wait, actually let me recheck: is +1153% really 12.53×? +1153% means increase OF 1153%, so final = 1000 + 11.53×1000 = 12,530 MHz. Yes, 12.53× the true rate. And "a 125 ms one" — 12.5 × 10 ms = 125 ms ✓.

**Check the benchmarks.md in-place correction:** "on 2026-09-21 the x86 boot calibration was found to be wrong by up to +1153%, always high, differently on every boot. An inflated rate makes ns/iter come out proportionally too small".

Check: ns = ticks / rate. Rate inflated → ns too small ✓. "so these figures may read faster than the run really was" ✓ (smaller ns/iter = looks faster).

**Check the before/after controlled pair:** "Before (one window): 1003.7 MHz, +0.37%. After (min of N): 1000.1 MHz, +0.01%." 1003.7/1000 = +0.37% ✓. 1000.1/1000 = +0.01% ✓.

**"at this host's load roughly one boot in four was wrong by more than a per cent"** — hmm! The 200-boot sweep at load 30 gave 56/200 = 28% ≈ one in 3.6. But the before/after pair was run at... ordinary session load presumably ("this host's load"). At ordinary load, what fraction of one-window boots were wrong by >1%? The 22-boot table at ordinary session load: 1001-1005 (≤0.5%) except 1042 (+4.2%). So 1 in 12 at ordinary load. Hmm, "roughly one boot in four" — where does that come from? 56/200 = 28% ≈ 1/3.6 ≈ "one in four"-ish? Hmm, 28% is closer to one in four than one in three? 1/3.57. "Roughly one in four" for 28%... eh, borderline. But "at this host's load" — the load during the controlled pair isn't stated as saturated. If it was ordinary load, then the sweep at ordinary load (30 boots, mentioned in the milestone: "At ordinary session load one window's worst of 30 boots was +773%") — how many of those 30 were >1%? Not stated. Hmm.

Actually wait, the milestone says: "Two hundred boots on a host deliberately saturated to load 30" for the table, and "Measured over the same 490 boots" for the early-stop means across three loads (200 + presumably 30 + ... hmm, 490 = 22 + ...? The tscdrift lane did 22 boots; "This lane widened that to 490 boots at three host loads". Then "Two hundred boots ... saturated to load 30" for the k-table. And "At ordinary session load one window's worst of 30 boots was +773%, and min-of-9 got to +0.01%; at load 22 the same estimators gave +318% and +0.34%." Hmm, 200 + 30 + (load-22 count) + ... = 490? 200+30 = 230; remaining 260 at load 22? Or the 490 includes the original 22. Unclear but not necessarily inconsistent.

"at this host's load roughly one boot in four was wrong by more than a per cent" — if "this host's load" = the load at which the controlled pair was run, and the pair was run at the time of writing (ordinary session load?), then the relevant stat would be from the ordinary-load sweep (30 boots). We don't know that count. Alternatively "this host's load" loosely refers to the load-30 table (56/200 = 28% ≈ one in four, rounding 28% → "one in four" is a stretch; one in 3.6. Eh).

Hmm, this is weak. Let me keep looking.

**Check the icount 37% figure:** tsc-under-tcg.md says the implied rate moved 37% between two workloads: alu ~662-672 MHz, port ~483-489 MHz. (672-483)/672 ≈ 28%? Or (662-486)/486 ≈ 36%? Depends on direction. 672/483 = 1.39 → 39%? The note says "moved by 37% between the two burns." Take midpoints: alu ~666, port ~486: 666/486 = 1.37 → 37% ✓ (relative to port) or (666-486)/666 = 27% (relative to alu). Fine, measured claim, consistent.

**The new benchmarks.md section says:** "under -icount a guest nanosecond is a function of the instruction stream rather than of real time: the tscdrift lane measured the implied rate moving 37% between two workloads inside one boot." ✓ consistent with tsc-under-tcg.md.

**"it is wrong by up to 1.8x"** in tsc note (before): 1000/561 cumulative ≈ 1.78x, or 1000/483 ≈ 2.07... they said "up to 1.8x" — hmm, that's the pre-existing text, not changed by this commit. Not in scope, and it's "up to" based on cumulative 561 MHz → 1.78x. Fine.

**Now, the milestone header:** "**Status: BUILT** 2026-09-21. ... *(Number provisional until the merge queue lands it; 524 onward is contested between in-flight branches.)*" — fine.

**"This lane widened that to 490 boots at three host loads"** vs tsc note update: "Milestone 526's sweep ran 490 boots at three host loads and saw +1153%." Consistent.

Wait, but the milestone says "Twenty-two boots of one binary stored 1001 MHz to 4330 MHz. This lane widened that to 490 boots at three host loads and the shape held without a single exception: every boot high, none ever low." And the table is "200 boots at load 30". The benchmarks.md new section says "Every one of the 490 boots measured across three host loads was high, never low." Consistent.

**Check the claim in benchmarks.md:** "An inflated rate makes ns/iter come out proportionally too small, so these figures may read faster than the run really was, by anything from a fraction of a per cent to a factor of several."

The 11 boots of the 2026-08-24 table were at ordinary session load presumably, so errors likely +0.1% to +4.2%. "a factor of several" overstates what those specific boots plausibly suffered, but since the rates weren't written down, "may" is the right hedge. Fine.

**Now let me re-examine the cancellation claim more carefully, because I think that may be the planted defect.**

In-place note: "The ticks they were derived from are unaffected, and so is every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

New section: "every conclusion in that section that rests on a ratio between two rows of one boot survives untouched, because the calibration cancels exactly in a ratio."

Commit message: "What survives untouched is every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them."

So the claim: (a) all conclusions in the 2026-08-24 section rest on ratios within one boot; (b) ratios within one boot cancel the calibration exactly.

Point (b) is mathematically true: if two ns figures from the same boot share the same stored rate, ns1/ns2 = ticks1/ticks2.

Point (a): is the debug-versus-release argument a within-boot ratio? The section ran "Six boots debug, five release." A debug-vs-release comparison must compare numbers from *different* boots (no boot is both debug and release). So the debug-versus-release ratio is a cross-boot ratio, and the calibration does NOT cancel. The stored rates on those boots are unknown (not written down) and "a different number each time" (the note itself says so!).

Wait, the note says "the rate each of these eleven boots stored was not written down, and it was a different number each time." Then two sentences later: "the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm — if the 4.2x is the debug/release ratio, then it's a ratio of a median-of-6-debug-boots to a median-of-5-release-boots. Not within a boot. The error doesn't cancel; it's multiplied by rate_release/rate_debug (unknown, but from the 22-boot sweep at ordinary load, each within ~0.1-4% of truth, so the ratio error is small, likely <5%). The claim "cancels exactly" is wrong; the honest claim is "the error doesn't cancel but is bounded by the ratio of the two boots' calibration errors, which at ordinary load was small."

Actually hold on, let me reconsider whether the debug-vs-release 4.2x could be within-boot. Is there any way? Could the section have booted a kernel that contains both debug and release benches? No — debug vs release is a build profile for the whole kernel. A single boot is one build. So no.

Alternatively, maybe "the debug-versus-release argument" doesn't involve ns at all — maybe it was argued in ticks (e.g., "release retires 4.2x fewer instructions"?). But ticks across boots are fine (ticks are raw rdtsc, unaffected). But then the justification "the calibration cancels exactly in a ratio" would be the wrong reason — ticks need no cancellation. Hmm. The note explicitly says "where the calibration cancels exactly," implying ns-based ratios.

Hmm, wait. Let me re-read once more: "The ticks they were derived from are unaffected, and so is every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

So "the debug-versus-release argument" is claimed to be "a ratio within a boot." Given six debug boots and five release boots, the debug-vs-release comparison cannot be within a boot. Unless the section somehow did per-boot debug/release... impossible.

Unless... hmm, wait. Maybe the 2026-08-24 section's table compared debug and release *medians*, and the "argument" was about ratios like "debug is 4.2x release" — a cross-boot ratio. If the calibration errors on those 11 boots are unknown and different, then the 4.2x is NOT exactly preserved. However — is the 4.2x conclusion *materially* threatened? At ordinary load, errors were mostly sub-1%, one boot +4.2%. A 4.2x ratio with ±few% error → still ~4.1-4.3x. The qualitative conclusion survives. But the stated *reason* ("cancels exactly") is wrong.

Is this "the defect hiding here"? It's plausible. The prompt says "A defect may be hiding here that no automated check can see." A mathematical/logical error in a design record fits.

But wait, I should double-check the alternative interpretation: maybe the 2026-08-24 section reported, per boot, BOTH debug and release? "Six boots debug, five release" — no. Each boot is one build.

Hmm, but actually — maybe the "debug-versus-release argument" is a ratio of two rows where the rows are "debug median" and "release median," and the author considers the medians as... no, medians are across boots. The calibration error does not cancel in a ratio of medians from different boots with different wrong rates. Period.

Let me also reconsider: is it possible that the section's conclusions are things like "tss_iomap_switch costs 4.2x yield_switch" — a within-boot, within-build ratio — and the "debug-versus-release argument" is something like "we quote release because debug is distorted," argued from... hmm. Without the section text, I'll frame the concern conditionally but firmly: a debug-vs-release ratio computed from separate debug and release boots is not a within-boot ratio, so the claim of exact cancellation is wrong; what actually protects the 4.2x is that the ordinary-load calibration errors were small (though the specific 11 boots' rates were never recorded, so even that can't be asserted for those boots — only inferred from the later 22-boot sweep).

Actually hmm, wait. Let me reconsider once more. Maybe I'm wrong about the structure of that section. The visible line: "QEMU 11.0.2 (.qemu-version, pinned), -machine q35 -cpu max, one hart, plain TCG. Six boots debug, five release; ns/iter computed from the guest's own calibrated TSC, the same arithmetic xtask's run_bench already does for every other leg."

So yes, 11 boots, 6 debug + 5 release. The debug-vs-release argument compares across boots. The claim of exact cancellation is false for it. I'm fairly confident this is a real concern.

Hmm, wait, but let me reconsider whether "two rows of the same run" might mean "two rows of the same table/section" rather than "same boot." "Run" — in this repo's idiom, "run" seems to mean an execution/boot. The commit message says "two rows of one boot." So the intended meaning is clear: same boot. And the debug-vs-release ratio isn't one.

OK so that's candidate defect #1.

**Now let me look for others.**

**The stopping rule detail:** "stops as soon as two of them agree" — header says "keeps the smallest, stops as soon as two of them agree." Fine.

**"One descheduling of the QEMU thread inside that window inflates the answer without bound: a 4330 MHz reading is a 43 ms window reported as a 10 ms one, and the worst this lane saw, +1153%, is a 125 ms one."** ✓.

**"Ten milliseconds per window is real time on every boot... Paying it sixteen times"** — "Sixteen windows would be a 160 ms boot tax" ✓.

**The table "Mean calibration cost": 3.53 → 35 ms, 4.77 → 48 ms, 5.20 → 52 ms.** ✓ (10 ms each, rounding).

**"calef's five is what this costs; sixteen is only what it is willing to spend"** — the mean at saturated load is 5.20 windows ≈ five. Cute. Consistent.

**"A tighter tolerance (one part in 2000) moved the mean by 0.07 windows and the error distribution not at all"** — unverifiable, fine.

**"`script/test --arch x86_64` boots the kernel four times, and those four boots took 3, 3, 4 and 3 windows. So the suite pays 90 ms more than the one-window design did"** — old design: 1 window = 10 ms per boot, 4 boots = 40 ms. New: (3+3+4+3)=13 windows = 130 ms. Delta = 90 ms ✓.

**"against a 3m38s runtime: 0.04%"** ✓ (90/218000 = 0.0413%).

**BUGS: "0 of 200 boots at load 30 were wrong by more than 1%"** ✓ matches table.

**"an accuracy gate costs a thirty-five-second suite leg"** — tsc_probe takes "thirty-five seconds of wall clock" per the note ✓ ("the question it answers takes thirty-five seconds of wall clock"). ✓.

**Wait — the repro section says `qemu-bounded.sh 55` and `75` seconds for the probe.** The probe spans 32-second windows (plain) and 48 s (icount). "thirty-five seconds a boot" refers to the plain probe. OK consistent.

**The local APIC timer BUG:** "The local APIC timer's rate gets the same treatment and has no CPUID escape. It is the minimum over the same windows, taken independently of the TSC's." Hmm — "the minimum over the same windows, taken independently" — so APIC min is computed separately over the same windows ✓ plausible.

But wait — the proposal's open question was "Whether the local APIC timer's rate should be derived from the same windows." The Follow-on says "its two open questions are both answered above (the boot does print its confidence, and the local APIC timer is derived from the same windows)." And the BUGS entry says it is "the minimum over the same windows, taken independently of the TSC's." Consistent-ish ("derived from the same windows" vs "minimum over the same windows, taken independently"). OK.

**Now the milestone says the boot line prints "best of N windows, worst M MHz"** and main.rs prints it. Fine.

**Notes/tsc-under-tcg.md change:** "The boot now times several windows and keeps the smallest... it stops as soon as two windows agree to one part in a thousand, so the mean cost is 3.5 windows on a quiet host" ✓ consistent with 3.53.

"After the fix, 0 of those 200 boots are wrong by more than one per cent and the worst is +0.47%." ✓.

**Wait, there's a subtle one in tsc-under-tcg.md:** "The numbers in this section are the defect, and they got worse when measured harder. This note recorded 4330 MHz as the worst of twenty-two boots. Milestone 526's sweep ran 490 boots at three host loads and saw +1153%..." — fine.

**Now, "the note... marks the affected table where a reader meets it rather than quietly restating it"** — commit message, fine.

**Let me check the in-place correction blockquote formatting:** The blockquote ends with "> `kernel/src/arch/x86_64/timer.rs`'s `BUGS`." then blank line then "QEMU 11.0.2...". Fine.

**New section heading level:** "## 2026-09-21: the x86 boot calibration was wrong by up to 12x, and what that does to the numbers above" — appended at end of file after a section ending "...teaches people to route around it." There's no blank line issue? The diff shows:

```
 teaches people to route around it.
+## 2026-09-21: ...
```

Directly appended with no blank line between "teaches people to route around it." and the new "##" heading. In Markdown, a heading immediately after a paragraph line without a blank line... Most renderers (CommonMark) actually require... no, CommonMark allows ATX headings to interrupt a paragraph? Let me recall: In CommonMark, an ATX heading *can* interrupt a paragraph (unlike setext?). Actually yes — CommonMark spec: ATX headings can interrupt paragraphs (no blank line needed). Setext headings cannot. So `text\n## heading` renders as paragraph then heading. GFM same. So not a rendering bug in CommonMark. But some stricter parsers (e.g., mdbook? which uses pulldown-cmark = CommonMark) handle it fine. So not a strong concern; stylistic at most. Also the original file might have a trailing newline issue — the diff shows the last line "+..." ends without explicit newline marker. Fine.

Hmm wait, actually, look more carefully at the end of the diff for benchmarks.md:

```
@@ -3781,3 +3794,84 @@ mirror image: a row that moved 26.4% while the code was byte-identical.
 already carries `x86_trap`, is already 152 bytes, and whose gate is green with room (+1.0%, +1.4%,
 +3.9%). The exemption is measured; asserting a property the tree does not hold is how a gate
 teaches people to route around it.
+## 2026-09-21: the x86 boot calibration was wrong by up to 12x, and what that does to the numbers above
```

So yes, no blank line before the new `##`. CommonMark allows ATX heading interrupting a paragraph — renders as heading. OK, not a defect (though sloppy style for a docs-as-deliverable repo).

**Now, the title "wrong by up to 12x"** vs "+1153%" = 12.53×. Hmm, and the section body says the worst is +1153%. 12.53 rounds to 12.5, not 12. "wrong by up to 12x" understates slightly. Actually hold on, is there another reading where "wrong by 12x" means "the rate was 12x the truth"? 12.53x ≈ 12x, fine-ish. Or does "wrong by Nx" mean the error is N×100%? Then +1153% = wrong by 11.53x ≈ 12x. Either way ≈ fine. Not a defect.

**Check "the `tscdrift` lane measured the implied rate moving 37% between two workloads inside one boot"** ✓.

**Check statement: "Every section of this file that quotes x86 numbers should say which one it used, and from here on they do."** — "from here on they do" — only this new section exists "from here on" (it's appended at the end). It does say (it discusses --real vs icount explicitly). OK.

**Check: "The 2026-09-15 lazy-TSS section and the icount baselines are not affected. They quote deterministic ticks under -icount and say so; bench/baseline-*.txt holds ticks."** — plausible; the section right before the new one (the tail we can see: "...green with room (+1.0%, +1.4%, +3.9%)") appears to be about a footprint gate... hmm, that's about bytes (152 bytes), maybe the lazy-TSS section. Can't verify fully. OK.

**"coremark's self-reported rate on x86 read the stored number, so any x86 CoreMark score computed from it carried the same error. No such score is published; the 2026-09-21 section above records that CoreMark 'reports correctness, not yet a score'."** — Wait: "the 2026-09-21 section above records that..." The quoted phrase "reports correctness, not yet a score" — in the visible (pre-change) text, the CoreMark paragraph says: "This binary reports correctness, not yet a score; timing a run needs a userspace clock..." That's in the milestone-19e section ("Compute vs. OS primitives"), not dated 2026-09-21 explicitly... Actually there might be a later 2026-09-21 coremark section in the truncated part. The quote matches the visible 19e text. The attribution "the 2026-09-21 section above" — hmm, there are two other 2026-09-21 sections visible: "The floors name the nightly..." and "The emulator is the other half..." Neither mentions CoreMark. But there could be a CoreMark section dated 2026-09-21 in the truncated 1200 lines. Actually wait — the quote "reports correctness, not yet a score" appears verbatim in the 19e section, which is undated in its heading ("Compute vs. OS primitives: two benchmarks that measure different things (milestone 19e)"). Hmm, but it's plausible a later dated section repeated it. Can't verify; weak.

Hmm OK wait, actually — let me reconsider. There's something potentially interesting: the new section says "the 2026-09-21 section above records that CoreMark 'reports correctness, not yet a score'". Since the file was truncated, I can't confirm. I'll not press this.

**Now the qemu-bounded/qemu-runner narrative:** "qemu-runner-x86_64.sh is the only one of the three runners that does not exec QEMU, because it has to translate isa-debug-exit's status afterwards... The shell died; QEMU was reparented to pid 1 and ran forever." Then: "The runner now runs QEMU in the background, traps TERM and HUP, forwards them, and reaps. It also needed qemu-bounded.sh's own fd-3 dance, because backgrounding a command gives it /dev/null on stdin under dash and would have broken every piped-input run in CI."

Hmm, "backgrounding a command gives it /dev/null on stdin under dash" — actually, backgrounding with `&` in a non-interactive shell gives /dev/null stdin only when job control is off... In dash, async commands get stdin redirected from /dev/null when job control is disabled (POSIX: "If job control is disabled... the standard input for an asynchronous list... shall be redirected from /dev/null"). Yes, POSIX says: asynchronous lists with job control disabled get stdin = /dev/null. ✓ technically accurate.

**Now — the tsc-under-tcg.md note's BUGS section (unchanged by this commit) said:** "scripts/qemu-bounded.sh's killer did not fire for this lane... This note is not where that gets fixed; see the lane's report and fix/qemu-bounded-stdin-under-dash, which is a different symptom of the same script." The milestone says the runner is fixed in this commit's lane. The note's BUGS entry remains (unchanged). Is that a stale cross-reference? The note's BUGS says "This note is not where that gets fixed; see the lane's report and fix/qemu-bounded-stdin-under-dash." The milestone fixes the runner and says the wrapper's BUGS is left stale deliberately. The note's BUGS entry about orphaned QEMUs remains accurate as history. Fine.

**Now, a big one to double-check: the milestone's "Index row" and "Follow-on" — the repo convention is that a milestone also adds an index row to some index file?** The milestone file itself contains a "## Index row" section, presumably to be pasted into an index. The commit doesn't modify any index. Hmm — is there a `design/roadmap/index.md`? I don't know. The milestone says "Index row" with "Built: 2026-09-21" — maybe the convention is the index is updated separately (merge queue). Can't assert.

**Let me re-examine the "eleven boots" and "each stored a different wrong rate and none was written down" vs the correction note's "it was a different number each time."** 11 boots; the 22-boot sweep showed repeats (1001, 1001, 1002, 1002...). "A different number each time" is loose (not literally true that each of the 11 differed — rates weren't written down at all, so they can't know). Commit message: "each of those eleven boots stored a different wrong rate and none was written down" — they can't *know* each was different; it's unknowable. Minor rhetorical overreach. Weak.

**Now — the most promising: check the math claim "the calibration cancels exactly in a ratio" applied to debug-vs-release. Also check "the 4.2x".**

Actually, wait. Let me look at the visible fragment of benchmarks.md for any "4.2x": The visible pre-change text mentions "~6.7x" debug-to-release tax on IPC (aarch64 cross-OS section) and "4.2x"... I see "read 2026-08-13"... "an IPC round trip in the low hundreds"... I don't see "4.2x" in the visible portion. The 2026-08-24 x86 section is in the truncated region (line 2664 is where the correction is inserted — wait, line 2664 is in the *after* numbering; the before file's line ~2664 area). The visible before-content shows sections up to ~2026-08-17 then truncated, then the tail (line 3781-3784 before) shows a fragment about "x86_trap ... 152 bytes ... +1.0%, +1.4%, +3.9%". So the 2026-08-24 x86 section is in the truncated middle. I can't read its exact arguments.

Given that, the strongest *verifiable-from-the-bundle* inconsistency regarding the cancellation claim: The note states the 11 boots each stored a different unknown rate, and the debug-vs-release comparison is necessarily across those boots; "cancels exactly" is therefore not true of the debug-versus-release argument. Unless the section's debug-vs-release argument was per-boot... which is impossible. So either the claim "that is all of them [within-boot ratios]" mischaracterizes the debug-vs-release argument, or the argument was never a within-boot ratio. Either way the record overstates what's salvaged: the debug-vs-release ratio survives only approximately (bounded by the ratio of two unknown calibration errors), not "exactly."

Hmm, but hold on — maybe the debug-versus-release argument in that section was structured as: within *each build*, compare two benches; then argue debug vs release... I can't know. But the natural reading of "the debug-versus-release argument" is "release is 4.2x faster than debug" — a cross-boot ratio. And the claim that it "cancels exactly" is wrong. Even the commit message says "every conclusion resting on a ratio between two rows of one boot... and that is all of them" — which is a false characterization if the debug-vs-release 4.2x is among the section's conclusions.

Actually — wait. Let me reconsider. Is it possible that in the 2026-08-24 section, each boot produced both a debug and release measurement? Like, the bench kernel could... no. Debug vs release is a compile-time profile of the whole kernel image. One boot = one build. Six boots debug, five release. A debug-vs-release ratio is across boots. Certain.

So the error: "the calibration cancels exactly in a ratio" only holds when both figures share one stored rate (one boot). The debug-vs-release 4.2x compares a median of six debug boots to a median of five release boots, each boot carrying its own unknown wrong rate. The errors don't cancel; at best they're small. The correction note's claim of exact cancellation for "the debug-versus-release argument" is mathematically wrong, and it matters because the note's purpose is to state precisely what is and isn't safe to quote; a reader would now quote the 4.2x as exact when its denominator and numerator each carry an unrecorded, up-to-several-percent (potentially worse) calibration error.

Hmm, though — is it "up to several percent"? The 11 boots were presumably at ordinary load, where the sweep showed +0.1% to +4.2%. The ratio error would be up to ~±4%. A 4.2x ratio with ±4% error → 4.03–4.37x. The qualitative conclusion stands, but "cancels exactly" is wrong.

Wait, actually, hmm: could the debug-vs-release comparison have been done within single boots?? Like, boot a debug kernel that ALSO contains a release-compiled bench? No, nonsense.

OK here's another thought — maybe the 2026-08-24 section's "debug-versus-release argument" was argued from the *icount* baselines or from ticks, not ns. The in-place note says "The ticks they were derived from are unaffected" — if the debug-vs-release argument was made in ticks, it's unaffected — but then saying "the calibration cancels exactly in a ratio" is the wrong justification (nothing needs cancelling). Either way, the note's reasoning is muddled. But the strongest statement I can make: a cross-boot ns ratio does not cancel the calibration, and the note incorrectly claims it does.

Hmm, hold on, hold on. Let me reconsider — maybe the section's debug and release numbers came from the SAME boots?? "Six boots debug, five release" — no. Definitively separate.

Hmm, wait, actually — unless the debug-vs-release comparison was done *per boot count*... no. Moving on. I'm fairly confident in concern #1, with the caveat that I can't read the original section.

**Let me now scrutinize the milestone for other checkable math/logic.**

"**The error being one-sided is the whole of the fix.** Every window is an upper bound on the true rate, so the minimum of several is the tightest bound taken, and it converges on the truth from above. An average would be a biased estimator for precisely the reason the minimum is an unbiased one..."

Hmm — "the minimum is an unbiased one"? The minimum of upper bounds is still an upper bound — it's *biased upward* (strictly ≥ truth, = truth only if a window had zero descheduling delay, and even then there's PIT quantization + poll latency, which is also one-sided late). The minimum is *not* unbiased; it's a lower-bias estimator than the average, or "consistent" (converges to truth from above as windows → ∞ under some conditions). The text itself says "converges on the truth from above" — converging from above means it's always above, i.e., biased high! Calling it "an unbiased estimator" contradicts the preceding sentence within the same paragraph. And the proposal (deleted) said it correctly: "the minimum of several windows converges on the truth from above... the minimum is the right [estimator]" — it avoided "unbiased." Also "An average would be a biased estimator" — yes the average is biased (carries every descheduling). But the minimum is also biased, just less so. The correct statement: both are biased (upward); the minimum has bias that shrinks toward zero with more windows and discards large errors, whereas the mean's bias includes every sample's inflation.

Hmm wait, let me re-read the exact sentence: "An average would be a biased estimator for precisely the reason the minimum is an unbiased one: it carries every descheduling into the answer instead of discarding it."

So it claims minimum is unbiased. But the same section says "Every window is an upper bound on the true rate, so the minimum of several is the tightest bound taken, and it converges on the truth from above." An estimator that is always ≥ the truth and converges from above is biased (upward), with bias → 0. "Unbiased" is flatly wrong terminology in a document that elsewhere prides itself on precision ("the shape that names the mechanism"). And even the cleanest window is late by one poll-loop iteration (the tsc note says the poll cost is 165–1031 counts — wait, that's the RTC poll in the probe; the PIT poll similarly is late by a poll iteration), so even the minimum is inflated by at least the poll granularity. So the minimum is *not* unbiased; it's a one-sided convergent estimator. That's a genuine technical error in the record. It matters because the document's core argument is about estimator choice, and it mislabels the chosen estimator's property — a reader could repeat the claim ("our calibration is unbiased") and be wrong; the true claim is "biased but one-sided and convergent, with bias bounded by the best window's lateness."

Hmm wait, but is this "the" defect or is the cancellation one? Both seem real. Let me keep going and then rank.

Also: "for two windows to land within a thousandth of each other they must both have been left alone, because two independent deschedulings agreeing to three decimal places is not a thing that happens."

"one part in a thousand" ≈ 0.1%. "three decimal places" — hmm, one part in a thousand is *one* decimal place in percent terms (0.1%) or three in fraction terms (0.001). "Agreeing to three decimal places" — if rates are ~1000 MHz, agreement within 1 MHz = agreement in the fourth significant digit... "three decimal places" is ambiguous but roughly 0.001 relative. Eh, loose but not clearly wrong. Skip.

**Check "the 99th percentile" of 200 boots:** the 99th percentile of 200 samples is around the 198th–199th order statistic; +884% vs worst +1153% — plausible.

**Check the milestone's "The defect, which another lane found..." section:** "Milestone 524 ... had just argued that a non-invariant TSC 'cannot be caught by measuring harder'." Then the tscdrift lane... "found that it does not [drift]: the counter ticks at exactly 1000.000 MHz, constant to within 42 ppm." Consistent with the note.

**"The `tscdrift` lane then went to check whether TCG's TSC drifts at all, built an RTC-referenced probe, and found that it does not"** ✓.

**Milestone title/number:** 526; references to milestone 524's gates and 161's CPUID path, 115's posture, 21's bench discipline... The Follow-on references "the proposal this promotes ... is removed by this milestone" — and indeed the commit deletes it. ✓.

**Hmm, wait — the Follow-on says "Done. The proposal this promotes, design/roadmap/proposals/the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md, is removed by this milestone: its two open questions are both answered above (the boot does print its confidence, and the local APIC timer is derived from the same windows)."**

The proposal's two open questions were: (1) whether the boot should say how confident it is; (2) whether the local APIC timer's rate should be derived from the same windows. ✓ both addressed.

**Now check the deleted proposal's title: "can be wrong by 4x"** — 4330 MHz = 4.33x ✓.

**Check the new benchmarks.md section's claim about `wait_for`:** "kernel::user::wait_for takes now() plus two seconds, so an inflated rate makes a timeout longer in real time, never shorter. The defect could only ever fail safe."

Hmm — wait. Is that right? wait_for(now + 2s): the deadline is computed in ticks: deadline_ticks = now_ticks + 2s × rate. If rate is inflated (stored rate > true rate), then deadline_ticks is *larger* than it should be, so the timeout fires later in real time (longer). ✓ "longer in real time, never shorter" ✓ fail safe ✓. Consistent with tsc note. ✓.

But hmm — one more: is it really true that *every* use of the rate fails safe? Consider computing how long something took (uptime, Instant): inflated rate → elapsed real time *under*-reported (ticks/rate too small). Not a safety issue, just wrong numbers. The note says reported numbers don't fail safe. ✓.

**Check the "twenty-two boots" numbers in the milestone vs note:** Note says ordinary load: 1001...1042 (12 boots), saturated: 1003...4330 (10 boots) = 22 ✓. Milestone: "Twenty-two boots of one binary stored 1001 MHz to 4330 MHz" ✓.

**Check "+333%":** in the note (before): "the stored rate ranges from +0.1% to +333%" — 4330/1000 = 4.33 → +333% ✓. The commit message says "4330 MHz over twenty-two boots became +1153% over 490" ✓.

**Now the "0.04%" and other derived claims — done.**

**"That is a computed figure and deliberately not a measured delta, because there is nothing to measure a 90 ms change against."** Hmm — "nothing to measure it against"? You could measure suite wall time before/after... but 90 ms against 218 s with run-to-run variance >> 90 ms means unmeasurable in practice ✓ reasonable.

**BUGS: "The cap was chosen against one machine. Every number here is patagonia, an eight-core Apple Silicon host running TCG."** ✓ consistent with note (patagonia = M-series, 8 cores). The note's table says "16 spinners" for saturation; milestone says "saturated to load 30 on eight cores" ✓ consistent (load average rose to 29 per the note; "load 30" ≈ fine).

**Hmm, "load 22":** milestone says "at load 22 the same estimators gave +318% and +0.34%". Note says saturated run had "load 10 -> 29". The 490-boot sweep used loads... "three host loads": ordinary, 22, 30? The note's saturated run was "16 spinners, load average 10 rising to 29". Fine, different runs. OK.

**Now check "The median is +0.00% at every count from three upward"** — table ✓.

**Check the stopping rule vs the table's "min over first k":** the shipped rule stops early; the table's 16-row is min-of-16. The claim "Measured over the same 490 boots, this reaches exactly the accuracy of taking the full cap every time, boot for boot."

Hmm, "exactly... boot for boot" — let me think about whether this can be literally true. Consider a boot where windows: w1 clean (1000.2), w2 clean (1000.1) → agree within 0.1%? |1000.2-1000.1|/1000.1 = 0.01% < 0.1% → stop at 2. min = 1000.1. Full-16 min might be 1000.0 (a slightly cleaner later window). Different answers! For "exactly equal" to hold for all 490 boots, clean windows must all quantize to the same value (PIT quantization: 10 ms window at 1 GHz = 10,000,000 counts; PIT at 1.193182 MHz, 10 ms = 11932 PIT ticks → quantization 1/11932 ≈ 0.0084% — so clean windows can differ by ~1 PIT tick ≈ 0.008%, and poll jitter similar). Two windows differing by one PIT tick agree within 0.1% → stop. A later window could be one tick lower → min-of-16 slightly lower than early-stop answer. So "exactly equal, boot for boot" is surprising. Unless the implementation's stopping rule is "stop when a window agrees with the current min within 0.1%" and... even so. Or unless "exactly the accuracy" means "the same error distribution to the precision reported" rather than bit-equal. The phrase "boot for boot" though claims per-boot equality. This is an over-strong claim that I can argue is almost surely false as stated... but I can't *prove* it false from the bundle, because the implementation might define "agreement" relative to the running min in a way that... no wait, even then a later window could beat the min by less than... hmm, if later window is lower than min by any amount, then min-of-16 < early answer. For equality always, clean windows must be bit-identical — possible if QEMU's PIT emulation delivers exactly 11932 ticks per 10 ms and the poll catches it with identical latency... Under TCG with instruction-driven virtual time? Wait — plain TCG: PIT driven from QEMU_CLOCK_VIRTUAL which is host clock in realtime mode... The TSC is host nanoseconds; the PIT counts at fixed 1193182 Hz in virtual time. A clean 10 ms window = exactly 10,000,000 TSC ns ± poll latency. Poll latency varies (165–1031 counts per the probe's RTC poll; the PIT poll similar magnitude). So clean windows differ by poll latency ~ hundreds of counts = ~0.005% — within the 0.1% tolerance, but NOT bit-identical. So min-of-16 would usually be slightly lower than the early-stop answer. "Exactly the accuracy... boot for boot" seems false in the strict sense.

But is that "the defect"? It's an empirical claim about their own measurement ("Measured over the same 490 boots, this reaches exactly the accuracy..."). If their measured data really showed exact per-boot equality, then maybe their windows quantize identically (e.g., the poll reads the PIT's output bit which flips at PIT tick boundaries, so each clean window = N PIT ticks exactly converted... the conversion: window_ms = pit_ticks/1193.182; TSC counts in window vary by poll latency though). Hmm, actually the error in the chosen rate from poll latency in a clean window is the *detection* latency — the TSC is read after the output goes high, late by poll iteration. That varies per window. So min-of-16 ≤ early-stop min, strictly less whenever a later window has shorter poll latency. For 490 boots all equal — implausible. But again, "accuracy" might be defined coarsely (to 0.1 MHz, say). The phrase is "reaches exactly the accuracy of taking the full cap every time, boot for boot" — maybe "the accuracy" = the reported MHz value rounded to integer MHz, and poll latency ~hundreds of ns over 10 ms = ~0.005% = 0.05 MHz — rounds to the same integer MHz usually but not always. Ugh, can't prove.

I'll note this as a weaker concern or fold it in. Actually, I think the two strong concerns are:

1. The "cancels exactly in a ratio" claim applied to the debug-vs-release argument (cross-boot ratio — does not cancel; only within-boot ratios cancel). And the commit message itself asserts "that is all of them."

2. "The minimum is an unbiased estimator" — self-contradictory with "converges on the truth from above" in the same paragraph; the minimum of one-sided upper bounds is still biased high (every window is late by at least a poll iteration), just much less biased than the mean. Terminology error in a doc whose core argument is estimator choice.

Let me look for more.

**Benchmarks.md new section: "Every section of this file that quotes x86 numbers should say which one it used, and from here on they do."** Fine.

**"The 2026-08-24 x86 ns/iter table (yield_switch, tss_iomap_switch, debug and release) is the one body of published x86 wall-clock figures derived from the stored rate."** Hmm — is it really the *only* one? The file's earlier visible text: any other x86 wall-clock figures? The visible aarch64 figures are HVF (aarch64, unaffected). The truncated region may contain other x86 --real figures... The claim "the one body of published x86 wall-clock figures" — can't verify, but the commit message says "Every x86 wall-clock figure this tree published was derived from the one-window calibration, so notes/benchmarks.md marks the affected table." Hmm, the commit message says every published x86 wall-clock figure was affected, and the section says the 2026-08-24 table is "the one body" of them. Consistent with each other. Can't falsify.

Wait, actually — what about the icount x86 baselines `bench/baseline-x86_64.txt`? Those are ticks, addressed ("not affected"). OK.

**Now the title says "wrong by up to 12x" but the correction note at the table says "+1153%" and the deleted proposal said "4x" (for 4330). The new section title: "wrong by up to 12x". +1153% = 12.53×... "12x" rounds down from 12.5. Pedantic. Skip.**

**Milestone: "QEMU's TCG does not offer the leaf under any invocation tried"** ✓ consistent with note ("TCG refuses to advertise CPUID.80000007H:EDX[8] under any invocation" — wait, that's the invariant-TSC bit, leaf 0x80000007. Leaf 0x15 is the crystal frequency. The note's line is about 80000007H. The milestone's claim is about leaf 0x15/0x16. Different leaves; both plausible; not contradictory).

**"arch::x86_64::isa::tsc_crystal_hz already reads it and init_frequency already prefers it"** — code claim, can't verify, fine.

**"The CMOS RTC ... Its seconds register has one-second granularity, so calibrating against it means either waiting for a seconds edge (up to one second of boot, a hundred times this fix's cost)"** — hmm: "a hundred times this fix's cost". The fix costs ~35-52 ms mean, cap 160 ms. Waiting up to 1 s = 1000 ms. 1000/35 ≈ 28×; 1000/10 (one window) = 100×. "a hundred times this fix's cost" — this fix's mean cost is ~50 ms, so 1 s is ~20×. Hmm, "a hundred times" seems off unless compared to a single 10 ms window. Let me re-read: "waiting for a seconds edge (up to one second of boot, a hundred times this fix's cost)". If "this fix's cost" = mean 35–52 ms, then 1000 ms is 20–28×, not 100×. If the comparison is to a 10 ms window, it's 100×. Also "then spanning thirty-two seconds" for tsc_probe — the probe waits for an edge then spans 32 s; calibrating at boot that way costs 33 s ≈ 1000×. Hmm. The sentence: "calibrating against it means either waiting for a seconds edge (up to one second of boot, a hundred times this fix