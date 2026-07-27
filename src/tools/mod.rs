//! Built-in tool implementations.
//!
//! Pi provides 8 built-in tools: read, bash, edit, write, grep, find, ls, hashline_edit.
//!
//! Tools are exposed to the model via JSON Schema (see [`crate::provider::ToolDef`]) and executed
//! locally by the agent loop. Each tool returns structured [`ContentBlock`] output suitable for
//! rendering in the TUI and for inclusion in provider messages as tool results.

// Tool sub-modules
mod bash;
mod edit;
mod find;
mod grep;
mod hashline;
mod ls;
mod pwsh;
mod read;
pub(crate) mod verify;
mod write;

pub use bash::{BashRunResult, BashTool};
pub use edit::EditTool;
pub use find::FindTool;
pub use grep::GrepTool;
pub use hashline::HashlineEditTool;
pub use ls::LsTool;
pub use pwsh::{PwshRunResult, PwshTool};
pub use read::ReadTool;
pub use write::WriteTool;

pub(crate) use bash::BashPipeFrame;
pub(crate) use bash::run_bash_command;
#[cfg(test)]
pub(crate) use edit::{
    build_normalized_content, count_overlapping_occurrences, fuzzy_find_text,
    map_normalized_range_to_original,
};
#[cfg(test)]
pub(crate) use grep::{process_rg_json_match_line, truncate_line};
#[cfg(test)]
pub(crate) use hashline::format_hashline_tag;
#[cfg(test)]
pub(crate) use hashline::{
    NIBBLE_STR, compute_line_hash, hashline_tag_regex, parse_hashline_tag, strip_hashline_prefix,
};
#[cfg(test)]
pub(crate) use pwsh::run_pwsh_command;

#[cfg(test)]
mod tests;

pub(crate) use crate::abort::AbortSignal;
use crate::agent_cx::AgentCx;
use crate::config::Config;
use asupersync::io::{AsyncReadExt, AsyncWriteExt};
use asupersync::time::{sleep, wall_now};
use async_trait::async_trait;
use pi_core::model::{ContentBlock, ImageContent, TextContent};
use pi_core::path_utils::{safe_canonicalize, strip_unc_prefix};
use pi_core::tool_config::ToolConfig;
use pi_provider_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

// ============================================================================
// Tool Trait
// ============================================================================

/// Coarse side-effect declaration for tool scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolEffects {
    bits: u8,
}

impl ToolEffects {
    const READ: u8 = 1 << 0;
    const WRITE: u8 = 1 << 1;
    const APPEND: u8 = 1 << 2;
    const NETWORK: u8 = 1 << 3;
    const PROCESS: u8 = 1 << 4;
    const BARRIER: u8 = Self::WRITE | Self::APPEND | Self::PROCESS;

    /// Tool reads local state without mutating it.
    #[must_use]
    pub const fn read() -> Self {
        Self { bits: Self::READ }
    }

    /// Tool may create, replace, or otherwise mutate local state.
    #[must_use]
    pub const fn write() -> Self {
        Self { bits: Self::WRITE }
    }

    /// Tool appends to existing local state.
    #[must_use]
    pub const fn append() -> Self {
        Self { bits: Self::APPEND }
    }

    /// Tool performs network I/O but does not mutate local state.
    #[must_use]
    pub const fn network() -> Self {
        Self {
            bits: Self::NETWORK,
        }
    }

    /// Tool starts a local process. This is treated as a scheduling barrier.
    #[must_use]
    pub const fn process() -> Self {
        Self {
            bits: Self::PROCESS,
        }
    }

    /// Combine multiple effect declarations for a single tool or batch.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    /// Whether this declaration reads local state.
    #[must_use]
    pub const fn reads(self) -> bool {
        self.bits & Self::READ != 0
    }

    /// Whether this declaration may mutate local state by replacing content.
    #[must_use]
    pub const fn writes(self) -> bool {
        self.bits & Self::WRITE != 0
    }

    /// Whether this declaration may append to local state.
    #[must_use]
    pub const fn appends(self) -> bool {
        self.bits & Self::APPEND != 0
    }

    /// Whether this declaration performs network I/O.
    #[must_use]
    pub const fn networks(self) -> bool {
        self.bits & Self::NETWORK != 0
    }

    /// Whether this declaration starts or controls a local process.
    #[must_use]
    pub const fn processes(self) -> bool {
        self.bits & Self::PROCESS != 0
    }

    /// Stable labels for machine-readable scheduling evidence.
    #[must_use]
    pub fn labels(self) -> Vec<&'static str> {
        let mut labels = Vec::with_capacity(5);
        if self.reads() {
            labels.push("read");
        }
        if self.writes() {
            labels.push("write");
        }
        if self.appends() {
            labels.push("append");
        }
        if self.networks() {
            labels.push("network");
        }
        if self.processes() {
            labels.push("process");
        }
        labels
    }

    /// Whether this effect set can run in a compatible concurrent batch.
    #[must_use]
    pub const fn parallel_safe(self) -> bool {
        self.bits != 0 && self.bits & Self::BARRIER == 0
    }

    /// Whether two effect sets can share a concurrent batch.
    #[must_use]
    pub const fn compatible_with(self, other: Self) -> bool {
        self.parallel_safe() && other.parallel_safe()
    }
}

/// A tool that can be executed by the agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;

    /// Get the tool label (display name).
    fn label(&self) -> &str;

    /// Get the tool description.
    fn description(&self) -> &str;

    /// Get the tool parameters as JSON Schema.
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool.
    ///
    /// Tools may call `on_update` to stream incremental results (e.g. while a long-running `bash`
    /// command is still producing output). The final return value is a [`ToolOutput`] which is
    /// persisted into the session as a tool result message.
    ///
    /// The `abort` parameter provides a cancellation signal. Long-running tools (bash, pwsh)
    /// should check `abort.is_aborted()` periodically and stop execution with a clean error.
    async fn execute(
        &self,
        tool_call_id: &str,
        input: serde_json::Value,
        on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
        abort: Option<AbortSignal>,
    ) -> Result<ToolOutput>;

    /// Declare the coarse side effects used by the agent scheduler.
    ///
    /// Defaults to local write effects so undeclared tools are serialized fail-closed.
    #[must_use]
    fn effects(&self) -> ToolEffects {
        ToolEffects::write()
    }
}

/// Tool execution output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolOutput {
    pub content: Vec<ContentBlock>,
    pub details: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_error: bool,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde requires `fn(&bool) -> bool` for `skip_serializing_if`
const fn is_false(value: &bool) -> bool {
    !*value
}

/// Incremental update during tool execution.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUpdate {
    pub content: Vec<ContentBlock>,
    pub details: Option<serde_json::Value>,
}

// ============================================================================
// Truncation
// ============================================================================

/// Default maximum lines for truncation.
pub const DEFAULT_MAX_LINES: usize = 2000;

/// Default maximum bytes for truncation.
pub const DEFAULT_MAX_BYTES: usize = 1_000_000; // 1MB

/// Maximum line length for grep results.
pub const GREP_MAX_LINE_LENGTH: usize = 500;

/// Default grep result limit.
pub const DEFAULT_GREP_LIMIT: usize = 100;

/// Default find result limit.
pub const DEFAULT_FIND_LIMIT: usize = 1000;

/// Default ls result limit.
pub const DEFAULT_LS_LIMIT: usize = 500;

/// Hard limit for directory scanning in ls tool to prevent OOM/hangs.
pub const LS_SCAN_HARD_LIMIT: usize = 20_000;

/// Hard limit for read tool file size (100MB) to prevent OOM.
pub const READ_TOOL_MAX_BYTES: u64 = 100 * 1024 * 1024;

/// Hard limit for write/edit tool file size (100MB) to prevent OOM.
pub const WRITE_TOOL_MAX_BYTES: usize = 100 * 1024 * 1024;

/// Maximum size for an image to be sent to the API (4.5MB).
pub const IMAGE_MAX_BYTES: usize = 4_718_592;

/// Default timeout (in seconds) for bash tool execution.
pub const DEFAULT_BASH_TIMEOUT_SECS: u64 = 120;

const BASH_TERMINATE_GRACE_SECS: u64 = 5;
const BASH_CANCELLATION_SCHEMA_V1: &str = "pi.tool.bash.cancellation.v1";

/// Hard limit for bash output file size (1GB) to prevent disk exhaustion DoS.
pub(crate) const BASH_FILE_LIMIT_BYTES: usize = 1024 * 1024 * 1024; // 1 GiB

const TOOL_OUTPUT_ARTIFACT_SCHEMA_V1: &str = "pi.tool_output_artifact.v1";
const TOOL_OUTPUT_ARTIFACT_REDACTION_POLICY_V1: &str = "pi.tool_output_artifact.redaction.v1";
const TOOL_OUTPUT_ARTIFACT_RETENTION_CLASS: &str = "session_scoped_temp_evidence";
const TOOL_OUTPUT_ARTIFACT_SPILLOVER_REASON: &str = "sourceBytesExceededPreviewThreshold";
const TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES: usize = DEFAULT_MAX_BYTES;
const TOOL_OUTPUT_ARTIFACT_REDACTION_MAX_BYTES_USIZE: usize = 64 * 1024 * 1024;
const TOOL_OUTPUT_ARTIFACT_REDACTION_MAX_BYTES: u64 = 64 * 1024 * 1024;
const TOOL_OUTPUT_ARTIFACT_MAX_BYTES_USIZE: usize = 1024 * 1024 * 1024;
const TOOL_OUTPUT_ARTIFACT_MAX_BYTES: u64 = 1024 * 1024 * 1024;

/// Result of truncation operation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TruncationResult {
    pub content: String,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated_by: Option<TruncatedBy>,
    pub total_lines: usize,
    pub total_bytes: usize,
    pub output_lines: usize,
    pub output_bytes: usize,
    pub last_line_partial: bool,
    pub first_line_exceeds_limit: bool,
    pub max_lines: usize,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TruncatedBy {
    Lines,
    Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BashCancellationReason {
    Timeout,
    AmbientCancellation,
}

impl BashCancellationReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::AmbientCancellation => "ambient_cancellation",
        }
    }
}

/// Detect text encoding from file contents.
///
/// Checks BOM signatures first, then validates UTF-8 via round-trip.
/// Returns a label like `"UTF-8"`, `"UTF-16 LE"`, `"UTF-16 BE"`, or
/// `"UTF-8 (BOM)"`.  Also returns the initial byte-slice that should be
/// skipped (the BOM length) for subsequent decoding.
pub(crate) fn detect_encoding(bytes: &[u8], hint: Option<&str>) -> (String, usize) {
    if let Some(h) = hint {
        return (h.to_string(), 0);
    }
    // BOM detection
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return ("UTF-8 (BOM)".to_string(), 3);
    }
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        return ("UTF-16 LE".to_string(), 2);
    }
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return ("UTF-16 BE".to_string(), 2);
    }
    // UTF-8 round-trip validation
    if let Ok(s) = std::str::from_utf8(bytes) {
        let re_encoded = s.as_bytes();
        #[allow(clippy::cast_precision_loss)]
        if (re_encoded.len() as f64) >= (bytes.len() as f64) * 0.95 {
            return ("UTF-8".to_string(), 0);
        }
    }
    ("UTF-8".to_string(), 0)
}

/// Decode file bytes to string using the detected encoding label.
pub(crate) fn decode_with_encoding(
    bytes: &[u8],
    encoding: &str,
    bom_skip: usize,
) -> Result<String> {
    let data = &bytes[bom_skip..];
    match encoding {
        e if e.starts_with("UTF-16 LE") => {
            let u16_words: Vec<u16> = data
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&u16_words)
                .map_err(|_| Error::tool("read", "Invalid UTF-16 LE data"))
        }
        e if e.starts_with("UTF-16 BE") => {
            let u16_words: Vec<u16> = data
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&u16_words)
                .map_err(|_| Error::tool("read", "Invalid UTF-16 BE data"))
        }
        e if e == "Latin-1 (or binary)" || e == "binary" => {
            Ok(data.iter().map(|&b| b as char).collect())
        }
        _ => {
            String::from_utf8(data.to_vec()).map_err(|_| Error::tool("read", "Invalid UTF-8 data"))
        }
    }
}

