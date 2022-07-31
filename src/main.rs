#![warn(clippy::pedantic, clippy::nursery, clippy::unwrap_used)]
#![allow(
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

pub mod cursor;
pub mod drawing;
pub mod input_handler;
pub mod render;
pub mod shell;
pub mod state;
pub use state::{CalloopData, ClientState, LimeWmState};
pub mod udev;
#[cfg(feature = "xwayland")]
pub mod xwayland;

use slog::Drain as _;

fn main() {
    // A logger facility, here we use the terminal here
    let log = if std::env::var("LIME_WM_MUTEX_LOG").is_ok() {
        slog::Logger::root(
            std::sync::Mutex::new(slog_term::term_full().fuse()).fuse(),
            slog::o!(),
        )
    } else {
        slog::Logger::root(
            slog_async::Async::default(slog_term::term_full().fuse()).fuse(),
            slog::o!(),
        )
    };

    let _guard = slog_scope::set_global_logger(log.clone());
    slog_stdlog::init().expect("Could not setup log backend");

    slog::info!(log, "Starting lime_wm on a tty using udev");
    crate::udev::run_udev(&log);
}
