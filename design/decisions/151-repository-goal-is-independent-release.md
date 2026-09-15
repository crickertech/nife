# 151. The goal of the repository split is independent release and third-party programs

**Status: DECIDED.** calef, 2026-09-15, in conversation, thinking through the eventual multi-repository
structure: *"Independent releases is the goal and third party programs is the goal."*

## What this settles

Milestone 39 (repository structure for a loosely-coupled OS) recorded four options and took no
decision, on the ground that nothing external needed one yet. Its options B, C and D were three ways
of reaching, or deferring, a decoupled structure; the open question underneath them was **what the
structure is for.** This decides that:

- **Independent release.** Each shippable piece can be versioned and released on its own cadence,
  behind an interface that does not change under its consumers. This is the property that decides the
  grain of the split (see BUGS): a thing earns separation when it can ship on its own schedule, not
  because it is a separate package.
- **Third-party programs.** The structure is the one an outside author writes a program against
  without this project in the loop, which is milestone 39's option C described as *"what an ecosystem
  with third-party components looks like"* and milestone 198's *"no third party sees nife until there
  is a package manager and a trivial install."*

So the endpoint is the decoupled ecosystem, not the monorepo held forever. Milestone 39's own
recommendation is consistent with this: get there through option B (workspaces inside one repository)
and let the distribution be a manifest repository (`basalt`, reserved by milestone 120), reaching the
split when the seams exist rather than by directory surgery up front.

## What this does not settle, and it is deliberately separate

**The order.** *When* the split happens, against what preconditions, is not decided here and should be
ruled on its own. The candidate sequencing, from the records rather than invented: the wire protocols
carry real version numbers and have stopped churning weekly (milestone 39, §14), and a package format
exists so `basalt` has something to assemble (milestone 198). Until then the single-command,
all-architecture `script/test` is the credibility mechanism milestone 39 says a premature split would
quietly break, so the strain has to justify the trade. That trade is a separate ruling.

## What is unblocked

- **Milestone 39** can move from RECORDED to a direction: its option C is the stated destination,
  reached as B. It no longer waits on "what is the structure for."
- **The wire-protocol crates gain a reason to carry real versions**, which milestone 39's packaging
  observation names as the thing that lets components evolve independently at all. Independent release
  is not possible while every crate is `version = "0.1.0"`.
- **`basalt`** (milestone 120's reserved distribution repository) has its job named: it assembles
  released pieces, and it can begin as a manifest repository that names what the distribution contains
  before it holds any code.

## BUGS

- **This decides the goal, not the grain.** "One repository per package" was calef's opening framing
  in the same conversation; the maintainer argued for splitting on release cadence instead, since a
  piece earns a repository when it can ship independently and almost nothing can yet while the
  protocols move. This section records that independent release is the *test* for the grain; which
  pieces pass it is measured when the split is scheduled, not asserted here.
- **The integration gate is the real cost and is not solved by this decision.** A multi-repository
  world has to keep proving the whole system on every supported architecture, and milestone 39 records
  that a naive split makes that proof "live somewhere awkward or quietly stop running." Deciding the
  goal does not build that gate.
