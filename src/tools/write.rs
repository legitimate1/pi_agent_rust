use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use async_trait::async_trait;
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
// ============================================================================
// Write Tool
// ============================================================================

/// Input parameters for the write tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteInput {
    path: String,
    content: String,
    /// If true, run syntax/format check after writing.
    #[serde(default)]
    verify: bool,
}

pub struct WriteTool {
    cwd: PathBuf,
}

impl WriteTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
        }
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }
    fn label(&self) -> &str {
        "write"
    }
    fn description(&self) -> &str {
        "将内容写入文件。文件不存在则创建，存在则覆盖。自动创建父目录。单次写入限 100MB；路径须指向文件（非目录）。可选 verify 参数在写入后运行语法检查。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write (relative or absolute)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                },
                "verify": {
                    "type": "boolean",
                    "description": "若为 true，写入后自动运行语法检查（.rs → rustfmt --check, .json/.toml → 进程内解析, .ts → prettier --check）。依赖工具需在 PATH 中可用。默认 false。",
                    "default": false
                }
            },
            "required": ["path", "content"]
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
        let input: WriteInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        if input.content.len() > WRITE_TOOL_MAX_BYTES {
            return Err(Error::validation(format!(
                "Content size exceeds maximum allowed ({} > {} bytes)",
                input.content.len(),
                WRITE_TOOL_MAX_BYTES
            )));
        }

        let path = resolve_path(&input.path, &self.cwd);

        if let Ok(meta) = asupersync::fs::metadata(&path).await {
            if !meta.is_file() {
                return Err(Error::tool(
                    "write",
                    format!("Path {} is not a regular file", path.display()),
                ));
            }
            if let Err(err) = asupersync::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .await
            {
                let message = match err.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied: {}", input.path)
                    }
                    _ => format!("Failed to open file for writing: {err}"),
                };
                return Err(Error::tool("write", message));
            }
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            asupersync::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::tool("write", format!("Failed to create directories: {e}")))?;
        }

        // Parity with legacy pi-mono: report JS string length (UTF-16 code units) as "bytes".
        let bytes_written = input.content.encode_utf16().count();

        // Write atomically using tempfile on a blocking thread
        let path_clone = path.clone();
        let content_bytes = input.content.into_bytes();
        asupersync::runtime::spawn_blocking_io(move || {
            // Capture original permissions before the file is replaced (new files get None).
            let original_perms = std::fs::metadata(&path_clone).ok().map(|m| m.permissions());
            let parent = path_clone.parent().unwrap_or_else(|| Path::new("."));
            let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;

            temp_file.as_file_mut().write_all(&content_bytes)?;
            super::tolerate_fsync_refusal(
                temp_file.as_file_mut().sync_all(),
                "temp file",
                &path_clone,
            )?;

            // Restore original file permissions (tempfile defaults to 0o600) before persisting.
            if let Some(perms) = original_perms {
                let _ = temp_file.as_file().set_permissions(perms);
            } else {
                // New file: default to 0644 (rw-r--r--) instead of tempfile's 0600.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = temp_file
                        .as_file()
                        .set_permissions(std::fs::Permissions::from_mode(0o644));
                }
            }

            crate::tools::persist_with_readonly_handling(temp_file, &path_clone)?;
            sync_parent_dir(&path_clone)?;
            Ok(())
        })
        .await
        .map_err(|e| Error::tool("write", format!("Failed to write file: {e}")))?;

        let mut details: Option<serde_json::Value> = None;

        // Optional: run file verification after successful write
        if input.verify {
            let verify_path = path.clone();
            match crate::tools::verify::verify_file(verify_path, abort).await {
                Ok(result) => {
                    let mut details_map = serde_json::Map::new();
                    let verify_json = crate::tools::verify::verify_result_to_json(&result);
                    details_map.insert("verify".to_string(), verify_json);
                    details = Some(serde_json::Value::Object(details_map));
                }
                Err(e) => {
                    let mut details_map = serde_json::Map::new();
                    details_map.insert(
                        "verify".to_string(),
                        serde_json::json!({
                            "passed": false,
                            "checker": "verify",
                            "message": format!("Verification error: {e}"),
                        }),
                    );
                    details = Some(serde_json::Value::Object(details_map));
                }
            }
        }

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(format!(
                "Successfully wrote {} bytes to {}",
                bytes_written, input.path
            )))],
            details,
            is_error: false,
        })
    }
}

// ============================================================================
