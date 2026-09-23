# 128. The automation gets its own identity, and the agents get their own voice

**Status: PARTIAL.** 2026-09-23: steps 3 and 5 are built and step 4 is deliberately not done.
`toolchain-bump.yml` now tries an App installation token first, falls through to the PAT, then to
`github.token`, and prints which rung it took; `notes/automation-identity.md` is the procedure for
steps 1 and 2, which need owner rights on `crickertech` and are calef's to run. Nothing changes
about how the job behaves until the two secrets exist. Minted 2026-08-15 at calef's request, the same day he set
`TOOLCHAIN_BUMP_PAT` on the transferred repository and asked what other developers would need
(answer: nothing, and that answer is what surfaced this).

**Gate: NONE.** Nothing blocks a start; the app creation is calef's to perform, like milestone
120's organization was, because it needs owner rights on `crickertech`. What makes this a later
milestone rather than this week's is honest priority, recorded below.

**In brief.** The toolchain-bump workflow authenticates as a fine-grained personal access token
(PAT) on calef's account, because a PR opened by the ephemeral `GITHUB_TOKEN` triggers no CI
(GitHub's anti-recursion rule; the workflow's own comment records it). A PAT works and has two
structural flaws: it expires on a personal timer, with "bump PRs silently stop getting checks" as
the only symptom, and it couples the project's automation to one person's account, so the
automation breaks the day that account leaves the organization. A **GitHub App** owned by
`crickertech`, installed on `nife`, fixes both: installation tokens are minted fresh per workflow
run (nothing stored expires), and the identity belongs to the organization rather than to a
person. This is the `needs-architect` principle applied to credentials: name the role, not the
person.

## The second deliverable: attribution, added 2026-08-16

calef, reading a repository whose every pull request, comment and review carries his name:
**"it looks like I'm talking to myself a lot and the record would be nice to clarify who is
talking."** That is a documentation defect as much as a vanity one. This project's whole claim is
that a system of this size can be built by one architect and many agents; a timeline in which the
architect appears to write, review and merge his own work in a single voice is evidence *against*
the claim it should be evidence for. The provenance tenet applies to authorship the same way it
applies to names: the record should say who did a thing where a reader meets it.

**Same App, so the setup is shared.** An installation token authors as `<app-name>[bot]` with its
own avatar and badge, so a lane's pull request is visibly not the architect's. What exists today
is half a mechanism: commits carry `Co-Authored-By: Claude ...` and pull request bodies carry the
Claude Code footer, but the *author* of every pull request and comment is calef's account, and
that is the half a reader actually sees.

**The alternatives, recorded so the choice is one:**

- **A separate machine account** (`nife-agent` or similar) works today with no App and reads
  unambiguously. Cost: a second identity to secure, an org seat, and the same
  `GITHUB_TOKEN`-cannot-trigger-CI trap the App was minted to escape.
- **Convention only**, a `**Lane:**` line in the body naming the agent and its milestone. Free,
  cosmetic, and adopted immediately (2026-08-16) as an interim rather than as the answer, because
  a line of prose is rung four and an identity is rung two.

**What it does not fix, and should say so plainly:** an App cannot make the *commits* authored by
the agent, only the pull request and its comments; commit authorship stays with whoever holds the
git identity. `Co-Authored-By` remains the record there, and that split is honest rather than
awkward: the human owns the change, the agent is named as its co-author, and the conversation
around it is visibly the agent's.

## The work, which is small

1. Create the App under the `crickertech` organization (calef; Settings → Developer settings →
   GitHub Apps). No webhook, no public listing. Repository permissions: Contents read/write,
   Pull requests read/write, the same pair the PAT carries.
2. Install it on `nife` only, and store the App ID and private key as repository secrets
   (`AUTOMATION_APP_ID`, `AUTOMATION_APP_KEY`).
