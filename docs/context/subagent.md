# Subagent 工具

> 触发调度、并行任务、chain、`continue` 会话续跑、worktree 隔离、结构化输出校验或子 Agent 失败诊断时阅读本页。

## 定位

- **实现入口** — `src/subagents.rs` 的原生 `SubagentTool`，通过 `ToolRegistry` 以 `subagent` 名称注册；工具效果类型为进程操作。
- **协同模块** — `src/agent_hub.rs` 管理子进程登记、状态和会话入口；`src/worktree_iso.rs` 管理 worktree 隔离、补丁收集与合入；`src/resources.rs` 负责技能资源和白名单过滤。
- **功能清单** — 功能概览见 `docs/context/features.md` 的“子代理与后台任务”章节；本页只承载深度契约和排障信息。

## 架构与生命周期

- **子进程模型** — 工具派生当前 `pi` 可执行文件（可由 `PI_SUBAGENT_PI_BINARY` 覆盖），以 JSON print 模式运行；stdin 关闭，stdout/stderr 分别通过管道读取。子进程由生命周期 guard 管理，避免父工具取消后留下孤儿进程。
- **Agent 定义发现** — 默认同时发现全局 `$PI_CODING_AGENT_DIR/agents/*.md` 和项目最近的 `.pi/agents/*.md`；同名项目定义覆盖全局定义。`scope` 可切换为 `both`、`user` 或 `project`。
- **SYSTEM.md 隔离** — Subagent 以显式 `--prompt-scope subagent` 启动，不自动加载用户级 `~/.pi/agent/SYSTEM.md` 或项目级 `<cwd>/.pi/SYSTEM.md`；这些文件仅属于 Main Agent 的系统提示词覆盖。Subagent 仍加载共享 `AGENTS.md`/`CLAUDE.md` 上下文，并接收 Agent 定义角色提示词与 schema 指令。该隔离是 prompt 自动注入边界，不是文件系统权限隔离。
- **工具继承** — Agent 定义显式声明 `tools` 时使用该列表，否则继承父 Agent 当前启用的工具；两者都没有时使用默认子工具列表。默认列表不包含 `subagent`，并通过最大深度限制阻止无限递归。
- **任务分发** — 单任务直接执行；`tasks` 使用受限并发执行后恢复输入顺序；`chain` 按顺序执行，前一步失败即停止后续步骤。
- **结果聚合** — 子进程 stdout 事件和 stderr 汇总为 `SubagentResult`，再生成工具的 `content` 与 `details.results`。可选结构化结果块会附加受限大小的 JSON，不能替代标准 `details`。

## 调用契约

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

- **持久会话** — 新任务会分配全局唯一 hubId，并以 `<global>/sessions/subagents/<hubId>.jsonl` 保存会话；规范关系是 `hubId == sessionId == worktreeId`。新任务产生的 hubId 会随工具结果返回，供后续 `continue` 使用。
- **会话回退** — 正常路径使用 `--session <path>`；部分内部调用仍可能在没有会话路径时回退到 `--no-session`，不要把所有子任务都假设为可续跑。
- **worktree 模式** — `isolation: "worktree"` 创建或重开隔离工作树；`isoApply` 可选 `keep`、`apply` 或 `drop`，默认按 `apply` 处理。
- **失败保护** — 失败或取消的子任务不会自动把半成品 apply 到父工作区；请求 apply 时会降级为 keep。补丁冲突会报告冲突文件并保留 worktree，绝不强制合入。
- **非隔离并行** — `isolation: "none"` 的任务共享父工作区。并行任务如果可能触碰同一文件，调用方必须自行协调，不能依赖 subagent 自动解决冲突。

## 输出与错误契约

- **正常输出** — `content` 展示子 Agent 的最终 `output`；普通成功 stderr 不默认混入模型可见文本。
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