/// Format file size in human-readable form.
#[allow(clippy::cast_precision_loss)]
pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Truncate from the beginning (keep first N lines).
///
/// Takes ownership of the input `String` to avoid allocation in the common
/// no-truncation case (content moved, zero-copy) and to enable in-place
/// truncation when the content exceeds limits (`String::truncate`, no new
/// allocation).
#[allow(clippy::too_many_lines)]
pub fn truncate_head(
    content: impl Into<String>,
    max_lines: usize,
    max_bytes: usize,
) -> TruncationResult {
    let mut content = content.into();
    let total_bytes = content.len();

    let total_lines = {
        let nl = memchr::memchr_iter(b'\n', content.as_bytes()).count();
        if content.is_empty() {
            0
        } else if content.ends_with('\n') {
            nl
        } else {
            nl + 1
        }
    };

    if max_lines == 0 {
        let truncated = !content.is_empty();
        content.clear();
        return TruncationResult {
            content,
            truncated,
            truncated_by: if truncated {
                Some(TruncatedBy::Lines)
            } else {
                None
            },
            total_lines,
            total_bytes,
            output_lines: 0,
            output_bytes: 0,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    if max_bytes == 0 {
        let truncated = !content.is_empty();
        let first_line_exceeds_limit = !content.is_empty();
        content.clear();
        return TruncationResult {
            content,
            truncated,
            truncated_by: if truncated {
                Some(TruncatedBy::Bytes)
            } else {
                None
            },
            total_lines,
            total_bytes,
            output_lines: 0,
            output_bytes: 0,
            last_line_partial: false,
            first_line_exceeds_limit,
            max_lines,
            max_bytes,
        };
    }

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content,
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    let first_newline = memchr::memchr(b'\n', content.as_bytes());
    let first_line_bytes = first_newline.unwrap_or(content.len());

    if first_line_bytes > max_bytes {
        let mut valid_bytes = max_bytes;
        while valid_bytes > 0 && !content.is_char_boundary(valid_bytes) {
            valid_bytes -= 1;
        }
        content.truncate(valid_bytes);
        return TruncationResult {
            content,
            truncated: true,
            truncated_by: Some(TruncatedBy::Bytes),
            total_lines,
            total_bytes,
            output_lines: usize::from(valid_bytes > 0),
            output_bytes: valid_bytes,
            last_line_partial: true,
            first_line_exceeds_limit: true,
            max_lines,
            max_bytes,
        };
    }

    let mut line_count = 0;
    let mut byte_count = 0;
    let mut truncated_by = None;
    let mut current_offset = 0;
    let mut last_line_partial = false;

    while current_offset < content.len() {
        if line_count >= max_lines {
            truncated_by = Some(TruncatedBy::Lines);
            break;
        }

        let next_newline = memchr::memchr(b'\n', &content.as_bytes()[current_offset..]);
        let line_end_without_nl = next_newline.map_or(content.len(), |idx| current_offset + idx);
        let line_end_with_nl = next_newline.map_or(content.len(), |idx| current_offset + idx + 1);

        if line_end_without_nl > max_bytes {
            let mut byte_limit = max_bytes.min(content.len());
            if byte_limit < current_offset {
                truncated_by = Some(TruncatedBy::Bytes);
                break;
            }
            while byte_limit > current_offset && !content.is_char_boundary(byte_limit) {
                byte_limit -= 1;
            }
            if byte_limit > current_offset {
                byte_count = byte_limit;
                line_count += 1;
                last_line_partial = true;
            }
            truncated_by = Some(TruncatedBy::Bytes);
            break;
        }

        if line_end_with_nl > max_bytes {
            if line_end_without_nl > current_offset {
                byte_count = line_end_without_nl;
                line_count += 1;
            }
            truncated_by = Some(TruncatedBy::Bytes);
            break;
        }

        byte_count = line_end_with_nl;
        line_count += 1;
        current_offset = line_end_with_nl;
    }

    content.truncate(byte_count);

    TruncationResult {
        truncated: truncated_by.is_some(),
        truncated_by,
        total_lines,
        total_bytes,
        output_lines: line_count,
        output_bytes: byte_count,
        last_line_partial,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
        content,
    }
}

/// Truncate from the end (keep last N lines).
///
/// Takes ownership of the input `String` to avoid allocation in the common
/// no-truncation case (content moved, zero-copy). When truncation is needed,
/// the prefix is drained in-place, reusing the original buffer.
#[allow(clippy::too_many_lines)]
pub fn truncate_tail(
    content: impl Into<String>,
    max_lines: usize,
    max_bytes: usize,
) -> TruncationResult {
    let mut content = content.into();
    let total_bytes = content.len();

    // Count lines correctly: trailing newline terminates the last line, it doesn't start a new one.
    // "a\n" -> 1 line. "a\nb" -> 2 lines. "a" -> 1 line. "" -> 0 lines (handled below).
    let mut total_lines = memchr::memchr_iter(b'\n', content.as_bytes()).count();
    if !content.ends_with('\n') && !content.is_empty() {
        total_lines += 1;
    }
    if content.is_empty() {
        total_lines = 0;
    }

    // Explicitly handle zero-line budgets. Keeping any line would violate the
    // contract (`output_lines <= max_lines`) and proptest invariants.
    if max_lines == 0 {
        let truncated = !content.is_empty();
        return TruncationResult {
            content: String::new(),
            truncated,
            truncated_by: if truncated {
                Some(TruncatedBy::Lines)
            } else {
                None
            },
            total_lines,
            total_bytes,
            output_lines: 0,
            output_bytes: 0,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    // No truncation needed — reuse the owned String (zero-copy move).
    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content,
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    let mut line_count = 0usize;
    let mut byte_count = 0usize;
    let mut start_idx = content.len();
    let mut partial_output: Option<String> = None;
    let mut partial_line_truncated = false;
    let mut truncated_by = None;
    let mut last_line_partial = false;

    // Scope the immutable borrow so we can mutate `content` afterwards.
    {
        let bytes = content.as_bytes();
        // Initialize search_limit outside the loop to track progress backwards.
        // If the file ends with a newline, we skip it for the purpose of finding
        // the *start* of the last line, but start_idx (at len) includes it.
        let mut search_limit = bytes.len();
        if search_limit > 0 && bytes[search_limit - 1] == b'\n' {
            search_limit -= 1;
        }

        loop {
            // Find the *previous* newline.
            let prev_newline = memchr::memrchr(b'\n', &bytes[..search_limit]);
            let line_start = prev_newline.map_or(0, |idx| idx + 1);

            // Bytes for this line (including its newline if it's not the last one,
            // or if the file ends with newline). start_idx is the end of the
            // segment we are accumulating.
            let added_bytes = start_idx - line_start;

            if byte_count + added_bytes > max_bytes {
                // Try to take a partial line if byte budget remains. This
                // preserves suffix stability under prepends while staying on a
                // valid UTF-8 boundary.
                let remaining = max_bytes.saturating_sub(byte_count);
                if remaining > 0 {
                    let chunk = &content[line_start..start_idx];
                    let truncated_chunk = truncate_string_to_bytes_from_end(chunk, remaining);
                    if !truncated_chunk.is_empty() {
                        partial_output = Some(truncated_chunk);
                        partial_line_truncated = true;
                        if line_count == 0 {
                            last_line_partial = true;
                        }
                    }
                }
                truncated_by = Some(TruncatedBy::Bytes);
                break;
            }

            line_count += 1;
            byte_count += added_bytes;
            start_idx = line_start;

            if line_count >= max_lines {
                truncated_by = Some(TruncatedBy::Lines);
                break;
            }

            if line_start == 0 {
                break;
            }

            // Prepare for next iter.
            // We just consumed line starting at `line_start`.
            // The separator before it is at `line_start - 1`.
            // That separator is the `\n` of the *previous* line.
            // We want to search *before* it.
            search_limit = line_start - 1;
        }
    } // immutable borrow of `content` released

    // Extract the suffix: drain the prefix in-place (reuses the buffer),
    // or use the partial output from the byte-truncation path.
    let partial_suffix = if partial_line_truncated {
        Some(content[start_idx..].to_string())
    } else {
        None
    };

    let mut output = partial_output.unwrap_or_else(|| {
        drop(content.drain(..start_idx));
        content
    });

    // If we have a partial last line, we need to append the *rest* of the content
    // that we successfully kept (the `byte_count` lines).
    // Wait, `partial_output` replaces the *current line*.
    // The previous successful lines are in `content[old_start_idx..]`.
    // My logic above for partial output:
    // `truncated_chunk` is the partial tail of the *current line*.
    // We need to prepend it to the lines we already collected?
    // Actually, `content` is the full string.
    // We are scanning backwards.
    // `start_idx` tracks the start of the valid suffix so far.
    // When we hit the byte limit, we are at `line_start..start_idx`.
    // `truncated_chunk` is the tail of *that* segment.
    // So final output = `truncated_chunk` + `content[start_idx..]`.

    if let Some(suffix) = partial_suffix {
        // Need to reconstruct.
        // `output` is currently just the truncated chunk.
        // We need to append the previously accumulated suffix.
        // `content` still holds everything.
        // `start_idx` points to the start of the *valid* suffix from previous iters.
        output.push_str(&suffix);
        // Recalculate line count from the final output.
        // Since truncated output is bounded (<= max_bytes), this scan is cheap.
        let mut count = memchr::memchr_iter(b'\n', output.as_bytes()).count();
        if !output.ends_with('\n') && !output.is_empty() {
            count += 1;
        }
        if output.is_empty() {
            count = 0;
        }
        line_count = count;
    }

    let output_bytes = output.len();

    TruncationResult {
        content: output,
        truncated: truncated_by.is_some(),
        truncated_by,
        total_lines,
        total_bytes,
        output_lines: line_count,
        output_bytes,
        last_line_partial,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
    }
}

/// Truncate a string to fit within a byte limit (from the end), preserving UTF-8 boundaries.
pub(crate) fn truncate_string_to_bytes_from_end(s: &str, max_bytes: usize) -> String {
    let bytes = s.as_bytes();
    if bytes.len() <= max_bytes {
        return s.to_string();
    }

    let mut start = bytes.len().saturating_sub(max_bytes);
    while start < bytes.len() && (bytes[start] & 0b1100_0000) == 0b1000_0000 {
        start += 1;
    }

    std::str::from_utf8(&bytes[start..])
        .map(str::to_string)
        .unwrap_or_default()
}

pub(crate) struct HeadTruncatingLineWriter {
    content: String,
    max_bytes: usize,
    total_lines: usize,
    total_bytes: usize,
    output_lines: usize,
    truncated: bool,
    last_line_partial: bool,
    first_line_exceeds_limit: bool,
}

impl HeadTruncatingLineWriter {
    fn new(max_bytes: usize) -> Self {
        Self {
            content: String::with_capacity(max_bytes.min(8192)),
            max_bytes,
            total_lines: 0,
            total_bytes: 0,
            output_lines: 0,
            truncated: false,
            last_line_partial: false,
            first_line_exceeds_limit: false,
        }
    }

    fn push_line(&mut self, line: &str) {
        debug_assert!(!line.contains('\n'));

        let line_index = self.total_lines;
        let separator_len = usize::from(line_index > 0);
        let piece_bytes = separator_len.saturating_add(line.len());
        self.total_lines = self.total_lines.saturating_add(1);
        self.total_bytes = self.total_bytes.saturating_add(piece_bytes);

        if self.truncated {
            return;
        }

        if self.max_bytes == 0 {
            self.truncated = true;
            self.first_line_exceeds_limit = line_index == 0 && !line.is_empty();
            return;
        }

        let remaining = self.max_bytes.saturating_sub(self.content.len());
        if piece_bytes <= remaining {
            if separator_len > 0 {
                self.content.push('\n');
            }
            self.content.push_str(line);
            self.output_lines = self.output_lines.saturating_add(1);
            return;
        }

        self.truncated = true;
        if line_index == 0 && line.len() > self.max_bytes {
            self.first_line_exceeds_limit = true;
        }

        let line_budget = if separator_len > 0 {
            if remaining == 0 {
                return;
            }
            self.content.push('\n');
            remaining - 1
        } else {
            remaining
        };

        let valid_bytes = utf8_prefix_len(line, line_budget);
        if valid_bytes > 0 {
            self.content.push_str(&line[..valid_bytes]);
            self.output_lines = self.output_lines.saturating_add(1);
            self.last_line_partial = valid_bytes < line.len();
        }
    }

    fn finish(self) -> TruncationResult {
        let output_bytes = self.content.len();
        TruncationResult {
            content: self.content,
            truncated: self.truncated,
            truncated_by: if self.truncated {
                Some(TruncatedBy::Bytes)
            } else {
                None
            },
            total_lines: self.total_lines,
            total_bytes: self.total_bytes,
            output_lines: self.output_lines,
            output_bytes,
            last_line_partial: self.last_line_partial,
            first_line_exceeds_limit: self.first_line_exceeds_limit,
            max_lines: usize::MAX,
            max_bytes: self.max_bytes,
        }
    }
}

pub(crate) fn utf8_prefix_len(s: &str, max_bytes: usize) -> usize {
    let mut valid_bytes = max_bytes.min(s.len());
    while valid_bytes > 0 && !s.is_char_boundary(valid_bytes) {
        valid_bytes -= 1;
    }
    valid_bytes
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolOutputArtifactRef {
    schema: &'static str,
    id: String,
    tool_name: String,
    source_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    path: String,
    metadata_path: String,
    sha256: String,
    byte_count: u64,
    line_count: usize,
    preview_bytes: usize,
    content_type: &'static str,
    retention_class: &'static str,
    spillover_reason: &'static str,
    redaction_summary: ToolOutputArtifactRedactionSummary,
    safe_delete_candidate: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolOutputArtifactRedactionSummary {
    policy: &'static str,
    status: &'static str,
    redacted_count: usize,
    fields: Vec<String>,
    raw_secret_bytes_emitted: usize,
    binary_suspect: bool,
    max_redaction_bytes: u64,
}

pub(crate) struct RedactedToolOutputArtifact {
    bytes: Vec<u8>,
    summary: ToolOutputArtifactRedactionSummary,
}

pub(crate) fn tool_output_artifact_root() -> PathBuf {
    std::env::var_os("PI_TOOL_OUTPUT_ARTIFACT_DIR").map_or_else(
        || Config::global_dir().join("tool-output-artifacts"),
        PathBuf::from,
    )
}

static TOOL_OUTPUT_ARTIFACT_SESSIONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

pub(crate) fn tool_output_artifact_sessions() -> &'static Mutex<HashMap<String, String>> {
    TOOL_OUTPUT_ARTIFACT_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) struct ToolOutputArtifactSessionGuard {
    tool_call_id: String,
    previous_session_id: Option<String>,
    active: bool,
}

impl Drop for ToolOutputArtifactSessionGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let Ok(mut sessions) = tool_output_artifact_sessions().lock() else {
            return;
        };
        if let Some(previous) = self.previous_session_id.take() {
            sessions.insert(self.tool_call_id.clone(), previous);
        } else {
            sessions.remove(&self.tool_call_id);
        }
    }
}

pub(crate) fn register_tool_output_artifact_session(
    tool_call_id: &str,
    session_id: &str,
) -> ToolOutputArtifactSessionGuard {
    if session_id.is_empty() {
        return ToolOutputArtifactSessionGuard {
            tool_call_id: String::new(),
            previous_session_id: None,
            active: false,
        };
    }
    let previous_session_id = tool_output_artifact_sessions()
        .lock()
        .ok()
        .and_then(|mut sessions| sessions.insert(tool_call_id.to_string(), session_id.to_string()));
    ToolOutputArtifactSessionGuard {
        tool_call_id: tool_call_id.to_string(),
        previous_session_id,
        active: true,
    }
}

pub(crate) fn tool_output_artifact_session_id(tool_call_id: &str) -> Option<String> {
    tool_output_artifact_sessions()
        .lock()
        .ok()
        .and_then(|sessions| sessions.get(tool_call_id).cloned())
}

pub(crate) fn tool_output_artifact_scope_dir(
    root: &Path,
    tool_call_id: &str,
) -> (PathBuf, Option<String>) {
    let call_scope = sanitize_artifact_scope(tool_call_id);
    if let Some(session_id) = tool_output_artifact_session_id(tool_call_id) {
        (
            root.join(sanitize_artifact_scope(&session_id))
                .join(call_scope),
            Some(session_id),
        )
    } else {
        (root.join(call_scope), None)
    }
}

pub(crate) fn sanitize_artifact_scope(scope: &str) -> String {
    let mut out = String::new();
    for ch in scope.chars().take(96) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.trim_matches('_').is_empty() {
        "tool-call".to_string()
    } else {
        out
    }
}

pub(crate) fn artifact_line_count(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        0
    } else {
        memchr::memchr_iter(b'\n', bytes).count() + usize::from(!bytes.ends_with(b"\n"))
    }
}

pub(crate) fn artifact_details_object(
    details: &mut Option<serde_json::Value>,
) -> &mut serde_json::Map<String, serde_json::Value> {
    let value = details.get_or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !value.is_object() {
        *value = serde_json::Value::Object(serde_json::Map::new());
    }
    value
        .as_object_mut()
        .expect("details value forced to object")
}

pub(crate) fn normalize_redaction_field(field: &str) -> String {
    let mut out = String::new();
    let mut previous_underscore = false;
    for ch in field.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            previous_underscore = false;
            ch.to_ascii_lowercase()
        } else if previous_underscore {
            continue;
        } else {
            previous_underscore = true;
            '_'
        };
        out.push(normalized);
    }
    out.trim_matches('_').to_string()
}

