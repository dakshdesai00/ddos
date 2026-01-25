/*
 * console.rs - Text Console Implementation for DDOS
 *
 * This module provides a text-based console that renders characters to the framebuffer.
 * It manages cursor position, handles newlines, and supports colored text output.
 *
 * The console uses an 8x8 bitmap font and writes directly to the framebuffer,
 * converting ASCII characters into pixel-perfect representations on screen.
 */

use super::framebuffer::FrameBuffer;
use crate::utils::font::{FONT_BASIC, FONT_HEIGHT, FONT_WIDTH};
use core::fmt;

/*
 * Console Structure
 *
 * Fields:
 * - fb: The underlying framebuffer for pixel operations
 * - cursor_x: Current horizontal cursor position in pixels
 * - cursor_y: Current vertical cursor position in pixels
 * - current_color: ARGB color value for text rendering (format: 0xAARRGGBB)
 */
pub struct Console {
    fb: FrameBuffer,
    cursor_x: u32,
    cursor_y: u32,
    current_color: u32, // 1. Added Color Field
}

impl Console {
    /*
     * Creates a new Console instance
     *
     * Parameters:
     * - fb: FrameBuffer to draw characters onto
     *
     * Returns: Console initialized with cursor at (0,0) and white text
     *
     * How it works:
     * Initializes all console state with cursor at top-left origin and
     * default white color (0xFFFFFFFF in ARGB format)
     */
    pub fn new(fb: FrameBuffer) -> Console {
        Console {
            fb,
            cursor_x: 0,
            cursor_y: 0,
            current_color: 0xFFFFFFFF, // Default to White
        }
    }

    /*
     * Sets the current text color for subsequent character rendering
     *
     * Parameters:
     * - color: 32-bit ARGB color value (0xAARRGGBB)
     *          AA = Alpha (transparency), RR = Red, GG = Green, BB = Blue
     *
     * How it works:
     * Simply stores the color value to be used by draw_char() for all
     * foreground pixels when rendering characters
     */
    pub fn set_color(&mut self, color: u32) {
        self.current_color = color;
    }

    /*
     * Draws a single character at the current cursor position
     *
     * Parameters:
     * - c: Character to draw (ASCII printable chars and newline supported)
     *
     * How it works:
     * 1. Handle newline character specially by advancing to next line
     * 2. Check if character would overflow current line; if so, wrap to next line
     * 3. Convert character to font bitmap index (space=0x20 maps to index 0)
     * 4. Iterate through 8x8 bitmap: for each '1' bit, draw colored pixel;
     *    for each '0' bit, draw black background pixel
     * 5. Advance cursor by character width (8 pixels)
     *
     * The font bitmap uses 1 bit per pixel, with 1=foreground, 0=background
     */
    pub fn draw_char(&mut self, c: char) {
        // Handle newline character: move to start of next line
        if c == '\n' {
            self.newline();
            return;
        }

        // Word wrap: if character doesn't fit on current line, start new line
        if self.cursor_x + FONT_WIDTH as u32 >= self.fb.width {
            self.newline();
        }

        // Map ASCII character to font array index
        // Printable ASCII is 0x20 (space) to 0x7E (tilde)
        // Font array starts at index 0 for space, so subtract 0x20
        // Invalid characters map to index 0 (space)
        let idx = if c as usize >= 0x20 && c as usize <= 0x7E {
            c as usize - 0x20
        } else {
            0
        };
        let bitmap = FONT_BASIC[idx];

        // Render the 8x8 character bitmap pixel by pixel
        for (row, byte) in bitmap.iter().enumerate() {
            // Each byte represents one row of the character (8 pixels)
            for bit in 0..8 {
                // Test each bit from left (bit 7) to right (bit 0)
                // Shift right by (7-bit) positions and mask with 1 to isolate the bit
                if (byte >> (7 - bit)) & 1 == 1 {
                    // Bit is 1: draw foreground pixel with current color
                    self.fb.draw_pixel(
                        self.cursor_x + bit as u32,
                        self.cursor_y + row as u32,
                        self.current_color,
                    );
                } else {
                    // Bit is 0: draw background pixel (black)
                    self.fb.draw_pixel(
                        self.cursor_x + bit as u32,
                        self.cursor_y + row as u32,
                        0xFF000000, // Black background (fully opaque)
                    );
                }
            }
        }
        // Move cursor right by one character width for next character
        self.cursor_x += FONT_WIDTH as u32;
    }

    /*
     * Erases the character before the cursor (backspace functionality)
     *
     * How it works:
     * 1. Check if we're at the start of a line; if so, do nothing
     * 2. Move cursor back by one character width
     * 3. Fill the character space with black pixels (8x8 rectangle)
     *
     * Note: This doesn't handle backspacing across line boundaries
     */
    pub fn backspace(&mut self) {
        // Don't backspace if at the beginning of a line
        if self.cursor_x < FONT_WIDTH as u32 {
            return;
        }

        // Move cursor back one character
        self.cursor_x -= FONT_WIDTH as u32;

        // Erase the character by drawing black pixels over the entire 8x8 space
        for row in 0..FONT_HEIGHT {
            for bit in 0..FONT_WIDTH {
                self.fb.draw_pixel(
                    self.cursor_x + bit as u32,
                    self.cursor_y + row as u32,
                    0xFF000000, // Black
                );
            }
        }
    }

    /*
     * Advances the cursor to the beginning of the next line
     *
     * How it works:
     * 1. Reset horizontal cursor to left edge (x=0)
     * 2. Move vertical cursor down by one character height
     * 3. If we've gone past the bottom of screen, wrap to top
     *
     * Note: This implements simple wrap-around scrolling. When reaching
     * the bottom, text wraps to the top, overwriting old content.
     * A more sophisticated implementation might scroll the display.
     */
    fn newline(&mut self) {
        self.cursor_x = 0; // Return to left edge
        self.cursor_y += FONT_HEIGHT as u32; // Move down one line

        // Wrap to top if we've exceeded screen height
        if self.cursor_y >= self.fb.height {
            self.cursor_y = 0;
        }
    }
}

/*
 * Implement Rust's fmt::Write trait for Console
 *
 * This allows Console to be used with Rust's formatting macros like
 * write!(), writeln!(), and format_args!(). By implementing write_str(),
 * we enable statements like: writeln!(console, "Hello {}", name)
 */
impl fmt::Write for Console {
    /*
     * Writes a string to the console
     *
     * Parameters:
     * - s: String slice to write
     *
     * Returns: fmt::Result indicating success or failure
     *
     * How it works:
     * Iterates through each character in the string and calls draw_char()
     * for each one. This handles all formatting, including newlines embedded
     * in the string.
     */
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.draw_char(c);
        }
        Ok(())
    }
}
