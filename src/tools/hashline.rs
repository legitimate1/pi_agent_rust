use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use async_trait::async_trait;
use serde::Deserialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use super::edit::{normalize_to_lf, strip_bom, detect_line_ending, restore_line_endings, generate_diff_string};
// ============================================================================
// Hashline Edit Tool
// ============================================================================

/// Custom nibble-encoding alphabet used for hashline tags.
pub const NIBBLE_STR: &[u8; 16] = b"ZPMQVRWSNKTXJBYH";

/// Pre-computed 256-entry lookup table mapping each byte value to its
/// 2-character NIBBLE_STR encoding.
static HASHLINE_DICT: OnceLock<[[u8; 2]; 256]> = OnceLock::new();

fn hashline_dict() -> &'static [[u8; 2]; 256] {
    HASHLINE_DICT.get_or_init(|| {
        let mut dict = [[0u8; 2]; 256];
        for i in 0..256 {
            dict[i] = [NIBBLE_STR[i & 0x0F], NIBBLE_STR[(i >> 4) & 0x0F]];
        }
        dict
    })
}

/// Compute a 2-character hash tag for a line at the given 0-indexed position.
///
/// The algorithm:
/// 1. Strip trailing `\r`
/// 2. Remove all whitespace to get a "significant" string
/// 3. If the significant string contains at least one letter or digit, seed = 0;
///    otherwise seed = line index (to disambiguate punctuation-only or blank lines)
/// 4. Compute `xxh32(significant_bytes, seed) & 0xFF`
/// 5. Encode the low byte as 2 nibble chars from `NIBBLE_STR`
pub fn compute_line_hash(line_idx: usize, line: &str) -> [u8; 2] {
    let line = line.strip_suffix('\r').unwrap_or(line);
    // Remove all whitespace
    let significant: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let has_alnum = significant.chars().any(char::is_alphanumeric);
    let seed = if has_alnum {
        0
    } else {
        #[allow(clippy::cast_possible_truncation)]
        let s = line_idx as u32;
        s
    };
    let hash = xxhash_rust::xxh32::xxh32(significant.as_bytes(), seed);
    let byte = (hash & 0xFF) as usize;
    hashline_dict()[byte]
}

/// Format a hashline tag as `"N#AB"` where N is the 1-indexed line number.
pub fn format_hashline_tag(line_idx: usize, line: &str) -> String {
    let h = compute_line_hash(line_idx, line);
    format!("{}#{}{}", line_idx + 1, h[0] as char, h[1] as char)
}

/// Compute a hashline tag, reapplying a stripped BOM for the first line if needed.
fn format_hashline_tag_with_bom(line_idx: usize, line: &str, had_bom: bool) -> String {
    let h = compute_line_hash_with_bom(line_idx, line, had_bom);
    format!("{}#{}{}", line_idx + 1, h[0] as char, h[1] as char)
}

fn compute_line_hash_with_bom(line_idx: usize, line: &str, had_bom: bool) -> [u8; 2] {
    if had_bom && line_idx == 0 {
        let mut with_bom = String::with_capacity(line.len().saturating_add(1));
        with_bom.push('\u{FEFF}');
        with_bom.push_str(line);
        compute_line_hash(line_idx, &with_bom)
    } else {
        compute_line_hash(line_idx, line)
    }
}

/// Regex for parsing hashline references like `5#KJ` or ` > +  5 # KJ `.
/// Tolerates leading whitespace, diff markers (`>`, `+`, `-`), and spaces around `#`.
static HASHLINE_TAG_RE: OnceLock<regex::Regex> = OnceLock::new();

pub fn hashline_tag_regex() -> &'static regex::Regex {
    HASHLINE_TAG_RE.get_or_init(|| {
        regex::Regex::new(r"^[\s>+\-]*(\d+)\s*#\s*([ZPMQVRWSNKTXJBYH]{2})")
            .expect("valid hashline regex")
    })
}

/// Parse a hashline tag reference string into (1-indexed line number, 2-byte hash).
pub fn parse_hashline_tag(ref_str: &str) -> std::result::Result<(usize, [u8; 2]), String> {
    let re = hashline_tag_regex();
    let caps = re
        .captures(ref_str)
        .ok_or_else(|| format!("Invalid hashline reference: {ref_str:?}"))?;
    let line_num: usize = caps[1]
        .parse()
        .map_err(|e| format!("Invalid line number in {ref_str:?}: {e}"))?;
    if line_num == 0 {
        return Err(format!("Line number must be >= 1, got 0 in {ref_str:?}"));
    }
    let hash_bytes = caps[2].as_bytes();
    Ok((line_num, [hash_bytes[0], hash_bytes[1]]))
}

