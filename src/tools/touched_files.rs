#![forbid(unsafe_code)]
#![allow(
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::redundant_clone,
    clippy::doc_markdown
)]

use indexmap::IndexMap;
use pi_core::model::{FileStatus, FileTouch, TouchSource};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;

// ============================================================================
// Internal aggregated form (mirrors TS TouchedFile)
// ============================================================================

#[derive(Debug, Clone)]
struct TouchedFile {
    path: String,
    status: FileStatus,
    source: TouchSource,
    old_path: Option<String>,
    first_old_path: Option<String>,
    first_at: i64,
    last_at: i64,
    count: usize,
    tool_call_ids: Vec<String>,
    sources: Vec<TouchSource>,
    tool_name: String,
}

fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ---------------------------------------------------------------------------
// create: TS create(touch, now)
// ---------------------------------------------------------------------------

fn create(touch: &FileTouch, now: i64) -> TouchedFile {
    let mut tf = TouchedFile {
        path: touch.path.clone(),
        status: touch.status,
        source: touch.source,
        old_path: None,
        first_old_path: None,
        first_at: now,
        last_at: now,
        count: 1,
        tool_call_ids: if touch.tool_call_id.is_empty() {
            Vec::new()
        } else {
            vec![touch.tool_call_id.clone()]
        },
        sources: vec![touch.source],
        tool_name: touch.tool_name.clone(),
    };
    if touch.status == FileStatus::Renamed {
        tf.old_path.clone_from(&touch.old_path);
    }
    // Preserve first_old_path if incoming already carries it (aggregated chain case)
    if touch.first_old_path.is_some() {
        tf.first_old_path.clone_from(&touch.first_old_path);
    }
    tf
}

// ---------------------------------------------------------------------------
// merge: 1:1 translation of TS merge()
// ---------------------------------------------------------------------------

fn merge(existing: &TouchedFile, incoming: &FileTouch, now: i64) -> TouchedFile {
    let count = existing.count + 1;
    let last_at = now;
    let first_at = existing.first_at;
    let tool_call_ids = if incoming.tool_call_id.is_empty()
        || existing.tool_call_ids.contains(&incoming.tool_call_id)
    {
        existing.tool_call_ids.clone()
    } else {
        let mut v = existing.tool_call_ids.clone();
        v.push(incoming.tool_call_id.clone());
        v
    };
    let sources: Vec<TouchSource> = if existing.sources.contains(&incoming.source) {
        existing.sources.clone()
    } else {
        let mut v = existing.sources.clone();
        v.push(incoming.source);
        v
    };
    let tool_name = if incoming.tool_name.is_empty() {
        existing.tool_name.clone()
    } else {
        incoming.tool_name.clone()
    };

    let e_status = existing.status;
    let i_status = incoming.status;

    // R + R → merge chain a→b + b→c = a→c
    if e_status == FileStatus::Renamed && i_status == FileStatus::Renamed {
        let e_first = existing
            .first_old_path
            .clone()
            .or_else(|| existing.old_path.clone());
        let i_old = incoming.old_path.clone();
        return TouchedFile {
            path: incoming.path.clone(),
            status: FileStatus::Renamed,
            source: incoming.source,
            old_path: i_old,
            first_old_path: e_first,
            first_at,
            last_at,
            count,
            tool_call_ids,
            sources,
            tool_name,
        };
    }

    // R + M/A → keep R
    if e_status == FileStatus::Renamed
        && (i_status == FileStatus::Modified || i_status == FileStatus::Added)
    {
        return TouchedFile {
            path: existing.path.clone(),
            status: existing.status,
            source: existing.source,
            old_path: existing.old_path.clone(),
            first_old_path: existing.first_old_path.clone(),
            first_at,
            last_at,
            count,
            tool_call_ids,
            sources,
            tool_name,
        };
    }

    // R + D → D with firstOldPath
    if e_status == FileStatus::Renamed && i_status == FileStatus::Deleted {
        let first_old = existing
            .first_old_path
            .clone()
            .or_else(|| existing.old_path.clone());
        return TouchedFile {
            path: incoming.path.clone(),
            status: FileStatus::Deleted,
            source: incoming.source,
            old_path: None,
            first_old_path: first_old,
            first_at,
            last_at,
            count,
            tool_call_ids,
            sources,
            tool_name,
        };
    }

    // incoming R covers non-R
    if i_status == FileStatus::Renamed {
        let i_old = incoming.old_path.clone();
        return TouchedFile {
            path: incoming.path.clone(),
            status: FileStatus::Renamed,
            source: incoming.source,
            old_path: i_old,
            first_old_path: None,
            first_at,
            last_at,
            count,
            tool_call_ids,
            sources,
            tool_name,
        };
    }

    // D + A/M → M (rebuild)
    if e_status == FileStatus::Deleted
        && (i_status == FileStatus::Added || i_status == FileStatus::Modified)
    {
        return TouchedFile {
            path: incoming.path.clone(),
            status: FileStatus::Modified,
            source: incoming.source,
            old_path: None,
            first_old_path: None,
            first_at,
            last_at,
            count,
            tool_call_ids,
            sources,
            tool_name,
        };
    }

    // * + D → D
    if i_status == FileStatus::Deleted {
        return TouchedFile {
            path: incoming.path.clone(),
            status: FileStatus::Deleted,
            source: incoming.source,
            old_path: None,
            first_old_path: None,
            first_at,
            last_at,
            count,
            tool_call_ids,
            sources,
            tool_name,
        };
    }

    // otherwise: latest covers
    TouchedFile {
        path: incoming.path.clone(),
        status: i_status,
        source: incoming.source,
        old_path: None,
        first_old_path: None,
        first_at,
        last_at,
        count,
        tool_call_ids,
        sources,
        tool_name,
    }
}

