//! Built-in themes for Zorto.
//!
//! Themes provide default templates and SCSS that ship with the Zorto binary.
//! A site can select a theme via `theme = "zorto"` in `config.toml`. Local
//! `templates/` and `sass/` files always override theme defaults.
//!
//! Each theme is gated behind a Cargo feature (`theme-zorto`, `theme-dkdc`,
//! etc.). All are enabled by default. In Python builds all themes are always
//! included.
//!
//! Every theme supports both light and dark mode via CSS variables. The
//! `:root` selector defines dark-mode defaults, and `[data-theme="light"]`
//! overrides for light mode. The light/dark toggle in the navbar works
//! identically across all themes.

/// A built-in theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Theme {
    /// Blue/green dark-default theme with animations. The zorto brand theme.
    #[cfg(feature = "theme-zorto")]
    Zorto,
    /// Violet/cyan dark-default theme with animations. The dkdc brand theme.
    #[cfg(feature = "theme-dkdc")]
    Dkdc,
    /// Clean blue theme. No animations.
    #[cfg(feature = "theme-default")]
    Default,
    /// Orange/amber dark-default theme. Warm and cozy.
    #[cfg(feature = "theme-ember")]
    Ember,
    /// Green/lime dark-default theme. Natural and earthy.
    #[cfg(feature = "theme-forest")]
    Forest,
    /// Teal/blue dark-default theme. Calm and professional.
    #[cfg(feature = "theme-ocean")]
    Ocean,
    /// Pink/purple dark-default theme. Soft and modern.
    #[cfg(feature = "theme-rose")]
    Rose,
    /// Neutral monochrome dark-default theme. Minimal and clean.
    #[cfg(feature = "theme-slate")]
    Slate,
    /// Navy/silver dark-default theme. Corporate and professional.
    #[cfg(feature = "theme-midnight")]
    Midnight,
    /// Red/orange warm dark-default theme. Creative and bold.
    #[cfg(feature = "theme-sunset")]
    Sunset,
    /// Green/cyan dark-default theme. Modern and minimalist.
    #[cfg(feature = "theme-mint")]
    Mint,
    /// Purple/lavender dark-default theme. Elegant and artistic.
    #[cfg(feature = "theme-plum")]
    Plum,
    /// Beige/brown dark-default theme. Warm and readable.
    #[cfg(feature = "theme-sand")]
    Sand,
    /// Ice blue/white dark-default theme. Clean and scientific.
    #[cfg(feature = "theme-arctic")]
    Arctic,
    /// Neon green/yellow dark-default theme. Tech and energetic.
    #[cfg(feature = "theme-lime")]
    Lime,
    /// Dark grey/silver dark-default theme. Technical and code-focused.
    #[cfg(feature = "theme-charcoal")]
    Charcoal,
}

