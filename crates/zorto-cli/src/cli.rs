use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::serve;
use crate::skill;
use crate::templates;
use zorto_core::site;

const DEFAULT_OUTPUT_DIR: &str = "public";
const DEFAULT_PREVIEW_PORT: &str = "1111";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
// Use a placeholder production URL by default so a user who accepts the
// default during `zorto init` doesn't end up with a sitemap.xml full of
// localhost links if they later run `zorto build`.
const DEFAULT_BASE_URL: &str = "https://example.com";
const DEFAULT_SITE_TITLE: &str = "My Site";

#[derive(Parser)]
#[command(
    name = "zorto",
    version,
    about = "The AI-native static site generator (SSG) with executable code blocks",
    after_help = "Quickstart:\n  \
        zorto init                  # set up a new site (interactive)\n  \
        zorto preview --open        # preview with live reload\n\
        \n\
        Docs: https://zorto.dev/docs"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Site root directory
    #[arg(short, long, default_value = ".", global = true)]
    root: PathBuf,

    /// Disable execution of code blocks ({python}, {bash}, {sh})
    #[arg(short = 'N', long, global = true)]
    no_exec: bool,

    /// Sandbox boundary for file operations (include shortcode, etc.).
    /// Paths cannot escape this directory. Defaults to --root.
    #[arg(long, global = true)]
    sandbox: Option<PathBuf>,

    /// Start the webapp CMS
    #[cfg(feature = "webapp")]
    #[arg(long)]
    webapp: bool,

    /// Start the desktop app
    #[cfg(feature = "app")]
    #[arg(long)]
    app: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new site
    Init {
        /// Site directory name (defaults to current --root)
        name: Option<String>,

        /// Template to use (default, blog, docs, business)
        #[arg(short, long, default_value = "default")]
        template: String,
    },

    /// Create a new site without prompts
    New {
        /// Site directory name
        name: String,

        /// Preset to scaffold
        #[arg(long, value_enum, default_value_t = Preset::Site)]
        preset: Preset,
    },

    /// Start a temporary demo deck
    Demo {
        /// Output directory
        #[arg(short, long, default_value = DEFAULT_OUTPUT_DIR)]
        output: PathBuf,

        /// Port number
        #[arg(short, long, default_value = DEFAULT_PREVIEW_PORT)]
        port: u16,

        /// Bind address
        #[arg(long, visible_alias = "host", visible_alias = "bind", default_value = DEFAULT_BIND_ADDRESS)]
        interface: String,
    },

    /// Start preview server with live reload
    Preview {
        /// Output directory
        #[arg(short, long, default_value = DEFAULT_OUTPUT_DIR)]
        output: PathBuf,

        /// Port number
        #[arg(short, long, default_value = DEFAULT_PREVIEW_PORT)]
        port: u16,

        /// Exclude draft pages (drafts are included by default in preview)
        #[arg(long)]
        no_drafts: bool,

        /// Open browser
        #[arg(short = 'O', long)]
        open: bool,

        /// Bind address
        #[arg(long, visible_alias = "host", visible_alias = "bind", default_value = DEFAULT_BIND_ADDRESS)]
        interface: String,
    },

    /// Build the site
    Build {
        /// Output directory
        #[arg(short, long, default_value = DEFAULT_OUTPUT_DIR)]
        output: PathBuf,

        /// Include draft pages
        #[arg(long)]
        drafts: bool,

        /// Base URL override
        #[arg(long)]
        base_url: Option<String>,
    },

    /// Check site for errors without building
    Check {
        /// Include draft pages
        #[arg(long)]
        drafts: bool,
        /// Treat lint warnings as errors
        #[arg(long)]
        deny_warnings: bool,
    },

    /// Remove output directory and/or cache
    Clean {
        /// Output directory to remove
        #[arg(short, long, default_value = DEFAULT_OUTPUT_DIR)]
        output: PathBuf,

        /// Also clear the code block execution cache (.zorto/cache/)
        #[arg(long)]
        cache: bool,
    },

    /// Install zorto skill files for AI agents
    #[command(hide = true)]
    Skill {
        #[command(subcommand)]
        command: Option<skill::SkillCommands>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Preset {
    Deck,
    Blog,
    Site,
    Docs,
}

#[derive(Debug)]
pub struct CliExit {
    code: i32,
}

impl CliExit {
    fn new(code: i32) -> Self {
        Self { code }
    }

    pub fn code(&self) -> i32 {
        self.code
    }
}

impl std::fmt::Display for CliExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exited with status {}", self.code)
    }
}

