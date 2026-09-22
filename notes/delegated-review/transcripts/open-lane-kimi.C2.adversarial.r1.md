[no text block returned; the model spent its budget on thinking]

Let me carefully review this commit. The task is to review a proposed change to the nife repository. The commit is documentation-only (Markdown design records and notes). The changed files:

1. `design/roadmap/526-the-x86-tsc-calibration-takes-the-smallest-of-several-windows.md` (new)
2. `design/roadmap/proposals/the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md` (deleted)
3. `notes/benchmarks.md` (modified)
4. `notes/tsc-under-tcg.md` (modified)

I need to look for a defect that no automated check can see — likely an internal inconsistency, a factual/logical error in the prose, a contradiction between documents, a wrong cross-reference, etc.

Let me read carefully.

First, the commit message:

"Record the calibration fix: milestone 526, the note, and the suspect numbers

Every x86 wall-clock figure this tree published was derived from the one-window calibration, so notes/benchmarks.md marks the affected table where a reader meets it rather than quietly restating it. Nothing can recover the true values: each of those eleven boots stored a different wrong rate and none was written down. What survives untouched is every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them.

The new dated section there also separates the two instruments that wear one name: script/bench --x86 pins the virtual clock and --real does not, and only the latter reads the calibrated rate.

notes/tsc-under-tcg.md's finding is marked fixed, with the correction that measuring harder made the defect worse: 4330 MHz over twenty-two boots became +1153% over 490.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01STu3VeDYnEHem3iKCbzBF2"

Wait — the commit says "the calibration cancels exactly in a ratio, and that is all of them." Hmm, is that correct? Let's think about the math.

The calibration defect: the stored TSC rate is inflated by factor (1+e), where e ≥ 0. So stored rate R = 1000 MHz × (1+e). The kernel computes time = ticks / R. So `ns/iter` = ticks_per_iter / R. If R is inflated by (1+e), then ns/iter is underestimated by factor (1+e). So an inflated rate makes ns/iter come out too small. The note says that. OK.

Now, a ratio of two ns/iter figures from the same boot: ns1/ns2 = (ticks1/R)/(ticks2/R) = ticks1/ticks2. So the calibration cancels. That's true — as long as both figures are derived using the same stored rate within one boot. OK, that's fine.

Now let's check the numbers' internal consistency.

The proposal said "4330 MHz is a 43 ms window reported as a 10 ms one." Check: true rate 1000 MHz → in 10 ms, TSC advances 10^7 counts. If the window actually lasted 43 ms (because of a descheduling noticed late), the TSC delta is 43×10^6 counts, attributed to 10 ms → rate = 4330 MHz. Yes, 43 ms / 10 ms = 4.33, so 4330 MHz. Consistent.

The new milestone says "the worst this lane saw, +1153%, is a 125 ms one." Check: +1153% means rate = 1000 × (1 + 11.53) = 12,530 MHz. That corresponds to a window of 12.53 × 10 ms = 125.3 ms. Yes, consistent: "125 ms one." OK.

Also "+1153%" vs the heading "the x86 boot calibration was wrong by up to 12x" in benchmarks.md new section: "## 2026-09-21: the x86 boot calibration was wrong by up to 12x, and what that does to the numbers above". 12.53x ≈ 12x. OK.

Wait, but the note tsc-under-tcg.md says "the stored rate ranges from +0.1% to +333% of the truth" — 4330/1000 = 4.33, so +333%. Yes.

Now the new milestone 526 document. Let me read it in detail.

"The x86 TSC calibration takes the smallest of several windows. Status: BUILT 2026-09-21. The boot times several 10 ms PIT windows instead of one, keeps the smallest, stops as soon as two of them agree, and prints the worst beside the chosen one so a calibration the host fought is visible rather than silently confident."

OK.

"## The defect, which another lane found while looking for something else

Milestone 524 (the three x86_64 boot gates: NX, SYSCALL, and the invariant TSC) had just argued that a non-invariant TSC "cannot be caught by measuring harder". The `tscdrift` lane then went to check whether TCG's TSC drifts at all, built an RTC-referenced probe, and found that it does not: the counter ticks at exactly 1000.000 MHz, constant to within 42 ppm whatever the host is doing.

What it found instead was that the number the boot **writes down** had almost nothing to do with that. Twenty-two boots of one binary stored 1001 MHz to 4330 MHz. This lane widened that to 490 boots at three host loads and the shape held without a single exception: **every boot high, none ever low.**"

