# 计划：将 Pi Agent 移植到 Rust

## 执行摘要

将 **pi-mono** AI 编程智能体平台从 TypeScript/Node.js 移植为地道的 Rust 实现。目标是提供单二进制 CLI，提供用于 AI 辅助编程的交互式终端界面，具备多提供方 LLM 支持、会话管理与可扩展的工具系统。

**为何叫 "py_agent_rust"？** 项目目录在发现这是 TypeScript 而非 Python 之前就已命名。名字保留不变。

---

## 为何移植到 Rust？

1. **单二进制分发** — 无需 Node.js 运行时、npm 或额外依赖即可安装
2. **启动性能** — 原生二进制毫秒级启动，相比 Node.js 的数百毫秒更快
3. **内存效率** — 无 V8 堆开销；对分配的精确控制
4. **可靠性** — 编译期保障；无运行时类型错误
5. **跨平台** — 便于面向 Linux/macOS/Windows 交叉编译
6. **终端处理** — Rust 拥有出色的终端库（crossterm、ratatui）

---

## 计划移植的内容

### Phase 1: 核心基础

- [ ] CLI 参数解析（Clap）
- [ ] 配置系统（全局 + 项目级设置）
- [ ] 错误处理框架
- [ ] 日志/追踪基础设施

### Phase 2: 提供方抽象（等价于 `pi-ai`）

- [ ] 统一 LLM API trait
- [ ] 流式响应处理
- [ ] 消息类型与内容块
- [ ] 工具调用/结果协议
- [ ] 提供方实现：
  - [ ] Anthropic (Claude)
  - [ ] OpenAI (GPT-4 等)
  - [ ] Google (Gemini)
  - [ ] 按需补充其他提供方

### Phase 3: 智能体运行时（等价于 `pi-agent`）

- [ ] 智能体循环与工具执行
- [ ] 消息历史管理
- [ ] 思考/推理块处理
- [ ] 流式事件系统
- [ ] 工具校验与错误处理

### Phase 4: 会话管理

- [ ] JSONL 会话文件格式（兼容 version 3）
- [ ] 会话头与条目类型
- [ ] 树导航与分支
- [ ] 压缩/摘要（Compaction/summarization）
- [ ] 基于项目的会话组织

### Phase 5: 内置工具

- [ ] `read` — 带截断的文件读取
- [ ] `bash` — 带流式的 Shell 命令执行
- [ ] `edit` — 基于 diff 的文件编辑
- [ ] `write` — 文件创建/覆盖
- [ ] `grep` — 内容搜索（ripgrep 集成）
- [ ] `find` — 文件搜索
- [ ] `ls` — 目录列举

### Phase 6: 终端 UI

- [ ] 差分渲染引擎
- [ ] 带补全的多行编辑器
- [ ] 斜杠命令系统
- [ ] 状态栏（tokens/cost/context）
- [ ] 思考块展示
- [ ] Markdown 渲染
- [ ] 图像处理（若终端支持）

### Phase 7: 认证

- [ ] API 密钥管理
- [ ] OAuth 流程支持（Anthropic、GitHub Copilot 等）
- [ ] 凭据存储与合适的权限控制

### Phase 8: 输出模式

- [ ] 交互模式（完整 TUI）
- [ ] Print 模式（单次响应）
- [ ] JSON 模式（结构化输出）
- [ ] HTML 导出

---

## 暂不移植的内容

| Feature                         | Reason                                                  |
| ------------------------------- | ------------------------------------------------------- |
| **Web UI** (`packages/web-ui`)  | 超出范围；优先 CLI                                      |
| **Slack bot** (`packages/mom`)  | 专用集成；非核心                                        |
| **GPU pods** (`packages/pods`)  | 基础设施工具；非核心                                    |
| **npm package system**          | 替换为原生插件/扩展系统                                 |
| **TypeScript type generation**  | 不适用                                                  |
| **Bun compilation**             | 改为原生 Rust 二进制                                    |
| **Full extension API**          | 简化的插件系统；后续可扩展                              |
| **All 20+ providers initially** | 先从 Anthropic、OpenAI、Google 起步；按需补充其他提供方 |
| **tmux integration**            | 复杂；延后到后续阶段                                    |
| **GitHub Gist sharing**         | 锦上添花；延后                                          |
| **Themes system**               | 改为简化的颜色配置                                      |
| **Skills system**               | 延后；先聚焦核心工具                                    |
| **Prompt templates**            | 初期采用简化方案                                        |

