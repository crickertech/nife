# 408. One home for `fn check(ok: bool)`, which nine programs now write out by hand

**Status: PROPOSED 2026-09-14.** Filed by milestone 291's lane, which added seven of the nine
copies and is saying so rather than leaving the count to be re-derived.

**Gate: DECISION.** The obvious home is `crates/user_rt`, and a public function name there is
calef's (AGENTS.md, "calef names the crates, the programs, and the shared modules", extended to
public function and method names on 2026-08-23).

## The duplicate

Every one of these is the same three lines, and they all mean *this program's only way to say no is
to die where the mistake was, because a failed check must be indistinguishable from a broken
program*:

```rust
fn check(ok: bool) {
    if !ok {
        user_rt::trap()
    }
}
```

Nine copies as of milestone 291: `components/src/block_driver.rs` (which panics instead, reaching
the same trap through `panic_handler!`), `fixtures/src/fs_test_client.rs`, and the seven fixtures
291 created (`console_test_client`, `memory_region_depleter`, `page_frame_producer`, `call_server`,
`frame_revoker`, `rendezvous_minter`, `rendezvous_peer`). `crates/loaded_image_check` has a tenth,
private, for its own use.

**This is not a `#[path]` module and `script/lint` check 5 will never see it**, because nobody
shared a file: each copy was typed again. That is the cheaper failure of the two and it is still a
failure, since the nine can drift on what "say no" means (two of them already do: a panic and a
trap are not the same path, though they arrive at the same instruction).

## Why it is a proposal and not a change

**291 chose the duplicate deliberately**, because the alternative was adding a public function to
`user_rt` in a lane already touching two archive tables and the filesystem's directory geometry,
and because `user_rt` is the crate every program in the tree links. The existing two copies were
the tree's established pattern; following it kept the change reviewable. The argument for fixing it
is that "fewer places to be wrong" is AGENTS.md's own definition of elegance, and nine is past the
point where a pattern is a convention.

**The name is the whole decision.** `check` is one of the generic words AGENTS.md names as a failure
mode: half the tree checks something. `require` is the kernel's own word for the same shape
(`trust::require` halts on a mismatch). `insist` and `must` were considered and are worse for
opposite reasons: the first is unusual enough to need explaining, the second reads as a modal verb
at the call site. A public name on `user_rt` is also the most-read function name this tree could
add, which is a reason to spend a decision on it rather than to take one.
