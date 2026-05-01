use crate::process::Pid;
use crate::sandbox::{ArxError, CapabilitySet};
use crate::ArxRuntime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadKind {
    App,
    Driver,
    Agent,
}

pub struct AppBinary {
    pub kind: LoadKind,
    pub entry: usize,
}

pub struct ArxLoader;

impl ArxLoader {
    pub fn load_binary(kind: LoadKind, entry: usize) -> AppBinary {
        AppBinary { kind, entry }
    }

    pub fn initialize_process(runtime: &mut ArxRuntime<'_>, _binary: AppBinary, capabilities: CapabilitySet, memory_limit: usize) -> Result<Pid, ArxError> {
        runtime.spawn(capabilities, memory_limit)
    }
}
