# 提供方测试义务（Provider Test Obligations）

> pi_agent_rust 中每个提供方的强制测试类别、覆盖率下限与制品要求。

---

## 概览（Overview）

代码库中的每个提供方都携带一个 `ProviderTestObligations` 结构体（定义于 `src/provider_metadata.rs`），用于声明需要哪些测试层级：

```rust
pub struct ProviderTestObligations {
    pub unit: bool,      // Unit-level checks (identity, request mapping, auth, tools)
    pub contract: bool,  // Wire-format contract tests with mock HTTP
    pub conformance: bool, // VCR cassette-based streaming conformance
    pub e2e: bool,       // End-to-end agent loop tests
}
```

常量 `TEST_REQUIRED` 将四项均设为 `true`，是 `PROVIDER_METADATA` 中每个提供方的默认值。CI 通过 `tests/provider_unit_checklist.rs` 强制执行这些义务。

---

## 层级 1：单元测试（Checklist）

**强制执行：** `tests/provider_unit_checklist.rs`
**范围：** 纯逻辑校验，无 HTTP、无 VCR、无异步运行时。

### 必需的测试类别

每个原生提供方必须通过全部六个 checklist 类别。若新增原生提供方时跳过任一类别，元测试 `checklist_all_native_providers_enumerated` 将导致 CI 失败。

| # | 类别 | 校验内容 | 宏/模式 |
|---|------|----------|---------|
| 1 | **Identity（身份）** | `name()`、`api()`、`model_id()` 返回非空且正确的值 | `checklist_all_native_providers_have_identity`（单个测试枚举所有提供方） |
| 2 | **Request mapping（请求映射）** | `build_request()` 生成至少含一个字段的可序列化有效 JSON | `checklist_request_mapping!` 宏（每个提供方） |
| 3 | **Auth/header composition（认证/请求头组合）** | 来自 `StreamOptions.api_key` 的 API 密钥流入请求（不会被静默丢弃） | `checklist_*_auth_key_flows_through` 测试 |
| 4 | **URL/endpoint resolution（URL/端点解析）** | `api()` 为该提供方返回预期的 API 类型字符串 | `checklist_providers_have_default_endpoint`（单个测试枚举所有） |
| 5 | **Tool-call serialization（工具调用序列化）** | 携带 `ToolDef` 条目的 `build_request()` 生成提供方特定的工具线缆格式 | `checklist_tool_serialization!` 宏（每个提供方） |
| 6 | **VCR 夹具存在性** | 至少存在 N 个匹配 `verify_{provider}_*.json` 的 VCR 磁带 | `checklist_vcr_fixture_coverage_floor` |

### VCR 覆盖率下限（按提供方）

| 提供方 | 最小磁带数 | 原因 |
|----------|---------------|------|
| anthropic | 3 | simple_text + tool_call + error_auth |
| openai | 3 | simple_text + tool_call + error_auth |
| gemini | 3 | simple_text + tool_call + error_auth |
| cohere | 3 | simple_text + tool_call + error_auth |
| azure | 1 | 至少 simple_text |
| bedrock | 1 | 至少 simple_text |
| vertex | 1 | 至少 simple_text |
| copilot | 1 | 至少 simple_text |
| gitlab | 1 | 至少 simple_text |

### 将新提供方加入 Checklist

1. 在 `checklist_all_native_providers_have_identity` 的 `providers` vec 中加入该提供方
2. 添加一处 `checklist_request_mapping!` 调用
3. 使用正确的工具定义 JSON 路径添加一处 `checklist_tool_serialization!` 调用
4. 添加认证透传测试
5. 将该提供方加入 `checklist_providers_have_default_endpoint` 的 `known_providers`
6. 将该提供方加入 `checklist_vcr_fixture_coverage_floor` 的 `required_providers`
7. 更新 `checklist_all_native_providers_enumerated` 中的计数

### 提供方特定的工具 JSON 路径

不同提供方在不同的 JSON 路径上序列化工具定义：

