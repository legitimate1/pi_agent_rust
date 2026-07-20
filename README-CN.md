<p align="center">
  <img src="pi_agent_rust_illustration.webp" alt="Pi Agent Rust" width="600"/>
</p>

<h1 align="center">pi_agent_rust</h1>

<p align="center">
  <strong>pi_agent_rust - 用 Rust 编写的高性能 AI 编程智能体 CLI</strong>
</p>

<p align="center">
  <a href="#why-should-you-care">为什么值得关注？</a> •
  <a href="#tldr-piopenclaw-users">概要</a> •
  <a href="#benchmark-methodology-and-claim-integrity">方法论</a> •
  <a href="#quick-start">快速开始</a> •
  <a href="#features">功能特性</a> •
  <a href="#installation">安装</a> •
  <a href="#commands">命令</a> •
  <a href="#configuration">配置</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-2024%20edition-orange?logo=rust" alt="Rust 2024">
  <img src="https://img.shields.io/badge/license-MIT%20%2B%20Rider-blue" alt="许可证: MIT + Rider">
  <img src="https://img.shields.io/badge/unsafe-forbidden-brightgreen" alt="禁止不安全代码">
</p>

```bash
# 安装最新发布版
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | bash
```

---

## 问题

你希望在终端中拥有一个 AI 编程助手，但现有工具存在以下问题：
- **启动缓慢**：Node.js/Python 运行时在你能输入之前就增加了 500ms+ 的延迟
- **内存占用高**：Electron 应用或重型运行时消耗数 GB 内存
- **不可靠**：流式传输中断、会话损坏、工具静默失败
- **难以扩展**：封闭的生态系统或复杂的插件系统

## 解决方案

