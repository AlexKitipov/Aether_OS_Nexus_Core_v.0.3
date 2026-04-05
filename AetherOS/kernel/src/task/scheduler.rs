#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::collections::{BTreeMap, VecDeque};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::kprintln;
use crate::memory::page_allocator::PageAllocator;
use crate::task::tcb::{Context, TaskControlBlock, TaskState};
use crate::caps::Capability;

const MILLICORES_PER_CORE: u32 = 1000;

/// Minimal round-robin scheduler state model used by runtime bring-up docs/tests.
#[derive(Debug, Default)]
pub struct Scheduler {
    pub tasks: VecDeque<u64>,
    pub current: usize,
}

impl Scheduler {
    pub fn add_task(&mut self, task_id: u64) {
        self.tasks.push_back(task_id);
    }

    pub fn next_task(&mut self) -> Option<u64> {
        if self.tasks.is_empty() {
            return None;
        }
        self.current = (self.current + 1) % self.tasks.len();
        self.tasks.get(self.current).copied()
    }

    pub fn current_task(&self) -> Option<u64> {
        if self.tasks.is_empty() {
            None
        } else {
            self.tasks.get(self.current).copied()
        }
    }
}

/// The run queue holds task IDs of tasks that are ready to be scheduled.
/// This uses a simple `VecDeque` for a round-robin like behavior.
static RUN_QUEUE: Mutex<VecDeque<u64>> = Mutex::new(VecDeque::new());

/// A map of all active tasks, indexed by their ID.
static TASKS: Mutex<BTreeMap<u64, TaskControlBlock>> = Mutex::new(BTreeMap::new());

/// The ID of the currently executing task.
static CURRENT_TASK_ID: Mutex<u64> = Mutex::new(0); // Starts with kernel as task 0
static RESCHEDULE_REQUESTED: AtomicBool = AtomicBool::new(true);

/// Initializes the scheduler, setting up necessary data structures.
pub fn init() {
    kprintln!("[kernel] scheduler: Initializing...");

    // Create a dummy kernel task and add it to the task list.
    // In a real system, the initial kernel thread would be set up differently.
    let mut kernel_task = TaskControlBlock::new(
        0,
        alloc::string::String::from("kernel"),
        // Grant full capabilities to the kernel task for simulation purposes.
        // This will be refined as specific capabilities are designed.
        alloc::vec![
            crate::caps::Capability::LogWrite,
            crate::caps::Capability::TimeRead,
            crate::caps::Capability::NetworkAccess,
            crate::caps::Capability::IrqRegister(0),
            crate::caps::Capability::DmaAlloc,
            crate::caps::Capability::DmaAccess,
            crate::caps::Capability::IrqAck(0),
            crate::caps::Capability::IpcManage,
            crate::caps::Capability::StorageAccess,
            crate::caps::Capability::ReadMetrics,
            crate::caps::Capability::WriteLogs,
            crate::caps::Capability::RestartVNode,
            crate::caps::Capability::SyncSnapshots,
            crate::caps::Capability::ReadOwnMetrics,
        ],
    );

    kernel_task.state = TaskState::Running;

    {
        let mut tasks = TASKS.lock();
        tasks.insert(kernel_task.id, kernel_task.clone());
    }

    *CURRENT_TASK_ID.lock() = kernel_task.id;

    kprintln!("[kernel] scheduler: Initialized kernel task (ID: 0).");
}

/// Adds a new task to the scheduler's management.
pub fn add_task(task: TaskControlBlock) {
    interrupts::without_interrupts(|| {
        let task_id = task.id;
        kprintln!(
            "[kernel] scheduler: Adding task '{}' (ID: {}).",
            task.name,
            task_id
        );
        TASKS.lock().insert(task_id, task);
        RUN_QUEUE.lock().push_back(task_id);
    });
}

/// Removes a task from the scheduler's management.
pub fn remove_task(task_id: u64) {
    interrupts::without_interrupts(|| {
        kprintln!("[kernel] scheduler: Removing task ID {}.", task_id);
        if let Some(task) = TASKS.lock().remove(&task_id) {
            release_task_resources(&task);
        }
        // Also remove from run queue if it's there (optional for simple stub)
        RUN_QUEUE.lock().retain(|&id| id != task_id);
    });
}

