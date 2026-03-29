use std::path::Path;

use crate::themes::Theme;

/// Compile SCSS with optional theme support.
///
/// When a theme is active, theme SCSS files are written to a temp directory
/// first, then local `sass/` files are overlaid on top. This lets a site
/// override just `_variables.scss` to change colors while keeping the rest
/// of the theme's stylesheet.
pub fn compile_sass_with_theme(
    sass_dir: &Path,
    output_dir: &Path,
    theme: Option<&Theme>,
) -> anyhow::Result<()> {
    match theme {
        Some(theme) => {
            let scss_files = theme.scss();
            if scss_files.is_empty() && !sass_dir.exists() {
                return Ok(());
            }

            let work_dir = tempfile::tempdir()
                .map_err(|e| anyhow::anyhow!("failed to create temp dir for SCSS: {e}"))?;

            // Write theme SCSS files as base layer
            for (filename, content) in &scss_files {
                std::fs::write(work_dir.path().join(filename), content)
                    .map_err(|e| anyhow::anyhow!("failed to write theme SCSS {filename}: {e}"))?;
            }

            // Overlay local sass files (local overrides theme)
            if sass_dir.exists() {
                for entry in std::fs::read_dir(sass_dir)
                    .map_err(|e| anyhow::anyhow!("cannot read sass dir: {e}"))?
                {
                    let entry = entry?;
                    if entry.path().is_file() {
                        std::fs::copy(entry.path(), work_dir.path().join(entry.file_name()))
                            .map_err(|e| {
                                anyhow::anyhow!("failed to copy {}: {e}", entry.path().display())
                            })?;
                    }
                }
            }

            compile_sass(work_dir.path(), output_dir)
        }
        None => {
            if sass_dir.exists() {
                compile_sass(sass_dir, output_dir)
            } else {
                Ok(())
            }
        }
    }
}

/// Compile all top-level SCSS files in `sass_dir` to CSS in `output_dir`.
///
/// Each `<name>.scss` produces `<name>.css`. Files starting with `_` are
/// treated as partials (imported by other files) and skipped.
pub fn compile_sass(sass_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let entries = std::fs::read_dir(sass_dir)
        .map_err(|e| anyhow::anyhow!("cannot read sass directory: {e}"))?;

    let mut compiled = false;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        // Skip non-SCSS, partials, and directories
        if !name.ends_with(".scss") || name.starts_with('_') || path.is_dir() {
            continue;
        }

        let css = grass::from_path(&path, &grass::Options::default())
            .map_err(|e| anyhow::anyhow!("SCSS compilation error in {name}: {e}"))?;

        if !compiled {
            std::fs::create_dir_all(output_dir)?;
            compiled = true;
        }

        let out_name = Path::new(name).with_extension("css");
        std::fs::write(output_dir.join(out_name), css)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compile_sass_basic() {
        let tmp = TempDir::new().unwrap();
        let sass_dir = tmp.path().join("sass");
        let output_dir = tmp.path().join("public");
        std::fs::create_dir_all(&sass_dir).unwrap();
        std::fs::write(
            sass_dir.join("style.scss"),
            "body { color: red; .inner { font-size: 14px; } }",
        )
        .unwrap();
        compile_sass(&sass_dir, &output_dir).unwrap();
        let css = std::fs::read_to_string(output_dir.join("style.css")).unwrap();
        assert!(css.contains("color: red"));
        assert!(css.contains("font-size: 14px"));
    }

    #[test]
    fn test_compile_sass_multiple_files() {
        let tmp = TempDir::new().unwrap();
        let sass_dir = tmp.path().join("sass");
        let output_dir = tmp.path().join("public");
        std::fs::create_dir_all(&sass_dir).unwrap();
        std::fs::write(sass_dir.join("style.scss"), "body { color: red; }").unwrap();
        std::fs::write(sass_dir.join("extra.scss"), "h1 { font-size: 2em; }").unwrap();
        compile_sass(&sass_dir, &output_dir).unwrap();
        assert!(output_dir.join("style.css").exists());
        assert!(output_dir.join("extra.css").exists());
        let extra = std::fs::read_to_string(output_dir.join("extra.css")).unwrap();
        assert!(extra.contains("font-size: 2em"));
    }

    #[test]
    fn test_compile_sass_skips_partials() {
        let tmp = TempDir::new().unwrap();
        let sass_dir = tmp.path().join("sass");
        let output_dir = tmp.path().join("public");
        std::fs::create_dir_all(&sass_dir).unwrap();
        std::fs::write(sass_dir.join("_vars.scss"), "$color: red;").unwrap();
        std::fs::write(
            sass_dir.join("style.scss"),
            "@use 'vars'; body { color: vars.$color; }",
        )
        .unwrap();
        compile_sass(&sass_dir, &output_dir).unwrap();
        assert!(output_dir.join("style.css").exists());
        assert!(!output_dir.join("_vars.css").exists());
    }

    #[test]
    fn test_compile_sass_no_files() {
        let tmp = TempDir::new().unwrap();
        let sass_dir = tmp.path().join("sass");
        let output_dir = tmp.path().join("public");
        std::fs::create_dir_all(&sass_dir).unwrap();
        compile_sass(&sass_dir, &output_dir).unwrap();
        // Output dir should not be created if nothing was compiled
        assert!(!output_dir.exists());
    }

    #[test]
    fn test_compile_sass_error() {
        let tmp = TempDir::new().unwrap();
        let sass_dir = tmp.path().join("sass");
        let output_dir = tmp.path().join("public");
        std::fs::create_dir_all(&sass_dir).unwrap();
        std::fs::write(sass_dir.join("style.scss"), "body { color: }").unwrap();
        let result = compile_sass(&sass_dir, &output_dir);
        assert!(result.is_err());
    }
}
