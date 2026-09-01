//! Workspace isolation for subagents (bd-cv653.5.2, C3 continuable).
//!
//! Two flavours:
//!
//! * **ephemeral** (`isolate`) — `git worktree add` a temp branch at HEAD into
//!   `<tmp>/pi-iso-<id>`; the child runs there; on completion the parent
//!   collects `{worktree_path, diff_stat, patch}` and applies per
//!   `keep`/`apply`/`drop`. Temporary worktrees are always cleaned up
//!   (`drop_worktree` or reaped via `reap_stale`).
//!
//! * **persistent** (`worktree_path_for_hub` / `reopen_or_isolate`) —
//!   `hubId == sessionId == worktreeId`. The worktree lives at
//!   `<global_dir>/worktrees/subagents/<hubId>` and is **reused** across
//!   `subagent(continue=true, hubId, task)` turns: the first turn creates it
//!   (materialising the parent's dirty tree once and committing a baseline),
//!   later turns reopen the same directory so edits accumulate incrementally.
//!   A persistent worktree is **never** auto-removed on `collect_diff`/`Drop`;
//!   only an explicit `cleanup_persistent_worktree` (or `drop_worktree` with a
//!   persistent handle) removes it. This matches the `Done` continuable
//!   contract: the child process exits but Session+Worktree are retained.
//!
//! Dirty-tree strategy (round-7 correctness): `git worktree add` starts
//! from HEAD, so uncommitted parent changes would be invisible to isolated
//! children. `isolate`/`isolate_persistent` materializes the working-tree state
//! into the copy: a tracked-diff patch (`git diff HEAD`) applied into the
//! worktree plus an untracked-file copy list. CoW backends (APFS clonefile,
//! btrfs reflink, overlayfs) are the documented full-fidelity path when they
//! land — same interface.
//!
//! Cleanup: only `pi-iso-` prefix-tagged worktrees are ever reaped;
//! foreign worktrees are untouchable. Persistent worktrees live under
//! `global_dir/worktrees/subagents/<hubId>` whose basename is the hubId
//! (no prefix), so the stale reaper naturally skips them — they are managed
//! by explicit `cleanup_persistent_worktree`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::error::{Error, Result};

/// Tool-result schema tag for isolation outcomes.
pub const ISO_SCHEMA: &str = "pi.worktree_iso.v1";

/// Branch/path prefix that marks worktrees created by us. The reaper only
/// ever touches this prefix.
const ISO_PREFIX: &str = "pi-iso-";

/// What to do with the worktree after the child completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoApplyMode {
    /// Leave the worktree in place for the user.
    Keep,
    /// Apply the patch into the parent tree (serialized), then drop.
    Apply,
    /// Report the patch, then drop.
    Drop,
}

impl IsoApplyMode {
    pub fn parse(raw: Option<&str>) -> Result<Self> {
        match raw.unwrap_or("apply").trim().to_ascii_lowercase().as_str() {
            "keep" => Ok(Self::Keep),
            "apply" => Ok(Self::Apply),
            "drop" => Ok(Self::Drop),
            other => Err(Error::validation(format!(
                "Unknown iso apply mode '{other}'; expected keep, apply, or drop"
            ))),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Apply => "apply",
            Self::Drop => "drop",
        }
    }
}

/// A live isolated worktree.
#[derive(Debug)]
pub struct IsoHandle {
    pub id: String,
    pub branch: String,
    pub path: PathBuf,
    pub repo_root: PathBuf,
    /// Commit recording the materialized parent state (HEAD + dirty
    /// materialization). The child's own work is the diff from here.
    pub baseline: String,
    /// Whether this worktree is the persistent hubId worktree
    /// (`<global_dir>/worktrees/subagents/<hubId>`). Persistent worktrees are
    /// reused across `continue` turns and are never auto-removed — only an
    /// explicit `cleanup_persistent_worktree`/`drop_worktree` deletes them.
    pub persistent: bool,
}

/// The outcome of an isolated run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsoOutcome {
    pub schema: String,
    pub worktree_path: String,
    pub branch: String,
    pub diff_stat: String,
    /// The full unified diff (worktree state vs parent HEAD).
    pub patch: String,
    /// Files that failed to apply (conflict path only).
    pub conflicted_files: Vec<String>,
    pub apply_mode: String,
    pub applied: bool,
}

