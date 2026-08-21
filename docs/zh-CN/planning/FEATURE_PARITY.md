# 功能对等：pi_agent_rust vs Pi Agent（TypeScript）

> **目的：** 实现状态的权威单一事实来源。
> **最后更新：** 2026-02-18（实现快照刷新）
> **发布声明护栏：** 本文档仅为进度证据。除非 `docs/evidence/dropin-certification-verdict.json` 报告 `overall_verdict = CERTIFIED`，否则禁止使用严格的 drop-in 替代措辞。

## 状态图例

| 状态 | 含义 |
|--------|---------|
| ✅ 已实现 | 功能已存在且有测试覆盖 |
| 🔶 部分实现 | 已有部分功能，仍有已知缺口 |
| ❌ 缺失 | 在范围内但尚未实现 |
| ⬜ 超出范围 | 本次移植有意排除 |

---

## 执行摘要

| 分类 | 已实现 | 部分实现 | 缺失 | 超出范围 | 总计 |
|----------|-------------|---------|---------|--------------|-------|
| **核心类型** | 8 | 0 | 0 | 0 | 8 |
| **提供方层** | 18 | 0 | 0 | 9 | 27 |
| **工具（共 7 个）** | 7 | 0 | 0 | 0 | 7 |
| **智能体运行时** | 7 | 0 | 0 | 0 | 7 |
| **会话管理** | 10 | 0 | 0 | 0 | 10 |
| **CLI** | 10 | 0 | 0 | 0 | 10 |
| **资源与定制** | 8 | 0 | 0 | 0 | 8 |
| **扩展运行时** | 12 | 0 | 0 | 0 | 12 |
| **TUI** | 18 | 0 | 0 | 2 | 20 |
| **配置** | 9 | 0 | 0 | 0 | 9 |
| **认证** | 8 | 0 | 0 | 0 | 8 |

---

## 1. 核心类型（Message/Content/Usage）

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| Message 联合类型（User/Assistant/ToolResult） | ✅ | `src/model.rs:13-19` | 单元 | 带 serde 的完整枚举 |
| UserMessage | ✅ | `src/model.rs:22-27` | 单元 | Text 或 Blocks 内容 |
| AssistantMessage | ✅ | `src/model.rs:38-50` | 单元 | 完整元数据 |
| ToolResultMessage | ✅ | `src/model.rs:53-63` | 单元 | 错误标志、详情 |
| ContentBlock 枚举 | ✅ | `src/model.rs:86-93` | 单元 | Text/Thinking/Image/ToolCall |
| StopReason 枚举 | ✅ | `src/model.rs:70-79` | 单元 | 全部 5 个变体 |
| Usage 跟踪 | ✅ | `src/model.rs:145-166` | 单元 | 输入/输出/缓存/成本 |
| StreamEvent 枚举 | ✅ | `src/model.rs:172-232` | 单元 | 全部 12 种事件类型 |

---

## 2. 提供方层

### 2.1 提供方 Trait

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| Provider trait 定义 | ✅ | `src/provider.rs:18-31` | - | 基于 async_trait |
| Context 结构体 | ✅ | `src/provider.rs:38-43` | - | 系统提示 + 消息 + 工具 |
| StreamOptions | ✅ | `src/provider.rs:62-72` | - | Temperature、max_tokens、thinking |
| ToolDef 结构体 | ✅ | `src/provider.rs:49-55` | - | JSON Schema 参数 |
| Model 定义 | ✅ | `src/provider.rs:108-121` | - | 成本、上下文窗口等 |
| ThinkingLevel 枚举 | ✅ | `src/model.rs:239-265` | 单元 | 6 个级别及预算 |
| CacheRetention 枚举 | ✅ | `src/provider.rs:75-81` | - | None/Short/Long |

### 2.2 提供方实现

