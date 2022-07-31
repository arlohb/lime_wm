use slog::{o, Drain};

fn main() {
    // A logger facility, here we use the terminal here
    let log = if std::env::var("LIME_WM_MUTEX_LOG").is_ok() {
        slog::Logger::root(
            std::sync::Mutex::new(slog_term::term_full().fuse()).fuse(),
            o!(),
        )
    } else {
        slog::Logger::root(
            slog_async::Async::default(slog_term::term_full().fuse()).fuse(),
            o!(),
        )
    };

    let _guard = slog_scope::set_global_logger(log.clone());
    slog_stdlog::init().expect("Could not setup log backend");

    slog::info!(log, "Starting lime_wm on a tty using udev");
    lime_wm::udev::run_udev(&log);
}