| 提供方 | 工具 JSON 路径 |
|----------|---------------|
| Anthropic | `tools[]`（含 `input_schema`） |
| OpenAI | `tools[]`（含 `function.parameters`） |
| Gemini | `tools[0].functionDeclarations[]` |
| Cohere | `tools[]`（含 `function`） |
| Bedrock | `toolConfig.tools[]` |

---

## 层级 2：契约测试（Mock HTTP）

**强制执行：** `tests/provider_native_contract.rs`
**范围：** 使用测试脚手架中的 `MockHttpRequest`/`MockHttpResponse` 进行完整的请求-响应周期。测试异步运行，但使用预制的 HTTP 响应，而非真实网络。

### 必需场景

每个原生提供方必须具备覆盖以下场景的契约测试：

| # | 场景 | 校验内容 |
|---|------|----------|
| 1 | **简单文本响应** | 提供方将流式文本响应正确解码为 `TextDelta` + `Done` 事件 |
| 2 | **工具调用响应** | 提供方将工具调用响应解码为 `ToolCallStart`/`ToolCallDelta`/`ToolCallEnd` 事件 |
| 3 | **认证请求头构造** | 发送正确的认证请求头（`Authorization: Bearer`、`X-API-Key`、`api-key` 等） |
| 4 | **请求载荷形态** | URL 路径、Content-Type 与 body JSON 结构符合提供方规范 |

### 测试基础设施

```rust
// Helper to create a ModelEntry for contract tests
fn make_model_entry(provider: &str, model_id: &str, base_url: &str) -> ModelEntry;

// Helper to create SSE-formatted mock responses
fn text_event_stream_response(body: String) -> MockHttpResponse;

// Helper to inspect captured request
fn request_header(headers: &[(String, String)], key: &str) -> Option<String>;
fn request_body_json(request: &MockHttpRequest) -> serde_json::Value;

// Drive a provider stream to completion and collect all events
fn collect_stream_events(provider, context, options) -> Vec<StreamEvent>;
```

### SSE Body 生成器

每个提供方都有专用的 SSE body 生成函数，用于生成匹配该提供方线缆格式的有效 SSE 数据：

- `anthropic_simple_sse()` / `anthropic_tool_call_sse()`
- `openai_simple_sse()` / `openai_tool_call_sse()`
- `gemini_simple_sse()` / `gemini_tool_call_sse()`
- `cohere_simple_sse()` / `cohere_tool_call_sse()`
- `azure_simple_sse()` / `azure_tool_call_sse()`
- 等

这些生成器为该场景生成最小有效 SSE 流。它们是各提供方线缆格式的规范参考。

---

## 层级 3：错误路径测试（VCR）

**强制执行：** `tests/provider_error_paths.rs`
**范围：** 使用 VCR 磁带回放，对 HTTP 错误处理与畸形 SSE 进行确定性离线测试。

### 必需的错误场景

| # | 场景 | 状态码 | 预期行为 |
|---|------|--------|----------|
| 1 | **HTTP 500** | 500 | `stream()` 返回包含 "HTTP 500" 与响应正文的 `Err` |
| 2 | **错误的 Content-Type** | 200 | `stream()` 返回带 "protocol error" 与 "content-type" 的 `Err` |
| 3 | **缺失 Content-Type** | 200 | `stream()` 返回带 "missing content-type" 的 `Err` |
| 4 | **SSE 中的无效 JSON** | 200 | 流产生带 "JSON" 或 "parse" 的 `Err` |
| 5 | **无效 UTF-8** | 200 | 流产生带 "SSE error" 的 `Err`（使用 base64 的 VCR body 块） |

### VCR 磁带辅助函数

```rust
fn vcr_client(
    test_name: &str,           // Cassette file name
    url: &str,                 // Expected request URL
    request_body: Value,       // Expected request body (must match exactly)
    status: u16,               // HTTP response status
    response_headers: Vec<(String, String)>,
    response_chunks: Vec<String>,
) -> (Client, TempDir);       // Client with VCR + temp dir (keep alive!)
```

