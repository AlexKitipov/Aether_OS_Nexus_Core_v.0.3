#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    Io,
    Time,
    Memory,
    Ipc,
    Adi,
    Apm,
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilitySet {
    bits: u16,
}

impl CapabilitySet {
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn with(self, capability: Capability) -> Self {
        Self {
            bits: self.bits | (1 << (capability as u16)),
        }
    }

    pub const fn has(self, capability: Capability) -> bool {
        (self.bits & (1 << (capability as u16))) != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArxError {
    PermissionDenied,
    ProcessNotFound,
    ProcessTableFull,
    EventQueueFull,
    MemoryLimitExceeded,
}
