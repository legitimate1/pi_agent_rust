# Subagent 工具

> 触发调度、并行任务、chain、`continue` 会话续跑、worktree 隔离、结构化输出校验或子 Agent 失败诊断时阅读本页。

## 定位

- **实现入口** — `src/subagents.rs` 的原生 `SubagentTool`，通过 `ToolRegistry` 以 `subagent` 名称注册；工具效果类型为进程操作。
- **协同模块** — `src/agent_hub.rs` 管理子进程登记、状态和会话入口；`src/worktree_iso.rs` 管理 worktree 隔离、补丁收集与合入；`src/resources.rs` 负责技能资源和白名单过滤。
- **功能清单** — 功能概览见 `docs/context/features.md` 的“子代理与后台任务”章节；本页只承载深度契约和排障信息。

## 架构与生命周期

- **子进程模型** — 工具派生当前 `pi` 可执行文件（可由 `PI_SUBAGENT_PI_BINARY` 覆盖），以 `--mode json --print` 运行；stdin 关闭，stdout/stderr 分别通过管道读取。JSON stdout 首行可能是 session header，后续为逐行序列化的 `AgentEvent`。子进程由生命周期 guard 管理，避免父工具取消后留下孤儿进程。
- **Agent 定义发现** — 默认同时发现全局 `$PI_CODING_AGENT_DIR/agents/*.md` 和项目最近的 `.pi/agents/*.md`；同名项目定义覆盖全局定义。`scope` 可切换为 `both`、`user` 或 `project`。
- **SYSTEM.md 隔离** — Subagent 以显式 `--prompt-scope subagent` 启动，不自动加载用户级 `~/.pi/agent/SYSTEM.md` 或项目级 `<cwd>/.pi/SYSTEM.md`；这些文件仅属于 Main Agent 的系统提示词覆盖。Subagent 仍加载共享 `AGENTS.md`/`CLAUDE.md` 上下文，并接收 Agent 定义角色提示词与 schema 指令。该隔离是 prompt 自动注入边界，不是文件系统权限隔离。
- **工具继承** — Agent 定义显式声明 `tools` 时使用该列表，否则继承父 Agent 当前启用的工具；两者都没有时使用默认子工具列表。默认列表不包含 `subagent`，并通过最大深度限制阻止无限递归。
- **任务分发** — 单任务直接执行；`tasks` 使用受限并发执行后恢复输入顺序；`chain` 按顺序执行，前一步失败即停止后续步骤。parallel 的恢复顺序来自 `tasks.into_iter().enumerate()` 的 0-based slot；该 slot 当前尚未进入 result/details。
- **结果聚合** — 子进程 stdout 事件和 stderr 汇总为 `SubagentResult`，再生成工具的 `content` 与 `details.results`。可选结构化结果块会附加受限大小的 JSON，不能替代标准 `details`。当前 `SubagentResult`/progress/result 不产生或透传 `contextUsage`、`index` 或 `revision`。

## Child print-mode 事件边界

- **stdout 生成** — `src/main.rs:6581-6735` 的 `run_print_mode()` 在 JSON 模式先输出 session header，随后对每个 `AgentEvent` 执行 `serde_json::to_string()` 并逐行写入 stdout。父侧不要假设 stdout 只有最终文本。
- **回合结束事件** — `turn_end` 已存在，由 `src/agent.rs:968-1006` 的 `AgentEvent::TurnEnd` 序列化。事件顶层包含 `type`、`sessionId`、`turnIndex`、`message`、`toolResults`，以及可选 `latencyBreakdown` 和 `touchedFiles`。`message` 是 `Message::Assistant` 时包含 provider `usage`。
- **agent_end** — `src/agent.rs:977-984` 的 `AgentEvent::AgentEnd` 顶层包含 `type`、`sessionId`、`messages` 和错误时的可选 `error`。`messages` 是本次 agent run 新增消息，不是完整 session prompt。
- **事件顺序** — 正常 tool loop 可以有多个 `turn_end`；最终 `agent_end` 在最后一个 turn 结束之后发送。provider/stream setup 错误和 abort 路径也会先发送 `turn_end` 再发送 `agent_end`，但对应 assistant message 的 `stopReason` 可能是 `error` 或 `aborted`，不能当作有效成功快照。
- **父侧当前消费范围** — `src/subagents.rs:2295-2332` 只处理 `message_update` 的文本 delta、`message_end` 的 assistant 文本和 `agent_end` 的文本兜底；`turn_end`、session header、retry、compaction 等未知事件当前忽略。当前 `agent_end` 不读取 usage/error，也不单独触发 progress。
- **当前协议状态** — `SubagentResult`、progress details 和最终 result details 当前没有 `contextUsage`、`index` 或 `revision`。新增字段只能作为未来的可选 additive 扩展，不能在本页描述为当前已实现能力。

