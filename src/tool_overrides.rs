//! Per-tool overrides for everything the LLM sees about built-in tools:
//! the descriptive text and the JSON Schema parameters.
//!
//! Loaded from optional `tools.toml` files:
//! - `<global_dir>/tools.toml` (user-level, e.g. `~/.pi/agent/tools.toml`)
//! - `<cwd>/.pi/tools.toml` (project-level, wins on key collisions)
//!
//! Format:
//! ```toml
//! [bash]
//! description = "Execute bash commands on the remote host via SSH"
//! parameters = """
//! { "type": "object", "properties": { "command": { "type": "string" } }, "required": ["command"] }
//! """
//! ```
//! Tools not listed keep their built-in defaults; deleting a key or the file
//! restores the default (overrides never freeze the built-in text).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

/// Overrides for a single tool.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolOverride {
    /// Replaces both the prompt text and the API schema description.
    pub description: Option<String>,
    /// Raw JSON text; replaces the tool's JSON Schema parameters wholesale.
    pub parameters: Option<String>,
}

/// All tool overrides merged from user- and project-level `tools.toml`.
#[derive(Debug, Clone, Default)]
pub struct ToolOverrides {
    pub descriptions: HashMap<String, String>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Load and merge `tools.toml` from the user-level and project-level
/// directories. Project-level wins on per-key collisions. Missing files are
/// skipped; malformed TOML or JSON is an error.
pub fn load_tool_overrides(global_dir: &Path, cwd: &Path) -> Result<ToolOverrides> {
    let user_path = global_dir.join("tools.toml");
    let project_path = cwd.join(".pi/tools.toml");

    let mut overrides = ToolOverrides::default();
    merge_tool_overrides(&mut overrides, &user_path)?;
    merge_tool_overrides(&mut overrides, &project_path)?;
    Ok(overrides)
}

fn merge_tool_overrides(overrides: &mut ToolOverrides, path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(path).map_err(|err| {
        anyhow::anyhow!(
            "Could not read tool overrides file {}: {err}",
            path.display()
        )
    })?;
    if content.trim().is_empty() {
        return Ok(());
    }

    let parsed: HashMap<String, ToolOverride> = toml::from_str(&content).map_err(|err| {
        anyhow::anyhow!(
            "Failed to parse tool overrides file {}: {err}",
            path.display()
        )
    })?;

    for (tool_name, tool_override) in parsed {
        if let Some(description) = tool_override.description {
            overrides
                .descriptions
                .insert(tool_name.clone(), description);
        }
        if let Some(parameters) = tool_override.parameters {
            let value: serde_json::Value = serde_json::from_str(&parameters).map_err(|err| {
                anyhow::anyhow!(
                    "Invalid JSON in parameters for tool {tool_name:?} ({}): {err}",
                    path.display()
                )
            })?;
            overrides.parameters.insert(tool_name, value);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_tool_overrides_missing_files_returns_empty() {
        let temp = tempfile::tempdir().expect("temp dir");
        let overrides = load_tool_overrides(temp.path(), temp.path()).expect("load");
        assert!(overrides.descriptions.is_empty());
        assert!(overrides.parameters.is_empty());
    }

    #[test]
    fn load_tool_overrides_merges_user_and_project_with_project_wins() {
        let temp = tempfile::tempdir().expect("temp dir");
        let global = temp.path().join("global");
        let project = temp.path().join("project");
        std::fs::create_dir_all(&global).expect("global dir");
        std::fs::create_dir_all(project.join(".pi")).expect("project .pi");

        std::fs::write(
            global.join("tools.toml"),
            r#"
[bash]
description = "user bash description"
parameters = """{"type":"object","properties":{"command":{"type":"string"}}}"""

[read]
description = "user read description"
"#,
        )
        .expect("user tools.toml");

        std::fs::write(
            project.join(".pi/tools.toml"),
            r#"
[bash]
description = "project bash description"
"#,
        )
        .expect("project tools.toml");

        let overrides = load_tool_overrides(&global, &project).expect("load");

        // Project wins on bash description; user value for read survives.
        assert_eq!(overrides.descriptions["bash"], "project bash description");
        assert_eq!(overrides.descriptions["read"], "user read description");
        // Parameters only present in user file.
        assert!(overrides.parameters.contains_key("bash"));
        assert_eq!(
            overrides.parameters["bash"]["properties"]["command"]["type"],
            "string"
        );
    }

    #[test]
    fn load_tool_overrides_rejects_malformed_json() {
        let temp = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            temp.path().join("tools.toml"),
            "[bash]\nparameters = \"{not json}\"\n",
        )
        .expect("write tools.toml");

        let err = load_tool_overrides(temp.path(), temp.path()).expect_err("must fail");
        assert!(
            err.to_string().contains("Invalid JSON"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn load_tool_overrides_rejects_malformed_toml() {
        let temp = tempfile::tempdir().expect("temp dir");
        std::fs::write(temp.path().join("tools.toml"), "not [valid toml").expect("write");
        let err = load_tool_overrides(temp.path(), temp.path()).expect_err("must fail");
        assert!(
            err.to_string().contains("Failed to parse tool overrides"),
            "unexpected: {err}"
        );
    }
}
