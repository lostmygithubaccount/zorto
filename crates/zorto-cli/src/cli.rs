use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::serve;
use crate::skill;
use crate::templates;
use zorto_core::site;

#[derive(Parser)]
#[command(
    name = "zorto",
    version,
    about = "The AI-native static site generator (SSG) with executable code blocks"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Site root directory
    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    /// Disable execution of code blocks ({python}, {bash}, {sh})
    #[arg(short = 'N', long)]
    no_exec: bool,

    /// Sandbox boundary for file operations (include shortcode, etc.).
    /// Paths cannot escape this directory. Defaults to --root.
    #[arg(long)]
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
    /// Build the site
    Build {
        /// Output directory
        #[arg(short, long, default_value = "public")]
        output: PathBuf,

        /// Include draft pages
        #[arg(long)]
        drafts: bool,

        /// Base URL override
        #[arg(long)]
        base_url: Option<String>,
    },

    /// Start preview server with live reload
    Preview {
        /// Output directory
        #[arg(short, long, default_value = "public")]
        output: PathBuf,

        /// Port number
        #[arg(short, long, default_value = "1111")]
        port: u16,

        /// Include draft pages
        #[arg(long)]
        drafts: bool,

        /// Open browser
        #[arg(short = 'O', long)]
        open: bool,

        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        interface: String,
    },

    /// Remove output directory and/or cache
    Clean {
        /// Output directory to remove
        #[arg(short, long, default_value = "public")]
        output: PathBuf,

        /// Also clear the code block execution cache (.zorto/cache/)
        #[arg(long)]
        cache: bool,
    },

    /// Initialize a new site
    Init {
        /// Site directory name (defaults to current --root)
        name: Option<String>,

        /// Template to use (default, blog, docs, business)
        #[arg(short, long, default_value = "default")]
        template: String,
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

    /// Install zorto skill files for AI agents
    Skill {
        #[command(subcommand)]
        command: Option<skill::SkillCommands>,
    },
}

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

    let root = std::fs::canonicalize(&cli.root)?;
    let sandbox = resolve_sandbox(&cli.sandbox)?;

    #[cfg(feature = "webapp")]
    if cli.webapp {
        let output = resolve_output(&root, std::path::PathBuf::from("public"));
        return zorto_webapp::run_webapp(&root, &output, sandbox.as_deref());
    }

    #[cfg(feature = "app")]
    if cli.app {
        return zorto_app::run_app(&root);
    }

    let Some(command) = cli.command else {
        Cli::parse_from(["zorto", "--help"]);
        unreachable!();
    };

    match command {
        Commands::Build {
            output,
            drafts,
            base_url,
        } => {
            let output = resolve_output(&root, output);
            let mut site = site::Site::load(&root, &output, drafts)?;
            site.no_exec = cli.no_exec;
            site.sandbox = sandbox;
            if let Some(url) = base_url {
                site.set_base_url(url);
            }
            site.build()?;
            println!("Site built to {}", output.display());
        }
        Commands::Preview {
            output,
            port,
            drafts,
            open,
            interface,
        } => {
            let output = resolve_output(&root, output);
            let cfg = serve::ServeConfig {
                root: &root,
                output_dir: &output,
                drafts,
                no_exec: cli.no_exec,
                sandbox: sandbox.as_deref(),
                interface: &interface,
                port,
                open_browser: open,
            };
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(serve::serve(&cfg))?;
        }
        Commands::Clean { output, cache } => {
            let output = resolve_output(&root, output);
            if output.exists() {
                std::fs::remove_dir_all(&output)?;
                println!("Removed {}", output.display());
            }
            if cache {
                zorto_core::cache::clear_cache(&root)?;
                println!("Cleared code block cache");
            }
        }
        Commands::Init { name, template } => {
            // Detect if we should launch the interactive flow:
            // - no explicit name was given
            // - the default template value wasn't overridden by the user
            // - stdin is a TTY
            let is_interactive = name.is_none() && template == "default" && atty_stdin();

            if is_interactive {
                interactive_init(&root)?;
            } else {
                let target = match name {
                    Some(n) => root.join(n),
                    None => root.clone(),
                };
                init_site(&target, &template)?;
            }
        }
        Commands::Check {
            drafts,
            deny_warnings,
        } => {
            let output = root.join("public");
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

fn init_site(target: &std::path::Path, template: &str) -> anyhow::Result<()> {
    if target.join("config.toml").exists() {
        anyhow::bail!("config.toml already exists in {}", target.display());
    }

    templates::write_template(target, template)?;

    println!(
        "Initialized new site at {} (template: {template})",
        target.display()
    );
    Ok(())
}

/// Interactive init flow using dialoguer prompts.
fn interactive_init(root: &std::path::Path) -> anyhow::Result<()> {
    use dialoguer::{Input, Select};

    println!();
    println!("  Welcome to Zorto!");
    println!("  Let's create your new site.");
    println!();

    // 1. Site name / directory
    let name: String = Input::new()
        .with_prompt("  Site directory name")
        .default(".".to_string())
        .interact_text()?;

    let target = if name == "." {
        root.to_path_buf()
    } else {
        root.join(&name)
    };

    if target.join("config.toml").exists() {
        anyhow::bail!("config.toml already exists in {}", target.display());
    }

    // 2. Template selection
    let template_labels: Vec<String> = templates::TEMPLATES
        .iter()
        .map(|t| format!("{:<12} {}", t.name, t.description))
        .collect();

    let template_idx = Select::new()
        .with_prompt("  Template")
        .items(&template_labels)
        .default(0)
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
            .with_prompt("  Theme")
            .items(&theme_labels)
            .default(default_idx)
            .interact()?;

        Some(available_themes[theme_idx])
    };

    // 4. Configuration
    let site_title: String = Input::new()
        .with_prompt("  Site title")
        .default("My Site".to_string())
        .interact_text()?;

    let author: String = Input::new()
        .with_prompt("  Author name")
        .default(String::new())
        .allow_empty(true)
        .interact_text()?;

    let base_url: String = Input::new()
        .with_prompt("  Base URL")
        .default("http://localhost:1111".to_string())
        .interact_text()?;

    // 5. Create the site
    println!();
    templates::write_template(&target, template_name)?;

    // Customize the generated config with user values
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

    // 6. Next steps
    println!("  Site created at {}", target.display());
    println!();
    if name != "." {
        println!("  Next steps:");
        println!("    cd {name} && zorto preview --open");
    } else {
        println!("  Next steps:");
        println!("    zorto preview --open");
    }
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_skill_install() {
        let cli = Cli::parse_from(["zorto", "skill", "install", "--target", "/tmp/skills"]);
        assert!(matches!(cli.command, Some(Commands::Skill { .. })));
    }

    #[test]
    fn parse_skill_install_all() {
        let cli = Cli::parse_from([
            "zorto",
            "skill",
            "install",
            "--target",
            "/tmp/skills",
            "--all",
        ]);
        assert!(matches!(cli.command, Some(Commands::Skill { .. })));
    }
}
