# 提供方 / Providers

本文档是 `pi_agent_rust` 仓内提供方基线的权威来源（canonical in-repo provider baseline）。
汇总提供方 ID、别名、API 家族、认证行为及当前实现模式。

快照依据（Snapshot basis）：
- `src/models.rs`（`built_in_models`、`ad_hoc_provider_defaults`）
- `src/auth.rs`（`env_keys_for_provider`）
- `src/providers/mod.rs`（`create_provider`、API 回退路由）
- `src/provider_metadata.rs`（`PROVIDER_METADATA`、别名、接入模式）
- `src/providers/*.rs` 原生实现
- 原始快照时间戳：2026-02-10
- 最近一次源码交叉核验：2026-05-10（`bd-8t27h.7`）

Provider-count rule: Pi has 12 native provider implementation modules, counted as the Rust files under `src/providers/` excluding `mod.rs`: `anthropic`, `openai`, `openai_responses`, `gemini`, `cohere`, `azure`, `bedrock`, `vertex`, `copilot`, `gitlab`, `cursor`, and `model_fetch`. User-visible canonical IDs, aliases, OpenAI-compatible presets, VCR coverage families, and extension-provided providers are separate counts.（提供方数量规则：Pi 拥有 12 个原生提供方实现模块（native provider implementation modules），以 `src/providers/` 下除 `mod.rs` 外的 Rust 文件计数：`anthropic`、`openai`、`openai_responses`、`gemini`、`cohere`、`azure`、`bedrock`、`vertex`、`copilot`、`gitlab`、`cursor` 与 `model_fetch`。用户可见的规范 ID、别名、OpenAI 兼容预设、VCR 覆盖家族及扩展提供的提供方为独立计数。）

该规则由 `tests/traceability_staleness.rs::native_provider_module_inventory_matches_provider_docs` 守护（将本文清单与实时的 `src/providers/*.rs` 清单对比）。

| 原生模块（Native module） | 主要用户可见表面（Primary user-visible surface） | 备注（Notes） |
|---------------|------------------------------|-------|
| `anthropic` | `anthropic` | Anthropic Messages API。 |
| `openai` | `openai` 及 OpenAI 兼容预设 | Chat Completions 兼容运行时路径。 |
| `openai_responses` | `openai`、`openai-codex` | Responses/Codex Responses 请求与流式路径。 |
| `gemini` | `google`、`gemini` | Google Generative AI 路径。 |
| `cohere` | `cohere` | Cohere chat 路径。 |
| `azure` | `azure-openai`、`azure`、`azure_openai`、`azure-cognitive-services`、`azure-openai-responses` | Azure OpenAI 部署路径。 |
| `bedrock` | `amazon-bedrock`、`bedrock` | Amazon Bedrock Converse 路径。 |
| `vertex` | `google-vertex`、`vertexai` | Vertex AI Gemini/Anthropic 发布方路径。 |
| `copilot` | `github-copilot`、`copilot` | GitHub Copilot chat/completions 路径。 |
| `gitlab` | `gitlab`、`gitlab-duo` | GitLab Duo chat 路径。 |

扩展提供的提供方（Extension-provided providers）经由 `src/providers/mod.rs` 中的扩展提供方桥接（extension provider bridge）路由；因其在运行时发现（discovered at runtime），有意排除在原生模块计数之外。

## 实现模式（Implementation Modes）

| 模式（Mode） | 含义（Meaning） |
|------|---------|
| `native-implemented` | 提供方在 `create_provider` 中拥有直接运行时路径，可立即调度。 |
| `native-partial` | 原生模块存在，但工厂装配或所需配置路径尚未完全集成。 |
| `oai-compatible-preset` | 提供方通过 OpenAI 兼容适配器（`openai-completions`）解析，使用预设的 base/auth 默认值。 |
| `alias-only` | 提供方 ID 为规范 ID 的已文档化同义词；无独立运行时实现。 |
| `missing` | 提供方 ID 在枚举/认证映射中可识别，但暂无可用运行时调度路径。 |

### 机器可读分类（`bd-3uqg.1.4`）

