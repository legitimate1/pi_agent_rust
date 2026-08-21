# 现有 Pi 结构与架构

> **阅读本文档后，你无需再查阅遗留的 TypeScript 代码。**

本文档是 pi-mono Rust 移植版的权威规范。

---

## 目录

1. [项目概览](#1-项目概览)
2. [消息类型与内容块](#2-消息类型与内容块)
3. [流式事件](#3-流式事件)
4. [提供方接口](#4-提供方接口)
5. [工具系统](#5-工具系统)
6. [会话文件格式](#6-会话文件格式)
7. [配置](#7-配置)
8. [认证存储](#8-认证存储)
9. [CLI 命令与标志](#9-cli-命令与标志)
10. [执行流程](#10-执行流程)
11. [RPC 模式协议（基于 stdio 的 JSON）](#11-rpc-模式协议基于-stdio-的-json)
12. [扩展 API](#12-扩展-api)
13. [资源系统](#13-资源系统)

---

## 1. 项目概览

Pi 是一个 AI 编程智能体平台，核心组件如下：

| 组件 | TypeScript 包 | Rust 等效模块 |
|-----------|-------------------|-----------------|
| LLM 提供方抽象 | `@mariozechner/pi-ai` | `pi::provider` 模块 |
| 智能体运行时 | `@mariozechner/pi-agent` | `pi::agent` 模块 |
| CLI 应用 | `@mariozechner/pi-coding-agent` | `pi` 二进制 |
| 终端 UI | `@mariozechner/pi-tui` | `pi::tui` 模块 |

### 关键统计（TypeScript）
- **核心遗留工具集**：read、bash、edit、write
- **20+ LLM 提供方**：Anthropic、OpenAI、Google、Bedrock 等
- **会话格式版本**：3
- **默认启用的工具**：read、bash、edit、write

---

## 2. 消息类型与内容块

### 2.1 消息联合类型

```rust
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    ToolResult(ToolResultMessage),
}
```

### 2.2 用户消息

```rust
pub struct UserMessage {
    pub content: UserContent,  // String or Vec<ContentBlock>
    pub timestamp: i64,        // Unix milliseconds
}

pub enum UserContent {
    Text(String),
    Blocks(Vec<ContentBlock>),  // TextContent | ImageContent only
}
```

### 2.3 助手消息

```rust
pub struct AssistantMessage {
    pub content: Vec<AssistantContentBlock>,  // Text | Thinking | ToolCall
    pub api: String,                          // e.g., "anthropic-messages"
    pub provider: String,                     // e.g., "anthropic"
    pub model: String,                        // Model ID
    pub usage: Usage,
    pub stop_reason: StopReason,
    pub error_message: Option<String>,
    pub timestamp: i64,
}
```

### 2.4 工具结果消息

```rust
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<ContentBlock>,  // TextContent | ImageContent
    pub details: Option<serde_json::Value>,
    pub is_error: bool,
    pub timestamp: i64,
}
```

### 2.5 停止原因

```rust
pub enum StopReason {
    Stop,      // Normal completion
    Length,    // Max tokens reached
    ToolUse,   // Tool call pending
    Error,     // Error occurred
    Aborted,   // User cancelled
}
```

### 2.6 内容块

```rust
pub enum ContentBlock {
    Text(TextContent),
    Thinking(ThinkingContent),
    Image(ImageContent),
    ToolCall(ToolCall),
}

pub struct TextContent {
    pub text: String,
    pub text_signature: Option<String>,  // Provider-specific
}

pub struct ThinkingContent {
    pub thinking: String,
    pub thinking_signature: Option<String>,  // For replay
}

pub struct ImageContent {
    pub data: String,       // Base64 encoded
    pub mime_type: String,  // "image/jpeg", "image/png", etc.
}

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    pub thought_signature: Option<String>,  // Google-specific
}
```

### 2.7 用量追踪

```rust
pub struct Usage {
    pub input: u64,        // Input tokens (excluding cache read)
    pub output: u64,       // Output tokens
    pub cache_read: u64,   // Tokens read from cache
    pub cache_write: u64,  // Tokens written to cache
    pub total_tokens: u64,
    pub cost: Cost,
}

pub struct Cost {
    pub input: f64,       // Dollars
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}
```

---

## 3. 流式事件

### 3.1 事件类型

```rust
pub enum StreamEvent {
    Start { partial: AssistantMessage },

    TextStart { content_index: usize, partial: AssistantMessage },
    TextDelta { content_index: usize, delta: String, partial: AssistantMessage },
    TextEnd { content_index: usize, content: String, partial: AssistantMessage },

    ThinkingStart { content_index: usize, partial: AssistantMessage },
    ThinkingDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ThinkingEnd { content_index: usize, content: String, partial: AssistantMessage },

    ToolCallStart { content_index: usize, partial: AssistantMessage },
    ToolCallDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ToolCallEnd { content_index: usize, tool_call: ToolCall, partial: AssistantMessage },

    Done { reason: StopReason, message: AssistantMessage },
    Error { reason: StopReason, error: AssistantMessage },
}
```

### 3.2 事件序列

**文本响应：**
```
Start → TextStart → TextDelta* → TextEnd → Done(Stop)
```

**工具调用：**
```
Start → ToolCallStart → ToolCallDelta* → ToolCallEnd → Done(ToolUse)
```

**带思考过程：**
```
Start → ThinkingStart → ThinkingDelta* → ThinkingEnd → TextStart → ... → Done
```

---

## 4. 提供方接口

### 4.1 提供方 Trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn api(&self) -> &str;

    async fn stream(
        &self,
        context: &Context,
        options: &StreamOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>>;
}
```

### 4.2 上下文

```rust
pub struct Context {
    pub system_prompt: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
}
```

### 4.3 流式选项

```rust
pub struct StreamOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub api_key: Option<String>,
    pub cache_retention: CacheRetention,
    pub session_id: Option<String>,
    pub headers: HashMap<String, String>,
    pub thinking_level: Option<ThinkingLevel>,
    pub thinking_budgets: Option<ThinkingBudgets>,
}

pub enum CacheRetention {
    None,
    Short,
    Long,  // 1 hour TTL on Anthropic
}

pub enum ThinkingLevel {
    Off,
    Minimal,  // 1024 tokens
    Low,      // 2048 tokens
    Medium,   // 8192 tokens
    High,     // 16384 tokens
    XHigh,    // Model max
}

pub struct ThinkingBudgets {
    pub minimal: u32,  // Default: 1024
    pub low: u32,      // Default: 2048
    pub medium: u32,   // Default: 8192
    pub high: u32,     // Default: 16384
}
```

### 4.4 模型定义

```rust
pub struct Model {
    pub id: String,
    pub name: String,
    pub api: String,              // "anthropic-messages", "openai-completions", etc.
    pub provider: String,         // "anthropic", "openai", etc.
    pub base_url: String,
    pub reasoning: bool,          // Supports thinking/reasoning
    pub input: Vec<InputType>,    // ["text", "image"]
    pub cost: ModelCost,
    pub context_window: u32,
    pub max_tokens: u32,
    pub headers: HashMap<String, String>,
}

pub struct ModelCost {
    pub input: f64,       // $/million tokens
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}
```

```rust
pub enum InputType {
    Text,
    Image,
}
```

### 4.5 已知 API

```rust
pub enum Api {
    AnthropicMessages,
    OpenAICompletions,
    OpenAIResponses,
    AzureOpenAIResponses,
    BedrockConverseStream,
    GoogleGenerativeAI,
    GoogleGeminiCli,
    GoogleVertex,
    Custom(String),
}
```

### 4.6 已知提供方

```rust
pub enum KnownProvider {
    Anthropic,
    OpenAI,
    Google,
    GoogleVertex,
    AmazonBedrock,
    AzureOpenAI,
    GithubCopilot,
    XAI,
    Groq,
    Cerebras,
    OpenRouter,
    Mistral,
    // ... more
}
```

---

## 5. 工具系统

### 5.1 工具 Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn label(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;  // JSON Schema

    async fn execute(
        &self,
        tool_call_id: &str,
        input: serde_json::Value,
        on_update: Option<Box<dyn Fn(ToolUpdate) + Send>>,
    ) -> Result<ToolOutput>;
}

pub struct ToolOutput {
    pub content: Vec<ContentBlock>,
    pub details: Option<serde_json::Value>,
}

pub struct ToolUpdate {
    pub content: Vec<ContentBlock>,
    pub details: Option<serde_json::Value>,
}
```

### 5.2 提供给 API 的工具定义

```rust
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
}
```

### 5.3 内置工具

#### READ 工具

**用途：** 读取文件内容（文本或图片）

**参数：**
```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string", "description": "File path (relative or absolute)" },
    "offset": { "type": "integer", "description": "1-indexed line to start from" },
    "limit": { "type": "integer", "description": "Max lines to read" }
  },
  "required": ["path"]
}
```

**行为：**
- 截断：2000 行或 50KB（以先达到者为准）
- 截断方式：`truncate_head`（保留开头）
- 图片支持：jpg、png、gif、webp → 转为 base64，可选缩放到 2000x2000
- 路径支持 `~` 展开

**错误条件：**
- 路径不存在
- 权限被拒绝
- 起始行超出文件末尾

---

#### BASH 工具

**用途：** 执行 shell 命令

**参数：**
```json
{
  "type": "object",
  "properties": {
    "command": { "type": "string", "description": "Bash command to execute" },
    "timeout": { "type": "integer", "description": "Timeout in seconds" }
  },
  "required": ["command"]
}
```

**行为：**
- 通过 `on_update` 回调流式输出（滚动保留最后 100KB）
- 截断：2000 行或 50KB（以先达到者为准）
- 截断方式：`truncate_tail`（保留末尾，便于查看错误）
- 若输出 > 50KB 则创建临时文件（路径位于 `details.full_output_path`）
- 继承 shell 环境
- 可选命令前缀（例如 "shopt -s expand_aliases"）

**错误条件：**
- 非零退出码 → 返回错误并附带输出 + "Command exited with code X"
- 超时 → 返回错误并附带输出 + "Command timed out after X seconds"
- 中止 → 错误 "Command aborted"
- 超时/中止时会终止整个进程树

---

#### WRITE 工具

**用途：** 创建或覆盖文件

**参数：**
```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string", "description": "File path" },
    "content": { "type": "string", "description": "Content to write" }
  },
  "required": ["path", "content"]
}
```

**行为：**
- 递归创建父目录
- UTF-8 编码
- 输出："Successfully wrote X bytes to path"

---

#### EDIT 工具

**用途：** 替换文件中的精确文本

**参数：**
```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string", "description": "File path" },
    "oldText": { "type": "string", "description": "Exact text to find" },
    "newText": { "type": "string", "description": "Replacement text" }
  },
  "required": ["path", "oldText", "newText"]
}
```

**匹配算法：**
1. 尝试精确匹配
2. 回退到模糊匹配：
   - 按行去除尾随空白
   - 标准化智能引号（U+2018-U+201F → ' 或 "）
   - 标准化破折号（U+2010-U+2015、U+2212 → -）
   - 标准化特殊空格（U+00A0、U+2002-U+200A 等 → 空格）

**校验：**
- 必须恰好找到一次 `oldText`
- 必须实际发生变更（old != new）

**换行符处理：**
- 检测原始换行符（CRLF vs LF）
- 内部统一为 LF
- 输出时恢复原始换行符

**BOM 处理：**
- 匹配前去除 UTF-8 BOM
- 输出时恢复 BOM

**输出包含：**
- 成功消息
- 带行号的统一 diff
- 首个变更行号

---

#### GREP 工具

**用途：** 搜索文件内容（基于 ripgrep）

**参数：**
```json
{
  "type": "object",
  "properties": {
    "pattern": { "type": "string", "description": "Regex or literal pattern" },
    "path": { "type": "string", "description": "Directory or file", "default": "." },
    "glob": { "type": "string", "description": "Glob filter (e.g., *.ts)" },
    "ignoreCase": { "type": "boolean", "default": false },
    "literal": { "type": "boolean", "default": false },
    "context": { "type": "integer", "default": 0 },
    "limit": { "type": "integer", "default": 100 }
  },
  "required": ["pattern"]
}
```

**行为：**
- 使用 ripgrep（`rg --json`）
- 遵循 `.gitignore`
- 搜索隐藏文件（`--hidden`）
- 单行截断至 500 字符

**限制：**
- 匹配数限制：100（默认）
- 字节限制：50KB
- 行长度：500 字符

**输出格式：**
```
path/file.ts:42: matching line content
path/file.ts-41- context line
```

---

#### FIND 工具

**用途：** 按 glob 模式查找文件（基于 fd）

**参数：**
```json
{
  "type": "object",
  "properties": {
    "pattern": { "type": "string", "description": "Glob pattern (e.g., *.ts)" },
    "path": { "type": "string", "description": "Directory", "default": "." },
    "limit": { "type": "integer", "default": 1000 }
  },
  "required": ["pattern"]
}
```

**行为：**
- 使用 fd（`fd --glob`）
- 遵循 `.gitignore`
- 搜索隐藏文件（`--hidden`）
- 返回相对路径
- 目录以尾随 `/` 标记

**限制：**
- 结果限制：1000（默认）
- 字节限制：50KB

---

#### LS 工具

**用途：** 列出目录内容

**参数：**
```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string", "description": "Directory", "default": "." },
    "limit": { "type": "integer", "default": 500 }
  }
}
```

**行为：**
- 按字母顺序排序（大小写不敏感）
- 目录以尾随 `/` 标记
- 包含隐藏文件
- 跳过 stat 失败的条目

**限制：**
- 条目限制：500（默认）
- 字节限制：50KB

---

### 5.4 截断常量

```rust
pub const DEFAULT_MAX_LINES: usize = 2000;
pub const DEFAULT_MAX_BYTES: usize = 50 * 1024;  // 50KB
pub const GREP_MAX_LINE_LENGTH: usize = 500;
```

---

## 6. 会话文件格式

### 6.1 文件组织

```
~/.pi/agent/sessions/
└── --{encoded-cwd}--/
    └── {timestamp}_{session-id}.jsonl
```

**CWD 编码：** `--${cwd.replace(/^[/\\]/, "").replace(/[/\\:]/g, "-")}--`
- 示例：`/home/user/project` → `--home-user-project--`

**时间戳格式：** `YYYY-MM-DDTHH-mm-ss.sssZ`（冒号替换为连字符）

### 6.2 会话版本

当前版本：**3**

### 6.3 头部结构

```rust
pub struct SessionHeader {
    pub r#type: String,              // "session"
    pub version: Option<u8>,         // Usually 3
    pub id: String,                  // UUID
    pub timestamp: String,           // ISO-8601
    pub cwd: String,                 // Absolute path
    pub provider: Option<String>,    // Provider name (optional)
    pub model_id: Option<String>,    // Model ID (optional)
    pub thinking_level: Option<String>,  // "off"|"minimal"|... (optional)
    pub parent_session: Option<String>,  // Parent session path (serialized as "branchedFrom"; accepts legacy "parentSession")
}
```

**序列化：**
```json
{
  "type": "session",
  "version": 3,
  "id": "uuid-string",
  "timestamp": "2024-01-15T10:30:45.123Z",
  "cwd": "/absolute/path/to/dir",
  "provider": "anthropic",
  "modelId": "claude-sonnet-4-20250514",
  "thinkingLevel": "medium",
  "branchedFrom": "/path/to/parent.jsonl"
}
```

### 6.4 条目类型

所有条目均包含基础字段：

```rust
pub struct EntryBase {
    pub id: Option<String>,   // 8-char hex or UUID (may be missing on disk)
    pub parent_id: Option<String>,
    pub timestamp: String,    // ISO-8601
}
```

#### 消息条目

```rust
pub struct MessageEntry {
    #[serde(flatten)]
    pub base: EntryBase,      // type: "message"
    pub message: SessionMessage,
}

pub enum SessionMessage {
    User { content: UserContent, timestamp: Option<i64> },
    Assistant { /* full AssistantMessage fields */ },
    ToolResult { tool_use_id: String, content: Vec<ContentBlock>, timestamp: Option<i64> },
    Custom { custom_type: String, content: String, display: bool, details: Option<Value> },
    BashExecution { command: String, output: String, exit_code: i32, /* ... */ },
    BranchSummary { summary: String, from_id: String },
    CompactionSummary { summary: String, tokens_before: u64 },
}
```

#### 模型变更条目

```rust
pub struct ModelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,      // type: "model_change"
    pub provider: String,
    pub model_id: String,
}
```

#### 思考级别变更条目

```rust
pub struct ThinkingLevelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,      // type: "thinking_level_change"
    pub thinking_level: String,  // "off"|"minimal"|"low"|"medium"|"high"|"xhigh"
}
```

#### 压缩条目

```rust
pub struct CompactionEntry {
    #[serde(flatten)]
    pub base: EntryBase,      // type: "compaction"
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    pub details: Option<Value>,
    pub from_hook: Option<bool>,
}
```

#### 分支摘要条目

```rust
pub struct BranchSummaryEntry {
    #[serde(flatten)]
    pub base: EntryBase,      // type: "branch_summary"
    pub from_id: String,      // "root" or entry ID
    pub summary: String,
    pub details: Option<Value>,
    pub from_hook: Option<bool>,
}
```

#### 标签条目

```rust
pub struct LabelEntry {
    #[serde(flatten)]
    pub base: EntryBase,      // type: "label"
    pub target_id: String,
    pub label: Option<String>,  // None = delete label
}
```

#### 会话信息条目

```rust
pub struct SessionInfoEntry {
    #[serde(flatten)]
    pub base: EntryBase,      // type: "session_info"
    pub name: Option<String>,
}
```

### 6.5 树结构

- 每个条目都有 `id` 和 `parent_id`
- 形成链表树以支持分支
- 叶指针追踪当前位置
- 移动叶指针即可实现分支，无需修改历史记录

### 6.6 ID 生成

- 格式：8 字符十六进制（截取自 UUID 切片）
- 碰撞检查：失败 100 次后改用完整 UUID
- 唯一性范围：仅限单会话内

---

## 7. 配置

### 7.1 文件位置

| 类型 | 路径 |
|------|------|
| 全局设置 | `~/.pi/agent/settings.json` |
| 项目设置 | `./.pi/settings.json` |
| 认证 | `~/.pi/agent/auth.json` |
| 模型 | `~/.pi/agent/models.json` |
| 会话 | `~/.pi/agent/sessions/` |

### 7.2 设置结构

```rust
pub struct Settings {
    // Appearance
    pub theme: Option<String>,
    pub hide_thinking_block: Option<bool>,  // Default: false
    pub show_hardware_cursor: Option<bool>, // Default: false

    // Model Configuration
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_thinking_level: Option<String>,
    pub enabled_models: Option<Vec<String>>,  // Patterns for Ctrl+P cycling

    // Message Handling
    pub steering_mode: Option<String>,        // "all" | "one-at-a-time"
    pub follow_up_mode: Option<String>,       // "all" | "one-at-a-time"

    // Terminal Behavior
    pub quiet_startup: Option<bool>,          // Default: false
    pub collapse_changelog: Option<bool>,     // Default: false
    pub double_escape_action: Option<String>, // "fork" | "tree" | "none"
    pub editor_padding_x: Option<u32>,        // Default: 0
    pub autocomplete_max_visible: Option<u32>,// Default: 5

    // Compaction
    pub compaction: Option<CompactionSettings>,

    // Branch Summarization
    pub branch_summary: Option<BranchSummarySettings>,

    // Retry Configuration
    pub retry: Option<RetrySettings>,

    // Shell
    pub shell_path: Option<String>,
    pub shell_command_prefix: Option<String>,

    // Images
    pub images: Option<ImageSettings>,

    // Terminal Display
    pub terminal: Option<TerminalSettings>,

    // Thinking Budgets
    pub thinking_budgets: Option<ThinkingBudgets>,

    // Extensions/Skills/etc.
    pub packages: Option<Vec<PackageSource>>,
    pub extensions: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub prompts: Option<Vec<String>>,
    pub themes: Option<Vec<String>>,
    pub enable_skill_commands: Option<bool>,  // Default: true
}

pub struct CompactionSettings {
    pub enabled: Option<bool>,         // Default: true
    pub reserve_tokens: Option<u32>,   // Default: 16384
    pub keep_recent_tokens: Option<u32>, // Default: 20000
}

pub struct RetrySettings {
    pub enabled: Option<bool>,         // Default: true
    pub max_retries: Option<u32>,      // Default: 3
    pub base_delay_ms: Option<u32>,    // Default: 2000
    pub max_delay_ms: Option<u32>,     // Default: 60000
}

pub struct ImageSettings {
    pub auto_resize: Option<bool>,     // Default: true (2000x2000 max)
    pub block_images: Option<bool>,    // Default: false
}

pub struct TerminalSettings {
    pub show_images: Option<bool>,     // Default: true (if supported)
    pub clear_on_shrink: Option<bool>, // Default: false
}
```

### 7.3 设置优先级

1. CLI 标志（最高）
2. 环境变量
3. 项目设置（`./.pi/settings.json`）
4. 全局设置（`~/.pi/agent/settings.json`）
5. 内置默认值（最低）

### 7.4 环境变量

```rust
// Config paths
PI_CODING_AGENT_DIR     // Override ~/.pi/agent
PI_PACKAGE_DIR          // Override package assets

// API Keys (per provider)
ANTHROPIC_API_KEY
OPENAI_API_KEY
GOOGLE_API_KEY
GOOGLE_CLOUD_API_KEY
AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY
XAI_API_KEY
GROQ_API_KEY
CEREBRAS_API_KEY
OPENROUTER_API_KEY
MISTRAL_API_KEY
// ... etc.
```

---

## 8. 认证存储

### 8.1 认证文件

- **路径：** `~/.pi/agent/auth.json`
- **权限：** `0o600`（仅所有者可读写）
- **锁定：** 文件锁，30 秒陈旧超时

### 8.2 凭证类型

```rust
pub enum AuthCredential {
    ApiKey { key: String },
    OAuth {
        access_token: String,
        refresh_token: String,
        expires: i64,  // Unix milliseconds
    },
}
```

### 8.3 认证文件结构

```json
{
  "anthropic": { "type": "api_key", "key": "sk-ant-..." },
  "github-copilot": {
    "type": "oauth",
    "access_token": "...",
    "refresh_token": "...",
    "expires": 1234567890000
  }
}
```

### 8.4 API 密钥解析优先级

1. 运行时覆盖（`--api-key` 标志）
2. 来自 `auth.json` 的 API 密钥（`type: "api_key"`）
3. 来自 `auth.json` 的 OAuth 令牌（若过期则自动刷新）
4. 环境变量（按提供方区分）
5. 回退解析器（自定义提供方）

### 8.5 OAuth：Anthropic（Claude Pro/Max）

本移植版支持 **Anthropic OAuth** 作为 API 密钥的替代方案。凭证存储在 `auth.json` 中提供方键 `"anthropic"` 下，类型为 `type: "oauth"`。

#### 8.5.1 PKCE

登录流程使用 PKCE（RFC 7636）：
- `verifier`：32 字节随机数，Base64URL（无填充）
- `challenge`：Base64URL(SHA256(verifier))

#### 8.5.2 授权 URL

- `client_id`：`9d1c250a-e61b-44d9-88ed-5944d1962f5e`
- `authorize_url`：`https://claude.ai/oauth/authorize`
- `redirect_uri`：`https://console.anthropic.com/oauth/code/callback`
- `scopes`：`org:create_api_key user:profile user:inference`

查询参数：
- `code=true`
- `client_id=<client_id>`
- `response_type=code`
- `redirect_uri=<redirect_uri>`
- `scope=<scopes>`
- `code_challenge=<challenge>`
- `code_challenge_method=S256`
- `state=<verifier>`

用户在浏览器中完成登录后粘贴以下任一形式：
- 完整的回调 URL，或
- `code#state`，或
- 仅 `code`（此时 `state` 默认为原始 `verifier`）

#### 8.5.3 令牌交换

POST `https://console.anthropic.com/v1/oauth/token`，JSON 体：
```json
{
  "grant_type": "authorization_code",
  "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
  "code": "<code>",
  "state": "<state>",
  "redirect_uri": "https://console.anthropic.com/oauth/code/callback",
  "code_verifier": "<verifier>"
}
```

响应 JSON：
```json
{ "access_token": "...", "refresh_token": "...", "expires_in": 1234 }
```

过期时间以毫秒存储为：
```
expires = now_ms + (expires_in * 1000) - (5 * 60 * 1000)
```

#### 8.5.4 刷新

若 OAuth 凭证已过期，必须自动刷新。

POST `https://console.anthropic.com/v1/oauth/token`，JSON 体：
```json
{
  "grant_type": "refresh_token",
  "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
  "refresh_token": "<refresh_token>"
}
```

刷新后的凭证会覆盖 `auth.json` 中的原有条目并立即持久化。

---

## 9. CLI 命令与标志

### 9.1 包管理命令

| 命令 | 语法 | 描述 |
|---------|--------|-------------|
| `install` | `pi install <source> [-l\|--local]` | 安装扩展/技能/提示模板/主题 |
| `remove` | `pi remove <source> [-l\|--local]` | 从设置中移除 |
| `update` | `pi update [source]` | 更新全部或指定来源 |
| `list` | `pi list` | 列出全局与项目包 |
| `config` | `pi config` | 打开 TUI 配置选择器 |

### 9.2 标志（完整列表）

#### 帮助与版本
| 标志 | 别名 | 类型 | 默认值 |
|------|---------|------|---------|
| `--help` | `-h` | bool | false |
| `--version` | `-v` | bool | false |

#### 模型配置
| 标志 | 类型 | 默认值 | 描述 |
|------|------|---------|-------------|
| `--provider` | string | "google" | 提供方名称 |
| `--model` | string | "gemini-2.5-flash" | 模型 ID |
| `--api-key` | string | None | API 密钥覆盖 |
| `--models` | string | None | 供循环切换的逗号分隔模型模式 |

#### 思考/推理
| 标志 | 类型 | 取值 | 默认值 |
|------|------|--------|---------|
| `--thinking` | enum | off、minimal、low、medium、high、xhigh | None |

#### 系统提示
| 标志 | 类型 | 描述 |
|------|------|-------------|
| `--system-prompt` | string | 覆盖系统提示 |
| `--append-system-prompt` | string | 追加到系统提示 |

#### 会话管理
| 标志 | 别名 | 类型 | 描述 |
|------|---------|------|-------------|
| `--continue` | `-c` | bool | 继续上一会话 |
| `--resume` | `-r` | bool | 从选择器中选择会话 |
| `--session` | | string | 指定会话文件路径 |
| `--session-dir` | | string | 会话存储目录 |
| `--no-session` | | bool | 临时会话 |

#### 模式与输出
| 标志 | 别名 | 类型 | 取值 |
|------|---------|------|--------|
| `--mode` | | enum | text、json、rpc |
| `--print` | `-p` | bool | 非交互模式 |
| `--verbose` | | bool | 详细启动日志 |

#### 工具
| 标志 | 类型 | 默认值 |
|------|------|---------|
| `--no-tools` | bool | 禁用所有工具 |
| `--tools` | string | "read,bash,edit,write" |

#### 扩展
| 标志 | 别名 | 类型 |
|------|---------|------|
| `--extension` | `-e` | string（可重复） |
| `--no-extensions` | | bool |

#### 技能
| 标志 | 类型 |
|------|------|
| `--skill` | string（可重复） |
| `--no-skills` | bool |

#### 提示模板
| 标志 | 类型 |
|------|------|
| `--prompt-template` | string（可重复） |
| `--no-prompt-templates` | bool |

#### 主题
| 标志 | 类型 |
|------|------|
| `--theme` | string（可重复） |
| `--no-themes` | bool |

#### 导出与列示
| 标志 | 类型 | 描述 |
|------|------|-------------|
| `--export` | string | 导出会话为 HTML |
| `--list-models` | bool/string | 列出可用模型 |

### 9.3 位置参数

- **文件参数：** 以 `@` 为前缀（例如 `@file.md`）
- **消息参数：** 任意非标志的位置参数

### 9.4 用法

```
pi [options] [@files...] [messages...]
```

---

## 10. 执行流程

```
1. 检查包管理命令（install/remove/update/list/config）
   └─ 若匹配：执行并 exit(0)

2. 运行迁移

3. 第一遍：解析扩展/技能/提示模板/主题标志
   └─ 加载资源

4. 第二遍：使用扩展注册的标志进行解析

5. 提前退出：
   ├─ --version → 打印版本并 exit(0)
   ├─ --help → 打印帮助并 exit(0)
   ├─ --list-models → 列出模型并 exit(0)
   └─ --export → 导出并 exit(0/1)

6. 处理 stdin（若非 TTY 且非 RPC 模式）
   └─ 将 stdin 前置到消息中，强制 print=true

7. 从 @files 和消息参数准备初始消息

8. 确定模式：
   └─ isInteractive = !print && mode == undefined

9. 创建会话管理器：
   ├─ --no-session → 内存中
   ├─ --session → 打开/分叉指定文件
   ├─ --continue → 继续最近一次
   ├─ --resume → 显示选择器
   └─ 默认 → 创建新会话

10. 从 --models 或设置解析模型作用域

11. 构建会话选项

12. 应用 --api-key 覆盖

13. 创建智能体会话
    └─ 若无模型且非交互式则 exit(1)

14. 将思考级别钳制到模型能力范围内

15. 运行模式：
    ├─ RPC → runRpcMode() [持续运行]
    ├─ 交互式 → InteractiveMode.run() [持续运行]
    └─ 打印 → runPrintMode() → exit(0)
```

### 10.1 退出码

| 代码 | 含义 |
|------|---------|
| 0 | 成功 |
| 1 | 错误（参数无效、文件未找到、API 错误等） |

---

## 11. RPC 模式协议（基于 stdio 的 JSON）

### 11.1 启动 RPC 模式

```bash
pi --mode rpc [options]
```

常用选项：
- `--provider <name>`
- `--model <id>`
- `--no-session`
- `--session-dir <path>`

### 11.2 协议概览

- **命令**：发送到 stdin 的 JSON 对象，每行一个。
- **响应**：带有 `type: "response"` 的 JSON 对象，表示成功/失败。
- **事件**：以 JSON 行形式流式输出到 stdout。
- 所有命令均支持可选 `id` 用于关联；响应会回显 `id`。

### 11.3 命令

#### 提示

**prompt**
- 请求：`{"id":"req-1","type":"prompt","message":"Hello"}`
- 可选 `images`：`ImageContent` 对象数组（`type:"image"`，`source` = `{type:"base64", mediaType, data}`）。
- 可选 `streamingBehavior`：`"steer"` 或 `"followUp"`。
  - 若智能体正在流式输出且未提供 `streamingBehavior` → 报错。
  - `"steer"`：在当前工具执行后中断；剩余工具调用被跳过。
  - `"followUp"`：在智能体完成后排队执行。
- 扩展命令（`/command`）即使在流式期间也会立即执行。
- 技能命令（`/skill:name`）和提示模板（`/template`）在排队前展开。
- 响应：`{"type":"response","command":"prompt","success":true}`

**steer**
- 请求：`{"type":"steer","message":"Stop and do this instead"}`
- 相同的展开规则；不允许扩展命令（请使用 `prompt`）。
- 响应：`{"type":"response","command":"steer","success":true}`

**follow_up**
- 请求：`{"type":"follow_up","message":"After you're done, also do this"}`
- 相同的展开规则；不允许扩展命令。
- 响应：`{"type":"response","command":"follow_up","success":true}`

**abort**
- 请求：`{"type":"abort"}`
- 响应：`{"type":"response","command":"abort","success":true}`

**new_session**
- 请求：`{"type":"new_session"}` 或 `{"type":"new_session","parentSession":"/path/to/parent.jsonl"}`
- 可被 `session_before_switch` 扩展处理器取消。
- 响应：`{"type":"response","command":"new_session","success":true,"data":{"cancelled":false}}`

#### 状态

**get_state**
- 请求：`{"type":"get_state"}`
- 响应数据：
  - `model`：完整 `Model` 或 `null`
  - `thinkingLevel`：`"off"|"minimal"|"low"|"medium"|"high"|"xhigh"`
  - `isStreaming`：bool
  - `isCompacting`：bool
  - `steeringMode`：`"all"|"one-at-a-time"`
  - `followUpMode`：`"all"|"one-at-a-time"`
  - `sessionFile`：string（内存会话中省略）
  - `sessionId`：string
  - `sessionName`：string（未设置时省略）
  - `autoCompactionEnabled`：bool
  - `messageCount`：number
  - `pendingMessageCount`：number

**get_messages**
- 请求：`{"type":"get_messages"}`
- 响应：`{"type":"response","command":"get_messages","success":true,"data":{"messages":[...]}}`
- 消息为 `AgentMessage` 对象（User/Assistant/ToolResult/BashExecution）。

#### 模型

**set_model**
- 请求：`{"type":"set_model","provider":"anthropic","modelId":"claude-sonnet-4-20250514"}`
- 响应：`{"type":"response","command":"set_model","success":true,"data":<Model>}`

**cycle_model**
- 请求：`{"type":"cycle_model"}`
- 响应：`{"type":"response","command":"cycle_model","success":true,"data":{"model":<Model>,"thinkingLevel":"medium","isScoped":false}}`
- 若仅有一个可用模型则返回 `data: null`。

**get_available_models**
- 请求：`{"type":"get_available_models"}`
- 响应：`{"type":"response","command":"get_available_models","success":true,"data":{"models":[<Model>,...]}}`

#### 思考

**set_thinking_level**
- 请求：`{"type":"set_thinking_level","level":"high"}`
- 思考级别：`"off"|"minimal"|"low"|"medium"|"high"|"xhigh"`
- 响应：success true

**cycle_thinking_level**
- 请求：`{"type":"cycle_thinking_level"}`
- 响应：`{"type":"response","command":"cycle_thinking_level","success":true,"data":{"level":"high"}}`
- 若模型不支持思考则返回 `data: null`。

#### 队列模式

**set_steering_mode**
- 请求：`{"type":"set_steering_mode","mode":"one-at-a-time"}`
- 模式：`"all"` 或 `"one-at-a-time"`（默认）
- 响应：success true

**set_follow_up_mode**
- 请求：`{"type":"set_follow_up_mode","mode":"one-at-a-time"}`
- 模式：`"all"` 或 `"one-at-a-time"`（默认）
- 响应：success true

#### 压缩

**compact**
- 请求：`{"type":"compact"}` 或 `{"type":"compact","customInstructions":"Focus on code changes"}`
- 响应数据：`{summary, firstKeptEntryId, tokensBefore, details}`

**set_auto_compaction**
- 请求：`{"type":"set_auto_compaction","enabled":true}`
- 响应：success true

#### 重试

**set_auto_retry**
- 请求：`{"type":"set_auto_retry","enabled":true}`
- 响应：success true

**abort_retry**
- 请求：`{"type":"abort_retry"}`
- 响应：success true

#### Bash

**bash**
- 请求：`{"type":"bash","command":"ls -la"}`
- 响应数据：
  - `output`：string
  - `exitCode`：number
  - `cancelled`：bool
  - `truncated`：bool
  - `fullOutputPath`：string | null（仅在截断时）

**abort_bash**
- 请求：`{"type":"abort_bash"}`
- 响应：success true

#### 会话

**get_session_stats**
- 请求：`{"type":"get_session_stats"}`
- 响应数据：
  - `sessionFile`、`sessionId`
  - `userMessages`、`assistantMessages`、`toolCalls`、`toolResults`、`totalMessages`
  - `tokens`：`{input, output, cacheRead, cacheWrite, total}`
  - `cost`：number（总计 $）

**export_html**
- 请求：`{"type":"export_html"}` 或 `{"type":"export_html","outputPath":"/tmp/session.html"}`
- 响应数据：`{path: "<output path>"}`

**switch_session**
- 请求：`{"type":"switch_session","sessionPath":"/path/to/session.jsonl"}`
- 可被 `session_before_switch` 扩展处理器取消。
- 响应数据：`{cancelled: false}`

**fork**
- 请求：`{"type":"fork","entryId":"abc123"}`
- `entryId` 必须是用户消息条目。
- 创建新会话（从所选条目的父级分叉）。
- 响应数据：`{text:"<user message text>", cancelled:false}`
- 可被 `session_before_fork` 扩展处理器取消。

**get_fork_messages**
- 请求：`{"type":"get_fork_messages"}`
- 响应数据：`{messages:[{entryId, text}, ...]}`

**get_last_assistant_text**
- 请求：`{"type":"get_last_assistant_text"}`
- 响应数据：`{text: "<assistant text>"}` 或 `{text: null}`

**set_session_name**
- 请求：`{"type":"set_session_name","name":"my-feature-work"}`
- 响应：success true

#### 命令

**get_commands**
- 请求：`{"type":"get_commands"}`
- 响应数据：`{"commands":[{name, description?, source, location?, path?}, ...]}`
  - `source`：`"extension"|"template"|"skill"`
  - `location`：`"user"|"project"|"path"`（扩展不含此字段）

### 11.4 事件

| 事件 | 字段 |
|-------|--------|
| `agent_start` | `{type:"agent_start"}` |
| `agent_end` | `{type:"agent_end", messages:[AgentMessage], error?}` |
| `turn_start` | `{type:"turn_start"}` |
| `turn_end` | `{type:"turn_end", message:AgentMessage, toolResults:[AgentMessage]}` |
| `message_start` | `{type:"message_start", message:AgentMessage}` |
| `message_update` | `{type:"message_update", message:AgentMessage, assistantMessageEvent:<delta>}` |
| `message_end` | `{type:"message_end", message:AgentMessage}` |
| `tool_execution_start` | `{type:"tool_execution_start", toolCallId, toolName, args}` |
| `tool_execution_update` | `{type:"tool_execution_update", toolCallId, toolName, args, partialResult}` |
| `tool_execution_end` | `{type:"tool_execution_end", toolCallId, toolName, result, isError}` |
| `auto_compaction_start` | `{type:"auto_compaction_start", reason:"threshold"|"overflow"}` |
| `auto_compaction_end` | `{type:"auto_compaction_end", result, aborted, willRetry, errorMessage?}` |
| `auto_retry_start` | `{type:"auto_retry_start", attempt, maxAttempts, delayMs, errorMessage}` |
| `auto_retry_end` | `{type:"auto_retry_end", success, attempt, finalError?}` |
| `extension_error` | `{type:"extension_error", extensionPath, event, error}` |

**assistantMessageEvent 增量类型**（流式）：
- `start`
- `text_start`、`text_delta`、`text_end`
- `thinking_start`、`thinking_delta`、`thinking_end`
- `toolcall_start`、`toolcall_delta`、`toolcall_end`（包含完整 `toolCall`）
- `done`（`reason`：`"stop"|"length"|"toolUse"`）
- `error`（`reason`：`"aborted"|"error"`）

### 11.5 扩展 UI 协议（RPC）

**extension_ui_request**（stdout）：
- 基础：`{type:"extension_ui_request", id, method, ...}`
- 对话框方法会阻塞直到响应或超时：
  - `select`：`{title, options:[{label,value}], placeholder?, default?, timeout?}`
  - `confirm`：`{title, message, default?, timeout?}`
  - `input`：`{title, placeholder?, default?, password?, timeout?}`
  - `editor`：`{title, language?, default?, readOnly?, timeout?}`
- 即发即忘方法（无需响应）：
  - `notify`：`{title, message, level?}`
  - `setStatus`：`{text}`
  - `setWidget`：`{content}`
  - `setTitle`：`{title}`
  - `set_editor_text`：`{text}`
- RPC 限制：`custom()` 返回 `undefined`；`setWorkingMessage`、`setFooter`、`setHeader`、`setEditorComponent` 为空操作；`getEditorText()` 返回 `""`。

**extension_ui_response**（stdin）：
- 基础：`{type:"extension_ui_response", id, value?, cancelled?}`
- 对话框响应：
  - select/input/editor：`{value: <selected/entered>}`
  - confirm：`{value: true|false}`
  - 取消：`{cancelled: true}`

### 11.6 RPC 类型

**Model**
- `id`、`name`、`api`、`provider`、`baseUrl`
- `reasoning`（bool）
- `input`：`["text","image"]`
- `contextWindow`、`maxTokens`
- `cost`：`{input, output, cacheRead, cacheWrite}`

**UserMessage**
- `{role:"user", content, timestamp, attachments:[]}`
- `content` 可以是字符串或 `TextContent`/`ImageContent` 数组。

**AssistantMessage**
- `{role:"assistant", content:[...], api, provider, model, usage, stopReason, timestamp}`
- `usage`：`{input, output, cacheRead, cacheWrite, cost:{input, output, cacheRead, cacheWrite, total}}`
- `stopReason`：`"stop"|"length"|"toolUse"|"error"|"aborted"`

**ToolResultMessage**
- `{role:"toolResult", toolCallId, toolName, content, isError, timestamp}`

**BashExecutionMessage**
- `{role:"bashExecution", command, output, exitCode, cancelled, truncated, fullOutputPath, timestamp}`

**Attachment**
- `{id, type:"image", fileName, mimeType, size, content, extractedText, preview}`

---

---

## 12. 扩展 API

扩展是可注册工具、命令和事件处理器的 Node.js 模块。Rust 移植版实现了 PiJS 运行时（连接器架构见 `EXTENSIONS.md`）。

### 12.1 扩展入口

```typescript
// 扩展模块导出 activate 函数
export async function activate(ctx: ExtensionContext): Promise<void>;
```

### 12.2 ExtensionContext

```rust
pub struct ExtensionContext {
    // Registration
    fn register_tool(name: &str, config: ToolConfig, handler: ToolHandler);
    fn register_command(name: &str, config: CommandConfig, handler: CommandHandler);

    // Event handlers
    fn on_agent_start(handler: Fn(AgentStartEvent));
    fn on_agent_end(handler: Fn(AgentEndEvent));
    fn on_turn_start(handler: Fn(TurnStartEvent));
    fn on_turn_end(handler: Fn(TurnEndEvent));
    fn on_tool_execution_start(handler: Fn(ToolExecutionStartEvent));
    fn on_tool_execution_end(handler: Fn(ToolExecutionEndEvent));
    fn on_session_before_switch(handler: Fn(SessionEvent) -> bool);  // Return false to cancel
    fn on_session_before_fork(handler: Fn(SessionEvent) -> bool);    // Return false to cancel
    fn on_startup(handler: Fn(StartupEvent));

    // UI access
    ui: ExtensionUi,

    // Session access
    session: SessionAccess,

    // Logging
    log: Logger,
}
```

### 12.3 工具注册

```typescript
interface ToolConfig {
    label: string;                // Display name
    description: string;          // For LLM context
    parameters: JsonSchema;       // JSON Schema for validation
}

type ToolHandler = (args: ToolArgs, update?: UpdateCallback) => Promise<ToolResult>;

interface ToolArgs {
    toolCallId: string;
    input: Record<string, unknown>;
}

interface ToolResult {
    content: ContentBlock[];
    details?: Record<string, unknown>;
    isError?: boolean;
}

type UpdateCallback = (partial: ToolResult) => void;
```

### 12.4 命令注册

```typescript
interface CommandConfig {
    description?: string;         // Help text
}

type CommandHandler = (args: string) => Promise<string | void>;
```

命令通过 `/command args` 语法调用。若处理器返回字符串，则该字符串将用作用户消息。

### 12.5 扩展 UI 方法

```typescript
interface ExtensionUi {
    // Dialogs (blocking, support cancellation)
    select(options: SelectOptions): Promise<string | undefined>;
    confirm(options: ConfirmOptions): Promise<boolean>;
    input(options: InputOptions): Promise<string | undefined>;
    editor(options: EditorOptions): Promise<string | undefined>;

    // Non-blocking updates
    notify(options: NotifyOptions): void;
    setStatus(text: string): void;
    setWidget(content: string): void;
    setTitle(title: string): void;

    // Editor interaction
    setEditorText(text: string): void;
    getEditorText(): string;  // Returns "" in RPC mode

    // Custom component (interactive TUI only)
    custom<T>(component: Component): Promise<T | undefined>;  // Returns undefined in RPC
}

interface SelectOptions {
    title: string;
    options: Array<{label: string; value: string}>;
    placeholder?: string;
    default?: string;
    timeout?: number;  // ms, undefined = no timeout
}

interface ConfirmOptions {
    title: string;
    message: string;
    default?: boolean;
    timeout?: number;
}

interface InputOptions {
    title: string;
    placeholder?: string;
    default?: string;
    password?: boolean;
    timeout?: number;
}

interface EditorOptions {
    title: string;
    language?: string;
    default?: string;
    readOnly?: boolean;
    timeout?: number;
}

interface NotifyOptions {
    title: string;
    message: string;
    level?: "info" | "warning" | "error";
}
```

**取消语义：**
- 对话框方法在用户取消（Esc 键）时返回 `undefined`
- 超时触发取消
- 在 RPC 模式下，`extension_ui_response` 中 `cancelled: true` 表示取消
- `confirm()` 在取消时返回 `false`（而非 `undefined`）

### 12.6 会话访问

```typescript
interface SessionAccess {
    getMessages(): Message[];
    getState(): SessionState;
    getFile(): string | undefined;  // undefined for in-memory sessions
}

interface SessionState {
    sessionId: string;
    messageCount: number;
    isStreaming: boolean;
    model: Model | null;
    thinkingLevel: ThinkingLevel;
}
```

### 12.7 扩展事件

```typescript
interface AgentStartEvent {
    sessionId: string;
}

interface AgentEndEvent {
    sessionId: string;
    messages: Message[];
    error?: string;
}

interface TurnStartEvent {
    sessionId: string;
    turnIndex: number;
}

interface TurnEndEvent {
    sessionId: string;
    turnIndex: number;
    message: AssistantMessage;
    toolResults: ToolResultMessage[];
}

interface ToolExecutionStartEvent {
    toolCallId: string;
    toolName: string;
    args: Record<string, unknown>;
}

interface ToolExecutionEndEvent {
    toolCallId: string;
    toolName: string;
    result: ToolResult;
    isError: boolean;
}

interface SessionEvent {
    currentSession: string | undefined;
    targetSession?: string;  // For switch
    forkEntryId?: string;    // For fork
}

interface StartupEvent {
    version: string;
    sessionFile?: string;
}
```

### 12.8 扩展日志

```typescript
interface Logger {
    debug(message: string, data?: Record<string, unknown>): void;
    info(message: string, data?: Record<string, unknown>): void;
    warn(message: string, data?: Record<string, unknown>): void;
    error(message: string, data?: Record<string, unknown>): void;
}
```

日志以结构化 JSON 发出（schema：`pi.ext.log.v1`）。

---

## 13. 资源系统

资源是可发现的用户内容：技能、提示模板和主题。

### 13.1 资源发现

资源按以下优先级从多个位置加载：

1. **显式 CLI 标志**（`--skill`、`--prompt-template`、`--theme`）
2. **设置数组**（settings.json 中的 `skills`、`prompts`、`themes`）
3. **已安装的包**（通过 `packages` 设置）

### 13.2 包清单

```json
{
  "name": "@scope/package-name",
  "pi": {
    "extensions": ["./dist/extension.js"],
    "skills": ["./skills/"],
    "prompts": ["./prompts/"],
    "themes": ["./themes/"]
  }
}
```

若 `pi` 字段缺失，则应用默认值：
- `extensions`：`[]`
- `skills`：若目录存在则为 `["./skills/"]`
- `prompts`：若目录存在则为 `["./prompts/"]`
- `themes`：若目录存在则为 `["./themes/"]`

### 13.3 技能

技能是带有 YAML 前置元数据的 Markdown 文件，定义智能体能力。

**文件位置：**
- 全局：`~/.pi/agent/skills/*.md`
- 项目：`./.pi/skills/*.md`
- 包：`<package>/skills/*.md`

**Frontmatter schema：**

```yaml
---
name: skill-name           # Required, kebab-case
description: Brief desc    # Required, shown in skill list
allowed_tools:             # Optional, restrict to specific tools
  - read
  - bash
---
```

**技能内容：** Markdown 正文成为技能提示。

**展开：** `/skill:name args` 展开为：
```xml
<skill name="skill-name" arguments="args">
[skill markdown content]
</skill>
```

**系统提示注入：** 当启用 `read` 工具时，活动技能会被追加：
```xml
<available_skills>
<skill name="name1">description1</skill>
<skill name="name2">description2</skill>
</available_skills>
```

### 13.4 提示模板

提示模板是用于可复用用户提示的 Markdown 文件。

**文件位置：**
- 全局：`~/.pi/agent/prompts/*.md`
- 项目：`./.pi/prompts/*.md`
- 包：`<package>/prompts/*.md`

**命令格式：** `/template-name arg1 arg2 ...`

**变量替换：**
| 变量 | 含义 |
|----------|---------|
| `$1`、`$2`、... | 位置参数 |
| `$@` | 所有参数以空格拼接 |
| `$ARGUMENTS` | 同 `$@` |
| `${@:N}` | 从位置 N 开始的所有参数 |

**展开结果：** 经过变量替换后成为用户消息。

### 13.5 主题

主题是定义终端配色方案的 JSON 文件。

**文件位置：**
- 全局：`~/.pi/agent/themes/*.json`
- 项目：`./.pi/themes/*.json`
- 包：`<package>/themes/*.json`

**Schema：**

```json
{
  "name": "theme-name",
  "colors": {
    "text": "#ffffff",
    "background": "#000000",
    "primary": "#007acc",
    "secondary": "#6c757d",
    "success": "#28a745",
    "warning": "#ffc107",
    "error": "#dc3545",
    "thinking": "#6c757d",
    "tool": "#17a2b8"
  }
}
```

### 13.6 包来源

包可指定为：

| 来源类型 | 格式 | 示例 |
|-------------|--------|---------|
| npm | `npm:<package>@<version>` | `npm:@pi/tools@1.0.0` |
| git | `git:<url>#<ref>` | `git:github.com/user/repo#main` |
| local | `path:<absolute-path>` | `path:/home/user/my-extension` |

设置字段：
```json
{
  "packages": [
    "npm:@pi/tools@1.0.0",
    "git:github.com/user/extension#v1.0"
  ]
}
```

---

## 总结

本规范涵盖：
- **消息类型：** User、Assistant、ToolResult 及其所有内容块变体
- **流式：** 完整的事件类型枚举与序列
- **提供方：** Trait 定义与模型注册结构
- **工具：** 内置工具的精确参数与行为
- **会话：** 带树结构的 JSONL 格式
- **配置：** 带优先级规则的设置结构
- **认证：** 带 OAuth 刷新的凭证存储
- **CLI：** 完整的标志列表与执行流程
- **RPC：** 带事件、类型和扩展 UI 的 JSON 命令协议
- **扩展：** 包含注册、事件、UI 和取消语义的完整 API 表面
- **资源：** 技能、提示模板和主题的发现与展开

**阅读本文档后，你无需再查阅遗留的 TypeScript 代码。**

---

## 附录 A：完整扩展 API 参考（提取于 2026-02-04）

本附录提供从遗留 pi-mono 代码库提取的完整、详细的扩展 API。

### A.1 扩展工厂模式

扩展导出一个接收 `ExtensionAPI` 对象的默认工厂函数：

```typescript
export default function (pi: ExtensionAPI) {
  // Register handlers, tools, commands
}

// Or async:
export default async function (pi: ExtensionAPI) {
  // Async initialization
}
```

### A.2 完整事件类型（20+）

| 事件 | 可修改 | 可取消 | 载荷 |
|-------|-----------|------------|---------|
| `resources_discover` | 是 | 否 | `{}` |
| `session_start` | 否 | 否 | `{}` |
| `session_before_switch` | 否 | 是 | `{}` |
| `session_switch` | 否 | 否 | `{reason, previousSessionFile?}` |
| `session_before_fork` | 否 | 是 | `{}` |
| `session_fork` | 否 | 否 | `{previousSessionFile?}` |
| `session_before_compact` | 是 | 是 | `{preparation, branchEntries, signal}` |
| `session_compact` | 否 | 否 | `{compactionEntry, fromExtension}` |
| `session_before_tree` | 是 | 是 | `{preparation, signal}` |
| `session_tree` | 否 | 否 | `{newLeafId, oldLeafId, summaryEntry?}` |
| `session_shutdown` | 否 | 否 | `{}` |
| `context` | 是 | 否 | `{messages}` |
| `before_agent_start` | 是 | 否 | `{prompt, images?, systemPrompt}` |
| `agent_start` | 否 | 否 | `{}` |
| `agent_end` | 否 | 否 | `{messages}` |
| `turn_start` | 否 | 否 | `{turnIndex, timestamp}` |
| `turn_end` | 否 | 否 | `{turnIndex, message, toolResults}` |
| `model_select` | 否 | 否 | `{model, previousModel?, source}` |
| `tool_call` | 否 | 是（阻止） | `{toolName, input, toolCallId}` |
| `tool_result` | 是 | 否 | `{toolName, input, content, isError}` |
| `user_bash` | 是 | 否 | `{command, excludeFromContext, cwd}` |
| `input` | 是 | 是（已处理） | `{text, images?, source}` |

### A.3 注册 API

**工具注册：**
```typescript
pi.registerTool({
  name: string,
  label: string,
  description: string,
  parameters: TypeBoxSchema,
  execute: (toolCallId, params, signal, onUpdate, ctx) => Promise<ToolOutput>,
  renderCall?: (args, theme) => Component,
  renderResult?: (result, options, theme) => Component,
});
```

**命令注册：**
```typescript
pi.registerCommand(name, {
  description: string,
  handler: (args, ctx) => Promise<void>,
  getArgumentCompletions?: (prefix) => CompletionItem[],
});
```

**快捷键注册：**
```typescript
pi.registerShortcut(key, {
  description: string,
  handler: (ctx) => Promise<void>,
});
```

**标志注册：**
```typescript
pi.registerFlag(name, {
  description: string,
  type: "boolean" | "string",
  default?: boolean | string,
});
```

**提供方注册：**
```typescript
pi.registerProvider(name, {
  baseUrl: string,
  apiKey: string,  // env var name
  api: "anthropic-messages" | "openai-responses",
  models: Model[],
  streamSimple?: StreamHandler,
  oauth?: OAuthConfig,
});
```

### A.4 消息 API

```typescript
// Custom message (not for LLM)
pi.sendMessage({
  customType: string,
  content: string,
  display?: boolean,
  details?: unknown,
}, {
  triggerTurn?: boolean,
  deliverAs?: "steer" | "followUp" | "nextTurn",
});

// User message (triggers LLM turn)
pi.sendUserMessage(content, {
  deliverAs?: "steer" | "followUp",
});

// Session entry (not sent to LLM)
pi.appendEntry(customType, data);
```

### A.5 会话与模型 API

```typescript
// Session metadata
pi.setSessionName(name);
pi.getSessionName();
pi.setLabel(entryId, label);

// Tool management
pi.getActiveTools();
pi.getAllTools();
pi.setActiveTools(toolNames);

// Model control
await pi.setModel(model);
pi.getThinkingLevel();
pi.setThinkingLevel(level);

// Execution
await pi.exec(command, args, options);

// Inter-extension events
pi.events.emit(eventName, data);
pi.events.on(eventName, handler);
```

### A.6 UI 上下文方法

**对话框（阻塞）：**
```typescript
await ctx.ui.select(title, options, opts);
await ctx.ui.confirm(title, message, opts);
await ctx.ui.input(title, placeholder?, opts);
await ctx.ui.editor(title, prefill?);
```

**非阻塞：**
```typescript
ctx.ui.notify(message, type?);
ctx.ui.setStatus(key, text);
ctx.ui.setWorkingMessage(message?);
ctx.ui.setWidget(key, content, options?);
ctx.ui.setFooter(factory?);
ctx.ui.setHeader(factory?);
ctx.ui.setTitle(title);
```

**编辑器：**
```typescript
ctx.ui.setEditorText(text);
ctx.ui.getEditorText();
ctx.ui.setEditorComponent(factory?);
```

**主题：**
```typescript
ctx.ui.theme;
ctx.ui.getAllThemes();
ctx.ui.getTheme(name);
ctx.ui.setTheme(theme);
```

### A.7 扩展上下文属性

```typescript
interface ExtensionContext {
  ui: ExtensionUIContext;
  hasUI: boolean;
  cwd: string;
  sessionManager: ReadonlySessionManager;
  modelRegistry: ModelRegistry;
  model: Model | undefined;

  isIdle(): boolean;
  abort(): void;
  hasPendingMessages(): boolean;
  shutdown(): void;
  getContextUsage(): ContextUsage | undefined;
  compact(options?): void;
  getSystemPrompt(): string;
}

interface ExtensionCommandContext extends ExtensionContext {
  waitForIdle(): Promise<void>;
  newSession(options?): Promise<{cancelled: boolean}>;
  fork(entryId): Promise<{cancelled: boolean}>;
  navigateTree(targetId, options?): Promise<{cancelled: boolean}>;
  switchSession(sessionPath): Promise<{cancelled: boolean}>;
}
```

---

## 附录 B：库集成要求

### B.1 asupersync 要求

**当前已用：**
- `Cx` 能力上下文
- `mpsc` / `oneshot` 通道
- `Mutex` / `Notify` 同步
- `timeout` / `sleep` 时间操作

**建议使用：**
- `Cx::region()` 用于结构化并发（扩展生命周期）
- `database::sqlite` 用于会话索引（可选）
- 用于原子消息投递的两阶段通道语义
- 用于有界清理的取消预算
- `LabRuntime` 用于确定性测试

### B.2 rich_rust 要求

**当前已用：**
- 带标记的 `Console`
- 用于带框内容的 `Panel`
- 用于数据的 `Table`
- 用于分隔线的 `Rule`

**建议启用：**
- `markdown` 特性用于助手响应渲染
- `syntax` 特性用于代码块高亮
- `json` 特性用于美化打印 JSON 输出

### B.3 charmed_rust 要求

**当前已用：**
- `bubbletea::Model` 用于 Elm 架构
- `bubbles::TextArea` 用于输入
- `bubbles::Viewport` 用于滚动
- `bubbles::Spinner` 用于加载
- `glamour::Renderer` 用于 Markdown

**建议使用：**
- `lipgloss::Style` 用于直接样式
- `bubbles::List` 用于历史导航
- `bubbles::Table` 用于结构化输出
- 匹配 Pi 视觉风格的自定义 glamour 主题
