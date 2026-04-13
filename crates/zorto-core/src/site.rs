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

/// Delay before retrying output directory removal during live-reload rebuilds.
/// macOS file handles can temporarily prevent deletion (ENOTEMPTY race).
const BUILD_DEBOUNCE_MS: u64 = 50;

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

        let mut loaded = content::load_content(&content_dir, &config.base_url)?;

        // Load external content directories
        for dir_config in &config.content_dirs {
            let dir_path = root.join(&dir_config.path);
            let external = content::load_content_dir(&dir_path, dir_config, &config.base_url)?;
            // Merge: external content doesn't override manually-authored content
            for (k, v) in external.sections {
                loaded.sections.entry(k).or_insert(v);
            }
            for (k, v) in external.pages {
                loaded.pages.entry(k).or_insert(v);
            }
        }

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

        // Phase 3.5: Remove pages that belong to non-rendering sections.
        // Their rendered content is preserved in section.pages for use in templates
        // (e.g. presentations), but they won't get individual HTML output files.
        let no_render_keys: std::collections::HashSet<&str> = self
            .sections
            .values()
            .filter(|s| !s.render_pages)
            .flat_map(|s| s.pages.iter().map(|p| p.relative_path.as_str()))
            .collect();
        self.pages
            .retain(|k, _| !no_render_keys.contains(k.as_str()));

        // Phase 4: TEMPLATE RENDERING
        let templates_dir = self.root.join("templates");
        let tera = templates::setup_tera(&templates_dir, &self.config, &self.sections)?;
        self.render_templates(&tera)?;

        // Phase 5: ASSETS
        if self.config.compile_sass {
            let sass_dir = self.root.join("sass");
            let theme = self
                .config
                .theme
                .as_deref()
                .and_then(crate::themes::Theme::from_name);
            sass::compile_sass_with_theme(&sass_dir, &self.output_dir, theme.as_ref())?;
            if self.config.compile_all_themes {
                sass::compile_all_theme_styles(&self.output_dir, theme.as_ref())?;
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

        // Generate search index
        #[cfg(feature = "search")]
        if self.config.generate_search {
            crate::search::generate_search_index(
                self.pages.values(),
                self.sections.values(),
                &self.config.base_url,
                &self.output_dir,
            )?;
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
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "internal error: page key '{key}' disappeared during link resolution"
                    )
                })?
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
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "internal error: section key '{key}' disappeared during link resolution"
                    )
                })?
                .raw_content = content;
        }

        // Render pages — field-level borrows let us access config/root while
        // iterating pages mutably.
        let config = &self.config;
        let root = &self.root;
        let sandbox = self.sandbox.as_deref().unwrap_or(root);
        let no_exec = self.no_exec;

        let mut page_keys: Vec<String> = self.pages.keys().cloned().collect();
        page_keys.sort();
        for key in &page_keys {
            let page = self.pages.get_mut(key).unwrap();
            let mut raw = std::mem::take(&mut page.raw_content);
            raw = shortcodes::process_shortcodes(&raw, &shortcode_dir, root, sandbox)?;

            let summary_raw = markdown::extract_summary(&raw);
            page.content = render_markdown_content(
                &raw,
                key,
                config,
                root,
                &content_dir,
                no_exec,
                Some(&page.extra),
            )?;
            page.summary = summary_raw.map(|md| {
                let mut dummy = Vec::new();
                markdown::render_markdown(&md, &config.markdown, &mut dummy, &config.base_url)
            });
            page.raw_content = raw;
        }

        let mut section_keys: Vec<String> = self.sections.keys().cloned().collect();
        section_keys.sort();
        for key in &section_keys {
            let section = self.sections.get_mut(key).unwrap();
            let raw = std::mem::take(&mut section.raw_content);
            if !raw.trim().is_empty() {
                let processed =
                    shortcodes::process_shortcodes(&raw, &shortcode_dir, root, sandbox)?;
                section.content = render_markdown_content(
                    &processed,
                    key,
                    config,
                    root,
                    &content_dir,
                    no_exec,
                    Some(&section.extra),
                )?;
                section.raw_content = processed;
            }
        }

        Ok(())
    }

    /// Render all templates and write output
    fn render_templates(&self, tera: &tera::Tera) -> anyhow::Result<()> {
        // Clean and create output dir.
        // Retry once on failure — during preview mode the dev server may hold
        // file handles that temporarily prevent deletion (macOS "Directory not
        // empty" / ENOTEMPTY race). If the retry also fails, proceed without
        // cleaning so the rebuild still succeeds with overwritten files.
        if self.output_dir.exists() {
            if let Err(_first) = std::fs::remove_dir_all(&self.output_dir) {
                std::thread::sleep(std::time::Duration::from_millis(BUILD_DEBOUNCE_MS));
                if let Err(_second) = std::fs::remove_dir_all(&self.output_dir) {
                    // Could not clean output dir — proceed anyway (files will
                    // be overwritten in place). This avoids hard failures during
                    // live-reload rebuilds.
                }
            }
        }
        std::fs::create_dir_all(&self.output_dir)
            .map_err(|e| anyhow::anyhow!("failed to create {}: {e}", self.output_dir.display()))?;

        // Render pages
        for page in self.pages.values() {
            let template_name = page.template.as_deref().unwrap_or("page.html");
            let ctx = templates::page_context(page, &self.config);
            let html = tera.render(template_name, &ctx)?;
            let out_path = self.output_dir.join(page.path.trim_start_matches('/'));
            std::fs::create_dir_all(&out_path)?;
            std::fs::write(out_path.join("index.html"), html)?;

            // Write .md version (post-shortcode markdown)
            if self.config.generate_md_files {
                let md_path = page.path.trim_start_matches('/').trim_end_matches('/');
                let md_file = if md_path.is_empty() {
                    self.output_dir.join("index.md")
                } else {
                    self.output_dir.join(format!("{md_path}.md"))
                };
                if let Some(parent) = md_file.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let md_content = format!("# {}\n\n{}\n", page.title, page.raw_content.trim());
                std::fs::write(md_file, md_content)?;
            }

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
                section.template.as_deref().unwrap_or("section.html")
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

            // Write .md version for sections with content
            if self.config.generate_md_files && !section.raw_content.trim().is_empty() {
                let md_path = section.path.trim_start_matches('/').trim_end_matches('/');
                let md_file = if md_path.is_empty() {
                    self.output_dir.join("index.md")
                } else {
                    self.output_dir.join(format!("{md_path}.md"))
                };
                if let Some(parent) = md_file.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let md_content = format!("# {}\n\n{}\n", section.title, section.raw_content.trim());
                std::fs::write(md_file, md_content)?;
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

            // Collect all terms.
            // Clones are necessary: we borrow `self.pages` immutably, and a
            // single page may belong to multiple terms, so each entry needs
            // its own owned copy.
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
    /// Check the site for errors and lint warnings.
    ///
    /// When `deny_warnings` is true, lint warnings are promoted to errors.
    pub fn check(&mut self, deny_warnings: bool) -> anyhow::Result<()> {
        if !self.drafts {
            self.pages.retain(|_, p| !p.draft);
        }

        let mut warnings = Vec::new();

        // Lint internal links BEFORE render_all_markdown. The link resolver
        // overwrites raw_content with permalink-resolved text, so the `@/`
        // markers vanish; running this lint after render produces zero hits.
        warnings.extend(crate::lint::lint_internal_links(
            &self.pages,
            &self.sections,
        ));

        // Print accumulated warnings even if render_all_markdown subsequently
        // errors (e.g. on the same broken links the resolver hard-fails on).
        let render_result = self.render_all_markdown();
        for w in &warnings {
            eprintln!("{w}");
        }
        render_result?;

        content::assign_pages_to_sections(&mut self.sections, &self.pages);

        let templates_dir = self.root.join("templates");
        let _tera = templates::setup_tera(&templates_dir, &self.config, &self.sections)?;

        let mut post_render_warnings = Vec::new();
        post_render_warnings.extend(crate::lint::lint_templates(&templates_dir));
        post_render_warnings.extend(crate::lint::lint_frontmatter(&self.pages, &self.sections));
        post_render_warnings.extend(crate::lint::lint_presentation_transitions(
            &self.pages,
            &self.sections,
        ));
        let static_dir = self.root.join("static");
        post_render_warnings.extend(crate::lint::lint_missing_assets(
            &self.pages,
            &self.sections,
            &static_dir,
        ));
        for w in &post_render_warnings {
            eprintln!("{w}");
        }
        warnings.extend(post_render_warnings);

        if deny_warnings && !warnings.is_empty() {
            anyhow::bail!(
                "{} lint warning{} found (--deny-warnings is set)",
                warnings.len(),
                if warnings.len() == 1 { "" } else { "s" }
            );
        }

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
                    format_page_link(&mut out, page, self.config.generate_md_files);
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
                format_page_link(&mut out, page, self.config.generate_md_files);
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
            std::fs::copy(asset_path, &dest).map_err(|e| {
                anyhow::anyhow!("failed to copy asset {}: {e}", asset_path.display())
            })?;
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
    page_extra: Option<&serde_json::Value>,
) -> anyhow::Result<String> {
    use crate::cache;

    let mut exec_blocks = Vec::new();
    let html = markdown::render_markdown(
        content,
        &config.markdown,
        &mut exec_blocks,
        &config.base_url,
    );

    if !exec_blocks.is_empty() && !no_exec {
        // Per-page cache opt-out: [extra] cache = false in frontmatter
        let page_cache_opted_out = page_extra
            .and_then(|extra| extra.get("cache"))
            .and_then(|v| v.as_bool())
            .map(|v| !v)
            .unwrap_or(false);
        let cache_enabled = config.cache.enable && !page_cache_opted_out;
        let page_cache = if cache_enabled {
            cache::load_page_cache(root, key)
        } else {
            None
        };

        let working_dir = Path::new(key)
            .parent()
            .map(|p| content_dir.join(p))
            .filter(|p| p.exists())
            .unwrap_or_else(|| root.to_path_buf());

        let mut new_cache = cache::PageCache::default();
        let mut any_executed = false;

        for (idx, block) in exec_blocks.iter_mut().enumerate() {
            let source_hash = cache::block_cache_key(
                &block.language,
                &block.source,
                block.file_ref.as_deref(),
                &working_dir,
            );
            let idx_key = idx.to_string();

            // Check cache for a hit
            let cache_hit = page_cache
                .as_ref()
                .and_then(|pc| pc.blocks.get(&idx_key).filter(|cb| cb.hash == source_hash));

            if let Some(cached) = cache_hit {
                block.output = cached.output.clone();
                block.error = cached.error.clone();
                block.viz = cached
                    .viz
                    .iter()
                    .map(|(k, d)| execute::VizOutput {
                        kind: k.clone(),
                        data: d.clone(),
                    })
                    .collect();
            } else {
                // Execute this single block
                let errors = execute::execute_blocks(
                    std::slice::from_mut(block),
                    &working_dir,
                    root,
                    config.execute.timeout_seconds,
                );
                for err in &errors {
                    eprintln!("warning: {key}: {err}");
                }
                any_executed = true;
            }

            // Record in new cache regardless
            new_cache.blocks.insert(
                idx_key,
                cache::CachedBlock {
                    hash: source_hash,
                    output: block.output.clone(),
                    error: block.error.clone(),
                    viz: block
                        .viz
                        .iter()
                        .map(|v| (v.kind.clone(), v.data.clone()))
                        .collect(),
                },
            );
        }

        // Write cache if enabled and we executed anything (or cache didn't exist yet)
        if cache_enabled && (any_executed || page_cache.is_none()) {
            if let Err(e) = cache::save_page_cache(root, key, &new_cache) {
                eprintln!("warning: failed to write cache for {key}: {e}");
            }
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
            std::fs::copy(path, &dest).map_err(|e| {
                anyhow::anyhow!(
                    "failed to copy {} -> {}: {e}",
                    path.display(),
                    dest.display()
                )
            })?;
        }
    }
    Ok(())
}

/// Format a page as a markdown link with optional description suffix.
///
/// When `md_links` is true, links point to `.md` versions of pages.
fn format_page_link(out: &mut String, page: &Page, md_links: bool) {
    let url = if md_links {
        let trimmed = page.permalink.trim_end_matches('/');
        format!("{trimmed}.md")
    } else {
        page.permalink.clone()
    };
    match page.description.as_deref() {
        Some(desc) if !desc.is_empty() => {
            let _ = writeln!(out, "- [{}]({url}): {}", page.title, desc);
        }
        _ => {
            let _ = writeln!(out, "- [{}]({url})", page.title);
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

    // --- Full site build pipeline tests ---

    #[test]
    fn test_build_includes_drafts_when_enabled() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, true).unwrap();
        site.build().unwrap();
        // Draft page should be included
        assert!(output.join("posts/draft/index.html").exists());
    }

    #[test]
    fn test_build_excludes_drafts_by_default() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(!output.join("posts/draft/index.html").exists());
    }

    #[test]
    fn test_build_page_content_rendered() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        let html = std::fs::read_to_string(output.join("posts/hello/index.html")).unwrap();
        assert!(html.contains("Hello World"));
        assert!(html.contains("Hello content"));
    }

    #[test]
    fn test_build_section_pages_populated() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        // The posts section HTML should exist
        assert!(output.join("posts/index.html").exists());
    }

    #[test]
    fn test_build_multiple_pages() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        // Add more pages
        std::fs::write(
            root.join("content/posts/second.md"),
            "+++\ntitle = \"Second Post\"\ndate = \"2025-02-01\"\n+++\nSecond content",
        )
        .unwrap();
        std::fs::write(
            root.join("content/posts/third.md"),
            "+++\ntitle = \"Third Post\"\ndate = \"2025-03-01\"\n+++\nThird content",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("posts/hello/index.html").exists());
        assert!(output.join("posts/second/index.html").exists());
        assert!(output.join("posts/third/index.html").exists());
    }

    #[test]
    fn test_build_nested_sections() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Create nested section: posts/tutorials/
        let tutorials = root.join("content/posts/tutorials");
        std::fs::create_dir_all(&tutorials).unwrap();
        std::fs::write(
            tutorials.join("_index.md"),
            "+++\ntitle = \"Tutorials\"\n+++\n",
        )
        .unwrap();
        std::fs::write(
            tutorials.join("intro.md"),
            "+++\ntitle = \"Intro Tutorial\"\n+++\nTutorial content",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("posts/tutorials/index.html").exists());
        assert!(output.join("posts/tutorials/intro/index.html").exists());
    }

    #[test]
    fn test_build_static_nested_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Create nested static files
        let img_dir = root.join("static/img");
        std::fs::create_dir_all(&img_dir).unwrap();
        std::fs::write(img_dir.join("logo.png"), "fake png").unwrap();
        std::fs::write(root.join("static/robots.txt"), "User-agent: *").unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("style.css").exists());
        assert!(output.join("img/logo.png").exists());
        assert!(output.join("robots.txt").exists());
    }

    #[test]
    fn test_build_sass_compilation() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Create sass directory with SCSS (use a unique filename to avoid static/ conflict)
        let sass_dir = root.join("sass");
        std::fs::create_dir_all(&sass_dir).unwrap();
        std::fs::write(
            sass_dir.join("custom.scss"),
            "$color: #333;\nbody { color: $color; }",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        let css_path = output.join("custom.css");
        assert!(css_path.exists());
        let css = std::fs::read_to_string(css_path).unwrap();
        assert!(css.contains("color"));
    }

    #[test]
    fn test_build_sass_disabled() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        std::fs::write(
            root.join("config.toml"),
            "base_url = \"https://example.com\"\ntitle = \"Test\"\ncompile_sass = false\n",
        )
        .unwrap();

        let sass_dir = root.join("sass");
        std::fs::create_dir_all(&sass_dir).unwrap();
        std::fs::write(sass_dir.join("style.scss"), "body { color: red; }").unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        // SCSS should not be compiled
        // (style.css from static/ might exist, but not from sass/)
        // The static style.css was copied, but let's check there's no sass-compiled output
        // by verifying it doesn't contain sass content
        let static_css = std::fs::read_to_string(output.join("style.css")).unwrap();
        assert!(!static_css.contains("color: red"));
    }

    #[test]
    fn test_build_feed_generation() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        std::fs::write(
            root.join("config.toml"),
            "base_url = \"https://example.com\"\ntitle = \"Test\"\ngenerate_feed = true\n",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("atom.xml").exists());
        let atom = std::fs::read_to_string(output.join("atom.xml")).unwrap();
        assert!(atom.contains("https://example.com"));
        assert!(atom.contains("Hello World"));
    }

    #[test]
    fn test_build_feed_disabled_by_default() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();
        assert!(!output.join("atom.xml").exists());
    }

    #[test]
    fn test_build_sitemap_content() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        let sitemap = std::fs::read_to_string(output.join("sitemap.xml")).unwrap();
        assert!(sitemap.contains("https://example.com/posts/hello/"));
        assert!(sitemap.contains("<urlset"));
    }

    #[test]
    fn test_build_page_with_aliases() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        std::fs::write(
            root.join("content/posts/hello.md"),
            "+++\ntitle = \"Hello\"\ndate = \"2025-01-01\"\naliases = [\"/old-hello/\"]\n+++\nContent",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        // Alias redirect should exist
        let alias_html = std::fs::read_to_string(output.join("old-hello/index.html")).unwrap();
        assert!(alias_html.contains("meta http-equiv=\"refresh\""));
        assert!(alias_html.contains("https://example.com/posts/hello/"));
    }

    #[test]
    fn test_build_md_file_generation() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        std::fs::write(
            root.join("config.toml"),
            "base_url = \"https://example.com\"\ntitle = \"Test\"\ngenerate_md_files = true\n",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        let md_path = output.join("posts/hello.md");
        assert!(md_path.exists());
        let md = std::fs::read_to_string(md_path).unwrap();
        assert!(md.contains("# Hello World"));
        assert!(md.contains("Hello content"));
    }

    #[test]
    fn test_build_colocated_content() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Create co-located content
        let post_dir = root.join("content/posts/my-post");
        std::fs::create_dir_all(&post_dir).unwrap();
        std::fs::write(
            post_dir.join("index.md"),
            "+++\ntitle = \"My Post\"\ndate = \"2025-03-01\"\n+++\nCo-located content",
        )
        .unwrap();
        std::fs::write(post_dir.join("diagram.svg"), "<svg></svg>").unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("posts/my-post/index.html").exists());
        // Co-located asset should be copied
        assert!(output.join("posts/my-post/diagram.svg").exists());
    }

    #[test]
    fn test_build_custom_page_template() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Add custom template
        std::fs::write(
            root.join("templates/custom.html"),
            r#"{% extends "base.html" %}{% block content %}CUSTOM: {{ page.title }}{% endblock %}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("content/special.md"),
            "+++\ntitle = \"Special\"\ntemplate = \"custom.html\"\n+++\nContent",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        let html = std::fs::read_to_string(output.join("special/index.html")).unwrap();
        assert!(html.contains("CUSTOM: Special"));
    }

    #[test]
    fn test_build_404_template() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        std::fs::write(
            root.join("templates/404.html"),
            r#"{% extends "base.html" %}{% block content %}Not found{% endblock %}"#,
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("404.html").exists());
        let html = std::fs::read_to_string(output.join("404.html")).unwrap();
        assert!(html.contains("Not found"));
    }

    #[test]
    fn test_build_taxonomy_pages() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Add taxonomy templates (must create the subdirectory)
        let tags_dir = root.join("templates/tags");
        std::fs::create_dir_all(&tags_dir).unwrap();
        std::fs::write(
            tags_dir.join("list.html"),
            r#"{% extends "base.html" %}{% block content %}Tags: {% for term in terms %}{{ term.name }}{% endfor %}{% endblock %}"#,
        )
        .unwrap();
        std::fs::write(
            tags_dir.join("single.html"),
            r#"{% extends "base.html" %}{% block content %}Tag: {{ term.name }}{% endblock %}"#,
        )
        .unwrap();

        // Add tagged page
        std::fs::write(
            root.join("content/posts/hello.md"),
            "+++\ntitle = \"Hello\"\ndate = \"2025-01-01\"\ntags = [\"rust\", \"web\"]\n+++\nTagged content",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("tags/index.html").exists());
        assert!(output.join("tags/rust/index.html").exists());
        assert!(output.join("tags/web/index.html").exists());
    }

    #[test]
    fn test_build_content_dir_scanning() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Set up an external docs directory
        let docs = root.parent().unwrap().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("README.md"), "# Documentation\n\nDocs overview.").unwrap();
        std::fs::write(docs.join("install.md"), "# Installation\n\nInstall steps.").unwrap();

        std::fs::write(
            root.join("config.toml"),
            r#"
base_url = "https://example.com"
title = "Test"

[[content_dirs]]
path = "../docs"
url_prefix = "docs"
"#,
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        // Set sandbox to parent dir so include can access ../docs
        site.sandbox = Some(root.parent().unwrap().to_path_buf());
        site.build().unwrap();

        assert!(output.join("docs/index.html").exists());
        assert!(output.join("docs/install/index.html").exists());
    }

    #[test]
    fn test_build_paginated_section() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Set up paginated section
        std::fs::write(
            root.join("content/posts/_index.md"),
            "+++\ntitle = \"Blog\"\nsort_by = \"date\"\npaginate_by = 1\n+++\n",
        )
        .unwrap();

        // Add another page so we have 2 pages (1 non-draft)
        std::fs::write(
            root.join("content/posts/second.md"),
            "+++\ntitle = \"Second\"\ndate = \"2025-02-01\"\n+++\nSecond",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        // Should have paginated pages
        assert!(output.join("posts/index.html").exists());
        assert!(output.join("posts/page/2/index.html").exists());
    }

    #[test]
    fn test_build_no_static_dir() {
        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);
        // Remove static dir
        std::fs::remove_dir_all(root.join("static")).unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        // Should not error
        site.build().unwrap();
        assert!(output.join("index.html").exists());
    }

    #[test]
    fn test_build_empty_content() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("site");
        let content = root.join("content");
        let templates = root.join("templates");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::create_dir_all(&templates).unwrap();

        std::fs::write(
            root.join("config.toml"),
            r#"base_url = "https://example.com""#,
        )
        .unwrap();
        std::fs::write(content.join("_index.md"), "+++\ntitle = \"Home\"\n+++\n").unwrap();
        std::fs::write(
            templates.join("base.html"),
            "<!DOCTYPE html><html><body>{% block content %}{% endblock %}</body></html>",
        )
        .unwrap();
        std::fs::write(
            templates.join("index.html"),
            r#"{% extends "base.html" %}{% block content %}Home{% endblock %}"#,
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        assert!(output.join("index.html").exists());
    }

    #[test]
    fn test_cache_opt_out_per_page() {
        use crate::cache;

        let tmp = TempDir::new().unwrap();
        let root = make_test_site(&tmp);

        // Enable caching globally
        std::fs::write(
            root.join("config.toml"),
            r#"base_url = "https://example.com"
title = "Test Site"

[cache]
enable = true
"#,
        )
        .unwrap();

        // Page WITH cache opt-out
        std::fs::write(
            root.join("content/posts/no-cache.md"),
            "+++\ntitle = \"No Cache\"\ndate = \"2025-01-02\"\n\n[extra]\ncache = false\n+++\n```{bash}\necho \"not cached\"\n```\n",
        )
        .unwrap();

        // Page WITHOUT cache opt-out (should be cached)
        std::fs::write(
            root.join("content/posts/yes-cache.md"),
            "+++\ntitle = \"Yes Cache\"\ndate = \"2025-01-03\"\n+++\n```{bash}\necho \"cached\"\n```\n",
        )
        .unwrap();

        let output = tmp.path().join("public");
        let mut site = Site::load(&root, &output, false).unwrap();
        site.build().unwrap();

        // The opted-out page should NOT have a cache file
        assert!(
            cache::load_page_cache(&root, "posts/no-cache.md").is_none(),
            "cache should not exist for page with cache = false"
        );

        // The normal page SHOULD have a cache file
        assert!(
            cache::load_page_cache(&root, "posts/yes-cache.md").is_some(),
            "cache should exist for page without cache opt-out"
        );
    }

    #[test]
    fn test_cache_opt_out_extra_field_detection() {
        // Unit test for the opt-out detection pattern used in render_markdown_content
        let extra_with_false = serde_json::json!({"cache": false});
        let extra_with_true = serde_json::json!({"cache": true});
        let extra_without = serde_json::json!({"color": "blue"});
        let extra_empty = serde_json::json!({});

        let check = |extra: &serde_json::Value| -> bool {
            extra
                .get("cache")
                .and_then(|v| v.as_bool())
                .map(|v| !v)
                .unwrap_or(false)
        };

        assert!(check(&extra_with_false), "cache=false should opt out");
        assert!(!check(&extra_with_true), "cache=true should not opt out");
        assert!(
            !check(&extra_without),
            "missing cache key should not opt out"
        );
        assert!(!check(&extra_empty), "empty extra should not opt out");
    }
}