pub(crate) fn record_redacted_field(fields: &mut Vec<String>, field: &str) {
    let field = normalize_redaction_field(field);
    if !field.is_empty() && !fields.iter().any(|existing| existing == &field) {
        fields.push(field);
    }
}

pub(crate) fn artifact_sensitive_key_value_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r#"(?i)\b([A-Za-z_][A-Za-z0-9_.-]*(?:api[_-]?key|token|secret|password|passwd|credential|authorization)[A-Za-z0-9_.-]*)(\s*[:=]\s*)("[^"\r\n]*"|'[^'\r\n]*'|[^\s,;}]+)"#,
        )
        .expect("valid artifact key-value redaction regex")
    })
}

pub(crate) fn artifact_bearer_token_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"(?i)\b(Bearer\s+)([A-Za-z0-9._~+/=-]{8,})")
            .expect("valid artifact bearer redaction regex")
    })
}

pub(crate) fn artifact_token_value_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"\b(sk-[A-Za-z0-9][A-Za-z0-9_-]{10,}|gh[pousr]_[A-Za-z0-9_]{10,}|AKIA[0-9A-Z]{12,})\b",
        )
        .expect("valid artifact token value redaction regex")
    })
}

pub(crate) fn redacted_literal_for_value(value: &str) -> &'static str {
    if value.starts_with('"') && value.ends_with('"') {
        "\"[REDACTED]\""
    } else if value.starts_with('\'') && value.ends_with('\'') {
        "'[REDACTED]'"
    } else {
        "[REDACTED]"
    }
}

pub(crate) fn redact_tool_output_artifact_text(
    text: &str,
    binary_suspect: bool,
) -> RedactedToolOutputArtifact {
    let mut fields = Vec::new();
    let mut redacted_count = 0usize;

    let redacted = artifact_sensitive_key_value_regex()
        .replace_all(text, |caps: &regex::Captures<'_>| {
            let key = caps.get(1).map_or("", |m| m.as_str());
            let sep = caps.get(2).map_or("", |m| m.as_str());
            let value = caps.get(3).map_or("", |m| m.as_str());
            if value == "[REDACTED]" || value == "\"[REDACTED]\"" || value == "'[REDACTED]'" {
                caps.get(0).map_or("", |m| m.as_str()).to_string()
            } else {
                redacted_count = redacted_count.saturating_add(1);
                record_redacted_field(&mut fields, key);
                format!("{key}{sep}{}", redacted_literal_for_value(value))
            }
        })
        .to_string();

    let redacted = artifact_bearer_token_regex()
        .replace_all(&redacted, |caps: &regex::Captures<'_>| {
            redacted_count = redacted_count.saturating_add(1);
            record_redacted_field(&mut fields, "authorization");
            let prefix = caps.get(1).map_or("", |m| m.as_str());
            format!("{prefix}[REDACTED]")
        })
        .to_string();

    let redacted = artifact_token_value_regex()
        .replace_all(&redacted, |_caps: &regex::Captures<'_>| {
            redacted_count = redacted_count.saturating_add(1);
            record_redacted_field(&mut fields, "tokenValue");
            "[REDACTED]".to_string()
        })
        .to_string();

    fields.sort();
    let raw_secret_bytes_emitted = estimate_raw_secret_bytes(&redacted);
    let summary = ToolOutputArtifactRedactionSummary {
        policy: TOOL_OUTPUT_ARTIFACT_REDACTION_POLICY_V1,
        status: if raw_secret_bytes_emitted > 0 {
            "unsafe"
        } else if redacted_count > 0 {
            "redacted"
        } else {
            "clean"
        },
        redacted_count,
        fields,
        raw_secret_bytes_emitted,
        binary_suspect,
        max_redaction_bytes: TOOL_OUTPUT_ARTIFACT_REDACTION_MAX_BYTES,
    };

    RedactedToolOutputArtifact {
        bytes: redacted.into_bytes(),
        summary,
    }
}

pub(crate) fn estimate_raw_secret_bytes(text: &str) -> usize {
    let key_value_bytes = artifact_sensitive_key_value_regex()
        .captures_iter(text)
        .filter_map(|caps| {
            let value = caps.get(3)?.as_str();
            if value == "[REDACTED]" || value == "\"[REDACTED]\"" || value == "'[REDACTED]'" {
                None
            } else {
                caps.get(0).map(|m| m.as_str().len())
            }
        })
        .sum::<usize>();
    let bearer_bytes = artifact_bearer_token_regex()
        .find_iter(text)
        .map(|m| m.as_str().len())
        .sum::<usize>();
    let token_bytes = artifact_token_value_regex()
        .find_iter(text)
        .map(|m| m.as_str().len())
        .sum::<usize>();
    key_value_bytes
        .saturating_add(bearer_bytes)
        .saturating_add(token_bytes)
}

pub(crate) fn redact_tool_output_artifact_bytes(
    bytes: &[u8],
) -> std::io::Result<RedactedToolOutputArtifact> {
    let binary_suspect =
        memchr::memchr(b'\0', bytes).is_some() || std::str::from_utf8(bytes).is_err();
    let text = String::from_utf8_lossy(bytes);
    let redacted = redact_tool_output_artifact_text(text.as_ref(), binary_suspect);
    if redacted.summary.raw_secret_bytes_emitted > 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "artifact redaction failed closed: raw secret-looking bytes remain",
        ));
    }
    Ok(redacted)
}

pub(crate) fn ensure_artifact_path_under_root(root: &Path, path: &Path) -> std::io::Result<()> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "artifact path {} is outside artifact root {}",
                path.display(),
                root.display()
            ),
        ))
    }
}

pub(crate) fn write_artifact_file_if_absent(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(bytes)?;
            tolerate_fsync_refusal(file.sync_all(), "artifact file", path)?;
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
    }
}

pub(crate) fn write_text_tool_output_artifact_at_root(
    root: &Path,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    full_text: &str,
    preview_bytes: usize,
) -> std::io::Result<ToolOutputArtifactRef> {
    let bytes = full_text.as_bytes();
    if bytes.len() > TOOL_OUTPUT_ARTIFACT_MAX_BYTES_USIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "artifact source exceeds {} hard limit",
                format_size(TOOL_OUTPUT_ARTIFACT_MAX_BYTES_USIZE)
            ),
        ));
    }
    if bytes.len() > TOOL_OUTPUT_ARTIFACT_REDACTION_MAX_BYTES_USIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "artifact source exceeds {} redaction limit",
                format_size(TOOL_OUTPUT_ARTIFACT_REDACTION_MAX_BYTES_USIZE)
            ),
        ));
    }
    let redacted = redact_tool_output_artifact_bytes(bytes)?;
    let bytes = redacted.bytes.as_slice();
    let sha256 = format!("{:x}", sha2::Sha256::digest(bytes));
    let (scope_dir, session_id) = tool_output_artifact_scope_dir(root, tool_call_id);
    std::fs::create_dir_all(&scope_dir)?;

    let id = format!("tool-artifact-{}", &sha256[..16]);
    let content_path = scope_dir.join(format!("{sha256}.txt"));
    let metadata_path = scope_dir.join(format!("{sha256}.json"));
    ensure_artifact_path_under_root(root, &content_path)?;
    ensure_artifact_path_under_root(root, &metadata_path)?;
    write_artifact_file_if_absent(&content_path, bytes)?;

    let artifact = ToolOutputArtifactRef {
        schema: TOOL_OUTPUT_ARTIFACT_SCHEMA_V1,
        id,
        tool_name: tool_name.to_string(),
        source_kind: source_kind.to_string(),
        session_id,
        path: content_path.display().to_string(),
        metadata_path: metadata_path.display().to_string(),
        sha256,
        byte_count: bytes.len().try_into().unwrap_or(u64::MAX),
        line_count: artifact_line_count(bytes),
        preview_bytes,
        content_type: "text/plain; charset=utf-8",
        retention_class: TOOL_OUTPUT_ARTIFACT_RETENTION_CLASS,
        spillover_reason: TOOL_OUTPUT_ARTIFACT_SPILLOVER_REASON,
        redaction_summary: redacted.summary,
        safe_delete_candidate: true,
    };
    let metadata = serde_json::to_vec_pretty(&artifact).map_err(std::io::Error::other)?;
    write_artifact_file_if_absent(&metadata_path, &metadata)?;
    Ok(artifact)
}

pub(crate) fn copy_text_tool_output_artifact_from_path_at_root(
    root: &Path,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    source_path: &Path,
    preview_bytes: usize,
) -> std::io::Result<ToolOutputArtifactRef> {
    let metadata = std::fs::metadata(source_path)?;
    if metadata.len() > TOOL_OUTPUT_ARTIFACT_MAX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "artifact source exceeds {} hard limit",
                format_size(TOOL_OUTPUT_ARTIFACT_MAX_BYTES_USIZE)
            ),
        ));
    }
    if metadata.len() > TOOL_OUTPUT_ARTIFACT_REDACTION_MAX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "artifact source exceeds {} redaction limit",
                format_size(TOOL_OUTPUT_ARTIFACT_REDACTION_MAX_BYTES_USIZE)
            ),
        ));
    }

    let mut source = std::fs::File::open(source_path)?;
    let mut source_bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    source.read_to_end(&mut source_bytes)?;
    let redacted = redact_tool_output_artifact_bytes(&source_bytes)?;
    let bytes = redacted.bytes.as_slice();

    let sha256 = format!("{:x}", sha2::Sha256::digest(bytes));
    let (scope_dir, session_id) = tool_output_artifact_scope_dir(root, tool_call_id);
    std::fs::create_dir_all(&scope_dir)?;
    let id = format!("tool-artifact-{}", &sha256[..16]);
    let content_path = scope_dir.join(format!("{sha256}.txt"));
    let metadata_path = scope_dir.join(format!("{sha256}.json"));
    ensure_artifact_path_under_root(root, &content_path)?;
    ensure_artifact_path_under_root(root, &metadata_path)?;
    write_artifact_file_if_absent(&content_path, bytes)?;

    let artifact = ToolOutputArtifactRef {
        schema: TOOL_OUTPUT_ARTIFACT_SCHEMA_V1,
        id,
        tool_name: tool_name.to_string(),
        source_kind: source_kind.to_string(),
        session_id,
        path: content_path.display().to_string(),
        metadata_path: metadata_path.display().to_string(),
        sha256,
        byte_count: bytes.len().try_into().unwrap_or(u64::MAX),
        line_count: artifact_line_count(bytes),
        preview_bytes,
        content_type: "text/plain; charset=utf-8",
        retention_class: TOOL_OUTPUT_ARTIFACT_RETENTION_CLASS,
        spillover_reason: TOOL_OUTPUT_ARTIFACT_SPILLOVER_REASON,
        redaction_summary: redacted.summary,
        safe_delete_candidate: true,
    };
    let metadata = serde_json::to_vec_pretty(&artifact).map_err(std::io::Error::other)?;
    write_artifact_file_if_absent(&metadata_path, &metadata)?;
    Ok(artifact)
}

pub(crate) fn append_tool_output_artifact_notice(
    output_text: &mut String,
    artifact: &ToolOutputArtifactRef,
) {
    let _ = write!(
        output_text,
        "\n\n[Full tool output artifact: {} ({} bytes, {} lines, sha256 {}). Use read on this path to inspect more.]",
        artifact.path, artifact.byte_count, artifact.line_count, artifact.sha256,
    );
}

pub(crate) fn append_artifact_source_line(full_text: &mut String, line: &str) {
    if !full_text.is_empty() {
        full_text.push('\n');
    }
    full_text.push_str(line);
}

pub(crate) fn record_tool_output_artifact_error(
    output_text: &mut String,
    details: &mut Option<serde_json::Value>,
    error: &std::io::Error,
) {
    let _ = write!(
        output_text,
        "\n\n[Tool output artifact persistence failed: {error}. Showing the bounded preview only.]"
    );
    artifact_details_object(details).insert(
        "artifactError".to_string(),
        serde_json::json!({
            "schema": TOOL_OUTPUT_ARTIFACT_SCHEMA_V1,
            "message": error.to_string(),
        }),
    );
}

