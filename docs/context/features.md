# 功能目录

## CLI 与交互

- **交互式 TUI** — 流式渲染、Markdown 样式、主题支持 | `src/interactive.rs` + `src/tui.rs`
- **非交互式 Print 模式** — 单次响应输出，无交互界面 | `src/main.rs`
- **RPC/stdin 服务器模式** — 客户端经 stdin 发送请求、接收流式事件 | `src/rpc.rs`
- **会话 token 用量估算** — 供客户端预判上下文占用 | `src/rpc.rs` + `src/compaction.rs`
- **斜杠命令查询** — 返回可用命令，含扩展注册的命令 | `src/rpc.rs` + `src/extensions.rs`
- **会话树查询** — 返回会话的分支/叶子结构 | `src/rpc.rs`
- **版本查询** — 返回 pi 版本号和 Git SHA | `src/rpc.rs`
- **系统提示词查询** — 返回当前会话的 system prompt 及注册的工具定义（tools 为应用 tools.toml 覆盖后的最终定义）| `src/rpc.rs` + `src/agent.rs`
- **会话状态查询** — 含模型兼容配置，供客户端渲染思考级别选项 | `src/rpc.rs`
- **消息队列管理** — 查看/精确取消/清空待处理消息，变更实时推送 | `src/rpc.rs`
- **模型/思考级别临时切换** — 可选仅内存切换，重启后恢复默认 | `src/rpc.rs` + `src/acp.rs` + `src/session.rs` + `src/agent.rs`
- **CLI 子命令** — doctor/config/list/info 等诊断与配置命令 | `src/main.rs` + `src/cli.rs`
- **doctor 环境诊断** — 配置/目录/认证/会话/swarm 预检，含跨平台磁盘余量探测 | `src/doctor.rs`
- **Extended Thinking 级别** — off/minimal/low/medium/high/xhigh，`--thinking` 或 `/thinking` 切换 | `src/agent.rs` + `src/models.rs`
- **交互式 Autocomplete** — `@` 文件引用 + `/` 斜杠命令补全，背景自动重建项目索引 | `src/autocomplete.rs` + `src/interactive/`
- **凭据感知模型选择** — `Ctrl+L` 选择器（只列凭据就绪模型），`Ctrl+P`/`Ctrl+Shift+P` 循环切换 | `src/model_selector.rs` + `src/interactive/model_selector_ui.rs`
- **用户级系统提示词覆盖** — `~/.pi/agent/SYSTEM.md` 替代默认提示词 | `src/app.rs`
- **项目级系统提示词覆盖** — `.pi/SYSTEM.md`，优先级高于用户级 | `src/app.rs`
- **系统提示词运行时事实注入** — 自动追加当前日期、工作目录、临时目录（test_mode 下用 `<TIMESTAMP>`/`<CWD>`/`<TEMP>` 占位符）| `src/app.rs`

## Provider 层

- **Anthropic API** — 流式 + 扩展思考 + 工具调用 | `src/providers/anthropic.rs`
- **OpenAI Chat Completions** — 兼容推理模型思考参数 + DeepSeek thinking 方言 | `src/providers/openai.rs`
- **OpenAI Responses / Codex Responses** — 新一代响应式 API | `src/providers/openai_responses.rs`
- **Gemini** — 流式 + 工具 + 思考链，思考强度可调 | `src/providers/gemini.rs` + `src/providers/vertex.rs`
- **Cohere** — 流式 + 工具调用 | `src/providers/cohere.rs`
- **Azure OpenAI** — 企业版 OpenAI 通道 | `src/providers/azure.rs`
- **Bedrock / Vertex AI / GitHub Copilot / GitLab Duo** | `src/providers/*.rs`
- **本地 Provider** — ollama/llamacpp/mistralrs/lmstudio，默认端口直连、无需 API key | `src/provider_metadata.rs` + `src/providers/openai.rs`
- **认证与凭据管理** — API Key / OAuth（PKCE+刷新）/ AWS 凭据链 / Service Key / Bearer Token，支持环境变量与命令动态解析 | `src/auth.rs`
- **Provider 运行时热切换** — 跨 provider 切换模型无需重启进程 | `src/agent.rs` + `src/providers/mod.rs`
- **OpenAI 兼容网关方言识别** — 按网关声明识别 DeepSeek 思考参数方言并正确发送 | `src/provider_metadata.rs` + `src/providers/openai.rs`
- **流式截断自动重试** — 上游 SSE 中途截断分类为瞬时错误自动重试，语法错误不重试 | `src/providers/openai.rs` + `src/rpc.rs`