## 上下文来源与快照语义

- **实际 request Context** — `src/agent.rs:2111-2133` 的 `stream_assistant_response()` 在调用 `provider.stream()` 前构造完整 `Context`。此时能看到当前 system prompt、经过过滤/注入后的 messages 和 tool definitions；`src/provider.rs:54-72` 定义了这三个字段。
- **context window 所属层** — `Context` 和 `Provider` trait 不包含 context window。实际模型窗口来自 `ModelEntry.model.context_window`，由外层 `AgentSession` 的 compaction settings 保存；CLI 初始化和重选模型时见 `src/main.rs:268-278,1481-1493,1694-1713`，SDK 初始化见 `src/sdk.rs:1825-1842`。父侧不得从 Agent definition 的 `model` 字符串推断 child 实际窗口。
- **0 窗口 fallback** — 当前 CLI/SDK 在 model entry 的 `context_window == 0` 时使用 `ResolvedCompactionSettings::default().context_window_tokens`；代码和测试实际值为 128000（`src/compaction.rs:70-88,2866-2874`）。注释中的旧值若与代码冲突，以代码/测试为准。
- **provider usage** — `Usage` 在 `src/model.rs:216-226` 使用 `input`、`output`、`cacheRead`、`cacheWrite`、`totalTokens`。`totalTokens` 不能直接当 prompt/context tokens，因为通常包含 output。
  - OpenAI-compatible adapter 在 `src/providers/openai.rs:850-870` 将 `input` 设为未缓存 prompt，`cacheRead` 单独保存；有完整 cache details 时 `input + cacheRead` 重建 provider 的 prompt token 数。
  - Anthropic adapter 在 `src/providers/anthropic.rs:863-873,1217-1228` 分别保存 input、cache read/write，并在 `src/providers/anthropic.rs:792-800` 将 output 也加入 total；prompt-side usage 应排除 output，但可以包含已报告 cache read/write。
  - Cohere 当前主要映射 input/output/total（`src/providers/cohere.rs:547-552`），本仓库没有跨 provider 的完整 prompt coverage 认证；不能仅凭非零 usage 标记为统一 reported。
- **现有估算** — `estimate_context_tokens()` 和 `estimate_post_compaction_context_tokens()` 位于 `src/compaction.rs:747-819,958-1009`。后者按 current-path/provider replay 语义处理 compaction summary、branch/bash 和旧历史；两者都不包含独立的 system prompt/tool schema token，也不是 provider 精确 tokenizer 结果。父侧不得复制这些算法。
- **可诚实的跨项目定义** — 若未来增加 `contextUsage`，`reported` 只能表示已认证 adapter 的 prompt-side usage（建议为 `input + cacheRead + cacheWrite`，不含 output）；否则使用 `estimated`，并明确它是基于 current-path/provider replay 的运行时估算，不代表完整 prompt，也不包含 system/tool token。

## 快照事件与 revision 实现边界（当前未实现）

- **推荐事件落点** — 若实现跨项目快照，建议把可选 `contextUsage` 放在 `turn_end`/`agent_end` 的 event 顶层，而不是放入 `message`。`message` 保持 provider assistant-message schema；event 顶层承载 runtime turn metadata。`agent_end` 只应复用最后一个有效 turn snapshot，不重新计算、不递增；没有有效 turn snapshot 时省略，而不是从 `messages` 猜测。
- **最小 child 字段** — child 侧最小应保证 `tokens`、`contextWindow`、`source`、`revision`；`percent` 是 Pidian 根据 tokens/window 派生的 UI 字段，`updatedAt` 当前没有 turn_end wall-clock 约定，第一阶段不应假设存在。parallel `index` 来自 parent 的 `tasks.into_iter().enumerate()`，不属于 child event。
- **turnIndex 不是 revision** — `Agent::run_loop()` 在 `src/agent.rs:1545-1564` 每次从局部 0 开始，并在正常 turn 完成后递增（`src/agent.rs:1956-1968`）。provider retry 通过 `revert_incomplete_response` + `run_continue_with_abort` 重新进入 run loop（`src/main.rs:6984-6999`、`src/agent.rs:11076-11120`），会重新从局部 0 开始；错误/abort turn 不能覆盖最后合法快照。
- **continue/retry 持久化边界** — continue 复用 session messages，但 `SessionHeader` 没有 turn index/revision/context snapshot 字段（`src/session.rs:4093-4123`），session entries 也不持久化 turn index。因此当前只能可靠定义“单个 child process invocation 内的有效 snapshot sequence 单调”；跨 continue 的全局 revision 需要新增持久 metadata。
- **schema corrective retry** — schema 纠正重跑是新的 child process（`src/subagents.rs:1188-1203`），当前没有跨 process attempt/revision 状态。默认应将每个 child process 视为独立 revision sequence，并由 parent 在同一次 execute 内只透传最终有效 result；若要求跨 process 单调，需要额外设计 parent/session/hub 的 sequence owner。
- **dynamic model switch 风险** — `AgentSession::set_provider_model()` 的 provider/model 切换路径（`src/agent.rs:9168-9247,9401-9450`）没有同步调用 `set_compaction_context_window()`；startup/reselection 路径才显式更新 window。因此动态切换后的 compaction window 同步当前不能保证，parent 不得自行修正或猜测。

