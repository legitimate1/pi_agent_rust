use super::*;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use asupersync::time::{sleep, wall_now};
use async_trait::async_trait;
use serde::Deserialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
// ============================================================================
// Bash Tool
// ============================================================================

/// Input parameters for the bash tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BashInput {
    command: String,
    timeout: Option<u64>,
}

pub struct BashTool {
    cwd: PathBuf,
    shell_path: Option<String>,
    command_prefix: Option<String>,
    artifact_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BashRunResult {
    pub output: String,
    pub exit_code: i32,
    pub cancelled: bool,
    pub cancellation_reason: Option<BashCancellationReason>,
    pub timeout_ms: Option<u64>,
    pub truncated: bool,
    pub full_output_path: Option<String>,
    pub truncation: Option<TruncationResult>,
}

#[derive(Debug)]
pub enum BashPipeFrame {
    Chunk(Vec<u8>),
    Error(String),
}

#[allow(clippy::unnecessary_lazy_evaluations)] // lazy eval needed on unix for signal()
fn exit_status_code(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or_else(|| {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt as _;
            status.signal().map_or(-1, |signal| -signal)
        }
        #[cfg(not(unix))]
        {
            -1
        }
    })
}

fn bash_cancellation_details(
    reason: BashCancellationReason,
    timeout_ms: Option<u64>,
    exit_code: i32,
) -> serde_json::Value {
    serde_json::json!({
        "schema": BASH_CANCELLATION_SCHEMA_V1,
        "status": "cancelled",
        "reason": reason.as_str(),
        "cleanup": "process_group_tree_terminated",
        "exitCode": exit_code,
        "timeoutMs": timeout_ms,
    })
}

