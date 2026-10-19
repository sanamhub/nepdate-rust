//! One Slint testing backend per test thread. `init_no_event_loop` sets a platform
//! per thread, and Slint panics if it is set twice on one thread, so the flag is thread-local.
//! `cargo nextest` gives each test its own process; `cargo test` may reuse a thread, which this
//! guard tolerates.

use std::cell::Cell;

thread_local! {
    static READY: Cell<bool> = const { Cell::new(false) };
}

/// Initializes the headless Slint testing backend once on the current thread.
pub(crate) fn init_platform() {
    READY.with(|ready| {
        if !ready.get() {
            i_slint_backend_testing::init_no_event_loop();
            ready.set(true);
        }
    });
}
