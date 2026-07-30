//! Internal file verification engine.
//!
//! Provides file-type detection and lightweight syntax/format checking
//! for files edited by Pi Agent. Used by edit, hashline_edit, and write
//! tools when their `verify` parameter is set to true.
//!
//! # Supported file types (MVP)
//!
//! | Extension | Checker | Method | Threshold |
//! |-----------|---------|--------|-----------|
//! | `.rs`     | `rustfmt --check` | external process | ≤1MB |
//! | `.json`   | `serde_json::from_str` | process-internal | unlimited |
//! | `.toml`   | `toml::from_str` | process-internal | unlimited |
//! | `.ts`     | `npx --no-install prettier --check` | external process | ≤1MB |
//!
//! All checkers report only — no automatic correction.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::abort::AbortSignal;
use crate::error::{Error, Result};

/// File type classification based on extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Rust,
    Json,
    Toml,
    TypeScript,
}

/// Single-file verification result.
#[derive(Debug, Clone)]
pub struct VerifyResult {
    pub path: PathBuf,
    pub file_type: FileType,
    pub passed: bool,
    /// Error/warning message (None when passed=true).
    pub message: Option<String>,
    /// Checker name (e.g. "rustfmt", "serde_json", "prettier").
    pub checker: &'static str,
    /// Verification duration in milliseconds.
    pub time_ms: u64,
}

/// Maximum file size for external process verification (1 MiB).
const VERIFY_MAX_EXTERNAL_BYTES: u64 = 1_048_576;

/// External process timeout in seconds.
const VERIFY_TIMEOUT_SECS: u64 = 10;

