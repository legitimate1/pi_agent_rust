# 提供方接入手册

本手册是 `providers.md` 的面向执行的配套文档。

在以下场景中使用本手册：
- 快速接入提供方配置，
- 无需猜测即可调试提供方认证/路由失败，以及
- 在不产生元数据/工厂漂移的前提下新增或更新提供方支持。

主要 Beads 覆盖：
- `bd-3uqg.9`（父任务）
- 对 `bd-3uqg.9.2` 和 `bd-3uqg.9.3` 的工作草案支持
- `bd-3uqg.9.4.1`（下方清单）

## 快速清单：新增提供方

### 确定接入模式

| 模式 | 适用场景 | 是否需要专用 `.rs` 文件？ | 示例 |
|---|---|---|---|
| `OpenAICompatiblePreset` | 标准 OpenAI 兼容 API | 否 | groq, deepinfra, mistral |
| `BuiltInNative` | 私有 API 格式 | 是 | anthropic, google, cohere |
| `NativeAdapterRequired` | 特殊认证或路由 | 是 | azure, bedrock, copilot, gitlab |

### 阶段 1：元数据（`src/provider_metadata.rs`）

- [ ] 向 `PROVIDER_METADATA` 数组添加 `ProviderMetadata` 条目
  - `canonical_id`：主规范提供方名称，小写
  - `aliases`：别名（如 google 的 `["gemini"]`）
  - `auth_env_keys`：按优先级排序的环境变量链（如 `["GROQ_API_KEY"]`）
  - `onboarding`：上述三种模式之一
  - `routing_defaults`：`Some(ProviderRoutingDefaults { api, base_url, auth_header, reasoning, input, context_window, max_tokens })` 或原生适配器填 `None`
  - `test_obligations`：通常为 `TEST_REQUIRED`
- [ ] 验证：`provider_metadata("your-id")` 返回对应条目
- [ ] 验证：别名可通过 `canonical_provider_id("alias")` 解析

### 阶段 2：工厂路由（`src/providers/mod.rs`）——仅原生提供方

`OpenAICompatiblePreset` 提供方可跳过本阶段。

- [ ] 在模块声明中添加 `pub mod {provider};`（约第 25 行）
- [ ] 向 `ProviderRouteKind` 枚举添加变体（约第 70 行）
- [ ] 在 `as_str()` 匹配中添加变体字符串（约第 89 行）
- [ ] 在 `resolve_provider_route()` 中添加规范 ID 模式（约第 121 行）
- [ ] 在 `create_provider()` 中添加实例化分支（约第 679 行）

### 阶段 3：实现（`src/providers/{provider}.rs`）——仅原生提供方

`OpenAICompatiblePreset` 提供方可跳过本阶段。

- [ ] 创建包含以下字段的结构体：`client: Client`、`model_id: String`、`provider_name: String`、`base_url: String`、`compat: Option<CompatConfig>`
- [ ] 实现构建方法：`new()`、`with_base_url()`、`with_provider_name()`、`with_compat()`、`with_client()`
- [ ] 实现 `Provider` trait：`name()`、`api()`、`model_id()`、`stream()`
- [ ] 处理流式响应解析（JSON -> `StreamEvent` 变体）
- [ ] 处理错误响应（认证、限流、服务端错误）

### 阶段 4：认证（`src/auth.rs`）

- [ ] 简单 API 密钥：无需改动（通过 `auth_env_keys` 元数据驱动）
- [ ] AWS SigV4（Bedrock 风格）：使用 `resolve_aws_credentials()`
- [ ] OAuth / 令牌交换：在提供方 `.rs` 或 `auth.rs` 中实现

### 阶段 5：测试

- [ ] `tests/provider_factory.rs`：工厂实例化测试
- [ ] `tests/provider_metadata_comprehensive.rs`：元数据查找 + 别名测试
- [ ] `tests/provider_native_contract.rs`：基于 VCR 的流式测试（原生提供方）
- [ ] `tests/provider_native_verify.rs`：一致性验证（原生提供方）
- [ ] 如需，在 `tests/fixtures/vcr/` 中创建 VCR 磁带
- [ ] 运行：`cargo test --test provider_factory --test provider_metadata_comprehensive`

### 阶段 6：验证

- [ ] `cargo check --all-targets`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt --check`
- [ ] `cargo test --lib`（全部 3269+ 测试通过）
- [ ] 提供方出现在 `pi --list-models` 中（已设置 API 密钥）
- [ ] `pi --provider {id} --model {model} -p "test"` 可用（如有可用真实密钥）

## 范围与权威来源

以下文件为权威来源：
- 提供方元数据（规范 ID、别名、环境变量、路由默认值）：`../src/provider_metadata.rs`
- 运行时路由选择与提供方工厂分发：`../src/providers/mod.rs`
- API 密钥解析优先级：`../src/app.rs`、`../src/auth.rs`、`../src/models.rs`
- 现有提供方基线与矩阵：`providers.md`
- 错误提示分类与修复消息：`../src/error.rs`

以下测试/制品作为验证锚点：
- 工厂/路由行为：`../tests/provider_factory.rs`
- 元数据不变量与别名正确性：`../tests/provider_metadata_comprehensive.rs`
- 流式/提供方契约：`../tests/provider_streaming.rs`
- 真实一致性/制品通道：`../tests/e2e_cross_provider_parity.rs`、`../tests/e2e_live_harness.rs`、`../tests/e2e_live.rs`

## 运行时模型：提供方选择实际如何工作

选择流水线：
1. 在 `../src/app.rs` 中选定模型条目（`--provider/--model`、默认值或作用域模型）。
2. 按以下顺序尝试解析 API 密钥：
   - `--api-key`
   - `auth.json` 中未过期的 OAuth/bearer 凭证
   - 提供方环境变量（经由 `../src/provider_metadata.rs` 的 `provider_auth_env_keys`）
   - `auth.json` 中存储的 API 密钥
   - 受支持的外部 coding-CLI 凭证（仅全局认证存储）
   - `models.json` 的 `providers.<id>.apiKey`（回退）
3. 在 `../src/providers/mod.rs` 的 `resolve_provider_route(...)` 中选择提供方路由。
4. 在 `create_provider(...)` 中创建具体的提供方实现。

重要注意事项：
- `github-copilot` 参与同一通用解析链。其提供方环境变量顺序为 `GITHUB_COPILOT_API_KEY`，其次 `GITHUB_TOKEN`；有效的已存储 bearer/OAuth 凭证仍优先于环境变量，而内联的 `models.json` 提供方密钥仅作为应用层回退。
- `amazon-bedrock` 与 `sap-ai-core` 为请求时/提供方管理的认证路由。不要将 AWS 区域/配置/会话令牌组件或 SAP 客户端凭证记载为原始 bearer API 密钥。

## 提供方家族映射

| 家族 | 典型规范 ID | 路由风格 | 核心配置面 |
|---|---|---|---|
| 内置原生 | `anthropic`、`openai`、`google`、`cohere` | 原生提供方模块 | 通常为 `--provider/--model` + 环境变量 |
| OpenAI 兼容预设 | `openrouter`、`xai`、`deepseek`、`groq`、`cloudflare-ai-gateway`、`cloudflare-workers-ai` 等 | API 回退至 `openai-completions` | 提供方元数据默认值 + 标准 bearer 认证 |
| 原生适配器 | `azure-openai`、`google-vertex`、`github-copilot`、`gitlab`、`amazon-bedrock`、`sap-ai-core` | 工厂中的专用适配器路由 | 提供方特定的环境/配置要求 |

## 复制即用配置示例

### 1) 内置原生提供方（快速 CLI）

```bash
export ANTHROPIC_API_KEY="..."
export OPENAI_API_KEY="..."
export GOOGLE_API_KEY="..."
export COHERE_API_KEY="..."