---

## 参考项目

| Project | Path                                         | Patterns to Copy                       |
| ------- | -------------------------------------------- | -------------------------------------- |
| dcg     | `/data/projects/destructive_command_guard`   | Clap derive、错误处理、release profile |
| cass    | `/data/projects/coding_agent_session_search` | SQLite、JSONL 解析、会话格式           |

---

## 架构决策

### 1. 单二进制

所有功能集中于一个二进制。不同于 TypeScript 版本的多包结构。

### 2. 异步运行时

使用 `tokio` 处理异步 I/O（HTTP 请求、流式、文件操作）。

### 3. 会话存储

- **JSONL 文件**作为可信源（对 Git 友好、人类可读）
- 会话不使用 SQLite（不同于 cass）；JSONL 已足够
- 后续可能为搜索索引引入 SQLite

### 4. 终端 UI

- 使用 `crossterm` 处理终端
- 自定义差分渲染器（初期不使用 ratatui — 过重）
- 用于终端状态的 RAII guards

### 5. 提供方架构

- 基于 trait 的提供方抽象
- 每个提供方独立模块
- 可选提供方使用 feature flags

### 6. 配置

- 配置文件采用 TOML 格式
- 环境变量覆盖
- 支持项目级 `.pi/` 目录

---

## 实现阶段

### Phase 1: 基础（第 1 周）

- Cargo.toml 与依赖
- rust-toolchain.toml（nightly、edition 2024）
- 基于 thiserror 的错误类型
- 配置加载（全局 + 项目级）
- 基于 Clap 的 CLI 骨架

### Phase 2: 提供方层（第 2 周）

- 提供方 trait 定义
- 消息/内容类型
- 流式事件类型
- Anthropic 提供方实现
- 基础请求/响应循环

### Phase 3: 智能体核心（第 3 周）

- 智能体循环实现
- 工具 trait 与内置工具
- 消息历史
- 流式响应处理

### Phase 4: 会话持久化（第 4 周）

- JSONL 会话格式
- 会话读写
- 会话列举与选择
- 基础树导航

### Phase 5: 终端 UI（第 5–6 周）

- 终端状态管理
- 差分渲染器
- 编辑器组件
- 斜杠命令
- 状态栏

### Phase 6: 打磨（第 7 周起）

- 补充提供方
- 压缩系统
- HTML 导出
- OAuth 流程
- 性能优化

---

## 关键数据结构

### 消息类型

```rust
pub enum AgentMessage {
    User { content: Content, timestamp: i64 },
    Assistant { content: Content, timestamp: i64, model: String },
    ToolResult { tool_use_id: String, content: Content, is_error: bool },
}

pub enum ContentBlock {
    Text(String),
    Image { data: String, media_type: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
    Thinking { thinking: String },
}
```

### 会话格式

```rust
pub struct SessionHeader {
    pub version: u8,  // 3
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    pub parent_session: Option<String>,
}

pub enum SessionEntry {
    Message(SessionMessage),
    ThinkingLevelChange { level: String, timestamp: i64 },
    ModelChange { model: String, timestamp: i64 },
    Compaction { summary: String, removed_count: usize },
    BranchSummary { summary: String, checkpoint_id: String },
}
```

### 提供方 Trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        options: &CompletionOptions,
    ) -> Result<impl Stream<Item = Result<StreamEvent>>>;
}
```

---

## 成功标准

1. **功能对等**：与 pi-mono 核心功能对等（交互模式、工具、会话）
2. **性能**：<100ms 启动，TUI 60fps 流畅
3. **二进制体积**：strip 后 <20MB
4. **跨平台**：Linux x86_64/ARM64、macOS Intel/Apple Silicon、Windows
5. **可靠性**：正常运行无 panic；优雅的错误处理

---

## 待定问题

1. **插件系统**：如何支持扩展？WASM？动态库？脚本？
2. **图像渲染**：哪些终端支持图像？如何检测？
3. **OAuth**：如何在 CLI 中处理基于浏览器的 OAuth？本地服务回调？
4. **压缩**：使用 LLM 摘要？还是更简单的启发式方案？

---

## 下一步

1. 创建 `EXISTING_PI_STRUCTURE.md` — 从遗留代码深度提取规格
2. 创建 `PROPOSED_ARCHITECTURE.md` — 详细的 Rust 设计
3. 以依赖引导 Cargo 项目
4. 开始 Phase 1 实现