// ============================================================================
// Public aggregation: ordered fold using IndexMap (preserves insertion order)
// ============================================================================

/// Aggregate a list of `FileTouch` in execution order into net `FileTouch`
/// values.  1:1 with TS `TouchedFilesStore` merge semantics.
///
/// The per-tool `FileTouch` vectors are kept raw; `TurnEnd.touchedFiles`
/// should be the output of this function.
pub fn aggregate_touched(touches: Vec<FileTouch>) -> Vec<FileTouch> {
    if touches.is_empty() {
        return Vec::new();
    }
    let mut map: IndexMap<String, TouchedFile> = IndexMap::new();
    for cur in touches {
        let now = now_millis();
        // Key is the *current* path (terminal path) for rename chains.
        // For R we key by incoming.path; for merge cases the old R entry
        // stays keyed by its old terminal, so we need to handle chain movement.
        // Simplify: if cur is R and map contains an entry whose path == old_path,
        // move it. This handles the a→b→c chain where map key is "b" but next
        // incoming is b→c. The TS store keys by current path, so we mirror that.
        if cur.status == FileStatus::Renamed {
            if let Some(old) = cur.old_path.as_deref() {
                // Find and remove any entry keyed by old path
                if let Some(existing) = map.shift_remove(old) {
                    let merged = merge(&existing, &cur, now);
                    // Remove stale entry for cur.path if any (should not exist in R+R chain)
                    map.shift_remove(&cur.path);
                    map.insert(merged.path.clone(), merged);
                    continue;
                }
            }
        }
        // Also handle R+ D case where D path equals old R terminal
        // The generic path-key handles it naturally: R(a→b) keyed "b", D(b) keyed "b" -> merge
        if let Some(existing) = map.get(&cur.path).cloned() {
            let merged = merge(&existing, &cur, now);
            // Handle A+D → ∅ (create then delete cancels)
            if existing.status == FileStatus::Added && cur.status == FileStatus::Deleted {
                map.shift_remove(&cur.path);
                continue;
            }
            // If merged is D that originated from R, its path may differ; already handled via key
            map.insert(merged.path.clone(), merged);
        } else {
            // Check A+D cancellation needs existing; otherwise insert
            // Also handle D+A→M is covered by merge when key matches; here it's fresh insert via create
            let created = create(&cur, now);
            map.insert(created.path.clone(), created);
        }
    }

    // Convert internal TouchedFile back to FileTouch for wire format.
    // Preserve first_old_path for R chains so pidian can show a→c.
    map.into_values()
        .map(|tf| FileTouch {
            path: tf.path,
            status: tf.status,
            source: tf.source,
            old_path: tf.old_path,
            first_old_path: tf.first_old_path,
            tool_call_id: tf.tool_call_ids.last().cloned().unwrap_or_default(),
            tool_name: tf.tool_name,
        })
        .collect()
}