| 提供方 | 状态 | Rust 位置 | 测试 | 备注 |
|----------|--------|---------------|-------|-------|
| **Anthropic** | ✅ | `src/providers/anthropic.rs` | 单元 | 完整流式 + 思考 + 工具 |
| **OpenAI** | ✅ | `src/providers/openai.rs` | 单元 | 完整流式 + 工具使用 |
| **Google Gemini** | ✅ | `src/providers/gemini.rs` | 4 | 完整流式 + 工具使用 |
| **Azure OpenAI** | ✅ | `src/providers/azure.rs` | 4 | 完整流式 + 工具使用 |
| Amazon Bedrock | ⬜ | - | - | 低优先级 |
| Google Vertex | ⬜ | - | - | 低优先级 |
| GitHub Copilot | ⬜ | - | - | OAuth 复杂 |
| XAI | ⬜ | - | - | 低优先级 |
| Groq | ⬜ | - | - | 低优先级 |
| Cerebras | ⬜ | - | - | 低优先级 |
| OpenRouter | ⬜ | - | - | 低优先级 |
| Mistral | ⬜ | - | - | 低优先级 |
| Custom providers | ⬜ | - | - | 延后 |

### 2.3 流式实现

| 功能 | 状态 | 位置 | 备注 |
|---------|--------|----------|-------|
| SSE 解析（Anthropic） | ✅ | `anthropic.rs` | `asupersync` HTTP 流（`src/http/client.rs`）+ `src/sse.rs` |
| SSE 解析器模块 | ✅ | `src/sse.rs` | 为 asupersync 迁移的自定义解析器 |
| 文本增量流式 | ✅ | `anthropic.rs:339-352` | 实时文本 |
| 思考增量流式 | ✅ | `anthropic.rs:354-367` | 扩展思考 |
| 工具调用流式 | ✅ | `anthropic.rs:368-382` | JSON 累积 |
| 用量更新 | ✅ | `anthropic.rs:430-448` | Token 计数 |
| 错误事件处理 | ✅ | `anthropic.rs:258-266` | API 错误 |

---

## 3. 内置工具

| 工具 | 状态 | Rust 位置 | 测试 | 一致性测试 |
|------|--------|---------------|-------|-------------------|
| **read** | ✅ | `src/tools.rs` | 4 | ✅ test_read_* |
| **bash** | ✅ | `src/tools.rs` | 3 | ✅ test_bash_* |
| **edit** | ✅ | `src/tools.rs` | 3 | ✅ test_edit_* |
| **write** | ✅ | `src/tools.rs` | 2 | ✅ test_write_* |
| **grep** | ✅ | `src/tools.rs` | 3 | ✅ test_grep_* |
| **find** | ✅ | `src/tools.rs` | 2 | ✅ test_find_* |
| **ls** | ✅ | `src/tools.rs` | 3 | ✅ test_ls_* |

### 3.1 工具特性详情

| 功能 | read | bash | edit | write | grep | find | ls |
|---------|------|------|------|-------|------|------|-----|
| 基础操作 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 截断（head/tail） | ✅ | ✅ | - | - | ✅ | ✅ | ✅ |
| 图片支持 | ✅ | - | - | - | - | - | - |
| 流式更新 | - | ✅ | - | - | - | - | - |
| 行号 | ✅ | - | - | - | ✅ | - | - |
| 模糊匹配 | - | - | ✅ | - | - | - | - |
| 路径解析 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| ~ 展开 | ✅ | - | ✅ | ✅ | ✅ | ✅ | ✅ |
| macOS 截图路径 | ✅ | - | - | - | - | - | - |

### 3.2 截断常量

| 常量 | 值 | 使用方 |
|----------|-------|---------|
| DEFAULT_MAX_LINES | 2000 | read、bash、grep |
| DEFAULT_MAX_BYTES | 50KB | read、bash、grep、find、ls |
| GREP_MAX_LINE_LENGTH | 500 | grep |

---

## 4. 智能体运行时

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| Agent 结构体 | ✅ | `src/agent.rs` | 单元 | 提供方 + 工具 + 配置 |
| Agent 循环 | ✅ | `src/agent.rs` | - | 工具迭代上限 |
| 工具执行 | ✅ | `src/agent.rs` | 单元 | 错误处理 |
| 事件回调 | ✅ | `src/agent.rs` | - | 9 种事件类型 |
| 流处理 | ✅ | `src/agent.rs` | - | 增量处理 |
| 上下文构建 | ✅ | `src/agent.rs` | - | System + 历史 + 工具 |
| 中止处理 | ✅ | `src/agent.rs`、`src/main.rs`、`src/interactive.rs` | - | Ctrl+C 取消进行中的请求 |

