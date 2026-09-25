# 上下文压缩

## 文档职责

本文是上下文压缩子系统的 Agent-facing 参考。它负责说明当前实现的模块边界、触发契约、压缩与 replay 数据流、运行时生命周期、持久化边界、扩展钩子和已知陷阱。

需要确认最终实现时，以源码和测试为准；本文不替代源码注释、配置定义或 provider usage 语义。

## 适用范围

遇到以下任务时读取本文：

- 修改自动或手动 context compaction；
- 修改 context token 估算、context window 或 reserve/keep-recent 配置；
- 修改 compaction cut point、split turn 或 tool-call/tool-result 保留规则；
- 修改 compaction summary、session entry、session replay 或 `tokensAfter`；
- 修改后台 compaction worker、超时、取消、冷却或尝试次数限制；
- 修改 `/compact`、RPC `compact`、auto-compaction 生命周期事件；
- 修改 `session_before_compact` / `session_compact` 扩展钩子。

纯 provider tokenizer、普通 provider retry 或与 compaction 无关的 session 功能，不需要因为关键词命中而读取本文。

## 子系统边界与源码入口

上下文压缩由以下组件共同负责：

- `src/compaction.rs`
  - 压缩设置、token 估算、触发判断、当前路径切点、摘要生成、压缩结果和 `tokensAfter` 估算。
- `src/compaction_worker.rs`
  - Agent 主路径使用的后台压缩任务、单 session 状态、取消、超时、冷却和尝试次数配额。
- `src/agent.rs`
  - 正常 Agent turn 开始前的两阶段后台压缩、压缩结果应用、事件传播和 session entry 写入。
- `src/session.rs`
  - `CompactionEntry` 的追加、当前 branch path、压缩后的 provider message replay。
- `src/rpc.rs`
  - RPC `compact`、RPC turn 结束后的自动压缩、响应和生命周期事件。
- `src/interactive/perf.rs`
  - 交互式 `/compact`，包括扩展钩子、session 保存和 TUI conversation reset。
- `src/config.rs`
  - `compaction.enabled`、`reserve_tokens`、`keep_recent_tokens` 及其默认值。

相关测试主要位于上述 Rust 模块的 `#[cfg(test)]` 区域，覆盖 token fallback、cut point、split turn、session replay、worker quota、事件序列化和 `tokensAfter`。

## 当前行为契约

### 触发条件

压缩启用且当前估算上下文达到以下阈值时才允许压缩：

```text
context_tokens >= context_window_tokens - reserve_tokens
```

配置默认值来自 `src/config.rs`：

- `compaction.enabled`：启用；
- `compaction.reserve_tokens`：`16384`；
- `compaction.keep_recent_tokens`：`20000`。

`ResolvedCompactionSettings` 提供兜底 context window 和设置值。正常 CLI、SDK、ACP 和 RPC 初始化路径应使用当前模型的 context window 覆盖兜底值。

### Token 估算

估算优先使用最近一条未中止、未报错的 assistant message 的 provider usage；没有有效 usage 时使用本地 chars/3 启发式。估算覆盖文本、thinking、tool call 参数、tool result、bash 内容、branch summary、compaction summary 和图片的固定估算值。

该估算只代表运行时的本地近似，不是 provider tokenizer 的精确结果。`tokensBefore` 和 `tokensAfter` 不能直接解释为完整 prompt token 数，因为它们不独立计入 system prompt 和 tool schema 的 token。

### 压缩输入与切点

压缩只处理当前 session leaf 的 chronological path，不处理其他分支。

如果当前路径已有上一个 compaction entry：

- 新一轮压缩从该 entry 之后的 segment 开始；
- 上一个 summary 作为 `previous_summary` 参与增量摘要；
- 当前 token 估算把上一个 summary 当作已压缩历史的一部分，而不是恢复原始历史大小。

切点从尾部向前累计，目标是保留约 `keep_recent_tokens` 的近期内容。切点规则必须满足：