/// Strip hashline tag prefixes that models sometimes copy into replacement content.
/// Matches patterns like `5#KJ:content` and returns just `content`.
static HASHLINE_PREFIX_RE: OnceLock<regex::Regex> = OnceLock::new();

pub fn strip_hashline_prefix(line: &str) -> &str {
    let re = HASHLINE_PREFIX_RE.get_or_init(|| {
        regex::Regex::new(r"^[\s>+\-]*\d+\s*#\s*[ZPMQVRWSNKTXJBYH]{2}\s*:")
            .expect("valid hashline prefix regex")
    });
    re.find(line).map_or(line, |m| &line[m.end()..])
}

/// Input parameters for the hashline edit tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HashlineEditInput {
    path: String,
    edits: Vec<HashlineOp>,
}

/// A single hashline edit operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HashlineOp {
    /// Operation type: "replace", "prepend", or "append"
    op: String,
    /// Start anchor in "LINE#HASH" format (optional for BOF prepend / EOF append)
    pos: Option<String>,
    /// End anchor for range replace (inclusive)
    end: Option<String>,
    /// Replacement / insertion lines
    lines: Option<serde_json::Value>,
}

impl HashlineOp {
    /// Extract lines from the `lines` field, handling string, array, and null variants.
    fn get_lines(&self) -> Vec<String> {
        match &self.lines {
            None | Some(serde_json::Value::Null) => vec![],
            Some(serde_json::Value::String(s)) => {
                normalize_to_lf(s).split('\n').map(String::from).collect()
            }
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => normalize_to_lf(s),
                    other => normalize_to_lf(&other.to_string()),
                })
                .collect(),
            Some(other) => vec![normalize_to_lf(&other.to_string())],
        }
    }
}

/// A resolved hashline edit operation ready for application.
struct ResolvedEdit<'a> {
    op: &'a str,
    /// 0-indexed start line (or 0 for BOF, `file_lines.len()` for EOF)
    start: usize,
    /// 0-indexed end line (inclusive, same as start for single-line ops)
    end: usize,
    lines: Vec<String>,
}

pub struct HashlineEditTool {
    cwd: PathBuf,
}

impl HashlineEditTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
        }
    }
}

/// Validate a hashline tag reference against actual file lines.
/// Returns `Ok(0-indexed line)` or `Err(message)` with context.
fn validate_line_ref(
    ref_str: &str,
    file_lines: &[&str],
    had_bom: bool,
) -> std::result::Result<usize, String> {
    let (line_num, expected_hash) = parse_hashline_tag(ref_str)?;
    let line_idx = line_num - 1;
    if line_idx >= file_lines.len() {
        return Err(format!(
            "Line {line_num} out of range (file has {} lines)",
            file_lines.len()
        ));
    }
    let actual_hash = compute_line_hash_with_bom(line_idx, file_lines[line_idx], had_bom);
    if actual_hash != expected_hash {
        let tag = format_hashline_tag_with_bom(line_idx, file_lines[line_idx], had_bom);
        return Err(format!(
            "Hash mismatch at line {line_num}: expected {}#{}{}, actual is {tag}",
            line_num, expected_hash[0] as char, expected_hash[1] as char,
        ));
    }
    Ok(line_idx)
}

/// Build a context snippet around a mismatched line for error reporting.
fn mismatch_context(file_lines: &[&str], line_idx: usize, context: usize, had_bom: bool) -> String {
    let start = line_idx.saturating_sub(context);
    let end = (line_idx + context + 1).min(file_lines.len());
    let mut out = String::new();
    for (i, &file_line) in file_lines.iter().enumerate().take(end).skip(start) {
        let tag = format_hashline_tag_with_bom(i, file_line, had_bom);
        if i == line_idx {
            let _ = writeln!(out, ">>> {tag}:{file_line}");
        } else {
            let _ = writeln!(out, "    {tag}:{file_line}");
        }
    }
    out
}

