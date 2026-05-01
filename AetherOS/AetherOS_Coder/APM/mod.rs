#![no_std]

extern crate alloc;

use adi::interface::ADIInterface;

pub mod manifest;
pub mod installer;
pub mod updater;
pub mod registry;

pub use manifest::*;
pub use installer::*;
pub use updater::*;
pub use registry::*;

pub struct ApmManager<'a> {
    adi: &'a ADIInterface,
    ticks: u64,
}

impl<'a> ApmManager<'a> {
    pub fn new(adi: &'a ADIInterface) -> Self {
        Self { adi, ticks: 0 }
    }

    pub fn tick(&mut self) {
        let _ = self.adi;
        self.ticks = self.ticks.wrapping_add(1);
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }
}
