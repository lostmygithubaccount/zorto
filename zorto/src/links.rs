use regex::Regex;
use std::collections::HashMap;

use crate::content::{Page, Section};

/// Resolve @/ internal links in raw markdown content.
/// @/path/to/file.md -> /resolved/url/
/// @/path/to/_index.md -> /section/url/
pub fn resolve_internal_links(
    content: &str,
    pages: &HashMap<String, Page>,
    sections: &HashMap<String, Section>,
    base_url: &str,
) -> String {
    let re = Regex::new(r"@/([^)#\s]+\.md)(#[^)\s]+)?").unwrap();

    re.replace_all(content, |caps: &regex::Captures| {
        let path = &caps[1];
        let anchor = caps.get(2).map_or("", |m| m.as_str());

        // Try pages first
        if let Some(page) = pages.get(path) {
            return format!("{}{anchor}", page.permalink);
        }

        // Try sections
        if let Some(section) = sections.get(path) {
            return format!("{}{anchor}", section.permalink);
        }

        // Warn and keep original
        eprintln!("WARNING: unresolved internal link: @/{path}");
        format!("{base_url}/@/{path}{anchor}")
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Frontmatter, Page, Section, build_page, build_section};
    use std::collections::HashMap;

    fn make_page(relative_path: &str, base_url: &str) -> Page {
        build_page(
            Frontmatter::default(),
            "body".into(),
            relative_path,
            base_url,
            None,
        )
    }

    fn make_section(relative_path: &str, base_url: &str) -> Section {
        build_section(
            Frontmatter::default(),
            "body".into(),
            relative_path,
            base_url,
        )
    }

    #[test]
    fn test_resolve_page_link() {
        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "https://example.com"),
        );
        let sections = HashMap::new();
        let input = "Check out [this post](@/posts/hello.md)";
        let result = resolve_internal_links(input, &pages, &sections, "https://example.com");
        assert!(result.contains("https://example.com/posts/hello/"));
        assert!(!result.contains("@/"));
    }

    #[test]
    fn test_resolve_section_link() {
        let pages = HashMap::new();
        let mut sections = HashMap::new();
        sections.insert(
            "posts/_index.md".into(),
            make_section("posts/_index.md", "https://example.com"),
        );
        let input = "See [blog](@/posts/_index.md)";
        let result = resolve_internal_links(input, &pages, &sections, "https://example.com");
        assert!(result.contains("https://example.com/posts/"));
        assert!(!result.contains("@/"));
    }

    #[test]
    fn test_resolve_with_anchor() {
        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "https://example.com"),
        );
        let sections = HashMap::new();
        let input = "[heading](@/posts/hello.md#section)";
        let result = resolve_internal_links(input, &pages, &sections, "https://example.com");
        assert!(result.contains("https://example.com/posts/hello/#section"));
    }

    #[test]
    fn test_no_internal_links() {
        let pages = HashMap::new();
        let sections = HashMap::new();
        let input = "No [links](https://example.com) here";
        let result = resolve_internal_links(input, &pages, &sections, "https://example.com");
        assert_eq!(result, input);
    }
}