impl std::error::Error for CliExit {}

/// Run the zorto CLI with the given arguments.
///
/// This is the main entry point, equivalent to calling `zorto` on the command line.
/// Pass `std::env::args()` for normal use, or synthetic args for testing.
pub fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    // Handle skill command before resolving root/sandbox (no site context needed)
    if matches!(&cli.command, Some(Commands::Skill { .. })) {
        let Some(Commands::Skill { command }) = cli.command else {
            unreachable!();
        };
        return skill::handle_skill(command);
    }

    let display_root = cli.root.clone();
    let root = std::fs::canonicalize(&cli.root).with_context(|| {
        if cli.root.exists() {
            format!("cannot resolve --root {}", cli.root.display())
        } else {
            format!(
                "--root path does not exist: {}. Pass --root <existing-dir> or cd into your site.",
                cli.root.display()
            )
        }
    })?;
    let sandbox = resolve_sandbox(&cli.sandbox)?;

    #[cfg(feature = "webapp")]
    if cli.webapp {
        let output = resolve_output(&root, std::path::PathBuf::from(DEFAULT_OUTPUT_DIR));
        return zorto_webapp::run_webapp(&root, &output, sandbox.as_deref());
    }

    #[cfg(feature = "app")]
    if cli.app {
        return zorto_app::run_app(&root);
    }

    let Some(command) = cli.command else {
        if root.join("config.toml").exists() {
            println!(
                "Detected a zorto site at {}.\n\
                 Run `zorto preview` to serve it with live reload, \
                 or `zorto --help` for the full command list.",
                display_root.display()
            );
        } else {
            Cli::parse_from(["zorto", "--help"]);
        }
        return Ok(());
    };

    match command {
        Commands::Build {
            output,
            drafts,
            base_url,
        } => {
            ensure_site_exists(&root, &display_root)?;
            let display_output = display_output_path(&display_root, &output);
            let output = resolve_output(&root, output);
            let mut site = site::Site::load(&root, &output, drafts)?;
            site.no_exec = cli.no_exec;
            site.sandbox = sandbox;
            if let Some(url) = base_url {
                site.set_base_url(url);
            }
            site.build()?;
            println!("Site built to {}", display_output.display());
        }
        Commands::Preview {
            output,
            port,
            no_drafts,
            open,
            interface,
        } => {
            ensure_site_exists(&root, &display_root)?;
            let output = resolve_output(&root, output);
            let cfg = serve::ServeConfig {
                root: &root,
                output_dir: &output,
                drafts: !no_drafts,
                no_exec: cli.no_exec,
                sandbox: sandbox.as_deref(),
                interface: &interface,
                port,
                open_browser: open,
            };
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(serve::serve(&cfg))?;
        }
        Commands::Demo {
            output,
            port,
            interface,
        } => {
            let demo_dir = tempfile::tempdir().context("failed to create demo workspace")?;
            create_site_with_defaults(demo_dir.path(), "presentation", Some("Demo Deck"))?;
            let output = resolve_output(demo_dir.path(), output);
            println!(
                "Scaffolding demo deck at {}...",
                demo_dir.path().display()
            );
            let cfg = serve::ServeConfig {
                root: demo_dir.path(),
                output_dir: &output,
                drafts: true,
                no_exec: cli.no_exec,
                sandbox: sandbox.as_deref(),
                interface: &interface,
                port,
                open_browser: true,
            };
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(serve::serve(&cfg))?;
        }
        Commands::Clean { output, cache } => {
            let display_output = display_output_path(&display_root, &output);
            let output = resolve_output(&root, output);
            if output.exists() {
                std::fs::remove_dir_all(&output)?;
                println!("Removed {}", display_output.display());
            }
            if cache {
                zorto_core::cache::clear_cache(&root)?;
                println!("Cleared code block cache");
            }
        }
        Commands::Init { name, template } => {
            // In a TTY we always run the interactive flow — positional args
            // become prompt defaults rather than skipping prompts entirely,
            // so a user gets to set theme / title / author / base_url even
            // when they typed `zorto init my-site`.
            // In a non-TTY (script, pipe) we use the positional args as-is.
            if atty_stdin() {
                interactive_init(&root, name.as_deref(), &template)?;
            } else {
                let target = match name.as_deref() {
                    Some(n) => root.join(n),
                    None => root.clone(),
                };
                init_site(&root, &target, &template)?;
            }
        }
        Commands::New { name, preset } => {
            let target = root.join(&name);
            create_site_with_defaults(&target, preset.template_name(), None)?;
            println!(
                "Created {} (preset: {})",
                target.display(),
                preset.to_possible_value().expect("preset value").get_name()
            );
            print_next_steps(&root, &target);
        }
        Commands::Check {
            drafts,
            deny_warnings,
        } => {
            ensure_site_exists(&root, &display_root)?;
            let output = root.join(DEFAULT_OUTPUT_DIR);
            let mut site = site::Site::load(&root, &output, drafts)?;
            site.no_exec = cli.no_exec;
            site.sandbox = sandbox;
            site.check(deny_warnings)?;
            println!("Site check passed.");
        }
        Commands::Skill { .. } => unreachable!("handled above"),
    }

    Ok(())
}

