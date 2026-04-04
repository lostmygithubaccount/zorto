use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use crate::{AppState, app};

/// Create a minimal zorto site structure in a tempdir and return (app, tmpdir).
fn test_app(tmp: &TempDir) -> axum::Router {
    let root = tmp.path().join("site");
    let content = root.join("content");
    let templates = root.join("templates");
    let static_dir = root.join("static");
    let output = root.join("public");

    std::fs::create_dir_all(&content).unwrap();
    std::fs::create_dir_all(content.join("posts")).unwrap();
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&static_dir).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    // Config
    std::fs::write(
        root.join("config.toml"),
        "base_url = \"https://example.com\"\ntitle = \"Test Site\"\n",
    )
    .unwrap();

    // Root section
    std::fs::write(
        content.join("_index.md"),
        "+++\ntitle = \"Home\"\n+++\nWelcome",
    )
    .unwrap();

    // Posts section
    std::fs::write(
        content.join("posts/_index.md"),
        "+++\ntitle = \"Blog\"\nsort_by = \"date\"\n+++\n",
    )
    .unwrap();

    // A page
    std::fs::write(
        content.join("posts/hello.md"),
        "+++\ntitle = \"Hello World\"\ndate = \"2025-01-01\"\n+++\nHello content",
    )
    .unwrap();

    // A draft page
    std::fs::write(
        content.join("posts/draft.md"),
        "+++\ntitle = \"Draft Post\"\ndraft = true\n+++\nDraft content",
    )
    .unwrap();

    // Templates
    std::fs::write(
        templates.join("base.html"),
        "<!DOCTYPE html><html><body>{% block content %}{% endblock %}</body></html>",
    )
    .unwrap();
    std::fs::write(
        templates.join("index.html"),
        r#"{% extends "base.html" %}{% block content %}{{ section.title }}{% endblock %}"#,
    )
    .unwrap();
    std::fs::write(
        templates.join("section.html"),
        r#"{% extends "base.html" %}{% block content %}{{ section.title }}{% endblock %}"#,
    )
    .unwrap();
    std::fs::write(
        templates.join("page.html"),
        r#"{% extends "base.html" %}{% block content %}{{ page.title }}{{ page.content | safe }}{% endblock %}"#,
    )
    .unwrap();

    // Static file
    std::fs::write(static_dir.join("style.css"), "body {}").unwrap();

    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root,
        output_dir: output,
        sandbox: None,
        reload_tx,
    });

    app(state)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

async fn post_form(app: &axum::Router, uri: &str, body: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

async fn post_body(app: &axum::Router, uri: &str, body: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "text/plain")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

// ── Dashboard ──────────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_returns_ok() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Dashboard"));
    assert!(body.contains("Test Site"));
}

// ── Pages ──────────────────────────────────────────────────────────

#[tokio::test]
async fn page_list_shows_pages() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/pages").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Hello World"));
    assert!(body.contains("Draft Post"));
    assert!(body.contains("draft")); // draft badge
}

#[tokio::test]
async fn page_edit_returns_editor() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/pages/posts/hello.md").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Hello World"));
    assert!(body.contains("2025-01-01"));
    assert!(body.contains("Hello content"));
}

#[tokio::test]
async fn page_save_updates_file() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Updated+Title&date=2025-06-01&description=new+desc&draft=false&tags=rust%2C+web&body=New+body+content&extra_frontmatter=";
    let (status, body) = post_form(&app, "/pages/posts/hello.md", form).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Updated Title"));

    // Verify file on disk
    let file = std::fs::read_to_string(tmp.path().join("site/content/posts/hello.md")).unwrap();
    assert!(file.contains("Updated Title"));
    assert!(file.contains("2025-06-01"));
    assert!(file.contains("New body content"));
    assert!(file.contains("rust"));
}

#[tokio::test]
async fn page_create_makes_new_file() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=My+New+Page&section=posts&date=2025-03-15&description=A+test&draft=false&tags=test&body=Some+content";
    let (status, _body) = post_form(&app, "/pages/new", form).await;
    // Should redirect (303 or 302)
    assert!(
        status == StatusCode::SEE_OTHER || status == StatusCode::FOUND,
        "expected redirect, got {status}"
    );

    // Verify file was created
    let file_path = tmp.path().join("site/content/posts/my-new-page.md");
    assert!(file_path.exists(), "new page file should exist");
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("My New Page"));
    assert!(content.contains("2025-03-15"));
    assert!(content.contains("Some content"));
}

#[tokio::test]
async fn page_create_as_draft() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Draft+Page&section=posts&date=2025-03-15&description=&draft=true&tags=&body=Draft+body";
    let (status, _) = post_form(&app, "/pages/new", form).await;
    assert!(status == StatusCode::SEE_OTHER || status == StatusCode::FOUND);

    let file_path = tmp.path().join("site/content/posts/draft-page.md");
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("draft = true"));
}

