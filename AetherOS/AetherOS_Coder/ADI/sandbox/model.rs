use alloc::string::String;
use alloc::vec::Vec;

/// Represents the capabilities a driver might require or provide.
#[derive(PartialEq, Debug)]
pub enum Capability {
    /// Access to hardware peripherals
    HardwareAccess,
    /// Network communication
    NetworkAccess,
    /// File system operations
    FileSystemAccess,
    /// Inter-process communication
    IpcAccess,
    /// Memory management operations
    MemoryManagement,
    // Add more capabilities as needed
}

/// Represents the model of a driver, detailing its required and provided capabilities.
#[derive(Default)]
pub struct DriverModel {
    /// Unique identifier for the driver
    pub id: u64,
    /// Name of the driver
    pub name: String,
    /// Version of the driver
    pub version: String,
    /// Capabilities required by this driver
    pub requires: Vec<Capability>,
    /// Capabilities provided by this driver
    pub provides: Vec<Capability>,
}

impl DriverModel {
    /// Creates a new DriverModel.
    pub fn new(id: u64, name: String, version: String, requires: Vec<Capability>, provides: Vec<Capability>) -> Self {
        DriverModel {
            id,
            name,
            version,
            requires,
            provides,
        }
    }
}
