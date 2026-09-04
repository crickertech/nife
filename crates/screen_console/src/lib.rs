//! **Text on a screen, for a machine with no serial port** (milestone 243).
//!
//! Every word nife had ever said, it said down a UART: the boot tour on all three machines, the
//! console server, the kernel's fault reports, and every automated gate that reads any of them.
//! **A commodity machine does not have one.** The six machines in calef's house that milestone 87's
//! USB stick could already boot (two desktops, a laptop, an Intel MacBook, cordoba) have between
//! them no serial port at all, and until this crate existed a nife stick booted on one of them
//! would have printed the loader's four lines through the firmware's own console and then gone
//! permanently silent at `ExitBootServices`.
//!
//! This is the smallest thing that fixes that: given a linear framebuffer the previous boot stage
//! found ([`machine_discovery::framebuffer`]) and the tree's existing 7x8 font ([`bitmap_font`]),
//! put bytes on the screen.
//!
//! # Why this is not `video_terminal`
//!
//! [`video_terminal::Vt`] is the real terminal: a cell grid, a full escape-sequence parser, 300
//! rows of scrollback, damage rectangles. It is the right engine for the interactive terminal
//! (milestone 177 wires it) and it is deliberately **not** used here, for three reasons that all
//! point the same way.
//!
//! - **It is a value of several hundred kilobytes.** Its own documentation says so and warns the
//!   reader off putting one on a stack. In a kernel it would be a static of that size, in `.bss`,
//!   present on every architecture, for a diagnostic path.
//! - **It would put an escape-sequence parser in the TCB.** A state machine over untrusted-ish
//!   bytes is exactly the kind of thing this project keeps *out* of the kernel, and the kernel's
//!   own `println!` emits no escape sequences to parse.
//! - **The thing being reported is often the reason the machine is broken.** That is the block's
//!   own constraint on this milestone, and it argues for the console with the least state that
//!   could work. This one holds a cursor and a geometry: five `u32`s and no buffer.
//!
//! What is deliberately shared is the **font**, so the letters on an early boot screen and the
//! letters in the graphical terminal are the same letters.
//!
//! # What it does not do, and that is a design rather than a gap
//!
//! No colour changes, no escape sequences, no cursor, no scrollback, no reflow. A newline moves
//! down, a carriage return moves to column zero, everything else is a glyph, and running off the
//! bottom scrolls the picture up by one row of cells. **A console that cannot be put into a
//! surprising state is worth more here than a capable one**, because its whole job is to be
//! working at the moment something else is not.
//!
//! # Examples
//!
//! A whole console, painted into ordinary memory, which is exactly what the host tests below do and
//! what makes this crate provable without a screen:
//!
//! ```
//! use machine_discovery::framebuffer::{Framebuffer, PixelOrder};
//! use screen_console::ScreenConsole;
//!
//! // Two cells wide, one tall: 14x8 pixels at four bytes each.
//! let screen = Framebuffer { base: 0, width: 14, height: 8, stride: 56, order: PixelOrder::Bgrx };
//! let mut pixels = [0u8; 56 * 8];
//! let mut console = ScreenConsole::new(screen).expect("a screen big enough for one cell");
//!
//! assert_eq!(console.size(), (2, 1));
//! console.write(&mut pixels, "F");
//!
//! // The top-left pixel of an `F` is background and the one beside it is ink, which is the
//! // property a mirrored or transposed painter gets wrong. See `bitmap_font`'s own example.
//! let pixel = |x: usize, y: usize| u32::from_le_bytes(
//!     pixels[y * 56 + x * 4..y * 56 + x * 4 + 4].try_into().unwrap(),
//! );
//! assert_eq!(pixel(0, 0), ScreenConsole::BACKGROUND);
//! assert_eq!(pixel(1, 0), ScreenConsole::FOREGROUND);
//! ```
//!
//! # BUGS
//!
//! - **Scrolling reads the framebuffer back**, one screenful of bytes per scrolled row, and a
//!   framebuffer aperture is mapped uncacheable by every caller this crate has. That is cheap under
//!   QEMU and is the slowest thing here on real silicon; a write-combining mapping or a shadow copy
//!   in RAM would both fix it and both cost more than this milestone is buying. The boot tour is
//!   shorter than a 1280x800 screen is tall, so nothing scrolls during the boot this was built for.
//! - **Only 32-bit pixels.** [`machine_discovery::framebuffer::PixelOrder`] expresses the two byte
//!   orders UEFI reports and nothing else, so a 24-bit packed or 16-bit mode has no console. Every
//!   machine in the fleet reports one of the two.
//! - **Nothing here is proved on real silicon.** It is proved on the host and under OVMF. A
//!   framebuffer that works under QEMU's emulated adapter is not a framebuffer that works on
//!   Graeme's laptop, and `notes/serial-less-output.md` carries the bench procedure that would
//!   settle it.
//! - **The right-hand and bottom edges are never painted** when the screen is not a whole number of
//!   cells. 1280 is not a multiple of 7, so a 1280-pixel-wide screen owns 182 columns and leaves
//!   six pixels of whatever the firmware last drew. The same is true of `display_terminal`'s
//!   scanout wiring and for the same reason.

