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
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
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

// ── Section CRUD (wave-4 H2) ───────────────────────────────────────

#[tokio::test]
async fn section_new_form_returns_ok() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/sections/new").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("New Section"));
    assert!(body.contains("name=\"title\""));
    assert!(body.contains("name=\"slug\""));
    assert!(body.contains("name=\"sort_by\""));
}

#[tokio::test]
async fn section_create_writes_index_md() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Marketing&slug=marketing&description=Campaigns&sort_by=date&paginate_by=5";
    let (status, _body) = post_form(&app, "/sections/new", form).await;
    assert!(
        status == StatusCode::SEE_OTHER || status == StatusCode::FOUND,
        "expected redirect after create, got {status}"
    );

    let index = tmp.path().join("site/content/marketing/_index.md");
    assert!(index.exists(), "marketing/_index.md should exist");
    let written = std::fs::read_to_string(&index).unwrap();
    assert!(written.contains("title = \"Marketing\""));
    assert!(written.contains("description = \"Campaigns\""));
    assert!(written.contains("sort_by = \"date\""));
    assert!(written.contains("paginate_by = 5"));
}

#[tokio::test]
async fn section_create_derives_slug_from_title_when_blank() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Case+Studies&slug=&description=&sort_by=date&paginate_by=";
    let (_status, _body) = post_form(&app, "/sections/new", form).await;

    let index = tmp.path().join("site/content/case-studies/_index.md");
    assert!(index.exists(), "slug should default to slugify(title)");
}

#[tokio::test]
async fn section_create_rejects_missing_title() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=&slug=anything&description=&sort_by=date&paginate_by=";
    let (status, _body) = post_form(&app, "/sections/new", form).await;
    // Redirect back to form with error flag
    assert!(
        status == StatusCode::SEE_OTHER || status == StatusCode::FOUND,
        "expected redirect on empty title, got {status}"
    );
    // And the directory was NOT created
    let dir = tmp.path().join("site/content/anything");
    assert!(
        !dir.exists(),
        "no section should be created when title is empty"
    );
}

#[tokio::test]
async fn section_create_rejects_duplicate_slug() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // `posts` already exists in the fixture
    let form = "title=Posts+Redux&slug=posts&description=&sort_by=date&paginate_by=";
    let (_status, _body) = post_form(&app, "/sections/new", form).await;

    // Existing index content must be untouched
    let existing =
        std::fs::read_to_string(tmp.path().join("site/content/posts/_index.md")).unwrap();
    assert!(
        existing.contains("Blog"),
        "existing posts/_index.md must not be overwritten on duplicate-slug attempt"
    );
}

#[tokio::test]
async fn section_create_rejects_unsafe_slug() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // Path traversal, whitespace, uppercase all invalid per slug_is_safe.
    for slug in ["../escape", "has space", "UPPER", "dots.in.name", ".hidden"] {
        let form = format!(
            "title=Something&slug={}&description=&sort_by=date&paginate_by=",
            urlencode(slug)
        );
        let (_status, _body) = post_form(&app, "/sections/new", &form).await;
    }

    // None of those should have created a section directory anywhere.
    for bad in [
        "site/content/has space",
        "site/content/UPPER",
        "site/content/dots.in.name",
        "site/content/.hidden",
    ] {
        assert!(
            !tmp.path().join(bad).exists(),
            "unsafe slug produced an unexpected directory: {bad}"
        );
    }
}

#[tokio::test]
async fn section_create_then_page_redirects_to_page_form_with_preselect() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Projects&slug=projects&description=&sort_by=date&paginate_by=&then=page";
    let req = Request::builder()
        .method("POST")
        .uri("/sections/new")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(form.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(
        resp.status() == StatusCode::SEE_OTHER || resp.status() == StatusCode::FOUND,
        "expected redirect, got {}",
        resp.status()
    );
    let location = resp
        .headers()
        .get("location")
        .expect("Location header")
        .to_str()
        .unwrap();
    assert_eq!(
        location, "/pages/new?preselect=projects",
        "inline-create path must return to /pages/new with preselect"
    );
}

