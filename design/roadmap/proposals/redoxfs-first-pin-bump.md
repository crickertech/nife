# The first RedoxFS pin bump, on the engine that holds backups

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 203's block.

**Gate: NONE.** Everything this needs exists: the pinned sha, the five recorded divergences,
`script/vendor-verify --write-patch`, the suite, and milestone 37's crash injector. One clause is
calef's if it comes up, and it is named below rather than blocking the start.

**In brief.** `script/vendor-watch`'s first run found upstream RedoxFS **37 non-merge commits ahead
of the pinned sha**, including `fix: do not hardcode # of sectors per block`, with 0.9.1 still the
newest published version. Bumping the pin means raising it, re-applying the five divergences,
regenerating the patch with `script/vendor-verify --write-patch`, and re-running the suite plus
milestone 37's crash injector. Nobody has ever performed this job in this tree, which is the second
reason to do it: the procedure in `vendor/README.md`'s "Bumping a pin" section has never been
executed.

## Why this matters

This is the case milestone 203 was built to catch, and it was live on the day the watch was built. A
correctness fix is sitting on upstream master and will not appear in a published version on any
schedule anyone here controls. The pinned engine is the one holding a filesystem, and the whole
argument for vendoring rather than writing it (§46: vendor where correctness is won by exposure) is
that upstream's exposure accrues to us. It only accrues if somebody moves the pin.

There is a second cost that grows on its own. The five divergences are re-applied by hand at every
bump, and three of them are re-applied forever. The further the pin drifts behind, the more upstream
restructuring those three have to survive at once, and `vendor/README.md` is already honest that if
upstream restructures the code they touch they may not be re-applicable at all. Thirty-seven commits
is the cheapest this job will ever be.

## The clause that is calef's, if it fires

Divergence 3 is kept rather than reverted only because reverting it is calef's call. A bump is where
the five get re-litigated. If a divergence will not re-apply, the honest outcome is a decision
(rewrite it, stay on the pin and record why, or fork permanently) rather than a forced patch, and
that is where a lane stops and writes it up.

## Where it came from

Milestone 203 (nothing will ever tell us RedoxFS moved) built the detection half and said plainly
that acting on the finding was a bump lane's job, not that milestone's. Its own follow-on named the
work: *"Perform the first real RedoxFS pin bump. [...] Nobody owns it, and this is the engine
holding backups."*
