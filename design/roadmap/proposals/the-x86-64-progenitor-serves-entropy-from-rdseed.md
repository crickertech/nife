# The x86_64 progenitor serves entropy from RDSEED

**Status: PROPOSED 2026-09-26.** Raised by lane `milestone/595-std-layout` (milestone 595
(provisional), the shell runs a `std` program), which could not prove its `std` layout at the
x86_64 prompt for want of it.

**Gate: DECISION.** Who confirms `RDSEED` exists (the progenitor by `CPUID`, or the kernel in the
boot endowment) is a question of what two programs agree on, so it is an architect's; the rest is
a lane's.

## The gap

The progenitor builds its entropy service only from a virtio-rng the kernel found, and the kernel
finds one only on a virtio-mmio slot (`kernel::user::boot_virtio_rng_device`). q35 has no mmio bus,
so at the x86_64 prompt nothing can draw random bytes. Two things are omitted from
`script/swish-check`'s x86_64 leg because of it, each with its reason in `swish_check_omits`:
the four `uuid` lines, since milestone 182 (x86_64's own interactive-boot entry point), and
`std_exerciser`, since milestone 595, whose transcript asserts two draws from
`std::random::SystemRng`.

## What already exists

- `components/src/entropy.rs` serves `MODE_INSTRUCTION`: `RDSEED` on x86_64, with no device, no DMA
  page and no interrupt. Milestone 162 (real hardware entropy on x86_64 and aarch64) built it.
- The kernel test harness already spawns it that way on x86_64
  (`kernel/src/user/std_service.rs`, `STD_ENTROPY_BUS`), and every x86_64 `std` test draws from it.
- The x86_64 runner passes `-cpu max`, which implements `RDSEED`.

## What it would take, and what it would change

In `crates/system_initializer`'s `boot`, where the virtio-rng probe answers "no device", build the
entropy service in `MODE_INSTRUCTION` on x86_64. Two things need deciding rather than typing:

- **Who confirms the instruction exists.** `entropy.rs` trusts its spawner to have checked the
  feature bit. `CPUID` is unprivileged on x86_64, so the progenitor could check it itself; the
  kernel could also say so in the boot endowment. The second is one more thing two programs agree
  on.
- **The login stack comes with it.** `have_login_stack` gates on a working entropy service, so
  x86_64 would start building `credentialer` and `login` at boot for the first time. That is more
  capability slots at peak (x86_64 measured 17 of 24 against 22 on the other two in milestone 182)
  and a boot path x86_64 has never run.

## What it unblocks

The five omitted `swish-check` lines, and with them §19 (architectural parity) for milestone 595's
`std` layout at the prompt and for `uuid`, which milestone 111 (a shell that can endow a child with
entropy) put there.