#[tokio::test]
async fn pages_new_preselects_from_query() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    // Create a section first
    let form = "title=Team&slug=team&description=&sort_by=date&paginate_by=";
    let _ = post_form(&app, "/sections/new", form).await;

    let (status, body) = get(&app, "/pages/new?preselect=team").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains(r#"value="team" selected"#),
        "team option must be preselected"
    );
    assert!(
        body.contains("Section created"),
        "success flash should appear when preselect is present"
    );
}

#[tokio::test]
async fn pages_new_shows_inline_create_link() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/pages/new").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("/sections/new?then=page"),
        "New Page form must link to inline section creation"
    );
}

#[tokio::test]
async fn section_delete_empty_section() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // Create a section with no pages
    let form = "title=Empty&slug=empty&description=&sort_by=date&paginate_by=";
    let _ = post_form(&app, "/sections/new", form).await;
    let section_dir = tmp.path().join("site/content/empty");
    assert!(section_dir.exists(), "precondition: empty/ should exist");

    let (status, _body) = post_form(&app, "/sections/delete/empty/_index.md", "").await;
    assert!(
        status == StatusCode::SEE_OTHER || status == StatusCode::FOUND,
        "expected redirect after delete, got {status}"
    );
    assert!(
        !section_dir.exists(),
        "empty/ should have been removed after delete"
    );
}

#[tokio::test]
async fn section_delete_refuses_when_section_has_pages() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // `posts` has hello.md + draft.md per fixture — delete must refuse.
    let req = Request::builder()
        .method("POST")
        .uri("/sections/delete/posts/_index.md")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let location = resp
        .headers()
        .get("location")
        .expect("Location header")
        .to_str()
        .unwrap();
    assert!(
        location.contains("error=not_empty"),
        "expected not_empty error flag, got {location}"
    );

    // Directory and pages still intact
    assert!(tmp.path().join("site/content/posts/_index.md").exists());
    assert!(tmp.path().join("site/content/posts/hello.md").exists());
    assert!(tmp.path().join("site/content/posts/draft.md").exists());
}

#[tokio::test]
async fn section_delete_refuses_root_section() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let req = Request::builder()
        .method("POST")
        .uri("/sections/delete/_index.md")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let location = resp
        .headers()
        .get("location")
        .expect("Location header")
        .to_str()
        .unwrap();
    // Root either classifies as not_empty (content/ has posts/ + other files)
    // or as root_section depending on fixture state. Either way, the delete
    // MUST NOT succeed: content/_index.md must still exist.
    assert!(
        location.contains("error="),
        "root section delete must carry an error flag: {location}"
    );
    assert!(
        tmp.path().join("site/content/_index.md").exists(),
        "root _index.md must not be deleted"
    );
}

#[tokio::test]
async fn section_delete_traversal_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let req = Request::builder()
        .method("POST")
        .uri("/sections/delete/../../etc/evil")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let location = resp
        .headers()
        .get("location")
        .expect("Location header")
        .to_str()
        .unwrap();
    assert!(
        location.contains("error=invalid_path"),
        "traversal must redirect with invalid_path, got {location}"
    );
}

#[tokio::test]
async fn section_list_shows_new_section_after_create() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "title=Docs+Hub&slug=docs&description=&sort_by=date&paginate_by=";
    let _ = post_form(&app, "/sections/new", form).await;

    let (_status, body) = get(&app, "/sections").await;
    assert!(
        body.contains("Docs Hub"),
        "new section should appear in the list"
    );
}

/// Minimal url-encoder for test fixture slugs. Only handles the characters
/// our test cases need.
fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "+".to_string(),
            '/' => "%2F".to_string(),
            '.' => "%2E".to_string(),
            _ => c.to_string(),
        })
        .collect()
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
    let (status, body) = post_body(&app, "/_render-markdown", "# Hello\n\nWorld").await;
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
    let (status, body) = post_body(&app, "/_render-markdown", content).await;
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

