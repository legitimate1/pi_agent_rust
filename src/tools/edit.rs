use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use asupersync::io::AsyncReadExt;
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};
// ============================================================================
// Write Tool
// ============================================================================

/// Input parameters for the edit tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditInput {
    path: String,
    old_text: String,
    new_text: String,
    /// If true, run syntax/format check after editing.
    #[serde(default)]
    verify: bool,
}

pub struct EditTool {
    cwd: PathBuf,
}

impl EditTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
        }
    }
}

pub fn strip_bom(s: &str) -> (&str, bool) {
    s.strip_prefix('\u{FEFF}')
        .map_or_else(|| (s, false), |stripped| (stripped, true))
}

pub fn detect_line_ending(content: &str) -> &'static str {
    let bytes = content.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\r' => {
                return if bytes.get(idx + 1) == Some(&b'\n') {
                    "\r\n"
                } else {
                    "\r"
                };
            }
            b'\n' => return "\n",
            _ => idx += 1,
        }
    }
    "\n"
}

pub fn normalize_to_lf(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            out.push('\n');
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn normalize_line_endings_chunk<'a>(
    chunk: &'a [u8],
    pending_cr: &mut bool,
) -> std::borrow::Cow<'a, [u8]> {
    if !*pending_cr && memchr::memchr(b'\r', chunk).is_none() {
        return std::borrow::Cow::Borrowed(chunk);
    }

    let mut normalized = Vec::with_capacity(chunk.len().saturating_add(usize::from(*pending_cr)));
    let mut idx = 0;

    if *pending_cr {
        normalized.push(b'\n');
        if chunk.first() == Some(&b'\n') {
            idx = 1;
        }
        *pending_cr = false;
    }

    while idx < chunk.len() {
        match chunk[idx] {
            b'\r' => {
                if chunk.get(idx + 1) == Some(&b'\n') {
                    normalized.push(b'\n');
                    idx += 2;
                } else if idx + 1 < chunk.len() {
                    normalized.push(b'\n');
                    idx += 1;
                } else {
                    *pending_cr = true;
                    idx += 1;
                }
            }
            byte => {
                normalized.push(byte);
                idx += 1;
            }
        }
    }

    std::borrow::Cow::Owned(normalized)
}