pub(crate) fn attach_text_artifact_if_needed_at_root(
    root: &Path,
    output_text: &mut String,
    details: &mut Option<serde_json::Value>,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    full_text: &str,
) -> bool {
    if full_text.len() <= TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES {
        return false;
    }
    match write_text_tool_output_artifact_at_root(
        root,
        tool_name,
        tool_call_id,
        source_kind,
        full_text,
        output_text.len(),
    ) {
        Ok(artifact) => {
            append_tool_output_artifact_notice(output_text, &artifact);
            artifact_details_object(details).insert(
                "artifact".to_string(),
                serde_json::to_value(&artifact).expect("artifact ref serializes"),
            );
            true
        }
        Err(err) => {
            record_tool_output_artifact_error(output_text, details, &err);
            false
        }
    }
}

pub(crate) fn attach_text_artifact_if_needed(
    output_text: &mut String,
    details: &mut Option<serde_json::Value>,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    full_text: &str,
) -> bool {
    let root = tool_output_artifact_root();
    attach_text_artifact_if_needed_at_root(
        &root,
        output_text,
        details,
        tool_name,
        tool_call_id,
        source_kind,
        full_text,
    )
}

pub(crate) fn attach_text_artifact_if_needed_with_root(
    root: Option<&Path>,
    output_text: &mut String,
    details: &mut Option<serde_json::Value>,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    full_text: &str,
) -> bool {
    if let Some(root) = root {
        attach_text_artifact_if_needed_at_root(
            root,
            output_text,
            details,
            tool_name,
            tool_call_id,
            source_kind,
            full_text,
        )
    } else {
        attach_text_artifact_if_needed(
            output_text,
            details,
            tool_name,
            tool_call_id,
            source_kind,
            full_text,
        )
    }
}

pub(crate) fn attach_text_artifact_from_path_if_needed_at_root(
    root: &Path,
    output_text: &mut String,
    details: &mut Option<serde_json::Value>,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    source_path: &Path,
) -> bool {
    let Ok(metadata) = std::fs::metadata(source_path) else {
        return false;
    };
    if metadata.len() <= u64::try_from(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES).unwrap_or(u64::MAX) {
        return false;
    }
    match copy_text_tool_output_artifact_from_path_at_root(
        root,
        tool_name,
        tool_call_id,
        source_kind,
        source_path,
        output_text.len(),
    ) {
        Ok(artifact) => {
            append_tool_output_artifact_notice(output_text, &artifact);
            artifact_details_object(details).insert(
                "artifact".to_string(),
                serde_json::to_value(&artifact).expect("artifact ref serializes"),
            );
            true
        }
        Err(err) => {
            record_tool_output_artifact_error(output_text, details, &err);
            false
        }
    }
}

pub(crate) fn attach_text_artifact_from_path_if_needed(
    output_text: &mut String,
    details: &mut Option<serde_json::Value>,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    source_path: &Path,
) -> bool {
    let root = tool_output_artifact_root();
    attach_text_artifact_from_path_if_needed_at_root(
        &root,
        output_text,
        details,
        tool_name,
        tool_call_id,
        source_kind,
        source_path,
    )
}

pub(crate) fn attach_text_artifact_from_path_if_needed_with_root(
    root: Option<&Path>,
    output_text: &mut String,
    details: &mut Option<serde_json::Value>,
    tool_name: &str,
    tool_call_id: &str,
    source_kind: &str,
    source_path: &Path,
) -> bool {
    if let Some(root) = root {
        attach_text_artifact_from_path_if_needed_at_root(
            root,
            output_text,
            details,
            tool_name,
            tool_call_id,
            source_kind,
            source_path,
        )
    } else {
        attach_text_artifact_from_path_if_needed(
            output_text,
            details,
            tool_name,
            tool_call_id,
            source_kind,
            source_path,
        )
    }
}

const TOOL_OUTPUT_CACHE_MAX_ENTRIES: usize = 128;
const TOOL_OUTPUT_CACHE_MAX_BYTES: usize = 8 * 1024 * 1024;
const TOOL_OUTPUT_CACHE_MAX_ENTRY_BYTES: usize = DEFAULT_MAX_BYTES + 64 * 1024;
const TOOL_OUTPUT_CACHE_MAX_FINGERPRINT_FILES: usize = 2048;
const TOOL_OUTPUT_CACHE_MAX_FINGERPRINT_BYTES: u64 = 8 * 1024 * 1024;
const TOOL_OUTPUT_CACHE_MAX_FILE_HASH_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCacheDependency {
    path: PathBuf,
    fingerprint: [u8; 32],
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ToolCacheFingerprintMode {
    FileContent,
    DirectoryImmediate,
    DirectoryRecursive,
}

#[derive(Debug, Clone)]
pub(crate) struct CachedToolOutput {
    deps: Vec<ToolCacheDependency>,
    output: ToolOutput,
    weight: usize,
    generation: u64,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ToolOutputCacheStats {
    hits: usize,
    misses: usize,
    inserts: usize,
    invalidations: usize,
    disabled: usize,
    side_effect_accesses: usize,
    side_effect_insert_attempts: usize,
}

#[derive(Debug, Default)]
pub(crate) struct ToolOutputCache {
    entries: HashMap<String, CachedToolOutput>,
    order: VecDeque<(String, u64)>,
    total_bytes: usize,
    generation: u64,
    #[cfg(test)]
    stats: ToolOutputCacheStats,
}

impl ToolOutputCache {
    fn get(&mut self, key: &str, deps: &[ToolCacheDependency]) -> Option<ToolOutput> {
        self.generation = self.generation.saturating_add(1);
        let generation = self.generation;
        #[cfg(test)]
        {
            if is_side_effect_tool_cache_key(key) {
                self.stats.side_effect_accesses = self.stats.side_effect_accesses.saturating_add(1);
            }
        }

        if self
            .entries
            .get(key)
            .is_some_and(|entry| entry.deps == deps)
        {
            let entry = self.entries.get_mut(key)?;
            entry.generation = generation;
            self.order.push_back((key.to_string(), generation));
            #[cfg(test)]
            {
                self.stats.hits = self.stats.hits.saturating_add(1);
            }
            return Some(entry.output.clone());
        }

        if let Some(removed) = self.entries.remove(key) {
            self.total_bytes = self.total_bytes.saturating_sub(removed.weight);
            #[cfg(test)]
            {
                self.stats.invalidations = self.stats.invalidations.saturating_add(1);
            }
        } else {
            #[cfg(test)]
            {
                self.stats.misses = self.stats.misses.saturating_add(1);
            }
        }

        None
    }

    fn insert(
        &mut self,
        key: String,
        deps: Vec<ToolCacheDependency>,
        output: ToolOutput,
        weight: usize,
    ) {
        if weight == 0 || weight > TOOL_OUTPUT_CACHE_MAX_ENTRY_BYTES {
            #[cfg(test)]
            {
                self.stats.disabled = self.stats.disabled.saturating_add(1);
            }
            return;
        }

        #[cfg(test)]
        {
            if is_side_effect_tool_cache_key(&key) {
                self.stats.side_effect_insert_attempts =
                    self.stats.side_effect_insert_attempts.saturating_add(1);
            }
        }

        if let Some(removed) = self.entries.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(removed.weight);
        }

        self.generation = self.generation.saturating_add(1);
        let generation = self.generation;
        self.total_bytes = self.total_bytes.saturating_add(weight);
        self.order.push_back((key.clone(), generation));
        self.entries.insert(
            key,
            CachedToolOutput {
                deps,
                output,
                weight,
                generation,
            },
        );
        #[cfg(test)]
        {
            self.stats.inserts = self.stats.inserts.saturating_add(1);
        }
        self.evict_to_limits();
    }

    fn evict_to_limits(&mut self) {
        while self.entries.len() > TOOL_OUTPUT_CACHE_MAX_ENTRIES
            || self.total_bytes > TOOL_OUTPUT_CACHE_MAX_BYTES
        {
            let Some((key, generation)) = self.order.pop_front() else {
                break;
            };
            if self
                .entries
                .get(&key)
                .is_some_and(|entry| entry.generation == generation)
                && let Some(removed) = self.entries.remove(&key)
            {
                self.total_bytes = self.total_bytes.saturating_sub(removed.weight);
            }
        }
    }
}

pub(crate) fn tool_output_cache() -> &'static Mutex<ToolOutputCache> {
    static CACHE: OnceLock<Mutex<ToolOutputCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ToolOutputCache::default()))
}

pub(crate) fn lock_tool_output_cache() -> std::sync::MutexGuard<'static, ToolOutputCache> {
    tool_output_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn tool_cache_key(tool: &str, cwd: &Path, input: &serde_json::Value) -> String {
    let input_json = serde_json::to_string(input).unwrap_or_else(|_| input.to_string());
    format!("{tool}\0{}\0{input_json}", cwd.display())
}

#[cfg(test)]
pub(crate) fn is_side_effect_tool_cache_key(key: &str) -> bool {
    key.starts_with("write\0") || key.starts_with("edit\0") || key.starts_with("bash\0")
}

pub(crate) fn cached_tool_output(
    key: &str,
    deps: Option<&[ToolCacheDependency]>,
) -> Option<ToolOutput> {
    let deps = deps?;
    lock_tool_output_cache().get(key, deps)
}

pub(crate) fn cache_tool_output(
    key: String,
    deps: Option<Vec<ToolCacheDependency>>,
    output: &ToolOutput,
) {
    let Some(deps) = deps else {
        return;
    };
    if output.details.as_ref().is_some_and(|details| {
        details.as_object().is_some_and(|details| {
            details.contains_key("artifact") || details.contains_key("artifactError")
        })
    }) {
        return;
    }
    let Some(weight) = cacheable_tool_output_weight(output) else {
        return;
    };
    lock_tool_output_cache().insert(key, deps, output.clone(), weight);
}

pub(crate) fn stable_cache_dependency_for_path(
    path: &Path,
    mode: ToolCacheFingerprintMode,
    before_deps: Option<&[ToolCacheDependency]>,
) -> Option<Vec<ToolCacheDependency>> {
    let before_deps = before_deps?;
    let after_deps = cache_dependency_for_path(path, mode)?;
    (before_deps == after_deps.as_slice()).then_some(after_deps)
}

pub(crate) fn cacheable_tool_output_weight(output: &ToolOutput) -> Option<usize> {
    let mut weight = output
        .details
        .as_ref()
        .and_then(|details| serde_json::to_vec(details).ok())
        .map_or(0, |details| details.len());

    for block in &output.content {
        match block {
            ContentBlock::Text(text) => {
                weight = weight.saturating_add(text.text.len());
                if let Some(signature) = &text.text_signature {
                    weight = weight.saturating_add(signature.len());
                }
            }
            ContentBlock::Image(_)
            | ContentBlock::Thinking(_)
            | ContentBlock::RedactedThinking(_)
            | ContentBlock::ToolCall(_) => return None,
        }
    }

    Some(weight)
}

pub(crate) fn cache_dependency_for_path(
    path: &Path,
    mode: ToolCacheFingerprintMode,
) -> Option<Vec<ToolCacheDependency>> {
    let fingerprint = match mode {
        ToolCacheFingerprintMode::FileContent => fingerprint_file_content(path)?,
        ToolCacheFingerprintMode::DirectoryImmediate => fingerprint_directory_immediate(path)?,
        ToolCacheFingerprintMode::DirectoryRecursive => fingerprint_directory_recursive(path)?,
    };

    Some(vec![ToolCacheDependency {
        path: path.to_path_buf(),
        fingerprint,
    }])
}

fn fingerprint_file_content(path: &Path) -> Option<[u8; 32]> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > TOOL_OUTPUT_CACHE_MAX_FILE_HASH_BYTES {
        return None;
    }

    let bytes = std::fs::read(path).ok()?;
    let mut hasher = sha2::Sha256::new();
    update_fingerprint_metadata(&mut hasher, Path::new(""), &metadata);
    hasher.update(sha2::Sha256::digest(&bytes));
    Some(hasher.finalize().into())
}

fn fingerprint_directory_immediate(path: &Path) -> Option<[u8; 32]> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_dir() {
        return None;
    }

    let mut entries = std::fs::read_dir(path)
        .ok()?
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    if entries.len() > TOOL_OUTPUT_CACHE_MAX_FINGERPRINT_FILES {
        return None;
    }
    entries.sort_by_key(std::fs::DirEntry::file_name);

    let mut hasher = sha2::Sha256::new();
    update_fingerprint_metadata(&mut hasher, Path::new(""), &metadata);
    for entry in entries {
        let entry_path = entry.path();
        let rel = entry.file_name();
        let rel = Path::new(&rel);
        let entry_metadata = std::fs::symlink_metadata(&entry_path).ok()?;
        update_fingerprint_metadata(&mut hasher, rel, &entry_metadata);
        if entry_metadata.file_type().is_symlink() {
            update_symlink_target(&mut hasher, &entry_path);
        }
    }

    Some(hasher.finalize().into())
}

fn fingerprint_directory_recursive(path: &Path) -> Option<[u8; 32]> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.is_file() {
        return fingerprint_file_content(path);
    }
    if !metadata.is_dir() {
        return None;
    }

    let mut budget = FingerprintBudget::default();
    let mut hasher = sha2::Sha256::new();
    update_fingerprint_metadata(&mut hasher, Path::new(""), &metadata);
    fingerprint_tree(path, path, &mut budget, &mut hasher)?;
    Some(hasher.finalize().into())
}

#[derive(Debug, Default)]
struct FingerprintBudget {
    entries: usize,
    bytes: u64,
}

fn fingerprint_tree(
    root: &Path,
    dir: &Path,
    budget: &mut FingerprintBudget,
    hasher: &mut sha2::Sha256,
) -> Option<()> {
    let mut entries = std::fs::read_dir(dir)
        .ok()?
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        budget.entries = budget.entries.saturating_add(1);
        if budget.entries > TOOL_OUTPUT_CACHE_MAX_FINGERPRINT_FILES {
            return None;
        }

        let entry_path = entry.path();
        let rel = entry_path.strip_prefix(root).unwrap_or(&entry_path);
        let metadata = std::fs::symlink_metadata(&entry_path).ok()?;
        update_fingerprint_metadata(hasher, rel, &metadata);

        if metadata.file_type().is_symlink() {
            update_symlink_target(hasher, &entry_path);
        } else if metadata.is_dir() {
            fingerprint_tree(root, &entry_path, budget, hasher)?;
        } else if metadata.is_file() {
            if metadata.len() > TOOL_OUTPUT_CACHE_MAX_FILE_HASH_BYTES {
                return None;
            }
            budget.bytes = budget.bytes.saturating_add(metadata.len());
            if budget.bytes > TOOL_OUTPUT_CACHE_MAX_FINGERPRINT_BYTES {
                return None;
            }
            let bytes = std::fs::read(&entry_path).ok()?;
            hasher.update(sha2::Sha256::digest(&bytes));
        }
    }

    Some(())
}