对于无效 UTF-8 测试，请使用接受 `Vec<Vec<u8>>` 并在磁带中编码为 base64 的 `vcr_client_bytes()`。

### 提供方特定的请求体构造器

每个提供方都有一个请求体构造器，用于生成该提供方实际序列化的精确 JSON：

```rust
fn anthropic_body(model: &str, prompt: &str) -> Value;  // messages + model + stream + max_tokens
fn openai_body(model: &str, prompt: &str) -> Value;     // messages + model + stream + stream_options
fn gemini_body(prompt: &str) -> Value;                   // contents + generationConfig
fn azure_body(prompt: &str) -> Value;                    // messages + stream + stream_options (no model)
```

这些必须与提供方的 `stream()` 方法序列化的内容完全一致，因为 VCR 匹配会逐字段比较请求体。

---

## 层级 4：流式一致性测试（VCR 磁带）

**强制执行：** `tests/provider_streaming.rs` + `tests/provider_streaming/` 下的按提供方子模块
**范围：** 针对真实 API 格式，使用 VCR 磁带录制/回放的完整流式往返。

### 子模块结构

```
tests/provider_streaming.rs        # Root: shared helpers, VCR config, StreamOutcome/StreamSummary
tests/provider_streaming/
    anthropic.rs                   # Anthropic-specific streaming tests
    openai.rs                      # OpenAI Chat Completions tests
    openai_responses.rs            # OpenAI Responses API tests
    gemini.rs                      # Gemini streaming tests
    azure.rs                       # Azure OpenAI streaming tests
    cohere.rs                      # Cohere streaming tests
```

### 每个提供方的必需场景

| # | 场景 | VCR 磁带模式 | 校验内容 |
|---|------|-------------|----------|
| 1 | **简单文本** | `verify_{provider}_simple_text.json` | 基础文本流式：Start → TextDelta(s) → Done |
| 2 | **工具调用** | `verify_{provider}_tool_call_single.json` | 工具使用：ToolCallStart → ToolCallDelta(s) → ToolCallEnd → Done |
| 3 | **Unicode 文本** | `verify_{provider}_unicode_text.json` | 非 ASCII 内容在流式过程中的保留 |
| 4 | **认证错误 (401)** | `verify_{provider}_error_auth_401.json` | 认证失败产生 Error 事件或流错误 |
| 5 | **错误请求 (400)** | `verify_{provider}_error_bad_request_400.json` | 畸形请求产生 Error 事件 |
| 6 | **限流 (429)** | `verify_{provider}_error_rate_limit_429.json` | 限流产生 Error 事件 |

### StreamEvent 序列校验

`StreamSummary` 结构体跟踪完整的事件时间线：

```rust
pub struct StreamSummary {
    pub timeline: Vec<String>,    // Ordered event type names
    pub event_count: usize,
    pub has_start: bool,          // Must be true for successful streams
    pub has_done: bool,           // Must be true for completed streams
    pub has_error_event: bool,    // True for error scenarios
    pub text: String,             // Accumulated text content
    pub thinking: String,         // Accumulated thinking content
    pub tool_calls: Vec<ToolCall>,
    pub text_deltas: usize,       // Count of TextDelta events
    pub stop_reason: Option<StopReason>,
    pub stream_error: Option<String>,
}
```

### VCR 模式

| 模式 | 环境变量 | 行为 |
|------|---------|------|
| **回放**（默认） | `VCR_MODE=playback` 或未设置 | 从磁带文件回放 |
| **录制** | `VCR_MODE=record` | 录制真实 API 交互（需要 API 密钥） |
| **自动** | `VCR_MODE=auto` | 若磁带可用则使用，否则录制 |
| **严格** | `VCR_STRICT=1` | 请求不匹配时失败，而非透传 |

### VCR 磁带位置