3. **Done 2026-09-23.** In `toolchain-bump.yml`, mint the installation token per run (the maintained
   `actions/create-github-app-token` action does exactly this, and taking it is a workflow-only
   dependency, not one in the shipping graph; note it in the §46 spirit anyway) and use it where
   `TOOLCHAIN_BUMP_PAT` is used today. Keep the `|| github.token` fallback: a fork without the
   App still opens PRs.
4. Delete the `TOOLCHAIN_BUMP_PAT` secret and revoke the PAT, in that order, and update the
   workflow's own explanatory comment, which is where this mechanism is documented for the next
   reader. **Not done, and deliberately sequenced after a run has been observed taking the App
   rung**: revoking a credential the workflow may still be reaching for turns a preparation into
   an outage. `notes/automation-identity.md` carries the order and the reason.
5. Any future workflow needing to trigger CI on its own PRs reuses the same App rather than
   minting another personal token; that reuse is the milestone's compounding value. The two steps
   to copy are in `notes/automation-identity.md`'s `EXAMPLES`.

6. **Added 2026-09-23, because an inert change and a broken change look identical.** The job prints
   which of the three identities it authenticated as and proves that token is live. Without it, a
   reader cannot tell an App run from a PAT run from a fallback run, and the PAT's expiry stays as
   quiet as the milestone says it is. This is the step that made the fallback observable rather
   than argued: dispatched on the lane's branch with no App secrets present, the job reported
   `identity: TOOLCHAIN_BUMP_PAT` and `token check: OK`, which is the fallback working.

## Why not now, and what the 2026-09-23 audit of that found

The original deferral, written 2026-08-15 and kept here because it is the reasoning the audit
below tests: with one architect, the PAT and the App fail in the same circumstances and the PAT
already exists, so the App earns its setup cost at the first of **a second architect joining**,
**the PAT's first silent expiry**, or **a second workflow needing the same authority**.

**Audited 2026-09-23, each trigger against the tree rather than against a recollection. None has
fired outright; the third is half-fired, and the audit's own difficulty is the finding.**

- **A second architect: no.** One architect. Stated and moved past.

