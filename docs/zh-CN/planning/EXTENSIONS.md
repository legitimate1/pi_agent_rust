# 扩展系统（Big-Guns 方案）

本文档定义 **pi_agent_rust** 的扩展架构，目标是**最大兼容性**、**形式化安全保障**与**可度量性能**。系统默认为 **尽力而为（best-effort）**，但设计上会逐步收敛至与旧版 Pi 扩展的完全对等。

---

## 0. 设计目标

1. **兼容性**：以尽力而为的保真度运行旧版 Pi 扩展。
2. **性能**：每次工具调用 p95 额外开销 < 2ms（不含工具本身耗时）。
3. **安全性**：显式、可审计的能力授予，支持可选的严格模式。
4. **稳定性**：版本化协议 + 一致性夹具。
5. **可移植性**：同一制品可在 Linux/macOS/Windows 上运行。

非目标：
- 来自扩展的自定义 TUI 渲染（UI 由核心拥有）。
- Node 原生 addon（必须使用宿主调用或 WASM）。

---

## 1. 运行时层级（混合、集众家之长）

**层级 A — WASM 组件（默认）：**
- 快速、沙箱化、可移植。
- 通过 WIT 的类型化宿主调用。

**层级 B — JS 兼容（已编译）：**
- 旧版 TS/JS 编译为单一 bundle。
- 预编译为 **QuickJS 字节码**或 **JS→WASM**。
- 无需 Node 运行时。

**层级 C — MCP（进程 IPC）：**
- 用于重型集成：IDE、数据库、云服务。

> WASM 为默认。JS 兼容是一个**编译步骤**，而非运行时。

---

## 1A. 无 Node/Bun 运行时：连接器 + 事件循环

Mario 的批评在狭义上是正确的：**QuickJS 仅仅是一个 JS 引擎**。它有意**不**提供 Node/Bun 风格的 OS API 表面或完整的通用事件循环。

我们的回答是：**很好**——我们不想要 Node/Bun 的表面积。

相反，Pi 提供了一组精简、能力门控的**连接器层**和一个显式事件循环，该循环*专为 Pi 扩展需求量身定制*（而非整个 web/Node 生态）。

### 1A.1 “连接器”模型（最小 OS 表面）

扩展不获得裸 OS 访问（没有 `fs`、没有 `child_process`、没有任意 socket）。它们获得**少量宿主调用**，映射到 Pi 已审计的操作（工具 + 会话/UI 动作）。

核心示例（名称仅为示意）：
- `pi.tool(name, input)` → 委托给内置工具注册表（read/write/edit/bash/grep/find/ls/hashline_edit）
- `pi.exec(command, args, options)` → 受约束的进程执行器（超时 + 进程树清理）
- `pi.fs.*` → *能力文件系统*，以项目/cwd 为根（不可路径逃逸）
- `pi.http(request)` → 受策略控制的受约束 HTTP 客户端
- `pi.session.*`、`pi.ui.*`、`pi.events.*` → Pi 内部 API（不暴露 OS）

这严格小于 Node/Bun，且是可审计的：每次连接器调用都是一次显式、已记录的能力检查。

### 1A.2 事件循环桥接（无 Node 的 Promise）

QuickJS 支持 Promise/microtask；只是需要宿主来**驱动**它们。

我们提供一个精简的“Pi 事件循环”：
- 排空 QuickJS 任务队列（microtask）
- 轮询未完成的宿主操作（通过 tokio/asupersync 的 Rust future）
- 解析/拒绝对应的 JS Promise
- 重复直到空闲（或直到 deadline/timer 触发）

换言之：Node 的事件循环是一个*产品*；我们的是一个*证明义务*：它仅实现 Pi 所需的部分，并带有确定性测试钩子。

### 1A.3 为何这样更好（安全 + 性能）

**安全性：** Node/Bun 默认暴露巨大的环境授权表面。我们的连接器层按构造就是基于能力且范围狭窄的。

**性能：** Node/Bun 为大规模兼容性付出了启动/内存成本。我们将 JS 预编译为字节码（或 WASM），运行时仅包含：1）JS 引擎 + 2）小型调度器 + 3）我们的连接器。

**确定性：** 借助 asupersync（LabRuntime），我们可以确定性地测试扩展异步 + 时间（无“真实时间”抖动）。

### 1A.4 PiJS 运行时契约（规范性）

本节定义在**无需 Node/Bun**的情况下运行 JS/TS 扩展的**权威 PiJS 运行时契约**，具备**确定性、可测试的事件循环**与显式、能力门控的宿主调用表面。

本契约是 JS 运行时、调度器、宿主调用桥接与测试工具链工作流的参考。

#### 1A.4.1 假设 / 约束

- **假设 QuickJS 没有 WebAssembly**：任何期望 `globalThis.WebAssembly` 的 JS bundle 必须使用 PiWasm 桥接（或层级 A 的 WASM 组件）。
- 无环境 OS API：所有副作用必须流经连接器调度器（能力检查 + 结构化审计日志）。
- PiWasm 导入链接为 fail-closed：模块仅接收在 `src/pi_wasm.rs` 中实现的宿主导入。不支持的函数/表/全局/内存导入会在实例化期间失败。默认返回的兼容存根仅限于 `COMPAT_STUB_IMPORTS` 中显式的 Emscripten 名称，且不授予宿主文件系统、网络或进程权限。

#### 1A.4.2 定义（术语）

- **Microtask**：QuickJS 任务队列（Promise 反应、`queueMicrotask`）。
- **Macrotask**：宿主驱动的任务（定时器、入站扩展事件、宿主调用完成）。
- **Tick**：一个确定性调度步骤，至多运行**一个** macrotask 并完成一次 microtask 排空。
- **宿主调用**：JS 向宿主发起的带副作用请求，在协议层面表示为 `host_call` / `host_result`（见 §3.2）。

#### 1A.4.3 模块 / 制品加载器契约

##### 制品输入

PiJS 执行由 `extc`（编译流水线）从**固定源**（见 `docs/extension-sample.json`）生成的扩展制品。

编译输出必须：
- 确定性（相同输入下逐字节稳定）
- 在 PiJS 内可 ESM 解析
- sourcemap 正确（运行时错误映射到原始 TS/JS）

##### 允许的说明符与解析

PiJS 模块解析器必须：
- 将 Node 内置规范化为 `node:*`（`fs`→`node:fs`，`path`→`node:path` 等）
- 优先解析虚拟内置（`node:*` 垫片 + Pi 运行时虚拟模块）
- 按此确定性顺序解析文件支持的模块：
  1. 精确文件路径
  2. 目录索引（`index.ts`、`index.tsx`、`index.js`、`index.mjs`、`index.json`）
  3. 扩展名回退（`.ts`、`.tsx`、`.js`、`.mjs`、`.json`，视情况而定）
- 接受相对（`./`、`../`）、绝对（`/...`）与 `file://...` 说明符
- 拒绝裸包说明符（PiJS 中无 `node_modules` 遍历）
- 拒绝网络导入（`http:` / `https:`）与其他环境加载器

确定性的不支持情况错误：
- 包导入：`Package module specifiers are not supported in PiJS: <specifier>`
- 网络导入：`Network module imports are not supported in PiJS: <specifier>`
- 其他不支持形式：`Unsupported module specifier: <specifier>`

`node:module.createRequire()` 目前仅支持 PiJS 暴露的 Node 内置，并有意拒绝包/本地文件系统解析。

##### 初始化契约

- 宿主加载制品入口模块。
- 入口模块必须导出一个**默认函数**，接收宿主提供的 `pi` 对象（扩展 API 表面）。
- 加载/初始化期间抛出的任何错误必须映射为带 sourcemap 位置的扩展错误，并作为结构化日志事件发出。

#### 1A.4.4 `pi` API 契约（面向 JS）

提供给扩展的 `pi` 对象是单一环境授权。它在内部必须经过能力门控。

##### 注册表面（面向协议）

至少（形态可能遵循旧版 API）：
- `pi.registerTool(spec)`
- `pi.registerSlashCommand(spec)`
- `pi.on(event_name, handler)` 用于生命周期/工具调用钩子

语义：
- 注册对 `(extension_id, name)` 必须幂等。
- 非法规约必须快速失败并给出可操作错误。
- 注册控制宿主为该扩展宣告/分发的内容。

##### 连接器表面（面向宿主调用）

至少：
- `pi.tool(name, input) -> Promise<ToolOutput>`
- `pi.exec(cmd, args, options) -> Promise<{ stdout, stderr, exitCode }>`
- `pi.http(request) -> Promise<response>`
- `pi.session.*` 访问器/变更，按协议定义
- `pi.ui.*` 原语（select/input/confirm/editor），在非交互模式下可被拒绝
- `pi.log(level, event, data)` 用于扩展作者的日志

规则：
- 每个连接器方法映射到一个带 `call_id`、能力、方法、参数与超时/取消元数据的 `host_call`（§3.2）。
- 每个连接器方法必须发出结构化审计日志（见 §3.1 / §3.4）。
- 错误必须映射到宿主调用错误分类（§3.2）：Denied/Timeout/IO/InvalidRequest/Internal。

