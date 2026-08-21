# 扩展运行时架构

本文档描述了 `pi_agent_rust` 的扩展运行时架构，涵盖运行时模型、宿主调用分发、能力策略、信任边界和结构化并发。

## 概述

扩展支持两种运行时模式：

1. 旧版 JS/TS 入口点（`.js/.jsx/.ts/.mjs/.cjs/.tsx/.mts/.cts`）在嵌入式 QuickJS 解释器中运行。
2. 原生描述符入口点（`*.native.json`）通过 native-rust 描述符运行时运行。

为保持与旧版 Pi 的兼容性，JS/TS 入口点直接加载，无需手动转换步骤。运行时选择根据入口点类型自动进行。宿主在分发每个请求前强制执行可配置的能力策略。

```
 Extension JS (untrusted)          Rust Host (trusted)
┌──────────────────────┐         ┌──────────────────────────────┐
│  pi.tool(...)        │         │  ToolRegistry.execute()      │
│  pi.exec(...)        │ ─────►  │  subprocess spawn            │
│  pi.http(...)        │ hostcall│  HttpConnector.dispatch()    │
│  pi.session(...)     │ channel │  ExtensionSession trait      │
│  pi.ui(...)          │         │  UI channel (mpsc)           │
│  pi.events(...)      │         │  ExtensionManager state      │
│  pi.log(...)         │         │  structured log sink         │
└──────────────────────┘         └──────────────────────────────┘
```

## 核心类型

### `ExtensionManager` (`src/extensions.rs`)

包装 `Arc<Mutex<ExtensionManagerInner>>` 的中心注册表。线程安全、可低成本克隆。拥有：

| Field                 | Type                                       | Purpose                                        |
| --------------------- | ------------------------------------------ | ---------------------------------------------- |
| `extensions`          | `Vec<RegisterPayload>`                     | 已注册的扩展元数据                             |
| `runtime`             | `Option<ExtensionRuntimeHandle>`           | 活动的扩展运行时句柄（QuickJS 或 native-rust） |
| `ui_sender`           | `Option<mpsc::Sender<ExtensionUiRequest>>` | 通往 TUI 的用户提示通道                        |
| `session`             | `Option<Arc<dyn ExtensionSession>>`        | 当前会话状态访问                               |
| `active_tools`        | `Option<Vec<String>>`                      | 此会话已启用的工具名称                         |
| `providers`           | `Vec<Value>`                               | 自定义 `streamSimple` 提供方模型               |
| `flags`               | `Vec<Value>`                               | 扩展注册的功能标志                             |
| `policy_prompt_cache` | `HashMap<String, HashMap<String, bool>>`   | 缓存的按会话权限决策                           |
| `permission_store`    | `Option<PermissionStore>`                  | 持久化的 Allow/Deny Always 存储                |
| `extension_budget`    | `Budget`                                   | 结构化并发超时预算                             |

### `ExtensionRegion` (`src/extensions.rs`)

包装 `ExtensionManager` 的 RAII 守卫，用于结构化并发。被丢弃时，它会使用可配置的清理预算（默认 5 秒）关闭活动的扩展运行时句柄。

```rust
pub struct ExtensionRegion {
    manager: ExtensionManager,
    cleanup_budget: Duration,       // default 5s
    shutdown_done: AtomicBool,      // prevents double-shutdown
}
```

用法：`AgentSession.extensions: Option<ExtensionRegion>`。调用者通过 `region.manager()` 访问内部管理器。

### `JsExtensionLoadSpec` (`src/extensions.rs`)

用于从磁盘加载 JavaScript 扩展的声明式规范：

- `extension_id` -- 唯一标识符（例如 `ext.github_copilot`）
- `entry_path` -- 指向 `.js`/`.ts` 入口点的规范 `PathBuf`
- `name`, `version`, `api_version` -- 来自 `extension.json` 的元数据

