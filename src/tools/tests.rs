use super::*;
use proptest::prelude::*;

#[cfg(target_os = "linux")]
use std::time::Duration;

use super::edit::{detect_line_ending, normalize_to_lf, restore_line_endings, strip_bom};

#[test]
fn test_truncate_head() {
    let content = "line1\nline2\nline3\nline4\nline5".to_string();
    let result = truncate_head(content, 3, 1000);

    assert_eq!(result.content, "line1\nline2\nline3\n");
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncatedBy::Lines));
    assert_eq!(result.total_lines, 5);
    assert_eq!(result.output_lines, 3);
}

#[test]
fn test_truncate_tail() {
    let content = "line1\nline2\nline3\nline4\nline5".to_string();
    let result = truncate_tail(content, 3, 1000);

    assert_eq!(result.content, "line3\nline4\nline5");
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncatedBy::Lines));
    assert_eq!(result.total_lines, 5);
    assert_eq!(result.output_lines, 3);
}

fn assert_same_head_truncation(actual: &TruncationResult, expected: &TruncationResult) {
    assert_eq!(actual.content, expected.content);
    assert_eq!(actual.truncated, expected.truncated);
    assert_eq!(actual.truncated_by, expected.truncated_by);
    assert_eq!(actual.total_lines, expected.total_lines);
    assert_eq!(actual.total_bytes, expected.total_bytes);
    assert_eq!(actual.output_lines, expected.output_lines);
    assert_eq!(actual.output_bytes, expected.output_bytes);
    assert_eq!(actual.last_line_partial, expected.last_line_partial);
    assert_eq!(
        actual.first_line_exceeds_limit,
        expected.first_line_exceeds_limit
    );
    assert_eq!(actual.max_lines, expected.max_lines);
    assert_eq!(actual.max_bytes, expected.max_bytes);
}

fn write_lines_with_builder(lines: &[&str], max_bytes: usize) -> TruncationResult {
    let mut writer = HeadTruncatingLineWriter::new(max_bytes);
    for line in lines {
        writer.push_line(line);
    }
    writer.finish()
}

#[test]
fn head_truncating_line_writer_matches_join_without_truncation() {
    let lines = ["alpha", "beta", "gamma"];
    let expected = truncate_head(lines.join("\n"), usize::MAX, 1000);
    let actual = write_lines_with_builder(&lines, 1000);

    assert_same_head_truncation(&actual, &expected);
}

#[test]
fn head_truncating_line_writer_matches_join_at_byte_boundary() {
    let lines = ["alpha", "beta", "gamma"];
    let expected = truncate_head(lines.join("\n"), usize::MAX, 8);
    let actual = write_lines_with_builder(&lines, 8);

    assert_same_head_truncation(&actual, &expected);
    assert_eq!(actual.content, "alpha\nbe");
}

#[test]
fn head_truncating_line_writer_preserves_utf8_boundary_and_order() {
    let lines = ["alpha", "βeta", "gamma"];
    let expected = truncate_head(lines.join("\n"), usize::MAX, 8);
    let actual = write_lines_with_builder(&lines, 8);

    assert_same_head_truncation(&actual, &expected);
    assert_eq!(actual.content, "alpha\nβ");
}

fn first_text(output: &ToolOutput) -> &str {
    output
        .content
        .first()
        .and_then(|block| match block {
            ContentBlock::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .unwrap_or("")
}

fn artifact_json(details: Option<&serde_json::Value>) -> &serde_json::Value {
    details
        .and_then(|value| value.get("artifact"))
        .expect("artifact details")
}

fn artifact_str_field<'a>(artifact: &'a serde_json::Value, field: &str) -> &'a str {
    artifact
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

#[test]
fn tool_output_artifact_respects_spill_threshold() {
    let tmp = tempfile::tempdir().expect("artifact root");
    let mut output = "small preview".to_string();
    let mut details = None;
    let spilled = attach_text_artifact_if_needed_at_root(
        tmp.path(),
        &mut output,
        &mut details,
        "read",
        "call-small",
        "selectedTextWindow",
        "small body",
    );

    assert!(!spilled);
    assert_eq!(output, "small preview");
    assert!(details.is_none());
}

#[test]
fn tool_output_artifact_writes_content_addressed_text_and_metadata()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir().expect("artifact root");
    let full = "a".repeat(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES + 1);
    let mut output = "bounded preview".to_string();
    let mut details = None;
    let _session_guard =
        register_tool_output_artifact_session("call/text:1", "session/artifacts:one");
    let spilled = attach_text_artifact_if_needed_at_root(
        tmp.path(),
        &mut output,
        &mut details,
        "read",
        "call/text:1",
        "selectedTextWindow",
        &full,
    );

    assert!(spilled);
    assert!(output.contains("Full tool output artifact:"));
    let artifact = artifact_json(details.as_ref());
    assert_eq!(artifact["schema"], TOOL_OUTPUT_ARTIFACT_SCHEMA_V1);
    assert_eq!(artifact["toolName"], "read");
    assert_eq!(artifact["sourceKind"], "selectedTextWindow");
    assert_eq!(artifact["sessionId"], "session/artifacts:one");
    assert_eq!(
        artifact["byteCount"].as_u64().unwrap(),
        u64::try_from(full.len()).unwrap()
    );

    let path_value = artifact_str_field(artifact, "path");
    let metadata_path_value = artifact_str_field(artifact, "metadataPath");
    assert!(!path_value.is_empty(), "artifact path must be a string");
    assert!(
        !metadata_path_value.is_empty(),
        "artifact metadataPath must be a string"
    );
    let path = PathBuf::from(path_value);
    let metadata_path = PathBuf::from(metadata_path_value);
    assert!(path.starts_with(tmp.path().join("session_artifacts_one").join("call_text_1")));
    assert_eq!(std::fs::read_to_string(path)?, full);
    let metadata_bytes = std::fs::read(metadata_path)?;
    let metadata: serde_json::Value = serde_json::from_slice(&metadata_bytes)?;
    assert_eq!(metadata["sha256"], artifact["sha256"]);
    assert_eq!(
        metadata["retentionClass"],
        TOOL_OUTPUT_ARTIFACT_RETENTION_CLASS
    );
    assert_eq!(
        metadata["spilloverReason"],
        TOOL_OUTPUT_ARTIFACT_SPILLOVER_REASON
    );
    assert_eq!(metadata["safeDeleteCandidate"], true);
    assert_eq!(
        metadata["redactionSummary"]["policy"],
        TOOL_OUTPUT_ARTIFACT_REDACTION_POLICY_V1
    );
    assert_eq!(metadata["redactionSummary"]["status"], "clean");
    assert_eq!(metadata["redactionSummary"]["rawSecretBytesEmitted"], 0);
    Ok(())
}

#[test]
fn tool_output_artifact_redacts_sensitive_text_before_persisting()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir().expect("artifact root");
    let leaked_token = "sk-redactionfixture1234567890";
    let leaked_bearer = "ghp_redactionfixture1234567890";
    let full = format!(
        "API_TOKEN={leaked_token}\nAuthorization: Bearer {leaked_bearer}\n{}",
        "x".repeat(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES + 1)
    );
    let mut output = "bounded preview".to_string();
    let mut details = None;

    let spilled = attach_text_artifact_if_needed_at_root(
        tmp.path(),
        &mut output,
        &mut details,
        "read",
        "call-secret",
        "selectedTextWindow",
        &full,
    );

    assert!(spilled);
    let artifact = artifact_json(details.as_ref());
    let path = PathBuf::from(artifact_str_field(artifact, "path"));
    let metadata_path = PathBuf::from(artifact_str_field(artifact, "metadataPath"));
    let persisted = std::fs::read_to_string(path)?;
    let metadata: serde_json::Value = serde_json::from_slice(&std::fs::read(metadata_path)?)?;

    assert!(!persisted.contains(leaked_token));
    assert!(!persisted.contains(leaked_bearer));
    assert!(persisted.contains("API_TOKEN=[REDACTED]"));
    assert_eq!(artifact["redactionSummary"]["status"], "redacted");
    assert_eq!(artifact["redactionSummary"]["rawSecretBytesEmitted"], 0);
    assert_eq!(metadata["redactionSummary"], artifact["redactionSummary"]);
    let fields = artifact["redactionSummary"]["fields"]
        .as_array()
        .expect("redaction fields");
    assert!(fields.iter().any(|field| field == "api_token"));
    assert!(fields.iter().any(|field| field == "authorization"));
    Ok(())
}

#[test]
fn tool_output_artifact_marks_binaryish_payloads_in_lifecycle_manifest() {
    let tmp = tempfile::tempdir().expect("artifact root");
    let full = format!(
        "{}\0{}",
        "z".repeat(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES / 2),
        "z".repeat(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES / 2 + 2)
    );
    let mut output = "bounded preview".to_string();
    let mut details = None;

    let spilled = attach_text_artifact_if_needed_at_root(
        tmp.path(),
        &mut output,
        &mut details,
        "read",
        "call-binaryish",
        "selectedTextWindow",
        &full,
    );

    assert!(spilled);
    let artifact = artifact_json(details.as_ref());
    assert_eq!(artifact["redactionSummary"]["binarySuspect"], true);
    assert_eq!(artifact["redactionSummary"]["rawSecretBytesEmitted"], 0);
    assert_eq!(artifact["safeDeleteCandidate"], true);
}

#[test]
fn tool_output_artifact_failure_records_degraded_preview() {
    let tmp = tempfile::tempdir().expect("artifact root parent");
    let root_file = tmp.path().join("not-a-directory");
    std::fs::write(&root_file, "not a directory").expect("root file");
    let full = "b".repeat(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES + 1);
    let mut output = "bounded preview".to_string();
    let mut details = None;

    let spilled = attach_text_artifact_if_needed_at_root(
        &root_file,
        &mut output,
        &mut details,
        "read",
        "call-fail",
        "selectedTextWindow",
        &full,
    );

    assert!(!spilled);
    assert!(output.contains("Tool output artifact persistence failed"));
    assert!(
        details
            .as_ref()
            .and_then(|value| value.get("artifactError"))
            .is_some()
    );
}

#[test]
fn read_tool_spills_oversized_selected_text_window_to_artifact() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().expect("workspace");
        let artifact_root = tempfile::tempdir().expect("artifact root");

        let body = "r".repeat(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES + 8);
        std::fs::write(tmp.path().join("large.txt"), &body).expect("large file");
        let read_tool = ReadTool::with_artifact_root(tmp.path(), artifact_root.path());
        let output = read_tool
            .execute(
                "read-artifact-call",
                serde_json::json!({ "path": "large.txt" }),
                None,
                None,
            )
            .await
            .expect("read large file");

        assert!(first_text(&output).contains("Full tool output artifact:"));
        let artifact = artifact_json(output.details.as_ref());
        assert_eq!(artifact["toolName"], "read");
        assert_eq!(artifact["sourceKind"], "selectedTextWindow");
        let path_value = artifact_str_field(artifact, "path");
        assert!(!path_value.is_empty(), "artifact path must be a string");
        let path = PathBuf::from(path_value);
        let spilled = match std::fs::read_to_string(&path) {
            Ok(spilled) => spilled,
            Err(err) => {
                assert!(false, "read spilled artifact {}: {err}", path.display());
                return;
            }
        };
        let prefix = "    1→";
        assert_eq!(spilled.len(), prefix.len() + DEFAULT_MAX_BYTES);
        assert_eq!(
            artifact["byteCount"].as_u64().unwrap(),
            u64::try_from(spilled.len()).unwrap()
        );
        assert!(spilled.starts_with(prefix));
        assert!(spilled[prefix.len()..].bytes().all(|byte| byte == b'r'));
        assert_eq!(
            artifact["retentionClass"],
            TOOL_OUTPUT_ARTIFACT_RETENTION_CLASS
        );
        assert_eq!(
            artifact["spilloverReason"],
            TOOL_OUTPUT_ARTIFACT_SPILLOVER_REASON
        );
        assert_eq!(artifact["safeDeleteCandidate"], true);
    });
}

#[test]
fn bash_tool_spills_truncated_full_output_to_artifact() {
    asupersync::test_utils::run_test(|| async {
        if !Path::new("/dev/zero").exists() {
            return;
        }

        let tmp = tempfile::tempdir().expect("workspace");
        let artifact_root = tempfile::tempdir().expect("artifact root");

        let bash_tool = BashTool::with_artifact_root(tmp.path(), artifact_root.path());
        let output = bash_tool
            .execute(
                "bash-artifact-call",
                serde_json::json!({
                    "command": "head -c 1001000 /dev/zero | tr '\\0' x",
                    "timeout": 10
                }),
                None,
                None,
            )
            .await
            .expect("bash large output");

        assert!(first_text(&output).contains("Full tool output artifact:"));
        let artifact = artifact_json(output.details.as_ref());
        assert_eq!(artifact["toolName"], "bash");
        assert_eq!(artifact["sourceKind"], "fullCommandOutput");
        let path = PathBuf::from(artifact_str_field(artifact, "path"));
        assert_eq!(std::fs::metadata(path).unwrap().len(), 1_001_000);
        assert_eq!(artifact["redactionSummary"]["status"], "clean");
        assert_eq!(artifact["safeDeleteCandidate"], true);
    });
}

#[test]
fn bash_tool_redacts_secret_like_full_output_artifacts() {
    asupersync::test_utils::run_test(|| async {
        if !Path::new("/dev/zero").exists() {
            return;
        }

        let tmp = tempfile::tempdir().expect("workspace");
        let artifact_root = tempfile::tempdir().expect("artifact root");
        let leaked_token = "sk-bashredactionfixture1234567890";

        let bash_tool = BashTool::with_artifact_root(tmp.path(), artifact_root.path());
        let output = bash_tool
            .execute(
                "bash-secret-artifact-call",
                serde_json::json!({
                    "command": format!("printf 'API_TOKEN={leaked_token}\\n'; head -c 1001000 /dev/zero | tr '\\0' x"),
                    "timeout": 10
                }),
                None,
                None,
            )
            .await
            .expect("bash large output");

        assert!(first_text(&output).contains("Full tool output artifact:"));
        let artifact = artifact_json(output.details.as_ref());
        assert_eq!(artifact["toolName"], "bash");
        assert_eq!(artifact["redactionSummary"]["status"], "redacted");
        assert_eq!(artifact["redactionSummary"]["rawSecretBytesEmitted"], 0);
        let path = PathBuf::from(artifact_str_field(artifact, "path"));
        let persisted = std::fs::read_to_string(path).expect("read redacted bash artifact");
        assert!(!persisted.contains(leaked_token));
        assert!(persisted.contains("API_TOKEN=[REDACTED]"));
    });
}

#[test]
fn grep_tool_spills_large_search_results_with_lifecycle_manifest() {
    asupersync::test_utils::run_test(|| async {
        if !rg_available() {
            return;
        }

        let tmp = tempfile::tempdir().expect("workspace");
        let artifact_root = tempfile::tempdir().expect("artifact root");
        let mut body = String::new();
        let suffix = "g".repeat(560);
        for idx in 0..2200 {
            let _ = writeln!(body, "target {idx:04} {suffix}");
        }
        std::fs::write(tmp.path().join("large-grep.txt"), body).expect("write grep fixture");

        let grep_tool = GrepTool::with_artifact_root(tmp.path(), artifact_root.path());
        let output = grep_tool
            .execute(
                "grep-artifact-call",
                serde_json::json!({
                    "pattern": "target",
                    "path": "large-grep.txt",
                    "literal": true,
                    "limit": 2200
                }),
                None,
                None,
            )
            .await
            .expect("grep large output");

        assert!(first_text(&output).contains("Full tool output artifact:"));
        let artifact = artifact_json(output.details.as_ref());
        assert_eq!(artifact["toolName"], "grep");
        assert_eq!(artifact["sourceKind"], "searchResults");
        assert_eq!(
            artifact["retentionClass"],
            TOOL_OUTPUT_ARTIFACT_RETENTION_CLASS
        );
        assert_eq!(artifact["safeDeleteCandidate"], true);
        assert_eq!(artifact["redactionSummary"]["status"], "clean");
        let path = PathBuf::from(artifact_str_field(artifact, "path"));
        let persisted = std::fs::read_to_string(path).expect("read grep artifact");
        assert!(persisted.contains("large-grep.txt:1: target 0000"));
        assert!(
            artifact["byteCount"].as_u64().unwrap()
                > u64::try_from(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES).unwrap()
        );
    });
}

