#![no_std]

extern crate alloc;

use adi::interface::ADIInterface;

pub mod loader;
pub mod process;
pub mod context;
pub mod api;

pub use loader::*;
pub use process::*;
pub use context::*;

pub struct ArxManager<'a> {
    adi: &'a ADIInterface,
    ticks: u64,
}

impl<'a> ArxManager<'a> {
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
