# 计划：完成 pi_agent_rust 移植

> **目标：** 100% 功能覆盖，具备清晰的一致性测试 harness 与基准测试，充分利用 asupersync、rich_rust 与 charmed_rust。

> **重要提示：** 本文档为历史概览，非实时待办。
> 以 Beads 为权威计划来源：
>
> - `bv --robot-plan`
> - `bv --robot-priority`
> - `br ready`
> - `br show <id>`

---

## 执行摘要

**当前状态：** 约 85-90% 完成

- 核心类型、工具、会话、CLI、交互式 TUI：✅ 已实现
- 多提供方（Anthropic/OpenAI/Gemini/Azure）：✅ 已实现
- RPC 模式（stdin/stdout JSON 协议）：✅ 已实现（见 `src/rpc.rs`、`tests/rpc_mode.rs`）
- 一致性 harness：✅ 工具夹具套件 + 集成测试
- 基准测试：✅ 截断 + SSE 解析基线

**目标状态：** 生产就绪的 CLI，具备：

- 基于 charmed_rust（Elm 架构）的完整交互式 TUI
- 全部流式经 asupersync（可取消正确、结构化并发）
- 通过 rich_rust 实现美观输出（markup、表格、面板）
- 全面的一致性测试套件（工具 + RPC + 扩展），带 TypeScript 参考捕获
- 证明性能目标的基准 harness（含扩展宿主调用分发）

**主要剩余工作：**

1. **扩展运行时** + 一致性 harness：`bd-btq`、`bd-1e0`、`bd-2i5`、`bd-269`
2. **主题集成**（应用/切换、`/theme`、settings）：`bd-22p`、`bd-qpm`、`bd-3d8`、`bd-ieym`
3. **asupersync 能力加固**（AgentCx、可取消正确性）：`bd-3i7u`、`bd-1xf`

---

## 第一部分：库集成策略

### 1.1 asupersync 集成

**现状（今日）：** `pi_agent_rust` 基于 `asupersync` 运行运行时 + HTTP/TLS 与提供方流式（见 `src/http/client.rs` + `src/sse.rs`）。

**剩余：** 能力包装器（`AgentCx`）与更深层的上下文联动在 `bd-3i7u` 与 `bd-1xf` 中跟踪。

**拟利用的关键 API：**

```rust
use asupersync::{Cx, Outcome, Budget, Scope};
use asupersync::http::client::HttpClient;
use asupersync::tls::TlsConnectorBuilder;
use asupersync::database::sqlite::SqliteConnection;
use asupersync::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
```

**收益：**

- 结构化并发（无孤儿任务）
- 可取消正确的操作（有界清理）
- 通过 LabRuntime 实现确定性测试
- 开箱即用的 HTTP/TLS/SQLite

### 1.2 rich_rust 集成

**用途：** 全部非交互式终端输出

**使用模式：**

```rust
use rich_rust::prelude::*;

// Console 用于格式化输出
let console = Console::new();
console.print("[bold green]✓[/] Tool executed successfully");

// 表格用于结构化数据
let mut table = Table::new().title("Session Info");
table.add_row_cells(["Tokens", &format!("{}", usage.total_tokens)]);
console.print_renderable(&table);

// 面板用于 boxed 内容
let panel = Panel::from_text(&response).title("Assistant").width(80);
console.print_renderable(&panel);

// 长耗时操作的进度条
let bar = ProgressBar::new().width(40);
bar.set_progress(0.5);

// Markdown 渲染（需 "markdown" feature）
let md = Markdown::new(&response_text);
console.print_renderable(&md);

// 语法高亮（需 "syntax" feature）
let syntax = Syntax::new(&code, "rust");
console.print_renderable(&syntax);
```

**追加 Features：**

```toml
rich_rust = { path = "../rich_rust", features = ["syntax", "markdown", "full"] }
```

### 1.3 charmed_rust 集成

**用途：** 使用 Elm 架构的完整交互式 TUI

**架构：**

