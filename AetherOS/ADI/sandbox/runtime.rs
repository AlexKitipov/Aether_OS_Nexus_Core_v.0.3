use super::model::DriverModel;
use super::governor::Governor;
use alloc::string::String;
use alloc::vec::Vec;

/// Represents the runtime environment for executing driver operations.
pub struct Runtime {
    /// Maximum memory allowed for the driver (in bytes).
    pub memory_limit: usize,
    /// Current memory usage of the driver (in bytes).
    pub current_memory_usage: usize,
    /// Maximum execution cycles allowed.
    pub cycle_limit: u64,
    /// Current execution cycles used.
    pub current_cycles: u64,
    pub governor: Governor,
}

impl Runtime {
    /// Creates a new Runtime instance with specified limits.
    pub fn new(memory_limit: usize, cycle_limit: u64, governor: Governor) -> Self {
        Runtime {
            memory_limit,
            current_memory_usage: 0,
            cycle_limit,
            current_cycles: 0,
            governor,
        }
    }

    /// Executes a single step of an operation, consuming resources.
    pub fn step(&mut self, cycles_consumed: u64, memory_allocated: usize) -> Result<(), &'static str> {
        if self.current_cycles + cycles_consumed > self.cycle_limit {
            return Err("Cycle limit exceeded");
        }
        if self.current_memory_usage + memory_allocated > self.memory_limit {
            return Err("Memory limit exceeded");
        }

        self.current_cycles += cycles_consumed;
        self.current_memory_usage += memory_allocated;
        // println!("Runtime step: cycles={}, memory={}", self.current_cycles, self.current_memory_usage); // Commented out for no_std compatibility
        Ok(())
    }

    /// Validates a driver model using the internal governor.
    pub fn validate_driver(&self, driver_model: &DriverModel) -> Result<(), &'static str> {
        self.governor.validate(driver_model)
    }
}
