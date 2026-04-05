extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DevInterfaceRequest {
    Ping,
    RunTask {
        name: String,
        inherit_from: Option<u64>,
        executable_path: Option<String>,
    },
    InspectSystemState,
    InspectVNodeCapabilities {
        task_id: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DevInterfaceResponse {
    Pong,
    TaskStarted {
        task_id: u64,
        name: String,
        mode: TaskLaunchMode,
    },
    SystemState {
        ticks: u64,
        current_task_id: u64,
        task_count: usize,
        runnable_tasks: usize,
        mem_used: usize,
        mem_free: usize,
    },
    VNodeCapabilities {
        task_id: u64,
        capabilities: Vec<String>,
        device_rights: Vec<DeviceRightSummary>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskLaunchMode {
    Inherit,
    SpawnFromAetherFs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRightSummary {
    pub device_id: u64,
    pub can_read: bool,
    pub can_write: bool,
    pub can_manage: bool,
    pub can_interrupt: bool,
}
