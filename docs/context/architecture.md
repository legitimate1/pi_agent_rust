# 架构骨架

## 核心数据流

```
CLI (clap) → main/app/config/resources → Agent Session
                         ↓
Provider Layer (native provider modules + extension providers)
                         ↓
Tool Registry (built-ins + extension tools) ↔ Extension Runtime (QuickJS + capability policy)
                         ↓
Surfaces: Interactive TUI + RPC/stdin modes
                         ↓
Session persistence + index (JSONL, optional SQLite)
    • RPC 模式: 额外有 RpcSessionPersister 背景线程实时追加写入
       (TurnEnd/ToolResult 等消息无需等 turn 结束即可落盘)
```

## 工具系统架构

```
                    ┌─────────────────────┐
                    │   ToolRegistry       │
                    │   Vec<Box<dyn Tool>> │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                 ▼
    ┌─────────────────┐ ┌──────────────┐ ┌──────────────────┐
    │ Built-in tools   │ │ Extension    │ │ Collection point │
    │ (read/write/...) │ │ tool wrappers│ │ (agent.rs)       │
    │ tools/mod.rs     │ │ extension_   │ │ extend_tools()   │
    │                  │ │ tools.rs     │ │ → dedup by name  │
    └─────────────────┘ └──────────────┘ │ → replace builtin│
                                         │ → append new     │
                                         └──────────────────┘

    ┌──────────────────────────────────────────────┐
    │  ProcessGuard 子进程生命周期管理              │
    │  (tools/mod.rs)                              │
    │                                              │
    │  spawn_managed() → 便捷构造器                │
    │  wait_with_cancellation() → 标准 wait 循环    │
    │    • ambient cancellation (cx.checkpoint)     │
    │    • abort 信号检查 (signal.is_aborted)       │
    │    • 超时 kill                                │
    │  Drop → spawn 线程 kill + wait 回收子进程     │
    │                                              │
    │  被 pwsh / bash / grep / find 使用            │
    │  全部使用 ProcessGroupTree + isolate 进程组   │
    └──────────────────────────────────────────────┘
```

abort 信号（`src/abort.rs` 共享原语）沿 Agent 循环 → 工具执行 → 子进程全链路传播：Rust 工具循环检查后 kill 进程树；扩展工具先通知 JS 侧 `AbortController` 优雅退出，仍挂起则硬中断兜底。

## 扩展加载流程

1. `main.rs` → 获取默认工具列表
2. 过滤 `disabledTools` 配置中列出的工具（如 `bash`）
3. 创建 `ToolRegistry` 内置工具（`tool_config_with_overrides` 合并 settings.json `toolDescriptions` + `tools.toml` 覆盖，tools.toml 优先）
4. `enable_extensions_with_policy()` 按能力策略加载扩展
5. 收集扩展工具包装器
6. `extend_tools()` → **同名扩展工具覆盖内置工具**

## 模块关系

