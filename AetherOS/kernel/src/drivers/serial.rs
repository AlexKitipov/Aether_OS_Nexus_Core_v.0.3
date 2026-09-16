// kernel/src/drivers/serial.rs

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt::{self, Write};
use x86_64::instructions::port::Port;

use crate::device::{
    Capability, CapabilitySet, Device, DeviceId, DeviceKind, IoDevice, IoResult, Rights,
    DEVICE_SERIAL,
};

/// The first PC-compatible serial port. QEMU exposes this as `-serial` COM1.
const COM1_BASE: u16 = 0x3f8;
const DATA_PORT: u16 = COM1_BASE;
const INTERRUPT_ENABLE_PORT: u16 = COM1_BASE + 1;
const FIFO_CONTROL_PORT: u16 = COM1_BASE + 2;
const LINE_CONTROL_PORT: u16 = COM1_BASE + 3;
const MODEM_CONTROL_PORT: u16 = COM1_BASE + 4;
const LINE_STATUS_PORT: u16 = COM1_BASE + 5;

const LINE_CONTROL_DLAB: u8 = 0x80;
const LINE_CONTROL_8N1: u8 = 0x03;
const FIFO_ENABLE_CLEAR_14_BYTE: u8 = 0xc7;
const MODEM_CONTROL_DTR_RTS_OUT2: u8 = 0x0b;
const LINE_STATUS_TRANSMIT_EMPTY: u8 = 0x20;

/// Limit each byte's wait so a missing or wedged UART cannot stop boot
/// diagnostics forever. This is deliberately polling-only: serial interrupts
/// remain disabled throughout early boot.
const TRANSMIT_POLL_LIMIT: usize = 100_000;

/// Initializes COM1 as a polling 16550-compatible UART at 38,400 baud, 8N1.
pub fn init() {
    unsafe {
        // SAFETY: COM1 is the conventional UART I/O range on PC-compatible
        // hardware and QEMU. Boot runs on one CPU with interrupts disabled.
        let mut interrupt_enable = Port::<u8>::new(INTERRUPT_ENABLE_PORT);
        let mut line_control = Port::<u8>::new(LINE_CONTROL_PORT);
        let mut data = Port::<u8>::new(DATA_PORT);
        let mut fifo_control = Port::<u8>::new(FIFO_CONTROL_PORT);
        let mut modem_control = Port::<u8>::new(MODEM_CONTROL_PORT);

        interrupt_enable.write(0x00);
        line_control.write(LINE_CONTROL_DLAB);
        data.write(0x03); // divisor low byte: 115200 / 3 = 38400 baud
        interrupt_enable.write(0x00); // divisor high byte
        line_control.write(LINE_CONTROL_8N1);
        fifo_control.write(FIFO_ENABLE_CLEAR_14_BYTE);
        // OUT2 is required for IRQ routing on real 16550s; keeping it set also
        // matches QEMU's conventional COM1 configuration while IRQs stay off.
        modem_control.write(MODEM_CONTROL_DTR_RTS_OUT2);
    }
}

#[inline]
fn transmit_ready() -> bool {
    unsafe {
        // SAFETY: See `init`; this is a read-only access to COM1's line status.
        let mut line_status = Port::<u8>::new(LINE_STATUS_PORT);
        (line_status.read() & LINE_STATUS_TRANSMIT_EMPTY) != 0
    }
}

#[inline]
fn write_byte(byte: u8) {
    for _ in 0..TRANSMIT_POLL_LIMIT {
        if transmit_ready() {
            unsafe {
                // SAFETY: The transmitter-empty bit was observed above.
                Port::<u8>::new(DATA_PORT).write(byte);
            }
            return;
        }
        core::hint::spin_loop();
    }
}

struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            // Make QEMU's serial log readable regardless of whether its peer
            // expects CRLF or LF line endings.
            if byte == b'\n' {
                write_byte(b'\r');
            }
            write_byte(byte);
        }
        Ok(())
    }
}

/// Prints formatted arguments through the allocation-free, polling UART path.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    let _ = SerialWriter.write_fmt(args);
}

pub struct SerialDevice;

impl SerialDevice {
    pub const fn new() -> Self {
        Self
    }
}

impl Device for SerialDevice {
    fn id(&self) -> DeviceId {
        DEVICE_SERIAL
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Serial
    }

    fn capabilities(&self) -> CapabilitySet {
        Vec::from([Capability {
            device: DEVICE_SERIAL,
            rights: Rights::READ.union(Rights::WRITE),
        }])
    }
}

impl IoDevice for SerialDevice {
    fn read(&self, _buf: &mut [u8]) -> IoResult<usize> {
        Ok(0)
    }

    fn write(&self, buf: &[u8]) -> IoResult<usize> {
        for &byte in buf {
            write_byte(byte);
        }
        Ok(buf.len())
    }
}
