# 207. The four things GitHub asks for, and which of them this project actually wants

**Status: NOT-STARTED.** Minted 2026-08-31 by calef, from GitHub's Community Standards checklist.
*(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** Two of the four are policy rather than files: whether issues are a channel at all,
and who a code of conduct names as its enforcement contact.

**In brief.** Measured against `repos/crickertech/nife/community/profile` on 2026-08-31: `README`,
`CONTRIBUTING` and `LICENSE` are present; **`CODE_OF_CONDUCT`, an issue template and a pull request
template are missing.** GitHub's checklist also offers a fourth item, *Repository admins accept
content reports*, which is a **setting rather than a file** and cannot be delivered by a pull
request.

This matters for AGENTS.md's third principle rather than for tidiness. A newcomer must be able to
succeed without asking anyone, and these are among the first things a stranger meets.

## The pull request template is the one with a mechanism behind it

AGENTS.md requires that **every pull request an agent writes opens by saying so**, one line, first
thing in the body. Today that is prose in a brief, remembered by whoever is writing, and **lanes have
already failed the neighbouring instruction twice this week** (milestone 204: two lanes pushed
branches and opened no draft pull request, though both briefs named it as the first act).

A template makes the line the default rather than a thing to remember, which is rung two replacing
rung four, and it is the cheapest item here.

## The issue template is premature, and the reason is worth deciding rather than defaulting

**Issues are disabled on this repository** (`has_issues=false`). A template would be a file nobody
can reach.

Behind that is a real question this project has never answered: **is an issue a channel here at
all?** Work lives in `design/roadmap/` and in pull requests, deliberately, and DECISIONS §203's own
option list refused an issue as an output on the grounds that this project does not read them. Two
coherent answers:

- **Keep issues off**, and say so where a stranger looks, so the absence reads as a decision rather
  than neglect. `CONTRIBUTING.md` is the place.
- **Turn them on** with a template that routes: a bug report is welcome, a feature request belongs
  against the roadmap, and a design argument belongs in `design/decisions/`.

The second becomes materially more attractive once milestone 198 (a package manager, and the trivial
install that makes a second customer possible) exists, because that is when a third party can appear.
**Today there are none**, by calef's own position, so nothing is lost by deciding late.

## The code of conduct is a commitment, not a file

Adopting the Contributor Covenant takes a minute. **What it costs is the promise to enforce it**, and
a project that ships one without meaning it has published a claim it will not honour, which is worse
than silence and is exactly the failure this tree's `BUGS` convention exists to avoid elsewhere.

What has to be decided: **who the enforcement contact is.** Today that is one person, and naming him
is honest; a role address is what a project with contributors would use. §109's (attribution is a
channel property) instinct applies, that a mechanism spelling one person's name has that name as its
failure mode.

## Content reports: calef's, and it is a setting

Enabling *Repository admins accept content reports* sends reports of disruptive content to repository
admins rather than only to GitHub staff, into a Reports tab and optionally an email. It costs
nothing, it cannot be done in a pull request, and its moderation load today is zero because the
project has no third-party contributors.

## BUGS

- **Three of the four are checklist items and one is a mechanism.** The pull request template earns
  its place on its own; the rest are worth doing because a stranger meets them, which is a weaker
  reason and should be said out loud rather than dressed up.
- **A code of conduct with an unresponsive contact is worse than none**, and nothing in this
  milestone can make anybody responsive.
- **This block does not write any of the text.** Choosing the Contributor Covenant version, the
  template fields and the routing language is the work, and it is all reversible except the
  enforcement promise.