fn update_fingerprint_metadata(
    hasher: &mut sha2::Sha256,
    path: &Path,
    metadata: &std::fs::Metadata,
) {
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update([0]);
    let file_type = metadata.file_type();
    hasher.update([
        u8::from(metadata.is_file()),
        u8::from(metadata.is_dir()),
        u8::from(file_type.is_symlink()),
    ]);
    hasher.update(metadata.len().to_le_bytes());
    let modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    hasher.update(modified_nanos.to_le_bytes());
    hasher.update([0xff]);
}

fn update_symlink_target(hasher: &mut sha2::Sha256, path: &Path) {
    if let Ok(target) = std::fs::read_link(path) {
        hasher.update(target.to_string_lossy().as_bytes());
    }
    hasher.update([0xfe]);
}

#[cfg(test)]
pub(crate) fn reset_tool_output_cache_for_tests() {
    *lock_tool_output_cache() = ToolOutputCache::default();
}

#[cfg(test)]
pub(crate) fn tool_output_cache_stats_for_tests() -> ToolOutputCacheStats {
    lock_tool_output_cache().stats
}

/// Format a byte count into a human-readable string with appropriate unit suffix.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;

    if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

#[cfg(test)]
pub(crate) fn js_string_length(s: &str) -> usize {
    // Match JavaScript's String.length (UTF-16 code units), not UTF-8 bytes.
    s.encode_utf16().count()
}

// ============================================================================
// Path Utilities (port of pi-mono path-utils.ts)
// ============================================================================

pub(crate) fn is_special_unicode_space(c: char) -> bool {
    matches!(c, '\u{00A0}' | '\u{202F}' | '\u{205F}' | '\u{3000}')
        || ('\u{2000}'..='\u{200A}').contains(&c)
}

fn normalize_unicode_spaces(s: &str) -> String {
    s.chars()
        .map(|c| if is_special_unicode_space(c) { ' ' } else { c })
        .collect()
}

#[cfg(test)]
pub(crate) fn normalize_for_match(s: &str) -> String {
    // Single-pass normalization: spaces, quotes, and dashes in one allocation.
    // Avoids 3 intermediate String allocations from chained replace calls.
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            // Unicode spaces → ASCII space
            c if is_special_unicode_space(c) => out.push(' '),
            // Curly single quotes → straight apostrophe
            '\u{2018}' | '\u{2019}' => out.push('\''),
            // Curly double quotes → straight double quote
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => out.push('"'),
            // Various dashes → ASCII hyphen
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => out.push('-'),
            // Everything else passes through
            c => out.push(c),
        }
    }
    out
}

pub(crate) fn expand_path(file_path: &str) -> String {
    let normalized = normalize_unicode_spaces(file_path);
    if normalized == "~" {
        return dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .to_string_lossy()
            .to_string();
    }
    if let Some(rest) = normalized.strip_prefix("~/") {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        return home.join(rest).to_string_lossy().to_string();
    }
    normalized
}

/// Resolve a path relative to `cwd`. Handles `~` expansion and absolute paths.
pub(crate) fn resolve_to_cwd(file_path: &str, cwd: &Path) -> PathBuf {
    let expanded = expand_path(file_path);
    let expanded_path = PathBuf::from(expanded);
    if expanded_path.is_absolute() {
        expanded_path
    } else {
        cwd.join(expanded_path)
    }
}

pub(crate) fn try_mac_os_screenshot_path(file_path: &str) -> String {
    // Replace " AM." / " PM." with a narrow no-break space variant used by macOS screenshots.
    file_path
        .replace(" AM.", "\u{202F}AM.")
        .replace(" PM.", "\u{202F}PM.")
}

pub(crate) fn try_curly_quote_variant(file_path: &str) -> String {
    // Replace straight apostrophe with macOS screenshot curly apostrophe.
    file_path.replace('\'', "\u{2019}")
}

pub(crate) fn try_nfd_variant(file_path: &str) -> String {
    // NFD normalization - decompose characters into base + combining marks
    // This handles macOS HFS+ filesystem normalization differences
    use unicode_normalization::UnicodeNormalization;
    file_path.nfd().collect::<String>()
}

pub(crate) fn file_exists(path: &Path) -> bool {
    std::fs::metadata(path).is_ok()
}

/// Resolve a file path for reading, including macOS screenshot name variants.
pub(crate) fn resolve_read_path(file_path: &str, cwd: &Path) -> PathBuf {
    let resolved = normalize_dot_segments(&resolve_to_cwd(file_path, cwd));
    let normalized_cwd = normalize_dot_segments(cwd);
    let within_cwd = resolved.starts_with(&normalized_cwd);
    if within_cwd && file_exists(&resolved) {
        return resolved;
    }
    if !within_cwd {
        // Avoid probing the filesystem outside the working directory.
        return resolved;
    }

    let Some(resolved_str) = resolved.to_str() else {
        return resolved;
    };

    let am_pm_variant = try_mac_os_screenshot_path(resolved_str);
    if am_pm_variant.ne(resolved_str) {
        let candidate = PathBuf::from(&am_pm_variant);
        if candidate.starts_with(&normalized_cwd) && file_exists(&candidate) {
            return candidate;
        }
    }

    let nfd_variant = try_nfd_variant(resolved_str);
    if nfd_variant.ne(resolved_str) {
        let candidate = PathBuf::from(&nfd_variant);
        if candidate.starts_with(&normalized_cwd) && file_exists(&candidate) {
            return candidate;
        }
    }

    let curly_variant = try_curly_quote_variant(resolved_str);
    if curly_variant.ne(resolved_str) {
        let candidate = PathBuf::from(&curly_variant);
        if candidate.starts_with(&normalized_cwd) && file_exists(&candidate) {
            return candidate;
        }
    }

    let nfd_curly_variant = try_curly_quote_variant(&nfd_variant);
    if nfd_curly_variant.ne(resolved_str) {
        let candidate = PathBuf::from(&nfd_curly_variant);
        if candidate.starts_with(&normalized_cwd) && file_exists(&candidate) {
            return candidate;
        }
    }

    resolved
}

pub(crate) fn enforce_cwd_scope(path: &Path, cwd: &Path, action: &str) -> Result<PathBuf> {
    let canonical_path = pi_core::path_utils::safe_canonicalize(path);
    let canonical_cwd = pi_core::path_utils::safe_canonicalize(cwd);
    if !canonical_path.starts_with(&canonical_cwd) {
        return Err(Error::validation(format!(
            "Cannot {action} outside the working directory (resolved: {}, cwd: {})",
            canonical_path.display(),
            canonical_cwd.display()
        )));
    }
    Ok(canonical_path)
}

/// Same scoping contract as `enforce_cwd_scope`, but also accepts paths under
/// the configured pi-agent directory (`Config::global_dir()`, default
/// `~/.pi/agent/`, override via `PI_CODING_AGENT_DIR`).
///
/// Read access is broadened so the model can fetch the bodies of skill files,
/// prompt templates, and other resources that ship under the agent dir
/// without needing to fall back to a `bash cat`. See pi_agent_rust#71.
///
/// Symlink escapes remain blocked because `safe_canonicalize` resolves
/// symlinks before the prefix check, so e.g. `~/.pi/agent/skills/foo/SKILL.md`
/// pointing at `/etc/passwd` resolves to `/etc/passwd` and fails the prefix
/// test against both cwd and agent dir.
fn enforce_read_scope_with_roots(path: &Path, cwd: &Path, agent_dir: &Path) -> Result<PathBuf> {
    let canonical_path = pi_core::path_utils::safe_canonicalize(path);
    let canonical_cwd = pi_core::path_utils::safe_canonicalize(cwd);
    if canonical_path.starts_with(&canonical_cwd) {
        return Ok(canonical_path);
    }

    let canonical_agent = pi_core::path_utils::safe_canonicalize(agent_dir);
    if canonical_path.starts_with(&canonical_agent) {
        return Ok(canonical_path);
    }

    Err(Error::validation(format!(
        "Cannot read outside the working directory or agent dir \
         (resolved: {}, cwd: {}, agent dir: {})",
        canonical_path.display(),
        canonical_cwd.display(),
        canonical_agent.display(),
    )))
}

/// Convenience wrapper that pulls the agent dir from the active config.
fn enforce_read_scope(path: &Path, cwd: &Path) -> Result<PathBuf> {
    let agent_dir = crate::config::Config::global_dir();
    enforce_read_scope_with_roots(path, cwd, &agent_dir)
}

// ============================================================================
// CLI @file Processor (used by src/main.rs)
// ============================================================================

/// Result of processing `@file` CLI arguments.
#[derive(Debug, Clone, Default)]
pub struct ProcessedFiles {
    pub text: String,
    pub images: Vec<ImageContent>,
}

pub(crate) fn normalize_dot_segments(path: &Path) -> PathBuf {
    use std::ffi::{OsStr, OsString};
    use std::path::Component;

    let mut out = PathBuf::new();
    let mut normals: Vec<OsString> = Vec::new();
    let mut has_prefix = false;
    let mut has_root = false;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                out.push(prefix.as_os_str());
                has_prefix = true;
            }
            Component::RootDir => {
                out.push(component.as_os_str());
                has_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => match normals.last() {
                Some(last) if last.as_os_str() != OsStr::new("..") => {
                    normals.pop();
                }
                _ => {
                    if !has_root && !has_prefix {
                        normals.push(OsString::from(".."));
                    }
                }
            },
            Component::Normal(part) => normals.push(part.to_os_string()),
        }
    }

    for part in normals {
        out.push(part);
    }

    out
}

#[cfg(feature = "fuzzing")]
pub fn fuzz_normalize_dot_segments(path: &Path) -> PathBuf {
    normalize_dot_segments(path)
}

/// Returns `true` when an `fsync`/`fdatasync` durability barrier was *refused*
/// by the filesystem rather than reflecting a real write failure.
///
/// The bytes are already handed to the kernel by the preceding `write(2)`;
/// `fsync` only asks the filesystem to make them durable. Some filesystems —
/// notably virtiofs / FUSE bind mounts (Docker Desktop for macOS) and various
/// network filesystems — do not implement `fsync` on a given descriptor and
/// report it with `EBADF`, `EINVAL`, or an "unsupported" error even though the
/// `write(2)` and the subsequent atomic `rename(2)` already landed the data
/// correctly. Failing the whole write tool in that case is wrong: the file is
/// complete and correct on disk. We downgrade these specific refusals to a
/// warning. Genuine I/O failures (`EIO`, `ENOSPC`, `EDQUOT`, …) still
/// propagate. See issue #136.
pub(crate) fn is_fsync_refused(err: &std::io::Error) -> bool {
    // EBADF = 9 and EINVAL = 22 on both Linux and macOS. `ErrorKind::Unsupported`
    // captures ENOTSUP/EOPNOTSUPP/ENOSYS portably without a `libc` dependency
    // (this crate is `#![forbid(unsafe_code)]`).
    matches!(err.raw_os_error(), Some(9 | 22)) || err.kind() == std::io::ErrorKind::Unsupported
}

/// Runs a durability `fsync` (`result`), treating a filesystem *refusal* (see
/// [`is_fsync_refused`]) as a non-fatal warning while still propagating real
/// I/O errors. `what` and `path` are used only for the diagnostic log line.
pub(crate) fn tolerate_fsync_refusal(
    result: std::io::Result<()>,
    what: &str,
    path: &Path,
) -> std::io::Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(err) if is_fsync_refused(&err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "{what} fsync refused by filesystem (non-POSIX durability semantics); \
                 data already written, continuing without a durability barrier"
            );
            Ok(())
        }
        Err(err) => Err(err),
    }
}

#[cfg(unix)]
pub fn sync_parent_dir(path: &Path) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    // Directory fsync is a pure durability nicety (it makes the rename durable);
    // on filesystems that refuse it the rename is still visible, so tolerate a
    // refusal rather than failing the write. See issue #136.
    tolerate_fsync_refusal(
        std::fs::File::open(parent).and_then(|dir| dir.sync_all()),
        "parent directory",
        parent,
    )
}

#[allow(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
#[cfg(not(unix))]
pub fn sync_parent_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn escape_file_tag_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\n' => escaped.push_str("&#10;"),
            '\r' => escaped.push_str("&#13;"),
            '\t' => escaped.push_str("&#9;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escaped_file_tag_name(path: &Path) -> String {
    escape_file_tag_attribute(&path.display().to_string())
}

fn append_file_notice_block(out: &mut String, path: &Path, notice: &str) {
    let path_str = escaped_file_tag_name(path);
    let _ = writeln!(out, "<file name=\"{path_str}\">\n{notice}\n</file>");
}

fn append_image_file_ref(out: &mut String, path: &Path, note: Option<&str>) {
    let path_str = escaped_file_tag_name(path);
    match note {
        Some(text) => {
            let _ = writeln!(out, "<file name=\"{path_str}\">{text}</file>");
        }
        None => {
            let _ = writeln!(out, "<file name=\"{path_str}\"></file>");
        }
    }
}

fn append_text_file_block(out: &mut String, path: &Path, bytes: &[u8]) {
    let content = String::from_utf8_lossy(bytes);
    let path_str = escaped_file_tag_name(path);
    let _ = writeln!(out, "<file name=\"{path_str}\">");

    let truncation = truncate_head(content.into_owned(), DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);
    let needs_trailing_newline = !truncation.truncated && !truncation.content.ends_with('\n');
    out.push_str(&truncation.content);

    if truncation.truncated {
        let _ = write!(
            out,
            "\n... [Truncated: showing {}/{} lines, {}/{} bytes]",
            truncation.output_lines,
            truncation.total_lines,
            format_size(truncation.output_bytes),
            format_size(truncation.total_bytes)
        );
    } else if needs_trailing_newline {
        out.push('\n');
    }
    let _ = writeln!(out, "</file>");
}

