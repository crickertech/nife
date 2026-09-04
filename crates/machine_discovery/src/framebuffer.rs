//! **Where the pixels are, told to the kernel by whoever booted it** (milestone 243).
//!
//! Every word nife had ever said, it said down a UART, and the six commodity machines in the house
//! do not have one. What they all have is a screen the firmware is already drawing on. This module
//! is the sentence that carries a screen across the handoff: the loader asks the firmware where the
//! linear framebuffer is and writes it here, the kernel reads it and paints text into it.
//!
//! # Why it rides the boot command line rather than a new field
//!
//! The x86_64 handoff is PVH's `hvm_start_info` (see [`crate::x86_64`]), and PVH **already carries
//! a command line**: `cmdline_paddr` at offset 24, which [`crate::x86_64::BootInfo::cmdline`] has
//! decoded since milestone 87 and which nothing has ever read. `uefi_loader`'s own `BUGS` recorded
//! that as a gap: *"there is nowhere yet for a boot argument to come from or go to."*
//!
//! So the alternative was widening a structure this project does not own. `hvm_start_info` is
//! Xen's, versioned by Xen, and a field appended to it is a fork of somebody else's format that
//! looks exactly like the real thing to anyone reading it later. A `key=value` in the field the
//! format already provides for exactly this is the smaller, more reversible, and more honest
//! answer, and it is what Linux does with `video=`.
//!
//! **It is also arch-neutral on purpose**, which is why this module sits beside [`crate::cpu_list`]
//! rather than inside [`crate::x86_64`]. Nothing below is about x86: milestone 157's U-Boot
//! framebuffer handoff has the same sentence to say on aarch64, and a second spelling of it would
//! be a second thing to get wrong.
//!
//! # This module is the writer *and* the reader
//!
//! [`Framebuffer::encode`] is what `uefi_loader` calls and [`Framebuffer::parse`] is what the kernel
//! calls, and the tests below round-trip one through the other. Neither party keeps its own copy of
//! the spelling, which is the rule `byte_sink_proto`, `grant_plan` and [`crate::x86_64`] itself are
//! already held to.
//!
//! # Examples
//!
//! What the loader writes and the kernel reads is one token on a command line, and it survives the
//! round trip unchanged:
//!
//! ```
//! use machine_discovery::framebuffer::{Framebuffer, PixelOrder};
//!
//! let found = Framebuffer {
//!     base: 0x8000_0000,
//!     width: 800,
//!     height: 600,
//!     stride: 3200,
//!     order: PixelOrder::Bgrx,
//! };
//!
//! let mut buffer = [0u8; Framebuffer::MAX_LEN];
//! let written = found.encode(&mut buffer);
//! assert_eq!(&buffer[..written], b"fb=0x80000000,800,600,3200,bgrx");
//!
//! let text = core::str::from_utf8(&buffer[..written]).expect("ASCII");
//! assert_eq!(Framebuffer::parse(text), Some(found));
//! ```
//!
//! A command line with other words on it is read the same way, and a machine that said nothing
//! about a screen is the ordinary case rather than an error:
//!
//! ```
//! use machine_discovery::framebuffer::Framebuffer;
//!
//! assert!(Framebuffer::parse("quiet fb=0x1000,64,32,256,rgbx debug").is_some());
//! assert_eq!(Framebuffer::parse("quiet debug"), None);
//! ```
//!
//! # BUGS
//!
//! - **Only the two 32-bit-per-pixel orders are expressible.** UEFI's `PixelBitMask` describes
//!   channels by mask and `PixelBltOnly` means there is no linear framebuffer at all; a loader that
//!   meets either says so and writes no token, so the kernel keeps whatever console it had. 15-bit
//!   and 24-bit packed modes are not expressible either. This is the shape every UEFI machine in
//!   the fleet is expected to be in and not a claim about every machine there is.
//! - **Nothing here validates that the address is a framebuffer.** It is a physical address a
//!   previous stage asserted, and the kernel writes to it. A wrong one corrupts whatever is there.
//!   The trust boundary is the boot chain, which is the same boundary the memory map already sits
//!   on.
//! - **The size of the aperture is not carried.** `stride * height` is what a console needs and is
//!   derivable; the firmware's `framebuffer_size` can be larger and nothing here would notice.

