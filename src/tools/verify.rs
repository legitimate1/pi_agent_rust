//! Internal file verification engine.
//!
//! Provides file-type detection and lightweight syntax/format checking
//! for files edited by Pi Agent. Used by edit, hashline_edit, and write
//! tools when their `verify` parameter is set to true.
//!
//! # Supported file types
//!
//! | Extension | Checker | Method | Threshold |
//! |-----------|---------|--------|-----------|
//! | `.rs`     | `rustfmt --check` | external process | ≤1MB |
//! | `.json`   | `serde_json::from_str` | process-internal | unlimited |
//! | `.toml`   | `toml::from_str` | process-internal | unlimited |
//! | `.ts`     | `prettier --check` (global install; `npx --no-install` fallback) | external process | ≤1MB |
//! | `.md`     | `prettier --check` (same checker as `.ts`) | external process | ≤1MB |
//!
//! # Architecture
//!
//! - **Process-internal checkers** (`json`/`toml`): parse in-process; errors
//!   already carry line/column info. No process, no diff.
//! - **External-process checkers** (`rustfmt`/`prettier`): declared as
//!   [`ExternalChecker`] table entries and executed through one shared
//!   runner ([`run_external_checker`]). Adding a checker means adding a
//!   `FileType` variant + extension mapping + one table entry — no new
//!   boilerplate.
//!
//! Failure messages are normalized: ANSI codes stripped, an optional unified
//! diff appended (via `similar`, when the checker can emit normalized text),
//! and a fix hint added. Messages are capped to avoid flooding tool output.
//!
//! External program names are resolved via [`resolve_program`]: on Windows
//! `npx.cmd`/`prettier.cmd` shims are located because `CreateProcess` cannot
//! spawn bare `.cmd` names; other platforms pass names through unchanged.
//!
//! Checker processes are spawned with stdin set to null: verification is
//! read-only and never consumes input. Inheriting the host's stdin (e.g. the
//! JSONL pipe in Obsidian-hosted mode) makes `cmd.exe`-wrapped shims hang
//! waiting on the pipe's write end, which the host keeps open (#34).
//!
//! All checkers report only — no automatic correction.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use regex::Regex;

use crate::abort::AbortSignal;
use crate::error::{Error, Result};

/// File type classification based on extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Rust,
    Json,
    Toml,
    TypeScript,
    Markdown,
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
        "md" | "markdown" => Some(FileType::Markdown),
        _ => None,
    }
}

/// Resolve a bare program name to a spawnable command on this platform.
///
/// Windows `CreateProcess` cannot spawn extension-less shims or `.cmd`/`.bat`
/// files by bare name (e.g. npm ships `npx` only as `npx.cmd`). We scan PATH
/// for `name.exe` first (directly spawnable), then `name.cmd` / `name.bat`
/// (Rust's `Command` wires those up via the shell shim). Non-Windows builds
/// return the name unchanged, so behavior there is untouched.
fn resolve_program(name: &str) -> String {
    let dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    resolve_program_in_dirs(name, &dirs)
}

/// PATH-scanning core of [`resolve_program`], parameterized for testability
/// (avoids `std::env::set_var`, which is unsafe in Rust 2024).
fn resolve_program_in_dirs(name: &str, dirs: &[PathBuf]) -> String {
    #[cfg(windows)]
    {
        for dir in dirs {
            for ext in ["exe", "cmd", "bat"] {
                let candidate = dir.join(format!("{name}.{ext}"));
                if candidate.is_file() {
                    return candidate.to_string_lossy().into_owned();
                }
            }
        }
        name.to_string()
    }
    #[cfg(not(windows))]
    {
        let _ = (name, dirs);
        name.to_string()
    }
}

// ---------------------------------------------------------------------------
// External-process checker table (declarative)
// ---------------------------------------------------------------------------

