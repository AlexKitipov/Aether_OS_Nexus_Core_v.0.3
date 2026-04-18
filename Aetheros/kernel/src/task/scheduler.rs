#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::collections::{BTreeMap, VecDeque};

use spin::Mutex;

use crate::kprintln;
use crate::task::tcb::{TaskContext, TaskControlBlock, TaskState};

/// The run queue holds task IDs of tasks that are ready to be scheduled.
static RUN_QUEUE: Mutex<VecDeque<u64>> = Mutex::new(VecDeque::new());

/// A map of all active tasks, indexed by their ID.
static TASKS: Mutex<BTreeMap<u64, TaskControlBlock>> = Mutex::new(BTreeMap::new());

/// The ID of the currently executing task.
static CURRENT_TASK_ID: Mutex<u64> = Mutex::new(0); // Starts with kernel as task 0

/// Initializes the scheduler, setting up necessary data structures.
pub fn init() {
    kprintln!("[kernel] scheduler: Initializing preemptive scheduler...");

    let mut kernel_task = TaskControlBlock::new(
        0,
        alloc::string::String::from("kernel"),
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
        ],
    )
    .with_timeslice(1);
    kernel_task.state = TaskState::Running;

    {
        let mut tasks = TASKS.lock();
        tasks.insert(kernel_task.id, kernel_task.clone());
    }

    *CURRENT_TASK_ID.lock() = kernel_task.id;

    kprintln!("[kernel] scheduler: Initialized kernel task (ID: 0).");
}

/// Adds a new task to the scheduler's management.
pub fn add_task(mut task: TaskControlBlock) {
    let task_id = task.id;
    task.state = TaskState::Ready;
    task.reset_timeslice();

    kprintln!(
        "[kernel] scheduler: Adding task '{}' (ID: {}, slice={} ticks).",
        task.name,
        task_id,
        task.timeslice_ticks
    );

    TASKS.lock().insert(task_id, task);
    RUN_QUEUE.lock().push_back(task_id);
}

/// Removes a task from the scheduler's management.
pub fn remove_task(task_id: u64) {
    kprintln!("[kernel] scheduler: Removing task ID {}.", task_id);
    TASKS.lock().remove(&task_id);
    RUN_QUEUE.lock().retain(|&id| id != task_id);
}

/// Marks the current task as exited and immediately schedules the next runnable task.
pub fn exit_current_task() {
    let current_id = *CURRENT_TASK_ID.lock();

    {
        let mut tasks = TASKS.lock();
        if let Some(task) = tasks.get_mut(&current_id) {
            task.state = TaskState::Exited;
            kprintln!(
                "[kernel] scheduler: Task '{}' (ID: {}) exited.",
                task.name,
                current_id
            );
        }
    }

    schedule();
}

/// Blocks the current task and schedules another task.
pub fn block_current_task() {
    let current_id = *CURRENT_TASK_ID.lock();

    {
        let mut tasks = TASKS.lock();
        if let Some(task) = tasks.get_mut(&current_id) {
            task.state = TaskState::Blocked;
            task.reset_timeslice();
            kprintln!(
                "[kernel] scheduler: Task '{}' (ID: {}) blocked.",
                task.name,
                current_id
            );
        }
    }

    schedule();
}

/// Marks a blocked task as ready and adds it to the run queue.
pub fn unblock_task(task_id: u64) {
    let mut tasks = TASKS.lock();
    if let Some(task) = tasks.get_mut(&task_id) {
        if task.state == TaskState::Blocked {
            task.state = TaskState::Ready;
            task.reset_timeslice();
            RUN_QUEUE.lock().push_back(task_id);
            kprintln!(
                "[kernel] scheduler: Task '{}' (ID: {}) unblocked.",
                task.name,
                task_id
            );
        }
    }
}

/// Voluntary yield by current task.
pub fn yield_current_task() {
    schedule();
}

/// Timer-driven preemption entry point.
///
/// Called from the timer interrupt path. It advances current task runtime,
/// and triggers a context switch when the task's timeslice expires.
pub fn on_timer_tick() {
    let current_id = *CURRENT_TASK_ID.lock();
    let mut should_preempt = false;

    {
        let mut tasks = TASKS.lock();
        if let Some(task) = tasks.get_mut(&current_id) {
            if task.state == TaskState::Running {
                task.consumed_ticks = task.consumed_ticks.saturating_add(1);
                if task.consumed_ticks >= task.timeslice_ticks {
                    should_preempt = true;
                }
            }
        }
    }

    if should_preempt {
        kprintln!(
            "[kernel] scheduler: Timeslice expired for task {}. Preempting.",
            current_id
        );
        schedule();
    }
}

