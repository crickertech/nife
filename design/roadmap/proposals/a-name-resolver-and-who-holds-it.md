# Nothing here resolves a hostname, and in a capability system the resolver is a grant

**Status: PROPOSED 2026-09-05.** Found by asking whether a milestone covered `curl` or `wget`. None
does, and the reason they could not work is smaller and nearer than the HTTP or TLS this tree does
not have.

**Gate: NONE.** `smoltcp` already ships the socket; enabling it is a feature flag and a program.

## The gap, measured

`user/Cargo.toml` builds `smoltcp` 0.14 with `alloc`, `medium-ethernet`, `multicast`, `proto-ipv4`,
`proto-dhcpv4`, `socket-udp`, `socket-tcp` and `socket-dhcpv4`. **`socket-dns` is not among them**,
and nothing else in `crates/` or `user/src/` resolves names: the only greps that match "resolve" are
about capability names and generational tables.

So **a program cannot turn a hostname into an address.** That is upstream of everything else a
network client needs. `curl` with a perfect HTTP implementation and a perfect TLS stack still cannot
fetch `https://example.com`, and DHCP already hands us a nameserver we throw away.

**It is also upstream of two proposals and two milestones.** The TLS proposal's remaining consumers
are milestone 99's `clone` half and milestone 174, and both of those reach a host by name.

## Why this is worth its own piece rather than a line in a bigger one

**Because the interesting question is not the resolver, it is who holds it**, and that question does
not arise on Unix.

On Unix, name resolution is ambient. `/etc/resolv.conf` is readable by every process, `getaddrinfo`
is in libc, and any program can resolve any name against whatever the system decided. A program that
should only ever talk to one host can silently look up any other, and the first sign is usually in a
packet capture.

**Here it should be a capability.** "Which resolver may this program use, and for what" is a grant,
the same shape as every other authority in this system:

- A client granted a resolver capability that answers for exactly one domain **cannot be induced to
  look up anything else**, whatever bug or injection it carries.
- A resolver is itself a network client, so it is a confined component with a socket grant, not a
  library linked into everyone.
- A program with no resolver grant and a literal address still works, which is what the tree does
  today and should keep working.

This is the same observation as the trust-store one in
`design/roadmap/proposals/a-tls-stack-and-which-one.md`, and the two belong to the same family:
**the ambient parts of a Unix network client are exactly the parts a capability system should make
explicit.** It is also a small, concrete instance of
[§145](../../decisions/145-compartmentalization-at-process-cost.md)'s argument, which is otherwise
stated at the scale of a whole operating system.

## What it would take

- Enable `socket-dns` in `smoltcp`, which already implements the client.
- Decide where the resolver lives: inside `net_stack` behind a verb on `socket_proto`, or as its own
  confined program that holds a socket grant. **The second is more in keeping with the tree**
  (`net_stack` is already a userspace server and this would be a client of it), and it is the more
  expensive one, so it should be argued rather than assumed.
- Decide the capability's shape, which is the part worth thinking about: a resolver grant that is
  "any name" is barely better than ambient, and one that is "exactly these names" may be too rigid
  to use. A domain suffix is the obvious middle and obvious is not the same as right.
- Take the nameserver from DHCP, which `proto-dhcpv4` already receives and nothing currently reads.

## What it is not

**It is not `curl`.** A resolver plus HTTP plus TLS is three pieces and this is the first. There is
no consumer today for any of them, which is why this is a proposal.

**And it does not need mDNS.** `mdns_responder` exists and answers for names on the local link; that
is a different protocol solving a different problem, and reusing it here would be a category error.

## BUGS

- **`socket-dns`'s cost is unmeasured.** It is a feature flag on a dependency already in the graph,
  so the change is small, but nobody has built with it and looked at what it adds.
- **The capability shape is the whole design and this proposal does not settle it.** A domain-suffix
  grant is named as the obvious middle without evidence that it is workable.
- **No consumer.** By AGENTS.md's ranking function that is a reason to rank this below anything that
  has one, and there are two proposals in the same condition standing behind it.
