# What the boot path is called, now that the role has a name

**Status: PROPOSED 2026-09-08.** Held out of milestone 266 deliberately: `script/initboot` to
`init-boot` was staged during the naming sweep and backed out pending the role's own name, because a
boot path named after a role cannot be named before the role is.

**Gate: DECISION.** Every string here is a name, and names are calef's.

## The three strings, in two naming domains

| thing | today | domain | rule |
|---|---|---|---|
| `script/initboot` | `initboot` | `script/` entry point | hyphens |
| `cargo xtask initboot` | `initboot` | shell subcommand | hyphens |
| the kernel Cargo feature | `initboot` | Rust identifier | `snake_case` |

They must move together. The script's whole body is `exec cargo xtask initboot "$@"`, and the xtask
subcommand builds the kernel with that feature; `script/lint` lints each boot-mode feature by name.

The defect they share today is one AGENTS.md names outright: a squished multiword name, of the kind
`sysinit`, `fsclient` and `credcli` were renamed away from. `jobmix` and `script/job-mix` have the
identical defect and should move in the same breath or the tree acquires a third convention.

## Two candidates

**`progenitor-boot` / `progenitor-boot` / `progenitor_boot`.** The mechanical answer, and it is a
mouthful three times over. Its virtue is that it says exactly which program the kernel hands to, and
a reader who knows `progenitor` needs nothing else.

**`handoff` / `handoff` / `handoff`.** What the flag actually selects is *skip the milestone tour and
hand the machine to userspace immediately*. That is a property of the boot, not of who receives it,
and its sibling feature is called `shell` for the same reason: it names what you get rather than the
program that builds it. It is also one word in every domain, so the two-domain split above stops
mattering.

The argument against `handoff` is that it is a generic word of the kind AGENTS.md flags, and that the
tree already uses "hand off" in prose for several unrelated transfers. The argument against
`progenitor-boot` is length and that it will read as redundant once every boot goes to the
progenitor, which is already true.

## What is blocked

Nothing. `initboot` works and is spelled the same in all three places, which is the only property
that has to hold. This is a naming debt with a recorded reason, not a defect.