fn maybe_append_image_argument(
    out: &mut ProcessedFiles,
    absolute_path: &Path,
    bytes: &[u8],
    auto_resize_images: bool,
) -> Result<bool> {
    let Some(mime_type) = detect_supported_image_mime_type_from_bytes(bytes) else {
        return Ok(false);
    };

    let resized = if auto_resize_images {
        resize_image_if_needed(bytes, mime_type)?
    } else {
        ResizedImage::original(bytes.to_vec(), mime_type)
    };

    if resized.bytes.len() > IMAGE_MAX_BYTES {
        let msg = if resized.resized {
            format!(
                "[Image is too large ({} bytes) after resizing. Max allowed is {} bytes.]",
                resized.bytes.len(),
                IMAGE_MAX_BYTES
            )
        } else {
            format!(
                "[Image is too large ({} bytes). Max allowed is {} bytes.]",
                resized.bytes.len(),
                IMAGE_MAX_BYTES
            )
        };
        append_file_notice_block(&mut out.text, absolute_path, &msg);
        return Ok(true);
    }

    let base64_data =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &resized.bytes);
    out.images.push(ImageContent {
        data: base64_data,
        mime_type: resized.mime_type.to_string(),
    });

    let note = if resized.resized {
        if let (Some(ow), Some(oh), Some(w), Some(h)) = (
            resized.original_width,
            resized.original_height,
            resized.width,
            resized.height,
        ) {
            if w > 0 {
                let scale = f64::from(ow) / f64::from(w);
                Some(format!(
                    "[Image: original {ow}x{oh}, displayed at {w}x{h}. Multiply coordinates by {scale:.2} to map to original image.]"
                ))
            } else {
                Some(format!(
                    "[Image: original {ow}x{oh}, displayed at {w}x{h}.]"
                ))
            }
        } else {
            None
        }
    } else {
        None
    };
    append_image_file_ref(&mut out.text, absolute_path, note.as_deref());
    Ok(true)
}

/// Process `@file` arguments into a single text prefix and image attachments.
///
/// Matches the legacy TypeScript behavior:
/// - Resolves paths (including `~` expansion + macOS screenshot variants)
/// - Skips empty files
/// - For images: attaches image blocks and appends `<file name="...">...</file>` references
/// - For text: embeds the file contents inside `<file>` tags
pub fn process_file_arguments(
    file_args: &[String],
    cwd: &Path,
    auto_resize_images: bool,
) -> Result<ProcessedFiles> {
    let mut out = ProcessedFiles::default();

    for file_arg in file_args {
        let resolved = resolve_read_path(file_arg, cwd);
        let absolute_path = normalize_dot_segments(&resolved);
        let absolute_path = enforce_read_scope(&absolute_path, cwd)?;

        let meta = std::fs::metadata(&absolute_path).map_err(|e| {
            Error::tool(
                "read",
                format!("Cannot access file {}: {e}", absolute_path.display()),
            )
        })?;
        if meta.is_dir() {
            append_file_notice_block(
                &mut out.text,
                &absolute_path,
                "[Path is a directory, not a file. Use the list tool to view its contents.]",
            );
            continue;
        }

        if meta.len() == 0 {
            continue;
        }

        if meta.len() > READ_TOOL_MAX_BYTES {
            append_file_notice_block(
                &mut out.text,
                &absolute_path,
                &format!(
                    "[File is too large ({} bytes). Max allowed is {} bytes.]",
                    meta.len(),
                    READ_TOOL_MAX_BYTES
                ),
            );
            continue;
        }

        let bytes = std::fs::read(&absolute_path).map_err(|e| {
            Error::tool(
                "read",
                format!("Could not read file {}: {e}", absolute_path.display()),
            )
        })?;

        if maybe_append_image_argument(&mut out, &absolute_path, &bytes, auto_resize_images)? {
            continue;
        }

        append_text_file_block(&mut out.text, &absolute_path, &bytes);
    }

    Ok(out)
}

/// Resolve a file path relative to the current working directory.
/// Public alias for `resolve_to_cwd` used by tools.
pub(crate) fn resolve_path(file_path: &str, cwd: &Path) -> PathBuf {
    normalize_dot_segments(&resolve_to_cwd(file_path, cwd))
}

#[cfg(feature = "fuzzing")]
pub fn fuzz_resolve_path(file_path: &str, cwd: &Path) -> PathBuf {
    resolve_path(file_path, cwd)
}

pub(crate) fn detect_supported_image_mime_type_from_bytes(bytes: &[u8]) -> Option<&'static str> {
    // Supported image types match the legacy tool: jpeg/png/gif/webp only.
    if bytes.len() >= 8 && bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some("image/jpeg");
    }
    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

#[derive(Debug, Clone)]
pub(crate) struct ResizedImage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) mime_type: &'static str,
    pub(crate) resized: bool,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) original_width: Option<u32>,
    pub(crate) original_height: Option<u32>,
}

impl ResizedImage {
    pub(crate) const fn original(bytes: Vec<u8>, mime_type: &'static str) -> Self {
        Self {
            bytes,
            mime_type,
            resized: false,
            width: None,
            height: None,
            original_width: None,
            original_height: None,
        }
    }
}

#[cfg(feature = "image-resize")]
#[allow(clippy::too_many_lines)]
pub(crate) fn resize_image_if_needed(
    bytes: &[u8],
    mime_type: &'static str,
) -> Result<ResizedImage> {
    // Match legacy behavior from pi-mono `utils/image-resize.ts`.
    //
    // Strategy:
    // 1) If image already fits within max dims AND max bytes: return original
    // 2) Otherwise resize to maxWidth/maxHeight (2000x2000)
    // 3) Encode as PNG and JPEG, pick smaller
    // 4) If still too large, try JPEG with different quality steps
    // 5) If still too large, progressively scale down dimensions
    //
    // Note: even if dimensions don't change, an oversized image may be re-encoded to fit max bytes.
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::png::PngEncoder;
    use image::imageops::FilterType;
    use image::{GenericImageView, ImageEncoder, ImageReader, Limits};
    use std::io::Cursor;

    const MAX_WIDTH: u32 = 2000;
    const MAX_HEIGHT: u32 = 2000;
    const DEFAULT_JPEG_QUALITY: u8 = 80;
    const QUALITY_STEPS: [u8; 4] = [85, 70, 55, 40];
    const SCALE_STEPS: [f64; 5] = [1.0, 0.75, 0.5, 0.35, 0.25];

    fn scale_u32(value: u32, numerator: u32, denominator: u32) -> u32 {
        let den = u64::from(denominator).max(1);
        let num = u64::from(value) * u64::from(numerator);
        let rounded = (num + den / 2) / den;
        u32::try_from(rounded).unwrap_or(u32::MAX)
    }

    fn encode_png(img: &image::DynamicImage) -> Result<Vec<u8>> {
        let rgba = img.to_rgba8();
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| Error::tool("read", format!("Failed to encode PNG: {e}")))?;
        Ok(out)
    }

    fn encode_jpeg(img: &image::DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let rgb = img.to_rgb8();
        let mut out = Vec::new();
        JpegEncoder::new_with_quality(&mut out, quality)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| Error::tool("read", format!("Failed to encode JPEG: {e}")))?;
        Ok(out)
    }

    fn try_both_formats(
        img: &image::DynamicImage,
        width: u32,
        height: u32,
        jpeg_quality: u8,
    ) -> Result<(Vec<u8>, &'static str)> {
        let resized = img.resize_exact(width, height, FilterType::Lanczos3);
        let png = encode_png(&resized)?;
        let jpeg = encode_jpeg(&resized, jpeg_quality)?;
        if png.len() <= jpeg.len() {
            Ok((png, "image/png"))
        } else {
            Ok((jpeg, "image/jpeg"))
        }
    }

    // Use ImageReader with explicit limits to prevent decompression bomb attacks.
    // 128MB allocation limit allows reasonable images but stops massive expansions.
    let mut limits = Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| Error::tool("read", format!("Failed to detect image format: {e}")))?;

    let mut reader = reader;
    reader.limits(limits);

    // ubs:ignore false positive: image decode, not JWT processing.
    let Ok(img) = reader.decode() else {
        return Ok(ResizedImage::original(bytes.to_vec(), mime_type));
    };

    let (original_width, original_height) = img.dimensions();
    let original_size = bytes.len();

    if original_width <= MAX_WIDTH
        && original_height <= MAX_HEIGHT
        && original_size <= IMAGE_MAX_BYTES
    {
        return Ok(ResizedImage {
            bytes: bytes.to_vec(),
            mime_type,
            resized: false,
            width: Some(original_width),
            height: Some(original_height),
            original_width: Some(original_width),
            original_height: Some(original_height),
        });
    }

    let mut target_width = original_width;
    let mut target_height = original_height;

    if target_width > MAX_WIDTH {
        target_height = scale_u32(target_height, MAX_WIDTH, target_width);
        target_width = MAX_WIDTH;
    }
    if target_height > MAX_HEIGHT {
        target_width = scale_u32(target_width, MAX_HEIGHT, target_height);
        target_height = MAX_HEIGHT;
    }

    let mut best = try_both_formats(&img, target_width, target_height, DEFAULT_JPEG_QUALITY)?;
    let mut final_width = target_width;
    let mut final_height = target_height;

    if best.0.len() <= IMAGE_MAX_BYTES {
        return Ok(ResizedImage {
            bytes: best.0,
            mime_type: best.1,
            resized: true,
            width: Some(final_width),
            height: Some(final_height),
            original_width: Some(original_width),
            original_height: Some(original_height),
        });
    }

    for quality in QUALITY_STEPS {
        best = try_both_formats(&img, target_width, target_height, quality)?;
        if best.0.len() <= IMAGE_MAX_BYTES {
            return Ok(ResizedImage {
                bytes: best.0,
                mime_type: best.1,
                resized: true,
                width: Some(final_width),
                height: Some(final_height),
                original_width: Some(original_width),
                original_height: Some(original_height),
            });
        }
    }

    for scale in SCALE_STEPS {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            final_width = (f64::from(target_width) * scale).round() as u32;
            final_height = (f64::from(target_height) * scale).round() as u32;
        }

        if final_width < 100 || final_height < 100 {
            break;
        }

        for quality in QUALITY_STEPS {
            best = try_both_formats(&img, final_width, final_height, quality)?;
            if best.0.len() <= IMAGE_MAX_BYTES {
                return Ok(ResizedImage {
                    bytes: best.0,
                    mime_type: best.1,
                    resized: true,
                    width: Some(final_width),
                    height: Some(final_height),
                    original_width: Some(original_width),
                    original_height: Some(original_height),
                });
            }
        }
    }

    Ok(ResizedImage {
        bytes: best.0,
        mime_type: best.1,
        resized: true,
        width: Some(final_width),
        height: Some(final_height),
        original_width: Some(original_width),
        original_height: Some(original_height),
    })
}

#[cfg(not(feature = "image-resize"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "The no-feature stub preserves the feature-enabled Result API at shared call sites."
)]
pub(crate) fn resize_image_if_needed(
    bytes: &[u8],
    mime_type: &'static str,
) -> Result<ResizedImage> {
    Ok(ResizedImage::original(bytes.to_vec(), mime_type))
}

// ============================================================================
// Tool Registry
// ============================================================================

/// Registry of enabled tools for a Pi run.
///
/// The registry is constructed from configuration (enabled tool names + settings) and is used for:
/// - Looking up a tool implementation by name during tool-call execution.
/// - Enumerating tool schemas when building provider requests.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    description_overrides: HashMap<String, String>,
}

impl ToolRegistry {
    /// Create a new registry with the specified tools enabled.
    pub fn new(enabled: &[&str], cwd: &Path, config: Option<ToolConfig>) -> Self {
        let mut tools: Vec<Box<dyn Tool>> = Vec::new();
        let config = config.unwrap_or_default();
        let shell_path = config.shell_path;
        let shell_command_prefix = config.shell_command_prefix;
        let image_auto_resize = config.image_auto_resize;
        let block_images = config.block_images;

        for name in enabled {
            match *name {
                "read" => tools.push(Box::new(ReadTool::with_settings(
                    cwd,
                    image_auto_resize,
                    block_images,
                ))),
                "pwsh" => tools.push(Box::new(PwshTool::new(cwd))),
                "bash" => tools.push(Box::new(BashTool::with_shell(
                    cwd,
                    shell_path.clone(),
                    shell_command_prefix.clone(),
                ))),
                "edit" => tools.push(Box::new(EditTool::new(cwd))),
                "write" => tools.push(Box::new(WriteTool::new(cwd))),
                "grep" => tools.push(Box::new(GrepTool::new(cwd))),
                "find" => tools.push(Box::new(FindTool::new(cwd))),
                "ls" => tools.push(Box::new(LsTool::new(cwd))),
                "hashline_edit" => tools.push(Box::new(HashlineEditTool::new(cwd))),
                _ => {}
            }
        }

        let description_overrides = config.tool_descriptions;
        Self {
            tools,
            description_overrides,
        }
    }

    /// Construct a registry from a pre-built tool list.
    pub fn from_tools(tools: Vec<Box<dyn Tool>>) -> Self {
        Self {
            tools,
            description_overrides: HashMap::new(),
        }
    }

    /// Convert the registry into the owned tool list.
    pub fn into_tools(self) -> Vec<Box<dyn Tool>> {
        self.tools
    }

    /// Append a tool.
    pub fn push(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Remove tools matching a predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Box<dyn Tool>) -> bool,
    {
        self.tools.retain(f);
    }

    /// Extend the registry with additional tools.
    pub fn extend<I>(&mut self, tools: I)
    where
        I: IntoIterator<Item = Box<dyn Tool>>,
    {
        self.tools.extend(tools);
    }

    /// Get all tools.
    pub fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    /// Find a tool by name.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(std::convert::AsRef::as_ref)
    }

    /// Get the description override for a tool, if any.
    pub fn description_override(&self, name: &str) -> Option<&str> {
        self.description_overrides.get(name).map(String::as_str)
    }
}
// ============================================================================
// Cleanup
// ============================================================================

