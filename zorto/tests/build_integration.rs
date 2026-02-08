mod common;

use tempfile::TempDir;
use zorto::site::Site;

#[test]
fn test_full_build_minimal() {
    let tmp = TempDir::new().unwrap();
    let root = common::make_test_site(&tmp);
    let output = tmp.path().join("public");

    let mut site = Site::load(&root, &output, false).unwrap();
    site.build().unwrap();

    // Root index
    assert!(output.join("index.html").exists());
    let index = std::fs::read_to_string(output.join("index.html")).unwrap();
    assert!(index.contains("Home"));

    // Posts section
    assert!(output.join("posts/index.html").exists());

    // Page
    assert!(output.join("posts/first/index.html").exists());
    let page = std::fs::read_to_string(output.join("posts/first/index.html")).unwrap();
    assert!(page.contains("First Post"));
    assert!(page.contains("<p>First post content.</p>"));
}

#[test]
fn test_full_build_with_sections() {
    let tmp = TempDir::new().unwrap();
    let root = common::make_test_site_with_sections(&tmp);
    let output = tmp.path().join("public");

    let mut site = Site::load(&root, &output, false).unwrap();
    site.build().unwrap();

    // All sections rendered
    assert!(output.join("index.html").exists());
    assert!(output.join("posts/index.html").exists());
    assert!(output.join("docs/index.html").exists());

    // All pages rendered
    assert!(output.join("posts/first/index.html").exists());
    assert!(output.join("posts/second/index.html").exists());
    assert!(output.join("docs/getting-started/index.html").exists());
}

#[test]
fn test_full_build_with_taxonomy() {
    let tmp = TempDir::new().unwrap();
    let root = common::make_test_site_with_tags(&tmp);
    let output = tmp.path().join("public");

    let mut site = Site::load(&root, &output, false).unwrap();
    site.build().unwrap();

    // Taxonomy list
    assert!(output.join("tags/index.html").exists());
    let list = std::fs::read_to_string(output.join("tags/index.html")).unwrap();
    assert!(list.contains("rust"));

    // Individual tag pages
    assert!(output.join("tags/rust/index.html").exists());
    let tag_page = std::fs::read_to_string(output.join("tags/rust/index.html")).unwrap();
    assert!(tag_page.contains("rust"));
    // Both posts tagged "rust"
    assert!(tag_page.contains("Rust Post"));
    assert!(tag_page.contains("Both Post"));

    assert!(output.join("tags/python/index.html").exists());
}

#[test]
fn test_full_build_with_pagination() {
    let tmp = TempDir::new().unwrap();
    let root = common::make_test_site_with_pagination(&tmp);
    let output = tmp.path().join("public");

    let mut site = Site::load(&root, &output, false).unwrap();
    site.build().unwrap();

    // First page of section (page 1)
    assert!(output.join("posts/index.html").exists());
    let page1 = std::fs::read_to_string(output.join("posts/index.html")).unwrap();
    // Should have a "next" link
    assert!(page1.contains("next"));

    // Page 2
    assert!(output.join("posts/page/2/index.html").exists());

    // Page 3
    assert!(output.join("posts/page/3/index.html").exists());
}