fn git(repo: &Path, args: &[&str]) -> Result<std::process::Output> {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| Error::tool("subagent", format!("Failed to run git: {e}")))
}

fn git_ok(repo: &Path, args: &[&str]) -> Result<String> {
    let output = git(repo, args)?;
    if !output.status.success() {
        return Err(Error::tool(
            "subagent",
            format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn sanitize_id(task_id: &str) -> String {
    let mut out = String::with_capacity(task_id.len().min(32));
    for ch in task_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed.chars().take(32).collect()
    }
}

/// Validate a hub/session id for filesystem safety.
///
/// Mirrors `subagents::is_valid_hub_id` / `agent_hub::is_valid_hub_id` — keeps
/// the three `hubId == sessionId == worktreeId` derivations in sync.
fn is_valid_hub_id(hub_id: &str) -> bool {
    let trimmed = hub_id.trim();
    !trimmed.is_empty()
        && !trimmed.contains('/')
        && !trimmed.contains('\\')
        && !trimmed.contains('\0')
        && trimmed != "."
        && trimmed != ".."
}

fn validate_hub_id(hub_id: &str) -> Result<()> {
    if !is_valid_hub_id(hub_id) {
        return Err(Error::tool(
            "subagent",
            format!("Invalid hubId: {hub_id:?}"),
        ));
    }
    Ok(())
}

/// Derive the persistent worktree directory for a hub.
///
/// Layout: `<global_dir>/worktrees/subagents/<hubId>` — the same
/// `hubId == sessionId == worktreeId` trinity as
/// `subagents::subagent_session_path` (`<global_dir>/sessions/subagents/<hubId>.jsonl`)
/// and `agent_hub::session_path_for_hub`. Callers that need validation should
/// check `hub_id` via [`try_worktree_path_for_hub`] or [`validate_hub_id`].
#[must_use]
pub fn worktree_path_for_hub(global_dir: &Path, hub_id: &str) -> PathBuf {
    global_dir.join("worktrees").join("subagents").join(hub_id)
}

/// Validated variant of [`worktree_path_for_hub`].
///
/// Returns `Error::tool("subagent", …)` when `hub_id` fails filesystem-safety
/// checks (empty, contains `/`/`\`/`\0`, or `.`/`..`).
///
/// # Errors
/// `Invalid hubId` when `hub_id` is not filesystem-safe.
pub fn try_worktree_path_for_hub(global_dir: &Path, hub_id: &str) -> Result<PathBuf> {
    validate_hub_id(hub_id)?;
    Ok(worktree_path_for_hub(global_dir, hub_id))
}

/// Derive the branch name for a persistent hub worktree.
///
/// Persistent branches are `pi-iso-<hubId>` so they remain prefix-tagged for
/// auditing even though the worktree *path* basename is the raw hubId.
#[must_use]
pub fn persistent_branch_for_hub(hub_id: &str) -> String {
    format!("{ISO_PREFIX}{hub_id}")
}

/// Whether a registered worktree entry matches the given path/branch.
///
/// Used to decide if an existing persistent directory can be reused.
fn is_worktree_registered(repo_root: &Path, worktree_path: &Path, branch: &str) -> bool {
    let Ok(porcelain) = git_ok(repo_root, &["worktree", "list", "--porcelain"]) else {
        return false;
    };
    let expected_branch = format!("refs/heads/{branch}");
    let worktree_str = worktree_path.to_string_lossy().to_string();
    let mut current_path: Option<String> = None;
    let mut current_branch = String::new();
    let mut found = false;
    for line in porcelain.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            // flush previous entry
            if let Some(path) = current_path.take() {
                if path == worktree_str || current_branch == expected_branch {
                    found = true;
                    break;
                }
            }
            current_path = Some(rest.to_string());
            current_branch.clear();
        } else if let Some(rest) = line.strip_prefix("branch ") {
            current_branch = rest.to_string();
        }
    }
    if !found {
        if let Some(path) = current_path {
            if path == worktree_str || current_branch == expected_branch {
                found = true;
            }
        }
    }
    found
}

/// Try to reopen an existing persistent worktree for `hub_id`.
///
/// Returns `Ok(Some(handle))` when the directory exists and is still
/// registered in `git worktree list`; `Ok(None)` when no reusable worktree
/// exists. The handle's `baseline` is refreshed from `HEAD` so later
/// `collect_diff` captures the cumulative edits since the original baseline.
///
/// # Errors
/// `Invalid hubId` or git failures.
pub fn reopen_worktree(
    global_dir: &Path,
    repo_root: &Path,
    hub_id: &str,
) -> Result<Option<IsoHandle>> {
    validate_hub_id(hub_id)?;
    let path = worktree_path_for_hub(global_dir, hub_id);
    let branch = persistent_branch_for_hub(hub_id);
    if !path.exists() {
        return Ok(None);
    }
    if !is_worktree_registered(repo_root, &path, &branch) {
        return Ok(None);
    }
    // Refresh baseline from the worktree's current HEAD (the original
    // materialized baseline commit). Keep the directory's edits intact.
    let baseline = git_ok(&path, &["rev-parse", "HEAD"])?.trim().to_string();
    Ok(Some(IsoHandle {
        id: hub_id.to_string(),
        branch,
        path,
        repo_root: repo_root.to_path_buf(),
        baseline,
        persistent: true,
    }))
}

/// Reopen a persistent worktree for `hub_id` or create it if absent.
///
/// This is the C3 entry point for continuable subagents (`hubId ==
/// worktreeId`): when the worktree directory already exists and is still
/// listed by `git worktree list`, it is reused (incremental-edit semantics);
/// otherwise a new persistent worktree is created at
/// `<global_dir>/worktrees/subagents/<hubId>` with branch
/// `pi-iso-<hubId>`, materialising the parent's dirty tree exactly once.
///
/// The returned handle has `persistent = true` and must be cleaned up via
/// [`cleanup_persistent_worktree`] or [`drop_worktree`] when the hub entry
/// reaches a terminal non-continuable state (`Failed`/`Cancelled`/`Killed`).
///
/// # Errors
/// `PI_ISO_NOT_GIT` when `repo_root` is not a git work tree, `Invalid hubId`,
/// or git failures.
pub fn reopen_or_isolate(
    global_dir: &Path,
    repo_root: &Path,
    hub_id: &str,
    task_id: &str,
) -> Result<IsoHandle> {
    validate_hub_id(hub_id)?;
    if let Some(handle) = reopen_worktree(global_dir, repo_root, hub_id)? {
        return Ok(handle);
    }
    isolate_persistent(global_dir, repo_root, hub_id, task_id)
}

/// Create a new persistent worktree at `<global_dir>/worktrees/subagents/<hubId>`.
///
/// Internal helper for [`reopen_or_isolate`]; prefer that function when
/// reuse semantics are desired.
///
/// # Errors
/// `PI_ISO_NOT_GIT`, `Invalid hubId`, or git failures.
pub fn isolate_persistent(
    global_dir: &Path,
    repo_root: &Path,
    hub_id: &str,
    _task_id: &str,
) -> Result<IsoHandle> {
    validate_hub_id(hub_id)?;
    let is_git = git(repo_root, &["rev-parse", "--is-inside-work-tree"])
        .is_ok_and(|output| output.status.success());
    if !is_git {
        return Err(Error::tool(
            "subagent",
            format!(
                "PI_ISO_NOT_GIT: {} is not a git work tree; isolation requires git \
                 (run non-isolated or copy the directory explicitly)",
                repo_root.display()
            ),
        ));
    }

    let branch = persistent_branch_for_hub(hub_id);
    let path = worktree_path_for_hub(global_dir, hub_id);

    // Ensure the parent directory exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::tool("subagent", format!("Failed to create worktree parent: {e}"))
        })?;
    }

    // Stale directory without a git registration — remove so `worktree add` can succeed.
    if path.exists() && !is_worktree_registered(repo_root, &path, &branch) {
        let _ = std::fs::remove_dir_all(&path);
        let _ = git(repo_root, &["branch", "-D", &branch]);
        let _ = git(repo_root, &["worktree", "prune"]);
    }

    // Stale branch without a directory — drop it so `-b <branch>` succeeds.
    if !path.exists() {
        let _ = git(repo_root, &["branch", "-D", &branch]);
    }

    // Suppress unused warning for task_id in persistent mode — the hubId is
    // the stable identity; task_id only influences the ephemeral fallback.

    git_ok(
        repo_root,
        &["worktree", "add", &path.to_string_lossy(), "-b", &branch],
    )?;

    let cleanup_on_err = |err: Error| -> Error {
        let handle = IsoHandle {
            branch: branch.clone(),
            id: hub_id.to_string(),
            path: path.clone(),
            repo_root: repo_root.to_path_buf(),
            baseline: String::new(),
            persistent: true,
        };
        let _ = drop_worktree(&handle);
        err
    };

    // Dirty-tree materialization: tracked diff + untracked files (same as ephemeral).
    let tracked_patch =
        git_ok(repo_root, &["diff", "--binary", "HEAD"]).map_err(&cleanup_on_err)?;
    if !tracked_patch.trim().is_empty() {
        let patch_file = path.join(".pi-iso-parent.patch");
        std::fs::write(&patch_file, &tracked_patch)
            .map_err(|e| Error::tool("subagent", format!("Failed to stage parent patch: {e}")))
            .map_err(&cleanup_on_err)?;
        git_ok(&path, &["apply", &patch_file.to_string_lossy()]).map_err(&cleanup_on_err)?;
        let _ = std::fs::remove_file(&patch_file);
    }
    let untracked = git_ok(repo_root, &["ls-files", "--others", "--exclude-standard"])
        .map_err(&cleanup_on_err)?;
    for relative in untracked.lines().filter(|line| !line.is_empty()) {
        let source = repo_root.join(relative);
        let target = path.join(relative);
        if source.is_file() {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(&source, &target);
        }
    }

    git_ok(&path, &["add", "-A"]).map_err(&cleanup_on_err)?;
    git_ok(
        &path,
        &[
            "commit",
            "--allow-empty",
            "--no-verify",
            "-m",
            &format!("pi-iso baseline {branch}"),
        ],
    )
    .map_err(&cleanup_on_err)?;
    let baseline = git_ok(&path, &["rev-parse", "HEAD"])
        .map_err(&cleanup_on_err)?
        .trim()
        .to_string();

    Ok(IsoHandle {
        branch,
        id: hub_id.to_string(),
        path,
        repo_root: repo_root.to_path_buf(),
        baseline,
        persistent: true,
    })
}