工厂方法：`JsExtensionLoadSpec::from_entry_path(path)` 解析清单并规范化路径。

### `NativeRustExtensionLoadSpec` (`src/extensions.rs`)

用于加载原生描述符扩展的声明式规范：

- `extension_id` -- 唯一标识符（例如 `ext.some_native_extension`）
- `entry_path` -- 指向 `*.native.json` 的规范 `PathBuf`
- `name`, `version`, `api_version` -- 来自 `extension.json` 的元数据

### `RegisterPayload` (`src/extensions.rs`)

扩展的 `activate()` 调用返回的数据：

- `name`, `version`, `api_version` -- 身份
- `capabilities: Vec<String>` -- 请求的能力令牌
- `capability_manifest: Option<CapabilityManifest>` -- 结构化的能力声明
- `tools`, `slash_commands`, `shortcuts`, `flags`, `event_hooks` -- 已注册的功能

## 宿主调用分发

来自 JavaScript 的每个 `pi.*()` 调用都会在宿主调用通道上入队一个 `HostcallRequest`。QuickJS 线程会阻塞等待响应。

### `HostcallKind` (`src/extensions_js.rs`)

```rust
pub enum HostcallKind {
    Tool { name: String },     // pi.tool(name, input)
    Exec { cmd: String },      // pi.exec(cmd, args)
    Http,                      // pi.http(request)
    Session { op: String },    // pi.session(op, args)
    Ui { op: String },         // pi.ui(op, args)
    Events { op: String },     // pi.events(op, args)
    Log,                       // pi.log(entry)
}
```

### 分发流程

```
HostcallRequest
  │
  ▼
dispatch_hostcall_with_runtime()     [src/extensions.rs]
  ├── 1. Test interceptor check (short-circuit for mocking)
  ├── 2. Convert to canonical HostCallPayload
  ├── 3. Build HostCallContext (policy, tools, http, manager)
  ├── 4. dispatch_host_call_shared()  [src/extensions.rs]
  │       └── capability derivation + policy check
  └── 5. Kind-specific handler:
          ├── dispatch_hostcall_tool()     → ToolRegistry.execute()
          ├── dispatch_hostcall_exec()     → subprocess spawn + capture
          ├── dispatch_hostcall_http()     → HttpConnector.dispatch()
          ├── dispatch_hostcall_session()  → ExtensionSession trait methods
          ├── dispatch_hostcall_ui()       → mpsc channel to TUI
          ├── dispatch_hostcall_events()   → event hook registration
          └── dispatch_hostcall_log()      → structured log emission
```

### 会话操作

`dispatch_hostcall_session()`（`src/extensions.rs`）将 `op` 值路由到 `ExtensionSession` trait 方法：

| JS call                          | Session method                  |
| -------------------------------- | ------------------------------- |
| `pi.session("getState")`         | `get_state()`                   |
| `pi.session("getMessages")`      | `get_messages()`                |
| `pi.session("setName", name)`    | `set_name(name)`                |
| `pi.session("appendMessage", m)` | `append_message(m)`             |
| `pi.session("setModel", p, m)`   | `set_model(provider, model_id)` |
| `pi.session("getModel")`         | `get_model()`                   |
| `pi.session("setThinkingLevel")` | `set_thinking_level(level)`     |
| `pi.session("getThinkingLevel")` | `get_thinking_level()`          |
| `pi.session("setLabel", id, l)`  | `set_label(target_id, label)`   |

`ExtensionSession` trait（`src/extensions.rs`）由以下实现：

- `SessionHandle`（`session.rs`）-- 由 SQLite 支持的生产会话
- `InteractiveExtensionSession`（`interactive.rs`）-- TUI 交互模式
- `NullSession` / `TestSession`（`extension_dispatcher.rs`）-- 测试替身

### 事件操作

`dispatch_hostcall_events()`（`src/extensions.rs`）处理注册 API 调用：

