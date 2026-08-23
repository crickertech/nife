# 113. Seven kernel object and identifier names move from contraction or borrowed jargon to the plain, standard term

**Status: DECIDED.** calef, 2026-08-23, after repeatedly having to ask what `Aspace`, `Endpoint`,
`Untyped`, and `Tcb` meant in the course of ordinary conversation about this tree: *"I have
repeatedly had to ask what these terms mean because they're often used without context. Thus
they're clearly not working."*

## The question

Five kernel object type names, plus two identifiers that name them, were checked one at a time
against whether a reader needs to
already know a specific external system (seL4, in every case) to recognize them on sight. The
check that mattered was not "is this a real term of art somewhere" but the plainer one calef named
directly: does the architect, working in this tree daily, still have to ask what it means. Prior
art turned out not to answer that question, because seL4, Mach, Windows and QNX disagree with each
other on several of these, so no single borrowed word is broadly recognized either.

## The decision

Seven renames, all of the same shape: replace a contraction or a borrowed short word with the plain,
standard English compound the field already uses for the concept, so a reader who knows the concept
recognizes it immediately and a reader who doesn't gets a running start from the words themselves.

| Was | Becomes | Kind of fix |
|---|---|---|
| `Aspace` | `AddressSpace` | contraction spelled out |
| `Untyped` | `MemoryRegion` | seL4's word replaced with the tree's own working name, disambiguated |
| `Endpoint` | `Rendezvous` | seL4's word replaced with the property that actually matters |
| `Frame` | `PageFrame` | bare word disambiguated with the standard OS qualifier |
| `Tcb` | `ThreadControlBlock` | acronym spelled out |
| `EpId` | `RendezvousId` | follows `Endpoint`'s rename; names what it identifies |
| `Tid` | `ThreadId` | acronym spelled out, follows `Tcb`'s rename |

## Why each one, with the in-tree evidence that decided it

**`EpId` -> `RendezvousId` and `Tid` -> `ThreadId`.** Both are id-typed companions of a renamed
object rather than independent decisions: `EpId` identifies an `Endpoint`/`Rendezvous`, `Tid`
identifies the thread a `Tcb`/`ThreadControlBlock` tracks. Leaving the identifier abbreviated
after spelling out the object it names would just move the same problem one field over, and
`ThreadId` in particular has no more external constraint than `Tcb` did: nothing outside this
tree requires the three-letter form.

**`Aspace` -> `AddressSpace`.** No rationale for the contraction exists anywhere in the tree
(checked: commit history, `notes/`, `DECISIONS.md`). It is also not seL4's term: seL4 calls the
equivalent object `VSpace`, so `Aspace` is neither the full English word nor borrowed vocabulary,
just an unexplained local abbreviation. Its own sibling variants in `kernel/src/cap.rs`'s `Object`
enum (`Endpoint`, `Untyped`, `Frame`, `Reply`) are unabbreviated; `Aspace` was the outlier before
this decision touched any of them.

**`Untyped` -> `MemoryRegion`.** `Untyped` is exactly seL4's word (`seL4_Untyped_Retype`,
confirmed against seL4's own docs), so a reader with seL4 background recognizes it and nobody else
has anywhere to start, because Linux, Mach and Windows expose no equivalent concept under any name.
Meanwhile this tree's own implementation never actually says "untyped" once you're inside it:
`kernel/src/untyped.rs`'s table is `REGIONS`, its cap is `MAX_REGIONS`, its accessor is
`region_bounds()`, every parameter is `region: u64`, there is a whole crate
(`crates/regions`, whose own doc comment calls itself "the untyped-region accounting"), and five or
more `notes/` files describe "the region model." Contributors independently reached for "region"
the moment they weren't being careful about the official name, which is stronger evidence of what
actually reads naturally here than any borrowed vocabulary. Bare `Region` was considered and
rejected: `crates/compositor` already uses "region" for a damaged screen rectangle
("keeping a region list," "the damaged region is window 1's"), a real in-tree collision.
`MemoryRegion` keeps the word the tree already converged on and removes the collision.

