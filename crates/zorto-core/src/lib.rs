//! Zorto core — the library powering the zorto static site generator.
//!
//! # Overview
//!
//! Zorto builds static websites from Markdown content with TOML frontmatter,
//! Tera templates, and optional SCSS compilation. Its distinguishing feature is
//! *executable code blocks*: fenced blocks marked `{python}`, `{bash}`, or
//! `{sh}` are executed at build time and their output is rendered inline.
//!
//! # Library usage
//!
//! The primary entry point for programmatic use is [`site::Site`]:
//!
//! ```no_run
//! use std::path::Path;
//! use zorto_core::site::Site;
//!
//! let root = Path::new("my-site");
//! let output = root.join("public");
//! let mut site = Site::load(root, &output, false)?;
//! site.build()?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod config;
pub mod content;
pub mod markdown;
pub mod site;
pub mod themes;

pub(crate) mod execute;
pub(crate) mod links;
pub mod lint;
pub(crate) mod sass;
pub mod search;
pub(crate) mod shortcodes;
pub(crate) mod templates;