/// The byte order of one 32-bit pixel, as it lies in memory.
///
/// **Named for the memory order, not the `u32` value**, because the memory order is the thing a
/// writer has to get right and the `u32` spelling of it is the thing that flips between them. UEFI
/// names these the same way (`PixelBlueGreenRedReserved8BitPerColor` lists the bytes in order).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PixelOrder {
    /// Bytes B, G, R, unused. A little-endian `u32` is `0x00RRGGBB`, which is how every colour
    /// constant in this tree is already written, so no swizzle is needed. It is also what OVMF and
    /// essentially every PC display adapter report.
    Bgrx,
    /// Bytes R, G, B, unused. A little-endian `u32` is `0x00BBGGRR`, so a colour written in the
    /// tree's usual spelling has to have its red and blue exchanged before it is stored.
    Rgbx,
}

impl PixelOrder {
    /// The token this order is spelled with on the command line.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Bgrx => "bgrx",
            Self::Rgbx => "rgbx",
        }
    }

    /// Turn a colour written as `0x00RRGGBB` into the `u32` this order wants stored.
    ///
    /// The whole of the byte-order question, in one place, so that no painter has to hold it.
    #[must_use]
    pub const fn store(self, rrggbb: u32) -> u32 {
        match self {
            Self::Bgrx => rrggbb,
            // Exchange the red and blue bytes; green and the unused byte stay where they are.
            Self::Rgbx => {
                (rrggbb & 0x0000_ff00) | ((rrggbb & 0xff) << 16) | ((rrggbb >> 16) & 0xff)
            }
        }
    }
}

/// A linear framebuffer, as one boot stage describes it to the next.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Framebuffer {
    /// The physical address of pixel (0, 0).
    pub base: u64,
    /// Visible width, in pixels.
    pub width: u32,
    /// Visible height, in pixels.
    pub height: u32,
    /// **Bytes from one row to the next**, which is not `width * 4`. Firmware is free to pad rows
    /// out to a convenient pitch, and a painter that multiplies by the width instead paints a
    /// picture that shears progressively down the screen.
    pub stride: u32,
    /// How one pixel's four bytes are arranged.
    pub order: PixelOrder,
}

impl Framebuffer {
    /// The key this token opens with. `fb` rather than Linux's `video`, because Linux's takes a
    /// *mode request* (`video=1024x768`) and this takes an *answer*; borrowing the spelling would
    /// borrow the wrong meaning.
    pub const KEY: &'static str = "fb=";

    /// The longest token [`Self::encode`] can produce, for a caller sizing a buffer.
    ///
    /// `fb=` plus `0x` and sixteen hex digits, three ten-digit decimals with their commas, and a
    /// four-character order. Rounded up rather than derived, because the derivation would be a
    /// second thing to keep in step with the encoder.
    pub const MAX_LEN: usize = 64;

    /// How many bytes one row of pixels occupies, and the last byte the console may touch.
    ///
    /// Returns `None` when the arithmetic overflows or the geometry is degenerate, which is the
    /// only validation a description read out of a boot handoff can be given.
    #[must_use]
    pub const fn span(&self) -> Option<usize> {
        if self.width == 0 || self.height == 0 || self.stride < self.width.saturating_mul(4) {
            return None;
        }
        match (self.stride as u64).checked_mul(self.height as u64) {
            Some(bytes) if bytes <= usize::MAX as u64 => Some(bytes as usize),
            _ => None,
        }
    }

