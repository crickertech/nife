# What rented metal costs, for all three architectures

*Name provisional (`rented-metal`), per the rule that a lane ships a provisional name and says so.*

**Why this file exists.** §203 (capacity is rented rather than bought) ruled *rent, do not buy*, and
left exactly one line open: which provider, and the spend split. Nobody had acted on it. calef,
2026-09-23: *"we want to price all three architectures and lean heavily on free/trial options to keep
costs low."* This is the priced comparison that open line needs. **No account was created and nothing
was spent.** Every figure below carries the page it came from and the date it was read, which was
**2026-09-23** for all of them.

**What this is not.** It is not a recommendation to sign up today, and it is not a benchmark. It
prices access to machines, against the four things a machine must do before nife can use it at all.

---

## The four hard requirements, and the fifth column

An offer that fails any of the first four is **not cheaper, it is unusable**, and it is listed here
as failing rather than as qualifying-with-a-caveat.

1. **nife must boot on it.** A documented boot path and the ability to put your own kernel on the
   disk. §203 refuses closed appliances at any price for this reason, and a virtual machine that
   only boots images from a vendor library fails it just as completely as an appliance does.
2. **A serial console**, or an equivalent out-of-band channel, because a kernel that panics before
   userspace has nothing else to say.
3. **A power or reset API**, since removing the person from the loop is most of the point.
4. **Multiple cores**, for fatal risk 5, whose whole premise is about multicore.

**The fifth column is not a requirement, it is the independent variable.** calef, 2026-09-23: *"I
also think it is helpful to have multiple cloud platforms to minimize our own cloud platform
assumptions"*, and *"a nife that runs on one cloud platform but not another is also its own form of
risk."* So each row also records **how a custom kernel is delivered, through what channel the console
and power are reached, and what the machine hands the kernel at entry.** Where two providers of the
same architecture differ there, the difference is the thing being bought. Where they do not differ,
this file says so rather than manufacturing a diversity argument.

---

## x86_64

Commodity, and the column with real choice in it.