impl Theme {
    /// Parse a theme name from a string.
    ///
    /// Returns `None` if the name is unknown or the corresponding feature is
    /// not enabled.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            #[cfg(feature = "theme-zorto")]
            "zorto" => Some(Self::Zorto),
            #[cfg(feature = "theme-dkdc")]
            "dkdc" => Some(Self::Dkdc),
            #[cfg(feature = "theme-default")]
            "default" => Some(Self::Default),
            #[cfg(feature = "theme-ember")]
            "ember" => Some(Self::Ember),
            #[cfg(feature = "theme-forest")]
            "forest" => Some(Self::Forest),
            #[cfg(feature = "theme-ocean")]
            "ocean" => Some(Self::Ocean),
            #[cfg(feature = "theme-rose")]
            "rose" => Some(Self::Rose),
            #[cfg(feature = "theme-slate")]
            "slate" => Some(Self::Slate),
            #[cfg(feature = "theme-midnight")]
            "midnight" => Some(Self::Midnight),
            #[cfg(feature = "theme-sunset")]
            "sunset" => Some(Self::Sunset),
            #[cfg(feature = "theme-mint")]
            "mint" => Some(Self::Mint),
            #[cfg(feature = "theme-plum")]
            "plum" => Some(Self::Plum),
            #[cfg(feature = "theme-sand")]
            "sand" => Some(Self::Sand),
            #[cfg(feature = "theme-arctic")]
            "arctic" => Some(Self::Arctic),
            #[cfg(feature = "theme-lime")]
            "lime" => Some(Self::Lime),
            #[cfg(feature = "theme-charcoal")]
            "charcoal" => Some(Self::Charcoal),
            _ => None,
        }
    }

    /// List all available theme names (only those whose features are enabled).
    #[allow(unused_mut, clippy::vec_init_then_push)]
    pub fn available() -> Vec<&'static str> {
        let mut names = Vec::new();
        #[cfg(feature = "theme-zorto")]
        names.push("zorto");
        #[cfg(feature = "theme-dkdc")]
        names.push("dkdc");
        #[cfg(feature = "theme-default")]
        names.push("default");
        #[cfg(feature = "theme-ember")]
        names.push("ember");
        #[cfg(feature = "theme-forest")]
        names.push("forest");
        #[cfg(feature = "theme-ocean")]
        names.push("ocean");
        #[cfg(feature = "theme-rose")]
        names.push("rose");
        #[cfg(feature = "theme-slate")]
        names.push("slate");
        #[cfg(feature = "theme-midnight")]
        names.push("midnight");
        #[cfg(feature = "theme-sunset")]
        names.push("sunset");
        #[cfg(feature = "theme-mint")]
        names.push("mint");
        #[cfg(feature = "theme-plum")]
        names.push("plum");
        #[cfg(feature = "theme-sand")]
        names.push("sand");
        #[cfg(feature = "theme-arctic")]
        names.push("arctic");
        #[cfg(feature = "theme-lime")]
        names.push("lime");
        #[cfg(feature = "theme-charcoal")]
        names.push("charcoal");
        names
    }

    /// Zorto templates shared by all themes.
    const BASE_HTML: (&'static str, &'static str) = (
        "base.html",
        include_str!("../themes/zorto/templates/base.html"),
    );
    const PAGE_HTML: (&'static str, &'static str) = (
        "page.html",
        include_str!("../themes/zorto/templates/page.html"),
    );
    const SECTION_HTML: (&'static str, &'static str) = (
        "section.html",
        include_str!("../themes/zorto/templates/section.html"),
    );
    const INDEX_HTML: (&'static str, &'static str) = (
        "index.html",
        include_str!("../themes/zorto/templates/index.html"),
    );
    const NOT_FOUND_HTML: (&'static str, &'static str) = (
        "404.html",
        include_str!("../themes/zorto/templates/404.html"),
    );
    const POST_MACRO_HTML: (&'static str, &'static str) = (
        "macros/post.html",
        include_str!("../themes/zorto/templates/macros/post.html"),
    );
    const TAGS_LIST_HTML: (&'static str, &'static str) = (
        "tags/list.html",
        include_str!("../themes/zorto/templates/tags/list.html"),
    );
    const TAGS_SINGLE_HTML: (&'static str, &'static str) = (
        "tags/single.html",
        include_str!("../themes/zorto/templates/tags/single.html"),
    );

    /// Get all template files for this theme as `(name, content)` pairs.
    ///
    /// Template names use forward slashes (e.g. `"macros/post.html"`).
    /// All themes share the same base templates from zorto. Themes only
    /// differ in CSS, not in HTML structure.
    #[allow(unreachable_patterns)]
    pub fn templates(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            Self::BASE_HTML,
            Self::PAGE_HTML,
            Self::SECTION_HTML,
            Self::INDEX_HTML,
            Self::NOT_FOUND_HTML,
            Self::POST_MACRO_HTML,
            Self::TAGS_LIST_HTML,
            Self::TAGS_SINGLE_HTML,
        ]
    }

    /// Shared SCSS partials included in every theme.
    const SHARED_STRUCTURE: &'static str = include_str!("../themes/shared/_structure.scss");
    const SHARED_COMPONENTS: &'static str = include_str!("../themes/shared/_components.scss");

    /// Get all SCSS files for this theme as `(filename, content)` pairs.
    ///
    /// Always includes shared partials (`_structure.scss`, `_components.scss`)
    /// alongside the theme's own `style.scss`.
    #[allow(unreachable_patterns)]
    pub fn scss(&self) -> Vec<(&'static str, &'static str)> {
        let mut files = vec![
            ("_structure.scss", Self::SHARED_STRUCTURE),
            ("_components.scss", Self::SHARED_COMPONENTS),
        ];
        match self {
            #[cfg(feature = "theme-zorto")]
            Self::Zorto => files.push((
                "style.scss",
                include_str!("../themes/zorto/sass/style.scss"),
            )),
            #[cfg(feature = "theme-dkdc")]
            Self::Dkdc => {
                files.push(("style.scss", include_str!("../themes/dkdc/sass/style.scss")))
            }
            #[cfg(feature = "theme-default")]
            Self::Default => files.push((
                "style.scss",
                include_str!("../themes/default/sass/style.scss"),
            )),
            #[cfg(feature = "theme-ember")]
            Self::Ember => files.push((
                "style.scss",
                include_str!("../themes/ember/sass/style.scss"),
            )),
            #[cfg(feature = "theme-forest")]
            Self::Forest => files.push((
                "style.scss",
                include_str!("../themes/forest/sass/style.scss"),
            )),
            #[cfg(feature = "theme-ocean")]
            Self::Ocean => files.push((
                "style.scss",
                include_str!("../themes/ocean/sass/style.scss"),
            )),
            #[cfg(feature = "theme-rose")]
            Self::Rose => {
                files.push(("style.scss", include_str!("../themes/rose/sass/style.scss")))
            }
            #[cfg(feature = "theme-slate")]
            Self::Slate => files.push((
                "style.scss",
                include_str!("../themes/slate/sass/style.scss"),
            )),
            #[cfg(feature = "theme-midnight")]
            Self::Midnight => files.push((
                "style.scss",
                include_str!("../themes/midnight/sass/style.scss"),
            )),
            #[cfg(feature = "theme-sunset")]
            Self::Sunset => files.push((
                "style.scss",
                include_str!("../themes/sunset/sass/style.scss"),
            )),
            #[cfg(feature = "theme-mint")]
            Self::Mint => {
                files.push(("style.scss", include_str!("../themes/mint/sass/style.scss")))
            }
            #[cfg(feature = "theme-plum")]
            Self::Plum => {
                files.push(("style.scss", include_str!("../themes/plum/sass/style.scss")))
            }
            #[cfg(feature = "theme-sand")]
            Self::Sand => {
                files.push(("style.scss", include_str!("../themes/sand/sass/style.scss")))
            }
            #[cfg(feature = "theme-arctic")]
            Self::Arctic => files.push((
                "style.scss",
                include_str!("../themes/arctic/sass/style.scss"),
            )),
            #[cfg(feature = "theme-lime")]
            Self::Lime => {
                files.push(("style.scss", include_str!("../themes/lime/sass/style.scss")))
            }
            #[cfg(feature = "theme-charcoal")]
            Self::Charcoal => files.push((
                "style.scss",
                include_str!("../themes/charcoal/sass/style.scss"),
            )),
            _ => {}
        }
        files
    }

    /// Return the theme's name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "theme-zorto")]
            Self::Zorto => "zorto",
            #[cfg(feature = "theme-dkdc")]
            Self::Dkdc => "dkdc",
            #[cfg(feature = "theme-default")]
            Self::Default => "default",
            #[cfg(feature = "theme-ember")]
            Self::Ember => "ember",
            #[cfg(feature = "theme-forest")]
            Self::Forest => "forest",
            #[cfg(feature = "theme-ocean")]
            Self::Ocean => "ocean",
            #[cfg(feature = "theme-rose")]
            Self::Rose => "rose",
            #[cfg(feature = "theme-slate")]
            Self::Slate => "slate",
            #[cfg(feature = "theme-midnight")]
            Self::Midnight => "midnight",
            #[cfg(feature = "theme-sunset")]
            Self::Sunset => "sunset",
            #[cfg(feature = "theme-mint")]
            Self::Mint => "mint",
            #[cfg(feature = "theme-plum")]
            Self::Plum => "plum",
            #[cfg(feature = "theme-sand")]
            Self::Sand => "sand",
            #[cfg(feature = "theme-arctic")]
            Self::Arctic => "arctic",
            #[cfg(feature = "theme-lime")]
            Self::Lime => "lime",
            #[cfg(feature = "theme-charcoal")]
            Self::Charcoal => "charcoal",
            #[allow(unreachable_patterns)]
            _ => "unknown",
        }
    }
}
