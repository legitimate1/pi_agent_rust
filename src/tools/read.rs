use super::edit::normalize_line_endings_chunk;
use super::hashline::format_hashline_tag;
use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, ImageContent, TextContent};
use asupersync::io::{AsyncRead, ReadBuf, SeekFrom};
use async_trait::async_trait;
use serde::Deserialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
// ============================================================================
// Read Tool
// ============================================================================

/// Input parameters for the read tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadInput {
    path: Option<String>,
    #[serde(default)]
    paths: Option<Vec<String>>,
    offset: Option<i64>,
    limit: Option<i64>,
    #[serde(default)]
    hashline: bool,
    head: Option<usize>,
    tail: Option<usize>,
    #[serde(default)]
    info: bool,
    diff: Option<String>,
    context: Option<usize>,
    #[serde(default)]
    summary_only: bool,
}

pub struct ReadTool {
    cwd: PathBuf,
    /// Whether to auto-resize images to fit token limits.
    auto_resize: bool,
    block_images: bool,
    artifact_root: Option<PathBuf>,
}

impl ReadTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            auto_resize: true,
            block_images: false,
            artifact_root: None,
        }
    }

    pub fn with_settings(cwd: &Path, auto_resize: bool, block_images: bool) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            auto_resize,
            block_images,
            artifact_root: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_artifact_root(cwd: &Path, artifact_root: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            auto_resize: true,
            block_images: false,
            artifact_root: Some(artifact_root.to_path_buf()),
        }
    }
}