规范规划制品（Canonical planning artifact）：`docs/provider-implementation-modes.json`

该 JSON 是提供方接入模式选择的执行事实来源（execution source-of-truth）：

| 模式（Mode） | 规划含义（Planning Meaning） |
|------|------------------|
| `native-adapter-required` | 需要专用运行时适配器路径（协议/认证/工具语义无法由通用 OAI 路由安全覆盖）。 |
| `oai-compatible-preset` | 可经由 OpenAI 兼容适配器路由，配合提供方特定的 base/auth 预设。 |
| `gateway-wrapper-routing` | 作为网关/元路由/别名路由表面；优先保障路由策略与诊断。 |
| `deferred` | 明确不在当前实现波次；为规划完整性保留。 |

当前制品覆盖（`docs/provider-implementation-modes.json`）：
- 93 个上游并集 ID 已分类（无遗漏）
- 6 个 Pi 补充别名 ID 已分类
- 99 条记录均含显式 profile、理由与运行时状态
- 20 个高风险提供方携带显式前置 beads 与所需诊断制品

## 验证证据图例（Verification Evidence Legend）

- 元数据与别名/路由锁：[`tests/provider_metadata_comprehensive.rs`](../../tests/provider_metadata_comprehensive.rs)
- 工厂与适配器选择锁：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)
- 原生提供方请求形态锁：[`tests/provider_backward_lock.rs`](../../tests/provider_backward_lock.rs)
- 提供方流式契约套件：[`tests/provider_streaming.rs`](../../tests/provider_streaming.rs)
- 实时对等冒烟通道：[`tests/e2e_cross_provider_parity.rs`](../../tests/e2e_cross_provider_parity.rs)
- 实时提供方集成通道：[`tests/e2e_live.rs`](../../tests/e2e_live.rs)
- 差异台账新鲜度门禁：`python3 scripts/check_provider_discrepancy_ledger.py --compact`

## Wave A 对等验证（`bd-3uqg.4.4`）

当前已跟踪的全部 Wave A OpenAI 兼容预设 ID 的单元 + 请求形态验证：
`groq`、`deepinfra`、`cerebras`、`openrouter`、`mistral`、`moonshotai`、`dashscope`、`deepseek`、`fireworks`、`togetherai`、`perplexity`、`xai` 及迁移别名 `fireworks-ai`。

验证制品：
- 默认/工厂锁：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)（`wave_a_presets_resolve_openai_compat_defaults_and_factory_route`）
- 流式路径/认证锁：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)（`wave_a_openai_compat_streams_use_chat_completions_path_and_bearer_auth`）
- 别名迁移锁：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)（`fireworks_ai_alias_migration_matches_fireworks_canonical_defaults`）

提供方逐项状态（本地验证 `cargo test --test provider_factory -- --nocapture`）：

| 提供方 ID（Provider ID） | 默认值+工厂路由锁 | 流式路径/认证锁 | 状态 |
|-------------|-------------------------------|--------------------------|--------|
| `groq` | yes | yes | pass |
| `deepinfra` | yes | yes | pass |
| `cerebras` | yes | yes | pass |
| `openrouter` | yes | yes | pass |
| `mistral` | yes | yes | pass |
| `moonshotai` | yes | yes | pass |
| `dashscope` | yes | yes | pass |
| `deepseek` | yes | yes | pass |
| `fireworks` | yes | yes | pass |
| `togetherai` | yes | yes | pass |
| `perplexity` | yes | yes | pass |
| `xai` | yes | yes | pass |
| `fireworks-ai`（别名） | yes | yes | pass |

迁移映射决策：
- `fireworks-ai` 仍作为规范 `fireworks` 的别名被接受。
- `fireworks` 与 `fireworks-ai` 的路由与认证行为保持对等锁定。
- 不引入兼容垫片层；规范配置应逐步使用 `fireworks`。

## Wave B1 接入验证（`bd-3uqg.5.2`）

批次 B1 提供方 ID 已集成并锁定测试：
`alibaba-cn`、`kimi-for-coding`、`minimax`、`minimax-cn`、`minimax-coding-plan`、`minimax-cn-coding-plan`。

