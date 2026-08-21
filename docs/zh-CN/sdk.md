# SDK 入门与迁移指南（SDK Cookbook and Migration Guide）

> 本文为英文原文的中文翻译，源文件：`docs/sdk.md`

本指南面向将 Pi 作为 Rust 库嵌入的团队。Rust SDK 为 Pi 的核心嵌入工作流提供符合 Rust 习惯的 API，采用 `Result` 类型与结构化并发等 Rust 原生模式。

**注意**：此 SDK 是 pi-mono TypeScript SDK 符合 Rust 习惯的配套实现，并非直接替代（drop-in equivalent）。其对等性仍由现行认证契约（certification contract）及其溯源（provenance）匹配的裁决所约束。

## 安装（Install）

```toml
[dependencies]
pi = { package = "pi_agent_rust", version = "0.2.0" }
futures = "0.3"
```

在基于本地检出进行开发时，将 `version = "0.2.0"` 替换为 `path = "/path/to/pi_agent_rust"`，同时保留 `package = "pi_agent_rust"`。

## SemVer 表面（SemVer Surface）

受支持的库表面为 crate 根别名 `pi::Error`、`pi::PiResult` 以及 `pi::sdk` 模块。其他根模块为 CLI、示例及仓内测试的实现细节；它们在已发布的 API 文档中被隐藏，且可能在不提供 SemVer 保障的情况下变更。

`semver` GitHub Actions 工作流会在触及 SDK/API 表面的 PR 与 `main` 推送上运行 `cargo-semver-checks`。它会将当前公开 API 与 PR 目标分支或上一次推送基线进行比较。对稳定条目的不兼容变更需要进行 SemVer 不兼容的版本提升（1.0 之前为 `0.y` 到 `0.(y+1)`，1.0 之后为大版本提升）。只有与 semver 兼容的增量才保持兼容；为 Rust 消费者添加公开枚举变体或必填结构体字段可能是破坏性变更。

### 稳定性注解（Stability Annotations）