---

## 5. 会话管理

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| Session 结构体 | ✅ | `src/session.rs` | - | Header + 条目 + 路径 |
| SessionHeader | ✅ | `src/session.rs` | - | Version 3 |
| JSONL 持久化 | ✅ | `src/session.rs` | - | 保存/加载 |
| 条目类型（7 种） | ✅ | `src/session.rs` | - | Message、ModelChange 等 |
| 树结构 | ✅ | `src/session.rs` | 7 | 完整父/子导航 |
| CWD 编码 | ✅ | `src/session.rs` | 1 | 会话目录命名 |
| 条目 ID 生成 | ✅ | `src/session.rs` | - | 8 字符 hex |
| 续接上一会话 | ✅ | `src/session.rs` | - | 按 mtime 取最近 |
| 会话选择器 UI | ✅ | `src/session_picker.rs` | 3 | 基于 bubbletea 的 TUI 选择器 |
| 分支/导航 | ✅ | `src/session.rs` | 7 | navigate_to、create_branch_from、list_leaves、branch_summary |

---

## 6. CLI

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| 参数解析 | ✅ | `src/cli.rs` | - | Clap derive |
| 子命令 | ✅ | `src/cli.rs`、`src/main.rs` | - | Install、Remove、Update、List、Config |
| @file 参数 | ✅ | `src/cli.rs` | - | 文件包含 |
| 消息参数 | ✅ | `src/cli.rs` | - | 位置文本 |
| 工具选择 | ✅ | `src/cli.rs` | - | --tools 标志 |
| 模型列表 | ✅ | `src/main.rs` | - | 表格输出 |
| 会话导出 | ✅ | `src/main.rs` | - | HTML 导出 |
| 打印模式 | ✅ | `src/main.rs` | - | 单次模式 |
| RPC 模式 | ✅ | `src/main.rs`、`src/rpc.rs` | `tests/rpc_mode.rs` | 无头 stdin/stdout JSON 协议（prompt/steer/follow_up/state/stats/model/thinking/compact/bash/fork） |
| 包管理 | ✅ | `src/package_manager.rs`、`src/main.rs` | 单元 | install/remove/update/list + 设置更新 + 启动自动安装 + 资源解析 |

---

## 6A. 资源与定制

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| 技能加载器 + 校验 | ✅ | `src/resources.rs` | 单元 | Agent 技能 frontmatter + 诊断 |
| 技能提示注入 | ✅ | `src/main.rs` | 单元 | 若启用 `read` 工具则追加 `<available_skills>` |
| 技能命令展开（`/skill:name`） | ✅ | `src/resources.rs`、`src/interactive.rs` | 单元 | 展开为 `<skill ...>` 块 |
| 提示模板加载器 | ✅ | `src/resources.rs` | 单元 | 全局/项目 + 显式路径 |
| 提示模板展开（`/name args`） | ✅ | `src/resources.rs`、`src/interactive.rs` | 单元 | `$1`、`$@`、`$ARGUMENTS`、`${@:N}` |
| 包资源发现 | ✅ | `src/resources.rs` | 单元 | 读取 `package.json` 的 `pi` 字段或默认值 |
| 主题发现 | ✅ | `src/theme.rs`、`src/interactive.rs` | 单元 + `tests/tui_state.rs` | 加载器 + /theme 切换 |
| 主题热重载 | ✅ | `src/interactive.rs` | `tests/tui_state.rs` | `/reload` 重新解析并应用当前主题 |

