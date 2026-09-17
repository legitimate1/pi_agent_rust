# 波次 04：turn recovery 自动续跑

## 分析目的

确认上游 turn recovery 候选是否能在当前 `custom` 的 Agent loop、session persistence、retry 和多入口结构中形成独立功能闭包，并按已定稿设计完成手动适配。未直接 merge 或 cherry-pick 上游提交。

## 分析边界

上游固定终点：

```text
upstream/main = 82b7c0c8e72c7fed5cc1ab74ebccfe378419c9e6
```

核心上游提交：

```text
1ba1e39d0d91ebf3c7b0c97647d148b7401bb459
feat(agent): turn recovery — unexpected-stop classifier + capped auto-continue
```

当前 custom 实现起点：

```text
6ece4b7f3ffec0df6ce7645f623df858c08369f1
```

设计与实现文档：

```text
docs/design/pi-agent-rust-turn-recovery-design.md
docs/design/pi-agent-rust-turn-recovery-implement.md
```

## 一句话结论

上游 turn recovery 已按 custom 的 Agent loop、AgentSession、retry、interactive、RPC、SDK 和 ACP 边界完成手动适配：正常 `Stop`/`Length` 的有限自动续跑使用普通 user nudge，单次逻辑 run 最多两次，provider error retry 与 recovery budget 显式隔离；未改变 session schema、compaction/`tokensAfter`、RPC/ACP wire 或 Cargo 依赖。

## 设计决策

### Recovery mode

支持三种配置：

```text
off          → TurnRecoveryMode::Off
conservative → TurnRecoveryMode::Conservative
aggressive   → TurnRecoveryMode::Aggressive
```

缺少 `turn_recovery` 时默认 `Conservative`；未知值沿现有 settings serde 错误路径失败。`Aggressive` 才启用 semantic promise heuristic。

### 分类范围

- `StopReason::Length` → `BudgetTruncated`；
- `StopReason::Stop` + 奇数 code fence 或末尾悬空列表项 → `UnclosedStructure`；
- `StopReason::Stop` + 固定 promise 末尾窗口 → `SemanticPrematureStop`；
- 普通 Stop、ToolUse、Error、Aborted 和未来未识别 stop reason → `CleanStop`。

assistant 文本只取 `ContentBlock::Text`，忽略 thinking、redacted thinking、image、tool call 和 arguments。

### Budget 与 scope

`LogicalRunScope` 不放入 Agent 全局字段。公开独立 `run*`/AgentSession prompt 创建 fresh scope；main/RPC 的同一高层 retry wrapper 将首次 provider attempt、recovery continuation 和 provider retry/resume 传入同一 scope；SDK/interactive 独立 continue 显式创建 fresh scope。

每个 logical run 最多两条 recovery nudge。首次 recovery 后，结构和 semantic heuristic 不再重复触发；剩余额度只允许 provider 明确返回 `Length` 时使用。provider retry 本身不增加 recovery continuation，也不重置已有 budget。

### Nudge、事件和持久化

nudge 格式固定为普通 `Message::User(UserMessage)`：

```text
[auto-continue {n}/2: {reason}] Continue from exactly where you stopped. Do not repeat content you already produced; finish the remaining work.
```

它沿既有 MessageStart/MessageEnd、TurnStart/TurnEnd、Agent transcript、provider context 和 session autosave 路径流动，不新增 recovery 专属事件、session entry 或 wire 字段。

`save_enabled=true` 时沿 AgentSession/interactive 原有高层批量阶段保存；`false` 时只留在内存 transcript、provider context 和 event stream，不写 session JSONL。未引入 recovery 专用 flush。

### Agent loop 优先级

实际插入顺序为：

```text
assistant 完成
→ tool call 优先执行，不 recovery
→ TurnEnd
→ drain steering，steering 优先
→ 无 tool/steering 时 evaluate recovery
→ 注入普通 user nudge
→ 下一 provider turn
→ recovery 完成后再进入 idle follow-up
```

`Error`、`Aborted`、provider Err、abort 和 tool execution abort 继续走既有错误/重试/结束路径。

## 实际源码变更

```text
src/turn_recovery.rs
src/lib.rs
src/agent.rs
src/config.rs
src/main.rs
src/rpc.rs
src/sdk.rs
src/acp.rs
src/interactive/agent.rs
```

### 纯机制模块

`src/turn_recovery.rs` 新增：

- `TurnRecoveryMode`、`RecoveryClass`、`RecoveryAction`、`TurnRecoveryState`；
- `classify`、mode gating、code fence/悬空列表/promise heuristic；
- nudge 生成、两次 cap、结构 heuristic 一次性限制；
- 纯单元测试。

### Agent 与 AgentSession

`src/agent.rs`：