```rust
use bubbletea::{Program, Model, Message, Cmd, KeyMsg};
use lipgloss::{Style, Border, Position};
use bubbles::{textinput::TextInput, spinner::Spinner, viewport::Viewport};
use glamour::{render as render_markdown, Style as MdStyle};

struct PiTui {
    // 编辑器状态
    input: TextInput,
    history: Vec<String>,
    history_index: Option<usize>,

    // 展示状态
    viewport: Viewport,
    spinner: Spinner,
    status: StatusLine,

    // 智能体状态
    messages: Vec<Message>,
    streaming: bool,
    current_response: String,
    thinking: Option<String>,

    // 会话状态
    session: Session,
    config: Config,
}

impl Model for PiTui {
    fn init(&mut self) -> Cmd {
        Cmd::batch(vec![
            TextInput::blink(),
            self.spinner.tick(),
        ])
    }

    fn update(&mut self, msg: Message) -> Cmd {
        // 处理键盘输入、API 响应、工具结果
    }

    fn view(&self) -> String {
        // 使用 lipgloss 布局渲染
        let layout = lipgloss::join_vertical(Position::Left, &[
            self.render_header(),
            self.render_messages(),
            self.render_status(),
            self.render_input(),
        ]);
        layout
    }
}
```

**追加依赖：**

```toml
bubbletea = { path = "../charmed_rust", package = "bubbletea" }
lipgloss = { path = "../charmed_rust", package = "lipgloss" }
bubbles = { path = "../charmed_rust", package = "bubbles" }
glamour = { path = "../charmed_rust", package = "glamour" }
```

---

## 第二部分：实现阶段

### Phase 1: 修复既有问题 ✅ 已完成

**1.1 修复失败的夹具测试**

- [x] 排查 bash、edit、read、write 工具中 detail 字段序列化
- [x] 修复夹具中的 `details` 字段预期
- [x] 修复 bash 退出码 bug（遗留竞态；现已无 tokio 依赖）
- [x] 全部 67 个夹具用例通过

**1.2 清理既有代码**

- [x] 运行 `cargo clippy --all-targets` - 警告已处理
- [x] 运行 `cargo fmt` - 格式一致
- [x] 运行 `cargo test` - 全部 67 个测试通过

### Phase 2: 库依赖 ✅ 已完成

**2.1 更新 Cargo.toml** ✅

```toml
# 交互式 TUI（charmed_rust - Elm 架构）
bubbletea = { path = "../charmed_rust/crates/bubbletea" }
lipgloss = { path = "../charmed_rust/crates/lipgloss" }
bubbles = { path = "../charmed_rust/crates/bubbles" }
glamour = { path = "../charmed_rust/crates/glamour" }
```

**2.2 创建包装类型**

- 🔶 `AgentCx` - 智能体操作的能力上下文（在 `bd-3i7u` 中跟踪）
- [x] `RichConsole` - rich_rust Console 的包装，带 Pi 专属方法（`PiConsole`）
- [x] `TuiApp` - bubbletea Model 实现（`src/interactive.rs`）

### Phase 3: 交互式 TUI ✅ 已完成

**3.1 核心 TUI 结构**

- [x] `src/interactive.rs` - 主 Model 实现（PiApp）
- [x] 带历史导航（up/down）的 TextInput
- [x] 带 markdown 渲染（glamour）的消息展示
- [x] 带 token 计数与成本的状态 footer
- [x] 处理中的 Spinner
- [x] 工具执行状态展示
- [x] 斜杠命令系统

**3.2 编辑器组件**

- [x] 通过 bubbles TextArea 实现多行文本输入
- [x] 历史导航（up/down）
- [x] Ctrl+C 取消/退出
- [x] 空闲时 Esc 退出
- 🔶 补全弹窗（斜杠命令、文件路径）- 在 `bd-1iwi` 中跟踪（另见 `bd-3dr9`）
- ⬜ Shift+Enter 换行、Enter 提交 - 已延期（TextArea 已处理）

**3.3 消息展示**

- [x] 带 markdown 渲染（glamour）的助手响应
- [x] 思考块（内联展示）
- [x] 工具执行状态（spinner + 工具名）
- [x] 工具结果（格式化输出）
- ⬜ 图像（依赖终端）- 在 `bd-1iwi` 中跟踪

