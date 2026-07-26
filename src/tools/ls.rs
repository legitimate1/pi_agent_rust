use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};

// ============================================================================
// Ls Tool
// ============================================================================

/// Input parameters for the ls tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LsInput {
    path: Option<String>,
    limit: Option<usize>,
}

pub struct LsTool {
    cwd: PathBuf,
    artifact_root: Option<PathBuf>,
}

impl LsTool {
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

#[async_trait]
#[allow(clippy::unnecessary_literal_bound, clippy::too_many_lines)]
impl Tool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }
    fn label(&self) -> &str {
        "ls"
    }
    fn description(&self) -> &str {
        "列出目录内容。返回字母排序的条目，目录以 '/' 结尾。包含点文件。路径须为目录（非目录路径报错）。条目超限自动截断。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to list (default: current directory)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of entries to return (default: 500)"
                }
            }
        })
    }

    fn effects(&self) -> ToolEffects {
        ToolEffects::read()
    }

    async fn execute(
        &self,
        tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
        _abort: Option<AbortSignal>,
    ) -> Result<ToolOutput> {
        let input_value = input.clone();
        let input: LsInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        if matches!(input.limit, Some(0)) {
            return Err(Error::validation(
                "`limit` must be greater than 0".to_string(),
            ));
        }

        let dir_path = input
            .path
            .as_ref()
            .map_or_else(|| self.cwd.clone(), |p| resolve_read_path(p, &self.cwd));

        let effective_limit = input.limit.unwrap_or(DEFAULT_LS_LIMIT);

        if !dir_path.exists() {
            return Err(Error::tool(
                "ls",
                format!("Path not found: {}", dir_path.display()),
            ));
        }
        if !dir_path.is_dir() {
            return Err(Error::tool(
                "ls",
                format!("Not a directory: {}", dir_path.display()),
            ));
        }

        let cache_key = tool_cache_key("ls", &self.cwd, &input_value);
        let cache_mode = ToolCacheFingerprintMode::DirectoryImmediate;
        let cache_deps = cache_dependency_for_path(&dir_path, cache_mode);
        if let Some(output) = cached_tool_output(&cache_key, cache_deps.as_deref()) {
            return Ok(output);
        }

        let mut entries = Vec::new();
        let mut read_dir = asupersync::fs::read_dir(&dir_path)
            .await
            .map_err(|e| Error::tool("ls", format!("Cannot read directory: {e}")))?;

        let mut scan_limit_reached = false;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| Error::tool("ls", format!("Cannot read directory entry: {e}")))?
        {
            if entries.len() >= LS_SCAN_HARD_LIMIT {
                scan_limit_reached = true;
                break;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            // Handle broken symlinks or permission errors by treating them as non-directories
            // Optimization: use file_type() first to avoid stat overhead on every file.
            let is_dir = match entry.file_type().await {
                Ok(ft) => {
                    if ft.is_dir() {
                        true
                    } else if ft.is_symlink() {
                        // Only stat if it's a symlink to see if it points to a directory
                        entry.metadata().await.is_ok_and(|meta| meta.is_dir())
                    } else {
                        false
                    }
                }
                Err(_) => entry.metadata().await.is_ok_and(|meta| meta.is_dir()),
            };
            entries.push((name, is_dir));
        }

        // Sort alphabetically (case-insensitive).
        entries.sort_by_cached_key(|(a, _)| a.to_lowercase());

        let mut output_builder = HeadTruncatingLineWriter::new(DEFAULT_MAX_BYTES);
        let mut artifact_source = String::new();
        let mut emitted_entries = 0usize;
        let mut entry_limit_reached = false;

        for (entry, is_dir) in entries {
            if emitted_entries >= effective_limit {
                entry_limit_reached = true;
                break;
            }
            let line = if is_dir { format!("{entry}/") } else { entry };
            output_builder.push_line(&line);
            append_artifact_source_line(&mut artifact_source, &line);
            emitted_entries = emitted_entries.saturating_add(1);
        }

        if emitted_entries == 0 {
            let output = ToolOutput {
                content: vec![ContentBlock::Text(TextContent::new("(empty directory)"))],
                details: None,
                is_error: false,
            };
            cache_tool_output(
                cache_key,
                stable_cache_dependency_for_path(&dir_path, cache_mode, cache_deps.as_deref()),
                &output,
            );
            return Ok(output);
        }

        // Apply byte truncation while writing, avoiding a second joined copy.
        let mut truncation = output_builder.finish();

        let mut output = std::mem::take(&mut truncation.content);
        let mut details_map = serde_json::Map::new();
        let mut notices: Vec<String> = Vec::new();

        if entry_limit_reached {
            notices.push(format!(
                "{effective_limit} entries limit reached. Use limit={} for more",
                effective_limit * 2
            ));
            details_map.insert(
                "entryLimitReached".to_string(),
                serde_json::Value::Number(serde_json::Number::from(effective_limit)),
            );
        }

        if scan_limit_reached {
            notices.push(format!(
                "Directory scan limited to {LS_SCAN_HARD_LIMIT} entries to prevent system overload"
            ));
            details_map.insert(
                "scanLimitReached".to_string(),
                serde_json::Value::Number(serde_json::Number::from(LS_SCAN_HARD_LIMIT)),
            );
        }

        if truncation.truncated {
            notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
            details_map.insert("truncation".to_string(), serde_json::to_value(truncation)?);
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
            "ls",
            tool_call_id,
            "directoryEntries",
            &artifact_source,
        );

        let output = ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(output))],
            details,
            is_error: false,
        };
        cache_tool_output(
            cache_key,
            stable_cache_dependency_for_path(&dir_path, cache_mode, cache_deps.as_deref()),
            &output,
        );
        Ok(output)
    }
}

// ============================================================================