Hmm, "every boot high, none ever low" — is that plausible? The original 22 boots: 1001 to 4330 MHz vs 1000.000 MHz true. All ≥ 1001. The claim is that across 490 boots, none ever measured below 1000.000 MHz. Since the error is one-sided (polling can only notice the terminal count late), the measured rate is always ≥ true. Well, actually is it strictly always high? The minimum possible measured value is exactly the true value (if the poll notices exactly at the terminal count — impossible, there's always some poll latency). But with quantization, maybe you could get exactly 1000. The claim "every boot high, none ever low" is the signature of one-sidedness. Plausible, but "none ever low" might include exactly-equal. Minor. Actually wait — could a boot measure *low*? The TSC delta over the window: TSC ticks at exactly 1000 MHz; PIT window is 10 ms. The poll reads port 0x61 until output goes high. The TSC is read before arming and after noticing. The delta ≥ true 10 ms worth. So rate ≥ true. But measurement of the PIT: the PIT in QEMU is driven from QEMU_CLOCK_VIRTUAL... hmm, the note says the PIT is disqualified as a reference because it's driven from the same clock as TSC. Interesting. Actually the PIT calibration is circular in a sense — but wait. In QEMU, the PIT is driven from QEMU_CLOCK_VIRTUAL, and the TSC under TCG on aarch64 host is host monotonic nanoseconds. Are those the same? QEMU_CLOCK_VIRTUAL under non-icount runs in real time (host clock). So both track host nanoseconds, so the PIT 10 ms is a true 10 ms. OK.

But actually there's a subtlety: the measured rate could be low if the PIT's 10 ms window is itself longer than 10 ms in real time? If the PIT runs on virtual time that lags real time... under TCG without icount, virtual time runs at real time. OK, fine. These are the document's own premises; not our concern.

Let me look for actual inconsistencies.

"## calef's ruling named the shape; the count was measured

> "Fix the calibration with min-of-five windows." (2026-09-21)

The brief for this lane said not to take five as given, and that was the right instruction: **five would have been the wrong number to ship.** Two hundred boots on a host deliberately saturated to load 30 on eight cores, each timing sixteen windows and reporting every one, give the error of the min over the first k of them:

| Windows | Median error | 99th percentile | Worst of 200 | Boots wrong by >1% |
|---|---|---|---|---|
| 1 (what shipped) | +0.36% | +884% | **+1153%** | 56 / 200 |
| 3 | +0.00% | +120% | +131% | 24 / 200 |
| 5 | +0.00% | +20.6% | +51.4% | 11 / 200 |
| 9 | +0.00% | +0.49% | +7.1% | 2 / 200 |
| 16 | +0.00% | +0.01% | **+0.47%** | **0 / 200** |

Five still leaves one boot in eighteen wrong by more than a per cent and a worst case of +51%."

Check: 11/200 = 1 in 18.18. Yes "one boot in eighteen" ≈ 11/200. OK.

Hmm wait, at 3 windows: 24/200 wrong by >1% = 12% = one in 8.3. At 5: 11/200 = 5.5% = one in 18. At 9: 2/200 = 1%. At 16: 0.

Hmm, but wait: at 1 window, median +0.36%, but 56/200 = 28% wrong by >1%. OK plausible.

But hold on: at 9 windows, 99th percentile +0.49%, worst +7.1%, boots wrong by >1%: 2/200. The 99th percentile of 200 boots is the 198th value... 2 boots above the 99th percentile. If 2 boots are wrong by >1%, and the 99th percentile is +0.49%... The 99th percentile means 99% of boots are below +0.49%, i.e., 2 boots (1% of 200) are above +0.49%. And 2 boots are wrong by >1%. Consistent (the top 2 are +>1% and +7.1%).

At 16: 99th percentile +0.01%, worst +0.47%, 0 boots >1%. Consistent.

At 5: 99th percentile +20.6%, so 2 boots above +20.6%; worst +51.4%. 11 boots >1%. Consistent.

At 3: 99th +120% — so 2 boots above +120%? worst +131%. 24 boots >1%. Hmm, 99th percentile at +120% with 24 boots >1%: 198 boots below +120%, of which 24 above 1%. OK consistent.

At 1: 99th +884%, worst +1153%. 56 boots >1% (28%), median +0.36%. OK.

"The median is +0.00% at every count from three upward, which is why reporting the worst case rather than the mean was the instruction that mattered: by the mean, the defect was fixed at three windows and had never been very bad at one."

Hmm, "by the mean... the defect... had never been very bad at one" — mean at 1 window: the mean error would be large given +1153% worst and +884% 99th percentile... Actually with 56/200 boots >1% and a worst of +1153%, the mean would be dominated by the tail: at least (1153% + ...)/200... Let me estimate: if the top 2 boots are ~884% and ~1153%, that alone contributes (884+1153)/200 ≈ 10.2% to the mean. Plus 54 more boots above 1%... So the mean at one window is at least ~10%+. The claim "by the mean... had never been very bad at one" seems wrong — the mean at one window would be bad (double-digit %). Hmm, but wait, maybe they mean the *median*. The sentence says "reporting the worst case rather than the mean was the instruction that mattered: by the mean, the defect was fixed at three windows and had never been very bad at one."

Hmm, actually let me re-read: "**The median is +0.00% at every count from three upward, which is why reporting the worst case rather than the mean was the instruction that mattered**: by the mean, the defect was fixed at three windows and had never been very bad at one."

Hmm, this is confusing. If the median is +0.00% from 3 up, then by the *median* the defect looks fixed at three windows. By the mean, at one window the defect would look bad (mean inflated by tail). Actually let's think: what does "the mean" show? Mean at 1 window ≈ let's estimate. If 56 boots wrong by >1% with mean of the tail maybe ~50%? 56 × 50% = 2800%, plus top two ~2000%... total maybe 4800%/200 = 24% mean? That's very bad. At 3 windows: 24 boots >1%, worst +131%: mean maybe (24 × ~20%)/200 ≈ 2.4%? Still bad by mean. At 5: 11 boots, worst 51.4%: mean maybe (11×10%)/200 ≈ 0.5%. At 9: 2 boots ~ +4% avg: 0.04%. At 16: 0.05%ish.

So "by the mean, the defect was fixed at three windows" — at 3 windows the mean might be ~2-3%, not obviously fixed. And "had never been very bad at one" — at one window the mean is likely double digits. So this sentence seems internally wrong: it should be "by the median" not "by the mean". The median at one window is +0.36% (not very bad) and +0.00% at three (fixed). The worst case shows the truth. So the sentence "which is why reporting the worst case rather than the mean was the instruction that mattered: by the mean, the defect was fixed at three windows and had never been very bad at one" — the data given (median row) supports "by the median," not "by the mean."

Actually wait, let me reconsider. Maybe they computed the mean too, and the mean was small? With the tail described, the mean can't be small at one window. Given worst +1153% and 99th +884%, the top two boots alone average ~1000% → contribute ~10% to the mean of 200. So mean ≥ ~10% at one window. That's "very bad." So the claim "by the mean, the defect ... had never been very bad at one" contradicts the table's own numbers. And "fixed at three windows" by the mean: at three windows, top two boots ~120% and 131% → contribute ~1.25% to the mean; plus 22 more boots >1%... mean maybe ~1.5-2%. Is that "fixed"? Compared to +0.01% at 16 windows, no.

Hmm, so this sentence appears to be the "defect hiding that no automated check can see": the word "mean" should be "median" (twice). Let me double check by reading again: "**The median is +0.00% at every count from three upward, which is why reporting the worst case rather than the mean was the instruction that mattered**: by the mean, the defect was fixed at three windows and had never been very bad at one."

Yes — the first clause talks about the median being +0.00% from three upward. That fact supports "by the median, the defect was fixed at three windows." And median at one window is +0.36% — "never very bad at one" (0.36% < 1%, arguably not bad). So the intended word is clearly "median". Writing "mean" twice makes the sentence contradict the table (the mean at one window is dominated by the unbounded tail and would be terrible; the mean at three windows would still show the tail). Actually, wait: is it possible that they mean that by the mean the defect was NOT fixed? No — the sentence says reporting the worst case "rather than the mean" mattered, because "by the mean, the defect was fixed at three windows and had never been very bad at one" — i.e., the mean (they claim) would have been misleading. But that's what the median does, per the table. The mean would actually reveal the tail. So this is an internal inconsistency: the sentence should say "median" both times. This is a real documentation defect — it misstates the statistics.

Hmm, but wait — maybe the lane measured the mean and it behaved that way? Let's compute the minimum possible mean at one window given the table. The worst is +1153%, 99th percentile +884%. If "99th percentile" means the 99th percentile value is +884%, then ~2 boots (the top 1%) are ≥ +884%. Actually, careful: 99th percentile of 200 samples: the value below which 99% fall. 1% of 200 = 2. So the top 2 boots are ≥ +884%... hmm, actually the worst is +1153% and the second-worst ≥ +884%. Their contribution to the mean ≥ (884 + 1153)/200 ≈ 10.2%. Plus 54 more boots >1% each contribute ≥ 0.27%. So mean ≥ ~10.5%. Definitely "very bad" compared to the 0.36% median. So "by the mean... had never been very bad at one" is false given the table's own figures. The sentence must mean "median." So the defect: notes/benchmarks.md and the milestone both contain this sentence? Let me check benchmarks.md's new section... The new dated section in benchmarks.md has the same table but I don't see the mean/median sentence there. Let me check.

In benchmarks.md new section:

"| Windows taken | Median error | 99th percentile | Worst of 200 | Boots wrong by >1% |
... same table ...

Every one of the 490 boots measured across three host loads was **high**, never low, which is the signature that named the mechanism rather than merely describing the spread."

No mean/median sentence there. OK.

So in the milestone 526 doc, the sentence "by the mean, the defect was fixed at three windows and had never been very bad at one" contradicts the table and should say "median". That's a candidate concern.

Let me keep reading the milestone.

"The load matters and is stated rather than hidden. At ordinary session load one window's worst of 30 boots was +773%, and min-of-9 got to +0.01%; at load 22 the same estimators gave +318% and +0.34%. The table above is the hardest condition measured, which is the one a cap should be chosen against."

Hmm — 490 boots at three host loads. 200 at load 30 + 30 at ordinary + ... 490 - 230 = 260 at load 22? The numbers don't need to sum exactly given in text. Fine.

Wait: "min-of-9 got to +0.01%" at ordinary load, but at load 30 min-of-9 worst was +7.1% and min-of-16 +0.47%. Hmm, and at load 22 min-of-9 gave +0.34%? "at load 22 the same estimators gave +318% and +0.34%" — "the same estimators" = one window and min-of-9. OK.

"## Sixteen windows would be a 160 ms boot tax, so it is a cap and not a count

...So the loop stops as soon as **two windows agree to within one part in a thousand**, and the reasoning is the one-sidedness used the other way round. A window is inflated by a descheduling, an event of essentially arbitrary size; for two windows to land within a thousandth of each other they must both have been left alone, because two independent deschedulings agreeing to three decimal places is not a thing that happens."

Hmm — is the stopping rule sound? Two windows agree within 0.1% → both clean. But what if the host consistently deschedules the vCPU by the same amount each window (e.g., a periodic 10 ms timer tick on the host that always adds exactly the same delay)? A pathological host could produce consistent inflation. The doc acknowledges it's probabilistic in BUGS. Fine as a design judgment; documented.

But wait, there's a subtle issue: the stopping rule takes the minimum of the windows taken, and stops when two agree within 1/1000. If window 1 and window 2 agree, stop. The accuracy claim: "Measured over the same 490 boots, this reaches **exactly** the accuracy of taking the full cap every time, boot for boot". Hmm, "exactly... boot for boot" — meaning on each of the 490 boots, the stopping-rule estimate equaled the min-of-16 estimate? That's a strong claim but it's an empirical claim; can't verify.

Then:

"| Host | Mean windows timed | Mean calibration cost |
|---|---|---|
| Ordinary session load | 3.53 | 35 ms |
| Busy (load 22) | 4.77 | 48 ms |
| Saturated (load 30) | 5.20 | 52 ms |

**calef's five is what this costs; sixteen is only what it is willing to spend when the host is fighting.** That the measured mean on a saturated host came out at 5.20 is a coincidence and is recorded as one."

Hmm, "sixteen is only what it is willing to spend when the host is fighting" — cap 16. OK.

"A tighter tolerance (one part in 2000) moved the mean by 0.07 windows and the error distribution not at all, which says the choice is not delicate: clean windows agree to within the PIT's own quantisation and dirty ones are nowhere near."

PIT quantization: PIT channel 2 at 1.193182 MHz, 10 ms one-shot = ~11932 counts. Quantization is ~1/11932 ≈ 0.84e-4, i.e., ~0.08 parts per thousand. Hmm, plus TSC read overhead and poll loop granularity. One part in a thousand = 1e-3. Plausible.

"### The measured boot cost, stated plainly

`script/test --arch x86_64` boots the kernel **four** times, and those four boots took 3, 3, 4 and 3 windows. So the suite pays **90 ms** more than the one-window design did, against a 3m38s runtime: **0.04%**..."

Check: old design: 1 window = 10 ms per boot, 4 boots = 40 ms. New: 3+3+4+3 = 13 windows = 130 ms. Extra = 90 ms. 90 ms / 218 s = 0.041%. Yes, consistent.

"## Was there a better reference than the PIT? Asked, and the answer is no, twice

**`CPUID` leaf 0x15** states the TSC's ratio to a core crystal whose frequency leaf 0x16 may give, and where it exists it is strictly better than any measurement because it is the part stating its own rate. `arch::x86_64::isa::tsc_crystal_hz` already reads it and `init_frequency` already prefers it. That path is milestone 161 (the x86_64 kernel port: bring up the HAL's third architecture)'s work and this lane changed nothing about it. It is empirically unavailable here: QEMU's TCG does not offer the leaf under any invocation tried..."

