use super::model::{Capability, DriverModel};
use alloc::vec::Vec;

/// Represents the security governor for enforcing access control and policies.
pub struct Governor {
    /// A list of allowed capabilities for drivers.
    pub allowed_capabilities: Vec<Capability>,
    // Add more security policies or rules here
}

impl Default for Governor {
    fn default() -> Self {
        Self {
            allowed_capabilities: Vec::new(),
        }
    }
}

impl Governor {
    /// Creates a new Governor instance with a set of allowed capabilities.
    pub fn new(allowed_capabilities: Vec<Capability>) -> Self {
        Governor {
            allowed_capabilities,
        }
    }

    /// Validates if a driver model's required capabilities are allowed by the governor.
    pub fn validate(&self, driver_model: &DriverModel) -> Result<(), &'static str> {
        for required_cap in &driver_model.requires {
            if !self.allowed_capabilities.contains(required_cap) {
                return Err("Driver requires an disallowed capability");
            }
        }
        // println!("Driver model {} validated successfully.", driver_model.name); // Commented out for no_std compatibility
        Ok(())
    }
}