## 工具系统

- **工具注册表** — 内置/扩展工具统一注册、JSON Schema 定义、按名路由 | `src/tools/mod.rs`
- **内置 9 工具** — read/bash/pwsh/edit/write/grep/find/ls/hashline | `src/tools/` 各子模块
- **子进程统一生命周期管理** — spawn 子进程受控清理（abort/超时/Drop），杀整棵进程树防孤儿 | `src/tools/mod.rs` + `src/abort.rs` + `src/tools/bash.rs` + `src/tools/pwsh.rs` + `src/tools/grep.rs` + `src/tools/find.rs`
- **工具可取消执行** — abort 信号穿透到工具层，长任务循环检查并主动终止 | `src/tools/mod.rs` + `src/abort.rs`
- **ReadTool** — 任意路径读取，head/tail/info/diff 参数，编码自动检测 | `src/tools/read.rs`
- **Write/Edit/HashlineEdit** — 任意绝对路径写入与编辑 | `src/tools/write.rs` + `src/tools/edit.rs` + `src/tools/hashline.rs`
- **@file CLI 参数** — `pi @路径` 读取任意目录文件，带大小上限与截断 | `src/tools/mod.rs`
- **Find/Grep/Ls** — 任意绝对路径下搜索与列出 | `src/tools/find.rs` + `src/tools/grep.rs` + `src/tools/ls.rs`
- **内置 pwsh 工具** — PowerShell 命令执行，长输出尾部截断，exit 0 时过滤 stderr | `src/tools/pwsh.rs`
- **bash 工具 Windows 支持** — 自动定位 Git Bash 安装，缺失时优雅降级 | `src/tools/bash.rs` + `src/tools/mod.rs` + `src/rpc.rs`
- **扩展工具收集与覆盖** — 收集扩展注册工具，同名覆盖内置工具 | `src/extension_tools.rs` + `src/tools/mod.rs` + `src/agent.rs`
- **扩展工具流式进度推送** — 工具执行中实时推送 content/details 进度 | `src/extensions.rs` + `src/extensions_js.rs` + `src/extension_tools.rs`
- **运行时禁用内置工具** — `disabledTools` 配置启动时过滤 | `src/config.rs` + `src/main.rs`
- **工具描述外部覆盖** — `toolDescriptions` 配置免编译修改描述（旧入口，tools.toml 优先）| `src/config.rs` + `src/tools/mod.rs` + `src/agent.rs`
- **tools.toml 工具可见信息覆盖** — `~/.pi/agent/tools.toml` + `.pi/tools.toml`（项目优先）逐工具覆盖 description 与 parameters（JSON Schema），description 同时作用于提示词文字层与 API schema 层，未覆盖工具保持默认 | `src/tool_overrides.rs` + `src/config.rs` + `src/tools/mod.rs` + `src/agent.rs` + `src/app.rs` + `src/main.rs` + `src/sdk.rs` + `src/acp.rs`
- **编辑后轻量验证** — edit/hashline_edit/write 可选 verify 参数，自动语法/格式检查，结果附 details 不阻断 | `src/tools/verify.rs` + `src/tools/edit.rs` + `src/tools/hashline.rs` + `src/tools/write.rs`

## 扩展系统