pi --provider anthropic --model claude-sonnet-4-5 -p "Say hello"
pi --provider openai --model gpt-4o-mini -p "Say hello"
pi --provider google --model gemini-2.5-flash -p "Say hello"
pi --provider cohere --model command-r-plus -p "Say hello"
```

预期检查：
- 命令返回模型文本输出，无提供方/认证错误。

### 2) OpenAI 兼容预设提供方（`models.json` 可选）

OpenRouter 最小路径（仅环境变量）：

```bash
export OPENROUTER_API_KEY="..."
pi --provider openrouter --model openai/gpt-4o-mini -p "Say hello"
```

OpenRouter 高级路径（显式配置 + 路由元数据 + 归因覆盖）：

```json
{
  "providers": {
    "openrouter": {
      "baseUrl": "https://openrouter.ai/api/v1",
      "api": "openai-completions",
      "compat": {
        "openRouterRouting": {
          "provider": { "order": ["anthropic", "openai"] }
        }
      },
      "models": [
        {
          "id": "anthropic/claude-3.5-sonnet",
          "name": "OpenRouter Claude 3.5 Sonnet",
          "compat": {
            "customHeaders": {
              "X-Debug-Trace": "openrouter-doc-example"
            }
          }
        }
      ]
    }
  }
}
```

```bash
export OPENROUTER_API_KEY="..."
# 可选归因覆盖（若缺失则注入默认值）
export OPENROUTER_HTTP_REFERER="https://example.com/pi-agent-rust"
export OPENROUTER_X_TITLE="Pi Agent Rust (Docs Example)"

# 提供方别名与模型别名均受支持：
pi --provider open-router --model claude-3.5-sonnet -p "Say hello"
```

OpenRouter 预期检查：
- OpenRouter 通过 `openai-completions` 解析为 `/chat/completions` 路由形态。
- 提供方别名 `open-router` 解析为规范 `openrouter`。
- 模型别名形式（如 `claude-3.5-sonnet`）规范化为规范 ID。
- 已配置时转发 `openRouterRouting`，且必须为 JSON 对象。
- 除非已被 compat/单次请求头设置，否则默认注入 `HTTP-Referer` 与 `X-Title` 头。

OpenRouter 证据锚点：
- `tests/provider_native_contract.rs` (`openrouter_contract::*`)
- `tests/provider_native_verify.rs` (`openrouter_conformance::*`)
- `tests/e2e_provider_scenarios.rs` (`e2e_openai_compatible_wave_presets`, `e2e_error_auth_all_families`, `e2e_error_rate_limit_all_families`, `e2e_error_schema_drift_all_families`)
- `src/providers/openai.rs` (`test_build_request_applies_openrouter_routing_overrides`, `test_stream_openrouter_injects_default_attribution_headers`, `test_stream_openrouter_respects_explicit_attribution_headers`)
- `tests/main_cli_selection.rs` (`select_model_and_thinking_resolves_model_flag_with_provider_prefixed_openrouter_id`, `select_model_and_thinking_resolves_openrouter_provider_alias_and_model_alias`)

其他预设显式配置示例（Cloudflare AI Gateway）：

```json
{
  "providers": {
    "cloudflare-ai-gateway": {
      "baseUrl": "https://gateway.ai.cloudflare.com/v1/<account_id>/<gateway_id>/openai",
      "models": [
        { "id": "gpt-4o-mini" }
      ]
    }
  }
}
```

```bash
export CLOUDFLARE_API_TOKEN="..."
pi --provider cloudflare-ai-gateway --model gpt-4o-mini -p "Say hello"
```

预期检查：
- 这些提供方的工厂解析为 `openai-completions` 路由（见 `../tests/provider_factory.rs`）。

预设家族的 Wave A 验证锁（`bd-3uqg.4.4`）：
- `wave_a_presets_resolve_openai_compat_defaults_and_factory_route`
- `wave_a_openai_compat_streams_use_chat_completions_path_and_bearer_auth`

### 2a) 别名迁移示例（`fireworks-ai` -> `fireworks`）

遗留配置（仍受支持）：

```json
{
  "providers": {
    "fireworks-ai": {
      "models": [
        { "id": "accounts/fireworks/models/llama-v3p3-70b-instruct" }
      ]
    }
  }
}
```

推荐配置（规范）：

```json
{
  "providers": {
    "fireworks": {
      "models": [
        { "id": "accounts/fireworks/models/llama-v3p3-70b-instruct" }
      ]
    }
  }
}
```

迁移行为保证：
- 两个 ID 均解析为 `openai-completions`，基地址为 `https://api.fireworks.ai/inference/v1`。
- 两个 ID 使用相同的认证环境变量映射（`FIREWORKS_API_KEY`）。
- 别名一致性由 `fireworks_ai_alias_migration_matches_fireworks_canonical_defaults` 锁测保障。

### 2b) Wave B1 规范 ID（区域 + 编程计划）

批次 B1 锁测（`bd-3uqg.5.2`）：
- `wave_b1_presets_resolve_metadata_defaults_and_factory_route`
- `wave_b1_alibaba_cn_openai_compat_streams_use_chat_completions_path_and_bearer_auth`
- `wave_b1_anthropic_compat_streams_use_messages_path_and_x_api_key`
- `wave_b1_family_coherence_with_existing_moonshot_and_alibaba_mappings`

代表性冒烟/端到端检查（`provider_native_verify`）：
- `wave_b1_smoke::b1_alibaba_cn_{simple_text,tool_call_single,error_auth_401}`
- `wave_b1_smoke::b1_kimi_for_coding_{simple_text,tool_call_single,error_auth_401}`
- `wave_b1_smoke::b1_minimax_{simple_text,tool_call_single,error_auth_401}`
- 命令：`cargo test --test provider_native_verify b1_ -- --nocapture`
- 生成的夹具：
  `tests/fixtures/vcr/verify_alibaba-cn_*.json`,
  `tests/fixtures/vcr/verify_kimi-for-coding_*.json`,
  `tests/fixtures/vcr/verify_minimax_*.json`.

关键映射决策：
- `kimi` 仍为规范 `moonshotai` 的别名。
- `kimi-for-coding` 为独立项，路由至 Anthropic 兼容路径，使用 `KIMI_API_KEY`。
- `alibaba-cn` 与 `alibaba`/`dashscope` 区分，使用中国区 DashScope 基地址。
- `minimax*` 变体为独立规范 ID，共享家族认证/环境变量映射：
  `MINIMAX_API_KEY` 用于全球，`MINIMAX_CN_API_KEY` 用于中国区。

