# A machine with no serial port

Milestone 243. Every word nife had ever said, it said down a UART: the boot tour on all three
architectures, the console server and the shell, the kernel's fault reports, and **every automated
gate that reads any of them** (`script/board-console`, the soak's heartbeat, `script/shell-check`,
`crates/board_console`'s stage judging).

A commodity machine does not have one. xenon does, and that was chosen rather than lucky: milestone
87 picked a Dell with a C4PDJ serial module and a null modem. Meanwhile the same milestone made a
much larger fleet reachable without anyone noticing, because `\EFI\BOOT\BOOTX64.EFI` on a FAT32
stick is the removable-media fallback **every** UEFI firmware looks for with no configuration:

> if I can boot a nife system off of a USB drive, then that opens up a lot of different hardware in
> our house that I can use for testing: Graeme's laptop, his desktop, cordoba, two MacBooks, Clay's
> desktop
>
> -- calef, 2026-09-03

**Not one of those machines has a serial port.** This note is what was chosen, what it cost, what it
does not solve, and the procedure for the bench nobody in this lane could reach.

## The two halves, and they are not equally hard

The block separates them and the separation survived the work, so it is the first thing to say.

|  | a **human** watching a machine boot | a **gate** reading a machine |
|---|---|---|
| needs | pixels on a monitor, live | text, after the fact, judged by a program |
| has today | the framebuffer console below | a screendump under QEMU, and **nothing on real hardware** |
| fails when | the screen is not yet initialised | the machine is not a virtual one |

**One mechanism serves both under QEMU and only one of them on a real machine**, and pretending
otherwise would be the dishonest half of this note. Under OVMF a gate can ask the emulator for a
picture of the screen; on Graeme's laptop nobody can. That gap is real, it is named in `BUGS` below,
and the follow-on proposals are where it goes.

## What was chosen: the firmware's framebuffer, carried across the handoff

**UEFI's `EFI_GRAPHICS_OUTPUT_PROTOCOL` reports a *linear framebuffer*, and its address survives
`ExitBootServices`.** That is the fact the whole design rests on and it is worth being precise about
why: the aperture is a BAR on the display adapter, not firmware memory, so ending the boot phase
takes away the firmware's *console* and not the *display*. The pixels stay where they were and
anyone holding the address can keep writing to them.

So:

```text
  firmware ──GOP──► uefi_loader ──"fb=0x80000000,1280,800,5120,bgrx"──► kernel ──glyphs──► monitor
                         │                  in hvm_start_info's                 │
                         │                  cmdline_paddr                       │
                    machine_discovery::framebuffer  (one crate, both parties)   screen_console
```

Four pieces, and the split is the tree's usual one: what two programs agree on is a crate, the logic
is on the host, and only the address arithmetic is in the kernel.

| Piece | Where | What it is |
|---|---|---|
| the sentence | `crates/machine_discovery/src/framebuffer.rs` | `Framebuffer` and its one command-line token, **written** by the loader and **parsed** by the kernel, round-tripped in host tests |
| the question | `uefi_loader::find_screen` | one `LocateProtocol` call and three field reads |
| the painter | `crates/screen_console` | a cursor over a `&mut [u8]`, with `bitmap_font`'s glyphs |
| the seam | `kernel::console`, `arch::x86_64::machine::attach_screen` | tee `print!` to the screen; record the aperture so `mmu::init` carries the mapping |
| the gate | `board_console::screen`, `cargo xtask uefi-boot` | a screendump, decoded back into text, judged by `board_console::progress` |

### Why it rides a command line and not a new structure field

The x86_64 handoff is PVH's `hvm_start_info`, and **PVH already carries a command line**:
`cmdline_paddr` at offset 24, which `machine_discovery::x86_64::BootInfo` has decoded since milestone
87 and which nothing had ever read. `uefi_loader`'s own `BUGS` called that a gap in as many words:
*"there is nowhere yet for a boot argument to come from or go to."*

The alternative was appending a field to `hvm_start_info`. That structure is **Xen's**, versioned by
Xen, and a field added below the last one is a fork of somebody else's layout that looks exactly like
the real thing to whoever reads it next. A `key=value` in the field the format already provides for
exactly this is smaller, more reversible, and what Linux does with `video=`.

**Is it a wire format, and therefore calef's?** The tenets say anything two programs agree on is
expensive. This one is unusually cheap and the reason is structural rather than an argument for
leniency: `uefi_loader` **embeds the kernel inside itself** (`uefi_loader/build.rs`), so the writer
and the reader ship as one file and are rebuilt together. There is no version of this system in which
one side has the new spelling and the other does not. The token is recorded here and in the crate,
and renaming it costs one commit.

### Why the console is not `video_terminal`

`video_terminal::Vt` is the real terminal and it is the right engine for milestone 177's interactive
one. It is deliberately not this, for three reasons that all point the same way:

- **It is a value of several hundred kilobytes** (its own documentation warns readers off putting one
  on a stack), which in a kernel means a `.bss` static of that size on every architecture for a
  diagnostic path.
- **It would put an escape-sequence parser in the TCB**, over bytes, when the kernel's own `println!`
  emits no escape sequences to parse.
- **The thing being reported is often the reason the machine is broken.** That is the block's own
  constraint, and it argues for the console with the least state that could work. `ScreenConsole`
  holds a cursor and a geometry: five `u32`s and no buffer.

What *is* shared is the **font**, so the letters on an early boot screen and the letters in the
graphical terminal are the same letters, which is also what makes the gate below possible.

## What the alternatives cost, priced rather than argued

The block listed candidates and endorsed none. Each was priced against **both** halves.

| Candidate | The human | The gate | Verdict |
|---|---|---|---|
| **Firmware framebuffer** (chosen) | yes, from the first tour line | under QEMU only | the only one that answers the human half at all, and the only one whose address survives `ExitBootServices` with no driver of ours |
| **Keep `ConOut` alive longer** | until `ExitBootServices` and no further | no | not a candidate, it is a fact: boot services *are* the console, and a kernel that never exits them never gets its memory map. The loader already prints there and this milestone kept that |
| **Postmortem to the boot medium** | no (it is postmortem) | **yes, and on real hardware** | the gate half's real answer, and it needs a **USB mass-storage driver**, which nife does not have. Milestone 242 is USB host. Deferred as a proposal, not refused |
| **A network console** | no | yes, and unattended | needs a NIC driver per board and says nothing until the stack is up, which excludes every failure before it. The block's own objection stands: a machine that can only report over a network cannot report a network failure |
| **A photograph of the screen** | it *is* the human | no | not pixel-aligned and not a screendump; `board_console::screen` cannot read one, and saying so is cheaper than discovering it |

**The honest summary is that the two halves want different mechanisms**, which the block anticipated
and this lane confirms. The framebuffer is the human's answer and is the right first increment
because it is the one a person needs standing in front of a machine that will not boot. The gate's
answer on real hardware is postmortem to storage, and it is blocked on a driver.

## The gate: a program reading a screen

`board_console::screen` turns a screendump back into text. It is not optical character recognition
and the difference is the whole reason it is trustworthy: `bitmap_font` is a **constant table of
monochrome 7x8 glyphs**, so a cell either matches a glyph bit for bit or matches nothing. There is no
threshold to tune and no confidence to report.

That gives `cargo xtask uefi-boot` an assertion nothing else in this tree could make. The serial
transcript it already checked would read identically if the screen were black; the screen check fails
if the loader's `LocateProtocol` regressed, if the pixel order flipped, if the stride were taken as
the width, or if `mmu::init` stopped carrying the aperture's mapping.

**Measured, 2026-09-04, OVMF on QEMU 11.x, `-display none`:**

```console
$ cargo xtask uefi-boot
nife uefi_loader: milestone 87
uefi_loader: screen at 0x0000000080000000..0x00000000803e8000
uefi_loader:   1280x800, stride 5120, bgrx
uefi_loader: kernel placed, exiting boot services

nife on x86_64 (long mode, ring 0, 4-level paging)
  screen      : 1280x800 bgrx at 0x80000000, 182x100 cells (boot cmdline)
  ...
nife x86_64: boot complete, halting.
uefi-boot: read 96 non-blank row(s) of text back off the framebuffer, ending
uefi-boot:   |   next        : real ELF user programs (user_rt has no x86_64 arms), ...
uefi-boot:   | nife x86_64: boot complete, halting.
uefi-boot:   |   capability slots: 2 of 24 at peak
uefi-boot: booted under OVMF from \EFI\BOOT\BOOTX64.EFI
```

**`-display none` suppresses the host window, not the emulated adapter**, which is why any of this
works headlessly: OVMF finds a GOP here for the same reason a real machine's firmware finds one.

## The bench: booting a serial-less machine

**This has not been done.** Everything above is QEMU with real firmware in the loop, which is as far
as a lane can get; the machines are calef's. This section is the procedure, written to be followed
rather than interpreted, and it is deliberately the same shape as `notes/x86-uefi-boot.md`'s.

### What you need

- Any x86_64 machine with UEFI firmware and a monitor. From the fleet calef named: Graeme's laptop,
  Graeme's desktop, cordoba, Clay's desktop, or an Intel MacBook. **The Apple Silicon MacBooks are
  not in this fleet**: they boot non-Apple kernels legitimately (Asahi's permissive-security mode)
  but they have no UEFI, so a `BOOTX64.EFI` stick will not start one. That is a port, not a boot.
- A USB stick, **formatted FAT32** with a GPT or MBR partition table. macOS Disk Utility: *Erase*,
  format **MS-DOS (FAT)**, scheme **GUID Partition Map**.
- **No serial cable, no adapter, nothing else.** That is the milestone.

### Build and copy

```console
$ cd /path/to/nife
$ cargo xtask uefi-image
wrote .../target/esp/EFI/BOOT/BOOTX64.EFI (the loader, the kernel and the archive)

$ mkdir -p /Volumes/NIFE/EFI/BOOT
$ cp target/esp/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
$ diskutil eject /Volumes/NIFE
```

One file. The path and the capitalisation are the interface.

### Firmware settings

1. **Secure Boot: off.** This image is unsigned and nothing in this tree signs it. Expect to have to
   do this; a Secure Boot machine refuses the stick with a security-violation message and no other
   explanation.
2. **Boot from UEFI, not Legacy/CSM.** Legacy boot looks for an MBR boot sector, which this stick
   does not have.
3. **Leave everything else alone on the first attempt.** A bring-up has enough variables.

The one-time boot menu is usually F12; on a Mac, hold Option at the chime.

### What you should see, in order

Everything is on the monitor. Nothing else is connected.

1. `nife uefi_loader: milestone 87`, then three or four more `uefi_loader:` lines. **This is the
   firmware's own console**, so seeing it proves the firmware found the stick, Secure Boot did not
   refuse it, and the loader started. It also prints the screen it found.
2. **The screen clears**, which is the kernel's console arming.
3. The boot tour, beginning `nife on x86_64 (long mode, ring 0, 4-level paging)`, with a
   `screen      :` line naming the geometry, and ending `nife x86_64: boot complete, halting.`

**Photograph the screen at that point.** That is the record, and it is the only record this machine
can produce today (see `BUGS`).

### Triage

Each row rules out everything above it.

| What you see | What it means | What to do |
|---|---|---|
| Nothing at all, machine boots its own OS | the firmware did not see the stick | check `/EFI/BOOT/BOOTX64.EFI`, the case, and FAT32 rather than exFAT |
| "Security violation", or the stick is skipped | Secure Boot | turn it off |
| `nife uefi_loader:` then a message and a halt | the loader refused, and it says why | every string is a literal in `uefi_loader/src/main.rs` |
| `uefi_loader: no linear framebuffer` | the adapter is `PixelBltOnly` or `PixelBitMask` | this machine cannot use this milestone; record the model, it is the first one |
| `wanted 0x...` and `in the way:` lines | the firmware will not give up the kernel's 32 MiB load range | milestone 195's `BUGS`: the image is not physically relocatable. Record the descriptors printed |
| The loader's lines, screen clears, then **nothing** | the kernel died between `ExitBootServices` and its first `println!` | the hardest case, and the one this milestone does not fix. See below |
| Text, but sheared or in the wrong colours | the stride or the pixel order | the `screen :` line says what the loader read; compare against the machine's real mode |
| The machine reboots in a loop | a triple fault | build with no archive (`NIFE_UEFI_INITRD` unset) to halve what is copied |

**The screen clearing and then staying black is the honest remaining hole**, and it is deliberate
rather than an oversight: `console::attach_screen` clears, because a boot tour written over a vendor
logo is a boot tour nobody can read. The cost is that the loader's lines are gone by the time the
kernel's first line would appear, so that window is silent. It is also a *reading*: a cleared screen
with no text means the kernel got as far as arming its console and no further, which is more than the
same machine could say yesterday.

## BUGS

- **Nothing here has run on real silicon.** It is proved on the host and under OVMF. A framebuffer
  that works under QEMU's emulated adapter is not a framebuffer that works on Graeme's laptop: the
  aperture may be above 4 GiB (this loader refuses that, see below), the mode may be `PixelBitMask`,
  and the firmware may hand over a mode the monitor is not actually showing.
- **A gate cannot read a real machine.** The screendump path is QEMU's. On the fleet, the record is a
  photograph and a person, which is exactly the state milestone 216 got the VisionFive 2 *out* of.
  Postmortem to the boot medium is the answer and it needs a USB mass-storage driver; see the
  proposals.
- **Early boot is still silent, and that is problem 3 of the block's three.** The screen is armed by
  the first statement of the x86 boot tour, which is as early as a kernel can, but everything before
  `kernel_main` (`boot.s`, the long-mode entry, the page-table trampoline) writes nothing anywhere. A
  fault there produces a black screen on a machine with no serial port. Commodity operating systems
  answer this with a firmware console, a splash, or postmortem logging; nife still has none of the
  three.
- **The framebuffer must be below 4 GiB**, because everything the loader hands the kernel has to be
  nameable by a 32-bit instruction stream running with paging off. `uefi_loader` does not currently
  check that for the framebuffer specifically; the aperture is a BAR the firmware placed, and a
  machine that puts it high would be handing the kernel an address its early direct map does not
  cover. Nothing has been seen doing this and nothing would notice if one did.
- **The aperture is mapped uncacheable**, like every other device window, so writing a screenful is
  slow on real silicon in a way it is not under QEMU. Write-combining is a PAT entry and this kernel
  does not program the PAT at all; a framebuffer is the first thing in it that would care.
  Scrolling additionally *reads* the aperture back, which is the worse half. The boot tour is shorter
  than a 1280x800 screen is tall, so this does not bite during the boot it was built for.
- **Only ASCII appears.** A byte outside `0x20..0x7f` is drawn as a space, so `§121` in the tour's
  last line reads as two blanks and a number on the screen. The serial console shows it correctly and
  the two transcripts therefore differ by exactly the non-ASCII characters in them.
- **`board_console::screen` duplicates a decoder `xtask` already has.** Milestone 177's graphical
  `shell-check` leg carries `parse_ppm`, `decode_cell` and `scanout_rows` in `xtask/src/main.rs`,
  hardcoded to the compositor's geometry and the terminal's default colours. The two should be one
  crate; unifying them touches another milestone's gate and is a proposal rather than a drive-by.
- **The other two architectures have no screen at all.** This is x86_64/UEFI only. Milestone 157 is
  the U-Boot framebuffer handoff and the crate half of this (`machine_discovery::framebuffer`,
  `screen_console`) was written arch-neutral for it deliberately: what is missing on aarch64 and
  riscv64 is the discovery, not the console. Rule 5's scope note, recorded here.
