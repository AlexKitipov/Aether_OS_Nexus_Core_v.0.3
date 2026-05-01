#![no_std]

extern crate alloc;

use adi::interface::ADIInterface;

use crate::sandbox::{ArxError, CapabilitySet};

pub mod api;
pub mod context;
pub mod loader;
pub mod process;
pub mod sandbox;

pub use context::*;
pub use loader::*;
pub use process::*;

pub const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEvent {
    ProcessScheduled { pid: Pid },
    ProcessTimedOut { pid: Pid },
    ProcessMessage { from: Pid, to: Pid },
    AdiRequest { pid: Pid, kind: AdiRequestKind },
    ApmRequest { pid: Pid, kind: ApmRequestKind },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdiRequestKind {
    DriverGeneration,
    Diagnostics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApmRequestKind {
    Manifest,
    DependencyGraph,
    Update,
    Sbom,
}

pub struct ArxRuntime<'a> {
    adi: &'a ADIInterface,
    process_table: ProcessTable,
    event_queue: EventQueue,
    next_pid: Pid,
    ticks: u64,
}

impl<'a> ArxRuntime<'a> {
    pub fn new(adi: &'a ADIInterface) -> Self {
        Self {
            adi,
            process_table: ProcessTable::new(),
            event_queue: EventQueue::new(DEFAULT_EVENT_QUEUE_CAPACITY),
            next_pid: 1,
            ticks: 0,
        }
    }

    pub fn spawn(&mut self, capabilities: CapabilitySet, memory_limit: usize) -> Result<Pid, ArxError> {
        let pid = self.next_pid;
        self.next_pid = self.next_pid.wrapping_add(1);

        let process = Process::new(pid, AppContext::new(memory_limit), capabilities, memory_limit);
        self.process_table.insert(process)?;
        Ok(pid)
    }

    pub fn tick(&mut self) {
        let _ = self.adi;
        self.ticks = self.ticks.wrapping_add(1);

        if let Some(pid) = self.process_table.next_ready_pid() {
            if let Some(process) = self.process_table.get_mut(pid) {
                process.state = ProcessState::Running;
            }
            let _ = self.event_queue.push(RuntimeEvent::ProcessScheduled { pid });
        }

        while let Some(event) = self.event_queue.pop() {
            self.dispatch(event);
        }
    }

    pub fn enqueue_event(&mut self, event: RuntimeEvent) -> Result<(), ArxError> {
        self.event_queue.push(event)
    }

    pub fn process_table(&self) -> &ProcessTable {
        &self.process_table
    }

    pub fn process_table_mut(&mut self) -> &mut ProcessTable {
        &mut self.process_table
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    fn dispatch(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::ProcessScheduled { pid } => {
                if let Some(process) = self.process_table.get_mut(pid) {
                    process.state = ProcessState::Ready;
                }
            }
            RuntimeEvent::ProcessTimedOut { pid } => {
                if let Some(process) = self.process_table.get_mut(pid) {
                    process.state = ProcessState::Waiting;
                }
            }
            RuntimeEvent::ProcessMessage { from: _, to } => {
                if let Some(process) = self.process_table.get_mut(to) {
                    process.state = ProcessState::Ready;
                }
            }
            RuntimeEvent::AdiRequest { pid, kind: _ } | RuntimeEvent::ApmRequest { pid, kind: _ } => {
                if let Some(process) = self.process_table.get_mut(pid) {
                    process.state = ProcessState::Waiting;
                }
            }
        }
    }
}

pub type ArxManager<'a> = ArxRuntime<'a>;