代表性 `models.json` 片段：

```json
{
  "providers": {
    "alibaba-cn": {
      "models": [{ "id": "qwen-plus" }]
    },
    "kimi-for-coding": {
      "models": [{ "id": "k2p5" }]
    },
    "minimax-coding-plan": {
      "models": [{ "id": "MiniMax-M2.1" }]
    }
  }
}
```

### 2c) Wave B2 规范 ID（区域 + 云端 OpenAI 兼容）

批次 B2 锁测（`bd-3uqg.5.1`）：
- `wave_b2_presets_resolve_metadata_defaults_and_factory_route`
- `wave_b2_openai_compat_streams_use_chat_completions_path_and_bearer_auth`
- `wave_b2_moonshot_cn_and_global_moonshot_mapping_are_distinct`

代表性冒烟/端到端检查（`provider_native_verify`）：
- `wave_b2_smoke::b2_modelscope_{simple_text,tool_call_single,error_auth_401}`
- `wave_b2_smoke::b2_moonshotai_cn_{simple_text,tool_call_single,error_auth_401}`
- `wave_b2_smoke::b2_nebius_{simple_text,tool_call_single,error_auth_401}`
- `wave_b2_smoke::b2_ovhcloud_{simple_text,tool_call_single,error_auth_401}`
- `wave_b2_smoke::b2_scaleway_{simple_text,tool_call_single,error_auth_401}`
- 命令：`cargo test --test provider_native_verify b2_ -- --nocapture`
- 生成的夹具：
  `tests/fixtures/vcr/verify_modelscope_*.json`,
  `tests/fixtures/vcr/verify_moonshotai-cn_*.json`,
  `tests/fixtures/vcr/verify_nebius_*.json`,
  `tests/fixtures/vcr/verify_ovhcloud_*.json`,
  `tests/fixtures/vcr/verify_scaleway_*.json`.

关键映射决策：
- `modelscope`、`nebius`、`ovhcloud` 与 `scaleway` 作为规范的 OpenAI 兼容预设 ID 接入。
- `moonshotai-cn` 为独立的规范区域 ID，不作为 `moonshotai` 的别名。
- `moonshotai` 与 `moonshotai-cn` 有意共享 `MOONSHOT_API_KEY`，同时保留不同的基地址。

代表性 `models.json` 片段：

```json
{
  "providers": {
    "modelscope": {
      "models": [{ "id": "ZhipuAI/GLM-4.5" }]
    },
    "moonshotai-cn": {
      "models": [{ "id": "kimi-k2-0905-preview" }]
    },
    "nebius": {
      "models": [{ "id": "NousResearch/hermes-4-70b" }]
    },
    "ovhcloud": {
      "models": [{ "id": "mixtral-8x7b-instruct-v0.1" }]
    },
    "scaleway": {
      "models": [{ "id": "qwen3-235b-a22b-instruct-2507" }]
    }
  }
}
```

### 2d) Wave B3 规范 ID（区域 + 编程计划 OpenAI 兼容）

批次 B3 锁测（`bd-3uqg.5.3`）：
- `wave_b3_presets_resolve_metadata_defaults_and_factory_route`
- `wave_b3_openai_compat_streams_use_chat_completions_path_and_bearer_auth`
- `wave_b3_family_and_coding_plan_variants_are_distinct`
- `ad_hoc_batch_b3_defaults_resolve_expected_routes`
- `ad_hoc_batch_b3_coding_plan_and_regional_variants_remain_distinct`

代表性冒烟/端到端检查（`provider_native_verify`）：
- `wave_b3_smoke::b3_siliconflow_{simple_text,tool_call_single,error_auth_401}`
- `wave_b3_smoke::b3_siliconflow_cn_{simple_text,tool_call_single,error_auth_401}`
- `wave_b3_smoke::b3_upstage_{simple_text,tool_call_single,error_auth_401}`
- `wave_b3_smoke::b3_venice_{simple_text,tool_call_single,error_auth_401}`
- `wave_b3_smoke::b3_zai_{simple_text,tool_call_single,error_auth_401}`
- `wave_b3_smoke::b3_zai_coding_{simple_text,tool_call_single,error_auth_401}`
- `wave_b3_smoke::b3_zhipuai_{simple_text,tool_call_single,error_auth_401}`
- `wave_b3_smoke::b3_zhipuai_coding_{simple_text,tool_call_single,error_auth_401}`
- 命令：`cargo test --test provider_native_verify b3_ -- --nocapture`
- 生成的夹具：
  `tests/fixtures/vcr/verify_siliconflow_*.json`,
  `tests/fixtures/vcr/verify_siliconflow-cn_*.json`,
  `tests/fixtures/vcr/verify_upstage_*.json`,
  `tests/fixtures/vcr/verify_venice_*.json`,
  `tests/fixtures/vcr/verify_zai_*.json`,
  `tests/fixtures/vcr/verify_zai-coding-plan_*.json`,
  `tests/fixtures/vcr/verify_zhipuai_*.json`,
  `tests/fixtures/vcr/verify_zhipuai-coding-plan_*.json`.

关键映射决策：
- `siliconflow` 与 `siliconflow-cn` 为独立的规范区域 ID，使用不同的认证环境变量（`SILICONFLOW_API_KEY`、`SILICONFLOW_CN_API_KEY`）。
- `zai` 与 `zai-coding-plan` 为独立规范 ID，共享 `ZHIPU_API_KEY` 但使用不同基地址。
- `zhipuai` 与 `zhipuai-coding-plan` 为独立规范 ID，共享 `ZHIPU_API_KEY` 但使用不同基地址。

代表性 `models.json` 片段：

```json
{
  "providers": {
    "siliconflow": {
      "models": [{ "id": "Qwen/Qwen3-Coder-480B-A35B-Instruct" }]
    },
    "upstage": {
      "models": [{ "id": "solar-pro2" }]
    },
    "venice": {
      "models": [{ "id": "venice-uncensored" }]
    },
    "zai-coding-plan": {
      "models": [{ "id": "glm-4.5" }]
    },
    "zhipuai-coding-plan": {
      "models": [{ "id": "glm-4.5" }]
    }
  }
}
```

### 2e) Wave C 规范 ID（本地/自托管/网关预发布）

本节默认值来源：
- `https://models.dev/api.json`（查询于 2026-02-12）
- 提取命令：

```bash
curl -s https://models.dev/api.json | jq '{
  baseten: {api: ."baseten".api, env: ."baseten".env},
  llama: {api: ."llama".api, env: ."llama".env},
  lmstudio: {api: ."lmstudio".api, env: ."lmstudio".env},
  "ollama-cloud": {api: ."ollama-cloud".api, env: ."ollama-cloud".env},
  opencode: {api: ."opencode".api, env: ."opencode".env},
  vercel: {api: ."vercel".api, env: ."vercel".env},
  zenmux: {api: ."zenmux".api, env: ."zenmux".env}
}'
```