/// Terminates a task and cleans up scheduler state and memory resources.
pub fn terminate_task(task_id: u64) {
    interrupts::without_interrupts(|| {
        let task_to_release = {
            let mut tasks = TASKS.lock();
            if let Some(task) = tasks.get_mut(&task_id) {
                task.state = TaskState::Exited;
            }
            tasks.remove(&task_id)
        };

        RUN_QUEUE.lock().retain(|&id| id != task_id);

        if let Some(task) = task_to_release {
            kprintln!(
                "[kernel] scheduler: Task '{}' (ID: {}) exited.",
                task.name,
                task.id
            );
            release_task_resources(&task);
        }
    });
}

/// Terminates the currently running task and dispatches the next runnable one.
pub fn terminate_current_task() {
    let current_task_id = *CURRENT_TASK_ID.lock();
    terminate_task(current_task_id);
    schedule();
}

/// Removes all non-kernel tasks from the scheduler.
pub fn kill_all() {
    interrupts::without_interrupts(|| {
        let current_id = *CURRENT_TASK_ID.lock();
        let mut tasks = TASKS.lock();
        let to_remove = tasks
            .keys()
            .copied()
            .filter(|task_id| *task_id != 0)
            .collect::<alloc::vec::Vec<_>>();

        for task_id in &to_remove {
            if let Some(task) = tasks.remove(task_id) {
                release_task_resources(&task);
            }
        }

        RUN_QUEUE.lock().retain(|task_id| *task_id == 0);
        if current_id != 0 {
            *CURRENT_TASK_ID.lock() = 0;
        }
    });
}

/// Marks that the current CPU should perform a scheduling decision soon.
///
/// This is intended to be called from interrupt context (e.g. timer IRQ),
/// where taking scheduler locks directly can deadlock.
#[inline]
pub fn request_reschedule_from_irq() {
    // Invariant: IRQ handlers only set this flag; they never clear it.
    // The main scheduler loop is the single clear point via
    // `take_reschedule_request`, which makes reschedule intent observable and
    // deterministic for diagnostics.
    RESCHEDULE_REQUESTED.store(true, Ordering::Release);
}

/// Marks that the scheduler should run on the next safe boundary.
#[inline]
pub fn request_reschedule() {
    RESCHEDULE_REQUESTED.store(true, Ordering::Release);
}

/// Returns whether a reschedule was requested and clears the request flag.
#[inline]
pub fn take_reschedule_request() -> bool {
    RESCHEDULE_REQUESTED.swap(false, Ordering::AcqRel)
}

/// Returns whether a reschedule is currently pending without clearing it.
#[inline]
pub fn reschedule_requested() -> bool {
    RESCHEDULE_REQUESTED.load(Ordering::Acquire)
}

fn release_task_resources(task: &TaskControlBlock) {
    if let Some(kernel_stack) = task.kernel_stack_base {
        PageAllocator::deallocate_page(kernel_stack);
    }

    if let Some(user_stack) = task.user_stack_base {
        PageAllocator::deallocate_page(user_stack);
    }

    for page in &task.address_space_pages {
        PageAllocator::deallocate_page(*page);
    }
}

/// Blocks the current task and adds it back to the queue as 'Blocked'.
/// In a real system, this would involve saving context and performing a context switch.
pub fn block_current_task() {
    interrupts::without_interrupts(|| {
        let current_id = *CURRENT_TASK_ID.lock();

        let mut tasks = TASKS.lock();
        if let Some(task) = tasks.get_mut(&current_id) {
            task.state = TaskState::Blocked;
            kprintln!(
                "[kernel] scheduler: Task '{}' (ID: {}) blocked.",
                task.name,
                current_id
            );
        }
    });

    // Trigger a schedule immediately if blocking.
    schedule();
}

