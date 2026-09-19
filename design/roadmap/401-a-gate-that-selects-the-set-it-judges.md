# 401. A gate that selects the set it judges can pass by checking nothing

**Status: NOT-STARTED.** Filed 2026-09-14 as an unnumbered proposal by milestone 265, which broke
one and caught it by hand rather than by anything red; numbered 2026-09-19 by milestone 433's drain
of the proposal pile. **Premise re-read against the tree on 2026-09-19: the instance is closed and
the class is not**, which is the distinction this block is about. `script/lint` check 3's glob is
`crates/*protocol` now, and twelve lines of comment above it record the hazard by name (*"this loop
selects the crates it then judges, so a glob that matches nothing passes by checking nothing"*).
That is rung three. What this block proposes is rung two, and nothing of it exists: the loop still
has no post-loop assertion that it selected anything, no other selector in `script/` has one, and
nobody has enumerated the selectors, which is this block's own first deliverable and the thing that
decides whether the rest is an afternoon or a sweep.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** A lane can close this. It is one assertion per selector, no wire format, no syscall
surface, and no name calef has not already ruled on.

**In brief.** Several gates pick the things they judge with a pattern, then judge them. When the
pattern stops matching, the loop body never runs and the gate reports clean. Make an empty selection
a failure.

## The instance, which was live

`script/lint` check 3 enforced one spelling for contract crates:

```sh
for dir in "$(dirname "$0")"/../crates/*proto; do
    name="$(basename "$dir")"
    case "$name" in
        *_proto) ;;
        *) echo "lint: contract crate $name should be spelled *_proto" >&2; exit 1 ;;
    esac
done
```

Milestone 265 renamed every one of those crates to `_protocol`. The glob then matched no directory,
the loop ran zero times, and the check passed **by checking nothing**, on the one change that was
precisely its subject. It was fixed in 265's own pull request, by hand, because somebody went looking
for it; nothing in CI could have said a word.

## Why it is worth a milestone and not a fix

**It is a different failure from the one `design/naming.md` already records.** That note has a row for
a stale `--exclude`: a gate that has quietly stopped covering one crate. This is the same family and
strictly worse, because an exclusion that goes stale still covers everything else, while a **selector**
that goes stale covers nothing at all. The subject disappears in one step and the green is read as an
answer.

**The tell generalises and is worth writing down beside the fix:** a gate that passed before your
change and passes after it, on a change that is exactly what the gate is about, has probably stopped
looking. That is a thing a person notices and no machine can, which is why the mechanism below is
rung two rather than rung four.

## What to build

**One assertion per selector: the selection is non-empty.** For the shell case that is a counter and a
test after the loop; for the `cargo metadata` and `git grep` cases it is the same shape one language
over.

**Then enumerate the selectors.** This proposal deliberately does not claim the count, because
265 measured only what it tripped over. What is known:

- Two shell glob-driven loops in `script/` select by path pattern. One is check 3 above. The other
  (`notes/*.md design/*.md` in `script/lint`) cannot go empty while either directory exists, which is
  the shape to look for: a selector over a set that is *allowed* to become empty.
- The `--exclude` lists in `script/lint`, `script/coverage` and `xtask` are the adjacent failure
  `design/naming.md` already names, and cargo takes an unknown `--exclude` silently.
- `.cargo/mutants.toml`'s exclusion globs are named in that same table: a glob that matches nothing is
  not an error.
- Two gates already moved off hand-kept lists for a related reason and record the argument:
  `script/verify` and `script/falsifications` both read `cargo metadata` now. That is evidence the
  class is real and that the tree has been paying for it one instance at a time.

**And a second pass worth its own paragraph**, because it is where the count will come from: the
python embedded in `script/lint`, `script/roadmap`, `script/fatal-risks` and `script/names` selects
files and records with patterns throughout, and a regex that stops matching is the same defect wearing
a different syntax. `script/fatal-risks` already guards one case explicitly (`no-history`, which fires
when a shallow clone makes its own check unable to fail, on the stated ground that "a check that
cannot fail is worse than one that is absent"). That comment is the argument for this whole proposal,
written by a gate about itself.

## BUGS

- **Nobody has counted the selectors**, so the size of this is unknown. It may be an afternoon or it
  may be a sweep across every gate in `script/`. The enumeration is the first deliverable and it is
  the one that decides whether the rest is worth doing.
- **A non-empty assertion is not the same as a correct selector.** A glob that matches three of
  fifteen crates passes the assertion and still covers a fifth of its subject. Nothing proposed here
  catches that, and a check that tried would need to know the right count, which is the hand-kept list
  both `script/verify` and `script/falsifications` moved away from.

## Index row

Several gates pick the things they judge with a pattern and then judge them, so when the pattern
stops matching the loop body never runs and the gate reports clean. The live instance was
`script/lint` check 3, which enforced one spelling for contract crates by globbing `crates/*proto`:
milestone 265 renamed every one of those crates to `_protocol`, the glob matched nothing, and the
check passed by checking nothing on the one change that was precisely its subject. It was caught by
hand, in 265's own pull request, because somebody went looking; nothing in CI could have said a
word. It is a different failure from the stale `--exclude` `design/naming.md` already records, and
strictly worse, because an exclusion that goes stale still covers everything else where a selector
that goes stale covers nothing at all. The mechanism is one assertion per selector, that the
selection is non-empty, and the first deliverable is the enumeration, because nobody has counted
them: two shell glob loops in `script/`, the `--exclude` lists in `script/lint`, `script/coverage`
and `xtask` where cargo takes an unknown exclusion silently, `.cargo/mutants.toml`'s globs, and the
python embedded in four `script/` entry points, where a regex that stops matching is the same defect
in another syntax. `script/fatal-risks` already guards one case on the stated ground that a check
which cannot fail is worse than one that is absent, which is this block's argument written by a gate
about itself. The tell generalises and belongs beside the fix: a gate that passed before your change
and after it, on a change that is exactly what the gate is about, has probably stopped looking.