当前 Wave C 路由立场：
- `baseten`、`llama`、`lmstudio` 与 `ollama-cloud` 作为 OpenAI 兼容预设接入（元数据 + 工厂已验证，VCR 待定）。
- `opencode` 与 `vercel` 作为 OpenAI 兼容预设接入并已通过 VCR 验证（各 3 个场景）。
- `zenmux` 作为 Anthropic 兼容预设接入并已通过 VCR 验证（3 个场景）。

Wave C 默认值（来自 `models.dev`）：

| 提供方 ID | API 家族目标 | 默认基地址 | 认证环境变量 |
|---|---|---|---|
| `baseten` | `openai-completions` | `https://inference.baseten.co/v1` | `BASETEN_API_KEY` |
| `llama` | `openai-completions` | `https://api.llama.com/compat/v1/` | `LLAMA_API_KEY` |
| `lmstudio` | `openai-completions` | `http://127.0.0.1:1234/v1` | `LMSTUDIO_API_KEY` |
| `ollama-cloud` | `openai-completions` | `https://ollama.com/v1` | `OLLAMA_API_KEY` |
| `opencode` | `openai-completions` | `https://opencode.ai/zen/v1` | `OPENCODE_API_KEY` |
| `vercel` | gateway-wrapper (`@ai-sdk/gateway`) | `models.dev` 中无静态 API URL | `AI_GATEWAY_API_KEY` |
| `zenmux` | `anthropic-messages` 目标（Anthropic 风格网关） | `https://zenmux.ai/api/anthropic/v1` | `ZENMUX_API_KEY` |

未阻塞的 Wave C 预设代表性 `models.json`：

```json
{
  "providers": {
    "baseten": {
      "models": [{ "id": "moonshotai/Kimi-K2-Instruct-0905" }]
    },
    "llama": {
      "models": [{ "id": "llama-3.3-70b-instruct" }]
    },
    "lmstudio": {
      "models": [{ "id": "openai/gpt-oss-20b" }]
    },
    "ollama-cloud": {
      "models": [{ "id": "glm-4.7" }]
    }
  }
}
```

特殊路由状态：
- `opencode`、`vercel` 与 `zenmux` 现已作为预设提供方接入并通过 VCR 验证。
- VCR 磁带：`tests/fixtures/vcr/verify_opencode_*.json`、`tests/fixtures/vcr/verify_vercel_*.json`、`tests/fixtures/vcr/verify_zenmux_*.json`。

### 3) Azure OpenAI（`azure-openai` / 别名 `azure`、`azure-cognitive-services`）

```json
{
  "providers": {
    "azure-openai": {
      "baseUrl": "https://<resource>.openai.azure.com",
      "models": [
        { "id": "gpt-4o" }
      ]
    }
  }
}
```

```bash
export AZURE_OPENAI_API_KEY="..."
# 运行时解析器使用的可选覆盖：
# export AZURE_OPENAI_RESOURCE="<resource>"
# export AZURE_OPENAI_DEPLOYMENT="<deployment>"
# export AZURE_OPENAI_API_VERSION="2024-08-01-preview"

pi --provider azure-openai --model gpt-4o -p "Say hello"
```

预期检查：
- 路由为原生 Azure 路径。
- 缺少部署/资源失败会包含来自 `../src/providers/mod.rs` 中 `resolve_azure_provider_runtime(...)` 的显式修复提示文本。

### 4) Google Vertex（`google-vertex` / 别名 `vertexai`）

推荐的显式基地址形态：

```json
{
  "providers": {
    "google-vertex": {
      "baseUrl": "https://us-central1-aiplatform.googleapis.com/v1/projects/<project>/locations/us-central1/publishers/google/models/gemini-2.0-flash",
      "models": [
        { "id": "gemini-2.0-flash", "api": "google-vertex" }
      ]
    }
  }
}
```

```bash
export GOOGLE_CLOUD_API_KEY="..."   # 或 VERTEX_API_KEY
export GOOGLE_CLOUD_PROJECT="<project>"   # 若已嵌入 baseUrl 则可选
export GOOGLE_CLOUD_LOCATION="us-central1" # 若已嵌入 baseUrl 则可选

pi --provider google-vertex --model gemini-2.0-flash -p "Say hello"
```

预期检查：
- 提供方路由为原生 vertex。
- 缺少项目/认证错误与 `../src/providers/vertex.rs` 中的消息匹配。

### 5) GitHub Copilot（`github-copilot` / 别名 `copilot`）

```json
{
  "providers": {
    "github-copilot": {
      "baseUrl": "https://api.github.com",
      "models": [
        { "id": "gpt-4o" }
      ]
    }
  }
}
```

```bash
export GITHUB_TOKEN="..."   # 或 GITHUB_COPILOT_API_KEY
pi --provider github-copilot --model gpt-4o -p "Say hello"
```

预期检查：
- 提供方在聊天调用前对 GitHub API 执行令牌交换。
- 若令牌交换失败，错误包含 Copilot 特定的诊断上下文。

### 6) GitLab Duo（`gitlab` / 别名 `gitlab-duo`）

```json
{
  "providers": {
    "gitlab": {
      "baseUrl": "https://gitlab.com",
      "models": [
        { "id": "gitlab-duo-chat", "api": "gitlab-chat" }
      ]
    }
  }
}
```

```bash
export GITLAB_TOKEN="..."   # 或 GITLAB_API_KEY
pi --provider gitlab --model gitlab-duo-chat -p "Say hello"
```

预期检查：
- 提供方向 `/api/v4/chat/completions` 发送请求，并返回非流式的 done 事件路径。

### 7) Bedrock / SAP AI Core（原生适配器 - 已通过 VCR 验证）

当前状态：
- `amazon-bedrock` 与 `sap-ai-core` 被归类为 `native-adapter-required` 且现已通过 VCR 验证。
- 认证/环境变量映射存在于 `../src/provider_metadata.rs` 与 `../src/auth.rs`。
- VCR 磁带：`tests/fixtures/vcr/verify_bedrock_*.json`（4 个场景）、`tests/fixtures/vcr/verify_sap_ai_core_*.json`（6 个场景）。
- 一致性证据：[`docs/provider-native-parity-report.json`](provider-native-parity-report.json)。

Bedrock 认证：
- SigV4 凭证：`AWS_ACCESS_KEY_ID`、`AWS_SECRET_ACCESS_KEY`、`AWS_SESSION_TOKEN`
- Bearer 令牌备选：`AWS_BEARER_TOKEN_BEDROCK`

SAP AI Core 认证：
- OAuth2 客户端凭证：`SAP_AI_CORE_CLIENT_ID`、`SAP_AI_CORE_CLIENT_SECRET`、`SAP_AI_CORE_TOKEN_URL`

## 故障排查矩阵（症状 -> 处置）

