//! Built-in themes for Zorto.
//!
//! Themes provide default templates and SCSS that ship with the Zorto binary.
//! A site can select a theme via `theme = "dkdc"` in `config.toml`. Local
//! `templates/` and `sass/` files always override theme defaults.
//!
//! Each theme is gated behind a Cargo feature (`theme-dkdc`, `theme-light`,
//! `theme-dark`). All are enabled by default. In Python builds all themes are
//! always included.

/// A built-in theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Theme {
    /// Violet/cyan dark-default theme with animations. The dkdc brand theme.
    #[cfg(feature = "theme-dkdc")]
    Dkdc,
    /// Clean light-default theme. No animations.
    #[cfg(feature = "theme-light")]
    Light,
    /// Clean dark-default theme. No animations.
    #[cfg(feature = "theme-dark")]
    Dark,
}

impl Theme {
    /// Parse a theme name from a string.
    ///
    /// Returns `None` if the name is unknown or the corresponding feature is
    /// not enabled.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            #[cfg(feature = "theme-dkdc")]
            "dkdc" => Some(Self::Dkdc),
            #[cfg(feature = "theme-light")]
            "light" => Some(Self::Light),
            #[cfg(feature = "theme-dark")]
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// List all available theme names (only those whose features are enabled).
    #[allow(unused_mut, clippy::vec_init_then_push)]
    pub fn available() -> Vec<&'static str> {
        let mut names = Vec::new();
        #[cfg(feature = "theme-dkdc")]
        names.push("dkdc");
        #[cfg(feature = "theme-light")]
        names.push("light");
        #[cfg(feature = "theme-dark")]
        names.push("dark");
        names
    }

    /// Get all template files for this theme as `(name, content)` pairs.
    ///
    /// Template names use forward slashes (e.g. `"macros/post.html"`).
    #[allow(unreachable_patterns)]
    pub fn templates(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            #[cfg(feature = "theme-dkdc")]
            Self::Dkdc => vec![
                (
                    "base.html",
                    include_str!("../themes/dkdc/templates/base.html"),
                ),
                (
                    "page.html",
                    include_str!("../themes/dkdc/templates/page.html"),
                ),
                (
                    "section.html",
                    include_str!("../themes/dkdc/templates/section.html"),
                ),
                (
                    "index.html",
                    include_str!("../themes/dkdc/templates/index.html"),
                ),
                (
                    "404.html",
                    include_str!("../themes/dkdc/templates/404.html"),
                ),
                (
                    "macros/post.html",
                    include_str!("../themes/dkdc/templates/macros/post.html"),
                ),
            ],
            #[cfg(feature = "theme-light")]
            Self::Light => vec![
                (
                    "base.html",
                    include_str!("../themes/light/templates/base.html"),
                ),
                (
                    "page.html",
                    include_str!("../themes/light/templates/page.html"),
                ),
                (
                    "section.html",
                    include_str!("../themes/light/templates/section.html"),
                ),
                (
                    "index.html",
                    include_str!("../themes/light/templates/index.html"),
                ),
                (
                    "404.html",
                    include_str!("../themes/light/templates/404.html"),
                ),
                (
                    "macros/post.html",
                    include_str!("../themes/light/templates/macros/post.html"),
                ),
            ],
            #[cfg(feature = "theme-dark")]
            Self::Dark => vec![
                (
                    "base.html",
                    include_str!("../themes/dark/templates/base.html"),
                ),
                (
                    "page.html",
                    include_str!("../themes/dark/templates/page.html"),
                ),
                (
                    "section.html",
                    include_str!("../themes/dark/templates/section.html"),
                ),
                (
                    "index.html",
                    include_str!("../themes/dark/templates/index.html"),
                ),
                (
                    "404.html",
                    include_str!("../themes/dark/templates/404.html"),
                ),
                (
                    "macros/post.html",
                    include_str!("../themes/dark/templates/macros/post.html"),
                ),
            ],
            _ => vec![],
        }
    }

    /// Get all SCSS files for this theme as `(filename, content)` pairs.
    #[allow(unreachable_patterns)]
    pub fn scss(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            #[cfg(feature = "theme-dkdc")]
            Self::Dkdc => vec![("style.scss", include_str!("../themes/dkdc/sass/style.scss"))],
            #[cfg(feature = "theme-light")]
            Self::Light => vec![(
                "style.scss",
                include_str!("../themes/light/sass/style.scss"),
            )],
            #[cfg(feature = "theme-dark")]
            Self::Dark => vec![("style.scss", include_str!("../themes/dark/sass/style.scss"))],
            _ => vec![],
        }
    }
}
