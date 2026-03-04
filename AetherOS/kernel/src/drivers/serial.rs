// kernel/src/drivers/serial.rs

#![allow(dead_code)]

use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;
use core::fmt::{self, Write};

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        // SAFETY: The `new` method for SerialPort takes an unchecked address.
        // We assume 0x3F8 is the correct and safe address for COM1.
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        // The `init` function of the SerialPort configures the port,
        // and potentially enables FIFO. It is safe to call.
        serial_port.init();
        Mutex::new(serial_port)
    };
}

/// Initializes the serial port for debugging output.
pub fn init() {
    // The lazy_static ensures SERIAL1 is initialized when first accessed.
    // Accessing it here guarantees initialization early in the boot process.
    let _ = SERIAL1.lock(); // Forces initialization
}

/// Prints the given formatted arguments to the serial port.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // We acquire a lock on the serial port to ensure exclusive access for printing.
    // If the lock is already held, this will block until it's released.
    // In a panic scenario, if the lock is poisoned, this might also panic,
    // but for debugging, that's often acceptable.
    // SAFETY: Writing to the serial port is generally safe, assuming the hardware is configured.
    // Errors during writing are simply ignored for a print function.
    let _ = SERIAL1.lock().write_fmt(args);
}


