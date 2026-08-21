# 提议架构（Pi Rust 移植版）

## 目标

- 单一二进制、快速启动、低内存占用
- 与旧版 pi-coding-agent 功能完全对齐
- 严格的 JSONL 会话兼容性（v3）
- 模块化的提供方与工具系统

## 模块布局

```
src/
├── main.rs                # CLI 入口，模式分发
├── cli.rs                 # Clap 定义与参数辅助
├── config.rs              # 配置加载/合并与默认值
├── auth.rs                # 认证文件读写与 API 密钥解析
├── models.rs              # 模型注册与解析
├── provider.rs            # 提供方 trait 与共享类型
├── providers/             # 提供方实现
├── tools.rs               # 内置工具与注册表
├── session.rs             # JSONL 会话持久化
├── session_index.rs       # SQLite 索引（派生，可选）
├── agent.rs               # 智能体循环、工具执行、流式处理
├── modes.rs               # print/rpc/interactive 入口
└── tui.rs                 # 交互式界面（当前为按行实现，面向 TUI 就绪）
```

## 核心数据流

```
CLI → Config → Models/Auth → Session → Agent → Provider
                 ↑                    ↓
           Session Index        Tools + Session Writes
```

1. CLI 解析参数，合并配置，解析模型/提供方。
2. 智能体准备上下文（系统提示 + 消息 + 工具）。
3. 提供方流式推送事件 → 智能体更新会话与界面。
4. 工具调用通过工具注册表执行，并以 ToolResult 消息返回。
5. 会话 JSONL 是可信数据源；SQLite 索引跟踪元数据。

## 会话存储

- **主存储：** JSONL 会话文件（v3），仅追加树结构。
- **索引存储：** SQLite（`session-index.sqlite`），存储每会话元数据：
  - path、id、cwd、时间戳、last_modified、消息计数
  - 可搜索的 label/title 字段
- 同步策略见 `SYNC_STRATEGY.md`。

## 运行模式

- **交互式（Interactive）：** 按行的 REPL，支持命令解析与流式展示。
- **打印（Print）：** 非交互式、单次执行；输出文本或 JSON。
- **RPC：** JSONL 事件流，供编程化集成。

## 提供方策略

- 共享的 `Provider` trait 用于流式事件。
- 内置提供方：优先 Anthropic，随后 OpenAI/Google 等。
- 模型从 `~/.pi/agent/models.json` 加载，支持按提供方的默认值。

## 工具系统

- 工具 schema 来自 JSON Schema 定义。
- 内置工具：read、bash、edit、write、grep、find、ls。
- 工具结果作为会话条目持久化。

## 可扩展性

- 包、扩展、技能、提示模板、主题均从配置加载。
- 所有外部资源在模式执行前完成解析。

## 错误处理与遥测

- `thiserror` 用于结构化错误，边界处使用 `anyhow`。
- `tracing` 用于调试/详细输出，由 env filter 控制。
