# The nine `INIT` roles, and the archive entry `spawn_hello` picks

**Status: PROPOSED 2026-09-14.** Filed by milestone 291's lane, which split twenty-two of
`fixtures/src/hello.rs`'s thirty-one roles into programs and stopped at these nine on purpose. See
[291](../291-one-program-one-job.md) for the inventory and the principle.

**Gate: MILESTONE 268.** That lane was rebuilding the boot sequence on all three architectures
while 291 ran, and the change proposed here is in `kernel::user::spawn_hello`. Two lanes in
that function is the collision this tree already knows how to avoid.

## What is left, and why it did not come apart with the rest

Nine roles of `hello`: `INIT` (20), `CHILD` (21), `DEV_CHILD` (22), `INIT_DEV` (23),
`INIT_CONSOLE` (24), `INIT_IRQ` (25), `IRQ_CHILD` (26), `INIT_LEAST_AUTHORITY_DEMO` (28) and
`INIT_COREMARK` (29). Six of them are a userspace parent that parses an ELF and builds a child;
three of them *are* the child that parent builds.

The other twenty-two came apart cheaply because the kernel spawns each of them directly, with
`run(image, Spawn { .. })`, so the only change was which bytes the caller passed. These nine do not,
for two reasons:

1. **`spawn_hello` always re-enters `HELLO_ENTRY`**, since milestone 166 moved the boot role's own
   entry out to `boot_progenitor`. Six parents becoming six programs makes that choice a table, or
   makes the entry a parameter the six call sites supply. The second is the smaller surface and is
   probably right; it is still a boot-path signature change.
2. **The parents find their children by looking themselves up.** `ROLES_ENTRY` is the string
   `"hello"`, and `init_build` reads *this binary's own ELF* out of the archive and re-enters it at
   a different role. Split, each parent names its child's archive entry instead, which is the
   shape `init_console`, `init_least_authority_demo` and `init_coremark` already have (they load
   `"console"`, `"least_authority_demo"` and `"coremark"` by name). So the target shape exists in
   the same file; it is the three roles that re-enter their own image that have to move.

## What it would take

- Three child programs, provisionally `reporting_child`, `device_identifying_child` and
  `interrupt_waiting_child`, packed in all three archives. Their two agreed words (`CHILD_WORD`,
  and the interrupt child's reported word) join `capability_demo_protocol`, which milestone 291
  created for exactly this.
- Six parent programs, each naming its child's entry.
- `spawn_hello` taking the entry name, or a role-to-entry table beside `PROGENITOR_ROLE`.
- Six `spawn_hello` test call sites updated. The role numbers themselves stay:
  [the progenitor's grant order](../301-one-grant-order-for-the-progenitor.md) recorded that those six
  tests name them, and the grant *order* the roles share is the thing that proposal is about.

## What it would settle

**The name.** calef deferred naming whatever remained of `hello` until there was something settled
to name (2026-09-14: *"If there is anything left then we can consider a name for what remains"*).
After 291 what remains is nine roles; after this, either six programs and three children with their
own names, or nothing at all, and `hello` is deleted rather than renamed. That is the cleaner
answer and it should be checked before anybody spends a naming decision on the interim.

## What it would not settle

The six parents would still dispatch on nothing: each would be a whole program with one `_start`.
But three of them differ from `INIT` by a single capability in the endowment (`INIT_DEV` adds the
UART, `INIT_IRQ` adds the interrupt), which is an argument that they are one program parameterised
by what it was granted rather than three. **That is a design question this proposal does not
answer**, and answering it by splitting first would be the wrong order: a program that reads its own
capability table to decide what to do is the capability-system-native shape, and it may make three
of these nine unnecessary rather than separate.