/// Save simulated context for a task.
fn save_context(task: &mut TaskControlBlock) {
    // This is architecture-neutral scaffolding. A real implementation would
    // capture register state from the interrupt frame.
    task.context.rip = task.context.rip.wrapping_add(1);
    task.context.rsp = task.context.rsp.wrapping_sub(8);
    task.context.regs[0] = task.context.regs[0].wrapping_add(1);
}

/// Restore simulated context for a task.
fn restore_context(_task: &TaskControlBlock) {
    // Real implementation would restore CPU registers and return from interrupt.
}

/// Returns the ID of the next runnable task.
fn pop_next_runnable(run_queue: &mut VecDeque<u64>, tasks: &BTreeMap<u64, TaskControlBlock>) -> Option<u64> {
    while let Some(next_task_id) = run_queue.pop_front() {
        if let Some(next_task) = tasks.get(&next_task_id) {
            if next_task.state == TaskState::Ready {
                return Some(next_task_id);
            }
        }
    }

    None
}

/// Performs a context switch to the next ready task.
pub fn schedule() {
    let mut run_queue = RUN_QUEUE.lock();
    let mut current_id_guard = CURRENT_TASK_ID.lock();
    let mut tasks = TASKS.lock();

    let old_task_id = *current_id_guard;

    if let Some(old_task) = tasks.get_mut(&old_task_id) {
        if old_task.state == TaskState::Running {
            save_context(old_task);
            old_task.state = TaskState::Ready;
            old_task.reset_timeslice();
            run_queue.push_back(old_task_id);
        }
    }

    if let Some(next_task_id) = pop_next_runnable(&mut run_queue, &tasks) {
        if let Some(next_task) = tasks.get_mut(&next_task_id) {
            next_task.state = TaskState::Running;
            next_task.switch_count = next_task.switch_count.saturating_add(1);
            *current_id_guard = next_task_id;
            restore_context(next_task);
            kprintln!(
                "[kernel] scheduler: Context switch: {} -> {} (switches={}).",
                old_task_id,
                next_task_id,
                next_task.switch_count
            );
            return;
        }
    }

    // Nothing ready: keep current task if possible, else fallback to kernel task.
    if let Some(current) = tasks.get_mut(&old_task_id) {
        if current.state != TaskState::Exited && current.state != TaskState::Blocked {
            current.state = TaskState::Running;
            *current_id_guard = old_task_id;
            return;
        }
    }

    if let Some(kernel_task) = tasks.get_mut(&0) {
        kernel_task.state = TaskState::Running;
        *current_id_guard = 0;
        kprintln!("[kernel] scheduler: No runnable tasks; switching to idle kernel task.");
    } else {
        kprintln!("[kernel] scheduler: ERROR: No runnable task and no kernel task.");
    }
}

/// Returns a cloned `TaskControlBlock` for the currently executing task.
pub fn get_current_task_tcb() -> TaskControlBlock {
    let current_id = *CURRENT_TASK_ID.lock();
    TASKS.lock().get(&current_id).cloned().unwrap_or_else(|| {
        kprintln!(
            "[kernel] scheduler: WARNING: Current task ID {} not found. Returning dummy task.",
            current_id
        );
        TaskControlBlock::new(
            current_id,
            alloc::string::String::from("dummy_task"),
            alloc::vec![crate::caps::Capability::LogWrite],
        )
    })
}

/// Called by architecture interrupt handlers when an interrupt frame is available.
///
/// For now this stores the frame-independent parts in a neutral context.
pub fn capture_interrupt_context(rip: u64, rsp: u64, rflags: u64) {
    let current_id = *CURRENT_TASK_ID.lock();
    let mut tasks = TASKS.lock();

    if let Some(task) = tasks.get_mut(&current_id) {
        task.context = TaskContext {
            rip,
            rsp,
            rflags,
            regs: task.context.regs,
        };
    }
}