##### 取消 + 超时

- 任何异步连接器调用可接受 `AbortSignal`；取消必须映射到宿主调用 cancel-token 语义。
- 超时必须在调度器中强制执行；JS 收到确定性的 Timeout 错误。

#### 1A.4.5 PiJS 事件循环：形式化状态机

##### 状态

将运行时状态定义为：

- `seq: u64` 单调计数器（全序 tie-breaker）
- `Q_micro`：QuickJS 任务队列（引擎内部；宿主可排空）
- `Q_macro`：macrotask 的 FIFO 队列，每个带入队 `seq`
- `Q_timer`：以 `(deadline_ms, seq)` 为键的定时器最小堆
- `clock`：单调时间源（测试可注入）

每个 macrotask 为下列之一：
- `TimerFired(timer_id)`
- `HostcallComplete(call_id, outcome)`
- `InboundEvent(event_id, payload)`（tool_call、slash_command、生命周期钩子、UI 响应等）

##### `tick()` 算法（规范性）

`tick(state)` 在给定当前状态与新到达的宿主完成集合时必须是确定性的。

算法：
1) **摄入宿主完成**：自上次 tick 以来完成的宿主调用以确定性顺序键入队到 `Q_macro`。
   - 建议：按到达顺序使用单调计数器为每个完成分配入队 `seq`。
2) **移动到期定时器**：当 `Q_timer.min.deadline_ms <= clock.now_ms` 时，弹出定时器并将 `TimerFired` 入队到 `Q_macro`（保持 `(deadline_ms, seq)` 顺序）。
3) **运行一个 macrotask**：
   - 若 `Q_macro` 非空：弹出最小 `seq` 的 macrotask 并执行。
   - 否则：空闲（no-op）。
4) **排空 microtask 至不动点**：反复排空 QuickJS 任务队列直到为空。
5) 返回更新后的状态。

##### 不变量（必须成立）

- **I1（单 macrotask）：** 每个 tick 至多执行一个 macrotask。
- **I2（microtask 不动点）：** 任意 macrotask 之后，microtask 排空至空。
- **I3（稳定定时器）：** 相同 deadline 的定时器按递增 `seq` 顺序触发。
- **I4（无重入）：** 宿主调用完成不直接同步重入 JS；它们入队为 macrotask。
- **I5（全序）：** 所有外部可观测调度按 `seq` 排序（确定性 tie-break）。

##### 定时器契约

- `setTimeout(fn, ms)` 入队一个定时器 `(deadline_ms = clock.now_ms + ms, seq = next_seq())`。
- `clearTimeout(id)` 若待触发则移除。
- `setInterval` 可选，除非固定样本要求；若实现，必须以重复 `setTimeout` 的稳定排序来规约。

##### 宿主调用完成契约

- 每个宿主调用有稳定的 `call_id` 与（建议的）签发 `seq`。
- 完成入队必须是确定性的：
  - 生产环境：按完成到达排序，再用单调 `seq` 稳定化。
  - 测试中：完成顺序可由录制夹具/确定性运行时控制。

#### 1A.4.6 确定性契约

##### 我们的承诺

给定：
- 相同的制品字节 + 垫片版本
- 相同的初始状态
- 相同的入站事件序列（工具调用、生命周期事件、UI 响应）
- 相同的宿主调用结果序列（含入队顺序）
- 相同的时钟行为（或确定性时钟）

则：
- 已执行 macrotask 序列与 resulting 可观测输出（工具结果、日志、UI 提示）完全相同。

##### 证明思路（为何）

- 调度器是 `(state, arrivals)` 的纯函数，带全序 tie-breaker `seq`。
- 定时器排序通过 `(deadline_ms, seq)` 确定。
- 宿主调用完成排序按构造确定（完成入队 `seq`）。
- microtask 排空至不动点确保无隐藏交错。
- 因此，对 tick 归纳，整个执行轨迹在固定输入下是确定性的。

#### 1A.4.7 可观测性 / 追踪契约

- 每个 tick 与每次入队/出队事件可在 `pi.ext.log.v1` 下记录（debug 级），带 `trace_id` / `span_id` 与关联 id。
- 确定性测试运行必须能在 §3.1 的归一化规则后对轨迹判等。

---

## 1B. 扩展分类 + 兼容性矩阵（规范性）

本节定义我们支持的**规范扩展形态**，并将每种形态映射到其**入口/配置**与**所需宿主能力**。它是选择、一致性与文档工作的参考。

### 1B.1 扩展形态（规范）

**运行时扩展（可执行）：**
- **PiJS (JS/TS)** — 编译为 JS 的旧版扩展（层级 B）。
- **WASM 组件** — 基于 WIT 的组件（层级 A）。

**外部服务（进程外）：**
- **MCP server** — stdio/http/sse 工具服务（层级 C）。

**资源包（不可执行）：**
- **技能包** — 用于智能体行为的 `SKILL.md` 束。
- **提示模板** — `.md` 提示文件。
- **主题** — UI 的 `.json` 主题定义。

**束/包（分发）：**
- **包源** — 可能包含上述任意项的束（扩展、技能、提示、主题）。由 `src/package_manager.rs` 解析。

### 1B.2 形态矩阵（入口/配置 → 运行时 → I/O）

| 形态 | 入口 / 配置 | 运行时 | 主要 I/O 表面 | 备注 |
|---|---|---|---|---|
| **PiJS 扩展** | `extension.json`（`pi.ext.manifest.v1`）或列出 `extensions` 的包清单；入口 `.ts`/`.js` | QuickJS + Pi 事件循环 | `register` + `host_call`/`host_result` | 旧版 TS/JS 已编译并加垫片；无 Node/Bun。 |
| **WASM 组件** | `extension.json` 含 `runtime="wasm"`；入口 `.wasm` 组件 | Wasmtime（组件模型） | WIT 宿主调用 → `host_call`/`host_result` | 通过 WIT 的类型化宿主调用。 |
| **MCP server** | MCP 配置（`*.json`）或 CLI 参数 | 外部进程 / 远程服务 | MCP 协议（stdio/http/sse） | 非扩展协议；由连接器策略门控。 |
| **技能包** | `SKILL.md` + 可选资源 | 无（资源） | 仅文件加载 | 注入到提示上下文；无宿主调用。 |
| **提示模板** | `.md` 提示文件 | 无（资源） | 仅文件加载 | 供 `/template` 调用使用。 |
| **主题** | `.json` 主题文件 | 无（资源） | 仅文件加载 | 供 TUI 渲染器使用。 |
| **包源** | `package.json` / 含资源的包清单 | 混合 | 取决于所含资源 | 可能包含扩展 + 技能 + 提示 + 主题。 |

### 1B.3 能力矩阵（注册类型 → 所需能力）

**能力始终从宿主调用派生**（永不信任扩展声明），但注册类型暗示典型的能力使用：

| 注册类型 | 协议表面 | 典型宿主调用 | 派生能力 | 备注 |
|---|---|---|---|---|
| **工具**（`registerTool`） | `register` → `tool_call`/`tool_result` | `pi.tool(...)` / `pi.exec(...)` | `read` / `write` / `exec` / `tool` | `read/write/exec` 按工具名派生；未知工具映射到 `tool`。 |
| **斜杠命令**（`registerCommand`） | `register` → `slash_command`/`slash_result` | `pi.ui.*`、`pi.session.*`、可选 `pi.exec` | `ui` / `session` / `exec` | 命令由 UI 驱动；exec 可选。 |
| **事件钩子**（`event_hook`） | `register` → `event_hook` | `pi.session.*`、`pi.ui.*`、`pi.exec`、`pi.http` | `session` / `ui` / `exec` / `http` | 能力取决于事件处理行为。 |
| **提供方**（`registerProvider`） | `register` + 流式钩子 | `pi.http(...)` | `http`（本地文件则 + `read`） | 提供方需要网络；若使用 API 密钥则记录为 `env`。 |
| **标志**（`registerFlag`） | 仅 `register` | 使用前无 | 无（注册时） | 标志为配置；能力由后续行为驱动。 |
| **快捷键**（`registerShortcut`） | 仅 `register` | 激活时 `pi.ui.*` | `ui` | 快捷键为 UI 级触发器。 |

**不可执行的资源包（技能/提示/主题）**不调用宿主调用，因此除文件加载外**无运行时能力需求**。

---

## 1C. 生态研究与候选池（资料性）

本节是扩展兼容性工作的**研究基础**。它记录**候选来源**、**验证方式**与我们跟踪的**规范元数据**，以便下游 bead 无需重复发现即可进行排序/选择。

### 1C.1 来源层级（候选来自何处）

我们按**来源层级**分类候选（非运行时层级）：

