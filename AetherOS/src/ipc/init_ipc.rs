
// src/ipc/init_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

use serde::{Deserialize, Serialize};

/// Represents requests from client V-Nodes to the init-service V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum InitRequest {
    /// Start a V-Node by its name.
    ServiceStart { service_name: String },
    /// Get the status of a V-Node.
    ServiceStatus { service_name: String },
    /// Restart a V-Node.
    ServiceRestart { service_name: String },
    /// Stop a V-Node.
    ServiceStop { service_name: String },
}

/// Represents responses from the init-service V-Node to client V-Nodes.
#[derive(Debug, Serialize, Deserialize)]
pub enum InitResponse {
    /// Indicates successful operation.
    Success(String), // Success message
    /// Returns the status of a V-Node.
    Status { service_name: String, is_running: bool, pid: Option<u64> },
    /// Indicates an error occurred.
    Error(String), // Error message
}
