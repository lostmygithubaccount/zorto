use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::serve;
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

    /// Remove output directory
    Clean {
        /// Output directory to remove
        #[arg(short, long, default_value = "public")]
        output: PathBuf,
    },

    /// Initialize a new site
    Init {
        /// Site directory name (defaults to current --root)
        name: Option<String>,

        /// Template to use (default, business)
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
        Commands::Clean { output } => {
            let output = resolve_output(&root, output);
            if output.exists() {
                std::fs::remove_dir_all(&output)?;
                println!("Removed {}", output.display());
            }
        }
        Commands::Init { name, template } => {
            let target = match name {
                Some(n) => root.join(n),
                None => root.clone(),
            };
            init_site(&target, &template)?;
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
