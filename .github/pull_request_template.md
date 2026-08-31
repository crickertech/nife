**Lane:** <branch or milestone>, written by an agent; calef's account is the author GitHub shows.

<!--
Delete the line above only if a human wrote this pull request. AGENTS.md requires it first thing in
the body of anything an agent writes, because every artifact here carries calef's name whether he
wrote it or not, and a reader cannot otherwise tell the architect's voice from a lane's.
-->

## What changed, and why

<!-- The diff shows what. Say why, and say what you decided against. -->

## Gates

<!--
`script/gates` runs all five, cheapest first. Paste the result, or name what you ran and what you
skipped and why. "Not run" is an answer; a missing answer is not.
-->

- [ ] `script/fmt --check`
- [ ] `script/lint`
- [ ] `script/icount`
- [ ] `script/test`
- [ ] `script/test --hvf`

## Identified work

<!--
AGENTS.md: work this change found but is not doing leaves in one of exactly two shapes, and "worth
doing someday" is neither. Either a proposed milestone (the integrator mints the number at merge),
or a `BUGS` entry written where a reader meets the feature. Say "none" if there is none.
-->

## Needs the architect

<!--
Delete this section unless something here is calef's call: a design fork, the syscall surface, a new
dependency, a name, or anything two programs agree on. If it is, add the `needs-architect` label and
say what the ask is, answerable without reading the diff, including what happens if he says no.
-->