所有 VCR 磁带位于 `tests/fixtures/vcr/`，并遵循以下命名约定：
```
verify_{provider}_{scenario}.json
```

当前磁带库存包含 90+ 个文件，覆盖 10 个原生提供方实现模块以及 OpenAI 兼容预设（alibaba-cn、kimi-for-coding、minimax、modelscope、moonshotai-cn、nebius、ovhcloud、scaleway、sap_ai_core、siliconflow、upstage、venice、zai、zhipuai-coding-plan 等）。

---

## 层级 5：端到端测试

**强制执行：** `tests/e2e_*.rs`
**范围：** 携带提供方（VCR 回放）的完整智能体循环，端到端验证多轮对话与工具使用场景。

### 端到端测试方法

- 使用 VCR 回放（`VCR_MODE=playback`）避免真实 API 调用
- 设置 `PI_TEST_MODE=1` 以获得确定性系统提示
- 使用 `--thinking off` 获得确定性测试行为
- 使用隔离标志：`--no-tools --no-extensions --no-skills --no-prompt-templates --no-themes`

### 原生提供方的端到端要求

每个原生提供方应至少有一个端到端测试，用于演示：
1. 完整的智能体轮次（用户消息 → 提供方响应 → 渲染输出）
2. 通过智能体循环的正确 `StreamEvent` 转换
3. 交互的会话持久化

---

## OpenAI 兼容预设义务

带有 `onboarding: OpenAICompatiblePreset` 的提供方通过共享的 `OpenAIProvider` 或 `OpenAIResponsesProvider` 路由。其测试义务更轻：

| 义务 | 是否必需？ | 需提供内容 |
|------------|-----------|-----------------|
| 单元（身份） | 否（由共享 OpenAI 测试覆盖） | N/A |
| 单元（请求映射） | 否（由共享 OpenAI 测试覆盖） | N/A |
| VCR 磁带 | **是** | 至少：`verify_{provider}_simple_text.json`、`verify_{provider}_error_auth_401.json`、`verify_{provider}_tool_call_single.json` |
| 错误路径 | 否（由共享 OpenAI 错误测试覆盖） | N/A |
| 端到端 | 否（由共享 OpenAI 端到端测试覆盖） | N/A |

这些 VCR 磁带用于验证提供方的实际 API 响应格式是否与 OpenAI 解析器兼容。这能捕获那些声称兼容 OpenAI 但存在细微线缆格式差异的提供方。

---

## 测试辅助函数参考

### `tests/common/` 共享基础设施

| 模块 | 用途 |
|--------|---------|
| `common/harness.rs` | `TestHarness`，含 JSONL 日志、制品跟踪、`MockHttpRequest`/`MockHttpResponse` |
| `common/mod.rs` | 用于在测试中阻塞等待异步代码的 `run_async()` 辅助函数 |

### 关键辅助函数

```rust
// Minimal context with one user message
fn minimal_context() -> Context;
fn simple_context() -> Context;

// Context with one or two ToolDef entries
fn context_with_tools() -> Context;

// StreamOptions with test API key
fn default_options() -> StreamOptions;
fn options_with_key(key: &str) -> StreamOptions;

// Count VCR cassettes matching a provider prefix
fn count_cassettes(provider_prefix: &str) -> usize;

// Collect all stream events until Done
fn collect_stream_events(provider, context, options) -> Vec<StreamEvent>;
async fn collect_events<S: Stream>(stream: S) -> StreamOutcome;

// Summarize event timeline for assertions
fn summarize_events(outcome: &StreamOutcome) -> StreamSummary;
```

---

## 运行提供方测试

```bash
# All provider tests (unit + contract + conformance + error paths)
cargo test provider

# Specific provider
cargo test anthropic
cargo test openai
cargo test gemini

# Unit checklist only
cargo test provider_unit_checklist

# Contract tests only
cargo test provider_native_contract

# Error path tests only
cargo test provider_error_paths

# Streaming conformance (VCR playback)
cargo test provider_streaming

# Streaming conformance for one provider
cargo test provider_streaming::anthropic_

# Record new VCR cassettes (requires API key)
ANTHROPIC_API_KEY=sk-ant-... VCR_MODE=record cargo test provider_streaming::anthropic_
```