| 条目（Item）                                                                                                                                                                                                                                                              | 稳定性（Stability） | 说明（Notes）                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------------- |
| `pi::Error`                                                                                                                                                                                                                                                               | Stable              | Crate 根错误类型别名目标。                                     |
| `pi::PiResult`                                                                                                                                                                                                                                                            | Stable              | 针对 `pi::Error` 的 Crate 根结果别名。                         |
| `pi::sdk::{Error, Result}`                                                                                                                                                                                                                                                | Stable              | SDK 错误/结果导出。                                            |
| `pi::sdk::{AbortHandle, AbortSignal}`                                                                                                                                                                                                                                     | Stable              | 提示取消句柄。                                                 |
| `pi::sdk::{Agent, AgentConfig, AgentEvent, AgentSession, QueueMode}`                                                                                                                                                                                                      | Stable              | 进程内智能体/会话集成导出。                                    |
| `pi::sdk::{AssistantMessage, ContentBlock, Cost, CustomMessage, ImageContent, Message, StopDetails, StopReason, StreamEvent, TextContent, ThinkingContent, ToolCall, ToolResultMessage, Usage, UserContent, UserMessage}`                                                 | Stable              | 消息、内容、流式与计费模型类型。                               |
| `pi::sdk::{Config, ExtensionManager, ExtensionPolicy, ExtensionRegion, Session, ThinkingLevel}`                                                                                                                                                                           | Stable              | 配置、扩展、会话与思考控制导出。                               |
| `pi::sdk::{InputType, Model, ModelCost, Provider, ProviderContext, ProviderThinkingBudgets, StreamOptions, ToolDef}`                                                                                                                                                      | Stable              | 提供方集成导出。                                               |
| `pi::sdk::{ModelEntry, ModelRegistry}`                                                                                                                                                                                                                                    | Stable              | 模型注册表导出。                                               |
| `pi::sdk::{Tool, ToolDefinition, ToolOutput, ToolRegistry, ToolUpdate}`                                                                                                                                                                                                   | Stable              | 工具集成导出。                                                 |
| `pi::sdk::BUILTIN_TOOL_NAMES`                                                                                                                                                                                                                                             | Stable              | 规范的默认非委托工具名称清单；按需启用的 `subagent` 另行提供。 |
| `pi::sdk::{create_read_tool, create_bash_tool, create_edit_tool, create_write_tool, create_grep_tool, create_find_tool, create_ls_tool, create_hashline_edit_tool, create_all_tools}`                                                                                     | Stable              | 默认非委托工具构造器。                                         |
| `pi::sdk::{tool_to_definition, all_tool_definitions}`                                                                                                                                                                                                                     | Stable              | 默认非委托工具契约辅助函数。                                   |
| `pi::sdk::{SubscriptionId, EventListeners, EventSubscriber, OnStreamEvent, OnToolEnd, OnToolStart}`                                                                                                                                                                       | Stable              | 事件订阅与钩子类型。                                           |
| `pi::sdk::{SessionOptions, ToolFactory, default_tool_registry}`                                                                                                                                                                                                           | Stable              | 进程内会话构造与自定义工具注册表扩展点。                       |
| `pi::sdk::{AgentSessionHandle, AgentSessionState, create_agent_session}`                                                                                                                                                                                                  | Stable              | 主要的进程内 SDK 入口与状态句柄。                              |
| `pi::sdk::{SessionPromptResult, SessionTransport, SessionTransportEvent, SessionTransportState}`                                                                                                                                                                          | Stable              | 统一的进程内/RPC 传输适配器。                                  |
| `pi::sdk::{RpcTransportClient, RpcTransportOptions}`                                                                                                                                                                                                                      | Stable              | 子进程 RPC 传输客户端。                                        |
| `pi::sdk::{RpcBashResult, RpcCancelledResult, RpcCommandInfo, RpcCompactionResult, RpcCycleModelResult, RpcExportHtmlResult, RpcExtensionUiResponse, RpcForkMessage, RpcForkResult, RpcLastAssistantText, RpcModelInfo, RpcSessionState, RpcSessionStats, RpcTokenStats}` | Stable              | RPC 请求/响应负载。                                            |

## 迁移映射（Migration Map (TypeScript -> Rust)）

| TypeScript 表面                               | Rust SDK 表面                                                          |
| --------------------------------------------- | ---------------------------------------------------------------------- |
| `createAgentSession(options)`                 | `pi::sdk::create_agent_session(SessionOptions)`                        |
| `session.prompt(text, onEvent)`               | `AgentSessionHandle::prompt(text, on_event)`                           |
| `session.subscribe(listener)`                 | `AgentSessionHandle::subscribe(listener)`                              |
| `unsubscribe()`                               | `AgentSessionHandle::unsubscribe(subscription_id)`                     |
| `session.setModel(provider, model)`           | `AgentSessionHandle::set_model(provider, model)`                       |
| `session.setThinkingLevel(level)`             | `AgentSessionHandle::set_thinking_level(level)`                        |
| `session.compact()`                           | `AgentSessionHandle::compact(on_event)`                                |
| `session.abort()`                             | `AgentSessionHandle::new_abort_handle()` + `prompt_with_abort(...)`    |
| `session.steer(...)`, `session.followUp(...)` | `RpcTransportClient::steer(...)`, `RpcTransportClient::follow_up(...)` |
| RPC bridge client                             | `RpcTransportClient` / `SessionTransport::RpcSubprocess`               |

## 示例 1：创建进程内会话并发起提示（Recipe 1: Create In-Process Session and Prompt）

```rust
use futures::executor::block_on;
use pi::sdk::{AgentEvent, SessionOptions, create_agent_session};

fn main() -> pi::sdk::Result<()> {
    let mut session = block_on(create_agent_session(SessionOptions {
        provider: Some("openai".to_string()),
        model: Some("gpt-4o".to_string()),
        api_key: Some(std::env::var("OPENAI_API_KEY").unwrap_or_default()),
        no_session: true,
        ..SessionOptions::default()
    }))?;

    let message = block_on(session.prompt("Summarize src/sdk.rs", |event: AgentEvent| {
        eprintln!("{event:?}");
    }))?;

    println!("{message:#?}");
    Ok(())
}
```