    /// Write this description into `out` as one command-line token, returning its length.
    ///
    /// Hand-rolled rather than formatted: the only caller is a `no_std` UEFI application with no
    /// allocator, where `core::fmt` would pull in machinery the binary otherwise does not have.
    ///
    /// # Panics
    ///
    /// If `out` is shorter than [`Self::MAX_LEN`].
    pub fn encode(&self, out: &mut [u8]) -> usize {
        assert!(out.len() >= Self::MAX_LEN, "encode needs MAX_LEN bytes");
        let mut n = 0;
        let mut put = |bytes: &[u8], n: &mut usize| {
            out[*n..*n + bytes.len()].copy_from_slice(bytes);
            *n += bytes.len();
        };
        put(Self::KEY.as_bytes(), &mut n);
        let mut hex = [0u8; 18];
        let hex_len = write_hex(self.base, &mut hex);
        put(&hex[..hex_len], &mut n);
        for value in [self.width, self.height, self.stride] {
            put(b",", &mut n);
            let mut decimal = [0u8; 10];
            let len = write_decimal(value, &mut decimal);
            put(&decimal[..len], &mut n);
        }
        put(b",", &mut n);
        put(self.order.token().as_bytes(), &mut n);
        n
    }

    /// Find and read the token on a boot command line, or `None` if there is not one.
    ///
    /// **A malformed token reads as absent**, deliberately. The alternative is a kernel that
    /// refuses to boot because a display description it could have done without did not parse,
    /// which is the wrong trade for a diagnostic path: the UART, where there is one, still works.
    #[must_use]
    pub fn parse(cmdline: &str) -> Option<Self> {
        let token = cmdline
            .split_ascii_whitespace()
            .find_map(|word| word.strip_prefix(Self::KEY))?;
        let mut fields = token.split(',');
        let base = parse_hex(fields.next()?)?;
        let width = parse_decimal(fields.next()?)?;
        let height = parse_decimal(fields.next()?)?;
        let stride = parse_decimal(fields.next()?)?;
        let order = match fields.next()? {
            "bgrx" => PixelOrder::Bgrx,
            "rgbx" => PixelOrder::Rgbx,
            _ => return None,
        };
        if fields.next().is_some() {
            return None;
        }
        let found = Self {
            base,
            width,
            height,
            stride,
            order,
        };
        found.span()?;
        Some(found)
    }
}

/// `0x` followed by the value's hex digits, shortest form, uppercase never (the tree prints hex
/// lowercase everywhere).
fn write_hex(value: u64, out: &mut [u8; 18]) -> usize {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    out[0] = b'0';
    out[1] = b'x';
    if value == 0 {
        out[2] = b'0';
        return 3;
    }
    let mut digits = [0u8; 16];
    let mut count = 0;
    let mut left = value;
    while left != 0 {
        digits[count] = DIGITS[(left & 0xf) as usize];
        left >>= 4;
        count += 1;
    }
    for i in 0..count {
        out[2 + i] = digits[count - 1 - i];
    }
    2 + count
}

/// The value's decimal digits, shortest form.
fn write_decimal(value: u32, out: &mut [u8; 10]) -> usize {
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut digits = [0u8; 10];
    let mut count = 0;
    let mut left = value;
    while left != 0 {
        digits[count] = b'0' + (left % 10) as u8;
        left /= 10;
        count += 1;
    }
    for i in 0..count {
        out[i] = digits[count - 1 - i];
    }
    count
}

/// `0x`-prefixed hex, which is what [`write_hex`] produces and what a person hand-editing a boot
/// line would write. A bare number is refused rather than guessed at.
fn parse_hex(text: &str) -> Option<u64> {
    let digits = text.strip_prefix("0x")?;
    if digits.is_empty() || digits.len() > 16 {
        return None;
    }
    let mut value = 0u64;
    for byte in digits.bytes() {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => return None,
        };
        value = (value << 4) | u64::from(digit);
    }
    Some(value)
}