---

## 为新提供方添加测试：分步指南

### 针对原生提供方

1. **单元 checklist**（`tests/provider_unit_checklist.rs`）：
   - 在 `checklist_all_native_providers_have_identity` 的 providers vec 中加入该提供方
   - 添加一处 `checklist_request_mapping!` 调用
   - 使用正确的工具定义 JSON 路径添加一处 `checklist_tool_serialization!` 调用
   - 添加认证透传测试
   - 将该提供方加入 `checklist_providers_have_default_endpoint` 的 known_providers vec
   - 将该提供方加入 `checklist_vcr_fixture_coverage_floor` 的 required_providers vec
   - 递增 `checklist_all_native_providers_enumerated` 中的计数

2. **契约测试**（`tests/provider_native_contract.rs`）：
   - 添加 SSE body 生成函数（例如 `your_provider_simple_sse()`）
   - 添加简单文本流式测试
   - 添加工具调用流式测试
   - 添加认证请求头校验测试

3. **错误路径测试**（`tests/provider_error_paths.rs`）：
   - 添加请求体构造器（例如 `your_provider_body()`）
   - 添加 HTTP 500 测试
   - 添加畸形 SSE 测试

4. **流式一致性**（`tests/provider_streaming/`）：
   - 创建 `tests/provider_streaming/your_provider.rs`
   - 在 `tests/provider_streaming.rs` 中添加 `#[path = "provider_streaming/your_provider.rs"] mod your_provider;`
   - 为全部 6 个场景录制 VCR 磁带
   - 编写对 `StreamSummary` 字段进行断言的测试

5. **VCR 磁带**（`tests/fixtures/vcr/`）：
   - `verify_your_provider_simple_text.json`
   - `verify_your_provider_tool_call_single.json`
   - `verify_your_provider_unicode_text.json`
   - `verify_your_provider_error_auth_401.json`
   - `verify_your_provider_error_bad_request_400.json`
   - `verify_your_provider_error_rate_limit_429.json`

### 针对 OpenAI 兼容预设

1. **VCR 磁带**（`tests/fixtures/vcr/`）：
   - `verify_your_provider_simple_text.json`
   - `verify_your_provider_tool_call_single.json`
   - `verify_your_provider_error_auth_401.json`

2. 在 `checklist_vcr_fixture_coverage_floor` 中加入 VCR 覆盖率下限（若该提供方足够重要，需要 CI 强制保障）

---

## 常见陷阱

1. **VCR 请求体不匹配**：VCR 匹配会精确比较请求体（经 JSON 规范化后）。若你的提供方添加了额外字段（例如 `stream_options`），测试请求体构造器也必须包含它们。

2. **在 VCR 测试中遗漏 `_dir`**：`vcr_client()` 返回的 `TempDir` 必须在测试期间保持存活。若过早丢弃，磁带文件会被删除，回放失败。

3. **缺失 `oauth_config: None`**：测试中每次构造 `ModelEntry` 都必须包含 `oauth_config: None`。

4. **提供方特定的 URL 模式**：部分提供方以不同方式拼接路径。Gemini 将 `?alt=sse&key=...` 作为查询参数附加。Azure 使用完全不同的 URL 结构。请确保测试 URL 与提供方实际发送的一致。

5. **用于无效 UTF-8 的 Base64 body 块**：标准 VCR 磁带将响应体存储为 UTF-8 字符串。对于需要原始字节（无效 UTF-8）的测试，请使用带 `body_chunks_base64` 的 `vcr_client_bytes()`。

6. **`common::run_async` vs `asupersync::test_utils::run_test`**：契约与错误路径测试为简洁起见使用 `common::run_async()`。流式一致性测试可使用完整运行时。两种模式均可接受。
