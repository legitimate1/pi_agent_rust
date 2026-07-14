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
        "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories."
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
        let path = enforce_cwd_scope(&path, &self.cwd, "write")?;

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
            temp_file.as_file_mut().sync_all()?;

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

            // Persist (atomic rename)
            temp_file.persist(&path_clone).map_err(|e| e.error)?;
            sync_parent_dir(&path_clone)?;
            Ok(())
        })
        .await
        .map_err(|e| Error::tool("write", format!("Failed to write file: {e}")))?;

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(format!(
                "Successfully wrote {} bytes to {}",
                bytes_written, input.path
            )))],
            details: None,
            is_error: false,
        })
    }
}

// ============================================================================