**`Endpoint` -> `Rendezvous`.** `notes/ipc-naming.md`'s own "Family resemblance" section records
that Mach calls this object a `port`, QNX calls it a `channel`, and seL4 calls it `Endpoint` --
three influential microkernels, three different words, so `Endpoint` privileges a reader with
seL4 background over one from Mach or Windows (whose ALPC ports descend from Mach's vocabulary)
with nothing to recommend it over the alternatives except which kernel got asked first. `Channel`
was considered and rejected on a concrete collision: Go's `chan` is probably the most common
thing "channel" means to a working developer today, and Go channels can be buffered, which
directly contradicts the strict rendezvous guarantee (`notes/ipc-naming.md`: "the only client
parked in `RECV(reply)`... is the one it just served") that is the entire reason this object's
semantics matter. `Port` was rejected for colliding with this tree's own hardware and networking
vocabulary. `Rendezvous` names the actual guarantee -- both sides must be present, nothing is
buffered -- rather than any kernel's incidental word for it, has real standing outside this tree
(Ada's concurrency model calls its synchronous meet-both-sides-required primitive a rendezvous),
and it is already the word this tree's own prose reaches for independently:
`crates/compositor/src/lib.rs:705` says, of its own IPC, "is a rendezvous, so a compositor that
narrated each frame would block."

**`Frame` -> `PageFrame`.** `Frame` is generic on its own and collides in-tree the same way bare
`Region` did: `crates/compositor` uses "frame" throughout for a rendered screen update ("per
frame," "the frame's damage," "a compositor that narrated each frame"), which is an unrelated
concept from the kernel's `Frame` object (one physical page, DECISIONS §102). `PageFrame` is not
invented vocabulary -- "page frame" is the standard OS term for a physical page in a virtual
memory system, used across the field regardless of kernel lineage -- so the fix is the same shape
as `Aspace` and `Tcb`: stop truncating the standard term.

**`Tcb` -> `ThreadControlBlock`.** The clean case: "Thread Control Block" is standard OS
terminology taught in essentially every operating systems course, so a reader who already knows
the acronym loses nothing recognizing the spelled-out form, and a reader who doesn't gets three
ordinary English words with real content ("thread," "control," "block") instead of three letters
with none. Unlike `elf`, `pci`, `dtb`, `gpt`, `ipc`, `paging`, `glob` and `asid` -- the crate-naming
rules' list of abbreviations kept because they are external wire-format or spec names this tree
cannot rename without becoming incompatible with what the rest of the world calls them -- nothing
external constrains what this tree calls its own internal thread object. There is no format
called "TCB" anywhere outside an OS textbook's prose.

## The general principle, stated so it applies to names not yet found

**A borrowed abbreviation is not accessible just because it is correct.** Prior art (seL4's
vocabulary, in every case here) proves a name is defensible, not that it works, and "works" is
measured by whether a reader needs to ask, not by whether the word has a pedigree. The distinction
that separates a name worth keeping abbreviated from one that is not: is the abbreviation forced
by something external the tree must interoperate with (`elf`, `pci`, `dtb`, `gpt`, `asid` all name
a wire format or hardware spec verbatim, and renaming them would just be wrong), or is it a purely
internal convenience this tree chose and could equally have spelled out. `Tcb`, `Aspace`,
`Untyped`, `Frame` and `Endpoint` were all the second kind.

## What this does not decide

The mechanical rename itself: each touches a real, measured surface (`Endpoint` alone is 81
occurrences across 23 `.rs` files; the others are unmeasured but comparable, being equally
foundational kernel object types) and is left to whoever executes it, tracked as its own piece of
work per name. This decision settles the five names, not the rename lane(s) that carry them out.

A systematic sweep for other internal abbreviations sharing this shape (checked against the same
external-constraint test above) is separate, ongoing work; findings land as their own entries once
checked, not folded into this one.

## What it unblocks

Seven renames the tree can now execute without re-litigating the name each time: `Aspace` ->
`AddressSpace`, `Untyped` -> `MemoryRegion`, `Endpoint` -> `Rendezvous`, `Frame` -> `PageFrame`,
`Tcb` -> `ThreadControlBlock`, `EpId` -> `RendezvousId`, `Tid` -> `ThreadId`.