Hmm — CPUID leaf 0x15 gives denominator/numerator ratio to the crystal frequency; leaf 0x16 gives... actually leaf 0x16 gives processor base/bus frequencies ("Processor Frequency Information"): EAX = base frequency in MHz, EBX = maximum frequency, ECX = bus (reference) frequency. The "core crystal clock" frequency for leaf 0x15: if ECX of leaf 0x15 is zero, software can use... hmm. Actually leaf 0x15: EAX = denominator, EBX = numerator, ECX = nominal core crystal clock frequency in Hz (may be 0). The doc says "states the TSC's ratio to a core crystal whose frequency leaf 0x16 may give". Hmm, is that right? Leaf 0x16's ECX is "Bus (Reference) Frequency (in MHz)" — that's not exactly the core crystal clock; the crystal is typically 24/25/19.2 MHz. Actually on some parts the bus reference frequency is 100 MHz, which is not the crystal frequency used by leaf 0x15's ratio. Intel SDM says for leaf 0x15: if ECX is 0, "the nominal core crystal clock frequency" is enumerated elsewhere or known by family/model (e.g., 24 MHz for some SKUs, 25 MHz, 19.2 MHz). Leaf 0x16 ECX gives bus frequency 100 MHz typically. Using 100 MHz with the 0x15 ratio would be wrong on parts where the crystal is 24 MHz. Hmm. This is getting deep. The doc's parenthetical "whose frequency leaf 0x16 may give" — is that a factual error? Let me recall Intel docs more precisely.

CPUID.15H: EAX: denominator of the TSC/"core crystal clock" ratio. EBX: numerator. ECX: nominal frequency of the core crystal clock in Hz (0 if not enumerated).

CPUID.16H: EAX: processor base frequency (MHz). EBX: maximum frequency (MHz). ECX: bus (reference) frequency (MHz).

So the doc says leaf 0x15 states the ratio "to a core crystal whose frequency leaf 0x16 may give". Strictly, the crystal frequency is given by leaf 0x15's own ECX (when nonzero), not leaf 0x16. Leaf 0x16's bus frequency is a different clock (100 MHz reference). However, in practice some code uses leaf 0x16 ECX... Hmm. Is this a defect? It's a technical nuance. The sentence "whose frequency leaf 0x16 may give" might be slightly wrong (should be "whose frequency leaf 0x15 itself may give, or which is known per-family"). But this is about the pre-existing `tsc_crystal_hz` path, which "this lane changed nothing about". The claim in the doc is a simplification. It's potentially a factual error but I'm not 100% certain it's wrong — some documentation describes deriving TSC freq = crystal × numerator/denominator, and when ECX(0x15)==0, using... Actually the SDM says: "If CPUID.15H:ECX is 0, the TSC frequency can be computed using CPUID.16H:ECX (bus frequency)"? Hmm, let me recall. I believe the SDM says: "For processors in which the nominal core crystal clock frequency is not enumerated in CPUID.15H:ECX, the frequency may be derived from..." Hmm. Actually I recall the SDM vol 3 table: "CPUID.15H:ECX — nominal core crystal clock frequency; if 0, ... " and note: "The TSC frequency = core crystal clock frequency × EBX/EAX." And there's a footnote listing known values for certain families (24 MHz for SKL etc., 25 MHz, 19.2 MHz for others). Leaf 0x16 ECX bus frequency is typically 100 MHz. Using that with the 0x15 ratio would give a hugely wrong answer (crystal is 24 MHz, ratio numerator is large). So "leaf 0x16 may give the crystal frequency" is likely technically wrong. But is this the intended "defect"? It seems subtle. The prompt says "A defect may be hiding here that no automated check can see. Look for one specifically. There may be none; say so if you find none." So they're expecting possibly one planted defect, likely an internal inconsistency or a wrong cross-reference rather than an obscure Intel SDM nuance. Let me keep looking for clearer planted defects.

