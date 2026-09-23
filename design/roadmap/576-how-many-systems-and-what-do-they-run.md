# 576. How many systems are out there, and what do they run

**Status: NOT-STARTED.** The number is **provisional**: the integrator mints it at merge. Minted
2026-09-22 at calef's request: *"We want to know system count and packages subscribed to... the
metrics will help us make and maintain a better product if we know what people are using."*

**Gate: MILESTONE 198, DECISION.** There is nothing to count until something installs, so this
waits on milestone 198 (a package manager, and the trivial install that makes a second customer
possible); and what leaves a stranger's machine is the irreversible category this tree is most
careful about, so the shape of the report is calef's.

## What this is

`basalt` will know which packages a system subscribes to, and the installer will know a system
exists. Neither fact reaches us today. Without them we choose what to build by taste, and the
three principles say to choose by what a customer runs.

## Why this is harder here than it looks, and it is one sentence

**A package list is a fingerprint.** Counting installs is easy to do anonymously; counting *package
popularity* is the part that de-anonymises, because a set of a few hundred packages identifies a
machine better than an IP address does. Any design that ships "the list this system subscribes to"
has shipped an identifier, whatever it calls the field. So the two things calef asked for pull in
opposite directions and have to be answered separately rather than by one report format.

## The prior art, which has already solved half of this

- **Fedora's `countme`** is the strong one and it counts systems **with no identifier at all**. Each
  system adds a bucket to an existing repository request saying roughly how many weeks it has been
  since it first checked in, and the server counts distinct addresses per week per bucket. Nothing
  unique is ever sent or stored, and the age bucket is what separates a new install from a long-lived
  one without following either.
- **Debian's `popularity-contest`** is the honest cautionary half. Opt-in, and it submits a machine's
  package list under a persistent identifier, which is the fingerprint problem accepted openly rather
  than solved.
- **Homebrew's analytics** is the failure worth not repeating: opt-out, to a third party, and the
  breach of expectation cost more goodwill than the data was worth.

## The idea that makes this a nife milestone rather than a generic feature

**The reporter should be a capability-confined program, and that is a demonstration rather than a
precaution.** On this system a program holds exactly the authority it was handed. A reporter can be
given one endpoint capability and one read-only view of the subscription list, and **a person can
see what it holds and revoke it**, which on Linux requires trusting a promise in a manual page.

That inverts the usual telemetry argument. Everywhere else, "it only sends what we say it sends" is
a claim; here it is a property of the object graph, checkable from outside the program. If this OS
is a demonstrator, a confined reporter is one of the better things it could demonstrate, because
telemetry is exactly where users have learned not to believe anyone.

## What calef has to decide, and none of it should be guessed

1. **Opt-in or opt-out**, and if opt-in, what the installer asks and when. Every other decision is
   downstream of this one.
2. **Whether package subscriptions are reported at all**, given the paragraph above. There is a
   middle option: report each package independently, with no linkage between reports from one
   system, so popularity is countable and no list is ever assembled. It costs more requests and it
   is the only shape that gets the number without the fingerprint.
3. **What else is worth knowing.** Architecture and release are cheap and low-risk. A hardware
   profile is neither, and upgrade success or failure is arguably the most useful signal available
   and the most sensitive to get wrong.
4. **Where it lands, and who can read it.** A fact that has left a machine cannot be recalled, so
   retention and access are part of the design rather than operations detail.

## What must be true before any of it ships

- **The wire format is `anything two programs agree on`**, so it is decided once and carefully.
- **The thing that leaves the machine is the irreversible category.** §79 (password-equivalent material) is the precedent: an hour of argument was worth it because deleting the code
  afterwards does not unsend anything.
- **A `BUGS` section states plainly what the reporter can and cannot see**, where a person meets it.
  A telemetry feature whose limits are documented anywhere other than beside the feature has
  learned nothing from why people distrust telemetry.

## Index row

How many systems run this, and which packages they subscribe to, learned without learning who they
are. Counting installs is solved prior art (Fedora's `countme` uses no identifier at all); counting
package popularity is the hard half, because a subscription list is a fingerprint. The reporter is
capability-confined, so what it may send is a checkable property rather than a promise.
