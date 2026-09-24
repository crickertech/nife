# What milestone 63 did not rename

*An appendix to [`design/naming.md`](../naming.md), which is the rule. This file holds the names milestone 63 left in place on purpose, so the next reader does not "fix" one by mistake. It exists to verify or challenge the main page, and a reader who only needs to name, ratify or rename something should not have to open it. The directory `design/naming/` and this file's stem are provisional names, minted 2026-09-24 by the lane that split the file; naming is calef's.*

## BUGS

What milestone 63 did **not** rename, each on purpose, so the next reader does not "fix" one of them
by mistake.

- **Two records under `design/` still spell names milestone 175 retired, and one of them is a
  present-tense claim.** `design/capsicum-and-the-retrofit-question.md`'s honest comparison says the
  system confines "`worker`, `budgeter`, `heeder`, `spinner`, a C component, and a filesystem we
  vendored", and `AGENTS.md`'s rule 7 section describes `user/src/` as a live directory. Both were
  left where they are because a developer lane edits its own milestone's roadmap block and nothing
  else under `design/`, and never `AGENTS.md`. Every *other* occurrence of the old names in
  `design/` is a dated narrative and correctly keeps them. Neither is load-bearing; both are one
  line for whoever next has the standing to make the edit.

  **Half of the first one closed on 2026-09-13**, and the way it closed is the point rather than the
  tidiness. The `memory_grant_depleter` rename lane was already editing that sentence's `budgeter`,
  because a present-tense claim moves whatever directory it sits in, so the word it was there to
  correct went with the sweep that had to touch the line anyway. `worker` is still there, and this
  entry is still open for it: performing *that* ruling was a different lane's, and a rename is not a
  thing to do on the way past. The general shape, worth more than either word: **a stale record gets
  fixed when something else brings a writer to the line**, not when somebody schedules a pass over
  it, which is why the entry names the line rather than filing a task.

- **The boot mode is still called `shell`, and the program is `swish`.** `cargo xtask shell` and the
  kernel's `--features shell` name a *configuration* (boot straight to a prompt, milestone tour
  compiled out), not the binary. Renaming them would have been a naming decision nobody made. The
  cost is that a reader who greps `shell` in `xtask` and in `kernel/Cargo.toml` meets a word that no
  longer names a program.
- **`caps` is still a shell builtin**, and it no longer shares a name with anything. It was the
  larger half of the 285 occurrences of `caps` in the tree before the rename, and it means "print
  this process's endowment". The crate that used to share the spelling is `capability`.
- **`crates/virtio`'s first line still says "A virtio-blk driver".** It also drives net, serves
  blocks through `run_blk_server`, and carries two deliberate attack roles. The crate keeps its name,
  which is right, but the sentence under it is wrong. Out of scope for 63 and not yet filed anywhere
  else.
- **A rename can reach into the vendored tree, and `script/supply-chain` is what says so.** Our one
  divergence comment in `vendor/redoxfs/Cargo.toml` names `redoxfs_host`, so the rename had to touch
  the vendored file **and** `vendor/redoxfs.divergence.patch` together or
  `script/vendor-verify` fails with "differs from upstream+patches". Milestone 63 changed the patch
  first and the vendored file not at all, and the gate caught it. Edit both, in the same commit.
- **The measured-boot manifest is still `target/init-measure-<arch>.txt`.** It is a build artifact
  name, not a crate reference; `kernel/build.rs` reads it and turns it into `TRUST_ROOT`.
- **Note filenames did not move**, and that is the rule rather than an oversight:
  [fs-server.md](../../notes/fs-server.md), [shell.md](../../notes/shell.md), [shell-navigation.md](../../notes/shell-navigation.md) and
  [line-discipline.md](../../notes/line-discipline.md) are markdown, so they stay lowercase-hyphenated even
  though the things they describe are now `redoxfs_server`, `swish` and `line_editor`.
