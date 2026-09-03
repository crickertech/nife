"""Reading this tree's Rust source, for the `script/` entry points that count things in it.

**Why this file exists.** Three derivations were written twice, once in a gate and once in the
dashboard, because each lived inside a `#!/bin/sh` script's inline `python3` heredoc: the `unsafe`
census (`script/lint` and `script/metrics`), the comment-and-literal strip the code-line and
comment-line split is built on (the same pair), and the proof-harness count (`script/falsifications`,
`script/metrics`, and a third derivation inside `script/lint`). A gate that changed its definition
would leave the dashboard quietly asserting the old one, with both looking authoritative and nothing
firing. See milestone 236 (three derivations are copied between scripts, and nothing notices when
they drift).

**The premise that made the copies look unavoidable is false, and checking it is what shrank this
milestone.** `script/lint` and `script/metrics` both `cd` to the repository root before they
`exec python3`, so their heredocs have a stable working directory, and a heredoc with a stable
working directory can `sys.path.insert(0, 'scripts')` and import like anything else. Nothing about
being a shell entry point prevented it. That matters because the alternative on the table was a host
crate, which would have made three `script/` commands depend on a `cargo build` and changed what a
`script/` command is; this changes nothing a caller can see.

**What is deliberately NOT here, and it is the part worth reading.** The file *sourcing* stays in
each caller, and only the derivation over text is shared. `script/lint` walks `git ls-files` and
reads the working tree, because a gate is asked about the tree in front of it. `script/metrics`
streams blobs out of `git cat-file --batch` at eight historical revisions and never checks anything
out, because a report is asked about the past and AGENTS.md forbids the checkout that would be
needed. That difference is exactly the tolerance milestone 236's block warned a shared
implementation might dissolve, so it is kept where it belongs: in the input, not in the count.

**And the harness count is still derived twice, on purpose.** `script/lint` and
`script/falsifications` attribute each harness to a workspace package from `cargo metadata`, which
needs a checked-out workspace and a cargo that can parse it. `script/metrics` cannot have either: it
reads blobs from July, nothing is on disk, and its own header records that nothing there is allowed
to build. `harness_count` below is the pure-text derivation the report uses; `script/lint` keeps its
own and **compares the two**, which is the honest answer for a question two callers genuinely cannot
ask the same way.

Every function here takes `files`, an iterable of `(path, text)` pairs, with `path` relative to the
repository root and slash-separated.
"""

import re

# The comment and literal stripper. Rust source is not a regular language, so this is lexer-shaped
# rather than a parse, and every count built on it is honest rather than close: a naive grep for
# `unsafe {` over this tree scores 1071 against the census's 1057, and the fourteen it invents are
# all `unsafe {}` written inside a `//!` doc example in `intrusive`, `ipc`, `paging` and
# `user_heap`. Those are documentation ABOUT unsafe, and a ceiling that counted them would fire when
# somebody improved a doc comment.
#
# Block comments are matched non-greedily and do NOT nest, which Rust's do. The tree has none
# nested; a nested one would end the strip early and could only ADD to a count, which fails loud
# rather than quiet.
_NON_CODE = re.compile(
    r'/\*.*?\*/'                        # block comment
    r'|//[^\n]*'                        # line comment, doc comment included
    r'|r(#*)"(?:.|\n)*?"\1'             # raw string, any hash count
    r'|"(?:[^"\\\n]|\\.)*"'             # ordinary string
    r"|'(?:[^'\\]|\\.)'",               # char literal (a lifetime has no closing quote, so it is
    re.S)                               # left alone, which is what we want)


def strip_non_code(text):
    """Blank every comment and literal, keeping the line structure so line numbers still line up.

    Replacing with spaces rather than deleting is what makes `non_blank` on the result a **code
    line** count: a comment block and a multi-line string literal both become blank lines and
    contribute nothing, so it is a quantity nobody can move by writing prose.
    """
    return _NON_CODE.sub(lambda m: re.sub(r'[^\n]', ' ', m.group(0)), text)


def non_blank(text):
    """Lines with something other than whitespace on them."""
    return sum(1 for line in text.split('\n') if line.strip())


# --- the unsafe census (milestone 134) -----------------------------------------------------------
#
# Raised by calef on 2026-08-18 as one question: "how much unsafe code is there in a code base, and
# is that something we should be monitoring and driving in a particular direction over time?" See
# notes/register-of-measures.md and notes/unsafe-obligations.md.
#
# Shapes assumed, each checked against the real tree rather than against expectation:
#   - a block is `unsafe` then optional whitespace then `{`, MINUS the `unsafe extern "C" {` form,
#     which is a linkage declaration and not a block anybody reasons about (zero in the tree today,
#     subtracted anyway so the day one lands it does not read as unsafe code);
#   - a thread-safety claim is an `unsafe impl` of Send or Sync SPECIFICALLY, which is a different
#     animal from an `unsafe impl` of an unsafe trait (`GlobalAlloc`, `intrusive_fifo::Node`): the
#     trait is unsafe there because its own contract is, while `unsafe impl Send for T` is a
#     hand-written assertion that the compiler is wrong about T, and it is the most consequential
#     unsafe in this tree. The `[^{;]*?` tail keeps it inside one item header so a body cannot
#     donate a match.
#
# There is deliberately NO `unsafe fn` count here. `script/lint`'s `==> unsafe fn contracts` check
# derives one and prints it, and a second count of the same thing taken from a slightly different
# scope is the drift milestone 134 exists to stop.
UNSAFE_BLOCK = re.compile(r'\bunsafe\s*\{')
UNSAFE_EXTERN = re.compile(r'\bunsafe\s+extern\s+"[^"]*"\s*\{')
THREAD_SAFETY = re.compile(r'\bunsafe\s+impl\b[^{;]*?\b(?:Send|Sync)\s+for\b')

