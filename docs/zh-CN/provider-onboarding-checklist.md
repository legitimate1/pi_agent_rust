# 提供方接入清单

> 在 pi_agent_rust 中添加或维护提供方的规范分步指南。

---

## 前置条件

开始前，请先确定你的提供方的**接入模式**（定义于 `src/provider_metadata.rs`）：

| 模式                     | 适用场景                                               | 示例                                                               |
| ------------------------ | ------------------------------------------------------ | ------------------------------------------------------------------ |
| `OpenAICompatiblePreset` | 提供方暴露了 OpenAI 兼容的 `/v1/chat/completions` 端点 | Groq, Cerebras, OpenRouter, Mistral, DeepSeek, Together, Fireworks |
| `BuiltInNative`          | 提供方使用非 OpenAI 线路格式，需要独立实现             | Anthropic, Google Gemini, Cohere                                   |
| `NativeAdapterRequired`  | 提供方需要自定义认证流程或非标准请求/响应处理          | Azure OpenAI, Amazon Bedrock, GitHub Copilot, GitLab Duo           |

大多数新提供方属于 **OpenAI 兼容预设**，除元数据注册外无需改动 Rust 代码。

---

## 路径 A：OpenAI 兼容预设（无需新增 Rust 代码）

### 步骤 1：添加提供方元数据

**文件：** `src/provider_metadata.rs`（位于 `PROVIDER_METADATA` 数组中）

添加一条新的 `ProviderMetadata` 条目：

```rust
ProviderMetadata {
    canonical_id: "your-provider",           // Lowercase, hyphen-separated
    aliases: &["alias1", "alias2"],          // Alternative names users might type
    auth_env_keys: &["YOUR_PROVIDER_API_KEY"], // Env var(s) for API key lookup
    onboarding: ProviderOnboardingMode::OpenAICompatiblePreset,
    routing_defaults: Some(ProviderRoutingDefaults {
        api: "openai-completions",           // Or "openai-responses" if supported
        base_url: "https://api.your-provider.com/v1",
        auth_header: true,                   // true = Authorization: Bearer <key>
        reasoning: true,                     // Does the provider support reasoning models?
        input: &INPUT_TEXT,                  // Or &INPUT_TEXT_IMAGE if multimodal
        context_window: 128_000,             // Default context window
        max_tokens: 16_384,                  // Default max output tokens
    }),
    test_obligations: TEST_REQUIRED,
}
```

**位置：** 在对应批次分区（Batch A1、A2、A3 等）内按字母顺序添加。

**关键决策：**

- `api`：对标准 Chat Completions API 使用 `"openai-completions"`，对 OpenAI Responses API 使用 `"openai-responses"`
- `auth_header`：`true` 表示密钥以 `Authorization: Bearer <key>` 形式发送。`false` 表示使用提供方特定的认证方式（例如 query 参数）
- `input`：`&INPUT_TEXT` 适用于纯文本，`&INPUT_TEXT_IMAGE` 适用于多模态

### 步骤 2：添加提供方枚举变体（按需）

**文件：** `src/provider.rs`

若希望提供方出现在 `KnownProvider` 枚举中以实现类型安全匹配：

```rust
// In KnownProvider enum
YourProvider,

// In Display impl
Self::YourProvider => write!(f, "your-provider"),

// In FromStr impl
"your-provider" => Ok(Self::YourProvider),
```

> **注意：** 此步骤对 OpenAI 兼容预设为可选。`Custom(String)` 回退会自动处理未知提供方。

### 步骤 3：在 README 中添加环境变量

**文件：** `README.md`（Environment Variables 表格）

```markdown
| `YOUR_PROVIDER_API_KEY` | Your Provider API key |
```

### 步骤 4：添加模型条目（可选）

**文件：** 用户的 `~/.pi/agent/models.json` 或内置注册表

若提供方拥有知名模型，请在 `models.json` 中添加条目：

```json
{
  "providers": {
    "your-provider": {
      "models": [
        {
          "id": "your-model-v1",
          "name": "Your Model v1",
          "reasoning": true,
          "contextWindow": 128000,
          "maxTokens": 16384,
          "cost": {
            "input": 1.0,
            "output": 3.0,
            "cacheRead": 0.1,
            "cacheWrite": 1.5
          }
        }
      ]
    }
  }
}
```