/// FIXME: ReadTool does not yet enforce scope (needs `enforce_cwd_scope` call in
/// `read_single_file`). This test is ignored until scope enforcement is added.
#[ignore = "flaky on CI"]
#[test]
fn read_tool_denied_path_does_not_emit_lifecycle_artifact() {
    asupersync::test_utils::run_test(|| async {
        let cwd = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        let artifact_root = tempfile::tempdir().expect("artifact root");
        let outside_path = outside.path().join("secret.txt");
        std::fs::write(&outside_path, "API_TOKEN=sk-deniedpathfixture1234567890")
            .expect("outside secret");

        let read_tool = ReadTool::with_artifact_root(cwd.path(), artifact_root.path());
        let err = read_tool
            .execute(
                "read-denied-artifact-call",
                serde_json::json!({ "path": outside_path }),
                None,
                None,
            )
            .await
            .expect_err("outside read should be denied");

        assert!(
            err.to_string()
                .contains("Cannot read outside the working directory or agent dir")
        );
        let mut entries = std::fs::read_dir(artifact_root.path()).expect("artifact root");
        assert!(
            entries.next().is_none(),
            "denied reads must not write artifacts"
        );
    });
}

#[test]
fn ls_tool_spills_oversized_directory_listing_to_artifact() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().expect("workspace");
        let artifact_root = tempfile::tempdir().expect("artifact root");
        let suffix = "x".repeat(224);
        for i in 0..4_500 {
            let name = format!("entry-{i:04}-{suffix}.txt");
            std::fs::write(tmp.path().join(name), "").expect("write listing fixture");
        }

        let ls_tool = LsTool::with_artifact_root(tmp.path(), artifact_root.path());
        let output = ls_tool
            .execute(
                "ls-artifact-call",
                serde_json::json!({ "path": ".", "limit": 4500 }),
                None,
                None,
            )
            .await
            .expect("ls large directory");

        assert!(first_text(&output).contains("Full tool output artifact:"));
        let artifact = artifact_json(output.details.as_ref());
        assert_eq!(artifact["toolName"], "ls");
        assert_eq!(artifact["sourceKind"], "directoryEntries");
        assert!(
            artifact["byteCount"].as_u64().unwrap()
                > u64::try_from(TOOL_OUTPUT_ARTIFACT_THRESHOLD_BYTES).unwrap()
        );
        let path = PathBuf::from(artifact_str_field(artifact, "path"));
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains("entry-0000-")
        );
    });
}

async fn assert_read_cache_hit_and_stale(tmp: &Path) {
    let note = tmp.join("note.txt");
    std::fs::write(&note, "alpha\n").expect("write note");

    let read_tool = ReadTool::new(tmp);
    let read_input = serde_json::json!({ "path": "note.txt" });
    let first = read_tool
        .execute("read-1", read_input.clone(), None, None)
        .await
        .expect("first read");
    assert!(first_text(&first).contains("alpha"));

    let hits_before = tool_output_cache_stats_for_tests().hits;
    let second = read_tool
        .execute("read-2", read_input.clone(), None, None)
        .await
        .expect("cached read");
    assert_eq!(first_text(&first), first_text(&second));
    assert!(tool_output_cache_stats_for_tests().hits > hits_before);

    let invalidations_before = tool_output_cache_stats_for_tests().invalidations;
    std::fs::write(&note, "beta\n").expect("rewrite note");
    let third = read_tool
        .execute("read-3", read_input.clone(), None, None)
        .await
        .expect("invalidated read");
    assert!(first_text(&third).contains("beta"));
    assert!(!first_text(&third).contains("alpha"));
    assert!(tool_output_cache_stats_for_tests().invalidations > invalidations_before);
}

async fn assert_ls_cache_hit_and_stale(tmp: &Path) {
    let ls_tool = LsTool::new(tmp);
    let ls_input = serde_json::json!({ "path": "." });
    let ls_first = ls_tool
        .execute("ls-1", ls_input.clone(), None, None)
        .await
        .expect("first ls");
    assert!(first_text(&ls_first).contains("note.txt"));

    let hits_before = tool_output_cache_stats_for_tests().hits;
    let ls_second = ls_tool
        .execute("ls-2", ls_input.clone(), None, None)
        .await
        .expect("cached ls");
    assert_eq!(first_text(&ls_first), first_text(&ls_second));
    assert!(tool_output_cache_stats_for_tests().hits > hits_before);

    let invalidations_before = tool_output_cache_stats_for_tests().invalidations;
    std::fs::write(tmp.join("new.txt"), "new\n").expect("write new file");
    let ls_third = ls_tool
        .execute("ls-3", ls_input.clone(), None, None)
        .await
        .expect("invalidated ls");
    assert!(first_text(&ls_third).contains("new.txt"));
    assert!(tool_output_cache_stats_for_tests().invalidations > invalidations_before);
}

async fn assert_grep_cache_hit_and_stale_when_available(tmp: &Path) {
    if find_rg_binary().is_none() {
        return;
    }

    let grep_tool = GrepTool::new(tmp);
    let grep_input = serde_json::json!({ "pattern": "needle", "path": "." });
    std::fs::write(tmp.join("a.txt"), "needle\n").expect("write grep file");

    let grep_first = grep_tool
        .execute("grep-1", grep_input.clone(), None, None)
        .await
        .expect("first grep");
    assert!(first_text(&grep_first).contains("a.txt"));

    let hits_before = tool_output_cache_stats_for_tests().hits;
    let grep_second = grep_tool
        .execute("grep-2", grep_input.clone(), None, None)
        .await
        .expect("cached grep");
    assert_eq!(first_text(&grep_first), first_text(&grep_second));
    assert!(tool_output_cache_stats_for_tests().hits > hits_before);

    let invalidations_before = tool_output_cache_stats_for_tests().invalidations;
    std::fs::write(tmp.join("b.txt"), "needle\n").expect("write new match");
    let grep_third = grep_tool
        .execute("grep-3", grep_input.clone(), None, None)
        .await
        .expect("invalidated grep");
    assert!(first_text(&grep_third).contains("b.txt"));
    assert!(tool_output_cache_stats_for_tests().invalidations > invalidations_before);
}

async fn assert_find_cache_hit_and_stale_when_available(tmp: &Path) {
    if find_fd_binary().is_none() {
        return;
    }

    let find_tool = FindTool::new(tmp);
    let find_input = serde_json::json!({ "pattern": "*find*.txt", "path": "." });
    std::fs::write(tmp.join("find-a.txt"), "find\n").expect("write first find file");

    let find_first = find_tool
        .execute("find-1", find_input.clone(), None, None)
        .await
        .expect("first find");
    assert!(first_text(&find_first).contains("find-a.txt"));

    let hits_before = tool_output_cache_stats_for_tests().hits;
    let find_second = find_tool
        .execute("find-2", find_input.clone(), None, None)
        .await
        .expect("cached find");
    assert_eq!(first_text(&find_first), first_text(&find_second));
    assert!(tool_output_cache_stats_for_tests().hits > hits_before);

    let invalidations_before = tool_output_cache_stats_for_tests().invalidations;
    std::fs::write(tmp.join("find-b.txt"), "find\n").expect("write second find file");
    let find_third = find_tool
        .execute("find-3", find_input.clone(), None, None)
        .await
        .expect("invalidated find");
    assert!(first_text(&find_third).contains("find-b.txt"));
    assert!(tool_output_cache_stats_for_tests().invalidations > invalidations_before);
}

async fn assert_side_effect_tools_remain_uncached(tmp: &Path) {
    let side_effect_stats_before = tool_output_cache_stats_for_tests();
    let write_tool = WriteTool::new(tmp);
    write_tool
        .execute(
            "write-1",
            serde_json::json!({
                "path": "side-effect.txt",
                "content": "one\n"
            }),
            None,
            None,
        )
        .await
        .expect("write side-effect file");

    let edit_tool = EditTool::new(tmp);
    edit_tool
        .execute(
            "edit-1",
            serde_json::json!({
                "path": "side-effect.txt",
                "oldText": "one",
                "newText": "two"
            }),
            None,
            None,
        )
        .await
        .expect("edit side-effect file");

    if bash_available() {
        let bash_tool = BashTool::new(tmp);
        bash_tool
            .execute(
                "bash-1",
                serde_json::json!({
                    "command": "printf 'cache-uncached\\n'",
                    "timeout": 5
                }),
                None,
                None,
            )
            .await
            .expect("run uncached bash");
    }

    let side_effect_stats_after = tool_output_cache_stats_for_tests();

    let side_effect_stats_after = tool_output_cache_stats_for_tests();
    assert_eq!(
        (
            side_effect_stats_after.side_effect_accesses,
            side_effect_stats_after.side_effect_insert_attempts
        ),
        (
            side_effect_stats_before.side_effect_accesses,
            side_effect_stats_before.side_effect_insert_attempts
        ),
        "write, edit, and bash must not consult or populate the read-only output cache"
    );
}

#[test]
fn tool_output_cache_reuses_and_invalidates_read_only_tool_outputs() {
    asupersync::test_utils::run_test(|| async {
        reset_tool_output_cache_for_tests();

        let tmp = tempfile::tempdir().expect("create temp dir");
        assert_read_cache_hit_and_stale(tmp.path()).await;
        assert_ls_cache_hit_and_stale(tmp.path()).await;
        assert_grep_cache_hit_and_stale_when_available(tmp.path()).await;
        assert_find_cache_hit_and_stale_when_available(tmp.path()).await;
        assert_side_effect_tools_remain_uncached(tmp.path()).await;
    });
}

#[test]
fn tool_output_context_cache_evidence_jsonl_covers_required_decisions()
-> std::result::Result<(), String> {
    let evidence = include_str!("../../docs/evidence/tool-output-context-cache.jsonl");
    let mut saw_read_hit = false;
    let mut saw_grep_stale = false;
    let mut saw_find_stale = false;
    let mut saw_ls_stale = false;
    let mut saw_write_uncached = false;
    let mut saw_edit_uncached = false;
    let mut saw_bash_uncached = false;

    for (line_number, line) in evidence.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let event: serde_json::Value = serde_json::from_str(line).map_err(|err| {
            format!(
                "invalid context-cache JSONL at line {}: {err}",
                line_number + 1
            )
        })?;
        assert_eq!(
            event.get("schema").and_then(serde_json::Value::as_str),
            Some("pi.tool_output_context_cache.evidence.v1")
        );
        assert_eq!(
            event.get("bead").and_then(serde_json::Value::as_str),
            Some("bd-dklqn.1")
        );
        let related_beads = event
            .get("related_beads")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("missing related_beads at line {}", line_number + 1))?;
        assert!(
            related_beads
                .iter()
                .any(|bead| bead.as_str() == Some("bd-dklqn.2")),
            "evidence line {} must cover bd-dklqn.2",
            line_number + 1
        );

        let tool = event
            .get("tool")
            .and_then(serde_json::Value::as_str)
            .expect("tool");
        let outcome = event
            .get("outcome")
            .and_then(serde_json::Value::as_str)
            .expect("outcome");
        let reason = event
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .expect("reason");

        match (tool, outcome, reason) {
            ("read", "hit", "unchanged_file_fingerprint") => saw_read_hit = true,
            ("grep", "stale", "recursive_directory_fingerprint_changed") => {
                saw_grep_stale = true;
            }
            ("find", "stale", "recursive_directory_fingerprint_changed") => {
                saw_find_stale = true;
            }
            ("ls", "stale", "directory_entry_fingerprint_changed") => saw_ls_stale = true,
            ("write", "uncached", "write_effect_tool") => saw_write_uncached = true,
            ("edit", "uncached", "write_effect_tool") => saw_edit_uncached = true,
            ("bash", "uncached", "process_effect_tool") => saw_bash_uncached = true,
            _ => {}
        }
    }

    assert!(saw_read_hit, "evidence must include a read cache hit");
    assert!(saw_grep_stale, "evidence must include grep stale bypass");
    assert!(saw_find_stale, "evidence must include find stale bypass");
    assert!(saw_ls_stale, "evidence must include ls stale bypass");
    assert!(saw_write_uncached, "evidence must include write uncached");
    assert!(saw_edit_uncached, "evidence must include edit uncached");
    assert!(saw_bash_uncached, "evidence must include bash uncached");
    Ok(())
}

#[test]
fn test_truncate_tail_zero_lines_returns_empty_output() {
    let result = truncate_tail("line1\nline2".to_string(), 0, 1000);

    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncatedBy::Lines));
    assert_eq!(result.output_lines, 0);
    assert_eq!(result.output_bytes, 0);
    assert!(result.content.is_empty());
}

#[test]
fn test_line_count_from_newline_count_matches_trailing_newline_semantics() {
    assert_eq!(line_count_from_newline_count(0, 0, false), 0);
    assert_eq!(line_count_from_newline_count(2, 1, true), 1);
    assert_eq!(line_count_from_newline_count(1, 0, false), 1);
    assert_eq!(line_count_from_newline_count(3, 1, false), 2);
}

#[test]
fn test_rg_match_requires_path_and_line_number() {
    let mut matches = Vec::new();
    let mut match_count = 0usize;
    let mut match_limit_reached = false;
    let scan_limit = 1;

    let missing_line = Ok(r#"{"type":"match","data":{"path":{"text":"file.txt"}}}"#.to_string());
    process_rg_json_match_line(
        missing_line,
        &mut matches,
        &mut match_count,
        &mut match_limit_reached,
        scan_limit,
    );
    assert!(matches.is_empty());
    assert_eq!(match_count, 0);
    assert!(!match_limit_reached);

    let valid_line =
        Ok(r#"{"type":"match","data":{"path":{"text":"file.txt"},"line_number":3}}"#.to_string());
    process_rg_json_match_line(
        valid_line,
        &mut matches,
        &mut match_count,
        &mut match_limit_reached,
        scan_limit,
    );
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].1, 3);
    assert_eq!(match_count, 1);
    assert!(match_limit_reached);
}

