# 症状排查手册（Debugging Playbooks）

> 源自原作者 pi-agent-rust 技能包，已本土化：对齐当前仓库结构、Windows 开发环境。
> 适用于：症状已知但根因不明的场景。每个 playbook 以症状为入口，以可验证的修复检查清单收尾。

## 症状路由表

- **Provider 流式/工具调用回归** → `cargo test provider_streaming -- --nocapture`；`rg -n "stream|tool|delta|event|SSE" src/providers src/sse.rs src/provider.rs`；`cargo test conformance`
- **中断后下次请求 400（tool_calls 未跟随 tool messages）** → `rg -n "build_abort_message|build_error_message|ToolCall" src/agent.rs`；`cargo test --lib abort_tests`（含回归 `abort_during_streaming_tool_call_strips_dangling_tool_call`）
- **会话重放/索引漂移** → `cargo test session -- --nocapture`；`rg -n "Session|save|open|index|jsonl|sqlite" src/session.rs src/session_index.rs src/session_sqlite.rs`；`cargo test conformance`
- **扩展策略/运行时故障** → `cargo test extension -- --nocapture`；`rg -n "policy|hostcall|capability|quickjs|deny|allow" src/extensions.rs src/extensions_js.rs src/extension_*.rs`；`cargo test conformance`
- **安装器/卸载器/技能问题** → `bash tests/installer_regression.sh`；`rg -n "AGENT_SKILL_STATUS|CHECKSUM_STATUS|SIGSTORE_STATUS|COMPLETIONS_STATUS" install.sh`；`rg -n "managed skill|expected skill directory|PIAR_AGENT_SKILL" uninstall.sh`
- **交互式与 RPC 行为分歧** → `cargo test e2e_rpc -- --nocapture`；`rg -n "interactive|rpc|stdin|event|session" src/main.rs src/interactive.rs src/rpc.rs`；`cargo test conformance`
- **RPC 斜杠命令交互超时（`JS extension runtime command timed out after 5000ms`）** → `rg -n "EXTENSION_COMMAND_BUDGET_MS|EXTENSION_EVENT_TIMEOUT_MS|run_extension_command|await_js_task" src/rpc.rs src/extensions.rs`；核 `run_extension_command` 是否用 `EXTENSION_COMMAND_BUDGET_MS`(30s) 而非 `EXTENSION_EVENT_TIMEOUT_MS`(5s)；RPC UI 请求含 `await ctx.ui.*` 时 5s 必超时（D50）。

> `cargo test <词>` 是子串匹配，`session` 会覆盖 session_index_tests / session_sqlite / session_store_v2 等，`extension` 会覆盖 ext_conformance 等，命中面比字面更宽，属预期。

---

## Playbook 1：Provider 流式 / 工具调用回归

### 症状

- 流式响应卡住、截断或产出畸形 delta。
- 不同 provider 后端之间的工具调用事件不一致。
- 改动 provider 或解析器后 provider 流式测试失败。
- 长思考后整个响应中断：`API error: JSON parse error: EOF while parsing a string at line N column M`（上游 SSE 中途截断，现已自动重试；若重试后仍频繁出现 → 反馈上游网关，如 opencode.ai）。
- 流式中途断开：`API error: Stream ended without Done event`（网络抖动/代理掐流，未收到 `Done/Error` 即 `FIN`，现已归入 `is_retryable_error` 自动走 `AutoRetryStart → 指数退避 → run_continue_with_abort`（最多 `retry.maxRetries=3` 次，已执行 `tool_call` 不重放）；必现则抓包确认是否缺 `data: [DONE]`/网关超时，compaction 链路的同错不自动重试、静默放弃压缩不影响主对话）。
- 模型间歇性「不思考」：assistant 消息的 thinking 块为空串（约半数轮次），用户输入轮尤为明显，且与操作类型无关。**根因通常是请求侧未发送思考参数**——`reasoning_style()` 通过 provider id / base_url 识别 DeepSeek 方言，opencode-go 网关（`opencode.ai/zen/go/v1`）两者都不匹配时落入 Standard 分支，请求体只有 `reasoning_effort` 没有 `thinking` 包装，模型自分配思考深度（时开时关）。修复：`compat.thinkingFormat == "deepseek"`（models.json）已纳入检测优先级（见 `reasoning_style()`）。排查时先核对 models.json 中该模型的 `compat.thinkingFormat` 是否声明，再检查请求体是否同时含 `thinking` + `reasoning_effort`。
- 中断（RPC `abort`/TUI Esc）后**下次请求 400**：`assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id'`。**根因**：流式输出 tool_call 期间中断时，abort/error 消息保留了已累积的 ToolCall blocks 并持久化，下次 `build_context` 原样发给 provider（opencode-go 网关校验严格）。**修复**（已落地）：`build_abort_message`/`build_error_message` strip dangling tool calls（与迭代上限路径一致）。排查时验证 abort 消息 content 无 `ToolCall` 块；**已损坏的旧会话**需手动删除 JSONL 中 `stop_reason: Aborted/Error` 且 content 含 `toolCall` 的那条 assistant 消息，代码不会自动清理历史数据。

