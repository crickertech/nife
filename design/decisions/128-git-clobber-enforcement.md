# 128. What enforces the git-clobber rule, now that it has crossed its own threshold

**Status: PROPOSED.** Milestone 118's own violation ledger (`notes/rule-violations.md`) already
says the git-clobber rule sits at four open strikes, one past its own three-strike threshold, with
no enforcement mechanism decided, and names this "calef's or the integrator's call, not this
lane's." This entry lays out the real options, researched rather than guessed at, for that call.

## The real incidents, precisely

All three recorded rows trace to **one day, 2026-08-04**, not a spread pattern, and none since
(`notes/rule-violations.md`, last run 2026-08-22):

1. **The squash-against-`origin/main` scar.** A lane ran `git reset --soft origin/main` (instead of
   its own recorded base SHA) to squash checkpoints, and silently staged four other lanes' files as
   its own, including a deletion. Caught only by reading `git status` before committing
   (`AGENTS.md`, "Commits"). One instance.
2. **The aggregated "four agents" row.** `AGENTS.md`'s own "What it costs, measured 2026-08-05"
   section reports "four agents clobbered work with `reset --hard`, `checkout` or `stash`" on
   2026-08-04, as one summary count rather than four individually detailed incidents; this is the
   ledger row currently sitting past threshold.
3. **The `pkill -f` incident.** A lane killed another lane's QEMU mid-test. One instance, same day.

All agent-caused; none human-typed. **Zero repeats in the three weeks since**, which matters for
weighing whether this is an ongoing pattern or a one-day cluster that predates a convention
(`AGENTS.md`'s own git-safety section) written in direct response to it.

## What already exists to build on

`script/setup` already runs `git config core.hooksPath .githooks`, a shared, versioned hooks
directory covering every worktree from the one `.git` this project's multi-lane structure shares.
One hook exists there today: `.githooks/pre-push`, running `script/fmt --check` before every push
(bypassable with `--no-verify`, the same escape hatch this project already accepts elsewhere).

No `.claude/settings.json` exists in this repo. Claude Code's own Bash tool permission system,
which every incident above went through (100% agent-caused, via the Bash tool), is currently
unconfigured for this class of command.

## The options, sized rather than asserted

- **A git hook.** Checked against git 2.55.0's actual hook list: there is no `pre-checkout`,
  `pre-reset`, or `pre-stash` hook, only `post-checkout`, which fires after the damage is done. A
  literal "pre-command check via a git hook" is infeasible for the commands this rule is actually
  about. This collapses "hook" into the same mechanism as the next option.
- **A `git` shim on `PATH`.** Technically works, but `checkout` has a real ambiguity a shim cannot
  resolve by prefix alone: `git checkout <path>` (destroys uncommitted changes) and
  `git checkout <branch>` / `git checkout -b <branch>` (ordinary, constant, safe) share the same
  prefix. `reset --hard`, `clean -f`, and `branch -D` do not have this problem; each is an
  unambiguous, always-dangerous form.
- **An alias** (`git undo`, `git safe-reset`). Cheapest option, but purely opt-in: offers a safer
  spelling without ever stopping the dangerous one from still being typed, agent or human.
- **Claude Code's own Bash permission system** (`.claude/settings.json` deny rules). Not one of
  the ledger's original four options, but the one that matches where every incident actually
  occurred: the Bash tool call, not a raw shell. `reset --hard`, `clean -f`, and `branch -D` are
  cleanly denyable today with a settings entry and zero new tooling, since the mechanism already
  exists in the tool this project's agents already run through. `checkout <path>` carries the same
  ambiguous-prefix problem a shim has, so a deny rule narrow enough not to block ordinary branch
  switching cannot fully close that one case either.
- **Accept and mark `resolved` as unenforceable.** Given zero repeats in three weeks, a real
  question worth weighing rather than dismissing: does the incident rate reflect an ongoing risk,
  or a one-day cluster that predates the AGENTS.md convention written in direct response to it,
  already largely doing the job?

## Recommendation

**Deny `reset --hard`, `clean -f`, and `branch -D` in `.claude/settings.json`.** It is the cheapest
option that actually prevents rather than merely discourages, it targets the layer every real
incident went through, and it requires no new tooling, only configuration of a mechanism already
in place. It is also fully reversible (a settings entry, not a structural commitment), which
matters less for how much deliberation this decision deserves and more as a note that revisiting
it later costs nothing.

**Leave `checkout <path>` unresolved by simple denial**, and record that as a known, named gap
rather than a silent one: no mechanism surveyed (hook, shim, or Claude Code deny rule) can
distinguish the dangerous form from ordinary branch switching by prefix alone. `AGENTS.md`'s own
existing prose guidance (check `git status` before a destructive-shaped command; prefer stash over
discard) remains the answer for that one case, same as it is today.

## What this does not decide

Whether the alias option is also worth adding alongside a deny rule (a safer spelling costs little
and is not mutually exclusive with denying the dangerous one); the exact `.claude/settings.json`
syntax and scope (repo-level vs. session-level), left to whoever implements this.