/// Resolve an output path relative to the site root.
fn resolve_output(root: &std::path::Path, output: PathBuf) -> PathBuf {
    if output.is_relative() {
        root.join(output)
    } else {
        output
    }
}

/// Build a user-facing display form of the output path, avoiding the
/// `/private/tmp` canonical form that macOS introduces when resolving `/tmp`
/// symlinks. Uses the pre-canonicalization `--root` input verbatim.
fn display_output_path(display_root: &std::path::Path, output: &std::path::Path) -> PathBuf {
    if output.is_absolute() {
        output.to_path_buf()
    } else {
        display_root.join(output)
    }
}

/// Canonicalize the sandbox path, returning an error if it doesn't exist.
fn resolve_sandbox(sandbox: &Option<PathBuf>) -> anyhow::Result<Option<PathBuf>> {
    match sandbox {
        Some(p) => {
            let canonical = std::fs::canonicalize(p)
                .map_err(|e| anyhow::anyhow!("cannot resolve sandbox path {}: {e}", p.display()))?;
            Ok(Some(canonical))
        }
        None => Ok(None),
    }
}

/// Check if stdin is a TTY.
fn atty_stdin() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

fn ensure_site_exists(root: &std::path::Path, display_root: &std::path::Path) -> anyhow::Result<()> {
    if root.join("config.toml").exists() {
        return Ok(());
    }
    eprintln!(
        "No zorto site in {}. Try `zorto new <name>` or `zorto init`.",
        display_root.display()
    );
    Err(CliExit::new(2).into())
}

fn init_site(
    root: &std::path::Path,
    target: &std::path::Path,
    template: &str,
) -> anyhow::Result<()> {
    if target.join("config.toml").exists() {
        anyhow::bail!(
            "A zorto site already exists in {} — run `zorto preview` to work with it",
            target.display()
        );
    }
    if target.join("website").join("config.toml").exists() {
        anyhow::bail!(
            "A zorto site already exists in {}/website/ — run `zorto --root website preview` to work with it",
            target.display()
        );
    }

    templates::write_template(target, template)?;

    println!(
        "Initialized new site at {} (template: {template})",
        target.display()
    );
    print_next_steps(root, target);
    Ok(())
}

