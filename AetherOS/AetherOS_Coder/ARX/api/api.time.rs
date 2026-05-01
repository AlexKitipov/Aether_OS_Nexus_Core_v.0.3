use crate::process::Pid;
use crate::sandbox::{ArxError, Capability};
use crate::ArxRuntime;

pub fn monotonic_ticks(runtime: &ArxRuntime<'_>) -> u64 {
    runtime.ticks()
}

pub fn sleep(runtime: &mut ArxRuntime<'_>, pid: Pid) -> Result<(), ArxError> {
    let process = runtime.process_table().get(pid).ok_or(ArxError::ProcessNotFound)?;
    if !process.capabilities.has(Capability::Time) {
        return Err(ArxError::PermissionDenied);
    }
    runtime.process_table_mut().wait(pid)
}

pub fn yield_now(runtime: &mut ArxRuntime<'_>, pid: Pid) -> Result<(), ArxError> {
    runtime.process_table_mut().yield_now(pid)
}