> 当前没有 child-side context snapshot producer、`contextUsage` JSON 字段或 revision counter。上面的快照字段和 source 规则是已确认实现边界内的推荐契约，不是现有运行时输出。

## 相关测试与验证入口

- AgentEvent 的 turn 顺序、多个 tool turn 和错误先 turn_end 后 agent_end：`src/agent.rs:8438-8592,8731-8795`。
- JSON print mode 与 retry event：`src/main.rs:6581-6735,6880-7067,8384-8392`。
- parent child-process fixture、agent_end 文本兜底、provider failure、cancel、schema retry：`src/subagents.rs:3112-3166,3170-3236,3271-3358,3414-3529`。
- compaction estimate 与 current-path replay：`src/compaction.rs:2504-2735`；session compaction/current path：`src/session.rs:3479-3529,8791-8854`。
- provider usage/cache mapping：`src/providers/openai.rs:3204-3261`、`src/providers/anthropic.rs:1887-1945`、`src/providers/cohere.rs:547-552`。

当前没有测试覆盖 child `turn_end` 的 context snapshot、context-only progress、agent_end snapshot fallback、parallel index 透传或 continue/schema retry 的跨 process revision；实现这些能力时必须补充定向 fixture 和回归测试。


- **三种模式互斥** — 请求必须且只能选择以下一种：
  - `agent` + `task`：单个 Agent 任务；
  - `tasks`：多个独立任务；
  - `chain`：多个串行任务。
- **并行上限** — `tasks`/`chain` 最多 8 项；`concurrency` 默认 4，并限制在 1 到 8 之间。
- **单任务字段** — 支持 `cwd`、`isolation`、`isoApply`、`outputSchema`、`schemaMode`、`continue` 和 `hubId`。顶层单任务参数与任务数组中的字段遵循相同语义。
- **chain 前序引用** — `{previous}` 引用上一步原始文本；`{{previous.data.path}}` 只在上一步通过 `outputSchema` 校验并产生结构化 `data` 时解析。字段不存在时保留原引用文本。
- **会话续跑** — `continue=true` 只允许单个 `agent` + `task`，必须同时提供非空 `hubId`；不能与 `tasks` 或 `chain` 组合。续跑复用同一会话和 hub/worktree 标识，并把新任务追加到该会话。
- **hubId 安全性** — hubId 必须是非空安全标识，不能包含路径分隔符、空字节，也不能是 `.` 或 `..`。

## 会话与工作区隔离

- **持久会话** — 新任务会分配全局唯一 hubId，并以 `<global>/sessions/subagents/<hubId>.jsonl` 保存会话；规范关系是 `hubId == sessionId == worktreeId`。新任务产生的 hubId 会同时出现在模型可见的 `content` 文本和结构化 `details.hubId` 中，供后续 `continue` 使用；续跑会复用并返回同一个 hubId。
- **会话回退** — 正常路径使用 `--session <path>`；部分内部调用仍可能在没有会话路径时回退到 `--no-session`，不要把所有子任务都假设为可续跑。
- **worktree 模式** — `isolation: "worktree"` 创建或重开隔离工作树；`isoApply` 可选 `keep`、`apply` 或 `drop`，默认按 `apply` 处理。
- **失败保护** — 失败或取消的子任务不会自动把半成品 apply 到父工作区；请求 apply 时会降级为 keep。补丁冲突会报告冲突文件并保留 worktree，绝不强制合入。
- **非隔离并行** — `isolation: "none"` 的任务共享父工作区。并行任务如果可能触碰同一文件，调用方必须自行协调，不能依赖 subagent 自动解决冲突。