/// Collect all hash mismatches from a set of edits, returning a combined error message.
fn collect_mismatches(
    edits: &[HashlineOp],
    file_lines: &[&str],
    had_bom: bool,
) -> std::result::Result<(), String> {
    let mut errors = Vec::new();
    for edit in edits {
        if let Some(ref pos) = edit.pos {
            if let Err(e) = validate_line_ref(pos, file_lines, had_bom) {
                // Find the line index for context
                if let Ok((line_num, _)) = parse_hashline_tag(pos) {
                    let idx = (line_num - 1).min(file_lines.len().saturating_sub(1));
                    errors.push(format!(
                        "{e}\n{}",
                        mismatch_context(file_lines, idx, 2, had_bom)
                    ));
                } else {
                    errors.push(e);
                }
            }
        }
        if let Some(ref end) = edit.end {
            if let Err(e) = validate_line_ref(end, file_lines, had_bom) {
                if let Ok((line_num, _)) = parse_hashline_tag(end) {
                    let idx = (line_num - 1).min(file_lines.len().saturating_sub(1));
                    errors.push(format!(
                        "{e}\n{}",
                        mismatch_context(file_lines, idx, 2, had_bom)
                    ));
                } else {
                    errors.push(e);
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Normalized representation of an edit for deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NormalizedEdit {
    op: String,
    pos_line: Option<usize>,
    end_line: Option<usize>,
    lines: Vec<String>,
}

/// Sort precedence for overlapping edits at the same line.
fn op_precedence(op: &str) -> u8 {
    match op {
        "replace" => 0,
        "append" => 1,
        "prepend" => 2,
        _ => 3,
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for HashlineEditTool {
    fn name(&self) -> &str {
        "hashline_edit"
    }
    fn label(&self) -> &str {
        "hashline edit"
    }
    fn description(&self) -> &str {
        "使用先前的 read 配合 hashline=true 获取的 LINE#HASH 标签进行精确文件编辑。 \
         每次编辑指定操作类型（replace/prepend/append）、定位锚点（\"N#AB\"）、可选的 \
         结束锚点用于范围替换，以及替换行内容。编辑会针对当前文件哈希进行验证， \
         并按从下到上的顺序应用以避免索引失效。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit (relative or absolute)"
                },
                "edits": {
                    "type": "array",
                    "description": "Array of edit operations to apply",
                    "items": {
                        "type": "object",
                        "properties": {
                            "op": {
                                "type": "string",
                                "enum": ["replace", "prepend", "append"],
                                "description": "Operation type"
                            },
                            "pos": {
                                "type": "string",
                                "description": "Anchor line reference in LINE#HASH format (e.g. \"5#KJ\")"
                            },
                            "end": {
                                "type": "string",
                                "description": "End anchor for range replace (inclusive)"
                            },
                            "lines": {
                                "description": "Replacement/insertion content as array of strings, single string, or null for deletion",
                                "oneOf": [
                                    { "type": "array", "items": { "type": "string" } },
                                    { "type": "string" },
                                    { "type": "null" }
                                ]
                            }
                        },
                        "required": ["op"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }

    #[allow(clippy::too_many_lines)]
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let input: HashlineEditInput = serde_json::from_value(input)
            .map_err(|e| Error::tool("hashline_edit", format!("Invalid input: {e}")))?;

        if input.edits.is_empty() {
            return Err(Error::tool("hashline_edit", "No edits provided"));
        }

        let resolved = resolve_read_path(&input.path, &self.cwd);
        let absolute_path = resolved;

        // Check file size
        let metadata = asupersync::fs::metadata(&absolute_path)
            .await
            .map_err(|err| {
                let message = match err.kind() {
                    std::io::ErrorKind::NotFound => format!("File not found: {}", input.path),
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied: {}", input.path)
                    }
                    _ => format!("Cannot read file metadata: {err}"),
                };
                Error::tool("hashline_edit", message)
            })?;
        if !metadata.is_file() {
            return Err(Error::tool(
                "hashline_edit",
                format!("Path {} is not a regular file", absolute_path.display()),
            ));
        }
        if metadata.len() > READ_TOOL_MAX_BYTES {
            return Err(Error::tool(
                "hashline_edit",
                format!(
                    "File too large ({} bytes, max {} bytes)",
                    metadata.len(),
                    READ_TOOL_MAX_BYTES
                ),
            ));
        }

        // Read file content
        let file = asupersync::fs::File::open(&absolute_path)
            .await
            .map_err(|e| Error::tool("hashline_edit", format!("Cannot open file: {e}")))?;
        let mut raw = Vec::new();
        let mut limiter = file.take(READ_TOOL_MAX_BYTES.saturating_add(1));
        limiter
            .read_to_end(&mut raw)
            .await
            .map_err(|e| Error::tool("hashline_edit", format!("Cannot read file: {e}")))?;

        if raw.len() as u64 > READ_TOOL_MAX_BYTES {
            return Err(Error::tool(
                "hashline_edit",
                format!("File too large (> {READ_TOOL_MAX_BYTES} bytes)"),
            ));
        }

        let raw_content = String::from_utf8(raw).map_err(|_| {
            Error::tool(
                "hashline_edit",
                "File contains invalid UTF-8 characters and cannot be safely edited as text."
                    .to_string(),
            )
        })?;

        let (content_no_bom, had_bom) = strip_bom(&raw_content);
        let original_ending = detect_line_ending(content_no_bom);
        let normalized = normalize_to_lf(content_no_bom);
        let file_lines: Vec<&str> = normalized.split('\n').collect();

        // Validate all hash references before making any changes
        if let Err(e) = collect_mismatches(&input.edits, &file_lines, had_bom) {
            return Err(Error::tool(
                "hashline_edit",
                format!("Hash validation failed — re-read the file to get current tags.\n\n{e}"),
            ));
        }

        // Deduplicate edits
        let mut seen = std::collections::HashSet::new();
        let mut deduped_edits: Vec<&HashlineOp> = Vec::new();
        for edit in &input.edits {
            let pos_line = edit
                .pos
                .as_ref()
                .and_then(|p| parse_hashline_tag(p).ok())
                .map(|(n, _)| n);
            let end_line = edit
                .end
                .as_ref()
                .and_then(|e| parse_hashline_tag(e).ok())
                .map(|(n, _)| n);
            let key = NormalizedEdit {
                op: edit.op.clone(),
                pos_line,
                end_line,
                lines: edit.get_lines(),
            };
            if seen.insert(key) {
                deduped_edits.push(edit);
            }
        }

        // Resolve line indices and sort bottom-up
        let mut resolved: Vec<ResolvedEdit<'_>> = Vec::new();
        for edit in &deduped_edits {
            let replacement_lines: Vec<String> = edit
                .get_lines()
                .into_iter()
                .map(|l| strip_hashline_prefix(&l).to_string())
                .collect();

            match edit.op.as_str() {
                "replace" => {
                    let start_idx = match &edit.pos {
                        Some(pos) => validate_line_ref(pos, &file_lines, had_bom)
                            .map_err(|e| Error::tool("hashline_edit", e))?,
                        None => {
                            return Err(Error::tool(
                                "hashline_edit",
                                "replace operation requires a pos anchor",
                            ));
                        }
                    };
                    let end_idx = match &edit.end {
                        Some(end) => validate_line_ref(end, &file_lines, had_bom)
                            .map_err(|e| Error::tool("hashline_edit", e))?,
                        None => start_idx,
                    };
                    if end_idx < start_idx {
                        return Err(Error::tool(
                            "hashline_edit",
                            format!(
                                "End anchor (line {}) is before start anchor (line {})",
                                end_idx + 1,
                                start_idx + 1
                            ),
                        ));
                    }
                    resolved.push(ResolvedEdit {
                        op: "replace",
                        start: start_idx,
                        end: end_idx,
                        lines: replacement_lines,
                    });
                }
                "prepend" => {
                    let idx = match &edit.pos {
                        Some(pos) => validate_line_ref(pos, &file_lines, had_bom)
                            .map_err(|e| Error::tool("hashline_edit", e))?,
                        None => 0, // BOF
                    };
                    let end_idx = if file_lines == [""] && edit.pos.is_none() {
                        0 // replace the empty line
                    } else {
                        idx
                    };
                    resolved.push(ResolvedEdit {
                        op: if file_lines == [""] && edit.pos.is_none() {
                            "replace"
                        } else {
                            "prepend"
                        },
                        start: idx,
                        end: end_idx,
                        lines: replacement_lines,
                    });
                }
                "append" => {
                    let idx = match &edit.pos {
                        Some(pos) => validate_line_ref(pos, &file_lines, had_bom)
                            .map_err(|e| Error::tool("hashline_edit", e))?,
                        None => {
                            if file_lines.len() > 1 && file_lines.last() == Some(&"") {
                                file_lines.len() - 2
                            } else {
                                file_lines.len().saturating_sub(1)
                            }
                        }
                    };
                    let end_idx = if file_lines == [""] && edit.pos.is_none() {
                        0 // replace the empty line
                    } else {
                        idx
                    };
                    resolved.push(ResolvedEdit {
                        op: if file_lines == [""] && edit.pos.is_none() {
                            "replace"
                        } else {
                            "append"
                        },
                        start: idx,
                        end: end_idx,
                        lines: replacement_lines,
                    });
                }
                other => {
                    return Err(Error::tool(
                        "hashline_edit",
                        format!("Unknown op: {other:?}. Must be replace, prepend, or append."),
                    ));
                }
            }
        }

        // Sort bottom-up: highest line first, then by precedence (replace < append < prepend)
        resolved.sort_by(|a, b| {
            b.start
                .cmp(&a.start)
                .then_with(|| op_precedence(a.op).cmp(&op_precedence(b.op)))
        });

        // Detect overlapping edit ranges (undefined behavior if applied bottom-up)
        for i in 0..resolved.len() {
            for j in (i + 1)..resolved.len() {
                let a = &resolved[i];
                let b = &resolved[j];
                if a.start <= b.end && b.start <= a.end {
                    return Err(Error::tool(
                        "hashline_edit",
                        format!(
                            "Overlapping edits detected: {} at line {}-{} and {} at line {}-{}. \
                             Please combine overlapping edits into a single operation.",
                            a.op,
                            a.start + 1,
                            a.end + 1,
                            b.op,
                            b.start + 1,
                            b.end + 1
                        ),
                    ));
                }
            }
        }

        // Apply splices bottom-up on a mutable Vec of lines
        let mut lines: Vec<String> = file_lines.iter().map(|s| (*s).to_string()).collect();
        let mut any_change = false;

        for edit in &resolved {
            match edit.op {
                "replace" => {
                    // Check if it's a no-op
                    let existing: Vec<&str> = lines[edit.start..=edit.end]
                        .iter()
                        .map(String::as_str)
                        .collect();
                    if existing.eq(&edit.lines.iter().map(String::as_str).collect::<Vec<&str>>()) {
                        continue; // no-op
                    }
                    // Splice: remove old range, insert new lines
                    lines.splice(edit.start..=edit.end, edit.lines.iter().cloned());
                    any_change = true;
                }
                "prepend" => {
                    // Insert before the target line
                    lines.splice(edit.start..edit.start, edit.lines.iter().cloned());
                    if !edit.lines.is_empty() {
                        any_change = true;
                    }
                }
                "append" => {
                    // Insert after the target line
                    let insert_at = edit.start + 1;
                    lines.splice(insert_at..insert_at, edit.lines.iter().cloned());
                    if !edit.lines.is_empty() {
                        any_change = true;
                    }
                }
                _ => {} // unreachable due to earlier validation
            }
        }

        if !any_change {
            return Err(Error::tool(
                "hashline_edit",
                format!(
                    "No changes made to {}. All edits were no-ops (replacement identical to existing content).",
                    input.path
                ),
            ));
        }

        // Reconstruct content
        let new_normalized = lines.join("\n");
        let new_content = restore_line_endings(&new_normalized, original_ending);
        let mut final_content = new_content;
        if had_bom {
            final_content = format!("\u{FEFF}{final_content}");
        }

        // Write directly (not atomic rename). Same reasoning as EditTool D10:
        // async-read handle contention on Windows makes tempfile::persist
        // unreliable after an asupersync read.
        let absolute_path_clone = absolute_path.clone();
        let final_content_bytes = final_content.into_bytes();
        asupersync::runtime::spawn_blocking_io(move || {
            std::fs::write(&absolute_path_clone, &final_content_bytes)?;
            Ok(())
        })
        .await
        .map_err(|e| Error::tool("hashline_edit", format!("Failed to write file: {e}")))?;

        // Generate diff
        let (diff, first_changed_line) = generate_diff_string(&normalized, &new_normalized);
        let mut details = serde_json::Map::new();
        details.insert("diff".to_string(), serde_json::Value::String(diff));
        if let Some(line) = first_changed_line {
            details.insert(
                "firstChangedLine".to_string(),
                serde_json::Value::Number(serde_json::Number::from(line)),
            );
        }

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(format!(
                "Successfully applied hashline edits to {}.",
                input.path
            )))],
            details: Some(serde_json::Value::Object(details)),
            is_error: false,
        })
    }
}