pub fn restore_line_endings(text: &str, ending: &str) -> String {
    match ending {
        "\r\n" => text.replace('\n', "\r\n"),
        "\r" => text.replace('\n', "\r"),
        _ => text.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct FuzzyMatchResult {
    pub(crate) found: bool,
    pub(crate) index: usize,
    pub(crate) match_length: usize,
    pub(crate) exact_match: bool,
}

/// Map a range in normalized content back to byte offsets in the original text.
///
/// Returns `(original_start_byte_idx, original_match_byte_len)`.
pub fn map_normalized_range_to_original(
    content: &str,
    norm_match_start: usize,
    norm_match_len: usize,
) -> (usize, usize) {
    let mut norm_idx = 0;
    let mut orig_idx = 0;
    let mut match_start = None;
    let mut match_end = None;
    let norm_match_end = norm_match_start + norm_match_len;
    let mut last_trimmed_end = 0;
    let mut last_has_newline = false;

    for line in content.split_inclusive('\n') {
        let line_content = line.strip_suffix('\n').unwrap_or(line);
        let has_newline = line.ends_with('\n');
        let trimmed_len = line_content
            .trim_end_matches(|c: char| c.is_whitespace() || is_special_unicode_space(c))
            .len();
        let trimmed_end = orig_idx + trimmed_len;
        last_trimmed_end = trimmed_end;
        last_has_newline = has_newline;

        for (char_offset, c) in line_content.char_indices() {
            // match_end can be detected at any position including trailing
            // whitespace — it correctly points to right after the last content char.
            if norm_idx == norm_match_end && match_end.is_none() {
                match_end = Some(orig_idx + char_offset);
            }

            if char_offset >= trimmed_len {
                continue;
            }

            // match_start must only be detected at non-trailing-whitespace positions.
            // During trailing whitespace, norm_idx is "frozen" at the value after the
            // last real char, which corresponds to the newline in normalized content —
            // not the trailing space. The post-loop newline check handles that case.
            if norm_idx == norm_match_start && match_start.is_none() {
                match_start = Some(orig_idx + char_offset);
            }
            if match_start.is_some() && match_end.is_some() {
                break;
            }

            let normalized_char = if is_special_unicode_space(c) {
                ' '
            } else if matches!(c, '\u{2018}' | '\u{2019}') {
                '\''
            } else if matches!(c, '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}') {
                '"'
            } else if matches!(
                c,
                '\u{2010}'
                    | '\u{2011}'
                    | '\u{2012}'
                    | '\u{2013}'
                    | '\u{2014}'
                    | '\u{2015}'
                    | '\u{2212}'
            ) {
                '-'
            } else {
                c
            };

            norm_idx += normalized_char.len_utf8();
        }

        orig_idx += line_content.len();

        if has_newline {
            if norm_idx == norm_match_start && match_start.is_none() {
                match_start = Some(orig_idx);
            }
            if norm_idx == norm_match_end && match_end.is_none() {
                match_end = Some(trimmed_end);
            }

            norm_idx += 1;
            orig_idx += 1;
        }

        if match_start.is_some() && match_end.is_some() {
            break;
        }
    }

    if norm_idx == norm_match_end && match_end.is_none() {
        match_end = Some(if last_has_newline {
            orig_idx
        } else {
            last_trimmed_end
        });
    }

    let start = match_start.unwrap_or(0);
    let end = match_end.unwrap_or(content.len());
    (start, end.saturating_sub(start))
}

pub fn build_normalized_content(content: &str) -> String {
    let mut normalized = String::with_capacity(content.len());
    let mut lines = content.split('\n').peekable();

    while let Some(line) = lines.next() {
        let trimmed_len = line
            .trim_end_matches(|c: char| c.is_whitespace() || is_special_unicode_space(c))
            .len();
        for (char_offset, c) in line.char_indices() {
            if char_offset >= trimmed_len {
                continue;
            }
            let normalized_char = if is_special_unicode_space(c) {
                ' '
            } else if matches!(c, '\u{2018}' | '\u{2019}') {
                '\''
            } else if matches!(c, '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}') {
                '"'
            } else if matches!(
                c,
                '\u{2010}'
                    | '\u{2011}'
                    | '\u{2012}'
                    | '\u{2013}'
                    | '\u{2014}'
                    | '\u{2015}'
                    | '\u{2212}'
            ) {
                '-'
            } else {
                c
            };
            normalized.push(normalized_char);
        }
        if lines.peek().is_some() {
            normalized.push('\n');
        }
    }
    normalized
}

#[cfg(test)]
pub fn fuzzy_find_text(content: &str, old_text: &str) -> FuzzyMatchResult {
    fuzzy_find_text_with_normalized(content, old_text, None, None)
}

/// Like [`fuzzy_find_text`], but accepts optional pre-computed normalized
/// versions.
fn fuzzy_find_text_with_normalized(
    content: &str,
    old_text: &str,
    precomputed_content: Option<&str>,
    precomputed_old: Option<&str>,
) -> FuzzyMatchResult {
    use std::borrow::Cow;

    // First, try exact match (fastest path)
    if let Some(index) = content.find(old_text) {
        return FuzzyMatchResult {
            found: true,
            index,
            match_length: old_text.len(),
            exact_match: true,
        };
    }

    // Build normalized versions (reuse pre-computed if available)
    let normalized_content = precomputed_content.map_or_else(
        || Cow::Owned(build_normalized_content(content)),
        Cow::Borrowed,
    );
    let normalized_old_text = precomputed_old.map_or_else(
        || Cow::Owned(build_normalized_content(old_text)),
        Cow::Borrowed,
    );

    // Try to find the normalized old_text in normalized content
    if let Some(normalized_index) = normalized_content.find(normalized_old_text.as_ref()) {
        let (original_start, original_match_len) =
            map_normalized_range_to_original(content, normalized_index, normalized_old_text.len());

        return FuzzyMatchResult {
            found: true,
            index: original_start,
            match_length: original_match_len,
            exact_match: false,
        };
    }

    FuzzyMatchResult {
        found: false,
        index: 0,
        match_length: 0,
        exact_match: false,
    }
}

pub fn count_overlapping_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }

    haystack
        .char_indices()
        .filter(|(idx, _)| haystack[*idx..].starts_with(needle))
        .count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffTag {
    Equal,
    Added,
    Removed,
}

#[derive(Debug, Clone)]
struct DiffPart {
    tag: DiffTag,
    value: String,
}

fn diff_parts(old_content: &str, new_content: &str) -> Vec<DiffPart> {
    use similar::ChangeTag;

    let diff = similar::TextDiff::from_lines(old_content, new_content);

    let mut parts: Vec<DiffPart> = Vec::new();
    let mut current_tag: Option<DiffTag> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => DiffTag::Equal,
            ChangeTag::Insert => DiffTag::Added,
            ChangeTag::Delete => DiffTag::Removed,
        };

        let mut line = change.value();
        if let Some(stripped) = line.strip_suffix('\n') {
            line = stripped;
        }

        if current_tag == Some(tag) {
            current_lines.push(line);
        } else {
            if let Some(prev_tag) = current_tag {
                parts.push(DiffPart {
                    tag: prev_tag,
                    value: current_lines.join("\n"),
                });
            }
            current_tag = Some(tag);
            current_lines = vec![line];
        }
    }

    if let Some(tag) = current_tag {
        parts.push(DiffPart {
            tag,
            value: current_lines.join("\n"),
        });
    }

    parts
}

