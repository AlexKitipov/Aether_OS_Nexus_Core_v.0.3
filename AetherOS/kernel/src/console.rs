// kernel/src/console.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use core::fmt::{self, Write};
use spin::Mutex;
use crate::kprintln;

const CONSOLE_BUFFER_SIZE: usize = 2048;

struct ConsoleBuffer {
    buf: [u8; CONSOLE_BUFFER_SIZE],
    len: usize,
}

impl ConsoleBuffer {
    const fn new() -> Self {
        Self {
            buf: [0; CONSOLE_BUFFER_SIZE],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }

    fn flush(&self) {
        let text = self.as_str();
        crate::drivers::serial::_print(format_args!("{}", text));
        crate::drivers::framebuffer::write_str(text);
        crate::drivers::vga_text::write_str(text);
    }
}

impl Write for ConsoleBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        if self.len + bytes.len() > self.buf.len() {
            return Err(fmt::Error);
        }
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

static OUTPUT_LOCK: Mutex<()> = Mutex::new(());

pub fn print_str(s: &str) {
    print_fmt(format_args!("{}", s));
}

pub fn print_u64(n: u64) {
    print_fmt(format_args!("{}", n));
}

pub fn print_hex(n: u64) {
    print_fmt(format_args!("{:x}", n));
}

pub fn print_fmt(args: fmt::Arguments) {
    let mut buffer = ConsoleBuffer::new();
    if buffer.write_fmt(args).is_ok() {
        let _guard = OUTPUT_LOCK.lock();
        buffer.flush();
    }
}

// Dummy console init function (original from lib.rs, moved here for clarity of previous step)
// This `init` function is now part of the `Uart` impl, but it's a dummy.
struct Uart {
    __private: (),
}

impl Uart {
    const fn new() -> Self {
        Uart { __private: () }
    }

    pub fn init(&self) {
        // In a real kernel, this would initialize the UART hardware.
        // For now, it's a placeholder. Serial driver handles actual init.
        crate::drivers::serial::init();
        kprintln!("[kernel] console: Console system initialized (via serial driver).");
    }
}
