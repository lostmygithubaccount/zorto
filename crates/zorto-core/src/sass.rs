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

/// Compile CSS for every available theme as `style-{name}.css`.
///
/// The active theme (already compiled as `style.css`) is skipped.
pub fn compile_all_theme_styles(
    output_dir: &Path,
    active_theme: Option<&Theme>,
) -> anyhow::Result<()> {
    // Compile individual theme files (for backward compat / direct linking)
    for name in Theme::available() {
        let Some(theme) = Theme::from_name(name) else {
            continue;
        };
        if active_theme == Some(&theme) {
            continue;
        }

        let work_dir = tempfile::tempdir()
            .map_err(|e| anyhow::anyhow!("failed to create temp dir for theme {name}: {e}"))?;

        for (filename, content) in &theme.scss() {
            std::fs::write(work_dir.path().join(filename), content)
                .map_err(|e| anyhow::anyhow!("failed to write theme SCSS {filename}: {e}"))?;
        }

        let css = grass::from_path(
            work_dir.path().join("style.scss"),
            &grass::Options::default(),
        )
        .map_err(|e| anyhow::anyhow!("SCSS compilation error for theme {name}: {e}"))?;

        std::fs::create_dir_all(output_dir)?;
        std::fs::write(output_dir.join(format!("style-{name}.css")), css)?;
    }

    // Compile a single themes.css with all theme variables scoped by attribute.
    // This enables instant theme switching via data-site-theme attribute with
    // zero FOUC — no stylesheet swapping needed.
    compile_themes_css(output_dir, active_theme)?;

    Ok(())
}

/// Compile a single `themes.css` containing CSS variables for all themes,
/// scoped under `[data-site-theme="X"]` selectors.
///
/// Each theme's `:root` variables become `[data-site-theme="X"]` and its
/// `[data-theme="light"]` variables become `[data-site-theme="X"][data-theme="light"]`.
/// This file is loaded alongside `style.css` and enables instant theme switching
/// by just setting an HTML attribute — no stylesheet swapping or loading delays.
fn compile_themes_css(output_dir: &Path, active_theme: Option<&Theme>) -> anyhow::Result<()> {
    let mut css = String::from(
        "/* All theme variables for instant switching via data-site-theme attribute */\n",
    );
    let active_name = active_theme.map(|t| t.name()).unwrap_or("zorto");

    for name in Theme::available() {
        // Skip active theme — its variables are already in style.css as :root
        if name == active_name {
            continue;
        }
        let Some(theme) = Theme::from_name(name) else {
            continue;
        };

        // Extract just the CSS variable declarations from the theme's SCSS.
        // We compile a minimal SCSS that only has the variable blocks (no
        // structure/component imports) to get pure CSS custom properties.
        let scss_files = theme.scss();
        let Some((_, style_content)) = scss_files.iter().find(|(f, _)| *f == "style.scss") else {
            continue;
        };

        // Strip @import lines (structure, components), Sass variables ($...),
        // and font @import url() lines (deduplicated at top of file by caller).
        let mut vars_scss = String::new();
        for line in style_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@import") || trimmed.starts_with("$") {
                continue;
            }
            vars_scss.push_str(line);
            vars_scss.push('\n');
        }

        // Compile the stripped SCSS to CSS (handles any SCSS syntax in variable blocks)
        let compiled = grass::from_string(vars_scss, &grass::Options::default()).map_err(|e| {
            anyhow::anyhow!("SCSS compilation error for theme {name} variables: {e}")
        })?;

        // Rewrite selectors: :root → [data-site-theme="X"],
        // [data-theme...] → [data-site-theme="X"][data-theme...]
        // grass may output [data-theme=light] or [data-theme="light"]
        let scoped = compiled
            .replace(":root", &format!("[data-site-theme=\"{name}\"]"))
            .replace(
                "[data-theme=light]",
                &format!("[data-site-theme=\"{name}\"][data-theme=light]"),
            )
            .replace(
                "[data-theme=\"light\"]",
                &format!("[data-site-theme=\"{name}\"][data-theme=\"light\"]"),
            );

        css.push_str(&format!("\n/* theme: {name} */\n"));
        css.push_str(&scoped);
    }

    std::fs::create_dir_all(output_dir)?;
    std::fs::write(output_dir.join("themes.css"), css)?;
    Ok(())
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