#[allow(clippy::too_many_lines)]
pub async fn run_bash_command(
    cwd: &Path,
    shell_path: Option<&str>,
    command_prefix: Option<&str>,
    command: &str,
    timeout_secs: Option<u64>,
    on_update: Option<&(dyn Fn(ToolUpdate) + Send + Sync)>,
) -> Result<BashRunResult> {
    let timeout_secs = match timeout_secs {
        None => Some(DEFAULT_BASH_TIMEOUT_SECS),
        Some(0) => None,
        Some(value) => Some(value),
    };
    let command = command_prefix.filter(|p| !p.trim().is_empty()).map_or_else(
        || command.to_string(),
        |prefix| format!("{prefix}\n{command}"),
    );
    let command = format!("trap 'code=$?; wait; exit $code' EXIT\n{command}");

    if !cwd.exists() {
        return Err(Error::tool(
            "bash",
            format!(
                "Working directory does not exist: {}\nCannot execute bash commands.",
                cwd.display()
            ),
        ));
    }

    let shell = shell_path.unwrap_or_else(|| {
        for path in ["/bin/bash", "/usr/bin/bash", "/usr/local/bin/bash"] {
            if Path::new(path).exists() {
                return path;
            }
        }
        "sh"
    });

    let mut cmd = command_with_default_sigpipe_in_dir(shell, cwd)
        .map_err(|e| Error::tool("bash", format!("Failed to prepare shell: {e}")))?;
    cmd.arg("-c")
        .arg(&command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Place the shell in its own process group so background children
    // can be killed reliably even if the shell exits first.
    isolate_command_process_group(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::tool("bash", format!("Failed to spawn shell: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::tool("bash", "Missing stdout".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::tool("bash", "Missing stderr".to_string()))?;

    // Wrap in ProcessGuard for cleanup (including tree kill)
    let mut guard = ProcessGuard::new(child, ProcessCleanupMode::ProcessGroupTree);

    // We use a bounded channel to provide backpressure. If the child process
    // produces output faster than the async loop can drain it (and spill to disk),
    // the pump threads will block on send(), which stops them from reading from the OS pipe.
    // The OS pipe buffer will fill up, causing the child's `write()` calls to block.
    // This correctly pauses the child until we catch up, preventing unbounded memory growth (OOM).
    let (tx, rx) = mpsc::sync_channel::<BashPipeFrame>(1024);
    let tx_stdout = tx.clone();

    // Design Decision (bd-xdcrh.4.3):
    // We intentionally use raw dedicated OS threads here rather than `asupersync::runtime::spawn_blocking`.
    // The `pump_stream` loop blocks indefinitely on `read()` until the subprocess closes the pipe (EOF).
    // If we used the runtime's blocking pool, concurrently running long-lived bash tools (like compilers
    // or servers) could easily exhaust the pool's thread limit, starving the rest of the application
    // of threads needed for short-lived blocking I/O (e.g., SQLite transactions or filesystem metadata).
    // Dedicated threads cleanly isolate this unbounded blocking risk.
    let stdout_thread = thread::spawn(move || pump_stream(stdout, "stdout", &tx_stdout));
    let stderr_thread = thread::spawn(move || pump_stream(stderr, "stderr", &tx));

    let max_chunks_bytes = DEFAULT_MAX_BYTES.saturating_mul(2);
    let mut bash_output = BashOutputState::new(max_chunks_bytes);
    bash_output.timeout_ms = timeout_secs.map(|s| s.saturating_mul(1000));

    let cx = AgentCx::for_current_or_request();
    let mut timed_out = false;
    let mut cancelled = false;
    let mut cancellation_reason: Option<BashCancellationReason> = None;
    let mut exit_code: Option<i32> = None;
    let start = cx
        .cx()
        .timer_driver()
        .map_or_else(wall_now, |timer| timer.now());
    let timeout = timeout_secs.map(Duration::from_secs);
    let mut terminate_deadline: Option<asupersync::Time> = None;

    let tick = Duration::from_millis(10);
    loop {
        let mut updated = false;
        while let Ok(frame) = rx.try_recv() {
            if let Err(err) = ingest_bash_pipe_frame(frame, &mut bash_output).await {
                let _ = guard.kill();
                return Err(err);
            }
            updated = true;
        }

        if updated {
            emit_bash_update(&bash_output, on_update)?;
        }

        match guard.try_wait_child() {
            Ok(Some(status)) => {
                exit_code = Some(exit_status_code(status));
                break;
            }
            Ok(None) => {}
            Err(err) => return Err(Error::tool("bash", err.to_string())),
        }

        let now = cx
            .cx()
            .timer_driver()
            .map_or_else(wall_now, |timer| timer.now());

        if let Some(deadline) = terminate_deadline {
            if now >= deadline {
                if let Some(status) = guard.kill() {
                    exit_code = Some(exit_status_code(status));
                }
                break; // Guard now owns no child after kill()
            }
        } else if let Some(timeout) = timeout {
            let elapsed = std::time::Duration::from_nanos(now.duration_since(start));
            if elapsed >= timeout {
                timed_out = true;
                cancellation_reason = Some(BashCancellationReason::Timeout);
                let pid = guard.child.as_ref().map(std::process::Child::id);
                terminate_process_group_tree(pid);
                terminate_deadline = Some(now + Duration::from_secs(BASH_TERMINATE_GRACE_SECS));
            }
        }

        if terminate_deadline.is_none() && cx.checkpoint().is_err() {
            cancelled = true;
            cancellation_reason = Some(BashCancellationReason::AmbientCancellation);
            let _ = guard.kill();
            exit_code = Some(-1);
            break;
        }

        sleep(now, tick).await;
    }

    // Drain any remaining channel frames while waiting for the pump threads
    // to observe EOF and exit. Because the channel is bounded, they may still
    // be blocked on send() until we consume the buffered output after the child
    // closes its pipe ends. The 5-second cap is a safety net for pathological
    // cases (e.g. the child spawned a grandchild that inherited the pipe fd
    // and is still running).
    {
        let drain_start = cx
            .cx()
            .timer_driver()
            .map_or_else(wall_now, |timer| timer.now());
        let drain_deadline = drain_start + Duration::from_secs(5);
        let allow_drain_cancellation = !cancelled && !timed_out && exit_code.is_none();
        loop {
            // Drain everything currently available in the channel.
            let mut got_data = false;
            while let Ok(frame) = rx.try_recv() {
                if let Err(err) = ingest_bash_pipe_frame(frame, &mut bash_output).await {
                    let _ = guard.kill();
                    return Err(err);
                }
                got_data = true;
            }
            if got_data {
                emit_bash_update(&bash_output, on_update)?;
            }

            // If both pump threads have finished, all data is in the channel
            // and we've drained it above, so we're done.
            if stdout_thread.is_finished() && stderr_thread.is_finished() {
                // One final drain in case they sent items between our last
                // try_recv loop and the is_finished check.
                while let Ok(frame) = rx.try_recv() {
                    if let Err(err) = ingest_bash_pipe_frame(frame, &mut bash_output).await {
                        let _ = guard.kill();
                        return Err(err);
                    }
                }
                break;
            }

            let now = cx
                .cx()
                .timer_driver()
                .map_or_else(wall_now, |timer| timer.now());
            if now >= drain_deadline {
                break;
            }
            if allow_drain_cancellation && cx.checkpoint().is_err() {
                cancelled = true;
                cancellation_reason.get_or_insert(BashCancellationReason::AmbientCancellation);
                break;
            }
            sleep(now, tick).await;
        }
    }

    // Explicitly reap the child process to prevent zombies. try_wait_child()
    // uses WNOHANG which *should* reap the zombie on the first successful
    // return, but calling wait() as a belt-and-suspenders ensures the zombie
    // is cleaned up even if try_wait missed it (observed on macOS when the
    // child is in its own process group).
    if guard.child.is_some() {
        if let Ok(status) = guard.wait() {
            exit_code.get_or_insert_with(|| exit_status_code(status));
        }
    }

    drop(bash_output.temp_file.take());

    let raw_output = concat_chunks(&bash_output.chunks);
    let full_output = String::from_utf8_lossy(&raw_output).into_owned();
    let full_output_last_line_len = full_output.split('\n').next_back().map_or(0, str::len);

    let mut truncation = truncate_tail(full_output, DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);
    if bash_output.total_bytes > bash_output.chunks_bytes {
        truncation.truncated = true;
        truncation.truncated_by = Some(TruncatedBy::Bytes);
        truncation.total_bytes = bash_output.total_bytes;
        truncation.total_lines = line_count_from_newline_count(
            bash_output.total_bytes,
            bash_output.line_count,
            bash_output.last_byte_was_newline,
        );
    }

    let mut output_text = if truncation.content.is_empty() {
        "(no output)".to_string()
    } else {
        std::mem::take(&mut truncation.content)
    };

    let mut full_output_path = None;
    if truncation.truncated {
        if let Some(path) = bash_output.temp_file_path.as_ref() {
            full_output_path = Some(path.display().to_string());
        }

        let start_line = truncation
            .total_lines
            .saturating_sub(truncation.output_lines)
            .saturating_add(1);
        let end_line = truncation.total_lines;

        let display_path = full_output_path.as_deref().unwrap_or("undefined");
        let file_limit_hit = bash_output.total_bytes > BASH_FILE_LIMIT_BYTES;
        let output_qualifier = if file_limit_hit {
            format!(
                "Partial output (capped at {})",
                format_size(BASH_FILE_LIMIT_BYTES)
            )
        } else {
            "Full output".to_string()
        };

        if truncation.last_line_partial {
            let last_line_size = format_size(full_output_last_line_len);
            let _ = write!(
                output_text,
                "\n\n[Showing last {} of line {end_line} (line is {last_line_size}). {output_qualifier}: {display_path}]",
                format_size(truncation.output_bytes)
            );
        } else if truncation.truncated_by == Some(TruncatedBy::Lines) {
            let _ = write!(
                output_text,
                "\n\n[Showing lines {start_line}-{end_line} of {}. {output_qualifier}: {display_path}]",
                truncation.total_lines
            );
        } else {
            let _ = write!(
                output_text,
                "\n\n[Showing lines {start_line}-{end_line} of {} ({} limit). {output_qualifier}: {display_path}]",
                truncation.total_lines,
                format_size(DEFAULT_MAX_BYTES)
            );
        }
    }

    if timed_out {
        cancelled = true;
        if !output_text.is_empty() {
            output_text.push_str("\n\n");
        }
        let timeout_display = timeout_secs.unwrap_or(0);
        let _ = write!(
            output_text,
            "Command timed out after {timeout_display} seconds"
        );
    }

    let exit_code = exit_code.unwrap_or(-1);
    if !cancelled && exit_code != 0 {
        let _ = write!(output_text, "\n\nCommand exited with code {exit_code}");
    }

    Ok(BashRunResult {
        output: output_text,
        exit_code,
        cancelled,
        cancellation_reason,
        timeout_ms: timeout_secs.map(|s| s.saturating_mul(1000)),
        truncated: truncation.truncated,
        full_output_path,
        truncation: if truncation.truncated {
            Some(truncation)
        } else {
            None
        },
    })
}

impl BashTool {
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            shell_path: None,
            command_prefix: None,
            artifact_root: None,
        }
    }

    pub fn with_shell(
        cwd: &Path,
        shell_path: Option<String>,
        command_prefix: Option<String>,
    ) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            shell_path,
            command_prefix,
            artifact_root: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_artifact_root(cwd: &Path, artifact_root: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            shell_path: None,
            command_prefix: None,
            artifact_root: Some(artifact_root.to_path_buf()),
        }
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }
    fn label(&self) -> &str {
        "bash"
    }
    fn description(&self) -> &str {
        "在当前工作目录执行 bash 命令。返回 stdout 和 stderr。输出截断为最后 2000 行或 1MB（以先到者为准）。截断时完整输出会保存到临时文件。`timeout` 默认 120 秒；设为 `timeout: 0` 可禁用超时。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Bash command to execute"
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

    #[allow(clippy::too_many_lines)]
    async fn execute(
        &self,
        tool_call_id: &str,
        input: serde_json::Value,
        on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
    ) -> Result<ToolOutput> {
        let input: BashInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        let result = run_bash_command(
            &self.cwd,
            self.shell_path.as_deref(),
            self.command_prefix.as_deref(),
            &input.command,
            input.timeout,
            on_update.as_deref(),
        )
        .await?;

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
        if let Some(reason) = result.cancellation_reason {
            details_map.insert(
                "cancellation".to_string(),
                bash_cancellation_details(reason, result.timeout_ms, result.exit_code),
            );
        }

        let details = if details_map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(details_map))
        };
        let mut details = details;
        let mut output_text = result.output;

        if let Some(path) = result.full_output_path.as_deref() {
            attach_text_artifact_from_path_if_needed_with_root(
                self.artifact_root.as_deref(),
                &mut output_text,
                &mut details,
                "bash",
                tool_call_id,
                "fullCommandOutput",
                Path::new(path),
            );
        }

        let is_error = result.cancelled || result.exit_code != 0;

        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(output_text))],
            details,
            is_error,
        })
    }
}

// ============================================================================
