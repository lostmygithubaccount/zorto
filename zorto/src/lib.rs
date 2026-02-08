pub mod config;
pub mod content;
pub mod site;

pub(crate) mod execute;
pub(crate) mod links;
pub(crate) mod markdown;
pub(crate) mod sass;
pub(crate) mod shortcodes;
pub(crate) mod templates;

pub(crate) mod serve;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "zorto",
    about = "A fast static site generator with executable code blocks"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Site root directory
    #[arg(short, long, default_value = ".")]
    root: PathBuf,
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
}

pub fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    let root = std::fs::canonicalize(&cli.root)?;

    match cli.command {
        Commands::Build {
            output,
            drafts,
            base_url,
        } => {
            let output = if output.is_relative() {
                root.join(output)
            } else {
                output
            };
            let mut site = site::Site::load(&root, &output, drafts)?;
            if let Some(url) = base_url {
                site.set_base_url(url);
            }
            site.build()?;
            println!("Site built to {}", output.display());
        }
        Commands::Preview {
            port,
            drafts,
            open,
            interface,
        } => {
            let output = root.join("public");
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(serve::serve(&root, &output, drafts, &interface, port, open))?;
        }
        Commands::Clean { output } => {
            let output = if output.is_relative() {
                root.join(output)
            } else {
                output
            };
            if output.exists() {
                std::fs::remove_dir_all(&output)?;
                println!("Removed {}", output.display());
            }
        }
    }

    Ok(())
}
