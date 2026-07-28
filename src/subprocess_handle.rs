//! Long-running subprocess handle for QuickJS extension LSP client support.
//!
//! Provides [`SubprocessHandle`] (manages a single child with stdin write
//! capability and background stdout/stderr pump) and [`SubprocessRegistry`]
//! (maps string keys to handles for lifecycle management).
//!
//! # Architecture
//!
//! ```text
//! JS Extension              Rust Host
//! ───────────              ─────────
//! __pi_spawn_native()  --> SubprocessHandle::spawn()
//!                           ├── spawn child via std::process::Command
//!                           ├── take stdin/stdout/stderr pipes
//!                           ├── spawn pump threads (stdout/stderr → buffer)
//!                           └── register in SubprocessRegistry
//! __pi_spawn_read()    --> SubprocessRegistry::get_mut() → read_output()
//! __pi_stdin_write()   --> SubprocessRegistry::get_mut() → write_stdin()
//! __pi_spawn_kill()    --> SubprocessRegistry::remove()  → kill()
//! ```

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::tools::{ProcessCleanupMode, ProcessGuard};

// ============================================================================
// OutputFrame
// ============================================================================

/// A buffered output chunk from a subprocess pipe.
#[derive(Debug, Clone)]
pub(crate) struct OutputFrame {
    /// `true` if this data came from stderr.
    pub(crate) is_stderr: bool,
    /// Raw bytes from the pipe.
    pub(crate) data: Vec<u8>,
}

// ============================================================================
// SubprocessHandle
// ============================================================================

/// A handle to a long-running subprocess with piped stdin/stdout/stderr.
///
/// Background threads continuously read stdout/stderr and buffer the data
/// in an [`Arc<Mutex<VecDeque>>`].  The JS side can drain this buffer via
/// [`Self::read_output`].
///
/// On drop, the child process is killed (process-group tree kill).
pub(crate) struct SubprocessHandle {
    /// Process guard for lifecycle/cleanup.
    guard: Option<ProcessGuard>,
    /// Writer end of the stdin pipe (taken before wrapping in ProcessGuard).
    stdin_writer: Option<std::process::ChildStdin>,
    /// Background pump threads join handles (kept alive, joined on drop if needed).
    _pump_threads: Vec<std::thread::JoinHandle<()>>,
    /// Shared output buffer written by pump threads, drained by JS.
    output_buffer: Arc<Mutex<VecDeque<OutputFrame>>>,
    /// The child's OS PID.
    pid: u32,
    /// Whether the process has exited.
    exited: Arc<Mutex<bool>>,
}

impl SubprocessHandle {
    /// Spawn a long-running subprocess.
    ///
    /// The process is spawned with stdin/stdout/stderr all piped. Background
    /// pump threads read stdout and stderr into a shared ring buffer.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if spawning fails or a pipe cannot be taken.
    pub(crate) fn spawn(command: &str, args: &[String], cwd: &Path) -> std::io::Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(cwd);

        crate::tools::isolate_command_process_group(&mut cmd);
        let mut child = cmd.spawn()?;
        let pid = child.id();

        // Take pipes *before* wrapping in ProcessGuard.
        let stdin_writer = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("Failed to take stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("Failed to take stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("Failed to take stderr"))?;

        let output_buffer: Arc<Mutex<VecDeque<OutputFrame>>> =
            Arc::new(Mutex::new(VecDeque::new()));
        let exited: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
        let mut pump_threads = Vec::with_capacity(2);

        // stdout pump thread
        let buf_out = Arc::clone(&output_buffer);
        pump_threads.push(std::thread::spawn(move || {
            pump_pipe(stdout, buf_out, false);
        }));

        // stderr pump thread
        let buf_err = Arc::clone(&output_buffer);
        pump_threads.push(std::thread::spawn(move || {
            pump_pipe(stderr, buf_err, true);
        }));

