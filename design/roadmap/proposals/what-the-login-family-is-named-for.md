# What the `login` family is named for

**Status: PROPOSED 2026-09-14.** Raised while working the unratified worklist: `components/src/login.rs`
came up for ratification and calef parked it, choosing to decide the whole family at once rather than
sign the least consequential member of it.

**Gate: DECISION.** calef names things, and this is one stem carried by five. Nothing is blocked
meanwhile: every file keeps the name it has, and `script/names` takes `recorded` as a truthful answer.

## Why the program could not be ratified alone

**Nothing types `login`.** Its own module docs say it is started by `kernel/src/user/login_service.rs`,
the same way `credentialer` is, and *"is not itself reachable"* from the prompt. Clients reach it by
`CONNECT` on a front-door endpoint. That matters because the argument for keeping the word was the
protected class (*the Unix name for the program that answers exactly this request*), and that
argument rests on a person meeting it. The only things that meet this one are other programs and a
reader of the source.

**And it does not authenticate.** Asked directly, the answer is no: it holds `WRITE` on the credential
service's verify endpoint and **relays**; `components/src/credentialer.rs` checks the secret. What this
program does is mint a session's worth of capabilities on the answer: a fresh `fs_subtree_caretaker`,
a budget, a logout ticket, the terminal when free. With `principal` ratified the same day, the sentence
is available: **it turns an identity into a principal.**

So milestone 63's own test points the other way from the current name. 63 refused to name the credential
service for its resource because *"a credential service never hands you a credential"*. By that test, a
login service never hands you a login; it hands you a principal's capabilities.

**The counter-argument is real and is why this is a decision rather than a defect.** `login` is the
field's term of art for this role even when only programs speak it, and a reader arriving from Unix
lands in the right place.

## The family, measured 2026-09-14

708 occurrences of the word across 94 files. Five names carry the stem:

| name | what it is |
|---|---|
| `crates/login_proto` | the wire vocabulary |
| `components/src/login.rs` | the service |
| `fixtures/src/login_test_client.rs` | its test client, **name ratified 2026-09-14** |
| `kernel/src/user/login_service.rs` | the kernel-side spawn |
| `kernel/src/user/login_tests.rs` | the suite |

## The sequencing, which is the urgent half

**`login_proto` is already in milestone 265's table**, to become `login_protocol`. That rename keeps
the stem. If the stem changes afterwards, those files are renamed twice, which is the exact cost
265's own block says it was written to avoid, and why calef's `mdns` and `ntp` stem rulings were taken
*into* 265 rather than performed separately.

**So this wants answering before 265 performs**, and its answer belongs in 265's table beside the four
stems already there.

## BUGS

- **`login_test_client` was ratified on 2026-09-14 while its stem was open**, deliberately: the
  ratification is recorded as being on the name and not the shape, and the `<service>_test_client`
  pattern survives whatever the service is called. A stem change would still move it.
- **This proposal does not recommend.** The tenet says recommend on reversible forks and give options
  on irreversible ones; a stem across 94 files, one of them a wire vocabulary two programs agree on,
  is the second kind.