// ── Onboarding ────────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_redirects_when_no_site() {
    let tmp = TempDir::new().unwrap();
    // Create an app with no config.toml — empty dir
    let root = tmp.path().join("empty");
    std::fs::create_dir_all(&root).unwrap();
    let output = root.join("public");
    std::fs::create_dir_all(&output).unwrap();

    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root,
        output_dir: output,
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    let (status, _) = get(&app, "/").await;
    // Should redirect to /setup
    assert!(
        status == StatusCode::SEE_OTHER || status == StatusCode::FOUND,
        "expected redirect to setup, got {status}"
    );
}

#[tokio::test]
async fn setup_welcome_returns_ok() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("empty");
    std::fs::create_dir_all(&root).unwrap();
    let output = root.join("public");
    std::fs::create_dir_all(&output).unwrap();

    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root,
        output_dir: output,
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    let (status, body) = get(&app, "/setup").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Create your site"));
}

#[tokio::test]
async fn setup_template_page_returns_ok() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("empty");
    std::fs::create_dir_all(&root).unwrap();
    let output = root.join("public");
    std::fs::create_dir_all(&output).unwrap();

    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root,
        output_dir: output,
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    let (status, body) = get(&app, "/setup/template").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Choose a template"));
    assert!(body.contains("blog"));
    assert!(body.contains("docs"));
}

// ── Asset delete ──────────────────────────────────────────────────

#[tokio::test]
async fn asset_delete_removes_file() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let file = tmp.path().join("site/static/style.css");
    assert!(file.exists());

    let (status, body) = post_form(&app, "/assets/delete", "path=style.css").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("File deleted"));
    assert!(!file.exists(), "file should be deleted");
}

#[tokio::test]
async fn asset_delete_traversal_blocked() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // Try to delete a file outside static dir
    let outside = tmp.path().join("site/config.toml");
    assert!(outside.exists());

    let (status, body) = post_form(&app, "/assets/delete", "path=../config.toml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("Invalid path") || body.contains("error"),
        "traversal should be blocked"
    );
    assert!(outside.exists(), "config.toml must not be deleted");
}

// ── HTMX served from webapp ──────────────────────────────────────

#[tokio::test]
async fn htmx_js_served() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let (status, body) = get(&app, "/static/htmx.min.js").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("htmx"), "should serve htmx library");
}

// ── Dashboard with welcome ────────────────────────────────────────

#[tokio::test]
async fn dashboard_shows_welcome_flash() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let (status, body) = get(&app, "/?welcome=1").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Site created successfully"));
}

#[tokio::test]
async fn dashboard_shows_recent_pages() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let (status, body) = get(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Recent Pages"));
    assert!(body.contains("Hello World"));
}

// ── Config visual mode ────────────────────────────────────────────

#[tokio::test]
async fn config_shows_visual_form() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let (status, body) = get(&app, "/config").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Settings"));
    assert!(body.contains("Theme"));
    assert!(body.contains("Raw Config"));
}

// ── Validation & error path coverage ─────────────────────────────

#[tokio::test]
async fn page_create_empty_title_rejected() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form =
        "title=&section=posts&date=2025-03-15&description=&draft=false&tags=&body=Some+content";
    let (status, body) = post_form(&app, "/pages/new", form).await;
    // Should NOT redirect — should return an error page
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("Title is required"),
        "expected title validation error in: {body}"
    );
}

#[tokio::test]
async fn page_edit_nonexistent_returns_empty_editor() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    // Request a page that doesn't exist on disk
    let (status, body) = get(&app, "/pages/posts/does-not-exist.md").await;
    assert_eq!(status, StatusCode::OK);
    // Should return the editor (with empty content), not panic
    assert!(body.contains("Edit:"), "expected editor page, got: {body}");
}

