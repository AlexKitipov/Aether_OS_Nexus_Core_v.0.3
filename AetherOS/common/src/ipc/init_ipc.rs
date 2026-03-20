extern crate alloc;

use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InitRequest {
    ServiceStart { service_name: String },
    ServiceStatus { service_name: String },
    ServiceRestart { service_name: String },
    ServiceStop { service_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InitResponse {
    Success(String),
    Error(String),
    Status {
        service_name: String,
        is_running: bool,
        pid: Option<u64>,
    },
}