### 前 3 条命令

```bash
cargo test provider_streaming -- --nocapture
rg -n "stream|tool|delta|event|SSE|responses|completions" src/providers src/provider.rs src/sse.rs
cargo test conformance
```

### 最小复现模板

```bash
# 替换为 provider_streaming 输出中最窄的失败用例名
cargo test provider_streaming::<failing_case> -- --nocapture
```

### 缩小改动面

```bash
rg -n "impl Provider|stream|tool" src/providers/*.rs src/providers/mod.rs src/provider.rs
rg -n "parse|event|data:" src/sse.rs
```

### 修复验证清单

- [ ] 修复前失败用例可复现。
- [ ] 修复后针对性 provider 测试通过。
- [ ] `cargo test conformance` 无回归。
- [ ] 错误/状态输出保持显式且未意外变更。

---

## Playbook 2：会话持久化 / 索引漂移

### 症状

- 两次运行之间会话重放/历史意外不一致。
- 会话索引元数据与存储条目不匹配。
- 改动会话相关代码后 save/open 路径回归。

### 前 3 条命令

```bash
cargo test session -- --nocapture
rg -n "Session|save|open|index|jsonl|sqlite" src/session.rs src/session_index.rs src/session_sqlite.rs src/session_store_v2.rs
cargo test conformance
```

### 最小复现模板

```bash
# 替换为输出中具体失败的会话测试
cargo test session::<failing_case> -- --nocapture
```

### 缩小改动面

```bash
rg -n "append|save|open|index|metadata" src/session.rs src/session_index.rs src/session_sqlite.rs
```

### 修复验证清单

- [ ] 修复前至少一个确定性会话测试可复现。
- [ ] 修复后会话测试切片通过。
- [ ] 受影响行为的一致性测试保持绿。
- [ ] 未引入无文档的格式/语义漂移。

---

## Playbook 3：扩展运行时 / 策略回归

### 症状

- hostcall 被意外拒绝/放行。
- 能力策略按 profile 表现不一致。
- QuickJS 运行时行为与预期策略执行偏离。

### 前 3 条命令

```bash
cargo test extension -- --nocapture
rg -n "extension|policy|hostcall|capability|quickjs|security|deny|allow" src/extensions.rs src/extensions_js.rs tests/
cargo test conformance
```

### 最小复现模板

```bash
# 替换为输出中具体失败的扩展测试
cargo test extension::<failing_case> -- --nocapture
```

### 缩小改动面

```bash
rg -n "allow|deny|policy|capability|hostcall" src/extensions.rs src/extensions_js.rs
```

### 修复验证清单

- [ ] 修复前失败扩展测试可复现。
- [ ] 修复后针对性扩展切片通过。
- [ ] 策略语义仍是最小权限且显式。
- [ ] 更广的一致性测试保持稳定。

---

## Playbook 4：安装器 / 卸载器 / 技能安装失败

### 症状

- 安装器汇总状态错误、含糊或自相矛盾。
- 既有自定义技能目录被意外改动。
- 卸载删除了非预期路径。
- checksum/签名/completion 分支回归。

### 前 3 条命令

```bash
bash tests/installer_regression.sh
rg -n "AGENT_SKILL_STATUS|CHECKSUM_STATUS|SIGSTORE_STATUS|COMPLETIONS_STATUS|install_skill_to_destination" install.sh
rg -n "remove_installed_skills|is_expected_skill_directory|is_managed_skill_file|PIAR_AGENT_SKILL" uninstall.sh
```

### 最小复现模板

```bash
# 编辑 tests/installer_regression.sh 隔离失败用例，然后：
bash tests/installer_regression.sh

# 技能完整性 + 内联同步门禁：
bash scripts/skill-smoke.sh
```

### 缩小改动面

```bash
rg -n "install_skill_to_destination|install_agent_skills|write_state|print_summary" install.sh
rg -n "remove_installed_skills|is_expected_skill_directory|is_managed_skill_file" uninstall.sh
```