- `official-pi-mono` — 规范上游语料库（“官方 60”及任意额外固定的上游示例）。
- `community` — 小型社区仓库/gist；常为单文件扩展。
- `third-party-github` — 较大的第三方仓库（可能多文件）。
- `npm-registry` — 包含 Pi 扩展的已发布包。
- `agents-mikeastock` — 特殊精选语料（保留为独立层级以便溯源推理）。
- `non-conformance` — 有趣但明确超出对等范围（仅用于研究/分流）。

上述层级标签与 `docs/` 中的静态扫描与主目录制品一致（见 §1C.4）。

### 1C.2 发现工作流（可重复 + 基于证据）

**权威发现源（v1，按序）：**

1. **上游 pi-mono**（`badlogic/pi-mono`）扩展示例语料（规范“官方”参考集）。
2. **精选语料快照**检入本仓库（如 `legacy_pi_mono_code/` 下），用于确定性扫描与可重复一致性运行。
3. **GitHub 发现扫描**（关键词 + 主题搜索）→ 候选仓库与原始文件（由研究 bead 跟踪）。
4. **npm registry 扫描**（关键词搜索 + 反向依赖路径）→ 候选包与 tarball（由研究 bead 跟踪）。
5. **市场/注册表**（适用时）如 OpenClaw/ClawHub 清单（由研究 bead 跟踪）。

我们将发现视为**流水线**，而非一次性搜索：

1. **枚举语料根**（按层级）：本地仓库快照、git 检出、npm 包 tarball。
2. **静态扫描**：
   - 查找候选入口（默认导出 / `register(...)` 模式）。
   - 记录“能力信号”（暗示宿主调用的导入/调用）。
   - 发出机器可读清单以便去重 + 分流。
3. **动态验证**（事实来源）：
   - 在 **pi-mono TS 运行时**（基于 Bun 的工具链）中加载每个候选。
   - 记录加载成功/失败、错误类别与注册输出。
   - 注意：动作方法可能在加载期间有意抛出；我们仅要求*注册*成功。
4. **合并 + 去重**为master候选池。
5. **富化 + 排序**（仅在池稳定后）：
   - 大小、文件数、依赖形态、IO 模式、流行度信号。
   - 生成分层执行计划（一致性排序、复杂度分桶）。

### 1C.3 候选身份与去重策略（确定性）

同一逻辑扩展可能通过多条路径出现（fork、镜像、供应商拷贝、npm 重打包）。我们使用**规范源键**与内容校验和去重：

- **规范源键**（稳定身份）：
  - Git：`git:<repo_url>#<path>`
  - npm：`npm:<package_name>@<version>#<path>`（未知时可省略 `@<version>`）
  - 本地快照：`local:<absolute_path>`
- **内容校验和**（稳定内容）：`sha256(file_bytes)`（单文件）或 `sha256(concat(sorted(file_checksums)))`（多文件目录）。

规则：
- 已知时优先使用上游规范 URL（避免按 fork 产生“新身份”）。
- 当两个候选共享校验和时，除非在动态验证下运行时行为不同，否则视为重复。
- 人类可读 `id` 尽可能稳定（清单 id 或文件名），但源键 + 校验和才是真实身份。

### 1C.4 规范制品（事实来源）

我们将研究输出保存在 `docs/` 中，以便可审查、可 diff、可被 CI/工具链使用：

- `docs/extension-entry-scan.json` — 静态扫描清单（入口 + 子模块 + 置信度 + 按层级统计）。
- `docs/extension-master-catalog.json` — 用于一致性的**去重后master池**（全层级、最小字段 + 校验和）。
- `docs/extension-catalog.json` — **完整已验证语料**的富化元数据（跨全来源层级的 223 个扩展，含一致性状态、能力、IO 模式、复杂度分桶、校验和与性能预算）。
- `docs/extension-catalog.schema.json` — `docs/extension-catalog.json` 的 JSON Schema（`pi.ext.catalog.v1`）。
- `docs/extension-priority.json` — 官方语料的排序/顺序计划（以可测试性优先的执行策略）。

下游 bead 应将这些作为输入，避免重复抓取/扫描，除非显式重建流水线。

#### 目录 Schema：`pi.ext.catalog.v1`

`docs/extension-catalog.json` 是**官方**扩展语料的富化元数据层。定义为：
- 版本标签：`schema: "pi.ext.catalog.v1"`（嵌入 JSON）
- 校验：`docs/extension-catalog.schema.json`

**顶层字段**
- `schema` *(string, const)*：schema 标识（`pi.ext.catalog.v1`）
- `generated_at` *(RFC3339 string)*：制品生成时间戳
- `total_extensions` *(int)*：目录条目数
- `items` *(array)*：目录条目（见下）
- `tier_summary` / `runtime_summary` *(object)*：聚合计数

**目录条目字段（v1 必需）**
- `id` *(string)*：稳定扩展标识
- `name` *(string)*：入口文件名（资料性）
- `source_tier` *(enum)*：溯源层级（official/community/npm 等）
- `source` *(union)*：固定源引用（`git`/`npm`/`url`）
- `runtime_tier` *(enum)*：打包形态分桶（`legacy-js`/`multi-file`/`pkg-with-deps`）
- `interaction_tags` *(enum[])*：工具/命令/事件/UI/提供方表面标签
- `capabilities` *(enum[])*：所需能力集合（read/write/http/exec/session/ui 等）
- `io_pattern` *(enum[])*：粗粒度 IO 行为分桶
- `complexity` *(enum)*：`small|medium|large`
- `file_count` / `total_bytes` *(int)*：制品大小元数据
- `checksum.sha256` *(hex string)*：稳定内容校验和

**保留字段（可选；由下游 bead 填充）**
- `version`：扩展版本（适用时；如 npm）
- `license`：许可证标识（`docs/extension-artifact-provenance.json`）
- `category_tags`：工作流标签（git/tests/devops 等）
- `compatibility_notes`：已知约束 / 警告原因（见 `docs/ext-compat.md`）
- `perf_budgets`：性能预期 + 已观测基线（bench 制品）

**映射 / 事实来源输入**
- 校验和 + 文件元数据：`docs/extension-master-catalog.json`
- 许可证 + 固定溯源：`docs/extension-artifact-provenance.json`
- Node API + 宿主调用使用：`docs/extension-api-matrix.json`
- 可测试性说明 + 执行顺序：`docs/extension-priority.json`

### 1C.5 覆盖目标与已达成结果

覆盖目标的目的是防止“高分短名单”遗漏整类真实行为。目标供选择 bead 生成大而分层、可辩护的 Tier-1“必须通过”语料。

**层级大小目标（选择约束）：**

- **Tier-0 基线：** 上游官方示例集（必须通过的基线）。
- **Tier-1 必须通过：** **≥ 200** 个未修改扩展，跨来源层级与行为分桶分层。
- **Tier-2 延伸：** 额外长尾扩展，主要为独特 API 表面/覆盖率而选（非流行度）。

**已达成覆盖（截至 2026-02-07）：**

全部 223 个已验证扩展均已测试。187 个通过（83.9%）。

| 来源层级 | 目标 | 实际 | 通过 | 通过率 |
|---|---:|---:|---:|---:|
| `official-pi-mono` | 60 | 61 | 60 | 98.4% |
| `npm-registry` | 50 | 75 | 48 | 64.0% |
| `community` | 50 | 58 | 52 | 89.7% |
| `third-party-github` | 20 | 23 | 16 | 69.6% |
| `agents-mikeastock` | all | 1 | 0 | 0% |
| **总计** | **≥ 200** | **223** | **187** | **83.9%** |

按一致性层级（复杂度分桶）：

| 层级 | 描述 | 总数 | 通过 | 通过率 |
|---|---|---:|---:|---:|
| T1 | 简单单文件 | 38 | 38 | 100% |
| T2 | 多注册 | 87 | 85 | 97.7% |
| T3 | 多文件 / 复杂 | 90 | 60 | 66.7% |
| T4 | npm 依赖 | 3 | 2 | 66.7% |
| T5 | exec/network | 5 | 2 | 40.0% |

**36 个失败分解为：**
- 清单注册不匹配（22）— 可通过审计清单修复
- 缺失 npm 包存根（5）— 可通过添加虚拟模块修复
- 多文件依赖（4）— 部分可修复（需打包）
- 运行时错误（4）— 需调查
- 测试夹具（1）— 非真实扩展

见 `tests/ext_conformance/reports/COMPATIBILITY_SUMMARY.md` 获取完整的一致性 + 性能合并报告。

**Tier-1 行为 / 能力配额（最小覆盖分桶）：**

注册 / 表面：
- **工具：** 包含所有注册工具的扩展（或随语料增长取 ≥ 60 的较大值）。
- **事件钩子：** 包含所有事件钩子扩展（或 ≥ 80）。
- **斜杠命令：** 包含 ≥ 25 个命令扩展。
- **提供方注册 / 流式：** 包含**全部**提供方注册扩展（稀有/高风险表面）。
- **UI 表面：** 包含 ≥ 15 个重 overlay 与 ≥ 40 个 UI 集成（header/footer/status/message-renderer）扩展。