/// Clean up old temporary files created by the bash tool.
///
/// Scans the system temporary directory for files matching `pi-bash-*.log`
/// that are older than 24 hours and deletes them. This prevents indefinite
/// accumulation of log files from long-running sessions.
pub fn cleanup_temp_files() {
    // Run in a detached thread to avoid blocking startup/shutdown.
    std::thread::spawn(|| {
        let temp_dir = std::env::temp_dir();
        let Ok(entries) = std::fs::read_dir(&temp_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            // Match "pi-bash-" or "pi-rpc-bash-" prefix and ".log" suffix.
            if (file_name.starts_with("pi-bash-") || file_name.starts_with("pi-rpc-bash-"))
                && std::path::Path::new(file_name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
                && let Ok(metadata) = entry.metadata()
                && metadata.modified().is_ok_and(|modified| {
                    modified
                        .elapsed()
                        .is_ok_and(|age| age > Duration::from_secs(24 * 60 * 60))
                })
                && let Err(e) = std::fs::remove_file(&path)
            {
                // Log but don't panic on cleanup failure
                tracing::debug!("Failed to remove temp file {}: {}", path.display(), e);
            }
        }
    });
}

// ============================================================================
// Helper functions
// ============================================================================

pub(crate) fn rg_available() -> bool {
    find_rg_binary().is_some()
}

/// Returns `true` if the `bash` binary is reachable on the current system.
///
/// Matches the shell resolution logic in [`run_bash_command`] so that
/// tests skip consistently wherever the tool itself would fail to spawn.
pub(crate) fn bash_available() -> bool {
    // Mirror the exact lookup BashTool uses: check Unix paths, then fallback to "sh".
    for path in ["/bin/bash", "/usr/bin/bash", "/usr/local/bin/bash"] {
        if Path::new(path).exists() {
            return true;
        }
    }
    // On platforms where none of the Unix paths exist (e.g. Windows), fall back
    // to checking whether "sh" is in PATH — the same fallback BashTool uses.
    std::process::Command::new("sh")
        .arg("-c")
        .arg("true")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub(crate) fn pump_stream<R: Read + Send + 'static>(
    mut reader: R,
    stream_name: &'static str,
    tx: &mpsc::SyncSender<BashPipeFrame>,
) {
    let mut buf = vec![0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if tx.send(BashPipeFrame::Chunk(buf[..n].to_vec())).is_err() {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(err) => {
                let _ = tx.send(BashPipeFrame::Error(format!(
                    "Failed to read bash {stream_name}: {err}"
                )));
                break;
            }
        }
    }
}

async fn ingest_bash_pipe_frame(frame: BashPipeFrame, state: &mut BashOutputState) -> Result<()> {
    match frame {
        BashPipeFrame::Chunk(chunk) => ingest_bash_chunk(chunk, state).await,
        BashPipeFrame::Error(message) => {
            let error_message = bash_capture_error_message(&message, state);
            state.abandon_spill_file();
            Err(Error::tool("bash", error_message))
        }
    }
}

fn bash_capture_error_message(message: &str, state: &BashOutputState) -> String {
    let raw = concat_chunks(&state.chunks);
    if raw.is_empty() {
        return message.to_string();
    }

    let full_text = String::from_utf8_lossy(&raw).into_owned();
    let truncation = truncate_tail(full_text, DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);
    let mut error_message = message.to_string();
    let partial_output = if truncation.content.is_empty() {
        "(no output)".to_string()
    } else {
        truncation.content
    };
    let _ = write!(
        error_message,
        "\n\nPartial output before failure:\n{partial_output}"
    );
    if truncation.truncated || state.total_bytes > state.chunks_bytes {
        let _ = write!(
            error_message,
            "\n\n[Partial output truncated before failure]"
        );
    }
    error_message
}

/// Read from a subprocess pipe until EOF while retaining only the first
/// `max_bytes + 1` bytes in memory so callers can detect truncation without
/// changing child-process behavior by closing the pipe early.
pub(crate) fn read_to_end_capped_and_drain<R: Read>(
    mut reader: R,
    max_bytes: u64,
) -> std::result::Result<Vec<u8>, String> {
    let capture_limit = usize::try_from(max_bytes.saturating_add(1)).unwrap_or(usize::MAX);
    let mut captured = Vec::with_capacity(capture_limit.min(8192));
    let mut chunk = [0u8; 8192];

    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                let remaining = capture_limit.saturating_sub(captured.len());
                if remaining > 0 {
                    let keep = remaining.min(read);
                    captured.extend_from_slice(&chunk[..keep]);
                }
            }
            Err(err) if matches!(err.kind(), std::io::ErrorKind::Interrupted) => {}
            Err(err) => return Err(err.to_string()),
        }
    }

    Ok(captured)
}

// Keep `rx` as `&mut Receiver`: `std::sync::mpsc::Receiver` is `Send` but not
// `Sync`, and this helper awaits between polls, so `&Receiver` would make the
// surrounding future non-Send.
#[allow(clippy::needless_pass_by_ref_mut)]
#[cfg(test)]
async fn drain_bash_output(
    rx: &mut mpsc::Receiver<BashPipeFrame>,
    bash_output: &mut BashOutputState,
    cx: &AgentCx,
    drain_deadline: asupersync::Time,
    tick: Duration,
    allow_cancellation: bool,
) -> Result<bool> {
    loop {
        match rx.try_recv() {
            Ok(frame) => ingest_bash_pipe_frame(frame, bash_output).await?,
            Err(mpsc::TryRecvError::Empty) => {
                let now = cx
                    .cx()
                    .timer_driver()
                    .map_or_else(wall_now, |timer| timer.now());
                if now >= drain_deadline {
                    return Ok(false);
                }
                if allow_cancellation && cx.checkpoint().is_err() {
                    return Ok(true);
                }
                sleep(now, tick).await;
            }
            Err(mpsc::TryRecvError::Disconnected) => return Ok(false),
        }
    }
}

pub(crate) fn concat_chunks(chunks: &VecDeque<Vec<u8>>) -> Vec<u8> {
    let total: usize = chunks.iter().map(Vec::len).sum();
    let mut out = Vec::with_capacity(total);
    for chunk in chunks {
        out.extend_from_slice(chunk);
    }
    out
}

pub(crate) struct BashOutputState {
    total_bytes: usize,
    line_count: usize,
    last_byte_was_newline: bool,
    start_time: std::time::Instant,
    timeout_ms: Option<u64>,
    temp_file_path: Option<PathBuf>,
    temp_file: Option<asupersync::fs::File>,
    chunks: VecDeque<Vec<u8>>,
    chunks_bytes: usize,
    max_chunks_bytes: usize,
    spill_failed: bool,
}

impl BashOutputState {
    fn new(max_chunks_bytes: usize) -> Self {
        Self {
            total_bytes: 0,
            line_count: 0,
            last_byte_was_newline: false,
            start_time: std::time::Instant::now(),
            timeout_ms: None,
            temp_file_path: None,
            temp_file: None,
            chunks: VecDeque::new(),
            chunks_bytes: 0,
            max_chunks_bytes,
            spill_failed: false,
        }
    }

    fn abandon_spill_file(&mut self) {
        self.spill_failed = true;
        self.temp_file = None;
        if let Some(path) = self.temp_file_path.take() {
            if let Err(e) = std::fs::remove_file(&path)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                tracing::debug!(
                    "Failed to remove incomplete bash spill file {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn ingest_bash_chunk(chunk: Vec<u8>, state: &mut BashOutputState) -> Result<()> {
    if chunk.is_empty() {
        return Ok(());
    }

    state.last_byte_was_newline = chunk.last().is_some_and(|byte| *byte == b'\n');
    state.total_bytes = state.total_bytes.saturating_add(chunk.len());
    state.line_count = state
        .line_count
        .saturating_add(memchr::memchr_iter(b'\n', &chunk).count());

    if state.total_bytes > DEFAULT_MAX_BYTES
        && state.temp_file.is_none()
        && state.temp_file_path.is_none()
        && !state.spill_failed
    {
        let id_full = Uuid::new_v4().simple().to_string();
        let id = &id_full[..16];
        let path = std::env::temp_dir().join(format!("pi-bash-{id}.log"));

        // Create the file synchronously with restricted permissions to avoid
        // a race condition where the file is world-readable before we fix it.
        // We also capture the inode (on Unix) to verify identity later.
        let path_clone = path.clone();
        let expected_inode: Option<u64> =
            asupersync::runtime::spawn_blocking_io(move || -> std::io::Result<Option<u64>> {
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);

                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }

                match options.open(&path_clone) {
                    Ok(file) => {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::MetadataExt;
                            Ok(file.metadata().ok().map(|m| m.ino()))
                        }
                        #[cfg(not(unix))]
                        {
                            drop(file);
                            Ok(None)
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create bash temp file: {e}");
                        Ok(None)
                    }
                }
            })
            .await
            .unwrap_or(None);

        if expected_inode.is_some() || !cfg!(unix) {
            match asupersync::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .await
            {
                Ok(mut file) => {
                    #[cfg_attr(not(unix), allow(unused_mut))]
                    let mut identity_match = true;
                    #[cfg(unix)]
                    if let Some(expected) = expected_inode {
                        use std::os::unix::fs::MetadataExt;
                        // asupersync 0.3.6's fs::Metadata no longer exposes the
                        // inode (and fs::File has no general AsRawFd), so re-stat
                        // the path with std symlink_metadata (does not follow
                        // symlinks) for the TOCTOU/identity guard.
                        match std::fs::symlink_metadata(&path) {
                            Ok(meta) => {
                                if !meta.ino().eq(&expected) {
                                    tracing::warn!(
                                        "Temp file identity mismatch (possible TOCTOU attack)"
                                    );
                                    identity_match = false;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to stat temp file: {e}");
                                identity_match = false;
                            }
                        }
                    }

                    if identity_match {
                        // Write buffered chunks to file first so it contains output from the beginning.
                        let mut failed_flush = false;
                        for existing in &state.chunks {
                            if let Err(e) = file.write_all(existing).await {
                                tracing::warn!("Failed to flush bash chunk to temp file: {e}");
                                failed_flush = true;
                                break;
                            }
                        }

                        state.temp_file_path = Some(path);
                        if failed_flush {
                            state.abandon_spill_file();
                        } else {
                            state.temp_file = Some(file);
                        }
                    } else {
                        state.temp_file_path = Some(path);
                        state.abandon_spill_file();
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to open temp file async: {e}");
                    state.temp_file_path = Some(path);
                    state.abandon_spill_file();
                }
            }
        } else {
            state.spill_failed = true;
        }
    }

    let mut close_spill_file = false;
    if let Some(file) = state.temp_file.as_mut() {
        let mut abandon_spill_file = false;
        if state.total_bytes <= BASH_FILE_LIMIT_BYTES {
            if let Err(e) = file.write_all(&chunk).await {
                tracing::warn!("Failed to write bash chunk to temp file: {e}");
                abandon_spill_file = true;
            }
        } else {
            // Hard limit reached. Stop writing and close the file to release the FD.
            if !state.spill_failed {
                tracing::warn!("Bash output exceeded hard limit; stopping file log");
                close_spill_file = true;
            }
        }
        if abandon_spill_file {
            state.abandon_spill_file();
        }
    }
    if close_spill_file {
        state.temp_file = None;
    }

    state.chunks_bytes = state.chunks_bytes.saturating_add(chunk.len());
    state.chunks.push_back(chunk);
    while state.chunks_bytes > state.max_chunks_bytes && state.chunks.len() > 1 {
        if let Some(front) = state.chunks.pop_front() {
            state.chunks_bytes = state.chunks_bytes.saturating_sub(front.len());
        }
    }
    Ok(())
}

const fn line_count_from_newline_count(
    total_bytes: usize,
    newline_count: usize,
    last_byte_was_newline: bool,
) -> usize {
    if total_bytes == 0 {
        0
    } else if last_byte_was_newline {
        newline_count
    } else {
        newline_count.saturating_add(1)
    }
}

pub(crate) fn emit_bash_update(
    state: &BashOutputState,
    on_update: Option<&(dyn Fn(ToolUpdate) + Send + Sync)>,
) -> Result<()> {
    if let Some(callback) = on_update {
        let raw = concat_chunks(&state.chunks);
        let full_text = String::from_utf8_lossy(&raw);
        let truncation =
            truncate_tail(full_text.into_owned(), DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);

        // Build the progress + details JSON using the json! macro instead of
        // manual Map::insert calls.  This eliminates 7+ String heap
        // allocations per update for the constant field-name keys
        // ("elapsedMs", "lineCount", …) that the manual path required.
        let elapsed_ms = state.start_time.elapsed().as_millis();
        let line_count = line_count_from_newline_count(
            state.total_bytes,
            state.line_count,
            state.last_byte_was_newline,
        );
        let mut details = serde_json::json!({
            "progress": {
                "elapsedMs": elapsed_ms,
                "lineCount": line_count,
                "byteCount": state.total_bytes
            }
        });
        let Some(details_map) = details.as_object_mut() else {
            return Ok(());
        };

        if let Some(timeout) = state.timeout_ms {
            if let Some(progress) = details_map
                .get_mut("progress")
                .and_then(|v| v.as_object_mut())
            {
                progress.insert("timeoutMs".into(), serde_json::json!(timeout));
            }
        }
        if truncation.truncated {
            details_map.insert("truncation".into(), serde_json::to_value(&truncation)?);
        }
        if let Some(path) = state.temp_file_path.as_ref() {
            details_map.insert(
                "fullOutputPath".into(),
                serde_json::Value::String(path.display().to_string()),
            );
        }

        callback(ToolUpdate {
            content: vec![ContentBlock::Text(TextContent::new(truncation.content))],
            details: Some(details),
        });
    }
    Ok(())
}

pub(crate) struct ProcessGuard {
    child: Option<std::process::Child>,
    cleanup_mode: ProcessCleanupMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessCleanupMode {
    ChildOnly,
    ProcessGroupTree,
}

impl ProcessGuard {
    pub(crate) const fn new(child: std::process::Child, cleanup_mode: ProcessCleanupMode) -> Self {
        Self {
            child: Some(child),
            cleanup_mode,
        }
    }

    /// Spawn a command and wrap it in a `ProcessGuard`.
    ///
    /// Convenience constructor that spawns the command and wraps the child
    /// process in a guard with automatic cleanup on drop.
    pub(crate) fn spawn_managed(
        cmd: &mut std::process::Command,
        cleanup_mode: ProcessCleanupMode,
    ) -> std::io::Result<Self> {
        let child = cmd.spawn()?;
        Ok(Self {
            child: Some(child),
            cleanup_mode,
        })
    }

    /// Wait for the child process with timeout, ambient cancellation, and abort signal support.
    ///
    /// Polls the child in a loop, checking for:
    /// - Ambient cancellation via [`AgentCx::checkpoint`]
    /// - Explicit abort via [`AbortSignal`]
    /// - Timeout expiration
    ///
    /// Returns `Ok(Some(exit_code))` if the child exited normally.
    /// Returns `Ok(None)` if the child was killed (timeout, cancellation, or abort).
    /// Returns `Err(io::Error)` on I/O error.
    pub(crate) async fn wait_with_cancellation(
        &mut self,
        timeout_secs: Option<u64>,
        abort: Option<&AbortSignal>,
    ) -> std::io::Result<Option<i32>> {
        let start = std::time::Instant::now();
        loop {
            // Check for ambient cancellation
            let agent_cx = AgentCx::for_current_or_request();
            let cx = agent_cx.cx();
            if cx.checkpoint().is_err() {
                self.kill();
                return Ok(None);
            }

            // Check explicit abort signal
            if let Some(signal) = abort {
                if signal.is_aborted() {
                    self.kill();
                    return Ok(None);
                }
            }

            // Check timeout
            let remaining =
                timeout_secs.map_or(u64::MAX, |s| s.saturating_sub(start.elapsed().as_secs()));
            if remaining == 0 {
                self.kill();
                return Ok(None);
            }

            // Try to get child exit status
            if let Some(status) = self.try_wait_child()? {
                return Ok(Some(status.code().unwrap_or(-1)));
            }

            // Adaptive sleep: shorter interval when close to timeout
            let now = wall_now();
            if remaining > 0 && remaining < 10 {
                sleep(now, Duration::from_millis(50)).await;
            } else {
                sleep(now, Duration::from_millis(100)).await;
            }
        }
    }

    pub(crate) fn try_wait_child(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child
            .as_mut()
            .map_or(Ok(None), std::process::Child::try_wait)
    }

    pub(crate) fn kill(&mut self) -> Option<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            cleanup_child(Some(child.id()), self.cleanup_mode);
            let _ = child.kill();
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            // We cannot return the exit status synchronously without blocking,
            // so we return None to indicate the process was forcefully killed.
            return None;
        }
        None
    }

    pub(crate) fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            return child.wait();
        }
        Err(std::io::Error::other("Already waited"))
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(None) => {}
                Ok(Some(_)) | Err(_) => return,
            }
            let cleanup_mode = self.cleanup_mode;
            std::thread::spawn(move || {
                cleanup_child(Some(child.id()), cleanup_mode);
                let _ = child.kill();
                let _ = child.wait();
            });
        }
    }
}

