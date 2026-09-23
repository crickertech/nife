# The automation's own identity

*Name: provisional. `automation-identity` is a lane's coinage; `design/naming.md` is the rule and
calef ratifies. The alternatives considered were `github-app.md` (names the vendor's mechanism
rather than what it is for, and this note would survive a move off GitHub only by lying) and
`bot-identity.md` (`bot` is what GitHub calls the badge, not what the thing is).*

The daily toolchain-bump workflow has to open a pull request that **gets CI**. GitHub's
anti-recursion rule says a pull request opened with a workflow's own ephemeral `GITHUB_TOKEN`
triggers no further workflow runs, so such a pull request sits with every required check stuck on
"Expected", permanently unmergeable. The workflow therefore authenticates as something else.

Today that something else is `TOOLCHAIN_BUMP_PAT`, a fine-grained personal access token on calef's
account. It works. It has two structural flaws: it expires on a personal timer that nobody reading
the workflow can inspect, and it couples the project's automation to one person's account. A
**GitHub App owned by the `crickertech` organization**, installed on `nife`, fixes both. Its
installation tokens are minted fresh per run, so nothing stored expires, and the identity belongs to
the role rather than to the person. That is `needs-architect`'s principle applied to credentials.

This note is the procedure. Creating the App needs owner rights on the organization, so it is
calef's to run; everything on the repository side is already in place and inert, waiting for the two
secrets.

## What it does not fix

**Commit authorship.** An App's installation token authors the *pull request* and its *comments*,
not the commits inside it. A commit's author is whoever holds the git identity at the moment
`git commit` runs, which in `toolchain-bump.yml` is `github-actions[bot]` by explicit
`git config`, and in a lane's worktree is calef. `Co-Authored-By:` stays the record of who actually
wrote a commit, and that split is honest rather than awkward: the human owns the change, the agent
is named as its co-author, and the conversation around the change is visibly the agent's.

**Anything else that authenticates as calef.** The App covers the workflows that are given its
token. A lane's `gh` calls from a developer machine still run as calef's `gh` login and still
produce pull requests with his byline; that is what the `**Lane:**` line in every body exists to
say in the meantime.

## Creating the App

Once, by an owner of the `crickertech` organization.

1. Go to **https://github.com/organizations/crickertech/settings/apps** and press **New GitHub App**.
   (The path by clicking: your avatar, **Your organizations**, `crickertech`, **Settings**,
   **Developer settings**, **GitHub Apps**, **New GitHub App**.)
2. **GitHub App name**: the display name is the `[bot]` byline a reader meets on every future pull
   request, so it is a naming decision and calef's. The lane's proposal is **`nife automation`**,
   which renders as `nife-automation[bot]`. Names are globally unique across GitHub, so a second
   choice is worth having ready.
3. **Homepage URL**: `https://github.com/crickertech/nife`. It is required and unused.
4. **Webhook**: untick **Active**. Nothing here listens for webhooks, and an inactive webhook is one
   fewer endpoint to secure.
5. **Repository permissions**, exactly two, matching what the PAT carries today and nothing wider:
   - **Contents**: Read and write (push the `toolchain/nightly-bump` branch).
   - **Pull requests**: Read and write (open, retitle and un-draft the proposal).
   Leave every other permission at **No access**. Leave organization and account permissions alone.
6. **Where can this GitHub App be installed?**: **Only on this account**. It is not a public listing.
7. Press **Create GitHub App**.

## Installing it and storing the secrets

8. On the App's settings page, note the **App ID** near the top. Then **Generate a private key**,
   which downloads a `.pem` file. GitHub never shows it again; a lost key is regenerated rather than
   recovered.
9. In the left sidebar, **Install App**, then **Install** next to `crickertech`. Choose **Only select
   repositories** and pick **`nife`** alone. An App installed on every repository holds authority
   over repositories it has no business in.
10. Store both as **repository** secrets on `nife` (not organization secrets: the App is only
    installed here, and a repository secret keeps the blast radius where the install is):

        gh secret set AUTOMATION_APP_ID  --repo crickertech/nife --body '<the App ID>'
        gh secret set AUTOMATION_APP_KEY --repo crickertech/nife < ~/Downloads/<app-name>.private-key.pem

    Then delete the downloaded `.pem`: `rm ~/Downloads/<app-name>.private-key.pem`. A key sitting in
    a downloads folder is the leak this whole exercise is meant to reduce.

    Both names are **provisional** until calef ratifies them.

