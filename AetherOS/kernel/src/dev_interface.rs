extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use aetheros_common::channel::well_known;
use aetheros_common::ipc::dev_interface_ipc::{
    DevInterfaceRequest,
    DevInterfaceResponse,
    DeviceRightSummary,
    TaskLaunchMode,
};

use crate::device::Right;
use crate::{ipc, kprintln, memory, task, timer};

const DEV_INTERFACE_TASK_ID: u32 = 0;

pub fn init() {
    let channel_id = ipc::mailbox::ensure_channel(well_known::DEV_INTERFACE);
    kprintln!(
        "[kernel] dev-interface: ready on channel {} for editor/IDE bridge.",
        channel_id
    );
}

pub fn poll_once() {
    let Some(message) = ipc::mailbox::recv(well_known::DEV_INTERFACE) else {
        return;
    };

    let request = match postcard::from_bytes::<DevInterfaceRequest>(&message.data) {
        Ok(request) => request,
        Err(_) => {
            let _ = send_response(DevInterfaceResponse::Error {
                message: String::from("invalid request payload"),
            });
            return;
        }
    };

    let response = handle_request(request);
    let _ = send_response(response);
}

fn handle_request(request: DevInterfaceRequest) -> DevInterfaceResponse {
    match request {
        DevInterfaceRequest::Ping => DevInterfaceResponse::Pong,
        DevInterfaceRequest::RunTask {
            name,
            inherit_from,
            executable_path,
        } => run_task(name, inherit_from, executable_path),
        DevInterfaceRequest::InspectSystemState => DevInterfaceResponse::SystemState {
            ticks: timer::get_current_ticks(),
            current_task_id: task::scheduler::get_current_task_id(),
            task_count: task::scheduler::task_count(),
            runnable_tasks: task::scheduler::runnable_count(),
            mem_used: memory::total_memory().saturating_sub(memory::free_memory()),
            mem_free: memory::free_memory(),
        },
        DevInterfaceRequest::InspectVNodeCapabilities { task_id } => inspect_vnode(task_id),
    }
}

fn run_task(
    name: String,
    inherit_from: Option<u64>,
    executable_path: Option<String>,
) -> DevInterfaceResponse {
    let task_id = task::scheduler::allocate_task_id();

    if let Some(path) = executable_path {
        let parent = inherit_from.unwrap_or(task::scheduler::get_current_task_id());
        let Some(parent_task) = task::scheduler::get_task(parent) else {
            return DevInterfaceResponse::Error {
                message: format!("could not inherit capabilities from task {}", parent),
            };
        };

        match task::spawn_from_file(&path, task_id, &name, parent_task.capabilities) {
            Ok(()) => DevInterfaceResponse::TaskStarted {
                task_id,
                name,
                mode: TaskLaunchMode::SpawnFromAetherFs,
            },
            Err(err) => DevInterfaceResponse::Error { message: err },
        }
    } else {
        let parent = inherit_from.unwrap_or(task::scheduler::get_current_task_id());
        if !task::create_task_inheriting(parent, task_id, &name) {
            return DevInterfaceResponse::Error {
                message: format!("could not inherit capabilities from task {}", parent),
            };
        }

        DevInterfaceResponse::TaskStarted {
            task_id,
            name,
            mode: TaskLaunchMode::Inherit,
        }
    }
}

fn inspect_vnode(task_id: u64) -> DevInterfaceResponse {
    let Some(tcb) = task::scheduler::get_task(task_id) else {
        return DevInterfaceResponse::Error {
            message: format!("task {} not found", task_id),
        };
    };

    let capabilities = tcb
        .capabilities
        .iter()
        .map(|cap| format!("{cap:?}"))
        .collect::<Vec<_>>();

    let vnode = crate::device::vnode_caps_from_task(task_id);
    let device_rights = vnode
        .caps
        .iter()
        .map(|cap| DeviceRightSummary {
            device_id: cap.device,
            can_read: cap.rights.allows(Right::Read),
            can_write: cap.rights.allows(Right::Write),
            can_manage: cap.rights.allows(Right::Manage),
            can_interrupt: cap.rights.allows(Right::Interrupt),
        })
        .collect::<Vec<_>>();

    DevInterfaceResponse::VNodeCapabilities {
        task_id,
        capabilities,
        device_rights,
    }
}

fn send_response(_response: DevInterfaceResponse) -> Result<(), ()> {
    let payload = Vec::new(); // TODO: postcard::to_allocvec(&response).map_err(|_| ())?;
    ipc::mailbox::send(well_known::DEV_INTERFACE, DEV_INTERFACE_TASK_ID, &payload).map_err(|_| ())
}
