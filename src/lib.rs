#![warn(clippy::pedantic, clippy::nursery, clippy::unwrap_used)]
#![allow(
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

#[macro_use]
extern crate slog;

pub mod cursor;
pub mod drawing;
pub mod input_handler;
pub mod render;
pub mod shell;
pub mod state;
pub mod udev;
#[cfg(feature = "xwayland")]
pub mod xwayland;

pub use state::{CalloopData, ClientState, LimeWmState};