**3.4 斜杠命令** ✅ 已实现

- [x] `/help` - 展示可用命令
- [x] `/model` - 展示/切换模型
- [x] `/thinking` - 设置思考级别
- [x] `/history` - 展示输入历史
- [x] `/clear` - 清空对话
- [x] `/quit`（`/exit`）- 退出应用
- [x] `/export` - 导出为 HTML

**3.5 状态行**

- [x] 头部当前模型/提供方
- [x] Token 计数（输入/输出）
- [x] 成本（$X.XX）
- [x] 流式指示器（spinner）
- [x] 斜杠命令的状态消息

**3.6 智能体集成** ✅ 已完成

- [x] 用于异步智能体事件的 PiMsg 枚举
- [x] 从 submit_message() 接线智能体执行
- [x] 通过 channel 处理流式事件
- [x] 每轮后会话持久化

### Phase 4: 基于 asupersync 的提供方流式 ✅ 已完成

**4.1 创建 HTTP 模块**

- [x] `src/http/mod.rs` - HTTP 客户端抽象
- [x] `src/http/client.rs` - 基于 asupersync 的客户端
- [x] `src/http/sse.rs` - SSE 流式解析器（复用既有）

**4.2 迁移 Anthropic 提供方**

- ✅ 提供方流式使用 asupersync HTTP 客户端（`src/http/client.rs`）+ SSE 解析器（`src/sse.rs`）
- ✅ TLS 使用带原生根证书的 asupersync 连接器
- 🔶 额外的可取消正确性 + 确定性 LabRuntime 覆盖在 `bd-1xf` 中跟踪

**4.3 遗留清理**

- ✅ `Cargo.toml` 中已无 tokio/reqwest 依赖
- 🔶 剩余能力联动在 `bd-3i7u`（AgentCx）中跟踪

### Phase 5: 补充提供方 ✅ 已完成

**5.1 OpenAI 提供方** ✅

- [x] `src/providers/openai.rs`
- [x] Chat completions API
- [x] 流式支持
- [x] 函数调用（工具）
- [x] 单元测试

**5.2 Google Gemini 提供方** ✅

- [x] `src/providers/gemini.rs`
- [x] Generative AI API
- [x] 流式支持
- [x] 工具调用
- [x] 单元测试

**5.3 Azure OpenAI 提供方** ✅

- [x] `src/providers/azure.rs`
- [x] Azure 专属端点
- [x] 流式支持
- [x] 工具调用
- [x] 单元测试

### Phase 6: 会话增强 ✅ 大部完成

**6.1 SQLite 索引**

- ⬜ 基于 SQLite 的会话索引 + 搜索（已延期；需要时考虑新增专用 bead）
- [x] 快速会话列举（通过文件系统 mtime 排序）
- ⬜ Sync-from-JSONL 索引（已延期）

**6.2 树导航** ✅

- [x] 分支创建（`create_branch_from`）
- [x] 分支切换（`navigate_to`）
- [x] TUI 中的可视化树展示（`/tree` 命令）
- [x] 分支摘要支持

**6.3 会话选择器 UI** ✅

- [x] 列举最近会话（`src/session_picker.rs`）
- [x] 搜索/过滤（按目录）
- [x] 内容预览（展示模型、消息数）
- [x] 选择并恢复（`--resume` 标志）

### Phase 7: 一致性测试（2-3 天）

**7.1 参考捕获设施**

- ⬜ 工具参考捕获程序（TS/Go）- 已延期；当前聚焦无模拟 Rust 覆盖（`bd-26s`）

**7.2 工具一致性**

- ✅ 工具夹具套件位于 `tests/conformance/fixtures/`，由 `tests/conformance_fixtures.rs` 执行（见 `FEATURE_PARITY.md`）
- 🔶 扩展夹具覆盖 / 增加 fuzz 发现的边界用例 - 在 `bd-26s` 下跟踪

**7.3 提供方一致性**

- ✅ 基于 VCR 的提供方流式测试（无模拟）- `bd-h7r`、`bd-gd1`
- ⬜ 额外的错误/限流一致性 - 在 `bd-26s` 下跟踪

