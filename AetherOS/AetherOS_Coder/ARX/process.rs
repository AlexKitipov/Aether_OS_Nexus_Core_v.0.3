use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::context::AppContext;
use crate::sandbox::{ArxError, CapabilitySet};

pub type Pid = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessState {
    Created,
    Ready,
    Running,
    Waiting,
    Terminated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessMessage {
    pub from: Pid,
    pub payload: [u8; 64],
    pub payload_len: usize,
}

#[derive(Clone, Debug)]
pub struct Process {
    pub pid: Pid,
    pub state: ProcessState,
    pub context: AppContext,
    pub capabilities: CapabilitySet,
    pub memory_limit: usize,
    inbox: VecDeque<ProcessMessage>,
}

impl Process {
    pub fn new(pid: Pid, context: AppContext, capabilities: CapabilitySet, memory_limit: usize) -> Self {
        Self {
            pid,
            state: ProcessState::Created,
            context,
            capabilities,
            memory_limit,
            inbox: VecDeque::new(),
        }
    }

    pub fn push_message(&mut self, msg: ProcessMessage) {
        self.inbox.push_back(msg);
    }

    pub fn pop_message(&mut self) -> Option<ProcessMessage> {
        self.inbox.pop_front()
    }
}

pub struct ProcessTable {
    entries: Vec<Process>,
}

impl ProcessTable {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn insert(&mut self, mut process: Process) -> Result<(), ArxError> {
        if self.entries.len() >= 256 {
            return Err(ArxError::ProcessTableFull);
        }
        process.state = ProcessState::Ready;
        self.entries.push(process);
        Ok(())
    }

    pub fn spawn(&mut self, pid: Pid, capabilities: CapabilitySet, memory_limit: usize) -> Result<(), ArxError> {
        self.insert(Process::new(pid, AppContext::new(memory_limit), capabilities, memory_limit))
    }

    pub fn kill(&mut self, pid: Pid) -> Result<(), ArxError> {
        let process = self.get_mut(pid).ok_or(ArxError::ProcessNotFound)?;
        process.state = ProcessState::Terminated;
        Ok(())
    }

    pub fn yield_now(&mut self, pid: Pid) -> Result<(), ArxError> {
        let process = self.get_mut(pid).ok_or(ArxError::ProcessNotFound)?;
        process.state = ProcessState::Ready;
        Ok(())
    }

    pub fn wait(&mut self, pid: Pid) -> Result<(), ArxError> {
        let process = self.get_mut(pid).ok_or(ArxError::ProcessNotFound)?;
        process.state = ProcessState::Waiting;
        Ok(())
    }

    pub fn get_mut(&mut self, pid: Pid) -> Option<&mut Process> {
        self.entries.iter_mut().find(|p| p.pid == pid)
    }

    pub fn get(&self, pid: Pid) -> Option<&Process> {
        self.entries.iter().find(|p| p.pid == pid)
    }

    pub fn next_ready_pid(&self) -> Option<Pid> {
        self.entries
            .iter()
            .find(|p| p.state == ProcessState::Ready)
            .map(|p| p.pid)
    }

    pub fn list(&self) -> &[Process] {
        &self.entries
    }
}

pub struct EventQueue {
    queue: VecDeque<crate::RuntimeEvent>,
    capacity: usize,
}

impl EventQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            capacity,
        }
    }

    pub fn push(&mut self, event: crate::RuntimeEvent) -> Result<(), ArxError> {
        if self.queue.len() >= self.capacity {
            return Err(ArxError::EventQueueFull);
        }
        self.queue.push_back(event);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<crate::RuntimeEvent> {
        self.queue.pop_front()
    }
}