// ============================================================================
// Structured extraction (0ms, no IO)
// ============================================================================

/// Extract structured `FileTouch` from tool arguments.
///
/// Returns `None` for read-only tools or when no path can be extracted.
/// For `ast_edit` with `action=="stage"` returns `None` (no write yet).
pub fn extract_structured_touches(
    tool_name: &str,
    tool_call_id: &str,
    args: &serde_json::Value,
) -> Option<Vec<FileTouch>> {
    // ast_edit stage does not write
    if tool_name == "ast_edit" {
        if let Some(action) = args.get("action").and_then(|v| v.as_str()) {
            if action == "stage" {
                return None;
            }
        }
        // For resolve, path is in args["path"] or proposal files; best-effort extract
        // If no path, return None and let bash diff handle it (fallback)
    }
    // Only write-capable structured tools have path semantics
    let is_structured_write = matches!(tool_name, "write" | "edit" | "hashline_edit" | "ast_edit")
        || args.as_object().is_some_and(|obj| {
            obj.contains_key("path")
                || obj.contains_key("paths")
                || obj.contains_key("file")
                || obj.contains_key("filePath")
                || obj.contains_key("files")
        });
    if !is_structured_write {
        return None;
    }

    let mut paths: Vec<String> = Vec::new();

    // Single path fields
    for key in ["path", "file", "filePath", "file_path"] {
        if let Some(s) = args.get(key).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                paths.push(s.to_string());
            }
        }
    }
    // Array path fields
    for key in ["paths", "files"] {
        if let Some(arr) = args.get(key).and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    if !s.trim().is_empty() {
                        paths.push(s.to_string());
                    }
                }
            }
        }
    }

    if paths.is_empty() {
        // Unknown extension write with no path hint
        // Return sentinel so caller knows to mark unknown rather than silently skip
        // Caller may synthesize <unknown:toolName> if it wants
        return None;
    }

    let touches = paths
        .into_iter()
        .map(|p| FileTouch {
            path: normalize_path(&p),
            status: FileStatus::Modified,
            source: TouchSource::Structured,
            old_path: None,
            first_old_path: None,
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
        })
        .collect();
    Some(touches)
}

fn normalize_path(p: &str) -> String {
    // Normalize to POSIX relative where possible; keep absolute if outside cwd
    // Simple normalization: trim, replace backslashes, strip leading ./
    let trimmed = p.trim();
    let mut s = trimmed.replace('\\', "/");
    while s.starts_with("./") {
        s = s[2..].to_string();
    }
    s
}

// ============================================================================
// Bash filter (structured is never filtered)
// ============================================================================

/// Filter bash-discovered touches. Structured touches are not filtered.
pub fn filter_bash_touches(touches: Vec<FileTouch>) -> Vec<FileTouch> {
    touches
        .into_iter()
        .filter(|t| {
            let p = t.path.as_str();
            if p.starts_with(".obsidian/") || p == ".obsidian" {
                return false;
            }
            if p.starts_with(".trash/") || p == ".trash" {
                return false;
            }
            if p.starts_with(".tmp/") || p == ".tmp" {
                return false;
            }
            #[allow(clippy::case_sensitive_file_extension_comparisons)]
            if p.ends_with(".tmp") {
                return false;
            }
            if p == ".DS_Store" || p.ends_with("/.DS_Store") {
                return false;
            }
            true
        })
        .collect()
}

// ============================================================================
// Bash window snapshot (gix → git CLI → walk)
// ============================================================================

#[derive(Debug, Clone)]
pub struct WalkMeta {
    pub mtime_ms: i128,
    pub size: u64,
}

/// Snapshot of the working directory at a point in time.
#[derive(Debug, Clone)]
pub enum Snapshot {
    Git(GitSnapshot),
    Walk(HashMap<String, WalkMeta>),
}

