# Five crates whose doctests the host gate never runs

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 68's block.

**Gate: NONE.** No decision is owed and nothing is missing. The split is ordinary refactoring inside
crates this tree owns, and the gate that would prove it is the one already running.

**In brief.** `swap_proto`, `virtio`, `supervision_proto` and `system_initializer` each take an
unconditional dependency on `user_rt`, which is EL0 syscall `asm!` and cannot compile for the host.
`--exclude` removes a package from the test selection but not from the dependency graph, so all four
are excluded by name in `xtask`, along with `user_rt` itself. Split each one so the pure logic lives
where the host can build it and the syscall half is what depends on `user_rt`. Then the host pass
runs their tests and their doctests instead of skipping them.

## Why this matters

Milestone 68 exists to be the gate that keeps documented examples honest. Inside that gate sit five
crates whose examples nothing ever compiles, so an example there can go stale and no check will say
so. That is the exact failure the milestone was built against, living in the milestone's own
blind spot.

The tree has already paid for this class of gap once, and the block records the bill. When five
crates went missing from the host selection by milestone 51, `filesystem_proto`, `compositor`,
`video_terminal`, `bitmap_font` and `grant_plan` carried **82 host tests the gate never ran**. All
82 passed when they were finally run, which is the point: nothing failed, so nobody noticed, and a
gate that quietly covers less than it claims is worse than no gate because it is trusted.

There is a second reason, and it is the reason the exclusion list exists at all. On 2026-08-03 those
unconditional dependencies broke the host build on x86_64 and nobody saw it, because CI had moved to
`ubuntu-24.04-arm` the same day and an aarch64 host builds `user_rt` by accident. A stranger with a
clean x86_64 checkout found it eleven days later, on milestone 117's first run, which is principle 3
failing in the only way it can be observed. `script/lint` now derives the exclusion set from `cargo
metadata` so the next crate to take a `user_rt` dependency breaks the gate rather than the host
build. That is a tripwire on the growth of the problem, not a fix for it.

## What it would take

Five crates, each a separate piece of work, and they are not equal. `swap_proto` and
`supervision_proto` are protocol crates whose pure half is message layout and whose syscall half is
the `CALL`, which is the cleanest shape. `virtio` and `system_initializer` carry more. The measure
of success is mechanical and already automated: a crate leaves the `--exclude` list in
`xtask/src/main.rs`, `script/lint`'s derived-set check agrees, and the host pass runs its doctests.

## Where it came from

Milestone 68's block: *"Split the pure half from the syscall half in `user_rt`, `swap_proto`,
`virtio`, `supervision_proto` and `system_initializer`. Each takes an unconditional `user_rt`
dependency, so the host test selection excludes it and nothing in CI ever runs its doctests. That is
five crates whose examples can rot unnoticed inside the gate milestone 68 exists to be."*

The exclusion and its history are commented at the host-test invocation in `xtask/src/main.rs`.
