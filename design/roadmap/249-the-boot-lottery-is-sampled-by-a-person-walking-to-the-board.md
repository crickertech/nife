# 249. The boot lottery is sampled by a person walking to the board, so nine draws is a whole evening

**Status: IN-PROGRESS** on `milestone/249-self-rebooting-soak`. Minted 2026-09-03 by calef, who asked
during the boot series whether the soak could reboot itself. *(Number provisional until the merge queue
lands it.)*

**Gate: NONE.** The mechanism is already in the kernel and already proven on this board.

**In brief.** On 2026-09-03 nine boots of radon produced a **fifteenfold** throughput range, and
milestone 240's census explained it: rate tracks the number of cores free of grinders and carrying an
IPC group. Every one of those nine draws cost calef a walk to the board and the maintainer a capture.

**The distribution is the thing that is missing, and nine draws barely sketch it.** Counting by rate,
the nine landed at two clean cores six times, one clean core twice, and zero once. **Three and four
clean cores have never been drawn at all**, so the top of the curve is unmeasured and it is unknown
whether it is rare or structurally unreachable. A hundred draws would answer that; nine cannot.

## The mechanism already exists and is called with the wrong argument

`kernel/src/arch/riscv64/semihosting.rs` calls SBI's `system_reset` today, under the `board` feature,
to power the board off at the end of a test run:

```rust
const SRST_RESET_TYPE_SHUTDOWN: usize = 0;
```

SRST defines reset type **0 as shutdown and 1 as cold reboot**. So a self-rebooting soak is one
constant and a timer, over an `ecall` this project already knows radon's OpenSBI honours, because the
shutdown path is in use. Milestone 218's `boot.scr` then drives the next boot with nothing typed at
it, which was proven on this board for the first time the same evening.

## The hazard, which is the part to design first

**A board that reboots itself on a timer is a board nobody can get back.** Every boot runs the same
image and reboots again, so short of pulling power and rewriting the card there is no way to reach a
normal boot. That is worse than the problem being solved, and it is the reason this is a milestone
rather than a one-line change somebody makes in an afternoon.

**The escape suggested by the situation: before rebooting, check whether a byte has arrived on the
UART.** A console is attached for the whole series anyway, the kernel already reads that device, and
it makes stopping the loop as cheap as pressing a key. Other shapes exist (a bounded count, a
build-time cap) and the choice is this milestone's, but **something must make an unattended loop
stoppable without physical access.**

## What it must not become

**This is not milestone 248** (a placement can only be drawn, never constructed, so the strongest
finding on this machine rests on two boots), and the two should not be folded together. 248 builds an
arrangement on purpose; this draws many more of them at random. They have different risks, and this
one can ship in an afternoon while 248 needs a design. Keeping them apart is what stops the cheap one
waiting on the expensive one.

## The proof that this milestone worked

**A distribution over at least fifty unattended boots**, recorded in notes/soak.md beside the nine
hand-drawn ones: how often each clean-core count comes up, and whether three or four ever does.

Not a kernel that reboots, which is the mechanism rather than the result.

## BUGS

- **It samples radon and nothing else.** The lottery is this board's, with four startable harts and
  this firmware; a distribution measured here says nothing about argon or xenon.
- **Cold reboot is not power cycling**, and the two may not draw from the same distribution. Anything
  that survives a warm reset (firmware state, cached tables) is held constant across every draw in a
  way a hand-cycled series did not hold it. The nine hand-drawn boots are the control, and the check
  is whether the automated distribution matches them where they overlap.
- **An unattended board can fail unattended.** A boot that hangs before the soak starts ends the
  series silently, and the run must be able to tell "fifty boots, all fine" from "three boots and then
  a wedge at 2am". `board_console` already judges silence; this needs that judgement per boot rather
  than per run.
- **Nothing here explains the lottery**, only measures it. Why placement lands where it does is
  DECISIONS 138's territory and is not answered by more samples.
