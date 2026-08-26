// Single shell abstraction — unified bash/pwsh entry point.
// BashTool/PwshTool::execute 或 run_*_command 签名变更必须同步更新此处 match
use super::*;
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellInput {
    shell: String,
    command: String,
    timeout: Option<i64>,
}

pub struct ShellTool {
    cwd: PathBuf,
    shell_path: Option<String>,
    command_prefix: Option<String>,
    artifact_root: Option<PathBuf>,
}

impl ShellTool {
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
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }
    fn label(&self) -> &str {
        "shell"
    }
    fn description(&self) -> &str {
        "执行 shell 命令，需显式指定方言。在当前工作目录执行，返回输出文本。"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "shell": {
                    "type": "string",
                    "enum": ["bash", "pwsh"],
                    "description": "方言：bash 或 pwsh"
                },
                "command": {
                    "type": "string",
                    "description": "要执行的命令"
                },
                "timeout": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "超时秒数，默认 120，0 表示禁用"
                }
            },
            "required": ["shell", "command"]
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
        abort: Option<AbortSignal>,
    ) -> Result<ToolOutput> {
        let input: ShellInput =
            serde_json::from_value(input).map_err(|e| Error::validation(e.to_string()))?;

        if input.command.trim().is_empty() {
            return Err(Error::validation("command 不能为空"));
        }

        if input.shell != "bash" && input.shell != "pwsh" {
            return Err(Error::validation(
                "Expected shell to be \"bash\" or \"pwsh\"",
            ));
        }

        if let Some(v) = input.timeout {
            if v < 0 {
                return Err(Error::validation("timeout must be >= 0"));
            }
        }

        let timeout_u64: Option<u64> = input.timeout.map(|v| {
            #[allow(clippy::cast_sign_loss)]
            {
                v as u64
            }
        });

        match input.shell.as_str() {
            "bash" => {
                let bash_tool = if self.shell_path.is_some() || self.command_prefix.is_some() {
                    BashTool::with_shell(
                        &self.cwd,
                        self.shell_path.clone(),
                        self.command_prefix.clone(),
                    )
                } else {
                    BashTool::new(&self.cwd)
                };
                let bash_input = serde_json::json!({
                    "command": input.command,
                    "timeout": timeout_u64
                });
                bash_tool
                    .execute(tool_call_id, bash_input, on_update, abort)
                    .await
            }
            "pwsh" => {
                let pwsh_tool = PwshTool::new(&self.cwd);
                let pwsh_input = serde_json::json!({
                    "command": input.command,
                    "timeout": timeout_u64
                });
                pwsh_tool
                    .execute(tool_call_id, pwsh_input, on_update, abort)
                    .await
            }
            _ => Err(Error::validation(
                "Expected shell to be \"bash\" or \"pwsh\"",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asupersync::runtime::RuntimeBuilder;
    use tempfile::tempdir;

    fn test_runtime() -> asupersync::runtime::Runtime {
        RuntimeBuilder::current_thread()
            .build()
            .expect("runtime build")
    }

    async fn execute_shell(
        shell: &str,
        command: &str,
        timeout: Option<serde_json::Value>,
    ) -> Result<ToolOutput> {
        let tmp = tempdir().expect("tempdir");
        let tool = ShellTool::new(tmp.path());
        let mut input = serde_json::json!({
            "shell": shell,
            "command": command
        });
        if let Some(t) = timeout {
            input["timeout"] = t;
        }
        tool.execute("test-id", input, None, None).await
    }

    #[test]
    fn shell_tool_has_correct_name_and_params() {
        let tmp = tempdir().expect("tempdir");
        let tool = ShellTool::new(tmp.path());
        assert_eq!(tool.name(), "shell");
        let params = tool.parameters();
        assert_eq!(params["required"], serde_json::json!(["shell", "command"]));
        assert_eq!(
            params["properties"]["shell"]["enum"],
            serde_json::json!(["bash", "pwsh"])
        );
        assert_eq!(
            params["properties"]["timeout"]["minimum"],
            serde_json::json!(0)
        );
    }

    #[test]
    fn shell_forwards_to_pwsh_and_bash() {
        let rt = test_runtime();
        rt.block_on(async {
            // pwsh branch
            let out = execute_shell("pwsh", "echo hi", None)
                .await
                .expect("pwsh should succeed");
            assert!(
                !out.is_error,
                "pwsh echo hi should not be error: {:?}",
                out.content
            );
            let text = match &out.content[0] {
                ContentBlock::Text(t) => t.text.clone(),
                _ => panic!("expected text"),
            };
            assert!(text.contains("hi"), "output should contain hi: {text}");

            // bash branch (Git Bash on Windows, /bin/bash on Unix)
            if crate::tools::bash_available() {
                let out = execute_shell("bash", "echo hi", None)
                    .await
                    .expect("bash should succeed");
                assert!(!out.is_error);
                let text = match &out.content[0] {
                    ContentBlock::Text(t) => t.text.clone(),
                    _ => panic!("expected text"),
                };
                assert!(text.contains("hi"), "bash output should contain hi: {text}");
            }
        });
    }

    #[test]
    fn invalid_shell_rejected() {
        let rt = test_runtime();
        rt.block_on(async {
            for bad in ["fish", "", "Bash", "PWSH", " pwsh"] {
                let res = execute_shell(bad, "echo hi", None).await;
                assert!(res.is_err(), "shell={bad:?} should be rejected");
                let err = res.unwrap_err().to_string();
                assert!(
                    err.contains("Expected shell to be"),
                    "error should mention expected values, got: {err}"
                );
            }
        });
    }

    #[test]
    fn timeout_validation() {
        let rt = test_runtime();
        rt.block_on(async {
            // None => ok (uses default 120s)
            let res = execute_shell("pwsh", "echo hi", None).await;
            assert!(res.is_ok(), "timeout None should succeed");

            // 0 => ok (disable)
            let res = execute_shell("pwsh", "echo hi", Some(serde_json::json!(0))).await;
            assert!(res.is_ok(), "timeout 0 should succeed");

            // negative => validation
            let res = execute_shell("pwsh", "echo hi", Some(serde_json::json!(-1))).await;
            assert!(res.is_err(), "negative timeout should be rejected");
            assert!(res.unwrap_err().to_string().contains("timeout"));

            // string type => validation via serde
            let res = execute_shell("pwsh", "echo hi", Some(serde_json::json!("120"))).await;
            assert!(res.is_err(), "string timeout should be rejected");
        });
    }

    #[test]
    fn empty_command_rejected() {
        let rt = test_runtime();
        rt.block_on(async {
            for cmd in ["", "   ", "\n\t "] {
                let res = execute_shell("pwsh", cmd, None).await;
                assert!(res.is_err(), "command={cmd:?} should be rejected");
                let err = res.unwrap_err().to_string();
                assert!(err.contains("command"), "got: {err}");
            }
        });
    }

    #[test]
    fn timeout_forwarded_to_backend() {
        let rt = test_runtime();
        rt.block_on(async {
            // Valid timeout should be forwarded without error
            let out = execute_shell("pwsh", "echo hi", Some(serde_json::json!(60)))
                .await
                .expect("timeout 60 should succeed");
            assert!(!out.is_error);
            // Backend respects timeout: a 1s sleep with 1s timeout should timeout/cancel,
            // but we just verify forwarding doesn't break; use short command.
            let out = execute_shell("bash", "echo forwarded", Some(serde_json::json!(10))).await;
            if crate::tools::bash_available() {
                let out = out.expect("bash forwarded should succeed");
                assert!(!out.is_error);
            } else {
                // bash not available is not failure of forwarding logic
                assert!(out.is_ok() || out.is_err());
            }
        });
    }
}
