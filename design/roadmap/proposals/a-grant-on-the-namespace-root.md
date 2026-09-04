# A grant on the root of the shell's namespace

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 31's block.

**Gate: DECISION.** Both permanent answers are calef's, and they are permanent in different ways. A
narrowing verb is an addition to `filesystem_proto`, which two programs agree on, so it cannot be
un-shipped. An interactive boot rooted one component below the image root changes what every other
command at that prompt means, which cannot be un-taught. Nothing can start until one is chosen.

**In brief.** A subtree caretaker attenuates by performing one `OPENDIR` *into* the directory it was
granted. The root of the shell's namespace has no name to descend into, and `filesystem_proto` has
no verb meaning "the directory I already hold, with fewer rights". So `rm rmtree/rm-solo` works at
the prompt and `rm gate.txt` is a refusal, and the only difference between them is one level of
path. Closing it is either a narrowing verb on the contract (`Rights::attenuate`, no name
resolution, small in the server) or an interactive boot whose shell starts one component below the
image root.

## Why this matters

Milestone 31's whole claim is that typing a name is the grant. At the top prompt that claim is
false, for a reason a user cannot see and cannot work around: the same command on the same file
succeeds or is refused depending on how deep the file sits. `script/shell-check` gates the working
case on both ISAs, so the gap is not a regression risk; it is a permanent hole in the headline
demonstration, and it is the first thing a newcomer typing at the prompt will hit, because the top
of a namespace is where anyone starts.

The two answers are not equal and the choice is not close to free. The narrowing verb is the smaller
code change and the larger commitment: it adds a right-attenuating operation to a wire two programs
agree on, forever. Rooting the shell below the image root puts nothing on the wire and costs
meaning instead, since every relative path at that prompt then refers to somewhere else.

## Where it came from

Milestone 31's block, under "The two shapes a grant cannot take": *"This is a design fork and
belongs to calef, because both answers are permanent: a narrowing verb on the contract (small in the
server, `Rights::attenuate` with no name resolution, and an addition to something two programs agree
on) or an interactive boot whose shell is rooted one component below the image root (nothing on the
wire, and it changes what every other command at that prompt means)."*

`notes/dir-capability.md` carries the sibling limitation beside the feature: a grant more than one
level down is a chain of caretakers and is also still a refusal. That one has a home in the BUGS
convention. This one does not, because it is a fork rather than a limitation.