#[tokio::test]
async fn page_delete_removes_file() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let file = tmp.path().join("site/content/posts/hello.md");
    assert!(file.exists());

    let (status, _) = post_form(&app, "/pages/delete/posts/hello.md", "").await;
    assert!(
        status == StatusCode::SEE_OTHER || status == StatusCode::FOUND,
        "expected redirect, got {status}"
    );
    assert!(!file.exists(), "file should be deleted");
}

#[tokio::test]
async fn page_list_excludes_index_files() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (_, body) = get(&app, "/pages").await;
    // _index.md files should not appear in the page list
    assert!(!body.contains("_index.md"));
}

// ── Page path traversal defense ────────────────────────────────────

#[tokio::test]
async fn page_edit_traversal_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // Attempt to read /etc/passwd via traversal
    let (status, body) = get(&app, "/pages/../../etc/passwd").await;
    assert_eq!(status, StatusCode::OK); // returns error page, not 500
    assert!(body.contains("Invalid path"));
}

#[tokio::test]
async fn page_save_traversal_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Hacked&date=&description=&draft=false&tags=&body=pwned&extra_frontmatter=";
    let (status, body) = post_form(&app, "/pages/../../etc/evil.md", form).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Invalid path"));
}

#[tokio::test]
async fn page_delete_traversal_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // Create a file outside content dir to ensure it survives
    let outside = tmp.path().join("site/outside.txt");
    std::fs::write(&outside, "safe").unwrap();

    let (status, _) = post_form(&app, "/pages/delete/../../outside.txt", "").await;
    // Should redirect without deleting
    assert!(status == StatusCode::SEE_OTHER || status == StatusCode::FOUND);
    assert!(
        outside.exists(),
        "file outside content dir must not be deleted"
    );
}

#[tokio::test]
async fn page_create_traversal_in_section_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Evil&section=../../etc&date=&description=&draft=false&tags=&body=pwned";
    let (status, _) = post_form(&app, "/pages/new", form).await;
    // Should redirect to /pages without creating file outside content
    assert!(status == StatusCode::SEE_OTHER || status == StatusCode::FOUND);

    // Ensure no file was created outside content dir
    assert!(
        !tmp.path().join("etc").exists(),
        "traversal should not create dirs outside site"
    );
}

// ── Sections ───────────────────────────────────────────────────────

#[tokio::test]
async fn section_list_shows_sections() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/sections").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Blog")); // posts section title
    assert!(body.contains("Home")); // root section title
}

#[tokio::test]
async fn section_list_shows_page_counts() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (_, body) = get(&app, "/sections").await;
    // posts/ has hello.md and draft.md = 2 pages
    assert!(body.contains("2 pages"));
}

#[tokio::test]
async fn section_edit_returns_editor() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/sections/posts/_index.md").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Blog"));
    assert!(body.contains("sort_by") || body.contains("Sort By"));
}

#[tokio::test]
async fn section_save_updates_file() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Updated+Blog&description=My+blog&sort_by=title&paginate_by=5&body=Section+body&extra_frontmatter=";
    let (status, body) = post_form(&app, "/sections/posts/_index.md", form).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Updated Blog"));

    let file = std::fs::read_to_string(tmp.path().join("site/content/posts/_index.md")).unwrap();
    assert!(file.contains("Updated Blog"));
    assert!(file.contains("sort_by = \"title\""));
    assert!(file.contains("paginate_by = 5"));
    assert!(file.contains("Section body"));
}

#[tokio::test]
async fn section_edit_traversal_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/sections/../../etc/passwd").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Invalid path"));
}

#[tokio::test]
async fn section_save_traversal_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Evil&description=&sort_by=date&paginate_by=&body=pwned&extra_frontmatter=";
    let (status, body) = post_form(&app, "/sections/../../etc/evil", form).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Invalid path"));
}

// ── Assets ─────────────────────────────────────────────────────────

#[tokio::test]
async fn asset_list_shows_files() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/assets").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("style.css"));
}

#[tokio::test]
async fn asset_upload_saves_file() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let boundary = "----TestBoundary123";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"subdir\"\r\n\r\n\
         \r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         hello world\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method("POST")
        .uri("/assets/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let uploaded = tmp.path().join("site/static/test.txt");
    assert!(uploaded.exists(), "uploaded file should exist");
    assert_eq!(std::fs::read_to_string(&uploaded).unwrap(), "hello world");
}

#[tokio::test]
async fn asset_upload_with_subdir() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let boundary = "----TestBoundary456";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"subdir\"\r\n\r\n\
         img\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"logo.png\"\r\n\
         Content-Type: image/png\r\n\r\n\
         PNG_DATA\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method("POST")
        .uri("/assets/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let uploaded = tmp.path().join("site/static/img/logo.png");
    assert!(uploaded.exists(), "uploaded file in subdir should exist");
}