| 症状 | 快速诊断 | 修复措施 |
|---|---|---|
| `Missing API key` / 启动时认证错误 | 检查 `provider_auth_env_keys(...)` 中的提供方环境变量映射 | 设置提供方环境变量，或 `--api-key`，或持久化的 `auth.json`；重新运行 |
| 提供方为 `openrouter` 时出现 `OpenAI API error (HTTP 401)` | 无效/缺失的 OpenRouter 密钥（或错误密钥被路由到提供方别名） | 设置 `OPENROUTER_API_KEY`（或 `--api-key`）并使用已知可用模型重跑（`openrouter/auto`、`openai/gpt-4o-mini`）。证据：`tests/provider_native_contract.rs::openrouter_contract::error_401_auth_failure`、`tests/provider_native_verify.rs::openrouter_conformance::error_auth_401` |
| 提供方为 `openrouter` 时出现 `OpenAI API error (HTTP 429)` | 提供方/模型配额或限流 | 带退避重试、减小请求/令牌大小，或切换模型/提供方路由。证据：`tests/provider_native_contract.rs::openrouter_contract::error_429_rate_limit`、`tests/provider_native_verify.rs::openrouter_conformance::error_rate_limit_429` |
| `openRouterRouting must be a JSON object when configured` | `models.json` 中 `compat.openRouterRouting` 不是对象 | 将 `compat.openRouterRouting` 改为对象（例如 `{ "provider": { "order": ["openai"] } }`）。证据：`src/providers/openai.rs::apply_openrouter_routing_overrides` 中的运行时守卫，行为锁测 `src/providers/openai.rs::test_build_request_applies_openrouter_routing_overrides` |
| `Provider not implemented (api: ...)` | 路由在 `resolve_provider_route(...)` 中落入未知提供方/api | 修正 `models.json` 中的提供方 ID/api；在 `../src/provider_metadata.rs` 中验证规范 ID 或别名 |
| Azure 缺少 resource/deployment | 解析器无法从基地址/环境变量推断 `resource` / `deployment` | 设置 `AZURE_OPENAI_RESOURCE`、`AZURE_OPENAI_DEPLOYMENT`，或包含完整 Azure 主机/部署路径 |
| Vertex 缺少 project | 项目既不在基地址中也不在环境变量中 | 设置 `GOOGLE_CLOUD_PROJECT` 或 `VERTEX_PROJECT`；或在基地址中编码项目 |
| Vertex 缺少 token | 无 `api_key` 且无 `GOOGLE_CLOUD_API_KEY`/`VERTEX_API_KEY` | 设置上述环境变量之一（bearer 令牌/access 令牌） |
| Copilot 认证失败 | GitHub 令牌缺失/无效或令牌交换被拒 | 设置 `GITHUB_COPILOT_API_KEY`/`GITHUB_TOKEN`；验证 Copilot 权益 |
| GitLab 认证失败 | 缺少或无效的 PAT/OAuth 令牌 | 设置 `GITLAB_TOKEN` 或 `GITLAB_API_KEY`；验证实例 URL 与作用域 |
| 429/quota/5xx | 提供方侧限流或故障 | 在设置中调整重试策略、减小请求大小，或切换模型/提供方 |

## OAuth 与登录注意事项

交互式斜杠帮助现已反映更广的 `/login` 覆盖面：
- 内置 OAuth 提供方：`anthropic`、`openai-codex`、`google-gemini-cli`、`google-antigravity`、`kimi-for-coding`、`github-copilot`、`gitlab`。
- 对支持交互式 API 密钥登录的提供方可提供基于元数据的 API 密钥提示。

对于没有内置 OAuth 流程或 API 密钥提示的提供方，优先使用显式的环境变量/auth.json 配置。扩展提供方需要 `oauth_config` 条目才能启用 `/login`。

## 强制测试与日志义务

本节定义新增或修改提供方时所需的最低测试与日志证据。`PROVIDER_METADATA` 中的每个提供方条目均设置 `test_obligations: TEST_REQUIRED`，将全部四类设为 `true`。

### 测试义务类别

`ProviderTestObligations` 结构体（`src/provider_metadata.rs`）强制四类必选类别：

| 类别 | 测试套件 | 证明内容 |
|----------|-----------|----------------|
| **unit** | `tests/provider_factory.rs` | 工厂分发解析为正确的 `ProviderRouteKind`；基地址规范化正确；API/提供方类型往返正确 |
| **contract** | `tests/provider_native_contract.rs` | HTTP 请求负载形态、认证头构造、工具 schema 转换以及 SSE→`StreamEvent` 解码正确（仅原生适配器；OpenAI 兼容预设继承 OpenAI 契约） |
| **conformance** | `tests/provider_native_verify.rs` | 基于 VCR 回放的规范场景产生预期的流式事件、工具调用、错误码与停止原因 |
| **e2e** | `tests/e2e_provider_scenarios.rs` | 多提供方确定性工作流，涵盖文本生成、工具调用、错误处理、事件排序与请求体稳定性 |

真实一致性（可选，CI 门控）：

| 套件 | 门控 | 证明内容 |
|-------|------|----------------|
| `tests/e2e_cross_provider_parity.rs` | `CI_E2E_TESTS=1` | 跨 10 个提供方的真实 API 调用产生一致的事件序列、令牌用量与错误语义 |

### 每种提供方类型的最低测试数量

**OpenAI 兼容预设**（仅元数据 + 工厂）：
- 1 个工厂分发测试（批次测试已覆盖）
- 3 项防漂移快照更新（规范 ID、别名、基地址）
- 3 个 VCR 一致性场景：`simple_text`、`tool_call_single`、`error_auth_401`

**原生适配器**（专用 `.rs` 文件）：
- 上述全部，外加：
- 针对请求形态、认证头、工具 schema 与响应解码的契约测试
- 额外 3 个 VCR 一致性场景：`unicode_text`、`error_bad_request_400`、`error_rate_limit_429`
- 至少 1 个端到端家族级场景

### 规范 VCR 场景

验证工具（`tests/provider_native_verify.rs`）定义了 7 个规范场景。每个场景有结构化期望：

| 场景标签 | 消息 | 工具 | 期望类型 | 关键断言 |
|-------------|----------|-------|-----------------|----------------|
| `simple_text` | 1 条用户消息 | 无 | `Stream` | `min_text_deltas >= 1`，停止原因 `EndTurn` |
| `unicode_text` | 1 条用户消息（日文/emoji） | 无 | `Stream` | `require_unicode = true`，输出含非 ASCII |
| `tool_call_single` | 1 条用户消息 | 1 个工具定义 | `Stream` | `min_tool_calls >= 1`，停止原因 `ToolUse` |
| `tool_call_multiple` | 1 条用户消息 | 2 个工具定义 | `Stream` | `min_tool_calls >= 2` |
| `error_auth_401` | 1 条用户消息 | 无 | `Error` | HTTP 401 状态 |
| `error_bad_request_400` | 畸形 | 无 | `Error` | HTTP 400 状态 |
| `error_rate_limit_429` | 1 条用户消息 | 无 | `Error` | HTTP 429 状态 |

### VCR 磁带命名约定

格式：`verify_{provider_id}_{scenario_tag}.json`

示例：
```
tests/fixtures/vcr/verify_anthropic_simple_text.json
tests/fixtures/vcr/verify_openai_tool_call_single.json
tests/fixtures/vcr/verify_gemini_error_rate_limit_429.json
tests/fixtures/vcr/verify_groq_error_auth_401.json
```