/// Explicitly clean up a persistent hub worktree.
///
/// Removes the worktree at `<global_dir>/worktrees/subagents/<hubId>` and its
/// `pi-iso-<hubId>` branch. A no-op when the directory is already absent
/// (still attempts to delete a dangling branch). Use this when a hub entry
/// reaches a terminal non-continuable status (`Failed`/`Cancelled`/`Killed`)
/// or on explicit `cleanup` of a `Done` entry.
///
/// # Errors
/// `Invalid hubId` or git failures when a worktree is present but cannot be
/// removed.
pub fn cleanup_persistent_worktree(
    global_dir: &Path,
    hub_id: &str,
    repo_root: &Path,
) -> Result<()> {
    validate_hub_id(hub_id)?;
    let path = worktree_path_for_hub(global_dir, hub_id);
    let branch = persistent_branch_for_hub(hub_id);
    if !path.exists() {
        let _ = git(repo_root, &["branch", "-D", &branch]);
        let _ = git(repo_root, &["worktree", "prune"]);
        return Ok(());
    }
    let handle = IsoHandle {
        id: hub_id.to_string(),
        branch: branch.clone(),
        path: path.clone(),
        repo_root: repo_root.to_path_buf(),
        baseline: String::new(),
        persistent: true,
    };
    // `drop_worktree` handles both the worktree removal and branch deletion.
    // If it fails (e.g. worktree already pruned but dir remains), fall back
    // to filesystem removal so we never leak the directory.
    match drop_worktree(&handle) {
        Ok(()) => {
            let _ = git(repo_root, &["worktree", "prune"]);
            Ok(())
        }
        Err(err) => {
            let _ = std::fs::remove_dir_all(&path);
            let _ = git(repo_root, &["branch", "-D", &branch]);
            let _ = git(repo_root, &["worktree", "prune"]);
            Err(err)
        }
    }
}

