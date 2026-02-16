use tempfile::TempDir;
use zorto::site::Site;

/// Path to the websites directory relative to the workspace root
fn websites_dir() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // zorto/zorto -> repo root -> websites/
    manifest.join("../../websites")
}

/// Build a real website into a temp output dir and return (site, output_dir, _tmp)
fn build_website(name: &str) -> (Site, std::path::PathBuf, TempDir) {
    let site_root = websites_dir().join(name);
    assert!(
        site_root.exists(),
        "Website {name} not found at {}",
        site_root.display()
    );

    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("public");

    let mut site = Site::load(&site_root, &output, false).unwrap();
    site.no_exec = true; // Don't execute code blocks during tests
    // Set sandbox to repo root so include shortcodes can reference sibling dirs
    site.sandbox = Some(websites_dir().join("..").canonicalize().unwrap());
    site.build().unwrap();

    (site, output, tmp)
}

#[test]
fn test_dkdc_dev_generates_sitemap() {
    let (_site, output, _tmp) = build_website("dkdc.dev");

    let sitemap_path = output.join("sitemap.xml");
    assert!(sitemap_path.exists(), "sitemap.xml should be generated");

    let sitemap = std::fs::read_to_string(&sitemap_path).unwrap();
    assert!(sitemap.contains("https://dkdc.dev"));
    assert!(sitemap.contains("<urlset"));
}

#[test]
fn test_dkdc_dev_respects_llms_txt_config() {
    let (site, output, _tmp) = build_website("dkdc.dev");

    if site.config.generate_llms_txt {
        assert!(output.join("llms.txt").exists());
        assert!(output.join("llms-full.txt").exists());

        let llms = std::fs::read_to_string(output.join("llms.txt")).unwrap();
        assert!(llms.starts_with("# "), "llms.txt should start with site title");
        assert!(llms.contains("##"), "llms.txt should have section headings");
        assert!(
            llms.contains("https://dkdc.dev/"),
            "llms.txt should contain page links"
        );

        let llms_full = std::fs::read_to_string(output.join("llms-full.txt")).unwrap();
        assert!(
            llms_full.starts_with("# "),
            "llms-full.txt should start with site title"
        );
        assert!(
            llms_full.len() > 100,
            "llms-full.txt should contain substantial content"
        );
    } else {
        assert!(!output.join("llms.txt").exists());
        assert!(!output.join("llms-full.txt").exists());
    }
}

#[test]
fn test_zorto_dev_builds_successfully() {
    let (site, output, _tmp) = build_website("zorto.dev");

    assert!(output.join("sitemap.xml").exists() == site.config.generate_sitemap);
    assert!(output.join("llms.txt").exists() == site.config.generate_llms_txt);
    assert!(output.join("index.html").exists());
}

#[test]
fn test_dkdc_io_builds_successfully() {
    let (site, output, _tmp) = build_website("dkdc.io");

    assert!(output.join("sitemap.xml").exists() == site.config.generate_sitemap);
    assert!(output.join("llms.txt").exists() == site.config.generate_llms_txt);
    assert!(output.join("index.html").exists());
}