## 示例 2：会话级订阅与类型化钩子（Recipe 2: Session-Level Subscribers and Typed Hooks）

```rust
use futures::executor::block_on;
use pi::sdk::{SessionOptions, create_agent_session};
use std::sync::Arc;

fn main() -> pi::sdk::Result<()> {
    let options = SessionOptions {
        on_tool_start: Some(Arc::new(|tool, args| eprintln!("tool start: {tool} {args}"))),
        on_tool_end: Some(Arc::new(|tool, output, is_error| {
            eprintln!("tool end: {tool}, error={is_error}, output={output:?}");
        })),
        on_stream_event: Some(Arc::new(|ev| eprintln!("stream: {ev:?}"))),
        ..SessionOptions::default()
    };

    let mut session = block_on(create_agent_session(options))?;
    let sub_id = session.subscribe(|event| eprintln!("session event: {event:?}"));

    let _ = block_on(session.prompt("read Cargo.toml", |_| {}))?;
    let _removed = session.unsubscribe(sub_id);
    Ok(())
}
```

## 示例 3：提示取消（Recipe 3: Prompt Cancellation）

```rust
use futures::executor::block_on;
use pi::sdk::{AgentSessionHandle, SessionOptions, create_agent_session};

fn main() -> pi::sdk::Result<()> {
    let mut session = block_on(create_agent_session(SessionOptions::default()))?;

    let (abort_handle, abort_signal) = AgentSessionHandle::new_abort_handle();
    let fut = session.prompt_with_abort("long running prompt", abort_signal, |_| {});
    abort_handle.abort();
    let _ = block_on(fut);
    Ok(())
}
```

## 示例 4：模型与思考控制（Recipe 4: Model and Thinking Controls）

```rust
use futures::executor::block_on;
use pi::sdk::{SessionOptions, ThinkingLevel, create_agent_session};

fn main() -> pi::sdk::Result<()> {
    let mut session = block_on(create_agent_session(SessionOptions::default()))?;
    block_on(session.set_model("openai", "gpt-4o"))?;
    block_on(session.set_thinking_level(ThinkingLevel::Low))?;

    let state = block_on(session.state())?;
    println!("provider={} model={}", state.provider, state.model_id);
    Ok(())
}
```

## 示例 5：在 SDK 会话中加载扩展（Recipe 5: Load Extensions in SDK Sessions）

```rust
use futures::executor::block_on;
use pi::sdk::{SessionOptions, create_agent_session};
use std::path::PathBuf;

fn main() -> pi::sdk::Result<()> {
    let session = block_on(create_agent_session(SessionOptions {
        extension_paths: vec![PathBuf::from("extensions/my_extension.js")],
        extension_policy: Some("safe".to_string()),
        repair_policy: Some("ask".to_string()),
        ..SessionOptions::default()
    }))?;

    if session.has_extensions() {
        eprintln!("extensions loaded");
    }
    Ok(())
}
```

## 示例 5b：处理扩展 UI 提示与权限作用域（Recipe 5b: Handle Extension UI Prompts and Permission Scope）

若未附加 UI 处理器，SDK 会话将失效关闭（fail closed）：扩展 UI 请求会报错，能力（capability）提示将解析为拒绝。请附加处理器以在进程内应答。