/// Create an isolated worktree for a child task, materializing the parent's
/// uncommitted state into it.
///
/// Ephemeral flavour — the worktree lives at `<tmp>/pi-iso-<id>` and is
/// expected to be removed via `drop_worktree` or `reap_stale` after
/// `collect_diff`. For the persistent hub-reused flavour, use
/// [`reopen_or_isolate`] / [`isolate_persistent`].
///
/// # Errors
static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Named `PI_ISO_NOT_GIT` for non-git directories; git failures otherwise.
pub fn isolate(repo_root: &Path, task_id: &str) -> Result<IsoHandle> {
    let is_git = git(repo_root, &["rev-parse", "--is-inside-work-tree"])
        .is_ok_and(|output| output.status.success());
    if !is_git {
        return Err(Error::tool(
            "subagent",
            format!(
                "PI_ISO_NOT_GIT: {} is not a git work tree; isolation requires git \
                 (run non-isolated or copy the directory explicitly)",
                repo_root.display()
            ),
        ));
    }

    let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let id = format!(
        "{ISO_PREFIX}{}-{}-{seq}",
        sanitize_id(task_id),
        std::process::id()
    );
    let path = std::env::temp_dir().join(&id);

    // Branch from HEAD into the temp path.
    git_ok(
        repo_root,
        &["worktree", "add", &path.to_string_lossy(), "-b", &id],
    )?;

    // Everything after worktree creation must clean up on failure, or every
    // failed spawn leaks a worktree + branch with no automatic reaper.
    let cleanup_on_err = |err: Error| -> Error {
        let handle = IsoHandle {
            branch: id.clone(),
            id: id.clone(),
            path: path.clone(),
            repo_root: repo_root.to_path_buf(),
            baseline: String::new(),
            persistent: false,
        };
        let _ = drop_worktree(&handle);
        err
    };
    // Dirty-tree materialization: tracked diff + untracked files.
    let tracked_patch =
        git_ok(repo_root, &["diff", "--binary", "HEAD"]).map_err(&cleanup_on_err)?;
    if !tracked_patch.trim().is_empty() {
        let patch_file = path.join(".pi-iso-parent.patch");
        std::fs::write(&patch_file, &tracked_patch)
            .map_err(|e| Error::tool("subagent", format!("Failed to stage parent patch: {e}")))
            .map_err(&cleanup_on_err)?;
        git_ok(&path, &["apply", &patch_file.to_string_lossy()]).map_err(&cleanup_on_err)?;
        let _ = std::fs::remove_file(&patch_file);
    }
    let untracked = git_ok(repo_root, &["ls-files", "--others", "--exclude-standard"])
        .map_err(&cleanup_on_err)?;
    for relative in untracked.lines().filter(|line| !line.is_empty()) {
        let source = repo_root.join(relative);
        let target = path.join(relative);
        if source.is_file() {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(&source, &target);
        }
    }

    // Commit the materialized baseline so the child's own work is a clean
    // diff against it (and the parent's dirty state never double-applies).
    git_ok(&path, &["add", "-A"]).map_err(&cleanup_on_err)?;
    git_ok(
        &path,
        &[
            "commit",
            "--allow-empty",
            "--no-verify",
            "-m",
            &format!("pi-iso baseline {id}"),
        ],
    )
    .map_err(&cleanup_on_err)?;
    let baseline = git_ok(&path, &["rev-parse", "HEAD"])
        .map_err(&cleanup_on_err)?
        .trim()
        .to_string();

    Ok(IsoHandle {
        branch: id.clone(),
        id,
        path,
        repo_root: repo_root.to_path_buf(),
        baseline,
        persistent: false,
    })
}

