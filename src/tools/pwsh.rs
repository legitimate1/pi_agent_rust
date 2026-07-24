use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use crate::tools::{ProcessCleanupMode, ProcessGuard};
use async_trait::async_trait;
use serde::Deserialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
// ============================================================================
// Pwsh Tool
// ============================================================================

#[derive(Debug, Deserialize)]
struct PwshInput {
    command: String,
    timeout: Option<u64>,
}

pub struct PwshTool {
    cwd: PathBuf,
}

impl PwshTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
        }
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for PwshTool {
    fn name(&self) -> &str {
        "pwsh"
    }
    fn label(&self) -> &str {
        "pwsh"
    }
    fn description(&self) -> &str {
        "在 Windows 上通过 pwsh 执行 PowerShell 命令。支持文件列表、文本处理和系统信息 — 与终端行为一致，但将输出作为文本返回。可用于任何 shell 操作。输出截断为最后 2000 行或 1MB（以先到者为准）。截断时完整输出会保存到临时文件。`timeout` 默认 120 秒；设为 `timeout: 0` 可禁用超时。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "PowerShell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 120; set 0 to disable)"
                }
            },
            "required": ["command"]
        })
    }

    fn effects(&self) -> ToolEffects {
        ToolEffects::process().union(ToolEffects::write())
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        input: serde_json::Value,
        _on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let input: PwshInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        let result = run_pwsh_command(&self.cwd, &input.command, input.timeout).await?;

        let mut details_map = serde_json::Map::new();
        if let Some(truncation) = result.truncation.as_ref() {
            details_map.insert("truncation".to_string(), serde_json::to_value(truncation)?);
        }
        if let Some(path) = result.full_output_path.as_ref() {
            details_map.insert(
                "fullOutputPath".to_string(),
                serde_json::Value::String(path.clone()),
            );
        }

        let details = if details_map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(details_map))
        };

        let is_error = result.exit_code != 0;

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(result.output))],
            details,
            is_error,
        })
    }
}

#[allow(clippy::too_many_lines)]
pub async fn run_pwsh_command(
    cwd: &Path,
    command: &str,
    timeout_secs: Option<u64>,
) -> Result<PwshRunResult> {
    let timeout_secs = match timeout_secs {
        None => Some(DEFAULT_BASH_TIMEOUT_SECS),
        Some(0) => None,
        Some(value) => Some(value),
    };

    if !cwd.exists() {
        return Err(Error::tool(
            "pwsh",
            format!(
                "Working directory does not exist: {}\nCannot execute pwsh commands.",
                cwd.display()
            ),
        ));
    }

    // Wrap command to ensure UTF-8 output encoding for proper Chinese character support
    let pwsh_command = format!(
        "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new(); $PSDefaultParameterOutputEncoding = [System.Text.UTF8Encoding]::new(); {command}"
    );

    let mut cmd = std::process::Command::new("pwsh");
    cmd.arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(&pwsh_command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        Error::tool(
            "pwsh",
            format!("Failed to spawn pwsh: {e}. Is PowerShell 7 installed?"),
        )
    })?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::tool("pwsh", "Missing stdout".to_string()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::tool("pwsh", "Missing stderr".to_string()))?;

    // Wrap child in ProcessGuard for automatic cleanup on drop/cancellation
    let mut guard = ProcessGuard::new(child, ProcessCleanupMode::ChildOnly);

    // Read output in blocking threads
    let (tx, rx) = std::sync::mpsc::channel::<PwshPipeFrame>();
    let tx_out = tx.clone();
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match stdout.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                }
            }
        }
        let _ = tx_out.send(PwshPipeFrame::Output(buf));
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match stderr.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                }
            }
        }
        let _ = tx.send(PwshPipeFrame::Stderr(buf));
    });

    // Wait for child with timeout and ambient cancellation support
    let exit_code = guard
        .wait_with_cancellation(timeout_secs)
        .await
        .ok()
        .flatten();

    // Wait for I/O threads
    stdout_thread.join().ok();
    stderr_thread.join().ok();

    // Collect both stdout and stderr
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        match frame {
            PwshPipeFrame::Output(b) => stdout_buf = b,
            PwshPipeFrame::Stderr(b) => stderr_buf = b,
        }
    }

    let exit_code = exit_code.unwrap_or(-1);

    // Build output: stdout always, stderr only on non-zero exit
    let mut output = String::from_utf8_lossy(&stdout_buf).to_string();
    if exit_code != 0 {
        let stderr = String::from_utf8_lossy(&stderr_buf);
        let stderr_trimmed = stderr.trim();
        if !stderr_trimmed.is_empty() {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(stderr_trimmed);
        }
    }

    // Save full output before truncation (for temp file if needed)
    let full_output = output.clone();
    let truncation = truncate_tail(output, DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);
    let mut output_text = if truncation.content.is_empty() {
        "(no output)".to_string()
    } else {
        truncation.content.clone()
    };

    let truncated = truncation.truncated;
    let mut full_output_path: Option<String> = None;

    if truncation.truncated {
        // Write full output to temp file for later inspection
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file_name = format!("pwsh-{ts}.txt");
        let temp_path = std::env::temp_dir().join(file_name);
        if std::fs::write(&temp_path, &full_output).is_ok() {
            full_output_path = Some(temp_path.to_string_lossy().to_string());
        }

        // Append truncation marker
        let start_line = truncation
            .total_lines
            .saturating_sub(truncation.output_lines)
            .saturating_add(1);
        let end_line = truncation.total_lines;
        let display_path = full_output_path
            .as_deref()
            .unwrap_or("(temp file unavailable)");
        let _ = write!(
            output_text,
            "\n\n[Showing lines {start_line}-{end_line} of {}. Full output: {display_path}]",
            truncation.total_lines,
        );
    }

    Ok(PwshRunResult {
        output: output_text,
        exit_code,
        timeout_ms: timeout_secs.map(|s| s * 1000),
        truncated,
        full_output_path,
        truncation: if truncated { Some(truncation) } else { None },
    })
}

enum PwshPipeFrame {
    Output(Vec<u8>),
    Stderr(Vec<u8>),
}

/// Result of running a pwsh command.
#[derive(Debug, Clone)]
pub struct PwshRunResult {
    pub output: String,
    pub exit_code: i32,
    pub timeout_ms: Option<u64>,
    pub truncated: bool,
    pub full_output_path: Option<String>,
    pub truncation: Option<TruncationResult>,
}

// ============================================================================
