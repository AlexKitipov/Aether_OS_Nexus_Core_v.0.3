// kernel/src/timer.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use core::sync::atomic::{AtomicU64, Ordering};
use crate::kprintln;

/// Global monotonic tick counter.
/// Incremented by the timer interrupt handler.
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Initializes the Programmable Interrupt Timer (PIT) or other timer hardware.
/// For a real system, this would configure the timer frequency.
pub fn init() {
    // In a real kernel, this would configure the PIT or other timer hardware
    // to generate interrupts at a regular interval (e.g., 100 Hz).
    kprintln!("[kernel] timer: Initialized (conceptual).");
}

/// Called by the timer interrupt handler.
/// Increments the global tick counter.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::SeqCst);
    // kprintln!("[kernel] timer: Tick! {}", TICKS.load(Ordering::SeqCst)); // Uncomment for noisy debug
}

/// Returns the current number of ticks since boot.
pub fn get_current_ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}