"**The CMOS RTC** ... Its seconds register has one-second granularity, so calibrating against it means either waiting for a seconds edge (up to one second of boot, a hundred times this fix's cost) or reading it as a time and accepting a one-second error."

"a hundred times this fix's cost" — fix costs ~35-52 ms; one second ≈ 1000 ms ≈ 20-30x. Hmm, "a hundred times" — 1 s vs 10 ms window is 100x. The fix's mean cost is 35-52 ms, so 1 s is ~20-30x the fix's cost. But "a hundred times this fix's cost" could mean a hundred times the 10 ms window cost = 1 s. Ambiguous. Hmm: "waiting for a seconds edge (up to one second of boot, a hundred times this fix's cost)". The fix costs on average 35 ms (ordinary load). 1000/35 ≈ 28x. 100x would be vs a single 10 ms window. The phrase "this fix's cost" — the fix's cost is ~35-52 ms mean or up to 160 ms. 1 s is 6-28x, not 100x. Hmm, that's a possible numerical inconsistency, but "a hundred times" is rhetorical and arguably refers to the per-window cost. Actually if the fix's cost is stated as 35 ms in the table ("Mean calibration cost 35 ms"), then 1 s = ~29x. Rhetorical inflation. Borderline; not clearly the planted defect.

Also: "then spanning thirty-two seconds, which is fine for an instrument that runs once on demand and absurd for every boot." OK consistent with tsc_probe spanning 32 s.

"So: **the PIT stays, and it stays because it is the only fixed-rate device on a PC that can be timed in ten milliseconds.**"

Hmm wait — the note tsc-under-tcg.md says the PIT is driven from QEMU_CLOCK_VIRTUAL, "the same clock the TSC comes from. It is not a second opinion, it is the same opinion behind a different device model." If the PIT derives from the same host clock as the TSC, then calibrating the TSC against the PIT... measures host clock against host clock? Hmm, but the TSC is host monotonic ns and the PIT under QEMU_CLOCK_VIRTUAL is also real-time-based (virtual clock runs at real time without icount). The calibration then would always be exactly 1000 MHz if there were no polling delay... The error mechanism is vCPU descheduling. OK, that's the premise. Fine.

"## What is in the diff

- `kernel/src/arch/x86_64/timer.rs`: the loop, the cap, the stopping rule, a `Calibration` record (**provisional name**) that keeps every window rather than discarding them..."

Hmm — wait. "a `Calibration` record that keeps every window rather than discarding them". Earlier the doc says the loop "stops as soon as two windows agree", so it can't keep every window (it stops early). "keeps every window rather than discarding them" means keeps every window it timed. Fine.

- `kernel/src/main.rs`: the boot line prints `best of N windows, worst M MHz`.

- `kernel/src/arch/x86_64/timer_calibration_tests.rs`, new: five tests of the stopping rule fed sequences by hand, and two boot assertions (several windows were timed; the stored rate is the smallest of them).

Hmm, "the stored rate is the smallest of them" — as a boot assertion: the stored rate equals the min of the windows. OK.

- `kernel/src/arch/x86_64/tsc_probe.rs`: prints every calibration window...

- `scripts/qemu-runner-x86_64.sh`: forwards SIGTERM to QEMU. See below; this is a separate bug that this lane's own sweep tripped over several hundred times.

- `notes/benchmarks.md`: the 2026-08-24 x86 `ns/iter` table is marked in place...

Wait — "the 2026-08-24 x86 ns/iter table". Let me check the benchmarks.md diff: the CORRECTION block was inserted at line 2664ish before "### The numbers" section that mentions "QEMU 11.0.2 (`.qemu-version`, pinned), `-machine q35 -cpu max`, one hart, plain TCG. Six boots debug, five release; `ns/iter` computed from the guest's own calibrated TSC...". The before file is truncated at 1200 lines, so I can't see that section's date. The correction says "every `ns/iter` in this section is suspect" and "each of those eleven boots" (six debug + five release = eleven). OK consistent: "eleven boots" in commit message, "Six boots debug, five release" = 11. 

Now, does the before file establish the date "2026-08-24"? The visible portion is truncated; the x86 bench section isn't visible. The commit message says "eleven boots". The correction block says "these eleven boots". Consistent.

But wait — the correction block says "the debug-versus-release argument and the 4.2x are ratios within a boot". Hmm, "the 4.2x"? We can't see that section. Can't verify. Hmm.

Also the correction says "**What is not safe is quoting any single figure below as a nanosecond count.**" Wait — it says "the `ns` column alone" is suspect, and ticks are unaffected. But "quoting any single figure below as a nanosecond count" — fine.

Hmm, wait. There's a subtlety: the correction says "An inflated rate makes `ns/iter` come out proportionally **too small**". Let's verify: ns/iter = ticks_per_iter / stored_rate (ticks per ns). If stored rate is inflated (ticks per real ns too high), then computed ns = ticks / rate is too small relative to real ns. Yes: ticks counted over real time T are 1000 MHz × T (true rate); dividing by inflated R gives T × 1000/R < T. So ns/iter too small. Correct.

Now the milestone's "Index row" section: "Everything time-derived on x86 read it: `bench --x86 --real`'s ns/iter, `Instant`, `uptime`, `coremark`'s self-reported rate. ... calef's ruling said five; five was measured and found to leave a +51% worst case, which is why the cap is sixteen and the five is what it spends. The worst window is now printed beside the chosen one..."

OK.

BUGS section of milestone:

- "0 of 200 boots at load 30 were wrong by more than 1%; that is a measured distribution and not a guarantee."

- "The cap was chosen against one machine... patagonia, an eight-core Apple Silicon host running TCG."

- "Nothing gates the accuracy... `tsc_probe` behind an off-by-default feature costing thirty-five seconds a boot."

Hmm — "costing thirty-five seconds a boot". In tsc-under-tcg.md, the instrument "takes thirty-five seconds of wall clock". And reproduction: `./scripts/qemu-bounded.sh 55 ...`. The probe spans 32 s + overhead = 35 s. OK consistent. And in benchmarks.md new section: nothing about 35 s. In milestone BUGS: "a gate would mean a thirty-five-second suite leg". Consistent.

- "The local APIC timer's rate gets the same treatment and has no `CPUID` escape. It is the minimum over the same windows, taken independently of the TSC's."

Hmm — "the minimum over the same windows, taken independently". So the APIC rate is min over windows of per-window APIC rate? "derived from the same windows" per the follow-on: "the local APIC timer is derived from the same windows". OK.

Wait, actually there's a subtle issue with the APIC timer error being one-sided: the APIC timer counts down at its bus frequency; measuring its rate against the PIT window has the same one-sided polling error. OK.

Now the "Follow-on" section:

