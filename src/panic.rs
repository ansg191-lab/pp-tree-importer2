use std::{
    backtrace::{Backtrace, BacktraceStatus},
    panic::PanicHookInfo,
};

pub fn panic_hook(panic_info: &PanicHookInfo) {
    let payload = panic_info.payload();

    #[allow(clippy::manual_map)]
    let payload = if let Some(s) = payload.downcast_ref::<&str>() {
        Some(*s)
    } else if let Some(s) = payload.downcast_ref::<String>() {
        Some(&**s)
    } else {
        None
    };

    let thread = std::thread::current();
    let name = thread.name().unwrap_or("<unnamed>");

    let task = tokio::task::try_id();

    let location = panic_info.location().map(|l| l.to_string());
    let backtrace = Backtrace::capture();
    let note = (backtrace.status() == BacktraceStatus::Disabled)
        .then_some("run with RUST_BACKTRACE=1 environment variable to display a backtrace");

    tracing::error!(
        panic.payload = payload,
        panic.location = location,
        panic.thread = name,
        panic.task = task.map(display),
        panic.backtrace = %backtrace,
        panic.note = note,
        "A panic occurred",
    );
}