fn diff_line_num_width(old_content: &str, new_content: &str) -> usize {
    // Count newlines with memchr (avoids iterator-item overhead of split().count())
    let old_line_count = memchr::memchr_iter(b'\n', old_content.as_bytes()).count() + 1;
    let new_line_count = memchr::memchr_iter(b'\n', new_content.as_bytes()).count() + 1;
    let max_line_num = old_line_count.max(new_line_count).max(1);
    max_line_num.ilog10() as usize + 1
}

fn split_diff_lines(value: &str) -> Vec<&str> {
    // value is joined by `\n` from a Vec<&str> in diff_parts, so there is no
    // spurious trailing newline. We can split exactly.
    // We only need to handle the case where value is empty but it originated from
    // 0 elements, but `diff_parts` only emits when there is at least 1 line.
    // If value is "", `split('\n')` returns `[""]`, which correctly represents 1 empty line.
    value.split('\n').collect()
}

#[inline]
const fn is_change_tag(tag: DiffTag) -> bool {
    matches!(tag, DiffTag::Added | DiffTag::Removed)
}

#[derive(Debug)]
struct DiffRenderState {
    output: String,
    old_line_num: usize,
    new_line_num: usize,
    last_was_change: bool,
    first_changed_line: Option<usize>,
    line_num_width: usize,
    context_lines: usize,
}

impl DiffRenderState {
    const fn new(line_num_width: usize, context_lines: usize) -> Self {
        Self {
            output: String::new(),
            old_line_num: 1,
            new_line_num: 1,
            last_was_change: false,
            first_changed_line: None,
            line_num_width,
            context_lines,
        }
    }

    #[inline]
    fn ensure_line_break(&mut self) {
        if !self.output.is_empty() {
            self.output.push('\n');
        }
    }

    const fn mark_first_change(&mut self) {
        if self.first_changed_line.is_none() {
            self.first_changed_line = Some(self.new_line_num);
        }
    }

    fn push_added_line(&mut self, line: &str) {
        self.ensure_line_break();
        let _ = write!(
            self.output,
            "+{line_num:>width$} {line}",
            line_num = self.new_line_num,
            width = self.line_num_width
        );
        self.new_line_num = self.new_line_num.saturating_add(1);
    }

    fn push_removed_line(&mut self, line: &str) {
        self.ensure_line_break();
        let _ = write!(
            self.output,
            "-{line_num:>width$} {line}",
            line_num = self.old_line_num,
            width = self.line_num_width
        );
        self.old_line_num = self.old_line_num.saturating_add(1);
    }

    fn push_context_line(&mut self, line: &str) {
        self.ensure_line_break();
        let _ = write!(
            self.output,
            " {line_num:>width$} {line}",
            line_num = self.old_line_num,
            width = self.line_num_width
        );
        self.old_line_num = self.old_line_num.saturating_add(1);
        self.new_line_num = self.new_line_num.saturating_add(1);
    }