fn cleanup_child(pid: Option<u32>, cleanup_mode: ProcessCleanupMode) {
    if cleanup_mode == ProcessCleanupMode::ProcessGroupTree {
        kill_process_group_tree(pid);
    }
}

pub fn kill_process_tree(pid: Option<u32>) {
    kill_process_tree_with(pid, sysinfo::Signal::Kill, false);
}

pub(crate) fn kill_process_group_tree(pid: Option<u32>) {
    kill_process_tree_with(pid, sysinfo::Signal::Kill, true);
}

pub(crate) fn terminate_process_group_tree(pid: Option<u32>) {
    kill_process_tree_with(pid, sysinfo::Signal::Term, true);
}

fn kill_process_tree_with(pid: Option<u32>, signal: sysinfo::Signal, include_process_group: bool) {
    let Some(pid) = pid else {
        return;
    };

    let root = sysinfo::Pid::from_u32(pid);

    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut children_map: HashMap<sysinfo::Pid, Vec<sysinfo::Pid>> = HashMap::new();
    for (p, proc_) in sys.processes() {
        if let Some(parent) = proc_.parent() {
            children_map.entry(parent).or_default().push(*p);
        }
    }

    let mut to_kill = Vec::new();
    let mut visited = std::collections::HashSet::new();
    collect_process_tree(root, &children_map, &mut to_kill, &mut visited);

    if include_process_group {
        // Some subprocess surfaces isolate the child into its own process group.
        // When they do, killing the group first catches background children even
        // if they have already been reparented away from the original root PID.
        #[cfg(unix)]
        {
            let sig_num = match signal {
                sysinfo::Signal::Kill => "9",
                _ => "15",
            };
            let _ = Command::new("kill")
                .arg(format!("-{sig_num}"))
                .arg("--")
                .arg(format!("-{pid}"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

    // Kill children first.
    for pid in to_kill.into_iter().rev() {
        if let Some(proc_) = sys.process(pid) {
            match proc_.kill_with(signal) {
                Some(true) => {}
                Some(false) | None => {
                    let _ = proc_.kill();
                }
            }
        }
    }
}

fn collect_process_tree(
    pid: sysinfo::Pid,
    children_map: &HashMap<sysinfo::Pid, Vec<sysinfo::Pid>>,
    out: &mut Vec<sysinfo::Pid>,
    visited: &mut std::collections::HashSet<sysinfo::Pid>,
) {
    if !visited.insert(pid) {
        return;
    }
    out.push(pid);
    if let Some(children) = children_map.get(&pid) {
        for child in children {
            collect_process_tree(*child, children_map, out, visited);
        }
    }
}

/// Build a child command whose Unix process image starts with SIGPIPE restored
/// to the platform default, without using `Command::pre_exec`.
///
/// Rust binaries ignore SIGPIPE by default, and POSIX inherits that disposition
/// across `exec(2)`. The tiny `/bin/sh` trampoline resets PIPE and then `exec`s
/// the requested program, preserving argv, cwd, stdio, and the process id that
/// later becomes the isolated process-group leader.
pub(crate) const SIGPIPE_TRAMPOLINE_EXEC_FAILURE_PREFIX: &str = "pi-sigpipe-reset: exec failed:";

pub(crate) fn command_with_default_sigpipe(program: impl AsRef<OsStr>) -> std::io::Result<Command> {
    command_with_default_sigpipe_for_cwd(program.as_ref(), None)
}

/// Variant of [`command_with_default_sigpipe`] for commands that will run with
/// `current_dir(cwd)`. This preserves relative `./program` lookup semantics.
pub(crate) fn command_with_default_sigpipe_in_dir(
    program: impl AsRef<OsStr>,
    cwd: &Path,
) -> std::io::Result<Command> {
    command_with_default_sigpipe_for_cwd(program.as_ref(), Some(cwd))
}

#[cfg(unix)]
fn command_with_default_sigpipe_for_cwd(
    program: &OsStr,
    cwd: Option<&Path>,
) -> std::io::Result<Command> {
    let program = resolve_executable_for_shell_trampoline(program, cwd)?;
    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg(
            "trap - PIPE\n\
             exec \"$@\"\n\
             status=$?\n\
             printf 'pi-sigpipe-reset: exec failed: %s\\n' \"$1\" >&2\n\
             exit \"$status\"",
        )
        .arg("pi-sigpipe-reset")
        .arg(program);
    Ok(command)
}

#[allow(clippy::unnecessary_wraps)]
#[cfg(not(unix))]
fn command_with_default_sigpipe_for_cwd(
    program: &OsStr,
    _cwd: Option<&Path>,
) -> std::io::Result<Command> {
    let command = Command::new(program); // ubs:ignore policy-checked non-Unix command runner
    Ok(command)
}

#[cfg(unix)]
pub(crate) fn resolve_executable_for_shell_trampoline(
    program: &OsStr,
    cwd: Option<&Path>,
) -> std::io::Result<OsString> {
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::fs::PermissionsExt as _;

    use std::ffi::OsString;
    fn executable_candidate(path: &Path) -> std::io::Result<bool> {
        let metadata = std::fs::metadata(path)?;
        Ok(metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    }

    fn absolutize_candidate(path: &Path, cwd: Option<&Path>) -> std::io::Result<PathBuf> {
        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }

        let base = std::env::current_dir()?;
        Ok(cwd.map_or_else(|| base.join(path), |cwd| base.join(cwd).join(path)))
    }

    if program.as_bytes().contains(&b'/') {
        let path = Path::new(program);
        let candidate = absolutize_candidate(path, cwd)?;
        if executable_candidate(&candidate)? {
            return Ok(candidate.into_os_string());
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("not an executable file: {}", candidate.display()),
        ));
    }

    let mut permission_denied = false;
    let paths = std::env::var_os("PATH").unwrap_or_else(|| OsString::from("/bin:/usr/bin"));
    for dir in std::env::split_paths(&paths) {
        let candidate = absolutize_candidate(&dir.join(program), cwd)?;
        match executable_candidate(&candidate) {
            Ok(true) => return Ok(candidate.into_os_string()),
            Ok(false) => permission_denied = true,
            Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound) => {}
            Err(err) if matches!(err.kind(), std::io::ErrorKind::PermissionDenied) => {
                permission_denied = true;
            }
            Err(_) => {}
        }
    }

    if permission_denied {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("command is not executable: {}", program.to_string_lossy()),
        ))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("command not found: {}", program.to_string_lossy()),
        ))
    }
}

/// Detach a child process from pi's controlling terminal.
#[allow(clippy::missing_const_for_fn, clippy::needless_pass_by_ref_mut)]
pub fn isolate_command_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }

    #[cfg(not(unix))]
    {
        let _ = command;
    }
}

pub fn format_grep_path(file_path: &Path, cwd: &Path) -> String {
    if let Ok(rel) = file_path.strip_prefix(cwd) {
        let rel_str = rel.display().to_string().replace('\\', "/");
        if !rel_str.is_empty() {
            return rel_str;
        }
    }

    let canonical_file = safe_canonicalize(file_path);
    let canonical_cwd = safe_canonicalize(cwd);
    if let Ok(rel) = canonical_file.strip_prefix(&canonical_cwd) {
        let rel_str = rel.display().to_string().replace('\\', "/");
        if !rel_str.is_empty() {
            return rel_str;
        }
    }

    file_path.display().to_string().replace('\\', "/")
}

async fn get_file_lines_async<'a>(
    path: &Path,
    cache: &'a mut HashMap<PathBuf, Vec<String>>,
) -> &'a [String] {
    if !cache.contains_key(path) {
        // Prevent OOM on huge files and hangs on pipes
        if let Ok(meta) = asupersync::fs::metadata(path).await {
            if !meta.is_file() || meta.len() > 10 * 1024 * 1024 {
                cache.insert(path.to_path_buf(), Vec::new());
                return &[];
            }
        } else {
            cache.insert(path.to_path_buf(), Vec::new());
            return &[];
        }

        // Match Node's `readFileSync(..., "utf-8")` behavior: decode lossily rather than failing.
        let bytes = match asupersync::fs::read(path).await {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::debug!("Failed to read grep file {}: {err}", path.display());
                cache.insert(path.to_path_buf(), Vec::new());
                return &[];
            }
        };
        let content = String::from_utf8_lossy(&bytes);
        let mut lines = Vec::new();
        for line in content.split('\n') {
            let trimmed = line.strip_suffix('\r').unwrap_or(line);
            for piece in trimmed.split('\r') {
                lines.push(piece.to_string());
            }
        }
        if content.ends_with('\n') && lines.last().is_some_and(std::string::String::is_empty) {
            lines.pop();
        }
        cache.insert(path.to_path_buf(), lines);
    }
    if let Some(lines) = cache.get(path) {
        lines.as_slice()
    } else {
        &[]
    }
}

pub(crate) fn find_fd_binary() -> Option<&'static str> {
    static BINARY: OnceLock<Option<&'static str>> = OnceLock::new();
    *BINARY.get_or_init(|| {
        if std::process::Command::new("fd")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some("fd");
        }
        if std::process::Command::new("fdfind")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some("fdfind");
        }
        None
    })
}

pub(crate) fn find_rg_binary() -> Option<&'static str> {
    static BINARY: OnceLock<Option<&'static str>> = OnceLock::new();
    *BINARY.get_or_init(|| {
        if std::process::Command::new("rg")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some("rg");
        }
        if std::process::Command::new("ripgrep")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some("ripgrep");
        }
        None
    })
}

/// Try an atomic rename (`NamedTempFile::persist`).  On Windows,
/// `MoveFileEx` fails with `ERROR_ACCESS_DENIED` when the destination
/// file has `FILE_ATTRIBUTE_READONLY`.  If the first attempt hits
/// `PermissionDenied` and the target has the readonly attribute, strip
/// the attribute, retry, and restore it on the new file.
pub fn persist_with_readonly_handling(
    temp_file: tempfile::NamedTempFile,
    target: &Path,
) -> std::io::Result<()> {
    match temp_file.persist(target) {
        Ok(_) => Ok(()),
        Err(e) => {
            let err = e.error;
            if err.kind() != std::io::ErrorKind::PermissionDenied {
                return Err(err);
            }

            let temp_file = e.file;

            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt as _;

                if let Ok(meta) = std::fs::metadata(target) {
                    if meta.file_attributes() & 0x1 != 0 {
                        let mut perms = meta.permissions();
                        perms.set_readonly(false);
                        std::fs::set_permissions(target, perms)?;

                        match temp_file.persist(target) {
                            Ok(_) => {
                                if let Ok(meta) = std::fs::metadata(target) {
                                    let mut perms = meta.permissions();
                                    perms.set_readonly(true);
                                    let _ = std::fs::set_permissions(target, perms);
                                }
                                return Ok(());
                            }
                            Err(e2) => return Err(e2.error),
                        }
                    }
                }
            }

            Err(err)
        }
    }
}