#[tokio::test]
async fn preview_render_empty_body() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let (status, body) = post_body(&app, "/_render-markdown", "").await;
    assert_eq!(status, StatusCode::OK);
    // Empty input should produce empty (or minimal) output, not error
    assert!(
        !body.contains("panic") && !body.contains("500"),
        "empty preview should not error: {body}"
    );
}

// ── Preview (embedded static server) ───────────────────────────────

#[tokio::test]
async fn preview_root_serves_index_with_livereload() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let out = tmp.path().join("site/public");
    std::fs::write(
        out.join("index.html"),
        "<html><body><h1>Home</h1></body></html>",
    )
    .unwrap();

    let (status, body) = get(&app, "/preview/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<h1>Home</h1>"));
    assert!(
        body.contains("__livereload"),
        "livereload JS must be injected"
    );
}

#[tokio::test]
async fn preview_serves_nested_file() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let out = tmp.path().join("site/public");
    std::fs::create_dir_all(out.join("posts/hello")).unwrap();
    std::fs::write(
        out.join("posts/hello/index.html"),
        "<html><body><p>nested</p></body></html>",
    )
    .unwrap();

    let (status, body) = get(&app, "/preview/posts/hello/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("nested"));
    assert!(body.contains("__livereload"));
}

#[tokio::test]
async fn preview_serves_non_html_asset_without_injection() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let out = tmp.path().join("site/public");
    std::fs::write(out.join("style.css"), "body { color: red; }").unwrap();

    let (status, body) = get(&app, "/preview/style.css").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "body { color: red; }");
    assert!(!body.contains("__livereload"));
}

#[tokio::test]
async fn preview_missing_file_returns_404() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let (status, _body) = get(&app, "/preview/does-not-exist.html").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn preview_blocks_directory_traversal() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    // A sibling file OUTSIDE output_dir that traversal could reach
    std::fs::write(tmp.path().join("site/secret.txt"), "nope").unwrap();

    let (status, _body) = get(&app, "/preview/../secret.txt").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cms_page_includes_livereload_script() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("__livereload"),
        "CMS pages must carry the livereload client"
    );
}

#[tokio::test]
async fn view_site_link_points_at_embedded_preview() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    let (status, body) = get(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains(r#"href="/preview""#),
        "View Site link should target /preview: {}",
        &body[..body.len().min(2000)]
    );
}

#[tokio::test]
async fn livereload_route_rejects_non_upgrade_request() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);
    // Plain GET without the WebSocket upgrade headers should be rejected by axum.
    let (status, _body) = get(&app, "/__livereload").await;
    assert!(
        status == StatusCode::UPGRADE_REQUIRED
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::METHOD_NOT_ALLOWED,
        "non-WS request on /__livereload returned unexpected {status}"
    );
}

#[tokio::test]
async fn config_save_visual_mode_updates_fields() {
    let tmp = TempDir::new().unwrap();
    let app = test_app(&tmp);

    let form = "mode=visual&content=&title=New+Title&base_url=https%3A%2F%2Fnew.example.com&description=A+great+site&theme=ocean&generate_feed=true&generate_sitemap=true";
    let (status, body) = post_form(&app, "/config", form).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("saved") || body.contains("success") || body.contains("New Title"),
        "expected success feedback: {body}"
    );

    let file = std::fs::read_to_string(tmp.path().join("site/config.toml")).unwrap();
    assert!(file.contains("New Title"), "title should be updated");
    assert!(
        file.contains("new.example.com"),
        "base_url should be updated"
    );
    assert!(file.contains("ocean"), "theme should be set");
    assert!(
        file.contains("generate_feed = true"),
        "feed flag should be set"
    );
}

// ── Config partial-update (wave-4 H1) ──────────────────────────────

