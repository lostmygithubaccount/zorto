use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::content::{self, Page, Section, escape_xml};
use crate::execute;
use crate::links;
use crate::markdown;
use crate::sass;
use crate::shortcodes;
use crate::templates::{self, Paginator, TaxonomyTerm};

/// The main entry point for building a zorto site.
///
/// A `Site` is loaded from disk with [`Site::load`], optionally configured
/// (e.g. [`set_base_url`](Self::set_base_url), `no_exec`, `sandbox`), and then
/// built with [`Site::build`].
pub struct Site {
    /// Parsed `config.toml`.
    pub config: Config,
    /// Sections keyed by their relative `_index.md` path.
    pub sections: HashMap<String, Section>,
    /// Pages keyed by their relative `.md` path.
    pub pages: HashMap<String, Page>,
    /// Absolute paths to co-located assets (non-markdown content files).
    pub assets: Vec<PathBuf>,
    /// Absolute path to the site root directory.
    pub root: PathBuf,
    /// Absolute path to the output directory (e.g. `public/`).
    pub output_dir: PathBuf,
    /// Include draft pages in the build.
    pub drafts: bool,
    /// When true, `{python}`/`{bash}`/`{sh}` code blocks are rendered as static
    /// syntax-highlighted code instead of being executed.
    pub no_exec: bool,
    /// Sandbox boundary for file operations (include shortcode, etc.).
    /// Paths cannot escape this directory. Defaults to [`root`](Self::root) if `None`.
    pub sandbox: Option<PathBuf>,
}

impl Site {
    /// Load site from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if `config.toml` is missing or invalid, or the
    /// content directory cannot be walked.
    pub fn load(root: &Path, output_dir: &Path, drafts: bool) -> anyhow::Result<Self> {
        let config = Config::load(root)?;
        let content_dir = root.join("content");

        let loaded = content::load_content(&content_dir, &config.base_url)?;

        Ok(Site {
            config,
            sections: loaded.sections,
            pages: loaded.pages,
            assets: loaded.assets,
            root: root.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            drafts,
            no_exec: false,
            sandbox: None,
        })
    }

    /// Override the base URL and rewrite all permalinks
    pub fn set_base_url(&mut self, new_base_url: String) {
        let old = &self.config.base_url;
        for page in self.pages.values_mut() {
            page.permalink = page.permalink.replacen(old.as_str(), &new_base_url, 1);
        }
        for section in self.sections.values_mut() {
            section.permalink = section.permalink.replacen(old.as_str(), &new_base_url, 1);
        }
        self.config.base_url = new_base_url;
    }

    /// Full build pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if markdown rendering, template rendering, SCSS
    /// compilation, or file I/O fails.
    pub fn build(&mut self) -> anyhow::Result<()> {
        // Filter drafts
        if !self.drafts {
            self.pages.retain(|_, p| !p.draft);
        }

        // Phase 2: RENDER MARKDOWN
        self.render_all_markdown()?;

        // Phase 3: ASSIGN pages to sections (after rendering so content is filled)
        content::assign_pages_to_sections(&mut self.sections, &self.pages);

        // Phase 4: TEMPLATE RENDERING
        let templates_dir = self.root.join("templates");
        let tera = templates::setup_tera(&templates_dir, &self.config, &self.sections)?;
        self.render_templates(&tera)?;

        // Phase 5: ASSETS
        if self.config.compile_sass {
            let sass_dir = self.root.join("sass");
            if sass_dir.exists() {
                sass::compile_sass(&sass_dir, &self.output_dir)?;
            }
        }

        // Copy static files
        let static_dir = self.root.join("static");
        if static_dir.exists() {
            copy_dir_recursive(&static_dir, &self.output_dir)?;
        }

        // Generate sitemap
        if self.config.generate_sitemap {
            self.generate_sitemap()?;
        }

        // Generate feed
        if self.config.generate_feed {
            self.generate_feed()?;
        }

        // Generate llms.txt and llms-full.txt
        if self.config.generate_llms_txt {
            self.generate_llms_txt()?;
            self.generate_llms_full_txt()?;
        }

        // Copy co-located assets
        self.copy_colocated_assets()?;

        Ok(())
    }

