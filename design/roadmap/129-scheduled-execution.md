# 129. Scheduled execution: a cron whose every entry is a grant

**Status: PARTIAL.** Minted 2026-08-15 at calef's request, from the observation that the
customer path wants it: a backup server owes housekeeping on a schedule (snapshot thinning,
scrub passes, log rotation) even though the Mac initiates the backups themselves. Nothing else
on the roadmap runs anything on a schedule.

**Gate: NONE.** It stays `NONE` rather than moving to `DECISION`, which is worth a sentence because
the temptation was there. Every ingredient existed as predicted: §43's clock authority and
`clock_proto`, the spawn machinery and program manifests (milestone 31's grant expressions), and
supervision (§40) for what happens when a scheduled child dies. No new syscall surface was needed.
The `--mem` grant needs nobody (the archive endowment was narrowed to the plan on 2026-08-18); the
per-entry image turned out to need a design fork rather than a lane's capability wiring, checked
2026-08-23 below; the **runtime** registration protocol is calef's. A gate reading `DECISION` would
say the milestone is blocked when most of it is not.

What the build did find is that the *absence* of a syscall shapes the whole program: see "The
finding" below.

## Built: the interval scheduler, 2026-08-18

This block's own "honest first deliverable" test, met: an interval scheduler running supervised
children on both ISAs, with the plan printed before anything fires.

- **`crates/timetable`** holds the decision and no IO, so the four answers registration can give are
  host-tested in milliseconds and the fire arithmetic is Kani-reached (five harnesses; the sixth law,
  phase preservation, is host-tested and notes/scheduled-execution.md says why).
- **`user/src/timetable.rs`** holds a budget, the monotonic counter and the loader. Its complete
  authority is four capabilities: an output endpoint, an untyped budget, a child report endpoint and
  a supervision endpoint. No clock, no directory, no console, no network.
- **`user/timetable.conf`** is the document, in `mdns_config`'s shape and carrying its recorded
  compiled-in limitation.
- **`kernel/src/user/timetable_tests.rs`**, one module for both ISAs.

**The demonstration is the refusals**, which is milestone 126's shape rather than a coincidence.
Registration has **four** answers where a crontab has one, and the split that matters is between "the
line is wrong" (fixed by editing the entry, and it is the prompt's own refusal arriving unchanged)
and "this scheduler holds nothing to back it" (fixed by granting the scheduler more). The sharpest
case is `every 1s date`: a legal line that runs in any crontab on earth, because there the clock is
ambient, refused here **in writing, at registration, before the first tick**, because the wall clock
is a capability (§43) and this scheduler was granted none. `every 1s ps` is the same shape over
milestone 126's process view. Neither ever runs, and the cross-ISA test asserts both the refusal and
the never-running.

**A correction to this block's own sketch.** It says "the service holds one clock capability and
subscribes to §43's monotonic time". It holds no clock, and it cannot: **monotonic time is ambient
here** (the kernel opened the counter to EL0, so `user_rt::monotonic_nanos` needs no capability), and
the wall clock is the thing §43 made a capability. So an interval scheduler needs no clock authority
at all, and the reason a *scheduled entry* can be refused a clock is that the scheduler has none to
give. That is a better story than the sketch's and it was found by building rather than by arguing.

**The registration story, decided for the compile-time case and deliberately not for the runtime
one.** The document is `include_str!`d, so today the authority to register an entry is the authority
to rebuild the image: the strongest possible answer and the least useful one. A runtime protocol is a
real fork (the boot endowment? the shell? a per-registrar endpoint whose entries can be no wider than
the registrar?) and settling it as a side effect of shipping a heartbeat would have been exactly the
accident AGENTS.md warns about.

## The finding: this is milestone 106's fifth consumer, and the first whose purpose is a deadline

**There is no timed wait anywhere in this kernel**, so a program whose entire reason for existing is
to act at a time can only yield and re-read the counter. A running timetable therefore costs a core's
worth of yields, and it reaps its dead children **lazily**, only when the budget cannot back another
instance, because reaping means blocking on the supervision endpoint and a process has exactly one
wait point.

Milestone 106's block counts four consumers of that fork (`net_stack`'s retransmit window,
`thread::sleep`, `RECV`'s no-timeout limitation, the shell's `^C` poll). This is the fifth, and the
first where a deadline is not a fallback path but the whole program. The shape of the fix is already
in the code: `Registry::next_deadline` computes exactly the instant a timed wait would block until,
computed today although nothing can use it, so the loop changes by one line rather than being
restructured.