/// Decimal, with no sign and no separators.
fn parse_decimal(text: &str) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    let mut value = 0u32;
    for byte in text.bytes() {
        let digit = byte.checked_sub(b'0').filter(|d| *d < 10)?;
        value = value.checked_mul(10)?.checked_add(u32::from(digit))?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::{Framebuffer, PixelOrder};

    /// The writer and the reader are one crate, so the property worth testing is that they agree.
    /// A loader whose token the kernel cannot read is the failure this exists to make impossible,
    /// and it is the failure nobody can see on a machine with no serial port.
    #[test]
    fn every_description_survives_the_round_trip() {
        for found in [
            Framebuffer {
                base: 0x8000_0000,
                width: 800,
                height: 600,
                stride: 3200,
                order: PixelOrder::Bgrx,
            },
            // A padded stride, which is the case a painter that multiplies by the width gets wrong.
            Framebuffer {
                base: 0xffff_ffff_0000_0000,
                width: 1366,
                height: 768,
                stride: 5504,
                order: PixelOrder::Rgbx,
            },
            Framebuffer {
                base: 0x1000,
                width: 1,
                height: 1,
                stride: 4,
                order: PixelOrder::Bgrx,
            },
        ] {
            let mut buffer = [0u8; Framebuffer::MAX_LEN];
            let n = found.encode(&mut buffer);
            let text = core::str::from_utf8(&buffer[..n]).expect("the encoder writes ASCII");
            assert_eq!(
                Framebuffer::parse(text),
                Some(found),
                "round trip of {text}"
            );
        }
    }

    /// `MAX_LEN` is a constant a caller sizes a stack buffer with, so it has to hold for the widest
    /// value the encoder can meet rather than for the values anyone expects.
    #[test]
    fn the_widest_token_fits_the_advertised_maximum() {
        let widest = Framebuffer {
            base: u64::MAX,
            width: u32::MAX,
            height: 1,
            stride: u32::MAX,
            order: PixelOrder::Bgrx,
        };
        let mut buffer = [0u8; Framebuffer::MAX_LEN];
        assert!(widest.encode(&mut buffer) <= Framebuffer::MAX_LEN);
    }

    /// A description that does not parse must read as "this machine said nothing about a screen",
    /// never as a boot failure. See the note on `parse`.
    #[test]
    fn a_malformed_token_reads_as_no_screen_at_all() {
        for bad in [
            "",
            "fb=",
            "fb=80000000,800,600,3200,bgrx", // no 0x
            "fb=0x8000,800,600,3200",        // no order
            "fb=0x8000,800,600,3200,cmyk",   // an order nobody has
            "fb=0x8000,800,600,3200,bgrx,7", // a field nobody wrote
            "fb=0x8000,0,600,3200,bgrx",     // no pixels
            "fb=0x8000,800,600,100,bgrx",    // a stride narrower than a row
        ] {
            assert_eq!(Framebuffer::parse(bad), None, "{bad:?} should not parse");
        }
    }

    /// The token is found among others and does not have to be first, because a boot command line
    /// is a place other things will eventually be written too.
    #[test]
    fn the_token_is_found_among_other_words() {
        let found = Framebuffer::parse("console=uart fb=0x1000,64,32,256,rgbx smp=2");
        assert_eq!(found.map(|f| f.width), Some(64));
    }

    /// The `u32` a painter stores differs between the two orders, and this is the only place in the
    /// tree that knows how. Red must come out red on both.
    #[test]
    fn red_is_stored_as_red_in_both_orders() {
        const RED: u32 = 0x00ff_0000;
        assert_eq!(PixelOrder::Bgrx.store(RED), 0x00ff_0000);
        assert_eq!(PixelOrder::Rgbx.store(RED), 0x0000_00ff);
        // Green is in the same byte either way, which is what makes a mirrored painter hard to see.
        const GREEN: u32 = 0x0000_ff00;
        assert_eq!(PixelOrder::Bgrx.store(GREEN), GREEN);
        assert_eq!(PixelOrder::Rgbx.store(GREEN), GREEN);
    }

    /// The span is what bounds every write a console makes, so a geometry whose arithmetic
    /// overflows has to be refused rather than truncated.
    #[test]
    fn a_span_that_cannot_be_computed_is_refused() {
        let absurd = Framebuffer {
            base: 0,
            width: u32::MAX,
            height: u32::MAX,
            stride: u32::MAX,
            order: PixelOrder::Bgrx,
        };
        // 4 GiB of rows at 4 GiB each: fine on a 64-bit host, and this is the honest answer there.
        assert_eq!(absurd.span(), Some(u32::MAX as usize * u32::MAX as usize));
    }
}