    /// Render markdown for all pages and sections
    fn render_all_markdown(&mut self) -> anyhow::Result<()> {
        let shortcode_dir = self.root.join("templates/shortcodes");
        let content_dir = self.root.join("content");

        // Resolve all internal links first (needs full pages + sections maps).
        // Collect resolved content before applying, since resolve_internal_links
        // borrows the full maps immutably.
        let resolved_pages: Vec<(String, String)> = self
            .pages
            .iter()
            .map(|(key, page)| {
                let resolved =
                    links::resolve_internal_links(&page.raw_content, &self.pages, &self.sections)?;
                Ok((key.clone(), resolved))
            })
            .collect::<anyhow::Result<_>>()?;
        for (key, content) in resolved_pages {
            self.pages
                .get_mut(&key)
                .expect("page key was just iterated")
                .raw_content = content;
        }

        let resolved_sections: Vec<(String, String)> = self
            .sections
            .iter()
            .filter(|(_, s)| !s.raw_content.trim().is_empty())
            .map(|(key, section)| {
                let resolved = links::resolve_internal_links(
                    &section.raw_content,
                    &self.pages,
                    &self.sections,
                )?;
                Ok((key.clone(), resolved))
            })
            .collect::<anyhow::Result<_>>()?;
        for (key, content) in resolved_sections {
            self.sections
                .get_mut(&key)
                .expect("section key was just iterated")
                .raw_content = content;
        }

        // Render pages — field-level borrows let us access config/root while
        // iterating pages mutably.
        let config = &self.config;
        let root = &self.root;
        let sandbox = self.sandbox.as_deref().unwrap_or(root);
        let no_exec = self.no_exec;

        for (key, page) in self.pages.iter_mut() {
            let mut raw = std::mem::take(&mut page.raw_content);
            raw = shortcodes::process_shortcodes(&raw, &shortcode_dir, root, sandbox)?;

            let summary_raw = markdown::extract_summary(&raw);
            page.content = render_markdown_content(&raw, key, config, root, &content_dir, no_exec)?;
            page.summary = summary_raw.map(|md| {
                let mut dummy = Vec::new();
                markdown::render_markdown(&md, &config.markdown, &mut dummy, &config.base_url)
            });
            page.raw_content = raw;
        }

        for (key, section) in self.sections.iter_mut() {
            let raw = std::mem::take(&mut section.raw_content);
            if !raw.trim().is_empty() {
                let processed =
                    shortcodes::process_shortcodes(&raw, &shortcode_dir, root, sandbox)?;
                section.content =
                    render_markdown_content(&processed, key, config, root, &content_dir, no_exec)?;
            }
            section.raw_content = raw;
        }

        Ok(())
    }

