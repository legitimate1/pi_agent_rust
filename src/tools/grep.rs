use super::hashline::format_hashline_tag;
use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use asupersync::time::{sleep, wall_now};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tracing;
// ============================================================================
// Grep Tool
// ============================================================================

/// Input parameters for the grep tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrepInput {
    pattern: String,
    path: Option<String>,
    glob: Option<String>,
    ignore_case: Option<bool>,
    literal: Option<bool>,
    context: Option<usize>,
    limit: Option<usize>,
    #[serde(default)]
    hashline: bool,
}

pub struct GrepTool {
    cwd: PathBuf,
    artifact_root: Option<PathBuf>,
}

impl GrepTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            artifact_root: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_artifact_root(cwd: &Path, artifact_root: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            artifact_root: Some(artifact_root.to_path_buf()),
        }
    }
}

/// Result of truncating a single grep output line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruncateLineResult {
    pub(crate) text: String,
    pub(crate) was_truncated: bool,
}

/// Truncate a single line to max characters, adding a marker suffix.
///
/// Matches pi-mono behavior: `${line.slice(0, maxChars)}... [truncated]`.
pub fn truncate_line(line: &str, max_chars: usize) -> TruncateLineResult {
    let mut chars = line.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_none() {
        return TruncateLineResult {
            text: line.to_string(),
            was_truncated: false,
        };
    }

    TruncateLineResult {
        text: format!("{prefix}... [truncated]"),
        was_truncated: true,
    }
}

pub fn process_rg_json_match_line(
    line_res: std::io::Result<String>,
    matches: &mut Vec<(PathBuf, usize)>,
    match_count: &mut usize,
    match_limit_reached: &mut bool,
    scan_limit: usize,
) {
    if *match_limit_reached {
        return;
    }

    let line = match line_res {
        Ok(l) => l,
        Err(e) => {
            tracing::debug!("Skipping ripgrep output line due to read error: {e}");
            return;
        }
    };
    if line.trim().is_empty() {
        return;
    }

    let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
        return;
    };

    if event.get("type").and_then(serde_json::Value::as_str) != Some("match") {
        return;
    }

    let file_path = event
        .pointer("/data/path/text")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from);
    let line_number = event
        .pointer("/data/line_number")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok());

    if let (Some(fp), Some(ln)) = (file_path, line_number) {
        matches.push((fp, ln));
        *match_count += 1;
        if *match_count >= scan_limit {
            *match_limit_reached = true;
        }
    }
}

fn drain_rg_stdout(
    stdout_rx: &std::sync::mpsc::Receiver<std::io::Result<String>>,
    matches: &mut Vec<(PathBuf, usize)>,
    match_count: &mut usize,
    match_limit_reached: &mut bool,
    scan_limit: usize,
) {
    while let Ok(line_res) = stdout_rx.try_recv() {
        process_rg_json_match_line(
            line_res,
            matches,
            match_count,
            match_limit_reached,
            scan_limit,
        );
        if *match_limit_reached {
            break;
        }
    }
}

