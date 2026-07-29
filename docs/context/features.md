# 功能目录

## CLI 与交互

| 功能 | 状态 | 涉及文件 |
|:-----|:----:|:---------|
| 交互式 TUI（流式渲染、Markdown、主题） | ✅ | `src/interactive.rs` + `src/tui.rs` |
| 非交互式 Print 模式 | ✅ | `src/main.rs` |
| RPC/stdin 服务器模式（含 sessionId 修复） | ✅ | `src/rpc.rs` |
| **RPC 进程侧主动会话持久化（防客户端崩溃丢数据）** | ✅ | `src/rpc.rs` |
| RPC 方法 `estimate_tokens`（会话 token 估算） | ✅ | `src/rpc.rs` `src/compaction.rs` |
| **RPC 方法 `get_commands` — 合并扩展注册的斜杠命令** | ✅ | `src/rpc.rs` `src/extensions.rs` |
| **RPC 方法 `get_tree` — 查询会话树形结构（分支/叶子）** | ✅ | `src/rpc.rs` |
| **RPC 方法 `get_version` — 返回 pi 版本号和 Git SHA** | ✅ | `src/rpc.rs` |
| **RPC 方法 `get_system_prompt` — 返回当前会话的 system prompt 及注册的工具定义** | ✅ | `src/rpc.rs` `src/agent.rs` |
| **`session_state` 返回模型 `compat` 字段** — 客户端可获取 `thinkingLevelMap` 等兼容性配置 | ✅ | `src/rpc.rs` |
| **RPC 方法 `remove_from_queue` — 按 messageId 精确取消队列消息** | ✅ | `src/rpc.rs` |
| **RPC 方法 `clear_queue` — 一键清空 steer/follow_up 队列** | ✅ | `src/rpc.rs` |
| **RPC 方法 `get_queue` — 查询队列当前内容（steering + follow_up）** | ✅ | `src/rpc.rs` |
| **`queue_update` 事件推送 — 队列状态变更时实时同步给客户端** | ✅ | `src/rpc.rs` |
| **队列消息携带 `messageId` — 客户端分配或服务端 UUID fallback** | ✅ | `src/rpc.rs` |
| **set_model/set_thinking_level 支持 persist 参数** — `persist=false` 仅内存切换，不写会话文件 | ✅ | `src/rpc.rs` `src/acp.rs` `src/extension_dispatcher.rs` `src/sdk.rs` `src/session.rs` `src/agent.rs` |
| CLI 子命令（doctor/config/list/info 等） | ✅ | `src/main.rs` |
| **`~/.pi/agent/SYSTEM.md` 覆盖默认系统提示词** | ✅ | `src/app.rs` |
| **`.pi/SYSTEM.md` 项目级系统提示词** — 优先级高于用户级 | ✅ | `src/app.rs` |

## Provider 层

| 功能 | 状态 | 涉及文件 |
|:-----|:----:|:---------|
| Anthropic API（流式 + 扩展思考 + 工具） | ✅ | `src/providers/anthropic.rs` |
| OpenAI Chat Completions（含所有兼容推理模型的 reasoning_effort） | ✅ | `src/providers/openai.rs` |
| OpenAI Responses / Codex Responses | ✅ | `src/providers/openai_responses.rs` |
| Gemini（流式 + 工具） | ✅ | `src/providers/gemini.rs` |
| Cohere（流式 + 工具） | ✅ | `src/providers/cohere.rs` |
| Azure OpenAI | ✅ | `src/providers/azure.rs` |
| Bedrock / Vertex AI / GitHub Copilot / GitLab Duo | ✅ | `src/providers/*.rs` |
| 扩展 stream-simple Provider 桥接 | ✅ | `src/providers/mod.rs` |
| Provider 工厂 + 路由 | ✅ | `src/providers/mod.rs` |
| **Provider 运行时热切换** — `set_model` 跨 provider 切换时直接在内存创建/替换 provider 实例（anthropic ↔ openai ↔ gemini 等），无需重启进程 | ✅ | `src/agent.rs` `src/providers/mod.rs` |
| opencode-go（OpenCode Zen Go，deepseek-v4-flash 等） | ✅ | `src/provider_metadata.rs` `src/app.rs` |

## 工具系统

