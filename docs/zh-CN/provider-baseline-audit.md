> 本文为英文原文的中文翻译，源文件：`docs/provider-baseline-audit.md`

# 提供方基线审计（Provider Baseline Audit）(`bd-3uqg.1.2`)

生成时间（Generated）：`2026-02-10T04:35:00Z`
上游快照（Upstream snapshot）：`bd-3uqg.1.1`（93 个规范提供方 ID）

> 仅为历史快照。当前真实来源（Source truth）为 `docs/providers.md` 加上
> `tests/traceability_staleness.rs::native_provider_module_inventory_matches_provider_docs`，
> 该测试会校验实时的 `src/providers/*.rs` 清单。下方的部分支持和缺失提供方章节早于
> Bedrock、Vertex、GitHub Copilot 和 GitLab 原生模块，不应视为当前状态。

## 执行摘要（Executive Summary）

| 类别（Category） | 数量（Count） | 占上游 93 项的比例（% of 93 upstream） |
|----------|-------|-------------------|
| 完全支持（原生模块 / native module） | 7 | 7.5% |
| 临时支持（OpenAI 兼容预设 / OAI-compatible preset） | 12 | 12.9% |
| 部分支持（仅认证/枚举 / auth/enum only） | 5 | 5.4% |
| 仅别名（Alias only） | 1 | 1.1% |
| 缺失（Missing） | 68 | 73.1% |

**有效覆盖率（Effective coverage）**：当前可用 19 个提供方（20.4%），另有 5 个部分接入，68 个完全未支持。

---

## 原生提供方模块（Native Provider Modules）（6 个文件，7 个结构体）

| 提供方（Provider） | 结构体（Struct） | 文件（File） | API 家族（API Family） | 认证环境变量（Auth Env Var） |
|----------|--------|------|-----------|--------------|
| anthropic | `AnthropicProvider` | `src/providers/anthropic.rs` | anthropic-messages | `ANTHROPIC_API_KEY` |
| openai | `OpenAIProvider` | `src/providers/openai.rs` | openai-completions | `OPENAI_API_KEY` |
| openai | `OpenAIResponsesProvider` | `src/providers/openai_responses.rs` | openai-responses | `OPENAI_API_KEY` |
| google | `GeminiProvider` | `src/providers/gemini.rs` | google-generative-ai | `GOOGLE_API_KEY` |
| cohere | `CohereProvider` | `src/providers/cohere.rs` | cohere-chat | `COHERE_API_KEY` |
| azure-openai | `AzureOpenAIProvider` | `src/providers/azure.rs` | azure-openai-responses | `AZURE_OPENAI_API_KEY` |
| (extension) | `ExtensionStreamSimpleProvider` | `src/providers/mod.rs` | dynamic | dynamic |

---

## 临时 OpenAI 兼容提供方（Ad-Hoc OpenAI-Compatible Providers）（12 项）

定义于 `src/models.rs:ad_hoc_provider_defaults()`。均使用 `openai-completions` API 家族，通过 `OpenAIProvider` 路由。

| 提供方 ID（Provider ID） | Pi 别名（Pi Aliases） | 基础 URL（Base URL） | 认证环境变量（Auth Env Var） | 上游匹配项（Upstream Match） |
|-------------|-----------|----------|--------------|----------------|
| groq | - | `api.groq.com/openai/v1` | `GROQ_API_KEY` | groq |
| deepinfra | - | `api.deepinfra.com/v1/openai` | `DEEPINFRA_API_KEY` | deepinfra |
| cerebras | - | `api.cerebras.ai/v1` | `CEREBRAS_API_KEY` | cerebras |
| openrouter | - | `openrouter.ai/api/v1` | `OPENROUTER_API_KEY` | openrouter |
| mistral | - | `api.mistral.ai/v1` | `MISTRAL_API_KEY` | mistral |
| moonshotai | moonshot, kimi | `api.moonshot.ai/v1` | `MOONSHOT_API_KEY` | moonshotai |
| alibaba | dashscope, qwen | `dashscope-intl.aliyuncs.com/compatible-mode/v1` | `DASHSCOPE_API_KEY` | alibaba |
| deepseek | - | `api.deepseek.com` | `DEEPSEEK_API_KEY` | deepseek |
| fireworks | - | `api.fireworks.ai/inference/v1` | `FIREWORKS_API_KEY` | fireworks-ai |
| togetherai | - | `api.together.xyz/v1` | `TOGETHER_API_KEY` | togetherai |
| perplexity | - | `api.perplexity.ai` | `PERPLEXITY_API_KEY` | perplexity |
| xai | - | `api.x.ai/v1` | `XAI_API_KEY` | xai |