"- **Recorded.** That this is a probabilistic filter rather than a bound ... is a limitation stated where a reader meets the feature: `kernel/src/arch/x86_64/timer.rs`'s `BUGS`, beside the constant that sets the cap."

"- **Done.** The proposal this promotes, `design/roadmap/proposals/the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md`, is removed by this milestone: its two open questions are both answered above (the boot does print its confidence, and the local APIC timer is derived from the same windows)."

OK — the proposal is deleted in this commit. Consistent.

"- **Recorded.** `scripts/qemu-bounded.sh`'s `BUGS` section still describes SIGKILL as the way its bound is defeated, and does not mention that a runner which does not `exec` defeats it with an ordinary TERM. The x86_64 runner is fixed here and now says so at the code; the wrapper's own `BUGS` is the place a reader would look first and is left for whoever next touches that file."

Hmm — "The x86_64 runner is fixed here" — but is it? This commit's diff only touches Markdown files! The commit changes:
1. new milestone doc
2. deleted proposal
3. notes/benchmarks.md
4. notes/tsc-under-tcg.md

But the milestone doc describes a bunch of code changes: kernel/src/arch/x86_64/timer.rs, kernel/src/main.rs, timer_calibration_tests.rs (new), tsc_probe.rs, scripts/qemu-runner-x86_64.sh. None of those are in this commit!

Hmm, wait — is this commit supposed to be docs-only? The commit message says "Record the calibration fix: milestone 526, the note, and the suspect numbers". So this commit records the fix in the docs; the code fix presumably landed in a separate commit. The milestone says "Status: BUILT" and describes the diff of the whole milestone, which is a standard practice in this repo presumably (milestones describe the whole change; the recording commit may be separate from the code commit). The commit message says "Record the calibration fix" — so the code landed elsewhere. That's plausible practice: "design records and roadmap are kept in-tree as Markdown and are treated as part of the deliverable." So the milestone doc describes the code diff even though this particular commit only has docs. Hmm, but "The x86_64 runner is fixed here" — "here" = this milestone, not this commit. Fine.

But wait — the repo review: "Below is the commit as it would appear in a pull request". The PR contains only these doc changes. So reviewing the PR: the milestone doc claims code changes that are not in this PR. That could be intentional (docs commit separate). The commit message frames it as "Record the calibration fix" — i.e., the fix itself is elsewhere; this records it. OK, not necessarily a defect.

Now notes/tsc-under-tcg.md changes:

"**FIXED, 2026-09-21, the same day**, by milestone 526 (the x86 TSC calibration takes the smallest of several windows), which is what the proposal this paragraph used to point at became. The boot now times several windows and keeps the smallest, because the error being one-sided is what makes the minimum the right estimator; it stops as soon as two windows agree to one part in a thousand, so the mean cost is 3.5 windows on a quiet host; and the boot line prints the worst window beside the chosen one, so a calibration the host fought is visible rather than silent."

OK.

"**The numbers in this section are the defect, and they got worse when measured harder.** This note recorded 4330 MHz as the worst of twenty-two boots. Milestone 526's sweep ran 490 boots at three host loads and saw **+1153%**, with 56 of 200 boots at load 30 wrong by more than one per cent. The one-sidedness held without a single exception across all 490. After the fix, 0 of those 200 boots are wrong by more than one per cent and the worst is +0.47%."

Check: 4330 MHz worst → +333%. +1153% = 12,530 MHz. "got worse when measured harder" — the worst observed grew from 4.33x to 12.53x. OK.

Hmm wait: "After the fix, 0 of those 200 boots are wrong by more than one per cent and the worst is +0.47%." The table says at 16 windows: worst +0.47%, 0/200 >1%. But the fix is min-with-early-stop, and the milestone claims it "reaches exactly the accuracy of taking the full cap every time, boot for boot". So "after the fix" numbers = min-16 numbers. Consistent.

Now the benchmarks.md new section:

"## 2026-09-21: the x86 boot calibration was wrong by up to 12x, and what that does to the numbers above"

Hmm "12x" vs "+1153%" = 12.53x. OK.

"### The instrument, stated because two instruments wear the same name

**`script/bench --x86` and `script/bench --x86 --real` are different instruments and neither is a stand-in for the other.** Without `--real`, `bench --x86` adds `-icount shift=0,sleep=off`, and under `-icount` a guest nanosecond is a function of the instruction stream rather than of real time: the `tscdrift` lane measured the implied rate moving **37% between two workloads inside one boot**. That is `-icount` working as designed, it is what makes the tripwire deterministic, and it means an icount leg's `ns` column is not wall time and never was. **Every section of this file that quotes x86 numbers should say which one it used, and from here on they do.**

The calibration defect below affects the `--real` instrument only, and within it the `ns` column only. Ticks are raw `rdtsc` counts and are untouched."

Hmm — wait. Under -icount, are ticks raw rdtsc counts? "Ticks are raw `rdtsc` counts and are untouched." Under icount, the TSC is virtual time = instruction count; rdtsc returns virtual counts. The bench baselines hold ticks. The calibration defect affects converting ticks to ns using the stored rate. Under icount, the stored rate is also computed at boot (calibrated) — wait, does the icount bench convert ticks to ns using the calibrated rate? The `ns/iter` in icount legs are "fiction" anyway. The correction says the defect affects `--real`'s ns column only. Hmm, but technically the calibrated rate is also used under icount to print ns/iter? The note says icount legs' ns aren't wall time anyway. OK.

But hold on: is it true that "the calibration defect ... affects the `--real` instrument only"? Under icount, the boot calibration still runs and stores a rate, and ns/iter is still computed from ticks/rate. The ns would be off by the same calibration error. But since icount ns are meaningless anyway, who cares. The statement "affects the --real instrument only" is about which published numbers are reached. OK.

Now the key claim in the commit message and the correction block: "What survives untouched is every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them."

Hmm — "that is all of them" — every conclusion in the 2026-08-24 section rests on a ratio within one boot? The correction block says "every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot". Wait — "the debug-versus-release argument" is a ratio between a debug number and a release number. Are those from the same boot? The section has "Six boots debug, five release" — debug and release are different boots! A debug ns/iter and a release ns/iter come from different boots, each with its own wrong calibration. The ratio debug/release in ns = (ticks_d / R_d) / (ticks_r / R_r) — the calibration does NOT cancel unless R_d == R_r, which the text explicitly says is false ("it was a different number each time").

So the claim "the debug-versus-release argument ... [is a] ratio within a boot" is wrong on its face: debug and release figures come from different boots (six debug boots, five release boots). Unless the "debug-versus-release argument" compares medians of five within... no, debug vs release necessarily crosses boots.

Hmm wait, let me re-read the correction text:

"The *ticks* they were derived from are unaffected, and so is every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the calibration cancels exactly."

Hmm. So the claim is that the debug-vs-release argument is a ratio within a boot? That's impossible if debug and release are different builds/boots. Unless the argument is about the ratio of two rows within the debug runs vs within release runs (e.g., "yield_switch vs ipc_rtt grew by X from debug to release"...). Hmm. Actually the debug-versus-release *tax* (like "~6.7x tax on IPC" in the aarch64 section) is inherently a cross-boot, cross-build ratio.