# What is out of the census, and every exclusion has a reason rather than a convenience.
#
# `bench/host/` holds the Linux and macOS programs the cross-OS comparison runs: ~100 `unsafe`
# blocks of libc FFI that those operating systems require and that say nothing about nife's
# soundness. Counting them would move the census every time somebody added a comparison. `xtask/`,
# `tools/`, `fuzz/` and `scripts/` are build and image tooling that runs on the developer's Mac and
# cannot fault the kernel.
#
# `patches/` is excluded for a different reason and it is the one worth reading: it holds our
# platform layer for Rust's `std`, which DOES run on the machine, so this is a real hole rather than
# a tidy boundary. It is out because a ceiling asserts a direction, and that code's shape is
# upstream's rather than ours: it implements `std`'s internal interfaces and cannot be restructured
# to hold fewer unsafe blocks without diverging further from the crate we are trying to track. It
# also matches the `unsafe fn contracts` check's scope, so there is one answer to "which Rust is
# ours" instead of two. The 37 blocks it leaves uncounted are recorded in
# notes/register-of-measures.md's BUGS, where a reader meets them.
#
# The list is exclusions rather than inclusions on purpose: a new subsystem directory is counted by
# default, and being left out has to be somebody's decision.
HOST_ONLY = ('bench/host/', 'xtask/', 'tools/', 'fuzz/', 'scripts/', 'patches/')


def unsafe_census(files):
    """Every unsafe number this tree tracks, in one pass over the Rust that runs on nife.

    Returns `outside_arch`, `inside_arch`, `thread_safety`, `code_lines` and `density`.

    **Density rather than a raw count, measured before it was chosen.** Outside
    `kernel/src/arch/` the block count went 171 to 747 between 2026-07-15 and 2026-08-18, almost all
    of it the system being BUILT rather than anything drifting, so a raw ceiling would have fired on
    nearly every lane. Per 10,000 code lines the same period reads 22.8, then 11.8, then 9.3,
    falling at every sample: the tree is getting proportionally safer, and holding that claim is
    what the ceiling is for. Per 10,000 rather than per 1,000 because the counted-claim markers read
    integers, and 93 keeps a digit that 9 would throw away.

    The denominator is the non-arch code only, so the density and its numerator describe the same
    code. Mixing arch lines in would let assembly-heavy months dilute a number that is deliberately
    not about assembly.
    """
    out = {'outside_arch': 0, 'inside_arch': 0, 'thread_safety': 0, 'code_lines': 0}
    for path, text in files:
        if path.startswith(HOST_ONLY):
            continue
        code = strip_non_code(text)
        blocks = len(UNSAFE_BLOCK.findall(code)) - len(UNSAFE_EXTERN.findall(code))
        out['thread_safety'] += len(THREAD_SAFETY.findall(code))
        if path.startswith('kernel/src/arch/'):
            out['inside_arch'] += blocks
        else:
            out['outside_arch'] += blocks
            out['code_lines'] += non_blank(code)
    # Truncated rather than rounded: a ceiling must never fail a tree that sits exactly on it.
    out['density'] = (10000 * out['outside_arch'] // out['code_lines']
                      if out['code_lines'] else 0)
    return out


# --- the proof-harness count, the text-only derivation -------------------------------------------
#
# The falsification record milestone 194 built and DECISIONS §134 ratified: what evidence each Kani
# harness carries that it can fail. `replayable` has a patch a script applies to turn the harness
# red; `attested` is a person who broke it and watched; `unfalsified` is the claim's honest
# denominator. A harness with no record at all counts as unfalsified, which is what it is.
PROOF = re.compile(r'#\[kani::proof\b')
FALSIFICATION = re.compile(r'\bFalsification:\s+(replayable|attested|unfalsified)\b')

# Directories holding `#[kani::proof]` that belong to no workspace package, expressed as path
# prefixes because that is all a text-only derivation has. `scripts/` is the `kani-lint-shim` source
# `script/lint` compiles by hand; `patches/` is our `std` platform layer. `vendor/redoxfs` is the
# third such file and is already outside every caller's file set.
NOT_A_PACKAGE = ('scripts/', 'patches/')


def harness_count(files):
    """Proof harnesses and how many carry a falsification record, from the source text alone.

    `script/lint` and `script/falsifications` answer the same question by attributing each harness
    to a workspace package out of `cargo metadata`, which is the better derivation where it can run
    and is why they keep it. This one exists for `script/metrics`, which reads blobs from revisions
    that are not checked out and could not run cargo against them if it wanted to. `script/lint`
    runs both and fails on a disagreement, which is what keeps the two honest.
    """
    total = falsified = 0
    for path, text in files:
        if path.startswith(NOT_A_PACKAGE):
            continue
        # Stripped, so the sentence in `kernel/src/syscall.rs` explaining what a `#[kani::proof]`
        # is does not count as one. A raw grep counts 151 where the tree has 146.
        total += len(PROOF.findall(strip_non_code(text)))
        # Raw, because a falsification record lives in a doc comment by construction.
        falsified += sum(1 for kind in FALSIFICATION.findall(text)
                         if kind in ('replayable', 'attested'))
    return {'total': total, 'falsified': falsified, 'unfalsified': max(total - falsified, 0)}