模型注册表（`src/models.rs`）会将用户的 `models.json` 与内置默认值合并。提供方级别字段（`baseUrl`、`api`、`apiKey`、`headers`、`authHeader`、`compat`）会级联到该提供方下的所有模型。

### 步骤 5：验证路由

运行快速冒烟测试以确认路由解析正确：

```bash
cargo test provider_metadata::tests -- --nocapture
```

检查 `canonical_provider_id("your-provider")` 是否返回 `Some("your-provider")`，且 `provider_routing_defaults("your-provider")` 是否返回预期默认值。

### 步骤 6：运行质量门

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## 路径 B：内置原生提供方（新增 Rust 实现）

完成**路径 A 的所有步骤**后，继续：

### 步骤 7：创建提供方模块

**文件：** `src/providers/<name>.rs`

实现 `Provider` trait：

```rust
use crate::error::{Error, Result};
use crate::http::client::Client;
use crate::model::{AssistantMessage, ContentBlock, StopReason, StreamEvent, TextContent, Usage};
use crate::models::CompatConfig;
use crate::provider::{Context, Provider, StreamOptions};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

pub struct YourProvider {
    model_id: String,
    base_url: String,
    provider_name: String,
    client: Client,
    compat: Option<CompatConfig>,
}

impl YourProvider {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            base_url: "https://api.your-provider.com/v1".to_string(),
            provider_name: "your-provider".to_string(),
            client: Client::new(),
            compat: None,
        }
    }

    // Builder methods following the established pattern:
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    pub fn with_provider_name(mut self, name: String) -> Self {
        self.provider_name = name;
        self
    }

    pub fn with_compat(mut self, compat: Option<CompatConfig>) -> Self {
        self.compat = compat;
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }
}

#[async_trait]
impl Provider for YourProvider {
    fn name(&self) -> &str { &self.provider_name }
    fn api(&self) -> &str { "your-api-type" }
    fn model_id(&self) -> &str { &self.model_id }

    async fn stream(
        &self,
        context: &Context,
        options: &StreamOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        // 1. Build provider-specific request body from context + options
        // 2. Resolve API key from options.api_key
        // 3. Send HTTP request via self.client
        // 4. Parse SSE/streaming response into StreamEvent items
        // 5. Return as a Stream
        todo!()
    }
}
```

**需发出的 `StreamEvent` 变体：**

- `StreamEvent::TextDelta { text }` - 每个文本片段
- `StreamEvent::ThinkingDelta { text }` - 思考/推理令牌（若支持）
- `StreamEvent::ToolCall { id, name, arguments }` - 用于工具调用
- `StreamEvent::Done { reason, message }` - 携带完整 `AssistantMessage` 的结束事件

### 步骤 8：在提供方工厂中注册

**文件：** `src/providers/mod.rs`

1. 添加模块声明：

   ```rust
   pub mod your_provider;
   ```

2. 在 `ProviderRouteKind` 中添加路由变体：

   ```rust
   NativeYourProvider,
   ```

3. 添加 `as_str()` 匹配分支：

   ```rust
   Self::NativeYourProvider => "native:your-provider",
   ```

4. 在 `resolve_provider_route()` 中添加路由：

   ```rust
   "your-provider" => ProviderRouteKind::NativeYourProvider,
   ```

5. 在 `create_provider()` 中添加构造逻辑：
   ```rust
   ProviderRouteKind::NativeYourProvider => Ok(Arc::new(
       your_provider::YourProvider::new(entry.model.id.clone())
           .with_base_url(entry.model.base_url.clone())
           .with_compat(entry.compat.clone())
           .with_client(client),
   )),
   ```

### 步骤 9：添加 URL 规范化（按需）

若提供方的 base URL 需要规范化（例如追加 `/chat/completions`），请添加辅助函数：

```rust
pub fn normalize_your_provider_base(base_url: &str) -> String {
    // See normalize_openai_base() and normalize_cohere_base() for patterns
}
```

---