    /// Render all templates and write output
    fn render_templates(&self, tera: &tera::Tera) -> anyhow::Result<()> {
        // Clean and create output dir
        if self.output_dir.exists() {
            std::fs::remove_dir_all(&self.output_dir)?;
        }
        std::fs::create_dir_all(&self.output_dir)?;

        // Render pages
        for page in self.pages.values() {
            let template_name = "page.html";
            let ctx = templates::page_context(page, &self.config);
            let html = tera.render(template_name, &ctx)?;
            let out_path = self.output_dir.join(page.path.trim_start_matches('/'));
            std::fs::create_dir_all(&out_path)?;
            std::fs::write(out_path.join("index.html"), html)?;

            // Generate alias redirects
            for alias in &page.aliases {
                let alias_path = self.output_dir.join(alias.trim_start_matches('/'));
                std::fs::create_dir_all(&alias_path)?;
                let redirect_html = format!(
                    r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0; url={}"></head><body></body></html>"#,
                    escape_xml(&page.permalink)
                );
                std::fs::write(alias_path.join("index.html"), redirect_html)?;
            }
        }

        // Render sections
        for section in self.sections.values() {
            let template_name = if section.path == "/" {
                "index.html"
            } else {
                "section.html"
            };

            // Render base page (or paginated pages)
            if let Some(paginate_by) = section.paginate_by {
                let total_pages = section.pages.len();
                let num_pagers = total_pages.div_ceil(paginate_by).max(1);

                for pager_idx in 0..num_pagers {
                    let start = pager_idx * paginate_by;
                    let end = (start + paginate_by).min(total_pages);
                    let pager_pages = section.pages[start..end].to_vec();

                    let previous = if pager_idx > 0 {
                        if pager_idx == 1 {
                            Some(section.permalink.clone())
                        } else {
                            Some(format!("{}page/{}/", section.permalink, pager_idx))
                        }
                    } else {
                        None
                    };

                    let next = if pager_idx < num_pagers - 1 {
                        Some(format!("{}page/{}/", section.permalink, pager_idx + 2))
                    } else {
                        None
                    };

                    let paginator = Paginator {
                        pages: pager_pages,
                        current_index: pager_idx + 1,
                        number_pagers: num_pagers,
                        previous,
                        next,
                        first: section.permalink.clone(),
                        last: if num_pagers > 1 {
                            format!("{}page/{}/", section.permalink, num_pagers)
                        } else {
                            section.permalink.clone()
                        },
                    };

                    let ctx = templates::section_context(section, &self.config, Some(&paginator));
                    let html = tera.render(template_name, &ctx)?;

                    let out_path = if pager_idx == 0 {
                        self.output_dir.join(section.path.trim_start_matches('/'))
                    } else {
                        self.output_dir
                            .join(section.path.trim_start_matches('/'))
                            .join("page")
                            .join((pager_idx + 1).to_string())
                    };
                    std::fs::create_dir_all(&out_path)?;
                    std::fs::write(out_path.join("index.html"), html)?;
                }
            } else {
                let ctx = templates::section_context(section, &self.config, None);
                let html = tera.render(template_name, &ctx)?;
                let out_path = self.output_dir.join(section.path.trim_start_matches('/'));
                std::fs::create_dir_all(&out_path)?;
                std::fs::write(out_path.join("index.html"), html)?;
            }
        }

        // Render taxonomy pages
        self.render_taxonomies(tera)?;

        // Render 404
        if tera.get_template_names().any(|n| n == "404.html") {
            let mut ctx = tera::Context::new();
            ctx.insert("config", &templates::config_to_value(&self.config));
            let html = tera.render("404.html", &ctx)?;
            std::fs::write(self.output_dir.join("404.html"), html)?;
        }

        Ok(())
    }

    /// Render taxonomy list and individual term pages
    fn render_taxonomies(&self, tera: &tera::Tera) -> anyhow::Result<()> {
        for tax_config in &self.config.taxonomies {
            let tax_name = &tax_config.name;

            // Collect all terms
            let mut term_map: HashMap<String, Vec<Page>> = HashMap::new();
            for page in self.pages.values() {
                if let Some(terms) = page.taxonomies.get(tax_name) {
                    for term in terms {
                        term_map.entry(term.clone()).or_default().push(page.clone());
                    }
                }
            }

            // Sort pages within each term by date (reverse chronological)
            for pages in term_map.values_mut() {
                content::sort_pages_by_date(pages);
            }

            // Build TaxonomyTerm structs
            let mut terms: Vec<TaxonomyTerm> = term_map
                .into_iter()
                .map(|(name, pages)| {
                    let term_slug = slug::slugify(&name);
                    TaxonomyTerm {
                        permalink: format!("{}/{tax_name}/{term_slug}/", self.config.base_url),
                        slug: term_slug,
                        name,
                        pages,
                    }
                })
                .collect();
            terms.sort_by(|a, b| a.name.cmp(&b.name));

            // Render taxonomy list page
            let list_template = format!("{tax_name}/list.html");
            if tera.get_template_names().any(|n| n == list_template) {
                let ctx = templates::taxonomy_list_context(&terms, &self.config);
                let html = tera.render(&list_template, &ctx)?;
                let out_path = self.output_dir.join(tax_name);
                std::fs::create_dir_all(&out_path)?;
                std::fs::write(out_path.join("index.html"), html)?;
            }

            // Render individual term pages
            let single_template = format!("{tax_name}/single.html");
            if tera.get_template_names().any(|n| n == single_template) {
                for term in &terms {
                    let ctx = templates::taxonomy_single_context(term, &self.config);
                    let html = tera.render(&single_template, &ctx)?;
                    let out_path = self.output_dir.join(tax_name).join(&term.slug);
                    std::fs::create_dir_all(&out_path)?;
                    std::fs::write(out_path.join("index.html"), html)?;
                }
            }
        }

        Ok(())
    }

