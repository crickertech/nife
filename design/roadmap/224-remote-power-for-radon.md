# 224. Nothing can power-cycle radon, so a hung soak needs a person

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from milestone 221's (the soak never
crosses cores, so build the hook that makes it) lane, which named it as the soak's remaining gap.
*(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** Not the technical kind. The plug exists and is wired; what is unresolved is a
network question on calef's own home network, and one of the answers weakens a security control he
may not want weakened.

**In brief.** **radon**'s power is on a Kasa KP303 strip. calef wired it on 2026-09-01, and outlet
2 is the board while another outlet feeds an external drive that must never be switched off.
`python-kasa` and a `vf2-power` wrapper are installed on patagonia, and the wrapper addresses the
board's outlet by alias with no way for a caller to name a different one, because an index is right
until a firmware update reorders the children.

**None of it can reach the plug.** Measured on 2026-09-01 from both patagonia and cordoba, by
broadcast and by unicast to every host on the subnet, with both the legacy and the newer discovery
protocols: **zero replies**. ARP for the plug's address is `incomplete` from wired and wireless
alike, while five other clients on the same subnet answer normally. The vendor app controls it,
which it can do through the vendor's cloud.

So the plug is deliberately isolated from other clients on the network it is joined to.

## Why it matters now rather than as a convenience

Fatal risk 5 (it cannot be made reliable on multicore, and the bugs appear only on silicon) wants
**sustained** runs, and after milestone 221 there are two experiments to sustain rather than one. A
soak that never crashes needs no power cycle, so this does not block a first run. **A soak that
hangs is exactly the outcome the experiment exists to produce**, and that outcome currently ends
with a board sitting hung until somebody walks to it.

`script/board-console` reads and never writes, so a boot that lands at U-Boot's prompt cannot be
rescued either.

## The options, none chosen, and the first is not free

- **Turn off client isolation** on the network the plug is joined to. One toggle, and it is a real
  security control: the isolation is what stops a compromised IoT device reaching **cordoba**, which
  holds the family's backups. Weakening it network-wide to control one plug is a larger change than
  it looks, and this block declines to recommend it.
- **Move the board's outlet to a plug that is already reachable**, leaving the KP303 alone. Avoids
  touching the strip an external drive depends on, and avoids weakening anything.
- **Accept manual power** and treat a hung soak as a bench event. Honest, and it costs an evening
  every time the experiment produces its most interesting result.

## BUGS

- **This block cannot verify its own diagnosis.** Client isolation is inferred from the symptom, a
  device that answers its vendor's cloud while being invisible to ARP from two machines on its
  subnet, and from the absence of any other explanation. Nobody has read the access point's
  configuration.
- **Remote power is not remote recovery.** A board wedged before U-Boot hands off is fixed by a power
  cycle; a board that needs a different card is not.