## 6B. 扩展运行时

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| 扩展发现（路径 + 包） | ✅ | `src/package_manager.rs`、`src/resources.rs` | 单元 | 从设置/自动发现/包/CLI 解析 `extensions/` 源 |
| 扩展协议（v1）+ JSON schema | ✅ | `src/extensions.rs`、`docs/schema/extension_protocol.json` | 单元 + `tests/extensions_manifest.rs` | `ExtensionMessage::parse_and_validate` + schema 编译测试 |
| 兼容性扫描器（Node API 审计） | ✅ | `src/extensions.rs`、`src/package_manager.rs` | `tests/ext_conformance_artifacts.rs` | 启用 `PI_EXT_COMPAT_SCAN` 时发出兼容性账本 |
| 能力清单 + 策略 | ✅ | `src/extensions.rs` | 单元 + `tests/extensions_manifest.rs` | `strict/prompt/permissive` + 作用域清单（`pi.ext.cap.v1`） |
| FS 连接器（作用域化、防逃逸） | ✅ | `src/extensions.rs` | 单元 | 路径遍历 + 符号链接逃逸加固 |
| HTTP 连接器（策略门控） | ✅ | `src/connectors/http.rs` | 单元 | TLS/allowlist/denylist/大小/超时 |
| PiJS 运行时（QuickJS） | ✅ | `src/extensions_js.rs` | 单元 + `tests/event_loop_conformance.rs` | 确定性调度器 + Promise 桥接 + 预算/超时 |
| Promise 宿主调用桥接（pi.* → 队列 → 完成） | ✅ | `src/extensions_js.rs` | 单元 | `pi.tool/exec/http/session/ui/events` + `setTimeout/clearTimeout` |
| 宿主调用 ABI（host_call/host_result 协议） | ✅ | `src/extensions.rs` | 单元 | 协议类型 + 校验已存在；端到端分发已打通 |
| 扩展 UI 桥接（select/confirm/input/editor） | ✅ | `src/extensions.rs`、`src/interactive.rs`、`src/rpc.rs` | 单元 | UI 请求/响应管道已存在；运行时分发已打通 |
| 扩展会话 API（get_state/messages/set_name） | ✅ | `src/extensions.rs`、`src/interactive.rs` | - | Trait + 交互式实现已存在；运行时分发已打通 |
| JS 扩展执行 + 注册（工具/命令/钩子） | ✅ | `src/extensions_js.rs`、`src/extension_dispatcher.rs`、`src/agent.rs`、`src/interactive.rs` | 单元 + E2E | QuickJS 运行时加载 JS/TS 扩展并支持工具/命令注册 + 执行 + 事件钩子 |

---

## 7. 配置

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| 配置加载 | ✅ | `src/config.rs` | - | 全局 + 项目合并 |
| Settings 结构体 | ✅ | `src/config.rs` | - | 全部字段可选 |
| 默认访问器 | ✅ | `src/config.rs` | - | 回退值 |
| 压缩设置 | ✅ | `src/config.rs` | - | enabled、reserve、keep |
| 重试设置 | ✅ | `src/config.rs` | - | enabled、max、delays |
| 图片设置 | ✅ | `src/config.rs` | - | auto_resize、block |
| 终端设置 | ✅ | `src/config.rs` | - | show_images、clear |
| 思考预算 | ✅ | `src/config.rs` | - | 按级别覆盖 |
| 环境变量 | ✅ | `src/config.rs` | - | PI_CONFIG_PATH/PI_CODING_AGENT_DIR/PI_PACKAGE_DIR/PI_SESSIONS_DIR + 提供方 API 密钥 |

---

## 8. 终端 UI

### 8.1 非交互输出（rich_rust）

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| PiConsole 包装器 | ✅ | `src/tui.rs` | 3 | rich_rust 集成 |
| 样式输出（markup） | ✅ | `src/tui.rs` | - | 颜色、加粗、暗淡 |
| 智能体事件渲染 | ✅ | `src/tui.rs` | - | 文本、思考、工具、错误 |
| 表格渲染 | ✅ | `src/tui.rs` | - | 经 rich_rust Tables |
| 面板渲染 | ✅ | `src/tui.rs` | - | 经 rich_rust Panels |
| 分割线渲染 | ✅ | `src/tui.rs` | - | 水平分割线 |
| Spinner 样式 | ✅ | `src/tui.rs` | 1 | Dots、line、simple |