    /// Validate site without writing output
    pub fn check(&mut self) -> anyhow::Result<()> {
        if !self.drafts {
            self.pages.retain(|_, p| !p.draft);
        }

        self.render_all_markdown()?;
        content::assign_pages_to_sections(&mut self.sections, &self.pages);

        let templates_dir = self.root.join("templates");
        let _tera = templates::setup_tera(&templates_dir, &self.config, &self.sections)?;

        Ok(())
    }

    /// Generate Atom feed
    fn generate_feed(&self) -> anyhow::Result<()> {
        let mut pages: Vec<&Page> = self.pages.values().filter(|p| p.date.is_some()).collect();
        content::sort_pages_by_date_ref(&mut pages);

        let updated = pages
            .first()
            .and_then(|p| p.date.as_deref())
            .unwrap_or("1970-01-01");
        let updated = normalize_date(updated);
        let base = &self.config.base_url;
        let title = escape_xml(&self.config.title);

        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
        let _ = writeln!(xml, "  <title>{title}</title>");
        let _ = writeln!(xml, "  <link href=\"{base}/atom.xml\" rel=\"self\"/>");
        let _ = writeln!(xml, "  <link href=\"{base}/\"/>");
        let _ = writeln!(xml, "  <updated>{updated}</updated>");
        let _ = writeln!(xml, "  <id>{base}/</id>");
        // Atom spec (RFC 4287) requires <author> on the feed or every entry
        if !self.config.title.is_empty() {
            let _ = writeln!(xml, "  <author><name>{title}</name></author>");
        }

        for page in &pages {
            let date = normalize_date(page.date.as_deref().unwrap_or("1970-01-01"));
            let page_title = escape_xml(&page.title);
            let permalink = escape_xml(&page.permalink);

            xml.push_str("  <entry>\n");
            let _ = writeln!(xml, "    <title>{page_title}</title>");
            let _ = writeln!(xml, "    <link href=\"{permalink}\"/>");
            let _ = writeln!(xml, "    <id>{permalink}</id>");
            let _ = writeln!(xml, "    <updated>{date}</updated>");
            if let Some(author) = &page.author {
                let _ = writeln!(
                    xml,
                    "    <author><name>{}</name></author>",
                    escape_xml(author)
                );
            }
            if let Some(summary) = &page.summary {
                let _ = writeln!(
                    xml,
                    "    <summary type=\"html\">{}</summary>",
                    escape_xml(summary)
                );
            } else if let Some(desc) = &page.description {
                let _ = writeln!(xml, "    <summary>{}</summary>", escape_xml(desc));
            }
            xml.push_str("  </entry>\n");
        }

        xml.push_str("</feed>\n");

        std::fs::write(self.output_dir.join("atom.xml"), xml)?;
        Ok(())
    }