- 尽量在完整 user/assistant turn 边界切分；
- 不把 `ToolResult` 直接作为切点；
- 不把 session metadata entry 当作可独立切点；
- 如果切点落在一个 turn 内，识别 turn 起点并启用 split-turn 处理；
- 没有可摘要消息时不发起 LLM 调用，也不追加空 compaction entry。

对于 split turn，旧历史和被截断 turn 的前缀分别生成摘要，最终摘要中包含 `Turn Context (split turn)` 部分，以便保留当前后缀所需的上下文。

### 摘要生成

摘要由当前 provider 通过无 tools 的单独请求生成。普通摘要要求模型输出结构化内容，至少覆盖：

- Goal；
- Constraints & Preferences；
- Progress，包括 Done、In Progress、Blocked；
- Key Decisions；
- Next Steps；
- Critical Context。

已有 summary 时，模型收到 `<previous-summary>`，执行增量更新而不是从头丢弃旧摘要。摘要提示要求保留准确文件路径、函数名和错误信息。

空摘要会被拒绝，不会写入 compaction entry。provider 错误、流结束但没有 Done、Aborted 或 Error stop reason 也不会产生有效压缩结果。

### Session 持久化与 provider replay

压缩不会物理删除旧 session entries。成功压缩只会追加一个 `SessionEntry::Compaction`，其中保存：

- summary；
- `first_kept_entry_id`；
- `tokens_before`；
- 文件读写追踪 details；
- 是否来自 extension hook 的标记。

构造当前 provider context 时，`Session::to_messages_for_current_path()` 查找最新 compaction entry，并生成：

```text
compaction summary
+ first_kept_entry_id 对应的保留消息
+ 之后的当前 branch 消息
```

旧消息仍可在 session JSONL 中查询、导出或恢复，但不会再次发送给 provider。summary 会被包装成 user message，使用固定的 compaction summary 标记包围。

如果 `first_kept_entry_id` 不存在，replay 必须 fail-safe：保留 compaction summary，并从该 compaction entry 之后的消息继续，而不是恢复全部旧历史。

### 运行时路径

#### Agent 主路径

`AgentSession` 的正常 prompt/continue 路径在构造 provider history 前调用 compaction 检查。其后台流程是两阶段的：

1. 先非阻塞接收并应用已完成的后台 compaction result；
2. 在 quota 允许且当前 path 需要压缩时，启动新的后台 compaction task；
3. 后续 turn 再应用该 task 的结果。

应用结果时追加 compaction entry、计算 `tokensAfter`、刷新 Agent message history，并发送 `auto_compaction_end`。

#### 手动交互式 `/compact`

交互式命令同步等待压缩完成。它会先检查 Agent 是否 idle，然后执行准备、`session_before_compact`、摘要生成、entry 写入、session 保存和 Agent conversation reset。

#### RPC `compact`

RPC 命令同步完成压缩并返回 summary、保留 entry ID、`tokensBefore`、`tokensAfter` 和 details。RPC 可以提供临时 `customInstructions`、`reserveTokens` 和 `keepRecentTokens` 覆盖值。

RPC 还可以在成功 agent run 结束后执行独立的 auto-compaction 路径。该路径与 Agent 主路径的后台 worker 不同，是 RPC 层同步执行的。

### 后台 worker 约束

Agent 主路径的 `CompactionWorkerState` 以 session 为单位管理后台任务。默认配额包括：

- 两次启动之间至少冷却 60 秒；
- 单次后台等待最多 120 秒；
- 单 session 最多尝试 100 次。

同一 session 已有 pending task、超过尝试次数、仍在 cooldown 或准备阶段没有可压缩内容时，不启动新任务。worker 支持 abort；超时、abort 或 worker panic 只报告失败，不写入部分结果。

worker 接收调用方提供的 memory/provider/queue admission signals，但自身不探测主机或 provider 状态。当前 Agent 正常路径使用默认 signal；若未来接入 admission signals，必须保持拒绝理由可解释且 fail-closed。