### 8.2 交互式 TUI（charmed_rust/bubbletea）

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| PiApp Model | ✅ | `src/interactive.rs` | 296+ | Elm 架构（296 个 tui_state + 226 个 lib 单元测试） |
| 带历史的 TextInput | ✅ | `src/interactive.rs` | - | bubbles TextInput |
| Markdown 渲染 | ✅ | `src/interactive.rs` | - | glamour Dark 样式 |
| Token/成本页脚 | ✅ | `src/interactive.rs` | - | 用量跟踪 |
| Spinner 动画 | ✅ | `src/interactive.rs` | - | bubbles spinner |
| 工具状态展示 | ✅ | `src/interactive.rs` | - | 运行中工具指示器 |
| 键盘导航 | ✅ | `src/interactive.rs` | - | 上/下历史、Esc 退出 |
| 智能体集成 | ✅ | `src/interactive.rs` | - | 智能体事件已联动；CLI 交互式使用 PiApp |
| 多行编辑器 | ✅ | `src/interactive.rs` | - | 带自动换行的 TextArea |
| 斜杠命令系统 | ✅ | `src/interactive.rs` | - | /help、/login、/logout、/clear、/model、/thinking、/exit、/history、/export、/session、/resume、/new、/copy、/name、/hotkeys |
| 视口滚动 | ✅ | `src/interactive.rs` | - | 带 scroll_to_bottom() 的 Viewport |
| 图片展示 | ⬜ | - | - | 依赖终端 |
| 自动补全 | ✅ | `src/autocomplete.rs`、`src/interactive.rs` | `tests/tui_state.rs` | Tab 触发下拉 + 路径补全 |

### 8.3 交互式命令（斜杠）

| 命令 | 状态 | Rust 位置 | 备注 |
|---------|--------|---------------|-------|
| `/help` | ✅ | `src/interactive.rs` | 帮助文本 |
| `/clear` | ✅ | `src/interactive.rs` | 清空内存中的会话视图 |
| `/model` | ✅ | `src/interactive.rs` | 切换模型/提供方 |
| `/thinking` | ✅ | `src/interactive.rs` | 设置思考级别 |
| `/history` | ✅ | `src/interactive.rs` | 显示输入历史 |
| `/export` | ✅ | `src/interactive.rs` | 导出会话为 HTML |
| `/exit` / `/quit` | ✅ | `src/interactive.rs` | 退出 Pi |
| `/login` | ✅ | `src/interactive/commands.rs`、`src/auth.rs` | Anthropic OAuth + OpenAI/Google API 密钥 + 扩展 OAuth |
| `/logout` | ✅ | `src/interactive.rs`、`src/auth.rs` | 移除已存储凭证 |
| `/session` | ✅ | `src/interactive.rs` | 显示会话信息（路径/token/成本） |
| `/resume` | ✅ | `src/interactive.rs` | 会话选择器浮层（禁用删除） |
| `/new` | ✅ | `src/interactive.rs` | 开始新的内存会话 |
| `/name <name>` | ✅ | `src/interactive.rs` | 设置会话显示名 |
| `/copy` | ✅ | `src/interactive.rs` | 剪贴板支持为特性门控（`--features clipboard`） |
| `/hotkeys` | ✅ | `src/interactive.rs` | 显示快捷键 |
| `/scoped-models` | ✅ | `src/interactive/commands.rs` | 模式匹配 + 持久化到项目设置 |
| `/settings` | ✅ | `src/interactive.rs` | 显示生效设置 + 资源计数 |
| `/tree` | ✅ | `src/interactive.rs` | 列出叶子并按 id/index 切换分支 |
| `/fork` | ✅ | `src/interactive.rs` | 从用户消息分叉新会话文件 |
| `/compact [prompt]` | ✅ | `src/interactive.rs`、`src/compaction.rs` | 手动压缩 |
| `/share` | ✅ | `src/interactive/share.rs` | HTML 导出 + 经 `gh` CLI 上传 GitHub Gist |
| `/reload` | ✅ | `src/interactive.rs`、`src/resources.rs` | 重载技能/提示/主题 + 刷新自动补全 |
| `/changelog` | ✅ | `src/interactive.rs` | 展示更新日志条目 |