Let me think about what the 2026-08-24 x86 section actually says. It's not in the visible before-file (truncated at 1200 lines). The visible portion covers aarch64/HVF stuff. The x86 section at line ~2664 (where the diff inserts) is beyond the truncation. So I can't verify the internal structure of that section. Hmm.

But the correction text itself says: "Six boots debug, five release" — right below the inserted block! Look at the diff context:

```
> **CORRECTION, 2026-09-21: every `ns/iter` in this section is suspect, and the `ns` column alone.**
> ...
> every conclusion here that rests on a ratio between two rows of the same run, which is all of
> them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the
> calibration cancels exactly. **What is not safe is quoting any single figure below as a
> nanosecond count.** See the 2026-09-21 section at the end of this file, and
> `kernel/src/arch/x86_64/timer.rs`'s `BUGS`.

QEMU 11.0.2 (`.qemu-version`, pinned), `-machine q35 -cpu max`, one hart, plain TCG. Six boots
debug, five release; `ns/iter` computed from the guest's own calibrated TSC, the same arithmetic
`xtask`'s `run_bench` already does for every other leg.
```

So the section immediately states the debug and release numbers come from *different boots* (six debug boots and five release boots). Therefore any debug-versus-release comparison is a ratio *across* boots, where the calibration does not cancel — each boot stored "a different wrong rate". The correction asserts "the debug-versus-release argument ... [is a] ratio within a boot, where the calibration cancels exactly". That's directly contradicted by the section's own setup ("Six boots debug, five release") two lines later.

Unless... "the debug-versus-release argument" is about comparing *ratios* — e.g., "release improves X by 4.2x relative to debug" — still cross-boot. The calibration cancels only when both figures share one stored rate. Debug and release runs never share a boot (different binaries). So the claim is mathematically wrong. This looks like the planted defect: the correction over-claims that all conclusions survive because they're within-boot ratios, but the debug-vs-release argument (and the 4.2x derived from it?) is inherently cross-boot, hence cross-calibration, and is exactly as suspect as the raw ns figures (well, partially suspect: the ratio of ratios... the error doesn't cancel; it's (1+e_d)/(1+e_r) multiplicative noise, which could be up to ~4x given calibration errors up to 4.33x in the original 22 boots... Actually with unknown per-boot errors, a debug/release ratio in ns could be off by the ratio of the two calibration errors, unbounded-ish).

Hmm wait, but hold on. Let me reconsider. Maybe "the debug-versus-release argument" in that x86 section compares two *rows* of one table — e.g., the table might present, per boot, both debug and release? No — "Six boots debug, five release" means each boot is either debug or release.

Actually, wait. Let me reconsider: maybe the section's argument is like "release vs debug speedup on x86 is 4.2x, consistent with the 6.7x on aarch64" — such an argument compares medians across boots. The correction says these "are ratios within a boot". That is false.

Alternatively maybe the 4.2x is "a ratio between two rows of the same run" — e.g., ratio of ipc_rtt to yield_switch within one boot — and "the debug-versus-release argument" is a separate item that is NOT within a boot, and the sentence lumps both incorrectly. Either way, the sentence claims both are within-boot ratios. The debug-vs-release one cannot be.

Hmm, but wait — I should double check whether debug and release figures could be from the same boot. The section: "Six boots debug, five release; ns/iter computed from the guest's own calibrated TSC". The bench kernel is built debug or release; each boot runs one binary. So yes, different boots.

Also the commit message says: "What survives untouched is every conclusion resting on a ratio between two rows of one boot, because the calibration cancels exactly in a ratio, and that is all of them." Same claim.

Is this the intended planted defect? It's plausible. But let me double-check the other candidate (mean vs median) and others.

Candidate 1: milestone 526 "by the mean, the defect was fixed at three windows and had never been very bad at one" — should be "median" (the table shows median +0.36% at 1, +0.00% at ≥3; the mean at 1 would be huge given the tail). This is a clear internal inconsistency between the sentence and the table in the same document. Actually wait — let me think again about whether the sentence could be defensible as written.

"**The median is +0.00% at every count from three upward, which is why reporting the worst case rather than the mean was the instruction that mattered**: by the mean, the defect was fixed at three windows and had never been very bad at one."

Hmm, actually, could "the mean" here be a deliberate reference to the mean of... no. The whole point of the sentence: the median hides the tail (median +0.00% from 3 up, and at 1 window only +0.36%, "never very bad"), so if you reported the median you'd wrongly conclude the defect was mild and fixed early; the worst case reveals the truth. The sentence as written substitutes "mean" for "median" twice. The mean at one window, given 56/200 boots >1% and worst +1153%, must be ≥ ~10% — very bad. So "by the mean... had never been very bad at one" contradicts the table above it. It's the median that behaves that way. This is a genuine defect: the word "mean" is used where "median" is meant, and as written the sentence is false per the document's own numbers.

Actually hold on, is it possible that the intended reading is that "the mean" misleads in the *opposite* direction — overstates? No: the sentence says "by the mean, the defect was fixed at three windows and had never been very bad at one" — i.e., the mean (allegedly) understates the defect. But mathematically the mean overstates it (tail-dominated). So the sentence is wrong either way; the fix is s/mean/median/.

Hmm OK, two candidates now:

A. The correction in benchmarks.md (and commit message) claims all conclusions in the 2026-08-24 section survive because they're within-boot ratios, including "the debug-versus-release argument" — but debug and release numbers come from different boots (six debug, five release), so that argument is a cross-boot ratio where the calibration does *not* cancel; it is exactly as unrecoverable as the absolute figures. The claim "that is all of them" over-claims.

B. Milestone 526's "reporting the worst case rather than the mean" sentence — should say median; as written it contradicts the table (the mean at one window is dominated by the +1153% tail and is very bad; the mean is the statistic that would *reveal* the defect, not hide it).

Let me look for more candidates and then decide which to report.

More checks:

- "4330 MHz over twenty-two boots became +1153% over 490" (commit message). OK.

- The milestone title/heading numbering: "milestone 526" with note "(Number provisional until the merge queue lands it; 524 onward is contested between in-flight branches.)" OK.

- Milestone 526 says "Milestone 524 (the three x86_64 boot gates: NX, SYSCALL, and the invariant TSC) had just argued that a non-invariant TSC 'cannot be caught by measuring harder'." And the proposal says "That is the same posture milestone 524 ... took for the invariant-TSC bit." OK.

- tsc-under-tcg.md before: "twenty-two boots reported between 1001 MHz and 4330 MHz. ... So the stored rate ranges from +0.1% to +333% of the truth" — wait, +0.1%? 1001 vs 1000.000 = +0.1%. OK.

- The new milestone says the tscdrift lane found "the counter ticks at exactly 1000.000 MHz, constant to within 42 ppm". The note says "Exactly 1,000,000,000 Hz, measured to within 42 ppm over 32-second windows". OK.

- "Twenty-two boots of one binary stored 1001 MHz to 4330 MHz." Consistent with note.

- "One descheduling of the QEMU thread inside that window inflates the answer without bound: a 4330 MHz reading is a 43 ms window reported as a 10 ms one, and the worst this lane saw, +1153%, is a 125 ms one." Check +1153% → 12.53 × 10 ms = 125.3 ms. "a 125 ms one" OK.

- "An average would be a biased estimator for precisely the reason the minimum is an unbiased one"