---

## 部分支持（Partially Supported）（认证/枚举桩，无模块 / auth/enum stubs, no module）

| 提供方 ID（Provider ID） | 已有内容（What exists） | 缺失内容（What's missing） |
|-------------|-------------|----------------|
| amazon-bedrock | 认证环境变量（`AWS_ACCESS_KEY_ID`）、`Api::BedrockConverseStream` 枚举 | 无提供方模块 |
| google-vertex | 认证环境变量（`GOOGLE_CLOUD_API_KEY`）、`KnownProvider::GoogleVertex`、`Api::GoogleVertex` 枚举 | 无提供方模块 |
| github-copilot | 认证环境变量（`GITHUB_COPILOT_API_KEY`）、`KnownProvider::GithubCopilot` 枚举 | 无提供方模块 |
| bedrock (opencode ID) | 同 amazon-bedrock | - |
| copilot (opencode ID) | 同 github-copilot | - |

---

## Pi 与上游之间的 ID 不一致（ID Mismatches Between Pi and Upstream）

| 上游 ID（Upstream ID） | Pi ID | 来源（Source） | 备注（Notes） |
|------------|-------|--------|-------|
| azure | azure-openai | models.dev | Pi 包含 '-openai' 后缀 |
| fireworks-ai | fireworks | models.dev | Pi 去掉了 '-ai' 后缀 |
| amazon-bedrock / bedrock | - | models.dev / opencode | 不同来源使用不同 ID |
| github-copilot / copilot | - | models.dev / opencode | 不同来源使用不同 ID |
| google / gemini | google | models.dev / opencode | opencode 使用产品名 'gemini' |
| google-vertex / vertexai | - | models.dev / opencode | 命名约定不同 |

---

## 工厂选择逻辑（Factory Selection Logic）(`src/providers/mod.rs:create_provider()`)

1. **首先检查扩展提供方（Extension providers checked first）**：`manager.provider_has_stream_simple(&provider_id)` -> `ExtensionStreamSimpleProvider`
2. **按 `entry.model.provider` 匹配（Match on `entry.model.provider`）**：anthropic | openai（按 api 字段分支到 completions/responses） | cohere | google | azure-openai
3. **回退按 `entry.model.api` 匹配（Fallback on `entry.model.api`）**：anthropic-messages | openai-completions | openai-responses | cohere-chat | google-generative-ai
4. **否则（Otherwise）**：错误 "Provider not implemented"

---

## 按上游来源统计的覆盖率（Coverage by Upstream Source）

| 来源（Source） | ID 总数（Total IDs） | 已支持（Supported） | 覆盖率（Coverage） |
|--------|-----------|-----------|----------|
| models.dev | 87 | 19 (native+ad-hoc) | 21.8% |
| opencode | 11 | 8 full + 3 partial | 72.7% |
| codex | 3 | 1 (openai) | 33.3% |

---

## 缺失的提供方（Missing Providers）（来自上游并集的 68 项）

高价值缺失项（位于 opencode 或 codex 中）：`sap-ai-core`、`cloudflare-ai-gateway`、`cloudflare-workers-ai`、`gitlab`、`lmstudio`、`ollama`、`zenmux`、`opencode`、`vercel`

其他缺失项：302ai, abacus, aihubmix, alibaba-cn, azure-cognitive-services, bailing, baseten, berget, chutes, cortecs, fastrouter, firmware, friendli, github-copilot-enterprise, github-models, google-vertex-anthropic, helicone, huggingface, iflowcn, inception, inference, io-net, jiekou, llama, lucidquery, minimax, minimax-cn, minimax-cn-coding-plan, minimax-coding-plan, moark, modelscope, moonshotai-cn, morph, nano-gpt, nebius, nova, novita-ai, nvidia, ollama-cloud, ovhcloud, poe, privatemode-ai, requesty, scaleway, siliconflow, siliconflow-cn, submodel, synthetic, upstage, v0, venice, vivgrid, vultr, wandb, xiaomi, zai, zai-coding-plan, zhipuai, zhipuai-coding-plan

---

## 架构说明（Architectural Notes）

