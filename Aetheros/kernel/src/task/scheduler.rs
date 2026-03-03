#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::collections::{BTreeMap, VecDeque};
use spin::Mutex;

use crate::kprintln;
use crate::memory::page_allocator::PageAllocator;
use crate::task::tcb::{TaskControlBlock, TaskState};

/// The run queue holds task IDs of tasks that are ready to be scheduled.
/// This uses a simple `VecDeque` for a round-robin like behavior.
static RUN_QUEUE: Mutex<VecDeque<u64>> = Mutex::new(VecDeque::new());

/// A map of all active tasks, indexed by their ID.
static TASKS: Mutex<BTreeMap<u64, TaskControlBlock>> = Mutex::new(BTreeMap::new());

/// The ID of the currently executing task.
static CURRENT_TASK_ID: Mutex<u64> = Mutex::new(0); // Starts with kernel as task 0

/// Initializes the scheduler, setting up necessary data structures.
pub fn init() {
    kprintln!("[kernel] scheduler: Initializing...");

    // Create a dummy kernel task and add it to the task list.
    // In a real system, the initial kernel thread would be set up differently.
    let kernel_task = TaskControlBlock::new(
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
        ],
    );

    {
        let mut tasks = TASKS.lock();
        tasks.insert(kernel_task.id, kernel_task.clone());
    }

    *CURRENT_TASK_ID.lock() = kernel_task.id;

    kprintln!("[kernel] scheduler: Initialized kernel task (ID: 0).");
}

/// Adds a new task to the scheduler's management.
pub fn add_task(task: TaskControlBlock) {
    let task_id = task.id;
    kprintln!(
        "[kernel] scheduler: Adding task '{}' (ID: {}).",
        task.name,
        task_id
    );
    TASKS.lock().insert(task_id, task);
    RUN_QUEUE.lock().push_back(task_id);
}

/// Removes a task from the scheduler's management.
pub fn remove_task(task_id: u64) {
    kprintln!("[kernel] scheduler: Removing task ID {}.", task_id);
    if let Some(task) = TASKS.lock().remove(&task_id) {
        release_task_resources(&task);
    }
    // Also remove from run queue if it's there (optional for simple stub)
    RUN_QUEUE.lock().retain(|&id| id != task_id);
}

/// Terminates a task and cleans up scheduler state and memory resources.
pub fn terminate_task(task_id: u64) {
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
}

/// Terminates the currently running task and dispatches the next runnable one.
pub fn terminate_current_task() {
    let current_task_id = *CURRENT_TASK_ID.lock();
    terminate_task(current_task_id);
    schedule();
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
    let current_id = *CURRENT_TASK_ID.lock();

    {
        let mut tasks = TASKS.lock();
        if let Some(task) = tasks.get_mut(&current_id) {
            task.state = TaskState::Blocked;
            kprintln!(
                "[kernel] scheduler: Task '{}' (ID: {}) blocked.",
                task.name,
                current_id
            );
        }
    }

    // Trigger a schedule immediately if blocking.
    schedule();
}

/// Marks a blocked task as ready and adds it to the run queue.
pub fn unblock_task(task_id: u64) {
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
}

/// Simulates a context switch to the next ready task (round-robin).
pub fn schedule() {
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
            kprintln!(
                "[kernel] scheduler: Context switch: from {} to {}.",
                old_task_id,
                next_task_id
            );
            // In a real scheduler, actual CPU context switch would occur here.
            return;
        }

        kprintln!(
            "[kernel] scheduler: ERROR: Next task ID {} not found in TASKS. Skipping.",
            next_task_id
        );
    }

    // No tasks in run queue. System might idle or panic.
    kprintln!("[kernel] scheduler: Run queue empty. Idling.");
    // In a real system, this would ideally lead to an idle loop or halt.
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