## 输出与错误契约

- **正常输出** — `content` 展示子 Agent 的最终 `output`，并在存在持久会话时返回 `hubId: <id>` 及续跑提示；普通成功 stderr 不默认混入模型可见文本。
- **失败输出** — 失败时 `content` 会展示已有部分 output，并追加 `error` 与 `stderr`。API 超时、认证失败、provider 错误等通常位于 stderr，因此排查失败时先看 `content` 中的 `Error:` 和 `Stderr:` 段。
- **结构化结果** — `details` 包含 `schema`、`mode`、`sessionIsolation`、`hubId` 和 `results`。每个 result 还可能包含 `status`、`exitCode`、`output`、`stderr`、`error`、`data`、schema 校验结果和 worktree 结果。
- **状态** — 子任务状态包括 `starting`、`running`、`completed`、`failed` 和 `cancelled`；非零退出码会标记为 failed，父取消会标记为 cancelled。
- **工具错误与子任务错误** — 参数反序列化、模式冲突、续跑契约错误等属于工具调用错误；子进程 API/退出/隔离失败属于结果中的子任务错误。两者都必须回到 Main Agent，但承载形式不同。

## Schema 与重试

- **Schema 来源** — 任务级 `outputSchema` 优先于 Agent 定义中的 schema；无法编译的 schema 会在启动子进程前失败。
- **纠正重跑** — 子 Agent 输出未通过 schema 时最多执行一次带校验反馈的纠正重跑。`schemaMode: "permissive"` 保留结果并标注校验失败；`strict` 将仍失败的结果标记为工具错误。
- **Provider 网络重试** — 子进程使用 `pi --mode json --print`；瞬时 provider/API 错误由子进程内部 Main Agent 的 print-mode retry 处理。重试在同一个 `AgentSession` 内通过 `revert_incomplete_response` + `run_continue_with_abort` 恢复失败请求，不由父进程重新 spawn，也不会重新执行已完成的工具调用。
- **两类重试独立** — provider 网络重试恢复同一 child turn；schema 纠正重跑解决输出格式失败，最多一次且是有意的新 child prompt。不要把 schema 不匹配误判为 API 失败。

## 已知陷阱

- **只看“Child exited with code 1”不够** — 真实 provider 原因通常在 stderr；先查看 `content` 的 `Stderr:`，再查看 `details.results[].stderr`、`error`、`status` 和 `exitCode`。
- **成功退出不等于任务完成** — 子 Agent 以退出码 0 结束但文字表示未完成时，程序只能将其视为 completed，语义判断仍由调用方负责。
- **失败隔离不会自动合入** — 失败任务的 worktree 可能保留在磁盘上；应根据结果中的 `iso.worktreePath`、`patch` 和 `applyMode` 决定是否人工处理。
- **chain 会短路** — 任一步失败都会停止后续步骤；不要假设后续任务仍会收到结果。
- **空技能白名单具有实际约束** — 父级和 Agent 级白名单同时存在时按交集处理；空交集意味着子 Agent 不可见任何允许技能。
- **嵌套调用受限** — 子 Agent 默认没有 `subagent` 工具，且深度达到上限时会被拒绝；需要继续工作时使用 `continue + hubId`，不要通过递归派生绕过限制。

## 扩展指南

- **新增 Agent 定义** — 在 `$PI_CODING_AGENT_DIR/agents/` 或项目 `.pi/agents/` 下添加 `.md` 文件。必须提供 `name` 和 `description`，可选 `tools`、`model`、`reasoning`/`thinking`、`skills`、`allowed-skills`/`allowed_skills` 和单行 JSON `output_schema`。
- **工具列表** — `tools` 使用逗号分隔；显式列表覆盖继承列表。新增 Agent 不应默认授予 `subagent`，除非明确验证嵌套深度和资源边界。
- **技能路径** — Agent 定义中的相对 `skills` 路径相对于定义文件所在目录解析；技能白名单名称不区分大小写。
- **结构化输出** — 优先在 Agent 定义或任务请求中声明 `outputSchema`，并明确选择 permissive 或 strict；chain 下游只有在 schema 有效时才能读取 `previous.data`。
- **修改工具行为** — 同步更新 `src/subagents.rs` 的单元测试、`docs/context/features.md`、本页契约和必要的 `AGENTS.md` 路由；错误输出、会话标识和 worktree 合入语义变化时必须补回归测试。