## Built: the archive endowment narrowed to the plan, 2026-08-18

The scheduler was handed the whole initrd, which is every program in the tree behind a document that
names one. It is now handed an archive holding **exactly the programs its plan will build**, and the
argument is the one this milestone is made of: `Registry::programs` is complete and known before the
first tick, so the endowment can be narrowed to it. `kernel/src/user/timetable_tests.rs` builds the
sub-archive with `nifefs::write_image`.

**The set is computed on the host, not in the kernel, and that is a constraint rather than a
preference.** `Registry::register` carries a 21632-byte frame (a `[Row; MAX_ENTRIES]` where a `Row`
holds a kilobyte of `Endowment`) and `grant_plan::plan` a further 12048, against a 4096-byte guard
page; `script/stack-frame-check` refuses both, correctly. So the spawn site carries a written list
and a host test asserts it equals the plan, by name, from the same document and the same `Held`. A
document edited without the list fails on the host in milliseconds, and fails the cross-ISA test
again through the program's own audit line.

**A process cannot narrow its own endowment, so it measures it.** `timetable::Audit` compares the
names the archive carries against the plan's program set and prints the answer next to the plan, in
one of two sentences. The test asserts the narrow one and asserts the wide one never appears, so a
spawn site that went back to handing over the initrd fails rather than passes.

```text
timetable: the archive it holds carries exactly the 1 program its plan names   <- now
timetable: the archive it holds carries 57 programs, 56 of them beyond its plan <- before
```

## Built: a backable `--mem` grant, 2026-08-22

`Held::mem_pages` was zero, so an entry naming a memory grant was refused although the process held
a budget. `timetable::SHIPPED_HELD.mem_pages` is now 4, and `timetable.conf`'s
`at-boot budgeter --mem 4` is planned, backed, and fires; `user/src/timetable.rs`'s `fire_with_grant`
and `collect_grant` are the mechanism, and its `BUGS` records the cost.

**This block's own sketch for backing it was wrong, and stayed corrected rather than reopened.** It
said to split the grant out of the *instance's own region* so a single `DESTROY` reclaims both.
`regions::destroy_outcome` answers `Refused` for any region with a live child, and
`sched::reap_supervised` passes that back, so a corpse whose region carries a grant cannot be
collected through `reap` at all until the grant is destroyed first, by its own separate capability.

The nesting survives the correction for the reason already found: **a refused reap is the only thing
that pairs a death with a grant.** A supervisor learns a tid and nothing else, because nothing hands
a builder its child's tid (`build_child` returns a TCB capability and `abi::tcb` reads none out).
What was left was a decision rather than wiring, and it was made here rather than deferred again:
**at most one `--mem` instance may be outstanding at a time.** `_start` drains everything already
running, fires the grant-bearing instance alone, and blocks until it dies and its grant is destroyed
before anything else in the document fires again, which is what makes the next death on the
supervision endpoint unambiguous without needing a tid at all. The price is paid by every *other*
entry, not by `--mem` ones: nothing else can fire while that wait is blocked, so an interval entry
due during it runs late rather than on schedule (never dropped: `next_after`'s ordinary
skip-not-catch-up rule covers a wait that outlasts more than one period, the same as any other
stall). The shipped document does not exercise that cost (`at-boot budgeter --mem 4` fires before
the first `every 150ms` tick can even become due); a document whose `--mem` entry shared the clock
with a fast interval would.

A generation counter or a slot table could track more than one outstanding grant; nothing here needs
its child's tid for any other reason, so building that now would be speculative machinery for a
milestone whose document schedules exactly one such entry. Serialising to one was the small answer
and is the one taken.

## Still to build

- **One image per entry rather than one archive per timetable, and this needs more than a capability
  per entry.** Checked by a 2026-08-23 lane rather than built, because the framing above turned out
  optimistic. The residual risk isn't the plan's union being reachable from a spawned *child's*
  loader; it's that `timetable` itself has to keep every entry's image mapped and readable for the
  whole life of its interval, so a compromise of `timetable` reaches the union regardless of how the
  bytes are filed. Splitting one combined archive into N per-entry archives doesn't shrink that: they
  would all still sit in `timetable`'s own address space simultaneously. Actually narrowing this
  means moving the "which image to build" decision out of `timetable`'s own code, into a
  `user/src/spawner.rs`-shaped helper endowed with exactly one image, replicated per entry, talking
  to `timetable` over its own request/reply endpoint. That is new spawn-time machinery this tree does
  not have (nothing today spawns N sub-builders sized to a document computed at registration), and it
  is a design fork rather than a lane's increment. Not scoped as its own milestone yet.