| 功能 | 状态 | 涉及文件 |
|:-----|:----:|:---------|
| ToolRegistry — 工具注册表 | ✅ | `src/tools/mod.rs` |
| 内置 9 工具（read/bash/pwsh/edit/write/grep/find/ls/hashline） | ✅ | `src/tools/` 各子模块 |
| **进程清理模式统一为 ProcessGroupTree** — 所有 spawn 子进程的工具（bash/pwsh/grep/find）均使用 `isolate_command_process_group` + `taskkill /F /T` 杀整个进程树 | ✅ | `src/tools/bash.rs` `src/tools/pwsh.rs` `src/tools/grep.rs` `src/tools/find.rs` |
| **`Tool::execute()` 支持 abort 信号** — trait 新增 `abort: Option<AbortSignal>` 参数，long-running 工具在循环中检查并主动 kill 子进程 | ✅ | `src/tools/mod.rs` `src/abort.rs` |
| **`ProcessGuard::wait_with_cancellation()` 支持 abort 信号** — 循环中检查外部 abort + cx.checkpoint + 超时 | ✅ | `src/tools/mod.rs:3381` |
| **ReadTool — 无 CWD/agent-dir 路径限制** + head/tail/info/diff 参数 + 编码自动检测 | ✅ | `src/tools/read.rs` |
| **WriteTool / EditTool — 无 CWD 路径限制**（可写入任意绝对路径） | ✅ | `src/tools/write.rs` `src/tools/edit.rs` |
| **EditTool — 直接写入**（非 tempfile 原子重命名，避让 Windows 句柄冲突） | ✅ | `src/tools/edit.rs` |
| **FindTool / GrepTool / LsTool — 无 CWD 路径限制**（可搜索/列出任意绝对路径） | ✅ | `src/tools/find.rs` `src/tools/grep.rs` `src/tools/ls.rs` |
| 扩展工具收集 | ✅ | `src/extension_tools.rs:100` |
| **扩展工具 onUpdate 流式进度推送**（工具执行中 `onUpdate({content, details})`，经 rquickjs Function 桥接到 Rust `ToolUpdate`） | ✅ | `src/extensions.rs` `src/extensions_js.rs` `src/extension_tools.rs` |
| **扩展工具同名覆盖内置工具** | ✅ | `src/tools/mod.rs` `src/agent.rs` |
| **内置 pwsh 工具**（PowerShell 命令执行、尾部截断 2000 行/1MB、exit 0 时自动过滤 stderr） | ✅ | `src/tools/pwsh.rs` |
| **运行时禁用内置工具**（`disabledTools` 配置） | ✅ | `src/config.rs` `src/main.rs` |
| **工具描述外部覆盖**（`toolDescriptions` 配置，免编译修改工具描述） | ✅ | `src/config.rs` `src/tools/mod.rs` `src/agent.rs` |
| **编辑后轻量验证（verify 参数）** — edit/hashline_edit/write 支持可选 verify 参数，编辑后自动运行语法/格式检查（.rs→rustfmt, .json/.toml→进程内解析, .ts→prettier）。结果附在 details.verify，不阻断流程。 | ✅ | `src/tools/verify.rs` `src/tools/edit.rs` `src/tools/hashline.rs` `src/tools/write.rs` |
| **ProcessGuard 子进程生命周期管理** — 统一管理 spawn 子进程的清理：ambient cancellation、超时 kill、abort 信号检查、Drop 自动回收 | ✅ | `src/tools/mod.rs:3337` |
| Tool trait + JSON Schema 定义 | ✅ | `src/tools/mod.rs` |
| **Abort 信号原语** — 共享 `AbortHandle`/`AbortSignal`，打破 agent.rs ↔ tools/mod.rs 循环依赖 | ✅ | `src/abort.rs` |

## 扩展系统

| 功能 | 状态 | 涉及文件 |
|:-----|:----:|:---------|
| JS/TS 扩展加载（QuickJS） | ✅ | `src/extensions_js.rs` |
| **JS Runtime 中断机制** — `InterruptBudget.external_trigger` 外部中断 + QuickJS interrupt hook 停止死循环 | ✅ | `src/extensions_js.rs:4616` |
| **扩展工具 abort 传播** — `await_js_task` 循环检查 abort 信号，触发 runtime 中断 + 返回错误 | ✅ | `src/extensions.rs` `src/extensions_js.rs` |
| Native Rust 扩展（`*.native.json`） | ✅ | `src/extensions.rs` |
| WASM 扩展 | ✅ | `src/extensions.rs` |
| 能力策略模型（Strict/Prompt/Permissive） | ✅ | `src/extensions.rs:1139` |
| Hostcall 调度（tool/exec/http/session/ui/events/log） | ✅ | `src/extensions_js.rs` |
| **RPC 模式扩展交互 UI（select/confirm/input）** | ✅ | `src/extensions.rs` `src/rpc.rs` |
| 虚拟模块系统（Node.js builtins + npm stubs） | ✅ | `src/extensions_js.rs` |
| **VFS write-through 落盘**（`writeFileSync`/`appendFileSync` 写 VFS 内存后同步写真实文件系统） | ✅ | `src/extensions_js.rs` |

## 会话管理

| 功能 | 状态 | 涉及文件 |
|:-----|:----:|:---------|
| JSONL 会话（v3） | ✅ | `src/session.rs` |
| 分支 / 树结构 | ✅ | `src/session.rs` |
| SQLite 会话后端 | ✅ | `src/session.rs` |
| 会话索引元数据 | ✅ | `src/session_index.rs` |

## 模型注册表

| 功能 | 状态 | 涉及文件 |
|:-----|:----:|:---------|
| 内置模型注册 | ✅ | `src/models.rs` |
| `models.json` 自定义模型加载 | ✅ | `src/models.rs:698` |
| Provider 元数据（别名 + 认证键） | ✅ | `src/provider_metadata.rs` |
| **`supports_xhigh()` 检查 `compat.thinkingLevelMap`** — 第三方 provider 在 models.json 中声明 `thinkingLevelMap.xhigh` 即可支持 xhigh，无需硬编码 | ✅ | `src/models.rs:30` |

## 资源与配置管理

| 功能 | 状态 | 涉及文件 |
|:-----|:----:|:---------|
| **`skill_mode: "project_only"` 配置** — 项目可跳过全局技能 | ✅ | `src/package_manager.rs` |
| **`global_skills` 白名单** — 选择加载特定全局技能，可与 `project_only` 叠加 | ✅ | `src/package_manager.rs` |
