//! Timer IRQ handler.

use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::{pic, IRQ_TIMER};

pub extern "x86-interrupt" fn handler(mut stack_frame: InterruptStackFrame) {
    crate::timer::tick();
    if let Some(Some(handler)) = crate::device::with_manager(|m| m.irq_handler(IRQ_TIMER)) {
        handler.handle_irq();
    }

    unsafe {
        // SAFETY: We are running in the timer IRQ context, so acknowledging
        // the corresponding PIC line is required to re-enable future timer
        // interrupts.
        //
        // Acknowledge the timer before any optional IRQ-exit dispatch. The
        // scheduler path below is try-lock based and may leave the reschedule
        // request deferred if any scheduler structure is busy.
        pic::end_of_interrupt(IRQ_TIMER);
    }

    // Keep deferred rescheduling as the first line of defense: `timer::tick()`
    // only accounts using IRQ-safe atomics and sets a request flag. Once the
    // handler is about to return, try to turn that request into a real
    // preemption by saving/restoring the interrupt trap frame. If scheduler
    // locks are not immediately available, this returns without spinning and
    // the main loop will dispatch at the next safe boundary.
    let _ = crate::task::scheduler::try_dispatch_from_irq_exit(&mut stack_frame);
}