## Confirming it took, and only then retiring the PAT

11. Run the workflow by hand and read its identity probe:

        gh workflow run toolchain-bump.yml --repo crickertech/nife
        gh run watch "$(gh run list --workflow=toolchain-bump.yml --repo crickertech/nife \
              --limit 1 --json databaseId --jq '.[0].databaseId')"

    The **Say which identity this run is authenticating as** step must print
    `identity: the crickertech automation App`. If it prints the PAT line instead, the secrets are
    not visible to the job: check the names, and check that they are repository secrets on
    `crickertech/nife`.

12. **Only after a real bump pull request has been opened by the App and received checks**, retire
    the token, in this order:

        gh secret delete TOOLCHAIN_BUMP_PAT --repo crickertech/nife
        # then revoke the PAT itself at https://github.com/settings/tokens?type=beta

    Deleting the secret first is what makes the revocation safe: with the secret gone the workflow
    has already fallen through to the App, so revoking the token cannot break a run that is still
    reaching for it. Then delete the PAT rung from `toolchain-bump.yml`'s comment and expressions,
    which is a two-line change and should not be done before this step.

## EXAMPLES

**Read which identity yesterday's bump used.** The probe is the first thing in the job, so this
needs no scrolling:

    gh run list --workflow=toolchain-bump.yml --repo crickertech/nife --limit 1 \
      --json databaseId --jq '.[0].databaseId' \
      | xargs -I{} gh run view {} --repo crickertech/nife --log \
      | grep -A4 'identity:'

**Check whether the bump pull request is actually getting CI**, which is the symptom the whole
mechanism exists to prevent. A bump pull request with one or two checks is a pull request opened by
the ephemeral token:

    gh pr list --head toolchain/nightly-bump --repo crickertech/nife --state all --limit 1 \
      --json number --jq '.[0].number' \
      | xargs -I{} gh pr view {} --repo crickertech/nife \
      --json author,statusCheckRollup \
      --jq '{author: .author.login, checks: (.statusCheckRollup | length)}'

On 2026-09-23 that reported `{"author":"calef","checks":16}`: the PAT path, working.

**Use the same App from a second workflow.** Copy the two steps, nothing else. The fallback chain is
per-workflow on purpose, so a workflow that has no business pushing can take the App's token without
inheriting a reason to keep a PAT beside it:

    env:
      AUTOMATION_APP_ID: ${{ secrets.AUTOMATION_APP_ID }}
    steps:
      - id: app-token
        if: env.AUTOMATION_APP_ID != ''
        uses: actions/create-github-app-token@v2
        with:
          app-id: ${{ secrets.AUTOMATION_APP_ID }}
          private-key: ${{ secrets.AUTOMATION_APP_KEY }}

## BUGS

- **The private key is still a stored secret.** An App swaps a stored *token* for a stored *signing
  key*. The win is organization ownership and per-run minting, not the absence of a secret. A leaked
  key is revoked in the App's settings, which is at least an organization-level act rather than a
  personal-account one.

- **The App's own permissions are not checked by anything.** Nothing in this tree verifies that the
  installation still holds Contents and Pull requests write, or that it has not quietly been granted
  more. If someone widens them in the web UI, no gate here notices. The identity probe proves the
  token can read the repository; it does not prove the permission set is the one this note describes.

- **The `if: env.AUTOMATION_APP_ID != ''` guard tests presence, not validity.** A secret set to the
  wrong App ID, or a private key that does not match it, takes the App rung and then fails the
  `create-github-app-token` step outright. That is loud rather than silent, which is the right
  failure, but the diagnostic comes from the action rather than from this note.

- **A bot byline changes what author filters see.** Bump pull requests will arrive as
  `<app-name>[bot]` rather than as `calef`. Nothing in this tree filters pull requests by author
  today, and `scripts/merge-drain.sh` is the file to re-read if that changes.

- **This note cannot tell you when the PAT expires**, and neither can anything else in the
  repository: a repository secret is opaque to every API the project can call, and only the account
  that minted the token can see its expiry at
  https://github.com/settings/tokens?type=beta. That opacity is not a gap in this note, it is the
  flaw the App removes.
