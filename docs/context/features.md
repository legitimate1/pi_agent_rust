# 功能目录

## CLI 与交互

- **交互式 TUI（流式渲染、Markdown、主题）** ✅ → `src/interactive.rs` + `src/tui.rs`
- **非交互式 Print 模式** ✅ → `src/main.rs`
- **RPC/stdin 服务器模式（含 sessionId 修复）** ✅ → `src/rpc.rs`
- **RPC 进程侧主动会话持久化（防客户端崩溃丢数据）** ✅ → `src/rpc.rs`
- **RPC 方法 `estimate_tokens`（会话 token 估算）** ✅ → `src/rpc.rs` `src/compaction.rs`
- **RPC 方法 `get_commands` — 合并扩展注册的斜杠命令** ✅ → `src/rpc.rs` `src/extensions.rs`
- **RPC 方法 `get_tree` — 查询会话树形结构（分支/叶子）** ✅ → `src/rpc.rs`
- **RPC 方法 `get_version` — 返回 pi 版本号和 Git SHA** ✅ → `src/rpc.rs`
- **RPC 方法 `get_system_prompt` — 返回当前会话的 system prompt 及注册的工具定义** ✅ → `src/rpc.rs` `src/agent.rs`
- **`session_state` 返回模型 `compat` 字段** — 客户端可获取 `thinkingLevelMap` 等兼容性配置 ✅ → `src/rpc.rs`
- **RPC 方法 `remove_from_queue` — 按 messageId 精确取消队列消息** ✅ → `src/rpc.rs`
- **RPC 方法 `clear_queue` — 一键清空 steer/follow_up 队列** ✅ → `src/rpc.rs`
- **RPC 方法 `get_queue` — 查询队列当前内容（steering + follow_up）** ✅ → `src/rpc.rs`
- **`queue_update` 事件推送 — 队列状态变更时实时同步给客户端** ✅ → `src/rpc.rs`
- **队列消息携带 `messageId` — 客户端分配或服务端 UUID fallback** ✅ → `src/rpc.rs`
- **set_model/set_thinking_level 支持 persist 参数** — `persist=false` 仅内存切换，不写会话文件 ✅ → `src/rpc.rs` `src/acp.rs` `src/extension_dispatcher.rs` `src/sdk.rs` `src/session.rs` `src/agent.rs`
- **CLI 子命令（doctor/config/list/info 等）** ✅ → `src/main.rs`
- **Extended Thinking 级别** — off/minimal/low/medium/high/xhigh，`--thinking` 或 `/thinking` 切换 ✅ → `src/agent.rs` `src/models.rs`
- **交互式 Autocomplete** — `@` 文件引用 + `/` 斜杠命令补全，模糊评分排序，背景线程每 30s 重建项目索引（WalkBuilder 尊重 .gitignore，5000 条缓存上限） ✅ → `src/autocomplete.rs` `src/interactive/`
- **Credential-Aware 模型选择** — `Ctrl+L` 打开模型选择器（只列凭据就绪的模型），`Ctrl+P`/`Ctrl+Shift+P` 循环切换，provider ID/别名大小写不敏感 ✅ → `src/model_selector.rs` `src/interactive/model_selector_ui.rs`
- **`~/.pi/agent/SYSTEM.md` 覆盖默认系统提示词** ✅ → `src/app.rs`
- **`.pi/SYSTEM.md` 项目级系统提示词** — 优先级高于用户级 ✅ → `src/app.rs`

## Provider 层

- **Anthropic API（流式 + 扩展思考 + 工具）** ✅ → `src/providers/anthropic.rs`
- **OpenAI Chat Completions（含所有兼容推理模型的 reasoning_effort）** ✅ → `src/providers/openai.rs`
- **OpenAI Responses / Codex Responses** ✅ → `src/providers/openai_responses.rs`
- **Gemini（流式 + 工具）** ✅ → `src/providers/gemini.rs`
- **Cohere（流式 + 工具）** ✅ → `src/providers/cohere.rs`
- **Azure OpenAI** ✅ → `src/providers/azure.rs`
- **Bedrock / Vertex AI / GitHub Copilot / GitLab Duo** ✅ → `src/providers/*.rs`
- **扩展 stream-simple Provider 桥接** ✅ → `src/providers/mod.rs`
- **Provider 工厂 + 路由** ✅ → `src/providers/mod.rs`
- **12 个 native provider 实现模块** — anthropic/openai/openai_responses/gemini/cohere/azure/bedrock/vertex/copilot/gitlab/cursor/model_fetch ✅ → `src/providers/*.rs`（除 mod.rs 外 12 个）
- **本地 Provider** — ollama/llamacpp/mistralrs/lmstudio 内置，默认端口直连、无需 API key ✅ → `src/provider_metadata.rs` `src/providers/openai.rs`
- **认证与凭据管理** — API Key / OAuth（PKCE+刷新）/ AWS 凭据链 / Service Key / Bearer Token，存 `~/.pi/agent/auth.json`（文件锁防并发损坏），支持 `$ENV:` 与 `$CMD:` 动态解析 ✅ → `src/auth.rs`
- **Provider 运行时热切换** — `set_model` 跨 provider 切换时直接在内存创建/替换 provider 实例（anthropic ↔ openai ↔ gemini 等），无需重启进程 ✅ → `src/agent.rs` `src/providers/mod.rs`
- **opencode-go（OpenCode Zen Go，deepseek-v4-flash 等）** ✅ → `src/provider_metadata.rs` `src/app.rs`
- **流式截断自动重试** — 上游 SSE 中途截断（`JSON parse error: EOF while parsing a string`）分类为瞬时错误自动重试，不再直接断流；语法类 parse error 仍不可重试 ✅ → `src/providers/openai.rs` `src/rpc.rs`