---

## 9. 认证

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| 来自 env 的 API 密钥 | ✅ | `src/auth.rs` | - | ANTHROPIC_API_KEY 等 |
| 来自 flag 的 API 密钥 | ✅ | `src/main.rs` | - | --api-key |
| auth.json 存储 | ✅ | `src/auth.rs` | - | 0600 权限文件 |
| 文件锁 | ✅ | `src/auth.rs` | - | 带超时的排他锁 |
| 密钥解析 | ✅ | `src/auth.rs` | - | override > auth.json > env |
| 多提供方密钥 | ✅ | `src/auth.rs` | - | 支持 12 个认证提供方族 |
| OAuth 流程 | ✅ | `src/auth.rs`、`src/interactive/commands.rs` | 单元 | Anthropic PKCE + 扩展注册的提供方 |
| Token 刷新 | ✅ | `src/auth.rs`、`src/main.rs` | 单元 | 启动时对全部 OAuth 提供方自动刷新 |

---

## 10. 错误处理

| 功能 | 状态 | Rust 位置 | 测试 | 备注 |
|---------|--------|---------------|-------|-------|
| Error 枚举 | ✅ | `src/error.rs` | - | 基于 thiserror |
| 配置错误 | ✅ | `src/error.rs` | - | |
| 会话错误 | ✅ | `src/error.rs` | - | 含 NotFound |
| 提供方错误 | ✅ | `src/error.rs` | - | 提供方 + 消息 |
| 认证错误 | ✅ | `src/error.rs` | - | |
| 工具错误 | ✅ | `src/error.rs` | - | 工具名 + 消息 |
| 校验错误 | ✅ | `src/error.rs` | - | |
| IO/JSON/HTTP 错误 | ✅ | `src/error.rs` | - | 来自 impl |

---

## 测试覆盖摘要

| 分类 | 单元测试 | 集成测试 | 夹具用例 | 总计 |
|----------|------------|-------------------|---------------|-------|
| 核心类型 | 4 | 0 | 0 | 4 |
| 提供方（Anthropic） | 2 | 0 | 0 | 2 |
| 提供方（OpenAI） | 3 | 0 | 0 | 3 |
| 提供方（Gemini） | 4 | 0 | 0 | 4 |
| 提供方（Azure） | 4 | 0 | 0 | 4 |
| SSE 解析器 | 11 | 0 | 0 | 11 |
| 工具 | 5 | 20 | 122 | 147 |
| CLI 标志（夹具） | 0 | 0 | 17 | 17 |
| TUI（rich_rust） | 3 | 0 | 0 | 3 |
| TUI（交互式 lib） | 226 | 0 | 0 | 226 |
| TUI（tui_state 集成） | 0 | 296 | 0 | 296 |
| TUI（e2e_tui_perf） | 0 | 103 | 0 | 103 |
| TUI（会话选择器） | 3 | 0 | 0 | 3 |
| TUI（性能单元：FrameTiming/Cache/Buffers） | 47 | 0 | 0 | 47 |
| 会话（分支） | 7 | 0 | 0 | 7 |
| 智能体 | 2 | 0 | 0 | 2 |
| 一致性基础设施 | 6 | 0 | 0 | 6 |
| 扩展 | 2 | 0 | 0 | 2 |
| 其他 lib 测试 | 2,800+ | 0 | 0 | 2,800+ |
| **总计（lib）** | **3,319** | - | - | **3,319** |
| **总计（全部目标）** | **3,319+** | **399+** | **139** | **3,857+** |

**全部测试通过**（`cargo test --lib`：3,319 通过；`tui_state`：296 通过；`e2e_tui_perf`：103 通过）

---

## 一致性测试状态