#[tokio::test]
async fn asset_upload_traversal_in_subdir_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let boundary = "----TestBoundary789";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"subdir\"\r\n\r\n\
         ../../etc\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"evil.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         pwned\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method("POST")
        .uri("/assets/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let resp_body = resp.into_body().collect().await.unwrap().to_bytes();
    let resp_str = String::from_utf8_lossy(&resp_body);
    assert!(
        resp_str.contains("Upload error") || resp_str.contains("Invalid"),
        "traversal in subdir should be rejected"
    );

    assert!(
        !tmp.path().join("etc/evil.txt").exists(),
        "file must not be written outside static dir"
    );
}

#[tokio::test]
async fn asset_upload_traversal_in_filename_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let boundary = "----TestBoundaryABC";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"subdir\"\r\n\r\n\
         \r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"../evil.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         pwned\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method("POST")
        .uri("/assets/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let resp_body = resp.into_body().collect().await.unwrap().to_bytes();
    let resp_str = String::from_utf8_lossy(&resp_body);
    assert!(
        resp_str.contains("Invalid filename") || resp_str.contains("Upload error"),
        "traversal in filename should be rejected"
    );
}

// ── Config ─────────────────────────────────────────────────────────

#[tokio::test]
async fn config_edit_shows_config() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/config").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("base_url"));
    assert!(body.contains("https://example.com"));
}

#[tokio::test]
async fn config_save_valid_toml() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let new_config =
        "content=base_url+%3D+%22https%3A%2F%2Fnew.example.com%22%0Atitle+%3D+%22New+Title%22%0A";
    let (status, _body) = post_form(&app, "/config", new_config).await;
    assert_eq!(status, StatusCode::OK);

    let file = std::fs::read_to_string(tmp.path().join("site/config.toml")).unwrap();
    assert!(file.contains("new.example.com"));
}

#[tokio::test]
async fn config_save_invalid_toml_rejected() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // Send invalid TOML
    let bad_config = "content=%5Binvalid+toml";
    let (status, body) = post_form(&app, "/config", bad_config).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("error") || body.contains("Error"));

    // Original config should be preserved
    let file = std::fs::read_to_string(tmp.path().join("site/config.toml")).unwrap();
    assert!(file.contains("https://example.com"));
}

// ── Build ──────────────────────────────────────────────────────────

#[tokio::test]
async fn build_trigger_succeeds() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = post_form(&app, "/build", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Built successfully"));
}

// ── Preview rendering ──────────────────────────────────────────────

#[tokio::test]
async fn preview_renders_markdown() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = post_body(&app, "/preview/render", "# Hello\n\nWorld").await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(body.contains("<h1"), "expected <h1 in: {body}");
    assert!(body.contains("Hello"));
    assert!(body.contains("World"));
}

#[tokio::test]
async fn preview_strips_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let content = "+++\ntitle = \"Test\"\n+++\n# Heading\n\nBody text";
    let (status, body) = post_body(&app, "/preview/render", content).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<h1"), "expected <h1 in: {body}");
    assert!(body.contains("Heading"));
    assert!(body.contains("Body text"));
    // Frontmatter should not appear in rendered output
    assert!(!body.contains("title = "));
}

// ── New page form ──────────────────────────────────────────────────

#[tokio::test]
async fn new_page_form_lists_sections() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/pages/new").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("posts")); // section option
    assert!(body.contains("(root)")); // root option
}

// ── Page create at root ────────────────────────────────────────────

#[tokio::test]
async fn page_create_at_root_section() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form =
        "title=About+Me&section=&date=2025-01-01&description=&draft=false&tags=&body=About+page";
    let (status, _) = post_form(&app, "/pages/new", form).await;
    assert!(status == StatusCode::SEE_OTHER || status == StatusCode::FOUND);

    let file_path = tmp.path().join("site/content/about-me.md");
    assert!(file_path.exists(), "root page should be created");
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("About Me"));
}

// ── Page save preserves extra frontmatter ──────────────────────────

#[tokio::test]
async fn page_save_preserves_extra_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Hello+World&date=2025-01-01&description=&draft=false&tags=&body=Updated&extra_frontmatter=custom_key+%3D+%22value%22%0A";
    let (status, _) = post_form(&app, "/pages/posts/hello.md", form).await;
    assert_eq!(status, StatusCode::OK);

    let file = std::fs::read_to_string(tmp.path().join("site/content/posts/hello.md")).unwrap();
    assert!(file.contains("custom_key = \"value\""));
}

// ── Section save preserves extra frontmatter ───────────────────────

#[tokio::test]
async fn section_save_preserves_extra_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Blog&description=&sort_by=date&paginate_by=&body=&extra_frontmatter=template+%3D+%22custom.html%22%0A";
    let (status, _) = post_form(&app, "/sections/posts/_index.md", form).await;
    assert_eq!(status, StatusCode::OK);

    let file = std::fs::read_to_string(tmp.path().join("site/content/posts/_index.md")).unwrap();
    assert!(file.contains("template = \"custom.html\""));
}
