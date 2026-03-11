use std::collections::BTreeMap;

use common::ipc::{IpcSend, vnode::VNodeChannel};
use common::syscall::{syscall3, SYS_LOG, SYS_TIME};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum InitRequest {
    ServiceStart { service_name: String },
    ServiceStatus { service_name: String },
    ServiceRestart { service_name: String },
    ServiceStop { service_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum InitResponse {
    Success(String),
    Error(String),
    Status {
        service_name: String,
        is_running: bool,
        pid: Option<u64>,
    },
}

fn log(msg: &str) {
    let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
}

#[derive(Debug, Clone)]
struct RunningVNode {
    pid: u64,
}

struct InitService {
    client_chan: VNodeChannel,
    running_vnodes: BTreeMap<String, RunningVNode>,
    next_pid: u64,
}

impl InitService {
    fn new(client_chan_id: u32) -> Self {
        Self { client_chan: VNodeChannel::new(client_chan_id), running_vnodes: BTreeMap::new(), next_pid: 1000 }
    }

    fn handle_request(&mut self, request: InitRequest) -> InitResponse {
        match request {
            InitRequest::ServiceStart { service_name } => {
                let pid = self.next_pid;
                self.next_pid += 1;
                self.running_vnodes.insert(service_name.clone(), RunningVNode { pid });
                InitResponse::Success(format!("Service '{service_name}' started with PID {pid}."))
            }
            InitRequest::ServiceStatus { service_name } => {
                let pid = self.running_vnodes.get(&service_name).map(|v| v.pid);
                InitResponse::Status { service_name, is_running: pid.is_some(), pid }
            }
            InitRequest::ServiceRestart { service_name } => {
                self.running_vnodes.remove(&service_name);
                self.handle_request(InitRequest::ServiceStart { service_name })
            }
            InitRequest::ServiceStop { service_name } => {
                if self.running_vnodes.remove(&service_name).is_some() {
                    InitResponse::Success(format!("Service '{service_name}' stopped."))
                } else {
                    InitResponse::Error(format!("Service '{service_name}' not running."))
                }
            }
        }
    }

    fn run_loop(&mut self) -> ! {
        log("Init Service: Entering main event loop.");
        loop {
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                match postcard::from_bytes::<InitRequest>(&req_data) {
                    Ok(request) => {
                        let response = self.handle_request(request);
                        let _ = self.client_chan.send(&response);
                    }
                    Err(_) => log("Init Service: Failed to deserialize InitRequest."),
                }
            }
            let _ = syscall3(SYS_TIME, 0, 0, 0);
        }
    }
}

fn main() {
    InitService::new(6).run_loop();
}
