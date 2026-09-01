# 209. A session starts in a fit state, or says which part is not

**Status: NOT-STARTED.** Minted 2026-08-31 by calef, after a day in which the machine slept twice and
killed two lanes mid-response, the `launchd` watchers were found unloaded, and the toolchain link
pointed into a worktree about to be pruned. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Every check it makes is a command that already exists; what is missing is anything
that runs them together and says which one is false.

**In brief.** Several things must be true for work to proceed safely, and each is currently checked
by somebody remembering. calef, 2026-08-31: *"Shall we make it a standard to caffeinate when a
session is started... Seems like maybe that should be built into a program of some sort."*

This is the operational half of what `script/catch-up` already does for information. That script is a
computed view of what changed; this is a computed view of whether the machine is fit to work in.

## What it checks, and every one of these failed today

| | what goes wrong | how it was found today |
|---|---|---|
| **Sleep is held** | an unattended lane dies mid-response and its uncommitted work is one prune from gone | two lanes killed in an hour; a 138-line enumeration and a real `rmle` overflow bug were untracked when it happened |
| **The watchers are loaded** | the queue stalls and trunk goes red with nobody assigned | `launchctl list` was empty five days after the plists were installed to make remembering unnecessary |
| **`nife-dev` points at the main checkout** | pruning a worktree breaks the toolchain account-wide, surfacing far from the cause | it pointed into a lane's worktree that was about to be pruned |
| **No leaked QEMU** | emulators accumulate and hold `target/nifefs.img`, failing unrelated tests | AGENTS.md records eleven leaked over one day, 729% CPU |
| **No unclaimed lane branch** | two lanes can take one milestone | three of five lanes today pushed a branch and opened no draft pull request |
| **Disk and load headroom** | the 2026-07-31 zero-bytes incident; today's OOM | load hit 83 on eight cores with three lanes and a mutation sweep |

## Everything here is session-scoped, and that is calef's correction to this block's first draft

The first version said machine-scoped things belong to `launchd` and the session only verifies them.
**calef, 2026-08-31, rejected that**: *"I don't want launchd without a means to disable all of the
environment if there isn't an active Claude session. The Claude session drives so much of the
remediation of issues."*

He is resolving a contradiction the record already carried rather than stating a new preference. On
2026-08-26 he declined an unattended scheduled agent for the merge drain, saying he *"would rather
this shut down when the session driving it does than run standing on a timer with nobody watching."*
**The same day, the watchers moved to `launchd`, which does precisely the opposite.** That move was
made for a good reason, a session having read the instruction and not acted, and it chose
machine-persistence over the stated preference without anybody noticing the two disagreed.

**The tension is not theoretical.** `scripts/merge-drain.sh`'s `notify()` posts once per stall and
then goes quiet by design, so a stall found with no session running lands in a pull request comment
nobody reads. That is the steward failure AGENTS.md already records in its own words: *"it reported
and never acted."* A watcher whose remediation requires a session, running when no session exists,
manufactures exactly that.

### The shape that satisfies both

The reason `launchd` was adopted is worth keeping: **nothing should have to remember to start these.**
The reason to scope them to a session is equally worth keeping: **nothing should run when nobody can
act on what it finds.**

Both hold if the `launchd` job stays loaded and **each invocation first asks whether a session is
alive**, exiting silently when none is. A heartbeat the preflight refreshes is enough: the watchers
no-op once it goes stale.

**A heartbeat beats a teardown**, and this is the design point worth defending. A session that ends
cleanly could unload its jobs; a session that is killed by an out-of-memory event or a sleeping
machine cannot, and both happened on 2026-08-31. A staleness check needs no cooperation from the
dying session, which is the only kind of cleanup that survives the failures this project actually
has.

`caffeinate` follows the same rule for the same reason, and needs no heartbeat because it dies with
the session that holds it.

## What it must not do, and each of these has bitten

- **Never kill a QEMU it finds.** AGENTS.md's rule is to walk `ps -o pid,ppid` **upward** first,
  because a QEMU whose parent chain ends in a live harness is somebody's gate in flight; the
  maintainer killed a lane's mid-suite emulator this way on 2026-08-15. **Report, never reap.**
- **Never relink `nife-dev` while a gate is running.** That is the same race in the other direction,
  the one that took the link out from under a lane on 2026-08-04. Detect and refuse rather than fix.
- **Never start a second `caffeinate`.** Check the assertion (`pmset -g assertions`) rather than the
  process, since a timed one from elsewhere may already be holding it and will expire.
- **Never load a `launchd` job silently.** Say it is unloaded and how to load it.
- **Never let a watcher act with no session alive.** That is the point above, stated as a
  prohibition: the heartbeat is what enforces it, and a watcher that runs anyway is worse than one
  that does not run at all, because its silence reads as "nothing is wrong".

## The rung it should reach

A script is rung two: it fires only when run. **The thing that makes it fire without anybody
remembering is a `SessionStart` hook in `.claude/settings.json`**, checked into the repository, which
is rung one for the sessions this project actually runs in.

That is worth doing and is not sufficient on its own: the script must stand alone for a person at a
prompt, for another tool, and for a machine that is not calef's. The hook is how it gets invoked, not
what it is.

## BUGS

- **This is operational hygiene and is on no fatal risk's path.** It buys back attention and prevents
  losses; it moves none of the nine.
- **The heartbeat is a new thing that can be wrong.** A stale one silences the watchers when work is
  in flight, and a fresh one left by a crashed session lets them run unattended for its lifetime.
  Whatever window is chosen is a guess until somebody measures how long a session's gaps actually
  are.
- **A preflight that reports six things nobody reads is worse than none.** The output has to be
  silent when everything is fine, which is `merge-drain`'s own posture, or it becomes noise within a
  week.
- **It cannot check the thing that actually costs most**, which is whether a lane is committing as it
  goes. Twice today uncommitted work survived only because a worktree did.
- **The name is provisional.** calef names things, and this one is awkward: it both reports and acts,
  so it sits between `script/`'s noun-shaped reporters and its verb-shaped doers.
