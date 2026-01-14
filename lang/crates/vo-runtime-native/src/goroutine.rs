//! Goroutine scheduler for JIT-compiled code.
//!
//! TODO: Implement actual scheduler

use std::cell::RefCell;

thread_local! {
    static CURRENT_SCHEDULER: RefCell<Option<*const Scheduler>> = RefCell::new(None);
}

/// Goroutine scheduler.
pub struct Scheduler {
    // TODO: Implement scheduler state
}

impl Scheduler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Set the current thread's scheduler.
pub fn set_current_scheduler(scheduler: &Scheduler) {
    CURRENT_SCHEDULER.with(|s| {
        *s.borrow_mut() = Some(scheduler as *const Scheduler);
    });
}

/// Get the current thread's scheduler.
pub fn current_scheduler() -> Option<&'static Scheduler> {
    CURRENT_SCHEDULER.with(|s| {
        s.borrow().map(|ptr| unsafe { &*ptr })
    })
}