#![no_std]

use machine_discovery::framebuffer::Framebuffer;

/// A cursor on a screen, and the arithmetic that puts a byte under it.
///
/// It holds no picture. The framebuffer is the only storage, and it is passed in on every call
/// rather than held, so that this type is a plain value a caller can put in a `static` without a
/// pointer to device memory living inside it.
#[derive(Clone, Copy, Debug)]
pub struct ScreenConsole {
    screen: Framebuffer,
    cols: u32,
    rows: u32,
    col: u32,
    row: u32,
}

impl ScreenConsole {
    /// Ink. A light grey rather than white: white on black at a 1:1 pixel scale is glare on a real
    /// monitor, and every other console in this tree is a terminal's default rather than maximum
    /// contrast.
    pub const FOREGROUND: u32 = 0x00c8_c8c8;

    /// Paper. Black, because the alternative is repainting a firmware logo one cell at a time and
    /// getting a boot tour written over a splash screen.
    pub const BACKGROUND: u32 = 0x0000_0000;

    /// A console over `screen`, or `None` if it cannot hold a single character cell.
    ///
    /// The geometry is validated by [`Framebuffer::span`], which is the same check the loader
    /// applied before writing the description down. Doing it twice is deliberate: this crate is
    /// handed a description that came across a boot handoff, and the cost of believing a bad one is
    /// a kernel writing over memory it does not own.
    #[must_use]
    pub fn new(screen: Framebuffer) -> Option<Self> {
        screen.span()?;
        let cols = screen.width / bitmap_font::GLYPH_W;
        let rows = screen.height / bitmap_font::GLYPH_H;
        if cols == 0 || rows == 0 {
            return None;
        }
        Some(Self {
            screen,
            cols,
            rows,
            col: 0,
            row: 0,
        })
    }

    /// The grid, in character cells: `(columns, rows)`.
    #[must_use]
    pub const fn size(&self) -> (u32, u32) {
        (self.cols, self.rows)
    }

    /// How many bytes of framebuffer this console addresses, which is what a caller has to map and
    /// how long the slice it passes to [`Self::write`] must be.
    #[must_use]
    pub fn span(&self) -> usize {
        self.screen.span().unwrap_or(0)
    }

