//! Zorto — a fast static site generator with executable code blocks.
//!
//! # Overview
//!
//! Zorto builds static websites from Markdown content with TOML frontmatter,
//! Tera templates, and optional SCSS compilation. Its distinguishing feature is
//! *executable code blocks*: fenced blocks marked `{python}`, `{bash}`, or
//! `{sh}` are executed at build time and their output is rendered inline.
//!
//! # Quick start
//!
//! ```bash
//! zorto init my-site
//! cd my-site
//! zorto build
//! zorto preview
//! ```
//!
//! # Library usage
//!
//! The primary entry point for programmatic use is [`site::Site`]:
//!
//! ```no_run
//! use std::path::Path;
//! use zorto::site::Site;
//!
//! let root = Path::new("my-site");
//! let output = root.join("public");
//! let mut site = Site::load(root, &output, false)?;
//! site.build()?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod config;
pub mod content;
pub mod site;

mod cli;
pub(crate) mod execute;
pub(crate) mod links;
pub(crate) mod markdown;
pub(crate) mod sass;
pub(crate) mod shortcodes;
pub(crate) mod templates;

pub(crate) mod serve;

pub use cli::run;