验证制品：
- 元数据 + 工厂路由锁：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)（`wave_b1_presets_resolve_metadata_defaults_and_factory_route`）
- OpenAI 兼容流式路径/认证锁（`alibaba-cn`）：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)（`wave_b1_alibaba_cn_openai_compat_streams_use_chat_completions_path_and_bearer_auth`）
- Anthropic 兼容流式路径/认证锁（`kimi-for-coding`、`minimax*`）：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)（`wave_b1_anthropic_compat_streams_use_messages_path_and_x_api_key`）
- 家族一致性锁：[`tests/provider_factory.rs`](../../tests/provider_factory.rs)（`wave_b1_family_coherence_with_existing_moonshot_and_alibaba_mappings`）
- 代表性冒烟/e2e 制品（离线 VCR  harness）：[`tests/provider_native_verify.rs`](../../tests/provider_native_verify.rs)

| 提供方 ID | API 家族 | 路由锁 | 流式/认证锁 | 状态 |
|-------------|------------|------------|------------------|--------|
| `alibaba-cn` | `openai-completions` | yes | yes | pass |
| `kimi-for-coding` | `anthropic-messages` | yes | yes | pass |
| `minimax` | `anthropic-messages` | yes | yes | pass |
| `minimax-cn` | `anthropic-messages` | yes | yes | pass |
| `minimax-coding-plan` | `anthropic-messages` | yes | yes | pass |
| `minimax-cn-coding-plan` | `anthropic-messages` | yes | yes | pass |

代表性冒烟/e2e 验证运行：
- `cargo test --test provider_native_verify b1_ -- --nocapture`
- 通过：`b1_alibaba_cn_{simple_text,tool_call_single,error_auth_401}`、`b1_kimi_for_coding_{simple_text,tool_call_single,error_auth_401}`、`b1_minimax_{simple_text,tool_call_single,error_auth_401}`。

规范映射决策：
- `kimi` 仍为规范 `moonshotai` 的别名。
- `kimi-for-coding` 为独立规范 ID，不别名至 `moonshotai`。
- `alibaba-cn` 区别于 `alibaba`/`dashscope`/`qwen`，使用 CN DashScope 路由默认值。
- `minimax-cn`、`minimax-coding-plan` 与 `minimax-cn-coding-plan` 通过共享家族行为 + 显式路由/认证锁继承代表性冒烟覆盖。

## Wave B2 接入验证（`bd-3uqg.5.1`）

批次 B2 提供方 ID 已集成并锁定测试：
`modelscope`、`moonshotai-cn`、`nebius`、`ovhcloud`、`scaleway`。

| 提供方 ID | API 家族 | 路由锁 | 流式/认证锁 | 状态 |
|-------------|------------|------------|------------------|--------|
| `modelscope` | `openai-completions` | yes | yes | pass |
| `moonshotai-cn` | `openai-completions` | yes | yes | pass |
| `nebius` | `openai-completions` | yes | yes | pass |
| `ovhcloud` | `openai-completions` | yes | yes | pass |
| `scaleway` | `openai-completions` | yes | yes | pass |

验证制品与代表性 VCR 夹具见 `tests/provider_native_verify.rs`（`wave_b2_smoke::b2_*`）及 `tests/fixtures/vcr/verify_*`。

规范映射决策：
- `modelscope`、`nebius`、`ovhcloud` 与 `scaleway` 为规范 OpenAI 兼容预设 ID。
- `moonshotai-cn` 为独立规范地域 ID，不别名至 `moonshotai`。
- `moonshotai` 与 `moonshotai-cn` 有意共享 `MOONSHOT_API_KEY`，但保留不同 base URL。

## Wave B3 接入验证（`bd-3uqg.5.3`）

批次 B3 提供方 ID 已集成并锁定测试：
`siliconflow`、`siliconflow-cn`、`upstage`、`venice`、`zai`、`zai-coding-plan`、`zhipuai`、`zhipuai-coding-plan`。