/// Declarative definition of an external-process checker.
///
/// All shared behavior (size threshold, availability probe, process run,
/// timeout, failure normalization) lives in [`run_external_checker`]; each
/// checker only declares its own differences here.
struct ExternalChecker {
    /// Checker display name (surfaced in `VerifyResult.checker`).
    name: &'static str,
    /// Bare program name (resolved via [`resolve_program`] for Windows
    /// `.exe`/`.cmd` shims).
    program: &'static str,
    /// Availability probe args (e.g. `--version`).
    version_args: &'static [&'static str],
    /// Message shown when the program is not found in PATH.
    not_found_hint: &'static str,
    /// Check args; the file path is appended by the runner.
    check_args: &'static [&'static str],
    /// Optional fix hint template (`<file>` is substituted with the path).
    fix_hint: &'static str,
    /// Optional formatter args: running `<program> <format_args> <path>`
    /// must print normalized text on stdout. When set, failures append a
    /// unified diff between the original and the formatted text.
    format_args: Option<&'static [&'static str]>,
    /// Optional failure classifier: `Some(warning)` means "soft failure"
    /// (report as passed with a warning, e.g. prettier module not cached);
    /// `None` means every non-zero exit is a hard failure.
    classify_failure: Option<fn(i32, &str) -> Option<String>>,
    /// Fallback checker used when this checker's program cannot be resolved
    /// or spawned (e.g. no global prettier → npx wrapper). `None` = report
    /// not-found. Chains are at most one level deep in practice.
    fallback: Option<&'static Self>,
}

/// rustfmt --check. Its stderr already contains the diff (with ANSI codes),
/// so no separate formatter command is needed.
static RUSTFMT_CHECKER: ExternalChecker = ExternalChecker {
    name: "rustfmt",
    program: "rustfmt",
    version_args: &["--version"],
    not_found_hint: "rustfmt not found in PATH. Run `rustup component add rustfmt` to install.",
    check_args: &["--check", "--edition", "2024"],
    fix_hint: "Run `rustfmt <file>` to fix.",
    format_args: None,
    classify_failure: None,
    fallback: None,
};

/// prettier --check via the global install (resolved to `prettier.cmd` on
/// Windows). The global shim is self-contained and network-free (it runs
/// `node %dp0%\node_modules\prettier\bin\prettier.cjs`), which is ~4x faster
/// than the npx wrapper (~270ms vs ~1.2s) and immune to npm registry stalls
/// that intermittently blew the 10s verify timeout. Falls back to
/// [`NPX_PRETTIER_CHECKER`] when no global prettier is on PATH.
static PRETTIER_CHECKER: ExternalChecker = ExternalChecker {
    name: "prettier",
    program: "prettier",
    version_args: &["--version"],
    not_found_hint: "prettier not found in PATH. Skipping prettier check. \
                     Install prettier globally (`npm i -g prettier`) to enable \
                     TypeScript formatting verification. Falls back to npx when absent.",
    check_args: &["--check"],
    fix_hint: "Run `prettier --write <file>` to fix.",
    format_args: Some(&[]),
    classify_failure: Some(prettier_classify_failure),
    fallback: Some(&NPX_PRETTIER_CHECKER),
};

/// npx --no-install prettier --check — fallback when no global prettier is
/// on PATH. Slower (cmd + node + npx resolution) and can stall on network
/// hiccups (npx probes the npm registry), hence secondary to
/// [`PRETTIER_CHECKER`]. Kept for environments without a global prettier.
static NPX_PRETTIER_CHECKER: ExternalChecker = ExternalChecker {
    name: "npx-prettier",
    program: "npx",
    version_args: &["--version"],
    not_found_hint: "npx not found in PATH. Skipping prettier check. \
                     Install Node.js to enable TypeScript formatting verification \
                     (Windows: ensure `npx.cmd` is on PATH).",
    check_args: &["--no-install", "prettier", "--check"],
    fix_hint: "Run `npx prettier --write <file>` to fix.",
    format_args: Some(&["--no-install", "prettier"]),
    classify_failure: Some(prettier_classify_failure),
    fallback: None,
};