| JS call                                | Action                 |
| -------------------------------------- | ---------------------- |
| `pi.events("registerTool", spec)`      | 向扩展的工具中添加工具 |
| `pi.events("registerSlashCommand")`    | 添加斜杠命令           |
| `pi.events("registerShortcut")`        | 添加键盘快捷键         |
| `pi.events("registerFlag")`            | 添加功能标志           |
| `pi.events("registerProvider")`        | 注册自定义 LLM 提供方  |
| `pi.events("getActiveTools")`          | 列出已启用的工具名称   |
| `pi.events("getAllTools")`             | 列出所有已注册的工具   |
| `pi.events("registerMessageRenderer")` | 注册消息渲染器         |

## 能力策略

### 策略模型 (`src/extensions.rs`)

```rust
pub enum ExtensionPolicyMode {
    Strict,      // deny-by-default
    Prompt,      // ask user for unknown capabilities
    Permissive,  // allow all with audit logging
}

pub struct ExtensionPolicy {
    pub mode: ExtensionPolicyMode,
    pub max_memory_mb: u32,                              // default 256
    pub default_caps: Vec<String>,                       // auto-allowed
    pub deny_caps: Vec<String>,                          // always denied
    pub per_extension: HashMap<String, ExtensionOverride>,// per-ext overrides
}
```

默认策略（`Prompt` 模式）：

- **允许**：`read`、`write`、`http`、`events`、`session`
- **拒绝**：`exec`、`env`

### 策略配置 (`src/extensions.rs`)

| Profile      | Mode         | Allowed caps                       | Denied caps |
| ------------ | ------------ | ---------------------------------- | ----------- |
| `Safe`       | `Strict`     | read, write                        | exec, env   |
| `Standard`   | `Prompt`     | read, write, http, events, session | exec, env   |
| `Permissive` | `Permissive` | all                                | none        |

通过 `pi.toml` 配置：

```toml
[extensions.policy]
profile = "safe"        # or "standard", "permissive"
allow_dangerous = false # override to allow exec/env
```

CLI 覆盖：`--extension-policy safe`

### 优先级链 (`src/extensions.rs`)

策略评估遵循严格的优先级：

1. **按扩展拒绝** -- 能力位于扩展覆盖的 `deny` 列表中
2. **全局 deny_caps** -- 能力位于全局 `deny_caps` 中
3. **按扩展允许** -- 能力位于扩展覆盖的 `allow` 列表中
4. **全局 default_caps** -- 能力位于 `default_caps` 中
5. **模式回退** -- Strict：拒绝，Prompt：提示，Permissive：允许

每一层要么产生最终决策，要么递交给下一层。

### 能力映射

每个 `HostcallKind` 通过 `required_capability_for_host_call()` 映射到所需的能力：

| HostcallKind | Required Capability |
| ------------ | ------------------- |
| `Tool`       | `tool`              |
| `Exec`       | `exec`              |
| `Http`       | `http`              |
| `Session`    | `session`           |
| `Ui`         | `ui`                |
| `Events`     | `events`            |
| `Log`        | `log`               |

## 信任边界