宿主调用 / 能力风险：
- **Exec-heavy（`exec_api`）**：包含全部（能力高风险）。
- **Network-heavy（`http`）**：包含 ≥ 25。
- **FS-heavy（`read/write/edit`）**：包含 ≥ 50。
- **Session/UI heavy（`session_api` / `ui_*`）**：合计包含 ≥ 50。

**分类覆盖（用户工作流分桶）：**

在每个高价值工作流分类中至少保持少量 quorum：
- `git` / 仓库卫生 / 检查点
- `tests` / lint / format / CI
- `devops` / infra / 云工具
- `research` / search / summarization
- `codegen` / refactor / scaffolding
- `ui` / interaction / TUI 增强
- `security` / policy / guardrails

说明：
- 这些目标有意混合**硬性最小值**与“包含全部稀有”规则。对于稀有但关键的表面（提供方注册、exec-heavy），选择应偏向全覆盖而非抽样。
- `docs/extension-*.json` 制品是计数与分桶分类的度量来源。

## 2. 制品流水线（旧版 → 优化）

**输入**
- `extension.json`（清单）
- 源文件（TS/JS 或 Rust/WASM）

**流水线**
1. **SWC 构建**：TS/JS → bundle（tree-shaken/minified）。
2. **兼容性扫描**：对禁用 API 的静态分析。
3. **协议垫片**：将旧版扩展导入重写为宿主调用。
4. **制品构建**：
   - **QuickJS 字节码**（快速启动），或
   - **WASM 组件**（可移植 + 沙箱化）。
5. **按哈希缓存**：
   ```
   hash = sha256(manifest + bundle + engine_version)
   ```

**输出**
- `extension.artifact` + `artifact.json`（元数据、引擎、哈希、能力）

---

## 2A. Extc 兼容性契约（规范性）

本节定义 **extc 编译器契约**，将旧版 Node/Bun 导入映射到 PiJS 垫片，使 **`docs/extension-sample.json` 中的全部 16 个扩展无需修改即可运行**（无需手动源码编辑）。

### 2A.1 通用性约束（不可协商）

- **无按扩展例外**的 extc 重写。
- 重写必须仅基于导入说明符与通用、语义保持的代码模式定义。
- 若样本暴露缺口，应通过添加**通用规则 + 测试**修复，而非按扩展 id 分支。

### 2A.2 规范导入重写规则

Extc 必须确保每个导入说明符在 PiJS 内无需 Node/Bun 即可解析。

#### A) Node 内置（`node:*`）

将 `node:*` 内置重写到 PiJS 提供的内部命名空间（使打包器不将其 externalize）：

| 源说明符        | 目标说明符           |
|-------------------------|----------------------------|
| `node:fs`               | `pi:node/fs`               |
| `node:fs/promises`      | `pi:node/fs_promises`      |
| `node:path`             | `pi:node/path`             |
| `node:os`               | `pi:node/os`               |
| `node:url`              | `pi:node/url`              |
| `node:crypto`           | `pi:node/crypto`           |
| `node:child_process`    | `pi:node/child_process`    |
| `node:module`           | `pi:node/module`           |

#### B) 裸内置（无前缀）

许多真实依赖不带 `node:` 前缀导入内置。同样处理：

| 源说明符        | 目标说明符           |
|-------------------------|----------------------------|
| `fs`                    | `pi:node/fs`               |
| `fs/promises`           | `pi:node/fs_promises`      |
| `path`                  | `pi:node/path`             |
| `os`                    | `pi:node/os`               |
| `url`                   | `pi:node/url`              |
| `crypto`                | `pi:node/crypto`           |
| `child_process`         | `pi:node/child_process`    |
| `module`                | `pi:node/module`           |

### 2A.3 全局 Polyfill 注入

Extc 可在 bundle 入口注入幂等的前置导入：

```javascript
import 'pi:polyfills/node_globals'  // 安装 process、Buffer、__dirname、__filename
import 'pi:polyfills/fetch'         // 如需：fetch、Headers、Request、Response
import 'pi:polyfills/webassembly'   // PiWasm 桥接（QuickJS 无原生 wasm）
```

**注入规则：**
- **确定性**：稳定排序，始终在入口模块顶部。
- **sourcemap 正确**：注入的导入不得破坏 sourcemap 行映射。
- **已版本化**：`shim_version` 必须包含在制品哈希中。
- **幂等**：多次注入产生相同输出。

**`pi:polyfills/node_globals` 提供的 Node 全局：**
- `process`（含 `process.env`、`process.cwd()`、`process.platform` 等）
- `Buffer`
- `__dirname` / `__filename`（由模块 URL 计算）
- `global`（`globalThis` 别名）
- `setImmediate` / `clearImmediate`

### 2A.4 禁用与标记 API

兼容性扫描器必须将 API 分类为：

#### 禁用（硬错误）

绕过能力策略或逃逸沙箱的 API。Extc 必须拒绝使用这些的 bundle：

| API / 模式                        | 原因                                   |
|--------------------------------------|------------------------------------------|
| `require('vm')` / `node:vm`          | 任意代码执行                 |
| `require('worker_threads')`          | 不支持的并发模型            |
| `require('cluster')`                 | 不支持的并发模型            |
| `require('dgram')`                   | 裸 UDP socket                          |
| `require('net')`（裸 socket）       | 绕过 HTTP 策略                     |
| `require('tls')`（裸 socket）       | 绕过 HTTP 策略                     |
| `require('inspector')`               | 调试器访问                          |
| `require('perf_hooks')`              | 性能计时预言机                |
| `require('v8')`                      | 引擎内部                         |
| `require('repl')`                    | 交互式 eval                         |
| `process.binding()`                  | 原生模块访问                     |
| `process.dlopen()`                   | 原生 addon 加载                     |
| 直接 `eval()` 带动态字符串  | 任意代码执行（见说明）      |

**关于 `new Function(...)` 的说明：** 固定样本包含 `new Function(...)` 用于加载打包脚本。这**被标记但允许**，需证据记录，而非直接禁用。

#### 已标记（警告 + 证据）

需要证据记录但不阻塞编译的风险构造：

| API / 模式                        | 所需证据                        |
|--------------------------------------|------------------------------------------|
| `new Function(...)`                  | 记录函数体哈希 + 调用点       |
| `eval(variable)`                     | 若变量非字面量则记录         |
| `setTimeout(string, ...)`            | 记录字符串体哈希                     |
| `setInterval(string, ...)`           | 记录字符串体哈希                     |
| `Proxy` / `Reflect`（反射）     | 记录使用模式                        |
| `Object.defineProperty` 作用于内置  | 记录目标 + 属性                    |

### 2A.5 Extc 输入/输出契约

#### 输入

- 扩展清单（`extension.json` 或 `package.json`）
- 源文件（TypeScript 或 JavaScript）
- 可选：`tsconfig.json` 用于类型解析

#### 输出

- **ESM bundle**：单一入口模块，已 tree-shaken、已压缩
- **Sourcemap**：到原始源码的精确行/列映射
- **制品元数据**（`artifact.json`）：
  ```json
  {
    "schema": "pi.ext.artifact.v1",
    "extension_id": "...",
    "entry_module": "index.js",
    "hash": "sha256:...",
    "shim_version": "1.0.0",
    "rewrite_log": [
      { "from": "node:fs", "to": "pi:node/fs", "locations": [...] }
    ],
    "injected_polyfills": ["pi:polyfills/node_globals"],
    "flagged_apis": [
      { "api": "new Function", "locations": [...], "evidence_hash": "..." }
    ],
    "forbidden_apis": [],
    "capabilities_required": ["read", "exec"]
  }
  ```

- `capabilities_required` 必须按 §2B.3（声明 ∪ 推断）计算，顺序确定。

#### 副作用策略

- Extc 在编译期间不得执行扩展代码。
- 仅静态分析；不做触发副作用的 `require()` 解析。
- 若依赖无法静态分析，发出警告并原样包含（运行时将处理能力检查）。

### 2A.6 兼容性矩阵

以下 Node API 通过垫片支持。每个映射到带显式能力需求的 PiJS 连接器：

