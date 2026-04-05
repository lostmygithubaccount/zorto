use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::path::PathBuf;

use zorto::core::config::Config as RustConfig;
use zorto::core::content::Page as RustPage;
use zorto::core::content::Section as RustSection;
use zorto::core::site::Site as RustSite;

// --- CLI entry point (existing) ---

#[pyfunction]
fn run_cli(argv: Vec<String>) -> PyResult<()> {
    zorto::run(argv.iter().map(|s| s.as_str()))
        .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))
}

// --- Python API types ---

/// Zorto site configuration.
#[pyclass(frozen, skip_from_py_object, name = "Config")]
#[derive(Clone)]
struct PyConfig {
    inner: RustConfig,
}

#[pymethods]
impl PyConfig {
    /// Site base URL.
    #[getter]
    fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Site title.
    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }

    /// Site description.
    #[getter]
    fn description(&self) -> &str {
        &self.inner.description
    }

    /// Default language code.
    #[getter]
    fn default_language(&self) -> &str {
        &self.inner.default_language
    }

    /// Theme name, if set.
    #[getter]
    fn theme(&self) -> Option<&str> {
        self.inner.theme.as_deref()
    }

    /// Whether SASS compilation is enabled.
    #[getter]
    fn compile_sass(&self) -> bool {
        self.inner.compile_sass
    }

    /// Whether feed generation is enabled.
    #[getter]
    fn generate_feed(&self) -> bool {
        self.inner.generate_feed
    }

    /// Whether sitemap generation is enabled.
    #[getter]
    fn generate_sitemap(&self) -> bool {
        self.inner.generate_sitemap
    }

    /// Whether llms.txt generation is enabled.
    #[getter]
    fn generate_llms_txt(&self) -> bool {
        self.inner.generate_llms_txt
    }

    /// Whether markdown file generation is enabled.
    #[getter]
    fn generate_md_files(&self) -> bool {
        self.inner.generate_md_files
    }

    fn __repr__(&self) -> String {
        format!(
            "Config(base_url={:?}, title={:?})",
            self.inner.base_url, self.inner.title
        )
    }
}

/// A content page.
#[pyclass(frozen, skip_from_py_object, name = "Page")]
#[derive(Clone)]
struct PyPage {
    inner: RustPage,
}

#[pymethods]
impl PyPage {
    /// Page title.
    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }

    /// Publication date string (e.g. "2026-01-15").
    #[getter]
    fn date(&self) -> Option<&str> {
        self.inner.date.as_deref()
    }

    /// Author name.
    #[getter]
    fn author(&self) -> Option<&str> {
        self.inner.author.as_deref()
    }

    /// Short description.
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// Whether this is a draft page.
    #[getter]
    fn draft(&self) -> bool {
        self.inner.draft
    }

    /// URL slug.
    #[getter]
    fn slug(&self) -> &str {
        &self.inner.slug
    }

    /// URL path relative to site root (e.g. "/posts/hello/").
    #[getter]
    fn path(&self) -> &str {
        &self.inner.path
    }

    /// Full permalink including base URL.
    #[getter]
    fn permalink(&self) -> &str {
        &self.inner.permalink
    }

    /// Raw markdown content (after frontmatter extraction).
    #[getter]
    fn raw_content(&self) -> &str {
        &self.inner.raw_content
    }

    /// Rendered HTML content.
    #[getter]
    fn content(&self) -> &str {
        &self.inner.content
    }

    /// Approximate word count.
    #[getter]
    fn word_count(&self) -> usize {
        self.inner.word_count
    }

    /// Estimated reading time in minutes.
    #[getter]
    fn reading_time(&self) -> usize {
        self.inner.reading_time
    }

    /// Source file path relative to the content directory.
    #[getter]
    fn relative_path(&self) -> &str {
        &self.inner.relative_path
    }

    fn __repr__(&self) -> String {
        format!("Page(title={:?}, path={:?})", self.inner.title, self.inner.path)
    }
}

/// A content section.
#[pyclass(frozen, skip_from_py_object, name = "Section")]
#[derive(Clone)]
struct PySection {
    inner: RustSection,
}

