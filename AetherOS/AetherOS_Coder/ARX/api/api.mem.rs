use crate::process::Pid;
use crate::sandbox::{ArxError, Capability};
use crate::ArxRuntime;

pub fn reserve(runtime: &mut ArxRuntime<'_>, pid: Pid, bytes: usize) -> Result<(), ArxError> {
    let process = runtime
        .process_table_mut()
        .get_mut(pid)
        .ok_or(ArxError::ProcessNotFound)?;
    if !process.capabilities.has(Capability::Memory) {
        return Err(ArxError::PermissionDenied);
    }
    process.context.reserve_memory(bytes)
}

pub fn release(runtime: &mut ArxRuntime<'_>, pid: Pid, bytes: usize) -> Result<(), ArxError> {
    let process = runtime
        .process_table_mut()
        .get_mut(pid)
        .ok_or(ArxError::ProcessNotFound)?;
    process.context.release_memory(bytes);
    Ok(())
}