## 工具系统

- **ToolRegistry — 工具注册表** ✅ → `src/tools/mod.rs`
- **内置 9 工具（read/bash/pwsh/edit/write/grep/find/ls/hashline）** ✅ → `src/tools/` 各子模块
- **进程清理模式统一为 ProcessGroupTree** — 所有 spawn 子进程的工具（bash/pwsh/grep/find）均使用 `isolate_command_process_group` + `taskkill /F /T` 杀整个进程树 ✅ → `src/tools/bash.rs` `src/tools/pwsh.rs` `src/tools/grep.rs` `src/tools/find.rs`
- **`Tool::execute()` 支持 abort 信号** — trait 新增 `abort: Option<AbortSignal>` 参数，long-running 工具在循环中检查并主动 kill 子进程 ✅ → `src/tools/mod.rs` `src/abort.rs`
- **`ProcessGuard::wait_with_cancellation()` 支持 abort 信号** — 循环中检查外部 abort + cx.checkpoint + 超时 ✅ → `src/tools/mod.rs:3381`
- **ReadTool — 无 CWD/agent-dir 路径限制** + head/tail/info/diff 参数 + 编码自动检测 ✅ → `src/tools/read.rs`
- **WriteTool / EditTool / HashlineEditTool — 无 CWD 路径限制**（可写入任意绝对路径） ✅ → `src/tools/write.rs` `src/tools/edit.rs` `src/tools/hashline.rs`
- **@file CLI 参数 — 无 CWD 路径限制**（`pi @任意路径` 可读任意目录文件，≤100MB 检查 + 1MB 截断） ✅ → `src/tools/mod.rs` `process_file_arguments`
- **EditTool — 直接写入**（非 tempfile 原子重命名，避让 Windows 句柄冲突） ✅ → `src/tools/edit.rs`
- **FindTool / GrepTool / LsTool — 无 CWD 路径限制**（可搜索/列出任意绝对路径） ✅ → `src/tools/find.rs` `src/tools/grep.rs` `src/tools/ls.rs`
- **扩展工具收集** ✅ → `src/extension_tools.rs:100`
- **扩展工具 onUpdate 流式进度推送**（工具执行中 `onUpdate({content, details})`，经 rquickjs Function 桥接到 Rust `ToolUpdate`） ✅ → `src/extensions.rs` `src/extensions_js.rs` `src/extension_tools.rs`
- **扩展工具同名覆盖内置工具** ✅ → `src/tools/mod.rs` `src/agent.rs`
- **内置 pwsh 工具**（PowerShell 命令执行、尾部截断 2000 行/1MB、exit 0 时自动过滤 stderr、绝对路径 fallback 解决带空格 PATH 解析 bug） ✅ → `src/tools/pwsh.rs`
- **运行时禁用内置工具**（`disabledTools` 配置） ✅ → `src/config.rs` `src/main.rs`
- **工具描述外部覆盖**（`toolDescriptions` 配置，免编译修改工具描述） ✅ → `src/config.rs` `src/tools/mod.rs` `src/agent.rs`
- **编辑后轻量验证（verify 参数）** — edit/hashline_edit/write 支持可选 verify 参数，编辑后自动运行语法/格式检查（.rs→rustfmt, .json/.toml→进程内解析, .ts/.md→prettier 全局直调、无全局安装时 npx 回退）。结果附在 details.verify，不阻断流程。**扩展 checker 见 `verify-tool.md`** ✅ → `src/tools/verify.rs` `src/tools/edit.rs` `src/tools/hashline.rs` `src/tools/write.rs`
- **ProcessGuard 子进程生命周期管理** — 统一管理 spawn 子进程的清理：ambient cancellation、超时 kill、abort 信号检查、Drop 自动回收 ✅ → `src/tools/mod.rs:3337`
- **Tool trait + JSON Schema 定义** ✅ → `src/tools/mod.rs`
- **Abort 信号原语** — 共享 `AbortHandle`/`AbortSignal`，打破 agent.rs ↔ tools/mod.rs 循环依赖 ✅ → `src/abort.rs`