| 提供方 ID | API 家族 | 路由锁 | 流式/认证锁 | 状态 |
|-------------|------------|------------|------------------|--------|
| `siliconflow` | `openai-completions` | yes | yes | pass |
| `siliconflow-cn` | `openai-completions` | yes | yes | pass |
| `upstage` | `openai-completions` | yes | yes | pass |
| `venice` | `openai-completions` | yes | yes | pass |
| `zai` | `openai-completions` | yes | yes | pass |
| `zai-coding-plan` | `openai-completions` | yes | yes | pass |
| `zhipuai` | `openai-completions` | yes | yes | pass |
| `zhipuai-coding-plan` | `openai-completions` | yes | yes | pass |

规范映射决策：
- `siliconflow` 与 `siliconflow-cn` 为不同规范地域 ID，分别使用 `SILICONFLOW_API_KEY`、`SILICONFLOW_CN_API_KEY`。
- `zai` 与 `zai-coding-plan` 有意共享 `ZHIPU_API_KEY` 但保留不同 base URL。
- `zhipuai` 与 `zhipuai-coding-plan` 同理共享 `ZHIPU_API_KEY`。

## Wave C 暂存快照（`bd-3uqg.6`）

Wave C 默认值的事实来源：
- `https://models.dev/api.json`（查询于 2026-02-12）
- 提取命令见原文 `curl -s https://models.dev/api.json | jq ...`

Wave C 执行状态：

| 提供方 ID | API 家族目标 | 默认 base URL | 认证 env | 当前跟踪状态 |
|-------------|-------------|------------------|----------|-------------------------|
| `baseten` | `openai-completions` | `https://inference.baseten.co/v1` | `BASETEN_API_KEY` | Wave C 预设候选（`bd-3uqg.6.1`） |
| `llama` | `openai-completions` | `https://api.llama.com/compat/v1/` | `LLAMA_API_KEY` | Wave C 预设候选（`bd-3uqg.6.2`） |
| `lmstudio` | `openai-completions` | `http://127.0.0.1:1234/v1` | `LMSTUDIO_API_KEY` | Wave C 预设候选（`bd-3uqg.6.2`） |
| `ollama-cloud` | `openai-completions` | `https://ollama.com/v1` | `OLLAMA_API_KEY` | Wave C 预设候选（`bd-3uqg.6.2`） |
| `opencode` | `openai-completions` | `https://opencode.ai/zen/v1` | `OPENCODE_API_KEY` | 特殊路由待定（`bd-3uqg.3.9`） |
| `vercel` | gateway-wrapper（`@ai-sdk/gateway`） | `models.dev` 中无静态 API URL | `AI_GATEWAY_API_KEY` | 分类/路由待定 |
| `zenmux` | `anthropic-messages` 目标（网关） | `https://zenmux.ai/api/anthropic/v1` | `ZENMUX_API_KEY` | 特殊路由待定 |

## 规范提供方矩阵（当前基线 + 证据链接）

