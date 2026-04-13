use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

const SKILL_ZORTO: &str = include_str!("skill-zorto.md");

#[derive(Subcommand)]
pub(crate) enum SkillCommands {
    /// Install the zorto skill
    #[command(arg_required_else_help = true)]
    Install {
        /// Target directory
        ///
        /// Examples: ./.claude/skills, ~/.claude/skills, ./.agents/skills
        #[arg(long, required = true)]
        target: String,
    },
}

pub(crate) fn handle_skill(cmd: Option<SkillCommands>) -> Result<()> {
    let Some(cmd) = cmd else {
        anyhow::bail!("run `zorto skill install --help` for usage");
    };
    match cmd {
        SkillCommands::Install { target } => {
            let target = if target.starts_with("~/") || target == "~" {
                let home = std::env::var("HOME").context("HOME environment variable not set")?;
                PathBuf::from(target.replacen('~', &home, 1))
            } else {
                PathBuf::from(&target)
            };

            let dir_name = "zorto";
            let dir = target.join(dir_name);
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("failed to create directory {}", dir.display()))?;
            let path = dir.join("SKILL.md");
            std::fs::write(&path, SKILL_ZORTO)
                .with_context(|| format!("failed to write {}", path.display()))?;
            let abs = std::fs::canonicalize(&path).unwrap_or(path);
            println!("Installed {dir_name} skill to {}", abs.display());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_zorto_content_is_not_empty() {
        assert!(!SKILL_ZORTO.is_empty());
        assert!(SKILL_ZORTO.contains("zorto"));
    }

    #[test]
    fn skill_zorto_has_frontmatter() {
        assert!(SKILL_ZORTO.starts_with("---"));
        assert!(SKILL_ZORTO.contains("name: zorto"));
    }

    #[test]
    fn install_writes_skill_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().to_str().unwrap().to_string();
        handle_skill(Some(SkillCommands::Install { target })).unwrap();
        let skill_path = tmp.path().join("zorto").join("SKILL.md");
        assert!(skill_path.exists());
        let content = std::fs::read_to_string(&skill_path).unwrap();
        assert!(content.contains("zorto"));
    }

    #[test]
    fn install_creates_nested_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp
            .path()
            .join("deep")
            .join("nested")
            .to_str()
            .unwrap()
            .to_string();
        handle_skill(Some(SkillCommands::Install { target })).unwrap();
        let skill_path = tmp
            .path()
            .join("deep")
            .join("nested")
            .join("zorto")
            .join("SKILL.md");
        assert!(skill_path.exists());
    }
}