**7.4 会话格式一致性**

- ✅ 由集成测试覆盖（`tests/session_conformance.rs`）
- ⬜ 迁移测试（旧版本）- 已延期

### Phase 8: 基准测试 Harness（1-2 天）

**8.1 基准设施**

- ✅ 既有 benches 位于 `benches/`（见 `BENCHMARKS.md`）
- ⬜ 增加启动/TUI/流式微基准 - 按需延期

**8.2 性能目标**

| 指标         | 目标    | 测量方式                          |
| ------------ | ------- | --------------------------------- |
| 启动时间     | <100ms  | 冷启动到首个提示符                |
| TUI 帧率     | 60fps   | 持续渲染基准                      |
| 二进制体积   | <15MB   | `cargo build --release && ls -la` |
| 内存（空闲） | <30MB   | 启动后、首个请求前                |
| SSE 吞吐     | >10MB/s | 流式事件解析速率                  |

**8.3 CI 集成**

- ⬜ CI 基准自动化 + 回归检测 - 在 `bd-gqtd` 中跟踪
- ⬜ README 中的 Bench/status 徽章 - `bd-3nrc`

### Phase 9: 打磨与文档（1-2 天）

**9.1 错误消息**

- [x] 用户友好的错误格式化（rich_rust 面板）
- 🔶 可操作的建议 + 上下文感知提示 - `bd-3am2`

**9.2 文档**

- [x] 带架构的 README.md
- 🔶 Rust API 文档（rustdoc）- `bd-14od`
- 🔶 配置参考 + 故障排查 - `bd-3m7f`

**9.3 发布准备**

- 🔶 发布工程（版本/变更日志/交叉编译/发布）- `bd-gqtd`

### Phase 10: 扩展运行时（新增 - 主要剩余工作）

**10.1 PiJS 运行时**（完整规格见 `EXTENSIONS.md`）

- 🔶 在 `bd-btq` 中跟踪（PiJS 运行时总线；见 `br show bd-btq`）

**10.2 扩展 API**

- 🔶 在 `bd-btq` 下跟踪（联动工作：`bd-2i5`）

**10.3 扩展 UI**

- 🔶 UI 表面在 `bd-btq` 下跟踪（RPC 协议已存在；运行时集成待定）

**10.4 扩展发现与加载**

- 🔶 发现 + 安装解析在 `bd-1e0` 中跟踪

**10.5 一致性测试**

- 🔶 Harness + 夹具在 `bd-269` 中跟踪（基准：`bd-1fg`）

### Phase 11: 主题发现（新增）

**11.1 主题系统**

- 🔶 主题系统在 `bd-22p` 中跟踪（应用颜色 `bd-qpm`、`/theme` `bd-3d8`、settings `bd-ieym`）

---

## 第三部分：一致性测试策略

### 3.1 基于夹具的测试

每个工具拥有一个 JSON 夹具文件，结构如下：

```json
{
  "version": "1.1",
  "tool": "read",
  "reference_impl": "typescript",
  "captured_at": "2026-02-02T00:00:00Z",
  "cases": [
    {
      "name": "read_simple_file",
      "description": "Read a simple text file",
      "setup": [
        {
          "type": "create_file",
          "path": "test.txt",
          "content": "Hello\nWorld\n"
        }
      ],
      "input": { "path": "test.txt" },
      "expected": {
        "is_error": false,
        "content_contains": ["Hello", "World"],
        "content_regex": "^\\s*1→Hello",
        "details": {
          "lines_read": 2,
          "truncated": false
        }
      }
    }
  ]
}
```

### 3.2 参考捕获流程

1. **TypeScript 参考：**

   ```bash
   cd legacy_pi_mono_code/pi-mono
   pnpm test:capture -- --tool read --output ../fixtures/read_tool.json
   ```

2. **Go 参考（用于额外校验）：**

   ```bash
   cd tests/conformance/go_reference
   go run ./cmd/capture_read --output ../fixtures/go_read_tool.json
   ```

3. **Rust 实现测试：**
   ```bash
   cargo test conformance::test_read -- --nocapture
   ```

### 3.3 覆盖率目标