#[test]
fn test_truncate_by_bytes() {
    let content = "short\nthis is a longer line\nanother".to_string();
    let result = truncate_head(content, 100, 15);

    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncatedBy::Bytes));
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[test]
fn test_command_with_default_sigpipe_restores_pipe_disposition() {
    // Verify the spawned child does NOT inherit the parent's
    // SIGPIPE=SIG_IGN. The probe parses the SigIgn: hex mask exposed by
    // Linux-format /proc/<pid>/status — available natively on Linux and,
    // on FreeBSD, through the linprocfs compat module mounted at
    // /compat/linux/proc. Skip with a one-line notice when linprocfs is
    // not mounted rather than failing the test.
    #[cfg(target_os = "freebsd")]
    let status_dir = {
        let probe = format!("/compat/linux/proc/{}/status", std::process::id());
        if !std::path::Path::new(&probe).exists() {
            eprintln!(
                "skipping sigpipe disposition test: linprocfs not mounted \
                 at /compat/linux/proc — add `linprocfs /compat/linux/proc \
                 linprocfs rw 0 0` to /etc/fstab and `mount /compat/linux/proc` \
                 to enable"
            );
            return;
        }
        "/compat/linux/proc"
    };
    #[cfg(not(target_os = "freebsd"))]
    let status_dir = "/proc";

    let probe_cmd = format!(
        "while read name value _; do [ \"$name\" = SigIgn: ] && \
         {{ printf '%s' \"$value\"; exit 0; }}; done < {status_dir}/$$/status"
    );

    let output = command_with_default_sigpipe("sh")
        .expect("prepare sigpipe disposition probe")
        .args(["-c", &probe_cmd])
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn sigpipe disposition probe");

    assert!(output.status.success(), "probe failed: {output:?}");
    let sigign = String::from_utf8(output.stdout).expect("SigIgn should be utf8");
    let ignored_mask = u64::from_str_radix(sigign.trim(), 16).expect("SigIgn should be a hex mask");
    let sigpipe_bit = 1_u64 << (13 - 1);
    assert_eq!(
        ignored_mask & sigpipe_bit,
        0,
        "child should not inherit ignored SIGPIPE: SigIgn={sigign}"
    );
}

#[cfg(unix)]
#[test]
fn test_command_with_default_sigpipe_in_dir_resolves_relative_program_after_cwd() {
    use std::os::unix::fs::PermissionsExt as _;

    let tmp = tempfile::tempdir().expect("create temp dir");
    let script = tmp.path().join("relative-probe");
    std::fs::write(&script, "#!/bin/sh\nprintf cwd-relative-ok\n").expect("write script");
    let mut permissions = std::fs::metadata(&script)
        .expect("stat script")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script, permissions).expect("make script executable");

    let output = command_with_default_sigpipe_in_dir("./relative-probe", tmp.path())
        .expect("prepare relative executable")
        .current_dir(tmp.path())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn relative executable");

    assert!(output.status.success(), "probe failed: {output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("probe stdout should be utf8"),
        "cwd-relative-ok"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn test_read_to_end_capped_and_drain_preserves_writer_exit_status() {
    let mut child = std::process::Command::new("dd")
        .args(["if=/dev/zero", "bs=1", "count=70000", "status=none"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn dd");

    let stdout = child.stdout.take().expect("dd stdout");
    let captured = read_to_end_capped_and_drain(stdout, 1024).expect("capture bounded stdout");
    let status = child.wait().expect("wait for dd");

    assert!(
        status.success(),
        "bounded reader should drain to EOF instead of SIGPIPEing the writer: {status:?}"
    );
    assert_eq!(captured.len(), 1025);
}

#[cfg(unix)]
#[test]
fn test_get_file_lines_async_unreadable_file_returns_empty() {
    asupersync::test_utils::run_test(|| async {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("secret.txt");
        std::fs::write(&path, "secret\n").unwrap();

        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&path, perms).unwrap();

        let mut cache = HashMap::new();
        let lines = get_file_lines_async(&path, &mut cache).await;
        assert!(lines.is_empty());
    });
}

#[test]
fn test_resolve_path_absolute() {
    let cwd = PathBuf::from("/home/user/project");
    let result = resolve_path("/absolute/path", &cwd);
    assert_eq!(result, PathBuf::from("/absolute/path"));
}

#[test]
fn test_resolve_path_relative() {
    let cwd = PathBuf::from("/home/user/project");
    let result = resolve_path("src/main.rs", &cwd);
    assert_eq!(result, PathBuf::from("/home/user/project/src/main.rs"));
}

#[test]
fn test_normalize_dot_segments_preserves_root() {
    let result = normalize_dot_segments(std::path::Path::new("/../etc/passwd"));
    assert_eq!(result, PathBuf::from("/etc/passwd"));
}

#[test]
fn test_normalize_dot_segments_preserves_leading_parent_for_relative() {
    let result = normalize_dot_segments(std::path::Path::new("../a/../b"));
    assert_eq!(result, PathBuf::from("../b"));
}

#[test]
fn test_detect_supported_image_mime_type_from_bytes() {
    assert_eq!(
        detect_supported_image_mime_type_from_bytes(b"\x89PNG\r\n\x1A\n"),
        Some("image/png")
    );
    assert_eq!(
        detect_supported_image_mime_type_from_bytes(b"\xFF\xD8\xFF"),
        Some("image/jpeg")
    );
    assert_eq!(
        detect_supported_image_mime_type_from_bytes(b"GIF89a"),
        Some("image/gif")
    );
    assert_eq!(
        detect_supported_image_mime_type_from_bytes(b"RIFF1234WEBP"),
        Some("image/webp")
    );
    assert_eq!(
        detect_supported_image_mime_type_from_bytes(b"not an image"),
        None
    );
}

#[test]
fn test_format_size() {
    assert_eq!(format_size(500), "500B");
    assert_eq!(format_size(1024), "1.0KB");
    assert_eq!(format_size(1536), "1.5KB");
    assert_eq!(format_size(1_048_576), "1.0MB");
    assert_eq!(format_size(1_073_741_824), "1024.0MB");
}

#[test]
fn test_js_string_length() {
    assert_eq!(js_string_length("hello"), 5);
    assert_eq!(js_string_length("😀"), 2);
}

#[test]
fn test_truncate_line() {
    let short = "short line";
    let result = truncate_line(short, 100);
    assert_eq!(result.text, "short line");
    assert!(!result.was_truncated);

    let long = "a".repeat(600);
    let result = truncate_line(&long, 500);
    assert!(result.was_truncated);
    assert!(result.text.ends_with("... [truncated]"));
}

// ========================================================================
// Helper: extract text from ToolOutput content blocks
// ========================================================================

fn get_text(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| {
            if let ContentBlock::Text(text) = block {
                Some(text.text.clone())
            } else {
                None
            }
        })
        .collect::<String>()
}

// ========================================================================
// Read Tool Tests
// ========================================================================

#[test]
fn test_read_valid_file() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("hello.txt"), "alpha\nbeta\ngamma").unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("hello.txt").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("alpha"));
        assert!(text.contains("beta"));
        assert!(text.contains("gamma"));
        assert!(!out.is_error);
    });
}

#[test]
fn test_read_nonexistent_file() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = ReadTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("nope.txt").to_string_lossy() }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
    });
}

/// FIXME: ReadTool does not yet enforce scope (needs `enforce_cwd_scope` call in
/// `read_single_file`). This test is ignored until scope enforcement is added.
#[ignore = "flaky on CI"]
#[test]
fn test_read_rejects_outside_cwd() {
    asupersync::test_utils::run_test(|| async {
        let cwd = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();

        let tool = ReadTool::new(cwd.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({ "path": outside.path().join("secret.txt").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("outside the working directory"));
    });
}

/// Issue #71: skill files, prompt templates, and themes live under the
/// agent dir (`~/.pi/agent/`, default). The agent legitimately needs to
/// read these even when cwd is a user project on a different path.
/// Ensure `enforce_read_scope_with_roots` accepts the agent dir as a
/// second valid root without breaking the cwd-only contract for paths
/// that are under neither.
#[test]
fn test_enforce_read_scope_allows_agent_dir_outside_cwd() {
    let cwd = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    let skill_dir = agent_dir.path().join("skills").join("freebsd-jails");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(&skill_path, "---\nname: test\n---\n# body\n").unwrap();

    let resolved =
        enforce_read_scope_with_roots(&skill_path, cwd.path(), agent_dir.path()).unwrap();
    assert!(
        resolved.starts_with(safe_canonicalize(agent_dir.path())),
        "agent-dir path must be allowed and returned canonicalised"
    );
}

#[test]
fn test_enforce_read_scope_still_rejects_unrelated_paths() {
    // Paths under neither cwd nor agent_dir must keep failing closed.
    let cwd = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    let unrelated = tempfile::tempdir().unwrap();
    std::fs::write(unrelated.path().join("secret.txt"), "secret").unwrap();
    let secret_path = unrelated.path().join("secret.txt");

    let err =
        enforce_read_scope_with_roots(&secret_path, cwd.path(), agent_dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("outside the working directory") && msg.contains("agent dir"),
        "error must mention both denied roots, got: {msg}"
    );
}

#[test]
fn test_enforce_read_scope_prefers_cwd_when_path_is_under_cwd() {
    // When a path is under cwd, we must not silently switch to agent-dir
    // resolution. This locks in the order of the prefix checks.
    let cwd = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.txt"), "in cwd").unwrap();

    let resolved =
        enforce_read_scope_with_roots(&cwd.path().join("a.txt"), cwd.path(), agent_dir.path())
            .unwrap();
    assert!(resolved.starts_with(safe_canonicalize(cwd.path())));
}

#[test]
fn test_read_empty_file() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("empty.txt"), "").unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("empty.txt").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert_eq!(text, "");
        assert!(!out.is_error);
    });
}

#[test]
fn test_read_empty_file_positive_offset_errors() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("empty.txt"), "").unwrap();

        let tool = ReadTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("empty.txt").to_string_lossy(),
                    "offset": 1
                }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("beyond end of file"));
    });
}

#[test]
fn test_read_rejects_zero_limit() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("lines.txt"), "a\nb\nc\n").unwrap();

        let tool = ReadTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("lines.txt").to_string_lossy(),
                    "limit": 0
                }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("`limit` must be greater than 0")
        );
    });
}

#[test]
fn test_read_offset_and_limit() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("lines.txt"),
            "L1\nL2\nL3\nL4\nL5\nL6\nL7\nL8\nL9\nL10",
        )
        .unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("lines.txt").to_string_lossy(),
                    "offset": 3,
                    "limit": 2
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("L3"));
        assert!(text.contains("L4"));
        assert!(!text.contains("L2"));
        assert!(!text.contains("L5"));
    });
}

#[test]
fn test_read_offset_and_limit_with_cr_only_line_endings() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("lines.txt"), b"L1\rL2\rL3\r").unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("lines.txt").to_string_lossy(),
                    "offset": 2,
                    "limit": 1
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("L2"));
        assert!(!text.contains("L1"));
        assert!(!text.contains("L3"));
        assert!(text.contains("offset=3"));
        assert!(!text.contains('\r'));
    });
}

#[test]
fn test_read_offset_and_limit_with_split_crlf_chunk_boundary() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let mut content = vec![b'x'; (64 * 1024) - 1];
        content.extend_from_slice(b"\r\nSECOND\r\nTHIRD");
        std::fs::write(tmp.path().join("lines.txt"), content).unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("lines.txt").to_string_lossy(),
                    "offset": 2,
                    "limit": 1
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("SECOND"));
        assert!(!text.contains("THIRD"));
        assert!(!text.contains("xxxx"));
        assert!(text.contains("offset=3"));
    });
}

#[test]
fn test_read_offset_beyond_eof() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("short.txt"), "a\nb").unwrap();

        let tool = ReadTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("short.txt").to_string_lossy(),
                    "offset": 100
                }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("beyond end of file"));
    });
}

#[test]
fn test_map_normalized_with_trailing_whitespace() {
    // "A   \nB" -> "A\nB" (normalized strips trailing spaces)
    let content = "A   \nB";
    let normalized = build_normalized_content(content);
    assert_eq!(normalized, "A\nB");

    // Find "A" (norm idx 0)
    let (start, len) = map_normalized_range_to_original(content, 0, 1);
    assert_eq!(start, 0);
    assert_eq!(len, 1);
    assert_eq!(&content[start..start + len], "A");

    // Find "\n" (norm idx 1)
    let (start, len) = map_normalized_range_to_original(content, 1, 1);
    assert_eq!(start, 4);
    assert_eq!(len, 1);
    assert_eq!(&content[start..start + len], "\n");

    // Find "B" (norm idx 2)
    let (start, len) = map_normalized_range_to_original(content, 2, 1);
    assert_eq!(start, 5);
    assert_eq!(len, 1);
    assert_eq!(&content[start..start + len], "B");
}

#[test]
fn test_read_binary_file_lossy() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let binary_data: Vec<u8> = (0..=255).collect();
        std::fs::write(tmp.path().join("binary.bin"), &binary_data).unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("binary.bin").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        // Binary files are read as lossy UTF-8 with replacement characters
        let text = get_text(&out.content);
        assert!(!text.is_empty());
        assert!(!out.is_error);
    });
}

#[test]
fn test_read_image_detection() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        // Minimal valid PNG header
        let png_header: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 pixel
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE, // bit depth, color type, etc
            0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, // compressed data
            0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, // CRC
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(tmp.path().join("test.png"), &png_header).unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("test.png").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();

        // Should return an image content block
        let has_image = out
            .content
            .iter()
            .any(|b| matches!(b, ContentBlock::Image(_)));
        assert!(has_image, "expected image content block for PNG file");
    });
}

#[cfg(feature = "image-resize")]
#[test]
fn test_read_resizes_large_source_image_before_api_limit_check() {
    asupersync::test_utils::run_test(|| async {
        use image::codecs::png::PngEncoder;
        use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};

        let tmp = tempfile::tempdir().unwrap();
        let image = RgbImage::from_fn(2600, 2600, |x, y| {
            let seed = x.wrapping_mul(1_973)
                ^ y.wrapping_mul(9_277)
                ^ x.rotate_left(7)
                ^ y.rotate_left(13);
            Rgb([
                u8::try_from(seed % 256).unwrap_or(0),
                u8::try_from((seed >> 8) % 256).unwrap_or(0),
                u8::try_from((seed >> 16) % 256).unwrap_or(0),
            ])
        });

        let mut png_bytes = Vec::new();
        PngEncoder::new(&mut png_bytes)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .unwrap();

        assert!(
            png_bytes.len() > IMAGE_MAX_BYTES,
            "fixture must exceed API image limit to exercise resize path"
        );
        assert!(
            png_bytes.len() < usize::try_from(READ_TOOL_MAX_BYTES).unwrap_or(usize::MAX),
            "fixture must stay within read-tool input bound"
        );

        let image_path = tmp.path().join("large.png");
        std::fs::write(&image_path, &png_bytes).unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": image_path.to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(!out.is_error, "resizable large images should succeed");
        assert!(
            out.content
                .iter()
                .any(|block| matches!(block, ContentBlock::Image(_))),
            "expected an image attachment after resizing"
        );

        let text = get_text(&out.content);
        assert!(text.contains("Read image file"));
        assert!(
            text.contains("displayed at"),
            "expected resize note in read output, got: {text}"
        );
    });
}

#[test]
fn test_read_blocked_images() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let png_header: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        std::fs::write(tmp.path().join("test.png"), &png_header).unwrap();

        let tool = ReadTool::with_settings(tmp.path(), false, true);
        let err = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("test.png").to_string_lossy() }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("blocked"));
    });
}

#[test]
fn test_read_truncation_at_max_lines() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let content: String = (0..DEFAULT_MAX_LINES + 500)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(tmp.path().join("big.txt"), &content).unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("big.txt").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        // Should have truncation details
        assert!(out.details.is_some(), "expected truncation details");
        let text = get_text(&out.content);
        assert!(text.contains("offset="));
    });
}

#[test]
fn test_read_first_line_exceeds_max_bytes() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let long_line = "a".repeat(DEFAULT_MAX_BYTES + 128);
        std::fs::write(tmp.path().join("too_long.txt"), long_line).unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("too_long.txt").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();

        let text = get_text(&out.content);
        let expected_limit = format!("exceeds {} limit", format_size(DEFAULT_MAX_BYTES));
        assert!(
            text.contains(&expected_limit),
            "expected limit hint '{expected_limit}', got: {text}"
        );
        let details = out.details.expect("expected truncation details");
        assert_eq!(
            details
                .get("truncation")
                .and_then(|v| v.get("firstLineExceedsLimit"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    });
}