```rust
use futures::executor::block_on;
use pi::sdk::{
    ExtensionUiHandler, ExtensionUiRequest, ExtensionUiResponse, SessionOptions,
    create_agent_session,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

struct AllowOnce;

#[async_trait::async_trait]
impl ExtensionUiHandler for AllowOnce {
    async fn request_ui(
        &self,
        request: ExtensionUiRequest,
    ) -> pi::sdk::Result<Option<ExtensionUiResponse>> {
        Ok(Some(ExtensionUiResponse {
            id: request.id,
            // Plain `Value::Bool(allow)` keeps default persistence; an object
            // controls it per decision ("persist": false = this session only).
            value: Some(json!({ "allow": true, "persist": false })),
            cancelled: false,
        }))
    }
}

fn main() -> pi::sdk::Result<()> {
    let _session = block_on(create_agent_session(SessionOptions {
        extension_paths: vec![PathBuf::from("extensions/my_extension.js")],
        extension_ui_handler: Some(Arc::new(AllowOnce)),
        // `false` scopes all prompt decisions to this session's memory instead
        // of `~/.pi/extension-permissions.json` (default `true` = CLI behavior).
        persist_extension_permissions: false,
        ..SessionOptions::default()
    }))?;
    Ok(())
}
```

## 示例 5c：按会话覆盖压缩设置（Recipe 5c: Override Compaction Settings Per Session）

```rust
use futures::executor::block_on;
use pi::sdk::{ResolvedCompactionSettings, SessionOptions, create_agent_session};

fn main() -> pi::sdk::Result<()> {
    let session = block_on(create_agent_session(SessionOptions {
        // Used verbatim; `None` keeps the config/model-derived defaults.
        compaction_settings: Some(ResolvedCompactionSettings {
            enabled: true,
            context_window_tokens: 200_000,
            reserve_tokens: 32_768,
            keep_recent_tokens: 40_000,
        }),
        ..SessionOptions::default()
    }))?;
    eprintln!("resolved: {:?}", session.compaction_settings());
    Ok(())
}
```

## 示例 6：使用 RPC 传输客户端（Recipe 6: Use RPC Transport Client）

```rust
use futures::executor::block_on;
use pi::sdk::{RpcTransportClient, RpcTransportOptions};

fn main() -> pi::sdk::Result<()> {
    let mut rpc = RpcTransportClient::connect(RpcTransportOptions::default())?;

    let state = block_on(rpc.get_state())?;
    println!("rpc session id: {}", state.session_id);

    let events = block_on(rpc.prompt("Hello from RPC"))?;
    println!("received {} rpc events", events.len());

    rpc.shutdown()?;
    Ok(())
}
```

## 示例 7：统一传输适配器（进程内或 RPC）（Recipe 7: Unified Transport Adapter (In-Process or RPC)）

```rust
use futures::executor::block_on;
use pi::sdk::{SessionOptions, SessionTransport};

fn main() -> pi::sdk::Result<()> {
    let mut transport = block_on(SessionTransport::in_process(SessionOptions::default()))?;

    let _result = block_on(transport.prompt("Status?", |_event| {}))?;
    let _state = block_on(transport.state())?;
    transport.shutdown()?;
    Ok(())
}
```

## 迁移集成者的兼容性说明（Compatibility Notes for Migrating Integrators）

- `SessionOptions::default().no_session` 为 `true`（默认为临时会话）。
- 进程内 `AgentSessionHandle` 目前暴露提示/状态/模型/思考/压缩流程；`steer`/`follow_up` 等队列控制位于 `RpcTransportClient` 上。
- `SessionTransport::prompt` 返回 `SessionPromptResult`，根据后端不同为 `InProcess(Box<AssistantMessage>)` 或 `RpcEvents(Vec<Value>)`。
- 扩展加载通过 `extension_paths` 按需启用，并由 `extension_policy`/`repair_policy` 控制。
- 扩展 UI/能力提示通过 `SessionOptions::extension_ui_handler` 应答；若未提供则失效关闭（拒绝）。
- 提示决策默认持久化到磁盘（与 CLI 保持一致）；`persist_extension_permissions: false` 或单次响应中的 `"persist": false` 会将其作用域限定至当前会话。
- 当为 `Some` 时，`SessionOptions::compaction_settings` 会按原样覆盖由配置/模型派生的压缩设置。

## 已验证参考表面（Verified Reference Surfaces）

- `src/sdk.rs`
- `tests/sdk_api.rs`
- `tests/sdk_unit.rs`
- `tests/sdk_integration.rs`