/// prettier failure classification: exit code 2 with a module-not-found
/// message means the module is not cached locally (soft failure, not a
/// formatting problem).
fn prettier_classify_failure(code: i32, stderr: &str) -> Option<String> {
    if code == 2 && (stderr.contains("Cannot find module") || stderr.contains("not found")) {
        Some(
            "prettier not cached locally. \
             Run `npx prettier --check <file>` once to cache it."
                .to_string(),
        )
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Failure message normalization (shared by all external checkers)
// ---------------------------------------------------------------------------

/// Maximum message length; anything longer is truncated with a marker.
const MAX_VERIFY_MESSAGE_CHARS: usize = 8192;

/// Maximum diff length embedded in a failure message.
const MAX_DIFF_CHARS: usize = 6000;

/// Lazily-initialized ANSI escape sequence matcher (same pattern as
/// `conformance.rs`).
static ANSI_REGEX: OnceLock<Regex> = OnceLock::new();

/// Strip ANSI escape sequences (e.g. rustfmt's colored diff) from text.
fn strip_ansi(text: &str) -> String {
    ANSI_REGEX
        .get_or_init(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex"))
        .replace_all(text, "")
        .into_owned()
}

/// Generate a unified diff between original and formatted text, capped at
/// [`MAX_DIFF_CHARS`] with a truncation marker.
fn format_diff(original: &str, formatted: &str) -> String {
    let diff = similar::TextDiff::from_lines(original, formatted)
        .unified_diff()
        .context_radius(3)
        .to_string();
    truncate_at(diff, MAX_DIFF_CHARS, "\n… (diff truncated)")
}

/// Truncate `text` to `max` chars at a UTF-8 boundary, appending `marker`.
fn truncate_at(mut text: String, max: usize, marker: &str) -> String {
    if text.len() > max {
        let mut boundary = max;
        while !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        text.truncate(boundary);
        text.push_str(marker);
    }
    text
}

/// Cap a full failure message at [`MAX_VERIFY_MESSAGE_CHARS`].
fn truncate_message(msg: String) -> String {
    truncate_at(msg, MAX_VERIFY_MESSAGE_CHARS, "\n… (message truncated)")
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
        FileType::Rust => verify_external(&RUSTFMT_CHECKER, &path, abort).await?,
        FileType::TypeScript | FileType::Markdown => {
            verify_external(&PRETTIER_CHECKER, &path, abort).await?
        }
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

/// Run an external-process checker declared in the checker table.
async fn verify_external(
    checker: &'static ExternalChecker,
    path: &Path,
    abort: Option<AbortSignal>,
) -> Result<(bool, Option<String>, &'static str)> {
    let path = path.to_path_buf();
    let abort = abort.clone();
    asupersync::runtime::spawn_blocking_io(move || {
        run_external_checker(checker, &path, abort.as_ref())
            .map_err(|e| std::io::Error::other(e.to_string()))
    })
    .await
    .map_err(|e| Error::tool("verify", format!("spawn_blocking_io failed: {e}")))
}

/// Shared execution path for all external-process checkers:
/// size threshold → program resolve/probe → check run → failure
/// normalization (soft classification, ANSI strip, diff, fix hint, cap).
fn run_external_checker(
    checker: &'static ExternalChecker,
    path: &Path,
    abort: Option<&AbortSignal>,
) -> Result<(bool, Option<String>, &'static str)> {
    run_external_checker_resolved(checker, path, abort, &resolve_program)
}

/// Core of [`run_external_checker`] with an injectable resolver, so tests
/// can simulate missing/available programs without touching the real PATH.
/// When the primary program cannot be spawned, the checker's `fallback`
/// chain is tried before reporting not-found.
fn run_external_checker_resolved(
    checker: &'static ExternalChecker,
    path: &Path,
    abort: Option<&AbortSignal>,
    resolve: &dyn Fn(&str) -> String,
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
            checker.name,
        ));
    }

    // Resolve program (Windows .exe/.cmd shims) and probe availability
    let program = resolve(checker.program);
    if std::process::Command::new(&program)
        .args(checker.version_args)
        .output()
        .is_err()
    {
        // Primary program unavailable: try the fallback chain (e.g. npx when
        // no global prettier), then report not-found only if every level
        // failed.
        if let Some(fallback) = checker.fallback {
            return run_external_checker_resolved(fallback, path, abort, resolve);
        }
        return Ok((
            false,
            Some(checker.not_found_hint.to_string()),
            checker.name,
        ));
    }

    // Run the check: <program> <check_args> <path>
    let path_str = path.to_string_lossy().into_owned();
    let mut check_args: Vec<&str> = checker.check_args.to_vec();
    check_args.push(&path_str);
    let output = run_external_process(&program, &check_args, abort)?;

    if output.status.success() {
        return Ok((true, None, checker.name));
    }

    // Failure handling: rustfmt emits its diff on stdout, prettier emits
    // warnings on stderr — merge both, keeping only diff-looking stdout.
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if let Some(code) = output.status.code() {
        // Soft failure classification (e.g. prettier module not cached)
        if let Some(classify) = checker.classify_failure {
            if let Some(warning) = classify(code, &stderr) {
                return Ok((
                    true,
                    Some(warning.replace("<file>", &path_str)),
                    checker.name,
                ));
            }
        }

        // Hard failure: normalize stderr, append diff if available, add hint
        let mut message = strip_ansi(&stderr);
        if looks_like_diff(&stdout) {
            if !message.trim().is_empty() {
                message.push('\n');
            }
            message.push_str(&strip_ansi(&stdout));
        }
        if let Some(format_args) = checker.format_args {
            if let Some(formatted) = run_formatter(&program, format_args, path, abort) {
                if let Ok(original) = std::fs::read_to_string(path) {
                    message.push_str("\n\n");
                    message.push_str(&format_diff(&original, &formatted));
                }
            }
        }
        message.push_str("\n\n");
        message.push_str(&checker.fix_hint.replace("<file>", &path_str));
        message = truncate_message(message);

        Ok((false, Some(message), checker.name))
    } else {
        Ok((
            false,
            Some(format!("{} terminated by signal", checker.name)),
            checker.name,
        ))
    }
}

/// Heuristic: does this stdout text look like a formatter diff (rustfmt
/// prints `Diff in <path>:` + `-`/`+` lines, unified diffs contain `@@`)?
fn looks_like_diff(text: &str) -> bool {
    text.contains("Diff in")
        || text.contains("@@")
        || text
            .lines()
            .any(|l| l.starts_with('-') || l.starts_with('+'))
}

/// Run `<program> <format_args> <path>` and return stdout (normalized text).
/// Failures are swallowed: diff generation is best-effort only.
fn run_formatter(
    program: &str,
    format_args: &[&str],
    path: &Path,
    abort: Option<&AbortSignal>,
) -> Option<String> {
    let path_str = path.to_string_lossy().into_owned();
    let mut args: Vec<&str> = format_args.to_vec();
    args.push(&path_str);
    let output = run_external_process(program, &args, abort).ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

// ---------------------------------------------------------------------------
// External process runner with timeout and abort support
// ---------------------------------------------------------------------------

/// Run an external command with timeout and abort signal support.
///
/// Polls the child process at 50ms intervals, checking both the abort signal
/// and the wall-clock timeout.  Returns the captured stdout/stderr on success,
/// or an error on timeout/abort. On timeout/abort the whole process tree is
/// terminated (see [`terminate_process_tree`]) so wrapper shells do not leak
/// orphaned grandchildren (e.g. `cmd.exe` → `node.exe`).
fn run_external_process(
    program: &str,
    args: &[&str],
    abort: Option<&AbortSignal>,
) -> Result<std::process::Output> {
    // stdin=null is required (#34): verification is read-only and never
    // consumes input. Inheriting the host's stdin (e.g. the JSONL pipe in
    // Obsidian-hosted mode) makes cmd.exe-wrapped shims (`prettier.cmd`,
    // `npx.cmd`) hang waiting on a pipe whose write end stays open in the
    // host, blowing the 10s timeout. The availability probe below uses
    // `.output()` (which defaults to null stdin) — that asymmetry is why
    // probes never hung while checks did.
    let mut child = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::tool("verify", format!("Failed to spawn {program}: {e}")))?;

    let start = Instant::now();
    let timeout = std::time::Duration::from_secs(VERIFY_TIMEOUT_SECS);

    loop {
        if let Some(abort) = abort {
            if abort.is_aborted() {
                terminate_process_tree(&mut child);
                return Err(Error::tool("verify", "Verification aborted by user"));
            }
        }

        if start.elapsed() >= timeout {
            terminate_process_tree(&mut child);
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

/// Terminate a child process and its descendants.
///
/// On Windows `Child::kill()` only terminates the direct child — for `.cmd`
/// shims that is the `cmd.exe` wrapper, leaving the real worker (e.g.
/// `node.exe` running prettier) orphaned with inherited pipe handles.
/// `taskkill /T` kills the whole tree; the plain kill below remains as a
/// best-effort fallback (and is the only path on non-Windows, where
/// process-group kill would require libc).
fn terminate_process_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .arg("/PID")
            .arg(child.id().to_string())
            .arg("/T")
            .arg("/F")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = child.kill();
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
        assert_eq!(
            detect_file_type(Path::new("foo.md")),
            Some(FileType::Markdown)
        );
        assert_eq!(
            detect_file_type(Path::new("foo.markdown")),
            Some(FileType::Markdown)
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

    // -----------------------------------------------------------------------
    // resolve_program (Windows .cmd shim resolution)
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_program_missing_returns_name() {
        // No match anywhere on any platform: name is returned unchanged,
        // letting the caller surface the original "not found" error path.
        assert_eq!(resolve_program_in_dirs("npx", &[]), "npx");
        assert_eq!(
            resolve_program_in_dirs("rustfmt", &[PathBuf::from("/nonexistent")]),
            "rustfmt"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_resolve_program_windows_prefers_exe_then_cmd() {
        let dir = tempfile::tempdir().unwrap();

        // Only a .cmd shim exists (npm's npx on Windows).
        std::fs::write(dir.path().join("npx.cmd"), "@echo off\necho fake npx\n").unwrap();
        // An .exe exists too; it must win over the .cmd.
        std::fs::write(dir.path().join("rustfmt.exe"), b"fake exe").unwrap();
        std::fs::write(
            dir.path().join("rustfmt.cmd"),
            "@echo off\necho fake rustfmt\n",
        )
        .unwrap();
        // Extension-less shim must NOT match (CreateProcess cannot run it).
        std::fs::write(dir.path().join("prettier"), b"#!node\n").unwrap();

        let dirs = vec![dir.path().to_path_buf()];

        assert_eq!(
            resolve_program_in_dirs("npx", &dirs),
            dir.path().join("npx.cmd").to_string_lossy().into_owned(),
            ".cmd shim should be resolved when no .exe exists"
        );
        assert_eq!(
            resolve_program_in_dirs("rustfmt", &dirs),
            dir.path()
                .join("rustfmt.exe")
                .to_string_lossy()
                .into_owned(),
            ".exe should take precedence over .cmd"
        );
        assert_eq!(
            resolve_program_in_dirs("prettier", &dirs),
            "prettier",
            "extension-less shim must not be resolved"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_resolve_program_cmd_is_spawnable() {
        // Core regression: the resolved .cmd path must actually be spawnable
        // by std::process::Command (this is the Windows failure mode of #30).
        let dir = tempfile::tempdir().unwrap();
        let cmd_path = dir.path().join("fake_tool.cmd");
        std::fs::write(&cmd_path, "@echo off\necho fake-tool-ok\n").unwrap();

        let resolved = resolve_program_in_dirs("fake_tool", &[dir.path().to_path_buf()]);
        assert_eq!(Path::new(&resolved), cmd_path);

        let out = std::process::Command::new(&resolved)
            .output()
            .expect("resolved .cmd path should spawn");
        assert!(out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("fake-tool-ok"),
            "resolved .cmd should actually execute"
        );
    }

    // -----------------------------------------------------------------------
    // Failure message normalization (strip_ansi / format_diff / truncation)
    // -----------------------------------------------------------------------

    #[test]
    fn test_strip_ansi_removes_color_codes() {
        let input = "Diff in file.rs:1:\n\x1b[31m-old line\x1b[0m\n\x1b[32m+new line\x1b[0m\n";
        let out = strip_ansi(input);
        assert!(
            !out.contains("\x1b["),
            "ANSI codes should be stripped: {out:?}"
        );
        assert!(out.contains("-old line"), "diff content preserved");
        assert!(out.contains("+new line"), "diff content preserved");
    }

    #[test]
    fn test_strip_ansi_plain_text_unchanged() {
        assert_eq!(strip_ansi("no codes here"), "no codes here");
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn test_format_diff_shows_added_removed_lines() {
        let original = "fn main() {\nprintln!(\"hi\");\n}\n";
        let formatted = "fn main() {\n    println!(\"hi\");\n}\n";
        let diff = format_diff(original, formatted);
        assert!(diff.contains("-println!(\"hi\");"), "removed line: {diff}");
        assert!(
            diff.contains("+    println!(\"hi\");"),
            "added line: {diff}"
        );
        assert!(!diff.contains("\x1b["), "no ANSI in generated diff");
    }

    #[test]
    fn test_format_diff_no_changes_empty() {
        let text = "a\nb\nc\n";
        let diff = format_diff(text, text);
        // identical text still yields a diff header but no +/- lines
        assert!(!diff.contains("+a"), "no added lines for identical input");
        assert!(!diff.contains("-a"), "no removed lines for identical input");
    }

    #[test]
    fn test_truncate_message_marks_overflow() {
        let msg = "x".repeat(10_000);
        let out = truncate_message(msg);
        assert!(out.len() < 10_000, "should be truncated");
        assert!(out.ends_with("(message truncated)"), "marker appended");
    }

    #[test]
    fn test_truncate_message_short_unchanged() {
        let msg = "short message".to_string();
        assert_eq!(truncate_message(msg), "short message");
    }

    #[test]
    fn test_truncate_at_respects_utf8_boundary() {
        // "é" is 2 bytes; truncating at an odd boundary must not panic
        let msg = "é".repeat(5_000); // 10_000 bytes
        let out = truncate_at(msg, 8_001, "…");
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn test_prettier_classify_failure() {
        // Soft failure: module not cached locally
        let soft = prettier_classify_failure(2, "Cannot find module 'prettier'").unwrap();
        assert!(soft.contains("not cached"), "soft warning text: {soft}");
        assert!(prettier_classify_failure(2, "Error: prettier not found").is_some());

        // Hard failures: other exit codes / messages
        assert_eq!(
            prettier_classify_failure(1, "Code style issues found"),
            None
        );
        assert_eq!(prettier_classify_failure(2, "Some other error"), None);
    }

    // -----------------------------------------------------------------------
    // Fallback chain (primary program missing → npx wrapper)
    // -----------------------------------------------------------------------

    /// Resolver that reports every program as missing (nonexistent path).
    fn resolver_all_missing(name: &str) -> String {
        format!(r"C:\nonexistent\{}", name)
    }

    #[test]
    fn test_prettier_fallback_reports_npx_hint_when_both_missing() {
        // Global prettier missing and npx missing too: the reported checker
        // and hint must come from the fallback (NPX), proving the chain ran.
        let file = tempfile::NamedTempFile::new().unwrap();
        let (passed, msg, checker) = run_external_checker_resolved(
            &PRETTIER_CHECKER,
            file.path(),
            None,
            &resolver_all_missing,
        )
        .unwrap();
        assert!(!passed, "no checker available → not passed");
        assert_eq!(checker, "npx-prettier", "fallback checker name surfaced");
        let msg = msg.unwrap();
        assert!(
            msg.contains("npx not found"),
            "fallback not-found hint expected: {msg}"
        );
        assert!(
            !msg.contains("npm i -g prettier"),
            "primary hint must not leak when fallback also missing"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_prettier_direct_check_via_cmd_shim() {
        // Global prettier.cmd on PATH: the check runs directly through it,
        // no npx involved (the regression scenario of #32).
        let dir = tempfile::tempdir().unwrap();
        let shim = dir.path().join("prettier.cmd");
        std::fs::write(&shim, "@echo off\r\necho fake prettier ok\r\n").unwrap();
        let shim_path = shim.to_string_lossy().into_owned();
        let file = dir.path().join("foo.ts");
        std::fs::write(&file, "const x: number = 1;\n").unwrap();

        let resolve = move |name: &str| -> String {
            if name == "prettier" {
                shim_path.clone()
            } else {
                resolver_all_missing(name)
            }
        };

        let (passed, msg, checker) =
            run_external_checker_resolved(&PRETTIER_CHECKER, &file, None, &resolve).unwrap();
        assert!(passed, "direct check should pass: {:?}", msg);
        assert_eq!(checker, "prettier");
    }

    #[cfg(windows)]
    #[test]
    fn test_verify_spawn_uses_null_stdin() {
        // #34 regression: checker subprocesses must get stdin=null, not the
        // inherited host pipe. The fake .cmd reads one line from stdin:
        // with null stdin, `set /p` hits EOF immediately (NO_INPUT, exit 0);
        // if stdin were inherited from an open pipe/console it would block
        // until the verify timeout kills the process tree.
        let dir = tempfile::tempdir().unwrap();
        let shim = dir.path().join("stdin_probe.cmd");
        std::fs::write(
            &shim,
            "@echo off\r\nset \"line=\"\r\nset /p line=\r\nif not defined line (echo NO_INPUT) else (echo GOT:%line%)\r\nexit /b 0\r\n",
        )
        .unwrap();
        let shim_path = shim.to_string_lossy().into_owned();
        let file = dir.path().join("foo.md");
        std::fs::write(&file, "probe\n").unwrap();

        let resolve = move |name: &str| -> String {
            if name == "prettier" {
                shim_path.clone()
            } else {
                resolver_all_missing(name)
            }
        };

        let (passed, msg, checker) =
            run_external_checker_resolved(&PRETTIER_CHECKER, &file, None, &resolve).unwrap();
        assert!(passed, "stdin probe shim must see EOF: {:?}", msg);
        assert_eq!(checker, "prettier");
    }

    #[cfg(windows)]
    #[test]
    fn test_prettier_falls_back_to_npx_when_global_missing() {
        // No global prettier but npx.cmd present: verify must run through
        // the npx fallback and pass (zero-regression path).
        let dir = tempfile::tempdir().unwrap();
        let npx_shim = dir.path().join("npx.cmd");
        std::fs::write(&npx_shim, "@echo off\r\necho fake npx ok\r\n").unwrap();
        let npx_path = npx_shim.to_string_lossy().into_owned();
        let file = dir.path().join("foo.ts");
        std::fs::write(&file, "const x: number = 1;\n").unwrap();

        let resolve = move |name: &str| -> String {
            if name == "npx" {
                npx_path.clone()
            } else {
                resolver_all_missing(name)
            }
        };

        let (passed, msg, checker) =
            run_external_checker_resolved(&PRETTIER_CHECKER, &file, None, &resolve).unwrap();
        assert!(passed, "npx fallback should pass: {:?}", msg);
        assert_eq!(checker, "npx-prettier");
    }

    #[test]
    fn test_prettier_not_found_no_fallback_checker() {
        // A checker without fallback still reports its own not-found hint.
        let file = tempfile::NamedTempFile::new().unwrap();
        let (passed, msg, checker) = run_external_checker_resolved(
            &NPX_PRETTIER_CHECKER,
            file.path(),
            None,
            &resolver_all_missing,
        )
        .unwrap();
        assert!(!passed);
        assert_eq!(checker, "npx-prettier");
        assert!(msg.unwrap().contains("npx not found"));
    }

    #[test]
    fn test_looks_like_diff() {
        // rustfmt-style diff (stdout)
        assert!(looks_like_diff(
            "Diff in file.rs:1:\n fn main() {\n-x\n+x\n }"
        ));
        // unified diff hunk header
        assert!(looks_like_diff("@@ -1,2 +1,2 @@\n-x\n+x"));
        // plain output (prettier "Checking formatting...") is not a diff
        assert!(!looks_like_diff(
            "Checking formatting...\nAll matched files use Prettier code style!"
        ));
        assert!(!looks_like_diff(""));
        assert!(!looks_like_diff("error: something happened"));
    }
}