#[test]
fn test_read_unicode_content() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("uni.txt"), "Hello 你好 🌍\nLine 2 café").unwrap();

        let tool = ReadTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("uni.txt").to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("你好"));
        assert!(text.contains("🌍"));
        assert!(text.contains("café"));
    });
}

// ========================================================================
// Write Tool Tests
// ========================================================================

#[test]
fn test_write_new_file() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WriteTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("new.txt").to_string_lossy(),
                    "content": "hello world"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        let contents = std::fs::read_to_string(tmp.path().join("new.txt")).unwrap();
        assert_eq!(contents, "hello world");
    });
}

#[test]
fn test_write_overwrite_existing() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("exist.txt"), "old content").unwrap();

        let tool = WriteTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("exist.txt").to_string_lossy(),
                    "content": "new content"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        let contents = std::fs::read_to_string(tmp.path().join("exist.txt")).unwrap();
        assert_eq!(contents, "new content");
    });
}

#[test]
fn test_write_creates_parent_dirs() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WriteTool::new(tmp.path());
        let deep_path = tmp.path().join("a/b/c/deep.txt");
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": deep_path.to_string_lossy(),
                    "content": "deep file"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        assert!(deep_path.exists());
        assert_eq!(std::fs::read_to_string(&deep_path).unwrap(), "deep file");
    });
}

#[test]
fn test_write_empty_file() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WriteTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("empty.txt").to_string_lossy(),
                    "content": ""
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        let contents = std::fs::read_to_string(tmp.path().join("empty.txt")).unwrap();
        assert_eq!(contents, "");
        let text = get_text(&out.content);
        assert!(text.contains("Successfully wrote 0 bytes"));
    });
}

/// FIXME: WriteTool does not yet enforce scope (needs `enforce_cwd_scope` call).
/// This test is ignored until scope enforcement is added.
#[ignore = "flaky on CI"]
#[test]
fn test_write_rejects_outside_cwd() {
    asupersync::test_utils::run_test(|| async {
        let cwd = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let tool = WriteTool::new(cwd.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": outside.path().join("escape.txt").to_string_lossy(),
                    "content": "nope"
                }),
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("outside the working directory"));

        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": "../escape.txt",
                    "content": "nope"
                }),
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("outside the working directory"));
    });
}

#[test]
fn test_write_unicode_content() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WriteTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("unicode.txt").to_string_lossy(),
                    "content": "日本語 🎉 Ñoño"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        let contents = std::fs::read_to_string(tmp.path().join("unicode.txt")).unwrap();
        assert_eq!(contents, "日本語 🎉 Ñoño");
    });
}

#[test]
#[cfg(unix)]
fn test_write_file_permissions_unix() {
    use std::os::unix::fs::PermissionsExt;
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WriteTool::new(tmp.path());
        let path = tmp.path().join("perms.txt");
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "content": "check perms"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);

        let meta = std::fs::metadata(&path).unwrap();
        let mode = meta.permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o644,
            "Expected default 0o644 permissions for new files"
        );
    });
}

// ========================================================================
// Edit Tool Tests
// ========================================================================

#[test]
fn test_edit_exact_match_replace() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("code.rs"), "fn foo() { bar() }").unwrap();

        let tool = EditTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("code.rs").to_string_lossy(),
                    "oldText": "bar()",
                    "newText": "baz()"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        let contents = std::fs::read_to_string(tmp.path().join("code.rs")).unwrap();
        assert_eq!(contents, "fn foo() { baz() }");
    });
}

#[test]
fn test_edit_no_match_error() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("code.rs"), "fn foo() {}").unwrap();

        let tool = EditTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("code.rs").to_string_lossy(),
                    "oldText": "NONEXISTENT TEXT",
                    "newText": "replacement"
                }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
    });
}

#[test]
fn test_edit_empty_old_text_error() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("code.rs");
        std::fs::write(&path, "fn foo() {}").unwrap();

        let tool = EditTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "oldText": "",
                    "newText": "prefix"
                }),
                None,
                None,
            )
            .await
            .expect_err("empty oldText should be rejected");

        let msg = err.to_string();
        assert!(
            msg.contains("old text cannot be empty"),
            "unexpected error: {msg}"
        );
        let after = std::fs::read_to_string(path).unwrap();
        assert_eq!(after, "fn foo() {}");
    });
}

#[test]
fn test_edit_ambiguous_match_error() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("dup.txt"), "hello hello hello").unwrap();

        let tool = EditTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("dup.txt").to_string_lossy(),
                    "oldText": "hello",
                    "newText": "world"
                }),
                None,
                None,
            )
            .await;
        assert!(err.is_err(), "expected error for ambiguous match");
    });
}

#[test]
fn test_edit_multi_line_replacement() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("multi.txt"),
            "line 1\nline 2\nline 3\nline 4",
        )
        .unwrap();

        let tool = EditTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("multi.txt").to_string_lossy(),
                    "oldText": "line 2\nline 3",
                    "newText": "replaced 2\nreplaced 3\nextra line"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        let contents = std::fs::read_to_string(tmp.path().join("multi.txt")).unwrap();
        assert_eq!(
            contents,
            "line 1\nreplaced 2\nreplaced 3\nextra line\nline 4"
        );
    });
}

#[test]
fn test_edit_unicode_content() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("uni.txt"), "Héllo wörld 🌍").unwrap();

        let tool = EditTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("uni.txt").to_string_lossy(),
                    "oldText": "wörld 🌍",
                    "newText": "Welt 🌎"
                }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        let contents = std::fs::read_to_string(tmp.path().join("uni.txt")).unwrap();
        assert_eq!(contents, "Héllo Welt 🌎");
    });
}

#[test]
fn test_edit_missing_file() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = EditTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().join("nope.txt").to_string_lossy(),
                    "oldText": "foo",
                    "newText": "bar"
                }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
    });
}

// ========================================================================
// Bash Tool Tests
// ========================================================================

struct FailingReader {
    responses: std::collections::VecDeque<std::io::Result<Vec<u8>>>,
}

impl FailingReader {
    fn new(responses: impl IntoIterator<Item = std::io::Result<Vec<u8>>>) -> Self {
        Self {
            responses: responses.into_iter().collect(),
        }
    }
}

impl Read for FailingReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.responses.pop_front().unwrap_or_else(|| Ok(Vec::new())) {
            Ok(bytes) => {
                assert!(
                    bytes.len() <= buf.len(),
                    "test reader only supports single-chunk reads"
                );
                buf[..bytes.len()].copy_from_slice(&bytes);
                Ok(bytes.len())
            }
            Err(err) => Err(err),
        }
    }
}

#[test]
fn test_bash_simple_command() {
    asupersync::test_utils::run_test(|| async {
        if !bash_available() {
            eprintln!("skipping: bash not available on this system");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = BashTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "echo hello_from_bash" }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("hello_from_bash"));
        assert!(!out.is_error);
    });
}

#[test]
fn test_bash_exit_code_nonzero() {
    asupersync::test_utils::run_test(|| async {
        if !bash_available() {
            eprintln!("skipping: bash not available on this system");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = BashTool::new(tmp.path());
        let out = tool
            .execute("t", serde_json::json!({ "command": "exit 42" }), None, None)
            .await
            .expect("non-zero exit should return Ok with is_error=true");
        assert!(out.is_error, "non-zero exit must set is_error");
        let msg = get_text(&out.content);
        assert!(
            msg.contains("42"),
            "expected exit code 42 in output, got: {msg}"
        );
    });
}

#[cfg(unix)]
#[test]
fn test_bash_signal_termination_is_error() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = BashTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "kill -KILL $$" }),
                None,
                None,
            )
            .await
            .expect("signal-terminated shell should return Ok with is_error=true");
        assert!(
            out.is_error,
            "signal-terminated shell must be reported as error"
        );
        let msg = get_text(&out.content);
        assert!(
            msg.contains("Command exited with code"),
            "expected explicit exit-code report, got: {msg}"
        );
        assert!(
            !msg.contains("Command exited with code 0"),
            "signal-terminated shell must not appear successful: {msg}"
        );
    });
}

#[test]
fn test_bash_stderr_capture() {
    asupersync::test_utils::run_test(|| async {
        if !bash_available() {
            eprintln!("skipping: bash not available on this system");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = BashTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "echo stderr_msg >&2" }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.contains("stderr_msg"),
            "expected stderr output in result, got: {text}"
        );
    });
}

#[test]
fn test_bash_timeout() {
    asupersync::test_utils::run_test(|| async {
        if !bash_available() {
            eprintln!("skipping: bash not available on this system");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = BashTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "sleep 60", "timeout": 2 }),
                None,
                None,
            )
            .await
            .expect("timeout should return Ok with is_error=true");
        assert!(out.is_error, "timeout must set is_error");
        let msg = get_text(&out.content);
        assert!(
            msg.to_lowercase().contains("timeout") || msg.to_lowercase().contains("timed out"),
            "expected timeout indication, got: {msg}"
        );
        let cancellation = out
            .details
            .as_ref()
            .and_then(|details| details.get("cancellation"))
            .expect("timeout should include structured cancellation details");
        assert_eq!(cancellation["schema"], BASH_CANCELLATION_SCHEMA_V1);
        assert_eq!(cancellation["status"], "cancelled");
        assert_eq!(cancellation["reason"], "timeout");
        assert_eq!(cancellation["cleanup"], "process_group_tree_terminated");
        assert_eq!(cancellation["timeoutMs"], 2000);
    });
}

#[cfg(target_os = "linux")]
#[test]
fn test_bash_timeout_kills_process_tree() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let marker = tmp.path().join("leaked_child.txt");
        let tool = BashTool::new(tmp.path());

        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "command": "(sleep 3; echo leaked > leaked_child.txt) & sleep 10",
                    "timeout": 1
                }),
                None,
                None,
            )
            .await
            .expect("timeout should return Ok with is_error=true");

        assert!(out.is_error, "timeout must set is_error");
        let msg = get_text(&out.content);
        assert!(msg.contains("Command timed out"));

        // If process tree cleanup fails, this file appears after ~3 seconds.
        std::thread::sleep(Duration::from_secs(4));
        assert!(
            !marker.exists(),
            "background child was not terminated on timeout"
        );
    });
}

#[cfg(target_os = "linux")]
#[test]
fn test_bash_cancelled_context_kills_process_tree() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let marker = tmp.path().join("leaked_child.txt");

        let ambient_cx = asupersync::Cx::for_testing();
        let cancel_cx = ambient_cx.clone();
        let _current = asupersync::Cx::set_current(Some(ambient_cx));

        let cancel_thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            cancel_cx.set_cancel_requested(true);
        });

        let result = run_bash_command(
            tmp.path(),
            None,
            None,
            "(sleep 3; echo leaked > leaked_child.txt) & sleep 10",
            Some(30),
            None,
        )
        .await
        .expect("cancelled bash should return a result");

        cancel_thread.join().expect("cancel thread");

        assert!(
            result.cancelled,
            "expected cancelled bash result: {result:?}"
        );
        assert_eq!(
            result.cancellation_reason,
            Some(BashCancellationReason::AmbientCancellation)
        );

        std::thread::sleep(Duration::from_secs(4));
        assert!(
            !marker.exists(),
            "background child was not terminated on cancellation"
        );
    });
}

#[test]
fn test_bash_pump_stream_emits_io_error_frame_after_partial_output() {
    let reader = FailingReader::new([
        Ok(b"partial stdout".to_vec()),
        Err(std::io::Error::other("simulated stdout failure")),
    ]);
    let (tx, rx) = mpsc::sync_channel::<BashPipeFrame>(4);

    pump_stream(reader, "stdout", &tx);

    match rx.recv().expect("partial chunk") {
        BashPipeFrame::Chunk(chunk) => assert_eq!(chunk, b"partial stdout"),
        BashPipeFrame::Error(message) => {
            unreachable!("expected output chunk before error, got error frame: {message}")
        }
    }

    match rx.recv().expect("io error frame") {
        BashPipeFrame::Chunk(chunk) => {
            unreachable!("expected io error after partial chunk, got chunk: {chunk:?}")
        }
        BashPipeFrame::Error(message) => {
            assert!(message.contains("Failed to read bash stdout"));
            assert!(message.contains("simulated stdout failure"));
        }
    }

    assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
}

#[test]
fn test_drain_bash_output_ignores_cancellation_after_process_exit() {
    asupersync::test_utils::run_test(|| async {
        let (tx, mut rx) = mpsc::sync_channel::<BashPipeFrame>(1);
        let mut bash_output = BashOutputState::new(DEFAULT_MAX_BYTES);

        let ambient_cx = asupersync::Cx::for_testing();
        ambient_cx.set_cancel_requested(true);
        let _current = asupersync::Cx::set_current(Some(ambient_cx));
        let cx = AgentCx::for_current_or_request();
        let now = cx
            .cx()
            .timer_driver()
            .map_or_else(wall_now, |timer| timer.now());

        let cancelled = drain_bash_output(
            &mut rx,
            &mut bash_output,
            &cx,
            now + std::time::Duration::from_millis(10),
            std::time::Duration::from_millis(1),
            false,
        )
        .await
        .expect("drain should complete without cancellation");

        drop(tx);

        assert!(
            !cancelled,
            "post-exit drain should ignore late ambient cancellation"
        );
        assert_eq!(bash_output.total_bytes, 0);
    });
}

#[test]
fn test_drain_bash_output_returns_pipe_read_error() {
    asupersync::test_utils::run_test(|| async {
        let (tx, mut rx) = mpsc::sync_channel::<BashPipeFrame>(2);
        tx.send(BashPipeFrame::Chunk(b"partial stderr".to_vec()))
            .expect("queue partial output");
        tx.send(BashPipeFrame::Error(
            "Failed to read bash stderr: simulated stderr failure".to_string(),
        ))
        .expect("queue error frame");
        drop(tx);

        let mut bash_output = BashOutputState::new(DEFAULT_MAX_BYTES);
        let cx = AgentCx::for_current_or_request();
        let now = cx
            .cx()
            .timer_driver()
            .map_or_else(wall_now, |timer| timer.now());

        let err = drain_bash_output(
            &mut rx,
            &mut bash_output,
            &cx,
            now + std::time::Duration::from_millis(10),
            std::time::Duration::from_millis(1),
            false,
        )
        .await
        .expect_err("pipe read failures must surface as errors");

        let message = err.to_string();
        assert!(message.contains("Failed to read bash stderr"));
        assert!(message.contains("simulated stderr failure"));
        assert!(message.contains("Partial output before failure"));
        assert!(message.contains("partial stderr"));
        assert_eq!(bash_output.total_bytes, "partial stderr".len());
    });
}

#[test]
fn test_drain_bash_output_honors_cancellation_while_process_still_active() {
    asupersync::test_utils::run_test(|| async {
        let (_tx, mut rx) = mpsc::sync_channel::<BashPipeFrame>(1);
        let mut bash_output = BashOutputState::new(DEFAULT_MAX_BYTES);

        let ambient_cx = asupersync::Cx::for_testing();
        ambient_cx.set_cancel_requested(true);
        let _current = asupersync::Cx::set_current(Some(ambient_cx));
        let cx = AgentCx::for_current_or_request();
        let now = cx
            .cx()
            .timer_driver()
            .map_or_else(wall_now, |timer| timer.now());

        let cancelled = drain_bash_output(
            &mut rx,
            &mut bash_output,
            &cx,
            now + std::time::Duration::from_secs(1),
            std::time::Duration::from_millis(1),
            true,
        )
        .await
        .expect("drain should complete under cancellation");

        assert!(
            cancelled,
            "active drain should still honor ambient cancellation"
        );
        assert_eq!(bash_output.total_bytes, 0);
    });
}

