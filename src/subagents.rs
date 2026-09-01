//! Native child-agent orchestration with continuable sessions.
//!
//! This module deliberately uses the Pi executable that is already running
//! (or the explicit `PI_SUBAGENT_PI_BINARY` override) instead of resolving a
//! `pi` binary through `PATH`.  That makes a Rust Pi parent reliably launch
//! Rust Pi children even on hosts that also have the TypeScript implementation
//! installed.
//!
//! Continuable subagents (C1): `hubId == sessionId == worktreeId`. A child
//! session is persisted at `<global_dir>/sessions/subagents/<hubId>.jsonl` and
//! is resumed by re-launching the child with `--session <path>` so the next
//! `task` is appended as a `user` message to the same session. New calls
//! generate a `hubId` and a persistent session path; `continue=true` calls
//! validate `hubId`+`task` and reuse the existing session file.

use crate::agent_cx::AgentCx;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::model::{ContentBlock, TextContent};
use crate::tools::{Tool, ToolEffects, ToolOutput, ToolUpdate};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

const MAX_PARALLEL_TASKS: usize = 8;
const DEFAULT_CONCURRENCY: usize = 4;
const MAX_SUBAGENT_DEPTH: usize = 3;
const MAX_CHILD_OUTPUT_BYTES: usize = 256 * 1024;
/// v2 (bd-cv653.5.1) extends v1 additively with `data`, `schemaValid`,
/// `validationErrors`, and `schemaRetries` on schema-bearing results; every
/// v1 field is unchanged, so v1 consumers keep working.
const SUBAGENT_RESULT_SCHEMA: &str = "pi.subagent.result.v2";
const SUBAGENT_PROGRESS_SCHEMA: &str = "pi.subagent.progress.v1";
/// Per-field byte budget for `output`/`error` in the opt-in structured block.
const STRUCTURED_FIELD_LIMIT_BYTES: usize = 2 * 1024;
/// Byte budget for the JSON payload of the opt-in structured block.
const STRUCTURED_BLOCK_LIMIT_BYTES: usize = 16 * 1024;
const STRUCTURED_BLOCK_OPEN: &str = "<subagent-structured-result>";
const STRUCTURED_BLOCK_CLOSE: &str = "</subagent-structured-result>";
const STRUCTURED_TRUNCATION_MARKER: &str = "…[truncated]";
const DEFAULT_CHILD_TOOLS: &str = "read,bash,edit,write,grep,find,ls,hashline_edit";
const TAN_RESULT_SCHEMA: &str = "pi.background-tan.result.v1";
const TAN_AGENT_NAME: &str = "tan";
const TAN_SYSTEM_PROMPT: &str = "You are a background tangential coding agent. Complete the assigned work autonomously in the current working directory. Keep your final response concise and lead with the concrete outcome, changed files, and verification performed. Do not ask follow-up questions.";

/// Hub session file layout: `<global_dir>/sessions/subagents/<hubId>.jsonl`.
///
/// `hubId == sessionId == worktreeId` — the registry id, session file id and
/// worktree dir name are the same value so `continue` can reopen both.
fn subagent_session_path(global_dir: &Path, hub_id: &str) -> PathBuf {
    global_dir
        .join("sessions")
        .join("subagents")
        .join(format!("{hub_id}.jsonl"))
}

/// Validate a hub/session id for filesystem safety.
fn is_valid_hub_id(hub_id: &str) -> bool {
    let trimmed = hub_id.trim();
    !trimmed.is_empty()
        && !trimmed.contains('/')
        && !trimmed.contains('\\')
        && !trimmed.contains('\0')
        && trimmed != "."
        && trimmed != ".."
}

type UpdateCallback = Arc<dyn Fn(ToolUpdate) + Send + Sync>;

/// Settled result from an interactive `/tan` background child.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TanCompletion {
    pub schema: &'static str,
    pub hub_id: Option<String>,
    pub task: String,
    pub status: String,
    pub output: String,
    pub error: Option<String>,
    pub is_error: bool,
}

impl TanCompletion {
    fn from_result(result: SubagentResult) -> Self {
        Self {
            schema: TAN_RESULT_SCHEMA,
            hub_id: result.hub_id,
            task: result.task,
            status: result.status.as_str().to_string(),
            output: result.output,
            error: result.error,
            is_error: result.is_error,
        }
    }

    /// Follow-up content delivered to the parent agent at its next idle turn
    /// boundary. The bounded excerpt prevents a verbose child from consuming
    /// the entire next-turn context.
    #[must_use]
    pub fn follow_up_text(&self) -> String {
        let summary_source = if self.output.trim().is_empty() {
            self.error.as_deref().unwrap_or("(no output)")
        } else {
            self.output.trim()
        };
        let summary = truncated_field(summary_source, 16 * 1024);
        format!(
            "[background tan {} settled: {}]\nwork: {}\nsummary:\n{}",
            self.hub_id.as_deref().unwrap_or("unregistered"),
            self.status,
            self.task,
            summary
        )
    }

    /// Display-only completion card for the interactive transcript. The
    /// authoritative model delivery is [`Self::follow_up_text`].
    #[must_use]
    pub fn card_text(&self) -> String {
        let marker = if self.is_error { "failed" } else { "completed" };
        format!("(/tan {marker})\n{}", self.follow_up_text())
    }
}

/// A native tool that delegates bounded work to isolated Pi child processes.
#[derive(Debug, Clone)]
pub(crate) struct SubagentRetryConfig {
    enabled: bool,
    max_retries: u32,
    base_delay_ms: u32,
    max_delay_ms: u32,
}

impl Default for SubagentRetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            base_delay_ms: 2000,
            max_delay_ms: 60000,
        }
    }
}

impl From<&Config> for SubagentRetryConfig {
    fn from(config: &Config) -> Self {
        Self {
            enabled: config.retry_enabled(),
            max_retries: config.retry_max_retries(),
            base_delay_ms: config.retry_base_delay_ms(),
            max_delay_ms: config.retry_max_delay_ms(),
        }
    }
}

impl SubagentRetryConfig {
    fn delay_ms(&self, attempt: u32) -> u32 {
        let base = u64::from(self.base_delay_ms);
        let max = u64::from(self.max_delay_ms);
        let shift = attempt.saturating_sub(1);
        let multiplier = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
        let delay = base.saturating_mul(multiplier).min(max);
        u32::try_from(delay).unwrap_or(u32::MAX)
    }
}

pub struct SubagentTool {
    cwd: PathBuf,
    global_dir: PathBuf,
    child_binary: PathBuf,
    structured_results: bool,
    /// Model spec children run with when their agent definition does not pin
    /// `model:` — the `task` role spec, else `smol` (bd-cv653.3.1).
    role_model_spec: Option<String>,
    retry_config: SubagentRetryConfig,
}

impl SubagentTool {
    #[must_use]
    pub fn new(cwd: &Path) -> Self {
        let child_binary = std::env::var_os("PI_SUBAGENT_PI_BINARY")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())
            .unwrap_or_else(|| PathBuf::from("<current executable unavailable>"));
        let retry_config = Config::load()
            .map(|c| SubagentRetryConfig::from(&c))
            .unwrap_or_default();
        Self {
            cwd: cwd.to_path_buf(),
            global_dir: Config::global_dir(),
            child_binary,
            structured_results: false,
            role_model_spec: None,
            retry_config,
        }
    }

    /// Override retry config (used by `ToolRegistry` and tests).
    #[must_use]
    pub(crate) fn with_retry_config(mut self, config: SubagentRetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Set the role model spec children fall back to when their agent
    /// definition has no `model:` pin (task role, else smol).
    #[must_use]
    pub fn with_role_model_spec(mut self, spec: Option<String>) -> Self {
        self.role_model_spec = spec.filter(|s| !s.trim().is_empty());
        self
    }

    /// Opt in to appending the machine-readable
    /// `<subagent-structured-result>` JSON block to the tool result text.
    ///
    /// Off by default; when disabled the tool output is byte-identical to
    /// previous releases. See pi_agent_rust#163.
    #[must_use]
    pub const fn with_structured_results(mut self, enabled: bool) -> Self {
        self.structured_results = enabled;
        self
    }

    /// Run one built-in tangential child in the current working directory.
    ///
    /// This uses the same process, task-role, cancellation, transcript, and
    /// hub machinery as the opt-in `subagent` tool while avoiding a required
    /// user-authored agent definition for the `/tan` host command.
    pub async fn run_background_tan(&self, task: &str) -> Result<TanCompletion> {
        if current_subagent_depth() >= MAX_SUBAGENT_DEPTH {
            return Err(Error::tool(
                "subagent",
                format!(
                    "Refusing nested subagent depth above {MAX_SUBAGENT_DEPTH}; child agents are isolated by default and do not receive the subagent tool."
                ),
            ));
        }
        let task = task.trim();
        if task.is_empty() {
            return Err(Error::validation("/tan requires non-empty work"));
        }

        let definition = tan_agent_definition();
        let agents = BTreeMap::from([(TAN_AGENT_NAME.to_string(), definition)]);
        let request = SubagentTask {
            agent: TAN_AGENT_NAME.to_string(),
            task: task.to_string(),
            cwd: Some(self.cwd.clone()),
            isolation: None,
            iso_apply: None,
            output_schema: None,
            schema_mode: SchemaMode::Permissive,
            continue_flag: None,
            hub_id: None,
        };
        let result = ChildRunner::new(
            self.cwd.clone(),
            self.global_dir.clone(),
            self.child_binary.clone(),
            self.role_model_spec.clone(),
            crate::agent_hub::ChildKind::Tan,
        )
        .with_retry_config(self.retry_config.clone())
        .run_one(&agents, request, None, None)
        .await;
        Ok(TanCompletion::from_result(result))
    }

    /// Construct with explicit discovery and child-runtime paths.
    ///
    /// This seam is intentionally narrow: embedders and hermetic conformance
    /// tests can exercise the real child protocol without consulting the
    /// process-global agent directory or current executable.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_paths(cwd: PathBuf, global_dir: PathBuf, child_binary: PathBuf) -> Self {
        Self {
            cwd,
            global_dir,
            child_binary,
            structured_results: false,
            role_model_spec: None,
            retry_config: SubagentRetryConfig {
                enabled: true,
                max_retries: 3,
                base_delay_ms: 2000,
                max_delay_ms: 60000,
            },
        }
    }

    /// Test-only helper to inject a deterministic retry schedule.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_retry_config_for_test(mut self, config: SubagentRetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    fn discover(&self, scope: AgentScope) -> Result<BTreeMap<String, AgentDefinition>> {
        discover_agents_with_roots(&self.cwd, &self.global_dir, scope)
    }

    async fn run_request(
        &self,
        request: SubagentRequest,
        on_update: Option<UpdateCallback>,
    ) -> Result<Vec<SubagentResult>> {
        let agents = self.discover(request.scope)?;
        // C1 continuable single: `continue=true` singles do not mix with
        // parallel/chain, and must carry hubId+task contract.
        if request.is_continuation() {
            request.validate_continuation()?;
            if request.tasks.is_some() || request.chain.is_some() {
                return Err(Error::tool(
                    "subagent",
                    "continue=true is only valid for single agent+task delegation, not for tasks/chain.",
                ));
            }
            let task = request.mode()?.into_single().ok_or_else(|| {
                Error::tool(
                    "subagent",
                    "continue=true requires agent+task single delegation.",
                )
            })?;
            return Ok(vec![self.run_one(&agents, task, None, on_update).await]);
        }
        // Guard: a single task requesting continue must also pass its own contract.
        // Fail closed so hubId formatting bugs do not become unbounded reuses.
        if request.tasks.is_none()
            && request.chain.is_none()
            && let Some(agent) = request.agent.as_deref()
            && !agent.trim().is_empty()
            && let Some(task) = request.task.as_deref()
            && !task.trim().is_empty()
        {
            let provisional = SubagentTask {
                agent: request.agent.clone().unwrap_or_default(),
                task: request.task.clone().unwrap_or_default(),
                cwd: None,
                isolation: None,
                iso_apply: None,
                output_schema: request.output_schema.clone(),
                schema_mode: request.schema_mode,
                continue_flag: request.continue_flag,
                hub_id: request.hub_id.clone(),
            };
            if provisional.is_continuation() {
                provisional.validate_continuation()?;
            }
        }
        let concurrency = request
            .concurrency
            .unwrap_or(DEFAULT_CONCURRENCY)
            .clamp(1, MAX_PARALLEL_TASKS);

        match request.mode()? {
            RequestMode::Single(task) => {
                Ok(vec![self.run_one(&agents, task, None, on_update).await])
            }
            RequestMode::Parallel(tasks) => {
                let cwd = self.cwd.clone();
                let global_dir = self.global_dir.clone();
                let binary = self.child_binary.clone();
                let role_spec = self.role_model_spec.clone();
                let retry_config = self.retry_config.clone();
                let update = on_update.clone();
                let results = stream::iter(tasks.into_iter().enumerate())
                    .map(move |(index, task)| {
                        let agents = agents.clone();
                        let cwd = cwd.clone();
                        let global_dir = global_dir.clone();
                        let binary = binary.clone();
                        let role_spec = role_spec.clone();
                        let retry_config = retry_config.clone();
                        let update = update.clone();
                        async move {
                            let runner = ChildRunner::new(
                                cwd,
                                global_dir,
                                binary,
                                role_spec,
                                crate::agent_hub::ChildKind::Subagent,
                            )
                            .with_retry_config(retry_config);
                            (index, runner.run_one(&agents, task, None, update).await)
                        }
                    })
                    .buffer_unordered(concurrency)
                    .collect::<Vec<_>>()
                    .await;
                let mut ordered = results;
                ordered.sort_by_key(|(index, _)| *index);
                Ok(ordered.into_iter().map(|(_, result)| result).collect())
            }
            RequestMode::Chain(tasks) => {
                let mut previous: Option<SubagentResult> = None;
                let mut results = Vec::with_capacity(tasks.len());
                for (step, task) in tasks.into_iter().enumerate() {
                    let task = task.with_rendered_previous_result(previous.as_ref());
                    let result = self
                        .run_one(&agents, task, Some(step + 1), on_update.clone())
                        .await;
                    let failed = result.is_error;
                    previous = Some(result.clone());
                    results.push(result);
                    if failed {
                        break;
                    }
                }
                Ok(results)
            }
        }
    }

    async fn run_one(
        &self,
        agents: &BTreeMap<String, AgentDefinition>,
        task: SubagentTask,
        step: Option<usize>,
        on_update: Option<UpdateCallback>,
    ) -> SubagentResult {
        ChildRunner::new(
            self.cwd.clone(),
            self.global_dir.clone(),
            self.child_binary.clone(),
            self.role_model_spec.clone(),
            crate::agent_hub::ChildKind::Subagent,
        )
        .with_retry_config(self.retry_config.clone())
        .run_one(agents, task, step, on_update)
        .await
    }
}

