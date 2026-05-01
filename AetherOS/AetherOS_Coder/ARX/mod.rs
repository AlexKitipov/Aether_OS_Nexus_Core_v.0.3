#![no_std]

extern crate alloc;

use adi::interface::ADIInterface;

use crate::sandbox::{ArxError, Capability, CapabilitySet};

pub mod api;
pub mod context;
pub mod loader;
pub mod process;
pub mod sandbox;
pub mod syscall;

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
    AdiResponse { pid: Pid, ok: bool },
    ApmResponse { pid: Pid, ok: bool },
    TimerWake { pid: Pid },
    Panic { pid: Pid },
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
    current_pid: Option<Pid>,
    panic_log: EventQueue,
    sleep_queue: SleepQueue,
    rr_cursor: Option<Pid>,
    adi_pending: EventQueue,
    apm_pending: EventQueue,
}

impl<'a> ArxRuntime<'a> {
    pub fn new(adi: &'a ADIInterface) -> Self {
        Self {
            adi,
            process_table: ProcessTable::new(),
            event_queue: EventQueue::new(DEFAULT_EVENT_QUEUE_CAPACITY),
            next_pid: 1,
            ticks: 0,
            current_pid: None,
            panic_log: EventQueue::new(DEFAULT_EVENT_QUEUE_CAPACITY),
            sleep_queue: SleepQueue::new(),
            rr_cursor: None,
            adi_pending: EventQueue::new(DEFAULT_EVENT_QUEUE_CAPACITY),
            apm_pending: EventQueue::new(DEFAULT_EVENT_QUEUE_CAPACITY),
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

        self.wake_sleepers();
        self.preempt_current();

        if let Some(pid) = self.schedule_next() {
            if let Some(process) = self.process_table.get_mut(pid) {
                process.state = ProcessState::Running;
                process.cpu_time_ticks = process.cpu_time_ticks.saturating_add(1);
                process.remaining_slice = process.remaining_slice.saturating_sub(1);
                self.current_pid = Some(pid);
            }
            let _ = self.event_queue.push(RuntimeEvent::ProcessScheduled { pid });
        }

        while let Some(event) = self.event_queue.pop() {
            self.dispatch(event);
        }
    }

    pub fn sleep_pid(&mut self, pid: Pid, sleep_ticks: u64) -> Result<(), ArxError> {
        let process = self.process_table.get_mut(pid).ok_or(ArxError::ProcessNotFound)?;
        process.state = ProcessState::Waiting;
        process.context.last_wakeup_tick = self.ticks.saturating_add(sleep_ticks);
        self.sleep_queue.insert(pid, process.context.last_wakeup_tick);
        Ok(())
    }

    pub fn queue_adi_request(&mut self, pid: Pid, kind: AdiRequestKind) -> Result<(), ArxError> {
        self.ensure_capability(pid, Capability::Adi)?;
        self.adi_pending.push(RuntimeEvent::AdiRequest { pid, kind })?;
        self.event_queue.push(RuntimeEvent::AdiRequest { pid, kind })
    }

    pub fn queue_apm_request(&mut self, pid: Pid, kind: ApmRequestKind) -> Result<(), ArxError> {
        self.ensure_capability(pid, Capability::Apm)?;
        self.apm_pending.push(RuntimeEvent::ApmRequest { pid, kind })?;
        self.event_queue.push(RuntimeEvent::ApmRequest { pid, kind })
    }

    pub fn debug_ps(&self) -> &[Process] { self.process_table.list() }
    pub fn debug_cap(&self, pid: Pid) -> Option<CapabilitySet> { self.process_table.get(pid).map(|p| p.capabilities) }
    pub fn debug_mem(&self, pid: Pid) -> Option<usize> { self.process_table.get(pid).map(|p| p.context.used_memory) }
    pub fn debug_msg(&self, pid: Pid) -> Option<usize> { self.process_table.get(pid).map(|p| p.inbox_len()) }

    fn ensure_capability(&self, pid: Pid, cap: Capability) -> Result<(), ArxError> {
        let p = self.process_table.get(pid).ok_or(ArxError::ProcessNotFound)?;
        if p.capabilities.has(cap) { Ok(()) } else { Err(ArxError::PermissionDenied) }
    }

    fn wake_sleepers(&mut self) {
        while let Some(pid) = self.sleep_queue.pop_ready(self.ticks) {
            let _ = self.event_queue.push(RuntimeEvent::TimerWake { pid });
        }
    }

    fn preempt_current(&mut self) {
        if let Some(pid) = self.current_pid {
            if let Some(proc_) = self.process_table.get_mut(pid) {
                if proc_.state == ProcessState::Running && proc_.remaining_slice == 0 {
                    proc_.state = ProcessState::Ready;
                    proc_.remaining_slice = proc_.time_slice;
                    self.current_pid = None;
                }
            }
        }
    }

    fn schedule_next(&mut self) -> Option<Pid> {
        let best = self.process_table.next_ready_pid()?;
        let best_prio = self.process_table.get(best)?.priority;
        let chosen = if let Some(cursor) = self.rr_cursor {
            self.process_table.next_ready_pid_after(cursor, best_prio).unwrap_or(best)
        } else { best };
        self.rr_cursor = Some(chosen);
        Some(chosen)
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
            RuntimeEvent::AdiResponse { pid, ok: _ } | RuntimeEvent::ApmResponse { pid, ok: _ } | RuntimeEvent::TimerWake { pid } => {
                if let Some(process) = self.process_table.get_mut(pid) {
                    process.state = ProcessState::Ready;
                    process.remaining_slice = process.time_slice;
                }
            }
            RuntimeEvent::Panic { pid } => {
                if let Some(process) = self.process_table.get_mut(pid) {
                    process.panic_flag = true;
                    process.state = if process.restart_on_fault { ProcessState::Ready } else { ProcessState::Terminated };
                    let _ = self.panic_log.push(RuntimeEvent::Panic { pid });
                }
            }
        }
    }
}

#[derive(Default)]
struct SleepQueue {
    entries: alloc::vec::Vec<(Pid, u64)>,
}

impl SleepQueue {
    fn new() -> Self { Self { entries: alloc::vec::Vec::new() } }
    fn insert(&mut self, pid: Pid, wake_tick: u64) { self.entries.push((pid, wake_tick)); }
    fn pop_ready(&mut self, now: u64) -> Option<Pid> {
        if let Some((idx, _)) = self.entries.iter().enumerate().find(|(_, (_, t))| *t <= now) {
            Some(self.entries.swap_remove(idx).0)
        } else {
            None
        }
    }
}

pub type ArxManager<'a> = ArxRuntime<'a>;