## 扩展与事件边界

压缩前可通过 `session_before_compact` 取消或提供替代 compaction result。压缩完成并写入 session 后发送 `session_compact`。

扩展提供的替代结果必须仍然满足 session replay 所需的 summary、`first_kept_entry_id` 和 `tokens_before` 语义。来自 hook 的 entry 会保留来源标记，便于后续区分 Pi 生成的摘要和扩展生成的摘要。

生命周期事件包括：

- `auto_compaction_start`；
- `auto_compaction_end`；
- `session_before_compact`；
- `session_compact`。

取消压缩时不得追加空或不完整的 compaction entry；事件应报告 aborted/cancelled 状态。

## `tokensAfter` 语义

压缩成功追加 entry 后，系统按照真实 current-path replay 规则重新计算下一次 provider request 的 session-message 估算：

```text
最新 compaction summary
+ first_kept_entry_id 之后可 replay 的消息
```

`tokensAfter` 用于 Agent、RPC、SDK 和自动压缩事件的结果展示。它是本地启发式估算，不写入 compaction entry，也不替代 provider usage 或账户 quota。

## 已知边界与排错路由

- **压缩没有触发**：先查 `compaction.enabled`、当前模型 context window、最近 assistant usage/fallback 估算和 `reserve_tokens`；再确认当前 branch 是否已有最新 compaction entry。
- **保留内容不符合预期**：先查 `keep_recent_tokens`、合法切点、tool-result 链和 split-turn 判断；再查 `first_kept_entry_id` 是否仍存在于当前 path。
- **压缩后 provider 仍看到旧历史**：查 `Session::to_messages_for_current_path()` 的最新 compaction replay 逻辑和 Agent/RPC 是否刷新了当前 message history。
- **压缩后上下文估算异常**：区分 provider usage、session message chars/3 估算和 `tokensAfter` replay 估算；不要把三个值当作同一个指标。
- **后台压缩一直未完成**：查 worker 的 pending、cooldown、timeout、attempt limit 和 runtime ownership；不要在 RPC event handler 中新增第二个 session writer。
- **摘要请求失败**：检查 provider key、stream Done、summary 是否为空，以及 compaction 链路是否需要独立处理，而不是套用普通 turn retry。
- **RPC 与 Agent 行为在阈值边界不同**：Agent 主路径使用 `>=`，RPC auto-compaction helper 使用 `>`；修改前必须同时检查两条路径及其测试。

## 修改约束

- 不要把 compaction 改成直接删除 session 历史；session entry 是 replay、分支和持久化的一部分。
- 不要复制 token 估算算法到 parent、subagent 或 RPC 新入口；复用 `src/compaction.rs` 的估算语义，并明确 estimated/reported 差异。
- 不要绕过 Session 的 autosave、sidecar lock、索引更新和 Windows 占锁重试，在 RPC 或事件 handler 中直接写 JSONL。
- 不要让 recovery continuation 在 Agent loop 内主动触发新的 compaction；context overflow 应沿既有 provider error/retry/final-error 路径处理。
- 修改 cut point 时必须同时验证 tool chain、split turn、branch path、重复 compaction 和缺失保留 ID 的 fallback。
- 修改后台生命周期时必须验证取消、超时、worker drop、session shutdown 和结果应用的所有权边界。

## 事实状态

- 当前实现：已确认，以上内容基于 `src/compaction.rs`、`src/compaction_worker.rs`、`src/agent.rs`、`src/session.rs`、`src/rpc.rs`、`src/interactive/perf.rs` 和对应模块测试。
- provider 精确 tokenizer 与 system/tool schema 的完整 token 覆盖：当前未知，不应从 `tokensBefore` 或 `tokensAfter` 推断。
- RPC auto-compaction 与 Agent 后台 compaction 的统一生命周期语义：当前不完全统一，修改任一路径时必须分别验证。