#[async_trait]
impl Tool for SubagentTool {
    fn name(&self) -> &'static str {
        "subagent"
    }

    fn label(&self) -> &'static str {
        "Subagent"
    }

    fn description(&self) -> &'static str {
        "Delegate an isolated task to a named Pi child agent. Supports one task, bounded parallel tasks, or a sequential chain whose tasks may reference {previous}. Agent definitions live in $PI_CODING_AGENT_DIR/agents/*.md or .pi/agents/*.md. Workspace isolation: per-task `isolation: \"worktree\"` runs the child in a git worktree carrying the parent's uncommitted state, returning {worktree_path, diff_stat, patch} and applying per `isoApply` (keep|apply|drop; serial application, conflicts reported never forced). Coordination: isolated worktree children need no file reservations by construction; NON-isolated children share the parent checkout, so concurrent edits to the same files should be coordinated (e.g. Agent Mail file reservations with reason=<task id>)."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "agent": {"type": "string", "description": "Named agent for a single delegation."},
                "task": {"type": "string", "description": "Task for a single delegation."},
                "outputSchema": {"type": "object", "description": "JSON Schema the single delegation's final output must match; the parent validates and returns parsed data."},
                "schemaMode": {"type": "string", "enum": ["permissive", "strict"], "default": "permissive", "description": "permissive keeps an invalid result with a warning; strict fails the task."},
                "tasks": {"type": "array", "maxItems": MAX_PARALLEL_TASKS, "items": {"$ref": "#/definitions/task"}, "description": "Independent tasks to run in parallel."},
                "chain": {"type": "array", "maxItems": MAX_PARALLEL_TASKS, "items": {"$ref": "#/definitions/task"}, "description": "Sequential tasks; {previous} is replaced with the prior child output, and {{previous.data.<field.path>}} addresses the prior task's schema-validated data."},
                "concurrency": {"type": "integer", "minimum": 1, "maximum": MAX_PARALLEL_TASKS},
                "scope": {"type": "string", "enum": ["both", "user", "project"], "default": "both"},
                "continue": {"type": "boolean", "description": "When true, continue the existing subagent session identified by hubId with a new task. Requires hubId."},
                "hubId": {"type": "string", "description": "Existing subagent session/hub id to continue. Required when continue is true."}
            },
            "definitions": {
                "task": {
                    "type": "object",
                    "required": ["agent", "task"],
                    "properties": {
                        "agent": {"type": "string"},
                        "task": {"type": "string"},
                        "cwd": {"type": "string"},
                        "isolation": {"type": "string", "enum": ["none", "worktree"], "default": "none", "description": "worktree runs the child in a git worktree with the parent's uncommitted state; non-git dirs refuse with PI_ISO_NOT_GIT."},
                        "isoApply": {"type": "string", "enum": ["keep", "apply", "drop"], "default": "apply", "description": "What to do with the isolated worktree after completion."},
                        "outputSchema": {"type": "object", "description": "JSON Schema this task's final output must match."},
                        "schemaMode": {"type": "string", "enum": ["permissive", "strict"], "default": "permissive"},
                        "continue": {"type": "boolean", "description": "When true, continue the existing subagent session identified by hubId."},
                        "hubId": {"type": "string", "description": "Existing subagent session/hub id to continue."}
                    }
                }
            },
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        input: Value,
        on_update: Option<Box<dyn Fn(ToolUpdate) + Send + Sync>>,
        _abort: Option<crate::abort::AbortSignal>,
    ) -> Result<ToolOutput> {
        if current_subagent_depth() >= MAX_SUBAGENT_DEPTH {
            return Err(Error::tool(
                "subagent",
                format!(
                    "Refusing nested subagent depth above {MAX_SUBAGENT_DEPTH}; child agents are isolated by default and do not receive the subagent tool."
                ),
            ));
        }
        let request: SubagentRequest = serde_json::from_value(input)
            .map_err(|error| Error::tool("subagent", format!("Invalid input: {error}")))?;
        if request.is_continuation() {
            request.validate_continuation()?;
        }
        let update = on_update.map(Arc::from);
        let mode = request.mode_name()?;
        let should_reject_parallel_continue =
            request.is_continuation() && (request.tasks.is_some() || request.chain.is_some());
        if should_reject_parallel_continue {
            return Err(Error::tool(
                "subagent",
                "continue=true is only valid for single agent+task delegation, not for tasks/chain.",
            ));
        }
        let results = self.run_request(request, update).await?;
        let is_error = results.iter().any(|result| result.is_error);
        let hub_id = results
            .first()
            .and_then(|r| r.hub_id.clone())
            .map(Value::String)
            .unwrap_or(Value::Null);
        let session_isolation = results
            .first()
            .map(|r| r.session_isolation)
            .unwrap_or("ephemeral_no_session");
        let mut content = render_results(&results);
        if self.structured_results {
            content.push_str("\n\n");
            content.push_str(&structured_result_block(&results));
        }
        Ok(ToolOutput {
            content: vec![ContentBlock::Text(TextContent::new(content))],
            details: Some(json!({
                "schema": SUBAGENT_RESULT_SCHEMA,
                "mode": mode,
                "sessionIsolation": session_isolation,
                "hubId": hub_id,
                "results": results,
            })),
            is_error,
            touched_files: Vec::new(),
        })
    }

    fn effects(&self) -> ToolEffects {
        ToolEffects::process()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubagentRequest {
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    task: Option<String>,
    /// Single-delegation form of the per-task `outputSchema` (bd-cv653.5.1).
    #[serde(default)]
    output_schema: Option<Value>,
    #[serde(default)]
    schema_mode: SchemaMode,
    #[serde(default)]
    tasks: Option<Vec<SubagentTask>>,
    #[serde(default)]
    chain: Option<Vec<SubagentTask>>,
    #[serde(default)]
    concurrency: Option<usize>,
    #[serde(default)]
    scope: AgentScope,
    /// Continue an existing subagent session instead of starting a new one (C1).
    #[serde(default, rename = "continue")]
    continue_flag: Option<bool>,
    #[serde(default)]
    hub_id: Option<String>,
}

impl SubagentRequest {
    fn is_continuation(&self) -> bool {
        self.continue_flag == Some(true)
    }

    fn validate_continuation(&self) -> Result<()> {
        if !self.is_continuation() {
            return Ok(());
        }
        let hub_id = self
            .hub_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::tool(
                    "subagent",
                    "continue=true requires hubId (existing subagent session id).",
                )
            })?;
        if !is_valid_hub_id(hub_id) {
            return Err(Error::tool(
                "subagent",
                format!("Invalid hubId: {hub_id:?}"),
            ));
        }
        let task = self
            .task
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::tool(
                    "subagent",
                    "continue=true requires task (new user message to append).",
                )
            })?;
        if task.is_empty() {
            return Err(Error::tool(
                "subagent",
                "continue=true requires task (new user message to append).",
            ));
        }
        // When continuing, agent+task must still select exactly one mode, so
        // we also ensure agent is present for single delegation.
        if self
            .agent
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            return Err(Error::tool(
                "subagent",
                "continue=true requires agent for the continuation turn.",
            ));
        }
        Ok(())
    }

    fn mode(&self) -> Result<RequestMode> {
        let single = self
            .agent
            .as_ref()
            .zip(self.task.as_ref())
            .map(|(agent, task)| SubagentTask {
                agent: agent.clone(),
                task: task.clone(),
                cwd: None,
                isolation: None,
                iso_apply: None,
                output_schema: self.output_schema.clone(),
                schema_mode: self.schema_mode,
                continue_flag: self.continue_flag,
                hub_id: self.hub_id.clone(),
            });
        let selected = usize::from(single.is_some())
            + usize::from(self.tasks.is_some())
            + usize::from(self.chain.is_some());
        if selected.ne(&1) {
            return Err(Error::tool(
                "subagent",
                "Provide exactly one of agent+task, tasks, or chain.",
            ));
        }
        if self.agent.is_some() != self.task.is_some() {
            return Err(Error::tool(
                "subagent",
                "Single delegation requires both agent and task.",
            ));
        }
        if let Some(tasks) = &self.tasks
            && (tasks.is_empty() || tasks.len() > MAX_PARALLEL_TASKS)
        {
            return Err(Error::tool(
                "subagent",
                format!("tasks must contain 1-{MAX_PARALLEL_TASKS} entries."),
            ));
        }
        if let Some(chain) = &self.chain
            && (chain.is_empty() || chain.len() > MAX_PARALLEL_TASKS)
        {
            return Err(Error::tool(
                "subagent",
                format!("chain must contain 1-{MAX_PARALLEL_TASKS} entries."),
            ));
        }
        Ok(single.map_or_else(
            || {
                self.tasks.as_ref().map_or_else(
                    || RequestMode::Chain(self.chain.clone().unwrap_or_default()),
                    |tasks| RequestMode::Parallel(tasks.clone()),
                )
            },
            RequestMode::Single,
        ))
    }

    fn mode_name(&self) -> Result<&'static str> {
        match self.mode()? {
            RequestMode::Single(_) => Ok("single"),
            RequestMode::Parallel(_) => Ok("parallel"),
            RequestMode::Chain(_) => Ok("chain"),
        }
    }
}

enum RequestMode {
    Single(SubagentTask),
    Parallel(Vec<SubagentTask>),
    Chain(Vec<SubagentTask>),
}

impl RequestMode {
    /// Extract the single task when this mode is `Single`.
    #[allow(clippy::wrong_self_convention)]
    fn into_single(self) -> Option<SubagentTask> {
        match self {
            Self::Single(task) => Some(task),
            Self::Parallel(_) | Self::Chain(_) => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubagentTask {
    agent: String,
    task: String,
    #[serde(default)]
    cwd: Option<PathBuf>,
    /// Workspace isolation: `none` (default) or `worktree` (bd-cv653.5.2).
    #[serde(default)]
    isolation: Option<String>,
    /// What to do with an isolated worktree after completion: `keep`,
    /// `apply` (default), or `drop`.
    #[serde(default)]
    iso_apply: Option<String>,
    /// JSON Schema the child's final output must match (bd-cv653.5.1).
    /// Overrides the agent definition's `output_schema` when both are set.
    #[serde(default)]
    output_schema: Option<Value>,
    #[serde(default)]
    schema_mode: SchemaMode,
    /// Continue an existing session identified by `hubId` (C1).
    #[serde(default, rename = "continue")]
    continue_flag: Option<bool>,
    #[serde(default)]
    hub_id: Option<String>,
}

impl SubagentTask {
    /// Whether this task requests continuation of an existing session.
    const fn is_continuation(&self) -> bool {
        matches!(self.continue_flag, Some(true))
    }

    /// Validate `continue` contract for a single task.
    fn validate_continuation(&self) -> Result<()> {
        if !self.is_continuation() {
            return Ok(());
        }
        let hub_id = self
            .hub_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::tool(
                    "subagent",
                    "continue=true requires hubId (existing subagent session id).",
                )
            })?;
        if !is_valid_hub_id(hub_id) {
            return Err(Error::tool(
                "subagent",
                format!("Invalid hubId: {hub_id:?}"),
            ));
        }
        if self.task.trim().is_empty() {
            return Err(Error::tool(
                "subagent",
                "continue=true requires task (new user message to append).",
            ));
        }
        Ok(())
    }
}

/// How a schema-validation failure that survives the corrective retry is
/// treated (bd-cv653.5.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum SchemaMode {
    /// Accept the invalid result, exposing `schema_valid: false` plus
    /// `validation_errors` as a warning.
    #[default]
    Permissive,
    /// Fail the task with the validation errors.
    Strict,
}