#[test]
fn test_bash_output_state_abandon_spill_file_clears_path_and_unlinks_file() {
    let tmp = tempfile::tempdir().unwrap();
    let spill_path = tmp.path().join("partial-bash.log");
    std::fs::write(&spill_path, b"partial output").unwrap();

    let mut bash_output = BashOutputState::new(DEFAULT_MAX_BYTES);
    bash_output.temp_file_path = Some(spill_path.clone());

    bash_output.abandon_spill_file();

    assert!(bash_output.spill_failed);
    assert!(bash_output.temp_file.is_none());
    assert!(bash_output.temp_file_path.is_none());
    assert!(
        !spill_path.exists(),
        "abandoned spill files should not be advertised or left behind"
    );
}

#[test]
fn test_bash_hard_limit_retains_partial_spill_file() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let spill_path = tmp.path().join("hard-limit-bash.log");
        std::fs::write(&spill_path, b"partial output").unwrap();

        let spill_file = asupersync::fs::OpenOptions::new()
            .append(true)
            .open(&spill_path)
            .await
            .unwrap();

        let mut bash_output = BashOutputState::new(DEFAULT_MAX_BYTES);
        bash_output.total_bytes = BASH_FILE_LIMIT_BYTES;
        bash_output.temp_file_path = Some(spill_path.clone());
        bash_output.temp_file = Some(spill_file);

        ingest_bash_chunk(vec![b'x'], &mut bash_output)
            .await
            .expect("hard-limit ingestion should still succeed");

        assert!(!bash_output.spill_failed);
        assert!(bash_output.temp_file.is_none());
        assert!(bash_output.temp_file_path.is_some());
        assert!(
            spill_path.exists(),
            "partial spill files must be retained once the hard limit is reached for diagnostics"
        );
    });
}

#[test]
#[cfg(unix)]
fn test_bash_working_directory() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = BashTool::new(tmp.path());
        let out = tool
            .execute("t", serde_json::json!({ "command": "pwd" }), None, None)
            .await
            .unwrap();
        let text = get_text(&out.content);
        let canonical = tmp.path().canonicalize().unwrap();
        assert!(
            text.contains(&canonical.to_string_lossy().to_string()),
            "expected cwd in output, got: {text}"
        );
    });
}

#[test]
fn test_bash_multiline_output() {
    asupersync::test_utils::run_test(|| async {
        if !bash_available() {
            eprintln!("skipping: bash not available on this system");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = BashTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "echo line1; echo line2; echo line3" }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("line1"));
        assert!(text.contains("line2"));
        assert!(text.contains("line3"));
    });
}

// ========================================================================
// Grep Tool Tests
// ========================================================================

#[test]
fn test_grep_basic_pattern() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("search.txt"),
            "apple\nbanana\napricot\ncherry",
        )
        .unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "ap",
                    "path": tmp.path().join("search.txt").to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("apple"));
        assert!(text.contains("apricot"));
        assert!(!text.contains("banana"));
        assert!(!text.contains("cherry"));
    });
}

#[test]
fn test_grep_allows_outside_cwd() {
    asupersync::test_utils::run_test(|| async {
        let cwd = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();

        let tool = GrepTool::new(cwd.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "secret",
                    "path": outside.path().join("secret.txt").to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.contains("secret"),
            "grep should work outside cwd: {text}"
        );
    });
}

#[test]
fn test_grep_rejects_zero_limit() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("search.txt"), "alpha\nbeta\n").unwrap();

        let tool = GrepTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "alpha",
                    "path": tmp.path().join("search.txt").to_string_lossy(),
                    "limit": 0
                }),
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("`limit` must be greater than 0"));
    });
}

#[test]
#[cfg(unix)]
fn test_grep_formats_paths_relative_to_symlinked_cwd() {
    asupersync::test_utils::run_test(|| async {
        let real = tempfile::tempdir().unwrap();
        let link_parent = tempfile::tempdir().unwrap();
        let link = link_parent.path().join("linked-cwd");
        std::os::unix::fs::symlink(real.path(), &link).unwrap();
        std::fs::write(real.path().join("needle.txt"), "needle\n").unwrap();

        let tool = GrepTool::new(&link);
        let out = tool
            .execute("t", serde_json::json!({ "pattern": "needle" }), None, None)
            .await
            .unwrap();

        let text = get_text(&out.content);
        assert!(
            text.contains("needle.txt:1: needle"),
            "grep output should use cwd-relative paths for symlinked cwd, got: {text}"
        );
        assert!(
            !text.contains(real.path().to_string_lossy().as_ref()),
            "grep output should not leak canonical temp root, got: {text}"
        );
    });
}

#[test]
fn test_grep_regex_pattern() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("regex.txt"),
            "foo123\nbar456\nbaz789\nfoo000",
        )
        .unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "foo\\d+",
                    "path": tmp.path().join("regex.txt").to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("foo123"));
        assert!(text.contains("foo000"));
        assert!(!text.contains("bar456"));
    });
}

#[test]
fn test_grep_case_insensitive() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("case.txt"), "Hello\nhello\nHELLO").unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "hello",
                    "path": tmp.path().join("case.txt").to_string_lossy(),
                    "ignoreCase": true
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("Hello"));
        assert!(text.contains("hello"));
        assert!(text.contains("HELLO"));
    });
}

#[test]
fn test_grep_case_sensitive_by_default() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("case_sensitive.txt"), "Hello\nHELLO").unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "hello",
                    "path": tmp.path().join("case_sensitive.txt").to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.contains("No matches found"),
            "expected case-sensitive search to find no matches, got: {text}"
        );
    });
}

#[test]
fn test_grep_append_non_matching_lines_invariant() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("base.txt");
        std::fs::write(&file, "needle one\nskip\nneedle two\n").unwrap();

        let tool = GrepTool::new(tmp.path());
        let base_out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "needle",
                    "path": file.to_string_lossy(),
                    "limit": 100
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let base_text = get_text(&base_out.content);

        std::fs::write(&file, "needle one\nskip\nneedle two\nalpha\nbeta\n").unwrap();
        let extended_out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "needle",
                    "path": file.to_string_lossy(),
                    "limit": 100
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let extended_text = get_text(&extended_out.content);

        assert_eq!(
            base_text, extended_text,
            "adding non-matching lines should not alter grep output"
        );
    });
}

#[test]
fn test_grep_no_matches() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("nothing.txt"), "alpha\nbeta\ngamma").unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "ZZZZZ_NOMATCH",
                    "path": tmp.path().join("nothing.txt").to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.to_lowercase().contains("no match")
                || text.is_empty()
                || text.to_lowercase().contains("no results"),
            "expected no-match indication, got: {text}"
        );
    });
}

#[test]
fn test_grep_context_lines() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("ctx.txt"),
            "aaa\nbbb\nccc\ntarget\nddd\neee\nfff",
        )
        .unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "target",
                    "path": tmp.path().join("ctx.txt").to_string_lossy(),
                    "context": 1
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("target"));
        assert!(text.contains("ccc"), "expected context line before match");
        assert!(text.contains("ddd"), "expected context line after match");
    });
}

#[test]
fn test_grep_limit() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let content: String = (0..200)
            .map(|i| format!("match_line_{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(tmp.path().join("many.txt"), &content).unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "match_line",
                    "path": tmp.path().join("many.txt").to_string_lossy(),
                    "limit": 5
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        // With limit=5, we should see at most 5 matches
        let match_count = text.matches("match_line_").count();
        assert!(
            match_count <= 5,
            "expected at most 5 matches with limit=5, got {match_count}"
        );
        let details = out.details.expect("expected limit details");
        assert_eq!(
            details
                .get("matchLimitReached")
                .and_then(serde_json::Value::as_u64),
            Some(5)
        );
    });
}

#[test]
fn test_grep_exact_limit_does_not_report_limit_reached() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let content = (0..5)
            .map(|i| format!("match_line_{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(tmp.path().join("exact.txt"), &content).unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "match_line",
                    "path": tmp.path().join("exact.txt").to_string_lossy(),
                    "limit": 5
                }),
                None,
                None,
            )
            .await
            .unwrap();

        let text = get_text(&out.content);
        assert_eq!(text.matches("match_line_").count(), 5);
        assert!(
            !text.contains("matches limit reached"),
            "exact-limit grep results should not claim truncation: {text}"
        );
        assert!(
            out.details
                .as_ref()
                .and_then(|details| details.get("matchLimitReached"))
                .is_none(),
            "exact-limit grep results should not set matchLimitReached"
        );
    });
}

#[test]
fn test_grep_large_output_does_not_deadlock_reader_threads() {
    asupersync::test_utils::run_test(|| async {
        use std::fmt::Write as _;

        let tmp = tempfile::tempdir().unwrap();
        let mut content = String::with_capacity(80_000);
        for i in 0..5000 {
            let _ = writeln!(&mut content, "needle_line_{i}");
        }
        let file = tmp.path().join("large_grep.txt");
        std::fs::write(&file, content).unwrap();

        let tool = GrepTool::new(tmp.path());
        let run = tool.execute(
            "t",
            serde_json::json!({
                "pattern": "needle_line_",
                "path": file.to_string_lossy(),
                "limit": 6000
            }),
            None,
            None,
        );

        let out = asupersync::time::timeout(
            asupersync::time::wall_now(),
            Duration::from_secs(15),
            Box::pin(run),
        )
        .await
        .expect("grep timed out; possible stdout/stderr reader deadlock")
        .expect("grep should succeed");

        let text = get_text(&out.content);
        assert!(text.contains("needle_line_0"));
    });
}

#[test]
fn test_grep_respects_gitignore() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(tmp.path().join("ignored.txt"), "needle in ignored file").unwrap();
        std::fs::write(tmp.path().join("visible.txt"), "nothing here").unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute("t", serde_json::json!({ "pattern": "needle" }), None, None)
            .await
            .unwrap();

        let text = get_text(&out.content);
        assert!(
            text.contains("No matches found"),
            "expected ignored file to be excluded, got: {text}"
        );
    });
}

#[test]
fn test_grep_literal_mode() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("literal.txt"), "a+b\na.b\nab\na\\+b").unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "a+b",
                    "path": tmp.path().join("literal.txt").to_string_lossy(),
                    "literal": true
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("a+b"), "literal match should find 'a+b'");
    });
}

#[test]
fn test_grep_hashline_output() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("hash.txt"),
            "apple\nbanana\napricot\ncherry",
        )
        .unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "ap",
                    "path": tmp.path().join("hash.txt").to_string_lossy(),
                    "hashline": true
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        // Hashline output should contain N#AB tags instead of bare line numbers
        // Line 1 (apple) and line 3 (apricot) should match
        assert!(text.contains("apple"), "should contain apple");
        assert!(text.contains("apricot"), "should contain apricot");
        assert!(
            !text.contains("banana"),
            "should not contain banana context"
        );
        // Verify hashline tag format: digit(s) followed by # and two uppercase letters
        let re = regex::Regex::new(r"\d+#[A-Z]{2}").unwrap();
        assert!(
            re.is_match(&text),
            "hashline output should contain N#AB tags, got: {text}"
        );
    });
}

#[test]
fn test_grep_hashline_with_context() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("ctx.txt"),
            "line1\nline2\ntarget\nline4\nline5",
        )
        .unwrap();

        let tool = GrepTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "target",
                    "path": tmp.path().join("ctx.txt").to_string_lossy(),
                    "hashline": true,
                    "context": 1
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        // With context=1, should include line2, target, line4
        assert!(text.contains("line2"), "should contain context line2");
        assert!(text.contains("target"), "should contain match");
        assert!(text.contains("line4"), "should contain context line4");
        // Match lines use `:` separator, context lines use `-`
        let re_match = regex::Regex::new(r"\d+#[A-Z]{2}: target").unwrap();
        assert!(
            re_match.is_match(&text),
            "match line should use : separator with hashline tag, got: {text}"
        );
        let re_ctx = regex::Regex::new(r"\d+#[A-Z]{2}- line").unwrap();
        assert!(
            re_ctx.is_match(&text),
            "context line should use - separator with hashline tag, got: {text}"
        );
    });
}

// ========================================================================
// Find Tool Tests
// ========================================================================

#[test]
fn test_find_glob_pattern() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file1.rs"), "").unwrap();
        std::fs::write(tmp.path().join("file2.rs"), "").unwrap();
        std::fs::write(tmp.path().join("file3.txt"), "").unwrap();

        let tool = FindTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.rs",
                    "path": tmp.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("file1.rs"));
        assert!(text.contains("file2.rs"));
        assert!(!text.contains("file3.txt"));
    });
}

#[test]
fn test_find_append_non_matching_file_invariant() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("match.txt"), "a").unwrap();

        let tool = FindTool::new(tmp.path());
        let base_out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": tmp.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let base_text = get_text(&base_out.content);

        std::fs::write(tmp.path().join("ignore.md"), "b").unwrap();
        let extended_out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": tmp.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let extended_text = get_text(&extended_out.content);

        assert_eq!(
            base_text, extended_text,
            "adding non-matching files should not alter find output"
        );
    });
}

#[test]
fn test_find_allows_outside_cwd() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let cwd = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();

        let tool = FindTool::new(cwd.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": outside.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let content = get_text(&out.content);
        assert!(
            content.contains("secret.txt"),
            "find should work outside cwd: {content}"
        );
    });
}

#[test]
fn test_find_limit() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..20 {
            std::fs::write(tmp.path().join(format!("f{i}.txt")), "").unwrap();
        }

        let tool = FindTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": tmp.path().to_string_lossy(),
                    "limit": 5
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        let file_count = text.lines().filter(|l| l.contains(".txt")).count();
        assert!(
            file_count <= 5,
            "expected at most 5 files with limit=5, got {file_count}"
        );
        let details = out.details.expect("expected limit details");
        assert_eq!(
            details
                .get("resultLimitReached")
                .and_then(serde_json::Value::as_u64),
            Some(5)
        );
    });
}

#[test]
fn test_find_exact_limit_does_not_report_limit_reached() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std::fs::write(tmp.path().join(format!("f{i}.txt")), "").unwrap();
        }

        let tool = FindTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": tmp.path().to_string_lossy(),
                    "limit": 5
                }),
                None,
                None,
            )
            .await
            .unwrap();

        let text = get_text(&out.content);
        assert_eq!(text.lines().filter(|line| line.contains(".txt")).count(), 5);
        assert!(
            !text.contains("results limit reached"),
            "exact-limit find results should not claim truncation: {text}"
        );
        assert!(
            out.details
                .as_ref()
                .and_then(|details| details.get("resultLimitReached"))
                .is_none(),
            "exact-limit find results should not set resultLimitReached"
        );
    });
}

#[test]
fn test_find_zero_limit_is_rejected() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "").unwrap();

        let tool = FindTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": tmp.path().to_string_lossy(),
                    "limit": 0
                }),
                None,
                None,
            )
            .await
            .expect_err("limit=0 should be rejected");

        assert!(
            err.to_string().contains("`limit` must be greater than 0"),
            "expected validation error, got: {err}"
        );
    });
}

#[test]
fn test_find_no_matches() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("only.txt"), "").unwrap();

        let tool = FindTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.rs",
                    "path": tmp.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.to_lowercase().contains("no files found")
                || text.to_lowercase().contains("no matches")
                || text.is_empty(),
            "expected no-match indication, got: {text}"
        );
    });
}

#[test]
fn test_find_nonexistent_path() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = FindTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.rs",
                    "path": tmp.path().join("nonexistent").to_string_lossy()
                }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
    });
}

