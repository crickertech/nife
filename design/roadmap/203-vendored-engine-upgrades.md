# 203. Nothing will ever tell us RedoxFS moved

**Status: NOT-STARTED.** Minted 2026-08-31 by calef: *"a regular process to check if RedoxFS has
changed and if it does then initiate work to update our version and our divergences."* *(Number
provisional until the merge queue lands it.)*

**Gate: NONE.** The detection half needs nothing that does not exist. The one fork below is about
what the check produces, and it carries a recommendation rather than blocking a start.

**In brief.** The vendored engine is pinned at `redoxfs` 0.9.1 and **no mechanism in this tree will
ever report that a newer version exists.**

- `script/vendor-verify` proves the pin is what we *say* it is, byte-identical to the published
  tarball plus the divergence patch. It never asks what upstream has published since.
- **Dependabot cannot see it.** `vendor/redoxfs` is deliberately its own workspace, kept out of the
  nife workspace so the `-D warnings` clippy gate and `cargo fmt` never touch upstream code, so it is
  outside the graph dependabot watches. `.github/dependabot.yml` does not mention it.
- DECISIONS §81 (a dependency stays reachable by an upgrade) governs exactly this failure and covers
  only the Cargo graph. **`vendor/` is §81's blind spot**, and the same argument applies: an upgrade
  nothing surfaces is an upgrade nobody takes.

## Why the gap costs more here than for an ordinary dependency

`vendor/README.md` records **five divergences**, and says of three of them that they are *"re-applied
forever, can conflict on a pin bump."* So the cost of a bump grows with the gap, and nothing is
counting the gap. That is the shape where waiting is silently expensive.

The narrow case that actually matters: **a correctness fix upstream, in the engine holding backups.**
That is the one where not knowing is not merely untidy.

## What it has to do

1. **Detect.** Compare the version in `vendor/redoxfs.pin` against what the crates.io index carries.
   Small, and it belongs beside `script/vendor-verify`, which already owns the pin.
2. **Run on a schedule that does not depend on anyone's laptop.** A GitHub Actions cron rather than
   the `launchd` watchers on patagonia, which is the one place this differs usefully from
   `notes/merge-queue.md`'s pair: calef accepted that a sleeping patagonia means nobody is watching,
   and that gap is fine for a merge queue measured in hours and wrong for a check measured in months.
3. **Produce something a person acts on**, not a log line nobody reads.
4. **Carry the update procedure**, so the person acting is not reconstructing it: bump the pin,
   re-apply the divergences, regenerate with `script/vendor-verify --write-patch`, extend
   `vendor/README.md`'s exhaustive list, and run the suite plus milestone 37's crash injector, since
   the store's safety claim is the thing a bump risks.

## The one fork worth deciding rather than assuming

**What step 3 produces.**

- **An issue.** Cheapest, and this project does not read issues; the roadmap and pull requests are
  where work lives. It would be the rung-four answer.
- **A draft pull request that bumps the pin and lets `script/vendor-verify` fail**, which turns "look
  at this some day" into a visible object with a red check on it. Matches how a lane already claims
  work (§90's draft-first rule).
- **The same, with the divergence patch re-applied where it applies cleanly**, so the pull request
  arrives saying which of the five carried over and which conflicted. Most useful, most work, and it
  is what dependabot does for an ordinary crate.

**Recommend the second**, and let the third arrive if the first bump proves the re-application is
mechanical. The value is in the upgrade being visible and unignorable; automating the patch rebase
before anyone has done one by hand is guessing at the shape of a job nobody has performed.

## BUGS

- **It reports; it cannot decide.** A newer version is not automatically a better one, and this tree
  pinned deliberately. The output is a prompt, and a prompt nobody acts on is the same silence with
  more steps.
- **One consumer.** RedoxFS is the only vendored engine, so this generalizes to `vendor/` on paper
  and is really about one directory. Writing it as a general mechanism would be abstraction ahead of
  requirement.
- **The crates.io index is not the only upstream.** `vendor/README.md` pins a git sha as well, and a
  fix can exist upstream in git long before it is published. Checking only the published version
  answers the smaller question, and the block does not attempt the larger one.
- **A bump is where the divergences get re-litigated**, and divergence 3 in particular is one
  `vendor/README.md` says is kept rather than reverted only because reverting it is calef's call.
  Any bump lane will meet that.