    fn push_skip_marker(&mut self, skip: usize) {
        if skip == 0 {
            return;
        }
        self.ensure_line_break();
        let _ = write!(
            self.output,
            " {:>width$} ...",
            " ",
            width = self.line_num_width
        );
        self.old_line_num = self.old_line_num.saturating_add(skip);
        self.new_line_num = self.new_line_num.saturating_add(skip);
    }
}

fn render_changed_part(tag: DiffTag, raw: &[&str], state: &mut DiffRenderState) {
    state.mark_first_change();
    for line in raw {
        match tag {
            DiffTag::Added => state.push_added_line(line),
            DiffTag::Removed => state.push_removed_line(line),
            DiffTag::Equal => {}
        }
    }
    state.last_was_change = true;
}

fn render_equal_part(raw: &[&str], next_part_is_change: bool, state: &mut DiffRenderState) {
    if !(state.last_was_change || next_part_is_change) {
        let raw_len = raw.len();
        state.old_line_num = state.old_line_num.saturating_add(raw_len);
        state.new_line_num = state.new_line_num.saturating_add(raw_len);
        state.last_was_change = false;
        return;
    }

    if state.last_was_change
        && next_part_is_change
        && raw.len() > state.context_lines.saturating_mul(2)
    {
        for line in raw.iter().take(state.context_lines) {
            state.push_context_line(line);
        }

        let skip = raw.len().saturating_sub(state.context_lines * 2);
        state.push_skip_marker(skip);

        for line in raw
            .iter()
            .skip(raw.len().saturating_sub(state.context_lines))
        {
            state.push_context_line(line);
        }
    } else {
        // Compute slice bounds directly instead of cloning Vecs
        let start = if state.last_was_change {
            0
        } else {
            raw.len().saturating_sub(state.context_lines)
        };
        let lines_after_start = raw.len().saturating_sub(start);
        let (end, skip_end) = if !next_part_is_change && lines_after_start > state.context_lines {
            (
                start + state.context_lines,
                lines_after_start - state.context_lines,
            )
        } else {
            (raw.len(), 0)
        };

        state.push_skip_marker(start);
        for line in &raw[start..end] {
            state.push_context_line(line);
        }
        state.push_skip_marker(skip_end);
    }

    state.last_was_change = false;
}

