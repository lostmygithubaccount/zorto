use std::path::Path;
#[cfg(feature = "python")]
use std::sync::Once;

/// A single visualization captured from a Python code block.
#[derive(Debug, Clone)]
pub struct VizOutput {
    /// `"img"` for base64 data-URI images, `"html"` for inline HTML (plotly/altair).
    pub kind: String,
    /// The data-URI string (for img) or raw HTML fragment (for html).
    pub data: String,
}

/// A detected executable code block
#[derive(Debug, Clone)]
pub struct ExecutableBlock {
    pub language: String,
    pub source: String,
    pub file_ref: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub viz: Vec<VizOutput>,
}

/// Execute all pending code blocks for a page.
///
/// Each block's `output` and `error` fields are populated with the execution
/// results. Errors in individual blocks are stored in `block.error` (they are
/// rendered inline as `<div class="code-error">`) and also surfaced via the
/// return value so the caller can decide whether to fail the build.
pub fn execute_blocks(
    blocks: &mut [ExecutableBlock],
    working_dir: &Path,
    site_root: &Path,
) -> Vec<String> {
    let mut errors = Vec::new();

    for block in blocks.iter_mut() {
        match block.language.as_str() {
            "python" => {
                #[cfg(feature = "python")]
                {
                    match execute_python(block, working_dir, site_root) {
                        Ok((stdout, stderr, viz)) => {
                            block.output = Some(stdout);
                            if !stderr.is_empty() {
                                block.error = Some(stderr);
                            }
                            block.viz = viz;
                        }
                        Err(e) => {
                            let msg = format!("Python execution error: {e}");
                            block.error = Some(msg.clone());
                            errors.push(msg);
                        }
                    }
                }
                #[cfg(not(feature = "python"))]
                {
                    let msg =
                        "Python execution not available (built without python feature)".to_string();
                    block.error = Some(msg.clone());
                    errors.push(msg);
                }
            }
            "node" | "javascript" | "js" => match execute_node(block, working_dir) {
                Ok((stdout, stderr)) => {
                    block.output = Some(stdout);
                    if !stderr.is_empty() {
                        block.error = Some(stderr);
                    }
                }
                Err(e) => {
                    let msg = format!("Node.js execution error: {e}");
                    block.error = Some(msg.clone());
                    errors.push(msg);
                }
            },
            "bash" | "sh" => match execute_bash(block, working_dir) {
                Ok((stdout, stderr)) => {
                    block.output = Some(stdout);
                    if !stderr.is_empty() {
                        block.error = Some(stderr);
                    }
                }
                Err(e) => {
                    let msg = format!("Bash execution error: {e}");
                    block.error = Some(msg.clone());
                    errors.push(msg);
                }
            },
            lang => {
                let msg = format!("Unsupported executable language: {lang}");
                block.error = Some(msg.clone());
                errors.push(msg);
            }
        }
    }

    errors
}