/// Collect the child's worktree state as a patch vs the materialized
/// baseline (so the parent's dirty state never double-applies).
///
/// # Errors
/// git failures.
pub fn collect_diff(handle: &IsoHandle) -> Result<(String, String)> {
    // Stage everything so untracked work appears in the diff.
    git_ok(&handle.path, &["add", "-A"])?;
    let patch = git_ok(
        &handle.path,
        &["diff", "--binary", "--cached", &handle.baseline],
    )?;
    let diff_stat = git_ok(
        &handle.path,
        &["diff", "--cached", "--stat", &handle.baseline],
    )?;
    Ok((patch, diff_stat))
}

/// Serialize parent-tree application across sibling children (they apply
/// in task order, never concurrently).
fn parent_apply_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));
    &LOCK
}

/// Apply the collected patch into the parent tree. On conflict, report the
/// conflicting files and leave the worktree for manual resolution — never
/// force.
///
/// # Errors
/// Named `PI_ISO_CONFLICT` listing conflicted files.
pub fn apply_to_parent(handle: &IsoHandle, patch: &str) -> Result<()> {
    if patch.trim().is_empty() {
        return Ok(());
    }
    let _guard = parent_apply_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // --check first so a failing apply reports files without partial writes.
    let patch_file = handle.path.join(".pi-iso-outgoing.patch");
    std::fs::write(&patch_file, patch)
        .map_err(|e| Error::tool("subagent", format!("Failed to stage patch: {e}")))?;
    let check = git(
        &handle.repo_root,
        &["apply", "--check", &patch_file.to_string_lossy()],
    )?;
    if !check.status.success() {
        let stderr = String::from_utf8_lossy(&check.stderr).into_owned();
        let mut files: Vec<String> = stderr
            .lines()
            .filter_map(|line| {
                line.strip_prefix("error: ")
                    .and_then(|rest| rest.split(':').nth(1))
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            })
            .collect();
        files.sort();
        files.dedup();
        let _ = std::fs::remove_file(&patch_file);
        return Err(Error::tool(
            "subagent",
            format!(
                "PI_ISO_CONFLICT: patch from {} does not apply cleanly to the parent tree. \
                 Conflicted files: {}. The worktree is left at {} for manual resolution.",
                handle.branch,
                if files.is_empty() {
                    "(see git apply output)".to_string()
                } else {
                    files.join(", ")
                },
                handle.path.display()
            ),
        ));
    }
    git_ok(&handle.repo_root, &["apply", &patch_file.to_string_lossy()])?;
    let _ = std::fs::remove_file(&patch_file);
    Ok(())
}

