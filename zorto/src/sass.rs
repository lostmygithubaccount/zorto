use std::path::Path;

/// Compile SCSS files to CSS
pub fn compile_sass(sass_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let style_path = sass_dir.join("style.scss");
    if !style_path.exists() {
        return Ok(());
    }

    let css = grass::from_path(&style_path, &grass::Options::default())
        .map_err(|e| anyhow::anyhow!("SCSS compilation error: {e}"))?;

    std::fs::create_dir_all(output_dir)?;
    let output_path = output_dir.join("style.css");
    std::fs::write(&output_path, css)?;

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
    fn test_compile_sass_no_file() {
        let tmp = TempDir::new().unwrap();
        let sass_dir = tmp.path().join("sass");
        let output_dir = tmp.path().join("public");
        std::fs::create_dir_all(&sass_dir).unwrap();
        // No style.scss — should return Ok
        compile_sass(&sass_dir, &output_dir).unwrap();
        assert!(!output_dir.join("style.css").exists());
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
