use std::path::Path;
#[cfg(feature = "python")]
use std::sync::Once;

/// A detected executable code block
#[derive(Debug, Clone)]
pub struct ExecutableBlock {
    pub language: String,
    pub source: String,
    pub file_ref: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
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
                        Ok((stdout, stderr)) => {
                            block.output = Some(stdout);
                            if !stderr.is_empty() {
                                block.error = Some(stderr);
                            }
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
fn find_venv(site_root: &Path) -> Option<std::path::PathBuf> {
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
fn activate_venv(py: pyo3::Python<'_>, site_root: &Path) -> pyo3::PyResult<()> {
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
#[cfg(feature = "python")]
fn execute_python(
    block: &ExecutableBlock,
    working_dir: &Path,
    site_root: &Path,
) -> anyhow::Result<(String, String)> {
    use pyo3::prelude::*;
    use std::ffi::CString;

    let code = if let Some(ref file) = block.file_ref {
        std::fs::read_to_string(working_dir.join(file))?
    } else {
        block.source.clone()
    };

    let code_cstr = CString::new(code.as_bytes())?;
    let site_root = site_root.to_path_buf();

    let result = Python::attach(|py: Python<'_>| -> PyResult<(String, String)> {
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

        // Execute
        let exec_result = py.run(code_cstr.as_c_str(), None, None);

        // Restore stdout/stderr
        sys.setattr("stdout", &old_stdout)?;
        sys.setattr("stderr", &old_stderr)?;

        // Get captured output
        let stdout: String = stdout_capture.call_method0("getvalue")?.extract()?;
        let stderr: String = stderr_capture.call_method0("getvalue")?.extract()?;

        if let Err(e) = exec_result {
            let err_msg = format!("{stderr}\n{e}");
            Ok((stdout, err_msg.trim().to_string()))
        } else {
            Ok((stdout, stderr))
        }
    })?;

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
        }];
        execute_blocks(&mut blocks, tmp.path(), tmp.path());
        assert_eq!(blocks[0].output.as_deref(), Some("from-file\n"));
    }
}
