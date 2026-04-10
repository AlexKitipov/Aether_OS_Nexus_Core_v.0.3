#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "serde")]
extern crate serde;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use aetheros_common::ipc::IpcSend;
use aetheros_common::ipc::init_ipc::{InitRequest, InitResponse};
use aetheros_common::ipc::vnode::VNodeChannel;
use aetheros_common::syscall::{syscall3, SUCCESS, SYS_LOG, SYS_TIME};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}

fn init_allocator() {
    unsafe {
        ALLOCATOR.lock().init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn log(msg: &str) {
    let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
}

#[derive(Debug, Clone)]
struct RunningVNode {
    pid: u64,
    capabilities: Vec<String>,
}

struct InitService {
    client_chan: VNodeChannel,
    running_vnodes: BTreeMap<String, RunningVNode>,
    next_pid: u64,
}

impl InitService {
    fn new(client_chan_id: u32) -> Self {
        Self {
            client_chan: VNodeChannel::new(client_chan_id),
            running_vnodes: BTreeMap::new(),
            next_pid: 1000,
        }
    }

    fn handle_request(&mut self, request: InitRequest) -> InitResponse {
        match request {
            InitRequest::BootstrapCoreServices => {
                let mut started_services = Vec::new();
                for service_name in ["echo"] {
                    let response = self.handle_request(InitRequest::ServiceStart {
                        service_name: service_name.to_string(),
                    });

                    if matches!(response, InitResponse::Success(_)) {
                        started_services.push(service_name.to_string());
                    }
                }

                InitResponse::BootstrapReport { started_services }
            }
            InitRequest::ServiceStart { service_name } => {
                if self.running_vnodes.contains_key(&service_name) {
                    return InitResponse::Success(format!(
                        "Service '{service_name}' is already running."
                    ));
                }
                let pid = self.next_pid;
                self.next_pid += 1;
                self.running_vnodes.insert(
                    service_name.clone(),
                    RunningVNode {
                        pid,
                        capabilities: vec!["NetworkAccess".to_string()],
                    },
                );
                InitResponse::Success(format!("Service '{service_name}' started with PID {pid}."))
            }
            InitRequest::ServiceStatus { service_name } => {
                let pid = self.running_vnodes.get(&service_name).map(|v| v.pid);
                InitResponse::Status {
                    service_name,
                    is_running: pid.is_some(),
                    pid,
                }
            }
            InitRequest::ServiceReady { service_name, pid } => {
                let assigned_pid = pid.unwrap_or_else(|| {
                    let pid = self.next_pid;
                    self.next_pid += 1;
                    pid
                });
                self.running_vnodes
                    .entry(service_name.clone())
                    .and_modify(|v| v.pid = assigned_pid)
                    .or_insert(RunningVNode {
                        pid: assigned_pid,
                        capabilities: Vec::new(),
                    });
                InitResponse::Success(format!(
                    "Service '{service_name}' marked ready with PID {assigned_pid}."
                ))
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
                let data: Vec<u8> = req_data.to_vec();
                match postcard::from_bytes::<InitRequest>(&data) {
                    Ok(request) => {
                        let response = self.handle_request(request);
                        if let Ok(serialized_response) = postcard::to_allocvec(&response) {
                            let _ = self.client_chan.send_raw(&serialized_response);
                        } else {
                            log("Init Service: Failed to serialize InitResponse.");
                        }
                    }
                    Err(_) => log("Init Service: Failed to deserialize InitRequest."),
                }
            }

            let _ = syscall3(SYS_TIME, 0, 0, 0);
            let _ = SUCCESS;
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    let mut init_service = InitService::new(6);
    let _ = init_service.handle_request(InitRequest::BootstrapCoreServices);
    init_service.run_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

fn main() -> ! {
    // TODO: реална логика за init-service
    loop {}
}
