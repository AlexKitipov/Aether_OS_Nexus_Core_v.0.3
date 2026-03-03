// kernel/src/console.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use core::fmt::{self, Write};
use spin::Mutex;

// We will re-route console output to the serial driver for now.
// The Uart struct and its methods are no longer directly used for output here,
// but kept as a placeholder if a direct framebuffer/VGA console is added later.
struct Uart {
    __private: (),
}

impl Uart {
    const fn new() -> Self {
        Uart { __private: () }
    }

    // write_byte is conceptually here but actual output happens via serial driver
    fn write_byte(&mut self, _byte: u8) {
        // No-op, output is handled by serial driver
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Call into the serial driver's printing function
        crate::drivers::serial::_print(format_args!("{}", s));
        Ok(())
    }
}

// Global static for the UART console (still needed for fmt::Write impl, but mostly dummy)
static CONSOLE: Mutex<Uart> = Mutex::new(Uart::new());

// Public interface for the kernel console, which now just calls through to serial
pub fn print_str(s: &str) {
    crate::drivers::serial::_print(format_args!("{}", s));
}

pub fn print_u64(n: u64) {
    crate::drivers::serial::_print(format_args!("{}", n));
}

pub fn print_hex(n: u64) {
    crate::drivers::serial::_print(format_args!("{:x}", n));
}

// Macro for kernel printing, similar to `println!`
#[macro_export]
macro_rules! kprint! {
    ($($arg:tt)*) => ($crate::drivers::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! kprintln! {
    () => ($crate::kprint!("\n"));
    ($fmt:expr) => ($crate::kprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::kprint!(concat!($fmt, "\n"), $($arg)*));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // This function is now just a passthrough to the serial driver's _print
    crate::drivers::serial::_print(args);
}

// Dummy console init function (original from lib.rs, moved here for clarity of previous step)
// This `init` function is now part of the `Uart` impl, but it's a dummy.
impl Uart {
    pub fn init(&self) {
        // In a real kernel, this would initialize the UART hardware.
        // For now, it's a placeholder. Serial driver handles actual init.
        crate::drivers::serial::init();
        kprintln!("[kernel] console: Console system initialized (via serial driver).");
    }
}
