// kernel/src/timer.rs

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};

use x86_64::instructions::port::Port;

use crate::kprintln;

/// PIT base input clock in Hz.
const PIT_BASE_FREQUENCY_HZ: u32 = 1_193_182;
/// Desired scheduler/system tick rate.
const PIT_FREQUENCY_HZ: u32 = 100;

const PIT_COMMAND_PORT: u16 = 0x43;
const PIT_CHANNEL0_PORT: u16 = 0x40;

/// Global monotonic tick counter incremented from IRQ0.
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Programs the PIT channel 0 in square-wave mode.
pub fn init() {
    let divisor: u16 = (PIT_BASE_FREQUENCY_HZ / PIT_FREQUENCY_HZ) as u16;

    unsafe {
        // SAFETY: Accessing PIT command/data ports is required for timer programming
        // on x86_64 PC-compatible platforms.
        let mut command: Port<u8> = Port::new(PIT_COMMAND_PORT);
        let mut channel0: Port<u8> = Port::new(PIT_CHANNEL0_PORT);

        // Channel 0 | access mode lobyte/hibyte | mode 3 (square wave) | binary.
        command.write(0x36);
        channel0.write((divisor & 0x00ff) as u8);
        channel0.write((divisor >> 8) as u8);
    }

    kprintln!(
        "[kernel] timer: PIT configured at {} Hz (divisor={}).",
        PIT_FREQUENCY_HZ,
        divisor
    );
}

/// Called by IRQ0 handler.
#[inline]
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Total number of timer ticks since boot.
#[inline]
pub fn get_current_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Configured timer frequency in Hz.
#[inline]
pub const fn frequency_hz() -> u32 {
    PIT_FREQUENCY_HZ
}

/// Uptime in milliseconds derived from the tick counter.
#[inline]
pub fn uptime_ms() -> u64 {
    get_current_ticks().saturating_mul(1_000) / PIT_FREQUENCY_HZ as u64
}
