# DROPIN-112：功能清单矩阵 — Pi TypeScript vs Pi Rust

> **Bead：** bd-w9i9o | **状态：** in_progress | **优先级：** P0
> **作者：** CobaltElk (claude-opus-4-6) | **日期：** 2026-02-14
> **目的：** 枚举两种实现中所有模式、标志、命令、事件、配置项、API 表面与集成契约。

---

## 图例

| 符号 | 含义 |
|--------|---------|
| Y | 已实现且可用 |
| P | 部分实现——存在但不完整或存在分歧 |
| N | 未实现 |
| X | 不适用（设计决策，非缺口） |
| ? | 未知 / 需进一步调研 |

---

## 1. CLI 标志与选项

### 模型配置

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--provider <name>` | Y | Y | 环境变量：两者均为 PI_PROVIDER |
| `--model <id>` | Y | Y | 环境变量：两者均为 PI_MODEL |
| `--api-key <key>` | Y | Y | 覆盖环境变量 |
| `--models <patterns>` | Y | Y | Ctrl+P 循环，逗号分隔的 glob |
| `--list-models [search]` | Y | Y | 可选模糊搜索模式 |
| `--list-providers` | N | Y | 仅 Rust 新增 |

### 思考 / 推理

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--thinking <level>` | Y | Y | off/minimal/low/medium/high/xhigh |

### 系统提示词

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--system-prompt <text>` | Y | Y | 覆盖系统提示词 |
| `--append-system-prompt <text>` | Y | Y | 追加文本或文件路径 |

### 会话管理

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `-c, --continue` | Y | Y | 继续上一会话 |
| `-r, --resume` | Y | Y | 会话选择器 UI |
| `--session <path>` | Y | Y | 指定会话文件 |
| `--session-dir <dir>` | Y | Y | 会话存储目录 |
| `--no-session` | Y | Y | 临时模式 |

### 模式与输出

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--mode text` | Y | Y | 文本输出模式 |
| `--mode json` | Y | Y | JSON 事件模式 |
| `--mode rpc` | Y | Y | JSON-RPC 协议模式 |
| `-p, --print` | Y | Y | 非交互模式 |
| `--verbose` | Y | Y | 强制详细启动 |

### 工具

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--no-tools` | Y | Y | 禁用全部内置工具 |
| `--tools <list>` | Y | Y | 默认：read,bash,edit,write |

### 扩展

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `-e, --extension <path>` | Y | Y | 加载扩展（可重复） |
| `--no-extensions` | Y | Y | 禁用发现 |
| `--extension-policy <profile>` | N | Y | 仅 Rust：safe/balanced/permissive |
| `--explain-extension-policy` | N | Y | 仅 Rust：打印策略后退出 |
| `--repair-policy <mode>` | N | Y | 仅 Rust：off/suggest/auto-safe/auto-strict |
| `--explain-repair-policy` | N | Y | 仅 Rust：打印修复策略后退出 |

### 技能与提示模板

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--skill <path>` | Y | Y | 加载技能（可重复） |
| `--no-skills` | Y | Y | 禁用技能发现 |
| `--prompt-template <path>` | Y | Y | 加载模板（可重复） |
| `--no-prompt-templates` | Y | Y | 禁用模板发现 |