#[test]
fn test_find_nested_directories() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        std::fs::write(tmp.path().join("top.rs"), "").unwrap();
        std::fs::write(tmp.path().join("a/mid.rs"), "").unwrap();
        std::fs::write(tmp.path().join("a/b/c/deep.rs"), "").unwrap();

        let tool = FindTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.rs",
                    "path": tmp.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("top.rs"));
        assert!(text.contains("mid.rs"));
        assert!(text.contains("deep.rs"));
    });
}

#[test]
fn test_find_results_are_sorted() {
    // FindTool sorts by modification time (most recent first), then alphabetically
    // as a tie-breaker for files with the same mtime.
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();

        // Create files with delays to ensure distinct modification times.
        // Order: oldest first, so the expected output (most recent first) is reversed.
        std::fs::write(tmp.path().join("oldest.txt"), "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(tmp.path().join("middle.txt"), "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(tmp.path().join("newest.txt"), "").unwrap();

        let tool = FindTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": tmp.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let lines: Vec<String> = get_text(&out.content)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();

        // Expected order: most recent first
        assert_eq!(
            lines,
            vec!["newest.txt", "middle.txt", "oldest.txt"],
            "expected mtime-sorted find output (most recent first)"
        );
    });
}

#[test]
fn test_find_respects_gitignore() {
    asupersync::test_utils::run_test(|| async {
        if find_fd_binary().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(tmp.path().join("ignored.txt"), "").unwrap();

        let tool = FindTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "pattern": "*.txt",
                    "path": tmp.path().to_string_lossy()
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.contains("No files found matching pattern"),
            "expected .gitignore'd files to be excluded, got: {text}"
        );
    });
}

// ========================================================================
// Ls Tool Tests
// ========================================================================

#[test]
fn test_ls_directory_listing() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file_a.txt"), "content").unwrap();
        std::fs::write(tmp.path().join("file_b.rs"), "fn main() {}").unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();

        let tool = LsTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("file_a.txt"));
        assert!(text.contains("file_b.rs"));
        assert!(text.contains("subdir"));
    });
}

#[test]
fn test_ls_allows_outside_cwd() {
    asupersync::test_utils::run_test(|| async {
        let cwd = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();

        let tool = LsTool::new(cwd.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": outside.path().to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.contains("secret"),
            "ls should work outside cwd: {text}"
        );
    });
}

#[test]
fn test_ls_trailing_slash_for_dirs() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "").unwrap();
        std::fs::create_dir(tmp.path().join("mydir")).unwrap();

        let tool = LsTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.contains("mydir/"),
            "expected trailing slash for directory, got: {text}"
        );
    });
}

#[test]
fn test_ls_limit() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..20 {
            std::fs::write(tmp.path().join(format!("item_{i:02}.txt")), "").unwrap();
        }

        let tool = LsTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().to_string_lossy(),
                    "limit": 5
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        let entry_count = text.lines().filter(|l| l.contains("item_")).count();
        assert!(
            entry_count <= 5,
            "expected at most 5 entries, got {entry_count}"
        );
        let details = out.details.expect("expected limit details");
        assert_eq!(
            details
                .get("entryLimitReached")
                .and_then(serde_json::Value::as_u64),
            Some(5)
        );
    });
}

#[test]
fn test_ls_zero_limit_is_rejected() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("item.txt"), "").unwrap();

        let tool = LsTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": tmp.path().to_string_lossy(),
                    "limit": 0
                }),
                None,
                None,
            )
            .await
            .expect_err("limit=0 should be rejected");

        assert!(
            err.to_string().contains("`limit` must be greater than 0"),
            "expected validation error, got: {err}"
        );
    });
}

#[test]
fn test_ls_nonexistent_directory() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = LsTool::new(tmp.path());
        let err = tool
            .execute(
                "t",
                serde_json::json!({ "path": tmp.path().join("nope").to_string_lossy() }),
                None,
                None,
            )
            .await;
        assert!(err.is_err());
    });
}

#[test]
fn test_ls_empty_directory() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let empty_dir = tmp.path().join("empty");
        std::fs::create_dir(&empty_dir).unwrap();

        let tool = LsTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "path": empty_dir.to_string_lossy() }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!out.is_error);
    });
}

#[test]
fn test_ls_default_cwd() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("in_cwd.txt"), "").unwrap();

        let tool = LsTool::new(tmp.path());
        let out = tool
            .execute("t", serde_json::json!({}), None, None)
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(
            text.contains("in_cwd.txt"),
            "expected cwd listing to include the file, got: {text}"
        );
    });
}

// ========================================================================
// Additional helper tests
// ========================================================================

#[test]
fn test_truncate_head_no_truncation() {
    let content = "short".to_string();
    let result = truncate_head(content, 100, 1000);
    assert!(!result.truncated);
    assert_eq!(result.content, "short");
    assert_eq!(result.truncated_by, None);
}

#[test]
fn test_truncate_tail_no_truncation() {
    let content = "short".to_string();
    let result = truncate_tail(content, 100, 1000);
    assert!(!result.truncated);
    assert_eq!(result.content, "short");
}

#[test]
fn test_truncate_head_empty_input() {
    let result = truncate_head(String::new(), 100, 1000);
    assert!(!result.truncated);
    assert_eq!(result.content, "");
}

#[test]
fn test_truncate_tail_empty_input() {
    let result = truncate_tail(String::new(), 100, 1000);
    assert!(!result.truncated);
    assert_eq!(result.content, "");
}

#[test]
fn test_detect_line_ending_crlf() {
    assert_eq!(detect_line_ending("hello\r\nworld"), "\r\n");
}

#[test]
fn test_detect_line_ending_cr() {
    assert_eq!(detect_line_ending("hello\rworld"), "\r");
}

#[test]
fn test_detect_line_ending_lf() {
    assert_eq!(detect_line_ending("hello\nworld"), "\n");
}

#[test]
fn test_detect_line_ending_no_newline() {
    assert_eq!(detect_line_ending("hello world"), "\n");
}

#[test]
fn test_normalize_to_lf() {
    assert_eq!(normalize_to_lf("a\r\nb\rc\nd"), "a\nb\nc\nd");
}

#[test]
fn test_count_overlapping_occurrences() {
    assert_eq!(count_overlapping_occurrences("aaaa", "aa"), 3);
    assert_eq!(count_overlapping_occurrences("abababa", "aba"), 3);
    assert_eq!(count_overlapping_occurrences("abc", "d"), 0);
    assert_eq!(count_overlapping_occurrences("abc", ""), 0);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, .. ProptestConfig::default() })]

    #[test]
    fn proptest_line_ending_roundtrip_invariant(
        input in arbitrary_text(),
        ending in prop_oneof![
            Just("\n".to_string()),
            Just("\r\n".to_string()),
            Just("\r".to_string()),
        ],
    ) {
        let normalized = normalize_to_lf(&input);
        let restored = restore_line_endings(&normalized, &ending);
        let renormalized = normalize_to_lf(&restored);
        prop_assert_eq!(renormalized, normalized);
    }
}

#[test]
fn test_strip_bom_present() {
    let (result, had_bom) = strip_bom("\u{FEFF}hello");
    assert_eq!(result, "hello");
    assert!(had_bom);
}

#[test]
fn test_strip_bom_absent() {
    let (result, had_bom) = strip_bom("hello");
    assert_eq!(result, "hello");
    assert!(!had_bom);
}

#[test]
fn test_resolve_path_tilde_expansion() {
    let cwd = PathBuf::from("/home/user/project");
    let result = resolve_path("~/file.txt", &cwd);
    // Tilde expansion depends on environment, but should not be literal ~/
    assert!(!result.to_string_lossy().starts_with("~/"));
}

fn arbitrary_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 0..512)
        .prop_map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn match_char_strategy() -> impl Strategy<Value = char> {
    prop_oneof![
        8 => any::<char>(),
        1 => Just('\u{00A0}'),
        1 => Just('\u{202F}'),
        1 => Just('\u{205F}'),
        1 => Just('\u{3000}'),
        1 => Just('\u{2018}'),
        1 => Just('\u{2019}'),
        1 => Just('\u{201C}'),
        1 => Just('\u{201D}'),
        1 => Just('\u{201E}'),
        1 => Just('\u{201F}'),
        1 => Just('\u{2010}'),
        1 => Just('\u{2011}'),
        1 => Just('\u{2012}'),
        1 => Just('\u{2013}'),
        1 => Just('\u{2014}'),
        1 => Just('\u{2015}'),
        1 => Just('\u{2212}'),
        1 => Just('\u{200D}'),
        1 => Just('\u{0301}'),
    ]
}

fn arbitrary_match_text() -> impl Strategy<Value = String> {
    prop_oneof![
        9 => prop::collection::vec(match_char_strategy(), 0..2048),
        1 => prop::collection::vec(match_char_strategy(), 8192..16384),
    ]
    .prop_map(|chars| chars.into_iter().collect())
}

fn line_char_strategy() -> impl Strategy<Value = char> {
    prop_oneof![
        8 => any::<char>().prop_filter("single-line chars only", |c| *c != '\n'),
        1 => Just('é'),
        1 => Just('你'),
        1 => Just('😀'),
    ]
}

fn boundary_line_text() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(0usize),
        Just(GREP_MAX_LINE_LENGTH.saturating_sub(1)),
        Just(GREP_MAX_LINE_LENGTH),
        Just(GREP_MAX_LINE_LENGTH + 1),
        0usize..(GREP_MAX_LINE_LENGTH + 128),
    ]
    .prop_flat_map(|len| {
        prop::collection::vec(line_char_strategy(), len)
            .prop_map(|chars| chars.into_iter().collect())
    })
}

fn safe_relative_segment() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::string::string_regex("[A-Za-z0-9._-]{1,12}")
            .expect("segment regex should compile"),
        Just("emoji😀".to_string()),
        Just("accent-é".to_string()),
        Just("rtl-עברית".to_string()),
        Just("line\nbreak".to_string()),
        Just("nul\0byte".to_string()),
    ]
    .prop_filter("segment cannot be . or ..", |segment| {
        segment != "." && segment != ".."
    })
}

fn safe_relative_path() -> impl Strategy<Value = String> {
    prop::collection::vec(safe_relative_segment(), 1..6).prop_map(|segments| segments.join("/"))
}

fn pathish_input() -> impl Strategy<Value = String> {
    prop_oneof![
        5 => safe_relative_path(),
        2 => safe_relative_path().prop_map(|p| format!("../{p}")),
        2 => safe_relative_path().prop_map(|p| format!("../../{p}")),
        1 => safe_relative_path().prop_map(|p| format!("/tmp/{p}")),
        1 => safe_relative_path().prop_map(|p| format!("~/{p}")),
        1 => Just("~".to_string()),
        1 => Just(".".to_string()),
        1 => Just("..".to_string()),
        1 => Just("././nested/../file.txt".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, .. ProptestConfig::default() })]

    #[test]
    fn proptest_truncate_head_invariants(
        input in arbitrary_text(),
        max_lines in 0usize..32,
        max_bytes in 0usize..256,
    ) {
        let result = truncate_head(input.clone(), max_lines, max_bytes);

        prop_assert!(result.output_lines <= max_lines);
        prop_assert!(result.output_bytes <= max_bytes);
        prop_assert_eq!(result.output_bytes, result.content.len());

        prop_assert_eq!(result.truncated, result.truncated_by.is_some());
        prop_assert!(input.starts_with(&result.content));

        let repeat = truncate_head(result.content.clone(), max_lines, max_bytes);
        prop_assert_eq!(&repeat.content, &result.content);

        if result.truncated {
            prop_assert!(result.total_lines > max_lines || result.total_bytes > max_bytes);
        } else {
            prop_assert_eq!(&result.content, &input);
            prop_assert!(result.total_lines <= max_lines);
            prop_assert!(result.total_bytes <= max_bytes);
        }

        if result.first_line_exceeds_limit {
            prop_assert!(result.truncated);
            prop_assert_eq!(result.truncated_by, Some(TruncatedBy::Bytes));
            prop_assert!(result.output_bytes <= max_bytes);
            prop_assert!(result.output_lines <= 1);
            prop_assert!(input.starts_with(&result.content));
        }
    }

    #[test]
    fn proptest_truncate_tail_invariants(
        input in arbitrary_text(),
        max_lines in 0usize..32,
        max_bytes in 0usize..256,
    ) {
        let result = truncate_tail(input.clone(), max_lines, max_bytes);

        prop_assert!(result.output_lines <= max_lines);
        prop_assert!(result.output_bytes <= max_bytes);
        prop_assert_eq!(result.output_bytes, result.content.len());

        prop_assert_eq!(result.truncated, result.truncated_by.is_some());
        prop_assert!(input.ends_with(&result.content));

        let repeat = truncate_tail(result.content.clone(), max_lines, max_bytes);
        prop_assert_eq!(&repeat.content, &result.content);

        if result.last_line_partial {
            prop_assert!(result.truncated);
            prop_assert_eq!(result.truncated_by, Some(TruncatedBy::Bytes));
            // Partial output may span 1-2 lines when the input has a
            // trailing newline (the empty line after \n is preserved).
            prop_assert!(result.output_lines >= 1 && result.output_lines <= 2);
            let content_trimmed = result.content.trim_end_matches('\n');
            prop_assert!(input
                .split('\n')
                .rev()
                .any(|line| line.ends_with(content_trimmed)));
        }
    }

    #[test]
    fn proptest_truncate_head_monotonic_limits(
        input in arbitrary_text(),
        max_lines_a in 0usize..32,
        max_lines_b in 0usize..32,
        max_bytes_a in 0usize..256,
        max_bytes_b in 0usize..256,
    ) {
        let low_lines = max_lines_a.min(max_lines_b);
        let high_lines = max_lines_a.max(max_lines_b);
        let low_bytes = max_bytes_a.min(max_bytes_b);
        let high_bytes = max_bytes_a.max(max_bytes_b);

        let small = truncate_head(input.clone(), low_lines, low_bytes);
        let large = truncate_head(input, high_lines, high_bytes);

        prop_assert!(large.content.starts_with(&small.content));
        prop_assert!(large.output_bytes >= small.output_bytes);
        prop_assert!(large.output_lines >= small.output_lines);
    }

    #[test]
    fn proptest_truncate_tail_monotonic_limits(
        input in arbitrary_text(),
        max_lines_a in 0usize..32,
        max_lines_b in 0usize..32,
        max_bytes_a in 0usize..256,
        max_bytes_b in 0usize..256,
    ) {
        let low_lines = max_lines_a.min(max_lines_b);
        let high_lines = max_lines_a.max(max_lines_b);
        let low_bytes = max_bytes_a.min(max_bytes_b);
        let high_bytes = max_bytes_a.max(max_bytes_b);

        let small = truncate_tail(input.clone(), low_lines, low_bytes);
        let large = truncate_tail(input, high_lines, high_bytes);

        prop_assert!(large.content.ends_with(&small.content));
        prop_assert!(large.output_bytes >= small.output_bytes);
        prop_assert!(large.output_lines >= small.output_lines);
    }

    #[test]
    fn proptest_truncate_head_prefix_invariant_under_append(
        base in arbitrary_text(),
        suffix in arbitrary_text(),
        max_lines in 0usize..32,
        max_bytes in 0usize..256,
    ) {
        let base_result = truncate_head(base.clone(), max_lines, max_bytes);
        let extended_result = truncate_head(format!("{base}{suffix}"), max_lines, max_bytes);
        prop_assert!(extended_result.content.starts_with(&base_result.content));
    }

    #[test]
    fn proptest_truncate_tail_suffix_invariant_under_prepend(
        base in arbitrary_text(),
        prefix in arbitrary_text(),
        max_lines in 0usize..32,
        max_bytes in 0usize..256,
    ) {
        let base_result = truncate_tail(base.clone(), max_lines, max_bytes);
        let extended_result = truncate_tail(format!("{prefix}{base}"), max_lines, max_bytes);
        prop_assert!(extended_result.content.ends_with(&base_result.content));
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn proptest_normalize_for_match_invariants(input in arbitrary_match_text()) {
        let normalized = normalize_for_match(&input);
        let renormalized = normalize_for_match(&normalized);

        prop_assert_eq!(&renormalized, &normalized);
        prop_assert!(normalized.len() <= input.len());
        prop_assert!(
            normalized.chars().all(|c| {
                !is_special_unicode_space(c)
                    && !matches!(
                        c,
                        '\u{2018}'
                            | '\u{2019}'
                            | '\u{201C}'
                            | '\u{201D}'
                            | '\u{201E}'
                            | '\u{201F}'
                            | '\u{2010}'
                            | '\u{2011}'
                            | '\u{2012}'
                            | '\u{2013}'
                            | '\u{2014}'
                            | '\u{2015}'
                            | '\u{2212}'
                    )
            }),
            "normalize_for_match should remove target punctuation/space variants"
        );
    }

    #[test]
    fn proptest_truncate_line_boundary_invariants(line in boundary_line_text()) {
        const TRUNCATION_SUFFIX: &str = "... [truncated]";

        let result = truncate_line(&line, GREP_MAX_LINE_LENGTH);
        let line_char_count = line.chars().count();
        let suffix_chars = TRUNCATION_SUFFIX.chars().count();

        if line_char_count <= GREP_MAX_LINE_LENGTH {
            prop_assert!(!result.was_truncated);
            prop_assert_eq!(result.text, line);
        } else {
            prop_assert!(result.was_truncated);
            prop_assert!(result.text.ends_with(TRUNCATION_SUFFIX));
            let expected_prefix: String = line.chars().take(GREP_MAX_LINE_LENGTH).collect();
            let expected = format!("{expected_prefix}{TRUNCATION_SUFFIX}");
            prop_assert_eq!(&result.text, &expected);
            prop_assert!(result.text.chars().count() <= GREP_MAX_LINE_LENGTH + suffix_chars);
        }
    }

    #[test]
    fn proptest_resolve_path_safe_relative_invariants(relative_path in safe_relative_path()) {
        let cwd = PathBuf::from("/tmp/pi-agent-rust-tools-proptest");
        let resolved = resolve_path(&relative_path, &cwd);
        let normalized = normalize_dot_segments(&resolved);

        prop_assert_eq!(&resolved, &cwd.join(&relative_path));
        prop_assert!(resolved.starts_with(&cwd));
        prop_assert!(normalized.starts_with(&cwd));
        prop_assert_eq!(normalize_dot_segments(&normalized), normalized);
    }

    #[test]
    fn proptest_normalize_dot_segments_pathish_invariants(path_input in pathish_input()) {
        let cwd = PathBuf::from("/tmp/pi-agent-rust-tools-proptest");
        let resolved = resolve_path(&path_input, &cwd);
        let normalized_once = normalize_dot_segments(&resolved);
        let normalized_twice = normalize_dot_segments(&normalized_once);

        prop_assert_eq!(&normalized_once, &normalized_twice);
        prop_assert!(
            normalized_once
                .components()
                .all(|component| !matches!(component, std::path::Component::CurDir))
        );

        if std::path::Path::new(&path_input).is_absolute() {
            prop_assert!(resolved.is_absolute());
            prop_assert!(normalized_once.is_absolute());
        }
    }
}