async fn read_some<R>(reader: &mut R, dst: &mut [u8]) -> std::io::Result<usize>
where
    R: AsyncRead + Unpin,
{
    if dst.is_empty() {
        return Ok(0);
    }

    futures::future::poll_fn(|cx| {
        let mut read_buf = ReadBuf::new(dst);
        match std::pin::Pin::new(&mut *reader).poll_read(cx, &mut read_buf) {
            std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(Ok(read_buf.filled().len())),
            std::task::Poll::Ready(Err(err)) => std::task::Poll::Ready(Err(err)),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    })
    .await
}

impl ReadTool {
    async fn read_single_file(
        &self,
        path: &str,
        input: &ReadInput,
        tool_call_id: &str,
    ) -> Result<ToolOutput> {
        let resolved = resolve_read_path(path, &self.cwd);

        let meta = asupersync::fs::metadata(&resolved).await.ok();
        if let Some(meta) = &meta {
            if !meta.is_file() {
                return Err(Error::tool(
                    "read",
                    format!("Path {} is not a regular file", resolved.display()),
                ));
            }
        } else {
            return Err(Error::tool(
                "read",
                format!("File not found: {}", resolved.display()),
            ));
        }

        // Info mode: file metadata only, no content read.
        if input.info {
            let meta = meta.as_ref().unwrap();
            let modified = meta.modified().ok();
            let mtime_str = modified
                .map(|t| {
                    let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    let secs = duration.as_secs();
                    let days = secs / 86400;
                    if days < 1 {
                        format!("{}h ago", secs / 3600)
                    } else {
                        format!("{days}d ago")
                    }
                })
                .unwrap_or_default();

            let mut buf = [0u8; 8192];
            let mut file = asupersync::fs::File::open(&resolved)
                .await
                .map_err(|e| Error::tool("read", e.to_string()))?;
            let n = read_some(&mut file, &mut buf)
                .await
                .map_err(|e| Error::tool("read", format!("Failed to read file: {e}")))?;
            let (encoding, _) = detect_encoding(&buf[..n], None);

            let size_str = format_file_size(meta.len());
            return Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(format!(
                    "📄 {} | {} | {} | encoding: {}",
                    resolved.file_name().unwrap_or_default().to_string_lossy(),
                    size_str,
                    mtime_str,
                    encoding,
                )))],
                details: None,
                is_error: false,
            });
        }

        // Build cache key from path + view parameters so that reads with
        // different offset/limit/head/tail/hashline return correct content.
        let cache_key = tool_cache_key(
            "read",
            &self.cwd,
            &serde_json::json!({
                "path": path,
                "offset": input.offset,
                "limit": input.limit,
                "head": input.head,
                "tail": input.tail,
                "hashline": input.hashline,
            }),
        );
        let cache_mode = ToolCacheFingerprintMode::FileContent;
        let cache_deps = cache_dependency_for_path(&resolved, cache_mode);
        if let Some(output) = cached_tool_output(&cache_key, cache_deps.as_deref()) {
            return Ok(output);
        }

        let mut file = asupersync::fs::File::open(&resolved)
            .await
            .map_err(|e| Error::tool("read", e.to_string()))?;

        let mut buffer = [0u8; 8192];
        let mut initial_read = 0;
        loop {
            let n = read_some(&mut file, &mut buffer[initial_read..])
                .await
                .map_err(|e| Error::tool("read", format!("Failed to read file: {e}")))?;
            if n == 0 {
                break;
            }
            initial_read += n;
            if initial_read == buffer.len() {
                break;
            }
        }
        let initial_bytes = &buffer[..initial_read];

        if let Some(mime_type) = detect_supported_image_mime_type_from_bytes(initial_bytes) {
            if self.block_images {
                return Err(Error::tool(
                    "read",
                    "Images are blocked by configuration".to_string(),
                ));
            }

            let max_image_input_bytes = usize::try_from(READ_TOOL_MAX_BYTES).unwrap_or(usize::MAX);
            if let Some(meta) = &meta {
                if meta.len() > READ_TOOL_MAX_BYTES {
                    return Err(Error::tool(
                        "read",
                        format!(
                            "Image is too large ({} bytes). Max allowed is {} bytes.",
                            meta.len(),
                            READ_TOOL_MAX_BYTES
                        ),
                    ));
                }
            }
            let mut all_bytes = Vec::with_capacity(initial_read);
            all_bytes.extend_from_slice(initial_bytes);

            let remaining_limit = max_image_input_bytes.saturating_sub(initial_read);
            let mut limiter = file.take((remaining_limit as u64).saturating_add(1));
            limiter
                .read_to_end(&mut all_bytes)
                .await
                .map_err(|e| Error::tool("read", format!("Failed to read image: {e}")))?;

            if all_bytes.len() > max_image_input_bytes {
                return Err(Error::tool(
                    "read",
                    format!(
                        "Image is too large ({} bytes). Max allowed is {} bytes.",
                        all_bytes.len(),
                        READ_TOOL_MAX_BYTES
                    ),
                ));
            }

            let resized = if self.auto_resize {
                resize_image_if_needed(&all_bytes, mime_type)?
            } else {
                ResizedImage::original(all_bytes, mime_type)
            };

            if resized.bytes.len() > IMAGE_MAX_BYTES {
                let message = if resized.resized {
                    format!(
                        "Image is too large ({} bytes) after resizing. Max allowed is {} bytes.",
                        resized.bytes.len(),
                        IMAGE_MAX_BYTES
                    )
                } else {
                    format!(
                        "Image is too large ({} bytes). Max allowed is {} bytes.",
                        resized.bytes.len(),
                        IMAGE_MAX_BYTES
                    )
                };
                return Err(Error::tool("read", message));
            }

            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &resized.bytes);

            let mut note = format!("Read image file [{}]", resized.mime_type);
            if resized.resized {
                if let (Some(ow), Some(oh), Some(w), Some(h)) = (
                    resized.original_width,
                    resized.original_height,
                    resized.width,
                    resized.height,
                ) {
                    if w > 0 {
                        let scale = f64::from(ow) / f64::from(w);
                        let _ = write!(
                            note,
                            "\n[Image: original {ow}x{oh}, displayed at {w}x{h}. Multiply coordinates by {scale:.2} to map to original image.]"
                        );
                    } else {
                        let _ =
                            write!(note, "\n[Image: original {ow}x{oh}, displayed at {w}x{h}.]");
                    }
                }
            }

            return Ok(ToolOutput {
                content: vec![
                    ContentBlock::Text(TextContent::new(note)),
                    ContentBlock::Image(ImageContent {
                        data: base64_data,
                        mime_type: resized.mime_type.to_string(),
                    }),
                ],
                details: None,
                is_error: false,
            });
        }

        // Diff mode
        if let Some(diff_target) = &input.diff {
            let diff_path = resolve_read_path(diff_target, &self.cwd);
            if !diff_path.exists() {
                return Err(Error::tool(
                    "read",
                    format!("Diff target file not found: {}", diff_path.display()),
                ));
            }

            let ctx = input.context.unwrap_or(3);
            let content_a = asupersync::fs::read(&resolved)
                .await
                .map_err(|e| Error::tool("read", format!("Failed to read file for diff: {e}")))?;
            let content_b = asupersync::fs::read(&diff_path)
                .await
                .map_err(|e| Error::tool("read", format!("Failed to read diff target: {e}")))?;
            let text_a = String::from_utf8_lossy(&content_a);
            let text_b = String::from_utf8_lossy(&content_b);

            if input.summary_only {
                let diff = similar::TextDiff::from_lines(&text_a, &text_b);
                let stats = diff.unified_diff().context_radius(ctx).to_string();
                let lines = stats.lines().count().saturating_sub(1);
                let changes = diff.ops().len();
                return Ok(ToolOutput {
                    content: vec![ContentBlock::Text(TextContent::new(format!(
                        "Diff: {} ↔ {}\nChanges: {changes} diff hunks, {lines} lines",
                        resolved.file_name().unwrap_or_default().to_string_lossy(),
                        diff_path.file_name().unwrap_or_default().to_string_lossy(),
                    )))],
                    details: None,
                    is_error: false,
                });
            }

            let diff_output = similar::TextDiff::from_lines(&text_a, &text_b)
                .unified_diff()
                .context_radius(ctx)
                .to_string();
            return Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(diff_output))],
                details: None,
                is_error: false,
            });
        }

        // Text reading logic
        let effective_offset: Option<i64> = if input.head.is_some() {
            Some(1)
        } else {
            input.offset
        };
        let effective_limit: Option<i64> = if let Some(n) = input.head {
            #[allow(clippy::cast_possible_wrap)]
            let v = n as i64;
            Some(v)
        } else {
            input.limit
        };

        let (encoding_label, bom_skip) = detect_encoding(initial_bytes, None);
        let is_utf8 = encoding_label == "UTF-8" || encoding_label == "UTF-8 (BOM)";

        if input.tail.is_some() || !is_utf8 {
            let full_bytes = asupersync::fs::read(&resolved)
                .await
                .map_err(|e| Error::tool("read", format!("Failed to read file: {e}")))?;
            let text_content = decode_with_encoding(&full_bytes, &encoding_label, bom_skip)?;
            let all_lines: Vec<&str> = text_content.split('\n').collect();
            let total_lines = all_lines.len();

            let start_line_idx = match effective_offset {
                Some(n) if n > 0 => {
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                    {
                        n.saturating_sub(1) as usize
                    }
                }
                _ => 0,
            };

            let selected_lines: Vec<&str> = if let Some(n) = input.tail {
                let take = n.min(total_lines);
                let start = total_lines.saturating_sub(take);
                all_lines[start..].to_vec()
            } else {
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let limit = effective_limit.map_or(usize::MAX, |l| l as usize);
                let end = start_line_idx.saturating_add(limit).min(total_lines);
                all_lines[start_line_idx..end].to_vec()
            };

            let display_start = if input.tail.is_some() {
                total_lines.saturating_sub(selected_lines.len())
            } else {
                start_line_idx
            };

            let mut output_text = String::new();
            let line_num_width = total_lines.to_string().len().max(5);
            for (i, line) in selected_lines.iter().enumerate() {
                if i > 0 {
                    output_text.push('\n');
                }
                let line_idx = display_start + i;
                let line = line.strip_suffix('\r').unwrap_or(line);
                if input.hashline {
                    let tag = format_hashline_tag(line_idx, line);
                    let _ = write!(output_text, "{tag}:{line}");
                } else {
                    let num = line_idx + 1;
                    let _ = write!(output_text, "{num:>line_num_width$}→{line}");
                }
            }

            if output_text.is_empty() {
                output_text = String::new();
            } else if input.tail.is_none()
                && input.head.is_none()
                && selected_lines.len() < total_lines - start_line_idx
            {
                let remaining = total_lines - start_line_idx - selected_lines.len();
                let next_offset = start_line_idx + selected_lines.len() + 1;
                let _ = write!(
                    output_text,
                    "\n\n[{remaining} more lines in file. Use offset={next_offset} to continue.]"
                );
            }

            return Ok(ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(output_text))],
                details: None,
                is_error: false,
            });
        }

        // ── UTF-8 streaming path ──
        if initial_read > 0 {
            file.seek(SeekFrom::Start(0))
                .await
                .map_err(|e| Error::tool("read", format!("Failed to seek: {e}")))?;
        }

        let mut raw_content = Vec::new();
        let mut newlines_seen = 0usize;

        let start_line_idx = match effective_offset {
            Some(n) if n > 0 => n.saturating_sub(1).try_into().unwrap_or(usize::MAX),
            _ => 0,
        };
        let limit_lines =
            effective_limit.map_or(usize::MAX, |l| l.try_into().unwrap_or(usize::MAX));
        let end_line_idx = start_line_idx.saturating_add(limit_lines);

        let mut collecting = start_line_idx == 0;
        let mut buf = vec![0u8; 64 * 1024].into_boxed_slice();
        let mut last_byte_was_newline = false;
        let mut pending_cr = false;
        let mut total_bytes_read = 0u64;

        loop {
            let n = read_some(&mut file, &mut buf)
                .await
                .map_err(|e| Error::tool("read", e.to_string()))?;
            if n == 0 {
                break;
            }
            total_bytes_read = total_bytes_read.saturating_add(n as u64);

            let chunk = normalize_line_endings_chunk(&buf[..n], &mut pending_cr);
            if chunk.is_empty() {
                continue;
            }
            last_byte_was_newline = chunk.last().is_some_and(|byte| *byte == b'\n');
            let mut chunk_cursor = 0;

            for pos in memchr::memchr_iter(b'\n', &chunk) {
                if collecting {
                    if newlines_seen + 1 == end_line_idx {
                        if raw_content.len() < DEFAULT_MAX_BYTES {
                            let remaining = DEFAULT_MAX_BYTES - raw_content.len();
                            let slice_len = (pos + 1 - chunk_cursor).min(remaining);
                            raw_content
                                .extend_from_slice(&chunk[chunk_cursor..chunk_cursor + slice_len]);
                        }
                        collecting = false;
                        chunk_cursor = pos + 1;
                    }
                }

                newlines_seen += 1;

                if !collecting && newlines_seen == start_line_idx {
                    collecting = true;
                    chunk_cursor = pos + 1;
                }
            }

            if collecting && chunk_cursor < chunk.len() && raw_content.len() < DEFAULT_MAX_BYTES {
                let remaining = DEFAULT_MAX_BYTES - raw_content.len();
                let slice_len = (chunk.len() - chunk_cursor).min(remaining);
                raw_content.extend_from_slice(&chunk[chunk_cursor..chunk_cursor + slice_len]);
            }
        }

        if pending_cr {
            last_byte_was_newline = true;
            if collecting && raw_content.len() < DEFAULT_MAX_BYTES {
                raw_content.push(b'\n');
            }
            newlines_seen += 1;
        }

        let total_lines = if total_bytes_read == 0 {
            0
        } else if last_byte_was_newline {
            newlines_seen
        } else {
            newlines_seen + 1
        };
        let text_content = String::from_utf8_lossy(&raw_content).into_owned();

        if total_lines == 0 {
            if input.offset.unwrap_or(0) > 0 {
                let offset_display = input.offset.unwrap_or(0);
                return Err(Error::tool(
                    "read",
                    format!(
                        "Offset {offset_display} is beyond end of file ({total_lines} lines total)"
                    ),
                ));
            }
            let output = ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(""))],
                details: None,
                is_error: false,
            };
            cache_tool_output(
                cache_key,
                stable_cache_dependency_for_path(&resolved, cache_mode, cache_deps.as_deref()),
                &output,
            );
            return Ok(output);
        }

        let start_line = start_line_idx;
        let start_line_display = start_line.saturating_add(1);

        if start_line >= total_lines {
            let offset_display = input.offset.unwrap_or(0);
            return Err(Error::tool(
                "read",
                format!(
                    "Offset {offset_display} is beyond end of file ({total_lines} lines total)"
                ),
            ));
        }

        let max_lines_for_truncation = input
            .limit
            .and_then(|l| usize::try_from(l).ok())
            .unwrap_or(DEFAULT_MAX_LINES);
        let display_limit = max_lines_for_truncation.saturating_add(1);

        let lines_to_take = limit_lines.min(display_limit);

        let mut selected_content = String::new();
        let line_iter = text_content.split('\n');

        let effective_iter = if text_content.ends_with('\n') {
            line_iter.take(lines_to_take)
        } else {
            line_iter.take(usize::MAX)
        };

        let max_line_num = start_line.saturating_add(lines_to_take).min(total_lines);
        let line_num_width = max_line_num.to_string().len().max(5);

        for (i, line) in effective_iter.enumerate() {
            if i >= lines_to_take || start_line + i >= total_lines {
                break;
            }
            if i > 0 {
                selected_content.push('\n');
            }
            let line_idx = start_line + i;
            let line = line.strip_suffix('\r').unwrap_or(line);
            if input.hashline {
                let tag = format_hashline_tag(line_idx, line);
                let _ = write!(selected_content, "{tag}:{line}");
            } else {
                let line_num = line_idx + 1;
                let _ = write!(selected_content, "{line_num:>line_num_width$}→{line}");
            }

            if selected_content.len() > DEFAULT_MAX_BYTES * 2 {
                break;
            }
        }

        let artifact_source = (selected_content.len() > TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES)
            .then(|| selected_content.clone());

        let mut truncation = truncate_head(
            selected_content,
            max_lines_for_truncation,
            DEFAULT_MAX_BYTES,
        );
        truncation.total_lines = total_lines;

        let mut output_text = std::mem::take(&mut truncation.content);
        let mut details: Option<serde_json::Value> = None;

        if truncation.first_line_exceeds_limit {
            let first_line = text_content.split('\n').next().unwrap_or("");
            let first_line = first_line.strip_suffix('\r').unwrap_or(first_line);
            let first_line_size = format_size(first_line.len());
            output_text = format!(
                "[Line {start_line_display} is {first_line_size}, exceeds {} limit. Use bash: sed -n '{start_line_display}p' '{}' | head -c {DEFAULT_MAX_BYTES}]",
                format_size(DEFAULT_MAX_BYTES),
                path.replace('\'', "'\\''")
            );
            details = Some(serde_json::json!({ "truncation": truncation }));
        } else if truncation.truncated {
            let end_line_display = start_line_display
                .saturating_add(truncation.output_lines)
                .saturating_sub(1);
            let next_offset = end_line_display.saturating_add(1);

            if truncation.truncated_by == Some(TruncatedBy::Lines) {
                let _ = write!(
                    output_text,
                    "\n\n[Showing lines {start_line_display}-{end_line_display} of {total_lines}. Use offset={next_offset} to continue.]"
                );
            } else {
                let _ = write!(
                    output_text,
                    "\n\n[Showing lines {start_line_display}-{end_line_display} of {total_lines} ({} limit). Use offset={next_offset} to continue.]",
                    format_size(DEFAULT_MAX_BYTES)
                );
            }

            details = Some(serde_json::json!({ "truncation": truncation }));
        } else {
            let displayed_lines = truncation.output_lines;
            let end_line_display = start_line_display
                .saturating_add(displayed_lines)
                .saturating_sub(1);

            if end_line_display < total_lines {
                let remaining = total_lines.saturating_sub(end_line_display);
                let next_offset = end_line_display.saturating_add(1);
                let _ = write!(
                    output_text,
                    "\n\n[{remaining} more lines in file. Use offset={next_offset} to continue.]"
                );
            }
        }

        if let Some(artifact_source) = artifact_source.as_deref() {
            attach_text_artifact_if_needed_with_root(
                self.artifact_root.as_deref(),
                &mut output_text,
                &mut details,
                "read",
                tool_call_id,
                "selectedTextWindow",
                artifact_source,
            );
        }

        let output = ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(output_text))],
            details,
            is_error: false,
        };
        cache_tool_output(
            cache_key,
            stable_cache_dependency_for_path(&resolved, cache_mode, cache_deps.as_deref()),
            &output,
        );
        Ok(output)
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }
    fn label(&self) -> &str {
        "read"
    }
    fn description(&self) -> &str {
        "读取文件内容。支持文本、图片（jpg/png/gif/webp）、文件信息、差异比较和编码检测。可使用 head/tail 进行部分读取、info 仅查看元数据、diff 比较文件。输出限制为 2000 行或 1MB。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to a single file to read (relative or absolute). Mutually exclusive with paths."
                },
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Multiple file paths to read in batch (relative or absolute). Mutually exclusive with path."
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-indexed). Mutually exclusive with head/tail."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read. Used with offset."
                },
                "hashline": {
                    "type": "boolean",
                    "description": "When true, output each line as N#AB:content where N is the line number and AB is a content hash. Use with hashline_edit tool for precise edits."
                },
                "head": {
                    "type": "integer",
                    "description": "Read only the first N lines. Mutually exclusive with offset/tail."
                },
                "tail": {
                    "type": "integer",
                    "description": "Read only the last N lines. Mutually exclusive with offset/head."
                },
                "info": {
                    "type": "boolean",
                    "description": "When true, show file metadata (size, line count, encoding, modified time) without reading content."
                },

                "diff": {
                    "type": "string",
                    "description": "Compare this file with another file. Shows a unified diff between the two files."
                },
                "context": {
                    "type": "integer",
                    "description": "Lines of context for diff mode (default: 3)."
                },
                "summary_only": {
                    "type": "boolean",
                    "description": "When true and used with diff, show only diff statistics without the full diff."
                }
            },
            "oneOf": [
                { "required": ["path"] },
                { "required": ["paths"] }
            ]
        })
    }

    fn effects(&self) -> ToolEffects {
        ToolEffects::read()
    }

    #[allow(clippy::too_many_lines, clippy::option_if_let_else)]
    async fn execute(
        &self,
        tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
        _abort: Option<AbortSignal>,
    ) -> Result<ToolOutput> {
        let input: ReadInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        if matches!(input.limit, Some(limit) if limit <= 0) {
            return Err(Error::validation(
                "`limit` must be greater than 0".to_string(),
            ));
        }
        if matches!(input.offset, Some(offset) if offset < 0) {
            return Err(Error::validation(
                "`offset` must be non-negative".to_string(),
            ));
        }

        // head, offset, tail are mutually exclusive
        let has_head = input.head.is_some();
        let has_offset = input.offset.is_some();
        let has_tail = input.tail.is_some();
        let mutex_count = [has_head, has_offset, has_tail]
            .iter()
            .filter(|&&x| x)
            .count();
        if mutex_count > 1 {
            let given = [
                has_head.then_some("head"),
                has_offset.then_some("offset"),
                has_tail.then_some("tail"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("`, `");
            return Err(Error::validation(format!(
                "`{given}` are mutually exclusive, but all were provided"
            )));
        }

        let paths: Vec<&str> = match (&input.path, &input.paths) {
            (Some(p), None) => vec![p.as_str()],
            (None, Some(ps)) => {
                if ps.is_empty() {
                    return Err(Error::validation(
                        "`paths` must contain at least one path".to_string(),
                    ));
                }
                ps.iter().map(String::as_str).collect()
            }
            (Some(_), Some(_)) => {
                return Err(Error::validation(
                    "`path` and `paths` are mutually exclusive".to_string(),
                ));
            }
            (None, None) => {
                return Err(Error::validation(
                    "Either `path` or `paths` must be provided".to_string(),
                ));
            }
        };

        if paths.len() == 1 {
            return self.read_single_file(paths[0], &input, tool_call_id).await;
        }

        let mut all_text = String::new();
        let combined_details: Option<serde_json::Value> = None;
        let mut had_error = false;

        for (i, p) in paths.iter().enumerate() {
            match self.read_single_file(p, &input, tool_call_id).await {
                Ok(output) => {
                    if i > 0 {
                        all_text.push_str("\n\n---\n\n");
                    }
                    let header = format!("File: {p}\n");
                    all_text.push_str(&header);
                    for block in &output.content {
                        if let ContentBlock::Text(t) = block {
                            all_text.push_str(&t.text);
                        }
                    }
                }
                Err(e) => {
                    if i > 0 {
                        all_text.push_str("\n\n---\n\n");
                    }
                    let _ = write!(all_text, "File: {p}\nError: {e}");
                    had_error = true;
                }
            }
        }

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(all_text))],
            details: combined_details,
            is_error: had_error,
        })
    }
}

// ============================================================================