| Provider | Offer | Cores | RAM | Per hour | Per month | Free or trial | 1 boot | 2 console | 3 power API | 4 cores | Source, read 2026-09-23 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Scaleway | Elastic Metal EM-A116X-SSD (Xeon E3-1220) | 4C/4T | 32 GB | €0.077 | €27.99 | none | yes | yes | yes | yes | [pricing](https://www.scaleway.com/en/pricing/elastic-metal/) |
| Scaleway | Elastic Metal EM-A610R-NVMe (Ryzen PRO 3600) | 6C/12T | 32 GB | €0.11 | €39.99 | none | yes | yes | yes | yes | [pricing](https://www.scaleway.com/en/pricing/elastic-metal/) |
| Hetzner | Dedicated AX42-1 (Ryzen 7 PRO 8700GE) | 8C/16T | 64 GB | n/a | €97.30 | none | yes | **no, not without asking** | partial | yes | [price adjustment](https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/), [AX42](https://www.hetzner.com/dedicated-rootserver/ax42/), [custom images](https://docs.hetzner.com/robot/dedicated-server/operating-systems/installing-custom-images/) |
| OVHcloud | Kimsufi / Eco KS-B (Xeon E5-1620v2) | 4C/8T | 32 GB | n/a | $11.10 | none | unclear | **no** | unclear | yes | [Kimsufi](https://eco.ovhcloud.com/en/kimsufi/), [IPMI](https://docs.ovhcloud.com/en/guides/bare-metal-cloud/dedicated-servers/ipmi) |
| OVHcloud | Rise RISE-1 (Xeon-E 2386G) | 6C/12T | 32 GB+ | n/a | $64 | none | yes | yes | yes | yes | [Rise](https://eco.ovhcloud.com/en/rise/), [IPMI](https://docs.ovhcloud.com/en/guides/bare-metal-cloud/dedicated-servers/ipmi) |
| Oracle | Always Free VM.Standard.E2.1.Micro | 1/8 OCPU | 1 GB | free | free | Always Free, 2 instances | unclear | yes | yes | **no** | [Always Free](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm) |
| Equinix | Metal, any plan | - | - | - | - | - | **gone** | - | - | - | [sunset](https://www.latitude.sh/blog/equinix-metal-sunset-how-to-reduce-the-migration-burden) |

**Equinix Metal is not an option and should be struck from any future list.** Equinix announced the
shutdown of its bare metal platform **by 30 June 2026**, which is already past. §203 named it as a
candidate; it no longer exists.

**OVH's cheap range is the trap in this column.** Kimsufi at $11.10 a month is the lowest price on
the page and OVH's own IPMI guide says the feature *"might be unavailable or limited on servers of
the Eco product line"*. No console means no way to see a kernel that dies before userspace, which is
precisely the failure this hardware is being rented to observe. It is listed as failing requirement 2
rather than as cheap. OVH's **Rise** range does have IPMI, with HTML KVM, a Java applet, **serial
over LAN through the browser and serial over LAN over SSH**, and that is the offer worth naming, at
roughly six times the price.

**Hetzner is the other one that looks better than it is, for this job.** Custom images are supported
and documented, but by two routes that both cost a person: `installimage` from the rescue system,
which wants a `.tar.gz` of a Linux distribution rather than a raw kernel image, or **a KVM console
requested through Robot support**, which mounts an ISO over SMB. There is a *"24h reset service via
web interface"*. That is a support ticket in the loop, not an API, and Hetzner's own page says
*"Hetzner Online doesn't offer any support for the installation of custom images and can't offer any
warranty on the functionality."* At €97.30 a month for AX42-1 it is also the most expensive row that
is not AWS. It fails requirement 2 in the form this project needs it: a console you have to ask for
is not a console you can script.

**Oracle's Always Free x86 tier fails on cores before anything else.** Two `VM.Standard.E2.1.Micro`
instances at 1/8 OCPU and 1 GB each cannot test a multicore claim, so requirement 4 ends it. It is
genuinely free and genuinely useless for this.

### x86_64 recommendation

**Scaleway Elastic Metal EM-A116X-SSD, €0.077 an hour or €27.99 a month.** Four real cores, serial
console, API power actions, hourly billing so a bench evening costs under a euro, and the same
provisioning surface as the RISC-V machine in the third table, which is worth something on its own.

**And a second provider for this architecture is worth buying, which is the one place this file says
that.** See "What a second provider is a test of" below: OVH Rise is the second, at $64 a month, and
the reason is that its boot delivery and console channel genuinely differ from Scaleway's rather than
duplicating them.

---

## aarch64

The thinnest column of the three once the requirements are applied, which was not the expected
answer.

| Provider | Offer | Cores | RAM | Per hour | Per month | Free or trial | 1 boot | 2 console | 3 power API | 4 cores | Source, read 2026-09-23 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| AWS | EC2 `c7g.metal` (Graviton3) | 64 vCPU | 128 GiB | $2.32 | n/a (hourly) | free tier does not cover it | probably, unverified | yes | yes | yes | [c7g](https://aws.amazon.com/ec2/instance-types/c7g/), [price](https://instances.vantage.sh/aws/ec2/c7g.metal), [serial console](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-serial-console-prerequisites.html) |
| Oracle | Always Free Ampere A1 | 2 OCPU | 12 GB | free | free | Always Free, standing | unclear, see below | yes | yes | yes, 2 | [Always Free](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm), [BYOI](https://docs.oracle.com/en-us/iaas/Content/Compute/References/bringyourownimage.htm), [console](https://docs.oracle.com/en-us/iaas/Content/Compute/References/serialconsole.htm) |
| Hetzner | RX line (Arm64 dedicated) | - | - | - | - | - | **unavailable** | - | - | - | [RX matrix](https://www.hetzner.com/dedicated-rootserver/matrix-rx/) |
| Hetzner | Cloud CAX31 (Ampere Altra VM) | 8 vCPU | 16 GB | n/a | ~€15.99 to €25.57 | none | **no** | VNC only | yes | yes | [cloud](https://www.hetzner.com/cloud/), price from third parties, see BUGS |
| Scaleway | Elastic Metal, Arm | - | - | - | - | - | **none offered** | - | - | - | [pricing](https://www.scaleway.com/en/pricing/elastic-metal/) |

**Hetzner's Arm64 dedicated line is not orderable.** Its own matrix page says *"Unfortunately, we are
currently unable to offer any RX servers."* Its Arm **cloud** line (CAX) exists and is cheap, and
fails requirement 1: Hetzner Cloud has no custom-image upload and no arbitrary ISO attach, only its
own library plus the snapshot and rescue-mode workarounds, and its console is a VNC view of an
emulated display rather than a serial line. A rescue-mode `dd` of a raw image is a real path that
third-party tooling exists for, but it is a workaround against a provider that does not support it,
and on a VM whose console cannot show early kernel output. Not qualifying.

**Scaleway has no Arm Elastic Metal at all.** Their published Elastic Metal price list is x86 from
end to end, which was checked rather than assumed.

**AWS `c7g.metal` is the only real aarch64 bare metal in this table, and its requirement 1 is
"probably".** The EC2 serial console prerequisites list *"All bare metal instances except"* a named
set, and `c7g.metal` is not in the exclusion list, so requirement 2 is documented and free.
Requirement 3 is the EC2 API. Requirement 4 is 64 vCPUs. Requirement 1 is the unverified one: EC2
boots an AMI, so nife would have to ship as a disk image with a UEFI boot path that Graviton's
firmware will start. That is a port of the same shape §203 describes for the RISC-V machine, not a
switch, and nothing on AWS's own pages says an arbitrary kernel will boot. **It is listed as
unverified because it was not verified, and it would be dishonest to score it yes.**

**Oracle's Always Free Ampere A1 is the most interesting free offer on this page and the one most
likely to disappoint.** The Always Free allowance is *"1,500 OCPU hours per month"* and *"9,000 GB
hours per month"*, described as *"equivalent to 2 OCPUs and 12 GB of memory"*. Serial console over
SSH exists and is documented, power actions are API-driven, and two OCPUs are two cores. Requirement
1 is where it breaks down, and the Oracle page is genuinely ambiguous rather than negative: custom
imported images are supported, **Arm shapes support them in paravirtualized mode only**, the listed
supported guests are all Linux and Windows, the stated requirement is *"Linux kernel version 3.4 or
later"* for paravirtualized drivers, and the firmware line says *"BIOS required for paravirtualized
instances"*, which does not obviously describe an aarch64 machine. Read flatly, the page describes a
path for Linux guests and says nothing that promises a non-Linux kernel will boot. **Ambiguous, and
recorded as ambiguous.**

### aarch64 recommendation

**None of these qualifies today without work that has not been done.** That is the honest answer and
it is worth stating plainly rather than picking the least-bad row.

If one must be picked, it is **AWS `c7g.metal` at $2.32 an hour**, because it is the only offer where
all four requirements are either documented or plausibly reachable, and because hourly billing makes
a failed experiment cost a few dollars. The work it implies is a UEFI-bootable aarch64 image, which
is a milestone rather than a purchase.

The **free** aarch64 option worth one evening of somebody's curiosity is **Oracle Always Free Ampere
A1**, precisely because it costs nothing to find out whether requirement 1 holds. It should be
described as an experiment, not as coverage.

---

## riscv64

One offer, and it is the one §203 already found.

| Provider | Offer | Cores | RAM | Per hour | Per month | Free or trial | 1 boot | 2 console | 3 power API | 4 cores | Source, read 2026-09-23 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Scaleway | Elastic Metal EM-RV1 (T-Head TH1520, C910, RV64GC, 1.85 GHz) | 4C/4T | 16 GB | €0.042 | €15.99 | none | yes | yes | yes | yes | [RV1](https://www.scaleway.com/en/elastic-metal-rv1/), [Labs](https://labs.scaleway.com/en/em-rv1/) |
| RISE | RISC-V GitHub Actions runners | 1 job per node | - | free | free | free, no allowlist | **no** | no | no | - | [announcement](https://riseproject.dev/2026/03/24/announcing-the-rise-risc-v-runners-free-native-risc-v-ci-on-github/) |
| AWS, Azure, Hetzner, OVH | - | - | - | - | - | - | **nothing offered** | - | - | - | [survey](https://en.wikipedia.org/wiki/RISC-V_ecosystem) |

**§203's figures are confirmed.** The RV1 page gives the SoC, four cores at 1.85 GHz, 16 GB LPDDR4,
128 GB eMMC, 100 Mbit/s networking, and **€0.042 an hour or €15.99 a month excluding VAT**, for
`EM-RV1-C4M16S128-A`. The Labs page confirms the Labs status and is blunter than §203 was: *"EM-RV1s
are considered as 'Labs' services, with a service level agreement (SLA) contractually defined in our
special conditions for BETA services"*, and the **SLA is 0%**. Custom operating systems are supported
on the record: *"Access to the server's serial console is available for installing the most exotic
operating systems."*

**The free RISC-V offer is real and does not qualify.** RISE runs free native RISC-V GitHub Actions
runners for any open source project on GitHub, with no approval process and no allowlist, on
Scaleway EM-RV1 hardware, one job per node. It is genuinely excellent and it is **a Kubernetes pod on
a shared RISC-V node**, so it cannot boot a kernel. It fails requirement 1 and with it 2 and 3. It is
worth knowing about for a different reason: if nife ever needs a RISC-V *build* or *cross-check* leg
in CI, this is free hardware for it, and that is a separate proposal from this file's subject.

### riscv64 recommendation

**Scaleway EM-RV1, €0.042 an hour or €15.99 a month.** It is the only offer that qualifies, in any
column, at any price. Buy it on its own merits and understand what it is not, below.

---

## What riscv64 cannot buy

**Rental does not substitute for radon, and §203 already said so.** The TH1520 is a **different SoC**
from radon's JH7110, so booting nife on an RV1 is a port, with its own UART address, its own device
tree, and whatever its firmware hands over. That makes it a **second RISC-V implementation**, which
is a good thing to have and a different thing from a spare radon.

The RV1's boot path is documented and is the concrete form of that port. Scaleway's own account:
the early stages are *"the full boot process (which cannot be modified by the customer)"*, a first
stage U-Boot SPL fixed in eMMC *"which ensures a stable and immutable boot"*, then U-Boot Proper,
and only then the customer-controlled final stage, *"again U-Boot Proper, but configured
differently, which loads OpenSBI and the Linux kernel."* So nife arrives on this machine as
something U-Boot loads with OpenSBI underneath it, which is the same shape radon uses and a
different set of addresses and tables.

**Three milestones name radon's JH7110 specifically and cannot be settled by rental at any price:**
milestone 163 (the JH7110's PCIe root complex), milestone 239 (radon's device tree does not describe
the TRNG), and milestone 493 (a disk-file job mix needs a disk radon can drive). An RV1 has no PCIe
root complex to drive, a different device tree, and 128 GB of eMMC rather than radon's storage. It
also has no remote power cycle of radon, which is milestone 224 (nothing can power-cycle radon)'s
subject, and rented metal does nothing for that either.

**And the RV1 will not settle milestone 225 (run the soak on radon, argon and xenon)'s RISC-V leg
as written**, because that milestone names radon. It would settle a *different* and arguably more
interesting question: whether the soak behaves the same on a second RISC-V implementation. That is
worth proposing as its own milestone rather than quietly redefining 225's.

---

## What a second provider is a test of

calef, 2026-09-23: *"a nife that runs on one cloud platform but not another is also its own form of
risk."* This section states that as an experiment, because framing it as redundancy would waste it.

**The gap it sits in.** Fatal risk 9 (the HAL is a fiction, and an architecture costs a restructure
rather than a port) rests on a claim about *"actual functioning on the three silicons"*. That is
about **architectures**. Two x86_64 providers are the **same** silicon with different firmware,
different boot delivery, and different tables at entry. A nife that boots on xenon and not on some
provider's metal falls through that gap entirely, and the HAL being a fiction would surface **first**
in that form, long before it surfaced as an architecture costing a restructure. Whether that widens
risk 9 or becomes its own entry is calef's call; this file does not touch `design/fatal-risks.md`.

**The hypothesis, stated before the numbers exist**, in the shape this tree already uses: nife boots
unchanged on two providers' x86_64 metal, and the only differences are in how the image and the
console are delivered. **A failure is the valuable outcome**: it would name a concrete assumption
nife makes about what a machine hands it at entry, and that assumption is currently invisible because
every x86_64 boot this project has ever done was on QEMU or on xenon.

**Finding no difference is also a result, and it would weaken the case for the second account.** This
file does not get to assume diversity.

### The fifth column, side by side, for x86_64

| | Scaleway Elastic Metal | OVHcloud Rise | Hetzner dedicated |
|---|---|---|---|
| How a custom kernel is delivered | rescue mode plus the serial console; documented as supporting *"the most exotic operating systems"* on the RV1, with `Reinstall a server`, `Configure custom disk partitioning` and `Use rescue mode` as the Elastic Metal how-tos | IPMI virtual media and netboot, with the OS installed through a KVM session | `installimage` from the rescue system, which wants a distribution tarball, **or** an ISO mounted over SMB through a KVM console **obtained by a support request** |
| Console channel | serial console, over SSH, activated per account | IPMI: HTML KVM, Java KVM, **serial over LAN in the browser and serial over LAN over SSH** | KVM console on request; no standing serial channel documented |
| Power channel | API `Start`, `Stop`, `Reboot` server actions | control panel and IPMI; API coverage not confirmed here | *"24h reset service via web interface"* |
| What the machine presents at entry | x86 firmware, provider-installed; not documented further | x86 firmware, vendor BMC present | x86 firmware, no BMC exposed by default |

**These are not the same interface, and that is the finding.** Scaleway hands you a serial line over
SSH and three REST verbs. OVH hands you a BMC, which is a different object with its own virtual
media and its own serial-over-LAN, and a genuinely different way of getting an image onto the disk.
Hetzner puts a human in the loop for the console. An automation built only against Scaleway would
express "provision a machine" as three REST calls and a serial SSH session, and would not even have
a verb for "mount this ISO through the BMC".

**Where it is weaker than it sounds**: the *firmware* row is the one that matters most for risk 9 and
it is the one this file could not fill in. None of the three publishes what their machines hand a
kernel at entry, which is exactly the kind of thing that gets discovered by printing it and getting
zero. So the anti-lock-in argument about **our tooling** is solidly evidenced by the table above; the
argument about **nife's own boot assumptions** is a hypothesis that buying the second machine tests,
and it is not evidence yet.

### What multi-vendor costs

Naming it, because pretending it is free would be the dishonest half.

- **Three accounts, three billing relationships.** Three invoices, three renewal dates, three places a
  forgotten machine accrues charges.
- **Three provisioning APIs and three auth models.** Scaleway's CLI and secret key, OVH's application
  key and consumer key, AWS's IAM. Any script that automates "get me a machine" has to speak all
  three or be written once per vendor.
- **Three sets of credentials to hold and rotate**, for one person, which is the kind of thing that
  quietly stops happening.
- **Three console mechanisms and three ways to get an image onto a disk**, which is the same fact as
  the benefit above read from the cost side. A second vendor is not a drop-in.

**The honest balance.** Two providers for x86_64 is worth it because the interfaces genuinely differ
and the difference is the experiment. A third x86_64 provider is not, and a second provider for
aarch64 or riscv64 is not purchasable today because there is barely a first.

---

## What it would cost to settle milestone 225's x86_64 and aarch64 legs

Milestone 225 (run the soak on radon, argon and xenon)'s gate is `HARDWARE` in milestone 53 (the
board's own peripherals)'s sense: **the boards are on the desk and it needs hands on them.** That is a
person gate, and renting a machine with a power API and a console is what removes the person. The
milestone prescribes no duration, so this prices **an eight-hour soak plus two hours of setup and
reading the first heartbeat**, ten hours, which is a bench evening.

| Leg | Machine | Rate | 10 hours | Note |
|---|---|---|---|---|
| x86_64 | Scaleway EM-A116X-SSD, 4 cores | €0.077/hr | **€0.77** | qualifies today; four cores is enough for a crossing soak |
| x86_64, second provider | OVHcloud Rise RISE-1 | $64/mo, no hourly | **$64** | monthly only, so one month is the minimum unit |
| aarch64 | AWS `c7g.metal`, 64 vCPU | $2.32/hr | **$23.20** | requirement 1 unverified; the image work is the real cost |
| aarch64, free attempt | Oracle Always Free Ampere A1, 2 OCPU | free | **€0** | requirement 1 ambiguous; costs an evening, not money |

**So the x86_64 leg of milestone 225 costs under one euro of machine time**, and the aarch64 leg
costs about twenty-three dollars of machine time plus a UEFI-bootable image that does not exist yet.
**The money is not the constraint on either leg.** That is the useful finding: this milestone was
priced as needing hardware and it actually needs a morning of image work and a euro.

**One caveat that matters more than the prices.** A rented x86_64 machine is not xenon. §203 records
that xenon is a Core i5-7500T and that the tree holds per-machine measurements keyed to it, including
the fastpath footprint against its 32 KB L1i, so **a rented machine of another microarchitecture is
not like-for-like for those records.** The soak does not need like-for-like, so this substitution is
sound for milestone 225 and unsound for anything comparing against xenon's numbers. Likewise the
aarch64 leg: argon is a Jetson TX1 chosen by milestone 127 (the seL4 machine) precisely so identical
silicon referees the seL4 comparison, and **no rented Graviton substitutes for that.** A rented
aarch64 soak answers a question about aarch64 multicore; it does not answer argon's.

---

## The recommendation in one table

| Architecture | Buy | Price | Why |
|---|---|---|---|
| x86_64 | Scaleway Elastic Metal EM-A116X-SSD | €0.077/hr, €27.99/mo | the only cheap row that passes all four; hourly billing suits a bench evening |
| x86_64, second | OVHcloud Rise RISE-1 | $64/mo | a genuinely different boot delivery and console channel, which is the experiment, not redundancy |
| aarch64 | **none qualifies**; try Oracle Always Free first, then AWS `c7g.metal` | €0, then $2.32/hr | the free one costs an evening to falsify; the paid one needs an image that does not exist |
| riscv64 | Scaleway Elastic Metal EM-RV1 | €0.042/hr, €15.99/mo | the only qualifying RISC-V rental anywhere, and it is a second implementation rather than a spare radon |

**Which free and trial tiers genuinely qualify: none of them.** RISE's RISC-V runners are free, real,
and cannot boot a kernel. Oracle's Always Free x86 micro shapes fail on cores. Oracle's Always Free
Ampere A1 passes three of four and its fourth is ambiguous, which makes it an experiment worth
running and not coverage that can be claimed. calef asked to lean hard on free and trial options; the
honest report is that **the cheap paid options are cheaper than they look and the free ones do not
do this job.** A soak on Scaleway metal costs less per evening than a cup of coffee.

---

## BUGS

- **Hetzner's own product pages do not render prices to a fetcher.** The AX matrix, the AX42 page and
  the cloud page all returned specifications with the price fields empty. The €97.30 for AX42-1 comes
  from Hetzner's **price adjustment** document, which is a Hetzner page but is a change notice rather
  than a price list, and it is dated *15 June 2026*. The CAX cloud prices in the aarch64 table come
  from **third-party aggregators** and disagree with each other by nearly 2x; they are marked with a
  range for that reason and should not be quoted.
- **The `c7g.metal` hourly price is third-party.** $2.32/hr for us-east-1 is from an aggregator, not
  from AWS's own pricing page, which did not render figures. Treat it as indicative.
- **Requirement 1 on AWS is unverified and requirement 1 on Oracle Ampere A1 is ambiguous.** Neither
  vendor documents whether an arbitrary non-Linux kernel will boot, and no account was created to
  find out. Both are scored honestly rather than optimistically, and both could turn out either way.
- **OVH's API coverage for reboot and netboot was not confirmed.** The IPMI guide documents the
  console and does not cover API-level power control, and the API documentation index did not render.
  The Rise row scores requirement 3 yes on the basis of IPMI, which is out-of-band power control by
  definition, not on a documented REST endpoint.
- **The Scaleway serial console was confirmed for the RV1 explicitly and for Elastic Metal generally
  from the documentation index**, where `Use the serial console` appears as a how-to. Two candidate
  how-to URLs both 404'd, so the per-offer availability of the serial console on the x86 Elastic
  Metal ranges rests on the index and on the Labs page's account, not on a page read end to end.
- **Every price excludes VAT, egress, storage and setup.** Scaleway's figures are stated excluding
  VAT. The Elastic Metal and RV1 prices are for the machine; network transfer beyond whatever is
  included is not priced here, and the RV1's 100 Mbit/s link is a specification rather than a quota
  that was checked. OVH's figures are shown in USD on a page that also advertises them as special
  offers, so a renewal price may differ from a first-term price. AWS's figure excludes storage, the
  EBS volume the AMI needs, and egress.
- **Prices move, and this column moved recently.** Hetzner repriced its whole dedicated line on
  **15 June 2026**, and Equinix Metal's shutdown on **30 June 2026** removed a provider §203 had
  listed. Anything here is worth re-reading before money is spent, and the read date on every row is
  2026-09-23 for exactly that reason.
- **The firmware row of the fifth-column table is empty for all three x86_64 providers**, which is the
  single most load-bearing gap in this file. What a machine hands a kernel at entry is not published
  by any of them, so the risk-9-shaped argument for a second provider is a hypothesis this file could
  not test from a browser.
- **No provider was contacted and no account was created.** Everything here is from public pages.