impl SubagentTask {
    /// Render `{previous}` (raw prior output — historical contract) and, when
    /// the prior result is schema-valid, `{{previous.data.<dotted.path>}}`
    /// field references (bd-cv653.5.1). Unresolvable field references are
    /// left verbatim so mistakes stay visible instead of silently vanishing.
    fn with_rendered_previous_result(mut self, previous: Option<&SubagentResult>) -> Self {
        let raw = previous.map_or("", |result| result.output.as_str());
        self.task = self.task.replace(concat!("{", "previous", "}"), raw);
        if let Some(data) = previous
            .filter(|result| result.schema_valid == Some(true))
            .and_then(|result| result.data.as_ref())
        {
            self.task = render_previous_data_fields(&self.task, data);
        }
        self
    }
}

/// Replace `{{previous.data.<path>}}` tokens with values addressed out of the
/// prior task's parsed `data`. Scalars render bare; objects/arrays render as
/// compact JSON. Missing paths leave the token untouched.
fn render_previous_data_fields(task: &str, data: &Value) -> String {
    const OPEN: &str = "{{previous.data.";
    const CLOSE: &str = "}}";
    let mut rendered = String::with_capacity(task.len());
    let mut rest = task;
    while let Some(start) = rest.find(OPEN) {
        rendered.push_str(&rest[..start]);
        let after_open = &rest[start + OPEN.len()..];
        let Some(end) = after_open.find(CLOSE) else {
            rendered.push_str(&rest[start..]);
            return rendered;
        };
        let path = &after_open[..end];
        let pointer = format!("/{}", path.replace('.', "/"));
        match data.pointer(&pointer) {
            Some(Value::String(text)) => rendered.push_str(text),
            Some(Value::Null) | None => {
                // Leave the token verbatim so the miss is visible.
                rendered.push_str(&rest[start..start + OPEN.len() + end + CLOSE.len()]);
            }
            Some(value) => rendered.push_str(&value.to_string()),
        }
        rest = &after_open[end + CLOSE.len()..];
    }
    rendered.push_str(rest);
    rendered
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AgentScope {
    User,
    Project,
    #[default]
    Both,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum AgentSource {
    BuiltIn,
    User,
    Project,
}

#[derive(Debug, Clone)]
struct AgentDefinition {
    name: String,
    description: String,
    model: Option<String>,
    reasoning: Option<String>,
    tools: Option<Vec<String>>,
    skills: Vec<PathBuf>,
    system_prompt: String,
    /// Default JSON Schema for the child's final output, from the definition's
    /// single-line `output_schema:` frontmatter field (bd-cv653.5.1). A task's
    /// `outputSchema` overrides it.
    output_schema: Option<Value>,
    source: AgentSource,
    file_path: PathBuf,
}

fn tan_agent_definition() -> AgentDefinition {
    AgentDefinition {
        name: TAN_AGENT_NAME.to_string(),
        description: "Background tangential coding agent".to_string(),
        model: None,
        reasoning: None,
        tools: None,
        skills: Vec::new(),
        system_prompt: TAN_SYSTEM_PROMPT.to_string(),
        output_schema: None,
        source: AgentSource::BuiltIn,
        file_path: PathBuf::from("<built-in:tan>"),
    }
}

fn discover_agents_with_roots(
    cwd: &Path,
    global_dir: &Path,
    scope: AgentScope,
) -> Result<BTreeMap<String, AgentDefinition>> {
    let mut agents = BTreeMap::new();
    if !matches!(scope, AgentScope::Project) {
        load_agent_dir(&global_dir.join("agents"), AgentSource::User, &mut agents)?;
    }
    if !matches!(scope, AgentScope::User)
        && let Some(project_dir) = nearest_project_agents_dir(cwd)
    {
        // Project definitions intentionally replace user definitions of the same name.
        load_agent_dir(&project_dir, AgentSource::Project, &mut agents)?;
    }
    Ok(agents)
}

fn nearest_project_agents_dir(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    loop {
        let candidate = current.join(".pi").join("agents");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn load_agent_dir(
    directory: &Path,
    source: AgentSource,
    agents: &mut BTreeMap<String, AgentDefinition>,
) -> Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    let entries = std::fs::read_dir(directory).map_err(|error| {
        Error::tool(
            "subagent",
            format!(
                "Cannot read agent directory {}: {error}",
                directory.display()
            ),
        )
    })?;
    let mut paths = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        })
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let raw = std::fs::read_to_string(&path).map_err(|error| {
            Error::tool(
                "subagent",
                format!("Cannot read agent definition {}: {error}", path.display()),
            )
        })?;
        let (frontmatter, body) = parse_frontmatter(&raw);
        let name = required_agent_field(&frontmatter, "name", &path)?;
        let description = required_agent_field(&frontmatter, "description", &path)?;
        let tools = frontmatter.get("tools").map(|value| split_csv(value));
        let definition_dir = path.parent().unwrap_or(directory);
        let skills = frontmatter
            .get("skills")
            .map(|value| {
                split_csv(value)
                    .into_iter()
                    .map(PathBuf::from)
                    .map(|skill| {
                        if skill.is_absolute() {
                            skill
                        } else {
                            definition_dir.join(skill)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let output_schema = frontmatter
            .get("output_schema")
            .map(|raw| {
                serde_json::from_str::<Value>(raw).map_err(|error| {
                    Error::tool(
                        "subagent",
                        format!(
                            "Agent definition {} has an invalid output_schema (must be single-line JSON): {error}",
                            path.display()
                        ),
                    )
                })
            })
            .transpose()?;
        agents.insert(
            name.clone(),
            AgentDefinition {
                name,
                description,
                model: frontmatter.get("model").cloned(),
                reasoning: frontmatter
                    .get("reasoning")
                    .or_else(|| frontmatter.get("thinking"))
                    .cloned(),
                tools,
                skills,
                system_prompt: body,
                output_schema,
                source,
                file_path: path,
            },
        );
    }
    Ok(())
}

fn required_agent_field(
    fields: &BTreeMap<String, String>,
    field: &str,
    path: &Path,
) -> Result<String> {
    fields
        .get(field)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            Error::tool(
                "subagent",
                format!(
                    "Agent definition {} requires frontmatter field {field:?}",
                    path.display()
                ),
            )
        })
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_frontmatter(raw: &str) -> (BTreeMap<String, String>, String) {
    let mut lines = raw.lines();
    if !matches!(lines.next(), Some(first) if first.trim().eq("---")) {
        return (BTreeMap::new(), raw.to_string());
    }
    let mut fields = BTreeMap::new();
    let mut body = Vec::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim().eq("---") {
            closed = true;
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim();
            if !key.is_empty() {
                fields.insert(
                    key.to_string(),
                    value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string(),
                );
            }
        }
    }
    if !closed {
        return (BTreeMap::new(), raw.to_string());
    }
    body.extend(lines);
    (fields, body.join("\n"))
}

#[derive(Debug, Clone)]
struct ChildRunner {
    cwd: PathBuf,
    global_dir: PathBuf,
    child_binary: PathBuf,
    role_model_spec: Option<String>,
    hub_kind: crate::agent_hub::ChildKind,
    retry_config: SubagentRetryConfig,
}

impl ChildRunner {
    fn new(
        cwd: PathBuf,
        global_dir: PathBuf,
        child_binary: PathBuf,
        role_model_spec: Option<String>,
        hub_kind: crate::agent_hub::ChildKind,
    ) -> Self {
        Self {
            cwd,
            global_dir,
            child_binary,
            role_model_spec,
            hub_kind,
            retry_config: SubagentRetryConfig::default(),
        }
    }

    #[must_use]
    fn with_retry_config(mut self, config: SubagentRetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Run one task, applying the typed-output contract (bd-cv653.5.1) when
    /// an `outputSchema` is in play: the child gets a schema directive
    /// appended to its system prompt; the parent validates the final output,
    /// grants exactly one corrective re-run on failure, and then either
    /// annotates (permissive) or fails (strict) a still-invalid result.
    ///
    /// Children are ephemeral (`--no-session`), so the corrective retry is a
    /// fresh child run carrying the validation errors, not an in-session
    /// follow-up. Tolerant-dialect repair before validation (bd-cv653.7.8)
    /// composes here once that layer exists.
    ///
    /// Network-transient failures are additionally retried here (IMPLEMENT C1):
    /// the flattened `error+output+stderr` of a failed run is fed to
    /// `crate::error::is_retryable_error`; on a hit the loop sleeps
    /// `retry_config.delay_ms(attempt)` with `AgentCx::checkpoint` cancellation
    /// before re-launching the child. Schema validation remains a separate
    /// single corrective retry that runs only after the network layer settles.
    async fn run_one(
        &self,
        agents: &BTreeMap<String, AgentDefinition>,
        task: SubagentTask,
        step: Option<usize>,
        on_update: Option<UpdateCallback>,
    ) -> SubagentResult {
        // Network layer: retry transient child failures before schema handling.
        let mut attempt: u32 = 0;
        let mut last_result: Option<SubagentResult> = None;
        let max_attempts = if self.retry_config.enabled {
            self.retry_config.max_retries.saturating_add(1)
        } else {
            1
        };
        loop {
            let schema_aware = self
                .run_one_schema_aware(agents, task.clone(), step, on_update.clone())
                .await;
            let is_transient_failure = schema_aware.is_error
                && Self::is_retryable_subagent_failure(&schema_aware)
                && !matches!(schema_aware.status, SubagentStatus::Cancelled);
            if !is_transient_failure {
                return schema_aware;
            }
            attempt = attempt.saturating_add(1);
            if last_result.is_some() {
                let _ = last_result.take();
            }
            last_result = Some(schema_aware);
            if attempt >= max_attempts {
                break;
            }
            // Cancellation before sleeping is itself a terminal signal.
            let cx = AgentCx::for_current_or_request();
            if cx.checkpoint().is_err() {
                break;
            }
            let delay_ms = self.retry_config.delay_ms(attempt);
            let delay = Duration::from_millis(u64::from(delay_ms));
            let cx_for_sleep = cx.cx();
            let start = cx_for_sleep
                .timer_driver()
                .map_or_else(asupersync::time::wall_now, |t| t.now());
            // Cancellable sleep: short ticks so checkpoint/abort propagates.
            let deadline = start + delay;
            loop {
                if cx.checkpoint().is_err() {
                    break;
                }
                let now = cx_for_sleep
                    .timer_driver()
                    .map_or_else(asupersync::time::wall_now, |t| t.now());
                if now >= deadline {
                    break;
                }
                let remaining = deadline.duration_since(now);
                let tick =
                    std::cmp::min(Duration::from_millis(50), Duration::from_millis(remaining));
                asupersync::time::sleep(now, tick).await;
            }
            if cx.checkpoint().is_err() {
                break;
            }
        }
        // Exhausted retries — surface the last transient failure.
        last_result.expect("at least one run")
    }

    fn is_retryable_subagent_failure(result: &SubagentResult) -> bool {
        // Flatten like the provider retry classifier does: error + output + stderr.
        let mut flat = String::new();
        if let Some(error) = &result.error {
            flat.push_str(error);
            flat.push(' ');
        }
        flat.push_str(&result.output);
        flat.push(' ');
        flat.push_str(&result.stderr);
        crate::error::is_retryable_error(&flat, None, None)
    }

    async fn run_one_schema_aware(
        &self,
        agents: &BTreeMap<String, AgentDefinition>,
        task: SubagentTask,
        step: Option<usize>,
        on_update: Option<UpdateCallback>,
    ) -> SubagentResult {
        let schema = task.output_schema.clone().or_else(|| {
            agents
                .get(&task.agent)
                .and_then(|agent| agent.output_schema.clone())
        });
        let Some(schema) = schema else {
            return self
                .run_child_process(agents, task, step, on_update, None)
                .await;
        };
        // Reject an uncompilable schema before spending a child launch.
        // (Compiled per call rather than held across awaits so the future
        // stays Send without depending on the validator's auto-traits.)
        if let Err(error) = compile_output_schema(&schema) {
            return agents.get(&task.agent).map_or_else(
                || SubagentResult::unknown(task.clone(), step),
                |agent| {
                    SubagentResult::failed(
                        agent,
                        task.clone(),
                        step,
                        format!("Invalid outputSchema: {error}"),
                    )
                },
            );
        }

        let schema_mode = task.schema_mode;
        let mut result = self
            .run_child_process(agents, task.clone(), step, on_update.clone(), Some(&schema))
            .await;
        if result.is_error {
            return result;
        }

        let mut retries = 0usize;
        let mut outcome = validate_child_output(&result.output, &schema);
        if let Err(errors) = &outcome {
            // Bounded corrective retry: exactly one fresh run with the errors.
            retries = 1;
            let corrective = SubagentTask {
                task: corrective_retry_task(&task.task, errors),
                ..task.clone()
            };
            let retry_result = self
                .run_child_process(agents, corrective, step, on_update, Some(&schema))
                .await;
            if !retry_result.is_error {
                result = retry_result;
                outcome = validate_child_output(&result.output, &schema);
            }
        }

        result.schema_retries = Some(retries);
        match outcome {
            Ok(data) => {
                result.data = Some(data);
                result.schema_valid = Some(true);
            }
            Err(errors) => {
                result.schema_valid = Some(false);
                result.validation_errors = Some(errors);
                if schema_mode == SchemaMode::Strict {
                    result.status = SubagentStatus::Failed;
                    result.is_error = true;
                    result.error.get_or_insert_with(|| {
                        "Child output failed schema validation after the corrective retry (schemaMode: strict)."
                            .to_string()
                    });
                }
            }
        }
        result
    }

    #[allow(clippy::too_many_lines)]
    async fn run_child_process(
        &self,
        agents: &BTreeMap<String, AgentDefinition>,
        task: SubagentTask,
        step: Option<usize>,
        on_update: Option<UpdateCallback>,
        output_schema: Option<&Value>,
    ) -> SubagentResult {
        let Some(agent) = agents.get(&task.agent) else {
            return SubagentResult::unknown(task, step);
        };
        let cwd = task.cwd.clone().unwrap_or_else(|| self.cwd.clone());
        if !cwd.is_dir() {
            return SubagentResult::failed(
                agent,
                task,
                step,
                format!("Working directory does not exist: {}", cwd.display()),
            );
        }

        // Workspace isolation (bd-cv653.5.2): `worktree` runs the child in
        // a git worktree carrying the parent's uncommitted state; the patch
        // is collected and applied per `iso_apply` at completion.
        // C3 persistent reuse: `continue=true` reopens the same
        // `<global_dir>/worktrees/subagents/<hubId>` so edits accumulate
        // incrementally (`hubId == worktreeId`). Fresh runs are also
        // persistent so a later `continue` can find the same worktree.
        let isolation = task
            .isolation
            .as_deref()
            .unwrap_or("none")
            .to_ascii_lowercase();
        let iso_requested = isolation == "worktree";

        // C1 session wiring: pick the child session path before launching.
        //
        // - `continue=true` + valid `hubId` → reuse `<global>/sessions/subagents/<hubId>.jsonl`
        // - otherwise (fresh subagent) → allocated by hub id once hub registration
        //   succeeds; we provisionally build `args` with `None` and patch after.
        let continue_session_path: Option<PathBuf> = if task.is_continuation() {
            let hub_id_owned = task.hub_id.clone().unwrap_or_default().trim().to_string();
            let hub_id = hub_id_owned.as_str();
            let path = task
                .hub_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty() && is_valid_hub_id(s))
                .map(|hub_id| subagent_session_path(&self.global_dir, hub_id));
            // Fail closed when the caller claims `continue` but the session
            // file is absent — likely a stale hubId after `global_dir` was
            // wiped or the parent PID recycled.
            if let Some(ref p) = path
                && !p.exists()
            {
                return SubagentResult::failed(
                    agent,
                    task,
                    step,
                    format!(
                        "hubId '{}' has no session file at {}; run a fresh subagent first to create it.",
                        hub_id,
                        p.display()
                    ),
                );
            }
            path
        } else {
            None
        };
        if let Some(path) = continue_session_path.as_deref()
            && let Some(parent) = path.parent()
        {
            let _ = std::fs::create_dir_all(parent);
        }
        // Hub registration determines the canonical hubId for fresh runs.
        // We do this BEFORE worktree creation so fresh runs can allocate a
        // persistent worktree at `global_dir/worktrees/subagents/<hubId>`.
        let hub_entry: Option<crate::agent_hub::ChildEntry> =
            if let Some(ref _path) = continue_session_path {
                let hub_id = task
                    .hub_id
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .to_string();
                crate::agent_hub::registry()
                    .lock()
                    .ok()
                    .and_then(|r| r.get(&hub_id))
                    .or_else(|| {
                        let dir = crate::config::Config::global_dir()
                            .join("agent-hub")
                            .join(std::process::id().to_string());
                        let _ = std::fs::create_dir_all(&dir);
                        Some(crate::agent_hub::ChildEntry {
                            id: hub_id.clone(),
                            name: agent.name.clone(),
                            kind: self.hub_kind,
                            task: task.task.clone(),
                            pid: None,
                            status: crate::agent_hub::ChildStatus::Starting,
                            started_ms: 0,
                            finished_ms: None,
                            output_bytes: 0,
                            transcript_path: dir.join(format!("{hub_id}.transcript.jsonl")),
                            steer_path: dir.join(format!("{hub_id}.steer")),
                            revived_from: None,
                        })
                    })
            } else {
                crate::agent_hub::registry()
                    .lock()
                    .ok()
                    .and_then(|mut reg| {
                        reg.register_kind(&agent.name, &task.task, self.hub_kind)
                            .ok()
                    })
            };

        // Determine the final session path (canonical hubId for fresh).
        let mut final_session_path: Option<PathBuf> = continue_session_path.clone();
        if final_session_path.is_none()
            && let Some(entry) = hub_entry.as_ref()
        {
            let canonical = subagent_session_path(&self.global_dir, &entry.id);
            let _ = std::fs::create_dir_all(canonical.parent().unwrap_or(Path::new(".")));
            final_session_path = Some(canonical);
        }

        // Now create or reopen the worktree using the canonical hubId.
        let iso_handle: Option<crate::worktree_iso::IsoHandle> = if iso_requested {
            let hub_id = hub_entry.as_ref().map(|e| e.id.as_str()).or_else(|| {
                task.hub_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && is_valid_hub_id(s))
            });
            if let Some(hub_id) = hub_id {
                match crate::worktree_iso::reopen_or_isolate(
                    &self.global_dir,
                    &cwd,
                    hub_id,
                    &task.task,
                ) {
                    Ok(handle) => Some(handle),
                    Err(err) => {
                        return SubagentResult::failed(agent, task, step, err.to_string());
                    }
                }
            } else {
                match crate::worktree_iso::isolate(&cwd, &task.task) {
                    Ok(handle) => Some(handle),
                    Err(err) => {
                        return SubagentResult::failed(agent, task, step, err.to_string());
                    }
                }
            }
        } else {
            None
        };
        let run_cwd = iso_handle
            .as_ref()
            .map_or_else(|| cwd.clone(), |handle| handle.path.clone());
        let iso_apply = task.iso_apply.clone();

        let mut args = child_args(
            agent,
            &task.task,
            self.role_model_spec.as_deref(),
            output_schema,
            final_session_path.as_deref(),
        );
        let mut pending_continue_session = final_session_path.clone();
        let mut result = SubagentResult::starting(
            agent,
            task.clone(),
            step,
            &self.child_binary,
            &run_cwd,
            &args,
        );
        // Agent-hub registration already done above — wire result fields.
        if let Some(entry) = hub_entry.as_ref() {
            result.hub_id = Some(entry.id.clone());
            result.session_isolation = "persistent_session";
            // Ensure args use the canonical session path (fresh case).
            if continue_session_path.is_none() {
                let canonical = subagent_session_path(&self.global_dir, &entry.id);
                let new_args = child_args(
                    agent,
                    &task.task,
                    self.role_model_spec.as_deref(),
                    output_schema,
                    Some(&canonical),
                );
                args = new_args;
                pending_continue_session = Some(canonical);
            }
        } else {
            result.hub_id = None;
        }
        let update = on_update.as_ref();
        emit_progress(update, &result);

        let mut command = Command::new(&self.child_binary);
        command
            .args(&args)
            .current_dir(&run_cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // `Command` inherits the rest of the parent environment, including API/router/auth
            // variables and `PI_CODING_AGENT_DIR`; set this explicitly for auditability.
            .env("PI_CODING_AGENT_DIR", &self.global_dir)
            .env("PI_SUBAGENT_PARENT_PID", std::process::id().to_string())
            .env("PI_SUBAGENT_DEPTH", child_depth().to_string());
        // Hub steering channel (bd-cv653.5.3): the child drains this file
        // between turns via its print-mode steering fetcher.
        if let Some(entry) = hub_entry.as_ref() {
            command
                .env("PI_SUBAGENT_STEER_FILE", &entry.steer_path)
                .env("PI_SUBAGENT_RUN_ID", &entry.id);
        }
        // Keep session-path alive for later use (e.g. diagnostics).
        let _ = pending_continue_session;

        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                result.fail(format!(
                    "Failed to launch {}: {error}",
                    self.child_binary.display()
                ));
                // Settle the roster entry registered above — otherwise the
                // hub shows this child as Starting forever and steer keeps
                // queueing messages to it.
                if let Some(hub_id) = &result.hub_id
                    && let Ok(mut reg) = crate::agent_hub::registry().lock()
                {
                    reg.settle(hub_id, crate::agent_hub::ChildStatus::Failed);
                }
                emit_progress(update, &result);
                return result;
            }
        };
        crate::tools::attach_child_job_discipline(&child);
        let mut child = ChildProcessGuard::new(child);
        result.pid = Some(child.id());
        result.status = SubagentStatus::Running;
        if let Some(hub_id) = &result.hub_id
            && let Ok(mut reg) = crate::agent_hub::registry().lock()
        {
            reg.mark_running(hub_id, child.id());
        }
        emit_progress(update, &result);

        if !child.has_stdout() {
            result.fail("Child stdout was not piped.".to_string());
            if let Some(hub_id) = &result.hub_id
                && let Ok(mut reg) = crate::agent_hub::registry().lock()
            {
                reg.settle(hub_id, crate::agent_hub::ChildStatus::Failed);
            }
            return result;
        }
        let stdout = child.take_stdout().expect("stdout checked above");
        let stderr = child.take_stderr().expect("stderr is piped");
        let (tx, rx) = mpsc::sync_channel(256);
        let stdout_thread = spawn_pipe_reader(stdout, PipeKind::Stdout, tx.clone());
        let stderr_thread = spawn_pipe_reader(stderr, PipeKind::Stderr, tx);
        let mut saw_cancellation = false;
        let cx = AgentCx::for_current_or_request();

        loop {
            drain_child_frames(&rx, &mut result, update);
            match child.try_wait() {
                Ok(Some(status)) => {
                    result.exit_code = status.code();
                    break;
                }
                Ok(None) => {}
                Err(error) => {
                    result.fail(format!("Failed while waiting for child: {error}"));
                    child.terminate();
                    break;
                }
            }
            if cx.checkpoint().is_err() {
                saw_cancellation = true;
                result.status = SubagentStatus::Cancelled;
                result.error = Some("Parent cancellation propagated to child process.".to_string());
                result.is_error = true;
                child.terminate();
                break;
            }
            let now = cx
                .cx()
                .timer_driver()
                .map_or_else(asupersync::time::wall_now, |timer| timer.now());
            asupersync::time::sleep(now, Duration::from_millis(10)).await;
        }

        drain_until_reader_exit(rx, &mut result, update, stdout_thread, stderr_thread).await;
        if !saw_cancellation && !matches!(result.status, SubagentStatus::Failed) {
            if result.exit_code == Some(0) {
                result.status = SubagentStatus::Completed;
            } else {
                result.status = SubagentStatus::Failed;
                result.is_error = true;
                result.error.get_or_insert_with(|| {
                    format!("Child exited with code {}.", result.exit_code.unwrap_or(-1))
                });
            }
        }
        child.disarm();
        // Hub settle (bd-cv653.5.3): operator kill (already recorded) beats
        // exit-code inference; otherwise map the run outcome.
        if let Some(hub_id) = &result.hub_id
            && let Ok(mut reg) = crate::agent_hub::registry().lock()
        {
            let prior = reg.get(hub_id).map(|entry| entry.status);
            if prior != Some(crate::agent_hub::ChildStatus::Killed) {
                let status = match result.status {
                    SubagentStatus::Completed => Some(crate::agent_hub::ChildStatus::Done),
                    SubagentStatus::Cancelled => Some(crate::agent_hub::ChildStatus::Cancelled),
                    SubagentStatus::Failed => Some(crate::agent_hub::ChildStatus::Failed),
                    SubagentStatus::Starting | SubagentStatus::Running => None,
                };
                if let Some(status) = status {
                    reg.settle(hub_id, status);
                }
            }
        }
        emit_progress(update, &result);

        // Worktree isolation completion (bd-cv653.5.2): collect the patch
        // and apply per `iso_apply` (keep/apply/drop). A conflicting apply
        // reports files and leaves the worktree — never force.
        if let Some(handle) = iso_handle {
            let mut mode = crate::worktree_iso::IsoApplyMode::parse(iso_apply.as_deref())
                .unwrap_or(crate::worktree_iso::IsoApplyMode::Apply);
            // Never auto-apply the half-finished edits of a failed or
            // cancelled child into the parent tree — that is exactly the
            // state isolation exists to contain. Keep the worktree so the
            // patch stays inspectable.
            if mode == crate::worktree_iso::IsoApplyMode::Apply
                && !matches!(result.status, SubagentStatus::Completed)
            {
                mode = crate::worktree_iso::IsoApplyMode::Keep;
            }
            let mut outcome = crate::worktree_iso::IsoOutcome {
                schema: crate::worktree_iso::ISO_SCHEMA.to_string(),
                worktree_path: handle.path.display().to_string(),
                branch: handle.branch.clone(),
                diff_stat: String::new(),
                patch: String::new(),
                conflicted_files: Vec::new(),
                apply_mode: mode.as_str().to_string(),
                applied: false,
            };
            match crate::worktree_iso::collect_diff(&handle) {
                Ok((patch, diff_stat)) => {
                    outcome.diff_stat = diff_stat;
                    outcome.patch.clone_from(&patch);
                    if mode == crate::worktree_iso::IsoApplyMode::Apply {
                        match crate::worktree_iso::apply_to_parent(&handle, &patch) {
                            Ok(()) => {
                                outcome.applied = true;
                                let _ = crate::worktree_iso::drop_worktree(&handle);
                            }
                            Err(err) => {
                                outcome.conflicted_files =
                                    err.to_string().lines().map(str::to_string).collect();
                                result.error = Some(err.to_string());
                                result.is_error = true;
                            }
                        }
                    } else if mode == crate::worktree_iso::IsoApplyMode::Drop {
                        let _ = crate::worktree_iso::drop_worktree(&handle);
                    }
                }
                Err(err) => {
                    result.error = Some(format!("failed to collect isolated diff: {err}"));
                    result.is_error = true;
                }
            }
            result.iso = Some(outcome);
        }
        result
    }
}

/// Owns a spawned child until it has been reaped.  The parent agent's abort
/// path drops tool futures, so this guard is the final cancellation boundary:
/// a dropped subagent future cannot leave a Rust Pi child running.
struct ChildProcessGuard {
    child: Option<std::process::Child>,
}

impl ChildProcessGuard {
    const fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    fn id(&self) -> u32 {
        self.child.as_ref().map_or(0, std::process::Child::id)
    }

    fn has_stdout(&self) -> bool {
        self.child
            .as_ref()
            .is_some_and(|child| child.stdout.is_some())
    }

    fn take_stdout(&mut self) -> Option<std::process::ChildStdout> {
        self.child.as_mut().and_then(|child| child.stdout.take())
    }

    fn take_stderr(&mut self) -> Option<std::process::ChildStderr> {
        self.child.as_mut().and_then(|child| child.stderr.take())
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child
            .as_mut()
            .map_or(Ok(None), std::process::Child::try_wait)
    }

    fn terminate(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn disarm(&mut self) {
        let _ = self.child.take();
    }
}

impl Drop for ChildProcessGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn child_args(
    agent: &AgentDefinition,
    task: &str,
    role_model_spec: Option<&str>,
    output_schema: Option<&Value>,
    session_path: Option<&Path>,
) -> Vec<OsString> {
    let mut args = vec!["--mode".into(), "json".into(), "--print".into()];
    // C1a: persistent subagent sessions. `Some(path)` → `--session <path>`
    // so the child replays the same JSONL and the next user message is
    // appended to the same session/worktree. `None` retains `--no-session`
    // as a fallback (gray path — remove once every caller allocates a session).
    if let Some(path) = session_path {
        args.extend(["--session".into(), path.as_os_str().to_os_string()]);
    } else {
        args.push("--no-session".into());
    }
    args.extend([
        "--tools".into(),
        agent
            .tools
            .as_ref()
            .map_or_else(|| DEFAULT_CHILD_TOOLS.to_string(), |tools| tools.join(","))
            .into(),
    ]);
    // Model precedence (bd-cv653.3.1): agent-def `model:` pin > task/smol role
    // spec from settings > nothing (child inherits the parent's ambient model).
    if let Some(model) = &agent.model {
        args.extend(["--model".into(), model.clone().into()]);
    } else if let Some(spec) = role_model_spec {
        args.extend(["--model".into(), spec.into()]);
    }
    if let Some(reasoning) = &agent.reasoning {
        args.extend(["--thinking".into(), reasoning.clone().into()]);
    }
    for skill in &agent.skills {
        args.extend(["--skill".into(), skill.clone().into_os_string()]);
    }
    // The schema directive rides the same appended system prompt as the
    // definition body (bd-cv653.5.1): one --append-system-prompt carrying
    // both keeps the child argv shape identical for schema-free tasks.
    let schema_directive = output_schema.map(|schema| {
        format!(
            "Your final answer MUST be a single JSON value matching this JSON Schema (no prose, no code fences):\n{schema}"
        )
    });
    let appended_prompt = match (agent.system_prompt.trim(), &schema_directive) {
        ("", None) => None,
        ("", Some(directive)) => Some(directive.clone()),
        (prompt, None) => Some(prompt.to_string()),
        (prompt, Some(directive)) => Some(format!("{prompt}\n\n{directive}")),
    };
    if let Some(prompt) = appended_prompt {
        args.extend(["--append-system-prompt".into(), prompt.into()]);
    }
    args.push(format!("Task: {task}").into());
    args
}

/// Compile an `outputSchema`, surfacing draft/keyword errors as strings.
fn compile_output_schema(schema: &Value) -> std::result::Result<jsonschema::Validator, String> {
    jsonschema::validator_for(schema).map_err(|error| error.to_string())
}

/// Validate a child's final output against `schema` (bd-cv653.5.1).
///
/// The output is located tolerantly before validation: the whole trimmed
/// text, else the payload of a ```json fence, else the first balanced
/// `{...}`/`[...]` region — models frequently wrap yields in prose despite
/// the directive. Returns the parsed value on success, or the collected
/// validation (or parse) errors.
fn validate_child_output(output: &str, schema: &Value) -> std::result::Result<Value, Vec<String>> {
    let validator = compile_output_schema(schema).map_err(|error| vec![error])?;
    let candidate = extract_json_candidate(output)
        .ok_or_else(|| vec!["child output contains no parseable JSON value".to_string()])?;
    let errors: Vec<String> = validator
        .iter_errors(&candidate)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(candidate)
    } else {
        Err(errors)
    }
}

/// Locate the JSON value in a child's final text output.
fn extract_json_candidate(output: &str) -> Option<Value> {
    let trimmed = output.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    // ```json ... ``` (or bare ```) fenced payloads.
    for fence in ["```json", "```"] {
        if let Some(start) = trimmed.find(fence) {
            let after = &trimmed[start + fence.len()..];
            if let Some(end) = after.find("```")
                && let Ok(value) = serde_json::from_str::<Value>(after[..end].trim())
            {
                return Some(value);
            }
        }
    }
    // First balanced object/array region.
    for open in ['{', '['] {
        if let Some(start) = trimmed.find(open)
            && let Some(candidate) = balanced_json_region(&trimmed[start..])
            && let Ok(value) = serde_json::from_str::<Value>(candidate)
        {
            return Some(value);
        }
    }
    None
}

/// The shortest prefix of `text` (which starts at `{` or `[`) that closes the
/// opening bracket, honoring strings and escapes. `None` if never balanced.
fn balanced_json_region(text: &str) -> Option<&str> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in text.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if in_string => escaped = true,
            b'"' => in_string = !in_string,
            b'{' | b'[' if !in_string => depth += 1,
            b'}' | b']' if !in_string => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&text[..=index]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The corrective follow-up task for the single bounded retry. (`child_args`
/// adds the `Task:` prefix, so this must not.)
fn corrective_retry_task(original_task: &str, errors: &[String]) -> String {
    format!(
        "{original_task}\n\nYour previous output failed schema validation:\n{}\n\nReturn ONLY the corrected JSON value matching the required schema — no prose, no code fences.",
        errors.join("\n")
    )
}

fn child_depth() -> usize {
    current_subagent_depth().saturating_add(1)
}

fn current_subagent_depth() -> usize {
    std::env::var("PI_SUBAGENT_DEPTH")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum SubagentStatus {
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl SubagentStatus {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubagentResult {
    agent: String,
    description: Option<String>,
    task: String,
    step: Option<usize>,
    source: Option<AgentSource>,
    definition_path: Option<PathBuf>,
    model: Option<String>,
    reasoning: Option<String>,
    tools: Vec<String>,
    cwd: PathBuf,
    binary: PathBuf,
    pid: Option<u32>,
    status: SubagentStatus,
    exit_code: Option<i32>,
    output: String,
    stderr: String,
    error: Option<String>,
    /// Parsed final output when an `outputSchema` validated it (bd-cv653.5.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    /// `Some(true)`/`Some(false)` when a schema applied; `None` otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_errors: Option<Vec<String>>,
    /// Corrective retries consumed (bounded to one).
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_retries: Option<usize>,
    /// Worktree-isolation outcome (bd-cv653.5.2) when the task ran isolated.
    #[serde(skip_serializing_if = "Option::is_none")]
    iso: Option<crate::worktree_iso::IsoOutcome>,
    session_isolation: &'static str,
    /// Agent-hub run id (bd-cv653.5.3) when the registry tracked this run.
    /// Serialized so callers can chain `hubId → continue` without parsing
    /// hub transcript. Persisted sessions make this id stable across turns.
    hub_id: Option<String>,
    #[serde(skip)]
    is_error: bool,
}

impl SubagentResult {
    fn starting(
        agent: &AgentDefinition,
        task: SubagentTask,
        step: Option<usize>,
        binary: &Path,
        cwd: &Path,
        _args: &[OsString],
    ) -> Self {
        Self {
            agent: agent.name.clone(),
            description: Some(agent.description.clone()),
            task: task.task,
            step,
            source: Some(agent.source),
            definition_path: Some(agent.file_path.clone()),
            model: agent.model.clone(),
            reasoning: agent.reasoning.clone(),
            tools: agent
                .tools
                .clone()
                .unwrap_or_else(|| split_csv(DEFAULT_CHILD_TOOLS)),
            cwd: cwd.to_path_buf(),
            binary: binary.to_path_buf(),
            pid: None,
            status: SubagentStatus::Starting,
            exit_code: None,
            output: String::new(),
            stderr: String::new(),
            error: None,
            data: None,
            schema_valid: None,
            validation_errors: None,
            schema_retries: None,
            iso: None,
            session_isolation: "ephemeral_no_session",
            hub_id: None,
            is_error: false,
        }
    }

    fn unknown(task: SubagentTask, step: Option<usize>) -> Self {
        Self {
            agent: task.agent.clone(),
            description: None,
            task: task.task,
            step,
            source: None,
            definition_path: None,
            model: None,
            reasoning: None,
            tools: Vec::new(),
            cwd: task.cwd.unwrap_or_default(),
            binary: PathBuf::new(),
            pid: None,
            status: SubagentStatus::Failed,
            exit_code: None,
            output: String::new(),
            stderr: String::new(),
            error: Some(format!("Unknown agent: {}", task.agent)),
            data: None,
            schema_valid: None,
            validation_errors: None,
            schema_retries: None,
            iso: None,
            session_isolation: "ephemeral_no_session",
            hub_id: None,
            is_error: true,
        }
    }

    fn failed(
        agent: &AgentDefinition,
        task: SubagentTask,
        step: Option<usize>,
        error: String,
    ) -> Self {
        let mut result = Self::starting(agent, task, step, Path::new(""), Path::new(""), &[]);
        result.fail(error);
        result
    }

    fn fail(&mut self, error: String) {
        self.status = SubagentStatus::Failed;
        self.error = Some(error);
        self.is_error = true;
    }
}

fn render_results(results: &[SubagentResult]) -> String {
    results
        .iter()
        .map(|result| {
            let heading = result.step.map_or_else(
                || result.agent.clone(),
                |step| format!("step {step}: {}", result.agent),
            );
            let body = if result.output.trim().is_empty() {
                result.error.as_deref().unwrap_or("(no output)")
            } else {
                result.output.trim()
            };
            format!("## {heading}\n{body}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Truncate `value` to at most `limit` bytes (on a char boundary), appending
/// an explicit marker when anything was cut.
fn truncated_field(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }
    let mut cut = limit;
    while !value.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}{STRUCTURED_TRUNCATION_MARKER}", &value[..cut])
}

/// Compact per-child entry for the opt-in structured block.
///
/// Field names deliberately match the `pi.subagent.result.v1` details schema
/// (`agent`, `step`, `status`, `exitCode`, `output`, `error`).
fn structured_result_entry(result: &SubagentResult) -> Value {
    json!({
        "agent": result.agent,
        "step": result.step,
        "status": result.status,
        "exitCode": result.exit_code,
        "output": truncated_field(&result.output, STRUCTURED_FIELD_LIMIT_BYTES),
        "error": result
            .error
            .as_deref()
            .map(|error| truncated_field(error, STRUCTURED_FIELD_LIMIT_BYTES)),
    })
}

/// Render the opt-in `<subagent-structured-result>` block: a JSON array of
/// per-child entries, capped at [`STRUCTURED_BLOCK_LIMIT_BYTES`].  When the
/// cap forces entries to be dropped, the final array element is an explicit
/// `{"truncated": true, "omittedResults": N}` marker.
///
/// Every `<` in the JSON body is escaped as the JSON unicode escape
/// `\\u003c` (identical after JSON parsing; in serialized JSON `<` can only
/// occur inside string literals) so child output containing
/// `</subagent-structured-result>` cannot inject a premature closing tag:
/// the wrapper tags are the only literal `<` bytes in the block.
fn structured_result_block(results: &[SubagentResult]) -> String {
    let mut entries: Vec<Value> = results.iter().map(structured_result_entry).collect();
    let mut omitted = 0usize;
    loop {
        let mut rendered = entries.clone();
        if omitted > 0 {
            rendered.push(json!({"truncated": true, "omittedResults": omitted}));
        }
        let body = serde_json::to_string(&rendered)
            .unwrap_or_else(|_| "[]".to_string())
            .replace('<', "\\u003c");
        if body.len() <= STRUCTURED_BLOCK_LIMIT_BYTES || entries.is_empty() {
            return format!("{STRUCTURED_BLOCK_OPEN}{body}{STRUCTURED_BLOCK_CLOSE}");
        }
        entries.pop();
        omitted += 1;
    }
}

fn emit_progress(update: Option<&UpdateCallback>, result: &SubagentResult) {
    let Some(update) = update else {
        return;
    };
    let preview = if result.output.trim().is_empty() {
        format!("{}: {:?}", result.agent, result.status)
    } else {
        format!("{}: {}", result.agent, result.output.trim())
    };
    update(ToolUpdate {
        content: vec![ContentBlock::Text(TextContent::new(preview))],
        details: Some(json!({
            "schema": SUBAGENT_PROGRESS_SCHEMA,
            "result": result,
        })),
    });
}

#[derive(Debug, Clone, Copy)]
enum PipeKind {
    Stdout,
    Stderr,
}

struct PipeFrame {
    kind: PipeKind,
    line: String,
}

fn spawn_pipe_reader<R: Read + Send + 'static>(
    pipe: R,
    kind: PipeKind,
    tx: mpsc::SyncSender<PipeFrame>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let reader = BufReader::new(pipe);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if tx.send(PipeFrame { kind, line }).is_err() {
                break;
            }
        }
    })
}

fn drain_child_frames(
    rx: &Receiver<PipeFrame>,
    result: &mut SubagentResult,
    update: Option<&UpdateCallback>,
) {
    while let Ok(frame) = rx.try_recv() {
        // Hub transcript persistence (bd-cv653.5.3): raw stdout frames land
        // in the child's session-scoped transcript file for roster paging.
        if matches!(frame.kind, PipeKind::Stdout)
            && let (Some(hub_id), Ok(mut reg)) =
                (result.hub_id.as_ref(), crate::agent_hub::registry().lock())
        {
            reg.append_transcript(hub_id, &frame.line);
        }
        match frame.kind {
            PipeKind::Stderr => append_bounded_line(&mut result.stderr, &frame.line),
            PipeKind::Stdout => ingest_child_event(&frame.line, result, update),
        }
    }
}

async fn drain_until_reader_exit(
    rx: Receiver<PipeFrame>,
    result: &mut SubagentResult,
    update: Option<&UpdateCallback>,
    stdout: thread::JoinHandle<()>,
    stderr: thread::JoinHandle<()>,
) {
    for _ in 0..500 {
        drain_child_frames(&rx, result, update);
        if stdout.is_finished() && stderr.is_finished() {
            break;
        }
        let now = asupersync::time::wall_now();
        asupersync::time::sleep(now, Duration::from_millis(10)).await;
    }
    drain_child_frames(&rx, result, update);
}

fn ingest_child_event(line: &str, result: &mut SubagentResult, update: Option<&UpdateCallback>) {
    let Ok(event) = serde_json::from_str::<Value>(line) else {
        append_bounded_line(&mut result.stderr, line);
        return;
    };
    match event.get("type").and_then(Value::as_str) {
        Some("message_update") => {
            if let Some(delta) = event
                .pointer("/assistantMessageEvent/delta")
                .and_then(Value::as_str)
            {
                append_bounded(&mut result.output, delta);
                emit_progress(update, result);
            }
        }
        Some("message_end") => {
            if let Some(text) = assistant_text(event.get("message")) {
                if result.output.is_empty() {
                    append_bounded(&mut result.output, &text);
                }
                emit_progress(update, result);
            }
        }
        Some("agent_end") => {
            if result.output.is_empty()
                && let Some(messages) = event.get("messages").and_then(Value::as_array)
            {
                for message in messages.iter().rev() {
                    if let Some(text) = assistant_text(Some(message)) {
                        append_bounded(&mut result.output, &text);
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

fn assistant_text(message: Option<&Value>) -> Option<String> {
    let message = message?;
    if message.get("role").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    let content = message.get("content")?.as_array()?;
    content.iter().find_map(|block| {
        (block.get("type").and_then(Value::as_str) == Some("text"))
            .then(|| {
                block
                    .get("text")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .flatten()
    })
}

fn append_bounded(target: &mut String, value: &str) {
    if target.len() >= MAX_CHILD_OUTPUT_BYTES {
        return;
    }
    let remaining = MAX_CHILD_OUTPUT_BYTES.saturating_sub(target.len());
    if value.len() <= remaining {
        target.push_str(value);
    } else {
        let mut cut = remaining;
        while !value.is_char_boundary(cut) {
            cut -= 1;
        }
        target.push_str(&value[..cut]);
        target.push_str("\n[output truncated]\n");
    }
}

fn append_bounded_line(target: &mut String, value: &str) {
    append_bounded(target, value);
    if !target.ends_with('\n') && target.len() < MAX_CHILD_OUTPUT_BYTES {
        target.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn write_agent(dir: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(dir).expect("create agent dir");
        std::fs::write(dir.join(format!("{name}.md")), body).expect("write agent");
    }

    #[test]
    fn project_agents_override_user_and_parse_runtime_configuration() {
        let temp = TempDir::new().expect("tempdir");
        let global = temp.path().join("global");
        let cwd = temp.path().join("workspace").join("nested");
        write_agent(
            &global.join("agents"),
            "scout",
            "---\nname: scout\ndescription: user\nmodel: provider/user\nreasoning: low\ntools: read,grep\nskills: one.md,two.md\n---\nuser prompt",
        );
        write_agent(
            &cwd.parent().expect("parent").join(".pi/agents"),
            "scout",
            "---\nname: scout\ndescription: project\nmodel: provider/project\nthinking: high\ntools: read,find\n---\nproject prompt",
        );
        let agents = discover_agents_with_roots(&cwd, &global, AgentScope::Both).expect("discover");
        let scout = agents.get("scout").expect("project scout");
        assert_eq!(scout.description, "project");
        assert_eq!(scout.model.as_deref(), Some("provider/project"));
        assert_eq!(scout.reasoning.as_deref(), Some("high"));
        assert_eq!(
            scout.tools.as_deref(),
            Some(["read".to_string(), "find".to_string()].as_slice())
        );
        assert_eq!(scout.system_prompt, "project prompt");
        assert!(matches!(scout.source, AgentSource::Project));
    }

    /// bd-cv653.3.1: agent-def `model:` pin beats the role spec; the role
    /// spec is used only when the definition has no pin; no spec at all keeps
    /// the ambient-inheritance behavior (no --model passed).
    #[test]
    fn child_args_role_model_precedence() {
        let base = AgentDefinition {
            name: "scout".to_string(),
            description: "inspect".to_string(),
            model: None,
            reasoning: None,
            tools: None,
            skills: Vec::new(),
            system_prompt: String::new(),
            output_schema: None,
            source: AgentSource::User,
            file_path: PathBuf::from("/tmp/scout.md"),
        };
        let args_of = |agent: &AgentDefinition, spec: Option<&str>| {
            child_args(agent, "inspect provider", spec, None, None)
                .iter()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        };
        let model_value = |args: &[String]| {
            args.windows(2)
                .find(|pair| pair[0] == "--model")
                .map(|pair| pair[1].clone())
        };

        // No pin + role spec → role spec is used (task/smol resolution
        // happens at the registry; here we only prove the wire shape).
        let args = args_of(&base, Some("openai/gpt-5-mini:low"));
        assert_eq!(
            model_value(&args).as_deref(),
            Some("openai/gpt-5-mini:low"),
            "role spec must be passed as --model when the agent def has no pin"
        );

        // Pin present → pin wins over the role spec.
        let pinned = AgentDefinition {
            model: Some("ai-router/gpt-5.6-sol".to_string()),
            ..base.clone()
        };
        let args = args_of(&pinned, Some("openai/gpt-5-mini:low"));
        assert_eq!(
            model_value(&args).as_deref(),
            Some("ai-router/gpt-5.6-sol"),
            "agent-def model pin must beat the role spec"
        );

        // No pin and no spec → no --model flag at all (ambient inheritance).
        let args = args_of(&base, None);
        assert!(
            model_value(&args).is_none(),
            "no role spec and no pin must not inject --model"
        );
    }

    #[test]
    fn child_args_keep_model_effort_tools_skills_and_prompt() {
        let agent = AgentDefinition {
            name: "scout".to_string(),
            description: "inspect".to_string(),
            model: Some("ai-router/gpt-5.6-sol".to_string()),
            reasoning: Some("high".to_string()),
            tools: Some(vec!["read".to_string(), "grep".to_string()]),
            skills: vec![PathBuf::from("/tmp/skill.md")],
            system_prompt: "be precise".to_string(),
            output_schema: None,
            source: AgentSource::User,
            file_path: PathBuf::from("/tmp/scout.md"),
        };
        let args = child_args(&agent, "inspect provider", None, None, None)
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--model", "ai-router/gpt-5.6-sol"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--thinking", "high"]));
        assert!(args.windows(2).any(|pair| pair == ["--tools", "read,grep"]));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--skill", "/tmp/skill.md"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--append-system-prompt", "be precise"])
        );
        assert_eq!(
            args.last().map(String::as_str),
            Some("Task: inspect provider")
        );
    }

    #[test]
    fn tan_definition_uses_task_role_and_default_non_recursive_tools() {
        let args = child_args(
            &tan_agent_definition(),
            "update the changelog",
            Some("ai-router/gpt-5.6-terra"),
            None,
            None,
        )
        .iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--model", "ai-router/gpt-5.6-terra"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--tools", DEFAULT_CHILD_TOOLS])
        );
        assert!(!args.iter().any(|arg| arg == "subagent"));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--append-system-prompt", TAN_SYSTEM_PROMPT])
        );
        assert_eq!(
            args.last().map(String::as_str),
            Some("Task: update the changelog")
        );
    }

    #[test]
    fn tan_completion_has_bounded_follow_up_and_distinct_card_shape() {
        let task = SubagentTask {
            agent: TAN_AGENT_NAME.to_string(),
            task: "update the changelog".to_string(),
            cwd: None,
            isolation: None,
            iso_apply: None,
            output_schema: None,
            schema_mode: SchemaMode::default(),
            continue_flag: None,
            hub_id: None,
        };
        let mut result = SubagentResult::unknown(task, None);
        result.hub_id = Some("tan-7".to_string());
        result.status = SubagentStatus::Completed;
        result.output = "changelog updated".to_string();
        result.error = None;
        result.is_error = false;
        let completion = TanCompletion::from_result(result);
        assert_eq!(completion.schema, TAN_RESULT_SCHEMA);
        assert_eq!(completion.hub_id.as_deref(), Some("tan-7"));
        assert_eq!(
            completion.follow_up_text(),
            "[background tan tan-7 settled: completed]\nwork: update the changelog\nsummary:\nchangelog updated"
        );
        assert!(completion.card_text().starts_with("(/tan completed)\n"));
    }

    #[test]
    fn request_requires_exactly_one_mode_and_renders_chain_context() {
        let invalid: SubagentRequest = serde_json::from_value(json!({
            "agent": "scout", "task": "x", "tasks": [{"agent": "review", "task": "y"}]
        }))
        .expect("parse");
        assert!(invalid.mode().is_err());
        let task = SubagentTask {
            agent: "review".to_string(),
            task: concat!("review {", "previous}").to_string(),
            cwd: None,
            isolation: None,
            iso_apply: None,
            output_schema: None,
            schema_mode: SchemaMode::default(),
            continue_flag: None,
            hub_id: None,
        };
        let mut previous = SubagentResult::unknown(task.clone(), None);
        previous.output = "evidence".to_string();
        assert_eq!(
            task.with_rendered_previous_result(Some(&previous)).task,
            "review evidence"
        );
    }

    /// bd-cv653.5.1: `{{previous.data.<path>}}` addresses the prior task's
    /// schema-validated data — scalars render bare, nested paths resolve via
    /// dots, and misses stay verbatim. Without a schema-valid prior result,
    /// tokens are untouched.
    #[test]
    fn chain_previous_data_field_addressing() {
        let base = SubagentTask {
            agent: "review".to_string(),
            task: "verdict {{previous.data.verdict}} n {{previous.data.stats.count}} miss {{previous.data.absent}}"
                .to_string(),
            cwd: None,
            isolation: None,
            iso_apply: None,
            output_schema: None,
            schema_mode: SchemaMode::default(),
            continue_flag: None,
            hub_id: None,
        };
        let mut previous = SubagentResult::unknown(base.clone(), None);
        previous.schema_valid = Some(true);
        previous.data = Some(json!({"verdict": "pass", "stats": {"count": 3}}));
        assert_eq!(
            base.clone()
                .with_rendered_previous_result(Some(&previous))
                .task,
            "verdict pass n 3 miss {{previous.data.absent}}"
        );

        // Not schema-valid → tokens untouched.
        let mut invalid = SubagentResult::unknown(base.clone(), None);
        invalid.schema_valid = Some(false);
        invalid.data = Some(json!({"verdict": "pass"}));
        assert!(
            base.with_rendered_previous_result(Some(&invalid))
                .task
                .contains("{{previous.data.verdict}}")
        );
    }

    /// bd-cv653.5.1: JSON extraction tolerates prose/fence wrapping, and
    /// validation reports keyword errors with instance paths.
    #[test]
    fn output_schema_validation_matrix() {
        let schema = json!({
            "type": "object",
            "required": ["verdict"],
            "properties": {"verdict": {"type": "string"}}
        });

        let valid = validate_child_output(r#"{"verdict": "pass"}"#, &schema);
        assert_eq!(valid.expect("valid")["verdict"], "pass");

        let fenced = validate_child_output(
            "Here you go:\n```json\n{\"verdict\": \"pass\"}\n```",
            &schema,
        );
        assert_eq!(fenced.expect("fenced")["verdict"], "pass");

        let embedded = validate_child_output(
            r#"The answer is {"verdict": "pass", "note": "{brace} inside"} — done."#,
            &schema,
        );
        assert_eq!(embedded.expect("embedded")["verdict"], "pass");

        let wrong_shape = validate_child_output(r#"{"verdict": 7}"#, &schema);
        let errors = wrong_shape.expect_err("type mismatch");
        assert!(
            errors.iter().any(|error| error.contains("verdict")),
            "{errors:?}"
        );

        let missing = validate_child_output(r#"{"other": true}"#, &schema);
        assert!(missing.is_err());

        let no_json = validate_child_output("no structured output here", &schema);
        assert_eq!(
            no_json.expect_err("no json"),
            vec!["child output contains no parseable JSON value".to_string()]
        );
    }

    /// bd-cv653.5.1: an agent definition may carry a single-line JSON
    /// `output_schema:`; invalid JSON there is a load-time error.
    #[test]
    fn agent_definition_output_schema_parses_and_rejects_invalid() {
        let temp = TempDir::new().expect("tempdir");
        let global = temp.path().join("global");
        write_agent(
            &global.join("agents"),
            "typed",
            "---\nname: typed\ndescription: typed agent\noutput_schema: {\"type\": \"object\"}\n---\nbody",
        );
        let agents = discover_agents_with_roots(temp.path(), &global, AgentScope::User)
            .expect("discover typed agent");
        assert_eq!(
            agents.get("typed").expect("typed").output_schema,
            Some(json!({"type": "object"}))
        );

        write_agent(
            &global.join("agents"),
            "broken",
            "---\nname: broken\ndescription: broken agent\noutput_schema: {not json\n---\nbody",
        );
        let error = discover_agents_with_roots(temp.path(), &global, AgentScope::User)
            .expect_err("invalid schema must fail agent loading");
        assert!(error.to_string().contains("output_schema"), "{error}");
    }

    #[test]
    fn tool_schema_advertises_all_three_workflows() {
        let tool =
            SubagentTool::with_paths(PathBuf::from("."), PathBuf::from("."), PathBuf::from("pi"));
        let schema = tool.parameters();
        assert!(schema["properties"].get("agent").is_some());
        assert!(schema["properties"].get("tasks").is_some());
        assert!(schema["properties"].get("chain").is_some());
    }

    fn execute_unknown_agent(structured: bool) -> ToolOutput {
        let temp = TempDir::new().expect("tempdir");
        let tool = SubagentTool::with_paths(
            temp.path().to_path_buf(),
            temp.path().join("global"),
            PathBuf::from("pi"),
        )
        .with_structured_results(structured);
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");
        runtime
            .block_on(tool.execute(
                "subagent-structured",
                json!({"agent": "scout", "task": "inspect"}),
                None,
                None,
            ))
            .expect("execute returns tool output")
    }

    fn output_text(output: &ToolOutput) -> String {
        let ContentBlock::Text(text) = &output.content[0] else {
            panic!("expected text output");
        };
        text.text.clone()
    }

    #[test]
    fn structured_block_disabled_by_default_keeps_output_byte_identical() {
        let output = execute_unknown_agent(false);
        let text = output_text(&output);
        assert_eq!(text, "## scout\nUnknown agent: scout");
        assert!(!text.contains(STRUCTURED_BLOCK_OPEN));
        assert!(output.is_error);
    }

    #[test]
    fn structured_block_appends_parseable_json_matching_details() {
        let output = execute_unknown_agent(true);
        let text = output_text(&output);
        let prefix = "## scout\nUnknown agent: scout\n\n";
        assert!(text.starts_with(prefix), "unexpected text: {text}");
        let block = &text[prefix.len()..];
        let body = block
            .strip_prefix(STRUCTURED_BLOCK_OPEN)
            .and_then(|rest| rest.strip_suffix(STRUCTURED_BLOCK_CLOSE))
            .expect("structured block is fenced");
        let parsed: Value = serde_json::from_str(body).expect("block payload parses as JSON");
        let entries = parsed.as_array().expect("payload is an array");
        assert_eq!(entries.len(), 1);
        let details = output.details.as_ref().expect("details present");
        assert_eq!(entries[0]["agent"], details["results"][0]["agent"]);
        assert_eq!(entries[0]["status"], details["results"][0]["status"]);
        assert_eq!(entries[0]["error"], details["results"][0]["error"]);
        assert_eq!(entries[0]["status"], "failed");
        assert_eq!(entries[0]["exitCode"], Value::Null);
    }

    #[test]
    fn structured_block_truncates_fields_and_caps_block() {
        let task = |name: &str| SubagentTask {
            agent: name.to_string(),
            task: "t".to_string(),
            cwd: None,
            isolation: None,
            iso_apply: None,
            output_schema: None,
            schema_mode: SchemaMode::default(),
            continue_flag: None,
            hub_id: None,
        };
        let mut long = SubagentResult::unknown(task("long"), None);
        long.output = "x".repeat(10 * 1024);
        let entry = structured_result_entry(&long);
        let rendered_output = entry["output"].as_str().expect("output is a string");
        assert!(rendered_output.ends_with(STRUCTURED_TRUNCATION_MARKER));
        assert!(
            rendered_output.len()
                <= STRUCTURED_FIELD_LIMIT_BYTES + STRUCTURED_TRUNCATION_MARKER.len()
        );

        let results: Vec<SubagentResult> = (0..20)
            .map(|index| {
                let mut result = SubagentResult::unknown(task(&format!("agent-{index}")), None);
                result.output = "y".repeat(4 * 1024);
                result
            })
            .collect();
        let block = structured_result_block(&results);
        let body = block
            .strip_prefix(STRUCTURED_BLOCK_OPEN)
            .and_then(|rest| rest.strip_suffix(STRUCTURED_BLOCK_CLOSE))
            .expect("capped block is fenced");
        assert!(body.len() <= STRUCTURED_BLOCK_LIMIT_BYTES);
        let parsed: Value = serde_json::from_str(body).expect("capped payload parses");
        let entries = parsed.as_array().expect("capped payload is an array");
        let marker = entries.last().expect("array is non-empty");
        assert_eq!(marker["truncated"], Value::Bool(true));
        let omitted = marker["omittedResults"]
            .as_u64()
            .expect("omittedResults present");
        assert!(omitted > 0);
        assert_eq!(
            entries.len() - 1 + usize::try_from(omitted).expect("fits"),
            20
        );
    }

    #[test]
    fn structured_block_escapes_close_tag_in_child_output() {
        let task = SubagentTask {
            agent: "inj".to_string(),
            task: "t".to_string(),
            cwd: None,
            isolation: None,
            iso_apply: None,
            output_schema: None,
            schema_mode: SchemaMode::default(),
            continue_flag: None,
            hub_id: None,
        };
        let mut result = SubagentResult::unknown(task, None);
        result.output = format!("before {STRUCTURED_BLOCK_CLOSE} after");
        let block = structured_result_block(&[result]);

        // The wrapper close tag must be the only literal close tag: child
        // output cannot inject a premature terminator.
        assert_eq!(block.matches(STRUCTURED_BLOCK_CLOSE).count(), 1);
        assert!(block.ends_with(STRUCTURED_BLOCK_CLOSE));
        let body = block
            .strip_prefix(STRUCTURED_BLOCK_OPEN)
            .and_then(|rest| rest.strip_suffix(STRUCTURED_BLOCK_CLOSE))
            .expect("block is fenced");
        assert!(
            !body.contains('<'),
            "JSON body must not contain literal '<'"
        );

        // The escaping is lossless: parsing yields the original output.
        let parsed: Value = serde_json::from_str(body).expect("payload parses");
        assert_eq!(
            parsed[0]["output"],
            Value::String(format!("before {STRUCTURED_BLOCK_CLOSE} after"))
        );
    }

    #[test]
    #[cfg(unix)]
    fn child_process_uses_selected_binary_streams_json_and_inherits_global_dir() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        write_agent(
            &global_dir.join("agents"),
            "scout",
            "---\nname: scout\ndescription: child-process fixture\n---\nReturn the child result.",
        );

        let child = temp.path().join("child-fixture.sh");
        std::fs::write(
            &child,
            r#"#!/bin/sh
printf '{"type":"message_update","assistantMessageEvent":{"delta":"streamed:"}}\n'
printf '{"type":"message_update","assistantMessageEvent":{"delta":"%s"}}\n' "$PI_CODING_AGENT_DIR"
printf '{"type":"agent_end","messages":[{"role":"assistant","content":[{"type":"text","text":"final child result"}]}]}\n'
"#,
        )
        .expect("write child fixture");
        let mut permissions = std::fs::metadata(&child)
            .expect("child metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&child, permissions).expect("make child executable");

        let tool =
            SubagentTool::with_paths(temp.path().to_path_buf(), global_dir.clone(), child.clone());
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");
        let output = runtime
            .block_on(tool.execute(
                "subagent-fixture",
                json!({"agent": "scout", "task": "verify child protocol"}),
                None,
                None,
            ))
            .expect("child execution succeeds");

        let ContentBlock::Text(text) = &output.content[0] else {
            panic!("expected text output");
        };
        assert!(text.text.contains("streamed:"));
        assert!(text.text.contains(global_dir.to_string_lossy().as_ref()));
        assert!(
            output.details.as_ref().is_some_and(|details| {
                details["results"][0]["binary"] == Value::String(child.display().to_string())
                    && details["results"][0]["status"] == "completed"
                    && details["sessionIsolation"] == "persistent_session"
            }),
            "missing child-process evidence: {output:?}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn background_tan_runs_child_and_registers_tan_hub_kind() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        let child = temp.path().join("tan-child-fixture.sh");
        std::fs::write(
            &child,
            r#"#!/bin/sh
sleep 1
printf '{"type":"agent_end","messages":[{"role":"assistant","content":[{"type":"text","text":"tan fixture completed"}]}]}\n'
"#,
        )
        .expect("write tan child fixture");
        let mut permissions = std::fs::metadata(&child)
            .expect("tan child metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&child, permissions).expect("make tan child executable");

        let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir, child)
            .with_role_model_spec(Some("test-provider/task-model".to_string()));
        let task = format!("tan-fixture-{}", std::process::id());
        let child_task = task.clone();
        let handle = std::thread::spawn(move || {
            let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
                .build()
                .expect("runtime build");
            runtime
                .block_on(tool.run_background_tan(&child_task))
                .expect("tan child execution succeeds")
        });

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let running_entry = loop {
            let candidate = crate::agent_hub::registry()
                .lock()
                .expect("hub registry lock")
                .roster()
                .into_iter()
                .find(|entry| {
                    entry.task == task && entry.status == crate::agent_hub::ChildStatus::Running
                });
            if let Some(entry) = candidate {
                break entry;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "tan child never appeared in the running hub roster"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(running_entry.kind, crate::agent_hub::ChildKind::Tan);

        let completion = handle.join().expect("join tan child thread");

        assert!(!completion.is_error, "{completion:?}");
        assert_eq!(completion.status, "completed");
        assert!(completion.output.contains("tan fixture completed"));
        let hub_id = completion.hub_id.expect("tan completion has hub id");
        let entry = crate::agent_hub::registry()
            .lock()
            .expect("hub registry lock")
            .get(&hub_id)
            .expect("tan child remains in roster");
        assert_eq!(entry.kind, crate::agent_hub::ChildKind::Tan);
        assert_eq!(entry.status, crate::agent_hub::ChildStatus::Done);
        assert_eq!(entry.task, task);
    }

    /// Write an executable stub child that emits `first` on its first run and
    /// `second` from then on (state via a marker file next to the script).
    #[cfg(unix)]
    fn write_two_phase_child(temp: &Path, first: &str, second: &str) -> PathBuf {
        let child = temp.join("two-phase-child.sh");
        let marker = temp.join("two-phase-marker");
        // `printf '%s\n' '<line>'` passes the JSON event through untouched:
        // no printf escape processing, and the single-quoted argument may
        // freely contain double quotes and backslashes (`first`/`second` are
        // already JSON-string-escaped payloads for the `text` field).
        std::fs::write(
            &child,
            format!(
                r#"#!/bin/sh
if [ -f "{marker}" ]; then
  printf '%s\n' '{{"type":"agent_end","messages":[{{"role":"assistant","content":[{{"type":"text","text":"{second}"}}]}}]}}'
else
  : > "{marker}"
  printf '%s\n' '{{"type":"agent_end","messages":[{{"role":"assistant","content":[{{"type":"text","text":"{first}"}}]}}]}}'
fi
"#,
                marker = marker.display(),
            ),
        )
        .expect("write two-phase child");
        let mut permissions = std::fs::metadata(&child)
            .expect("child metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&child, permissions).expect("make child executable");
        child
    }

    /// bd-cv653.5.1 acceptance 1+2 (happy half): invalid first output, one
    /// corrective retry, valid second output → parsed `data`, `schemaValid`
    /// true, one recorded retry, task not an error.
    #[test]
    #[cfg(unix)]
    fn output_schema_retry_then_valid_yields_parsed_data() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        write_agent(
            &global_dir.join("agents"),
            "typed",
            "---\nname: typed\ndescription: typed child\n---\nReturn JSON.",
        );
        let child =
            write_two_phase_child(temp.path(), "not json at all", r#"{\"verdict\": \"pass\"}"#);

        let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir, child);
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");
        let output = runtime
            .block_on(tool.execute(
                "typed-retry",
                json!({
                    "agent": "typed",
                    "task": "produce a verdict",
                    "outputSchema": {
                        "type": "object",
                        "required": ["verdict"],
                        "properties": {"verdict": {"type": "string"}}
                    }
                }),
                None,
                None,
            ))
            .expect("typed subagent run");

        assert!(!output.is_error, "{output:?}");
        let details = output.details.expect("details");
        let result = &details["results"][0];
        assert_eq!(result["schemaValid"], true, "{result}");
        assert_eq!(result["schemaRetries"], 1, "{result}");
        assert_eq!(result["data"]["verdict"], "pass", "{result}");
        assert_eq!(details["schema"], SUBAGENT_RESULT_SCHEMA);
    }

    /// bd-cv653.5.1 acceptance 2 (exhaustion half): output stays invalid
    /// through the single retry — permissive keeps the raw result with
    /// warning fields; strict fails the task with the validation errors.
    #[test]
    #[cfg(unix)]
    fn output_schema_exhaustion_permissive_warns_strict_fails() {
        for (mode, expect_error) in [("permissive", false), ("strict", true)] {
            let temp = TempDir::new().expect("tempdir");
            let global_dir = temp.path().join("global");
            write_agent(
                &global_dir.join("agents"),
                "typed",
                "---\nname: typed\ndescription: typed child\n---\nReturn JSON.",
            );
            let child = write_two_phase_child(temp.path(), "still not json", "also not json");

            let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir, child);
            let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
                .build()
                .expect("runtime build");
            let output = runtime
                .block_on(tool.execute(
                    "typed-exhausted",
                    json!({
                        "agent": "typed",
                        "task": "produce a verdict",
                        "outputSchema": {"type": "object"},
                        "schemaMode": mode
                    }),
                    None,
                    None,
                ))
                .expect("typed subagent run");

            assert_eq!(output.is_error, expect_error, "mode={mode}: {output:?}");
            let details = output.details.expect("details");
            let result = &details["results"][0];
            assert_eq!(result["schemaValid"], false, "mode={mode}: {result}");
            assert_eq!(result["schemaRetries"], 1, "mode={mode}: {result}");
            assert!(
                result["validationErrors"]
                    .as_array()
                    .is_some_and(|errors| !errors.is_empty()),
                "mode={mode}: {result}"
            );
            assert_eq!(
                result["status"],
                if expect_error { "failed" } else { "completed" },
                "mode={mode}: {result}"
            );
        }
    }

    // ——— Network retry (IMPLEMENT C4) ———

    #[cfg(unix)]
    fn write_network_retry_child(temp: &Path, retryable: bool) -> PathBuf {
        let child = temp.join("network-retry-child.sh");
        let marker = temp.join("network-retry-marker");
        let first_text = if retryable {
            "fetch failed: transient connection drop"
        } else {
            "prompt is too long: not retryable"
        };
        std::fs::write(
            &child,
            format!(
                r#"#!/bin/sh
if [ -f "{marker}" ]; then
  printf '%s\n' '{{"type":"agent_end","messages":[{{"role":"assistant","content":[{{"type":"text","text":"recovered"}}]}}]}}'
  exit 0
else
  : > "{marker}"
  printf '%s\n' '{{"type":"agent_end","messages":[{{"role":"assistant","content":[{{"type":"text","text":"{first_text}"}}]}}]}}' >&2
  printf '%s\n' '{first_text}' >&2
  exit 1
fi
"#,
                marker = marker.display(),
                first_text = first_text,
            ),
        )
        .expect("write network retry child");
        let mut permissions = std::fs::metadata(&child)
            .expect("child metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&child, permissions).expect("make child executable");
        child
    }

    #[test]
    #[cfg(unix)]
    fn network_retry_transient_succeeds_on_second_attempt() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        write_agent(
            &global_dir.join("agents"),
            "scout",
            "---\nname: scout\ndescription: network retry\n---\nbody",
        );
        let child = write_network_retry_child(temp.path(), true);
        let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir, child)
            .with_retry_config_for_test(SubagentRetryConfig {
                enabled: true,
                max_retries: 3,
                base_delay_ms: 1,
                max_delay_ms: 5,
            });
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");
        let output = runtime
            .block_on(tool.execute(
                "net-retry",
                json!({"agent": "scout", "task": "do work"}),
                None,
                None,
            ))
            .expect("network retry run");
        assert!(!output.is_error, "{output:?}");
        let details = output.details.expect("details");
        assert_eq!(details["results"][0]["status"], "completed");
        // Second attempt's output wins.
        let ContentBlock::Text(text) = &output.content[0] else {
            panic!("expected text");
        };
        assert!(text.text.contains("recovered"), "{text:?}");
    }

    #[test]
    #[cfg(unix)]
    fn network_retry_non_retryable_does_not_retry() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        write_agent(
            &global_dir.join("agents"),
            "scout",
            "---\nname: scout\ndescription: network retry\n---\nbody",
        );
        let child = write_network_retry_child(temp.path(), false);
        let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir, child)
            .with_retry_config_for_test(SubagentRetryConfig {
                enabled: true,
                max_retries: 3,
                base_delay_ms: 1,
                max_delay_ms: 5,
            });
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");
        let output = runtime
            .block_on(tool.execute(
                "net-no-retry",
                json!({"agent": "scout", "task": "do work"}),
                None,
                None,
            ))
            .expect("non-retryable run");
        assert!(output.is_error, "{output:?}");
        assert_eq!(
            output.details.as_ref().unwrap()["results"][0]["status"],
            "failed"
        );
        // Must not have recovered — still the first failure.
        let ContentBlock::Text(text) = &output.content[0] else {
            panic!("expected text");
        };
        assert!(!text.text.contains("recovered"), "{text:?}");
    }

    #[test]
    #[cfg(unix)]
    fn network_retry_disabled_does_not_retry() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        write_agent(
            &global_dir.join("agents"),
            "scout",
            "---\nname: scout\ndescription: network retry\n---\nbody",
        );
        let child = write_network_retry_child(temp.path(), true);
        let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir, child)
            .with_retry_config_for_test(SubagentRetryConfig {
                enabled: false,
                max_retries: 3,
                base_delay_ms: 1,
                max_delay_ms: 5,
            });
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");
        let output = runtime
            .block_on(tool.execute(
                "net-disabled",
                json!({"agent": "scout", "task": "do work"}),
                None,
                None,
            ))
            .expect("disabled retry run");
        assert!(output.is_error, "{output:?}");
        assert_eq!(
            output.details.as_ref().unwrap()["results"][0]["status"],
            "failed"
        );
    }

    // ——— Continuable subagent (C5) ———
    // Fresh→continue must reuse the same hubId/session/worktree. The child is a
    // tiny shell stub; persistence is verified via the hubId round-trip and the
    // `--session <global>/sessions/subagents/<hubId>.jsonl` argv.

    #[cfg(unix)]
    fn write_arg_logging_child(temp: &Path, log_path: &Path, output_text: &str) -> PathBuf {
        let child = temp.join("logging-child.sh");
        std::fs::write(
            &child,
            format!(
                r#"#!/bin/sh
echo "$@" > "{log}"
printf '{{"type":"agent_end","messages":[{{"role":"assistant","content":[{{"type":"text","text":"{out}"}}]}}]}}\n'
"#,
                log = log_path.display(),
                out = output_text,
            ),
        )
        .expect("write logging child");
        let mut permissions = std::fs::metadata(&child)
            .expect("child metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&child, permissions).expect("make child executable");
        child
    }

    /// Fresh delegation returns a persistent hubId and launches the child with
    /// `--session <global>/sessions/subagents/<hubId>.jsonl`.
    #[test]
    #[cfg(unix)]
    fn continuable_fresh_returns_persistent_hub_and_session_path() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        write_agent(
            &global_dir.join("agents"),
            "scout",
            "---\nname: scout\ndescription: continuable fresh\n---\nbody",
        );
        let log_path = temp.path().join("fresh-args.log");
        let child = write_arg_logging_child(temp.path(), &log_path, "fresh done");
        let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir.clone(), child);
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");
        let output = runtime
            .block_on(tool.execute(
                "continuable-fresh",
                json!({"agent": "scout", "task": "first task"}),
                None,
                None,
            ))
            .expect("fresh run");
        assert!(!output.is_error, "{output:?}");
        let details = output.details.expect("details");
        let hub_id = details["hubId"].as_str().expect("hubId string").to_string();
        assert!(!hub_id.is_empty(), "hubId must be returned");
        assert_eq!(
            details["sessionIsolation"], "persistent_session",
            "all subagent children are now persistent sessions"
        );
        assert_eq!(
            details["results"][0]["hubId"].as_str(),
            Some(hub_id.as_str()),
            "result hubId must mirror top-level hubId"
        );
        let expected = global_dir
            .join("sessions")
            .join("subagents")
            .join(format!("{hub_id}.jsonl"));
        assert!(
            expected.parent().is_some_and(|p| p.exists()),
            "session parent dir must be created: {}",
            expected.display()
        );
        let args = std::fs::read_to_string(&log_path).expect("args log");
        assert!(
            args.contains("--session"),
            "child must be launched with --session, got: {args}"
        );
        assert!(
            args.contains(&hub_id),
            "session path must contain hubId, got: {args}"
        );
        assert!(
            args.contains(expected.to_string_lossy().as_ref()),
            "session path must be canonical {}, got: {args}",
            expected.display()
        );
    }

    /// `continue=true` with the fresh hubId reuses the same session path and
    /// hubId; the child sees the same `--session` file rather than
    /// `no session file` failure.
    #[test]
    #[cfg(unix)]
    fn continuable_continue_reuses_same_hub_and_session() {
        let temp = TempDir::new().expect("tempdir");
        let global_dir = temp.path().join("global");
        write_agent(
            &global_dir.join("agents"),
            "scout",
            "---\nname: scout\ndescription: continuable continue\n---\nbody",
        );
        let log_path = temp.path().join("continue-args.log");
        let child = write_arg_logging_child(temp.path(), &log_path, "first done");
        let tool = SubagentTool::with_paths(temp.path().to_path_buf(), global_dir.clone(), child);
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build");

        // Fresh run — capture hubId.
        let first = runtime
            .block_on(tool.execute(
                "continuable-continue-fresh",
                json!({"agent": "scout", "task": "first task"}),
                None,
                None,
            ))
            .expect("fresh run");
        assert!(!first.is_error, "{first:?}");
        let hub_id = first.details.as_ref().expect("details")["hubId"]
            .as_str()
            .expect("hubId")
            .to_string();
        let expected = global_dir
            .join("sessions")
            .join("subagents")
            .join(format!("{hub_id}.jsonl"));
        // Materialise the session file so the `continue` guard
        // (`!path.exists() => Failed "no session file"`) passes. A real Pi child
        // would have created this JSONL; the stub does not.
        std::fs::create_dir_all(expected.parent().expect("session parent"))
            .expect("create session dir");
        std::fs::write(
            &expected,
            "{\"role\":\"user\",\"content\":\"first task\"}\n",
        )
        .expect("seed session file");
        let first_args = std::fs::read_to_string(&log_path).expect("first args");

        // Overwrite the stub so the second turn has distinct output.
        let child2 = write_arg_logging_child(temp.path(), &log_path, "second done");
        // The tool holds the child binary path; point it at the new stub content
        // by reusing the same file path (write_arg_logging_child overwrites it).
        let _ = child2;
        let tool2 = SubagentTool::with_paths(
            temp.path().to_path_buf(),
            global_dir.clone(),
            temp.path().join("logging-child.sh"),
        );

        let second = runtime
            .block_on(tool2.execute(
                "continuable-continue-second",
                json!({"agent": "scout", "task": "second task", "continue": true, "hubId": hub_id}),
                None,
                None,
            ))
            .expect("continue run");
        assert!(!second.is_error, "continue must not fail: {second:?}");
        let details = second.details.expect("details");
        assert_eq!(
            details["hubId"].as_str(),
            Some(hub_id.as_str()),
            "continue must preserve hubId"
        );
        assert_eq!(
            details["results"][0]["hubId"].as_str(),
            Some(hub_id.as_str())
        );
        assert_eq!(details["sessionIsolation"], "persistent_session");
        let second_args = std::fs::read_to_string(&log_path).expect("second args");
        assert!(
            second_args.contains("--session"),
            "continue child must be launched with --session, got: {second_args}"
        );
        assert!(
            second_args.contains(&hub_id),
            "continue session path must reuse hubId, got: {second_args}"
        );
        assert!(
            second_args.contains(expected.to_string_lossy().as_ref()),
            "continue must reuse canonical session path {}, got: {second_args}",
            expected.display()
        );
        // Both turns used the same session file location.
        assert!(
            first_args.contains(expected.to_string_lossy().as_ref())
                && second_args.contains(expected.to_string_lossy().as_ref()),
            "fresh and continue must share the same session path"
        );
        let ContentBlock::Text(text) = &second.content[0] else {
            panic!("expected text");
        };
        assert!(
            text.text.contains("second done"),
            "second turn output must be visible, got: {}",
            text.text
        );
        assert!(
            !text.text.contains("no session file"),
            "continue must not report missing session: {}",
            text.text
        );
    }
}
