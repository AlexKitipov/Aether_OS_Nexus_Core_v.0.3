// kernel/src/console.rs

#![allow(dead_code)]

use core::fmt::{self, Write};
use spin::Mutex;

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

    fn flush_secondary(&self) {
        let text = self.as_str();
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

// This lock protects only the secondary framebuffer/VGA sinks. Serial output
// intentionally never takes it, so diagnostics remain available from panics,
// exception handlers, and recursive logging paths.
static SECONDARY_OUTPUT_LOCK: Mutex<()> = Mutex::new(());

pub fn print_str(s: &str) {
    print_fmt(format_args!("{}", s));
}

pub fn print_u64(n: u64) {
    print_fmt(format_args!("{}", n));
}

pub fn print_hex(n: u64) {
    print_fmt(format_args!("{:x}", n));
}

/// Emits a diagnostic directly to COM1 without allocations or console locks.
/// Use this from panic/fatal paths where another console invocation may be in
/// progress and secondary sinks are not safe to touch.
pub fn emergency_print(args: fmt::Arguments<'_>) {
    crate::drivers::serial::_print(args);
}

pub fn print_fmt(args: fmt::Arguments<'_>) {
    // Serial is the primary early-boot sink. Formatting directly into its
    // writer keeps it independent of the bounded secondary-sink buffer.
    emergency_print(args);

    let mut buffer = ConsoleBuffer::new();
    if buffer.write_fmt(args).is_ok() {
        // Never spin on a lock from an exception or a recursively invoked
        // logger. Missing one secondary update is preferable to a deadlock.
        if let Some(_guard) = SECONDARY_OUTPUT_LOCK.try_lock() {
            buffer.flush_secondary();
        }
    }
}