| 规范 ID（Canonical ID） | 别名（Aliases） | 能力标志（Capability flags） | API 家族 | Base URL 模板 | 认证模式（Auth mode） | 模式（Mode） | 运行时状态 | 验证证据 |
|--------------|---------|------------------|------------|-------------------|-----------|------|----------------|------------------------------------|
| `anthropic` | - | text + image + thinking + tool-calls | `anthropic-messages` | `https://api.anthropic.com/v1/messages` | `x-api-key`（`ANTHROPIC_API_KEY`）或 `auth.json` OAuth/API key | `native-implemented` | 已实现可调度 | [unit](../../tests/provider_streaming/anthropic.rs) |
| `openai` | - | text + image + reasoning + tool-calls | `openai-responses`（默认）、`openai-completions`（兼容） | `https://api.openai.com/v1` | `Authorization: Bearer`（`OPENAI_API_KEY`） | `native-implemented` | 已实现可调度 | [unit](../../tests/provider_streaming/openai.rs) |
| `openai-codex` | `codex`、`chatgpt-codex` | text + reasoning | `openai-codex-responses` | `https://chatgpt.com/backend-api/codex/responses` | ChatGPT/Codex OAuth 经 `/login openai-codex` | `native-adapter-required` | 经 Codex Responses 路径可调度 | [responses](../../src/providers/openai_responses.rs) |
| `google` | `gemini` | text + image + reasoning + tool-calls | `google-generative-ai` | `https://generativelanguage.googleapis.com/v1beta` | query key（`GOOGLE_API_KEY`，回退 `GEMINI_API_KEY`） | `native-implemented` | 已实现可调度 | [unit](../../tests/provider_streaming/gemini.rs) |
| `google-gemini-cli` | `gemini-cli` | text + image + reasoning | `google-gemini-cli` | 项目级 Code Assist 端点 | Google OAuth 经 `/login google-gemini-cli` | `native-adapter-required` | 经 Gemini CLI 路径可调度 | [gemini](../../src/providers/gemini.rs) |
| `google-antigravity` | `antigravity` | text + image + reasoning | `google-gemini-cli` | 项目级 Antigravity 端点 | Google OAuth 经 `/login google-antigravity` | `native-adapter-required` | 经 Gemini CLI 路径可调度 | [gemini](../../src/providers/gemini.rs) |
| `google-vertex` | `vertexai` | text + image + reasoning + tool-calls | `google-vertex` | `https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/{publisher}/models/{model}` | `Authorization: Bearer`（`GOOGLE_CLOUD_API_KEY`，备选 `VERTEX_API_KEY`） | `native-implemented` | 已实现可调度 | [unit](../../src/providers/vertex.rs) |
| `cohere` | - | text + tool-calls | `cohere-chat` | `https://api.cohere.com/v2` | `Authorization: Bearer`（`COHERE_API_KEY`） | `native-implemented` | 已实现可调度 | [unit](../../tests/provider_streaming/cohere.rs) |
| `azure-openai` | `azure`、`azure_openai` 等 | text + tool-calls | Azure chat/completions | `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}` | `api-key` 头（`AZURE_OPENAI_API_KEY`） | `native-implemented` | 经工厂可调度 | [unit](../../tests/provider_streaming/azure.rs) |
| `groq` | - | text | `openai-completions` | `https://api.groq.com/openai/v1` | `Authorization: Bearer`（`GROQ_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `deepinfra` | - | text | `openai-completions` | `https://api.deepinfra.com/v1/openai` | `Authorization: Bearer`（`DEEPINFRA_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `cerebras` | - | text | `openai-completions` | `https://api.cerebras.ai/v1` | `Authorization: Bearer`（`CEREBRAS_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `openrouter` | `open-router` | text | `openai-completions` | `https://openrouter.ai/api/v1` | `Authorization: Bearer`（`OPENROUTER_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `mistral` | - | text | `openai-completions` | `https://api.mistral.ai/v1` | `Authorization: Bearer`（`MISTRAL_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `moonshotai` | `moonshot`、`kimi` | text | `openai-completions` | `https://api.moonshot.ai/v1` | `Authorization: Bearer`（`MOONSHOT_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `deepseek` | - | text | `openai-completions` | `https://api.deepseek.com` | `Authorization: Bearer`（`DEEPSEEK_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `fireworks` | `fireworks-ai` | text | `openai-completions` | `https://api.fireworks.ai/inference/v1` | `Authorization: Bearer`（`FIREWORKS_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `togetherai` | - | text | `openai-completions` | `https://api.together.xyz/v1` | `Authorization: Bearer`（`TOGETHER_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `perplexity` | - | text | `openai-completions` | `https://api.perplexity.ai` | `Authorization: Bearer`（`PERPLEXITY_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `xai` | - | text | `openai-completions` | `https://api.x.ai/v1` | `Authorization: Bearer`（`XAI_API_KEY`） | `oai-compatible-preset` | 经 OpenAI 兼容回退可调度 | [metadata](../../tests/provider_metadata_comprehensive.rs) |
| `amazon-bedrock` | `bedrock` | text + tool-calls | `bedrock-converse-stream` | 区域化 AWS 端点 | SigV4/Bearer | `native-adapter-required` | VCR 已验证 | [cassette](../../tests/fixtures/vcr/verify_bedrock_simple_text.json) |
| `sap-ai-core` | `sap` | text + tool-calls | OAuth2 + OpenAI-compatible | SAP AI Core 服务 URL | OAuth2 客户端凭证 | `native-adapter-required` | VCR 已验证 | [cassette](../../tests/fixtures/vcr/verify_sap_ai_core_simple_text.json) |
| `github-copilot` | `copilot` | text + tool-calls | Copilot chat/completions | `https://api.githubcopilot.com` | `Authorization: Bearer`（`GITHUB_COPILOT_API_KEY`） | `native-adapter-required` | VCR 已验证 | [cassette](../../tests/fixtures/vcr/verify_copilot_simple_text.json) |
| `gitlab` | `gitlab-duo` | text | GitLab AI API | GitLab 实例 URL | `Authorization: Bearer`（`GITLAB_TOKEN`） | `native-adapter-required` | VCR 已验证 | [cassette](../../tests/fixtures/vcr/verify_gitlab_simple_text.json) |