// ========================================================================
// Fuzzy find / edit-matching strategies
// ========================================================================

/// Strategy generating content text with occasional Unicode normalization
/// targets (curly quotes, special spaces, em-dashes) and trailing
/// whitespace.
fn fuzzy_content_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            8 => any::<char>().prop_filter("no nul", |c| *c != '\0'),
            1 => Just('\u{00A0}'),
            1 => Just('\u{2019}'),
            1 => Just('\u{201C}'),
            1 => Just('\u{2014}'),
        ],
        1..512,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

/// Strategy for generating a needle substring from content. Picks a
/// random sub-slice of the content (may be empty).
fn needle_from_content(content: String) -> impl Strategy<Value = (String, String)> {
    let len = content.len();
    if len == 0 {
        return Just((content, String::new())).boxed();
    }
    (0..len)
        .prop_flat_map(move |start| {
            let c = content.clone();
            let remaining = c.len() - start;
            let max_needle = remaining.min(256);
            (Just(c), start..=start + max_needle.saturating_sub(1))
        })
        .prop_filter_map("valid char boundary", |(c, end)| {
            // Find the nearest valid char boundaries
            let start_candidates: Vec<usize> =
                (0..c.len()).filter(|i| c.is_char_boundary(*i)).collect();
            if start_candidates.is_empty() {
                return None;
            }
            let start = *start_candidates
                .iter()
                .min_by_key(|&&i| i.abs_diff(end.saturating_sub(end / 2)))
                .unwrap_or(&0);
            let end_clamped = end.min(c.len());
            // Find next valid char boundary >= end_clamped
            let actual_end = (end_clamped..=c.len())
                .find(|i| c.is_char_boundary(*i))
                .unwrap_or(c.len());
            if start >= actual_end {
                return Some((c, String::new()));
            }
            Some((c.clone(), c[start..actual_end].to_string()))
        })
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    /// Exact substrings of content are always found by `fuzzy_find_text`.
    #[test]
    fn proptest_fuzzy_find_text_exact_match_invariants(
        (content, needle) in fuzzy_content_strategy().prop_flat_map(needle_from_content)
    ) {
        let result = fuzzy_find_text(&content, &needle);
        if needle.is_empty() {
            // Empty needle: exact match at index 0 (str::find("") == Some(0))
            prop_assert!(result.found, "empty needle should always match");
            prop_assert_eq!(result.index, 0);
            prop_assert_eq!(result.match_length, 0);
        } else {
            prop_assert!(
                result.found,
                "exact substring must be found: content len={}, needle len={}",
                content.len(),
                needle.len()
            );
            // The matched span should be valid UTF-8 byte indices
            prop_assert!(content.is_char_boundary(result.index));
            prop_assert!(content.is_char_boundary(result.index + result.match_length));
            // The matched text should contain the needle (exact match path)
            let matched = &content[result.index..result.index + result.match_length];
            prop_assert_eq!(matched, needle.as_str());
        }
    }

    /// Normalized text with Unicode variants is found via fuzzy matching.
    /// If we take content containing curly quotes / em-dashes, normalize
    /// it, then search for the normalized version, `fuzzy_find_text` must
    /// locate it.
    #[test]
    fn proptest_fuzzy_find_text_normalized_match_invariants(
        content in arbitrary_match_text()
    ) {
        // Normalize the whole content to get an ASCII-equivalent version
        let normalized = build_normalized_content(&content);
        if normalized.is_empty() {
            return Ok(());
        }
        // Take a prefix of normalized as needle (up to 128 chars)
        let needle_end = normalized
            .char_indices()
            .nth(128.min(normalized.chars().count().saturating_sub(1)))
            .map_or(normalized.len(), |(i, _)| i);
        // Find the nearest char boundary
        let needle_end = (needle_end..=normalized.len())
            .find(|i| normalized.is_char_boundary(*i))
            .unwrap_or(normalized.len());
        let needle = &normalized[..needle_end];
        if needle.is_empty() {
            return Ok(());
        }

        let result = fuzzy_find_text(&content, needle);
        prop_assert!(
            result.found,
            "normalized needle should be found via fuzzy match: needle={:?}",
            needle
        );
        // Verify the result points to valid UTF-8
        prop_assert!(content.is_char_boundary(result.index));
        prop_assert!(content.is_char_boundary(result.index + result.match_length));
    }

    /// `build_normalized_content` should be idempotent and never larger
    /// than the input.
    #[test]
    fn proptest_build_normalized_content_invariants(input in arbitrary_match_text()) {
        let normalized = build_normalized_content(&input);
        let renormalized = build_normalized_content(&normalized);

        // Idempotency
        prop_assert_eq!(
            &renormalized,
            &normalized,
            "build_normalized_content should be idempotent"
        );

        // Size: normalized text strips trailing whitespace per line and
        // may replace multi-byte Unicode with single-byte ASCII, so it
        // should never be larger than the input.
        prop_assert!(
            normalized.len() <= input.len(),
            "normalized should not be larger: {} vs {}",
            normalized.len(),
            input.len()
        );

        // Line count should be preserved (normalization does not add or
        // remove newlines).
        let input_lines = input.split('\n').count();
        let norm_lines = normalized.split('\n').count();
        prop_assert_eq!(
            norm_lines, input_lines,
            "line count must be preserved by normalization"
        );

        // No target Unicode chars should remain
        prop_assert!(
            normalized.chars().all(|c| {
                !is_special_unicode_space(c)
                    && !matches!(
                        c,
                        '\u{2018}'
                            | '\u{2019}'
                            | '\u{201C}'
                            | '\u{201D}'
                            | '\u{201E}'
                            | '\u{201F}'
                            | '\u{2010}'
                            | '\u{2011}'
                            | '\u{2012}'
                            | '\u{2013}'
                            | '\u{2014}'
                            | '\u{2015}'
                            | '\u{2212}'
                    )
            }),
            "normalized content should not contain target Unicode chars"
        );
    }

    /// Appending trailing whitespace to each line should not change the
    /// normalized content (metamorphic invariant).
    #[test]
    fn proptest_build_normalized_content_trailing_whitespace_invariant(
        input in arbitrary_match_text()
    ) {
        let normalized = build_normalized_content(&input);
        let mut with_trailing = String::new();
        let mut lines = input.split('\n').peekable();

        while let Some(line) = lines.next() {
            with_trailing.push_str(line);
            with_trailing.push_str("  \t");
            if lines.peek().is_some() {
                with_trailing.push('\n');
            }
        }

        let normalized_trailing = build_normalized_content(&with_trailing);
        prop_assert_eq!(normalized_trailing, normalized);
    }

    /// `map_normalized_range_to_original` should produce valid byte
    /// ranges in the original content and the extracted original slice,
    /// when re-normalized, should start with the expected normalized
    /// prefix. Trailing whitespace at line ends makes an exact match
    /// impossible (normalization strips it), so we verify the key
    /// structural invariant: the range is valid and the non-whitespace
    /// content round-trips correctly.
    #[test]
    fn proptest_map_normalized_range_roundtrip(input in arbitrary_match_text()) {
        let normalized = build_normalized_content(&input);
        if normalized.is_empty() {
            return Ok(());
        }

        // Pick a range in the normalized text at char boundaries
        let norm_chars: Vec<(usize, char)> = normalized.char_indices().collect();
        let norm_len = norm_chars.len();
        if norm_len == 0 {
            return Ok(());
        }

        // Use the first quarter as the match range for determinism
        let end_char = (norm_len / 4).max(1).min(norm_len);
        let norm_start = norm_chars[0].0;
        let norm_end = if end_char < norm_chars.len() {
            norm_chars[end_char].0
        } else {
            normalized.len()
        };
        let norm_match_len = norm_end - norm_start;

        let (orig_start, orig_len) =
            map_normalized_range_to_original(&input, norm_start, norm_match_len);

        // Invariant 1: result is within input bounds
        prop_assert!(
            orig_start + orig_len <= input.len(),
            "mapped range {orig_start}..{} exceeds input len {}",
            orig_start + orig_len,
            input.len()
        );

        // Invariant 2: result is at valid char boundaries
        prop_assert!(
            input.is_char_boundary(orig_start),
            "orig_start {} is not a char boundary",
            orig_start
        );
        prop_assert!(
            input.is_char_boundary(orig_start + orig_len),
            "orig_end {} is not a char boundary",
            orig_start + orig_len
        );

        // Invariant 3: original range is at least as large as
        // normalized range (original may include trailing whitespace
        // and multi-byte Unicode chars that normalize to fewer bytes)
        prop_assert!(
            orig_len >= norm_match_len
                || orig_len == 0
                || norm_match_len == 0,
            "original range ({orig_len}) should be >= normalized range ({norm_match_len})"
        );

        // Invariant 4: the normalized expected slice, when searched
        // for in the original content via fuzzy_find_text, should be
        // found at or before the mapped position.
        let expected_norm = &normalized[norm_start..norm_end];
        if !expected_norm.is_empty() {
            let fuzzy_result = fuzzy_find_text(&input, expected_norm);
            prop_assert!(
                fuzzy_result.found,
                "normalized needle should be findable in original content"
            );
        }
    }
}

#[test]
fn test_truncate_head_preserves_newline() {
    // "Line1\nLine2" truncated to 1 line should be "Line1\n"
    let content = "Line1\nLine2".to_string();
    let result = truncate_head(content, 1, 1000);
    assert_eq!(result.content, "Line1\n");

    // "Line1" truncated to 1 line should be "Line1"
    let content = "Line1".to_string();
    let result = truncate_head(content, 1, 1000);
    assert_eq!(result.content, "Line1");

    // "Line1\n" truncated to 1 line should be "Line1\n"
    let content = "Line1\n".to_string();
    let result = truncate_head(content, 1, 1000);
    assert_eq!(result.content, "Line1\n");
}

#[test]
fn test_edit_crlf_content_correctness() {
    // Regression test: ensure we don't mix original indices with normalized content slices.
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("crlf.txt");
        // "line1" (5) + "\r\n" (2) + "line2" (5) + "\r\n" (2) + "line3" (5) = 19 bytes
        let content = "line1\r\nline2\r\nline3";
        std::fs::write(&path, content).unwrap();

        let tool = EditTool::new(tmp.path());

        // Replacing "line2" should work correctly and preserve CRLF.
        // Original "line2" is at index 7. Normalized "line2" is at index 6.
        // If we used original index (7) on normalized string ("line1\nline2\nline3"),
        // we would start at "ine2..." instead of "line2...", corrupting the file.
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "oldText": "line2",
                    "newText": "changed"
                }),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(!out.is_error);
        let new_content = std::fs::read_to_string(&path).unwrap();

        // Expect: "line1\r\nchanged\r\nline3"
        assert_eq!(new_content, "line1\r\nchanged\r\nline3");
    });
}

#[test]
fn test_edit_cr_content_correctness() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cr.txt");
        std::fs::write(&path, "line1\rline2\rline3").unwrap();

        let tool = EditTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "oldText": "line2",
                    "newText": "changed"
                }),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(!out.is_error);
        let new_content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(new_content, "line1\rchanged\rline3");
    });
}

// ========================================================================
// Hashline tests
// ========================================================================

#[test]
fn test_compute_line_hash_basic() {
    // Same content at same index should produce same hash
    let h1 = compute_line_hash(0, "fn main() {");
    let h2 = compute_line_hash(0, "fn main() {");
    assert_eq!(h1, h2);

    // Different content should (usually) produce different hash
    let h3 = compute_line_hash(0, "fn foo() {");
    // Not guaranteed different for all inputs, but these specific ones should differ
    assert_ne!(h1, h3);

    // Hash is 2 bytes from NIBBLE_STR
    for &b in &h1 {
        assert!(NIBBLE_STR.contains(&b), "hash byte {b} not in NIBBLE_STR");
    }
}

#[test]
fn test_compute_line_hash_punctuation_only() {
    // Punctuation-only lines use line_idx as seed, so same content at
    // different indices should produce different hashes.
    let h1 = compute_line_hash(0, "}");
    let h2 = compute_line_hash(1, "}");
    assert_ne!(
        h1, h2,
        "punctuation-only lines at different indices should differ"
    );

    // Blank lines also use idx as seed
    let h3 = compute_line_hash(0, "");
    let h4 = compute_line_hash(1, "");
    assert_ne!(h3, h4);
}