    /// Paint the whole surface in [`Self::BACKGROUND`] and put the cursor at the top left.
    ///
    /// Called once when a console is armed, because whatever the firmware left on the screen is not
    /// this kernel's and text drawn over a logo is text nobody can read.
    pub fn clear(&mut self, pixels: &mut [u8]) {
        let paper = self.screen.order.store(Self::BACKGROUND).to_le_bytes();
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&paper);
        }
        self.col = 0;
        self.row = 0;
    }

    /// Write `text` at the cursor, advancing it.
    ///
    /// `pixels` is the framebuffer, and a slice shorter than [`Self::span`] simply truncates the
    /// picture rather than panicking: this is the code that runs when something else has already
    /// gone wrong, and a bounds panic inside the console would take the message with it.
    pub fn write(&mut self, pixels: &mut [u8], text: &str) {
        for byte in text.bytes() {
            self.put(pixels, byte);
        }
    }

    /// One byte.
    fn put(&mut self, pixels: &mut [u8], byte: u8) {
        match byte {
            b'\n' => {
                self.col = 0;
                self.newline(pixels);
                return;
            }
            b'\r' => {
                self.col = 0;
                return;
            }
            _ => {}
        }
        if self.col >= self.cols {
            self.col = 0;
            self.newline(pixels);
        }
        // A byte with no glyph is drawn as a space rather than dropped, so that a run of them still
        // occupies the columns it occupies on the UART and the two transcripts line up.
        let glyph = if (0x20..0x7f).contains(&byte) {
            byte as char
        } else {
            ' '
        };
        self.draw(pixels, glyph);
        self.col += 1;
    }

    /// Move to the next row, scrolling when there is not one.
    fn newline(&mut self, pixels: &mut [u8]) {
        if self.row + 1 < self.rows {
            self.row += 1;
        } else {
            self.scroll(pixels);
        }
    }

    /// Move the picture up by one row of cells and blank the row that opens at the bottom.
    ///
    /// See this crate's `BUGS`: the read half of this is the expensive half on real silicon.
    fn scroll(&mut self, pixels: &mut [u8]) {
        let stride = self.screen.stride as usize;
        let band = stride * bitmap_font::GLYPH_H as usize;
        let live = stride * (self.rows * bitmap_font::GLYPH_H) as usize;
        if live > pixels.len() || band >= live {
            return;
        }
        pixels.copy_within(band..live, 0);
        let paper = self.screen.order.store(Self::BACKGROUND).to_le_bytes();
        for pixel in pixels[live - band..live].chunks_exact_mut(4) {
            pixel.copy_from_slice(&paper);
        }
    }

    /// Paint one glyph at the cursor.
    fn draw(&self, pixels: &mut [u8], glyph: char) {
        let stride = self.screen.stride as usize;
        let left = (self.col * bitmap_font::GLYPH_W) as usize * 4;
        let top = (self.row * bitmap_font::GLYPH_H) as usize;
        for y in 0..bitmap_font::GLYPH_H {
            let row = (top + y as usize) * stride + left;
            for x in 0..bitmap_font::GLYPH_W {
                let at = row + x as usize * 4;
                let Some(pixel) = pixels.get_mut(at..at + 4) else {
                    return;
                };
                let colour =
                    bitmap_font::cell_pixel(glyph, x, y, Self::FOREGROUND, Self::BACKGROUND);
                pixel.copy_from_slice(&self.screen.order.store(colour).to_le_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use machine_discovery::framebuffer::{Framebuffer, PixelOrder};

    use super::ScreenConsole;

    /// A screen with a **padded stride**, which is the geometry that catches the classic
    /// framebuffer bug: a painter that multiplies by the width instead of the stride paints a
    /// picture that shears progressively down the screen, and it looks perfectly correct on the
    /// first row.
    fn screen(cols: u32, rows: u32, pad: u32, order: PixelOrder) -> (Framebuffer, [u8; 4096 * 16]) {
        let width = cols * bitmap_font::GLYPH_W;
        let height = rows * bitmap_font::GLYPH_H;
        let found = Framebuffer {
            base: 0,
            width,
            height,
            stride: width * 4 + pad,
            order,
        };
        assert!(found.span().expect("a valid geometry") <= 4096 * 16);
        (found, [0u8; 4096 * 16])
    }

    /// Read one pixel back as a `u32`, the way a screendump would.
    fn pixel(found: &Framebuffer, pixels: &[u8], x: u32, y: u32) -> u32 {
        let at = (y * found.stride + x * 4) as usize;
        u32::from_le_bytes(pixels[at..at + 4].try_into().expect("four bytes"))
    }

    /// The whole crate is a pure function from text to pixels, so the expected picture is
    /// computable and can be *printed*. This is the same property `bitmap_font` claims for itself
    /// and it is the reason a console nobody can see is nonetheless provable.
    #[test]
    fn a_letter_is_drawn_the_right_way_up_and_the_right_way_round() {
        let (found, mut pixels) = screen(4, 2, 12, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("four by two cells");
        console.write(&mut pixels, "F");

        let art: [[char; 7]; 8] = core::array::from_fn(|y| {
            core::array::from_fn(|x| {
                if pixel(&found, &pixels, x as u32, y as u32) == ScreenConsole::FOREGROUND {
                    '#'
                } else {
                    '.'
                }
            })
        });
        assert_eq!(
            art,
            [
                ['.', '#', '#', '#', '#', '#', '.'], // the top bar
                ['.', '#', '.', '.', '.', '.', '.'], // the stem is on the LEFT, which is what a
                ['.', '#', '.', '.', '.', '.', '.'], // mirrored font gets wrong
                ['.', '#', '#', '#', '#', '.', '.'], // one column short: that is what makes it an F
                ['.', '#', '.', '.', '.', '.', '.'],
                ['.', '#', '.', '.', '.', '.', '.'],
                ['.', '#', '.', '.', '.', '.', '.'],
                ['.', '.', '.', '.', '.', '.', '.'], // row 7 is the descender row, and F has none
            ]
        );
    }

    /// The stride is padding, and a painter that ignores it writes the second row seven pixels to
    /// the left of where it belongs. The pad bytes must be untouched.
    #[test]
    fn the_padding_between_rows_is_never_written() {
        const PAD: u32 = 12;
        let (found, mut pixels) = screen(4, 2, PAD, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("four by two cells");
        console.clear(&mut pixels);
        // Poison the padding, then paint over the whole grid and check it survived.
        let visible = (found.width * 4) as usize;
        for y in 0..found.height as usize {
            let at = y * found.stride as usize + visible;
            pixels[at..at + PAD as usize].fill(0xa5);
        }
        console.write(&mut pixels, "MMMM\nMMMM");
        for y in 0..found.height as usize {
            let at = y * found.stride as usize + visible;
            assert_eq!(
                &pixels[at..at + PAD as usize],
                &[0xa5; PAD as usize],
                "row {y}'s padding was painted over"
            );
        }
    }

    /// Running past the last column wraps rather than painting outside the screen, and running past
    /// the last row scrolls. Both are checked by where a known glyph ends up.
    #[test]
    fn text_wraps_at_the_right_edge_and_scrolls_at_the_bottom() {
        let (found, mut pixels) = screen(2, 2, 0, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("two by two cells");
        assert_eq!(console.size(), (2, 2));
        console.clear(&mut pixels);

        // Five characters into a 2x2 grid: "ab" wraps to "cd", then "e" scrolls and lands at (0,1).
        console.write(&mut pixels, "abcde");
        let cell = |pixels: &[u8], col: u32, row: u32, ch: char| {
            (0..bitmap_font::GLYPH_H).all(|y| {
                (0..bitmap_font::GLYPH_W).all(|x| {
                    let want = bitmap_font::cell_pixel(
                        ch,
                        x,
                        y,
                        ScreenConsole::FOREGROUND,
                        ScreenConsole::BACKGROUND,
                    );
                    pixel(
                        &found,
                        pixels,
                        col * bitmap_font::GLYPH_W + x,
                        row * bitmap_font::GLYPH_H + y,
                    ) == want
                })
            })
        };
        assert!(cell(&pixels, 0, 0, 'c'), "the second line scrolled up");
        assert!(cell(&pixels, 1, 0, 'd'));
        assert!(cell(&pixels, 0, 1, 'e'), "the fifth character opened a row");
        assert!(cell(&pixels, 1, 1, ' '), "the rest of the new row is blank");
    }

    /// The byte order is the one thing that cannot be seen by looking at the screen in a test, and
    /// is the one thing that makes the picture wrong on half the machines that could run this.
    #[test]
    fn the_two_pixel_orders_store_different_bytes_for_the_same_ink() {
        let (bgrx, mut a) = screen(1, 1, 0, PixelOrder::Bgrx);
        let (rgbx, mut b) = screen(1, 1, 0, PixelOrder::Rgbx);
        ScreenConsole::new(bgrx)
            .expect("one cell")
            .write(&mut a, "#");
        ScreenConsole::new(rgbx)
            .expect("one cell")
            .write(&mut b, "#");
        // FOREGROUND is grey, so its red and blue bytes are equal and the two orders agree. That is
        // exactly why the check below uses a colour whose channels differ.
        assert_eq!(
            PixelOrder::Bgrx.store(0x0011_2233),
            0x0011_2233,
            "bgrx stores a colour as written"
        );
        assert_eq!(
            PixelOrder::Rgbx.store(0x0011_2233),
            0x0033_2211,
            "rgbx exchanges red and blue"
        );
        assert_eq!(a, b, "grey ink is the same bytes in both orders");
    }

    /// A screen too small for one character cell has no console, rather than a console that writes
    /// outside it.
    #[test]
    fn a_screen_smaller_than_one_cell_has_no_console() {
        for (width, height) in [(6, 8), (7, 7), (0, 0)] {
            let found = Framebuffer {
                base: 0,
                width,
                height,
                stride: width * 4,
                order: PixelOrder::Bgrx,
            };
            assert!(ScreenConsole::new(found).is_none(), "{width}x{height}");
        }
    }

    /// A framebuffer slice shorter than the geometry claims must truncate the picture, never panic.
    /// The console runs on the path where something else has already gone wrong.
    #[test]
    fn a_short_framebuffer_truncates_rather_than_panicking() {
        let (found, _) = screen(8, 8, 0, PixelOrder::Bgrx);
        let mut console = ScreenConsole::new(found).expect("eight by eight cells");
        let mut pixels = [0u8; 64];
        console.clear(&mut pixels);
        console.write(
            &mut pixels,
            "this is far more text than sixty-four bytes can hold\n",
        );
    }
}