| Node API                 | 垫片模块             | 同步/异步 | 能力   | 备注                          |
|--------------------------|-------------------------|------------|--------------|--------------------------------|
| `fs.readFileSync`        | `pi:node/fs`            | 同步       | `read`       | 阻塞事件循环              |
| `fs.writeFileSync`       | `pi:node/fs`            | 同步       | `write`      | 阻塞事件循环              |
| `fs.promises.readFile`   | `pi:node/fs_promises`   | 异步      | `read`       | 推荐                      |
| `fs.promises.writeFile`  | `pi:node/fs_promises`   | 异步      | `write`      | 推荐                      |
| `fs.existsSync`          | `pi:node/fs`            | 同步       | `read`       |                                |
| `fs.readdirSync`         | `pi:node/fs`            | 同步       | `read`       |                                |
| `fs.statSync`            | `pi:node/fs`            | 同步       | `read`       |                                |
| `fs.mkdirSync`           | `pi:node/fs`            | 同步       | `write`      |                                |
| `path.join`              | `pi:node/path`          | 同步       | (无)       | 纯计算               |
| `path.resolve`           | `pi:node/path`          | 同步       | (无)       | 使用 `process.cwd()`           |
| `path.dirname`           | `pi:node/path`          | 同步       | (无)       | 纯计算               |
| `path.basename`          | `pi:node/path`          | 同步       | (无)       | 纯计算               |
| `path.extname`           | `pi:node/path`          | 同步       | (无)       | 纯计算               |
| `os.platform`            | `pi:node/os`            | 同步       | `env`        | 返回宿主平台          |
| `os.homedir`             | `pi:node/os`            | 同步       | `env`        | 返回家目录         |
| `os.tmpdir`              | `pi:node/os`            | 同步       | `env`        | 返回临时目录         |
| `child_process.spawn`    | `pi:node/child_process` | 异步      | `exec`       | 流式 stdout/stderr；支持 `timeout` |
| `child_process.exec`     | `pi:node/child_process` | 异步      | `exec`       | 缓冲输出；返回 `ChildProcess` |
| `child_process.execFile` | `pi:node/child_process` | 异步      | `exec`       | 直接命令执行；返回 `ChildProcess` |
| `child_process.execSync` | `pi:node/child_process` | 同步       | `exec`       | 阻塞；优先异步           |
| `child_process.execFileSync` | `pi:node/child_process` | 同步  | `exec`       | 直接命令执行       |
| `child_process.spawnSync` | `pi:node/child_process` | 同步      | `exec`       | 结构化结果对象       |
| `crypto.randomBytes`     | `pi:node/crypto`        | 同步       | (无)       | CSPRNG                         |
| `crypto.createHash`      | `pi:node/crypto`        | 同步       | (无)       | 纯计算               |
| `url.parse`              | `pi:node/url`           | 同步       | (无)       | 纯计算               |
| `url.URL`                | `pi:node/url`           | 同步       | (无)       | WHATWG URL                     |
| `process.env`            | `pi:polyfills/...`      | 同步       | `env`        | 按策略过滤             |
| `process.cwd()`          | `pi:polyfills/...`      | 同步       | (无)       | 项目根                   |
| `process.exit()`         | `pi:polyfills/...`      | 同步       | (无)       | 抛出；扩展不可退出  |
| `Buffer.from`            | `pi:polyfills/...`      | 同步       | (无)       | 二进制数据处理           |
| `Buffer.alloc`           | `pi:polyfills/...`      | 同步       | (无)       | 二进制数据处理           |
| `fetch`                  | `pi:polyfills/fetch`    | 异步      | `http`       | WHATWG Fetch                   |

**错误映射：** 所有垫片错误必须映射到宿主调用错误分类（§3.2）：`denied`、`timeout`、`io`、`invalid_request`、`internal`。

### 2A.7 Sourcemap 契约

Extc 必须生成满足以下要求的 sourcemap：

1. **精确映射**：每个生成行/列必须映射到正确的原始源码位置。
2. **经重写保持**：导入重写不得破坏映射。
3. **包含源码**：sourcemap 应包含 `sourcesContent` 以便离线调试。
4. **内联或外部**：同时支持内联（`//# sourceMappingURL=data:...`）与外部（`.map` 文件）格式。

**运行时使用：**
- 发生错误时，运行时必须使用 sourcemap 生成带原始文件/行/列的堆栈。
- 结构化日志（§3.1）必须在 `source.location` 中包含 sourcemap 后的位置。

### 2A.8 测试要求

- **单元转换夹具**：常见导入 + 注入模式及预期输出。
- **负向测试**：禁用 API 必须产生精确错误消息。
- **E2E 工具链**：验证重写后的 bundle 能以可操作的失败诊断运行 16/16 样本扩展。

---

## 2B. 扩展清单 + 能力推断（规范性）

本节定义：
- 磁盘上的**扩展清单**（`extension.json`），以及
- 工具如何从 bundle 确定性推导**所需能力**（能力推断）并与清单合并。

这是以下各方的契约：
- **extc**（编译器 + 兼容性扫描器）在制品构建期间（§2A），以及
- **运行时 + 工具链**在决定 prompt/deny 与校验一致性时。

### 2B.1 扩展清单（`extension.json`，`pi.ext.manifest.v1`）

**位置：** `<扩展根>/extension.json`

**回退：** 若缺失 `extension.json`，extc 可从 `package.json#pi` 读取相同 schema。此时 `name` / `version` 默认取顶层 `package.json` 字段，除非在 `pi` 内被覆盖。若两者都存在，`extension.json` 优先。

**规范化（v1）：**
- 清单哈希必须使用**规范 JSON**（UTF-8、无空白、对象键按字典序排序、数组保持顺序）。
- 流水线哈希（§2）在规范清单字节上计算。

**机器 schema：** `docs/schema/extension_manifest.json`

**Schema（v1）—— 人类可读形式：**
```json
{
  "schema": "pi.ext.manifest.v1",
  "extension_id": "ext.todo",
  "name": "Todo",
  "version": "0.1.0",
  "api_version": "1.0",
  "runtime": "native-rust",
  "entrypoint": "extension.native.json",

  "capabilities": ["read"],
  "capability_manifest": {
    "schema": "pi.ext.cap.v1",
    "capabilities": [
      { "capability": "read", "methods": ["tool"], "scope": { "paths": ["src/**"] } }
    ]
  }
}
```

字段：
- `schema`（必需）：必须为 `pi.ext.manifest.v1`。
- `extension_id`（必需）：用于日志（`ext.log.v1`）与工具链夹具的稳定标识。
- `name` / `version` / `api_version`（必需）：必须与协议 `register` 载荷（§3）一致。
- `runtime`（必需）：`native-rust` 或 `wasm`。
- `entrypoint`（必需）：相对于扩展根的路径：
  - Native Rust 运行时：描述符入口，如 `extension.native.json`。
  - WASM：组件制品路径，如 `dist/extension.wasm`。
- `capabilities`（可选，旧版）：在所有扩展发出作用域清单前的粗粒度能力集合。
- `capability_manifest`（可选，推荐）：使用 §3.3 中 `pi.ext.cap.v1` schema 的作用域需求。

### 2B.2 能力推断（`pi.ext.infer.v1`）

**目标：** 以可审计证据确定性推导制品*看似*需要的最小已知能力集合。

**输出：** 推断的 `pi.ext.cap.v1` 形态需求集及证据记录。推断集以以下形式写入 `artifact.json`：
- `capabilities_required`：能力键的稳定、已排序列表（`read`、`write`、`exec`、`http` 等），以及可选的
- `capability_scope_inferred`：当推断能提取稳定作用域（路径/主机）时的作用域清单。

**证据来源（v1，按序）：**
0) **配置文件（源码不可用时）：**
   - `package.json#pi.capabilities` 可视为粗粒度证据。
   - 依赖签名可用于粗粒度推断（如 `node-fetch`、`axios`、`undici` → `http`），`kind=config_hint`。
1) **导入说明符**（重写后）：
   - `pi:node/fs` / `pi:node/fs_promises` → 基于所用 API 推断 `read` 和/或 `write`（见下规则）。
   - `pi:node/child_process` → `exec`。
   - `pi:polyfills/fetch` 或 `fetch(` 使用 → `http`。
2) **PiJS 原语**：
   - `pi.tool("read"|"grep"|"find"|"ls", ...)` → `read`
   - `pi.tool("write"|"edit", ...)` → `write`
   - `pi.tool("bash", ...)` 或 `pi.exec(...)` → `exec`
   - `pi.http(...)` → `http`
3) **字面量作用域提示**（尽力而为）：
   - `read`/`write` 路径：看起来像相对路径的字符串字面量。
   - `http` 主机：解析为 URL 的字符串字面量；提取主机。

**推断规则（v1）：**
- 确定性：推断在各平台稳定；排序为：`capability` 升序，再 `method` 升序，再作用域排序。
- 可靠性目标：推断必须**保守**（允许过近似），但不得从非字面量源凭空发明作用域。动态值产生**未指定作用域**（按策略强制 prompt/deny）。
- JS vs WASM：能力名与作用域语义完全相同（§3.2A）。WASM 推断可基于：
  - 组件的静态分析（若可用），或
  - 捕获模式下观测到的 `host_call` 轨迹（为正确性首选）。

### 2B.3 合并策略（声明 ∪ 推断 + 用户覆盖）

定义：
- `declared`：若存在则来自 `extension.json.capability_manifest`，否则来自旧版 `extension.json.capabilities`（粗粒度）。
- `inferred`：来自推断引擎（§2B.2）。
- `overrides`：来自配置的用户策略覆盖（allow/deny/收窄作用域）。