- `AgentConfig.turn_recovery`、Debug 和 Default；
- `LogicalRunScope` 与显式 scope-aware Agent API；
- `AgentSession` 的 scope-aware text/content/continue API；
- assistant 可见文本提取；
- Agent loop recovery 注入；
- Agent loop、scope、tool/steering/follow-up/error/abort focused tests。

### 配置和入口

- `src/config.rs` 增加 settings 字段、merge 和最终 accessor；
- `src/main.rs`、`src/rpc.rs` 的 retry wrapper 显式持有同一 scope；
- `src/sdk.rs` 的 prompt 继续使用 AgentSession fresh scope，独立 continue 使用 fresh scope；
- `src/acp.rs` 使用最终配置并沿既有 session/update wire；
- `src/interactive/agent.rs` 的裸 Agent continue 显式创建 fresh scope；
- 所有受影响的 AgentConfig literal 和测试 fixture 已迁移，旧行为测试使用 `Off`，recovery 测试使用 Conservative/Aggressive。

实现提交：

```text
33b4751f2 feat: add turn recovery mechanism
 d13a0350c feat: integrate turn recovery into agent loop
2ed6037fc feat: configure turn recovery mode
4f6b90a53 feat: wire turn recovery across entry points
eefefce7b fix: bound structural turn recovery
```

实现计划/设计文档提交：

```text
3f4b690e6 docs: add turn recovery implementation plan
```

## 验证摘要

### Targeted 验证

以下命令均通过，并通过 Windows `pwsh` 执行：

```text
cargo test --lib turn_recovery
  15 passed
cargo test --lib agent
  215 passed
cargo test --lib config
  227 passed
cargo test --lib sdk
  36 passed
cargo test --lib acp
  37 passed
cargo test --lib rpc::retry_tests
  10 passed
cargo test --lib turn_event_tests::
  13 passed
cargo fmt --check
cargo clippy --lib -- -D warnings
```

受影响的 integration/example 编译检查通过：

```text
cargo test --test e2e_agent_loop --no-run
cargo test --test sdk_integration --no-run
cargo test --test agent_loop_reliability --no-run
cargo test --test e2e_rpc --no-run
cargo test --example pi_debug --no-run
```

### 收尾质量门禁

```text
cargo fmt --check                 PASS
cargo clippy --all-targets -- -D warnings  PASS
```

全量 `cargo test` 结果：

```text
6498 passed; 5 failed; 1 ignored
```

5 个失败均为现有 Windows jobs/Hub 进程与 PTY 基线问题，与本波次修改文件无交集：

```text
jobs::tests::cancel_kills_running_job
  TERM-ignoring job 未观察到 KILL escalation

hub::tests::logs_cursor_advances_incrementally
hub::tests::send_text_drives_repl
hub::tests::status_stays_running_for_live_repl
  Windows PTY readiness 失败，日志尾部为 ESC [6n

hub::tests::restart_after_completion_works
  Windows PTY 子进程状态/退出码观察失败
```

本波次未修改 `src/jobs.rs` 或 `src/hub.rs`，未将这些失败归因于 turn recovery。未执行真实 provider 网络验证、release build、release-max build 或部署。

## 未处理风险

- promise、code fence、悬空列表均为启发式，不是 Markdown parser 或语义模型判断；默认只启用 Conservative；
- nudge 是真实 transcript user message，会增加后续上下文 token、请求成本和未来 compaction 负担；
- custom 采用 eventual persistence，nudge 注入后到高层调用返回前进程崩溃时，尚未 flush 的消息可能丢失；
- 全量测试仍有 5 个与本波次无关的 Windows jobs/Hub 基线失败；
- 尚未进行真实 provider 网络验证；
- recovery 入口暂不增加独立 observability 事件，调用方通过普通 nudge/message/event 流观察。

## 停止边界

本波次停止在：

```text
normal Stop/Length classification
    → shared Agent loop
    → explicit logical-run scope
    → normal user nudge/event/transcript flow
    → existing session persistence and retry boundaries
    → interactive/print/RPC/SDK/ACP configuration wiring
```

明确未处理：

- 不直接 merge/cherry-pick `1ba1e39d...`；
- 不迁移 `PauseTurn`、`Refusal`、FTUI、MCP、workspace trust 或 provider 变化；
- 不修改 session JSONL/schema、compaction/`tokensAfter`、RPC/ACP wire 或 Cargo 依赖；
- 不处理 Windows jobs/Hub 既有基线失败；
- 不自动构建、部署或运行 release-max。

## 状态

```text
status: implemented
implementation_started: true
implementation_complete: true
wave: 04
upstream_endpoint: 82b7c0c8e72c7fed5cc1ab74ebccfe378419c9e6
next: wait for user instruction / select next independent upstream candidate
```