pub fn generate_diff_string(old_content: &str, new_content: &str) -> (String, Option<usize>) {
    let parts = diff_parts(old_content, new_content);
    let mut state = DiffRenderState::new(diff_line_num_width(old_content, new_content), 4);

    for (i, part) in parts.iter().enumerate() {
        let raw = split_diff_lines(&part.value);
        let next_part_is_change = parts.get(i + 1).is_some_and(|next| is_change_tag(next.tag));

        match part.tag {
            DiffTag::Added | DiffTag::Removed => render_changed_part(part.tag, &raw, &mut state),
            DiffTag::Equal => render_equal_part(&raw, next_part_is_change, &mut state),
        }
    }

    (state.output, state.first_changed_line)
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }
    fn label(&self) -> &str {
        "edit"
    }
    fn description(&self) -> &str {
        "通过替换文本编辑现有文件。oldText 须唯一匹配文件中一处区域；替换无变化时报错。返回替换差异。文件限 100MB。可选 verify 参数在编辑后运行语法检查。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit (relative or absolute)"
                },
                "oldText": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Text to find and replace. Must match exactly one region; matching normalizes line endings, Unicode whitespace/quotes/dashes, and ignores trailing whitespace."
                },
                "newText": {
                    "type": "string",
                    "description": "New text to replace the old text with"
                },
                "verify": {
                    "type": "boolean",
                    "description": "若为 true，编辑后自动运行语法检查（.rs → rustfmt --check, .json/.toml → 进程内解析, .ts → prettier --check）。依赖工具需在 PATH 中可用。默认 false。",
                    "default": false
                }
            },
            "required": ["path", "oldText", "newText"]
        })
    }

    #[allow(clippy::too_many_lines)]
    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
        abort: Option<AbortSignal>,
    ) -> Result<ToolOutput> {
        let input: EditInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        if input.new_text.len() > WRITE_TOOL_MAX_BYTES {
            return Err(Error::validation(format!(
                "New text size exceeds maximum allowed ({} > {} bytes)",
                input.new_text.len(),
                WRITE_TOOL_MAX_BYTES
            )));
        }

        let absolute_path = resolve_read_path(&input.path, &self.cwd);

        let meta = asupersync::fs::metadata(&absolute_path)
            .await
            .map_err(|err| {
                let message = match err.kind() {
                    std::io::ErrorKind::NotFound => format!("File not found: {}", input.path),
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied: {}", input.path)
                    }
                    _ => format!("Failed to access file {}: {err}", input.path),
                };
                Error::tool("edit", message)
            })?;

        if !meta.is_file() {
            return Err(Error::tool(
                "edit",
                format!("Path {} is not a regular file", absolute_path.display()),
            ));
        }
        if meta.len() > READ_TOOL_MAX_BYTES {
            return Err(Error::tool(
                "edit",
                format!(
                    "File is too large ({} bytes). Max allowed for editing is {} bytes.",
                    meta.len(),
                    READ_TOOL_MAX_BYTES
                ),
            ));
        }

        if let Err(err) = asupersync::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&absolute_path)
            .await
        {
            let message = match err.kind() {
                std::io::ErrorKind::NotFound => format!("File not found: {}", input.path),
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied: {}", input.path)
                }
                _ => format!("Failed to open file for editing: {err}"),
            };
            return Err(Error::tool("edit", message));
        }

        // Read bytes strictly up to the limit to prevent OOM if metadata failed or file grows.
        let file = asupersync::fs::File::open(&absolute_path)
            .await
            .map_err(|e| Error::tool("edit", format!("Failed to open file: {e}")))?;
        let mut raw = Vec::new();
        let mut limiter = file.take(READ_TOOL_MAX_BYTES.saturating_add(1));
        limiter
            .read_to_end(&mut raw)
            .await
            .map_err(|e| Error::tool("edit", format!("Failed to read file: {e}")))?;

        if raw.len() > usize::try_from(READ_TOOL_MAX_BYTES).unwrap_or(usize::MAX) {
            return Err(Error::tool(
                "edit",
                format!("File is too large (> {READ_TOOL_MAX_BYTES} bytes)."),
            ));
        }

        let raw_content = String::from_utf8(raw).map_err(|_| {
            Error::tool(
                "edit",
                "File contains invalid UTF-8 characters and cannot be safely edited as text."
                    .to_string(),
            )
        })?;

        // Strip BOM before matching (LLM won't include invisible BOM in oldText).
        let (content_no_bom, had_bom) = strip_bom(&raw_content);

        let original_ending = detect_line_ending(content_no_bom);
        let normalized_content = normalize_to_lf(content_no_bom);
        let content_for_matching =
            if content_no_bom.contains('\r') && !content_no_bom.contains('\n') {
                std::borrow::Cow::Owned(content_no_bom.replace('\r', "\n"))
            } else {
                std::borrow::Cow::Borrowed(content_no_bom)
            };
        let normalized_old_text = normalize_to_lf(&input.old_text);

        if normalized_old_text.is_empty() {
            return Err(Error::tool(
                "edit",
                "The old text cannot be empty. To prepend text, include the first line's content in oldText and newText.".to_string(),
            ));
        }
        if build_normalized_content(&normalized_old_text).is_empty() {
            return Err(Error::tool(
                "edit",
                "The old text must include at least one non-whitespace character.".to_string(),
            ));
        }

        // Try variants of old_text to handle Unicode normalization differences (NFC vs NFD)
        // and potential input normalization (clipboard, LLM output).
        //
        // Note: normalized_content is already LF-normalized but preserves Unicode form
        // (from String::from_utf8).

        let mut variants = Vec::with_capacity(3);
        variants.push(normalized_old_text.clone());

        let nfc = normalized_old_text.nfc().collect::<String>();
        if nfc != normalized_old_text {
            variants.push(nfc);
        }

        let nfd = normalized_old_text.nfd().collect::<String>();
        if nfd != normalized_old_text {
            variants.push(nfd);
        }

        // Pre-compute normalized versions once and reuse for both matching and
        // occurrence counting (avoids 2x redundant O(n) normalization).
        let precomputed_content = build_normalized_content(content_for_matching.as_ref());

        let mut best_match: Option<(FuzzyMatchResult, String, String)> = None;

        for variant in variants {
            let precomputed_variant = build_normalized_content(&variant);
            let match_result = fuzzy_find_text_with_normalized(
                content_for_matching.as_ref(),
                &variant,
                Some(precomputed_content.as_str()),
                Some(precomputed_variant.as_str()),
            );

            if match_result.found {
                best_match = Some((match_result, precomputed_variant, variant));
                break;
            }
        }

        let Some((match_result, normalized_old_text, matched_variant)) = best_match else {
            return Err(Error::tool(
                "edit",
                format!(
                    "Could not find the exact text in {}. The old text must match exactly including all whitespace and newlines.",
                    input.path
                ),
            ));
        };

        // Count occurrences in the same matching mode to avoid false ambiguity
        // when normalized matching collapses distinct trailing whitespace.
        let occurrences = if match_result.exact_match {
            count_overlapping_occurrences(content_for_matching.as_ref(), &matched_variant)
        } else {
            count_overlapping_occurrences(&precomputed_content, &normalized_old_text)
        };

        if occurrences > 1 {
            return Err(Error::tool(
                "edit",
                format!(
                    "Found {occurrences} occurrences of the text in {}. The text must be unique. Please provide more context to make it unique.",
                    input.path
                ),
            ));
        }

        // Perform replacement in the original coordinate space to preserve
        // line endings and unmatched content exactly.
        let idx = match_result.index;
        let match_len = match_result.match_length;

        // Adapt new_text to match the file's line endings.
        // normalize_to_lf ensures we start from a known state (LF), then
        // restore_line_endings converts LFs to the target ending (e.g. CRLF).
        let adapted_new_text =
            restore_line_endings(&normalize_to_lf(&input.new_text), original_ending);

        let new_len = content_no_bom.len() - match_len + adapted_new_text.len();
        let mut new_content = String::with_capacity(new_len);
        new_content.push_str(&content_no_bom[..idx]);
        new_content.push_str(&adapted_new_text);
        new_content.push_str(&content_no_bom[idx + match_len..]);

        if content_no_bom.eq(&new_content) {
            return Err(Error::tool(
                "edit",
                format!(
                    "No changes made to {}. The replacement produced identical content. This might indicate an issue with special characters or the text not existing as expected.",
                    input.path
                ),
            ));
        }

        let new_content_for_diff = normalize_to_lf(&new_content);

        // Re-add BOM if present.
        let mut final_content = new_content;
        if had_bom {
            final_content = format!("\u{FEFF}{final_content}");
        }

        // Write directly (not atomic rename).  The file is confirmed
        // writable above; async-read handle contention on Windows makes
        // tempfile::persist unreliable after an asupersync read.
        let absolute_path_clone = absolute_path.clone();
        let final_content_bytes = final_content.into_bytes();
        asupersync::runtime::spawn_blocking_io(move || {
            std::fs::write(&absolute_path_clone, &final_content_bytes)?;
            Ok(())
        })
        .await
        .map_err(|e| Error::tool("edit", format!("Failed to write file: {e}")))?;

        let (diff, first_changed_line) =
            generate_diff_string(&normalized_content, &new_content_for_diff);
        let mut details = serde_json::Map::new();
        details.insert("diff".to_string(), serde_json::Value::String(diff));
        if let Some(line) = first_changed_line {
            details.insert(
                "firstChangedLine".to_string(),
                serde_json::Value::Number(serde_json::Number::from(line)),
            );
        }

        // Optional: run file verification after successful edit
        if input.verify {
            let verify_path = absolute_path.clone();
            match crate::tools::verify::verify_file(verify_path, abort).await {
                Ok(result) => {
                    let verify_json = crate::tools::verify::verify_result_to_json(&result);
                    details.insert("verify".to_string(), verify_json);
                }
                Err(e) => {
                    details.insert(
                        "verify".to_string(),
                        serde_json::json!({
                            "passed": false,
                            "checker": "verify",
                            "message": format!("Verification error: {e}"),
                        }),
                    );
                }
            }
        }

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(format!(
                "Successfully replaced text in {}.",
                input.path
            )))],
            details: Some(serde_json::Value::Object(details)),
            is_error: false,
        })
    }
}

// ============================================================================