## 路径 C：需要原生适配器（自定义认证/运行时）

完成**路径 B 的所有步骤**后，继续：

### 步骤 10：实现自定义认证

**文件：** `src/auth.rs`

若提供方使用非标准认证（OAuth、服务密钥、device flow）：

1. 添加认证常量（client ID、URL、scope）
2. 实现认证流程函数（例如 `start_your_provider_oauth()`、`complete_your_provider_oauth()`）
3. 按需在 `AuthCredential` 枚举中添加凭据类型变体

**可参考的现有认证模式：**

- **OAuth（基于浏览器）：** 参考 Anthropic OAuth（`start_anthropic_oauth`、`complete_anthropic_oauth`）
- **Device flow：** 参考 GitHub Copilot（`start_github_device_flow`）
- **服务密钥（client credentials）：** 参考 SAP AI Core（`resolve_sap_credentials`）
- **AWS IAM：** 参考 Bedrock（`resolve_aws_credentials_async`）

### 步骤 11：添加运行时解析

**文件：** `src/providers/mod.rs`

若提供方除 base URL 外还需要运行时配置（例如 Azure 需要 resource/deployment/api-version）：

```rust
fn resolve_your_provider_runtime(entry: &ModelEntry) -> Result<YourProviderRuntime> {
    // Extract config from entry.model.base_url, env vars, etc.
}
```

模式请参考 `resolve_azure_provider_runtime()` 与 `vertex::resolve_vertex_provider_runtime()`。

---

## 认证解析链

API 密钥解析遵循以下优先级（命中即止）：

1. **CLI 覆盖：** `--api-key <KEY>` 标志
2. **环境变量：** 来自元数据中 `auth_env_keys` 的提供方专属环境变量
3. **认证存储（auth.json）：** 来自 `/login` 命令保存的凭据
4. **规范回退：** 若提供方存在别名，则在认证存储中尝试规范 ID

**代码路径：** `src/auth.rs:316` 中的 `AuthStorage::resolve_api_key()` → `src/app.rs:629` 中的 `app::resolve_api_key()`

---

## `CompatConfig` 参考

当提供方的线路格式偏离标准时，请在 `models.json` 中使用 `CompatConfig` 覆盖：

| 字段                           | 类型     | 用途                                                   |
| ------------------------------ | -------- | ------------------------------------------------------ |
| `supports_store`               | `bool`   | 提供方是否支持在请求中使用 `store: true`               |
| `supports_developer_role`      | `bool`   | 提供方是否接受 `developer` 角色而非 `system`           |
| `supports_reasoning_effort`    | `bool`   | 提供方是否支持 `reasoning_effort` 参数                 |
| `supports_usage_in_streaming`  | `bool`   | 使用量统计是否在流式事件中到达（而非仅在结束时）       |
| `supports_tools`               | `bool`   | 提供方是否支持工具调用                                 |
| `supports_streaming`           | `bool`   | 提供方是否支持流式响应                                 |
| `supports_parallel_tool_calls` | `bool`   | 提供方是否可在单轮中并行调用多个工具                   |
| `max_tokens_field`             | `String` | 覆盖字段名（例如对 o1 使用 `"max_completion_tokens"`） |
| `system_role_name`             | `String` | 覆盖系统角色名（例如某些提供方使用 `"developer"`）     |
| `stop_reason_field`            | `String` | 覆盖响应中的 stop-reason 字段名                        |

---

## 测试要求

每个提供方必须满足其元数据中定义的测试义务（`ProviderTestObligations`）：

### 单元测试

**位置：** `src/providers/<name>.rs`（内联 `#[cfg(test)] mod tests`）

- 请求体构造（验证 JSON 是否符合提供方规范）
- 响应解析（合法与畸形响应）
- URL 规范化
- 认证头注入

### 契约测试

**位置：** `tests/provider_native_contract.rs` 或 `tests/provider_streaming.rs`

- 基于 VCR 磁带的测试，验证真实 API 线路格式
- 工具调用往返
- 错误响应处理（认证错误、限流、畸形响应）

### 一致性测试

**位置：** `tests/fixtures/provider_streaming/`