- **A runtime registration protocol, and who holds the right to use it.** calef's, per above.
  **Held 2026-08-22 pending milestones 49 and 152; both blockers cleared, rechecked 2026-08-28.**

  The hold's reason was that calef wants a scheduled job's capabilities to reflect the scheduling
  user's own authority, which makes the registrar in #387's Option 3 a user's session rather than a
  fixed system component, and that collided with DECISIONS §92 (a caretaker is supervised by the
  client it serves), whose rule is that derived authority dies with that client, for a job meant to
  outlive the session that registered it. There was no durable "user" principal to supervise such a
  delegation, and no design for one.

  **Both are now on `main`.** Milestone 49 (users, login, and attribution) reached `BUILT` on
  2026-08-27. Milestone 152 (durable delegation) cleared its own gate the same day, and its design is
  not merely named but worked out and ratified: [DECISIONS §108](../decisions/108-credential-revocation-kills-durable-session.md),
  [§122](../decisions/122-durable-schedule-store-format.md),
  [§123](../decisions/123-boot-time-rederivation-privilege.md) and
  [§125](../decisions/125-durable-schedule-manifest.md) are all `DECIDED`, and three of 152's four
  design pieces are built and gated on both ISAs (`smb_server`'s `DurableSession`, kept alive past a
  disconnect by §16's live-children rule; `crates/schedule_store`, the on-disk per-identity schedule
  and its manifest; `user/src/session_reviver.rs`, boot-time re-derivation of both).

  **So the question this bullet was held on is answered**: the registrar is a user's own durable
  login session, handed a `Held` narrower than the scheduler's own, which `Registry::register(doc,
  held)` has always taken as a parameter. §92 is not violated, because 152's answer is that the
  client a scheduled job's authority is supervised by is the session, not the connection.

  **What is left under this heading is smaller than what was held, and is build work plus two
  residual asks**, both worth naming so they are decided rather than discovered:

  - **The registration wire format**, if the registrar and the scheduler are separate programs.
    `Registry::register` is an in-process call and needs no opcode; a session registering into a
    running `timetable` does, and whether a call carries one entry or a whole document is the kind of
    thing two programs agree on, so it is calef's under AGENTS.md rather than a lane's.
  - **Removal.** The original ask read "add *or remove*", and 152 answers removal only as a cascade:
    revoking a user's credentials kills the durable session and everything derived from it (§108).
    Deregistering one entry, by the user who registered it, while the session lives, has no answer
    yet, and it is the half a person actually meets.

  One mechanical consequence, not a decision: `DurableSession` is private to
  `user/src/smb_server.rs`, so a registrar in any other binary means lifting it into a crate, whose
  name is calef's like every other. #387's `--mem` grant (built, and the whole of that pull request)
  was never affected by this hold.
- **Calendar syntax, wall-clock entries, persistence.** Each its own later decision, per the scope
  note below, and none of them started.

**In brief.** Unix cron is a daemon that reads a text file and runs arbitrary commands as
ambient authority made periodic: whatever root's crontab says, happens, and the crontab is the
attack surface. The capability shape inverts it: a scheduler service holds a clock capability
and, per entry, exactly the grant expression that entry's program is endowed with, checked at
registration the way the shell checks a command line at the prompt (milestone 31). An entry
cannot name what its manifest does not; compromising the scheduler yields the entries' summed
endowments, not the system.

## The shape, sketched for whoever scopes it

- **An entry is a manifest plus a schedule.** The schedule vocabulary starts embarrassingly
  small: every N seconds, and at-boot. Calendar cron syntax (minute/hour/day fields, its DST
  ambiguities) is a later decision, not a default; the housekeeping the backup server needs is
  interval-shaped.
- **The service holds one clock capability** and subscribes to §43's monotonic time; wall-clock
  scheduling waits for a decision about what a wall-clock entry should do across an NTP step,
  which the era-pivot work (`ntp_proto`) already gives vocabulary for.
- **A fired entry is an ordinary spawn** through the existing verbs, supervised per §40: a
  scheduled child that dies is reaped like any other, and the entry's failure count is state the
  service reports rather than hides.
- **Registration is the security boundary.** Whoever can register an entry can make its grants
  periodic, nothing more. Who may register is itself a capability question the scoping should
  answer deliberately (the boot endowment? the shell? both?).

## Scope note

Sequenced by need, not dependency: nothing blocks starting it today, but its first real customer
is milestone 55's housekeeping, so scoping it before 54/55 take shape risks building the wrong
verbs. The honest first deliverable is the interval scheduler running a no-op heartbeat program
under supervision on both ISAs, with the registration story decided; calendar syntax, wall-clock
entries, and persistence of the entry table across reboot are each their own later decision.

## BUGS

- The name is a placeholder in the oldest tradition ("cron"), and the eventual program name is
  calef's like every other (AGENTS.md). This file deliberately does not propose one. **The 2026-08-18
  lane shipped `timetable` as a provisional name** for the crate, the program and the document, said
  so in every module header, and recorded what it refused (`cron`, `almanac`, `metronome`, and
  `scheduler`, which this tree already spends on `kernel/src/sched.rs`).
- Persistence is unaddressed: entries registered at runtime die with the boot. Fine for a
  heartbeat, wrong for a backup server's housekeeping; the persistence story probably belongs to
  whatever milestone gives services durable configuration at all, which does not exist yet.

- **The document is compiled in, not read from disk**, which is the limitation
  `user/src/mdns_responder.rs` records and has the same fix (a `FileSpec` grant plus an `fs_proto`
  open-and-read at startup, milestone 131). It is load-bearing here in a way it is not there, because
  where the document lives is also what answers "who may register".

- **At most one `--mem` instance may be outstanding at a time**, recorded in full in
  `user/src/timetable.rs`'s own `BUGS`. The scheduler is fully blocked, unable to fire anything else
  in the document, for as long as that one instance takes to die and its grant to be reclaimed. A
  document whose `--mem` entry competes with a fast interval for the clock pays that cost as a late
  fire rather than a dropped one; the shipped document does not exercise it.
## Follow-on

- **Outstanding.** One image per entry still needs machinery nothing has: `user/src/spawner.rs` is
  the right shape, one budget and one image behind a request channel, but `user/src/root_supervisor.rs`
  builds exactly one of it, so nothing spawns sub-builders sized to a document. Checked 2026-09-03.
- **Outstanding.** The registration wire format is unbuilt and undecided: `crates/timetable` has no
  opcode and no register constant, and no record under `design/decisions/` covers a session
  registering into a running scheduler. Checked 2026-09-03.
- **Outstanding.** Deregistering one entry while the session lives still has no answer.
  `crates/schedule_store` carries no removal verb, and §108's credential-revocation cascade is the
  only removal in the tree. Checked 2026-09-03.
- **Done.** The private-session-type obstacle went with its file: the SMB server was removed on
  2026-08-30 and only historical mentions survive in `user/src/session_reviver.rs`, so nothing has
  to be lifted into a crate any more.
- **Done.** Persistence is no longer unaddressed: milestone 152 built `crates/schedule_store`, the
  per-identity on-disk schedule in the timetable's own document format, plus its manifest (§122,
  §125) and boot-time re-derivation in `user/src/session_reviver.rs` (§123).
- **Outstanding.** Wiring that store into a running scheduler has not happened: nothing in
  `crates/timetable` or `user/src/timetable.rs` mentions the store, so entries still die with the
  boot in the scheduler itself. Checked 2026-09-03.
- **Outstanding.** Calendar syntax has not started. `crates/calendar` is milestone 51's civil-date
  arithmetic and supplies the vocabulary, and the timetable's grammar is still every-N-seconds and
  at-boot. Checked 2026-09-03.
- **Outstanding.** Wall-clock entries have not started, and the reason this block found by building
  still holds: the scheduler holds no clock capability at all, so it has none to give an entry.
  Checked 2026-09-03.
- **Recorded.** The document is still compiled in (`user/src/timetable.rs`), and where it lives is
  also what answers who may register. The pointer this block gives to milestone 131 for the fix is
  dead: that block is NOT-STARTED and its subject was removed on 2026-08-30.
- **Milestone 106.** The absence of a timed wait still costs a core's worth of yields and lazy
  reaping. The registry already computes the instant a timed wait would block until.
- **Recorded.** At most one memory-budgeted instance may be outstanding, with the cost paid by
  every other entry as a late fire, recorded in full in `user/src/timetable.rs`'s own `BUGS`.
- **Recorded.** `timetable` is a provisional name for the crate, the program and the document, said
  so in every module header, with `cron`, `almanac`, `metronome` and `scheduler` recorded as
  refused.