> 完整矩阵含 80+ 规范 ID，上表为代表性摘录；完整清单以 `src/provider_metadata.rs` 与 `docs/provider-implementation-modes.json` 为准。

## 历史验证状态汇总

下表为 `docs/provider-native-parity-report.json` 的历史证据（`report.generated_at`: `2026-02-12T16:45:00Z`），不应作为当前原生模块清单使用。当前源码清单请使用上文提供方数量规则及核验 `src/providers/*.rs` 的治理测试。

| 类别（Category） | 数量 | VCR 覆盖 | 状态 |
|----------|-------|-------------|--------|
| 原生已实现元数据 ID | 6 | 6/6 (100%) | 完整 6 场景 VCR 套件 |
| 需原生适配器 | 4 | 4/4 (100%) | 4-6 场景 VCR 套件 |
| Wave B1-B3 预设 | 19 | 19/19 (100%) | 3 场景 VCR 套件 |
| Wave C 特殊路由 | 3 | 3/3 (100%) | 3 场景 VCR 套件 |
| 批次 A1-A4 预设 | 34 | 0/34 (0%) | 元数据+工厂已验证；个体 VCR 夹具待定（`bd-3uqg.8.4`） |

合并对等报告：[`docs/provider-native-parity-report.json`](provider-native-parity-report.json)

## 延期提供方与理由

| 提供方 ID | 分类 | 延期原因 | 用户影响（当前） | 毕业条件 | 跟踪（负责人） |
|-------------|---------------|-----------------|-----------------------|----------------------|------------------|
| `v0` | deferred-watchlist | `models.dev` 未发布 API 端点 | 用户无法直接以 `v0` 为目标 | Vercel 发布稳定 REST API 端点 | `bd-3uqg.11.10.8` |
| `google-vertex-anthropic` | native-new-high-risk | 需独立协议/认证路径 | Anthropic-on-Vertex 语义非一等 | 流式+工具调用对等验证通过 | `bd-3uqg.11.10.9` |
| `azure-cognitive-services` | native-new-high-risk | 需独立路由语义 | 可经 `azure-openai` 别名族到达 | 确认是否需独立路径 | `bd-3uqg.11.10.10` |
| `local` | native-new-high-risk | 需显式进程/模型生命周期集成 | 无一等本地进程生命周期提供方 | 本地生命周期适配器实现 | `bd-3uqg.11.10.11` |
| `ollama` | native-new-high-risk | 需专用进程编排适配器 | 本地 Ollama 守护进程非原生路径 | 进程生命周期适配器实现 | `bd-3uqg.11.10.11` |

### 批次 A1-A4 VCR 缺口

34 个提供方已完成元数据注册与工厂验证但缺乏独立 VCR 夹具：`302ai`、`abacus`、`aihubmix`、`bailing`、`berget`、`chutes`、`cortecs`、`fastrouter`、`firmware`、`friendli`、`github-models`、`helicone`、`huggingface`、`iflowcn`、`inception`、`inference`、`io-net`、`jiekou`、`lucidquery`、`moark`、`morph`、`nano-gpt`、`nova`、`novita-ai`、`nvidia`、`poe`、`privatemode-ai`、`requesty`、`submodel`、`synthetic`、`vivgrid`、`vultr`、`wandb`、`xiaomi`。

