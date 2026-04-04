//! Search index generation using SQLite.
//!
//! At build time, generates a `search.db` file containing a search index of all
//! site pages. Uses a regular table with LIKE queries for broad compatibility
//! (including sql.js WASM which lacks FTS5 support).

use std::path::Path;

use crate::content::{Page, Section};

/// Strip HTML tags from content, returning plain text.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity = String::new();

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '&' if !in_tag => {
                in_entity = true;
                entity.clear();
                entity.push(ch);
            }
            ';' if in_entity => {
                in_entity = false;
                // Decode common entities
                match entity.as_str() {
                    "&amp" => result.push('&'),
                    "&lt" => result.push('<'),
                    "&gt" => result.push('>'),
                    "&quot" => result.push('"'),
                    "&#39" | "&apos" => result.push('\''),
                    "&nbsp" => result.push(' '),
                    _ => {} // skip unknown entities
                }
            }
            _ if in_entity => entity.push(ch),
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    // Collapse multiple whitespace into single spaces
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_space = false;
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
            }
            prev_space = true;
        } else {
            collapsed.push(ch);
            prev_space = false;
        }
    }

    collapsed.trim().to_string()
}

/// Generate a SQLite search index at `output_dir/search.db`.
///
/// The database contains a `pages` table with columns:
/// `title`, `url`, `description`, `content` (stripped HTML).
/// Uses a regular table (not FTS5) for sql.js WASM compatibility.
pub fn generate_search_index<'a>(
    pages: impl IntoIterator<Item = &'a Page>,
    sections: impl IntoIterator<Item = &'a Section>,
    output_dir: &Path,
) -> anyhow::Result<()> {
    let db_path = output_dir.join("search.db");

    // Remove existing db if present
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let conn = rusqlite::Connection::open(&db_path)?;

    // Create regular table with indexes for LIKE queries
    conn.execute_batch(
        "CREATE TABLE pages (title TEXT, url TEXT, description TEXT, content TEXT);
         CREATE INDEX idx_pages_title ON pages(title);
         CREATE INDEX idx_pages_content ON pages(content);",
    )?;

    let mut stmt = conn
        .prepare("INSERT INTO pages (title, url, description, content) VALUES (?1, ?2, ?3, ?4)")?;

    // Index pages
    for page in pages {
        let plain_text = strip_html(&page.content);
        let description = page.description.as_deref().unwrap_or("");
        stmt.execute(rusqlite::params![
            page.title,
            page.permalink,
            description,
            plain_text
        ])?;
    }

    // Index sections that have content
    for section in sections {
        if section.content.trim().is_empty() {
            continue;
        }
        let plain_text = strip_html(&section.content);
        let description = section.description.as_deref().unwrap_or("");
        stmt.execute(rusqlite::params![
            section.title,
            section.permalink,
            description,
            plain_text
        ])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
    }

    #[test]
    fn test_strip_html_entities() {
        assert_eq!(strip_html("&amp; &lt; &gt;"), "& < >");
    }

    #[test]
    fn test_strip_html_whitespace() {
        assert_eq!(strip_html("<p>  Hello   world  </p>"), "Hello world");
    }

    #[test]
    fn test_strip_html_empty() {
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn test_generate_search_index() {
        use std::collections::HashMap;

        let tmp = tempfile::TempDir::new().unwrap();
        let pages = vec![Page {
            title: "Test Page".to_string(),
            date: None,
            author: None,
            permalink: "https://example.com/test/".to_string(),
            description: Some("A test page".to_string()),
            content: "<p>Hello world</p>".to_string(),
            draft: false,
            slug: "test".to_string(),
            template: None,
            path: "/test/".to_string(),
            summary: None,
            raw_content: String::new(),
            taxonomies: HashMap::new(),
            extra: serde_json::Value::Null,
            aliases: vec![],
            word_count: 2,
            reading_time: 1,
            relative_path: "test.md".to_string(),
        }];
        let sections: Vec<Section> = vec![];

        generate_search_index(pages.iter(), sections.iter(), tmp.path()).unwrap();

        let db_path = tmp.path().join("search.db");
        assert!(db_path.exists());

        // Verify the content
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM pages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // Test LIKE search
        let title: String = conn
            .query_row(
                "SELECT title FROM pages WHERE lower(content) LIKE '%hello%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Test Page");
    }
}