- **`abort.rs`** → 共享 AbortHandle/AbortSignal 原语，打破 agent ↔ tools 循环依赖
- **`app.rs`** → 系统提示词构建（SYSTEM.md 加载、default_system_prompt、project context files、date/cwd/temp 运行时事实注入）
- **`tool_overrides.rs`** → tools.toml 覆盖加载：合并用户级 `~/.pi/agent/tools.toml` + 项目级 `.pi/tools.toml`（项目 key 优先），产出 description / parameters 覆盖表
- **`tools/` 模块目录** → ToolRegistry + 内置工具模块 + verify 内部验证引擎
- **`tools/verify.rs`** → 编辑后轻量验证引擎：文件类型检测→检查器映射（.rs/.json/.toml/.ts/.js/.py/.go/.md）→进程内/外部进程执行，支持 Format+Lint 并行（oxfmt+oxlint/ruff），诊断直写 content 正文
- **`agent.rs`** → Agent 循环（工具迭代、扩展合并、ToolDef 构建）
- **`extensions.rs`** → 扩展管理器、能力策略、生命周期
- **`extensions_js.rs`** → QuickJS 运行时、虚拟模块、HostcallKind
- **`hostcall_amac.rs`** → AMAC 批量调度器：hostcall 按类型分组，遥测驱动并发/串行决策
- **`extension_tools.rs`** → 扩展工具包装器 + 收集函数
- **`rpc.rs`** → RPC/stdin 服务器模式、RPC 方法分发、RpcSessionPersister（进程侧主动会话持久化）
- **`providers/mod.rs`** → Provider 工厂 + 扩展 stream-simple 桥接
- **`providers/`** → native provider 实现：anthropic/openai/openai_responses/gemini/cohere/azure/bedrock/vertex/copilot/gitlab/cursor/model_fetch
- **`provider_metadata.rs`** → Provider 元数据：别名、认证键、本地 provider（ollama/llamacpp/mistralrs/lmstudio）
- **`auth.rs`** → 凭据管理：API Key / OAuth / AWS / Bearer，auth.json 文件锁
- **`models.rs`** → 内置 + models.json 模型注册表
- **`session.rs`** → JSONL 会话持久化（+ SQLite 后端）
- **`session_store_v2.rs`** → sidecar 分段日志：偏移索引 + 检查点回滚
- **`autocomplete.rs`** → `@` 文件引用 + `/` 命令补全索引（WalkBuilder + 后台刷新）
- **`resources.rs`** → Skills / prompt templates / themes / extensions 资源加载
- **`package_manager.rs`** → 包安装/移除/更新（pi install 等）
- **`doctor.rs`** → 环境诊断（config/dirs/auth/sessions/swarm preflight）

## 运行时不变量

1. **Turn 作用域的 agent 生命周期** — 主循环按稳定顺序发 `AgentStart → TurnStart → TurnEnd → AgentEnd`；工具递归由 `max_tool_iterations`（默认 50）封顶
2. **Abort/超时行为显式化** — abort 检查在 turn 边界与工具执行处；bash 超时走升级路径（终止进程树 → 宽限期 → 硬杀）
3. **会话写入 crash-resilient** — JSONL 保存经临时文件 + 原子 persist；会话索引用 SQLite WAL + 锁文件协调多实例
4. **Compaction 阈值驱动、边界感知** — 触发条件为估算 token 超 `context_window - reserve_tokens`；cut-point 优先 user turn 边界，保留近期上下文预算
5. **能力策略 fail-closed、优先级明确** — 解析顺序：per-extension deny → 全局 deny → per-extension allow → 默认 caps → 模式回退
6. **流式解析器容忍真实网络分块** — CR/LF 变体、多行 `data:`、UTF-8 部分尾部、EOF flush

## Extension Hostcall 协议

扩展（QuickJS 沙箱）通过 hostcall 请求与宿主通信，`ExtensionDispatcher` 检查能力策略后分派到 ToolRegistry / HTTP / session 等：

- **`pi.tool(...)`** — read/write/exec 等；read/write 拒绝，exec 允许
- **`pi.http(request)`** — http；拒绝
- **`pi.exec(cmd, args)`** — exec；允许
- **`pi.env(key)`** — env；允许
- **`pi.session(op, ...)`** — session；拒绝
- **`pi.ui(op, ...)`** — ui；拒绝
- **`pi.log(entry)`** — log；始终允许

关键机制：

- **能力门控**：所有 hostcall 经能力策略 + 审计日志；exec 再经命令级调解（默认阻断递归删除/磁盘设备写入/反弹 shell 等危险签名）
- **批量调度（AMAC）**：hostcall 按类型分组，stall 遥测驱动并发/串行决策；exec 组冷启动即并发（秒级阻塞收益确定），逃生开关与环境变量见 `commands.md`「AMAC hostcall 调度」
- **Trust 生命周期**：`pending → acknowledged → trusted → killed`；kill switch 隔离扩展并写审计记录

## Compaction 算法

