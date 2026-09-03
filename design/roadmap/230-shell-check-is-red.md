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

## What the gate caught on its first CI run, which is the second half of this milestone

Adding `script/shell-check` to `script/ci-build` made it fail immediately on the riscv64 leg (run
33702132439), on a tree whose local runs were green on both architectures. **The divergence is
real** and is a better argument for the CI step than the cost argument above.

The check reported that init never dropped its construction budget. The transcript it printed
contained the line it said was missing:

```text
  user thread 17 killed: scause 0x3 (code 3)
    pc 0x0000000000406aa0   stval 0x000000000i0406aa0   usern sp 0x0000000000500da0
it:   cthe kernel is fine.
onstruction budget dropped; retype answers NoSuchSlot
```

**Nothing was truncated and no byte was lost.** `init: construction` is spliced through the
kernel's register line one and two characters at a time, and every byte of both writers is present
and in order. The kernel prints a user fault report with its own UART driver; the userspace
`console` server drives the same UART from its own address space; nothing arbitrates between two
processes writing one device. So a thread dying while init prints shuffles the two lines together.

This is **not** the partial-line problem `crates/board_console` solved for milestone 216 (nothing in
this tree can read a board, so every hardware milestone waits on a person). That crate's
`observe_partial` is about a line the reader has not finished receiving, and its answer, `contains`
on a completed line, is exactly what fails here.
Its distinction is worth keeping and does not apply; the reader in `shell_check_leg` already
accumulates, and accumulating more would not have helped.

It also happens on **every** local riscv64 boot. The interleaving simply landed somewhere harmless
there, which is why local was green: whether the shuffle lands on a line the gate reads is timing,
and a runner is not this laptop.

Three things changed as a result:

- **The matcher tolerates it, under a signature rather than a budget.** `find_marker` accepts a
  marker whose characters are interleaved only when the text wedged into it carries the kernel's own
  fault-report signature, and only for the two boot markers rather than for the typed-line echoes. A
  first version used a character budget alone and accepted `construction budget NOT dropped` as
  `construction budget dropped`; a second let a match start inside the kernel's report and swallow
  the signature. Both are pinned as tests, with the real CI transcript as the fixture.
- **The diagnostic says what it knows.** It used to assert two capability states ("it still holds
  the kernel's root untyped, or the delete did not take") on the evidence that a string was missing,
  which sent a maintainer looking for a bug that does not exist. It now names the marker, reports
  the longest run of it that did appear, and asserts nothing about the kernel. An interleaved match
  passes but says so on stderr, because a transcript that needed the tolerance is itself evidence.
- **The dying thread turned out to be a real defect**, below.

## `login` does not run, and this gate is why anyone knows

The thread whose death caused the shuffle is `login`, and it dies on **every** interactive boot on
**both** architectures. Measured rather than deduced: instrumenting `login::fail` to fault at
`0xFA11_0000 + step` gives `far 0x00000000fa110001`, which is step 1, `nifefs::Fs::parse` refusing
the archive. `_start` reads the archive from `initrd_len` in `a1`;
`crates/system_initializer` starts it with `thread_control_block_start(login_tcb, 0, 0, 0)` and
endows it with no mapping of the archive, so it gets a zero-length slice.

The boot still prints `init: login ready` with a generated password, because init measured the
identity provisioning rather than this process's survival. **So a green `script/shell-check` does
not currently mean the login service runs**, and that is recorded in `user/src/login.rs`'s own
`BUGS` where a reader meets the program.

Not fixed here. Handing `login` the archive costs `crates/system_initializer` capability slots at
the exact peak this milestone has just measured and sized the table against, so the fix and the
accounting move together and want a lane of their own.

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
- **The gate does not fail on a killed user thread**, which is how `login`'s death went unnoticed
  through a passing run. Adding that assertion is the obvious next ratchet and is deliberately not
  added here: it would be red on both architectures until `login` is wired, and this lane could not
  test what the typed script does when a command traps on purpose. It belongs with the `login` fix.
- **The interleaving is worked around rather than removed.** Two processes writing one UART with
  nothing arbitrating is the actual defect, and `find_marker` is a reader coping with it. The
  kernel's fault report has to reach a person somehow, so where it should go once userspace owns
  the console is a design question rather than a patch.
- **The bisect is over first-parent merges**, so it names PR #556 rather than a commit inside it.
  `d1c81062` is identified by reading that branch, not by a boot at that commit.