**有效需求（v1）：**
1) 以 `declared ∪ inferred` 为起点（按键合并）。
2) 应用用户**拒绝**覆盖：
   - 移除能力是允许的；运行时宿主调用将返回 `denied`。
   - 收窄作用域是允许的；应用作用域交集。
   - 若**声明的**能力被拒绝：
     - `strict` → 注册失败
     - `prompt` → 需要用户决策
     - `permissive` → 允许但记录日志
3) 应用用户**允许**覆盖（添加能力 / 扩大作用域）。
4) 发出 `capability.resolve` 日志（见 §2B.5）含完整分解。

### 2B.4 校验（硬错误）

运行时/工具链在下列情况必须拒绝扩展清单：
- `schema` 未知。
- `name`/`version`/`api_version` 为空。
- 声明的能力键在分类（§3.2A）中未知。
- 声明的作用域包含非法形态（非字符串项）或非规范化模式（实现定义，但必须确定性）。

### 2B.5 能力解析日志（ext.log.v1）

在扩展加载时（制品或 dev），宿主必须发出一条日志：
- `event`：`capability.resolve`
- `data`：declared/inferred/overrides/effective 及证据哈希。

示例：
```json
{
  "schema": "pi.ext.log.v1",
  "ts": "2026-02-03T00:00:00Z",
  "level": "info",
  "event": "capability.resolve",
  "message": "Resolved effective capabilities",
  "correlation": { "extension_id": "ext.todo", "scenario_id": "scn-local" },
  "data": {
    "declared": ["read"],
    "inferred": ["read", "http"],
    "effective": ["read", "http"],
    "evidence": [
      { "capability": "http", "kind": "literal_url", "value_hash": "sha256:..." }
    ]
  }
}
```

说明：
- 证据值应哈希（可选包含脱敏预览）以避免泄露机密；遵循 §3.1 的脱敏规则。

### 2B.6 测试要求

- 推断的单元夹具，顺序确定（相同输入 → 相同推断输出）。
- 非法清单的负向夹具（未知能力、非法作用域）。
- 工具链夹具断言 `capability.resolve` 日志在归一化后稳定。

---

## 3. 扩展协议（v1）

所有通信使用**版本化、JSON 编码的协议**：`docs/schema/extension_protocol.json`。

核心消息类型：
- `register`
- `tool_call` / `tool_result`
- `slash_command` / `slash_result`
- `event_hook`
- `host_call` / `host_result`（扩展 → 核心连接器调用）
- `log` / `error`

WASM 组件使用 `docs/wit/extension.wit` 中的 **WIT 接口**。

---

### 3.1 结构化日志（ext.log.v1）

跨**捕获**、**工具链**与**运行时**的所有扩展相关日志必须使用同一 JSONL schema。协议 `log` 消息载荷与此 schema 完全一致。每行一条日志。

**日志条目 schema（必填字段标 *）：**
```json
{
  "schema": "pi.ext.log.v1",          // *
  "ts": "2026-02-03T03:01:02.123Z",   // * RFC3339
  "level": "info",                    // * debug|info|warn|error
  "event": "tool_call.start",         // * 稳定事件名
  "message": "tool call dispatched",  // * 人类摘要
  "correlation": {                    // * 用于关联日志的 ID
    "extension_id": "ext.my_ext",     // *
    "scenario_id": "scn-001",         // *
    "session_id": "sess-abc123",
    "run_id": "run-20260203-0001",
    "artifact_id": "sha256:...",
    "tool_call_id": "tool-42",
    "slash_command_id": "slash-7",
    "event_id": "evt-9",
    "host_call_id": "host-13",
    "rpc_id": "rpc-55",
    "trace_id": "trace-...",
    "span_id": "span-..."
  },
  "source": {                         // 可选发射器信息
    "component": "runtime",           // capture|harness|runtime|extension
    "host": "host.name",
    "pid": 4242
  },
  "data": { "duration_ms": 12 }
}
```

**事件命名（示例）：**
- `extension.register`、`extension.ready`
- `tool_call.start`、`tool_call.end`
- `slash_command.start`、`slash_command.end`
- `event_hook.start`、`event_hook.end`
- `host_call.start`、`host_call.end`
- `policy.decision`、`compat.warning`

**关联规则：**
- `extension_id` + `scenario_id` 对所有扩展日志**必填**。
- 填充最具体的可用 ID（`tool_call_id`、`slash_command_id`、`event_id`、`host_call_id`、`rpc_id`）。
- `trace_id`/`span_id` 可选但推荐用于长链路。

**脱敏规则（强制）：**
- 将机密/凭证替换为 `"[REDACTED]"`。
- 始终脱敏匹配（大小写不敏感）以下键：`api_key`、`token`、`authorization`、`cookie`、`password`、`secret`、`private_key`、`credential`、`bearer`。
- 对于 PII（email/phone/address），脱敏或哈希。
- 永不记录完整文件内容；仅记录大小/路径/摘要。

**用于夹具的归一化（确定性 diff）：**
- 将 `ts`、`pid`、`host`、`run_id`、`session_id`、`artifact_id`、`trace_id`、`span_id` 替换为占位符。
- 将绝对路径归一化为 `<cwd>/...`。
- 稳定 ID（如 `scenario_id`）必须确定且**不**随机化。

**日志落盘（文档化契约）：**
- **运行时：** `~/.pi/agent/logs/extensions/<session_id>.jsonl`（可用 `PI_EXTENSION_LOG_DIR` 覆盖）。
- **捕获：** `tests/ext_conformance/capture/<ext>/<scenario>/extension.log.jsonl`
- **工具链：** `target/ext_conformance/logs/<scenario_id>.jsonl`

**CI 消费：**
- CI 应将 `target/ext_conformance/logs/**` 作为制品归档。
- 工具链将归一化日志与夹具对比；diff 按 `event` 与 `correlation` ID 分流。

---

### 3.2 宿主调用 ABI（`host_call` / `host_result`）

`host_call` 是扩展向核心请求特权 I/O 的**唯一**方式。每次调用都是显式、能力门控且已记录的。

**`host_call.payload` 字段（v1）：**
- `call_id`（string，必填）：关联请求 ↔ 响应。
- `capability`（string，必填）：由策略评估的能力键。**必须**与核心从 `method` + `params` 派生的能力一致（防止伪造）。
- `method`（string，必填）：连接器方法名（如 `tool`、`exec`、`http`、`session`、`ui`、`log`）。
- `params`（object，必填）：方法特定参数。
- `timeout_ms`（int，可选）：宿主操作的墙钟超时。
- `cancel_token`（string，可选）：幂等取消句柄（未来）。
- `context`（object，可选）：自由形式元数据（永不用于策略决策）。

示例（`tool` 调用）：
```json
{
  "call_id": "host-1",
  "capability": "read",
  "method": "tool",
  "params": { "name": "grep", "input": { "pattern": "TODO", "path": "src/" } },
  "timeout_ms": 2500
}
```

**能力派生（核心定义，v1）：**
- 对于 `method="tool"`，所需能力由 `params.name` 派生：
  - `read|grep|find|ls` → `read`
  - `write|edit` → `write`
  - `bash` → `exec`
  - 未知工具 → `tool`（按策略强制 prompt/deny）
- 对于其他方法，所需能力即方法本身（`http`、`exec` 等）。

**`host_result.payload` 字段（v1）：**
- `call_id`（string，必填）
- `output`（object，必填）：方法特定结果对象（错误时可为空）
- `is_error`（bool，必填）
- `error`（object，可选）：当 `is_error=true` 时必填，否则禁止
- `chunk`（object，可选）：结果分块时的流式元数据

错误示例：
```json
{
  "call_id": "host-1",
  "output": {},
  "is_error": true,
  "error": {
    "code": "denied",
    "message": "capability denied by policy",
    "retryable": false,
    "details": { "capability": "exec" }
  }
}
```

**错误分类（v1）：**
- `timeout`：达到 deadline。
- `denied`：能力未授予或超出作用域。
- `io`：连接器 I/O 失败（fs/network/process）。
- `invalid_request`：畸形 method/params/capability 不匹配。
- `internal`：宿主中的 bug 或不变量违规。

**流式契约（v1）：**
- 核心可能对同一 `call_id` 发出多条 `host_result` 消息。
- 流式时，每条消息包含 `chunk.index` 从 0 递增，且 `chunk.is_last=true` 标记最后一块。
- `chunk.backpressure` 为未来流控提示预留。

---

### 3.2A 统一 JS + WASM 能力模型（规范性）

本节定义适用于 **PiJS (JS)** 与 **WASM** 扩展的**单一、一致的能力模型**。策略评估、日志与工具**不得**按运行时分化。

#### 能力分类（v1）

