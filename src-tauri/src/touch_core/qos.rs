//! Thread QoS for the latency-critical hot path.
//!
//! The app is an accessory app and is usually not frontmost, so threads
//! created at runtime inherit a background/utility QoS. macOS then schedules
//! them on efficiency cores and preempts them while the game is running,
//! which turns a few milliseconds of detection work into tens of milliseconds.
//!
//! `pthread_set_qos_class_self_np` raises the current thread for the action,
//! the capture thread, and (by inheritance) the event-tap thread.

const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;

unsafe extern "C" {
    fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
}

/// Raise the calling thread to user-interactive QoS.
pub fn promote_current_thread() {
    // SAFETY: the function only mutates the calling thread's QoS class.
    let rc = unsafe { pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0) };
    if rc != 0 {
        log::warn!("Failed to promote thread QoS: {rc}");
    }
}
