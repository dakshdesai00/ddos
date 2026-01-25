use crate::font::{FONT_BASIC, FONT_HEIGHT, FONT_WIDTH};
use crate::framebuffer::FrameBuffer;
use core::fmt;

pub struct Console {
    fb: FrameBuffer,
    cursor_x: u32,
    cursor_y: u32,
    current_color: u32, // 1. Added Color Field
}

impl Console {
    pub fn new(fb: FrameBuffer) -> Console {
        Console {
            fb,
            cursor_x: 0,
            cursor_y: 0,
            current_color: 0xFFFFFFFF, // Default to White
        }
    }

    // 2. Function to change color (0xAABBGGRR)
    pub fn set_color(&mut self, color: u32) {
        self.current_color = color;
    }

    pub fn draw_char(&mut self, c: char) {
        if c == '\n' {
            self.newline();
            return;
        }
        if self.cursor_x + FONT_WIDTH as u32 >= self.fb.width {
            self.newline();
        }

        let idx = if c as usize >= 0x20 && c as usize <= 0x7E {
            c as usize - 0x20
        } else {
            0
        };
        let bitmap = FONT_BASIC[idx];

        for (row, byte) in bitmap.iter().enumerate() {
            for bit in 0..8 {
                if (byte >> (7 - bit)) & 1 == 1 {
                    // 3. Use current_color instead of hardcoded white
                    self.fb.draw_pixel(
                        self.cursor_x + bit as u32,
                        self.cursor_y + row as u32,
                        self.current_color,
                    );
                } else {
                    self.fb.draw_pixel(
                        self.cursor_x + bit as u32,
                        self.cursor_y + row as u32,
                        0xFF000000,
                    ); // Black background
                }
            }
        }
        self.cursor_x += FONT_WIDTH as u32;
    }

    pub fn backspace(&mut self) {
        if self.cursor_x < FONT_WIDTH as u32 {
            return;
        }
        self.cursor_x -= FONT_WIDTH as u32;
        for row in 0..FONT_HEIGHT {
            for bit in 0..FONT_WIDTH {
                self.fb.draw_pixel(
                    self.cursor_x + bit as u32,
                    self.cursor_y + row as u32,
                    0xFF000000,
                );
            }
        }
    }

    fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += FONT_HEIGHT as u32;
        if self.cursor_y >= self.fb.height {
            self.cursor_y = 0;
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.draw_char(c);
        }
        Ok(())
    }
}