| 能力 | JS 表面（PiJS） | WASM 宿主调用 | 作用域 | 备注 |
|---|---|---|---|---|
| `read` | `pi.tool(read/grep/find/ls)`；`pi.fs.read/list/stat` | `host_call(method=tool, name in {read,grep,find,ls})`；`host_call(method=fs, op in {read,list,stat})` | `paths` | 路径作用域由连接器强制。 |
| `write` | `pi.tool(write/edit)`；`pi.fs.write/mkdir/delete` | `host_call(method=tool, name in {write,edit})`；`host_call(method=fs, op in {write,mkdir,delete})` | `paths` | 包含变更；严格模式下默认拒绝。 |
| `exec` | `pi.exec(...)`；`pi.tool(bash)` | `host_call(method=exec)`；`host_call(method=tool, name=bash)` | 无 | 进程执行；高风险。 |
| `http` | `pi.http(request)` | `host_call(method=http)` | `hosts` | 强制主机 allow-list。 |
| `session` | `pi.session.*` | `host_call(method=session)` | 无 | 会话元数据访问。 |
| `ui` | `pi.ui.*` | `host_call(method=ui)` | 无 | 非交互模式下可被拒绝。 |
| `log` | `pi.log(...)` | `host_call(method=log)` | 无 | 仅结构化日志。 |
| `tool` | `pi.tool(<non-core>)` | `host_call(method=tool, name=<non-core>)` | 无 | 用于未知/自定义工具；在 strict/prompt 模式下强制 prompt/deny。 |

说明：
- `fs` 宿主调用方法在 FS 连接器落地前可选，但**一旦存在**必须如上精确映射到 `read`/`write`。
- `tool` 能力是针对非核心工具的**兜底**；宿主应对内置工具优先显式 `read`/`write`/`exec` 映射。

#### 映射规则（必需）

1) **核心派生能力**基于 `method` + `params`（永不信任扩展提供的能力用于鉴权）。
2) **JS 与 WASM 映射到相同能力名**。为 JS 做出的策略决策对等效 WASM 调用必须相同。
3) **不匹配为错误**：若 `host_call.payload.capability` 与派生能力不一致，响应 `invalid_request`。

#### 策略 + 日志对齐

- **同一策略评估器**适用于两运行时。
- 审计日志**必须包含** `capability`、`method` 与派生决策。
- 建议：在 `log.data` 中包含 `runtime` 字段（`js` 或 `wasm`）以便跨运行时对比。

---

### 3.3 能力清单（`pi.ext.cap.v1`）

`register.payload.capability_manifest` 可选地预先声明扩展所需能力，以便策略可确定性地 prompt/deny，且工具链可校验一致性。

Schema（v1）：
```json
{
  "schema": "pi.ext.cap.v1",
  "capabilities": [
    { "capability": "read", "methods": ["tool"], "scope": { "paths": ["src/**"] } },
    { "capability": "http", "methods": ["http"], "scope": { "hosts": ["api.github.com"] } }
  ]
}
```

字段：
- `capabilities[].capability`：能力键（与策略及 `host_call.payload.capability` 使用的同一字符串）。
- `capabilities[].methods`（可选）：限制可与此能力配合使用的连接器方法集合（纵深防御）。
- `capabilities[].scope`（可选）：
  - `paths`：相对于项目根/cwd 的类 glob 模式。
  - `hosts`：网络调用的主机名/域 allow-list。
  - `env`：环境变量名 allow-list（未来连接器）。

说明：
- `register.payload.capabilities` 保留为旧版扁平列表；在所有扩展发出清单前将其视为粗粒度能力集。
- 清单**等同适用于 JS 与 WASM**运行时；两者的能力名与作用域语义完全相同。
- 扩展应在 `capability_manifest` 中镜像已解析集合（声明 ∪ 推断，§2B.3）；宿主必须记录任何漂移。

---

### 3.4 宿主调用证据账本（逐调用日志契约）

对于每次宿主调用，运行时使用 `pi.ext.log.v1` 发出仅追加的证据账本：
- `host_call.start`：分发前立即发出
- `host_call.end`：完成时发出一次（成功、错误或超时）

**账本必填字段（位于 `log.data`）：**
- `capability` / `method`
- `params_hash`（sha256 hex）
- `timeout_ms`（若存在）
- `duration_ms`（结束事件）
- `is_error` + `error.code`（结束事件，若错误）

**`params_hash` 规范化（v1）：**
- 对以下内容的规范 JSON 序列化求哈希：`{ "method": <method>, "params": <params> }`
- 规范 JSON 规则：UTF-8、无空白、对象键按字典序排序、数组保持顺序。
- 除非夹具或调试模式显式允许，否则永不将原始 `params` 写入日志（仅哈希）。

---

## 4. 能力策略（可配置模式）

`extensions.policy.mode` 支持：
- `strict`：默认拒绝，需显式授予。
- `prompt`：每能力询问一次。
- `permissive`：允许大多数；警告并记录。

建议配置（仅文档）：
```json
{
  "extensions": {
    "policy": {
      "mode": "prompt",
      "max_memory_mb": 256,
      "default_caps": ["read", "write", "http"],
      "deny_caps": ["exec", "env"]
    }
  }
}
```

能力按宿主调用强制并记录在**审计账本**中。

### 4.1 操作员预设（已实现）

Pi 通过 `extensionPolicy.profile` 与 `--extension-policy` 暴露面向用户的预设：
- `safe` → 严格默认拒绝。
- `balanced` → 带安全默认的 prompt 模式（旧别名：`standard`）。
- `permissive` → 允许大多数，主要用于短期排错。

要精确查看每项能力为何被允许/提示/拒绝，运行：

```bash
pi --explain-extension-policy
pi --explain-extension-policy --extension-policy safe
pi --explain-extension-policy --extension-policy balanced
PI_EXTENSION_ALLOW_DANGEROUS=1 pi --extension-policy balanced --explain-extension-policy
```

`--explain-extension-policy` 输出：
- 已解析的预设与来源（CLI/env/config/default）、
- 每能力决策及原因、
- 精确的 CLI 与 `settings.json` 修复片段。

### 4.2 操作员发布剧本（本地 + CI）

推荐发布顺序：
1. 从 `safe` 起步并检查决策（`pi --explain-extension-policy`）。
2. 切到 `balanced` 以在危险能力仍被拒绝时验证 prompt 模式 UX。
3. 仅对需要危险能力的运行使用 `PI_EXTENSION_ALLOW_DANGEROUS=1`。
4. 仅将 `permissive` 作为短期调试覆盖，随后回退。

本地操作员基线（`settings.json`）：

```json
{
  "extensionPolicy": {
    "profile": "balanced",
    "allowDangerous": false
  }
}
```

本地校验：

```bash
pi --explain-extension-policy
pi --extension-policy balanced --explain-extension-policy
PI_EXTENSION_ALLOW_DANGEROUS=1 pi --extension-policy balanced --explain-extension-policy
```

CI 基线（默认拒绝姿态）：

```bash
pi --extension-policy safe --explain-extension-policy
```

CI 按需任务（仅对需要危险能力的套件）：

```bash
PI_EXTENSION_ALLOW_DANGEROUS=1 pi --extension-policy balanced --explain-extension-policy
```

回滚：
- 从环境中移除 `PI_EXTENSION_ALLOW_DANGEROUS`，
- 将 `extensionPolicy.profile` 设为 `safe`，
- 重新运行 `pi --explain-extension-policy` 并验证危险能力决策为 `deny`。

### 4.3 危险能力运行的审计期望

启用危险能力时，操作员应捕获：
- 精确调用的 explain-policy JSON 输出、
- allow/prompt/deny 结果的结构化 `policy.decision` 日志、
- 敏感方法的宿主调用账本条目（`host_call.start` / `host_call.end`）。

最小 incident-ready 制品集：
- 命令调用（含预设/env）、
- explain-policy 载荷快照、
- 本次运行的 stderr/stdout 日志、
- 在 CI 中执行时的 test/e2e 摘要制品路径。

---

## 5. 能力安全（形式化决策）

我们应用**损失感知、证据驱动**模型来决定能力授予。

**证据账本**（示例）：
```
E = { uses_fs: 0.8, uses_exec: 0.1, unsigned: 0.6, size_mb: 0.2 }
```

**损失矩阵**（风险厌恶）：
```
           | grant | deny |
-----------+-------+------+
benign     |   0   |   2  |
malicious  | 100   |   1  |
```

决策规则：若期望损失更低则授予。这以数学可追溯的决策支持 **strict** 与 **prompt** 模式。

> 这有意保守：误拒成本低；误授成本高。

---

## 6. 一致性工具链

一致性工具链通过对比来自已验证清单（`VALIDATED_MANIFEST.json`）的预期注册，验证扩展在 Rust QuickJS 运行时中正确加载与注册。

### 6.1 测试基础设施

- **`tests/ext_conformance_generated.rs`** — 为语料中全部 223 个扩展自动生成的 `conformance_test!` 宏调用。
- **`tests/ext_conformance/mod.rs`** — 工具链核心：在 QuickJS 中加载扩展，捕获注册（工具、命令、标志、提供方、钩子、快捷键），与已验证清单对比。
- **`tests/ext_conformance/fixtures/*.json`** — 16 个代表性扩展的黄金夹具（用于差异预言机测试）。
- **`VALIDATED_MANIFEST.json`** — 来自 pi-mono TS 运行时 的事实来源（通过在 Bun 中加载每个扩展并捕获其注册生成）。

