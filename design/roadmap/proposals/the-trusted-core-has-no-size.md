# The trusted core has no size, and the tree cannot say how big it is

**Status: PROPOSED 2026-09-20.** Filed by `maintainer/redleaf-comparison` from the RedLeaf and
Tock-founding-paper reading (`notes/redleaf.md`). *(Number and slug provisional until the merge queue
lands it.)*

**Gate: NONE.** One note, one `script/metrics` column, and a cross-reference. No hardware, no
decision, no syscall surface. **One sentence inside it is calef's**: any published claim about how
small this kernel's trusted base is, relative to anyone else's, is a fact that leaves the machine.
Produce the number and the method; leave the comparison for ratification.

## What was found

Levy et al., *The Case for Writing a Kernel in Rust* (APSys '17), makes its entire quantitative case
in one sentence: *"The kernel's trusted computing base includes the Rust core library as well as
under 1000 lines out of over 6000 lines of kernel code."* That number is why the paper is cited. It
is the thing a reader remembers.

**RedLeaf, which builds on that argument, gives no such figure anywhere in its ten pages.** It
enumerates its TCB instead (microkernel, trusted crates, device crates, the IDL compiler, the trusted
compilation environment) and never measures it, while adding two components Tock does not have. The
authors' own follow-up (PLOS '21) calls one of those additions *"incomplete and error prone."*

**And this tree cannot answer the question either**, which is the part that makes it a proposal
rather than an observation about somebody else. Measured on 2026-09-20:

- `script/metrics` tracks `kernel_code_lines` (39,892) and an unsafe **density** of 82 blocks per
  10,000 non-arch code lines against an 88 ceiling. Neither is a TCB size.
- The unsafe census counts **blocks**, not lines: 955 outside `kernel/src/arch/`, 314 inside. There is
  no code in the tree that converts a block count to a line count.
- Most of those blocks are in userspace crates that are **outside** the TCB, and the tree has never
  defined which packages are inside it, so even a correct line count would have no membership filter
  to apply.
- `notes/tcb.md` is about the **Thread** Control Block. Its own acronym-collision section says the
  Trusted Computing Base sense *"is unrelated."* **There is no note for the other meaning**, which is
  the meaning DECISIONS §14 (a verified-Rust capability microkernel that runs real workloads) spends
  the project's thesis on.

## Why it is worth doing

**Because the claim is already being made and is currently unfalsifiable.** DECISIONS §14 says *"a
small, machine-checked trusted core."* `notes/prior-art.md` builds its entire build-versus-reuse rule
on one boundary: *"the reuse boundary is the TCB boundary. Inside it, always build."* A rule that
decides what this project writes by hand cannot point at a definition of the set it is about.

Three of this project's own principles land on the same answer:

- **A newcomer must be able to succeed without asking anyone.** The first question anyone asks a
  microkernel is how big the trusted part is. seL4 answers in one number. Tock answers in one number.
  This tree cannot answer at all, and a stranger reading `notes/prior-art.md` will assume the
  boundary it keeps citing is written down somewhere.
- **Anything that only works because someone knows it is a defect.** TCB membership is currently a
  judgment a maintainer makes per question.
- **Benchmarks and cross-OS comparisons are first-class. Measure, do not argue.** This is the same
  discipline applied to a size rather than a latency, and it is the one axis where the comparison
  against the language-isolated systems is currently unavailable in either direction.

**And note which way the number might cut**, because a proposal that only anticipates a flattering
result is not one. nife's trusted core is 39,892 code lines against Tock's ~6,000, and the honest
framing of that gap is not obvious: Tock's kernel is a uniprocessor embedded kernel with no MMU work,
and nife's carries three architectures, page tables, a scheduler and an IPC path. The number could
easily read badly and should be produced anyway.

## What the work is

- **Define TCB membership as data, not prose.** Which packages and which directories are inside the
  trusted core, written where a script reads it rather than where a reader infers it. The obvious
  first cut is `kernel/src` plus the crates it links, and the interesting part is the argument for
  each inclusion, not the list.
- **A `script/metrics` column**, so the number is a series rather than a snapshot and a lane that
  grows the TCB can see that it did. The existing `unsafe_density` column is the model, including its
  honesty about why it is a density and not a count.
- **A note under a name that is free**, since `notes/tcb.md` is spent. The note carries what the
  number includes, what it excludes and why, and the two things it is not: it is not a claim that the
  excluded code is correct, and it is not comparable to a figure from a system that means something
  different by "trusted" (`notes/redleaf.md` has the worked version of that trap).
- **A `BUGS` section** stating that a line count is a proxy for a proof obligation and a poor one, and
  that a tree can shrink this number by moving code rather than by removing it.

## What would make this not worth doing

If calef judges that the TCB size is not a number this project wants to publish, close it: the
measurement's main value is external, and internally the unsafe-density ceiling is already doing the
work of keeping a lane honest. It costs one lane, it is documentation and one metric column, and
skipping it defers rather than compounds.