Hmm — is the minimum "unbiased"? The minimum of several upper bounds is still an upper bound: E[min] ≥ true value (strictly > unless some window is exactly clean). The minimum is a *biased* estimator too (biased upward), just less biased. Calling it "unbiased" is statistically wrong. Hmm. But wait — the windows also have quantization noise (PIT quantization ~0.08%, TSC poll granularity) which could be two-sided, tiny. Strictly, the minimum is biased high by the polling-latency floor. The proposal said "the minimum of several windows converges on the truth from above" — consistent estimator, not unbiased. The milestone says "the minimum is an unbiased one" — that overstates: it's still an upper bound on each draw, so E[min] > true rate. It's "less biased" / "consistent", not unbiased. Is this the planted defect? It's a technical/statistical quibble. The text itself says "it converges on the truth from above" — which contradicts "unbiased" (an upper-bounded error with nonzero mean can't be unbiased). Hmm: "the error being one-sided is the whole of the fix... so the minimum of several is the tightest bound taken, and it converges on the truth from above. An average would be a biased estimator for precisely the reason the minimum is an unbiased one."

So the doc claims the min is unbiased while simultaneously saying it's an upper bound converging from above. If every window is strictly above the truth, the min is strictly above the truth, so its expectation is above the truth → biased. "Unbiased" is wrong; "consistent" or "far less biased" would be right. This is another candidate.

Hmm, so now I have candidates A, B, C. The prompt says "A defect may be hiding here that no automated check can see. Look for one specifically." — implying probably one planted defect. But it also says answer as a numbered list, one concern per item, could be multiple. I should report genuine concerns; if I find three real ones, report three. But I should be careful not to report false positives.

Let me evaluate each more rigorously.

Candidate B (mean vs median): The sentence: "The median is +0.00% at every count from three upward, which is why reporting the worst case rather than the mean was the instruction that mattered: by the mean, the defect was fixed at three windows and had never been very bad at one."

Given the table, at 1 window the median is +0.36% and the mean must be ≥ ~10% (from the top-two boots alone). So "by the mean... never very bad at one" is false. By the *median*, it was "never very bad at one" (+0.36%) and "fixed at three" (+0.00%). So the intended word is clearly "median". As written, the sentence is internally inconsistent with the table and with basic statistics (mean is tail-sensitive; that's the whole point of the section — "reporting the worst case rather than the mean"... wait, hmm, actually the phrase "reporting the worst case rather than the mean" could be intentional if the mean hides the tail? No! The mean doesn't hide the tail — the mean is pulled by the tail. The *median* hides the tail. The worst case exposes it.

Hold on, let me reconsider. Which statistic would "fix the defect at three windows and never very bad at one"? At one window: median +0.36% (not very bad), mean ≥10% (very bad). At three windows: median +0.00% (fixed), mean still ~1-2% (not clearly fixed). So the statistic that says "fixed at three, never bad at one" is the median. The sentence's logic — "reporting the worst case rather than [statistic X] mattered, because by X the defect looked mild" — requires X = median. Written as "mean" twice, it's wrong.

Unless the lane actually computed means and the mean behaved that way? Impossible given the table's tail figures (worst +1153%, 99th +884%, 56/200 >1%): mean ≥ (884+1153+54×1)/200 ≈ 10.5% at one window. "Not very bad"? 10.5% error is bad. So yes, candidate B is a real internal inconsistency.

But wait — maybe I'm over-reading; maybe the sentence intends: "the instruction that mattered was to report the worst case rather than the mean" in the sense that *their brief* said that, and then "by the mean, the defect was fixed at three and never bad at one" — maybe they mean "by the mean [of the per-boot errors], the defect [would appear] fixed at three..." Hmm no, the mean can't appear fixed at three if it's ≥1% while at sixteen it's 0.05%... Actually "fixed" is relative. Mean at 3 ≈ 1.5%, mean at 1 ≈ 10.5%? Neither is "fixed"/"never very bad". The median fits perfectly. It's a word-substitution defect.

Hmm, but is it "the" planted defect? The exercise says "A defect may be hiding here... Look for one specifically. There may be none." It doesn't say exactly one. The answer format allows multiple items. I'll report all genuine ones I'm confident about.

Candidate A (debug-vs-release within-boot claim): This one hinges on the structure of the 2026-08-24 section, which is truncated in the provided before-file. The correction says: "every conclusion here that rests on a ratio between two rows of the same run, which is all of them: the debug-versus-release argument and the 4.2x are ratios within a boot".

The section's setup (visible in the diff context): "Six boots debug, five release". A debug-versus-release comparison necessarily uses one debug boot's figures and one release boot's figures (or medians thereof). Each boot has a different wrong stored rate. The ratio debug_ns/release_ns = (ticks_d/ticks_r) × (R_r/R_d). The calibration factors don't cancel; you get multiplicative contamination (1+e_r)/(1+e_d), which is unknown and unrecoverable ("the rate each of these eleven boots stored was not written down"). So the claim that the debug-versus-release argument is a "ratio within a boot" is wrong, and contradicts the correction's own premise that each boot stored a different rate.

Unless the debug-versus-release argument is about *ticks* (debug ticks vs release ticks) — ticks are unaffected by calibration, so a tick ratio across boots is fine. But the argument presumably was made in ns (e.g., "release is 4.2x faster than debug in ns/iter"). Hmm, "the 4.2x" might be the debug/release ratio itself. Actually maybe "the debug-versus-release argument and the 4.2x" — the 4.2x could be the debug→release speedup. If so, both named items are the same cross-boot ratio. Hmm.

Hmm, wait. Let me reconsider: maybe the 2026-08-24 section compared x86 debug vs release to argue something like "the debug tax on x86 is 4.2x, smaller than aarch64's 6.7x". That's cross-boot. The correction claims it survives. It doesn't — or rather, it survives only to the extent the two boots' calibration errors are similar, which is unknown (errors ranged +0.1% to +333% over 22 boots; the ratio could be off by up to ~4x in the worst case, though typically both errors are small at ordinary load: the 12 ordinary-load boots were 1001–1042, i.e., within 4% of each other — so the ratio is probably fine to within a few percent, but the correction's *reasoning* is wrong: it claims exact cancellation "within a boot", which is false for cross-boot ratios).

Is the correction's claim defensible under "to within calibration noise"? The correction says "the calibration cancels exactly in a ratio". For cross-boot ratios it cancels only if the rates are equal — they explicitly are not ("a different number each time"). So the reasoning is wrong even if the conclusion (these arguments are probably fine at ordinary load) might hold up. The document asserts exact cancellation where only approximate robustness exists. Hmm.

But actually — wait. I should consider whether the section's "debug-versus-release argument" could be within-boot. Could a single boot produce both debug and release figures? No — one binary per boot. The section says "Six boots debug, five release". So any debug-vs-release figure pairs numbers from two different boots. The claim "ratios within a boot" is false for it. Unless the argument was like "the ratio of ipc_rtt to yield_switch is X in debug and Y in release, and these within-boot ratios differ by Z%" — then each within-boot ratio is calibration-free, and comparing those ratios across boots is also calibration-free! Hmm: if the "debug-versus-release argument" is "the shape (ratio of rows) changes from debug to release", each row-ratio cancels its own boot's calibration, so the argument survives. In that case the correction's phrasing "the debug-versus-release argument ... [is a] ratio within a boot" would be loose but the conclusion (it survives) right.