将所有磁带置于 `tests/fixtures/vcr/`。工具按约定发现它们。

### 失败路径期望

每个提供方**必须**对以下失败模式展示正确的错误处理：

| 失败模式 | HTTP 状态 | 所需证据 |
|-------------|------------|-------------------|
| 无效/缺失 API 密钥 | 401 | VCR 磁带 `verify_{provider}_error_auth_401.json`；提供方返回错误事件（非 panic） |
| 畸形请求体 | 400 | VCR 磁带 `verify_{provider}_error_bad_request_400.json`；错误消息可解析 |
| 超过限流 | 429 | VCR 磁带 `verify_{provider}_error_rate_limit_429.json`；错误包含限流上下文 |

原生适配器的契约测试（`provider_native_contract.rs`）还额外验证：
- 认证失败产生结构化错误（非原始 HTTP 体）
- 限流错误在可用时包含 retry-after 提示
- 服务端错误（5xx）作为 `StreamEvent::Error` 传播

### JSONL 日志义务

所有使用 `TestHarness` 的提供方测试必须输出符合以下 schema 的 JSONL 日志：

**日志 schema（`pi.test.log.v2`）**——必填字段：

| 字段 | 类型 | 说明 |
|-------|------|-------------|
| `schema` | string | 必须为 `"pi.test.log.v2"` |
| `type` | string | 必须为 `"log"` |
| `trace_id` | string | 链路关联 ID（v2 必需） |
| `seq` | integer | 单调递增序号 |
| `ts` | string | ISO-8601 时间戳 |
| `t_ms` | integer | 自测试开始以来的毫秒数 |
| `level` | string | 之一：`debug`、`info`、`warn`、`error` |
| `category` | string | 结构化类别（如 `setup`、`action`、`verify`、`teardown`） |
| `message` | string | 人类可读日志消息 |

可选字段：`span_id`、`parent_span_id`、`test`、`context`（键值映射）。

**制品 schema（`pi.test.artifact.v1`）**——必填字段：

| 字段 | 类型 | 说明 |
|-------|------|-------------|
| `schema` | string | 必须为 `"pi.test.artifact.v1"` |
| `type` | string | 必须为 `"artifact"` |
| `seq` | integer | 序号 |
| `ts` | string | ISO-8601 时间戳 |
| `t_ms` | integer | 毫秒数 |
| `name` | string | 逻辑制品名（如 `verification_report`） |
| `path` | string | 制品文件路径 |

可选字段：`test`、`size_bytes`、`sha256`。

### 日志类别约定

在提供方测试中一致使用以下类别：

| 类别 | 使用时机 |
|----------|------------|
| `setup` | 测试工具初始化、模拟服务启动、VCR 磁带加载 |
| `action` | 提供方创建、流启动、请求发送 |
| `verify` | 断言检查、事件序列验证 |
| `teardown` | 清理、资源释放 |
| `stream` | 单个流事件处理 |
| `error` | 错误处理路径 |

### JSONL 规范化与脱敏

在生成确定性测试输出（用于快照对比或 CI）时，使用 `TestLogger::dump_jsonl_normalized()`，它会替换：

| 占位符 | 替换对象 |
|------------|---------|
| `<TIMESTAMP>` | 所有 ISO-8601 时间戳 |
| `<PROJECT_ROOT>` | Cargo manifest 目录 |
| `<TEST_ROOT>` | 临时目录路径 |
| `<RUN_ID>` | `run-{uuid}` 模式 |
| `<UUID>` | UUID 字符串 |
| `<PORT>` | `http://127.0.0.1:{port}` |
| `<TRACE_ID>` | 链路 ID |
| `<SPAN_ID>` | Span ID |

敏感字段在 JSONL 上下文映射中自动脱敏：`api_key`、`authorization`、`bearer`、`cookie`、`credential`、`password`、`private_key`、`secret`、`token`。

### 事件序列验证规则

端到端测试（`e2e_provider_scenarios.rs`）验证流事件遵循以下顺序：

```
Start? → (TextDelta | ThinkingDelta | ToolCallStart | ToolCallDelta)* → Done | Error
```

具体规则：
- `Start` 事件（若存在）必须为首个 — Bedrock 豁免（`require_start_event: false`）
- `Done` 事件必须携带有效 `StopReason`（`EndTurn`、`ToolUse`、`MaxTokens`、`StopSequence`）
- `Error` 事件终止流；其后无事件
- 同一工具索引的 `ToolCallStart` 必须在任何 `ToolCallDelta` 之前
- 令牌用量（`input_tokens`、`output_tokens`）必须在 `Done` 事件中报告

### 跨提供方一致性记录 schema

真实一致性测试（`e2e_cross_provider_parity.rs`）输出 `ParityRecord` 条目，包含：

| 字段 | 是否必需 | 说明 |
|-------|----------|-------------|
| `check` | 是 | 一致性检查名 |
| `provider` | 是 | 提供方规范 ID |
| `status` | 是 | `pass`、`fail` 或 `skip` |
| `event_count` | 是 | 接收到的流事件总数 |
| `sequence_valid` | 是 | 事件排序是否正确 |
| `sequence` | 是 | 按序排列的事件类型名数组 |
| `usage_total_tokens` | 是 | 报告的总令牌数 |
| `elapsed_ms` | 是 | 墙钟时间 |

### 防漂移快照测试

`tests/provider_metadata_comprehensive.rs` 中的这些测试使用硬编码快照，新增或修改提供方时**必须更新**：

| 测试 | 触发更新时机 |
|------|---------------|
| `canonical_id_snapshot_detects_additions_and_removals` | 新增/移除任何规范 ID |
| `alias_mapping_snapshot_is_current` | 别名数组的任何变更 |
| `base_url_snapshot_for_key_providers` | 关键提供方基地址变更 |
| `vcr_fixture_coverage_for_core_providers` | 新增核心提供方 |
| `gap_providers_have_setup_documentation` | 新增 gap 类提供方 |
| `no_accidental_duplicate_routing_defaults` | 新增具有相同 (api, base_url) 对的提供方 |

### 快速验证命令

新增或修改提供方后，按顺序运行：

```bash
# 1. 一致性（VCR 回放）
cargo test --test provider_native_verify {provider}_conformance:: -- --nocapture

# 2. 工厂分发
cargo test --test provider_factory -- --nocapture

# 3. 元数据不变量 + 漂移快照
cargo test --test provider_metadata_comprehensive -- --nocapture

# 4. 契约测试（仅原生适配器）
cargo test --test provider_native_contract {provider}_contract:: -- --nocapture

# 5. 端到端场景
cargo test --test e2e_provider_scenarios -- --nocapture

# 6. 质量门
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 文档与接入变更的验证命令

针对性检查（快速）：

```bash
cargo test provider_factory -- --nocapture
cargo test provider_metadata_comprehensive -- --nocapture
cargo test --test provider_native_contract openrouter_contract:: -- --nocapture
cargo test --test provider_native_verify openrouter_conformance:: -- --nocapture
cargo test --test e2e_provider_scenarios e2e_openai_compatible_wave_presets -- --nocapture
```

更广的质量门：

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

真实一致性通道（门控，真实 API）：

```bash
CI_E2E_TESTS=1 cargo test e2e_cross_provider_parity -- --nocapture
CI_E2E_TESTS=1 cargo test e2e_live_harness -- --nocapture
```

## CI 制品保留与回放分流工作流（bd-3uqg.9.4.3）

提供方变更在 CI 失败可通过保留制品复现之前不算完成。对每个面向提供方的 PR 使用此契约与回放流程：

### 所需制品输出

权威来源：`docs/provider_e2e_artifact_contract.json`

按套件制品（必需）：
- `output.log`
- `result.json`
- `test-log.jsonl`（`pi.test.log.v2`）
- `artifact-index.jsonl`（`pi.test.artifact.v1`）

按运行制品（必需）：
- `summary.json`
- `environment.json`
- `evidence_contract.json`
- `replay_bundle.json`
- `failure_digest.json`（每个失败套件）

### 验证命令

```bash
# 契约与保留检查
cargo test --test ci_artifact_retention -- --nocapture

