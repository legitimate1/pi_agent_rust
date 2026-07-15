# pi_agent_rust — 高性能 AI 编码助手 CLI

Rust 移植版 Pi Agent。提供流式交互终端、工具执行、会话持久化。

## 技术栈

`Rust 2024 nightly` | `asupersync` | `rich_rust` | `serde`/`serde_json` | `clap` | `rquickjs`

## 关键文件

| 文件 | 说明 |
|:-----|:-----|
| `src/main.rs` | CLI 入口 + 会话初始化（含 disabledTools 过滤） |
| `src/agent.rs` | Agent 循环（工具迭代 + 扩展加载合并 + 工具去重） |
| `src/tools/` 模块目录 | ToolRegistry + 9 个内置工具模块（read/bash/pwsh/edit/write/grep/find/ls/hashline） |
| `src/config.rs` | 配置定义（含 disabled_tools 字段） |
| `src/cli.rs` | CLI 参数解析（含 --tools 默认列表） |
| `src/extensions.rs` | 扩展管理器、策略、调度 |
| `src/extensions_js.rs` | QuickJS 运行时桥接 |
| `src/providers/mod.rs` | Provider 工厂 + 扩展 stream-simple 桥接 |
| `src/session.rs` | JSONL 会话持久化 |
| `src/models.rs` | 内置 + models.json 模型注册表 |

## 给 AI 助手

请根据当前问题，主动查阅下方对应的文件：

- 项目有哪些功能、代码在哪 → `docs/context/features.md`
- 项目架构、核心流程、模块关系 → `docs/context/architecture.md`
- 命名规范、隐含假设、反模式 → `docs/context/conventions.md`
- 开发命令、构建部署 → `docs/context/commands.md`
- 关键设计决策（含本次扩展覆写内置工具改动） → `docs/context/design-decisions.md`

## 知识库文件

- `docs/context/index.md`
- `docs/context/features.md`
- `docs/context/architecture.md`
- `docs/context/conventions.md`
- `docs/context/commands.md`
- `docs/context/design-decisions.md`