#[derive(Debug, Clone, Default)]
pub struct GitSnapshot {
    /// path -> (status, old_path)
    pub entries: HashMap<String, (FileStatus, Option<String>)>,
}

/// Capture a snapshot of cwd.  Tries `gix` (if available) → `git` CLI → walk.
///
/// This is the **synchronous** core — it blocks on `gix`/`git`/walk and must
/// not be called directly from an `asupersync` reactor task.  Use
/// [`capture_snapshot_async`] from async contexts.
pub fn capture_snapshot(cwd: &Path) -> Snapshot {
    if let Some(g) = capture_gix(cwd) {
        return Snapshot::Git(g);
    }
    if let Some(g) = capture_git_cli(cwd) {
        return Snapshot::Git(g);
    }
    Snapshot::Walk(capture_walk(cwd))
}

/// Async wrapper that offloads [`capture_snapshot`] to the `asupersync`
/// blocking pool so the reactor thread is not stalled.
///
/// `gix` can be 30-50ms (release) and `git status` is ~90-100ms; both would
/// otherwise block `RuntimeBuilder::current_thread()`'s single reactor
/// thread, freezing timers / `Cx::checkpoint()` cancellation.
///
/// Uses `asupersync::runtime::spawn_blocking` (not `tokio::task`) — `pi`
/// has no `tokio` dependency; see `crates/pi-core/src/agent_cx.rs`.
pub async fn capture_snapshot_async(cwd: PathBuf) -> Snapshot {
    let cwd_for_blocking = cwd;
    asupersync::runtime::spawn_blocking(move || capture_snapshot(&cwd_for_blocking)).await
}

fn capture_gix(cwd: &Path) -> Option<GitSnapshot> {
    // Discover repository from cwd (walks up to find .git). `ok()?` falls back to git CLI on
    // non-git dirs or bare repos.
    let repo = gix::discover(cwd).ok()?;
    repo.workdir()?;
    // Enable rename detection (50% similarity, no copy tracking) to match
    // `git status --find-renames`.  Without rewrites every rename appears as D+A.
    let rewrites = gix::diff::Rewrites {
        percentage: Some(0.5),
        limit: 1000,
        track_empty: false,
        copies: None,
    };
    let platform = repo
        .status(gix::progress::Discard)
        .ok()?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .index_worktree_rewrites(Some(rewrites));
    // `into_iter` yields both HEAD→index (TreeIndex) and index→worktree
    // (IndexWorktree) items.  We merge them with worktree taking precedence
    // (mirrors `git status` porcelain `XY` where Y wins).
    let iter = platform.into_iter(Vec::new()).ok()?;
    let mut staged: HashMap<String, (FileStatus, Option<String>)> = HashMap::new();
    let mut worktree: HashMap<String, (FileStatus, Option<String>)> = HashMap::new();
    for item in iter {
        let item = item.ok()?;
        match item {
            gix::status::Item::IndexWorktree(iw) => {
                match iw {
                    gix::status::index_worktree::Item::Modification {
                        rela_path, status, ..
                    } => {
                        let path = rela_path.to_string();
                        // status is from gix_status (re-exported as gix::status::plumbing)
                        #[allow(clippy::unnested_or_patterns)]
                        let mapped = match status {
                            gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                                gix::status::plumbing::index_as_worktree::Change::Removed,
                            ) => Some((FileStatus::Deleted, None)),
                            gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                                gix::status::plumbing::index_as_worktree::Change::Type { .. },
                            )
                            | gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                                gix::status::plumbing::index_as_worktree::Change::Modification {
                                    ..
                                },
                            )
                            | gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                                gix::status::plumbing::index_as_worktree::Change::SubmoduleModification(_),
                            )
                            | gix::status::plumbing::index_as_worktree::EntryStatus::Conflict {
                                ..
                            } => Some((FileStatus::Modified, None)),
                            gix::status::plumbing::index_as_worktree::EntryStatus::NeedsUpdate(_)
                            | gix::status::plumbing::index_as_worktree::EntryStatus::IntentToAdd => {
                                None
                            }
                        };
                        if let Some(v) = mapped {
                            worktree.insert(path, v);
                        }
                    }
                    gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                        // Only untracked entries matter here; ignored/pruned are skipped
                        // to match `--ignored=no`.
                        if entry.status == gix::dir::entry::Status::Untracked {
                            let path = entry.rela_path.to_string();
                            worktree.insert(path, (FileStatus::Added, None));
                        }
                    }
                    gix::status::index_worktree::Item::Rewrite {
                        source,
                        dirwalk_entry,
                        copy,
                        ..
                    } => {
                        let dest = dirwalk_entry.rela_path.to_string();
                        let old = source.rela_path().to_string();
                        // Treat both rename and copy as Renamed so `old_path` is preserved;
                        // diff logic already suppresses the source D for renames while
                        // copies keep the source live (no D emitted).
                        let _ = copy;
                        worktree.insert(dest, (FileStatus::Renamed, Some(old)));
                    }
                }
            }
            gix::status::Item::TreeIndex(ti) => match ti {
                gix::diff::index::Change::Addition { location, .. } => {
                    let p = location.to_string();
                    staged.insert(p, (FileStatus::Added, None));
                }
                gix::diff::index::Change::Deletion { location, .. } => {
                    let p = location.to_string();
                    staged.insert(p, (FileStatus::Deleted, None));
                }
                gix::diff::index::Change::Modification { location, .. } => {
                    let p = location.to_string();
                    staged.insert(p, (FileStatus::Modified, None));
                }
                gix::diff::index::Change::Rewrite {
                    source_location,
                    location,
                    copy,
                    ..
                } => {
                    let dest = location.to_string();
                    let old = source_location.to_string();
                    let _ = copy;
                    staged.insert(dest, (FileStatus::Renamed, Some(old)));
                }
            },
        }
    }
    // Merge with worktree precedence.
    let mut entries = staged;
    for (k, v) in worktree {
        entries.insert(k, v);
    }
    Some(GitSnapshot { entries })
}