fn create_site_with_defaults(
    target: &std::path::Path,
    template: &str,
    title_override: Option<&str>,
) -> anyhow::Result<()> {
    if target.join("config.toml").exists() {
        anyhow::bail!(
            "A zorto site already exists in {} — run `zorto preview` to work with it",
            target.display()
        );
    }
    if target.join("website").join("config.toml").exists() {
        anyhow::bail!(
            "A zorto site already exists in {}/website/ — run `zorto --root website preview` to work with it",
            target.display()
        );
    }

    templates::write_template(target, template)?;
    let title = title_override
        .map(std::borrow::ToOwned::to_owned)
        .unwrap_or_else(|| default_site_title(target));
    templates::customize_config(
        target,
        &title,
        DEFAULT_BASE_URL,
        Some("zorto"),
        None,
    )?;
    Ok(())
}

fn default_site_title(target: &std::path::Path) -> String {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .map(title_case_slug)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| DEFAULT_SITE_TITLE.to_string())
}

fn title_case_slug(input: &str) -> String {
    input
        .split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut word = String::new();
            word.extend(first.to_uppercase());
            word.push_str(chars.as_str());
            word
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print a `Next steps:` block pointing the user at `zorto preview --open`.
/// Prefers a relative `cd` when the target is under `root`; otherwise prints
/// just the preview command (target == root) or an absolute cd as fallback.
fn print_next_steps(root: &std::path::Path, target: &std::path::Path) {
    println!();
    println!("Next steps:");
    if target == root {
        println!("  zorto preview --open");
    } else {
        match target.strip_prefix(root) {
            Ok(rel) if !rel.as_os_str().is_empty() => {
                println!("  cd {} && zorto preview --open", rel.display());
            }
            _ => {
                println!("  cd {} && zorto preview --open", target.display());
            }
        }
    }
    println!();
}

/// Interactive init flow using dialoguer prompts.
///
/// Positional `name` and `template` arguments (if given on the CLI) are
/// used as defaults — the user is still prompted, so they get to set
/// theme / site title / author / base_url even when they typed
/// `zorto init my-site -t blog`.
fn interactive_init(
    root: &std::path::Path,
    name_default: Option<&str>,
    template_default: &str,
) -> anyhow::Result<()> {
    use dialoguer::{Input, Select};

    println!();
    println!("Welcome to Zorto!");
    println!("Let's create your new site.");
    println!();

    // 1. Site name / directory
    let name: String = Input::new()
        .with_prompt("Site directory name")
        .default(name_default.unwrap_or(".").to_string())
        .interact_text()?;

    let target = if name == "." {
        root.to_path_buf()
    } else {
        root.join(&name)
    };

    if target.join("config.toml").exists() {
        anyhow::bail!(
            "A zorto site already exists in {} — run `zorto preview` to work with it",
            target.display()
        );
    }
    if target.join("website").join("config.toml").exists() {
        anyhow::bail!(
            "A zorto site already exists in {}/website/ — run `zorto --root website preview` to work with it",
            target.display()
        );
    }

    // 2. Template selection (positional arg pre-selects)
    let template_labels: Vec<String> = templates::TEMPLATES
        .iter()
        .map(|t| format!("{:<12} {}", t.name, t.description))
        .collect();

    let default_template_idx = templates::TEMPLATES
        .iter()
        .position(|t| t.name == template_default)
        .unwrap_or(0);

    let template_idx = Select::new()
        .with_prompt("Template")
        .items(&template_labels)
        .default(default_template_idx)
        .interact()?;
    let template_name = templates::TEMPLATES[template_idx].name;

    // 3. Theme selection
    let available_themes = zorto_core::themes::Theme::available();
    let theme_choice = if available_themes.is_empty() {
        None
    } else {
        let theme_descriptions: Vec<(&str, &str)> = available_themes
            .iter()
            .map(|name| {
                let desc = match *name {
                    "zorto" => "Blue/green with animations (Zorto brand)",
                    "dkdc" => "Violet/cyan with animations (dkdc brand)",
                    "default" => "Clean blue, no animations",
                    "ember" => "Warm orange/amber",
                    "forest" => "Natural green/lime",
                    "ocean" => "Calm teal/blue",
                    "rose" => "Soft pink/purple",
                    "slate" => "Minimal monochrome",
                    "midnight" => "Navy/silver corporate",
                    "sunset" => "Bold red/orange",
                    "mint" => "Modern green/cyan",
                    "plum" => "Rich purple/magenta",
                    "sand" => "Warm neutral/earth tones",
                    "arctic" => "Cool blue/white",
                    "lime" => "Bright green/yellow",
                    "charcoal" => "Dark grey/silver",
                    _ => "",
                };
                (*name, desc)
            })
            .collect();

        let theme_labels: Vec<String> = theme_descriptions
            .iter()
            .map(|(name, desc)| format!("{:<12} {}", name, desc))
            .collect();

        let default_idx = available_themes
            .iter()
            .position(|n| *n == "default")
            .unwrap_or(0);

        let theme_idx = Select::new()
            .with_prompt("Theme")
            .items(&theme_labels)
            .default(default_idx)
            .interact()?;

        Some(available_themes[theme_idx])
    };

    // 4. Configuration
    let site_title: String = Input::new()
        .with_prompt("Site title")
        .default(DEFAULT_SITE_TITLE.to_string())
        .interact_text()?;

    let author: String = Input::new()
        .with_prompt("Author name")
        .default(String::new())
        .allow_empty(true)
        .interact_text()?;

    let base_url: String = Input::new()
        .with_prompt("Base URL (set to your production URL before deploying)")
        .default(DEFAULT_BASE_URL.to_string())
        .interact_text()?;

    // 5. Create the site
    println!();
    templates::write_template(&target, template_name)?;
    templates::customize_config(
        &target,
        &site_title,
        &base_url,
        theme_choice,
        if author.is_empty() {
            None
        } else {
            Some(author.as_str())
        },
    )?;

    // 6. Done + next steps
    println!("Site created at {}", target.display());
    print_next_steps(root, &target);

    Ok(())
}

impl Preset {
    fn template_name(self) -> &'static str {
        match self {
            Self::Deck => "presentation",
            Self::Blog => "blog",
            Self::Site => "default",
            Self::Docs => "docs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    #[test]
    fn parse_skill_install() {
        let cli = Cli::parse_from(["zorto", "skill", "install", "--target", "/tmp/skills"]);
        assert!(matches!(cli.command, Some(Commands::Skill { .. })));
    }

    #[test]
    fn preview_defaults_to_drafts_on() {
        // No --no-drafts → drafts should be included. Marketing users previewing
        // their site should see draft pages without having to know a flag.
        let cli = Cli::parse_from(["zorto", "preview"]);
        match cli.command {
            Some(Commands::Preview { no_drafts, .. }) => assert!(!no_drafts),
            other => panic!("expected Preview, got {other:?}", other = other.is_some()),
        }
    }

    #[test]
    fn preview_no_drafts_opts_out() {
        let cli = Cli::parse_from(["zorto", "preview", "--no-drafts"]);
        match cli.command {
            Some(Commands::Preview { no_drafts, .. }) => assert!(no_drafts),
            _ => panic!("expected Preview"),
        }
    }

    #[test]
    fn build_defaults_to_drafts_off() {
        // Production build stays drafts-off by default.
        let cli = Cli::parse_from(["zorto", "build"]);
        match cli.command {
            Some(Commands::Build { drafts, .. }) => assert!(!drafts),
            _ => panic!("expected Build"),
        }
    }

    #[test]
    fn interface_accepts_host_alias() {
        let cli = Cli::parse_from(["zorto", "preview", "--host", "0.0.0.0"]);
        match cli.command {
            Some(Commands::Preview { interface, .. }) => assert_eq!(interface, "0.0.0.0"),
            _ => panic!("expected Preview"),
        }
    }

    #[test]
    fn interface_accepts_bind_alias() {
        let cli = Cli::parse_from(["zorto", "preview", "--bind", "0.0.0.0"]);
        match cli.command {
            Some(Commands::Preview { interface, .. }) => assert_eq!(interface, "0.0.0.0"),
            _ => panic!("expected Preview"),
        }
    }

    #[test]
    fn global_flag_root_accepted_after_subcommand() {
        // Marketing users naturally type `zorto init my-site --root /tmp/foo`
        // rather than `zorto --root /tmp/foo init my-site`. Both orderings
        // must parse; clap#global = true makes it so.
        let cli = Cli::parse_from(["zorto", "init", "my-site", "--root", "/tmp/foo"]);
        assert_eq!(cli.root, PathBuf::from("/tmp/foo"));
        assert!(matches!(cli.command, Some(Commands::Init { .. })));
    }

    #[test]
    fn global_flag_root_accepted_before_subcommand() {
        let cli = Cli::parse_from(["zorto", "--root", "/tmp/foo", "init", "my-site"]);
        assert_eq!(cli.root, PathBuf::from("/tmp/foo"));
        assert!(matches!(cli.command, Some(Commands::Init { .. })));
    }

    #[test]
    fn global_flag_sandbox_accepted_after_subcommand() {
        let cli = Cli::parse_from(["zorto", "build", "--sandbox", "/tmp"]);
        assert_eq!(cli.sandbox, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn global_flag_no_exec_accepted_after_subcommand() {
        let cli = Cli::parse_from(["zorto", "preview", "--no-exec"]);
        assert!(cli.no_exec);
    }

    #[test]
    fn missing_root_error_is_actionable() {
        // Marketing users who mistype --root should get a pointer back to safety,
        // not a bare io::Error.
        let result = run([
            "zorto",
            "--root",
            "/definitely/does/not/exist/zorto-dx-test",
            "build",
        ]);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("--root"), "got: {err}");
        assert!(err.contains("does not exist"), "got: {err}");
    }

    #[test]
    fn parse_init_with_name_and_template() {
        let cli = Cli::parse_from(["zorto", "init", "my-site", "--template", "blog"]);
        match cli.command {
            Some(Commands::Init { name, template }) => {
                assert_eq!(name.as_deref(), Some("my-site"));
                assert_eq!(template, "blog");
            }
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn parse_new_with_preset() {
        let cli = Cli::parse_from(["zorto", "new", "my-deck", "--preset", "deck"]);
        match cli.command {
            Some(Commands::New { name, preset }) => {
                assert_eq!(name, "my-deck");
                assert_eq!(preset, Preset::Deck);
            }
            _ => panic!("expected New"),
        }
    }

    #[test]
    fn title_case_slug_handles_common_names() {
        assert_eq!(title_case_slug("my-deck"), "My Deck");
        assert_eq!(title_case_slug("docs_site"), "Docs Site");
    }

    #[test]
    fn ensure_site_exists_returns_exit_2() {
        let dir = tempfile::tempdir().unwrap();
        let err = ensure_site_exists(dir.path(), dir.path()).unwrap_err();
        let exit = err.downcast_ref::<CliExit>().expect("CliExit");
        assert_eq!(exit.code(), 2);
    }

    #[test]
    fn skill_subcommand_is_hidden() {
        let mut cmd = Cli::command();
        let skill = cmd
            .find_subcommand_mut("skill")
            .expect("skill subcommand should exist");
        assert!(
            skill.is_hide_set(),
            "`zorto skill` should be hidden from top-level --help"
        );
    }

    #[test]
    fn default_base_url_is_not_localhost() {
        // Accepting the default during interactive init should not
        // produce a sitemap.xml full of localhost links.
        assert!(
            !DEFAULT_BASE_URL.contains("localhost"),
            "DEFAULT_BASE_URL should not be a localhost URL: {DEFAULT_BASE_URL}"
        );
    }
}