### 6.2 差异预言机（TS vs Rust）

一致性工具链采用**差异预言机**方法：
1. 在 **pi-mono TS 运行时**（基于 Bun）中加载每个扩展 → 记录注册的工具、命令、钩子、标志、提供方、快捷键。
2. 在 **Rust QuickJS 运行时**中加载同一扩展 → 记录相同内容。
3. 对比两者输出。任何差异即为一致性失败。

这确保 Rust 运行时产生与参考实现相同的行为，而不将测试耦合到实现细节。

### 6.3 运行一致性测试

```bash
# 运行全部 223 个一致性测试
cargo test --test ext_conformance_generated --features ext-conformance -- --nocapture

# 生成完整一致性报告（JSONL + JSON + MD）
cargo test --test ext_conformance_generated conformance_full_report \
  --features ext-conformance -- --nocapture
```

### 6.4 当前结果（2026-02-07）

- **223 个扩展中 187 个通过**（83.9%）
- **Tier 1（简单单文件）扩展 100% 通过**
- **官方 pi-mono 扩展 98.4% 通过**（60/61；1 个测试夹具）
- **30 个负向测试通过**（畸形/恶意扩展被正确拒绝）

报告：
- `tests/ext_conformance/reports/conformance_baseline.json` — 机器可读基线
- `tests/ext_conformance/reports/conformance_summary.json` — 含失败分类的摘要
- `tests/ext_conformance/reports/CONFORMANCE_REPORT.md` — 逐扩展详细结果
- `tests/ext_conformance/reports/COMPATIBILITY_SUMMARY.md` — 一致性 + 性能合并报告

---

## 7. 性能工具链

性能工具链度量扩展加载时间与事件分发延迟，强制预算并检测回归。

### 7.1 基准基础设施

- **`tests/ext_bench_harness.rs`** — 含 3 场景的基准运行器：冷加载（全新运行时）、热加载（缓存运行时）、事件分发。
- **`tests/perf_budgets.rs`** — 读取基线数据并在阈值超限时失败的 CI 强制预算检查。
- **`BENCHMARKS.md`** — 工作流文档（模式、环境变量、解读）。

### 7.2 运行基准

```bash
# 快速 PR 检查（10 个多样扩展，3 次迭代）
PI_BENCH_MODE=pr cargo test --test ext_bench_harness --features ext-conformance -- --nocapture

# 夜间全量（103 个安全扩展，10 次迭代）
PI_BENCH_MODE=nightly PI_BENCH_MAX=103 PI_BENCH_ITERATIONS=10 \
  cargo test --test ext_bench_harness --features ext-conformance -- --nocapture
```

### 7.3 性能预算

| 预算 | 阈值 | 实际（debug） | 状态 |
|--------|-----------|----------------|--------|
| 冷加载 P95（跨扩展） | < 200ms | 106ms | 通过 |
| 冷加载单扩展 P99 | < 100ms | 134ms | 失败* |
| 热加载 P95 | < 100ms | 734us | 通过 |
| 热加载单扩展 P99 | < 100ms | 926us | 通过 |
| 事件分发 P99（PR 模式） | < 5ms | 616us | 通过 |

*仅 debug 构建；release 构建快 5-10 倍（~5-10ms 冷加载）。

### 7.4 性能亮点（2026-02-07）

| 指标 | 值 |
|--------|-------|
| 中位冷加载（P50） | 77ms |
| 最快冷加载 | 67ms（trigger-compact） |
| 最慢冷加载 | 126ms（hjanuschka-plan-mode） |
| 中位热加载（P50） | 333us |
| 最慢热加载 | 836us（jyaunches-pi-canvas） |
| 已做基准的扩展 | 100 / 103 |

报告：
- `tests/perf/reports/ext_bench_baseline.json` — 机器可读基线
- `tests/perf/reports/BASELINE_REPORT.md` — 逐扩展分解
- `tests/perf/reports/budget_summary.json` — 预算通过/失败摘要

---

## 8. 尽力而为兼容性规则

兼容性扫描器输出：
- **compatible**（安全）
- **warning**（可用但受限）
- **blocked**（不安全 / 不支持）

除非设置 `strict`，系统始终**尝试运行**并给出警告。

### 8.1 已知限制

依赖以下能力的扩展在 Rust QuickJS 运行时中将无法工作：

| 限制 | 影响 |  workaround |
|------------|--------|------------|
| **无存根的 npm 包** | 导入未列出 npm 包的扩展加载失败 | 添加虚拟模块存根（见 §8.2 当前存根列表） |
| **未打包的包风格多文件导入** | 需要更广包解析的布局失败（`../../shared`、`./dist/extension.js` 等） | 加载前打包为单文件 |
| **原生 Node addon** | 已阻断 | 使用宿主调用或 WASM |
| **Worker 线程 / cluster** | 已阻断 | 不支持的并发模型 |
| **裸 socket（`net`/`tls`/`dgram`）** | 已阻断 | 使用 `pi.http()` 连接器 |
| **清单注册不匹配** | 22 个失败 | 对照实际注册审计清单 |

### 8.2 支持的 Node API 垫片

QuickJS 运行时为常见 Node API 提供垫片。完整兼容矩阵见 §2A.6。关键支持模块：
- `node:fs` — `readFileSync`、`writeFileSync`、`existsSync`、`readdirSync`、`statSync`、`mkdirSync`、`realpathSync`、promises API
- `node:path` — `join`、`resolve`、`dirname`、`basename`、`extname`、`sep`
- `node:os` — `platform`、`homedir`、`tmpdir`、`hostname`、`type`、`arch`
- `node:crypto` — `randomBytes`、`createHash`、`randomUUID`
- `node:url` — `URL`、`parse`、`fileURLToPath`
- `node:child_process` — `spawn`、`spawnSync`、`exec`、`execFile`、`execSync`、`execFileSync`（经 `exec` 能力）
- `node:readline` — 用于交互式提示的基础接口
- `node:module` — `createRequire` 存根

### 8.3 Bun 全局/模块兼容子集

运行时还通过 `globalThis.Bun` 与 `import "bun"` 暴露聚焦的 Bun 子集：

- `Bun.argv`
- `Bun.file(path)`（`exists()`、`text()`、`arrayBuffer()`、`json()`）
- `Bun.write(pathOrFileLike, data)`
- `Bun.which(command)`
- `Bun.spawn(command, options)` / `Bun.spawn([cmd, ...args], options)`

对 Bun socket API 提供兼容存根，但它们**不**创建真实网络连接：

- `Bun.connect(...)` — 内存存根 socket 发射器（无网络 I/O）
- `Bun.listen(...)` — 内存存根 server 发射器（无网络 I/O）

如需真实网络访问，请使用 `pi.http(...)` 或 `node:http`。

若扩展需要不支持的 Bun API，保持扩展不变，通过运行时兼容性工作（新增通用垫片/连接器支持）或能力管控的替代 API 来解决。

已为常见第三方依赖提供 16+ npm 包存根（`openai`、`adm-zip`、`linkedom`、`@sourcegraph/scip-typescript`、`node-pty`、`chokidar`、`jsdom`、`turndown`、`@opentelemetry/*` 等）。

---

## 9. 添加新扩展

要向已验证语料添加新扩展：

1. **放置扩展源码**到合适的语料目录（如 `legacy_pi_mono_code/corpus/community/`）。

2. **在 TS 预言机中验证** — 通过基于 Bun 的工具链运行扩展以捕获其预期注册：
   ```bash
   cd tests/ext_conformance/ts_oracle
   bun run validate.ts /path/to/extension.ts
   ```

3. **加入 `VALIDATED_MANIFEST.json`** — 将预言机输出合并到清单，使 Rust 一致性测试有事实来源可对比。

4. **重新生成一致性测试** — `tests/ext_conformance_generated.rs` 中的 `conformance_test!` 宏条目由清单生成。

5. **运行一致性** — 验证扩展通过：
   ```bash
   cargo test --test ext_conformance_generated test_<extension_id> \
     --features ext-conformance -- --nocapture
   ```

6. **更新目录** — 按 `pi.ext.catalog.v1` schema（§1C.4）向 `docs/extension-catalog.json` 添加条目。

若扩展未通过一致性，按 §1C.5 的失败分解分类，并判断是否需要新的 Node 垫片、npm 存根或清单修正。

## 10. 未来工作

- **WASM 组件运行时**（层级 A）— wasmtime 与 WIT 宿主调用集成。
- **`extc` 编译器流水线** — 基于 SWC 的 TS→JS 打包 + QuickJS 字节码预编译以加快冷加载。
- **扩展 npm 存根** — 随语料出现新增包存根。
- **多文件打包** — 为复杂扩展解析跨目录导入。
- **Release 构建基准** — 建立 release 模式基线（预期比 debug 快 5-10 倍）。