/// Find a .venv directory: check site root, walk up parents, then fall back to VIRTUAL_ENV env var
#[cfg(feature = "python")]
pub(crate) fn find_venv(site_root: &Path) -> Option<std::path::PathBuf> {
    // Walk up from site root looking for .venv
    let mut dir = Some(site_root);
    while let Some(d) = dir {
        let candidate = d.join(".venv");
        if candidate.is_dir() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    // Fall back to VIRTUAL_ENV env var (active venv)
    std::env::var("VIRTUAL_ENV")
        .ok()
        .map(std::path::PathBuf::from)
}

/// Activate a venv's site-packages in the embedded Python interpreter (once per process)
#[cfg(feature = "python")]
pub(crate) fn activate_venv(py: pyo3::Python<'_>, site_root: &Path) -> pyo3::PyResult<()> {
    use pyo3::prelude::*;
    static VENV_ACTIVATED: Once = Once::new();
    let venv_dir = match find_venv(site_root) {
        Some(d) => d,
        None => return Ok(()),
    };

    let venv_str = venv_dir.to_string_lossy().to_string();
    VENV_ACTIVATED.call_once(|| {
        let result: pyo3::PyResult<()> = (|| {
            let site_mod = py.import("site")?;

            // Find site-packages by scanning the venv's lib/ directory.
            // We can't use sysconfig because the embedded interpreter's Python version
            // may differ from the venv's Python version.
            let lib_dir = venv_dir.join("lib");
            if let Ok(entries) = std::fs::read_dir(&lib_dir) {
                for entry in entries.flatten() {
                    let sp = entry.path().join("site-packages");
                    if sp.is_dir() {
                        let sp_str = sp.to_string_lossy().to_string();
                        site_mod.call_method1("addsitedir", (&sp_str,))?;
                        eprintln!("zorto: activated venv at {venv_str}");
                        return Ok(());
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("zorto: failed to activate venv: {e}");
        }
    });

    Ok(())
}

/// Execute a Python code block using the embedded PyO3 interpreter.
///
/// # Thread safety
///
/// This function calls `os.chdir()` to set the working directory for the
/// executed code. `chdir` is process-global state, so this is not safe to call
/// from multiple threads concurrently. Page rendering is currently sequential,
/// so this is fine — but must be revisited if parallel rendering is added.
/// Python code injected after user code to detect visualization objects.
///
/// `__main__` persists across all blocks/posts in one build, so we track
/// already-rendered objects in a `WeakSet` keyed by object identity. A block
/// that rebinds `fig = ...` creates a new object → not in the set → rendered;
/// a block that just mutates or re-references an existing rendered object →
/// already in the set → skipped. Weak references avoid `id()` recycling: when
/// the original object is GC'd, its entry auto-clears, so a new object that
/// happens to land at the same id is treated as unseen.
#[cfg(feature = "python")]
const VIZ_DETECTION_CODE: &str = r#"
import sys as _sys, weakref as _weakref
__zorto_internal_viz_output__ = []
if '__zorto_internal_rendered__' not in dir():
    # Fast path for hashable, weak-refable viz types (altair Chart).
    __zorto_internal_rendered__ = _weakref.WeakSet()
    # Fallback for unhashable-but-weak-refable types. plotly's `BaseFigure`
    # defines `__eq__` without `__hash__` (Python auto-drops `__hash__` in
    # that case), so plotly Figures raise TypeError on WeakSet.add() and
    # need an id-keyed weakref map instead. WeakValueDictionary auto-purges
    # when the referent is GC'd, so id-recycling is still handled (the new
    # object at a recycled id finds no live entry → treated as fresh).
    __zorto_internal_rendered_refs__ = _weakref.WeakValueDictionary()
    # Last-resort: neither hashable nor weak-referenceable. Strong id set
    # with a one-time warning per offending type. id-recycling can silently
    # dedup here, but we can't do better without keeping strong refs to
    # every chart forever. Not expected for first-party viz types.
    __zorto_internal_rendered_ids__ = set()
    __zorto_internal_warned_types__ = set()

def _zorto_mark(_obj):
    """Return True if _obj is new (first time seen); record it as seen."""
    # 1. Hashable + weak-refable (e.g. altair Chart): WeakSet is ideal.
    try:
        if _obj in __zorto_internal_rendered__:
            return False
        __zorto_internal_rendered__.add(_obj)
        return True
    except TypeError:
        pass
    # 2. Unhashable but weak-refable (e.g. plotly Figure): id-keyed
    #    WeakValueDictionary preserves id-recycling resistance.
    _oid = id(_obj)
    try:
        _alive = __zorto_internal_rendered_refs__.get(_oid)
        if _alive is _obj:
            return False
        __zorto_internal_rendered_refs__[_oid] = _obj
        return True
    except TypeError:
        pass
    # 3. Last resort — neither hashable nor weak-refable. Warn once.
    _t = type(_obj).__name__
    if _t not in __zorto_internal_warned_types__:
        __zorto_internal_warned_types__.add(_t)
        print(
            f'zorto: warning: viz object of type {_t} is neither hashable '
            f'nor weak-referenceable; id-based dedup may miss id-recycled '
            f'charts across blocks',
            file=_sys.stderr,
        )
    if _oid in __zorto_internal_rendered_ids__:
        return False
    __zorto_internal_rendered_ids__.add(_oid)
    return True

def _zorto_typed_globals(_types):
    """Yield (name, obj) for globals bound to any instance of _types."""
    for _k, _v in list(globals().items()):
        if _k.startswith('__zorto_') or _k in ('_zorto_mark', '_zorto_typed_globals'):
            continue
        if isinstance(_v, _types):
            yield _k, _v

# matplotlib (also covers seaborn). Matplotlib figures live in pyplot's registry
# rather than user globals, so we iterate fignums directly and close after.
if 'matplotlib' in _sys.modules or 'matplotlib.pyplot' in _sys.modules:
    try:
        import matplotlib.pyplot as _plt
        for _fig_num in _plt.get_fignums():
            import io as _io, base64 as _b64
            _buf = _io.BytesIO()
            _plt.figure(_fig_num).savefig(_buf, format='png', bbox_inches='tight', dpi=100)
            _buf.seek(0)
            __zorto_internal_viz_output__.append(('img', 'data:image/png;base64,' + _b64.b64encode(_buf.read()).decode()))
            _buf.close()
    except Exception as _e:
        print(f'zorto: warning: matplotlib capture failed: {_e}', file=_sys.stderr)
    try:
        _plt.close('all')
    except:
        pass

# plotly
if 'plotly' in _sys.modules:
    try:
        import plotly.graph_objects as _go
        for _name, _obj in _zorto_typed_globals(_go.Figure):
            if _zorto_mark(_obj):
                __zorto_internal_viz_output__.append(('html', _obj.to_html(full_html=False, include_plotlyjs='cdn')))
    except Exception as _e:
        print(f'zorto: warning: plotly capture failed: {_e}', file=_sys.stderr)

# altair — embed the spec as a JSON island and load it via JSON.parse.
# The spec is user-reachable (e.g. axis labels from a DataFrame) and can
# contain strings like `</script>`. To prevent HTML-parser breakout, escape
# every `<` as `\u003c` in the serialized JSON — still valid JSON, no `<`
# reaches the HTML tokenizer, works in both inline and JSON-island scripts.
# to_html() returns a full HTML document which breaks embedding; to_dict()
# + vega-embed renders cleanly inline.
if 'altair' in _sys.modules:
    try:
        import altair as _alt, json as _json, uuid as _uuid
        _alt_types = (_alt.Chart,)
        for _cls_name in ('LayerChart', 'HConcatChart', 'VConcatChart'):
            _cls = getattr(_alt, _cls_name, None)
            if _cls is not None:
                _alt_types = _alt_types + (_cls,)
        for _name, _obj in _zorto_typed_globals(_alt_types):
            if not _zorto_mark(_obj):
                continue
            _vid = 'vega-' + _uuid.uuid4().hex
            _sid = _vid + '-spec'
            _spec_json = _json.dumps(_obj.to_dict()).replace('<', '\\u003c')
            # SRI pins on the CDN bundles so a jsdelivr republish or npm
            # compromise can't serve attacker JS to readers. Bump the version
            # → regenerate the hash with `openssl dgst -sha384`.
            _html = (
                '<script src="https://cdn.jsdelivr.net/npm/vega@6.1.2" '
                'integrity="sha384-3Zeq0Gb8jqDivp2a+zwPu5uJJZV+yM0iuorzVFGDPuzChE4tO/3/TecDON0JtQEl" '
                'crossorigin="anonymous"></script>'
                '<script src="https://cdn.jsdelivr.net/npm/vega-lite@6.1.0" '
                'integrity="sha384-YqChHqvOfmjffD8s4u/yesYKLLyylMUVuswYnnT4Xzlo8KtF3u/4Zuu5enUGzFrj" '
                'crossorigin="anonymous"></script>'
                '<script src="https://cdn.jsdelivr.net/npm/vega-embed@7.0.2" '
                'integrity="sha384-MsH1NE1KRDpmMJCcJ8Ulqhadgiz4lT0GIaHN7DKB1POKAoBhSWywPDQ2sJDkP8M7" '
                'crossorigin="anonymous"></script>'
                f'<div id="{_vid}"></div>'
                f'<script type="application/json" id="{_sid}">{_spec_json}</script>'
                f'<script>vegaEmbed('
                f'document.getElementById("{_vid}"), '
                f'JSON.parse(document.getElementById("{_sid}").textContent), '
                f'{{mode: "vega-lite"}});</script>'
            )
            __zorto_internal_viz_output__.append(('html', _html))
    except Exception as _e:
        print(f'zorto: warning: altair capture failed: {_e}', file=_sys.stderr)
"#;

#[cfg(feature = "python")]
fn execute_python(
    block: &ExecutableBlock,
    working_dir: &Path,
    site_root: &Path,
) -> anyhow::Result<(String, String, Vec<VizOutput>)> {
    use pyo3::prelude::*;
    use std::ffi::CString;

    let code = if let Some(ref file) = block.file_ref {
        std::fs::read_to_string(working_dir.join(file))?
    } else {
        block.source.clone()
    };

    let code_cstr = CString::new(code.as_bytes())?;
    let site_root = site_root.to_path_buf();

    let result = Python::attach(
        |py: Python<'_>| -> PyResult<(String, String, Vec<VizOutput>)> {
            // Activate venv if present (once per process)
            activate_venv(py, &site_root)?;

            // Set up stdout/stderr capture
            let sys = py.import("sys")?;
            let io = py.import("io")?;
            let stdout_capture = io.call_method0("StringIO")?;
            let stderr_capture = io.call_method0("StringIO")?;

            let old_stdout = sys.getattr("stdout")?;
            let old_stderr = sys.getattr("stderr")?;

            sys.setattr("stdout", &stdout_capture)?;
            sys.setattr("stderr", &stderr_capture)?;

            // Set working directory
            let os = py.import("os")?;
            os.call_method1("chdir", (working_dir.to_string_lossy().as_ref(),))?;

            // Execute user code
            let exec_result = py.run(code_cstr.as_c_str(), None, None);

            // Detect visualizations (only if user code succeeded)
            let mut viz = Vec::new();
            if exec_result.is_ok() {
                let viz_code = CString::new(VIZ_DETECTION_CODE)?;
                if let Err(e) = py.run(viz_code.as_c_str(), None, None) {
                    eprintln!("zorto: viz detection error: {e}");
                } else {
                    // Read __zorto_internal_viz_output__ from __main__ namespace
                    if let Ok(main_mod) = py.import("__main__") {
                        if let Ok(viz_list) = main_mod.getattr("__zorto_internal_viz_output__") {
                            if let Ok(items) = viz_list.extract::<Vec<(String, String)>>() {
                                for (kind, data) in items {
                                    viz.push(VizOutput { kind, data });
                                }
                            }
                        }
                    }
                }
            }

            // Restore stdout/stderr
            sys.setattr("stdout", &old_stdout)?;
            sys.setattr("stderr", &old_stderr)?;

            // Get captured output
            let stdout: String = stdout_capture.call_method0("getvalue")?.extract()?;
            let stderr: String = stderr_capture.call_method0("getvalue")?.extract()?;

            if let Err(e) = exec_result {
                let err_msg = format!("{stderr}\n{e}");
                Ok((stdout, err_msg.trim().to_string(), viz))
            } else {
                Ok((stdout, stderr, viz))
            }
        },
    )?;

    Ok(result)
}

/// Execute a bash code block
fn execute_bash(block: &ExecutableBlock, working_dir: &Path) -> anyhow::Result<(String, String)> {
    let code = if let Some(ref file) = block.file_ref {
        std::fs::read_to_string(working_dir.join(file))?
    } else {
        block.source.clone()
    };

    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(&code)
        .current_dir(working_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok((stdout, stderr))
}

/// Execute a Node.js code block via `node -e`
fn execute_node(block: &ExecutableBlock, working_dir: &Path) -> anyhow::Result<(String, String)> {
    let code = if let Some(ref file) = block.file_ref {
        std::fs::read_to_string(working_dir.join(file))?
    } else {
        block.source.clone()
    };

    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(&code)
        .current_dir(working_dir)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "Node.js is not installed or not in PATH. \
                     Install it from https://nodejs.org to use {{node}} code blocks."
                )
            } else {
                anyhow::anyhow!("Failed to run node: {e}")
            }
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok((stdout, stderr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_execute_bash_stdout() {
        let tmp = TempDir::new().unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "bash".into(),
            source: "echo hello".into(),
            file_ref: None,
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some("hello\n"));
        assert!(blocks[0].error.is_none());
    }

    #[test]
    fn test_execute_bash_stderr() {
        let tmp = TempDir::new().unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "bash".into(),
            source: "echo oops >&2".into(),
            file_ref: None,
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some(""));
        assert_eq!(blocks[0].error.as_deref(), Some("oops\n"));
    }

    #[test]
    fn test_execute_bash_from_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("script.sh"), "echo from-file").unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "bash".into(),
            source: String::new(),
            file_ref: Some("script.sh".into()),
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some("from-file\n"));
    }

    #[test]
    fn test_execute_node_stdout() {
        let tmp = TempDir::new().unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "node".into(),
            source: "console.log('hello from node')".into(),
            file_ref: None,
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some("hello from node\n"));
        assert!(blocks[0].error.is_none());
    }

    #[test]
    fn test_execute_node_stderr() {
        let tmp = TempDir::new().unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "node".into(),
            source: "console.error('oops')".into(),
            file_ref: None,
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some(""));
        assert_eq!(blocks[0].error.as_deref(), Some("oops\n"));
    }

    #[test]
    fn test_execute_javascript_alias() {
        let tmp = TempDir::new().unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "javascript".into(),
            source: "console.log(1 + 2)".into(),
            file_ref: None,
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some("3\n"));
        assert!(blocks[0].error.is_none());
    }

    #[test]
    fn test_execute_js_alias() {
        let tmp = TempDir::new().unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "js".into(),
            source: "console.log('js alias')".into(),
            file_ref: None,
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some("js alias\n"));
        assert!(blocks[0].error.is_none());
    }

    #[test]
    fn test_execute_node_from_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("script.js"), "console.log('from-file')").unwrap();
        let mut blocks = vec![ExecutableBlock {
            language: "node".into(),
            source: String::new(),
            file_ref: Some("script.js".into()),
            output: None,
            error: None,
            viz: Vec::new(),
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some("from-file\n"));
    }
}