/// Detect file type from extension.
fn detect_file_type(path: &Path) -> Option<FileType> {
    match path.extension()?.to_str()? {
        "rs" => Some(FileType::Rust),
        "json" => Some(FileType::Json),
        "toml" => Some(FileType::Toml),
        "ts" | "tsx" => Some(FileType::TypeScript),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run verification on a single file.
///
/// Detects the file type by extension, selects the appropriate checker,
/// executes it, and returns the result. External process checkers are
/// wrapped in [`asupersync::runtime::spawn_blocking_io`] to avoid blocking
/// the async runtime, while process-internal checkers run inline.
///
/// # Errors
///
/// Returns `Err` only for truly exceptional cases (file unreadable, I/O
/// failure). Checker failures (syntax errors, formatting differences) are
/// reported as `passed: false` in the [`VerifyResult`].
pub async fn verify_file(path: PathBuf, abort: Option<AbortSignal>) -> Result<VerifyResult> {
    let file_type = detect_file_type(&path).ok_or_else(|| {
        Error::tool(
            "verify",
            format!("Unsupported file type: {}", path.display()),
        )
    })?;

    let start = Instant::now();

    let (passed, message, checker) = match file_type {
        FileType::Json => verify_json(&path)?,
        FileType::Toml => verify_toml(&path)?,
        FileType::Rust => verify_rustfmt(&path, abort).await?,
        FileType::TypeScript => verify_prettier(&path, abort).await?,
    };

    #[allow(clippy::cast_possible_truncation)]
    let time_ms = start.elapsed().as_millis() as u64;

    Ok(VerifyResult {
        path,
        file_type,
        passed,
        message,
        checker,
        time_ms,
    })
}

// ---------------------------------------------------------------------------
// Process-internal checkers (zero extra dependencies, instant)
// ---------------------------------------------------------------------------

/// Check JSON syntax by parsing with serde_json (process-internal).
fn verify_json(path: &Path) -> Result<(bool, Option<String>, &'static str)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::tool("verify", format!("Cannot read {}: {e}", path.display())))?;

    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(_) => Ok((true, None, "serde_json")),
        Err(e) => Ok((false, Some(format!("JSON parse error: {e}")), "serde_json")),
    }
}

/// Check TOML syntax by parsing with toml (process-internal).
fn verify_toml(path: &Path) -> Result<(bool, Option<String>, &'static str)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::tool("verify", format!("Cannot read {}: {e}", path.display())))?;

    match toml::from_str::<toml::Value>(&content) {
        Ok(_) => Ok((true, None, "toml")),
        Err(e) => Ok((false, Some(format!("TOML parse error: {e}")), "toml")),
    }
}

// ---------------------------------------------------------------------------
// External process checkers (wrapped in spawn_blocking_io)
// ---------------------------------------------------------------------------

/// Check Rust formatting by running rustfmt --check.
async fn verify_rustfmt(
    path: &Path,
    abort: Option<AbortSignal>,
) -> Result<(bool, Option<String>, &'static str)> {
    let path = path.to_path_buf();
    let abort = abort.clone();
    asupersync::runtime::spawn_blocking_io(move || {
        verify_rustfmt_sync(&path, abort.as_ref()).map_err(|e| std::io::Error::other(e.to_string()))
    })
    .await
    .map_err(|e| Error::tool("verify", format!("spawn_blocking_io failed: {e}")))
}

fn verify_rustfmt_sync(
    path: &Path,
    abort: Option<&AbortSignal>,
) -> Result<(bool, Option<String>, &'static str)> {
    // Check file size threshold
    let metadata = std::fs::metadata(path).map_err(|e| {
        Error::tool(
            "verify",
            format!("Cannot read metadata for {}: {e}", path.display()),
        )
    })?;

    if metadata.len() > VERIFY_MAX_EXTERNAL_BYTES {
        return Ok((
            true,
            Some(format!("Skipped: file > 1MB ({} bytes)", metadata.len())),
            "rustfmt",
        ));
    }

    // Check if rustfmt is available
    if std::process::Command::new("rustfmt")
        .arg("--version")
        .output()
        .is_err()
    {
        return Ok((
            false,
            Some(
                "rustfmt not found in PATH. Run `rustup component add rustfmt` to install."
                    .to_string(),
            ),
            "rustfmt",
        ));
    }

    // Run rustfmt --check
    let output = run_external_process(
        "rustfmt",
        &["--check", "--edition", "2024", &path.to_string_lossy()],
        abort,
    )?;

    if output.status.success() {
        Ok((true, None, "rustfmt"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((false, Some(stderr), "rustfmt"))
    }
}

/// Check TypeScript formatting by running npx --no-install prettier --check.
async fn verify_prettier(
    path: &Path,
    abort: Option<AbortSignal>,
) -> Result<(bool, Option<String>, &'static str)> {
    let path = path.to_path_buf();
    let abort = abort.clone();
    asupersync::runtime::spawn_blocking_io(move || {
        verify_prettier_sync(&path, abort.as_ref())
            .map_err(|e| std::io::Error::other(e.to_string()))
    })
    .await
    .map_err(|e| Error::tool("verify", format!("spawn_blocking_io failed: {e}")))
}

fn verify_prettier_sync(
    path: &Path,
    abort: Option<&AbortSignal>,
) -> Result<(bool, Option<String>, &'static str)> {
    // Check file size threshold
    let metadata = std::fs::metadata(path).map_err(|e| {
        Error::tool(
            "verify",
            format!("Cannot read metadata for {}: {e}", path.display()),
        )
    })?;

    if metadata.len() > VERIFY_MAX_EXTERNAL_BYTES {
        return Ok((
            true,
            Some(format!("Skipped: file > 1MB ({} bytes)", metadata.len())),
            "prettier",
        ));
    }

    // Check if npx is available
    if std::process::Command::new("npx")
        .arg("--version")
        .output()
        .is_err()
    {
        return Ok((
            false,
            Some(
                "npx not found. Skipping prettier check. \
                 Install Node.js to enable TypeScript formatting verification."
                    .to_string(),
            ),
            "prettier",
        ));
    }

    // Run npx --no-install prettier --check
    let output = run_external_process(
        "npx",
        &[
            "--no-install",
            "prettier",
            "--check",
            &path.to_string_lossy(),
        ],
        abort,
    )?;

    if output.status.success() {
        Ok((true, None, "prettier"))
    } else if let Some(code) = output.status.code() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if code == 2 {
            if stderr.contains("Cannot find module") || stderr.contains("not found") {
                Ok((
                    true,
                    Some(
                        "prettier not cached locally. \
                         Run `npx prettier --check <file>` once to cache it."
                            .to_string(),
                    ),
                    "prettier",
                ))
            } else {
                Ok((false, Some(stderr), "prettier"))
            }
        } else {
            Ok((false, Some(stderr), "prettier"))
        }
    } else {
        Ok((
            false,
            Some("prettier terminated by signal".to_string()),
            "prettier",
        ))
    }
}

// ---------------------------------------------------------------------------
// External process runner with timeout and abort support
// ---------------------------------------------------------------------------

/// Run an external command with timeout and abort signal support.
///
/// Polls the child process at 50ms intervals, checking both the abort signal
/// and the wall-clock timeout.  Returns the captured stdout/stderr on success,
/// or an error on timeout/abort.
fn run_external_process(
    program: &str,
    args: &[&str],
    abort: Option<&AbortSignal>,
) -> Result<std::process::Output> {
    let mut child = std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::tool("verify", format!("Failed to spawn {program}: {e}")))?;

    let start = Instant::now();
    let timeout = std::time::Duration::from_secs(VERIFY_TIMEOUT_SECS);

    loop {
        if let Some(abort) = abort {
            if abort.is_aborted() {
                let _ = child.kill();
                return Err(Error::tool("verify", "Verification aborted by user"));
            }
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            return Err(Error::tool(
                "verify",
                format!("{program} timed out after {VERIFY_TIMEOUT_SECS}s"),
            ));
        }

        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child.wait_with_output().map_err(|e| {
                    Error::tool("verify", format!("Failed to collect {program} output: {e}"))
                })?;
                return Ok(output);
            }
            Ok(None) => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                return Err(Error::tool(
                    "verify",
                    format!("{program} process error: {e}"),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// JSON serialization for embedding in tool output details
// ---------------------------------------------------------------------------

/// Serialize a [`VerifyResult`] into a JSON Value suitable for
/// `ToolOutput.details.verify`.
pub fn verify_result_to_json(result: &VerifyResult) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("passed".to_string(), serde_json::Value::Bool(result.passed));
    map.insert(
        "checker".to_string(),
        serde_json::Value::String(result.checker.to_string()),
    );
    map.insert(
        "fileType".to_string(),
        serde_json::Value::String(format!("{:?}", result.file_type).to_lowercase()),
    );
    map.insert(
        "timeMs".to_string(),
        serde_json::Value::Number(serde_json::Number::from(result.time_ms)),
    );
    if let Some(ref message) = result.message {
        map.insert(
            "message".to_string(),
            serde_json::Value::String(message.clone()),
        );
    }
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_file_type() {
        assert_eq!(detect_file_type(Path::new("foo.rs")), Some(FileType::Rust));
        assert_eq!(
            detect_file_type(Path::new("foo.json")),
            Some(FileType::Json)
        );
        assert_eq!(
            detect_file_type(Path::new("foo.toml")),
            Some(FileType::Toml)
        );
        assert_eq!(
            detect_file_type(Path::new("foo.ts")),
            Some(FileType::TypeScript)
        );
        assert_eq!(
            detect_file_type(Path::new("foo.tsx")),
            Some(FileType::TypeScript)
        );
        assert_eq!(detect_file_type(Path::new("foo.py")), None);
        assert_eq!(detect_file_type(Path::new("foo")), None);
    }

    #[test]
    fn test_verify_json_valid() {
        let mut tmp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(tmp.as_file_mut(), b"{\"a\": 1, \"b\": [2, 3]}").unwrap();
        let (passed, msg, checker) = verify_json(tmp.path()).unwrap();
        assert!(passed, "valid JSON should pass: {:?}", msg);
        assert_eq!(checker, "serde_json");
    }

    #[test]
    fn test_verify_json_invalid() {
        let mut tmp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(tmp.as_file_mut(), b"{invalid}").unwrap();
        let (passed, msg, checker) = verify_json(tmp.path()).unwrap();
        assert!(!passed, "invalid JSON should fail");
        assert!(msg.unwrap().contains("JSON parse error"));
        assert_eq!(checker, "serde_json");
    }

    #[test]
    fn test_verify_json_empty_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(tmp.as_file_mut(), b"").unwrap();
        let (passed, msg, _checker) = verify_json(tmp.path()).unwrap();
        // Empty string is not valid JSON (serde_json expects a value)
        assert!(!passed, "empty file should fail as invalid JSON");
        assert!(msg.unwrap().contains("EOF"));
    }

    #[test]
    fn test_verify_toml_valid() {
        let mut tmp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(tmp.as_file_mut(), b"name = \"test\"\nversion = \"0.1.0\"\n")
            .unwrap();
        let (passed, msg, checker) = verify_toml(tmp.path()).unwrap();
        assert!(passed, "valid TOML should pass: {:?}", msg);
        assert_eq!(checker, "toml");
    }

    #[test]
    fn test_verify_toml_invalid() {
        let mut tmp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(tmp.as_file_mut(), b"key = 'unclosed string\n").unwrap();
        let (passed, msg, checker) = verify_toml(tmp.path()).unwrap();
        assert!(!passed, "invalid TOML should fail");
        assert!(msg.unwrap().contains("TOML parse error"));
        assert_eq!(checker, "toml");
    }
}