- **JS/TS 扩展加载** — QuickJS 沙箱运行，无需 Node/Bun | `src/extensions_js.rs`
- **JS 运行时中断** — 死循环/长任务可被外部中断硬停 | `src/extensions_js.rs`
- **扩展工具取消传播** — 先通知 JS 侧 AbortController 优雅退出，仍挂起则硬中断兜底 | `src/extension_tools.rs` + `src/extensions.rs` + `src/extensions_js.rs`
- **Native Rust 扩展** — `*.native.json` 声明式注册 | `src/extensions.rs`
- **WASM 扩展** | `src/extensions.rs`
- **能力策略模型** — Strict/Prompt/Permissive 三档门控 hostcall | `src/extensions.rs`
- **Hostcall 调度** — tool/exec/http/session/ui/events/log 分派 | `src/extensions_js.rs`
- **Exec hostcall 并行执行** — 批量命令有界并发拉起子进程，保序收集 | `src/hostcall_amac.rs` + `src/extensions.rs` + `src/extension_dispatcher.rs`
- **RPC 模式扩展交互 UI** — select/confirm/input 提示组件 | `src/extensions.rs` + `src/rpc.rs`
- **虚拟模块系统** — Node.js builtins 垫片 + npm 包 stub | `src/extensions_js.rs`
- **VFS 写穿透落盘** — 虚拟文件系统写入同步到真实文件系统 | `src/extensions_js.rs`

## 会话管理

- **JSONL 会话存储** — 增量追加的会话文件格式 | `src/session.rs`
- **分支 / 树结构** — 会话可分支、多叶 | `src/session.rs`
- **SQLite 会话后端** — 可选替代存储 | `src/session.rs`
- **会话索引元数据** — 按目录/时间索引会话供选择器与恢复 | `src/session_index.rs`
- **RPC 模式进程侧主动会话持久化** — 消息实时落盘，客户端崩溃不丢数据 | `src/rpc.rs` + `src/session.rs`
- **Windows 文件竞争自动重试** — 保存遇瞬态句柄占用退避重试 | `src/session.rs`
- **分段日志 Sidecar 存储** — 快速恢复大会话，损坏帧 fail-closed | `src/session_store_v2.rs`
- **会话存储迁移命令** — `pi migrate` 将 JSONL 迁移到 sidecar，支持 dry-run 校验 | `src/cli.rs` + `src/session_store_v2.rs`
- **客户端自定义消息注入** — 经 pi 落盘但不进入 LLM 消息链路 | `src/rpc.rs` + `src/session.rs`
- **中断时清理未完成工具调用** — 避免持久化无响应的 tool_call 污染会话 | `src/agent.rs`
- **print 模式会话落盘 opt-in** — 显式 `--session-dir`/`--session` 时持久化，成功与失败都落盘 | `src/app.rs` + `src/main.rs` + `src/session.rs`
- **SDK 独立认证文件** — 可选指定 auth.json 加载路径，供嵌入方与测试隔离凭据 | `src/sdk.rs`

## 模型注册表

- **内置模型注册** | `src/models.rs`
- **自定义模型加载** — `models.json` 扩展模型定义 | `src/models.rs`
- **Provider 元数据** — 别名 + 认证键声明 | `src/provider_metadata.rs`
- **第三方模型 xhigh 档支持** — 经 `compat.thinkingLevelMap` 声明即可，无需硬编码 | `src/models.rs`

## 资源与配置管理

- **技能仅项目模式** — `skill_mode: "project_only"` 跳过全局技能 | `src/package_manager.rs`
- **全局技能白名单** — `global_skills` 只加载指定全局技能，可与 project_only 叠加 | `src/package_manager.rs`
- **Skills 系统** — `~/.pi/agent/skills/` 或 `.pi/skills/` 下 `SKILL.md`，`/skill:name` 调用 | `src/resources.rs` + `src/package_manager.rs`
- **Prompt Templates** — `.pi/prompts/` 或 `~/.pi/agent/prompts/` Markdown，`/template` 调用，支持 `$1`/`$2`/`$@` 位置参数 | `src/resources.rs`
- **Packages 共享** — `pi install` 安装技能/提示词/主题/扩展包 | `src/package_manager.rs`