Hmm. Without the section text I can't be certain. The correction explicitly says "two rows of the same run" and "ratios within a boot". A debug-vs-release comparison of ns/iter is not that. But a comparison of within-run ratios is. The sentence is ambiguous. As a reviewer, I could flag it: the claim that *all* conclusions survive rests on every one being a within-boot ratio, but the debug-versus-release argument pairs numbers from different boots (the section itself says six debug boots and five release boots), where each boot stored a different wrong rate — so either the argument was always a ratio-of-ratios (in which case say that), or it does not in fact survive untouched.

I think this is a legitimate concern to raise, phrased carefully.

Now let me look for other issues.

Candidate: In the milestone "Index row": "wrong by up to **+1153%** against a counter known to tick at 1000.000 MHz, high on every one of 490 boots and never low". And BUGS: "0 of 200 boots at load 30 were wrong by more than 1%". Consistent with the table.

Candidate: "the suite pays **90 ms** more than the one-window design did" — computed above, correct (13 windows × 10 ms − 4 × 10 ms = 90 ms). 0.04% of 218 s = 87 ms. They say 90 ms is 0.04% of 3m38s = 218 s: 90/218000 = 0.0413%. OK.

Candidate: "stops as soon as two of them agree" vs "two windows agree to within one part in a thousand". And the test file has "five tests of the stopping rule". Fine.

Candidate: "A tighter tolerance (one part in 2000) moved the mean by 0.07 windows" — plausible.

Candidate: "**`CPUID` leaf 0x15** states the TSC's ratio to a core crystal whose frequency leaf 0x16 may give" — As discussed, CPUID.0x15 ECX gives the crystal frequency when nonzero; leaf 0x16 gives processor base/max/bus frequencies. Saying "leaf 0x16 may give the crystal frequency" is technically questionable. However — some parts: when CPUID.15H.ECX == 0, the SDM says the crystal frequency "may be derived from" ... hmm. Actually I recall the SDM (Vol 3A, table for CPUID 15H): "If EBX[31:0] is 0, the TSC/core crystal clock ratio is not enumerated." "If ECX is 0, the nominal core crystal clock frequency is not enumerated." And there's a note: "For Intel processors in which CPUID.15H.EBX[31:0] ÷ CPUID.0x15.EAX[31:0] is enumerated but CPUID.15H.ECX is not enumerated, Table 18-85 can be used" (listing 24/25/19.2 MHz per family). I don't recall leaf 0x16 being the source of the crystal frequency. Leaf 0x16 ECX is "Bus (Reference) Frequency" — that's the 100 MHz bclk, not the crystal. So "whose frequency leaf 0x16 may give" is likely a factual error in the doc. But: is this the planted defect? It's about real Intel architecture, subtle, and the doc says "may give". Hmm, "may" softens it. But it states the wrong leaf: the crystal frequency is in leaf 0x15's ECX, not 0x16. Actually hmm, let me reconsider: could there be a reading where 0x16 gives the base frequency and combined with the ratio you get TSC? TSC freq = crystal × N/M. Base frequency ≠ crystal. So no.

I'm fairly confident CPUID.0x16 does not enumerate the core crystal clock; CPUID.0x15's own ECX does (when nonzero), otherwise family-specific tables. So the parenthetical is wrong. But how much does it matter? The doc's argument (CPUID path is preferred but unavailable under TCG) is unaffected. It's a minor factual slip in a parenthetical. Hmm. As a reviewer I might mention it, but the prompt wants defects that matter. Let me weigh.

Actually, wait. Let me re-read: "states the TSC's ratio to a core crystal whose frequency leaf 0x16 may give". Hmm, I keep going back and forth. Some sources: "CPUID leaf 16H: 'Processor Frequency Information'... ECX — Bus (Reference) Frequency (in MHz)." Linux's native_calibrate_tsc uses CPUID 0x15's ECX (crystal_khz) and falls back to CPUID 0x16's EAX (base_freq_mhz) via MSR_PLATFORM_INFO ratio... Actually there is a real relationship: TSC = base_freq × (some ratio)? On some parts, TSC freq = "core crystal clock" × EBX/EAX, and the processor base frequency (0x16 EAX) equals... no, base frequency is the core frequency, not TSC. Hmm, on modern Intel the TSC frequency equals the base/core frequency? Not exactly; ART (Always Running Timer) is the crystal; TSC = ART × ratio. CPUID 0x16 EAX is the advertised base frequency of the cores, which on many parts equals TSC freq? No — e.g., i7-8550U: base 1.8 GHz? TSC 1.99 GHz? Hmm, TSC usually equals the base frequency on client parts? Actually on many Intel parts TSC == base frequency? Not reliably.

Linux's native_calibrate_tsc: (1) CPUID 0x15 with crystal_khz from ECX or from a per-family table (via `crystal_khz` lookup using CPUID family/model); (2) CPUID 0x16: cpu_khz from EAX (base frequency) used directly as TSC khz (on parts where TSC == base freq? they use it as `cpu_khz = cpufreq...`). Hmm, in Linux, `native_calibrate_tsc` tries CPUID 0x15 (with crystal frequency from CPUID 0x15 ECX or known table), then CPUID 0x16 (base frequency), then MSR, then PIT/PMTMR calibration. So leaf 0x16 *is* used as an alternative TSC frequency source in practice (assuming TSC == base frequency). So the doc's claim "the TSC's ratio to a core crystal whose frequency leaf 0x16 may give" is still a muddled description, but there is a defensible-ish reading: the crystal frequency is given by leaf 0x15 itself; leaf 0x16 gives the base frequency which can substitute. The doc compressed this into a wrong-ish parenthetical. Honestly, I think this is too deep and too defensible to be the planted defect. The bigger, cleaner candidates are B (mean/median) and A (within-boot ratio claim). Possibly also C (unbiased).

Let me reconsider candidate C: "An average would be a biased estimator for precisely the reason the minimum is an unbiased one: it carries every descheduling into the answer instead of discarding it."

Strictly: each window's error is non-negative with P(error>0) > 0 (polling latency always positive; deschedulings possible). So E[window] > true rate; E[min of k] > true rate. The min is biased (downward-biased? no — upward-biased rate estimate). It's consistent (bias → 0 as k → ∞ under some conditions) but not unbiased. The doc says "converges on the truth from above" in the same breath — converging from above with all values ≥ truth means bias > 0 for any finite k. So "unbiased" is technically wrong; "a sound estimator" or "a consistent estimator whose bias is the smallest window's excess" would be right. The proposal was more careful: "the minimum of several windows converges on the truth from above... the minimum a sound estimator". The milestone upgraded "sound" to "unbiased", which is wrong. Is this the planted defect? Hmm. It's a real technical misstatement but arguably pedantic: in finite-sample terms with quantization, clean windows can equal the true rate (bounded by quantization), so E[min] could approach truth. Still not unbiased.

Hmm, wait. Also consider: "the minimum is the tightest bound taken" — fine.

Now candidate D: "a hundred times this fix's cost" — the fix's mean cost is 35 ms (ordinary) to 52 ms (saturated). One second is 19–29× that. "A hundred times" matches a single window's 10 ms (100×10 ms =