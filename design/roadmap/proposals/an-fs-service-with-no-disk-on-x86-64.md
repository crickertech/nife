# x86_64's FS service has a server and no disk it can find, so `std::fs` and `ripgrep` cannot run there

**Status: PROPOSED 2026-09-14.** Written by the milestone 184 lane (`std` on x86_64), which made
`std_exerciser` pass on that architecture and then found that the next two things it wanted to prove,
`std::fs` and unmodified `ripgrep`, wait on a disk rather than on `std`.

**Gate: NONE.** Milestone 215 already routes a PCI function's interrupt to a userspace driver on this
port, and milestone 164 already packs `redoxfs_server` in the x86_64 archive.

**What the work is.** `fs_service::wire_servers` asks `virtio::find_block_device_n(1)` for its RedoxFS
disk, and that lookup walks virtio-mmio slots only. `q35` has no virtio-mmio bus, and no x86_64 runner
attaches the `-redoxfs.img` fixture at all. So every test that needs a directory capability takes its
"no RedoxFS disk attached" arm on x86_64, `std_exerciser`'s `std::fs` half is compiled and never run,
and `ripgrep_tests` skips even when `rg` is in the archive. Three pieces: a transport-blind lookup for
the second block device, the runner attaching the fixture as a `virtio-blk-pci` function, and whatever
VT-d confinement then wants for a second function behind the IOMMU.

**What it would settle.** Whether unmodified `ripgrep`'s transcript on x86_64 matches the 62 bytes the
other two architectures print, which is what `design/fatal-risks.md` risk 1 is waiting on for its third
leg. The x86_64 `ripgrep` build already exists (4.1 MB, zero source changes).

**Why it was not done in 184.** It is FS-service and platform wiring, not `std`: the `std::fs` PAL has
nothing architecture-specific in it. Milestone 164 built and reverted a `virtio-blk-pci` disk route
before milestone 215 fixed the interrupt that sank it, so the shape is priced but not re-proven.

**Recorded in the meantime** where a reader meets the gap: `design/roadmap/184-std-x86-64.md`'s `BUGS`
and `notes/ripgrep-on-nife.md`'s parity table.