# 回放包形态与命令有效性
cargo test --test e2e_replay_bundles -- --nocapture
cargo test --test e2e_replay_bundle_validation -- --nocapture

# 生成一套新的 CI 风格制品集
./scripts/e2e/run_all.sh --profile ci
```

### 分流与回放序列

1. 从 `tests/e2e_results/<timestamp>/summary.json` 入手，检查 `failed_names` / `failed_unit_names`。
2. 读取 `tests/e2e_results/<timestamp>/replay_bundle.json` 并运行 `one_command_replay`。
3. 对每个失败套件，打开 `failed_suites[].digest_path`（`failure_digest.json`）并运行：
   - `remediation_pointer.suite_replay_command`
   - `remediation_pointer.targeted_test_replay_command`
4. 在 Bead 线程中记录根因类别与修复措施后再合并。
5. 若涉及认证相关失败，交叉检查脱敏测试：
   - `tests/e2e_artifact_retention_triage.rs::log_redacts_api_keys`
   - `tests/e2e_artifact_retention_triage.rs::log_redacts_authorization_headers`

操作员参考：`docs/ci-operator-runbook.md`。

## 接入反模式、护栏与回滚协议（bd-3uqg.9.4.4）

本节为合并提供方元数据、工厂或适配器变更前的必读内容。

### 反模式目录与预防检查

| 反模式 | 具体示例 | 预防检查（必须通过） |
|---|---|---|
| 文档与运行时之间的别名漂移 | `providers.md` 称别名为 `open-router`，但 `provider_metadata.rs` 的别名列表未更新 | `cargo test --test provider_metadata_comprehensive alias_mapping_snapshot_is_current -- --nocapture` |
| 验证覆盖不完整 | 新提供方仅添加了 `simple_text` 磁带，无认证/错误场景 | `cargo test --test provider_native_verify <provider>_conformance:: -- --nocapture` 加必需场景（`simple_text`、`tool_call_single`、`error_auth_401`） |
| 日志/制品中未脱敏的诊断信息 | `Authorization` 头或 API 密钥出现在 JSONL 或磁带输出中 | `cargo test --test ci_artifact_retention -- --nocapture` 与 `tests/e2e_artifact_retention_triage.rs` 中的脱敏检查 |
| 陈旧的文档/运行时映射 | 运行时变更后设置 JSON 列出错误的 `auth_env` 或基地址 | `cargo test --test provider_native_contract docs_runtime -- --nocapture` |
| 缺少 CI 可回放性 | 存在失败摘要却无确定性回放命令 | `cargo test --test e2e_replay_bundles -- --nocapture` 与 `cargo test --test e2e_replay_bundle_validation -- --nocapture` |
| 路由默认值冲突 | 新提供方无意中复制了现有 `(api, base_url)` 对 | `cargo test --test provider_metadata_comprehensive no_accidental_duplicate_routing_defaults -- --nocapture` |
| 合并后提供方矩阵/文档漂移 | 提供方在代码中可用但矩阵/证据行陈旧 | 合并前更新文档并运行 `cargo test --test provider_native_contract docs_runtime -- --nocapture` |

### 必需的合并前护栏

在关闭提供方 Bead 前全部运行以下项：

```bash
# 元数据 + 路由漂移护栏
cargo test --test provider_metadata_comprehensive -- --nocapture
cargo test --test provider_factory -- --nocapture

# 文档/运行时一致性
cargo test --test provider_native_contract docs_runtime -- --nocapture

# CI 制品 + 回放保障
cargo test --test ci_artifact_retention -- --nocapture
cargo test --test e2e_replay_bundles -- --nocapture
cargo test --test e2e_replay_bundle_validation -- --nocapture

# 原生适配器 / 提供方特定一致性（如适用）
cargo test --test provider_native_verify <provider>_conformance:: -- --nocapture

# 全局质量门
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

合并前阻塞规则：
- 任何脱敏失败、缺失回放命令或文档/运行时不一致均为合并阻塞项。

### 必需的合并后监控钩子

合并后首个 CI 周期内：
1. 确认保留制品存在（失败时的 `summary.json`、`replay_bundle.json`、`failure_digest.json`）。
2. 验证 `replay_bundle.one_command_replay` 可从 CI 制品集本地执行。
3. 检查新失败套件的失败分类与修复指引。
4. 通过制品保留脱敏检查确认无密钥泄露。
5. 在 Bead 线程中发布通过/失败状态与制品路径的备注。

### 回滚协议（可操作阈值 + 交接）

严重度阈值：

| 严重度 | 触发条件 | 要求处置时限 |
|---|---|---|
| `SEV-1` | CI 制品中密钥暴露或脱敏失效 | 同一响应窗口内立即遏制并回滚 |
| `SEV-2` | 提供方路由/认证中断导致广泛请求失败且无安全变通方案 | 确认后 30 分钟内做出回滚决策 |
| `SEV-3` | 有文档化变通方案的局部回归 | 1 个工作日内决定修复前进或回滚 |

回滚步骤：
1. **分类与遏制**：指派严重度并停止受影响提供方范围的进一步合并。
2. **保留证据**：在 Bead 线程中归档失败的 `summary.json`、`replay_bundle.json` 与 `failure_digest.json` 路径。
3. **执行回滚**：
   - 首选：对问题提供方变更执行针对性 `git revert <commit>`。
   - 备选：临时提供方禁用/路径重路由，并附显式后续 Bead。
4. **重跑门禁**：重跑制品/回放与提供方验证命令以确认恢复。
5. **交接**：发布最终事件备注，包含：
   - 根因类别，
   - 回滚提交/变更引用，
   - 验证命令输出摘要，
   - 永久修复工作的后续 bead id。

## 贡献者清单（新提供方或重大提供方更新）

### 阶段 1：元数据注册

1. 在 `../src/provider_metadata.rs` 中添加或更新规范元数据条目：
   - `canonical_id`：小写、连字符分隔（如 `my-provider`）。
   - `aliases`：常见备选名称。
   - `auth_env_keys`：主环境变量在前，回退在后（如 `&["MY_PROVIDER_API_KEY"]`）。
   - `onboarding`：`BuiltInNative`、`OpenAICompatiblePreset`、`NativeAdapterRequired` 之一。
   - `routing_defaults`：OAI 兼容必填；设置 `api`、`base_url`、`auth_header` 等。
   - `test_obligations`：生产提供方全部设为 `true`。