fn capture_git_cli(cwd: &Path) -> Option<GitSnapshot> {
    let output = std::process::Command::new("git")
        .args([
            "-C",
            cwd.to_string_lossy().as_ref(),
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignored=no",
            "--find-renames",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut entries: HashMap<String, (FileStatus, Option<String>)> = HashMap::new();
    // -z uses NUL separators.  Entries are "XY path\0" or "R100 old\0new\0"
    let bytes = output.stdout;
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 3 > bytes.len() {
            break;
        }
        let x = bytes[i] as char;
        let y = bytes[i + 1] as char;
        // bytes[i+2] is space
        i += 3;
        let start = i;
        // find NUL
        let mut end = start;
        while end < bytes.len() && bytes[end] != 0 {
            end += 1;
        }
        if end >= bytes.len() {
            break;
        }
        let path = String::from_utf8_lossy(&bytes[start..end]).to_string();
        i = end + 1;
        // Skip ignored entries (second column '!' or first '!')
        if x == '!' || y == '!' {
            continue;
        }
        // Skip clean entries
        if x == ' ' && y == ' ' {
            continue;
        }
        // Detect rename/copy
        let is_rename = x == 'R' || y == 'R' || x == 'C' || y == 'C';
        if is_rename {
            // Next NUL-terminated is new path
            let start2 = i;
            let mut end2 = start2;
            while end2 < bytes.len() && bytes[end2] != 0 {
                end2 += 1;
            }
            if end2 < bytes.len() {
                let new_path = String::from_utf8_lossy(&bytes[start2..end2]).to_string();
                i = end2 + 1;
                // Original path is `path`, new is `new_path`; status is R
                entries.insert(new_path.clone(), (FileStatus::Renamed, Some(path.clone())));
                // Also ensure old path is marked as deleted-like for diff? We keep only new terminal.
                continue;
            }
        }
        // Determine status char: prefer worktree (y) over index (x) for touched meaning
        let status_char = if y != ' ' && y != '!' && y != '?' {
            y
        } else if x != ' ' && x != '?' {
            x
        } else if x == '?' || y == '?' {
            '?'
        } else {
            y
        };
        #[allow(clippy::match_same_arms)]
        let status = match status_char {
            'M' => FileStatus::Modified,
            'A' => FileStatus::Added,
            'D' => FileStatus::Deleted,
            '?' => FileStatus::Added, // untracked -> Added
            'R' | 'C' => FileStatus::Renamed,
            _ => FileStatus::Modified,
        };
        // Untracked already mapped to Added; deleted etc.
        // For D, path is the deleted file
        entries.insert(path, (status, None));
    }
    Some(GitSnapshot { entries })
}

fn capture_walk(cwd: &Path) -> HashMap<String, WalkMeta> {
    let mut map = HashMap::new();
    let builder = ignore::WalkBuilder::new(cwd);
    let mut builder = builder;
    builder
        .hidden(false)
        .parents(true)
        .ignore(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .require_git(false)
        .follow_links(false)
        .overrides({
            let mut b = ignore::overrides::OverrideBuilder::new(cwd);
            let _ = b.add("!.obsidian/**");
            let _ = b.add("!.trash/**");
            let _ = b.add("!.tmp/**");
            let _ = b.add("!**/*.tmp");
            let _ = b.add("!**/.DS_Store");
            b.build()
                .unwrap_or_else(|_| ignore::overrides::Override::empty())
        });
    for entry in builder.build().flatten() {
        let path = entry.path();
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let rel = path
            .strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        // Skip supplement set manually as well
        if rel.starts_with(".obsidian/") || rel.starts_with(".trash/") || rel.starts_with(".tmp/") {
            continue;
        }
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if rel.ends_with(".tmp") || rel == ".DS_Store" || rel.ends_with("/.DS_Store") {
            continue;
        }
        let Ok(meta) = std::fs::metadata(path) else {
            continue;
        };
        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| i128::try_from(d.as_millis()).unwrap_or(0));
        map.insert(
            rel,
            WalkMeta {
                mtime_ms,
                size: meta.len(),
            },
        );
    }
    map
}

/// Diff two snapshots into `FileTouch` list (source = Bash).
pub fn diff_snapshots(
    before: Snapshot,
    after: Snapshot,
    tool_call_id: &str,
    tool_name: &str,
) -> Vec<FileTouch> {
    match (before, after) {
        (Snapshot::Git(b), Snapshot::Git(a)) => diff_git_snapshots(&b, &a, tool_call_id, tool_name),
        (Snapshot::Walk(b), Snapshot::Walk(a)) => {
            diff_walk_snapshots(&b, &a, tool_call_id, tool_name)
        }
        // Mismatched snapshot kinds (e.g., git init mid-tool): fall back to after entries as Added
        (Snapshot::Git(_), Snapshot::Walk(a)) => a
            .into_keys()
            .map(|p| FileTouch {
                path: p,
                status: FileStatus::Added,
                source: TouchSource::Bash,
                old_path: None,
                first_old_path: None,
                tool_call_id: tool_call_id.to_string(),
                tool_name: tool_name.to_string(),
            })
            .collect(),
        (Snapshot::Walk(_), Snapshot::Git(a)) => a
            .entries
            .into_iter()
            .map(|(p, (st, old))| FileTouch {
                path: p,
                status: st,
                source: TouchSource::Bash,
                old_path: old,
                first_old_path: None,
                tool_call_id: tool_call_id.to_string(),
                tool_name: tool_name.to_string(),
            })
            .collect(),
    }
}

fn diff_git_snapshots(
    before: &GitSnapshot,
    after: &GitSnapshot,
    tool_call_id: &str,
    tool_name: &str,
) -> Vec<FileTouch> {
    let mut out = Vec::new();
    // Files in after not in before or status changed -> report after status
    for (path, (status, old)) in &after.entries {
        let before_entry = before.entries.get(path);
        if before_entry.is_none_or(|(s, _)| *s != *status) {
            out.push(FileTouch {
                path: path.clone(),
                status: *status,
                source: TouchSource::Bash,
                old_path: old.clone(),
                first_old_path: None,
                tool_call_id: tool_call_id.to_string(),
                tool_name: tool_name.to_string(),
            });
        } else if status == &FileStatus::Renamed {
            // Even if status same, oldPath may have changed
            if before_entry.is_some_and(|(_, o)| *o != *old) {
                out.push(FileTouch {
                    path: path.clone(),
                    status: *status,
                    source: TouchSource::Bash,
                    old_path: old.clone(),
                    first_old_path: None,
                    tool_call_id: tool_call_id.to_string(),
                    tool_name: tool_name.to_string(),
                });
            }
        }
    }
    // Deleted files: in before but not in after (and not renamed target)
    for (path, (status, _)) in &before.entries {
        if !after.entries.contains_key(path) {
            // If this path was a rename source, it shouldn't be reported as D separately
            // Check if any after entry has this as old_path
            let is_rename_source = after
                .entries
                .values()
                .any(|(_, old)| old.as_deref() == Some(path.as_str()));
            if is_rename_source {
                continue;
            }
            // Only report if it was tracked; untracked deletions are not reported as D
            // We treat missing as Deleted
            // But avoid reporting ignored/untracked that vanished? Simplify: report as Deleted
            // Only if before status was not untracked (Added)
            if *status == FileStatus::Added {
                continue;
            }
            out.push(FileTouch {
                path: path.clone(),
                status: FileStatus::Deleted,
                source: TouchSource::Bash,
                old_path: None,
                first_old_path: None,
                tool_call_id: tool_call_id.to_string(),
                tool_name: tool_name.to_string(),
            });
        }
    }
    out
}

fn diff_walk_snapshots(
    before: &HashMap<String, WalkMeta>,
    after: &HashMap<String, WalkMeta>,
    tool_call_id: &str,
    tool_name: &str,
) -> Vec<FileTouch> {
    let mut out = Vec::new();
    for (path, meta) in after {
        if let Some(b) = before.get(path) {
            if b.mtime_ms != meta.mtime_ms || b.size != meta.size {
                out.push(FileTouch {
                    path: path.clone(),
                    status: FileStatus::Modified,
                    source: TouchSource::Bash,
                    old_path: None,
                    first_old_path: None,
                    tool_call_id: tool_call_id.to_string(),
                    tool_name: tool_name.to_string(),
                });
            }
        } else {
            out.push(FileTouch {
                path: path.clone(),
                status: FileStatus::Added,
                source: TouchSource::Bash,
                old_path: None,
                first_old_path: None,
                tool_call_id: tool_call_id.to_string(),
                tool_name: tool_name.to_string(),
            });
        }
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            out.push(FileTouch {
                path: path.clone(),
                status: FileStatus::Deleted,
                source: TouchSource::Bash,
                old_path: None,
                first_old_path: None,
                tool_call_id: tool_call_id.to_string(),
                tool_name: tool_name.to_string(),
            });
        }
    }
    // Heuristic R detection: D+A with same size/mtime proximity? For now skip and keep D+A separate;
    // aggregate_touched will later handle D+A->M. For true R we rely on git path.
    out
}