### 主题

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--theme <name>` | Y | Y | 选择活跃主题 |
| `--theme-path <dir>` | N | Y | 仅 Rust：新增主题发现路径 |
| `--no-themes` | Y | Y | 禁用主题发现 |

### 导出与信息

| 标志 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| `--export <path>` | Y | Y | 导出会话为 HTML |
| `--help, -h` | Y | Y | 显示帮助 |
| `--version, -v` | Y | Y | 显示版本 |

### 位置参数

| 功能 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `@file` 引用 | Y | Y | 包含文件内容 |
| 消息参数 | Y | Y | 非 @ 字符串作为消息 |

---

## 2. 子命令（包管理）

| 命令 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `install <source> [-l]` | Y | Y | npm:/git:/local 来源 |
| `remove <source> [-l]` | Y | Y | 从设置中移除 |
| `update [source]` | Y | Y | 更新全部或指定项 |
| `list` | Y | Y | 列出已安装包 |
| `config` | Y | Y | 打开配置 |
| `update-index` | N | Y | 仅 Rust：刷新扩展索引缓存 |
| `info <name>` | N | Y | 仅 Rust：扩展详情 |
| `search <query>` | N | Y | 仅 Rust：搜索扩展 |
| `doctor [path]` | N | Y | 仅 Rust：诊断环境健康 |

---

## 3. 执行模式

| 模式 | TS Pi | Rust Pi | 备注 |
|------|-------|---------|-------|
| 交互式（默认 TUI） | Y | Y | 完整终端 UI |
| 打印（`-p`） | Y | Y | 单次、非交互 |
| RPC（`--mode rpc`） | Y | Y | 基于 stdin/stdout 的 JSON 协议 |
| JSON（`--mode json`） | Y | Y | JSON 事件行 |
| 文本（`--mode text`） | Y | Y | 纯文本输出 |

---

## 4. 内置工具

| 工具 | TS Pi | Rust Pi | 默认？ | 备注 |
|------|-------|---------|----------|-------|
| `read` | Y | Y | Y | 文件/图像读取 |
| `bash` | Y | Y | Y | Shell 命令执行 |
| `edit` | Y | Y | Y | 字符串替换编辑 |
| `write` | Y | Y | Y | 文件创建/覆盖 |
| `grep` | Y | Y | N | 带上下文的内容搜索 |
| `find` | Y | Y | N | 按模式发现文件 |
| `ls` | Y | Y | N | 目录清单 |

### 工具限制

| 限制 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| DEFAULT_MAX_LINES | 2000 | 2000 | 一致 |
| DEFAULT_MAX_BYTES | 1,000,000 | 1,000,000 | 一致 |
| GREP_MAX_LINE_LENGTH | ? | 500 | 需验证 TS |
| DEFAULT_GREP_LIMIT | ? | 100 | 需验证 TS |
| DEFAULT_FIND_LIMIT | ? | 1000 | 需验证 TS |
| DEFAULT_LS_LIMIT | ? | 500 | 需验证 TS |
| DEFAULT_BASH_TIMEOUT_SECS | ? | 120 | 需验证 TS |
| IMAGE_MAX_BYTES | ? | 4.5MB | 需验证 TS |
| READ_TOOL_MAX_BYTES | ? | 100MB | 需验证 TS |

---

## 5. 斜杠命令（交互模式）

| 命令 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `/settings` | Y | ? | TS 设置菜单 |
| `/model` | Y | Y | 模型选择器 |
| `/scoped-models` | Y | ? | 启用/禁用用于循环的模型 |
| `/export` | Y | Y | 导出会话为 HTML |
| `/share` | Y | ? | 分享为 GitHub gist |
| `/copy` | Y | ? | 复制最后一条消息到剪贴板 |
| `/name` | Y | ? | 设置会话名称 |
| `/session` | Y | ? | 会话信息/统计 |
| `/changelog` | Y | ? | 显示变更日志 |
| `/hotkeys` | Y | ? | 显示快捷键 |
| `/fork` | Y | ? | 从上一条消息派生 |
| `/tree` | Y | Y | 导航会话树 |
| `/login` | Y | ? | OAuth 登录 |
| `/logout` | Y | ? | OAuth 登出 |
| `/new` | Y | ? | 启动新会话 |
| `/compact` | Y | Y | 手动压缩 |
| `/resume` | Y | ? | 恢复其他会话 |
| `/reload` | Y | ? | 重载扩展/技能/提示/主题 |
| `/help` | Y | Y | 显示帮助 |
| `/clear` | Y | Y | 清空消息 |
| `/exit` | Y | Y | 退出应用 |
| 动态提示模板 | Y | Y | 注册为 `/template-name` |
| 动态技能命令 | Y | Y | 注册为 `/skill:name` |
| 扩展命令 | Y | Y | 动态注册 |

---

## 6. 配置项（settings.json）

### 外观

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `theme` | Y | Y | 主题名称/路径 |
| `hideThinkingBlock` | Y | Y | 在 TUI 中隐藏思考块 |
| `showHardwareCursor` | Y | Y | 显示硬件光标 |

### 模型默认值

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `defaultProvider` | Y | Y | 默认 LLM 提供方 |
| `defaultModel` | Y | Y | 默认模型 ID |
| `defaultThinkingLevel` | Y | Y | 默认思考级别 |
| `enabledModels` | Y | Y | 用于循环的模型模式 |

### 消息队列

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `steeringMode` | Y | Y | all / one-at-a-time |
| `followUpMode` | Y | Y | all / one-at-a-time |

### 终端行为

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `quietStartup` | Y | Y | 不显示变更日志 |
| `collapseChangelog` | Y | Y | 默认折叠 |
| `lastChangelogVersion` | Y | Y | 记录已显示版本 |
| `doubleEscapeAction` | Y | Y | fork/tree/none |
| `editorPaddingX` | Y | Y | 编辑器内边距 |
| `autocompleteMaxVisible` | Y | Y | 最大自动补全建议数 |
| `sessionPickerInput` | N | Y | 仅 Rust：非交互式选择器 |
| `sessionStore` | N | Y | 仅 Rust：jsonl/sqlite 后端 |

### 压缩

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `compaction.enabled` | Y | Y | 启用压缩 |
| `compaction.reserveTokens` | Y (16384) | Y (16384) | 一致 |
| `compaction.keepRecentTokens` | Y (20000) | Y (20000) | 一致 |

### 分支摘要

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `branchSummary.reserveTokens` | Y (16384) | Y | 一致 |

### 重试

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `retry.enabled` | Y | Y | 启用自动重试 |
| `retry.maxRetries` | Y (3) | Y (3) | 一致 |
| `retry.baseDelayMs` | Y (2000) | Y (2000) | 一致 |
| `retry.maxDelayMs` | Y (60000) | Y (60000) | 一致 |

### Shell

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `shellPath` | Y | Y | Shell 可执行文件路径 |
| `shellCommandPrefix` | Y | Y | 命令前缀 |
| `ghPath` | N | Y | 仅 Rust：GitHub CLI 路径 |

### 图像

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `images.autoResize` | Y | Y | 自动缩放大图 |
| `images.blockImages` | Y | Y | 屏蔽所有图像 |

### 终端显示

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `terminal.showImages` | Y | Y | 显示图像 |
| `terminal.clearOnShrink` | Y | Y | 收缩时清屏 |

### 思考预算

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `thinkingBudgets` | Y | Y | 按级别的 token 预算 |

### 包与资源

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `packages` | Y | Y | 包来源 |
| `extensions` | Y | Y | 扩展路径 |
| `skills` | Y | Y | 技能路径 |
| `prompts` | Y | Y | 提示路径 |
| `themes` | Y | Y | 主题路径 |
| `enableSkillCommands` | Y | Y | 启用 /skill 命令 |

### Markdown

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `markdown.codeBlockIndent` | Y | Y | 已验证（渲染缩进） |

### 扩展策略（仅 Rust）

| 配置项 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `extensionPolicy.profile` | N | Y | safe/balanced/permissive |
| `extensionPolicy.allowDangerous` | N | Y | 允许危险能力 |
| `repairPolicy.mode` | N | Y | off/suggest/auto-safe/auto-strict |
| `extensionRisk.enabled` | N | Y | 运行时风险控制器 |
| `extensionRisk.alpha` | N | Y | 第一类错误目标 |
| `extensionRisk.windowSize` | N | Y | 滑动窗口 |
| `extensionRisk.ledgerLimit` | N | Y | 最大条目数 |
| `extensionRisk.decisionTimeoutMs` | N | Y | 决策超时 |
| `extensionRisk.failClosed` | N | Y | 失效关闭行为 |
| `extensionRisk.enforce` | N | Y | 强制执行决策 |

---

## 7. 环境变量

### 提供方 API 密钥

| 变量 | TS Pi | Rust Pi | 备注 |
|----------|-------|---------|-------|
| `ANTHROPIC_API_KEY` | Y | Y | |
| `ANTHROPIC_OAUTH_TOKEN` | Y | ? | 需在 Rust 中验证 |
| `OPENAI_API_KEY` | Y | Y | |
| `GOOGLE_API_KEY` / `GEMINI_API_KEY` | Y | Y | TS 使用 GEMINI_，Rust 使用 GOOGLE_ |
| `AZURE_OPENAI_API_KEY` | Y | Y | |
| `AZURE_OPENAI_BASE_URL` | Y | ? | 需验证 |
| `AZURE_OPENAI_RESOURCE_NAME` | Y | ? | 需验证 |
| `AZURE_OPENAI_API_VERSION` | Y | ? | 需验证 |
| `AZURE_OPENAI_DEPLOYMENT_NAME_MAP` | Y | ? | 需验证 |
| `AWS_ACCESS_KEY_ID` | Y | Y | Bedrock |
| `AWS_SECRET_ACCESS_KEY` | Y | Y | Bedrock |
| `AWS_BEARER_TOKEN_BEDROCK` | Y | ? | 需验证 |
| `AWS_REGION` | Y | ? | 需验证 |
| `AWS_PROFILE` | Y | ? | 需验证 |
| `GROQ_API_KEY` | Y | Y | |
| `CEREBRAS_API_KEY` | Y | Y | |
| `XAI_API_KEY` | Y | Y | |
| `OPENROUTER_API_KEY` | Y | Y | |
| `MISTRAL_API_KEY` | Y | Y | |
| `TOGETHER_API_KEY` | Y | Y | |
| `DEEPSEEK_API_KEY` | Y | Y | |
| `PERPLEXITY_API_KEY` | Y | Y | |
| `COHERE_API_KEY` | N | Y | 仅 Rust 提供方 |
| `AI_GATEWAY_API_KEY` | Y | ? | Vercel AI Gateway |
| `ZAI_API_KEY` | Y | ? | ZAI 提供方 |
| `MINIMAX_API_KEY` | Y | ? | MiniMax 提供方 |
| `KIMI_API_KEY` | Y | ? | Kimi 提供方 |
| `MOONSHOT_API_KEY` | N | Y | 仅 Rust |
| `DASHSCOPE_API_KEY` | N | Y | 仅 Rust（Qwen） |
| `FIREWORKS_API_KEY` | N | Y | 仅 Rust |
| `DEEPINFRA_API_KEY` | N | Y | 仅 Rust |
| `GITLAB_API_TOKEN` | N | Y | 仅 Rust（GitLab Duo） |

### 配置变量

| 变量 | TS Pi | Rust Pi | 备注 |
|----------|-------|---------|-------|
| `PI_CODING_AGENT_DIR` | Y | Y | 配置根目录 |
| `PI_PACKAGE_DIR` | Y | Y | 包目录 |
| `PI_SESSIONS_DIR` | N | Y | 仅 Rust |
| `PI_CONFIG_PATH` | N | Y | 仅 Rust |
| `PI_SHARE_VIEWER_URL` | Y | ? | 分享查看器基础 URL |

### 开发 / 测试

| 变量 | TS Pi | Rust Pi | 备注 |
|----------|-------|---------|-------|
| `PI_TEST_MODE` | Y | Y | 确定性渲染 |
| `PI_TIMING` | Y | ? | 计时输出 |
| `PI_SKIP_VERSION_CHECK` | Y | ? | 跳过版本检查 |
| `PI_HARDWARE_CURSOR` | Y | ? | 硬件光标 |
| `PI_CLEAR_ON_SHRINK` | Y | ? | 收缩时清屏 |
| `VCR_MODE` | N | Y | 仅 Rust VCR 测试 |
| `VCR_CASSETTE_DIR` | N | Y | 仅 Rust VCR 测试 |
| `PI_VCR_TEST_NAME` | N | Y | 仅 Rust VCR 测试 |
| `PI_EXTENSION_ALLOW_DANGEROUS` | N | Y | 仅 Rust |
| `PI_REPAIR_POLICY` | N | Y | 仅 Rust |
| `PI_EXT_COMPAT_SCAN` | N | Y | 仅 Rust |

---

## 8. 提供方

| 提供方 | TS Pi | Rust Pi | 备注 |
|----------|-------|---------|-------|
| Anthropic (Claude) | Y | Y | 主要提供方 |
| OpenAI (GPT) | Y | Y | Chat Completions API |
| OpenAI Responses | Y | Y | Responses API |
| Google Gemini | Y | Y | |
| Azure OpenAI | Y | Y | |
| Amazon Bedrock | Y | Y | |
| Google Vertex AI | Y | Y | |
| Groq | Y | Y | |
| Cerebras | Y | Y | |
| xAI (Grok) | Y | Y | |
| OpenRouter | Y | Y | |
| Mistral | Y | Y | |
| Together AI | Y | Y | |
| DeepSeek | Y | Y | |
| Perplexity | Y | Y | |
| Cohere | N | Y | 仅 Rust |
| GitLab Duo | N | Y | 仅 Rust |
| GitHub Copilot | Y | Y | |
| Ollama | N | Y | 仅 Rust（本地） |
| Vercel AI Gateway | Y | ? | 需验证 |
| ZAI | Y | ? | 需验证 |
| MiniMax | Y | ? | 需验证 |
| Kimi | Y | ? | 需验证 |
| Moonshot | N | Y | 仅 Rust |
| DashScope/Qwen | N | Y | 仅 Rust |
| Fireworks | N | Y | 仅 Rust |
| DeepInfra | N | Y | 仅 Rust |
| 扩展提供方 | Y | Y | 通过 streamSimple 桥接 |

---

## 9. 智能体事件

| 事件 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| `agent_start` | Y | Y | 智能体循环开始 |
| `agent_end` | Y | Y | 智能体循环结束 |
| `turn_start` | Y | Y | 轮次开始 |
| `turn_end` | Y | Y | 轮次结束 |
| `content_block_start` | Y | Y | 内容块开始 |
| `content_block_delta` | Y | Y | 内容块增量 |
| `content_block_end` | Y | Y | 内容块结束 |
| `tool_call_start` | Y | Y | 工具调用开始 |
| `tool_call_end` | Y | Y | 工具调用结束 |
| `tool_execution_start` | Y | Y | 工具执行开始 |
| `tool_execution_update` | Y | Y | 工具执行流式更新 |
| `tool_execution_end` | Y | Y | 工具执行结束 |
| `message` | Y | Y | 新增消息 |
| `message_update` | Y | Y | 消息更新 |
| `error` | Y | Y | 发生错误 |
| `auto_compaction_start` | Y | Y | 自动压缩生命周期开始 |
| `auto_compaction_end` | Y | Y | 自动压缩生命周期结束 |
| `auto_retry_start` | Y | Y | 自动重试生命周期开始 |
| `auto_retry_end` | Y | Y | 自动重试生命周期结束 |

---

## 10. 扩展事件（钩子点）

### 会话事件

| 事件 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| `session_start` | Y | Y | 初始会话加载 |
| `session_before_switch` | Y | ? | 可取消 |
| `session_switch` | Y | ? | 切换后 |
| `session_before_fork` | Y | ? | 可取消 |
| `session_fork` | Y | ? | 派生后 |
| `session_before_compact` | Y | ? | 可取消、可自定义 |
| `session_compact` | Y | ? | 压缩后 |
| `session_before_tree` | Y | ? | 可取消 |
| `session_tree` | Y | ? | 树导航后 |
| `session_shutdown` | Y | Y | 退出时 |
| `resources_discover` | Y | ? | 资源发现 |

### 智能体事件

| 事件 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| `context` | Y | Y | LLM 调用前（可修改） |
| `before_agent_start` | Y | ? | 可取消 |
| `agent_start` | Y | Y | 循环开始 |
| `agent_end` | Y | Y | 循环结束 |
| `turn_start` | Y | Y | 轮次开始 |
| `turn_end` | Y | Y | 轮次结束 |
| `model_select` | Y | ? | 模型选择 |

### 工具事件

| 事件 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| `tool_call` | Y | Y | 执行前（可阻断） |
| `tool_result` | Y | Y | 执行后（可修改） |

### 用户事件

| 事件 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| `user_bash` | Y | ? | 带 ! 前缀的用户 shell |
| `input` | Y | ? | 用户输入（可转换） |

---

## 11. RPC 协议命令

| 命令 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `prompt` | Y | ? | 初始提示 |
| `steer` | Y | Y | 使用用户消息引导 |
| `follow_up` / `queue-follow-up` | Y | Y | 排队跟进 |
| `abort` | Y | Y | 中止当前操作 |
| `new_session` | Y | ? | 启动新会话 |
| `get_state` / `get-state` | Y | Y | 获取会话状态 |
| `set_model` / `set-model` | Y | Y | 设置活跃模型 |
| `cycle_model` | Y | ? | 切换到下一模型 |
| `get_available_models` | Y | ? | 列出模型 |
| `set_thinking_level` | Y | ? | 设置思考级别 |
| `cycle_thinking_level` | Y | ? | 循环思考级别 |
| `set_steering_mode` | Y | ? | 设置引导模式 |
| `set_follow_up_mode` | Y | ? | 设置跟进模式 |
| `compact` | Y | Y | 压缩会话 |
| `set_auto_compaction` / `set-auto-compaction` | Y | Y | 启用/禁用压缩 |
| `set_auto_retry` / `set-auto-retry` | Y | Y | 启用/禁用重试 |
| `abort_retry` | Y | ? | 中止重试 |
| `bash` | Y | ? | 执行 bash |
| `abort_bash` | Y | ? | 中止 bash |
| `get_session_stats` | Y | ? | 会话统计 |
| `export_html` | Y | ? | 导出为 HTML |
| `switch_session` | Y | ? | 切换会话 |
| `fork` | Y | ? | 从条目派生 |
| `get_fork_messages` | Y | ? | 获取派生消息 |
| `get_last_assistant_text` | Y | ? | 最后一条助手文本 |
| `set_session_name` | Y | ? | 设置名称 |
| `get_messages` | Y | ? | 获取全部消息 |
| `get_commands` | Y | ? | 列出命令 |
| `query-completion` | N | Y | 仅 Rust：补全查询 |

### RPC 事件（响应）

| 事件 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| 智能体事件（流式） | Y | Y | 全部智能体事件 |
| `extension_ui_request` | Y | Y | 由 `tests/e2e_rpc.rs` + `tests/json_mode_parity.rs` 覆盖 |
| `extension_ui_response` | Y | Y | 由 `tests/e2e_rpc.rs` 覆盖（成功与负向路径） |
| `extension_error` | N | Y | 仅 Rust：在扩展分发/运行时失败时发出 |

---

## 12. 会话条目类型

| 条目类型 | TS Pi | Rust Pi | 备注 |
|------------|-------|---------|-------|
| `session`（头部） | Y | Y | Version 3 格式 |
| `message` | Y | Y | 聊天消息 |
| `model_change` | Y | Y | 模型变更 |
| `thinking_level_change` | Y | Y | 思考级别变更 |
| `compaction` | Y | Y | 上下文压缩 |
| `branch_summary` | Y | Y | 分支摘要 |
| `custom` | Y | Y | 扩展数据 |
| `label` | N | Y | 仅 Rust：会话标签 |
| `branch` | N | Y | 仅 Rust：分支标记 |
| `note` | N | Y | 仅 Rust：自定义备注 |

---

## 13. 思考级别

| 级别 | TS Pi | Rust Pi | 备注 |
|-------|-------|---------|-------|
| `off` | Y | Y | 无思考 |
| `minimal` | Y | Y | 轻量推理 |
| `low` | Y | Y | 低推理 |
| `medium` | Y (默认) | Y (默认) | 均衡推理 |
| `high` | Y | Y | 深度推理 |
| `xhigh` | Y | Y | 最大推理 |

### 别名（仅 Rust）

| 别名 | 映射至 |
|-------|---------|
| `none`、`0` | off |
| `min` | minimal |
| `1` | low |
| `med`、`2` | medium |
| `3` | high |
| `4` | xhigh |

---

## 14. 快捷键（交互模式）

| 操作 | TS Pi | Rust Pi | 默认按键 | 备注 |
|--------|-------|---------|-------------|-------|
| 中断 | Y | Y | Escape | |
| 清空 | Y | Y | Ctrl+C | |
| 退出 | Y | Y | Ctrl+D | 为空时 |
| 挂起 | Y | Y | Ctrl+Z | |
| 循环思考 | Y | Y | Shift+Tab | |
| 正向循环模型 | Y | Y | Ctrl+P | |
| 反向循环模型 | Y | Y | Shift+Ctrl+P | |
| 选择模型 | Y | Y | Ctrl+L | |
| 展开工具 | Y | ? | Ctrl+O | 需验证 |
| 切换思考 | Y | ? | Ctrl+T | 需验证 |
| 切换会话命名过滤 | Y | ? | Ctrl+N | 需验证 |
| 外部编辑器 | Y | ? | Ctrl+G | 需验证 |
| 跟进 | Y | Y | Alt+Enter | |
| 出队 | Y | ? | Alt+Up | 需验证 |
| 粘贴图像 | Y | ? | Ctrl+V | 需验证 |
| 新建会话 | Y | ? | （无） | 需验证 |
| 树 | Y | ? | （无） | 需验证 |
| 派生 | Y | ? | （无） | 需验证 |

### 自定义

| 功能 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| `keybindings.json` | Y | Y | `~/.pi/agent/keybindings.json` |

---

## 15. 扩展 API 表面

### 注册

| API | TS Pi | Rust Pi | 备注 |
|-----|-------|---------|-------|
| `registerTool()` | Y | Y | 注册自定义工具 |
| `registerCommand()` | Y | Y | 注册斜杠命令 |
| `registerShortcut()` | Y | Y | 注册快捷键 |
| `registerFlag()` | Y | Y | 注册扩展标志 |
| `registerProvider()` | Y | Y | 注册自定义提供方 |

### 会话

| API | TS Pi | Rust Pi | 备注 |
|-----|-------|---------|-------|
| `getState()` | Y | Y | 会话状态 |
| `getMessages()` | Y | Y | 当前消息 |
| `setSessionName()` | Y | Y | 设置会话名称 |
| `setModel()` | Y | Y | 切换模型 |
| `setLabel()` | Y | Y | 标记条目 |
| `sendMessage()` | Y | Y | 发送消息 |
| `appendEntry()` | Y | Y | 添加自定义条目 |
| `getActiveTools()` | Y | Y | 活跃工具列表 |
| `getAllTools()` | Y | Y | 全部工具 |
| `setActiveTools()` | Y | Y | 设置活跃工具 |
| `getThinkingLevel()` | Y | Y | 当前思考 |
| `setThinkingLevel()` | Y | Y | 设置思考 |

### UI

| API | TS Pi | Rust Pi | 备注 |
|-----|-------|---------|-------|
| `ui.select()` | Y | Y | 选择对话框 |
| `ui.confirm()` | Y | Y | 确认对话框 |
| `ui.input()` | Y | Y | 输入对话框 |
| `ui.notify()` | Y | Y | 通知 |
| `ui.setStatus()` | Y | Y | 状态栏 |
| `ui.setWorkingMessage()` | Y | ? | 工作消息 |
| `ui.setWidget()` | Y | Y | 自定义小组件 |
| `ui.setFooter()` | Y | ? | 自定义页脚 |
| `ui.setHeader()` | Y | ? | 自定义页首 |
| `ui.setTitle()` | Y | Y | 窗口标题 |
| `ui.custom()` | Y | ? | 自定义组件 |
| `ui.setEditorText()` | Y | ? | 设置编辑器文本 |
| `ui.getEditorText()` | Y | ? | 获取编辑器文本 |
| `ui.editor()` | Y | ? | 完整编辑器对话框 |
| `ui.theme` | Y | Y | 当前主题 |
| `ui.getAllThemes()` | Y | ? | 主题列表 |
| `ui.getTheme()` | Y | ? | 按名称获取主题 |
| `ui.setTheme()` | Y | ? | 设置活跃主题 |

### 宿主调用

| API | TS Pi | Rust Pi | 备注 |
|-----|-------|---------|-------|
| `exec()` | Y | Y | 执行 shell 命令 |
| `events` 总线 | Y | Y | 共享事件总线 |
| `fetch()` | Y | Y | HTTP 请求 |
| `read()` | Y | Y | 读取文件 |
| `write()` | Y | Y | 写入文件 |
| `grep()` | Y | Y | 搜索文件 |
| `find()` | Y | Y | 查找文件 |
| `ls()` | Y | Y | 列出目录 |

### 能力策略

| 功能 | TS Pi | Rust Pi | 备注 |
|---------|-------|---------|-------|
| 策略配置 | N | Y | 仅 Rust：safe/balanced/permissive |
| 按能力审计 | N | Y | 仅 Rust：细粒度控制 |
| 风险控制器 | N | Y | 仅 Rust：统计风险 |

---

## 16. 交互式 UI 组件

| 组件 | TS Pi | Rust Pi | 备注 |
|-----------|-------|---------|-------|
| 模型选择器 | Y | Y | |
| 作用域模型选择器 | Y | ? | 需验证 |
| 思考选择器 | Y | Y | |
| 会话选择器/拾取器 | Y | Y | |
| 树选择器 | Y | Y | |
| 设置选择器 | Y | ? | 需验证 |
| 登录对话框 | Y | ? | OAuth 流程 |
| 配置选择器 | Y | ? | 包资源配置 |
| 工具执行展示 | Y | Y | |
| Bash 执行展示 | Y | Y | |
| 技能调用展示 | Y | Y | |
| 扩展编辑器 | Y | ? | 自定义 UI |
| 自动补全 | Y | Y | @file 与 /commands |

---

## 差异总结

### TS Pi 中存在但 Rust Pi 缺失/待确认的功能

1. **智能体事件**：`auto_compaction_start/end`、`auto_retry_start/end`（bd-2ilgm 已覆盖）
2. **RPC 命令**：多项 TS RPC 命令待验证（`cycle_model`、`set_thinking_level`、`cycle_thinking_level`、`bash`、`abort_bash`、`get_session_stats`、`fork`、`get_messages` 等）
3. **扩展事件**：若干钩子点待验证（`session_before_*`、`model_select`、`user_bash`、`input`）
4. **扩展 UI**：若干 UI 方法待验证（`setWorkingMessage`、`setFooter`、`setHeader`、`custom`、`setEditorText`、`editor`）
5. **提供方支持**：Vercel AI Gateway、ZAI、MiniMax、Kimi——尚不明确是否已在 Rust 中

### 仅 Rust Pi 具备的功能（Rust 独有）

1. **CLI 标志**：`--list-providers`、`--extension-policy`、`--explain-extension-policy`、`--repair-policy`、`--explain-repair-policy`、`--theme-path`
2. **子命令**：`update-index`、`info`、`search`、`doctor`
3. **提供方**：Cohere、GitLab Duo、Ollama、Moonshot、DashScope、Fireworks、DeepInfra
4. **扩展安全**：能力策略配置、风险控制器、修复策略
5. **会话**：标签、分支标记、备注条目类型；SQLite 后端
6. **配置**：sessionStore、sessionPickerInput、ghPath、extensionPolicy、repairPolicy、extensionRisk
7. **思考别名**：数字别名（0-4）与简写（min、med）
8. **VCR 测试设施**：VCR_MODE、VCR_CASSETTE_DIR、PI_VCR_TEST_NAME

---

## 待调研项（标记为 ?）

本矩阵中每个 `?` 代表在一侧实现中存在但需在另一侧验证的表面。后续任务应系统性地消除这些未知项：
1. 在 Rust 代码库中 grep 每个 TS 功能名
2. 在 TS 源码中检查每个仅 Rust 功能名
3. 用 Y/N/P 更新本矩阵中的每一项

待解决的未知项总数：约 60 项
