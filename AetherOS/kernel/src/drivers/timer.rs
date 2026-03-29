#![allow(dead_code)]

use alloc::vec::Vec;

use crate::device::{Capability, CapabilitySet, Device, DeviceId, DeviceKind, InterruptHandler, IoDevice, IoError, IoResult, Rights, DEVICE_TIMER};

pub struct TimerDriver;

impl TimerDriver {
    pub const fn new() -> Self {
        Self
    }
}

impl Device for TimerDriver {
    fn id(&self) -> DeviceId {
        DEVICE_TIMER
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Timer
    }

    fn capabilities(&self) -> CapabilitySet {
        Vec::from([Capability {
            device: DEVICE_TIMER,
            rights: Rights::READ,
        }])
    }
}

impl IoDevice for TimerDriver {
    fn read(&self, buf: &mut [u8]) -> IoResult<usize> {
        if buf.len() < core::mem::size_of::<u64>() {
            return Err(IoError::Fault);
        }
        let ticks = crate::timer::get_current_ticks().to_le_bytes();
        let len = ticks.len();
        buf[..len].copy_from_slice(&ticks);
        Ok(len)
    }

    fn write(&self, _buf: &[u8]) -> IoResult<usize> {
        Err(IoError::Unsupported)
    }
}


impl InterruptHandler for TimerDriver {
    fn handle_irq(&self) {
        // Timer IRQ side-effects are handled in the global timer module.
    }
}
