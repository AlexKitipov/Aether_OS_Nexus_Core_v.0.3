//! Timer IRQ handler.

use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::{pic, IRQ_TIMER};

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    crate::timer::tick();
    if let Some(Some(handler)) = crate::device::with_manager(|m| m.irq_handler(IRQ_TIMER)) {
        handler.handle_irq();
    }

    unsafe {
        // SAFETY: We are running in the timer IRQ context, so acknowledging
        // the corresponding PIC line is required to re-enable future timer
        // interrupts.
        //
        // Scheduling is intentionally deferred to the main loop and gated by
        // `timer::tick()`'s quantum policy, preventing a context-switch request
        // on every single PIT interrupt.
        pic::end_of_interrupt(IRQ_TIMER);
    }
}