/// Marks a blocked task as ready and adds it to the run queue.
pub fn unblock_task(task_id: u64) {
    interrupts::without_interrupts(|| {
        let mut tasks = TASKS.lock();
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.state == TaskState::Blocked {
                task.state = TaskState::Ready;
                RUN_QUEUE.lock().push_back(task_id);
                kprintln!(
                    "[kernel] scheduler: Task '{}' (ID: {}) unblocked.",
                    task.name,
                    task_id
                );
            }
        }
    });
}

/// Simulates a context switch to the next ready task (round-robin).
pub fn schedule() {
    interrupts::without_interrupts(|| {
        let mut run_queue = RUN_QUEUE.lock();
        let mut current_id_guard = CURRENT_TASK_ID.lock();
        let mut tasks = TASKS.lock();

        let old_task_id = *current_id_guard;

        // If the old task is still running, set its state to Ready and put it back in the queue.
        // (Unless it explicitly blocked itself)
        if let Some(old_task) = tasks.get_mut(&old_task_id) {
            if old_task.state == TaskState::Running {
                old_task.state = TaskState::Ready;
                run_queue.push_back(old_task_id);
            }
        }

        // Get the next task from the run queue.
        while let Some(next_task_id) = run_queue.pop_front() {
            if let Some(next_task) = tasks.get_mut(&next_task_id) {
                next_task.state = TaskState::Running;
                *current_id_guard = next_task_id;
                let next_context = next_task.context;
                let next_address_space = next_task.address_space_root;
                kprintln!(
                    "[kernel] scheduler: Context switch: from {} to {}.",
                    old_task_id,
                    next_task_id
                );
                restore_task_context(next_context, next_address_space);
                return;
            }

            kprintln!(
                "[kernel] scheduler: ERROR: Next task ID {} not found in TASKS. Skipping.",
                next_task_id
            );
        }

        // No task was runnable; keep current task active as idle fallback.
        if let Some(old_task) = tasks.get_mut(&old_task_id) {
            old_task.state = TaskState::Running;
        }
        *current_id_guard = old_task_id;
        kprintln!("[kernel] scheduler: Run queue empty. Continuing task {}.", old_task_id);
    });
}

/// Saves a hardware trap-frame snapshot into the currently running task.
pub fn save_current_context(snapshot: Context) {
    let current_id = *CURRENT_TASK_ID.lock();
    if let Some(task) = TASKS.lock().get_mut(&current_id) {
        task.context = snapshot;
    }
}

/// Returns the context snapshot for a task if present.
pub fn get_task_context(task_id: u64) -> Option<Context> {
    TASKS.lock().get(&task_id).map(|task| task.context)
}


/// Returns the ID of the currently executing task.
pub fn get_current_task_id() -> u64 {
    *CURRENT_TASK_ID.lock()
}


/// Returns the total number of registered tasks.
pub fn task_count() -> usize {
    TASKS.lock().len()
}

/// Returns the number of runnable tasks currently queued.
pub fn runnable_count() -> usize {
    RUN_QUEUE.lock().len()
}

/// Allocates a fresh task identifier by scanning current scheduler state.
pub fn allocate_task_id() -> u64 {
    TASKS
        .lock()
        .keys()
        .copied()
        .max()
        .map(|id| id.saturating_add(1))
        .unwrap_or(1)
}

/// Returns the detected number of logical CPU cores.
///
/// The value is clamped to at least 1 to provide a safe fallback on platforms
/// where CPUID cannot be queried.
pub fn available_cpu_cores() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        let logical_cores = unsafe { core::arch::x86_64::__cpuid(1) }.ebx >> 16 & 0xff;
        if logical_cores == 0 { 1 } else { logical_cores }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        1
    }
}

/// Returns the global AI budget expressed in millicores.
pub fn ai_cpu_budget_millicores() -> u32 {
    available_cpu_cores().saturating_mul(MILLICORES_PER_CORE)
}

/// Returns a cloned task by id if it exists.
pub fn get_task(task_id: u64) -> Option<TaskControlBlock> {
    get_task_by_id(task_id)
}

/// Returns a cloned task by id if it exists.
pub fn get_task_by_id(task_id: u64) -> Option<TaskControlBlock> {
    TASKS.lock().get(&task_id).cloned()
}