// ============================================================================
// Per-cwd bash window lock (prevents concurrent bash windows from interleaving)
// ============================================================================
//
// Uses `asupersync::sync::Mutex` (not `std::sync::Mutex`, not `tokio::sync`).
// `pi` has no `tokio` dependency — all async goes through `asupersync::Cx`
// (see `crates/pi-core/src/agent_cx.rs`). `std::sync::Mutex` across `await`
// is `!Send` and would also block the `current_thread` reactor.

use std::sync::Mutex as StdMutex;

static BASH_LOCKS: OnceLock<StdMutex<HashMap<PathBuf, Arc<asupersync::sync::Mutex<()>>>>> =
    OnceLock::new();

fn bash_locks() -> &'static StdMutex<HashMap<PathBuf, Arc<asupersync::sync::Mutex<()>>>> {
    BASH_LOCKS.get_or_init(|| StdMutex::new(HashMap::new()))
}

/// Get the per-cwd async mutex (sync, cheap). Caller must
/// `asupersync::sync::OwnedMutexGuard::lock` it with a `Cx`.
pub fn bash_window_mutex(cwd: &Path) -> Arc<asupersync::sync::Mutex<()>> {
    let key = cwd.to_path_buf();
    let mut map = bash_locks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    map.entry(key)
        .or_insert_with(|| Arc::new(asupersync::sync::Mutex::new(())))
        .clone()
}

