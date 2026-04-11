extern crate alloc;

use alloc::string::String;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InitRequest {
    BootstrapCoreServices,
    ServiceStart { service_name: String },
    ServiceStatus { service_name: String },
    ServiceReady { service_name: String, pid: Option<u64> },
    ServiceRestart { service_name: String },
    ServiceStop { service_name: String },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InitResponse {
    Success(String),
    Error(String),
    BootstrapReport {
        started_services: alloc::vec::Vec<String>,
    },
    Status {
        service_name: String,
        is_running: bool,
        pid: Option<u64>,
    },
}
