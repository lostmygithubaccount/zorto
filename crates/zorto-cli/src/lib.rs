//! Zorto — the AI-native static site generator (SSG) with executable code blocks.
//!
//! This crate provides the CLI binary and the `run` entry point used by
//! the PyO3 Python bindings.

pub use zorto_core as core;
pub use zorto_core::config;
pub use zorto_core::content;
pub use zorto_core::site;

mod cli;
pub(crate) mod serve;
mod templates;

/// Run the zorto CLI with the given arguments.
///
/// This is the main entry point, equivalent to calling `zorto` on the command line.
/// Pass `std::env::args()` for normal use, or synthetic args for testing.
pub fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    cli::run(args)
}
