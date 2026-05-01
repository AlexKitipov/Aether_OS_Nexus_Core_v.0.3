use crate::process::Pid;
use crate::sandbox::{ArxError, Capability};
use crate::ArxRuntime;

pub fn console_write(runtime: &ArxRuntime<'_>, pid: Pid, _bytes: &[u8]) -> Result<(), ArxError> {
    let process = runtime.process_table().get(pid).ok_or(ArxError::ProcessNotFound)?;
    if !process.capabilities.has(Capability::Io) {
        return Err(ArxError::PermissionDenied);
    }
    Ok(())
}

pub fn log(runtime: &ArxRuntime<'_>, pid: Pid, msg: &[u8]) -> Result<(), ArxError> {
    console_write(runtime, pid, msg)
}