- **The PAT's first silent expiry: not yet, and the project cannot see how close it is.** The
  evidence that it is alive is positive rather than inferred. `toolchain-bump.yml` has run and
  succeeded every day through 2026-09-23; that day's proposal, pull request #1112, was authored by
  `calef` (the PAT's identity, not `github-actions[bot]`), collected sixteen checks including
  `build + test`, `prove` and `cpu matrix`, and merged. A PAT that had expired would have produced
  either a red checkout or a pull request under the ephemeral token with no CI, and neither
  happened. **What cannot be established from inside the repository is when it expires**: a
  repository secret is opaque to every API this project can call, and the expiry is visible only to
  the account that minted it. The milestone said the symptom would be silent; the audit's actual
  result is stronger and worse, which is that *the warning* is silent too. There is no way to
  schedule around this trigger, only to be hit by it. That asymmetry is the best argument in this
  block for doing the work before the trigger rather than on it.

- **A second workflow needing the authority: half.** Two workflows calling `gh` landed after this
  milestone was minted, and they are not the same case as each other.
  - `architect-hold.yml` genuinely needs nothing beyond `GITHUB_TOKEN`. It is read-only (one
    `gh api` call for a pull request's labels) and creates no events, so the anti-recursion rule
    does not reach it. **This is the case the milestone's trigger would have over-counted**: "calls
    `gh`" is not "needs a PAT".
  - `coe-architect-label.yml` is the half. It **writes**: it adds `needs-architect` to a pull
    request that adds a correction-of-error record. An event created by `GITHUB_TOKEN` triggers no
    workflow run, and GitHub's rule is not limited to `push` and `pull_request`, so the `labeled`
    event this job creates does not re-run `architect-hold.yml`. The required check on the pull
    request page therefore stays at whatever it last reported, which is green, while the label says
    the opposite. **The gate itself still holds**, because `architect-hold.yml` also runs on
    `merge_group` and reads labels fresh from the API, so the merge queue catches it and evicts the
    entry. So this is a display defect rather than an escape, and it is untested in practice:
    `notes/corrections/` does not exist on `main` yet (decision 210 is unmerged), so the labeller
    has never once fired. It is a second workflow that **would** be better on the App, not one that
    is broken without it.

**Verdict: still parked as a forced move, and no longer parked as a cheap one.** No trigger
compels the work today. The preparation that does not need owner rights was done anyway, because
it costs nothing to carry (the App rung is inert without its secrets) and because it converts the
expiry from an outage into a fifteen-minute procedure someone can run without reading this block.
## BUGS

- **The private key is still a stored secret.** An App swaps a stored *token* for a stored
  *signing key*; the win is org ownership and per-run minting, not the absence of a secret. A
  leaked key is revoked in the App's settings, which is at least an org-level act rather than a
  personal-account one.
- **Bot-authored PRs change the byline.** Bump PRs would arrive as `<app-name>[bot]` rather than
  as calef; anything filtering PRs by author (none known in-tree today) would need updating. The
  workflow's own `**Lane:**` line is computed from which identity it took, for the same reason: a
  pull request that says calef's account is its author is false the day the App lands.

- **Nothing checks the App's permissions after it is installed.** The identity probe proves the
  token can read the repository; it cannot prove the installation still holds exactly Contents and
  Pull requests write, or that nobody widened it in the web UI.

- **The App's display name is unratified.** `nife automation`, hence `nife-automation[bot]`, is a
  lane's proposal. It is the byline a reader meets on every future bump pull request, so it is
  `design/naming.md`'s call and calef's, and it is also globally unique across GitHub, so it can be
  refused by someone outside this project entirely.

## Follow-on

- **Outstanding.** Steps 1 and 2: creating the App under `crickertech` and storing
  `AUTOMATION_APP_ID` and `AUTOMATION_APP_KEY`. Owner rights on the organization are required, so
  no lane can do it. Checked 2026-09-23 by `gh secret list`, which shows `TOOLCHAIN_BUMP_PAT` and
  neither App secret. `notes/automation-identity.md` is the procedure.

- **Outstanding.** Step 4: deleting the PAT secret and revoking the PAT. Blocked on the above and
  deliberately sequenced after a run has been observed taking the App rung, because revoking a
  credential a scheduled job may still reach for turns a preparation into an outage. The order and
  the reason are in `notes/automation-identity.md`.

- **Outstanding.** Ratifying the App's display name. `nife automation` (rendering as
  `nife-automation[bot]`) is a lane's provisional proposal, and it is the byline every future bump
  pull request carries, so it is `design/naming.md`'s call. Checked 2026-09-23: it appears nowhere
  in the tree except this block and `notes/automation-identity.md`, both of which mark it
  provisional, so nothing has been built on it.

- **Recorded.** `coe-architect-label.yml` adds `needs-architect` with `GITHUB_TOKEN`, whose
  `labeled` event re-runs no workflow, so `architect-hold.yml`'s required check on the pull request
  page stays stale-green while the label says otherwise. The merge-queue run of the same check
  still catches it, so this is a display defect and not an escape. It lives in the BUGS section of
  `notes/automation-identity.md`, beside the mechanism that would fix it.

- **Done.** The second deliverable (attribution) needs nothing further from a lane. The interim
  `**Lane:**` convention is in `AGENTS.md` and applied; `toolchain-bump.yml` now computes that line
  from which identity the run took, so it stops claiming calef's byline the moment the App lands.
  Carried by this milestone's own commits.

## Index row

The toolchain-bump PAT expires on a personal timer and couples the project's automation to one
account; an org-owned App names the role, not the person. Deliberately parked until a second
architect, the PAT's first expiry, or a second workflow needing the authority