## 扩展系统

- **JS/TS 扩展加载（QuickJS）** ✅ → `src/extensions_js.rs`
- **JS Runtime 中断机制** — `InterruptBudget.external_trigger` 外部中断 + QuickJS interrupt hook 停止死循环 ✅ → `src/extensions_js.rs:4616`
- **扩展工具 abort 信号桥接** — agent 的 `AbortSignal` 经 `ExtensionToolWrapper → JsRuntimeCommand → await_js_task` 全链路透传：首次检测到 abort 时调 JS `__pi_abort_task(task_id)` 触发扩展的 `AbortController`（扩展可用 `signal.aborted` / `addEventListener('abort')` 优雅退出），下一轮仍 pending 则 `request_interrupt()` 硬中断兜底 ✅ → `src/extension_tools.rs` `src/extensions.rs` `src/extensions_js.rs`
- **Native Rust 扩展（`*.native.json`）** ✅ → `src/extensions.rs`
- **WASM 扩展** ✅ → `src/extensions.rs`
- **能力策略模型（Strict/Prompt/Permissive）** ✅ → `src/extensions.rs:1139`
- **Hostcall 调度（tool/exec/http/session/ui/events/log）** ✅ → `src/extensions_js.rs`
- **RPC 模式扩展交互 UI（select/confirm/input）** ✅ → `src/extensions.rs` `src/rpc.rs`
- **虚拟模块系统（Node.js builtins + npm stubs）** ✅ → `src/extensions_js.rs`
- **VFS write-through 落盘**（`writeFileSync`/`appendFileSync` 写 VFS 内存后同步写真实文件系统） ✅ → `src/extensions_js.rs`

## 会话管理

- **JSONL 会话（v3）** ✅ → `src/session.rs`
- **分支 / 树结构** ✅ → `src/session.rs`
- **SQLite 会话后端** ✅ → `src/session.rs`
- **会话索引元数据** ✅ → `src/session_index.rs`
- **会话保存 Windows 文件竞争重试** — persist/append 遇 PermissionDenied（os error 5）或 sharing violation（os error 32）退避重试；append fsync 拒绝降级警告（数据已入页缓存） ✅ → `src/session.rs`
- **RPC 会话持久化链根修复** — persister 启动扫描计入 session header id，首条 entry parentId 链接链根 ✅ → `src/rpc.rs`
- **RPC 持久化补写 user 消息** — MessageStart(User) 实时落盘（防 session.save 关闭时 user 丢失） ✅ → `src/rpc.rs`
- **Session Store V2 Sidecar** — 分段日志 + 偏移索引 + 周期检查点 + 迁移/回滚台账；大会话 O(index+tail) 快速恢复，损坏帧 fail-closed ✅ → `src/session_store_v2.rs`
- **`pi migrate` 子命令** — JSONL → V2 sidecar 迁移（`--dry-run` 校验不落盘） ✅ → `src/cli.rs` `src/session_store_v2.rs`
- **RPC 方法 `append_custom_entry`** — 客户端（如 pidian 苏格拉底）注入自定义 entry 经 pi 会话管理落盘（CustomEntry，不影响 API 消息链路） ✅ → `src/rpc.rs` `src/session.rs`

## 模型注册表

- **内置模型注册** ✅ → `src/models.rs`
- **`models.json` 自定义模型加载** ✅ → `src/models.rs:698`
- **Provider 元数据（别名 + 认证键）** ✅ → `src/provider_metadata.rs`
- **`supports_xhigh()` 检查 `compat.thinkingLevelMap`** — 第三方 provider 在 models.json 中声明 `thinkingLevelMap.xhigh` 即可支持 xhigh，无需硬编码 ✅ → `src/models.rs:30`

## 资源与配置管理

- **`skill_mode: "project_only"` 配置** — 项目可跳过全局技能 ✅ → `src/package_manager.rs`
- **`global_skills` 白名单** — 选择加载特定全局技能，可与 `project_only` 叠加 ✅ → `src/package_manager.rs`
- **Skills 系统** — `~/.pi/agent/skills/` 或 `.pi/skills/` 下 `SKILL.md`，`/skill:name` 调用 ✅ → `src/resources.rs` `src/package_manager.rs`
- **Prompt Templates** — `.pi/prompts/` 或 `~/.pi/agent/prompts/` Markdown，`/template` 调用，支持 `$1`/`$2`/`$@` 位置参数展开 ✅ → `src/resources.rs`
- **Packages 共享** — `pi install npm:@org/pi-packages` 安装技能/提示词/主题/扩展包 ✅ → `src/package_manager.rs`