    /// Generate sitemap.xml
    fn generate_sitemap(&self) -> anyhow::Result<()> {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        // Sections (sorted by path for deterministic output)
        let mut sorted_sections: Vec<&Section> = self.sections.values().collect();
        sorted_sections.sort_by_key(|s| &s.path);
        for section in &sorted_sections {
            xml.push_str("  <url>\n");
            let _ = writeln!(xml, "    <loc>{}</loc>", escape_xml(&section.permalink));
            xml.push_str("  </url>\n");
        }

        // Pages (sorted by path for deterministic output)
        let mut sorted_pages: Vec<&Page> = self.pages.values().collect();
        sorted_pages.sort_by_key(|p| &p.path);
        for page in &sorted_pages {
            xml.push_str("  <url>\n");
            let _ = writeln!(xml, "    <loc>{}</loc>", escape_xml(&page.permalink));
            if let Some(date) = &page.date {
                let _ = writeln!(xml, "    <lastmod>{date}</lastmod>");
            }
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>\n");

        std::fs::write(self.output_dir.join("sitemap.xml"), xml)?;
        Ok(())
    }

    /// Generate llms.txt — structured index of site content
    fn generate_llms_txt(&self) -> anyhow::Result<()> {
        let mut out = String::new();

        // H1: site title
        let _ = writeln!(out, "# {}", self.config.title);

        // Blockquote: site description
        if !self.config.description.is_empty() {
            let _ = write!(out, "\n> {}\n", self.config.description);
        }

        // Collect pages assigned to sections (to find orphans later)
        let mut section_page_paths: std::collections::HashSet<&str> =
            std::collections::HashSet::new();
        for section in self.sections.values() {
            for page in &section.pages {
                section_page_paths.insert(&page.path);
            }
        }

        // Sort sections: root ("/") first, then alphabetically
        let mut sorted_sections: Vec<&Section> = self.sections.values().collect();
        sorted_sections.sort_by(|a, b| match (a.path.as_str(), b.path.as_str()) {
            ("/", _) => std::cmp::Ordering::Less,
            (_, "/") => std::cmp::Ordering::Greater,
            _ => a.path.cmp(&b.path),
        });

        for section in &sorted_sections {
            let _ = write!(out, "\n## {}\n", section.title);
            if let Some(desc) = &section.description
                && !desc.is_empty()
            {
                let _ = write!(out, "\n{desc}\n");
            }

            // Pages are already sorted by assign_pages_to_sections
            if !section.pages.is_empty() {
                out.push('\n');
                for page in &section.pages {
                    format_page_link(&mut out, page);
                }
            }
        }

        // Orphan pages (not in any section)
        let mut orphans: Vec<&Page> = self
            .pages
            .values()
            .filter(|p| !section_page_paths.contains(p.path.as_str()))
            .collect();
        if !orphans.is_empty() {
            content::sort_pages_by_date_ref(&mut orphans);
            out.push_str("\n## Pages\n\n");
            for page in &orphans {
                format_page_link(&mut out, page);
            }
        }

        std::fs::write(self.output_dir.join("llms.txt"), out)?;
        Ok(())
    }

    /// Generate llms-full.txt — full raw markdown content of all pages
    fn generate_llms_full_txt(&self) -> anyhow::Result<()> {
        let mut out = String::new();

        // H1: site title
        let _ = writeln!(out, "# {}", self.config.title);

        // Blockquote: site description
        if !self.config.description.is_empty() {
            let _ = write!(out, "\n> {}\n", self.config.description);
        }

        // All pages sorted by date (reverse chrono), undated last
        let mut pages: Vec<&Page> = self.pages.values().collect();
        content::sort_pages_by_date_ref(&mut pages);

        for page in &pages {
            let _ = write!(out, "\n## {}\n\n", page.title);
            out.push_str(page.raw_content.trim());
            out.push('\n');
        }

        std::fs::write(self.output_dir.join("llms-full.txt"), out)?;
        Ok(())
    }

    /// Copy co-located assets to their page's output directory
    fn copy_colocated_assets(&self) -> anyhow::Result<()> {
        let content_dir = self.root.join("content");

        for asset_path in &self.assets {
            let relative = asset_path.strip_prefix(&content_dir)?;
            let dest = self.output_dir.join(relative);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(asset_path, &dest)?;
        }

        Ok(())
    }
}

/// Render markdown content: shortcodes → markdown → execute → replace placeholders.
fn render_markdown_content(
    content: &str,
    key: &str,
    config: &Config,
    root: &Path,
    content_dir: &Path,
    no_exec: bool,
) -> anyhow::Result<String> {
    let mut exec_blocks = Vec::new();
    let html = markdown::render_markdown(
        content,
        &config.markdown,
        &mut exec_blocks,
        &config.base_url,
    );

    if !exec_blocks.is_empty() && !no_exec {
        let working_dir = Path::new(key)
            .parent()
            .map(|p| content_dir.join(p))
            .unwrap_or_else(|| content_dir.to_path_buf());
        let errors = execute::execute_blocks(&mut exec_blocks, &working_dir, root);
        for err in &errors {
            eprintln!("warning: {key}: {err}");
        }
    }

    Ok(markdown::replace_exec_placeholders(
        &html,
        &exec_blocks,
        &config.markdown,
    ))
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(src)?;
        let dest = dst.join(relative);

        if path.is_dir() {
            std::fs::create_dir_all(&dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, &dest)?;
        }
    }
    Ok(())
}