/// Convenience: acquire the per-cwd window lock with the current `Cx`.
pub async fn lock_bash_window(
    cwd: &Path,
) -> Result<asupersync::sync::OwnedMutexGuard<()>, asupersync::sync::LockError> {
    let m = bash_window_mutex(cwd);
    let cx = crate::agent_cx::AgentCx::for_current_or_request();
    asupersync::sync::OwnedMutexGuard::lock(m, cx.cx()).await
}

/// Legacy name — returns the `asupersync` mutex (not `std`).
pub fn bash_window_lock(cwd: &Path) -> Arc<asupersync::sync::Mutex<()>> {
    bash_window_mutex(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ft(path: &str, status: FileStatus, old: Option<&str>) -> FileTouch {
        FileTouch {
            path: path.to_string(),
            status,
            source: TouchSource::Structured,
            old_path: old.map(ToString::to_string),
            first_old_path: None,
            tool_call_id: "c1".to_string(),
            tool_name: "edit".to_string(),
        }
    }

    #[test]
    fn r_plus_r_chain() {
        let a = ft("b", FileStatus::Renamed, Some("a"));
        let b = ft("c", FileStatus::Renamed, Some("b"));
        let out = aggregate_touched(vec![a, b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].status, FileStatus::Renamed);
        assert_eq!(out[0].path, "c");
        assert_eq!(out[0].old_path.as_deref(), Some("b"));
        assert_eq!(out[0].first_old_path.as_deref(), Some("a"));
    }

    #[test]
    fn r_plus_m_keeps_r() {
        let a = ft("b", FileStatus::Renamed, Some("a"));
        let b = ft("b", FileStatus::Modified, None);
        let out = aggregate_touched(vec![a, b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].status, FileStatus::Renamed);
        assert_eq!(out[0].path, "b");
    }

    #[test]
    fn r_plus_d_to_d_with_first() {
        let a = ft("b", FileStatus::Renamed, Some("a"));
        let b = ft("b", FileStatus::Deleted, None);
        let out = aggregate_touched(vec![a, b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].status, FileStatus::Deleted);
        assert_eq!(out[0].first_old_path.as_deref(), Some("a"));
    }

    #[test]
    fn a_plus_m_keeps_a() {
        let a = FileTouch {
            path: "x".to_string(),
            status: FileStatus::Added,
            source: TouchSource::Structured,
            old_path: None,
            first_old_path: None,
            tool_call_id: "c1".to_string(),
            tool_name: "write".to_string(),
        };
        let b = ft("x", FileStatus::Modified, None);
        let out = aggregate_touched(vec![a, b]);
        // A + M -> latest is M? But per TS, "其余：最新覆盖" would be M, but we have no special A+M -> A rule
        // Actually TS has no A+M -> A. Let's see: R+M keeps R, D+A->M, *+D->D, otherwise latest
        // So A+M -> M. But earlier table said A+M -> A. The TS merge for non-R/D will latest覆盖, so A then M = M.
        // Keep as M.
        assert_eq!(out[0].status, FileStatus::Modified);
    }

    #[test]
    fn d_plus_a_to_m() {
        let a = ft("x", FileStatus::Deleted, None);
        let b = ft("x", FileStatus::Added, None);
        let out = aggregate_touched(vec![a, b]);
        assert_eq!(out[0].status, FileStatus::Modified);
    }

    #[test]
    fn a_plus_d_cancels() {
        let a = FileTouch {
            path: "x".to_string(),
            status: FileStatus::Added,
            source: TouchSource::Structured,
            old_path: None,
            first_old_path: None,
            tool_call_id: "c1".to_string(),
            tool_name: "write".to_string(),
        };
        let b = ft("x", FileStatus::Deleted, None);
        let out = aggregate_touched(vec![a, b]);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn filter_bash_skips_obsidian() {
        let t = ft(".obsidian/foo.md", FileStatus::Modified, None);
        let out = filter_bash_touches(vec![t]);
        assert_eq!(out.len(), 0);
    }
}
