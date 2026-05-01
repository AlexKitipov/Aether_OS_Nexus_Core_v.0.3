use crate::sandbox::{ArxError, Capability};
use crate::{ApmRequestKind, ArxRuntime, Pid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyscallId {
    Yield,
    Sleep,
    RequestApmManifest,
}

pub type SyscallFn = fn(&mut ArxRuntime<'_>, Pid, usize) -> Result<usize, ArxError>;

pub struct SyscallEntry {
    pub id: SyscallId,
    pub func: SyscallFn,
}

pub const SYSCALL_TABLE: &[SyscallEntry] = &[
    SyscallEntry { id: SyscallId::Yield, func: sys_yield },
    SyscallEntry { id: SyscallId::Sleep, func: sys_sleep },
    SyscallEntry { id: SyscallId::RequestApmManifest, func: sys_apm_manifest },
];

pub fn dispatch(runtime: &mut ArxRuntime<'_>, caller: Pid, id: SyscallId, arg0: usize) -> Result<usize, ArxError> {
    let entry = SYSCALL_TABLE.iter().find(|e| e.id == id).ok_or(ArxError::InvalidSyscall)?;
    (entry.func)(runtime, caller, arg0)
}

fn sys_yield(runtime: &mut ArxRuntime<'_>, caller: Pid, _arg0: usize) -> Result<usize, ArxError> {
    runtime.process_table_mut().yield_now(caller)?;
    Ok(0)
}

fn sys_sleep(runtime: &mut ArxRuntime<'_>, caller: Pid, ticks: usize) -> Result<usize, ArxError> {
    runtime.sleep_pid(caller, ticks as u64)?;
    Ok(0)
}

fn sys_apm_manifest(runtime: &mut ArxRuntime<'_>, caller: Pid, _arg0: usize) -> Result<usize, ArxError> {
    let caller_proc = runtime.process_table().get(caller).ok_or(ArxError::ProcessNotFound)?;
    if !caller_proc.capabilities.has(Capability::Apm) {
        return Err(ArxError::PermissionDenied);
    }
    runtime.queue_apm_request(caller, ApmRequestKind::Manifest)?;
    Ok(1)
}