#[tokio::test]
async fn config_visual_save_preserves_explicit_sitemap_true() {
    // Regression for agent3 H1: a user sets `generate_sitemap = true`
    // explicitly in raw TOML, then saves a different field via the visual
    // tab without touching the sitemap checkbox. The checkbox in the visual
    // form was pre-checked from disk, so the form submission carries
    // `generate_sitemap=true`. The save must round-trip true; the bug was
    // that the handler unconditionally wrote whatever the form said,
    // independent of the on-disk default semantics.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("site");
    std::fs::create_dir_all(root.join("content")).unwrap();
    std::fs::create_dir_all(root.join("templates")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();
    std::fs::write(
        root.join("config.toml"),
        "base_url = \"https://example.com\"\ntitle = \"Test\"\ngenerate_sitemap = true\n",
    )
    .unwrap();
    std::fs::write(root.join("content/_index.md"), "+++\ntitle=\"Home\"\n+++\n").unwrap();
    std::fs::write(root.join("templates/index.html"), "<html>{{section.title}}</html>").unwrap();
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root: root.clone(),
        output_dir: root.join("public"),
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    // The form mirrors what a browser would submit when the user only
    // changed the title: both checkboxes were pre-rendered checked by the
    // editor (because their values are on disk OR effectively-true) and
    // are submitted as "true".
    let form = "mode=visual&content=&title=New+Title&base_url=https%3A%2F%2Fexample.com&description=&theme=&generate_feed=&generate_sitemap=true";
    let _ = post_form(&app, "/config", form).await;

    let file = std::fs::read_to_string(root.join("config.toml")).unwrap();
    assert!(
        file.contains("generate_sitemap = true"),
        "sitemap=true must survive a visual save: {file}"
    );
}

#[tokio::test]
async fn config_visual_save_does_not_stomp_default_sitemap() {
    // The trickier H1 case: user has NO `generate_sitemap` line — they're
    // relying on the zorto-core default (true). The visual form's checkbox
    // must reflect that effective state and submit `generate_sitemap=true`,
    // and the handler must NOT promote that into an explicit `false` (or
    // even an explicit `true`) on save. The field stays absent so future
    // default changes propagate.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("site");
    std::fs::create_dir_all(root.join("content")).unwrap();
    std::fs::create_dir_all(root.join("templates")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();
    // No generate_sitemap line — relying on zorto-core default (true).
    std::fs::write(
        root.join("config.toml"),
        "base_url = \"https://example.com\"\ntitle = \"Test\"\n",
    )
    .unwrap();
    std::fs::write(root.join("content/_index.md"), "+++\ntitle=\"Home\"\n+++\n").unwrap();
    std::fs::write(root.join("templates/index.html"), "<html>{{section.title}}</html>").unwrap();
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root: root.clone(),
        output_dir: root.join("public"),
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    // Browser submits the visual form. Because the checkbox is now rendered
    // from the default-applied Config (sitemap=true), the user sees it
    // checked and the submission carries "true".
    let form = "mode=visual&content=&title=Updated&base_url=https%3A%2F%2Fexample.com&description=&theme=&generate_feed=&generate_sitemap=true";
    let _ = post_form(&app, "/config", form).await;

    let file = std::fs::read_to_string(root.join("config.toml")).unwrap();
    assert!(
        !file.contains("generate_sitemap"),
        "absent-but-default sitemap must remain absent after a no-op save (was promoted to explicit, file: {file})"
    );
    assert!(file.contains("Updated"), "title update must still apply");
}

#[tokio::test]
async fn config_visual_save_unchecks_explicit_true_to_false() {
    // The other side of the partial-update rule: when the user actually
    // unchecks a previously-true boolean, that change must persist. Browser
    // sends NO `generate_feed` field at all when unchecked.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("site");
    std::fs::create_dir_all(root.join("content")).unwrap();
    std::fs::create_dir_all(root.join("templates")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();
    std::fs::write(
        root.join("config.toml"),
        "base_url = \"https://example.com\"\ntitle = \"Test\"\ngenerate_feed = true\n",
    )
    .unwrap();
    std::fs::write(root.join("content/_index.md"), "+++\ntitle=\"Home\"\n+++\n").unwrap();
    std::fs::write(root.join("templates/index.html"), "<html>{{section.title}}</html>").unwrap();
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root: root.clone(),
        output_dir: root.join("public"),
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    // generate_feed is OMITTED entirely — that's how browsers serialize an
    // unchecked checkbox.
    let form = "mode=visual&content=&title=Same&base_url=https%3A%2F%2Fexample.com&description=&theme=&generate_sitemap=true";
    let _ = post_form(&app, "/config", form).await;

    let file = std::fs::read_to_string(root.join("config.toml")).unwrap();
    assert!(
        file.contains("generate_feed = false"),
        "unchecking an explicit true must persist as explicit false (file: {file})"
    );
}

#[tokio::test]
async fn config_visual_save_preserves_unrelated_sections() {
    // Saving via the visual form must not touch [extra], [[taxonomies]],
    // [markdown], or any other table the form doesn't render. Round-trip
    // through toml::Value preserves these — but only if we don't accidentally
    // serialize through a typed struct that drops unknown fields.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("site");
    std::fs::create_dir_all(root.join("content")).unwrap();
    std::fs::create_dir_all(root.join("templates")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();
    let original = r#"base_url = "https://example.com"
title = "Test"

[extra]
twitter = "@example"
analytics_id = "UA-12345"

[markdown]
smart_punctuation = true
external_links_target_blank = true

[[taxonomies]]
name = "categories"
"#;
    std::fs::write(root.join("config.toml"), original).unwrap();
    std::fs::write(root.join("content/_index.md"), "+++\ntitle=\"Home\"\n+++\n").unwrap();
    std::fs::write(root.join("templates/index.html"), "<html>{{section.title}}</html>").unwrap();
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root: root.clone(),
        output_dir: root.join("public"),
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    let form = "mode=visual&content=&title=Updated&base_url=https%3A%2F%2Fexample.com&description=&theme=&generate_feed=&generate_sitemap=true";
    let _ = post_form(&app, "/config", form).await;

    let file = std::fs::read_to_string(root.join("config.toml")).unwrap();
    assert!(file.contains("Updated"), "title update should apply");
    assert!(
        file.contains("twitter") && file.contains("@example"),
        "[extra].twitter must survive: {file}"
    );
    assert!(
        file.contains("UA-12345"),
        "[extra].analytics_id must survive"
    );
    assert!(
        file.contains("smart_punctuation"),
        "[markdown] table must survive"
    );
    assert!(
        file.contains("external_links_target_blank"),
        "[markdown] settings must survive"
    );
    assert!(
        file.contains("[[taxonomies]]") && file.contains("categories"),
        "[[taxonomies]] must survive"
    );
}

#[tokio::test]
async fn config_visual_form_renders_default_sitemap_as_checked() {
    // The render side of the H1 fix: a config without `generate_sitemap`
    // relies on the zorto-core default (true). The visual checkbox must
    // show that — otherwise the UI lies about what the build will do, and
    // the next save bakes the lie into disk.
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("site");
    std::fs::create_dir_all(root.join("content")).unwrap();
    std::fs::create_dir_all(root.join("templates")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();
    std::fs::write(
        root.join("config.toml"),
        "base_url = \"https://example.com\"\ntitle = \"Test\"\n",
    )
    .unwrap();
    std::fs::write(root.join("content/_index.md"), "+++\ntitle=\"Home\"\n+++\n").unwrap();
    std::fs::write(root.join("templates/index.html"), "<html>{{section.title}}</html>").unwrap();
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = Arc::new(AppState {
        root,
        output_dir: tmp.path().join("site/public"),
        sandbox: None,
        reload_tx,
        preview_base_url: "http://127.0.0.1:0/preview".to_string(),
    });
    let app = app(state);

    let (status, body) = get(&app, "/config").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains(r#"name="generate_sitemap" value="true" checked"#),
        "sitemap checkbox must render checked when relying on default"
    );
    assert!(
        !body.contains(r#"name="generate_feed" value="true" checked"#),
        "feed checkbox must render unchecked (default is false)"
    );
}