fn drain_rg_stderr(
    stderr_rx: &std::sync::mpsc::Receiver<std::result::Result<Vec<u8>, String>>,
    stderr_bytes: &mut Vec<u8>,
) -> Result<()> {
    while let Ok(chunk_result) = stderr_rx.try_recv() {
        let chunk = chunk_result
            .map_err(|err| Error::tool("grep", format!("Failed to read stderr: {err}")))?;
        stderr_bytes.extend_from_slice(&chunk);
    }
    Ok(())
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }
    fn label(&self) -> &str {
        "grep"
    }
    fn description(&self) -> &str {
        "在文件内容中搜索匹配模式。返回匹配行及文件路径与行号。遵循 .gitignore。输出限制为 100 条匹配或 1MB（以先到者为准）。超长行截断至 500 字符。使用 hashline=true 可获取 N#AB 内容哈希标签，配合 hashline_edit 使用。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern (regex or literal string)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search (default: current directory)"
                },
                "glob": {
                    "type": "string",
                    "description": "Filter files by glob pattern, e.g. '*.ts' or '**/*.spec.ts'"
                },
                "ignoreCase": {
                    "type": "boolean",
                    "description": "Case-insensitive search (default: false)"
                },
                "literal": {
                    "type": "boolean",
                    "description": "Treat pattern as literal string instead of regex (default: false)"
                },
                "context": {
                    "type": "integer",
                    "description": "Number of lines to show before and after each match (default: 0)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of matches to return (default: 100)"
                },
                "hashline": {
                    "type": "boolean",
                    "description": "When true, output each line as N#AB:content where N is the line number and AB is a content hash. Use with hashline_edit tool for precise edits."
                }
            },
            "required": ["pattern"]
        })
    }

    fn effects(&self) -> ToolEffects {
        ToolEffects::read()
    }

    #[allow(clippy::too_many_lines)]
    async fn execute(
        &self,
        tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
        _abort: Option<AbortSignal>,
    ) -> Result<ToolOutput> {
        let input_value = input.clone();
        let input: GrepInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        if matches!(input.limit, Some(0)) {
            return Err(Error::validation(
                "`limit` must be greater than 0".to_string(),
            ));
        }

        if !rg_available() {
            return Err(Error::tool(
                "grep",
                "ripgrep (rg) is not available (please install ripgrep)".to_string(),
            ));
        }

        let search_dir = input.path.as_deref().unwrap_or(".");
        let search_path = resolve_read_path(search_dir, &self.cwd);

        let is_directory = asupersync::fs::metadata(&search_path)
            .await
            .map_err(|e| {
                Error::tool(
                    "grep",
                    format!("Cannot access path {}: {e}", search_path.display()),
                )
            })?
            .is_dir();

        let context_value = input.context.unwrap_or(0);
        let effective_limit = input.limit.unwrap_or(DEFAULT_GREP_LIMIT).max(1);
        // Overfetch one match so limit notices only appear after confirmed overflow.
        let scan_limit = effective_limit.saturating_add(1);
        let cache_key = tool_cache_key("grep", &self.cwd, &input_value);
        let cache_mode = if is_directory {
            ToolCacheFingerprintMode::DirectoryRecursive
        } else {
            ToolCacheFingerprintMode::FileContent
        };
        let cache_deps = cache_dependency_for_path(&search_path, cache_mode);
        if let Some(output) = cached_tool_output(&cache_key, cache_deps.as_deref()) {
            return Ok(output);
        }

        let mut args: Vec<String> = vec![
            "--json".to_string(),
            "--line-number".to_string(),
            "--color=never".to_string(),
            "--hidden".to_string(),
            // Prevent massive JSON lines from minified files causing OOM
            "--max-columns=10000".to_string(),
        ];

        if input.ignore_case.unwrap_or(false) {
            args.push("--ignore-case".to_string());
        }
        if input.literal.unwrap_or(false) {
            args.push("--fixed-strings".to_string());
        }
        if let Some(glob) = &input.glob {
            args.push("--glob".to_string());
            args.push(glob.clone());
        }

        // Mirror find-tool behavior: explicitly pass root/nested .gitignore files
        // so ignore rules apply consistently even outside a git worktree.
        let ignore_root = if is_directory {
            search_path.clone()
        } else {
            search_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        };
        // NOTE: We rely on rg's native .gitignore discovery. We only explicitly pass
        // the root .gitignore if it exists, to ensure it's respected even if the
        // search path logic might otherwise miss it (e.g. searching a subdir).
        // We do NOT perform a blocking `glob("**/.gitignore")` here, as that stalls
        // the async runtime on large repos.
        let workspace_gitignore = self.cwd.join(".gitignore");
        if workspace_gitignore.exists() {
            args.push("--ignore-file".to_string());
            args.push(workspace_gitignore.display().to_string());
        }
        let root_gitignore = ignore_root.join(".gitignore");
        if root_gitignore != workspace_gitignore && root_gitignore.exists() {
            args.push("--ignore-file".to_string());
            args.push(root_gitignore.display().to_string());
        }

        args.push("--".to_string());
        args.push(input.pattern.clone());
        args.push(search_path.display().to_string());

        let rg_cmd = find_rg_binary().ok_or_else(|| {
            Error::tool(
                "grep",
                "rg is not available (please install ripgrep or rg)".to_string(),
            )
        })?;

        let mut cmd = command_with_default_sigpipe(rg_cmd)
            .map_err(|e| Error::tool("grep", format!("Failed to prepare ripgrep: {e}")))?;
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        isolate_command_process_group(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| Error::tool("grep", format!("Failed to run ripgrep: {e}")))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::tool("grep", "Missing stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::tool("grep", "Missing stderr".to_string()))?;

        let mut guard = ProcessGuard::new(child, ProcessCleanupMode::ProcessGroupTree);

        let (stdout_tx, stdout_rx) = std::sync::mpsc::sync_channel(1024);
        let (stderr_tx, stderr_rx) =
            std::sync::mpsc::sync_channel::<std::result::Result<Vec<u8>, String>>(1024);

        let stdout_thread = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                if stdout_tx.send(line).is_err() {
                    break;
                }
            }
        });

        let stderr_thread = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            let _ = stderr_tx.send(read_to_end_capped_and_drain(reader, READ_TOOL_MAX_BYTES));
        });

        let mut matches: Vec<(PathBuf, usize)> = Vec::new();
        let mut match_count: usize = 0;
        let mut match_scan_limit_reached = false;
        let mut stderr_bytes = Vec::new();

        let tick = Duration::from_millis(10);
        let mut cx_cancelled = false;

        let exit_status = loop {
            let agent_cx = AgentCx::for_current_or_request();
            let cx = agent_cx.cx();
            if cx.checkpoint().is_err() {
                cx_cancelled = true;
                break None;
            }

            drain_rg_stdout(
                &stdout_rx,
                &mut matches,
                &mut match_count,
                &mut match_scan_limit_reached,
                scan_limit,
            );
            drain_rg_stderr(&stderr_rx, &mut stderr_bytes)?;

            if match_scan_limit_reached {
                break None;
            }

            match guard.try_wait_child() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    let now = cx.timer_driver().map_or_else(wall_now, |timer| timer.now());
                    sleep(now, tick).await;
                }
                Err(e) => return Err(Error::tool("grep", e.to_string())),
            }
        };

        drain_rg_stdout(
            &stdout_rx,
            &mut matches,
            &mut match_count,
            &mut match_scan_limit_reached,
            scan_limit,
        );

        let code = if match_scan_limit_reached || cx_cancelled {
            // Avoid buffering unbounded stdout/stderr once we've hit the match limit.
            // `kill()` terminates the process, and we reap it in a background thread
            // so the stdout reader threads can exit promptly without blocking this task.
            let _ = guard.kill();
            // Drop any buffered stdout/stderr lines that were queued before termination.
            while stdout_rx.try_recv().is_ok() {}
            while stderr_rx.try_recv().is_ok() {}
            0
        } else {
            let status = exit_status.expect("rg exit status");
            status.code().unwrap_or(0)
        };

        // Keep draining while waiting for reader threads to finish; otherwise a
        // bounded channel can fill and block the sender thread, causing join()
        // to hang after ripgrep has already exited.
        while !stdout_thread.is_finished() || !stderr_thread.is_finished() {
            if match_scan_limit_reached || cx_cancelled {
                while stdout_rx.try_recv().is_ok() {}
            } else {
                drain_rg_stdout(
                    &stdout_rx,
                    &mut matches,
                    &mut match_count,
                    &mut match_scan_limit_reached,
                    scan_limit,
                );
            }
            drain_rg_stderr(&stderr_rx, &mut stderr_bytes)?;
            sleep(wall_now(), Duration::from_millis(1)).await;
        }

        if cx_cancelled {
            return Err(Error::tool("grep", "Command cancelled"));
        }

        // Ensure stdout/stderr reader threads have fully drained the pipes before
        // we decide whether matches were found. Without this, fast ripgrep runs can
        // exit before the reader thread has delivered JSON match lines, causing
        // false "No matches found" results.
        stdout_thread
            .join()
            .map_err(|_| Error::tool("grep", "ripgrep stdout reader thread panicked"))?;
        stderr_thread
            .join()
            .map_err(|_| Error::tool("grep", "ripgrep stderr reader thread panicked"))?;

        // Drain any remaining stdout/stderr produced after the last poll.
        if match_scan_limit_reached {
            while stdout_rx.try_recv().is_ok() {}
        } else {
            drain_rg_stdout(
                &stdout_rx,
                &mut matches,
                &mut match_count,
                &mut match_scan_limit_reached,
                scan_limit,
            );
        }
        drain_rg_stderr(&stderr_rx, &mut stderr_bytes)?;

        let mut stderr_text = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
        if stderr_bytes.len() as u64 > READ_TOOL_MAX_BYTES {
            stderr_text.push_str("\n... [stderr truncated] ...");
        }
        if !match_scan_limit_reached && code != 0 && code != 1 {
            let msg = if stderr_text.is_empty() {
                format!("ripgrep exited with code {code}")
            } else {
                stderr_text
            };
            return Err(Error::tool("grep", msg));
        }

        let match_limit_reached = match_count > effective_limit;
        if match_limit_reached {
            matches.truncate(effective_limit);
            match_count = effective_limit;
        }

        if match_count == 0 {
            let output = ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new("No matches found"))],
                details: None,
                is_error: false,
            };
            cache_tool_output(
                cache_key,
                stable_cache_dependency_for_path(&search_path, cache_mode, cache_deps.as_deref()),
                &output,
            );
            return Ok(output);
        }

        let mut file_cache: HashMap<PathBuf, Vec<String>> = HashMap::new();
        let mut output_builder = HeadTruncatingLineWriter::new(DEFAULT_MAX_BYTES);
        let mut artifact_source = String::new();
        let mut lines_truncated = false;

        // Group matches by file to merge overlapping context windows
        let mut file_order: Vec<PathBuf> = Vec::new();
        let mut matches_by_file: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for (file_path, line_number) in &matches {
            if !matches_by_file.contains_key(file_path) {
                file_order.push(file_path.clone());
            }
            matches_by_file
                .entry(file_path.clone())
                .or_default()
                .push(*line_number);
        }

        for file_path in file_order {
            let Some(mut match_lines) = matches_by_file.remove(&file_path) else {
                continue;
            };
            let relative_path = format_grep_path(&file_path, &self.cwd);
            let lines = get_file_lines_async(&file_path, &mut file_cache).await;

            if lines.is_empty() {
                if let Some(first_match) = match_lines.first() {
                    let line = format!(
                        "{relative_path}:{first_match}: (unable to read file or too large)"
                    );
                    output_builder.push_line(&line);
                    append_artifact_source_line(&mut artifact_source, &line);
                }
                continue;
            }

            match_lines.sort_unstable();
            match_lines.dedup();

            let mut blocks: Vec<(usize, usize)> = Vec::new();
            for &line_number in &match_lines {
                let start = if context_value > 0 {
                    line_number.saturating_sub(context_value).max(1)
                } else {
                    line_number
                };
                let end = if context_value > 0 {
                    line_number.saturating_add(context_value).min(lines.len())
                } else {
                    line_number
                };

                if let Some(last_block) = blocks.last_mut() {
                    if start <= last_block.1.saturating_add(1) {
                        last_block.1 = last_block.1.max(end);
                        continue;
                    }
                }
                blocks.push((start, end));
            }

            for (i, (start, end)) in blocks.into_iter().enumerate() {
                if i > 0 {
                    output_builder.push_line("--");
                    append_artifact_source_line(&mut artifact_source, "--");
                }
                for current in start..=end {
                    let line_text = lines.get(current - 1).map_or("", String::as_str);
                    let sanitized = line_text.replace('\r', "");
                    let truncated = truncate_line(&sanitized, GREP_MAX_LINE_LENGTH);
                    if truncated.was_truncated {
                        lines_truncated = true;
                    }

                    if input.hashline {
                        let line_idx = current - 1; // 0-indexed for hashline
                        let tag = format_hashline_tag(line_idx, &sanitized);
                        let line = if match_lines.binary_search(&current).is_ok() {
                            format!("{relative_path}:{tag}: {}", truncated.text)
                        } else {
                            format!("{relative_path}-{tag}- {}", truncated.text)
                        };
                        output_builder.push_line(&line);
                        append_artifact_source_line(&mut artifact_source, &line);
                    } else if match_lines.binary_search(&current).is_ok() {
                        let line = format!("{relative_path}:{current}: {}", truncated.text);
                        output_builder.push_line(&line);
                        append_artifact_source_line(&mut artifact_source, &line);
                    } else {
                        let line = format!("{relative_path}-{current}- {}", truncated.text);
                        output_builder.push_line(&line);
                        append_artifact_source_line(&mut artifact_source, &line);
                    }
                }
            }
        }

        // Apply byte truncation while writing, avoiding a second joined copy.
        let mut truncation = output_builder.finish();

        let mut output = std::mem::take(&mut truncation.content);
        let mut notices: Vec<String> = Vec::new();
        let mut details_map = serde_json::Map::new();

        if match_limit_reached {
            notices.push(format!(
                "{effective_limit} matches limit reached. Use limit={} for more, or refine pattern",
                effective_limit * 2
            ));
            details_map.insert(
                "matchLimitReached".to_string(),
                serde_json::Value::Number(serde_json::Number::from(effective_limit)),
            );
        }

        if truncation.truncated {
            notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
            details_map.insert("truncation".to_string(), serde_json::to_value(truncation)?);
        }

        if lines_truncated {
            notices.push(format!(
                "Some lines truncated to {GREP_MAX_LINE_LENGTH} chars. Use read tool to see full lines"
            ));
            details_map.insert("linesTruncated".to_string(), serde_json::Value::Bool(true));
        }

        if !notices.is_empty() {
            let _ = write!(output, "\n\n[{}]", notices.join(". "));
        }

        let mut details = if details_map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(details_map))
        };

        attach_text_artifact_if_needed_with_root(
            self.artifact_root.as_deref(),
            &mut output,
            &mut details,
            "grep",
            tool_call_id,
            "searchResults",
            &artifact_source,
        );

        let output = ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(output))],
            details,
            is_error: false,
        };
        cache_tool_output(
            cache_key,
            stable_cache_dependency_for_path(&search_path, cache_mode, cache_deps.as_deref()),
            &output,
        );
        Ok(output)
    }
}
