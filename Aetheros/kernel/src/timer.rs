// kernel/src/timer.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use core::sync::atomic::{AtomicU64, Ordering};

use crate::kprintln;

/// Global monotonic tick counter.
/// Incremented by the timer interrupt handler.
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Default timer IRQ number for PIT on x86 PIC.
pub const TIMER_IRQ: u8 = 0;

/// Initializes the Programmable Interrupt Timer (PIT) or other timer hardware.
pub fn init() {
    kprintln!("[kernel] timer: Initialized for periodic preemption.");
}

/// Called by the timer interrupt handler.
/// Increments the global tick counter and drives scheduler preemption.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::SeqCst);
    crate::task::on_timer_tick();
}

/// Returns the current number of ticks since boot.
pub fn get_current_ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}
