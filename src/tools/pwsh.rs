use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use asupersync::time::{sleep, wall_now};
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
impl Tool for PwshTool {
    fn name(&self) -> &str {
        "pwsh"
    }
    fn label(&self) -> &str {
        "pwsh"
    }
    fn description(&self) -> &str {
        "在 Windows 上通过 pwsh 执行 PowerShell 命令。支持文件列表、文本处理和系统信息 — 与终端行为一致，但将输出作为文本返回。可用于任何 shell 操作。"
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
        tool_call_id: &str,
        input: serde_json::Value,
        on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
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

pub(crate) async fn run_pwsh_command(
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

    // Read output in blocking threads
    let (tx, rx) = std::sync::mpsc::channel::<PwshPipeFrame>();
    let tx_out = tx.clone();
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match stdout.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                }
                Err(_) => break,
            }
        }
        let _ = tx_out.send(PwshPipeFrame::Output(buf));
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match stderr.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(PwshPipeFrame::Stderr(buf));
    });

    // Wait for child with timeout
    let start = std::time::Instant::now();
    let exit_code = loop {
        let remaining = timeout_secs
            .map(|s| s.saturating_sub(start.elapsed().as_secs()))
            .unwrap_or(u64::MAX);
        if remaining == 0 {
            let _ = child.kill();
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status_code(&status)),
            Ok(None) => {
                // Wait a bit before polling again
            }
            Err(_) => break None,
        }
        // Use a short sleep instead of busy-wait
        let now = wall_now();
        if remaining > 0 && remaining < 10 {
            sleep(now, std::time::Duration::from_millis(50)).await;
        } else {
            sleep(now, std::time::Duration::from_millis(100)).await;
        }
    };

    // Wait for I/O threads
    stdout_thread.join().ok();
    stderr_thread.join().ok();

    // Collect output
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        match frame {
            PwshPipeFrame::Output(b) => stdout_buf = b,
            PwshPipeFrame::Stderr(b) => stderr_buf = b,
        }
    }

    let output = String::from_utf8_lossy(&stdout_buf).to_string();
    let exit_code = exit_code.unwrap_or(-1);

    Ok(PwshRunResult {
        output,
        exit_code,
        timeout_ms: timeout_secs.map(|s| s * 1000),
        truncated: false,
        full_output_path: None,
        truncation: None,
    })
}

fn status_code(status: &std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
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
