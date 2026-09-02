# 226. `qemu-bounded.sh` leaves an emulator behind, and the next run blames the wrong thing

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from milestone 127's EL2 entry lane,
which hit it twice in one session. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** A shell script and its process handling.

**In brief.** `scripts/qemu-bounded.sh` exists because `timeout(1)` does not exist on macOS and
`perl -e 'alarm N; exec @ARGV'` does not work on QEMU, which installs its own `SIGALRM` handler and
swallows the alarm. The script uses `SIGTERM`, which QEMU honours, and detaches its killer so it
survives a pipeline whose reader exits early.

**It does not survive its target detaching.** When the EL2 lane's run was killed, the emulator
survived with `ppid 1`. It then held the write lock on `nifefs-blank.img`, and **the next boot failed
with `Failed to get "write" lock`**, which names nothing about the real cause. The lane lost time to
it twice before recognising the shape.

## Why this is worth a block rather than a habit

AGENTS.md already narrates this failure from the other direction, at length: killing a loop script
does not kill its descendants, so a check honestly reporting "no qemu" is followed by a command that
finds one. Its advice is to ask who holds the file (`lsof target/nifefs.img`) rather than whether a
process matches a name, and to kill the tree at its root.

**That advice is rung four**, and this is the instance where rung four failed in the ordinary way: a
lane that had read the file still lost time, because the symptom arrives on a *later* run as a lock
error about a file, with nothing pointing at a leaked process.

Every lane uses this script. A leak here is not one lane's problem, it is the next lane's mysterious
failure, and this project runs many lanes.

## What it needs

**An emulator that cannot outlive the thing that started it**, or a failure that names the cause when
one does. Two shapes, and the block picks neither:

- **Make the leak impossible.** A process group killed as a group, or a supervisor that dies with its
  parent. Stronger, and it must not break the reason the killer is detached in the first place, which
  is a real requirement rather than an accident: a pipeline whose reader exits early must still get
  its emulator killed.
- **Make the symptom honest.** Whatever reports `Failed to get "write" lock` could say which process
  holds it, since `lsof` answers that in one call. Weaker, and it does not stop the leak, but it
  turns a mystery into a sentence.

**These are not exclusive and the second is worth having regardless**, because a lock can be held by
something this project did not start.

## BUGS

- **This block does not say how often it happens.** Two occurrences in one lane on one day is what
  prompted it, and nobody has looked for others.
- **The macOS constraint that produced this script is unchanged.** Any fix must keep working without
  `timeout(1)` and against a QEMU that swallows `SIGALRM`.
- **It says nothing about the emulators a killed harness leaves**, which AGENTS.md records as a
  separate and larger failure: eleven QEMU processes over one day, the oldest with eight hours of CPU
  time on it.