2. **更新防漂移快照**——这些测试在更新前会失败：
   - `tests/provider_metadata_comprehensive.rs` 中的 `canonical_id_snapshot_detects_additions_and_removals` — 向有序 `EXPECTED` 数组添加新 ID。
   - `alias_mapping_snapshot_is_current` — 向 `EXPECTED_ALIASES` 添加任何新别名。
   - `base_url_snapshot_for_key_providers` — 若为关键/gap 提供方则添加基地址。

3. 确保别名解析 + 环境变量映射被现有不变量测试覆盖：
   - `all_canonical_ids_are_unique`、`no_alias_collides_with_canonical_id` — 自动覆盖。
   - `auth_env_keys_are_screaming_snake_case` — 自动覆盖。

### 阶段 2：工厂装配与测试

4. 在 `../src/providers/mod.rs` 中装配路由与提供方工厂行为。

5. 添加/更新提供方特定测试：
   - **工厂选择**：`tests/provider_factory.rs`（批次预设测试）。
   - **元数据不变量**：`tests/provider_metadata_comprehensive.rs`（结构测试自动覆盖）。
   - **流式契约**：`tests/provider_streaming.rs` 或 `tests/provider_native_contract.rs`。

6. 在 `tests/fixtures/vcr/` 中添加 VCR 夹具：
   - 最低要求：`verify_<provider>_simple_text.json`
   - 推荐：`verify_<provider>_error_auth_401.json`、`verify_<provider>_tool_call_single.json`
   - 若为核心提供方，添加到 `tests/provider_metadata_comprehensive.rs` 中的 `vcr_fixture_coverage_for_core_providers`。

### 阶段 3：文档

7. 更新提供方文档：
   - `providers.md` — 矩阵/状态行。
   - 本手册 — 配置示例与故障排查条目。
   - 对于 gap 提供方（groq、cerebras、openrouter、moonshotai、alibaba 类）：创建专用设置文档 `docs/provider-<name>-setup.json`，遵循 schema `pi.provider_setup_guide.v1`。
   - 使用环境变量、CLI 示例与注意事项更新 `docs/provider-config-examples.json`。
   - 若提供方有非标准行为则更新 `docs/provider-migration-guide.md`。
   - 使用认证失败模式更新 `docs/provider-auth-troubleshooting.md`。

8. **验证文档/运行时一致性**——这些测试会捕获文档漂移：
   - `tests/provider_native_contract.rs` 中的 `docs_runtime_consistency::setup_doc_auth_env_matches_runtime`。
   - `docs_runtime_consistency::setup_doc_base_url_matches_runtime_default`。
   - `docs_runtime_consistency::config_examples_env_vars_match_runtime`。

### 阶段 4：质量门

9. 关闭前运行质量门：

```bash
# 防漂移（必须通过 — 会捕获快照不一致）
CARGO_TARGET_DIR=target/<agent> cargo test --test provider_metadata_comprehensive -- --nocapture

# 工厂 + 路由
CARGO_TARGET_DIR=target/<agent> cargo test --test provider_factory -- --nocapture

# 文档/运行时一致性
CARGO_TARGET_DIR=target/<agent> cargo test --test provider_native_contract docs_runtime -- --nocapture

# 完整 lint/format
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

10. 关闭提供方 Beads 前附上证据链接（测试输出 + 制品路径）。

## 质量门参考

### 防漂移测试（bd-3uqg.11.10.4）

`tests/provider_metadata_comprehensive.rs` 中的这些测试使用硬编码快照来强制对元数据变更的有意确认：

| 测试 | 捕获内容 | 何时更新 |
|---|---|---|
| `canonical_id_snapshot_detects_additions_and_removals` | 提供方新增/移除 | 新增或移除任何 canonical_id 时 |
| `alias_mapping_snapshot_is_current` | 别名新增/移除/重分配 | 别名数组的任何变更 |
| `base_url_snapshot_for_key_providers` | 静默端点 URL 变更 | 关键提供方 base_url 变更 |
| `vcr_fixture_coverage_for_core_providers` | 核心提供方缺少 VCR 夹具 | 新增核心提供方 |
| `gap_providers_have_setup_documentation` | gap 提供方缺少设置文档 | 新增 gap 类提供方 |
| `no_accidental_duplicate_routing_defaults` | 复制粘贴路由错误 | 新增具有相同 (api, base_url) 对的提供方 |

### 文档/运行时一致性测试（bd-3uqg.11.12.5）

`tests/provider_native_contract.rs` 中的这些测试验证文档不会与运行时静默偏离：

| 测试 | 捕获内容 |
|---|---|
| `setup_docs_exist_and_parse_as_valid_json` | 损坏/缺失的 JSON 设置文档 |
| `setup_doc_provider_ids_match_metadata` | 文档 provider_id 与元数据不一致 |
| `setup_doc_auth_env_matches_runtime` | 文档 auth_env 与运行时环境变量不一致 |
| `setup_doc_base_url_matches_runtime_default` | 文档 base_url 与运行时默认值不一致 |
| `config_examples_env_vars_match_runtime` | 配置示例环境变量与运行时不一致 |
| `migration_guide_references_correct_env_vars` | 迁移指南环境变量引用 |

提供方对照表新鲜度也由 `python3 scripts/check_provider_discrepancy_ledger.py --compact` 检查。编辑 `src/provider_metadata.rs`、`docs/providers.md` 或 `docs/provider-auth-troubleshooting.md` 后运行它；它会将规范 ID、别名与认证环境变量与已检入的对照表对比。

## 提供方特定文档引用

| 提供方家族 | 设置文档 | 配置示例 | 迁移说明 |
|---|---|---|---|
| Groq | `docs/provider-groq-setup.json` | `docs/provider-config-examples.json` | `docs/provider-migration-guide.md` |
| Cerebras | `docs/provider-cerebras-setup.json` | `docs/provider-config-examples.json` | `docs/provider-migration-guide.md` |
| OpenRouter | `docs/provider-openrouter-setup.json` | `docs/provider-config-examples.json` | `docs/provider-migration-guide.md` |
| Kimi (moonshotai) | `docs/provider-kimi-setup.json` | `docs/provider-config-examples.json` | `docs/provider-migration-guide.md` |
| Qwen (alibaba) | `docs/provider-qwen-setup.json` | `docs/provider-config-examples.json` | `docs/provider-migration-guide.md` |
| 认证故障排查（全部） | `docs/provider-auth-troubleshooting.md` 由 `python3 scripts/check_provider_discrepancy_ledger.py --compact` 检查 | — | — |
| 长尾证据 | `docs/provider-longtail-evidence.md` | — | — |

## 当前有证据支撑的限制

`providers.md` 中的规范矩阵/证据表正处于活跃的并行编辑中。以该文件作为最终矩阵状态的来源，本手册作为接入与故障排查的操作性实现指南。
