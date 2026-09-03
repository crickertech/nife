# 230. `script/shell-check` is red on `main`, on both architectures, and nothing says so

**Status: BUILT 2026-09-02.** Minted the same day by the maintainer, from milestone 192's (a
keyboard on real silicon) lane, which found it while proving an unrelated boot path. *(Number
provisional until the merge queue lands it.)*

It was minted with no gate, on the grounds that the defect was reproducible on patagonia and needed
nothing this project does not have. That held: it was reproduced, root-caused, bisected to a merge,
fixed, and `script/shell-check` is green on both architectures.

**In brief.** With a virtio-rng attached, which both plain legs set unconditionally via `NIFE_RNG=1`,
the interactive boot **traps in init at `user_rt::trap` with no message**. The same build with the
device absent reaches a prompt normally. The cause is capability-slot exhaustion in
`crates/system_initializer`, and the reason nobody knew is that `script/shell-check` ran in neither
`script/test` nor CI.

## What was actually wrong

The reported hypothesis was the sixteen-slot wall that this file's own entropy-block comment
describes. **The wall held; the location did not.** Init does not run out of slots building the
entropy service. It runs out four blocks later, at
`let verify_page = must(retype_page_frame(ut))` in the login stack, building `credentialer`. The
entropy service is only the reason that block executes at all: `have_login_stack` requires
`entropy_client.is_some()`, and nothing else in the boot sets it.

Established by instrumentation rather than by reading, in four boots:

1. The kernel's fault line names `user_rt::trap`, which is one address shared by every failure in a
   program, so it identifies nothing. Making `system_initializer::fail` take a *data* abort at
   `return address + 0x1_0000_0000` put the caller in `far`, which is printed. That named
   `crates/system_initializer/src/lib.rs:1430`.
2. `MemoryRegion::RETYPE` answers `OutOfMemory` both when the region is out of pages and when the
   capability table is full. Splitting the two with a temporary `println!` said **capability table
   full**.
3. Dumping init's table at that moment showed all seventeen slots occupied.
4. Raising `CAPABILITY_TABLE_SLOTS` to 40 and printing the highest slot ever granted measured the
   boot's real high-water mark: **21**, in init, while `build_child` lays `credentialer` down.

## How far back it goes, and why the commit that broke it looked correct

Bisected over the 105 first-parent commits between milestone 49's login-stack wiring and `main`,
seven boots. The first red merge is **PR #556** (`a731b4ce`, milestone 49's boot terminal
attribution), landed **2026-08-28**. `main` was red for five days.

The commit inside it that did the damage is `d1c81062`, *"cap: `CAPABILITY_TABLE_SLOTS` is 17, not
the 28 the bisection left behind"*. Milestone 49's lane had set the constant to
`28 // TEMP: generous bisection value` while chasing an unrelated login-suite flake, and then built
and shipped the login stack against it. The cleanup restored 17 on two true observations: the doc
comment carefully justifies 17, and a full `script/test` on all three architectures is green at 17.

Both are true and neither could see this. **No suite in `script/test` boots the real init**; every
test that runs a shell has the kernel play that role. The only gate that runs
`crates/system_initializer` is `script/shell-check`, and it ran nowhere. So a lane verified the
change with the most thorough thing available to it, was right about what it checked, and shipped a
system that could not boot.

## What was done

- **`kernel::cap::CAPABILITY_TABLE_SLOTS` 17 -> 24**, and `abi::CAPABILITY_TABLE_SLOTS` with it.
  Measured 21 plus three of headroom: at 32 bytes a slot that is 96 bytes a thread and 24 KiB across
  `MAX_THREADS`. Every previous raise took the number to exactly what that day's boot needed and
  every time the next addition hit the wall in the same silence, so the margin is the point rather
  than a rounding.
- **`script/shell-check` now runs in `script/gates` and in CI**, and the reasoning for each is in
  the file that carries it. It is 30 seconds for both architectures against a warm target directory,
  measured, which is why it belongs in the command a person runs before pushing. In CI it is
  appended to `script/ci-build` rather than given a job of its own: a separate job would repeat that
  job's ten-minute build and claim another runner to do thirty seconds of work, and runner
  concurrency is a real ceiling here.
- **The `BUGS` sections that described this as a mystery now describe it as a fact**, in
  `crates/system_initializer` and beside the constant in `kernel/src/cap.rs`.

## BUGS

- **A fault line still cannot say where a program died.** The kernel prints the `pc` of the
  breakpoint inside `user_rt::trap`, which is one address per program however many callers it has.
  The return address is in `x30`/`ra`/on the stack at the moment the kernel takes the fault and is
  not printed. That is what made the first day of this milestone a day rather than a boot, and it is
  recorded in `user_rt::trap`'s own `BUGS`. Fixing it is a change to the three
  `kernel/src/arch/*/exceptions.rs` fault printers, so it wants a lane of its own.
- **Nothing counts init's capability slots.** The high-water mark that decides whether this whole
  class of failure recurs is invisible from inside a boot and was measured here with temporary
  kernel `println!`s that are now reverted. The three slots of headroom are what stands in for a
  mechanism.
- **The two graphical legs remain red** behind milestone 177's (attaching a GPU and keyboard to the
  real boot) display driver bug. `script/shell-check` with no arguments is green; `--graphical` and
  `--graphical-serial` are not, and neither `script/gates` nor CI runs them.
- **The bisect is over first-parent merges**, so it names PR #556 rather than a commit inside it.
  `d1c81062` is identified by reading that branch, not by a boot at that commit.