- 包含已录制 API 交互的夹具文件
- 验证 `StreamEvent` 序列是否符合预期

### 端到端测试

**位置：** `tests/e2e_*.rs` 或 `scripts/e2e/`

- 携带提供方的完整智能体循环（VCR 回放）
- 多轮对话
- 工具使用场景

### 运行提供方测试

```bash
# All provider tests
cargo test provider

# Specific provider
cargo test anthropic
cargo test openai
cargo test gemini

# Contract tests
cargo test provider_native_contract

# Streaming conformance
cargo test provider_streaming
```

---

## 证据与产物更新

添加提供方后，请更新以下产物：

| 产物             | 位置                                   | 更新内容                     |
| ---------------- | -------------------------------------- | ---------------------------- |
| 提供方元数据测试 | `src/provider_metadata.rs`（内联测试） | 为新的规范/别名解析添加测试  |
| 提供方路由测试   | `src/providers/mod.rs`（内联测试）     | 为路由解析添加测试           |
| README 环境变量  | `README.md`                            | 在表格中添加环境变量         |
| models.json 模式 | 面向用户的文档                         | 说明可用模型                 |
| CI 脚本          | `.github/workflows/`                   | 按需在测试矩阵中添加环境变量 |

---

## 常见陷阱

1. **遗漏 `auth_env_keys`**：若未在元数据中列出环境变量，认证解析器将无法找到环境变量。

2. **`api` 字段错误**：OpenAI 兼容提供方必须使用 `"openai-completions"` 或 `"openai-responses"`，而非自定义字符串。路由会回退到 `Api::Custom` 并失败。

3. **缺失 URL 规范化**：OpenAI completions 提供方会对未以该路径结尾的 base URL 追加 `/chat/completions`。若你的提供方 base URL 已包含路径，可能会导致路径重复。

4. **`oauth_config: None`**：每个 `ModelEntry` 构造点都必须包含 `oauth_config: None`（OAuth 提供方则为 `Some(...)`）。在 `src/models.rs` 与 `src/extensions.rs` 中约有 9 个构造点。

5. **提供方 ID 大小写敏感**：`provider_metadata()` 查询时使用 `eq_ignore_ascii_case`，但规范 ID 应始终为小写。

6. **Clippy 严格性**：项目使用 `-D warnings` 并启用 pedantic + nursery lints。常见问题：
   - `doc_markdown`：在文档注释中将类型名置于反引号内
   - `too_many_lines`：若无法避免，请添加 `#[allow(clippy::too_many_lines)]`
   - `needless_borrows_for_generic_args`：当 `format!(...)` 已满足需求时，不要使用 `&format!(...)`

7. **VCR 磁带匹配**：使用 VCR 回放的测试要求请求体 JSON 精确匹配。若提供方新增额外字段，需重新录制磁带。

---

## 清单汇总

### OpenAI 兼容预设

- [ ] 在 `src/provider_metadata.rs` 中添加 `ProviderMetadata` 条目
- [ ] 在 `README.md` 中添加环境变量
- [ ] 在 `models.json` 中添加模型条目（若存在知名模型）
- [ ] 运行 `cargo test provider_metadata::tests`
- [ ] 运行 `cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check`

### 内置原生

- [ ] 上述 OpenAI 兼容预设的所有项
- [ ] 创建实现 `Provider` trait 的 `src/providers/<name>.rs`
- [ ] 在 `src/providers/mod.rs` 中添加 `pub mod <name>`
- [ ] 添加 `ProviderRouteKind` 变体及 `as_str()` 匹配
- [ ] 在 `resolve_provider_route()` 中添加路由
- [ ] 在 `create_provider()` 中添加构造逻辑
- [ ] 添加 URL 规范化函数（按需）
- [ ] 在提供方模块中编写单元测试
- [ ] 使用 VCR 磁带编写契约/一致性测试

### 需要原生适配器

- [ ] 上述内置原生的所有项
- [ ] 在 `src/auth.rs` 中实现自定义认证流程
- [ ] 在 `src/providers/mod.rs` 中添加运行时解析函数
- [ ] 端到端测试认证流程
- [ ] 在排障文档中说明认证配置