| 组件               | 用例数   | 状态        |
| ------------------ | -------- | ----------- |
| read 工具          | 25+      | 10 完成     |
| write 工具         | 15+      | 7 完成      |
| edit 工具          | 20+      | 6 完成      |
| bash 工具          | 30+      | 12 完成     |
| grep 工具          | 25+      | 12 完成     |
| find 工具          | 15+      | 6 完成      |
| ls 工具            | 15+      | 8 完成      |
| truncation         | 20+      | 9 完成      |
| SSE 解析           | 30+      | 11 完成     |
| Session format     | 25+      | 0 完成      |
| Provider responses | 20+      | 0 完成      |
| **总计**           | **240+** | **81 完成** |

---

## 第四部分：架构图

```
┌────────────────────────────────────────────────────────────────────┐
│                          pi CLI Binary                              │
├────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────┐│
│  │   CLI       │───▶│   Config    │───▶│      Agent Loop         ││
│  │  (clap)     │    │   Loader    │    │   (tool iteration)      ││
│  └─────────────┘    └─────────────┘    └───────────┬─────────────┘│
│                                                     │              │
│  ┌──────────────────────────────────────────────────┼──────────────┤
│  │                    TUI Layer (charmed_rust)      │              │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────▼───────────┐ │
│  │  │   Editor    │  │   Status    │  │     Message Display     │ │
│  │  │ (bubbles)   │  │   Line      │  │  (glamour markdown)     │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│  │  ┌─────────────┐  ┌─────────────┐                              │
│  │  │  Thinking   │  │   Slash     │                              │
│  │  │   Block     │  │  Commands   │                              │
│  │  └─────────────┘  └─────────────┘                              │
│  └─────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┤
│  │                 Provider Layer (asupersync HTTP)                │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  │  Anthropic  │  │   OpenAI    │  │   Google    │             │
│  │  │  Provider   │  │  Provider   │  │  Provider   │             │
│  │  └─────────────┘  └─────────────┘  └─────────────┘             │
│  └─────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┤
│  │                    Tool Layer                                    │
│  │  ┌────┐ ┌────┐ ┌────┐ ┌─────┐ ┌────┐ ┌────┐ ┌──┐              │
│  │  │read│ │bash│ │edit│ │write│ │grep│ │find│ │ls│              │
│  │  └────┘ └────┘ └────┘ └─────┘ └────┘ └────┘ └──┘              │
│  └─────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┤
│  │              Session Layer (JSONL + SQLite index)               │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  │   JSONL     │  │   SQLite    │  │    Tree     │             │
│  │  │   Files     │  │   Index     │  │  Navigator  │             │
│  │  └─────────────┘  └─────────────┘  └─────────────┘             │
│  └─────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┤
│  │              Output Layer (rich_rust)                           │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  │  Console    │  │   Tables    │  │   Panels    │             │
│  │  │  (markup)   │  │  Progress   │  │  Spinners   │             │
│  │  └─────────────┘  └─────────────┘  └─────────────┘             │
│  └─────────────────────────────────────────────────────────────────┤
└────────────────────────────────────────────────────────────────────┘
```

---

## 第五部分：时间线估算

| 阶段                              | 时长        | 状态                                | 依赖       |
| --------------------------------- | ----------- | ----------------------------------- | ---------- |
| Phase 1: 修复问题                 | 1-2 天      | ✅ 已完成                           | 无         |
| Phase 2: 依赖                     | 1 天        | ✅ 已完成                           | Phase 1    |
| Phase 3: 交互式 TUI               | 3-5 天      | ✅ 已完成                           | Phase 2    |
| Phase 4: 提供方流式（asupersync） | 2-3 天      | ✅ 已完成                           | Phase 2    |
| Phase 5: 提供方                   | 2-3 天      | ✅ 已完成                           | Phase 4    |
| Phase 6: 会话                     | 2 天        | ✅ 大部完成                         | Phase 3    |
| Phase 7: 工具一致性               | 2-3 天      | ✅ 已完成（见 `FEATURE_PARITY.md`） | Phases 1-6 |
| Phase 8: 基准测试                 | 1-2 天      | ✅ 已完成                           | Phase 7    |
| Phase 9: 打磨                     | 1-2 天      | 🔶 进行中                           | Phase 8    |
| **Phase 10: 扩展**                | **3-5 天**  | 🔶 进行中（`bd-btq`）               | Phase 4    |
| **Phase 11: 主题**                | **1-2 天**  | 🔶 进行中（`bd-22p`）               | Phase 9    |
| **剩余**                          | **~5-8 天** |                                     |            |