这些提供方可经 OpenAI 兼容回退调度并通过元数据 + 工厂测试。个体 VCR 夹具扩展在 `bd-3uqg.8.4` 跟踪。

## 别名迁移说明

本节记录全部别名到规范 ID 的映射及迁移指引。别名为向后兼容永久支持；别名归一化不引入破坏性变更。

### 迁移保障

全部别名在提供方选择时透明解析为规范 ID：
- 使用别名的配置文件（`"provider": "gemini"`）与规范形式（`"provider": "google"`）行为完全一致。
- 认证 env var 或账户凭证共享：别名与规范 ID 使用同一认证源。
- API 路由完全一致：解析至同一 base URL、API 家族与流式行为。
- 别名使用不产生弃用告警。

### 别名到规范映射表

| 别名（Alias） | 规范 ID（Canonical ID） | API 家族 | 共享认证 Env | 备注 |
|-------|-------------|------------|----------------------|-------|
| `gemini` | `google` | `google-generative-ai` | `GOOGLE_API_KEY`、`GEMINI_API_KEY` | Gemini 为模型家族；`google` 为规范提供方 ID。 |
| `codex` | `openai-codex` | `openai-codex-responses` | `/login openai-codex` | ChatGPT/Codex 账户桥接的简写。 |
| `moonshot` | `moonshotai` | `openai-completions` | `MOONSHOT_API_KEY` | `moonshot` 为原始 ID；`moonshotai` 为规范。 |
| `kimi` | `moonshotai` | `openai-completions` | `MOONSHOT_API_KEY`、`KIMI_API_KEY` | 注意：`kimi-for-coding` 为独立规范 ID。 |
| `qwen` | `alibaba` | `openai-completions` | `DASHSCOPE_API_KEY`、`QWEN_API_KEY` | Qwen 为模型家族。 |
| `fireworks-ai` | `fireworks` | `openai-completions` | `FIREWORKS_API_KEY` | 历史命名；`fireworks` 为规范。 |
| `vertexai` | `google-vertex` | `google-vertex` | `GOOGLE_CLOUD_API_KEY` | Vertex AI 别名。 |
| `bedrock` | `amazon-bedrock` | `bedrock-converse-stream` | `AWS_*` | 简写；`amazon-bedrock` 为规范。 |
| `copilot` | `github-copilot` | Copilot chat/completions | `GITHUB_COPILOT_API_KEY` | 简写；`github-copilot` 为规范。 |

常见坑位：
- `kimi` vs `kimi-for-coding`：前者为 `moonshotai` 别名（OpenAI 兼容），后者为独立规范 ID（`anthropic-messages`）。
- `alibaba` vs `alibaba-cn`：前者走国际 DashScope 端点，后者为独立 CN 端点。
- `moonshotai` vs `moonshotai-cn`：同认证 key，不同 base URL，独立规范 ID。

## 提供方选择与配置

凭证解析优先级（运行时）：
1. 显式 CLI 覆盖（`--api-key`）
2. 来自元数据的提供方 env var（有序，含 `GOOGLE_API_KEY` → `GEMINI_API_KEY` 等共享回退）
3. 已持久化的 `auth.json` 凭证（`ApiKey` 或未过期 OAuth `access_token`）
4. 内联 `models.json` 的 `apiKey` 回退（literal/env/file/shell 源解析）

认证诊断与脱敏契约：
- 全部认证诊断以 `redaction_policy=redact-secrets` 输出，永不在面向用户的提示中包含原始密钥。
- 提供方缺 key 提示派生自 `provider_auth_env_keys(...)`，别名继承规范 key 列表与顺序。

经由以下方式选择提供方/模型：
- CLI 标志：`pi --provider openai --model gpt-4o "Hello"`
- Env var：`PI_PROVIDER`、`PI_MODEL`
- 设置：`~/.pi/agent/settings.json` 中的 `default_provider`、`default_model`

自定义端点与覆盖应在 `models.json` 中配置，见 [models.md](models.md)。