```
┌─────────────────────────────────────────────────────────────┐
│                    Untrusted Zone                            │
│                                                             │
│   Extension JavaScript (QuickJS sandbox)                    │
│   - No direct filesystem access                            │
│   - No direct network access                               │
│   - No direct process spawning                             │
│   - Heap limited to max_memory_mb                          │
│                                                             │
├─────────────────── Hostcall Boundary ───────────────────────┤
│                                                             │
│   Policy Enforcement Layer                                  │
│   - Capability derivation from HostcallKind                │
│   - Policy evaluation (5-layer precedence)                 │
│   - Permission prompting (Prompt mode)                     │
│   - Audit logging (all modes)                              │
│                                                             │
├─────────────────── Host Dispatch ───────────────────────────┤
│                                                             │
│                    Trusted Zone                              │
│                                                             │
│   Tool execution, subprocess spawn, HTTP client,            │
│   session state, UI prompts, event hooks, logging           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

关键安全属性：

- **无环境授权**：扩展无法绕过宿主调用通道。所有危险操作都需要显式的能力授权。
- **故障关闭**：未知配置解析为 `Safe`。空能力字符串被拒绝。缺失的会话/管理器会产生错误结果。
- **按扩展隔离**：`ExtensionOverride` 允许按扩展 ID 进行细粒度的允许/拒绝，而不影响其他扩展。
- **提示疲劳缓解**：批量提示（250ms 窗口）、Allow/Deny Always 持久化以及决策审计日志。
- **弱引用循环断开**：`JsRuntimeHost` 持有 `Weak<Mutex<ExtensionManagerInner>>` 以防止管理器与 JS 线程之间的 Arc 循环。

## 运行时生命周期

### 加载

1. 发现：扫描 `~/.pi/agent/extensions/` 以查找 `extension.json` 清单
2. 解析：`JsExtensionLoadSpec::from_entry_path(path)` 验证清单
3. QuickJS 初始化：使用虚拟模块 + 策略创建 `PiJsRuntime`
4. 执行：运行扩展的入口点，调用 `pi.register(payload)`
5. 注册：`RegisterPayload` 存储在 `ExtensionManagerInner` 中

### 虚拟模块系统

扩展 `require()` 的 Node/npm 模块在 QuickJS 中被填充：

**Node 内置模块**：`node:fs`、`node:path`、`node:os`、`node:crypto`、`node:child_process`、`node:events`、`node:buffer`、`node:url`、`node:http`、`node:net`、`node:readline`、`node:util`、`node:stream`

**npm/包填充与存根**：`uuid`、有界 HMAC `jsonwebtoken`、`shell-quote`、`glob`、`chalk`、`chokidar`、`jsdom`、`turndown`、`node-pty`、`@opentelemetry/*`、`@xterm/*`、`vscode-languageserver-protocol`、`@sinclair/typebox`、`@mariozechner/pi-ai`

**Pi SDK**：`@mariozechner/pi-coding-agent`（提供 `keyHint`、`compact`、`completeSimple`、`fuzzyMatch`、`fuzzyFilter`）

### 关闭

1. `ExtensionRegion` 被丢弃（会话结束）
2. `JsRuntimeCommand::Shutdown` 发送到 QuickJS 线程
3. QuickJS 线程在清理预算内退出（默认 5s）
4. 如果超出预算，线程被遗弃（不强制终止，依赖进程退出）

### 结构化并发

- `ExtensionRegion` 保证在所有退出路径（正常、恐慌、提前返回）上进行清理
- `Budget` 跟踪扩展操作的剩余时间
- `effective_timeout()` 将管理器预算与按操作超时取交集
- 取消通过宿主调用通道传播

## 提供方扩展 (`streamSimple`)

扩展可以通过 `pi.events("registerProvider", spec)` 注册自定义 LLM 提供方。提供方实现 `streamSimple(model, context, options)` 并返回 `AsyncIterable<string>`。

遵循 `@mariozechner/pi-ai/compat` API 的扩展可以在调用 `pi.registerProvider` 之前改为调用 `registerApiProvider({ api, stream, streamSimple }, sourceId)`。运行时将该 API 提供方作用域限定于注册的扩展，通过 `getApiProvider` 和 `getApiProviders` 暴露它，并将其 `streamSimple` 处理器绑定到匹配的提供方模型。`unregisterApiProviders(sourceId)` 仅移除调用扩展拥有的条目。这使得兼容提供方能够提供协议处理器，而不会让另一扩展访问它。

Rust 侧：`src/providers/mod.rs` 中的 `ExtensionStreamSimpleProvider` 实现了 `Provider` trait。来自 JS 的每个块都成为一个 `StreamEvent::TextDelta`。取消通过流状态的 `Drop` 实现。

对于需要基于令牌认证的提供方，可通过 `ModelEntry` 上的 `OAuthConfig` 获得 OAuth 支持。

## 测试架构

| Layer       | Infrastructure                     | Location                     |
| ----------- | ---------------------------------- | ---------------------------- |
| Unit tests  | Direct struct/function tests       | `tests/extensions_*.rs`      |
| VCR tests   | HTTP interaction playback          | `tests/provider_*.rs`        |
| Conformance | Differential oracle (TS vs Rust)   | `tests/ext_conformance_*.rs` |
| E2E         | Full CLI + tmux scripting          | `tests/e2e_*.rs`             |
| Property    | proptest random inputs             | `tests/ext_proptest.rs`      |
| Stress      | Concurrent load + memory profiling | `tests/extensions_stress.rs` |
| Security    | FS escape, policy negative tests   | `tests/security_*.rs`        |

crate 本地的特征化套件由 `src/extensions/tests.rs` 路由，并按行为拆分到 `src/extensions/tests/` 下：`core`、`registration`、`baseline`、`risk_math`、`enforcement`、`policy_transition`、`exec_security`、`event_timeouts`、`concurrency`、`reactor`、`shared_dispatch`、`runtime_parity`、`ui_protocol` 和 `security_alerts`。将这些名称保留为测试路径域，使得诸如 `cargo test extensions::tests::reactor` 之类的聚焦命令保持稳定，而无需将完整套件放回生产外观中。

测试拦截器：`HostcallInterceptor` trait 允许测试代码短路宿主调用分发，返回预定结果而无需触及真实工具、网络或文件系统。

## 文件映射

| File                                                  | Responsibility                                           |
| ----------------------------------------------------- | -------------------------------------------------------- |
| `src/extensions.rs`                                   | 稳定的公共外观；公共类型定义、管理器状态和共享分发入口点 |
| `src/extensions/extension_manager_impl.rs`            | `ExtensionManager` 生命周期、策略、注册表和事件编排实现  |
| `src/extensions/protocol.rs`                          | 版本化消息验证以及宿主调用反应器、编组和操作码内部实现   |
| `src/extensions/fs_connector.rs`                      | 能力作用域的文件系统连接器实现                           |
| `src/extensions/exec_mediation.rs`                    | 危险命令分类和密钥代理策略实现                           |
| `src/extensions/permission_drift.rs`                  | 权限快照漂移分类与证据                                   |
| `src/extensions/event_coalescer_impl.rs`              | 合并的事件分发实现                                       |
| `src/extensions/native_runtime_duplicate_scaffold.rs` | 活动的确定性原生描述符运行时                             |
| `src/extensions/native_runtime_experimental.rs`       | 保留的、编译禁用的原生运行时原型                         |
| `src/extensions/wasm_host.rs`                         | 功能门控的 Wasmtime 组件宿主及其一致性测试               |
| `src/extensions/compatibility.rs`                     | 扩展兼容性扫描器                                         |
| `src/extensions/policy_snapshot_tests.rs`             | 已编译策略快照特征化测试                                 |
| `src/extensions/tests.rs`, `src/extensions/tests/`    | 测试路由器和行为域单元/特征化套件                        |
| `src/extensions_js.rs`                                | QuickJS 运行时、虚拟模块和 `HostcallKind`                |
| `src/extension_dispatcher.rs`                         | `ExtensionSession` 实现和分发器集成                      |
| `src/config.rs`                                       | `ExtensionPolicyConfig` 和已解析的策略                   |
| `src/providers/mod.rs`                                | `ExtensionStreamSimpleProvider`                          |
| `src/connectors/mod.rs`, `src/connectors/http.rs`     | 共享连接器外观和 `HttpConnector` 实现                    |
| `src/auth.rs`                                         | 扩展提供方的 OAuth 令牌管理                              |