        Ok(Self {
            guard: Some(ProcessGuard::new(
                child,
                ProcessCleanupMode::ProcessGroupTree,
            )),
            stdin_writer: Some(stdin_writer),
            _pump_threads: pump_threads,
            output_buffer,
            pid,
            exited,
        })
    }

    /// Write raw bytes to the subprocess's stdin.
    pub(crate) fn write_stdin(&mut self, data: &[u8]) -> std::io::Result<()> {
        if let Some(writer) = self.stdin_writer.as_mut() {
            writer.write_all(data)?;
            writer.flush()?;
            Ok(())
        } else {
            Err(std::io::Error::other("stdin pipe closed"))
        }
    }

    /// Drain all buffered output and return it as a JSON string.
    ///
    /// Returns `{ "stdout": "...", "stderr": "..." }` with the concatenated
    /// text from each stream.  Fields are empty strings when there is no data.
    pub(crate) fn read_output(&self) -> String {
        let mut buf = self.output_buffer.lock().unwrap();
        let mut stdout = String::new();
        let mut stderr = String::new();

        for frame in buf.drain(..) {
            let text = String::from_utf8_lossy(&frame.data);
            if frame.is_stderr {
                stderr.push_str(&text);
            } else {
                stdout.push_str(&text);
            }
        }

        drop(buf);

        serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
        })
        .to_string()
    }

    /// Kill the subprocess (process-group tree kill).
    pub(crate) fn kill(&mut self) {
        if let Some(guard) = self.guard.as_mut() {
            guard.kill();
        }
        self.stdin_writer = None;
        *self.exited.lock().unwrap() = true;
    }

    /// Return the child's OS PID.
    pub(crate) const fn pid(&self) -> u32 {
        self.pid
    }

    /// Non-blocking check whether the child has exited.
    pub(crate) fn try_wait(&mut self) -> std::io::Result<Option<i32>> {
        self.guard.as_mut().map_or(Ok(None), |guard| {
            guard.try_wait_child().map(|opt| {
                if let Some(status) = opt {
                    *self.exited.lock().unwrap() = true;
                    Some(status.code().unwrap_or(-1))
                } else {
                    None
                }
            })
        })
    }

    /// Whether the process has exited (from try_wait or kill).
    pub(crate) fn has_exited(&self) -> bool {
        *self.exited.lock().unwrap()
    }
}

impl Drop for SubprocessHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

// ============================================================================
// SubprocessRegistry
// ============================================================================

/// Maps string keys to active [`SubprocessHandle`]s.
///
/// The registry is stored on [`PiJsRuntime`](crate::extensions_js::PiJsRuntime)
/// and is used by the spawn/read/write/kill hostcalls to locate handles.
#[derive(Default)]
pub(crate) struct SubprocessRegistry {
    handles: HashMap<String, SubprocessHandle>,
}

impl SubprocessRegistry {
    pub(crate) fn new() -> Self {
        Self {
            handles: HashMap::new(),
        }
    }

    /// Register a handle under the given key.  If a handle already exists
    /// under that key it is killed and replaced.
    pub(crate) fn register(&mut self, key: String, handle: SubprocessHandle) {
        if let Some(mut old) = self.handles.insert(key, handle) {
            old.kill();
        }
    }

    /// Get a mutable reference to a registered handle.
    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut SubprocessHandle> {
        self.handles.get_mut(key)
    }

    /// Remove and return a registered handle (killing it).
    pub(crate) fn remove(&mut self, key: &str) -> Option<SubprocessHandle> {
        self.handles.remove(key)
    }

    /// Number of active handles.
    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.handles.len()
    }

    /// Kill and remove all registered subprocesses.
    pub(crate) fn kill_all(&mut self) {
        for (_key, mut handle) in self.handles.drain() {
            handle.kill();
        }
    }
}

// ============================================================================
// Pump helpers
// ============================================================================

/// Read from a pipe in a background thread and buffer all data.
#[allow(clippy::needless_pass_by_value, clippy::needless_continue)]
fn pump_pipe(
    mut reader: impl Read + Send + 'static,
    buffer: Arc<Mutex<VecDeque<OutputFrame>>>,
    is_stderr: bool,
) {
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let mut guard = buffer.lock().unwrap();
                guard.push_back(OutputFrame {
                    is_stderr,
                    data: buf[..n].to_vec(),
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                // Interrupted, retry
            }
            Err(_) => {
                // Pipe error – no point continuing.
                return;
            }
        }
    }
}