/// Remove the worktree and its branch.
///
/// Works for both ephemeral (`<tmp>/pi-iso-*`) and persistent
/// (`<global_dir>/worktrees/subagents/<hubId>`) handles. Persistent
/// worktrees are only removed when this function (or
/// [`cleanup_persistent_worktree`]) is explicitly called — `collect_diff`
/// and normal completion never auto-delete them.
///
/// # Errors
/// git failures.
pub fn drop_worktree(handle: &IsoHandle) -> Result<()> {
    git_ok(
        &handle.repo_root,
        &[
            "worktree",
            "remove",
            "--force",
            &handle.path.to_string_lossy(),
        ],
    )?;
    // Branch deletion is best-effort (the worktree removal usually drops it).
    let _ = git(&handle.repo_root, &["branch", "-D", &handle.branch]);
    Ok(())
}

/// One of our worktrees with its age.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
    pub path: String,
    pub branch: String,
    pub age_ms: u64,
}

/// List live pi-iso worktrees under a repo.
///
/// # Errors
/// git failures.
pub fn list_mine(repo_root: &Path) -> Result<Vec<WorktreeInfo>> {
    let porcelain = git_ok(repo_root, &["worktree", "list", "--porcelain"])?;
    let mut out = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch = String::new();
    let flush = |path: Option<String>, branch: String, out: &mut Vec<WorktreeInfo>| {
        let Some(path) = path else { return };
        // Match on the BASENAME prefix: a foreign worktree whose path
        // merely contains "pi-iso-" somewhere must never look like ours
        // to the reaper. Persistent hub worktrees live under
        // `worktrees/subagents/<hubId>` (no prefix) and are intentionally
        // excluded from the ephemeral reaper — they are managed by explicit
        // `cleanup_persistent_worktree`.
        let is_ours = std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(ISO_PREFIX));
        if !is_ours {
            return;
        }
        let age_ms = worktree_age_ms(Path::new(&path));
        out.push(WorktreeInfo {
            path,
            branch,
            age_ms,
        });
    };
    for line in porcelain.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            flush(
                current_path.take(),
                std::mem::take(&mut current_branch),
                &mut out,
            );
            current_path = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("branch refs/heads/") {
            current_branch = rest.to_string();
        }
    }
    flush(current_path, current_branch, &mut out);
    Ok(out)
}

/// Age of a worktree based on the NEWEST modification time anywhere in its
/// tree (bd-ajg8l #14): the root directory mtime only changes when entries
/// are added/removed at the top level, so a long-running child writing into
/// nested paths used to look stale and got reaped mid-run.
///
/// Bounded walk (`.git` skipped, 5_000-entry cap) — liveness detection, not
/// a full audit. Falls back to the root mtime when the walk yields nothing.
fn worktree_age_ms(path: &Path) -> u64 {
    fn mtime_elapsed_ms(path: &Path) -> Option<u128> {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|mtime| mtime.elapsed().ok())
            .map(|elapsed| elapsed.as_millis())
    }

    let root_age = mtime_elapsed_ms(path).unwrap_or(u128::MAX);
    let mut newest = root_age;
    let mut visited = 0usize;
    let walker = ignore::WalkBuilder::new(path)
        .hidden(false)
        .follow_links(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();
    for entry in walker.flatten() {
        visited += 1;
        if visited > 5_000 {
            break;
        }
        if let Some(age) = mtime_elapsed_ms(entry.path())
            && newest > age
        {
            newest = age;
        }
    }
    if visited <= 1 {
        // Nothing walkable: fall back to the root mtime alone.
        return u64::try_from(root_age).unwrap_or(u64::MAX);
    }
    u64::try_from(newest).unwrap_or(u64::MAX)
}

