use crate::process::{Pid, ProcessMessage};
use crate::sandbox::{ArxError, Capability};
use crate::ArxRuntime;

pub fn send_message(runtime: &mut ArxRuntime<'_>, from: Pid, to: Pid, msg: &[u8]) -> Result<(), ArxError> {
    let sender = runtime.process_table().get(from).ok_or(ArxError::ProcessNotFound)?;
    if !sender.capabilities.has(Capability::Ipc) {
        return Err(ArxError::PermissionDenied);
    }

    let receiver = runtime
        .process_table_mut()
        .get_mut(to)
        .ok_or(ArxError::ProcessNotFound)?;

    let mut payload = [0u8; 64];
    let len = msg.len().min(payload.len());
    payload[..len].copy_from_slice(&msg[..len]);
    receiver.push_message(ProcessMessage { from, payload, payload_len: len });
    runtime.enqueue_event(crate::RuntimeEvent::ProcessMessage { from, to })
}

pub fn recv_message(runtime: &mut ArxRuntime<'_>, pid: Pid) -> Result<Option<ProcessMessage>, ArxError> {
    let process = runtime
        .process_table_mut()
        .get_mut(pid)
        .ok_or(ArxError::ProcessNotFound)?;
    Ok(process.pop_message())
}