- `Provider` trait 要求：`name()`、`api()`、`model_id()`、`stream()`——全部 6 个原生模块均已实现
- 临时提供方（Ad-hoc providers）使用 `ad_hoc_model_entry()` 在用户指定已知提供方 ID 时动态创建 `ModelEntry`
- 扩展（Extension）的 `streamSimple` 可在无需修改原生代码的情况下覆盖**任意**缺口
- `Api` 枚举包含前向声明的变体（`BedrockConverseStream`、`GoogleVertex`），尚无对应模块
- Anthropic 与扩展提供方已具备 OAuth 框架；其他原生提供方仅使用 API 密钥认证

---

## 提供方测试覆盖率映射种子（Provider Test Coverage Map Seed）(`bd-3uqg.8`，更新于 2026-02-12)

本次更新的真实来源（Source of truth for this update）：
- `docs/provider-native-parity-report.json`（`report.generated_at`：`2026-02-12T16:45:00Z`）
- `tests/provider_native_verify.rs`
- `tests/provider_metadata_comprehensive.rs`
- `tests/provider_factory.rs`

当前测试通道健康度（Current test-lane health）：

| 通道（Lane） | 通过（Passed） | 失败（Failed） | 总数（Total） | 状态（Status） | 备注（Notes） |
|------|--------|--------|-------|--------|-------|
| `provider_native_verify` | 206 | 0 | 206 | green | 原生与预设一致性夹具均通过。 |
| `provider_metadata_comprehensive` | 112 | 0 | 112 | green | 元数据/路由覆盖率正常。 |
| `provider_factory` | 134 | 10 | 144 | yellow | 仅基础设施失败，因缺失 `pi_runtime.json` VCR 磁带（VCR cassette）（非提供方逻辑回归）。 |

覆盖率足迹快照（Coverage footprint snapshot）：

| 指标（Metric） | 数值（Value） |
|--------|-------|
| 已注册提供方总数（Total registered providers） | 84 |
| 已通过 VCR 验证的提供方（Providers with VCR verification） | 29 |
| 未通过 VCR 验证的提供方（Providers without VCR verification） | 55 |
| VCR 夹具场景总数（Total VCR fixture scenarios） | 114 |
| 通过的测试场景总数（Total test scenarios passing） | 206 |
| 失败的测试场景总数（Total test scenarios failing） | 0 |

已通过 VCR 验证的提供方层级（VCR-verified provider tiers）：
- Tier-1 内置原生（6 场景 / 6-scenario）：`6`
- Tier-2 原生适配器（Tier-2 native adapter）：`4`
- Wave B1 区域/编码计划（3 场景 / 3-scenario）：`3`
- Wave B2 区域/云（3 场景 / 3-scenario）：`5`
- Wave B3（3 场景 / 3-scenario）：`8`
- Wave C 特殊路由（3 场景 / 3-scenario）：`3`

已知偏差及对应跟进项（Known deviations and mapped follow-ups）：

| 偏差 ID（Deviation ID） | 范围（Scope） | 缺口（Gap） | 跟进 bead（Follow-up bead） |
|--------------|-------|-----|----------------|
| `DEV-001` | `gitlab` | 缺失 `tool_call_single` VCR 夹具（API 形态不匹配）。 | `bd-3uqg.3` |
| `DEV-002` | `amazon-bedrock` | 缺失 `error_bad_request_400` 和 `error_rate_limit_429` 夹具。 | `bd-3uqg.8.2` (contract coverage expansion) |
| `DEV-003` | Wave B + C presets | 仅 3 场景覆盖（`simple_text`、`tool_call_single`、`error_auth_401`）对比 Tier-1 基线的 6 场景。 | `bd-3uqg.8` |
| `DEV-004` | `provider_factory` lane | 因缺失 `pi_runtime.json` 磁带（cassette）导致 10 项失败。 | `bd-3uqg.8.4` |

对 `bd-3uqg.8` 的执行影响（Execution implications for `bd-3uqg.8`）：
1. 优先处理 `bd-3uqg.8.2` 以闭环原生适配器契约（contract）收口（Bedrock/GitLab 差异 + 模式断言）。
2. 落地 `bd-3uqg.8.4` 以修复基础设施磁带缺口并稳定工厂冒烟覆盖率。
3. 保持文档矩阵工作（`bd-3uqg.9.1.2`）与本次一致性（conformance）证据对齐，使能力（capability）/认证（auth）/API 声明保持由制品支撑。