---

## 第六部分：成功标准

### 功能需求

- ✅ 核心对等状态在 `FEATURE_PARITY.md` 中跟踪
- 🔶 扩展运行时对等在 `bd-btq`（+ 子任务）中跟踪
- 🔶 主题对等在 `bd-22p`（+ `bd-qpm`、`bd-3d8`、`bd-ieym`）中跟踪

### 性能需求

- ✅ 体积/启动目标在 README + `BENCHMARKS.md` 中跟踪
- 🔶 扩展性能目标 + 证据在 `bd-20p` 中跟踪（基准：`bd-1fg`）

### 质量需求

- ✅ 禁止 `unsafe`（`#![forbid(unsafe_code)]`）
- 🔶 零 clippy + 全量门禁在 CI / 发布工作流中强制（`bd-gqtd`）
- 🔶 Rust API 文档（rustdoc）在 `bd-14od` 中跟踪

### 一致性需求

- ✅ 工具夹具存在并在 CI 中运行（计数见 `FEATURE_PARITY.md`）
- 🔶 通过 VCR 的提供方流式一致性（无模拟）：`bd-h7r`、`bd-gd1`
- ⬜ 会话格式迁移夹具/测试 - 已延期

---

## 第七部分：待创建/修改文件

### 新文件

```
src/tui/mod.rs           # TUI 模块组织
src/tui/app.rs           # 主 Model 实现
src/tui/input.rs         # 多行编辑器
src/tui/messages.rs      # 消息展示
src/tui/status.rs        # 状态行
src/tui/thinking.rs      # 思考块
src/tui/commands.rs      # 斜杠命令

src/http/mod.rs          # HTTP 抽象
src/http/client.rs       # asupersync HTTP 客户端

src/providers/openai.rs  # OpenAI 提供方
src/providers/google.rs  # Google Gemini 提供方

src/session/index.rs     # SQLite 会话索引
src/session/tree.rs      # 树导航

benches/startup.rs       # 启动基准
benches/tui_render.rs    # TUI 基准
benches/tools.rs         # 工具基准
benches/streaming.rs     # SSE 基准

tests/conformance/reference/    # 参考实现
tests/conformance/fixtures/     # 扩展夹具
```

### 待修改文件

```
Cargo.toml               # 添加 charmed_rust、更新 asupersync features
src/main.rs              # 接线 TUI
src/agent.rs             # 使用 AgentCx
src/providers/anthropic.rs # 迁移到 asupersync HTTP
src/session.rs           # 添加 SQLite 索引、树导航
src/tools.rs             # 修复 detail 字段序列化
FEATURE_PARITY.md        # 随功能完成更新
```

---

## 紧急下一步

1. ~~**修复失败的夹具测试** - detail 字段序列化~~ ✅ 已完成
2. ~~**添加 charmed_rust 依赖**到 Cargo.toml~~ ✅ 已完成
3. ~~**创建 TUI 模块结构**含基础 Model 实现~~ ✅ 已完成
4. ~~**使用 bubbles TextInput 实现多行编辑器**~~ ✅ 已完成
5. ~~**运行 `cargo check`** 验证全部可编译~~ ✅ 已完成

**当前优先级：**

使用 Beads 查看实时队列：

- `bv --robot-plan`
- `bv --robot-priority`
- `br ready`

高层工作流：

1. **扩展运行时** - `bd-btq`（发现：`bd-1e0`）
2. **主题** - `bd-22p`（应用颜色：`bd-qpm`）
3. **asupersync 能力加固** - `bd-3i7u`、`bd-1xf`
4. **VCR 测试设施** - `bd-30u`、`bd-h7r`、`bd-gd1`