| 组件 | 是否有夹具测试 | 夹具文件 | 用例数 | 状态 |
|-----------|-------------------|--------------|-------|--------|
| read 工具 | ✅ 有 | `read_tool.json` | 23 | ✅ 全部通过 |
| write 工具 | ✅ 有 | `write_tool.json` | 7 | ✅ 全部通过 |
| edit 工具 | ✅ 有 | `edit_tool.json` | 23 | ✅ 全部通过 |
| bash 工具 | ✅ 有 | `bash_tool.json` | 34 | ✅ 全部通过 |
| grep 工具 | ✅ 有 | `grep_tool.json` | 12 | ✅ 全部通过 |
| find 工具 | ✅ 有 | `find_tool.json` | 6 | ✅ 全部通过 |
| ls 工具 | ✅ 有 | `ls_tool.json` | 8 | ✅ 全部通过 |
| truncation | ✅ 有 | `truncation.json` | 9 | ✅ 全部通过 |
| 会话格式 | ✅ 有 | `tests/session_conformance.rs` | 28 | ✅ 全部通过 |
| 提供方响应 | ✅ 有 | `tests/provider_streaming.rs` | 4 | ✅ 全部通过（VCR） |
| CLI 标志 | ✅ 有 | `cli_flags.json` | 17 | ✅ 全部通过 |
| **总计** | **11/11** | - | **171** | ✅ |

### 夹具 Schema

夹具为 `tests/conformance/fixtures/` 中的 JSON 文件，结构如下：

```json
{
  "version": "1.0",
  "tool": "tool_name",
  "cases": [
    {
      "name": "test_name",
      "setup": [{"type": "create_file", "path": "...", "content": "..."}],
      "input": {"param": "value"},
      "expected": {
        "content_contains": ["..."],
        "content_regex": "...",
        "details_exact": {"key": "value"}
      }
    }
  ]
}
```

---

## 性能目标

| 指标 | 目标 | 当前 | 状态 |
|--------|--------|---------|--------|
| 启动时间 | <100ms | 13ms（`pi --version`） | ✅ |
| 二进制体积（release） | <20MB | 8.3MB | ✅ |
| TUI 帧率 | 60fps | 已埋点（PERF-3：帧时序遥测） | ✅ |
| 帧预算 | <16ms | 已强制（PERF-4：超限自动降级） | ✅ |
| 内存（空闲） | <50MB | 已监控（PERF-6：基于 RSS 的压力检测） | ✅ |

### 性能特性（PERF 轨道 — 已完成）

| 特性 | Bead | 描述 | 状态 |
|---------|------|-------------|--------|
| 消息渲染缓存 | PERF-1 | 按消息的记忆化 + 基于 generation 的失效 | ✅ |
| 增量前缀 | PERF-2 | 流式快速路径：缓存前缀 + 仅追加尾部 | ✅ |
| 帧时序遥测 | PERF-3 | view()/update() 的微秒级埋点 | ✅ |
| 帧预算 + 降级 | PERF-4 | 帧超出 16ms 预算时自动降级渲染 | ✅ |
| 内存压力检测 | PERF-6 | RSS 监控，阈值处渐进式折叠 | ✅ |
| 缓冲区预分配 | PERF-7 | 可复用渲染缓冲区、容量提示、零拷贝路径 | ✅ |
| Criterion 基准 | PERF-8 | 覆盖全部关键渲染路径的基准套件 | ✅ |
| CI 回归门 | PERF-9 | 性能回退 >20% 时 CI 失败 | ✅ |
| 跨平台回退 | PERF-CROSS | /proc 不可用时优雅降级（macOS/Windows） | ✅ |

---

## 后续步骤（优先级排序）

1. ~~**完成打印模式** - 非交互单次响应~~ ✅ 已完成
2. ~~**添加 OpenAI 提供方** - 第二个提供方实现~~ ✅ 已完成
3. ~~**实现 auth.json** - 凭证存储~~ ✅ 已完成（src/auth.rs）
4. ~~**会话选择器 UI** - --resume 的基础 TUI~~ ✅ 已完成（src/session_picker.rs）
5. ~~**分支/导航** - 树操作~~ ✅ 已完成（src/session.rs）
6. ~~**基准工具链** - 性能验证~~ ✅ 已完成（benches/tools.rs、BENCHMARKS.md）
7. ~~**一致性夹具** - TypeScript 参考捕获~~ ✅ 已完成（tests/conformance/）