/// Reap our stale worktrees (prefix-tagged only; foreign worktrees are
/// never touched).
///
/// # Errors
/// git failures on the first worktree that cannot be listed.
pub fn reap_stale(repo_root: &Path, older_than: Duration) -> Result<Vec<String>> {
    let mut reaped = Vec::new();
    for info in list_mine(repo_root)? {
        if info.age_ms < u64::try_from(older_than.as_millis()).unwrap_or(u64::MAX) {
            continue;
        }
        let handle = IsoHandle {
            id: info.branch.clone(),
            branch: info.branch.clone(),
            path: PathBuf::from(&info.path),
            repo_root: repo_root.to_path_buf(),
            baseline: String::new(),
            persistent: false,
        };
        if drop_worktree(&handle).is_ok() {
            reaped.push(info.path);
        }
    }
    Ok(reaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pi-iso-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("repo dir");
        git_ok(&dir, &["init", "-b", "main"]).expect("git init");
        git_ok(&dir, &["config", "user.email", "iso@test"]).expect("config");
        git_ok(&dir, &["config", "user.name", "Iso Test"]).expect("config");
        std::fs::write(dir.join("file.txt"), "one\n").expect("write");
        git_ok(&dir, &["add", "."]).expect("add");
        git_ok(&dir, &["commit", "-m", "init"]).expect("commit");
        dir
    }

    #[test]
    fn non_git_refuses_with_named_error() {
        let dir = std::env::temp_dir().join(format!("pi-iso-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");

        // Remote-execution harnesses may resolve temp_dir INSIDE the synced
        // project tree (which is itself a git work tree). The refusal under
        // test only applies to genuinely non-git directories — self-skip
        // when the environment cannot provide one.
        let inside = git(&dir, &["rev-parse", "--is-inside-work-tree"])
            .is_ok_and(|output| output.status.success());
        if inside {
            eprintln!("skipping: temp_dir resolves inside a git work tree");
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }

        let err = isolate(&dir, "x").unwrap_err();
        assert!(
            err.to_string().contains("PI_ISO_NOT_GIT"),
            "expected named refusal: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn isolate_collects_dirty_state_and_child_edits() {
        let repo = init_repo("dirty");
        // Dirty the parent: tracked edit + untracked file.
        std::fs::write(repo.join("file.txt"), "one\nparent-dirty\n").expect("dirty");
        std::fs::write(repo.join("untracked.txt"), "loose\n").expect("untracked");

        let handle = isolate(&repo, "task-one").expect("isolate");
        // The child sees the uncommitted content (round-7 invariant).
        let seen = std::fs::read_to_string(handle.path.join("file.txt")).expect("read");
        assert!(
            seen.contains("parent-dirty"),
            "worktree must carry dirty state: {seen}"
        );
        assert!(
            handle.path.join("untracked.txt").exists(),
            "untracked file must be copied"
        );

        // Child edits; collect the diff vs parent HEAD.
        std::fs::write(handle.path.join("child.txt"), "child work\n").expect("child write");
        let (patch, stat) = collect_diff(&handle).expect("collect");
        assert!(patch.contains("child.txt"), "patch must include child work");
        assert!(!stat.is_empty());

        // Apply mode lands it byte-identical in the parent.
        apply_to_parent(&handle, &patch).expect("apply");
        let landed = std::fs::read_to_string(repo.join("child.txt")).expect("landed");
        assert_eq!(landed, "child work\n");

        drop_worktree(&handle).expect("drop");
        assert!(!handle.path.exists());
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn conflicting_apply_reports_files_and_never_forces() {
        let repo = init_repo("conflict");
        let handle = isolate(&repo, "task-two").expect("isolate");
        // Child edits file.txt one way...
        std::fs::write(handle.path.join("file.txt"), "one\nchild-version\n").expect("child edit");
        let (patch, _) = collect_diff(&handle).expect("collect");
        // ...while the parent diverges on the same lines and COMMITS, so
        // the patch's context no longer matches.
        std::fs::write(repo.join("file.txt"), "one\nparent-version\n").expect("parent edit");
        git_ok(&repo, &["add", "."]).expect("add");
        git_ok(&repo, &["commit", "-m", "parent diverges"]).expect("commit");

        let err = apply_to_parent(&handle, &patch).unwrap_err();
        assert!(
            err.to_string().contains("PI_ISO_CONFLICT"),
            "expected conflict refusal: {err}"
        );
        // Worktree left for manual resolution; parent untouched by force.
        assert!(handle.path.exists());
        let parent = std::fs::read_to_string(repo.join("file.txt")).expect("parent");
        assert!(parent.contains("parent-version"));
        drop_worktree(&handle).expect("drop");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn reaper_only_touches_our_prefix() {
        let repo = init_repo("reap");
        let ours = isolate(&repo, "task-three").expect("isolate");
        // A foreign worktree without the prefix.
        let foreign_path = std::env::temp_dir().join(format!("foreign-wt-{}", std::process::id()));
        git_ok(
            &repo,
            &[
                "worktree",
                "add",
                &foreign_path.to_string_lossy(),
                "-b",
                "foreign-branch",
            ],
        )
        .expect("foreign worktree");

        // Backdate ours so it counts as stale.
        let reaped = reap_stale(&repo, Duration::from_millis(0)).expect("reap");
        assert!(
            reaped.iter().any(|path| path.contains(ISO_PREFIX)),
            "our worktree must be reaped: {reaped:?}"
        );
        assert!(
            foreign_path.exists(),
            "foreign worktree must survive the sweep"
        );
        git_ok(
            &repo,
            &[
                "worktree",
                "remove",
                "--force",
                &foreign_path.to_string_lossy(),
            ],
        )
        .expect("foreign cleanup");
        let _ = git(&repo, &["branch", "-D", "foreign-branch"]);
        let _ = std::fs::remove_dir_all(&repo);
        let _ = ours;
    }

    /// bd-ajg8l #14: a long-running child writing into NESTED paths must
    /// keep the worktree fresh even when the root directory mtime is old.
    #[test]
    fn worktree_age_uses_newest_nested_mtime() {
        let repo = init_repo("fresh-nested");
        let ours = isolate(&repo, "task-fresh").expect("isolate");

        // Age the root directory far into the past (as if nothing happened
        // at the top level since creation).
        let old = filetime::FileTime::from_unix_time(1_600_000_000, 0);
        filetime::set_file_mtime(&ours.path, old).expect("set root mtime");

        // Child activity: write into a nested path NOW. The freshness walk
        // must find it even though the ROOT mtime says the tree is old.
        // (The converse — a fully-aged tree reporting old — cannot be
        // asserted reliably on remote-execution harnesses whose own sync
        // touches worktree files mid-test.)
        let nested = ours.path.join("src").join("deep");
        std::fs::create_dir_all(&nested).expect("mkdir nested");
        std::fs::write(nested.join("mod.rs"), "child work").expect("write");

        let age = worktree_age_ms(&ours.path);
        assert!(
            age < 60_000,
            "nested writes must keep the worktree fresh, got {age}ms"
        );
    }

    #[test]
    fn reap_stale_spares_fresh_nested_worktrees() {
        let repo = init_repo("reap-fresh");
        let ours = isolate(&repo, "task-live").expect("isolate");

        // Root mtime aged; child keeps writing to nested paths.
        let old = filetime::FileTime::from_unix_time(1_600_000_000, 0);
        filetime::set_file_mtime(&ours.path, old).expect("set root mtime");
        let deep = ours.path.join("deeply").join("nested");
        std::fs::create_dir_all(&deep).expect("mkdir deep");
        std::fs::write(deep.join("work.txt"), "still running").expect("write");

        let reaped = reap_stale(&repo, Duration::from_secs(60)).unwrap();
        assert!(
            !reaped
                .iter()
                .any(|p| p == &ours.path.to_string_lossy().to_string()),
            "live worktree with fresh nested writes must survive the reaper"
        );
        assert!(ours.path.exists(), "worktree must still exist");
    }
}