```
全量对话 → 找合法 turn 边界 cut point → LLM 摘要旧消息
        → 存 Compaction entry 到 JSONL → 下次调用用 [summary] + 近期消息
```

- 触发：每 agent turn 后，估算 token 超 `context_window - reserve_tokens`
- Cut point：优先完整 user-assistant turn 边界；被迫中途切时包含前缀消息保上下文连贯
- 手动触发：`/compact`（交互）或 RPC `compact`

## 会话索引 + Sidecar 存储

**SQLite 索引**（`session-index.sqlite`）：会话元数据表（path/id/cwd/timestamp/message_count/last_modified/size_bytes），保存后 upsert；`pi -c` 按 `cwd + last_modified DESC` 查询；WAL + 锁文件串行化并发写入；索引过期触发全量重扫。

**Sidecar 存储**（`session_store_v2.rs`）：分段追加日志 + 偏移索引行（直接 seek/快速尾读）+ 周期检查点 + 迁移台账。恢复路径：sidecar 新鲜 → 从索引+分段打开；过期 → 回退 JSONL；索引损坏但分段有效 → 重建。完整性：帧带 payload+chain hash，索引行 CRC32C，截断尾帧可恢复，非 EOF 帧损坏 fail-closed。`pi migrate` 做 JSONL→sidecar 迁移。

## TUI 架构（交互模式）

Elm 架构（Model-Update-View），`crossterm` + `bubbletea` 家族：

```
Terminal (crossterm raw mode / alt screen)
   ↓
bubbletea Program Loop (Init → Update(Msg) → View)
   ↓
PiApp (Model) ── TextArea(editor) + Viewport(convo) + Spinner(status)
   ↓  overlay 栈：Model Selector / Session Picker / /tree / Settings / Capability Prompt
   ↓  mpsc 异步通道
Agent Async Task（asupersync 运行时，流式 provider 响应 + 执行工具）
```

- 异步/同步桥：agent 在独立线程跑 asupersync 运行时，经 mpsc 把每个流事件（text delta/tool start/tool update/agent done）作为 `PiMsg` 投递到 `PiApp::update()`
- 视口自动跟随：用户未上滚时跟随流尾部；上滚禁用，按 `End` 或发消息恢复
- 渲染缓存：按消息缓存 markdown→ANSI 渲染，仅主题/尺寸变化失效；内存压力三级（Normal/Pressure/Critical 渐进降级）

## SSE 解析器

自研状态机（`src/sse.rs`）：`Bytes → Line Accumulator → Event Parser → Typed StreamEvent`。

- 事件变体覆盖 MessageStart/ContentBlockStart/Delta/Stop/MessageDelta/MessageStop/Ping/Error + thinking 事件
- 处理多行 `data:`、TCP 分块跨界事件、`event:` 在 `data:` 前后、CRLF/LF 混用、UTF-8 部分尾部缓冲
- 错误事件记录不崩流；固定大小滚动缓冲防无界增长

## 认证与凭据管理

凭据存 `~/.pi/agent/auth.json`（文件锁防并发损坏）。存储值可为字面量、`$ENV:VAR` 引用、`$CMD:shell command`（请求时解析 trimmed stdout）。

- **API Key**：Anthropic/OpenAI/Gemini/Cohere 等；环境变量或 settings
- **OAuth**：Anthropic/OpenAI Codex/Gemini CLI/Antigravity/Kimi/Copilot/GitLab/扩展定义；PKCE + 自动刷新；Kimi 用 device flow
- **AWS 凭据**：Bedrock；access key + secret + 可选 session token
- **Service Key**：SAP AI Core；client id/secret 换 bearer
- **Bearer Token**：自定义 provider；静态 token

`pi config` 报告各 provider 凭据状态：`Missing` / `ApiKey` / `OAuthValid`（含过期倒计时）/ `OAuthExpired` / `AwsCredentials` / `BearerToken`。认证失败返回机器可读诊断码（MissingApiKey/InvalidApiKey/QuotaExceeded/OAuthTokenRefreshFailed 等）。