/// Checks whether a task holds a specific capability.
pub fn task_has_capability(task_id: u64, cap: Capability) -> bool {
    TASKS
        .lock()
        .get(&task_id)
        .map(|task| task.capabilities.iter().any(|c| *c == cap))
        .unwrap_or(false)
}

/// Grants a capability to a task when not already present.
pub fn grant_capability(task_id: u64, cap: Capability) -> bool {
    let mut tasks = TASKS.lock();
    if let Some(task) = tasks.get_mut(&task_id) {
        if !task.capabilities.iter().any(|c| *c == cap) {
            task.capabilities.push(cap);
            return true;
        }
    }
    false
}

/// Copies all capabilities from `from_task_id` into `to_task_id`.
/// Existing capabilities in destination are preserved and duplicates are avoided.
pub fn inherit_capabilities(from_task_id: u64, to_task_id: u64) -> bool {
    let source_caps = {
        let tasks = TASKS.lock();
        let Some(source) = tasks.get(&from_task_id) else {
            return false;
        };
        source.capabilities.clone()
    };

    let mut tasks = TASKS.lock();
    if let Some(target) = tasks.get_mut(&to_task_id) {
        for cap in source_caps {
            if !target.capabilities.iter().any(|c| *c == cap) {
                target.capabilities.push(cap);
            }
        }
        return true;
    }

    false
}

fn restore_task_context(context: Context, address_space_root: u64) {
    // CR3 reload / low-level register restore lives in architecture assembly glue.
    // For now we expose deterministic observability for scheduler decisions.
    kprintln!(
        "[kernel] scheduler: restore rip={:#x}, rsp={:#x}, rflags={:#x}, as_root={:#x}.",
        context.rip,
        context.rsp,
        context.rflags,
        address_space_root
    );
}

/// Returns a cloned `TaskControlBlock` for the currently executing task.
pub fn get_current_task_tcb() -> TaskControlBlock {
    let current_id = *CURRENT_TASK_ID.lock();
    TASKS.lock().get(&current_id).cloned().unwrap_or_else(|| {
        // Fallback for when current_id might not be in TASKS (e.g., during early boot)
        kprintln!(
            "[kernel] scheduler: WARNING: Current task ID {} not found. Returning dummy task.",
            current_id
        );
        TaskControlBlock::new(
            current_id,
            alloc::string::String::from("dummy_task"),
            alloc::vec![crate::caps::Capability::LogWrite], // Grant minimal caps
        )
    })
}

#[cfg(test)]
pub fn reset_for_tests() {
    TASKS.lock().clear();
    RUN_QUEUE.lock().clear();
    *CURRENT_TASK_ID.lock() = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caps::Capability;
    use alloc::string::String;

    #[test]
    fn terminate_current_task_removes_it_and_switches_to_next() {
        reset_for_tests();

        let mut current_task =
            TaskControlBlock::new(1, String::from("current"), vec![Capability::LogWrite]);
        current_task.state = TaskState::Running;

        let next_task = TaskControlBlock::new(2, String::from("next"), vec![Capability::LogWrite]);

        TASKS.lock().insert(current_task.id, current_task);
        TASKS.lock().insert(next_task.id, next_task);
        *CURRENT_TASK_ID.lock() = 1;
        RUN_QUEUE.lock().push_back(2);

        terminate_current_task();

        assert!(!TASKS.lock().contains_key(&1));
        assert_eq!(*CURRENT_TASK_ID.lock(), 2);
        assert_eq!(
            TASKS.lock().get(&2).map(|task| task.state),
            Some(TaskState::Running)
        );
    }

    #[test]
    fn terminate_task_cleans_queue_entries() {
        reset_for_tests();

        let task = TaskControlBlock::new(11, String::from("worker"), vec![Capability::LogWrite]);
        TASKS.lock().insert(task.id, task);

        {
            let mut queue = RUN_QUEUE.lock();
            queue.push_back(11);
            queue.push_back(42);
            queue.push_back(11);
        }

        terminate_task(11);

        assert!(!TASKS.lock().contains_key(&11));
        assert_eq!(
            RUN_QUEUE
                .lock()
                .iter()
                .copied()
                .collect::<alloc::vec::Vec<_>>(),
            vec![42]
        );
    }
}