### 修复验证清单

- [ ] 修复前失败安装器回归用例可复现。
- [ ] 修复后 `bash tests/installer_regression.sh` 通过。
- [ ] 修复后 `bash scripts/skill-smoke.sh` 通过。
- [ ] 自定义技能保留、仅托管技能删除的行为不变。

---

## Playbook 5：CLI/TUI 与 RPC 行为分歧

### 症状

- 交互模式正常而 RPC/stdin 模式失败（或反之）。
- 不同表面的事件顺序/形状不一致。

### 前 3 条命令

```bash
cargo test e2e_rpc -- --nocapture
rg -n "interactive|rpc|stdin|event|session" src/main.rs src/interactive.rs src/rpc.rs
cargo test conformance
```

### 最小复现模板

```bash
# 替换为输出中具体失败的 RPC 测试
cargo test e2e_rpc::<failing_case> -- --nocapture
```

### 修复验证清单

- [ ] 修复前失败 RPC 用例可复现。
- [ ] 修复后针对性 RPC 测试通过。
- [ ] 事件行为跨表面保持一致。
- [ ] 更广的一致性测试仍通过。

---

## 安装器专项：失败排查

### 状态不一致 / 摘要错误

安装器汇总状态由四个变量驱动，排查时在 `install.sh` 中追踪其赋值路径：

- `AGENT_SKILL_STATUS` — 技能安装结果（skipped / partial / installed 等）
- `CHECKSUM_STATUS` — 发布包 checksum 校验结果
- `SIGSTORE_STATUS` — sigstore/cosign 签名校验结果
- `COMPLETIONS_STATUS` — shell completion 安装结果

### 卸载安全

卸载逻辑必须同时满足两个条件才删除，二者缺一不可：

1. **标记检查**：文件含 `pi_agent_rust installer managed skill` 标记（`is_managed_skill_file`）
2. **路径形状检查**：目标符合 `*/skills/pi-agent-rust` 形态（`is_expected_skill_destination`）

### 已知坑点

- 自定义产物安装路径若无兼容的发布上下文，且未显式加保护，可能错误回退。
- 技能状态在混合结果下会失真，除非部分/失败分支是显式的。
- 卸载逻辑必须同时校验标记和期望的目标目录路径形状。
- 当 stdout 用于数据管道时，安装器的进度/状态文本应保持在 stderr。
- 捆绑技能与内联回退可能静默漂移，除非显式校验（`scripts/skill-smoke.sh` 即此门禁）。

### 补丁模式

**模式 1：混合结果状态清晰化**

```bash
# 之前：所有情况都折叠成 "skipped custom"
if [ "$skipped_custom" -ge 1 ]; then
  AGENT_SKILL_STATUS="skipped (existing custom skill)"
fi

# 之后：区分"自定义技能跳过"与"写入失败"
if [ "$skipped_custom" -ge 1 ] && [ "$failed_writes" -ge 1 ]; then
  AGENT_SKILL_STATUS="partial (custom skill kept; other install failed)"
elif [ "$skipped_custom" -ge 1 ]; then
  AGENT_SKILL_STATUS="skipped (existing custom skill)"
fi
```

**模式 2：安全的技能替换（暂存后原子移动）**

```bash
# 之前：先删目标，再验证复制结果
rm -rf "$destination"
cp "$source" "$destination/SKILL.md"

# 之后：先暂存，再原子移动到目标位置
staged="$(mktemp -d ...)"
cp "$source" "$staged/SKILL.md"
mv "$staged" "$destination"
```

---

## 标准升级路径

仅在针对性复现 + 聚焦切片确认有必要之后，才按此顺序扩大范围：

```bash
# 1) 先跑针对性失败切片
cargo test <targeted-slice> -- --nocapture

# 2) 改动代码的本地不变量
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# 3) 更广的行为信号
cargo test conformance
```

> 重负载编译/测试在 Windows 本地直接跑 cargo 即可；若在 Linux 容器环境，用 `rch exec -- <cargo ...>` 包裹。

## 根因确认清单

- [ ] 用确定性命令复现了失败。
- [ ] 找到一个能解释症状的最小改动。
- [ ] 为修复路径新增或更新了回归覆盖。
- [ ] 针对性切片上验证了"修复前失败、修复后通过"。
- [ ] 对受影响表面跑了更广的安全门禁。
- [ ] 用户可见行为变更时更新了文档/技能指引。
