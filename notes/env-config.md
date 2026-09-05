# The inert-configuration page: `TZ`, `LANG`, `TERM`, and what still waits

Milestone 47's environment-variable fork. Built 2026-08-23. The contract is `environment_proto` (a
provisional name; naming crates is calef's call, per AGENTS.md); the decision is DECISIONS §111.

## What this is

Milestone 47 splits what Unix puts in one string-to-string environment map into three parts:
**inert configuration** (`TZ`, `LANG`, `TERM`, genuinely just data, no authority in it), **names**
(`PATH`, `HOME`, directory capabilities wearing a string costume), and **secrets** (authority badly
encoded as a bearer string). Only the first third is built here. Names wait on `bind` (milestone
154's two-directory endowment, not yet built); secrets are answered elsewhere, by an endpoint
(§41), and are never meant to arrive as a string on this table at all.

**The wire encoding is a read-only page**, the same rights-ladder shape the clock page uses: no
capability, or a `Frame` with `READ`. A config value is never something a person designates on a
command line, and there is nothing to propose or set from inside the process that holds it, so an
endpoint (as the clock's propose half uses) would be the wrong shape here.

**Each declared key is validated against a closed domain before it is ever written to the page.**
`TZ` must be a real IANA-style timezone identifier, `LANG` a real locale identifier, `TERM` a real
terminal type; a value that does not parse as a member of its key's domain is refused when the page
is assembled, not carried through disguised as configuration. This closes the gap DECISIONS §111
identifies: a capability governs reach, not meaning, so once a value is bytes on a page nothing
about the capability model can tell a password from a timezone. `environment_proto::domain`'s three lists
are curated and real, not the full IANA/glibc/terminfo databases; see the crate's own `BUGS`
section for why growing them on demand is deliberate rather than a gap.

## Why no seqlock, unlike the clock page

The clock page is published to repeatedly by a running service while readers hold a stale mapping,
so a reader can catch a writer mid-publish and must retry rather than blend. The config page has
**exactly one writer, and it finishes before the page has a second reader**: it is assembled in a
buffer nothing else can see, and only *then* mapped read-only into the process that will read it.
There is nothing to race, so `environment_proto::ConfigPage` is a plain length-prefixed byte layout with no
atomics at all.

## What's built end to end

- `crates/environment_proto`: the page layout, the three validated domains, `PageBuilder` (assemble,
  validated) and `ConfigPage` (read). Host-tested: round trips, refusals, an unrecognized magic
  or a zeroed frame reading as "no configuration" (DECISIONS §42's rule, applied here), every
  domain member fitting its field's byte cap.
- `kernel/src/user/std_service.rs`: assembles a page (`TZ=UTC`, `LANG=C`, `TERM=dumb`, the
  conservative universal defaults, chosen because there is no shell yet to hold a *different*
  default and pass it explicitly) and maps it read-only into a std program at `CONFIG_PAGE_STD`
  (`0x1300_0000`), granting a `Frame` with `READ` at `CONFIG_SLOT` (7), the same shape as the
  clock's slot 5.
- `patches/std-nife/overlay/std/src/sys/pal/nife/rt.rs`: `CONFIG_SLOT`/`CONFIG_PAGE` constants,
  the std PAL's twin of the kernel-side ones.
- `patches/std-nife/overlay/std/src/sys/pal/nife/mod.rs`: the `envproto` module (generated
  verbatim from `crates/environment_proto` by `cargo xtask std-src`, the same mechanism `clockproto` and
  `fsproto` use) and the call into `sys::env::seed()` from `pal::nife::init`, before `main` runs.
- `patches/std-nife/overlay/std/src/sys/env/nife.rs`: `seed()`, which probes `CONFIG_SLOT` the
  same way `sys/time/nife.rs` probes `CLOCK_SLOT` (an invocation with a method number no object
  defines, so an empty slot and an occupied one answer differently), and pushes whatever keys the
  page carries into `std::env`'s table.
- `std_exerciser`: asserts `TZ`/`LANG`/`TERM` read back exactly what the kernel wrote, before
  `std::env::vars().count()` is asserted (now 3, not 0: the count changed from "always empty" to
  "exactly what was granted"). `kernel::user::std_tests::a_whole_std_program_runs_on_the_native_abi`
  pins the whole transcript on both ISAs.

## What this does not do, and why

- **No shipped program declares wanting this page.** `grant_plan::Manifest` carries no `config`
  field the way it carries `clock` for `date`; nothing in the shell's `Prog` enum has a reason to
  read `TZ`/`LANG`/`TERM` yet. `clock` waited for `date` to be the real customer before it was
  wired into the shell's spawn protocol; this page is in the same position. Extending `date` to
  read `TZ` and apply a real UTC offset would need timezone/DST arithmetic this project has
  explicitly declined to build (`notes/calendar.md`: "no zone names, no DST, no `TZ`"), so the
  natural next customer is a program that merely *displays* what it was configured with, not one
  that computes from it.
- **The `caps` preview extension DECISIONS §111 also asks for** (showing a program's declared
  inert-config values before it runs) has no page to preview without a `Manifest` field to hang
  it on, so it waits on the same customer.
- **`PATH` and `HOME` are not seeded here.** Both are namespace questions (directory capabilities),
  not variable ones, and the roadmap's own text is explicit that they wait on `bind`.

## Tests

`crates/environment_proto` host suite: 9 unit tests plus 2 doctests, covering the round trip, every domain
check, an undeclared key reading as absent rather than empty, a zeroed frame and an unrecognized
magic both reading as "no configuration," and the layout offsets not overlapping.
`kernel::user::std_tests::a_whole_std_program_runs_on_the_native_abi`, both ISAs: the real
`std_exerciser` binary, granted a real config page assembled by the real kernel wiring, reads
`TZ`/`LANG`/`TERM` back through the real std PAL and the real `std::env` API.

## BUGS

- **No wire announcement on the shell's spawn protocol.** The clock and config pages are both
  driven purely by what a program's manifest declares (`wants_clock`/would-be `wants_config`),
  which init already knows without anything on the wire, so this is not a missing mechanism, only
  a missing declaration: no `Manifest` field exists because no program needs one yet.
- **The kernel-side default (`UTC`/`C`/`dumb`) is a placeholder rather than a real "shell holds a
  default config set" mechanism.** The roadmap's "inheritance with visibility" shape (a shell
  holds its own default, passes it explicitly, and `caps run prog` shows it) is not built; what
  exists today is one fixed default `std_service.rs` assembles for every std program it spawns,
  which is closer to `date`'s "init endows unconditionally" shape than to a per-shell,
  per-session default.
- **The domains are curated, not exhaustive.** See `environment_proto`'s own `BUGS` section.
