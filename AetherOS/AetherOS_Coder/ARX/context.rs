use crate::sandbox::{ArxError, Capability};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppContext {
    pub used_memory: usize,
    pub memory_limit: usize,
    pub last_wakeup_tick: u64,
}

impl AppContext {
    pub const fn new(memory_limit: usize) -> Self {
        Self {
            used_memory: 0,
            memory_limit,
            last_wakeup_tick: 0,
        }
    }

    pub fn reserve_memory(&mut self, bytes: usize) -> Result<(), ArxError> {
        let next = self.used_memory.saturating_add(bytes);
        if next > self.memory_limit {
            return Err(ArxError::MemoryLimitExceeded);
        }
        self.used_memory = next;
        Ok(())
    }

    pub fn release_memory(&mut self, bytes: usize) {
        self.used_memory = self.used_memory.saturating_sub(bytes);
    }

    pub fn requires(&self, _capability: Capability) -> bool {
        true
    }
}