/// Format a page as a markdown link with optional description suffix
fn format_page_link(out: &mut String, page: &Page) {
    match page.description.as_deref() {
        Some(desc) if !desc.is_empty() => {
            let _ = writeln!(out, "- [{}]({}): {}", page.title, page.permalink, desc);
        }
        _ => {
            let _ = writeln!(out, "- [{}]({})", page.title, page.permalink);
        }
    }
}

/// Normalize a date string to RFC 3339 (append `T00:00:00Z` if date-only).
///
/// Handles dates with timezone offsets (`+05:00`, `-05:00`), UTC `Z` suffix,
/// and bare datetime strings (no timezone → appends `Z`).
fn normalize_date(s: &str) -> String {
    // Try full RFC 3339 / offset datetime first
    if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
        return s.to_string();
    }
    // Try naive datetime (no timezone)
    if chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok() {
        return format!("{s}Z");
    }
    // Try date-only
    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
        return format!("{s}T00:00:00Z");
    }
    // Fallback: return as-is (shouldn't happen with valid frontmatter)
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a minimal site structure in a tempdir
    fn make_test_site(tmp: &TempDir) -> std::path::PathBuf {
        let root = tmp.path().join("site");
        let content = root.join("content");
        let templates = root.join("templates");
        let static_dir = root.join("static");

        std::fs::create_dir_all(&content).unwrap();
        std::fs::create_dir_all(content.join("posts")).unwrap();
        std::fs::create_dir_all(&templates).unwrap();
        std::fs::create_dir_all(&static_dir).unwrap();

        // Config
        std::fs::write(
            root.join("config.toml"),
            r#"base_url = "https://example.com"
title = "Test Site"
"#,
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

        root
    }

    #[test]
    fn test_site_load() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let site = Site::load(&root, &output, false).unwrap();
        assert_eq!(site.config.base_url, "https://example.com");
        assert!(!site.pages.is_empty());
        assert!(!site.sections.is_empty());
    }

    #[test]
    fn test_set_base_url() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.set_base_url("http://localhost:1111".into());
        assert_eq!(site.config.base_url, "http://localhost:1111");
        for page in site.pages.values() {
            assert!(
                page.permalink.starts_with("http://localhost:1111"),
                "page permalink not rewritten: {}",
                page.permalink
            );
        }
        for section in site.sections.values() {
            assert!(
                section.permalink.starts_with("http://localhost:1111"),
                "section permalink not rewritten: {}",
                section.permalink
            );
        }
    }

    #[test]
    fn test_build_filters_drafts() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        // Before build, draft is present
        assert!(site.pages.values().any(|p| p.draft));
        site.build().unwrap();
        // After build, draft is filtered out
        assert!(!site.pages.values().any(|p| p.draft));
    }

    #[test]
    fn test_build_creates_output() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(output.join("index.html").exists());
        assert!(output.join("posts/hello/index.html").exists());
    }

    #[test]
    fn test_build_copies_static() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(output.join("style.css").exists());
    }

    #[test]
    fn test_build_generates_sitemap_by_default() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(output.join("sitemap.xml").exists());
    }

    #[test]
    fn test_build_sitemap_disabled() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        // Rewrite config to disable sitemap
        std::fs::write(
            root.join("config.toml"),
            r#"base_url = "https://example.com"
title = "Test Site"
generate_sitemap = false
"#,
        )
        .unwrap();
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(!output.join("sitemap.xml").exists());
    }

    #[test]
    fn test_build_generates_llms_txt_by_default() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(output.join("llms.txt").exists());
        assert!(output.join("llms-full.txt").exists());

        let llms = std::fs::read_to_string(output.join("llms.txt")).unwrap();
        assert!(llms.starts_with("# Test Site\n"));
        assert!(llms.contains("## Blog"));
        assert!(llms.contains("[Hello World]"));
        assert!(llms.contains("https://example.com/posts/hello/"));

        let llms_full = std::fs::read_to_string(output.join("llms-full.txt")).unwrap();
        assert!(llms_full.starts_with("# Test Site\n"));
        assert!(llms_full.contains("## Hello World"));
        assert!(llms_full.contains("Hello content"));
    }

    #[test]
    fn test_build_llms_txt_disabled() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        std::fs::write(
            root.join("config.toml"),
            r#"base_url = "https://example.com"
title = "Test Site"
generate_llms_txt = false
"#,
        )
        .unwrap();
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(!output.join("llms.txt").exists());
        assert!(!output.join("llms-full.txt").exists());
    }

    #[test]
    fn test_llms_txt_with_description() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        std::fs::write(
            root.join("config.toml"),
            r#"base_url = "https://example.com"
title = "Test Site"
description = "A site for testing"
"#,
        )
        .unwrap();
        // Add description to a page
        std::fs::write(
            root.join("content/posts/hello.md"),
            "+++\ntitle = \"Hello World\"\ndate = \"2025-01-01\"\ndescription = \"A hello post\"\n+++\nHello content",
        )
        .unwrap();
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        let llms = std::fs::read_to_string(output.join("llms.txt")).unwrap();
        assert!(llms.contains("> A site for testing"));
        assert!(llms.contains(": A hello post"));
    }

    // --- normalize_date ---

    #[test]
    fn test_normalize_date_date_only() {
        assert_eq!(normalize_date("2025-01-15"), "2025-01-15T00:00:00Z");
    }

    #[test]
    fn test_normalize_date_with_utc_z() {
        assert_eq!(
            normalize_date("2025-01-15T10:30:00Z"),
            "2025-01-15T10:30:00Z"
        );
    }

    #[test]
    fn test_normalize_date_bare_datetime() {
        assert_eq!(
            normalize_date("2025-01-15T10:30:00"),
            "2025-01-15T10:30:00Z"
        );
    }

    #[test]
    fn test_normalize_date_positive_offset() {
        assert_eq!(
            normalize_date("2025-01-15T10:30:00+05:00"),
            "2025-01-15T10:30:00+05:00"
        );
    }

    #[test]
    fn test_normalize_date_negative_offset() {
        // This was previously broken — negative offsets were not detected.
        assert_eq!(
            normalize_date("2025-01-15T10:30:00-05:00"),
            "2025-01-15T10:30:00-05:00"
        );
    }
}
