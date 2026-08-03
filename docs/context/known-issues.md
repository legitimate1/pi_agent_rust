# 已知问题 — 会话交接记录

> 本文件记录**已确认存在、尚未修复**的问题，供后续会话接手处理。
> 每条包含：现象、根因（如有调查结论）、复现命令、建议修法。
> 新增条目时保持此格式；修复后在条目上标记 `✅ 已修复` 并注明 commit。

---

## 1. Windows 无 bash 环境导致 bash 工具与相关测试失败（B 类）

**状态**：未修复 · **影响**：5 个 lib 测试 + 全部 bash_tool conformance fixture

**现象**（`cargo test --lib`）：
```
rpc::retry_tests::run_bash_rpc_large_output_completes_without_deadlock
agent::extensions_integration_tests::tool_call_hook_can_rewrite_bash_command_via_replace_input
→ 均报 "Tool error: bash: Failed to spawn shell: program not found"
```
`cargo test --test conformance_fixtures test_bash_fixtures`：全部 bash_* 用例同样报 `program not found`。

**根因**：Windows 上无 `bash` 可执行文件（BashTool 按 `/bin/bash`、`/usr/bin/bash`、`sh` 顺序查找）。非代码 bug，是环境缺失。

**复现**：`cargo test --test conformance_fixtures test_bash_fixtures`

**建议修法**（三选一，待用户决策）：
1. 安装 bash（Git Bash / WSL）→ 测试自然通过
2. 测试侧：Windows 上 skip bash 用例（`#[cfg(not(windows))]` 或 fixture runner 检测 `bash_available()`）
3. 工具侧：Windows 上 BashTool 回退到 `cmd`/`pwsh`（需确认产品意图——是否希望 bash 工具在 Windows 上可用）

---

## 2. auth SAP mock 测试在全量运行时偶发失败（并发竞争）

**状态**：未修复 · **影响**：2 个 lib 测试（全量时偶发）

**现象**（`cargo test --lib` 全量时）：
```
auth::tests::test_exchange_sap_access_token_with_client_http_error
auth::tests::test_exchange_sap_access_token_with_client_success
→ panicked: token exchange: ... IO error: 远程主机强迫关闭了一个现有的连接。 (os error 10054)
```
**单独运行均通过**（`cargo test --lib auth::tests::test_exchange_sap_access_token_with_client_success` → ok）。

**根因**：`spawn_json_server`（src/auth.rs:4894）起本地 TCP mock server，全量测试并发时连接被重置（疑似 mock server 线程竞争 / 连接复用冲突）。非业务代码问题。

**复现**：连续跑几次全量 `cargo test --lib` 可偶发复现。

**建议修法**：mock server 增加连接容错（accept 循环 / 重试），或测试加串行化（`serial_test`）。

---

## 3. extensions 策略断言失败（既有问题，与本次改动无关）

**状态**：未修复 · **影响**：1 个 lib 测试

**现象**（`cargo test --lib`）：
```
extensions::tests::explain_effective_policy_with_extension_override
→ assertion failed: explanation.dangerous_denied.contains(&"exec".to_string())
```
（src/extensions.rs:52492）

**根因**：未深入调查。与本次会话的 cwd 清理 / Windows 修复无关（stash 对照确认 pre-existing）。

**复现**：`cargo test --lib extensions::tests::explain_effective_policy_with_extension_override`

**建议修法**：调查 `explain_effective_policy_with_extension_override` 的策略叠加逻辑，确认 extension override 后 `dangerous_denied` 是否应含 `exec`。

---

## 4. cli_flags conformance golden 过时（CLI 新增 flag 未同步）

**状态**：未修复 · **影响**：1 个 conformance 测试

**现象**：
```
cargo test --test conformance_fixtures test_cli_flag_fixtures
[CLI-FLAGS-DEFAULTS, CLI-FLAGS-FULL-SHAPE] defaults FAILED: Details mismatch against golden 'tests/conformance/goldens/cli_flags/defaults.json'
```

**根因**：CLI 结构新增了 flag（`Cli` 结构体变化），但 `tests/conformance/goldens/cli_flags/defaults.json` 未同步更新。Expected/Actual 前 40 字段一致，差异在尾部字段。

**复现**：`cargo test --test conformance_fixtures test_cli_flag_fixtures`

**建议修法**：用 `pi` CLI 实际解析默认参数，导出 details JSON 更新 golden（或用测试输出中的 Actual 部分替换 golden）。

---

## 附：本次会话已修复的问题（存档）

以下问题已在 commit `c71b7d6e`（v0.1.58）修复，仅存档备查：

- **#33 golden 路径断言**：`resolve_golden_path` 增加 `RootDir/Prefix` 组件检查（Windows root-relative 路径 `is_absolute()` 为 false 的坑）
- **Windows 路径修复**：auth HOMEDRIVE+HOMEPATH 缺分隔符、migrations 测试 JSON 反斜杠未转义、theme 断言路径分隔符、logging.rs root-relative 拒绝
- **golden 文件 CRLF 污染**：`.gitattributes` 强制 `tests/conformance/goldens/** eol=lf`（修 5 个 conformance 失败）
- **cwd 限制残留清理**：移除 @file `enforce_read_scope` + 死代码 `enforce_cwd_scope`，删除过期测试与 fixture（工具可任意路径访问，与产品意图一致）
- **verify 支持 .md**：prettier 复用（`FileType::Markdown`）
- **deploy 脚本**：sweep `--file` 先、`--stamp` 后（防止误删最新构建）
- **AGENTS.md**：两档测试策略（日常针对性 / 收尾全量）
