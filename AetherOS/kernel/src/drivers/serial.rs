// kernel/src/drivers/serial.rs

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;

use crate::device::{Capability, CapabilitySet, Device, DeviceId, DeviceKind, IoDevice, IoResult, Rights, DEVICE_SERIAL};

/// Initializes the serial driver.
pub fn init() {
    // Stub implementation for early bring-up.
}

/// Prints the given formatted arguments to the serial port.
#[doc(hidden)]
pub fn _print(_args: fmt::Arguments) {
    // Stub implementation for early bring-up.
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
        for byte in buf {
            _print(format_args!("{}", *byte as char));
        }
        Ok(buf.len())
    }
}