#[pymethods]
impl PySection {
    /// Section title.
    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }

    /// Short description.
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// URL path relative to site root (e.g. "/posts/").
    #[getter]
    fn path(&self) -> &str {
        &self.inner.path
    }

    /// Full permalink including base URL.
    #[getter]
    fn permalink(&self) -> &str {
        &self.inner.permalink
    }

    /// Pages belonging to this section.
    #[getter]
    fn pages(&self) -> Vec<PyPage> {
        self.inner
            .pages
            .iter()
            .map(|p| PyPage { inner: p.clone() })
            .collect()
    }

    /// Number of pages in this section.
    fn __len__(&self) -> usize {
        self.inner.pages.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Section(title={:?}, path={:?}, pages={})",
            self.inner.title,
            self.inner.path,
            self.inner.pages.len()
        )
    }
}

/// A loaded Zorto site.
#[pyclass(frozen, name = "Site")]
struct PySite {
    config: RustConfig,
    sections: Vec<RustSection>,
    pages: Vec<RustPage>,
}

#[pymethods]
impl PySite {
    /// Site configuration.
    ///
    /// >>> site = zorto.load("website")
    /// >>> site.config.title
    /// 'zorto'
    #[getter]
    fn config(&self) -> PyConfig {
        PyConfig {
            inner: self.config.clone(),
        }
    }

    /// All sections in the site.
    ///
    /// >>> site = zorto.load("website")
    /// >>> len(site.sections) > 0
    /// True
    #[getter]
    fn sections(&self) -> Vec<PySection> {
        self.sections
            .iter()
            .map(|s| PySection { inner: s.clone() })
            .collect()
    }

    /// All pages in the site.
    #[getter]
    fn pages(&self) -> Vec<PyPage> {
        self.pages
            .iter()
            .map(|p| PyPage { inner: p.clone() })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Site(title={:?}, sections={}, pages={})",
            self.config.title,
            self.sections.len(),
            self.pages.len()
        )
    }
}

// --- Python API functions ---

/// Load a Zorto site from the given root directory.
///
/// Returns a Site object with config, sections, and pages loaded.
/// Content is loaded but not rendered (no HTML generation).
///
/// >>> type(zorto.load("website")).__name__
/// 'Site'
#[pyfunction]
#[pyo3(signature = (root="."))]
fn load(root: &str) -> PyResult<PySite> {
    let root = PathBuf::from(root);
    let site = RustSite::load(&root, &root.join("public"), false)
        .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;

    let mut all_sections: Vec<RustSection> = site.sections.values().cloned().collect();
    all_sections.sort_by(|a, b| a.path.cmp(&b.path));

    let mut all_pages: Vec<RustPage> = site.pages.values().cloned().collect();
    all_pages.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(PySite {
        config: site.config,
        sections: all_sections,
        pages: all_pages,
    })
}

/// Build a Zorto site from the given root directory.
///
/// Equivalent to `zorto build` on the command line.
#[pyfunction]
#[pyo3(signature = (root=".", output_dir=None))]
fn build(root: &str, output_dir: Option<&str>) -> PyResult<()> {
    let root = PathBuf::from(root);
    let output = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("public"));
    let mut site = RustSite::load(&root, &output, false)
        .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;
    site.build()
        .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;
    Ok(())
}

/// Return the zorto version string.
///
/// >>> zorto.version()
/// '0.20.4'
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// --- Module definition ---

#[pymodule]
mod core {
    use super::*;

    #[pymodule_init]
    fn module_init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Functions
        m.add_function(wrap_pyfunction!(run_cli, m)?)?;
        m.add_function(wrap_pyfunction!(load, m)?)?;
        m.add_function(wrap_pyfunction!(build, m)?)?;
        m.add_function(wrap_pyfunction!(version, m)?)?;

        // Classes
        m.add_class::<PySite>()?;
        m.add_class::<PyConfig>()?;
        m.add_class::<PyPage>()?;
        m.add_class::<PySection>()?;

        Ok(())
    }
}
