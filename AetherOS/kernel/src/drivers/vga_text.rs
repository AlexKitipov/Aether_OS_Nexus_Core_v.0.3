use core::fmt::{self, Write};
use core::ptr::write_volatile;
use spin::Mutex;

const VGA_BUFFER_ADDRESS: usize = 0xb8000;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const DEFAULT_COLOR: u8 = 0x0f; // white on black

pub struct VgaTextWriter {
    column_position: usize,
    row_position: usize,
    color_code: u8,
}

impl VgaTextWriter {
    pub const fn new() -> Self {
        Self {
            column_position: 0,
            row_position: 0,
            color_code: DEFAULT_COLOR,
        }
    }

    pub fn clear_screen(&mut self) {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                self.write_cell(row, col, b' ', self.color_code);
            }
        }
        self.column_position = 0;
        self.row_position = 0;
    }

    pub fn write_text(&mut self, text: &str) {
        for byte in text.bytes() {
            self.write_byte(byte);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= VGA_WIDTH {
                    self.new_line();
                }

                self.write_cell(
                    self.row_position,
                    self.column_position,
                    byte,
                    self.color_code,
                );
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        self.column_position = 0;
        if self.row_position + 1 >= VGA_HEIGHT {
            self.row_position = 0;
            self.clear_screen();
        } else {
            self.row_position += 1;
        }
    }

    fn write_cell(&self, row: usize, col: usize, ascii_char: u8, color_code: u8) {
        let index = row * VGA_WIDTH + col;
        let ptr = (VGA_BUFFER_ADDRESS as *mut u16).wrapping_add(index);
        let value = ((color_code as u16) << 8) | ascii_char as u16;
        unsafe {
            write_volatile(ptr, value);
        }
    }
}

impl Write for VgaTextWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_text(s);
        Ok(())
    }
}

static VGA_WRITER: Mutex<VgaTextWriter> = Mutex::new(VgaTextWriter::new());

pub fn init() {
    VGA_WRITER.lock().clear_screen();
}

pub fn write_str(text: &str) {
    VGA_WRITER.lock().write_text(text);
}

pub fn write_fmt(args: fmt::Arguments<'_>) {
    let _ = VGA_WRITER.lock().write_fmt(args);
}