**pi_agent_rust** 是从头编写的 [Pi Agent](https://github.com/badlogic/pi)（作者 [Mario Zechner](https://github.com/badlogic)）的 Rust 移植版（已获作者许可！）。单二进制文件，瞬间启动，稳定的流式传输，内置 8 个工具。

这不是逐行翻译，而是基于两个专为本次移植构建的 Rust 库：
- **[asupersync](https://github.com/Dicklesworthstone/asupersync)**：结构化并发异步运行时，内置 HTTP、TLS 和 SQLite
- **[rich_rust](https://github.com/Dicklesworthstone/rich_rust)**：Will McGugan 的 [Rich](https://github.com/Textualize/rich) 的 Rust 移植版，提供带标记语法的精美终端输出

```bash
# 启动会话
pi "Help me refactor this function to use async/await"

# 继续之前的会话
pi --continue

# 单次模式（无会话）
pi -p "What does this error mean?" < error.log
```

## 为什么值得关注？

如果你已经在使用 Pi Agent（尤其是通过 OpenClaw），这个项目保留了核心工作流，同时升级了底层引擎：

- **在真实的端到端流程中显著更快**（非合成微基准测试）
- **长时间运行会话的内存占用大幅降低**
- **扩展/工具执行的安全模型实质性增强**，包括命令级阻止危险的扩展 shell 模式

安全性是本项目的首要设计目标，而非事后补救：

- 基于能力的主机调用门控（`tool`/`exec`/`http`/`session`/`ui`/`events`）
- 两阶段扩展 `exec` 执行：先能力门控，再命令调解——默认阻止关键 shell 类别（例如递归删除、磁盘/设备写入、反向 shell），可在严格/安全策略下收紧以阻止高等级类别
- 执行路径上的策略 + 运行时风险 + 配额强制执行
- 每个扩展的信任生命周期（`pending` -> `acknowledged` -> `trusted` -> `killed`），带 kill-switch 审计日志和操作员显式溯源
- 主机调用通道紧急控制，可在快速通道行为需要立即限制时，强制全局或单个扩展使用兼容通道执行
- 通过 `asupersync` 实现结构化并发，提供更可预测的取消/生命周期行为
- 可审计的运行时信号/账本和扩展行为的脱敏安全告警

## 概要（Pi/OpenClaw 用户）

Rust 移植版专为大会话、多智能体和扩展密集型工作负载设计。面向发布版的性能数据仅在已签入的证据制品为最新、具有匹配的运行溯源、且无 CI 无数据或数据合同失败时才发布。历史基准快照保留在规划/证据制品中，但在性能证据门禁重新生成干净之前，不视为当前 README 声明。

扩展运行时保证也是具体的：

| 扩展保证信号 | 为什么你应该关注 |
|---|---|
| 两阶段 `exec` 守卫（`exec` 能力策略 + 命令级调解 + DCG/heredoc AST 信号） | 危险 shell 意图在生成之前即被捕获，包括隐藏在多行包装器中的破坏性负载 |
| 信任生命周期 + kill-switch（`pending/acknowledged/trusted/killed`） | 你可以立即隔离扩展，记录谁拉下了开关及原因，在恢复访问前需要显式重新确认 |
| 主机调用通道 kill-switch 控制（`forced_compat_global_kill_switch`、`forced_compat_extension_kill_switch`） | 快速路径回归可通过强制使用兼容通道执行来立即控制，无需禁用扩展系统 |
| 确定性主机调用反应器网格（分片亲和性、有界 SPSC 通道、背压遥测、可选 NUMA slab 跟踪） | 运行时行为在争用下保持可预测；队列压力和路由决策可观察而非不透明 |
| JS 运行时的启动预暖 + 暖隔离区复用 | 运行时创建与启动重叠，暖复用使重复扩展运行保持低延迟，无需 Node/Bun 进程模型 |
| 防篡改运行时风险账本（`verify`/`replay`/`calibrate`） | 安全决策通过哈希链关联，可从真实运行时轨迹重放或阈值调优 |

底线：Pi 的架构目标是更低的延迟、更低的内存使用以及在真实工作负载压力下更强的扩展运行时安全性；当前的数值声明必须来自新鲜的、溯源匹配的证据制品。

<sub>数据来源：`docs/planning/BENCHMARK_COMPARISON_BETWEEN_RUST_VERSION_AND_ORIGINAL__GPT.md`（最新安全路径 + 完整编排检查点，2026-04-23）。</sub>

### README 引用约定

本 README 中的所有数值性能声明均包含内联引用，格式为：
`*(来源：[artifact-path]，运行 [correlation-id])*`

示例：`*(来源：[artifact-path]，运行 [correlation-id])*`

CI 检查文件新鲜度和制品内容，因此过时、无数据或关联不匹配的证据无法支持面向用户的性能声明。README 证据检查器报告已引用声明的带行号证明义务，并提取声明门控的性能短语供审查者审计。显式的历史快照引用单独映射，不满足当前面向发布版的声明要求。

## 我们如何做到如此之快

在本 README 中，`我们`指项目所有者和协作的编程智能体。
速度提升来自运行时设计，而非单一技巧。

| 技术 | 我们做了什么 | 运行时效果 |
|---|---|---|
| 冷启动最小化 | 单一静态二进制，无 Node/Bun 运行时引导，无 JIT 预热，扩展运行时路径的启动预暖 | 更快的首次交互时间 |
| 热路径减少复制 | `Arc`/`Cow` 消息流，零拷贝主机调用/工具负载处理，减少高克隆的 provider/会话路径 | 更低的 CPU 和分配压力 |
| 确定性分发核心 | 类型化主机调用操作码，快速通道/兼容通道路由，带反应器网格遥测的有界分片队列 | 并发扩展负载下更好的尾延迟 |
| 高效长会话存储 | SQLite 会话索引 + v2 sidecar（分段日志 + 偏移索引），O(index+tail) 重开路径 | 大历史记录快速恢复 |
| 针对真实网络调优的流式解析器 | SSE 解析器跟踪扫描字节，处理 UTF-8 尾部，标准化分块边界，驻留事件类型字符串 | 更低的流式开销和更少的解析器停顿 |
| 安全快速路径控制 | 影子双执行采样，分歧/开销时自动回退，兼容通道 kill-switch 用于限制 | 保持优化快速，无静默行为漂移 |
| CI 级性能治理 | 场景矩阵，严格制品合同，失败关闭的性能门禁 | 回归在发布前即被捕获 |

如果你想查看完整的实现清单，请参阅[性能工程](#performance-engineering)。

## 基准测试方法与声明完整性

基准证据策略旨在保持结果真实、可复现且难以造假。

我们测量的内容：

- **匹配状态工作负载**：恢复一个大会话并追加相同的 10 条消息。
- **真实 E2E 工作负载**：恢复 + 追加 + 扩展活动 + 斜杠风格状态变更 + 分叉 + 导出 + 压缩。
- **规模级别**：从 `100k` 到 `5M` token 级别的会话状态。
- **启动/就绪**：命令级就绪（`--help`、`--version`）与长会话工作负载分开测量。

我们如何保持比较公平：

- **基准报告中有两个范围**：
  - 苹果对苹果（`pi_agent_rust` vs 旧版 `coding-agent`）
  - 苹果对橙子（包含旧版堆栈组件，在旧版行为外包的情况下）
- **发布模式二进制文件**，每个矩阵单元重复运行。
- **核心延迟/内存表中无付费 provider 噪声**（provider 调用成本从这些核心比较中排除）。

我们如何保持声明诚实：

- **安全控制在安全路径测量期间保持开启**（无策略/风险/配额绕过以换取速度声明）。
- **原始制品被保留**（JSON/跟踪/时间输出）并在基准报告中注明。
- **阻塞因素被明确披露**：当缺少工作区依赖阻止直接旧版重运行时，我们说明这一点，并与先前验证的旧版制品进行比较，而不是假装重运行成功。
- **解读说明明确**：报告区分基线部分与新鲜重运行，使读者能确切看到哪些值来自哪组运行。
- **可复现性优先于营销**：方法论、注意事项和已知限制与成绩一起包含在内。

如果你想了解完整详情，请参阅：

- `docs/planning/BENCHMARK_COMPARISON_BETWEEN_RUST_VERSION_AND_ORIGINAL__GPT.md`（方法论 + 结果 + 注意事项 + 原始制品路径）

## 为什么选择 Pi？

| 特性 | Pi (Rust) | 典型 TS/Python CLI |
|---------|-----------|----------------------|
| **启动** | <100ms | 500ms-2s |
| **二进制大小** | ~21.1 MiB（默认发布版） | 100MB+（含运行时）|
| **内存（空闲）** | <50MB | 200MB+ |
| **流式传输** | 原生 SSE 解析器 | 依赖库 |
| **工具执行** | 进程树管理 | 基本子进程 |
| **会话** | 带分支的 JSONL | 各有不同 |
| **不安全代码** | 禁止 | 不适用 |

## 快速示例

```bash
# 1) 启动交互式会话
pi

# 2) 询问代码库问题
pi "Summarize the architecture in src/"

# 3) 内联附加文件
pi @src/main.rs "Explain startup flow"

# 4) 运行单次模式用于脚本
pi -p "List likely regression risks for this diff"

# 5) 继续你上一个项目会话
pi --continue

# 6) 查看可用的模型/provider
pi --list-models
pi --list-providers
```

---

## 基础库

### asupersync

[asupersync](https://github.com/Dicklesworthstone/asupersync) 是一个结构化并发异步运行时，专为需要可预测资源清理的应用设计。pi_agent_rust 使用的主要特性：

- **基于能力的上下文（`Cx`）**：异步函数接收显式上下文，控制它们能做什么（HTTP、文件系统、时间）。这使得测试具有确定性。
- **带 TLS 的 HTTP 客户端**：内置 HTTP API，使用 rustls，避免 OpenSSL 依赖地狱
- **结构化取消**：当父任务取消时，所有子任务干净地取消。无悬空 future。

`pi_agent_rust` 目前端到端运行在 `asupersync` 上（运行时 + HTTP/TLS + 取消）。Provider 流式传输使用一个最小 HTTP 客户端（`src/http/client.rs`）提供数据给自定义 SSE 解析器（`src/sse.rs`）。

### rich_rust

[rich_rust](https://github.com/Dicklesworthstone/rich_rust) 是 Will McGugan 的 [Rich](https://github.com/Textualize/rich) Python 库的 Rust 移植版。它提供：

- **标记语法**：`[bold red]error[/]` 渲染为粗体红色文本
- **表格**：ASCII/Unicode 表格渲染，支持对齐和边框
- **面板**：带标题的框式内容
- **进度条**：动画进度指示器
- **Markdown**：终端渲染的 markdown，支持语法高亮
- **主题**：跨组件的一致配色方案

终端 UI 使用 rich_rust 进行所有输出格式化，提供与基于 Rich 的 Python 工具相同的视觉质量。

---

## 快速开始

### 1. 安装

```bash
# 安装最新发布版二进制文件
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | bash
```

如果你已经安装了原始的 TypeScript 版 `pi`，安装程序会询问是否将 Rust Pi 设为规范的 `pi` 命令，并自动创建 `legacy-pi` 用于旧命令。

### 2. 配置 API 密钥

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### 3. 运行

```bash
# 交互模式
pi

# 附带初始消息
pi "Explain this codebase structure"

# 读取文件作为上下文
pi @src/main.rs "What does this do?"
```

---

## 功能特性

### 流式响应

实时 token 流式传输，支持扩展思考：

```
pi "Write a quicksort implementation"
```

观看响应逐个 token 出现，思考块内联显示。

### 8 个内置工具

| 工具 | 描述 | 示例 |
|------|------|------|
| `read` | 读取文件内容，支持图片 | 读取 src/main.rs |
| `write` | 创建或覆盖文件 | 写入新的配置文件 |
| `edit` | 精确字符串替换 | 修复第 42 行的拼写错误 |
| `hashline_edit` | 使用 LINE#HASH 标签进行精确编辑 | 使用 hashline 锚点对特定行应用编辑 |
| `bash` | 带超时的 shell 命令执行 | 运行测试套件 |
| `grep` | 搜索文件内容，支持上下文 | 查找所有 TODO 注释 |
| `find` | 按模式搜索文件 | 查找所有 *.rs 文件 |
| `ls` | 列出目录内容 | src/ 里有什么？ |

所有工具包括：
- 大输出自动截断（2000 行 / 1MB）
- 响应中的详细元数据
- bash 的进程树清理（无悬空进程）

### 会话管理

会话持久化为 JSONL 文件，包含完整的对话历史：

```bash
# 继续最近的会话
pi --continue

# 打开特定会话
pi --session ~/.pi/agent/sessions/--home-user-project--/2024-01-15T10-30-00.jsonl

# 临时会话（不持久化）
pi --no-session
```

会话支持：
- 树结构，用于对话分支
- 模型/思考级别变更跟踪
- 长对话自动压缩

### 扩展思考

为复杂问题启用深度推理：

```bash
pi --thinking high "Design a distributed rate limiter"
```

思考级别：`off`、`minimal`、`low`、`medium`、`high`、`xhigh`

### 自定义（技能与提示词模板）

- **技能**：将 `SKILL.md` 放在 `~/.pi/agent/skills/` 或 `.pi/skills/` 下，用 `/skill:name` 调用。
- **提示词模板**：`~/.pi/agent/prompts/` 或 `.pi/prompts/` 下的 Markdown 文件；通过 `/<template> [args]` 调用。
- **包**：使用 `pi install npm:@org/pi-packages` 共享包（技能、提示词、主题、扩展）。

### 自动补全

Pi 在交互式编辑器中提供上下文感知的自动补全：

- **`@` 文件引用**：输入 `@` 后跟路径片段即可附加文件内容。补全引擎索引项目文件（遵循 `.gitignore`），通过 `ignore` crate 的 `WalkBuilder` 实现，上限为 5,000 条。
- **`/` 斜杠命令**：内置命令（`/help`、`/model`、`/tree`、`/clear`、`/compact`、`/exit`）和用户定义的提示词模板及技能都显示为补全项。
- **模糊评分**：前缀匹配优先于子串匹配。结果按匹配质量排序，然后按类型排序（命令 > 模板 > 技能 > 文件 > 路径）。
- **后台刷新**：后台线程每 30 秒重新索引项目文件树，使补全保持最新且不阻塞输入循环。

### 三种执行模式

Pi 有三种运行模式，每种适用于不同的工作流：

| 模式 | 调用方式 | 适用场景 |
|------|---------|----------|
| **交互模式** | `pi`（默认）| 完整 TUI，带流式传输、工具、会话分支、自动补全 |
| **打印模式** | `pi -p "..."` | 单次响应到 stdout，无 TUI，可脚本化 |
| **RPC 模式** | `pi --mode rpc` | 无头 JSON 协议，通过 stdin/stdout 用于 IDE 集成 |

**交互模式**提供完整体验：带历史记录的多行文本编辑器、可滚动对话视口、模型选择器（`Ctrl+L`）、作用域模型切换（`Ctrl+P`/`Ctrl+Shift+P`）、会话分支导航器（`/tree`）以及实时 token/成本跟踪。

**打印模式**发送一条消息，将响应流式输出到 stdout，然后退出。适用于 shell 脚本和一次性查询。

**RPC 模式**暴露一个行分隔的 JSON 协议，用于程序化控制。客户端发送命令（`prompt`、`steer`、`follow-up`、`abort`、`get-state`、`compact`）并接收流式事件。这是 IDE 扩展和自定义前端与 Pi 集成的方式。参见 [RPC 协议](#rpc-protocol)了解线格式。

### 扩展

Pi 支持两个扩展运行时家族，带基于能力的主机连接器门控：

- JS/TS 入口点**无需 Node 或 Bun**，在嵌入式 QuickJS 运行时中运行。
- `*.native.json` 描述符在原生 Rust 描述符运行时中运行。

- 扩展入口点自动检测：
  - `.js/.ts/.mjs/.cjs/.tsx/.mts/.cts` 直接在嵌入式 QuickJS 中运行（无需描述符转换）。
  - `*.native.json` 加载原生 Rust 描述符运行时。
  - 一个会话目前一次使用一个运行时家族（JS/TS 或原生描述符）。
- **冷加载 <100ms**（P95），**暖加载 <1ms**（P99）
- 为 `fs`、`path`、`os`、`crypto`、`child_process`、`url` 等提供 Node API shim
- 基于能力的安全性：扩展调用显式的连接器（`tool/exec/http/session/ui`），带审计日志
- 命令级 exec 调解：危险 shell 签名在生成前被分类并阻止，带脱敏拒绝告警和调解账本条目
- 信任状态生命周期和 kill-switch 控制，带审计的状态转换（`pending`/`acknowledged`/`trusted`/`killed`）
- 主机调用反应器网格，带确定性分片路由、有界队列背压和可选的 NUMA 感知遥测
- 运行时预暖路径，带暖隔离区复用，使扩展启动成本在第一个提示词之前就基本支付

### 凭据感知的模型选择

- `/model`（或 `Ctrl+L`）打开一个选择器，聚焦于当前凭据可立即运行的模型。
- `Ctrl+P` 和 `Ctrl+Shift+P` 在不打开覆盖层的情况下循环浏览作用域模型集。
- Provider ID 和别名在模型选择和 `/login` 中大小写不敏感匹配。
- 不需要配置凭据的模型可以无密钥运行。

扩展可以注册工具、斜杠命令、事件钩子、标志、provider 和快捷键。详见 [EXTENSIONS.md](docs/planning/EXTENSIONS.md) 了解完整架构，以及 [docs/extension-catalog.json](docs/extension-catalog.json) 了解包含 223 条条目的目录（每个扩展的合规状态和性能预算）。

## 扩展验证管道

本项目通过三轨管道验证扩展兼容性：

- **供应商内 corpus（224 条）**：确定性合规、兼容性矩阵和场景套件。
- **供应商外 corpus（777 条）**：源码获取和接入优先级排序。
- **发布版二进制实时 provider E2E**：真实的 `target/release/pi` 执行，针对非模拟的 provider/模型路径。

### 为什么存在

- 捕获 QuickJS 主机 shim 和能力策略中的运行时/API 回归。
- 通过发布版二进制路径上的真实命令调解捕获危险的扩展 shell 调用模式。
- 针对真实 provider 响应验证扩展行为，而不仅仅是夹具/模拟流程。
- 使扩展支持可量化而非凭传闻。
- 为将供应商外候选者接入供应商内合规生成优先级队列。

### 管道组件

1. **获取供应商外源代码 corpus**
   - 二进制：`ext_unvendored_fetch_run`
   - 典型命令：
     - `cargo run --example ext_unvendored_fetch_run -- run-all --workers 8 --no-probe`
   - 目的：
     - 克隆 GitHub 仓库并将 npm tarball 解压到 `.tmp-codex-unvendored-cache/`
     - 为所有供应商外候选者生成机器可读的获取状态
   - 制品：
     - `tests/ext_conformance/reports/pipeline/unvendored_fetch_probe_report.json`
     - `tests/ext_conformance/reports/pipeline/unvendored_fetch_probe_events.jsonl`

2. **运行端到端验证编排**
   - 二进制：`ext_full_validation`
   - 典型命令：
     - `cargo run --example ext_full_validation --`
   - 阶段（按顺序）：
     1. `refresh_onboarding_queue`（运行 `ext_onboarding_queue`）
     2. `conformance_shard_0..N`（运行 `ext_conformance_generated` 分片矩阵）
     3. `conformance_failure_dossiers`
     4. `provider_compat_matrix`
     5. `scenario_conformance_suite`
     6. `auto_repair_full_corpus`
     7. `differential_suite`（可选，通过 `--run-diff` 启用；npm diff 通过 `--run-npm-diff`）
   - 制品：
     - `tests/ext_conformance/reports/pipeline/full_validation_report.json`
     - `tests/ext_conformance/reports/pipeline/full_validation_report.md`
     - 以及 `tests/ext_conformance/reports/**` 下的阶段特定报告

3. **运行开发者优先集实时 provider 门禁（发布构建前必须通过）**
   - 二进制：`ext_release_binary_e2e`
   - 典型命令：
     - `cargo build --bin pi --bin ext_release_binary_e2e`
     - `PI_HTTP_REQUEST_TIMEOUT_SECS=0 target/debug/ext_release_binary_e2e --pi-bin target/debug/pi --provider ollama --model qwen2.5:0.5b --jobs 10 --timeout-secs 600 --max-cases 20 --extension-policy balanced --out-json tests/ext_conformance/reports/release_binary_e2e/ollama_firstset_dev_20260219_jobs10_timeout600.json --out-md tests/ext_conformance/reports/release_binary_e2e/ollama_firstset_dev_20260219_jobs10_timeout600.md`
   - 目的：
     - 在支付发布构建成本之前，证明当前代码路径在代表性优先集上端到端工作。
     - 作为全面发布版二进制验证的晋升门禁。
   - 门禁：
     - 要求 `pass=20 / total=20`，`fail=0`。
   - 制品：
     - `tests/ext_conformance/reports/release_binary_e2e/ollama_firstset_dev_20260219_jobs10_timeout600.json`
     - `tests/ext_conformance/reports/release_binary_e2e/ollama_firstset_dev_20260219_jobs10_timeout600.md`

4. **运行完整发布版二进制实时 provider E2E（步骤 3 通过后）**
   - 二进制：`ext_release_binary_e2e`
   - 典型命令：
     - `cargo build --release --bin pi --bin ext_release_binary_e2e`
     - `PI_HTTP_REQUEST_TIMEOUT_SECS=0 target/release/ext_release_binary_e2e --pi-bin target/release/pi --provider ollama --model qwen2.5:0.5b --jobs 10 --timeout-secs 600 --extension-policy balanced --out-json tests/ext_conformance/reports/release_binary_e2e/ollama_full_release_20260219_jobs10_timeout600.json --out-md tests/ext_conformance/reports/release_binary_e2e/ollama_full_release_20260219_jobs10_timeout600.md`
   - 目的：
     - 为每个选定的扩展用例直接执行 `target/release/pi`。
     - 使用实时 provider/模型路径（默认 `ollama` + `qwen2.5:0.5b`）来演练非模拟的端到端行为。
     - 输出每个用例的 stdout/stderr 捕获及摘要制品（`pi.ext.release_binary_e2e.v1`）。
   - 制品：
     - `tests/ext_conformance/reports/release_binary_e2e/ollama_full_release_20260219_jobs10_timeout600.json`
     - `tests/ext_conformance/reports/release_binary_e2e/ollama_full_release_20260219_jobs10_timeout600.md`
     - `tests/ext_conformance/reports/release_binary_e2e/cases/*`

5. **聚合与分类**
   - `full_validation_report.json` 结合：
     - 阶段级通过/失败（`stageSummary`、`stageResults`）
     - Corpus 计数（`corpus`）
     - 供应商内合规总计（`conformance`）
     - Provider 矩阵总计（`providerCompat`）
     - 场景总计（`scenario`）
     - 审查队列 + 判定分类（`reviewQueue`、`verdictCounts`）
   - 重要解读规则：
     - `not_tested_unvendored` 表示尚未进入供应商内合规的供应商外候选者；这是库存状态，而非供应商内回归。

### 推荐的运行环境

这些运行编译大量 crate，可能占用大量磁盘空间。将 Cargo 制品和临时文件指向大容量卷：

```bash
export CARGO_TARGET_DIR="/data/tmp/pi_agent_rust_cargo/${USER:-agent}/target"
export TMPDIR="/data/tmp/pi_agent_rust_cargo/${USER:-agent}/tmp"
mkdir -p "$CARGO_TARGET_DIR" "$TMPDIR"
```

然后运行：

```bash
cargo run --example ext_unvendored_fetch_run -- run-all --workers 8 --no-probe
cargo run --example ext_full_validation --
```

### 最新运行快照（扩展门禁刷新 2026-05-15）

来源：
- `tests/ext_conformance/reports/gate/must_pass_gate_verdict.json`（生成于 `2026-05-15T17:03:02.000Z`，运行 `local-20260515T170218075Z`）
- `tests/ext_conformance/reports/health_delta/health_delta_report.json`（生成于 `2026-05-13T03:37:59.568Z`）
- `tests/ext_conformance/reports/journeys/journey_report.json`（生成于 `2026-05-13T02:59:58.302Z`）
- `tests/evidence_bundle/index.json`（生成于 `2026-05-12T19:26:21.441Z`，运行 `local-20260512T192621Z`）
- `tests/full_suite_gate/certification_verdict.json`（生成于 `2026-05-14T19:59:37.227Z`）
- `docs/evidence/dropin-certification-verdict.json`（生成于 `2026-05-18T19:37:26Z`）

- 严格 drop-in 状态：**22/22 认证门禁通过，16/16 阻塞门禁通过** - `CERTIFIED` *（来源：docs/evidence/dropin-certification-verdict.json；严格替换措辞仍由 docs/contracts/dropin-certification-contract.json 和本判定制品管理）*
- 统一证据包：`29/29` 个部分存在，`0` 个缺失，`0` 个无效 *（来源：tests/evidence_bundle/index.json）*
- 扩展必过门禁：`123/123` 个必须通过的扩展已通过；信息性延伸集 `100/101` 通过，一个非阻塞延伸失败 *（来源：tests/ext_conformance/reports/gate/must_pass_gate_verdict.json）*
- 扩展健康增量：`223/223` 个测试的扩展通过（`100.0%`），`0` 个回归，`13` 个修复（相比 2026-02-07 基线），其中 `1` 个故意排除的测试夹具在报告中披露 *（来源：tests/ext_conformance/reports/health_delta/health_delta_report.json）*
- 健康增量完整清单非通过扩展：`0` 个；`base_fixtures` 是仅用于测试的负面夹具，从面向发布版的通过率声明中排除，处置记录在 `docs/evidence/extension-health-delta-failure-disposition.json` 中。
- 扩展旅程覆盖率：`123/123` 个旅程场景通过（`100.0%`）；命令、事件订阅者、多能力、被动和工具 provider 类别均为绿色 *（来源：tests/ext_conformance/reports/journeys/journey_report.json）*
- 压力分类：`1,500` 个事件，`0` 个错误，p99 延迟 `396us`，RSS 增长 `0.0%` *（来源：tests/perf/reports/stress_triage.json，运行 bd-2zcs5.71-darkgoose-20260510T0058Z）*

---

## 安装

### Curl 安装程序（推荐）

```bash
# 最新发布版
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | bash

# 非交互 + 自动 PATH 更新
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | bash -s -- --yes --easy-mode

# 固定发布标签版本
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | bash -s -- --version v0.1.0

# 从显式制品 URL + 校验和 URL 安装
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | \
  bash -s -- \
    --artifact-url "https://github.com/Dicklesworthstone/pi_agent_rust/releases/download/v0.1.0/pi-linux-amd64.tar.xz" \
    --checksum-url "https://github.com/Dicklesworthstone/pi_agent_rust/releases/download/v0.1.0/SHA256SUMS"

# 跳过补全设置（CI/非交互最小安装）
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | \
  bash -s -- --yes --no-completions
```

安装程序是幂等的，并支持从 TypeScript Pi 的迁移路径：
- 检测现有 TS `pi` 命令
- 提示将 Rust Pi 安装为规范的 `pi`
- 旧 CLI 保留在 `legacy-pi` 后
- 记录状态用于干净卸载/恢复

值得注意的安装程序标志：
- `--offline [TARBALL]`：强制离线模式；可选的本地制品路径（`.tar.gz`、`.tar.xz`、`.zip` 或原始二进制文件）
- `--artifact-url`：强制指定特定发布制品 URL
- `--checksum` / `--checksum-url`：覆盖显式制品的校验和来源
- `--sigstore-bundle-url`：覆盖 `cosign verify-blob` 使用的 Sigstore 包 URL
- `--completions auto|off|bash|zsh|fish`：强制指定 shell 补全安装目标（`off` 等同于 `--no-completions`）
- `--no-completions`：禁用补全安装
- `--no-agent-skills`：跳过自动将 `pi-agent-rust` 技能安装到 `~/.claude/skills/` 和 `~/.codex/skills/`
- `--no-verify`：跳过校验和 + 签名验证（仅测试）
- `--artifact-url` 不带 `--version` 时仅对发布模式使用合成标签；如果制品下载失败，安装退出而不尝试源码回退
- 安装程序对所有网络获取遵循 `HTTPS_PROXY` / `HTTP_PROXY`

默认情况下，安装程序还会为 Claude Code 和 Codex CLI 安装 `pi-agent-rust` 技能：
- Claude Code：`~/.claude/skills/pi-agent-rust/SKILL.md`
- Codex CLI：`~/.codex/skills/pi-agent-rust/SKILL.md`（如果设置了 `CODEX_HOME`，则为 `$CODEX_HOME/skills/pi-agent-rust/SKILL.md`）
- 升级期间，安装程序管理的旧版本遗留预工具条目会自动删除（幂等、路径作用域、非破坏性），前提是存在先前的安装程序状态。

安装程序回归测试（选项 + 校验和 + 签名 + 补全）：

```bash
bash tests/installer_regression.sh
```

### 分发兼容性合同（打包/调用范围）

为便于迁移采用，打包和调用兼容性遵循以下合同：

- 本节仅涵盖打包/调用行为；功能对等和认证状态在 `docs/contracts/dropin-certification-contract.json` 中跟踪。
- 规范的可执行文件名为 `pi`，适用于发布资产和安装程序管理的安装。
- 安装程序管理的安装也会创建一个 `rpi` 兼容启动器（如果 PATH 上没有冲突的 `rpi` 命令）。
- 现有的 TypeScript `pi` 安装可以原地迁移；之前的命令保留为 `legacy-pi`。
- 如果你保留 TypeScript `pi` 作为规范版本（`--keep-existing-pi`），Rust Pi 将安装为 `pi-rust`。
- 在 Apple Silicon 上，安装程序即使在 Rosetta 转换的 shell 中启动也优先选择原生 arm64 制品。
- 版本固定的安装通过 `install.sh --version vX.Y.Z` 支持，用于确定性部署。
- 每个 GitHub 发布都提供平台二进制文件及 `SHA256SUMS` 用于完整性验证。

代表性冒烟检查：

```bash
# 规范命令应存在并执行
command -v pi
pi --version
pi --help >/dev/null

# 如果执行了 TS 迁移，旧命令仍可用
command -v legacy-pi && legacy-pi --version
```

### 从源码构建

需要 Rust nightly（2024 版特性）：

```bash
# 安装 Rust nightly
rustup install nightly
rustup default nightly

# 克隆并构建
git clone https://github.com/Dicklesworthstone/pi_agent_rust.git
cd pi_agent_rust
cargo build --release

# 二进制文件位于 target/release/pi
./target/release/pi --version

# 安装到系统范围（--locked 确保可复现的依赖解析）
cargo install --path . --locked
```

### 依赖

Pi 的最小运行时依赖：
- `fd`：`find` 工具需要（通过 `apt install fd-find` 或 `brew install fd` 安装）
- `rg`：`grep` 工具需要（通过 `apt install ripgrep` 或 `brew install ripgrep` 安装）

### 卸载

```bash
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/uninstall.sh" | bash
```

默认情况下，卸载程序移除安装程序管理的 Rust 二进制文件/别名和技能目录，
然后恢复已迁移的 TypeScript `pi`（如果保留的话）。

---

## 命令

### 基本用法

```bash
pi [OPTIONS] [MESSAGE]...

# 示例
pi                              # 启动交互式会话
pi "Hello"                      # 带消息启动
pi @file.rs "Explain this"      # 包含文件作为上下文
pi -p "Quick question"          # 打印模式（无会话）
```

交互式文件引用：
- 在编辑器中输入 `@relative/path` 来附加文件内容（自动补全会插入 `@` 形式）。

### 选项

| 选项 | 描述 |
|--------|------|
| `-c, --continue` | 继续最近的会话 |
| `-r, --resume` | 打开会话选择器 UI |
| `--session <PATH>` | 打开特定会话文件 |
| `--session-dir <DIR>` | 覆盖本次运行的会话存储目录 |
| `--session-durability strict|balanced|throughput` | 调优持久化耐久性模式 |
| `--no-session` | 不持久化对话 |
| `-p, --print` | 单次响应，无需交互 |
| `--mode text|json|rpc` | 输出/协议模式 |
| `--provider <NAME>` | 强制本次运行使用指定 provider（支持别名）|
| `--model <MODEL>` | 使用的模型（自动选择回退：`anthropic/claude-sonnet-4-6`，然后 `anthropic/claude-opus-4-7`，然后 `openai/gpt-5.1-codex`）|
| `--thinking <LEVEL>` | 思考级别：off/minimal/low/medium/high/xhigh |
| `--tools <TOOLS>` | 逗号分隔的工具列表 |
| `--api-key <KEY>` | API 密钥（或使用 provider 特定的环境变量，如 `ANTHROPIC_API_KEY`、`OPENAI_API_KEY` 等）|
| `--extension-policy safe|balanced|permissive` | 扩展能力配置文件 |
| `--repair-policy off|suggest|auto-safe|auto-strict` | 扩展自动修复策略 |
| `--list-models [PATTERN]` | 列出可用模型（可选模糊过滤）|
| `--list-providers` | 列出规范的 provider ID、别名和认证环境密钥 |
| `--export <PATH>` | 将会话文件导出为 HTML |

额外的高杠杆标志：

- `--no-migrations` 跳过启动迁移检查
- `--explain-extension-policy` 打印有效的策略决策并退出
- `--explain-repair-policy` 打印有效的修复策略解析并退出

### 子命令

```bash
# 包管理
pi install <source> [-l|--local]    # 安装包源并添加到设置
pi remove <source> [-l|--local]     # 从设置中移除包源
pi update [source]                  # 更新所有（或一个）非固定版本包
pi list                             # 从设置中列出用户 + 项目包

# 配置
pi config                           # 显示设置路径 + 优先级
```

更多实用子命令：

```bash
# 扩展目录索引 + 发现
pi update-index
pi search "git"
pi info pi-search-agent

# 环境和扩展诊断
pi doctor
pi doctor --only sessions --format json
pi doctor --only swarm --format json
pi doctor ./path/to/extension --policy safe --fix

# 只读 swarm 进度 SLO 评估（基于标准化证据）
pi swarm-progress --input progress-slo-input.json --format json
pi swarm-progress --input progress-slo-input.json --since HEAD~1 --out-json progress-slo.json

# 会话存储迁移（JSONL -> v2 sidecar 存储）
pi migrate ~/.pi/agent/sessions --dry-run
pi migrate ~/.pi/agent/sessions
```

- `update-index` 刷新扩展索引元数据，用于 `search` 和 `info`。
- `search` 和 `info` 让你无需离开 CLI 即可发现和检查扩展元数据。
- `doctor` 检查配置、目录、认证、shell 设置、会话、swarm 协调就绪度和扩展兼容性。`pi doctor --only swarm --format json` 还会在大型多智能体运行前报告 cgroup CPU 配额、cpuset 大小、NUMA 拓扑、cgroup 内存限制、target/tmp 余量以及推荐的并发预算。
- `swarm-progress` 评估标准化的进度 SLO 快照并输出咨询性 JSON/文本；它不会改变 Beads、git、Agent Mail、RCH、验证 broker 槽位、runpack 或源文件。操作员工作流、隐私边界、降级 Agent Mail/RCH 解读、陈旧 Beads 处理和无开放工作收敛指南位于 [docs/swarm-operations-runbook.md#progress-slo-operator-workflow](docs/swarm-operations-runbook.md#progress-slo-operator-workflow)。
- `migrate` 验证或创建 v2 会话 sidecar 格式，用于更大历史记录的更快恢复。

---

## 配置

Pi 从 `~/.pi/agent/settings.json` 读取配置：

```json
{
  "default_provider": "anthropic",
  "default_model": "claude-opus-4-5",
  "default_thinking_level": "medium",

  "compaction": {
    "enabled": true,
    "reserve_tokens": 8192,
    "keep_recent_tokens": 20000
  },

  "retry": {
    "enabled": true,
    "max_retries": 3,
    "base_delay_ms": 1000,
    "max_delay_ms": 30000
  },

  "images": {
    "auto_resize": true,
    "block_images": false
  },

  "terminal": {
    "show_images": true,
    "clear_on_shrink": false
  },

  "shell_path": "/bin/bash",
  "shell_command_prefix": "set -e"
}
```

### 配置优先级

设置按优先级顺序解析（首次匹配优先）：

1. **CLI 标志**（`--model`、`--thinking`、`--provider` 等）
2. **环境变量**（`ANTHROPIC_API_KEY`、`PI_CONFIG_PATH` 等）
3. **项目设置**（工作目录中的 `.pi/settings.json`）
4. **全局设置**（`~/.pi/agent/settings.json`）
5. **内置默认值**

这意味着 CLI 标志始终覆盖 `settings.json` 值，项目级设置覆盖全局设置。

### 资源解析

技能、提示词模板、主题和扩展遵循相同的解析顺序：

1. CLI 指定的路径（`--skill`、`--prompt-template`、`--theme`、`-e`）
2. 项目目录（`.pi/skills/`、`.pi/prompts/`、`.pi/themes/`、`.pi/extensions/`）
3. 全局目录（`~/.pi/agent/skills/`、`~/.pi/agent/prompts/` 等）
4. 已安装的包（`~/.pi/agent/packages/`）

当多个资源共享相同名称时，首次出现优先。冲突会记录为诊断信息。

**提示词模板扩展**支持位置参数：`$1`、`$2`、`$@`（所有参数）和切片语法 `${@:start}`、`${@:start:length}`。例如，以 `/review src/main.rs --strict` 调用的模板会将 `src/main.rs` 作为 `$1`，`--strict` 作为 `$2`。

### 环境变量

| 变量 | 描述 |
|----------|------|
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 |
| `OPENAI_API_KEY` | OpenAI API 密钥 |
| `GOOGLE_API_KEY` | Google Gemini API 密钥 |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI API 密钥 |
| `COHERE_API_KEY` | Cohere API 密钥 |
| `GROQ_API_KEY` | Groq API 密钥（OpenAI 兼容）|
| `DEEPINFRA_API_KEY` | DeepInfra API 密钥（OpenAI 兼容）|
| `CEREBRAS_API_KEY` | Cerebras API 密钥（OpenAI 兼容）|
| `OPENROUTER_API_KEY` | OpenRouter API 密钥（OpenAI 兼容）|
| `MISTRAL_API_KEY` | Mistral API 密钥（OpenAI 兼容）|
| `MOONSHOT_API_KEY` | Moonshot/Kimi API 密钥（OpenAI 兼容）|
| `DASHSCOPE_API_KEY` | DashScope/Qwen API 密钥（OpenAI 兼容）|
| `DEEPSEEK_API_KEY` | DeepSeek API 密钥（OpenAI 兼容）|
| `FIREWORKS_API_KEY` | Fireworks API 密钥（OpenAI 兼容）|
| `TOGETHER_API_KEY` | Together API 密钥（OpenAI 兼容）|
| `PERPLEXITY_API_KEY` | Perplexity API 密钥（OpenAI 兼容）|
| `XAI_API_KEY` | xAI API 密钥（OpenAI 兼容）|
| `PI_CONFIG_PATH` | 自定义配置文件路径 |
| `PI_CODING_AGENT_DIR` | 覆盖全局配置目录 |
| `PI_PACKAGE_DIR` | 覆盖包目录 |
| `PI_SESSIONS_DIR` | 自定义会话目录 |

---

## 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                           CLI (clap)                            │
│  • 参数解析    • @文件扩展    • 子命令                           │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────────┐
│                          智能体循环                              │
│  • 消息历史     • 工具迭代    • 事件回调                        │
└────────┬──────────────────────┬──────────────────────┬──────────┘
         │                      │                      │
┌────────▼────────┐  ┌─────────▼──────────┐  ┌───────▼──────────┐
│  Provider 层    │  │  工具注册表         │  │  扩展管理器      │
│ • Anthropic     │  │  • read  • bash    │  │  • QuickJS JS/TS │
│ • OpenAI (Chat/ │  │  • write • grep    │  │  • 原生描述符    │
│   Responses)    │  │  • edit  • find    │  │    运行时         │
│ • Gemini/Cohere │  │  • ls              │  │  • 能力策略       │
│ • Azure/Bedrock │  │  • ext-registered  │  │  • Node shims     │
│ • Vertex/Copilot│  │                    │  │  • 事件钩子       │
│ • GitLab/Ext    │  │                    │  │  • 运行时风险控制 │
└────────┬────────┘  └─────────┬──────────┘  └───────┬──────────┘
         │                     │                      │
┌────────▼─────────────────────▼──────────────────────▼──────────┐
│                        会话持久化                                │
│  • JSONL 格式 (v3)   • 树结构   • 会话索引/缓存                  │
│  • 每项目目录        • 默认启用的 SQLite 后端支持               │
└─────────────────────────────────────────────────────────────────┘
```

Provider 计数规则：Pi 有 10 个原生 provider 实现模块，计为 `src/providers/` 下排除 `mod.rs` 的 Rust 文件。这些模块是 `anthropic`、`openai`、`openai_responses`、`gemini`、`cohere`、`azure`、`bedrock`、`vertex`、`copilot` 和 `gitlab`。用户可见的 provider ID、别名、OpenAI 兼容预设和扩展提供的 `streamSimple` provider 分别计数，因为几个原生模块暴露了多条路由。

### 关键设计决策

1. **无 unsafe 代码**：`#![forbid(unsafe_code)]` 在项目范围内强制执行
2. **流式优先**：自定义 SSE 解析器，无阻塞等待响应
3. **进程树管理**：`sysinfo` crate 确保无悬空进程
4. **结构化错误**：`thiserror`，每个组件有特定的错误类型
5. **大小预算发布配置**：LTO + strip + `opt-level = "z"`，用于预算合规的发布制品

### asupersync 上下文 vs TypeScript Pi (pi-mono)

这个 Rust 移植保留了 Pi 的用户体验，但有意改变了运行时基础。原始的 TypeScript Pi（`pi-mono`、`packages/coding-agent`）构建在 Node.js + 包级抽象之上。`pi_agent_rust` 将这些相同的行为迁移到 `asupersync` 原语上，使生命周期保证在运行时模型中显式化。

| 关注点 | TypeScript Pi (pi-mono 基线) | pi_agent_rust + asupersync |
|---------|-------------------------------|-----------------------------|
| **运行时模型** | Node 事件循环 + Promise/AbortSignal 约定 | `RuntimeBuilder` + 显式反应器和运行时句柄 |
| **异步所有权** | 任务生命周期由框架/库代码协调 | 结构化任务所有权和显式跨线程通道（TUI/RPC 桥接）|
| **取消语义** | 主要是 API 和工具层约定 | 运行时感知的取消检查 + 工具中的有界超时处理 |
| **I/O 能力形状** | 环境 Node API + 扩展层策略 | 能力作用域上下文（基于 `asupersync::Cx` 的 `AgentCx`）和显式主机调用策略 |
| **HTTP 流式传输** | 依赖 provider/客户端 | 自定义构建的 asupersync HTTP/TLS 客户端，提供数据给自定义 SSE 解析器 |
| **确定性测试钩子** | 传统异步测试设置 | asupersync 测试/运行时钩子广泛用于单元/集成测试 |

为什么这在实际中有用：
- **更可预测的故障行为**：在中止/超时时，因为取消在显式循环边界和工具运行器中检查。
- **更清晰的资源生命周期**：因为运行时、定时器和 I/O 路径共享一个并发基础。
- **更少的隐藏耦合**：因为主要不变量位于 Rust 类型/算法中，而不是分布在框架约定中。

### 运行时不变量（及其重要性）

以下是我们在本实现中依赖的具体不变量：

1. **轮次作用域的智能体生命周期**
   - 主循环以稳定顺序发出 `AgentStart`、`TurnStart`、`TurnEnd` 和 `AgentEnd`。
   - 工具递归由 `max_tool_iterations`（默认 `50`）限制，以避免无限自工具循环。
   - 收益：为 TUI/RPC 消费者提供稳定的事件排序，以及可预测的终止行为。

2. **中止和超时行为显式化**
   - 智能体中止检查在轮次边界和工具执行前后进行。
   - `bash` 超时遵循清晰的升级路径：终止进程树、宽限期、然后硬终止。
   - 收益：更少的"挂起"会话，减少激进工具使用期间的悬空进程风险。

3. **会话写入崩溃弹性**
   - JSONL 保存写入临时文件并原子化持久化。
   - 会话索引使用 SQLite WAL + 锁文件协调来处理并发实例。
   - 收益：在多进程使用下更好的耐久性和恢复可靠性。

4. **压缩基于阈值且边界感知**
   - 触发条件：估计上下文 token 超过 `context_window - reserve_tokens`。
   - 切割点逻辑优先选择用户轮次边界，并保留近期上下文预算。
   - 收益：压缩在不破坏近期任务连续性的前提下恢复上下文。

5. **能力策略失败关闭且优先级明确**
   - 解析顺序：逐扩展拒绝 -> 全局拒绝 -> 逐扩展允许 -> 默认能力 -> 模式回退。
   - 收益：策略结果可解释、确定且可审计。

6. **流式解析器容忍真实网络分块**
   - SSE 解析器处理 CR/LF 变体、多行 `data:` 字段、部分 UTF-8 尾部和流结束刷新。
   - 收益：增量渲染跨 provider 和网络分片保持稳健。

### 从 asupersync 继承到 Pi 的设计原则

以下 `asupersync` 原则直接反映在 `pi_agent_rust` 架构中：

- **单一异步基础**：运行时、定时器、文件系统和 HTTP/TLS 全部在一个连贯基础上运行。
- **显式上下文线程化**：`AgentCx` 在子系统边界（智能体/工具/会话/RPC）包装 `asupersync::Cx`。
- **有界操作优于尽力而为清理**：超时路径和压缩阈值是可参数化且可执行的。
- **测试的确定性钩子**：定时器驱动感知的休眠和 asupersync 测试辅助减少非确定性片状。

与原始的 TypeScript 实现相比，这将更多的正确性责任转移到了运行时和核心算法本身，而不是主要依赖生态系统约定。

### 额外的重大差异（原始 pi-mono vs Rust 移植）

这是第二次对比传递，聚焦于高影响的架构差异及其理由。

| 领域 | 原始 pi-mono (`packages/coding-agent`) | `pi_agent_rust` | 为什么存在这种差异 |
|------|----------------------------------------|-----------------|---------------------|
| **分发模型** | npm 包（`npm install -g @mariozechner/pi-coding-agent`） | 单一 Rust 二进制文件（`pi`） | 移除 Node 运行时依赖，改善启动/部署可移植性 |
| **执行表面** | 交互 + 打印 + JSON 模式 + RPC + SDK | 交互 + 打印 + JSON 模式 + RPC + Rust SDK | Rust SDK 提供惯用的配套 API，用于编程化嵌入 Pi（文档见 `docs/sdk.md`）|
| **默认内置工具姿态** | 默认为 `read/write/edit/bash`（其他可用）| 八个内置工具视为一等公民（`read/write/edit/bash/grep/find/ls/hashline_edit`）| 保持通用代码导航、shell 和 hashline 锚定编辑工作流无需额外配置即可使用 |
| **扩展信任模型** | 扩展/包模型记录为完全系统访问 | 基于能力的门控主机调用 + 策略配置文件的嵌入式运行时 | 减少环境权限，使扩展行为可审计/默认拒绝 |
| **会话架构重点** | JSONL 树会话模型和分支导航 | JSONL v3 树 + 显式会话索引（SQLite sidecar）+ 默认启用的 SQLite 会话后端支持 | 大规模下更快的恢复/查找和更安全的多实例协调 |
| **流式传输堆栈** | Node 运行时网络堆栈 | 自定义构建的 HTTP/TLS 客户端 + asupersync 上的自定义 SSE 解析器 | 对长流中的分块、解析和故障处理有更强的控制 |
| **取消/超时机制** | 平台/事件循环取消约定 | 显式中止信号、有界工具迭代、进程树终止 | 最小化挂起/孤儿，使停止行为在负载下可预测 |
| **运行时上下文模型** | 框架级约定和扩展 API | 显式 `AgentCx`/`asupersync::Cx` 能力作用域上下文线程化 | 使效果边界和可测试性成为一等架构约束 |

这些差异的实际影响：
- 扩展/包工作流在两种实现间兼容。
- 目标是功能等价于 pi-mono，同时采用 Rust 习惯模式和性能改进。
- Rust SDK 提供了等效能力，无需特定于 TypeScript 的适配模式。
- `docs/parity-certification.json` 跟踪功能对等进展和认证状态。

### 算法机制：pi-mono 基线 vs Rust 实现

本节比较等效高级行为的具体实现机制。

| 算法 | pi-mono 基线机制 | Rust 实现机制 | 为什么存在 Rust 变体 |
|-----------|--------------------------|--------------------------|--------------------------|
| **压缩后会话上下文重建** | `buildSessionContext()` 输出压缩摘要，然后是 `firstKeptEntryId`（压缩前路径）的消息，然后是压缩后条目 | `to_messages_for_current_path()` 使用相同顺序，并在 `first_kept_entry_id` 缺失时添加回退 | 避免压缩锚点孤立/损坏时出现静默上下文丢失 |
| **JSONL 持久化** | 增量追加（`appendFileSync`）加完整重写（`writeFileSync`）用于迁移/重写 | 通过临时文件 + 原子持久化/替换保存 | 使磁盘上会话状态在保存操作期间具有崩溃弹性 |
| **会话发现/恢复** | 目录/文件扫描和 JSONL 文件的 mtime 排序 | SQLite 会话索引 sidecar + WAL + 锁文件 + 陈旧触发完整重建索引 | 限制恢复查找成本并协调并发进程 |
| **压缩 token 核算** | 使用助手使用量（`totalTokens` 否则 `input+output+cacheRead+cacheWrite`）加启发式尾部估计 | 使用助手使用量（`total_tokens` 否则 `input+output`）加启发式尾部估计；固定图像 token 估计 | 在缓存 token 报告不均的 provider 间保持稳定核算，同时保持保守 |
| **切割点 + 拆分轮次处理** | 有效切割点排除工具结果；拆分轮次总结为历史 + 轮次前缀上下文 | Rust 条目/消息模型中相同的切割点类和拆分轮次策略 | 保留工具调用/结果邻接性和轮次连贯性（在预算压力下）|
| **Bash 超时/进程清理** | 超时/中止杀死进程树（`killProcessTree`）并返回尾部截断输出 | 超时升级（`TERM` 然后宽限然后 `KILL`）+ 进程树遍历 + shell 退出陷阱 + 尾部截断 | 强制执行有界清理并减少后台作业的后代进程泄漏 |
| **流式事件解码** | 传输语义暴露（`sse`/`websocket`/`auto`）；解析器细节运行时内部 | 显式 SSE 解析器，带 BOM 剥离、CR/LF 标准化、UTF-8 尾部缓冲和结束刷新 | 使字节到事件行为确定且独立于 provider SDK |

### 特性超集亮点（超越 pi-mono 基线）

以上部分比较机制。本节指出 Rust 移植中存在的具体特性，这些不是 pi-mono 基线实现模型的一部分。

| Rust 移植特性 | 为什么有用/引人注目 |
|-------------------|----------------------|
| **`pi doctor` 诊断命令**（`text`/`json`/`markdown`、`--only`、`--fix`、swarm 预检、扩展兼容性检查）| 提供可操作的环境 + 兼容性诊断，支持 CI 门控（失败时非零返回），可自动修复安全的问题（如缺失目录/权限），并在 swarm 工作前报告只读的多智能体就绪度 |
| **基于能力门控的扩展策略配置文件**（`safe`/`balanced`/`permissive`），支持逐扩展覆盖 | 让操作员以显式能力边界运行共享扩展，而非环境级完全系统访问 |
| **秘密感知的扩展环境变量过滤**（`pi.env()` 对密钥/token/机密的阻止列表）| 减少来自扩展代码路径的意外凭据暴露 |
| **逐扩展信任生命周期 + kill-switch 审计追踪**（`pending`/`acknowledged`/`trusted`/`killed`、`kill_switch`、`lift_kill_switch`）| 支持即时隔离、显式操作员溯源以及审查后的受控重新进入 |
| **主机调用兼容通道紧急控制**（全局/逐扩展强制兼容开关 + 原因代码）| 为快速通道事件提供确定性回滚路径，同时不损失扩展可用性 |
| **扩展主机调用的运行时风险控制器**（可配置，默认失败关闭）| 为可疑运行时行为添加超越静态策略的额外执行层 |
| **shell 路径的参数感知运行时风险评分**（`dcg_rule_hit`、`dcg_heredoc_hit`、跨 Bash/Python/JS/TS/Ruby 的 heredoc AST 检查）| 在主机调用执行前检测隐藏在多行脚本和包装命令中的破坏性意图 |
| **防篡改运行时风险账本工具**（`ext_runtime_risk_ledger verify|replay|calibrate`）| 安全决策经过哈希链关联，可从真实轨迹验证、重放和阈值校准 |
| **统一事件证据包导出**（风险账本、安全告警、主机调用遥测、exec 调解、秘密代理事件）| 事件响应可以从一个结构化制品集进行分类，而不是拼凑临时日志 |
| **确定性主机调用反应器网格（可选 NUMA slab 池）**（分片亲和性、全局顺序 drain、有界 SPSC 通道、遥测）| 保持扩展分派在负载下可预测，并暴露队列/背压行为以进行调优 |
| **暖隔离区池 + 启动预暖交接** | 将 JS 运行时准备移出首次交互轮次，并在运行间安全重用预热状态 |
| **扩展预检静态分析**（导入/禁止模式扫描，带策略感知提示）| 在运行时执行前捕获危险扩展模式 |
| **无需 Node/Bun 依赖的 Node/Bun 兼容扩展运行时**（嵌入式 QuickJS + shims）| 在单一原生二进制部署模型中运行旧版扩展工作流 |
| **扩展兼容性扫描器 + 合规测试框架** | 使扩展支持可量化、可审计，而非仅凭传闻 |
| **SQLite 会话索引 sidecar**（WAL + 锁 + 陈旧重建索引路径）| 提供大规模下的快速会话恢复/列表操作，无需每次查询扫描每个 JSONL 文件 |
| **会话存储 V2 回滚和迁移账本**（分段日志 + 检查点 + 回滚事件）| 长会话恢复可回滚到已知检查点，带有显式迁移/回滚溯源 |
| **默认启用的 SQLite 会话存储支持**（`sqlite-sessions` 特性）| 支持需要数据库支持的会话持久化的部署；构建最小二进制文件时使用 `--no-default-features` 禁用 |
| **崩溃弹性会话保存路径**（临时文件 + 原子持久化）| 改善会话文件在写入期间的耐久性，减少部分写入故障模式 |
| **统一主机调用调度器，带类型化分类映射**（`timeout`/`denied`/`io`/`invalid_request`/`internal`）| 产生一致的扩展/运行时错误语义，更方便客户端处理 |
| **失败关闭的世系门禁**（`run_id`/`correlation_id` + 跨制品世系检查）| 在发布门禁时间拒绝过时或选择性挑选的合规/性能制品 |
| **结构化认证诊断，带稳定机器代码** | 改善故障排除和运维可见性，同时不泄露敏感凭据材料 |

---

## 深入探索：核心算法

### 数学驱动的决策系统

Pi 有意在能改善运行时行为或基准置信度的地方使用高级数学。目标不是"文档中的花哨公式"；而是更安全的策略决策、更快的工作负载变化恢复以及更可信的性能归因。

### 状态转移检测（CUSUM + BOCPD）

在扩展调度器中，Pi 结合 CUSUM 和贝叶斯在线变化点检测来早期检测负载状态变化（例如当主机调用流量突然激增或停滞时）。

$$
S_t^+ = \max\left(0,\;S_{t-1}^+ + (-z_t - k)\right), \quad
S_t^- = \max\left(0,\;S_{t-1}^- + (z_t - k)\right)
$$

$$
H(r)=\frac{1}{\lambda}, \quad
P(r_t=0 \mid x_{1:t}) \propto \sum_r P(r_{t-1}=r)\,H(r)\,P(x_t \mid r)
$$

直觉：CUSUM 捕获持续漂移；BOCPD 无需脆弱的固定阈值即可捕获突然的状态变化。

### 共形预测包络

Pi 跟踪非一致性分数（与运行均值的绝对残差），并将区间外事件视为异常。

$$
q = \text{score}_{\lceil (n+1)\cdot \text{confidence} \rceil - 1}, \quad
\text{anomaly if } |x_t - \mu_t| > q
$$

直觉：阈值根据近期行为自适应调整，而非硬编码一个静态延迟截止值。

### PAC-Bayes 安全界限

Pi 的安全包络包括一个针对扩展结果的 PAC-Bayes-kl 界限，并可在界限过高时否决激进的优化。

$$
\mathrm{kl}(\hat q \,\|\, q_{\text{bound}})\;\le\;\frac{\mathrm{KL}(Q\|P)+\ln\!\left(2\sqrt{n}/\delta\right)}{n}
$$

直觉：这为在允许更激进的运行时行为之前，提供了一个显式的不确定性感知的真实错误风险上限。

### 离线策略评估（IPS/WIS/DR + ESS + 后悔门禁）

在批准策略变更之前，Pi 从跟踪数据中评估候选行为：

$$
w_i=\frac{\pi(a_i\mid x_i)}{\mu(a_i\mid x_i)}, \quad
\hat V_{\text{IPS}}=\frac{1}{n}\sum_i w_i r_i
$$

$$
\hat V_{\text{WIS}}=\frac{\sum_i w_i r_i}{\sum_i w_i}, \quad
\hat V_{\text{DR}}=\frac{1}{n}\sum_i\left(\hat r_i + w_i(r_i-\hat r_i)\right)
$$

$$
N_{\text{eff}}=\frac{(\sum_i w_i)^2}{\sum_i w_i^2}, \quad
\Delta_{\text{regret}}=\bar r_{\text{baseline}}-\hat V_{\text{DR}}
$$

直觉：如果样本支持弱、不确定性高或估计后悔超过阈值，Pi 失败关闭。

### VOI 驱动的实验选择

VOI 规划器在严格的开销预算下，优先选择提供最多预期学习收益的探测。

$$
\text{priority}_i \propto \frac{\text{utility}_i}{\text{overhead}_i}
$$

直觉：只运行可能改变决策的实验；跳过过时或低价值的探测。

### 加权瓶颈归因（基准测试）

对于阶段 1 矩阵基准测试，Pi 按真实工作负载大小（`session_messages`）加权的阶段归因，并报告置信区间。

$$
\text{weighted\_contribution}_s
=
\frac{\sum_i w_i\,m_{i,s}}{\sum_i w_i\,t_i}\cdot 100,
\quad w_i=\text{session\_messages}_i
$$

$$
n_{\text{eff}}=\frac{(\sum_i w_i)^2}{\sum_i w_i^2}, \quad
\mathrm{CI}_{95}=\mu \pm 1.96\sqrt{\frac{\sigma^2}{n_{\text{eff}}}}
$$

直觉：优先优化主导真实端到端延迟的部分，而非孤立的微基准热点。

### 在线凸控制 + 后悔跟踪

Pi 还包含一个在线调优器路径，用于批处理/时间片控制，具有显式回滚行为：

$$
\tau_{t+1}
=
\mathrm{clip}\!\left(\tau_t - \eta\nabla_{\tau}\mathcal{L}_t,\;\tau_{\min},\tau_{\max}\right)
$$

直觉：系统持续自适应，但如果瞬时损失超过回滚阈值，它会立即返回更安全的配置。

### 数学一览

| 技术 | 在 Pi 中的位置 | 为什么有帮助 |
|----------|--------------|--------------|
| CUSUM + BOCPD | 扩展调度器状态检测器 | 早期且稳健地检测流量状态变化 |
| 共形区间 | 安全包络 | 无需静态魔术数字的自适应异常门控 |
| PAC-Bayes 界限 | 安全包络否决路径 | 当不确定性/风险过高时失败关闭 |
| IPS/WIS/DR + ESS | 离线策略评估器 | 仅在有足够支持时才批准策略变更 |
| VOI 规划 | 实验调度器 | 在最高价值的探测上使用开销预算 |
| 加权归因 + CI | 阶段 1 性能矩阵报告 | 根据真实用户影响对优化工作排序 |
| OCO + 后悔回滚 | 运行时控制器 | 在负载下自适应，同时限制不安全漂移 |

### SSE 流式解析器

SSE（服务器发送事件）解析器是一个自定义实现，处理 Anthropic 的流式响应格式。与基于库的方法不同，解析器作为一个状态机运行，增量处理字节：

```
Bytes → 行累加器 → 事件解析器 → 类型化 StreamEvent
```

**关键特性：**

| 属性 | 实现 |
|----------|--------|
| **缓冲** | 尽可能零拷贝；仅在不完整时累加行 |
| **事件类型** | 12 种不同变体：MessageStart、ContentBlockStart、ContentBlockDelta、ContentBlockStop、MessageDelta、MessageStop、Ping、Error 和思考特定事件 |
| **错误恢复** | 格式错误的事件记录但不崩溃流 |
| **内存** | 固定大小滚动缓冲区防止无界增长 |

解析器处理以下边缘情况：
- 多行 `data:` 字段（用换行符连接）
- 跨 TCP 包边界分割的事件
- `event:` 字段出现在 `data:` 之前或之后
- CRLF 和 LF 行结尾可互换

### 截断算法

工具的大输出（文件读取、命令输出、grep 结果）必须截断以避免耗尽 LLM 的上下文窗口。截断算法在保持有用性的同时保持在限制内：

```
┌─────────────────────────────────────────┐
│              原始内容                  │
│         （可能非常大）                  │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│  HEAD: 前 N/2 行                       │
│  ─────────────────────────              │
│  [... 省略 X 行 ...]                    │
│  ─────────────────────────              │
│  TAIL: 后 N/2 行                       │
└─────────────────────────────────────────┘
```

**常量：**

| 限制 | 值 | 理由 |
|-------|------|--------|
| `MAX_LINES` | 2000 | 平衡上下文使用与完整性 |
| `MAX_BYTES` | 1MB | 防止二进制文件事故 |
| `GREP_MAX_LINE_LENGTH` | 500 字符 | 截断压缩代码 |

算法：
1. 将内容分割为行
2. 如果行数超过 `MAX_LINES`，取前 1000 和后 1000 行
3. 插入标记显示省略了多少行
4. 如果字节数仍超过 `MAX_BYTES`，应用字节级截断
5. 返回指示发生截断的元数据，使 LLM 可请求特定范围

### 进程树管理

`bash` 工具必须处理失控进程、无限循环和 fork 炸弹，而不留下孤儿进程。实现使用 `sysinfo` crate 遍历进程树：

```rust
// 进程清理的伪代码
fn kill_process_tree(root_pid: Pid) {
    let system = System::new();
    let children = find_all_descendants(root_pid, &system);

    // 先杀子进程（最深优先），再杀父进程
    for child in children.iter().rev() {
        kill(child, SIGKILL);
    }
    kill(root_pid, SIGKILL);
}
```

**超时行为：**

1. 命令以可配置超时启动（默认 120s）
2. 输出实时流式传输到滚动缓冲区
3. 超时时：发送 SIGTERM，5s 宽限期，然后 SIGKILL
4. 遍历进程树并杀死所有后代
5. 退出码指示超时 vs 正常终止

为避免悬空后台作业（例如 `cmd &`），bash 脚本安装一个 `EXIT` 陷阱，等待任何剩余子进程，然后以原始命令的状态退出。

这防止了常见的故障模式——杀死 shell 而让其子进程继续运行。

### 会话树结构

会话使用树结构而非平面列表，支持对话分支（在探索不同方法时很有用）：

```
                    ┌─────────┐
                    │ 消息 #1  │ (根)
                    └────┬────┘
                         │
                    ┌────▼────┐
                    │ 消息 #2  │
                    └────┬────┘
                         │
              ┌──────────┼──────────┐
              │                     │
         ┌────▼────┐          ┌────▼────┐
         │ 消息 #3  │          │ 消息 #3b│ (分支)
         └────┬────┘          └────┬────┘
              │                    │
         ┌────▼────┐          ┌────▼────┐
         │ 消息 #4  │          │ 消息 #4b│
         └─────────┘          └─────────┘
```

**JSONL 格式 (v3)：**

每行是一个自包含的 JSON 对象，带有 `type` 鉴别器：

```json
{"type":"session","version":3,"cwd":"/project","created":"2024-01-15T10:30:00Z"}
{"type":"message","id":"a1b2c3d4","parent":"root","role":"user","content":[...]}
{"type":"message","id":"e5f6g7h8","parent":"a1b2c3d4","role":"assistant","content":[...]}
{"type":"model_change","id":"i9j0k1l2","parent":"e5f6g7h8","model":"claude-sonnet-4-20250514"}
```

`parent` 字段创建树结构。回放会话从根遍历树到当前叶子。分支创建一条具有与上次延续不同的 `parent` 的新消息。

### Provider 抽象

`Provider` trait 抽象了不同的 LLM 后端：

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn models(&self) -> &[Model];

    async fn stream(
        &self,
        context: &Context,
        options: &StreamOptions,
    ) -> Result<impl Stream<Item = Result<StreamEvent>>>;
}
```

**上下文字结构：**

```rust
pub struct Context {
    pub system: Option<String>,      // 系统提示词
    pub messages: Vec<Message>,       // 对话历史
    pub tools: Vec<ToolDef>,          // 可用工具及其 JSON 模式
}
```

**StreamOptions：**

```rust
pub struct StreamOptions {
    pub model: String,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub thinking: Option<ThinkingConfig>,  // 扩展思考设置
    pub stop_sequences: Vec<String>,
}
```

这种设计允许添加新的 provider（OpenAI、Gemini）而无需修改智能体循环。每个 provider 将通用类型转换为其线格式，并发出统一的 `StreamEvent` 流。

### 压缩算法

长对话最终会超过模型的上下文窗口。Pi 的压缩算法通过总结较旧的消息同时保留近期上下文来回收空间。

该算法在每个智能体轮次后自动运行，当估计 token 使用量超过 `context_window - reserve_tokens` 时：

```
┌──────────────────────────────────────────────────────────────┐
│                        完整对话                                │
│  msg1 → msg2 → msg3 → ... → msgN-5 → msgN-4 → ... → msgN   │
│  ├──── 较旧的消息 ─────┤ ├─── 近期消息 ─────────────────┤    │
│                                                              │
│  步骤 1: 在有效的轮次边界找到切割点                          │
│  步骤 2: LLM 将 msgs 1..N-5 总结为紧凑段落                  │
│  步骤 3: 在会话 JSONL 中存储 Compaction 条目                 │
│  步骤 4: 下次智能体调用使用 [summary] + msgs N-4..N         │
└──────────────────────────────────────────────────────────────┘
```

**Token 估计**对文本使用保守的 `chars ÷ 4` 启发式，每张图像使用固定的 1,200 tokens。当助手消息包含来自 API 的 `usage` 字段时，该测量值优先于启发式。

**切割点选择**优先选择完整用户-助手轮次之间的边界。如果预算迫使在轮次中间切割，算法包含来自拆分轮次的前缀消息，以使模型保留在边界处讨论内容的上下文。

**文件操作跟踪**从被总结的消息中提取 `read`、`write` 和 `edit` 工具调用。压缩提示包含这些路径，使摘要保留检查或修改了哪些文件的意识：

```
<read-files>
src/main.rs
src/config.rs
</read-files>

<modified-files>
src/auth.rs
</modified-files>
```

**可配置参数：**

| 参数 | 默认值 | 用途 |
|-----------|---------|--------|
| `reserve_tokens` | 上下文窗口的 8% | 响应生成的安全裕度 |
| `keep_recent_tokens` | 上下文窗口的 10% | 保留的最小近期上下文 |

压缩也可以在交互模式下通过 `/compact` 或 RPC 命令 `compact` 手动触发。

### 多 Provider 路由与模型注册表

Pi 通过一个 provider 工厂路由模型请求，该工厂从 `(provider, model, api)` 元组解析出正确的后端实现。

**解析流程：**

```
用户指定 --provider openai --model gpt-4o
               │
               ▼
  ┌──────────────────────────┐
  │  Provider 元数据表       │  将 "openai" 映射到规范 ID，
  │                          │  确定 API 类型 (Completions
  │                          │  vs Responses vs custom)
  └────────────┬─────────────┘
               │
  ┌────────────▼─────────────┐
  │  URL 规范化              │  根据检测到的 API 类型追加
  │                          │  /chat/completions、
  │                          │  /responses 或 /chat
  └────────────┬─────────────┘
               │
  ┌────────────▼─────────────┐
  │  兼容配置                │  应用逐模型覆盖：
  │                          │  system_role_name、max_tokens
  │                          │  字段名、特性标志
  └────────────┬─────────────┘
               │
  ┌────────────▼─────────────┐
  │  Provider 实例           │  Anthropic | OpenAI | Gemini
  │                          │  Cohere | Azure | Bedrock | ...
  └──────────────────────────┘
```

**`models.json` 覆盖**：用户可以在 `~/.pi/agent/models.json` 或 `.pi/models.json` 中定义自定义 provider。每个条目指定一个模型 ID、基础 URL、API 类型和可选的兼容标志，让你路由到自托管模型、代理或 Pi 本身不原生支持的 provider。

**兼容配置**处理 OpenAI 兼容 API 之间的差异：

| 覆盖项 | 示例 | 用途 |
|----------|---------|-------|
| `system_role_name` | `"developer"` | o1 模型使用 "developer" 而非 "system" |
| `max_tokens_field` | `"max_completion_tokens"` | 某些模型需要不同的字段名 |
| `supports_tools` | `false` | 对拒绝工具的模型抑制工具定义 |
| `supports_streaming` | `false` | 对不兼容端点回退到非流式 |
| `custom_headers` | `{"X-Custom": "val"}` | 逐 provider 头部注入 |

**模糊匹配**：当 provider 名称不匹配任何已知 provider 时，Pi 计算与所有注册名称的编辑距离，并在错误消息中建议最接近的匹配。

### 扩展主机调用协议

扩展在嵌入式 QuickJS 运行时（`rquickjs` crate）中运行，并通过结构化主机调用协议与 Pi 通信。这是让 JavaScript 代码调用 Pi 内置工具、发出 HTTP 请求和与会话交互的机制，所有这些都不需要直接 OS 访问。

**执行模型：**

```
┌─────────────────── QuickJS VM ───────────────────┐
│                                                   │
│  extension.js 调用：                              │
│    pi.tool("read", {path: "src/main.rs"})         │
│      │                                            │
│      ▼                                            │
│    入队 HostcallRequest {                         │
│      call_id: "hc-0042",                          │
│      kind: Tool { name: "read" },                 │
│      payload: { path: "src/main.rs" },            │
│    }                                              │
│      │                                            │
│      ▼                                            │
│    返回 Promise (resolve/reject 存储在 map 中)    │
│                                                   │
└────────────────────────┬──────────────────────────┘
                         │
    drain_hostcall_requests()
                         │
                         ▼
┌─────────────── ExtensionDispatcher ──────────────┐
│                                                   │
│  1. 检查能力策略：                                │
│     read 工具 → 需要 "read" 能力                  │
│     → 策略指示：允许 / 拒绝 / 提示                │
│                                                   │
│  2. 如果允许 → 分派到 ToolRegistry                │
│     → 执行 read 工具                              │
│     → 获取 ToolOutput                             │
│                                                   │
│  3. complete_hostcall("hc-0042", Ok(result))      │
│     → 解析 QuickJS 中的 Promise                   │
│                                                   │
│  4. runtime.tick()                                │
│     → 处理 Promise .then() 链                     │
│     → Extension JS 继续执行                       │
│                                                   │
└───────────────────────────────────────────────────┘
```

**能力映射**：每个主机调用类型映射到一个必需的能力：

| 主机调用 | 必需能力 | 危险？ |
|----------|---------|--------|
| `pi.tool("read", ...)` | `read` | 否 |
| `pi.tool("write", ...)` | `write` | 否 |
| `pi.tool("bash", ...)` | `exec` | 是 |
| `pi.http(request)` | `http` | 否 |
| `pi.exec(cmd, args)` | `exec` | 是 |
| `pi.env(key)` | `env` | 是 |
| `pi.session(op, ...)` | `session` | 否 |
| `pi.ui(op, ...)` | `ui` | 否 |
| `pi.log(entry)` | `log` | 否（始终允许）|

**去重**：每个主机调用的参数被规范化（对象键排序、结构标准化）并进行 SHA-256 哈希。短窗口内的相同请求可去重以避免冗余工具执行。

**快速通道 vs 兼容通道**：Pi 为主机调用提供两条执行通道：

- **快速通道**：当调用形状匹配已知安全模式时使用（例如常见的 `tool` 和 `session` 操作）。这避免了额外的分配和解析工作。
- **兼容通道**：对于不常见或部分指定的调用，作为回退。
- 两条通道仍执行相同的能力策略和权限检查。
- 操作员可以强制全局或逐扩展使用兼容通道路由作为紧急控制路径。

为便于可观测性，每个调用都带有一个稳定的通道键（例如 `tool|tool.read|filesystem` 或 `tool|fallback|filesystem`），使延迟和故障趋势可以一致分组。

**内置一致性守卫（影子双执行）**：Pi 可以采样一小部分只读主机调用，通过两条通道执行它们，并比较规范化的输出指纹。如果分歧超过配置的预算，Pi 自动在一段时间内回退快速通道。这在不静默改变行为的前提下带来性能收益。

**负载下的自适应调度模式**：Pi 可以在以下模式间切换：

- `sequential_fast_path`：针对更简单/低争用工作负载
- `interleaved_batching`：当争用和队列压力上升时

模式变更受样本覆盖率和风险检查门控，因此 Pi 不会基于薄弱或选择性挑选的证据切换。

**用于调试和调优的运行时遥测**：Pi 记录结构化主机调用遥测（`pi.ext.hostcall_telemetry.v1`），包含通道选择、回退原因、调度延迟占比、编组路径和优化命中/未命中等字段。这些用于性能报告和可靠性诊断。

**自动修复管道**：当扩展加载失败或产生运行时错误时，Pi 的修复系统可以自动修复常见问题：

| 修复模式 | 行为 |
|-------------|--------|
| `Off` | 不修复 |
| `Suggest` | 记录建议，不应用 |
| `AutoSafe`（默认）| 应用可证明安全的修复（缺失文件路径、资产引用）|
| `AutoStrict` | 应用激进启发式修复（基于模式的转换）|

**兼容性扫描器**：加载前，Pi 静态分析扩展源代码中的导入、`require()` 调用和禁止模式（`eval`、`Function()`、`process.binding`、`dlopen`）。扫描产生能力证据账本，为策略决策提供信息。

**环境变量过滤**：扩展调用 `pi.env()` 遇到一个阻止列表，拒绝访问 API 密钥、凭据、token 和私钥。过滤器阻止精确匹配（`ANTHROPIC_API_KEY`、`AWS_SECRET_ACCESS_KEY`）、后缀模式（`*_API_KEY`、`*_SECRET`、`*_TOKEN`）和前缀模式（`AWS_SECRET_*`、`AWS_SESSION_*`）。只有 `PI_*` 变量无条件允许。

**信任生命周期和 kill-switch**：扩展信任状态显式跟踪（`pending`、`acknowledged`、`trusted`、`killed`）。Kill-switch 将扩展降级为 `killed`，在运行时风险控制器中隔离它，发出严重告警，并写入审计记录。解除开关需要显式操作员操作，并将扩展移回 `acknowledged`。

### 扩展运行时决策逻辑（通俗解释）

扩展运行时包含几个小型决策引擎，使行为在工作负载模式变化时保持稳定：

- **信息价值规划器（VOI）**：按"每毫秒预期学习"对候选探测排序，并在严格开销预算下选择最佳组合。过时或低价值的候选以显式原因跳过。
- **分片负载控制器**：基于队列压力、延迟和饥饿风险调整路由权重、批量预算和退避/帮助因子。阻尼和振荡防护防止过度反应。
- **策略安全评估器**：使用多个估计器回放历史样本，仅当样本支持强、不确定性低且预测后悔保持在限制内时才批准策略。

这些部分有意保守：如果置信度弱，Pi 保持稳定而不是做出激进的切换。

### 交互式 TUI 架构

交互模式使用 **Elm 架构**（模型-更新-视图），通过 `charmed_rust` 库家族实现，这是 Go 的 [Bubble Tea](https://github.com/charmbracelet/bubbletea) 框架的 Rust 移植版。

**组件堆栈：**

```
┌────────────────────────────────────────────────────┐
│                    终端 (crossterm)                  │
│  原始模式 │ 备选屏幕 │ 键盘/鼠标事件                │
└──────────────────────┬─────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────┐
│              bubbletea 程序循环                      │
│  Init() → Update(Msg) → View() → 渲染循环           │
└──────────────────────┬─────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────┐
│                   PiApp (模型)                      │
│                                                     │
│  ┌─────────────┐ ┌──────────────┐ ┌─────────────┐  │
│  │  TextArea    │ │  Viewport    │ │  Spinner     │  │
│  │  (编辑器)    │ │  (对话)      │ │  (状态)      │  │
│  └─────────────┘ └──────────────┘ └─────────────┘  │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │              覆盖层堆栈                       │    │
│  │  模型选择器 │ 会话选择器 │ /tree              │    │
│  │  设置 UI    │ 主题选择器 │ 分支               │    │
│  │  能力提示（扩展 UI）                          │    │
│  └─────────────────────────────────────────────┘    │
└──────────────────────┬─────────────────────────────┘
                       │
              async channels (mpsc)
                       │
┌──────────────────────▼─────────────────────────────┐
│              智能体异步任务                          │
│  运行在 asupersync 运行时上                          │
│  流式传输 provider 响应                             │
│  执行工具                                           │
│  发送 PiMsg 事件回 TUI 线程                         │
└────────────────────────────────────────────────────┘
```

**异步/同步桥接**：智能体在单独的线程中运行在 `asupersync` 异步运行时上。它通过 `mpsc` 通道与 bubbletea UI 线程通信。每个流式事件（文本增量、工具开始、工具更新、智能体完成）变成一个 `PiMsg` 变体，传递给 `PiApp::update()`，保持 UI 在 API 流式传输和工具执行期间响应。

**视口滚动**：对话视口跟踪用户是否在底部。当新内容到达且用户未向上滚动时，视口自动跟随流尾部。向上滚动禁用自动跟随；按下 `End` 或输入新消息重新启用它。

**覆盖系统**：模态 UI（模型选择器、会话选择器、分支导航器、扩展能力提示）堆叠在主对话视图之上。每个覆盖捕获键盘输入直到被解除。只有最上面的活跃覆盖接收事件。

**交互式编辑器中可用的斜杠命令：**

| 命令 | 操作 |
|---------|-------|
| `/help` | 显示可用命令和快捷键 |
| `/model` 或 `Ctrl+L` | 打开带模糊搜索的模型选择器 |
| `Ctrl+P` / `Ctrl+Shift+P` | 前/后循环作用域模型 |
| `/tree` | 浏览和分叉对话树 |
| `/clear` | 清除对话并重新开始 |
| `/compact` | 触发手动压缩 |
| `/thinking <level>` | 在对话中更改思考级别 |
| `/share` | 将会话导出为 GitHub Gist |
| `/exit` 或 `Ctrl+C` | 退出 Pi |

### RPC 协议

RPC 模式（`pi --mode rpc`）通过 stdin/stdout 暴露一个行分隔的 JSON 协议，用于程序化集成。每一行是一个自包含的 JSON 对象。

**客户端 → Pi (stdin)：**

```json
{"type": "prompt", "message": "Explain this function", "id": "req-001"}
{"type": "steer", "message": "Focus on error handling"}
{"type": "follow_up", "message": "Now add tests"}
{"type": "abort"}
{"type": "get_state"}
{"type": "compact", "reserveTokens": 8192, "keepRecentTokens": 20000}
```

**Pi → 客户端 (stdout)：**

```json
{"type": "agent_start", "sessionId": "..."}
{"type": "message_update", "message": {...}, "assistantMessageEvent": {"type": "text_delta", "delta": "The function", "contentIndex": 0}}
{"type": "tool_execution_start", "toolCallId": "...", "toolName": "read", "args": {}}
{"type": "tool_execution_end", "toolCallId": "...", "toolName": "read", "result": {}, "isError": false}
{"type": "agent_end", "sessionId": "...", "messages": [...]}
{"type": "response", "id": "req-001", "command": "prompt", "success": true, "data": {"status": "ok"}}
```

**I/O 架构**：两个专用线程处理 stdin 读取和 stdout 写入，通过通道桥接到异步智能体运行时。stdin 线程在临时错误时重试，以防止输入丢失。stdout 线程在每行后刷新，以防止缓冲延迟。

**消息队列**：当智能体正在流式传输响应时，传入消息路由到两个队列之一：

| 队列 | 行为 | 使用场景 |
|-------|--------|----------|
| **引导** | 中断当前响应；在下一轮次处理 | 方向修正 |
| **后续** | 排队直到当前响应完成 | 顺序指令 |

队列模式（`All` 或 `OneAtATime`）控制多个排队消息是批处理为一个轮次还是单独处理。

**通过 RPC 的扩展 UI**：当扩展请求用户输入（能力提示、选择对话框）时，Pi 发出 `extension_ui_request` 事件。客户端在自己的 UI 中渲染提示，并以 `extension_ui_response` 消息响应。然后 IDE 扩展可以为能力决策呈现原生 UI，而不是回退到终端提示。

### 会话索引

会话恢复（`pi -c` 或 `pi -r`）需要为当前项目找到最近的会话，而无需扫描磁盘上的每个 JSONL 文件。Pi 维护一个 SQLite 索引（`session-index.sqlite`），提供常量时间查找。

**模式：**

```sql
CREATE TABLE sessions (
    path            TEXT PRIMARY KEY,
    id              TEXT NOT NULL,
    cwd             TEXT NOT NULL,
    timestamp       TEXT NOT NULL,
    message_count   INTEGER NOT NULL,
    last_modified   INTEGER NOT NULL,
    size_bytes      INTEGER NOT NULL,
    name            TEXT
);
```

**更新生命周期：**

1. 保存会话 JSONL 文件后，Pi 将其元数据 upsert 到索引中
2. `pi -c` 查询 `WHERE cwd = ? ORDER BY last_modified DESC LIMIT 1`
3. `pi -r` 查询同一表并显示按最近排序的选择器

**并发**：基于文件的锁（`session-index.lock`）序列化来自并发 Pi 实例的写入。读取使用 WAL 模式实现非阻塞访问。

**基于陈旧的重新索引**：如果索引比可配置阈值更旧，Pi 运行会话目录的完整重新扫描，以捕获由其他实例或手动编辑创建的文件。重新扫描保持索引准确，无需集中守护进程。

### 会话存储 V2 Sidecar（大会话快速路径）

Pi 还支持 JSONL 会话旁边的 v2 sidecar 存储，用于更快的恢复和更强的大历史记录损坏检查。

**新增内容：**

- 分段追加日志文件（而不是一个不断增长的 JSONL 文件）
- 偏移索引行，用于直接查找和快速尾部读取
- 定期检查点和清单快照
- 用于可审计性的迁移账本条目
- 基于检查点的回滚路径，带显式回滚事件日志

**恢复工作方式：**

1. 如果 v2 sidecar 存在且新鲜，Pi 从 sidecar 索引 + 段打开。
2. 如果 sidecar 数据相对于源 JSONL 过时，Pi 回退到 JSONL 解析。
3. 如果索引数据缺失/损坏但段有效，Pi 重建索引。

**完整性策略：**

- 段帧携带 payload 和链哈希。
- 索引行存储字节偏移量加 CRC32C 校验和。
- 验证在信任 sidecar 之前检查偏移边界、校验和匹配以及帧/索引对齐。
- 截断的尾部帧在重建期间可恢复；非 EOF 的帧损坏失败关闭，而不是静默丢弃数据。

**CLI 支持：**

- `pi migrate <path> --dry-run` 验证迁移而不写入。
- `pi migrate <path>` 执行 JSONL 到 v2 的迁移并验证一致性。

### 认证与凭据管理

除了简单的 API 密钥，Pi 还支持 OAuth、AWS 凭据链、服务密钥交换和 bearer-token 认证。凭据存储在 `~/.pi/agent/auth.json` 中，通过文件锁定防止并发实例损坏。存储的 API 密钥可以是字符串字面量、`$ENV:VAR_NAME` 引用或 `$CMD:shell command` / `$COMMAND:shell command` 来源（在请求时解析修剪后的 stdout）。

| 机制 | Provider | 详情 |
|----------|-----------|---------|
| **API 密钥** | Anthropic、OpenAI、Gemini、Cohere 和许多 OpenAI 兼容 provider | 通过环境变量或设置设置静态密钥 |
| **OAuth** | Anthropic、OpenAI Codex、Google Gemini CLI、Google Antigravity、Kimi for Coding、GitHub Copilot、GitLab 和扩展定义的 OAuth provider | PKCE/状态验证流程，自动刷新；Kimi 使用设备流程 |
| **AWS 凭据** | Bedrock | 访问密钥 + 秘密 + 可选会话 token；区域感知 |
| **服务密钥** | SAP AI Core | 客户端 ID/秘密交换为 bearer token |
| **Bearer Token** | 自定义 provider | 认证存储中的静态 token |

**OAuth token 生命周期：**

1. 用户运行配置了 OAuth provider 的 `pi`
2. Pi 检查 `auth.json` 中是否存在现有 token
3. 如果缺失：在浏览器中打开授权 URL，用户认证，Pi 接收授权码，将其交换为访问 + 刷新 token，存储两者及过期时间戳
4. 如果过期但刷新 token 有效：交换刷新 token 获取新访问 token，更新 `auth.json`
5. Bearer token 附加到 API 请求

Google CLI 风格的 OAuth provider 将项目元数据与 token 负载一起携带。Pi 保留并刷新该负载，并可在需要时从 `GOOGLE_CLOUD_PROJECT` 或本地 `gcloud` 配置解析项目 ID。

**凭据状态报告**：`pi config` 显示每个已配置 provider 的凭据状态：`Missing`、`ApiKey`、`OAuthValid`（含过期前时间）、`OAuthExpired`（含过期后时间）、`AwsCredentials` 或 `BearerToken`。

**诊断代码**：认证失败产生特定的诊断代码（`MissingApiKey`、`InvalidApiKey`、`QuotaExceeded`、`OAuthTokenRefreshFailed`、`MissingAzureDeployment`、`MissingRegion` 等），并附带上下文特定的错误提示，而非通用消息。

---

## 工具详情

### read

读取文件内容（可选图片）：

```
Input: { "path": "src/main.rs", "offset": 10, "limit": 50 }
```

- 支持图片（jpg、png、gif、webp），可选自动调整大小
- 分块流式传输文件字节，带有硬大小限制以减少峰值内存使用
- 应用防御性图像解码限制，阻止解压炸弹/OOM 输入
- 截断至 2000 行或 1MB
- 如果截断则返回继续提示

### bash

执行 shell 命令，带超时和输出捕获：

```
Input: { "command": "cargo test", "timeout": 120 }
```

- 默认 120s 超时，每次调用可配置
- 设置 `timeout: 0` 禁用默认超时
- 超时时进程树清理（杀死子进程）
- 实时输出的滚动缓冲区
- 如果截断，完整输出保存到临时文件

### edit

精确字符串替换：

```
Input: { "path": "src/lib.rs", "old": "fn foo()", "new": "fn bar()" }
```

- 精确字符串匹配（无正则表达式）
- 如果旧字符串未找到或不明确则失败
- 返回差异预览

### grep

搜索文件内容：

```
Input: { "pattern": "TODO", "path": "src/", "context": 2, "limit": 100 }
```

- 支持正则表达式模式
- 匹配前后上下文行
- 遵循 .gitignore

### find

按模式发现文件：

```
Input: { "pattern": "*.rs", "path": "src/", "limit": 1000 }
```

- 通过 `fd` 使用 glob 模式
- 按修改时间排序
- 遵循 .gitignore

### ls

列出目录内容：

```
Input: { "path": "src/", "limit": 500 }
```

- 按字母排序
- 目录以尾部 `/` 标记
- 在限制处截断

---

## 性能工程

### 为什么 Rust 对 CLI 工具重要

CLI 工具的性能要求与服务器或 GUI 应用不同。关键指标是**首次交互时间**：用户调用命令后多快可以开始输入？

| 阶段 | TypeScript/Node.js | Rust |
|-------|-------------------|------|
| 进程生成 | ~10ms | ~10ms |
| 运行时初始化 | 200-500ms | 0ms（无运行时）|
| 模块加载 | 100-300ms | 0ms（静态链接）|
| JIT 预热 | 50-100ms | 0ms（AOT 编译）|
| **总计** | **360-910ms** | **~10ms** |

这种差异随着使用频率而累积，特别是在短迭代的终端工作流中。

### 极致优化手册

Pi 的优化更类似于低延迟引擎，而非典型的 CLI 应用。我们有意在多个层次应用激进优化，而不仅仅是"用 `--release` 编译然后祈祷"。

速度来源：

- **启动路径保持最小**：无 JS 运行时引导、无模块图加载、无 JIT 预热。
- **热主机调用特化**：常见的扩展主机调用使用类型化快速路径；不常见形状回退到兼容路径。
- **负载下自适应调度**：主机调用调度可在争用上升时切换模式，压力下降时切回。
- **快速路径安全防护栏**：采样影子双执行检查确保优化不会静默改变行为。
- **低分配渲染**：TUI 渲染缓冲区和 markdown 渲染结果被缓存/重用，而非每帧重建。
- **快速恢复内部机制**：会话索引加 v2 sidecar 布局避免在恢复时进行昂贵的完整历史扫描。
- **有界增长控制**：压缩和截断使 token/上下文增长和工具输出负载增长不会降低长会话的响应性。
- **测量优先文化**：性能制品经过模式验证并在 CI 中声明门控，因此优化工作由证据驱动，回归早期捕获。

这就是为什么即使有大量流式传输、工具使用、大会话历史和扩展工作负载同时运行，Pi 也能保持响应。

### 优化目录（代码 + 提交历史）

该目录反映了跨运行时、存储、流式传输和 UI 的长期变更序列。

本代码库中的具体工程工作包括：

| 领域 | 我们构建了什么 | 为什么重要 |
|------|---------------|-------------|
| 扩展调度核心 | 类型化主机调用操作码快速路径、兼容回退通道、零拷贝负载 arena、规范化哈希快捷方式、驻留操作路径 | 在最热的扩展操作上削减每次调用开销，同时保留正确性回退路径 |
| 注册表/策略查找 | 不可变策略快照，O(1) 能力检查，以及 RCU 风格元数据快照用于扩展注册表/工具元数据 | 消除热授权/调度路径中重复的动态查找开销 |
| 队列 + 并发 | 核心固定的 SPSC 反应器网格，受 S3-FIFO 启发的准入机制带公平守卫，BRAVO 风格回退行为 | 改善争用下的尾延迟，而不仅仅是优化中位延迟 |
| 批量执行 | AMAC 风格交错批次执行器，带 stall 感知切换 | 当许多独立主机调用在途时避免队头阻塞 |
| IO 特化 | io_uring 通道策略 + 遥测和显式执行器不可用回退 | 评估 IO 密集型调用，不将占位符桥接作为真正的 io_uring 执行呈现 |
| 扩展运行时启动 | QuickJS 预暖路径、暖隔离区/字节码缓存行为和启动管道并行化 | 在第一个真实扩展工作负载之前降低冷启动开销 |
| JS 桥接/调度器路径 | 待处理作业快速路径调优和热扩展循环中的桥接级调度清理 | 减少 JS 请求和 Rust 执行之间的开销 |
| 自适应控制循环 | 状态转移检测（CUSUM/BOCPD）、预算控制器、基于 VOI 的实验规划器、均场负载控制器、离线策略评估门禁 | 让运行时行为适应工作负载变化，无需盲目调优 |
| 快速路径安全 | 影子双执行采样，分歧时自动回滚/退避 | 防止加速工作静默改变语义 |
| 跟踪级优化 | 主机调用超指令编译器 + 二级跟踪 JIT 路径 | 融合重复的操作码跟踪，降低重复调度开销 |
| 工具执行策略 | 效果兼容工具分类 + 屏障感知并行执行路径 | 当多个工具调用可以安全地同时运行而不重新排序可变工具时提高吞吐量 |
| 会话写入路径 | 写后自动保存队列、耐久性模式、增量追加、检查点重写策略、单缓冲区追加序列化 | 保持交互快速，同时在需要时支持更强的持久性 |
| 会话索引路径 | 异步合并索引更新、待处理 drain 热路径调优、减少分配/装箱开销 | 保持恢复/发现元数据更新脱离交互热路径 |
| 会话恢复路径 | 会话存储 V2 sidecar（分段日志 + 偏移索引）、O(index+tail) 打开路径、陈旧 sidecar 检测、迁移/回滚工具 | 避免大历史记录上的完整文件重新扫描，改善恢复行为 |
| 长会话维护 | 后台压缩/快照工作器，带配额 + 惰性水合路径 | 控制会话增长，保持长时间运行的工作区响应 |
| 压缩内部 | 二分搜索切割点逻辑、序列化辅助提取、压缩路径上的零分配导向清理 | 降低压缩开销，防止压缩成为暂停源 |
| 流式内部 | SSE 解析器事件类型驻留、扫描字节跟踪（`scanned_len`）、UTF-8 恢复加固 | 减少 token 流期间的重复扫描/分配，改善韧性 |
| Provider/消息内存路径 | 零拷贝请求上下文迁移（`Cow`）、基于 `Arc` 的消息/结果共享、流结束路径中的克隆消除 | 消除核心智能体/provider 循环中的热路径克隆和分配抖动 |
| TUI 渲染路径 | 消息渲染缓存、对话前缀缓存、可重用渲染缓冲区、视口/渲染路径重构、Criterion 性能门禁 | 减少流式长输出时的重绘成本和抖动 |
| 启动/资源加载 | 并行化的技能/提示词/主题加载加预计算工具定义/命令名称 | 将繁重初始化移出关键路径以改善交互时间 |
| 分配器/构建配置 | 大小预算的默认发布配置、可选的 jemalloc 基准变体、性能声明的严格制品验证 | 保持发布制品在预算内，防止基准/报告漂移 |
| 性能治理 | 场景矩阵、声明完整性门禁、严格无数据失败行为、方差/置信度制品、可复现编排包 | 使性能声明可审计，回归检测自动化 |
| 主机调用编组规划器 | `HostcallRewriteEngine` 使用一个小型成本模型，仅在明确更便宜且无歧义时选取快速操作码融合；否则保持在规范路径并记录回退原因 | 在热编组形状上获得加速，同时不冒因模糊重写导致的静默语义漂移风险 |
| 工具文本处理热路径 | `truncate_head`/`truncate_tail` 使用惰性行遍历和 `memchr` 行计数；规范化切换为单次遍历转换而非链式字符串重写 | 大文件/工具输出避免不必要的中间分配并保持响应 |
| JS 桥接正则和解析器微路径 | 扩展 JS 桥接中频繁的正则检查使用 `OnceLock` 缓存；热桥接调用避免重复设置工作 | 削减扩展密集型会话中的重复每次调用开销 |
| CLI/自动补全过程生成减少 | `fd`/`rg` 可用性检查被缓存，自动补全文件索引刷新在后台运行，带陈旧更新丢弃 | 即使在非常大的仓库中也保持补全和命令处理快速 |
| 会话身份/索引微优化 | O(1) 条目 ID 缓存、单次遍历元数据终态化、追加路径清理替代多遍/高复制行为 | 当会话变大时减少追加/保存/重建的开销 |
| 基准驱动回滚纪律 | 候选微优化经过基准测试，可在真实工作负载回归时回退（例如，SSE 中的换行符扫描 `memchr` 交换已被回滚）| 防止理论上看起来快但实际使用中变慢的"优化" |

优化涵盖算法、执行通道、内存移动、队列纪律、存储布局和验证策略，作为一个系统。

### 发布配置和二进制大小门禁

发布的发布配置经过调优，使发布制品保持在二进制大小预算内，同时保留跨 crate 优化：

```toml
[profile.release]
opt-level = "z"      # 为大小优化生成的代码
lto = true           # 跨所有 crate 的链接时优化
codegen-units = 1    # 单一代码生成单元（编译更慢，优化更好）
panic = "abort"      # 无展开机制
strip = true         # 移除符号表
```

二进制大小在 CI 中通过 `binary_size_release` 显式预算，目标阈值为 `22.0 MiB`（测试工具计算字节 / 1024 / 1024）。默认发布构建将重量级额外功能保持为可选，目前 `pi` 使用标准 Cargo `release` 配置测量为 `21.12 MiB`。当你需要在一构建中包含图像、剪贴板、wasm、jemalloc 和语法高亮额外功能时，使用 `--features full`。

### 基准证据 vs 发布制品

Pi 将性能证据制品与可分发的发布制品分开：

- **基准证据制品**（PERF-3X 和认证输入）由 `scripts/perf/orchestrate.sh` 和 `scripts/bench_extension_workloads.sh` 生成，必须带有运行溯源（`correlation_id`）和配置标签（例如 `build_profile=perf`）。
- **发布制品**（最终用户二进制文件）使用 Cargo `release` 配置构建，并通过 GitHub Releases + 安装程序路径分发。

策略含义：单独的发布/大小制品不是全局性能声明的有效证据。性能声明必须引用具有可复现溯源的基准证据包。详见 `docs/testing-policy.md` 和 `docs/releasing.md`。

当前签入的性能证据状态：
- 运行输出：`tests/perf/reports/`（budget_summary.json、PERF_BUDGETS.md）
- 当前预算摘要是阻塞制品，而非声明支持：大多数 CI 强制预算没有测量数据，数据合同部分报告缺失或过时的必需证据。
- 在进行明确的刷新之前，运行 `python3 scripts/perf/preflight_budget_inputs.py` 列出缺失的预算输入、预期的制品路径和仅 RCH 的刷新命令。
- 编排的性能运行还会在 `budget_summary.json` 刷新之前写入 `results/perf_budget_preflight_before_refresh.json`，在收集后写入 `results/perf_artifact_staging_manifest.json`。staging 清单记录每个必需制品的源路径、通过 `PERF_REMOTE_TARGET_DIR` 提供的 RCH 远程源前缀、检索到的本地路径、模式、大小、mtime、校验和以及显式阻塞状态。
- 性能证据缓存条目位于 `PERF_EVIDENCE_CACHE_DIR`（默认：`$CARGO_TARGET_DIR/perf/evidence_cache`），模式为 `pi.perf.evidence_cache.v1`。预检和 staging 仅在条目的模式、命令、git 提交、构建配置、运行 ID/关联 ID、主机/工具链溯源、校验和和 TTL 验证通过时重用缓存证据；重用的制品在 JSON 输出中标记为 `source_kind=cache`/`evidence_source=cache`。使用 `PI_PERF_EVIDENCE_CACHE_TTL_HOURS` 覆盖缓存生命周期。
- `env_fingerprint.json` 记录 cgroup 感知的主机拓扑，模式为 `pi.perf.host_topology_fingerprint.v1`：cgroup v2 CPU 配额、cpuset 大小、内存限制、NUMA 节点数、注意事项以及容器化主机的受限 `budget_profile`。
- 在向本 README 添加面向发布版的速度、吞吐量、内存或启动数字之前，重新生成性能证据包。

最新认证/证据刷新（`2026-05-15` 进度 SLO 关闭；`2026-05-15` 扩展门禁；`2026-05-14` 全套件报告；`2026-05-18` drop-in 认证判定）：
- 统一证据包：`29/29` 个部分存在，`0` 个缺失，`0` 个无效 *（来源：tests/evidence_bundle/index.json）*
- 全套件门禁：`20/20` 个门禁通过，包括 `14/14` 个阻塞门禁 *（来源：tests/full_suite_gate/full_suite_verdict.json）*
- Drop-in 认证：`22/22` 个认证门禁通过，总体判定 `CERTIFIED` *（来源：docs/evidence/dropin-certification-verdict.json）*
- 扩展必过门禁：`123/123` 个必须通过的扩展通过；延伸集 `100/101` 通过，仅有非阻塞延伸失败 *（来源：tests/ext_conformance/reports/gate/must_pass_gate_verdict.json）*
- 上下文智能关闭门禁：`pass`，子 Beads 映射到代码、测试、docs/evidence、验证命令、推送提交、脱敏姿态、性能预算证据、README 新鲜度、staged UBS 和 Beads 账本对账 *（来源：docs/evidence/context-intelligence-closeout-gate.json）*
- 进度 SLO 关闭门禁：`pass`，子 Beads 映射到代码、测试、docs/evidence、验证命令、推送提交、源边界检查、压力预算证据、README 新鲜度、staged UBS 和 Beads 账本对账 *（来源：docs/evidence/swarm-progress-slo-closeout-gate.json）*
- 运行时智能关闭门禁：`pass`，子 Beads 映射到压缩准入、工具输出制品、provider 路由、调度器公平性、帧预算遥测、取消清理、扩展安全溯源、docs/evidence、源边界检查、推送提交、staged UBS 和 Beads 账本对账 *（来源：docs/evidence/runtime-intelligence-closeout-gate.json）*

### 快速循环 vs 确定性基准

在日常实现中，使用针对性检查保持迭代快速。将确定性基准结论保留在完整证据重新生成的集成边界。

- **快速循环（非权威）**：文件范围的 `cargo fmt --check` 和针对性测试回放（当编译非平凡时使用 `rch exec -- cargo test --test ...`）。
- **确定性通过（权威）**：将繁重运行卸载到严格远程门控（`rch exec -- ...` 或带 `--require-rch` 的脚本包装器），然后要求更新证据制品：
  - `tests/perf/reports/phase1_matrix_validation.json`
  - `tests/full_suite_gate/full_suite_verdict.json`
  - `tests/full_suite_gate/certification_verdict.json`
  - `tests/full_suite_gate/extension_remediation_backlog.json`

这使内部循环保持响应，同时在发布时保留严格的声明完整性。

### 扩展工作负载热点分析

Pi 包含一个专用的扩展运行时瓶颈工作负载测试工具：

```bash
cargo run --example ext_workloads -- \
  --out artifacts/perf/ext_workloads.jsonl \
  --matrix-out artifacts/perf/ext_hostcall_hotspot_matrix.json \
  --trace-out artifacts/perf/ext_hostcall_bridge_trace.jsonl
```

此测试工具所做的不仅仅是原始计时：

- 将主机调用成本分解为六个阶段：`marshal`、`queue`、`schedule`、`policy`、`execute`、`io`
- 生成热点矩阵（`pi.ext.hostcall_hotspot_matrix.v1`）用于快速瓶颈排序
- 生成桥接跟踪事件（`pi.ext.hostcall_trace.v1`）用于每次调用调试
- 测量阶段对如何交互并验证完整的阶段对覆盖
- 生成 VOI 调度器计划（`pi.ext.voi_scheduler.v1`），推荐在固定开销预算下下一个最有价值的实验

通俗来说：它帮助回答"下一步应该优化什么？"，用数据而非猜测。

对于多智能体操作员交接，`scripts/build_swarm_operator_runpack.py` 还可以从尾延迟报告、swarm 飞行记录器报告、swarm 资源预检、主机调用准入 swarm 配置文件、会话恢复 swarm 配置文件、并发 RPC swarm E2E 证据和 RCH 制品同步预检中投射一个仅诊断的 `bottleneck_attribution` 仪表板。该仪表板仅为操作员证据；面向发布版的速度、drop-in 或性能声明仍需通过以下声明完整性门禁。

runpack 还嵌入了 `predictive_telemetry_ledger`（`pi.swarm.predictive_telemetry_ledger.v1`），并可通过 `--out-predictive-telemetry-ledger-json` 单独写入或通过 `--print-predictive-telemetry-ledger` 打印。账本是只读咨询证据：它连接现有的 RCH、Agent Mail、Beads、轮次压力、瓶颈归因和声明就绪信号，形成有序压力观察和下一瓶颈假设，但不改变调度器、Agent Mail、RCH、Beads、git 或发布/容量声明。

runpack 还嵌入了 `validation_scheduler_plan`（`pi.swarm.validation_scheduler_plan.v1`），并可通过 `--out-validation-scheduler-plan-json` 单独写入或通过 `--print-validation-scheduler-plan` 打印。这是一个咨询性的 RCH 感知模拟器：它根据当前 git 变更配置文件、预测遥测、RCH 准入、远程证明和目标缓存证据对快速脚本检查、证据再生、针对性测试、E2E/合规、`cargo check --all-targets` 和 clippy 进行排序。它保留确切的命令字符串和所需的 RCH 环境，但不执行 cargo、不改变 RCH、不保留 Agent Mail、不声明 Beads、不删除临时制品，也不允许繁重的 cargo 失败开放到本地构建。

如果通过 `--validation-broker-json` 提供了验证 broker 状态或计划 JSON，runpack 将源状态、槽位计数、过时槽位警告、重复门禁机会和推荐下一步操作投射为咨询交接数据。此投射不替代 RCH、Doctor、Beads、Agent Mail、CI、UBS、`cargo_headroom.sh` 或发布声明门禁。操作员工作流、隐私和降级数据指南位于 [docs/swarm-operations-runbook.md#validation-broker-operator-workflow](docs/swarm-operations-runbook.md#validation-broker-operator-workflow)。

如果通过 `--progress-slo-json` 提供了进度 SLO JSON，runpack 将当前 swarm 进度姿态、置信度、饱和度摘要、源新鲜度、脱敏姿态和下一步操作为咨询交接数据。此投射不替代 Beads、Agent Mail、RCH、Doctor、验证门禁、git 或发布声明门禁。操作员工作流、隐私、降级源处理、陈旧 Beads 处理、RCH 压力解读和无开放工作收敛指南位于 [docs/swarm-operations-runbook.md#progress-slo-operator-workflow](docs/swarm-operations-runbook.md#progress-slo-operator-workflow)。

同一 runpack 命令可以输出 dry-run swarm 自动驾驶仪输入包和计划，旁边是交接包。当请求这些伴随制品时，runpack JSON/Markdown 包含一个 `autopilot_handoff` 摘要（`pi.swarm.autopilot_handoff.v1`），带有 `pi.swarm.autopilot_input_pack.v1` 和 `pi.swarm.autopilot_plan.v1` 模式引用、选定的咨询动作、制品路径和源溯源。自动驾驶仪从不改变所有权或替代 Doctor、Beads、Agent Mail、RCH、git 或源制品；它只将这些输入转化为可复现的操作员下一步行动指南。其工作准入门禁包括一个只读 dry-run 执行器，将计划项分类为 `would_execute`、`blocked`、`requires_operator` 或 `never_execute`，使得删除请求、Agent Mail/RCH 变更、本地重量级 Cargo 和 Beads 所有权绕过在不执行任何操作的情况下被拒绝。

对于降级协调交接证明，runpack 脚本还有一个无模拟 E2E 模式（`--run-degraded-coordination-e2e`），将真实临时 Beads 状态加夹具捕获的 Agent Mail 和 RCH 故障通过 runpack 构建器传递。发出的证据保持仅操作员：它证明 Beads 软锁定回退、降级验证和无清理/删除命令输出，而不支持面向发布版的速度或 drop-in 声明。

第四波 swarm 自愈指导将只读过时证据更新队列、dry-run 行动计划、工作准入门禁、预算租赁模拟、轮次压力账本、扩展隔离演练和脱敏交接摘要层叠到该 runpack 流程之上。这些制品可以推荐 `renew_stale_evidence`、`wait_for_pressure`、`use_beads_soft_lock` 或类似的操作员操作，但它们不会改变 Beads、Agent Mail、扩展配置、git、证据文件、RCH 作业或发布声明。完整工作流和安全交接措辞位于 [docs/swarm-operations-runbook.md#fourth-wave-self-healing-workflow](docs/swarm-operations-runbook.md#fourth-wave-self-healing-workflow)。

第四波关闭门禁输出 `pi.swarm.fourth_wave_self_healing.closeout_gate.v1`。它将 `bd-63x3v.7` 子 Beads 映射到实现制品、合同、夹具、操作员文档、验证命令、推送引用、源边界检查和剩余咨询限制。它受 `docs/contracts/fourth-wave-self-healing-closeout-gate-contract.json` 管理；当前关闭制品是 `docs/evidence/fourth-wave-self-healing-closeout-gate.json`。

离线 swarm 回放跟踪可以在交接前通过 `pi swarm-replay-preview --trace <trace.json> --format json` 预览。命令是只读的，输出 `pi.swarm.replay_preview.v1` JSON 或简洁文本，拒绝覆盖请求的输出文件，并可以馈送到 `scripts/build_swarm_operator_runpack.py --swarm-replay-preview-json <preview.json>`，以便 runpack 携带回放策略比较，而不将 runpack 视为回放证据的真实来源。操作员工作流、隐私和降级数据指南位于 [docs/swarm-replay-operator-workflow.md](docs/swarm-replay-operator-workflow.md)。

自动驾驶仪关闭门禁输出 `pi.swarm.autopilot_decision_gate.v1`，在 swarm-autopilot 史诗关闭前审计已发布的输入包、规划器、工作分区、故障操作、预算漂移监视器、E2E/日志证据、runpack 交接、安全守卫、推送提交和质量门禁。

上下文智能有自己的操作员指南和关闭门禁。详见 [docs/context-intelligence.md](docs/context-intelligence.md)。上下文智能关闭门禁输出 `pi.context_intelligence.closeout_gate.v1`，受 `docs/contracts/context-intelligence-closeout-gate-contract.json` 管理；当前关闭制品是 `docs/evidence/context-intelligence-closeout-gate.json`。

进度 SLO 关闭门禁输出 `pi.swarm.progress_slo.closeout_gate.v1`。它审计已发布的合同/源清单、确定性评估器、只读 CLI、Doctor/runpack 投射、无模拟 E2E 证据、大型主机压力预算、操作员文档、推送子提交、源边界检查、staged UBS 和 Beads 账本对账。它受 `docs/contracts/swarm-progress-slo-closeout-gate-contract.json` 管理；当前关闭制品是 `docs/evidence/swarm-progress-slo-closeout-gate.json`。

运行时智能关闭门禁输出 `pi.runtime_intelligence.closeout_gate.v1`。它将 `bd-h66tp` 子 Beads 映射到压缩准入、工具输出制品、provider 路由、调度器公平性、帧预算遥测、取消清理、扩展安全溯源、docs/evidence、源边界检查、推送引用和质量门禁。它受 `docs/contracts/runtime-intelligence-closeout-gate-contract.json` 管理；当前制品是 `docs/evidence/runtime-intelligence-closeout-gate.json`。该门禁仅为咨询性关闭证据，不替代 Beads、git、RCH、UBS、CI、声明完整性门禁或源制品。

第六波验证加固关闭门禁输出 `pi.swarm.validation_hardening.closeout_gate.v1`。它将 `bd-63x3v.9` 子 Beads 映射到 RCH 工作区阴影检测、Agent Mail 降级 Beads 软锁定证据、验证证明回放、临时制品清单、过时声明启发式、降级协调 E2E 证明、cgroup/NUMA/RCH 预算建模、provider/RPC/TUI 背压预算、操作员说明、推送引用、源边界检查和质检门禁。provider/RPC/TUI 合同现在包括一个合成公平性压力门禁，用于平衡负载、一个慢速 provider 流、RPC 输出洪泛和 TUI 帧预算压力，当某个表面报告成功而另一个表面饥饿时失败关闭。它受 `docs/contracts/sixth-wave-validation-hardening-closeout-gate-contract.json` 管理；当前制品是 `docs/evidence/sixth-wave-validation-hardening-closeout-gate.json`。该门禁仅为咨询性关闭证据，不替代 Beads、git、RCH、UBS、CI、声明完整性门禁或源制品。

第七波运行时自治关闭门禁输出 `pi.swarm.runtime_autonomy.closeout_gate.v1`。它将 `bd-63x3v.10` 子 Beads 映射到效果感知工具批处理、失败关闭验证证明重用、cgroup/NUMA 通道放置、大会话回放加速、provider/RPC/TUI 公平性、工作准入 dry-run 执行、主机调用 QoS 饥饿证据、源边界检查、推送引用和质量门禁。它受 `docs/contracts/seventh-wave-runtime-autonomy-closeout-gate-contract.json` 管理；当前制品是 `docs/evidence/seventh-wave-runtime-autonomy-closeout-gate.json`。该门禁仅为咨询性关闭证据，不替代 Beads、git、RCH、UBS、CI、声明完整性门禁、子证据或源制品。

证据携带的 swarm 测试结构关闭门禁输出 `pi.swarm.proof_carrying_test_fabric.closeout_gate.v1`。它将 `bd-zeccr` 子 Beads 映射到无模拟生命周期 E2E 证据、跨表面合规、操作员证据金标准、结构感知模糊/属性覆盖、变形回放等价性、源边界检查、推送引用、负面控制和质量门禁。它受 `docs/contracts/proof-carrying-swarm-test-fabric-closeout-gate-contract.json` 管理；当前制品是 `docs/evidence/proof-carrying-swarm-test-fabric-closeout-gate.json`。该门禁仅为咨询性关闭证据，不替代 Beads、git、RCH、Agent Mail、UBS、CI、声明完整性门禁、子证据或源制品。

预测运维关闭门禁输出 `pi.swarm.predictive_operations.closeout_gate.v1`。它将 `bd-63x3v.11` 子 Beads 映射到预测遥测融合、RCH 感知验证调度、语义压缩质量、扩展主机调用成本归因、操作员感知延迟、冗余智能体工作检测、源边界检查、推送引用、staged UBS 和 Beads 账本对账。它受 `docs/contracts/predictive-operations-closeout-gate-contract.json` 管理；当前制品是 `docs/evidence/predictive-operations-closeout-gate.json`。该门禁仅为咨询性关闭证据，不替代 Beads、git、RCH、Agent Mail、UBS、CI、声明完整性门禁、子证据或源制品。

第九波事件回放和证明内存关闭门禁输出 `pi.swarm.incident_replay_proof_memory.closeout_gate.v1`。它将 `bd-9yq7i` 子 Beads 映射到事件语料库、事件回放、验证证明内存、操作员工作推荐、操作员流畅度 SLO、扩展资源防火墙矩阵和无模拟事件回放 E2E 证据。它受 `docs/contracts/ninth-wave-incident-replay-proof-memory-closeout-gate-contract.json` 管理；当前制品是 `docs/evidence/ninth-wave-incident-replay-proof-memory-closeout-gate.json`。该门禁仅为咨询性关闭证据，不替代 Beads、git、RCH、Agent Mail、UBS、CI、声明完整性门禁、子证据、生成的 target/perf 输出、前波证据或源制品。

位于 `docs/contracts/closeout-evidence-registry.json` 的关闭证据注册表是当前咨询性关闭门禁制品的机器可读索引。它将每个当前制品绑定到其管理合同、决策模式、源 Bead 或史诗、Markdown 引用和咨询边界，包括以散文形式展开的当前制品：`docs/evidence/adaptive-execution-closeout-gate.json`、`docs/evidence/extension-compatibility-closeout-gate.json` 和 `docs/evidence/swarm-replay-closeout-gate.json`。注册表不是发布清单、性能证据包、Beads 真实来源、Agent Mail 权威、RCH 权威、CI/UBS 替代品、drop-in 认证门禁或删除/变更文件的权限。

操作员感知延迟跟踪输出 `pi.operator.perceived_latency_trace.v1`。它连接确定性 provider、RPC、TUI、工具更新和操作员可见夹具时间线，使操作员能够看到语义输出何时变得可见，而低价值更新则被合并。它受 `docs/contracts/operator-perceived-latency-trace-contract.json` 管理；当前夹具制品是 `docs/evidence/operator-perceived-latency-trace.json`。跟踪仅为咨询性，不授权基准、容量、发布性能、严格 drop-in 或背压预算替代声明。

操作员流畅度 SLO 输出 `pi.operator.smoothness_slo.v1`。它使用确定性高容量夹具用于 provider 流增量、RPC 输出压力、TUI 帧渲染、工具更新合并和会话写入压力。它受 `docs/contracts/operator-smoothness-slo-contract.json` 管理；当前夹具制品是 `docs/evidence/operator-smoothness-slo.json`。其 p50/p95/p99 计数器仅为工程夹具计数器，证明语义里程碑在低价值更新可合并时仍然可见；它们不是发布基准、容量、性能或严格 drop-in 证据。

扩展资源防火墙矩阵从确定性扩展压力夹具输出 `pi.ext.resource_firewall_matrix.v1`。它涵盖廉价读取洪泛、大负载发射、被拒绝的能力抖动、慢速主机调用、重复失败和稳定对等进度，同时保留负载脱敏和现有扩展能力边界。合同是 `docs/contracts/extension-resource-firewall-matrix-contract.json`；针对性测试运行在配置的 `target/perf` 目录下写入 `resource_firewall_matrix.json`。此矩阵仅为咨询性压力证据，不替代运行时执行、主机调用成本归因、RCH 验证、Agent Mail、Beads、UBS、CI 或基准/容量/发布声明。

Swarm 事件语料库输出 `pi.swarm.incident_corpus.v1`。它记录确定性降级源夹具，用于 Agent Mail 模式损坏、RCH 饱和（本地回退拒绝）、过时证据、重复工作风险、脏工作树准入拒绝、格式错误源以及删除或实时变更拒绝。它受 `docs/contracts/swarm-incident-corpus-contract.json` 管理；当前夹具制品是 `docs/evidence/swarm-incident-corpus.json`。语料库仅为操作员证据，不替代发布性能、drop-in 认证、Agent Mail、RCH、Beads、git、源制品或破坏性操作权限。

Swarm 事件回放测试工具输出 `pi.swarm.incident_replay.v1`。它使用已签入的事件语料库，并将健康、降级 Agent Mail、饱和 RCH、重复工作、过时证据、格式错误源、脏工作树和删除请求场景回放为有序阶段、每步断言、脱敏摘录、选定的安全操作和后续建议。它受 `docs/contracts/swarm-incident-replay-contract.json` 管理；当前夹具制品是 `docs/evidence/swarm-incident-replay.json`。回放输出是只读操作员证据，不能替代源系统或授权实时的 Agent Mail、RCH、Beads、git、删除、发布、基准或 drop-in 声明。

Swarm 事件回放 E2E 测试工具输出 `pi.swarm.incident_replay_e2e.v1` 加 `pi.swarm.incident_replay_e2e.event.v1` JSONL 事件。它结合了真实临时 Beads 和 git 工作区与夹具捕获的降级 Agent Mail 和 RCH 输入，然后验证健康回放、Beads 软锁定回退、RCH 证明刷新退避、重复工作风险、脏工作树拒绝、过时证明内存刷新、扩展资源防火墙失败和流畅度 SLO 失败。它受 `docs/contracts/swarm-incident-replay-e2e-contract.json` 管理；当前夹具制品是 `docs/evidence/swarm-incident-replay-e2e.json`。此证据仅为咨询性，不授权实时的源变更、本地重量级 Cargo 回退、发布、基准、容量或 drop-in 声明。

验证证明内存索引输出 `pi.validation.proof_memory_index.v1`。它按命令指纹、git head、触碰路径、RCH 溯源、制品检索哈希、新鲜度和重用资格索引现有远程验证证明夹具。索引受 `docs/contracts/validation-proof-memory-index-contract.json` 管理；当前夹具制品是 `docs/evidence/validation-proof-memory-index.json`。它仅为咨询性操作员证据：过时引用、缺失制品、本地回退、脏工作树不匹配、命令不匹配、未覆盖路径或非权威覆盖都失败关闭并需要新鲜验证。

操作员工作推荐器输出 `pi.swarm.operator_work_recommendation.v1`。它消耗事件回放和证明内存制品，然后对就绪 Beads、无就绪工作、Agent Mail 损坏、RCH 饱和、过时证明刷新、重复工作风险和脏工作树准入拒绝的只读下一步决策进行排序。它受 `docs/contracts/operator-work-recommendation-contract.json` 管理；当前夹具制品是 `docs/evidence/operator-work-recommendation.json`。其推荐仅为咨询性，从不声明、保留、启动 RCH、运行 cargo、变更 git、删除文件或替代源系统。

有关大规模多智能体运行的完整启动、节流、恢复和交接工作流，请参阅 [docs/swarm-operations-runbook.md](docs/swarm-operations-runbook.md)。

### 性能报告的声明完整性门禁

Pi 的性能管道包括严格的证据检查，因此全局速度声明不能基于部分或过时的数据。

- `scripts/perf/orchestrate.sh` 生成绑定到同一运行共享 `correlation_id` 的制品。
- `scripts/e2e/run_all.sh` 在认为声明有效之前验证必需的模式、新鲜度和 `correlation_id` 对齐。
- `tests/release_evidence_gate.rs` 当合规/性能制品缺失 `run_id` 或 `correlation_id`，或链路制品间的世系字段不一致时失败关闭。
- `scripts/e2e/run_all.sh` 输出证据裁定矩阵，并且仅在新鲜度和世系检查都通过时才将证据视为规范。
- 关键面向发布版的制品包括：
  - `pi.perf.extension_benchmark_stratification.v1`
  - `pi.perf.phase1_matrix_validation.v1`
  - `pi.claim_integrity.evidence_adjudication_matrix.v1`

如果证据集不完整或矛盾，声明完整性门禁保持关闭并报告确切原因。

### 基准的分配器策略

默认发布构建使用平台分配器以保持在发布二进制大小预算内。FreeBSD 和 MSVC 构建也始终使用平台分配器；FreeBSD libc 内部已使用 jemalloc，混合 libc/pthread 和 C 依赖间的分配器域可能将堆损坏转化为线程生成崩溃。对于分配器实验，Pi 支持显式的 `system` 和 `jemalloc` 基准变体：

```bash
# 系统分配器基线 + jemalloc 变体在同一个可复现运行中
BENCH_ALLOCATORS_CSV=system,jemalloc \
  ./scripts/bench_extension_workloads.sh
```

基准测试工具在其 JSONL 输出中记录请求和有效的分配器元数据（`allocator_requested`、`allocator_effective`、`allocator_fallback_reason`），通过 `PI_BENCH_ALLOCATOR`。

- `system`：除 `jemalloc` 外，使用显式基准特性集构建
- `jemalloc`：在目标支持时使用 `--features jemalloc` 构建
- `auto`：优先选择 `jemalloc`，如果构建失败则回退到 `system`

如果请求了 `jemalloc` 但对当前构建不可用，运行失败关闭到编译的分配器，并在制品中包括回退原因。

### 内存使用

Rust 的所有权模型使得内存使用可预测，无垃圾收集暂停：

| 状态 | 内存 |
|-------|-------|
| 启动（空闲）| ~15MB |
| 活跃会话（小）| ~25MB |
| 上下文中的大文件 | ~30-50MB |
| 流式响应 | +0MB（流式传输，非缓冲）|

没有 GC 意味着流式输出期间无意外延迟峰值。

### 流式架构

响应以最小缓冲从 API 逐 token 流式传输到终端：

```
API Server → TCP → SSE Parser → Event Handler → Terminal
     │                              │
     └──────── 无缓冲 ──────────────┘
```

每个 token 在离开 Anthropic 服务器后的毫秒内出现在屏幕上。SSE 解析器在事件到达时处理，而不是等待完整响应。

### TUI 渲染性能

交互式 TUI 以 60fps 渲染为目标，具有多个优化层：

**帧定时遥测**：每个渲染周期都有仪器化。慢帧（>16ms）被跟踪并按阶段分类：视口同步、消息编码、markdown 渲染。此数据馈送到内部性能监视器。

**消息渲染缓存**：Markdown 到 ANSI 的转换涉及语法高亮、表格布局和链接检测。Pi 缓存每条消息的渲染输出，仅在主题更改或终端调整大小时失效。在流式传输期间，只有正在变化的消息被重新渲染；所有先前的消息都命中缓存。

**预分配渲染缓冲区**：`RenderBuffers` 结构在渲染周期之间重用。Pi 不是每帧分配新的 `String` 缓冲区，而是写入预分配大小的缓冲区并在重用前清除它们，从而在流式传输期间消除了每秒数千次的小型分配。

**内存压力监控**：`MemoryMonitor` 采样进程堆大小并将其分为三个等级：

| 等级 | 阈值 | 操作 |
|------|--------|-------|
| **正常** | 预算的 <80% | 无操作 |
| **压力** | 预算的 80-95% | 折叠工具输出显示，隐藏思考块 |
| **严重** | 预算的 >95% | 截断旧消息，强制压缩 |

渐进式降级使 Pi 在具有累积工具输出的长会话期间保持响应。

---

## 故障排除

更完整的指南请参见 [docs/troubleshooting.md](docs/troubleshooting.md)。

### "fd not found"

`find` 工具需要 `fd`：

```bash
# Ubuntu/Debian
apt install fd-find

# macOS
brew install fd

# 二进制文件可能名为 fdfind
ln -s $(which fdfind) ~/.local/bin/fd
```

### "API key not set"

```bash
export ANTHROPIC_API_KEY="sk-ant-..."

# 或在 settings.json 中
{ "apiKey": "sk-ant-..." }

# 或每命令设置
pi --api-key "sk-ant-..." "Hello"
```

### "Session corrupted"

会话是仅追加的 JSONL。如果发生损坏：

```bash
# 重新开始
pi --no-session

# 或删除有问题的会话
rm ~/.pi/agent/sessions/--home-user-project--/corrupted-session.jsonl
```

### "Streaming hangs"

检查你的网络连接。Pi 使用 SSE，需要稳定连接：

```bash
# 用 curl 测试
curl -N https://api.anthropic.com/v1/messages
```

### "Tool output truncated"

这是有意为之，以防止上下文溢出。使用偏移量/限制：

```bash
# 在对话中
"Read lines 2000-4000 of that file"
```

---

## 局限性

Pi 诚实地说明它不能做什么：

| 局限性 | 解决方法 |
|------------|------------|
| **并非所有 provider API** | 内置支持由 10 个原生 provider 实现模块支持：Anthropic、OpenAI Chat、OpenAI Responses/Codex Responses、Gemini、Cohere、Azure OpenAI、Bedrock、Vertex AI、GitHub Copilot 和 GitLab Duo；部分生态系统特定 API 仍有待确定 |
| **无网页浏览** | 使用 bash 配合 curl |
| **无 GUI** | 设计上仅限终端 |
| **部分扩展需要 npm stubs** | 提供常见 stubs；未列出的 npm 包仍需要 stub。详见 docs/planning/EXTENSIONS.md §8.1 |
| **以英语为中心** | 可用但未针对其他语言优化 |
| **需要 Rust nightly** | 使用 2024 版特性 |

---

## 设计哲学

### 规范优先移植

本移植采用"规范提取"方法论，而非逐行翻译：

1. **提取行为**：研究 TypeScript 实现以理解它*做什么*，而非*如何做*
2. **文档化规范**：记录预期行为、边缘情况和不变量
3. **根据规范实现**：编写满足规范的惯用 Rust 代码
4. **一致性测试**：通过基于夹具的测试验证行为匹配

这种方法比机械翻译产生更好的代码。TypeScript 惯用语（回调、promise、类层次结构）不能干净地映射到 Rust（所有权、trait、枚举）。与语言对抗会产生比接受它更差的结果。

### 一致性测试

测试套件包括基于夹具的一致性测试，用于验证工具行为：

```json
{
  "version": "1.0",
  "tool": "edit",
  "cases": [
    {
      "name": "edit_simple_replace",
      "setup": [
        {"type": "create_file", "path": "test.txt", "content": "Hello, World!"}
      ],
      "input": {
        "path": "test.txt",
        "oldText": "World",
        "newText": "Rust"
      },
      "expected": {
        "content_contains": ["Successfully replaced"],
        "details": {"oldLength": 5, "newLength": 4}
      }
    }
  ]
}
```

每个夹具指定：
- **设置**：测试前要创建的文件/目录
- **输入**：工具参数
- **期望**：输出内容模式、精确字段匹配或错误条件

Rust 实现可以针对 TypeScript 原版进行验证，而无需耦合到实现细节。

### 扩展系统

Pi 通过嵌入式 QuickJS 运行时支持旧版 JS/TS 扩展。与传统的插件系统不同，扩展在**沙箱化、能力门控**的环境中运行，没有环境 OS 访问权限：

1. **无需 Node/Bun**：QuickJS + Pi 提供的常见 Node API shims
2. **基于能力的安全性**：每个主机连接器调用都经过策略检查和日志记录
3. **经过一致性测试**：状态在 `docs/ext-compat.md` 和 `tests/ext_conformance/reports/pipeline/` 中跟踪
4. **加载时间 <100ms**：扩展在 <100ms（P95）内加载，无需 JIT 预热

旧版扩展行为自动生效：
- 现有的 `.js/.ts` 扩展直接运行（无需手动转换步骤）。
- `*.native.json` 描述符是可选的，主要用于原生 Rust 运行时工作流。
- 一个会话目前一次使用一个运行时家族（JS/TS 或原生描述符）。

策略预设快速入门：

```bash
# 查看当前有效策略
pi --explain-extension-policy

# 为一条命令切换配置文件（safe | balanced | permissive）
pi --extension-policy balanced --explain-extension-policy

# 旧版别名仍被接受：
pi --extension-policy standard --explain-extension-policy

# 缩小危险能力的 opt-in（优于 permissive）
PI_EXTENSION_ALLOW_DANGEROUS=1 pi --extension-policy balanced --explain-extension-policy
```

操作员部署剧本（兼容优先本地默认 + 显式锁定）：

```bash
# 1) 基线：验证默认是兼容优先（`permissive`）
pi --explain-extension-policy

# 2) 预发布：使用 balanced 提示，危险能力仍默认拒绝
pi --extension-policy balanced --explain-extension-policy

# 3) 为严格本地/CI 运行显式锁定
pi --extension-policy safe --explain-extension-policy

# 4) 危险能力的窄范围 opt-in（推荐路径）
PI_EXTENSION_ALLOW_DANGEROUS=1 pi --extension-policy balanced --explain-extension-policy

# 5) 当你希望明确无歧义时的显式 permissive 模式
pi --extension-policy permissive --explain-extension-policy
```

本地/开发的 `settings.json` 基线：

```json
{
  "extensionPolicy": {
    "defaultPermissive": true
  }
}
```

使用以下恢复更严格的回退，无需 CLI 标志：

```json
{
  "extensionPolicy": {
    "defaultPermissive": false
  }
}
```

交互式 TUI：打开 `/settings` 并切换 `extensionPolicy.defaultPermissive`。

CI 指南：

```bash
# CI 默认：保持危险能力禁用
pi --extension-policy safe --explain-extension-policy

# CI opt-in 作业（仅必要时），保持显式和可审计
PI_EXTENSION_ALLOW_DANGEROUS=1 pi --extension-policy balanced --explain-extension-policy
```

回滚规则：移除 `PI_EXTENSION_ALLOW_DANGEROUS`，将 `extensionPolicy.profile` 设置回 `safe` 或将 `extensionPolicy.defaultPermissive` 设置为 `false`，然后重新运行 `pi --explain-extension-policy` 以确认拒绝决策。

参见 [EXTENSIONS.md](docs/planning/EXTENSIONS.md) 了解完整架构、运行时合同和一致性结果。

### 禁止不安全代码

`#![forbid(unsafe_code)]` 指令是项目范围的，不可协商。理由：

- **攻击面**：Pi 执行用户提供的 shell 命令并读取任意文件
- **内存错误 = 安全错误**：在此上下文中，缓冲区溢出或释放后使用可能是可利用的
- **性能无关**：瓶颈是到 API 的网络延迟，而非 CPU 周期
- **依赖经过审计**：所有依赖要么不使用 unsafe，要么经过良好审计（例如 `rustls`）

安全的 Rust 子集提供所有必要功能，无需妥协安全性。

---

## 常见问题

**问：与原始 Pi Agent 的关系是什么？**
答：这是 [Pi Agent](https://github.com/badlogic/pi)（作者 [Mario Zechner](https://github.com/badlogic)）的授权 Rust 移植版，已获作者许可。架构与 TypeScript 原版有显著不同：它使用 [asupersync](https://github.com/Dicklesworthstone/asupersync) 进行结构化并发，使用 [rich_rust](https://github.com/Dicklesworthstone/rich_rust)（Will McGugan 的 [Rich](https://github.com/Textualize/rich) 库的移植版）进行终端渲染。目标是保留 Pi Agent 的用户体验同时采用惯用 Rust。

**问：为什么用 Rust 重写？**
答：当你在终端中工作一整天时，启动时间很重要。Rust 给我们 <100ms 启动 vs Node.js 的 500ms+。此外，无需管理运行时依赖。

**问：我可以使用 Anthropic 以外的 provider（OpenAI/Gemini/Cohere/Azure/Bedrock/Vertex/Copilot/GitLab/Codex）吗？**
答：可以。Pi 有 10 个原生 provider 实现模块：Anthropic、OpenAI Chat、OpenAI Responses/Codex Responses、Gemini（原生 + Gemini CLI + Antigravity 路由）、Cohere、Azure OpenAI、Amazon Bedrock、Vertex AI、GitHub Copilot 和 GitLab Duo。Pi 还支持许多 OpenAI 兼容预设（例如 Groq、OpenRouter、Mistral、Together、DeepSeek、Cerebras、DeepInfra、阿里云/Qwen 和 Moonshot/Kimi）。Provider ID 和别名不区分大小写。设置凭据并通过 `--provider`/`--model` 选择；运行 `pi --list-providers` 查看规范 ID、别名和环境密钥。

**问：会话如何工作？**
答：默认情况下，每个会话是一个 JSONL v3 文件，包含消息条目、用于分支的父引用和压缩元数据。构建默认包含 `sqlite-sessions` 支持，因此配置的部署也可以使用 SQLite 支持的会话存储；JSONL 仍然是默认存储，除非配置选择 SQLite。

**问：为什么禁止 unsafe？**
答：对于执行任意命令的工具来说，内存安全不可协商。此用例的性能成本可以忽略不计。

**问：如何扩展 Pi？**
答：Pi 有一个完整的扩展系统，具有两个运行时家族：JS/TS 入口点在嵌入式 QuickJS 中运行，`*.native.json` 描述符在原生 Rust 描述符运行时中运行。两者都通过相同的策略系统进行能力门控和审计。一个会话一次使用一个运行时家族。扩展可以注册工具、斜杠命令、事件钩子、标志和自定义 provider。详见 [EXTENSIONS.md](docs/planning/EXTENSIONS.md)。对于内置工具的更改，在 `src/tools.rs` 中实现 `Tool` trait。

**问：为什么没有包含 X 功能？**
答：Pi 专注于核心编码辅助。网页浏览、图像生成等功能不在范围内。请使用专门工具完成这些任务。

**问：压缩是如何工作的？**
答：当对话超过模型的上下文窗口时，Pi 使用 LLM 自身总结较旧的消息，将摘要存储为会话条目。近期消息保持原样。切割点在轮次边界选择，摘要包括读取或修改了哪些文件的记录，以便模型保留该意识。压缩在需要时在每个智能体轮次后自动运行，或通过 `/compact` 手动运行。

**问：我可以添加 Pi 本身不原生支持的自定义 provider 吗？**
答：可以。在 `~/.pi/agent/` 或 `.pi/` 中创建一个 `models.json` 文件，包含指定模型 ID、基础 URL 和 API 类型（对于 OpenAI 兼容端点通常为 `openai-completions`）的条目。Pi 的兼容配置系统处理字段名差异和特性标志覆盖。扩展也可以注册完全自定义的 provider。

**问：Pi 如何决定恢复哪个会话？**
答：Pi 维护一个 SQLite 会话元数据索引 sidecar，带有 WAL/锁处理和陈旧索引重建。当你运行 `pi -c` 时，它查询该索引，查找工作目录与当前项目匹配且最近修改的会话，包括 JSONL 会话和配置的 SQLite 支持的会话。这避免了每次恢复时扫描文件系统。

**问：如果扩展尝试访问危险内容会怎样？**
答：来自扩展的每个主机调用在执行前都会根据活动能力策略进行检查。危险能力（`exec`、`env`）在 `safe` 和 `balanced` 下默认拒绝，除非显式 opt-in（例如通过 `PI_EXTENSION_ALLOW_DANGEROUS=1`），在 `permissive` 下可用。对于 `exec`，Pi 在生成前应用命令调解：它对命令+参数签名进行分类，并默认阻止关键类别（例如递归删除、磁盘/设备写入、反向 shell），严格/安全策略也可以阻止高等级类别（例如关机、进程终止、凭据文件修改）。被拒绝的调用将错误返回给扩展 Promise 路径，拒绝事件记录在脱敏的安全告警和 exec 调解审计制品中。敏感的环境密钥（API 密钥/token/机密）仍被过滤。如果行为升级，你可以立即将该扩展 kill-switch 到隔离的 `killed` 状态，或在调查时强制使用兼容通道路由作为遏制步骤。

**问：Pi 可以与自托管或代理 LLM 一起使用吗？**
答：可以。通过 `models.json` 将任何 provider 指向自定义基础 URL。Pi 按 API 类型规范化 URL 路径，并对字段名和特性差异应用兼容性覆盖。这适用于 vLLM、Ollama、LiteLLM 和类似的 OpenAI 兼容服务器。

---

## 对比

| 特性 | Pi | Claude Code | Aider | Cursor |
|---------|-----|-------------|-------|--------|
| **语言** | Rust | TypeScript | Python | Electron |
| **启动** | <100ms | ~1s | ~2s | ~5s |
| **内存** | <50MB | ~200MB | ~150MB | ~500MB |
| **Provider** | 10 个原生 provider 实现模块 + OpenAI 兼容预设 | Anthropic | 多 | 多 |
| **工具** | 8 个内置 | 多 | 文件聚焦 | IDE 集成 |
| **会话** | JSONL 树 | 专有 | 基于 Git | 专有 |
| **开源** | 是 | 是 | 是 | 否 |

---

## 开发

### 构建

```bash
./scripts/cargo_headroom.sh build           # 调试构建（远程卸载）
./scripts/cargo_headroom.sh build --release # 发布构建（优化，远程卸载）
./scripts/cargo_headroom.sh test --all-targets # 完整测试运行（带磁盘预检）
# Lint 检查（远程安全拆分以避免 rch clippy 超时失败开放）
./scripts/cargo_headroom.sh clippy --lib --bins -- -D warnings
./scripts/cargo_headroom.sh clippy --tests -- -D warnings
./scripts/cargo_headroom.sh clippy --benches -- -D warnings
./scripts/cargo_headroom.sh clippy --examples -- -D warnings
```

当这些变量未设置时，`cargo_headroom.sh` 将 `CARGO_TARGET_DIR` 和 `TMPDIR` 默认为 `/data/tmp/pi_agent_rust_cargo/...` 下的每个智能体目录，写入 `CACHEDIR.TAG`，拒绝意外的仓库根目标目录，并在目标或临时挂载空间不足时在编译前失败。设置 `PI_CARGO_RUNNER=local` 进行仅本地运行，`PI_CARGO_BUILD_ROOT=<dir>` 用于不同的大容量卷，或 `PI_CARGO_HEADROOM_MIN_FREE_MB=<mb>` 用于更小的针对性检查。

在启动 swarm 或重量级全目标门禁之前，运行 `pi doctor --only swarm --format json`。`pi.doctor.swarm_resource_preflight.v1` 结果在 `CARGO_TARGET_DIR` 或 `TMPDIR` 无法证明足够的暂存余量时失败关闭，其 `recommended_budgets` 对象给出从有效 cgroup CPU、cpuset、NUMA 和内存限制派生的保守智能体、工具、扩展主机调用、RCH 扇出、队列深度和 RSS 预算。同一对象包括预算说明、本地 cargo/rustc 压力和可回放 RCH 队列姿态。对于确定性回放，提供 `PI_DOCTOR_LOCAL_BUILD_PROCESS_COUNT`、`PI_DOCTOR_RCH_QUEUE_JSON` 或 `PI_DOCTOR_RCH_QUEUE_JSON_PATH`；这些输入是咨询性预算控制，而非面向发布版的性能声明。同一检查结果还包括 `lane_placement`（`pi.doctor.swarm_lane_placement.v1`），它将当前 cpuset/NUMA 拓扑分组为只读操作员通道，附带 CPU 亲和性提示、`/data/tmp/pi_agent_rust_cargo/<agent>/` 下的每通道 `CARGO_TARGET_DIR`/`TMPDIR` 根目录，以及最大智能体/工具/主机调用/RCH 扇出建议。Doctor 报告注意事项，如未知 NUMA 数据、部分 cpuset、紧张的内存限制或 RCH 队列压力，但它从不固定进程或改变 OS/RCH 状态。

当 `PI_VALIDATION_BROKER_STORE` 指向一个验证 broker 槽位 JSONL 存储时，Doctor 还会输出 `pi.doctor.validation_broker_posture.v1`，包含 runpack 和操作员交接的咨询性槽位姿态。缺失的 broker 配置报告为可选且非阻塞；过时或降级的 broker 存储保持可见而非被提升为绿色验证证据。

### Cargo 特性默认值

默认构建仅启用 `sqlite-sessions`。因此 `sqlite-sessions` 特性在普通 `cargo build`、`cargo test`、发布和安装程序构建中开启；JSONL 仍然是默认会话存储，除非配置选择 SQLite 存储。重量级额外功能（`image-resize`、`jemalloc`、`clipboard`、`wasm-host` 和语法高亮）是 opt-in 的，以使默认发布二进制文件保持在大小预算内。要构建所有可选的面向用户的额外功能，请使用：

```bash
./scripts/cargo_headroom.sh build --features full
```

要构建不带 SQLite 会话后端支持的更小自定义子集，使用 `--no-default-features` 并显式重新启用你需要的特性：

```bash
./scripts/cargo_headroom.sh build --no-default-features --features clipboard
```

### 测试

```bash
# 统一验证运行器（推荐用于确定性证据制品）
./scripts/e2e/run_all.sh --profile focused
./scripts/e2e/run_all.sh --profile ci
./scripts/e2e/run_all.sh --rerun-from tests/e2e_results/<timestamp>/summary.json --skip-unit

# 快速冒烟/扩展质量包装器，带严格远程执行
./scripts/smoke.sh --require-rch
./scripts/ext_quality_pipeline.sh --require-rch

# 多智能体安全：设置 CODEX_THREAD_ID 后，run_all 默认
# 将 CARGO_TARGET_DIR 设置为 target/agents/<CODEX_THREAD_ID>，除非被覆盖。
# 如果你想要自定义共享或隔离目标，请显式设置 CARGO_TARGET_DIR。

# 所有测试
./scripts/cargo_headroom.sh test --all-targets

# 特定模块
./scripts/cargo_headroom.sh test tools::tests
./scripts/cargo_headroom.sh test sse::tests

# 一致性测试
./scripts/cargo_headroom.sh test conformance
```

针对性验证工具：

```bash
# 发布构建前的开发者优先集门禁
rch exec -- cargo build --bin pi --bin ext_release_binary_e2e
PI_HTTP_REQUEST_TIMEOUT_SECS=0 rch exec -- \
  cargo run --example ext_release_binary_e2e -- \
  --pi-bin target/debug/pi \
  --provider ollama --model qwen2.5:0.5b \
  --jobs 10 --timeout-secs 600 --max-cases 20 --extension-policy balanced

# 门禁通过后的完整优化发布二进制运行
rch exec -- cargo build --release --bin pi --bin ext_release_binary_e2e
PI_HTTP_REQUEST_TIMEOUT_SECS=0 target/release/ext_release_binary_e2e \
  --pi-bin target/release/pi \
  --provider ollama --model qwen2.5:0.5b \
  --jobs 10 --timeout-secs 600 --extension-policy balanced

# 运行时风险账本取证（验证、重放、校准）
rch exec -- cargo run --example ext_runtime_risk_ledger -- verify --input path/to/runtime_risk_ledger.json
rch exec -- cargo run --example ext_runtime_risk_ledger -- replay --input path/to/runtime_risk_ledger.json
rch exec -- cargo run --example ext_runtime_risk_ledger -- calibrate --input path/to/runtime_risk_ledger.json --objective balanced_accuracy
```

- `ext_runtime_risk_ledger` 操作 `pi.ext.runtime_risk_ledger.v1` 制品（例如来自事件包导出）。

### 发布与发布管理

版本由标签驱动，必须与 `Cargo.toml` 版本一致。

- 标签格式：`vX.Y.Z`（预发布如 `vX.Y.Z-rc.N` 允许但跳过 crates.io 发布）。
- 标签版本**必须**匹配 `Cargo.toml` 中的 `package.version`。
- 依赖的发布顺序：`asupersync` → `rich_rust` → `charmed-*`（lipgloss、bubbletea、bubbles、glamour）→ `pi_agent_rust`。
- `.github/workflows/publish.yml` 在设置 `CARGO_REGISTRY_TOKEN` 时处理 crates.io 发布。

### 覆盖率

覆盖率使用 `cargo-llvm-cov`：

```bash
# 一次性安装
cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview

# 摘要（最快）
cargo llvm-cov --all-targets --workspace --summary-only

# LCOV 报告（用于 CI/制品）
CI=true VCR_MODE=playback VCR_CASSETTE_DIR=tests/fixtures/vcr \
  cargo llvm-cov --all-targets --workspace --lcov --output-path lcov.info

# HTML 报告（默认 target/llvm-cov/html）
cargo llvm-cov --all-targets --workspace --html
```

### 项目结构

选定的核心模块（非穷举）：

```
src/
├── main.rs                # CLI 入口点
├── lib.rs                 # 库导出
├── app.rs                 # 启动/模型选择辅助
├── agent.rs               # 智能体循环 + 事件编排
├── agent_cx.rs            # asupersync 能力上下文接线
├── cli.rs                 # 参数解析
├── config.rs              # 配置
├── auth.rs                # API 密钥/OAuth/AWS 凭据存储
├── model.rs               # 消息/内容/流事件类型
├── provider.rs            # Provider trait
├── provider_metadata.rs   # 规范 provider ID + 路由默认值
├── models.rs              # 模型注册表 + models.json 覆盖
├── providers/
│   ├── anthropic.rs        # Anthropic Messages API
│   ├── openai.rs           # OpenAI Chat Completions
│   ├── openai_responses.rs # OpenAI Responses API
│   ├── gemini.rs           # Gemini API
│   ├── cohere.rs           # Cohere Chat API
│   ├── azure.rs            # Azure OpenAI
│   ├── bedrock.rs          # Amazon Bedrock Converse
│   ├── vertex.rs           # Google Vertex AI
│   ├── copilot.rs          # GitHub Copilot 后端
│   ├── gitlab.rs           # GitLab Duo 后端
│   └── mod.rs              # Provider 工厂 + 扩展桥接
├── tools.rs                # 内置工具实现
├── sse.rs                  # 流式 SSE 解析器
├── http/
│   ├── client.rs           # 基于 asupersync 的 HTTP 客户端
│   ├── sse.rs              # HTTP SSE 辅助
│   └── mod.rs
├── session.rs              # JSONL 会话持久化/树操作
├── session_index.rs        # SQLite 会话元数据索引/缓存
├── session_sqlite.rs       # 默认启用的 sqlite-sessions 后端支持
├── compaction.rs           # 上下文压缩算法
├── interactive.rs          # 交互式 TUI 应用循环/状态
├── interactive/            # Bubble Tea 风格 TUI 子模块
├── rpc.rs                  # RPC/stdio 模式
├── extensions.rs           # 扩展协议 + 策略 + 安全
├── extensions_js.rs        # QuickJS 运行时桥接 + 主机调用
├── extension_dispatcher.rs # 主机调用/工具调度管道
├── extension_preflight.rs  # 扩展兼容性扫描器
├── extension_validation.rs # 扩展验证管道胶水
├── resources.rs            # 技能/提示词/主题/扩展加载
└── tui.rs                  # 终端 UI 渲染辅助
```

---

## 文档索引

本索引有意列出承载性文档，而非 `docs/` 下的每个生成快照。历史规划和审查材料位于[规划存档](docs/planning/)中；更广泛的清单参见[文档分类记录](docs/docs-classification-bd-we34i1.md)。

| 领域 | 主要文档 |
|---|---|
| 入门与操作 | [开发](docs/development.md)、[终端设置](docs/terminal-setup.md)、[设置](docs/settings.md)、[模型](docs/models.md)、[快捷键](docs/keybindings.md)、[包](docs/packages.md)、[故障排除](docs/troubleshooting.md)、[发布](docs/releasing.md) |
| Swarm 操作与回放 | [操作手册](docs/swarm-operations-runbook.md)、[回放操作员工作流](docs/swarm-replay-operator-workflow.md)、[活动账本](docs/swarm-activity-ledger.md)、[飞行记录器](docs/swarm-flight-recorder.md) |
| 核心运行时表面 | [会话](docs/session.md)、[树](docs/tree.md)、[TUI](docs/tui.md)、[RPC](docs/rpc.md)、[SDK](docs/sdk.md)、[技能](docs/skills.md)、[提示词模板](docs/prompt-templates.md)、[流式主机调用](docs/streaming-hostcalls.md)、[上下文智能](docs/context-intelligence.md) |
| Drop-in 认证与迁移 | [认证合同](docs/contracts/dropin-certification-contract.json)、[认证判定](docs/evidence/dropin-certification-verdict.json)、[对等差距账本](docs/evidence/dropin-parity-gap-ledger.json)、[差异证据套件](docs/evidence/dropin-differential-evidence-suite.json)、[特性清单](docs/evidence/dropin-feature-inventory-matrix.json)、[迁移剧本](docs/integrator-migration-playbook.md)、[对等快照](docs/parity-certification.json)、[项目管理](docs/program-governance.md) |
| 扩展 | [架构](docs/extension-architecture.md)、[兼容性指南](docs/ext-compat.md)、[兼容性矩阵](docs/extension-compatibility-matrix.md)、[一致性测试计划](docs/extension-conformance-test-plan.json)、[运行时威胁模型](docs/extension-runtime-threat-model.md)、[故障排除](docs/extension-troubleshooting.md)、[注册表](docs/extension-registry.md)、[WIT ABI](docs/wit/extension.wit) |
| Provider | [provider 指南](docs/providers.md)、[认证故障排除核对表](docs/provider-auth-troubleshooting.md)、[配置示例](docs/provider-config-examples.md)、[规范 ID 策略](docs/provider-canonical-id-policy.md)、[接入剧本](docs/provider-onboarding-playbook.md)、[测试义务](docs/provider-test-obligations.md)、[上游目录快照](docs/provider-upstream-catalog-snapshot.md)、[支持基线审计](docs/provider-support-baseline-audit.md) |
| QA 与证据 | [QA 手册](docs/qa-runbook.md)、[测试策略](docs/testing-policy.md)、[一致性操作员剧本](docs/conformance-operator-playbook.md)、[覆盖率矩阵](docs/TEST_COVERAGE_MATRIX.md)、[证据模式](docs/evidence-contract-schema.json)、[覆盖率基线图](docs/coverage-baseline-map.json)、[E2E 场景矩阵](docs/e2e_scenario_matrix.json)、[无模拟评分标准](docs/non-mock-rubric.json) |
| 安全 | [基线审计](docs/security/baseline-audit.md)、[威胁模型](docs/security/threat-model.md)、[安全不变量](docs/security/invariants.md)、[操作员手册](docs/security/operator-handbook.md)、[操作员快速参考](docs/security/operator-quick-reference.md)、[事件响应](docs/security/incident-response-runbook.md)、[运行时主机调用遥测](docs/security/runtime-hostcall-telemetry.md)、[安全 SLO](docs/security/security-slos.md) |
| 模式与机器合同 | [扩展清单模式](docs/schema/extension_manifest.json)、[扩展协议模式](docs/schema/extension_protocol.json)、[会话存储 v2 合同](docs/schema/session_store_v2_contract.json)、[语义工作区图合同](docs/contracts/semantic-workspace-graph-contract.json)、[语义上下文图合同](docs/contracts/semantic-context-graph-contract.json)、[swarm 回放跟踪合同](docs/contracts/swarm-replay-trace-contract.json)、[swarm 回放预览模式](docs/schema/swarm_replay_preview.json)、[远程验证证明账本合同](docs/contracts/remote-validation-proof-ledger-contract.json)、[远程验证证明重用门禁合同](docs/contracts/remote-validation-proof-reuse-gate-contract.json)、[验证证明内存索引合同](docs/contracts/validation-proof-memory-index-contract.json)、[验证证明内存索引证据](docs/evidence/validation-proof-memory-index.json)、[操作员工作推荐合同](docs/contracts/operator-work-recommendation-contract.json)、[操作员工作推荐证据](docs/evidence/operator-work-recommendation.json)、[操作员流畅度 SLO 合同](docs/contracts/operator-smoothness-slo-contract.json)、[操作员流畅度 SLO 证据](docs/evidence/operator-smoothness-slo.json)、[扩展资源防火墙矩阵合同](docs/contracts/extension-resource-firewall-matrix-contract.json)、[验证 broker 合同](docs/contracts/validation-broker-contract.json)、[验证 broker 关闭门禁合同](docs/contracts/validation-broker-closeout-gate-contract.json)、[验证 broker 关闭门禁证据](docs/evidence/validation-broker-closeout-gate.json)、[上下文智能关闭门禁合同](docs/contracts/context-intelligence-closeout-gate-contract.json)、[上下文智能关闭门禁证据](docs/evidence/context-intelligence-closeout-gate.json)、[进度 SLO 关闭门禁合同](docs/contracts/swarm-progress-slo-closeout-gate-contract.json)、[进度 SLO 关闭门禁证据](docs/evidence/swarm-progress-slo-closeout-gate.json)、[运行时智能关闭门禁合同](docs/contracts/runtime-intelligence-closeout-gate-contract.json)、[运行时智能关闭门禁证据](docs/evidence/runtime-intelligence-closeout-gate.json)、[第六波验证加固关闭门禁合同](docs/contracts/sixth-wave-validation-hardening-closeout-gate-contract.json)、[第六波验证加固关闭门禁证据](docs/evidence/sixth-wave-validation-hardening-closeout-gate.json)、[第七波运行时自治关闭门禁合同](docs/contracts/seventh-wave-runtime-autonomy-closeout-gate-contract.json)、[第七波运行时自治关闭门禁证据](docs/evidence/seventh-wave-runtime-autonomy-closeout-gate.json)、[预测 swarm 遥测账本合同](docs/contracts/predictive-swarm-telemetry-ledger-contract.json)、[预测 swarm 遥测账本证据](docs/evidence/predictive-swarm-telemetry-ledger.json)、[测试证据日志合同](docs/schema/test_evidence_logging_contract.json)、[运行时主机调用遥测模式](docs/schema/runtime_hostcall_telemetry.json)、[模拟规范模式](docs/schema/mock_spec.json)、[CLI 表面差异模式](docs/schema/cli-surface-diff.json)、[安全可追溯性矩阵](docs/sec_traceability_matrix.md) |

---

## 关于贡献

请不要误会，但我不接受对我任何项目的外部贡献。我实在没有精力审查任何东西，而且这是我的名字在上面，所以我对其造成的任何问题负责；因此，从我的角度来看，风险回报是高度不对称的。我还得担心其他"利益相关者"，这对于我主要是为了自己免费使用的工具来说似乎不明智。欢迎提交 issue，甚至 PR 如果你希望说明一个建议的修复，但要知道我不会直接合并它们。相反，我会让 Claude 或 Codex 通过 `gh` 审查提交，并独立决定是否以及如何解决。特别是 bug 报告，欢迎。如果这冒犯了你，我很抱歉，但我希望避免浪费时间和伤害感情。我理解这与寻求社区贡献的主流开源精神不符，但这是我能够保持如此速度并保持理智的唯一方式。

---

## 许可证

MIT 许可证（附带 OpenAI/Anthropic Rider）。详见 [LICENSE](LICENSE)。

---

<p align="center">
  <sub>用 Rust 构建，献给活在终端中的开发者。</sub>
</p>
