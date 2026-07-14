use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use async_trait::async_trait;
use serde::Deserialize;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use asupersync::time::{sleep, wall_now};
// ============================================================================
// Find Tool
// ============================================================================

/// Input parameters for the find tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FindInput {
    pattern: String,
    path: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug)]
struct FindEntry {
    rel: String,
    modified: Option<SystemTime>,
}

pub struct FindTool {
    cwd: PathBuf,
    artifact_root: Option<PathBuf>,
}

impl FindTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            artifact_root: None,
        }
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for FindTool {
    fn name(&self) -> &str {
        "find"
    }
    fn label(&self) -> &str {
        "find"
    }
    fn description(&self) -> &str {
        "Search for files by glob pattern. Returns matching file paths relative to the search directory. Sorted by modification time (newest first). Respects .gitignore. Output is truncated to 1000 results or 1MB (whichever is hit first)."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files, e.g. '*.ts', '**/*.json', or 'src/**/*.spec.ts'"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current directory)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 1000)"
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
    ) -> Result<ToolOutput> {
        let input_value = input.clone();
        let input: FindInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        if matches!(input.limit, Some(0)) {
            return Err(Error::validation(
                "`limit` must be greater than 0".to_string(),
            ));
        }

        let search_dir = input.path.as_deref().unwrap_or(".");
        let search_path = resolve_read_path(search_dir, &self.cwd);
        let search_path = enforce_cwd_scope(&search_path, &self.cwd, "find")?;
        let search_path = strip_unc_prefix(search_path);
        let effective_limit = input.limit.unwrap_or(DEFAULT_FIND_LIMIT);
        // Overfetch one result so limit notices only appear after confirmed overflow.
        let scan_limit = effective_limit.saturating_add(1);

        if !search_path.exists() {
            return Err(Error::tool(
                "find",
                format!("Path not found: {}", search_path.display()),
            ));
        }

        let cache_key = tool_cache_key("find", &self.cwd, &input_value);
        let cache_mode = if search_path.is_dir() {
            ToolCacheFingerprintMode::DirectoryRecursive
        } else {
            ToolCacheFingerprintMode::FileContent
        };
        let cache_deps = cache_dependency_for_path(&search_path, cache_mode);
        if let Some(output) = cached_tool_output(&cache_key, cache_deps.as_deref()) {
            return Ok(output);
        }

        let fd_cmd = find_fd_binary().ok_or_else(|| {
            Error::tool(
                "find",
                "fd is not available (please install fd-find or fd)".to_string(),
            )
        })?;

        // Build fd arguments
        let mut args: Vec<String> = vec![
            "--glob".to_string(),
            "--color=never".to_string(),
            "--hidden".to_string(),
            "--max-results".to_string(),
            scan_limit.to_string(),
        ];

        // NOTE: We rely on fd's native .gitignore discovery. We only explicitly pass
        // the root .gitignore if it exists, to ensure it's respected even if the
        // search path logic might otherwise miss it.
        // We do NOT perform a blocking `glob("**/.gitignore")` here.
        let workspace_gitignore = self.cwd.join(".gitignore");
        if workspace_gitignore.exists() {
            args.push("--ignore-file".to_string());
            args.push(workspace_gitignore.display().to_string());
        }
        let root_gitignore = search_path.join(".gitignore");
        if root_gitignore != workspace_gitignore && root_gitignore.exists() {
            args.push("--ignore-file".to_string());
            args.push(root_gitignore.display().to_string());
        }

        args.push("--".to_string());
        args.push(input.pattern.clone());
        args.push(search_path.display().to_string());

        let mut child = command_with_default_sigpipe_in_dir(fd_cmd, &self.cwd)
            .map_err(|e| Error::tool("find", format!("Failed to prepare fd: {e}")))?
            .args(args)
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::tool("find", format!("Failed to run fd: {e}")))?;

        let stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| Error::tool("find", "Missing stdout"))?;
        let stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| Error::tool("find", "Missing stderr"))?;

        let mut guard = ProcessGuard::new(child, ProcessCleanupMode::ChildOnly);

        let stdout_handle = std::thread::spawn(move || -> std::result::Result<Vec<u8>, String> {
            read_to_end_capped_and_drain(stdout_pipe, READ_TOOL_MAX_BYTES)
        });

        let stderr_handle = std::thread::spawn(move || -> std::result::Result<Vec<u8>, String> {
            read_to_end_capped_and_drain(stderr_pipe, READ_TOOL_MAX_BYTES)
        });

        let tick = Duration::from_millis(10);
        let start_time = std::time::Instant::now();
        let timeout_ms = 60_000; // 60 seconds
        let mut timed_out = false;
        let mut cx_cancelled = false;

        let status = loop {
            let agent_cx = AgentCx::for_current_or_request();
            let cx = agent_cx.cx();
            if cx.checkpoint().is_err() {
                cx_cancelled = true;
                let _ = guard.kill();
                break None;
            }

            // Check if process is done
            match guard.try_wait_child() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    if start_time.elapsed().as_millis() > timeout_ms {
                        timed_out = true;
                        let _ = guard.kill();
                        break None;
                    }
                    let now = cx.timer_driver().map_or_else(wall_now, |timer| timer.now());
                    sleep(now, tick).await;
                }
                Err(e) => return Err(Error::tool("find", e.to_string())),
            }
        };

        let stdout_bytes = stdout_handle
            .join()
            .map_err(|_| Error::tool("find", "fd stdout reader thread panicked"))?
            .map_err(|err| Error::tool("find", format!("Failed to read fd stdout: {err}")))?;
        let stderr_bytes = stderr_handle
            .join()
            .map_err(|_| Error::tool("find", "fd stderr reader thread panicked"))?
            .map_err(|err| Error::tool("find", format!("Failed to read fd stderr: {err}")))?;

        if cx_cancelled {
            return Err(Error::tool("find", "Command cancelled"));
        }
        if timed_out {
            return Err(Error::tool("find", "Command timed out after 60 seconds"));
        }
        let status = status.expect("fd exit status after successful completion");

        let mut stdout = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
        if stdout_bytes.len() as u64 > READ_TOOL_MAX_BYTES {
            stdout.push_str("\n... [stdout truncated] ...");
        }
        let mut stderr = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
        if stderr_bytes.len() as u64 > READ_TOOL_MAX_BYTES {
            stderr.push_str("\n... [stderr truncated] ...");
        }

        if !status.success() && stdout.is_empty() {
            if status.code() == Some(1) && stderr.is_empty() {
                // fd uses exit code 1 for "no matches"; treat as empty result.
            } else {
                let code = status.code().unwrap_or(1);
                let msg = if stderr.is_empty() {
                    format!("fd exited with code {code}")
                } else {
                    stderr
                };
                return Err(Error::tool("find", msg));
            }
        }

        if stdout.is_empty() {
            let output = ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(
                    "No files found matching pattern",
                ))],
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

        let mut entries: Vec<FindEntry> = Vec::new();
        for raw_line in stdout.lines() {
            let line = raw_line.trim_end_matches('\r').trim();
            if line.is_empty() {
                continue;
            }

            // On Windows, fd may emit `//?/…` or `\\?\…` extended-length
            // paths. Strip the prefix so relativization works correctly.
            let clean = strip_unc_prefix(PathBuf::from(line));
            let line_path = clean.as_path();
            let mut rel = if line_path.is_absolute() {
                line_path.strip_prefix(&search_path).map_or_else(
                    |_| line_path.to_string_lossy().to_string(),
                    |stripped| stripped.to_string_lossy().to_string(),
                )
            } else {
                line_path.to_string_lossy().to_string()
            };

            let full_path = if line_path.is_absolute() {
                line_path.to_path_buf()
            } else {
                search_path.join(line_path)
            };
            if full_path.is_dir() && !rel.ends_with('/') {
                rel.push('/');
            }

            let modified = std::fs::metadata(&full_path)
                .and_then(|meta| meta.modified())
                .ok();
            entries.push(FindEntry { rel, modified });
        }

        entries.sort_by(|a, b| {
            let ordering = match (&a.modified, &b.modified) {
                (Some(a_time), Some(b_time)) => b_time.cmp(a_time),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            ordering.then_with(|| {
                let a_lower = a.rel.to_lowercase();
                let b_lower = b.rel.to_lowercase();
                a_lower.cmp(&b_lower).then_with(|| a.rel.cmp(&b.rel))
            })
        });

        if entries.is_empty() {
            let output = ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new(
                    "No files found matching pattern",
                ))],
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

        let result_limit_reached = entries.len() > effective_limit;
        let mut output_builder = HeadTruncatingLineWriter::new(DEFAULT_MAX_BYTES);
        let mut artifact_source = String::new();
        for entry in entries.into_iter().take(effective_limit) {
            output_builder.push_line(&entry.rel);
            append_artifact_source_line(&mut artifact_source, &entry.rel);
        }
        let mut truncation = output_builder.finish();

        let mut result_output = std::mem::take(&mut truncation.content);
        let mut notices: Vec<String> = Vec::new();
        let mut details_map = serde_json::Map::new();

        if !status.success() {
            let code = status.code().unwrap_or(1);
            notices.push(format!("fd exited with code {code}"));
        }

        if result_limit_reached {
            notices.push(format!(
                "{effective_limit} results limit reached. Use limit={} for more, or refine pattern",
                effective_limit * 2
            ));
            details_map.insert(
                "resultLimitReached".to_string(),
                serde_json::Value::Number(serde_json::Number::from(effective_limit)),
            );
        }

        if truncation.truncated {
            notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
            details_map.insert("truncation".to_string(), serde_json::to_value(truncation)?);
        }

        if !notices.is_empty() {
            let _ = write!(result_output, "\n\n[{}]", notices.join(". "));
        }

        let mut details = if details_map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(details_map))
        };

        attach_text_artifact_if_needed_with_root(
            self.artifact_root.as_deref(),
            &mut result_output,
            &mut details,
            "find",
            tool_call_id,
            "fileResults",
            &artifact_source,
        );

        let output = ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(result_output))],
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

// ============================================================================