#[test]
fn test_compute_line_hash_whitespace_invariant() {
    // Leading/trailing whitespace should not affect hash (whitespace stripped)
    let h1 = compute_line_hash(0, "return 42;");
    let h2 = compute_line_hash(0, "    return 42;");
    let h3 = compute_line_hash(0, "\treturn 42;");
    assert_eq!(h1, h2);
    assert_eq!(h1, h3);
}

#[test]
fn test_format_hashline_tag() {
    let tag = format_hashline_tag(0, "fn main() {");
    // Should be "1#XX" format (1-indexed)
    assert!(
        tag.starts_with("1#"),
        "tag should start with 1#, got: {tag}"
    );
    assert_eq!(tag.len(), 4, "tag should be 4 chars: N#AB");

    let tag10 = format_hashline_tag(9, "line 10");
    assert!(tag10.starts_with("10#"));
    assert_eq!(tag10.len(), 5); // "10#AB"
}

#[test]
fn test_parse_hashline_tag_valid() {
    // Simple valid tag
    let (line, hash) = parse_hashline_tag("5#KJ").unwrap();
    assert_eq!(line, 5);
    assert_eq!(hash, [b'K', b'J']);

    // With spaces around #
    let (line, hash) = parse_hashline_tag("  10 # QR ").unwrap();
    assert_eq!(line, 10);
    assert_eq!(hash, [b'Q', b'R']);

    // With diff markers
    let (line, hash) = parse_hashline_tag("> + 3#ZZ").unwrap();
    assert_eq!(line, 3);
    assert_eq!(hash, [b'Z', b'Z']);
}

#[test]
fn test_parse_hashline_tag_invalid() {
    // Line number 0
    assert!(parse_hashline_tag("0#KJ").is_err());
    // No hash
    assert!(parse_hashline_tag("5#").is_err());
    // Invalid chars in hash
    assert!(parse_hashline_tag("5#AA").is_err()); // 'A' not in NIBBLE_STR
    // No number
    assert!(parse_hashline_tag("#KJ").is_err());
    // Empty
    assert!(parse_hashline_tag("").is_err());
}

#[test]
fn test_strip_hashline_prefix() {
    assert_eq!(strip_hashline_prefix("5#KJ:hello world"), "hello world");
    assert_eq!(strip_hashline_prefix("100#ZZ:fn main() {"), "fn main() {");
    assert_eq!(strip_hashline_prefix(" 5 # KJ:hello world"), "hello world");
    assert_eq!(strip_hashline_prefix("> + 5#KJ:hello world"), "hello world");
    assert_eq!(strip_hashline_prefix("5#KJ :hello world"), "hello world");
    // No prefix → unchanged
    assert_eq!(strip_hashline_prefix("hello world"), "hello world");
    assert_eq!(strip_hashline_prefix(""), "");
}

#[test]
fn test_hashline_edit_single_replace() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "line1\nline2\nline3\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());

        // Get the hash for line 2 (idx=1)
        let tag2 = format_hashline_tag(1, "line2");

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": tag2,
                "lines": ["changed"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "line1\nchanged\nline3\n");
    });
}

#[test]
fn test_hashline_edit_range_replace() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\nd\ne\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());

        let tag_b = format_hashline_tag(1, "b");
        let tag_d = format_hashline_tag(3, "d");

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": tag_b,
                "end": tag_d,
                "lines": ["X", "Y"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "a\nX\nY\ne\n");
    });
}

#[test]
fn test_hashline_edit_prepend() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "prepend",
                "pos": tag_b,
                "lines": ["inserted"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "a\ninserted\nb\nc\n");
    });
}

#[test]
fn test_hashline_edit_append() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "append",
                "pos": tag_b,
                "lines": ["inserted"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "a\nb\ninserted\nc\n");
    });
}

#[test]
fn test_hashline_edit_bottom_up_ordering() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\nd\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");
        let tag_d = format_hashline_tag(3, "d");

        // Two edits at different positions — both should apply correctly
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [
                { "op": "replace", "pos": tag_b, "lines": ["B"] },
                { "op": "replace", "pos": tag_d, "lines": ["D"] }
            ]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "a\nB\nc\nD\n");
    });
}

#[test]
fn test_hashline_edit_hash_mismatch() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello\nworld\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());

        // Use a deliberately wrong hash
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": "1#ZZ",
                "lines": ["changed"]
            }]
        });

        let result = tool.execute("test", input, None, None).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Hash validation failed"),
            "error should mention hash validation: {err_msg}"
        );
    });
}

#[test]
fn test_hashline_edit_dedup() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");

        // Duplicate edits should be deduplicated
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [
                { "op": "replace", "pos": &tag_b, "lines": ["B"] },
                { "op": "replace", "pos": &tag_b, "lines": ["B"] }
            ]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "a\nB\nc\n");
    });
}

#[test]
fn test_hashline_edit_noop_detection() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");

        // Replacing with identical content is a no-op
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": &tag_b,
                "lines": ["b"]
            }]
        });

        let result = tool.execute("test", input, None, None).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("no-ops"),
            "error should mention no-ops: {err_msg}"
        );
    });
}

#[test]
fn test_hashline_read_output_format() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let tool = ReadTool::new(dir.path());
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "hashline": true
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);
        let text = get_text(&out.content);

        // Each line should be in N#AB:content format
        for line in text.lines() {
            if line.starts_with('[') || line.is_empty() {
                continue; // skip metadata lines
            }
            assert!(
                hashline_tag_regex().is_match(line),
                "line should match hashline format: {line:?}"
            );
            assert!(
                line.contains(':'),
                "line should contain ':' separator: {line:?}"
            );
        }

        // First line should start with "1#"
        let first_line = text.lines().next().unwrap();
        assert!(first_line.starts_with("1#"), "first line: {first_line:?}");
    });
}

#[test]
fn test_hashline_edit_prefix_stripping() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");

        // Model copies hashline tags into replacement — they should be stripped
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": &tag_b,
                "lines": ["2#KJ:changed"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "a\nchanged\nc\n");
    });
}

#[test]
fn test_hashline_edit_delete_lines() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\nd\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");
        let tag_c = format_hashline_tag(2, "c");

        // Replace range with null (delete)
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": &tag_b,
                "end": &tag_c,
                "lines": null
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "a\nd\n");
    });
}

#[test]
fn test_hashline_edit_crlf_preservation() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "line1\r\nline2\r\nline3").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag2 = format_hashline_tag(1, "line2");

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": tag2,
                "lines": ["changed"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "line1\r\nchanged\r\nline3");
    });
}

#[test]
fn test_hashline_edit_cr_preservation() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "line1\rline2\rline3").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag2 = format_hashline_tag(1, "line2");

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": tag2,
                "lines": ["changed"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "line1\rchanged\rline3");
    });
}

#[test]
fn test_hashline_edit_empty_file_append() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        std::fs::write(&file, "").unwrap();

        let tool = HashlineEditTool::new(dir.path());

        // EOF append with no pos on empty file
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "append",
                "lines": ["new_line"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("new_line"));
    });
}

#[test]
fn test_hashline_edit_single_line_no_trailing_newline() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("single.txt");
        std::fs::write(&file, "hello").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag = format_hashline_tag(0, "hello");

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": tag,
                "lines": ["world"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "world");
    });
}

#[test]
fn test_hashline_edit_preserves_bom_hash_validation() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bom.txt");
        let bom = "\u{FEFF}";
        std::fs::write(&file, format!("{bom}alpha\nbeta\n")).unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag1 = format_hashline_tag(0, &format!("{bom}alpha"));

        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": tag1,
                "lines": ["gamma"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, format!("{bom}gamma\nbeta\n"));
    });
}

#[test]
fn test_hashline_edit_bof_prepend_no_pos() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());

        // Prepend with no pos should insert at BOF (before line 0)
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "prepend",
                "lines": ["header"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "header\na\nb\nc\n");
    });
}

#[test]
fn test_hashline_edit_eof_append_no_pos() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());

        // Append with no pos should insert at EOF (after last line)
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "append",
                "lines": ["footer"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert!(
            content.contains("footer"),
            "content should contain footer: {content:?}"
        );
    });
}

#[test]
fn test_hashline_edit_overlapping_replace_ranges_rejected() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\nd\ne\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");
        let tag_d = format_hashline_tag(3, "d");
        let tag_c = format_hashline_tag(2, "c");
        let tag_e = format_hashline_tag(4, "e");

        // Two overlapping replace ranges: lines 2-4 and lines 3-5
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [
                { "op": "replace", "pos": &tag_b, "end": &tag_d, "lines": ["X"] },
                { "op": "replace", "pos": &tag_c, "end": &tag_e, "lines": ["Y"] }
            ]
        });

        let result = tool.execute("test", input, None, None).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Overlapping"),
            "error should mention overlapping: {err_msg}"
        );
    });
}

#[test]
fn test_hashline_edit_reversed_range_rejected() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\nd\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag_b = format_hashline_tag(1, "b");
        let tag_d = format_hashline_tag(3, "d");

        // End anchor before start anchor
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": &tag_d,
                "end": &tag_b,
                "lines": ["X"]
            }]
        });

        let result = tool.execute("test", input, None, None).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("before start"),
            "error should mention before start: {err_msg}"
        );
    });
}

#[test]
fn test_hashline_edit_trailing_newline_semantics() {
    asupersync::test_utils::run_test(|| async {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        // File with trailing newline: split produces ["line1", "line2", ""]
        std::fs::write(&file, "line1\nline2\n").unwrap();

        let tool = HashlineEditTool::new(dir.path());
        let tag2 = format_hashline_tag(1, "line2");

        // Replace line2, trailing newline should be preserved
        let input = serde_json::json!({
            "path": file.to_str().unwrap(),
            "edits": [{
                "op": "replace",
                "pos": tag2,
                "lines": ["changed"]
            }]
        });

        let out = tool.execute("test", input, None, None).await.unwrap();
        assert!(!out.is_error);

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "line1\nchanged\n");
    });
}

#[test]
fn test_read_large_file_different_offsets_produce_different_content() {
    asupersync::test_utils::run_test(|| async {
        use std::fmt::Write as _;
        let tmp = tempfile::tempdir().unwrap();
        // 12000 lines, ~59 bytes each = ~700KB — far past the 8KB initial_read
        let mut content = String::with_capacity(700_000);
        for i in 1..=12000 {
            let _ = writeln!(
                content,
                "Line {i:05}: p p p p p p p p p p p p p p p p p p p p p p"
            );
        }
        std::fs::write(tmp.path().join("big.txt"), &content).unwrap();

        let tool = ReadTool::new(tmp.path());

        // Read offset=1 (beginning of file)
        let out1 = tool
            .execute(
                "t1",
                serde_json::json!({
                    "path": tmp.path().join("big.txt").to_string_lossy(),
                    "offset": 1,
                    "limit": 5,
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text1 = get_text(&out1.content);

        // Read offset=8001 (deep in the file, far past 8KB initial_read)
        let out2 = tool
            .execute(
                "t2",
                serde_json::json!({
                    "path": tmp.path().join("big.txt").to_string_lossy(),
                    "offset": 8001,
                    "limit": 5,
                }),
                None,
                None,
            )
            .await
            .unwrap();
        let text2 = get_text(&out2.content);

        eprintln!("=== offset=1 content (first 200 chars) ===");
        eprintln!("{}", &text1[..text1.len().min(200)]);
        eprintln!("=== offset=8001 content (first 200 chars) ===");
        eprintln!("{}", &text2[..text2.len().min(200)]);

        assert!(
            text1.contains("00001"),
            "offset=1 should contain line 00001"
        );
        assert!(
            !text1.contains("08001"),
            "offset=1 should NOT contain line 08001"
        );
        assert!(
            text2.contains("08001"),
            "offset=8001 should contain line 08001, got: {}",
            &text2[..text2.len().min(300)]
        );
        assert!(
            !text2.contains("00001"),
            "offset=8001 should NOT contain line 00001"
        );

        let count2 = text2.lines().filter(|l| l.contains('→')).count();
        assert!(
            (3..=6).contains(&count2),
            "offset=8001 should return ~5 lines, got {count2}"
        );
    });
}

// ========================================================================
// Pwsh Tool Tests
// ========================================================================

#[test]
fn test_pwsh_simple_command() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = PwshTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "echo hello_from_pwsh" }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("hello_from_pwsh"));
        assert!(!out.is_error);
    });
}

#[test]
fn test_pwsh_exit_code_nonzero() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = PwshTool::new(tmp.path());
        let out = tool
            .execute("t", serde_json::json!({ "command": "exit 42" }), None, None)
            .await
            .expect("non-zero exit should return Ok with is_error=true");
        assert!(out.is_error, "non-zero exit must set is_error");
    });
}

#[test]
fn test_pwsh_stderr_capture_on_error() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = PwshTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "Write-Error 'test_error_msg'; exit 1" }),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(out.is_error);
        let text = get_text(&out.content);
        assert!(
            text.contains("test_error_msg"),
            "expected stderr in output on error, got: {text}"
        );
    });
}

#[test]
fn test_pwsh_timeout() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = PwshTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "Start-Sleep 60", "timeout": 2 }),
                None,
                None,
            )
            .await
            .expect("timeout should return Ok with is_error=true");
        assert!(out.is_error, "timeout must set is_error");
    });
}

#[test]
fn test_pwsh_multiline_output() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = PwshTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "echo line1; echo line2; echo line3" }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("line1"));
        assert!(text.contains("line2"));
        assert!(text.contains("line3"));
        assert!(!out.is_error);
    });
}

#[test]
fn test_pwsh_working_directory() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = PwshTool::new(tmp.path());
        let out = tool
            .execute("t", serde_json::json!({ "command": "pwd" }), None, None)
            .await
            .unwrap();
        let text = get_text(&out.content);
        let cwd = tmp.path().canonicalize().unwrap();
        // Windows canonicalize adds \\?\ prefix; pwsh outputs the regular path
        let cwd_str = cwd.to_string_lossy();
        let cwd_clean = cwd_str.strip_prefix(r"\\?\").unwrap_or(&cwd_str);
        assert!(
            text.contains(cwd_clean),
            "expected cwd ({cwd_clean}) in output, got: {text}"
        );
    });
}

#[test]
fn test_pwsh_cjk_output() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();
        let tool = PwshTool::new(tmp.path());
        let out = tool
            .execute(
                "t",
                serde_json::json!({ "command": "echo '你好世界'" }),
                None,
                None,
            )
            .await
            .unwrap();
        let text = get_text(&out.content);
        assert!(text.contains("你好世界"), "CJK output should be preserved");
        assert!(!out.is_error);
    });
}

#[cfg(target_os = "windows")]
#[test]
fn test_pwsh_ambient_cancellation() {
    asupersync::test_utils::run_test(|| async {
        let tmp = tempfile::tempdir().unwrap();

        let ambient_cx = asupersync::Cx::for_testing();
        let cancel_cx = ambient_cx.clone();
        let _current = asupersync::Cx::set_current(Some(ambient_cx));

        let cancel_thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            cancel_cx.set_cancel_requested(true);
        });

        let result = run_pwsh_command(tmp.path(), "Start-Sleep 60", Some(30), None)
            .await
            .expect("run_pwsh_command should complete");

        cancel_thread.join().expect("cancel thread");

        // Cancellation should produce a non-zero exit code
        assert_ne!(
            result.exit_code, 0,
            "cancelled command should have non-zero exit"
        );
    });
}
