# 提供方认证排障矩阵 (bd-3uqg.11.12.2)

各提供方在 gap 与长尾场景下的认证故障模式及精确修复路径，关联测试证据与 `src/error.rs` 中的错误提示系统。

已与实时注册表及认证运行时对账：2026-08-06

## 快速参考

| 提供方 | 主环境变量 | 备用环境变量 | 获取密钥 |
|---|---|---|---|
| groq | `GROQ_API_KEY` | - | console.groq.com |
| cerebras | `CEREBRAS_API_KEY` | - | cloud.cerebras.ai |
| openrouter | `OPENROUTER_API_KEY` | - | openrouter.ai/keys |
| moonshotai | `MOONSHOT_API_KEY` | `KIMI_API_KEY` | platform.moonshot.cn |
| alibaba | `DASHSCOPE_API_KEY` | `QWEN_API_KEY` | dashscope.console.aliyun.com |
| stackit | `STACKIT_API_KEY` | - | portal.stackit.cloud |
| mistral | `MISTRAL_API_KEY` | - | console.mistral.ai |
| deepinfra | `DEEPINFRA_API_KEY` | - | deepinfra.com/dash |
| togetherai | `TOGETHER_API_KEY` | - | api.together.xyz |
| nvidia | `NVIDIA_API_KEY` | - | build.nvidia.com |
| huggingface | `HF_TOKEN` | - | huggingface.co/settings/tokens |
| ollama-cloud | `OLLAMA_API_KEY` | - | ollama.com |

## 提供方名称对照表（规范 ID / 别名 / 环境变量 / 端点）

本对照表将所有面向用户的提供方名称（含来自 opencode 与 models.dev 的上游别名）映射到 Pi 规范 ID、可用别名、认证环境变量及默认端点。当用户报告“缺失提供方”或对使用哪个名称感到困惑时，请使用本表。

**总计**：102 个已注册规范提供方 ID 与 61 个别名。注册覆盖度并不代表每个 ID 都有可执行的原生链路；运行时状态请使用 `pi --list-providers` 及实现模式证据确认。

### 原生提供方（专用适配器）

| 规范 ID | 别名 | 认证环境变量 | 默认端点 | API 类型 |
|---|---|---|---|---|
| `anthropic` | — | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1/messages` | anthropic-messages |
| `openai` | — | `OPENAI_API_KEY` | `https://api.openai.com/v1` | openai-responses |
| `openai-codex` | `codex`, `chatgpt-codex` | _(无；请使用 `/login openai-codex`)_ | `https://chatgpt.com/backend-api/codex/responses` | openai-codex-responses |
| `google` | `gemini` | `GOOGLE_API_KEY`, `GEMINI_API_KEY` | `https://generativelanguage.googleapis.com/v1beta` | google-generative-ai |
| `google-gemini-cli` | `gemini-cli` | _(无；请使用 `/login google-gemini-cli`)_ | _(按项目划分的 Code Assist 端点)_ | google-gemini-cli |
| `google-antigravity` | `antigravity` | _(无；请使用 `/login google-antigravity`)_ | _(按项目划分的 Antigravity 端点)_ | google-gemini-cli |
| `cohere` | — | `COHERE_API_KEY` | `https://api.cohere.com/v2` | cohere-chat |
| `google-vertex` | `vertexai`, `google-vertex-anthropic` | `GOOGLE_CLOUD_API_KEY`, `VERTEX_API_KEY` | _(按项目划分的 URL)_ | google-vertex |
| `amazon-bedrock` | `bedrock` | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `AWS_BEARER_TOKEN_BEDROCK`, `AWS_PROFILE`, `AWS_REGION` | _(按区域划分的 URL)_ | bedrock-converse-stream |
| `azure-openai` | `azure`, `azure_openai`, `azure-cognitive-services`, `azure-openai-responses` | `AZURE_OPENAI_API_KEY` | _(按资源划分的 URL)_ | native-azure |
| `github-copilot` | `copilot`, `github-copilot-enterprise` | `GITHUB_COPILOT_API_KEY`, `GITHUB_TOKEN` | _(令牌交换)_ | native-copilot |
| `gitlab` | `gitlab-duo` | `GITLAB_TOKEN`, `GITLAB_API_KEY` | _(可配置实例)_ | native-gitlab |
| `cursor` | `cursor-agent` | `CURSOR_API_KEY`, `CURSOR_ACCESS_TOKEN` | `https://api2.cursor.sh/agent.v1.AgentService/Run` | cursor-agent |
| `sap-ai-core` | `sap` | `AICORE_SERVICE_KEY`, `SAP_AI_CORE_CLIENT_ID`, `SAP_AI_CORE_CLIENT_SECRET`, `SAP_AI_CORE_TOKEN_URL`, `SAP_AI_CORE_SERVICE_URL` | _(按实例划分)_ | native-sap |
| `v0` | — | `V0_API_KEY` | _(按实例划分)_ | native-v0 |

### OpenAI 兼容预设（主要）

| 规范 ID | 别名 | 认证环境变量 | 默认端点 |
|---|---|---|---|
| `groq` | — | `GROQ_API_KEY` | `https://api.groq.com/openai/v1` |
| `cerebras` | — | `CEREBRAS_API_KEY` | `https://api.cerebras.ai/v1` |
| `atlascloud` | `atlas-cloud`, `atlas` | `ATLASCLOUD_API_KEY`, `ATLAS_CLOUD_API_KEY` | `https://api.atlascloud.ai/v1` |
| `openrouter` | `open-router` | `OPENROUTER_API_KEY` | `https://openrouter.ai/api/v1` |
| `mistral` | `mistralai` | `MISTRAL_API_KEY` | `https://api.mistral.ai/v1` |
| `deepseek` | `deep-seek` | `DEEPSEEK_API_KEY` | `https://api.deepseek.com` |
| `deepinfra` | `deep-infra` | `DEEPINFRA_API_KEY` | `https://api.deepinfra.com/v1/openai` |
| `fireworks` | `fireworks-ai` | `FIREWORKS_API_KEY` | `https://api.fireworks.ai/inference/v1` |
| `togetherai` | `together`, `together-ai` | `TOGETHER_API_KEY`, `TOGETHER_AI_API_KEY` | `https://api.together.xyz/v1` |
| `perplexity` | `pplx` | `PERPLEXITY_API_KEY` | `https://api.perplexity.ai` |
| `xai` | `grok`, `x-ai` | `XAI_API_KEY` | `https://api.x.ai/v1` |
| `nvidia` | `nim`, `nvidia-nim` | `NVIDIA_API_KEY` | `https://integrate.api.nvidia.com/v1` |
| `huggingface` | `hf`, `hugging-face` | `HF_TOKEN` | `https://router.huggingface.co/v1` |
| `moonshotai` | `moonshot`, `kimi` | `MOONSHOT_API_KEY`, `KIMI_API_KEY` | `https://api.moonshot.ai/v1` |
| `alibaba` | `dashscope`, `qwen` | `DASHSCOPE_API_KEY`, `QWEN_API_KEY` | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` |

### OpenAI 兼容预设（区域 + 编程套餐）

| 规范 ID | 别名 | 认证环境变量 | 默认端点 |
|---|---|---|---|
| `alibaba-cn` | — | `DASHSCOPE_API_KEY` | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| `alibaba-us` | — | `DASHSCOPE_API_KEY`, `QWEN_API_KEY` | `https://dashscope-us.aliyuncs.com/compatible-mode/v1` |
| `moonshotai-cn` | — | `MOONSHOT_API_KEY` | `https://api.moonshot.cn/v1` |
| `siliconflow` | `silicon-flow` | `SILICONFLOW_API_KEY` | `https://api.siliconflow.com/v1` |
| `siliconflow-cn` | — | `SILICONFLOW_CN_API_KEY` | `https://api.siliconflow.cn/v1` |
| `modelscope` | — | `MODELSCOPE_API_KEY` | `https://api-inference.modelscope.cn/v1` |
| `nebius` | — | `NEBIUS_API_KEY` | `https://api.tokenfactory.nebius.com/v1` |
| `ovhcloud` | — | `OVHCLOUD_API_KEY` | `https://oai.endpoints.kepler.ai.cloud.ovh.net/v1` |
| `scaleway` | — | `SCALEWAY_API_KEY` | `https://api.scaleway.ai/v1` |
| `stackit` | — | `STACKIT_API_KEY` | `https://api.openai-compat.model-serving.eu01.onstackit.cloud/v1` |
| `upstage` | — | `UPSTAGE_API_KEY` | `https://api.upstage.ai/v1/solar` |
| `venice` | — | `VENICE_API_KEY` | `https://api.venice.ai/api/v1` |
| `zhipuai` | `zhipu`, `glm` | `ZHIPU_API_KEY` | `https://open.bigmodel.cn/api/paas/v4` |
| `zhipuai-coding-plan` | — | `ZHIPU_API_KEY` | `https://open.bigmodel.cn/api/coding/paas/v4` |
| `zai` | — | `ZHIPU_API_KEY` | `https://api.z.ai/api/paas/v4` |
| `zai-coding-plan` | — | `ZHIPU_API_KEY` | `https://api.z.ai/api/coding/paas/v4` |

### Anthropic 兼容预设

| 规范 ID | 别名 | 认证环境变量 | 默认端点 |
|---|---|---|---|
| `kimi-for-coding` | `kimi-coding`, `kimi-code` | `KIMI_API_KEY` | `https://api.kimi.com/coding/v1/messages` |
| `minimax` | — | `MINIMAX_API_KEY` | `https://api.minimax.io/anthropic/v1/messages` |
| `minimax-cn` | — | `MINIMAX_CN_API_KEY` | `https://api.minimaxi.com/anthropic/v1/messages` |
| `minimax-coding-plan` | — | `MINIMAX_API_KEY` | `https://api.minimax.io/anthropic/v1/messages` |
| `minimax-cn-coding-plan` | — | `MINIMAX_CN_API_KEY` | `https://api.minimaxi.com/anthropic/v1/messages` |
| `zenmux` | — | `ZENMUX_API_KEY` | `https://zenmux.ai/api/anthropic/v1/messages` |
| `umans` | `umans-ai` | `UMANS_AI_CODING_PLAN_API_KEY` | `https://api.code.umans.ai/v1/messages` |

`kimi-for-coding` 有两条有意的请求认证链路。以 `sk-*` 开头的直连 API 密钥（含通过 `KIMI_API_KEY` 传入的）会以 `x-api-key` 发送。通过 `/login kimi-for-coding` 获取或从 Kimi CLI 导入的令牌则以 `Authorization: Bearer` 发送，并附带所需的 Kimi 设备头。当 OAuth/外部令牌生效时，请勿将缺失 `x-api-key` 诊断为故障。

### OpenAI 兼容预设（长尾）

| 规范 ID | 别名 | 认证环境变量 | 默认端点 |
|---|---|---|---|
| `302ai` | — | `302AI_API_KEY` | `https://api.302.ai/v1` |
| `abacus` | — | `ABACUS_API_KEY` | `https://routellm.abacus.ai/v1` |
| `aihubmix` | — | `AIHUBMIX_API_KEY` | `https://aihubmix.com/v1` |
| `bailing` | — | `BAILING_API_TOKEN` | `https://api.tbox.cn/api/llm/v1` |
| `baseten` | — | `BASETEN_API_KEY` | `https://inference.baseten.co/v1` |
| `berget` | — | `BERGET_API_KEY` | `https://api.berget.ai/v1` |
| `chutes` | — | `CHUTES_API_KEY` | `https://llm.chutes.ai/v1` |
| `cloudflare-ai-gateway` | — | `CLOUDFLARE_API_TOKEN` | `https://gateway.ai.cloudflare.com/v1/...` |
| `cloudflare-workers-ai` | — | `CLOUDFLARE_API_TOKEN` | `https://api.cloudflare.com/client/v4/accounts/.../ai/v1` |
| `coreweave` | `coreweave-serverless` | `COREWEAVE_API_KEY`, `WANDB_API_KEY` | `https://api.inference.wandb.ai/v1` |
| `cortecs` | — | `CORTECS_API_KEY` | `https://api.cortecs.ai/v1` |
| `fastrouter` | — | `FASTROUTER_API_KEY` | `https://go.fastrouter.ai/api/v1` |
| `firmware` | — | `FIRMWARE_API_KEY` | `https://app.firmware.ai/api/v1` |
| `friendli` | — | `FRIENDLI_TOKEN` | `https://api.friendli.ai/serverless/v1` |
| `gmi` | `gmi-cloud`, `gmi-serving` | `GMI_API_KEY` | `https://api.gmi-serving.com/v1` |
| `helicone` | — | `HELICONE_API_KEY` | `https://ai-gateway.helicone.ai/v1` |
| `iflowcn` | — | `IFLOW_API_KEY` | `https://apis.iflow.cn/v1` |
| `inception` | — | `INCEPTION_API_KEY` | `https://api.inceptionlabs.ai/v1` |
| `inference` | — | `INFERENCE_API_KEY` | `https://inference.net/v1` |
| `io-net` | — | `IOINTELLIGENCE_API_KEY` | `https://api.intelligence.io.solutions/api/v1` |
| `jiekou` | — | `JIEKOU_API_KEY` | `https://api.jiekou.ai/openai` |
| `kilo` | `kilo-gateway`, `kilo-ai` | `KILO_API_KEY` | `https://api.kilo.ai/api/gateway` |
| `llama` | — | `LLAMA_API_KEY` | `https://api.llama.com/compat/v1` |
| `llamacpp` | `llama-cpp`, `llama.cpp`, `llama-server` | _(无；本地 llama-server)_ | `http://127.0.0.1:8080/v1` |
| `lmstudio` | `lm-studio` | `LMSTUDIO_API_KEY` | `http://127.0.0.1:1234/v1` |
| `lucidquery` | — | `LUCIDQUERY_API_KEY` | `https://lucidquery.com/api/v1` |
| `mistralrs` | `mistral-rs`, `mistral.rs` | _(无；本地 mistral.rs 服务)_ | `http://127.0.0.1:1234/v1` |
| `moark` | — | `MOARK_API_KEY` | `https://moark.com/v1` |
| `morph` | — | `MORPH_API_KEY` | `https://api.morphllm.com/v1` |
| `nano-gpt` | `nanogpt` | `NANO_GPT_API_KEY` | `https://nano-gpt.com/api/v1` |
| `nova` | — | `NOVA_API_KEY` | `https://api.nova.amazon.com/v1` |
| `novita-ai` | `novita` | `NOVITA_API_KEY` | `https://api.novita.ai/openai` |
| `ollama` | — | _(无)_ | `http://127.0.0.1:11434/v1` |
| `ollama-cloud` | — | `OLLAMA_API_KEY` | `https://ollama.com/v1` |
| `opencode` | `opencode-zen` | `OPENCODE_API_KEY` | `https://opencode.ai/zen/v1` |
| `opencode-go` | — | `OPENCODE_API_KEY` | `https://opencode.ai/zen/go/v1` |
| `poe` | — | `POE_API_KEY` | `https://api.poe.com/v1` |
| `privatemode-ai` | — | `PRIVATEMODE_API_KEY` | `http://localhost:8080/v1` |
| `qianfan` | `baidu-qianfan` | `QIANFAN_API_KEY` | `https://qianfan.baidubce.com/v2` |
| `requesty` | — | `REQUESTY_API_KEY` | `https://router.requesty.ai/v1` |
| `sakana` | `sakana-ai` | `SAKANA_API_KEY`, `FUGU_API_KEY` | `https://api.sakana.ai/v1` (openai-responses) |
| `submodel` | — | `SUBMODEL_INSTAGEN_ACCESS_KEY` | `https://llm.submodel.ai/v1` |
| `synthetic` | — | `SYNTHETIC_API_KEY` | `https://api.synthetic.new/v1` |
| `vercel` | `vercel-ai-gateway` | `AI_GATEWAY_API_KEY` | `https://ai-gateway.vercel.sh/v1` |
| `vivgrid` | — | `VIVGRID_API_KEY` | `https://api.vivgrid.com/v1` |
| `vultr` | — | `VULTR_API_KEY` | `https://api.vultrinference.com/v1` |
| `wafer` | `wafer-serverless` | `WAFER_SERVERLESS_API_KEY` | `https://pass.wafer.ai/v1` |
| `wandb` | — | `WANDB_API_KEY` | `https://api.inference.wandb.ai/v1` |
| `xiaomi` | — | `XIAOMI_API_KEY` | `https://api.xiaomimimo.com/v1` |

### 别名解析总览

若用户输入以下任意别名（左侧），Pi 将解析为规范 ID（右侧）：

| 用户输入 | 解析为 |
|------------|------------|
| `gemini` | `google` |
| `codex`, `chatgpt-codex` | `openai-codex` |
| `gemini-cli` | `google-gemini-cli` |
| `antigravity` | `google-antigravity` |
| `open-router` | `openrouter` |
| `moonshot`, `kimi` | `moonshotai` |
| `dashscope`, `qwen` | `alibaba` |
| `deep-seek` | `deepseek` |
| `deep-infra` | `deepinfra` |
| `fireworks-ai` | `fireworks` |
| `together`, `together-ai` | `togetherai` |
| `pplx` | `perplexity` |
| `grok`, `x-ai` | `xai` |
| `nim`, `nvidia-nim` | `nvidia` |
| `hf`, `hugging-face` | `huggingface` |
| `mistralai` | `mistral` |
| `vertexai`, `google-vertex-anthropic` | `google-vertex` |
| `bedrock` | `amazon-bedrock` |
| `sap` | `sap-ai-core` |
| `azure`, `azure_openai`, `azure-cognitive-services`, `azure-openai-responses` | `azure-openai` |
| `copilot`, `github-copilot-enterprise` | `github-copilot` |
| `gitlab-duo` | `gitlab` |
| `silicon-flow` | `siliconflow` |
| `zhipu`, `glm` | `zhipuai` |
| `nanogpt` | `nano-gpt` |
| `novita` | `novita-ai` |
| `lm-studio` | `lmstudio` |
| `kimi-coding`, `kimi-code` | `kimi-for-coding` |
| `vercel-ai-gateway` | `vercel` |
| `atlas`, `atlas-cloud` | `atlascloud` |
| `cursor-agent` | `cursor` |
| `llama-cpp`, `llama.cpp`, `llama-server` | `llamacpp` |
| `mistral-rs`, `mistral.rs` | `mistralrs` |
| `gmi-cloud`, `gmi-serving` | `gmi` |
| `coreweave-serverless` | `coreweave` |
| `sakana-ai` | `sakana` |
| `wafer-serverless` | `wafer` |
| `baidu-qianfan` | `qianfan` |
| `umans-ai` | `umans` |
| `kilo-gateway`, `kilo-ai` | `kilo` |
| `opencode-zen` | `opencode` |

### 共享环境变量组

部分不同规范 ID 共享环境变量（为提供方族有意设计）：

| 共享环境变量 | 规范 ID |
|---------------|--------------|
| `DASHSCOPE_API_KEY` | `alibaba`, `alibaba-cn` |
| `MOONSHOT_API_KEY` | `moonshotai`, `moonshotai-cn` |
| `ZHIPU_API_KEY` | `zhipuai`, `zhipuai-coding-plan`, `zai`, `zai-coding-plan` |
| `MINIMAX_API_KEY` | `minimax`, `minimax-coding-plan` |
| `MINIMAX_CN_API_KEY` | `minimax-cn`, `minimax-cn-coding-plan` |
| `CLOUDFLARE_API_TOKEN` | `cloudflare-ai-gateway`, `cloudflare-workers-ai` |
| `WANDB_API_KEY` | `wandb`, `coreweave` |
| `OPENCODE_API_KEY` | `opencode`, `opencode-go` |

**验证**：`cargo test --test provider_metadata_comprehensive provider_auth_reference_artifacts_match_runtime_metadata -- --exact`

原 `github-models` 预设已在 GitHub 于 2026-07-30 下线 GitHub Models 服务后移除。`github-copilot` 为独立的原生提供方，仍受支持。

## 故障模式矩阵

### 1. 缺少 API 密钥

**症状**：`Missing API key` 或 `No API key provided`

**错误提示摘要**："Provider API key is missing."（提供方 API 密钥缺失。）

**按提供方修复**：

| 提供方 | 修复方法 |
|---|---|
| groq | `export GROQ_API_KEY=gsk_...` |
| cerebras | `export CEREBRAS_API_KEY=csk-...` |
| openrouter | `export OPENROUTER_API_KEY=sk-or-...` |
| moonshotai | `export MOONSHOT_API_KEY=sk-...` 或 `export KIMI_API_KEY=sk-...` |
| alibaba | `export DASHSCOPE_API_KEY=sk-...` 或 `export QWEN_API_KEY=sk-...` |
| stackit | `export STACKIT_API_KEY=...` |
| mistral | `export MISTRAL_API_KEY=...` |
| deepinfra | `export DEEPINFRA_API_KEY=...` |
| togetherai | `export TOGETHER_API_KEY=...` |
| nvidia | `export NVIDIA_API_KEY=nvapi-...` |
| huggingface | `export HF_TOKEN=hf_...` |
| ollama-cloud | `export OLLAMA_API_KEY=...` |

**测试证据**：`cargo test --test provider_native_contract -- failure_taxonomy::all_providers_produce_hint_summary_for_missing_key`

### 2. 认证失败（HTTP 401）

**症状**：`401 Unauthorized`、`Invalid API key`、`API key expired`

**错误提示摘要**："Provider authentication failed."（提供方认证失败。）

**常见原因**：
- API 密钥拼写错误
- 密钥已被撤销或过期
- 提供方与密钥不匹配（例如将 Groq 密钥用于 Cerebras）
- 密钥配置了 IP/引用来源限制策略

**修复步骤**：
1. 验证密钥已设置：`echo $GROQ_API_KEY`（或对应变量）
2. 使用 curl 测试：`curl -H "Authorization: Bearer $GROQ_API_KEY" https://api.groq.com/openai/v1/models`
3. 从提供方控制台重新生成密钥
4. 检查密钥是否被限制为特定 IP

**测试证据**：`cargo test --test provider_native_contract -- failure_taxonomy::all_providers_produce_hint_for_auth_failure`

### 3. 限流（HTTP 429）

**症状**：`429 Too Many Requests`、`Rate limit exceeded`

**错误提示摘要**："Provider rate limited the request."（提供方对请求限流。）

**修复步骤**：
1. 等待后重试（提供方通常有按分钟配额）
2. 降低 `max_tokens` 以减少单次请求的计算量
3. 在提供方控制台查看当前限流阈值
4. 考虑升级到更高级别的套餐

**提供方限流详情**：

| 提供方 | 典型限制 | 说明 |
|---|---|---|
| groq | 30 RPM（免费层） | 提供更高层级 |
| cerebras | 因模型而异 | 查看控制台 |
| openrouter | 取决于上游提供方 | 限流会级联 |
| moonshotai | 因套餐而异 | 可能有区域限制 |
| alibaba | 因模型而异 | DashScope 配额体系 |
| mistral | 因层级而异 | API 密钥控制台显示限制 |

**测试证据**：`cargo test --test provider_native_contract -- failure_taxonomy::all_providers_produce_hint_for_rate_limit`

### 4. 禁止访问（HTTP 403）

**症状**：`403 Forbidden`、`Access denied`

**错误提示摘要**："Provider access forbidden."（提供方禁止访问。）

**常见原因**：
- 账户无权访问所请求的模型
- 组织/项目限制
- 地域限制

**修复步骤**：
1. 验证模型 ID 正确且账户可用
2. 检查组织级权限
3. 联系提供方支持以提升访问权限

### 5. 配额超限

**症状**：`insufficient_quota`、`billing hard limit`、`not enough credits`

**错误提示摘要**："Provider quota or billing limit reached."（提供方配额或计费上限已达。）

**修复步骤**：
1. 在提供方控制台检查计费状态
2. 充值或更新付款方式
3. 审查消费限额并按需调整

### 6. 过载（HTTP 529）

**症状**：`529 Overloaded`、`Service temporarily unavailable`

**错误提示摘要**："Provider is overloaded."（提供方过载。）

**修复步骤**：
1. 等待后重试（通常数分钟内恢复）
2. 考虑切换到负载较低的模型
3. 若持续存在，请查看提供方状态页

## 环境变量优先级

对于拥有多个环境变量的提供方，优先级如下：

| 提供方 | 优先级（命中即胜出） |
|---|---|
| moonshotai | `MOONSHOT_API_KEY` > `KIMI_API_KEY` |
| alibaba | `DASHSCOPE_API_KEY` > `QWEN_API_KEY` |

其余提供方仅有一个环境变量。

**测试证据**：`cargo test --test provider_native_contract -- failure_taxonomy::provider_key_hints_reference_correct_env_var`

## 运行时错误提示系统

`src/error.rs` 中的错误提示系统提供结构化修复指引：

```rust
// Example: creating a provider error
// 示例：创建提供方错误
let err = Error::Provider {
    provider: "groq".to_string(),
    message: "401 Unauthorized".to_string(),
};
let hints = err.hints();
// hints.summary: "Provider authentication failed."
// hints.hints: ["Set `GROQ_API_KEY` for provider `groq`.", "If using OAuth, run `/login` again."]
// hints.context: [("provider", "groq"), ("details", "401 Unauthorized")]
```

该提示系统已针对全部 12 个故障分类提供方 ID、覆盖 7 个故障类别进行测试：
`cargo test --test provider_native_contract -- failure_taxonomy`

## 认证故障签名目录（原生提供方）

本节为每个原生提供方族归档具体的认证故障签名，包含诊断码、响应体形态及 VCR 证据链接。

### 诊断码参考

`AuthDiagnosticCode` 枚举（`src/error.rs:67-81`）提供稳定的机器码。每个码包含 wire 字符串、修复文案及脱敏策略（所有码均为 `redact-secrets`）。

| 编码 | Wire 字符串 | 触发条件 |
|------|-------------|------------|
| `MissingApiKey` | `auth.missing_api_key` | 请求前：环境变量/配置/覆盖中均无密钥 |
| `InvalidApiKey` | `auth.invalid_api_key` | HTTP 401/403、"unauthorized"、"invalid api key" |
| `QuotaExceeded` | `auth.quota_exceeded` | "insufficient_quota"、"billing hard limit" |
| `OAuthTokenExchangeFailed` | `auth.oauth.token_exchange_failed` | "token exchange failed" |
| `OAuthTokenRefreshFailed` | `auth.oauth.token_refresh_failed` | "token refresh failed" |
| `MissingAzureDeployment` | `config.azure.missing_deployment` | "resource+deployment"、"missing deployment" |
| `MissingRegion` | `config.auth.missing_region` | "missing region" |
| `MissingProject` | `config.auth.missing_project` | "missing project" |
| `MissingCredentialChain` | `auth.credential_chain.missing` | "credential chain"、"aws_access_key_id" |

### Anthropic

**认证机制**：通过 `x-api-key` 请求头发送 API 密钥（非 Bearer）
**环境变量**：`ANTHROPIC_API_KEY`
**OAuth**：内置（claude.ai 授权 → console.anthropic.com 令牌）

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少密钥 | N/A | 请求前校验 | `MissingApiKey` | — |
| 无效密钥 | 401 | `{"type":"error","error":{"type":"authentication_error","message":"..."}}` | `InvalidApiKey` | `verify_anthropic_error_auth_401.json` |
| 限流 | 429 | `{"type":"error","error":{"type":"rate_limit_error","message":"..."}}` | — | `verify_anthropic_error_rate_limit_429.json` |
| 错误请求 | 400 | `{"type":"error","error":{"type":"invalid_request_error","message":"..."}}` | — | `verify_anthropic_error_bad_request_400.json` |

**面向用户的消息**：`"Missing API key for Anthropic. Set ANTHROPIC_API_KEY or use 'pi auth'."`
**源码**：`src/providers/anthropic.rs:158-167`

### OpenAI

**认证机制**：通过 `Authorization` 请求头发送 Bearer 令牌
**环境变量**：`OPENAI_API_KEY`

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少密钥 | N/A | 请求前校验 | `MissingApiKey` | — |
| 无效密钥 | 401 | `{"error":{"code":"invalid_api_key","message":"...","param":null,"type":"invalid_request_error"}}` | `InvalidApiKey` | `verify_openai_error_auth_401.json` |
| 限流 | 429 | `{"error":{"code":"rate_limit_exceeded","message":"...","type":"requests"}}` | — | `verify_openai_error_rate_limit_429.json` |

**面向用户的消息**：`"Missing API key for OpenAI. Set OPENAI_API_KEY or configure in settings."`
**源码**：`src/providers/openai.rs:256-276`

### Gemini（Google Generative AI）

**认证机制**：API 密钥作为 URL 查询参数（`?key=<key>`）
**环境变量**：`GOOGLE_API_KEY`、`GEMINI_API_KEY`（备选）

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少密钥 | N/A | 请求前校验 | `MissingApiKey` | — |
| 无效密钥 | 401 | `{"error":{"code":401,"message":"API key not valid...","status":"UNAUTHENTICATED"}}` | `InvalidApiKey` | `verify_gemini_error_auth_401.json` |
| 限流 | 429 | `{"error":{"code":429,"message":"...","status":"RESOURCE_EXHAUSTED"}}` | — | `verify_gemini_error_rate_limit_429.json` |

**面向用户的消息**：`"Missing API key for Google/Gemini. Set GOOGLE_API_KEY or GEMINI_API_KEY."`
**特有说明**：API 密钥通过 URL 传递，而非请求头。
**源码**：`src/providers/gemini.rs:152-162`

### Cohere

**认证机制**：通过 `Authorization` 请求头发送 Bearer 令牌
**环境变量**：`COHERE_API_KEY`

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少密钥 | N/A | 请求前校验 | `MissingApiKey` | — |
| 无效密钥 | 401 | `{"message":"..."}` | `InvalidApiKey` | `verify_cohere_error_auth_401.json` |
| 限流 | 429 | `{"message":"..."}` | — | `verify_cohere_error_rate_limit_429.json` |

**面向用户的消息**：`"Missing API key for Cohere. Set COHERE_API_KEY or configure in settings."`
**源码**：`src/providers/cohere.rs:118-133`

### Azure OpenAI

**认证机制**：通过 `api-key` 请求头发送 API 密钥（非 `Authorization`）
**环境变量**：`AZURE_OPENAI_API_KEY`
**额外必需配置**：资源名称、部署名称、API 版本

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少密钥 | N/A | 请求前校验 | `MissingApiKey` | — |
| 无效密钥 | 401 | `{"error":{"code":"401","message":"Access denied due to invalid subscription key..."}}` | `InvalidApiKey` | `verify_azure_error_auth_401.json` |
| 缺少部署 | N/A | 请求前校验 | `MissingAzureDeployment` | — |
| 错误端点 | 401 | 与无效密钥相同（资源错误返回 401） | `InvalidApiKey` | — |

**面向用户的消息**：`"Missing API key for Azure OpenAI. Set AZURE_OPENAI_API_KEY or configure in settings."`
**特有说明**：使用 `api-key` 请求头，而非 `Authorization`。需配置 resource + deployment。
**源码**：`src/providers/azure.rs:167, 188-196`

### Amazon Bedrock

**认证机制**：显式 `--api-key`/按请求 bearer 覆盖优先；否则使用 `AWS_BEARER_TOKEN_BEDROCK` 或 AWS SigV4 签名。
**环境变量**：`AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY`（可选 `AWS_SESSION_TOKEN`）、`AWS_BEARER_TOKEN_BEDROCK`、`AWS_PROFILE` 或 `AWS_DEFAULT_PROFILE`，以及 `AWS_REGION` 或 `AWS_DEFAULT_REGION`

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少凭据 | N/A | 请求前校验 | `MissingCredentialChain` | — |
| 无效凭据 | 401 | `{"__type":"UnrecognizedClientException","message":"..."}` | `InvalidApiKey` | `verify_bedrock_error_auth_401.json` |
| 区域错误 | 403 | `{"__type":"AccessDeniedException","message":"..."}` | `MissingRegion` | — |

**面向用户的消息**：`"Amazon Bedrock requires AWS credentials. Set AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY, AWS_BEARER_TOKEN_BEDROCK, AWS_PROFILE (static or SSO), or store amazon-bedrock credentials in auth.json. For SSO profiles, run: aws sso login --profile <name>. An explicit direct bearer token may be passed via --api-key or StreamOptions.api_key."`
**特有说明**：非空的显式直连 bearer 覆盖优先于所有自动 AWS 来源。若无覆盖，解析器依次检查 Bedrock bearer 环境令牌、完整的环境 IAM 密钥、静态或 SSO 配置文件、再到已存储凭据。仅有区域、仅有配置文件名、仅有 access-key ID 或孤立的 session token 绝不会被视为 bearer 凭据。SigV4 签名在请求时进行。
**源码**：`src/providers/bedrock.rs`（`BedrockProvider::resolve_auth_context`，请求签名）与 `src/auth.rs`（`resolve_aws_credentials_async`）

### GitHub Copilot

**认证机制**：两步 OAuth（GitHub 令牌 → Copilot 会话令牌交换）
**环境变量**：`GITHUB_TOKEN`
**OAuth**：内置设备码流

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少令牌 | N/A | 请求前校验 | `MissingApiKey` | — |
| 令牌交换失败 | 401 | `{"message":"..."}` 于 `/copilot_internal/v2/token` | `OAuthTokenExchangeFailed` | `verify_copilot_error_auth_401.json` |
| 无 Copilot 权限 | 403 | 令牌交换期间 GitHub API 403 | `InvalidApiKey` | — |

**面向用户的消息**：`"Copilot token exchange failed (HTTP 401). Verify your GitHub token has Copilot access."`
**特有说明**：需具备 GitHub Copilot 权益。每次请求前都会发生令牌交换。
**源码**：`src/providers/copilot.rs:134-213`

### GitLab Duo

**认证机制**：PAT 或 OAuth Bearer 令牌
**环境变量**：`GITLAB_TOKEN`、`GITLAB_API_KEY`
**OAuth**：支持自托管（可配置 base URL）

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少令牌 | N/A | 请求前校验 | `MissingApiKey` | — |
| 无效令牌 | 401 | `{"message":"..."}` | `InvalidApiKey` | `verify_gitlab_error_auth_401.json` |
| 实例错误 | N/A | 连接错误（base URL 错误） | `MissingEndpoint` | — |

**面向用户的消息**：`"GitLab API token is required. Set GITLAB_TOKEN or GITLAB_API_KEY environment variable."`
**特有说明**：自托管需配置正确的 base URL。默认作用域：`api read_api read_user`。
**源码**：`src/providers/gitlab.rs:261-265, 292-296`

### Google Vertex AI

**认证机制**：Bearer 令牌，附带项目/区域解析
**环境变量**：`GOOGLE_CLOUD_API_KEY`、`VERTEX_API_KEY`、`GOOGLE_CLOUD_PROJECT`、`VERTEX_PROJECT`、`GOOGLE_CLOUD_LOCATION`、`VERTEX_LOCATION`

| 故障模式 | HTTP 状态 | 响应体形态 | 诊断码 | VCR 磁带 |
|-------------|------------|--------------------|-----------------|----|
| 缺少令牌 | N/A | 请求前校验 | `MissingApiKey` | — |
| 无效令牌 | 401 | `{"error":{"code":401,"message":"...","status":"UNAUTHENTICATED"}}` | `InvalidApiKey` | `verify_vertex_error_auth_401.json` |
| 缺少项目 | N/A | 请求前校验 | `MissingProject` | — |
| 缺少区域 | N/A | 请求前校验 | `MissingRegion` | — |

**面向用户的消息**：`"Missing Vertex AI API key / access token. Set GOOGLE_CLOUD_API_KEY or VERTEX_API_KEY."`
**特有说明**：除认证外还需 project + location。支持多条环境变量回退路径。
**源码**：`src/providers/vertex.rs:125-140, 256-267`

### 响应体形态参考

各提供方遵循不同的错误封包格式：

| 提供方族 | 错误封包 |
|----------------|---------------|
| Anthropic | `{"type":"error","error":{"type":"...","message":"..."}}` |
| OpenAI 兼容 | `{"error":{"code":"...","message":"...","param":null,"type":"..."}}` |
| Google（Gemini/Vertex） | `{"error":{"code":N,"message":"...","status":"..."}}` |
| AWS Bedrock | `{"__type":"ExceptionName","message":"..."}` |
| Simple（Cohere/Copilot/GitLab） | `{"message":"..."}` |
| Azure OpenAI | `{"error":{"code":"...","message":"..."}}`（类似 OpenAI，但 `code` 为字符串） |

### 认证凭据解析优先级

认证系统（`src/auth.rs`）按以下顺序解析凭据：

1. **显式覆盖** — `--api-key` 标志或单次请求密钥
2. **已存储的 OAuth 或 bearer 令牌** — `~/.pi/agent/auth.json` 中未过期的 OAuth 访问令牌或 `BearerToken` 条目
3. **环境变量** — 来自 `provider_auth_env_keys()` 的提供方专属变量，按文档顺序
4. **已存储的 API 密钥** — `~/.pi/agent/auth.json` 中的 `ApiKey` 条目
5. **外部编程 CLI 凭据** — 仅在使用 Pi 全局认证存储时，从另一本地编程 CLI 自动检测到的受支持凭据

规范 ID 与别名在各适用层级共享凭据查找；别名解析并非独立的、更低优先级的凭据源。若全部五个认证来源均缺失，模型选择仍可使用 `models.json` 内联 `apiKey` 回退。

**OAuth 主动刷新**：令牌在过期前 10 分钟刷新，以避免请求中途过期。

**凭据类型**（`src/auth.rs`）：
- `ApiKey` — 静态密钥
- `OAuth` — 含过期元数据的访问令牌 + 刷新令牌
- `AwsCredentials` — IAM 密钥 + 可选 session token + 区域
- `BearerToken` — 预认证的 bearer 令牌
- `ServiceKey` — 客户端凭据（client_id + client_secret → 令牌交换）

## 脱敏与安全诊断预期

本节定义哪些敏感数据绝不能出现在日志、转录或产物中、如何验证脱敏，以及运维人员如何安全地调试认证问题。

### 输出中绝不能出现的内容

以下数据类别被归为敏感信息，在所有可观测面（JSONL 日志、VCR 磁带、错误消息、终端输出）中必须脱敏：

| 数据类别 | 示例 | 脱敏占位符 |
|--------------|---------|----------------------|
| API 密钥 | `ANTHROPIC_API_KEY`、`OPENAI_API_KEY`、任意 `*_API_KEY` | `[REDACTED]` |
| Bearer 令牌 | OAuth 访问令牌、刷新令牌、会话令牌 | `[REDACTED]` |
| 密码 | 客户端密钥、数据库密码 | `[REDACTED]` |
| 私钥 | PEM 密钥、SSH 密钥 | `[REDACTED]` |
| 会话 cookie | 含认证上下文的 HTTP cookie | `[REDACTED]` |
| AWS 凭据 | `AWS_SECRET_ACCESS_KEY`、`AWS_SESSION_TOKEN` | `[REDACTED]` |

### 脱敏层

三层独立脱敏确保纵深防御：

**第 1 层：VCR 磁带脱敏**（`src/vcr.rs`）
- **请求头**：`authorization`、`x-api-key`、`api-key`、`x-goog-api-key`、`x-azure-api-key`、`proxy-authorization`
- **JSON 请求体字段**：键名包含 `api_key`、`apikey`、`authorization`、`token`（单数，非 `tokens`）、`access_tokens`、`refresh_tokens`、`id_tokens`、`secret`、`password` 的任意字段
- **生效时机**：在磁带录制时自动执行（`redact_cassette()`）及回放时的请求体对比阶段（`redact_json()`）
- **占位符**：`"[REDACTED_BY_VCR]"`

**第 2 层：JSONL 日志上下文脱敏**（`tests/common/logging.rs`）
- **上下文映射键**：`api_key`、`api-key`、`authorization`、`bearer`、`cookie`、`credential`、`password`、`private_key`、`secret`、`token`
- **生效时机**：调用 `TestLogger.info_ctx()` 或类似方法时自动执行
- **匹配方式**：大小写不敏感的子串匹配（例如 `MY_API_KEY_HEADER` 会匹配 `api_key`）
- **占位符**：`"[REDACTED]"`

**第 3 层：在线端到端请求头脱敏**（`tests/common/harness.rs`）
- **请求头键**：与第 2 层相同的 10 个片段
- **生效时机**：`redact_sensitive_header_pairs()` 对所有在线 HTTP 请求头在记录前脱敏
- **占位符**：`"[REDACTED]"`

**第 4 层：错误诊断脱敏**（`src/error.rs`）
- **策略**：所有 `AuthDiagnosticCode` 变体均返回 `redaction_policy: "redact-secrets"`
- **生效**：下游消费者（错误展示、遥测）必须遵守该策略
- **效果**：错误消息包含诊断码与修复文案，但绝不包含原始凭据

### 验证方法

**自动化测试：`find_unredacted_keys()`**（`tests/common/logging.rs`）

该函数递归扫描任意 JSON 值，返回未使用脱敏占位符的敏感键路径。用作断言：

```rust
let leaks = find_unredacted_keys(&json_artifact);
assert!(leaks.is_empty(), "Unredacted sensitive data found: {leaks:?}");
```

**VCR 磁带脱敏测试**（`src/vcr.rs` 测试模块）：
- `redact_json_flat_object` — 验证扁平对象中的 `api_key` 已脱敏
- `redact_json_nested` — 验证嵌套 JSON 请求体被递归脱敏
- `oauth_refresh_invalid_matches_after_redaction` — 验证真实 OAuth 磁带在脱敏后仍能匹配
- `sensitive_key_token_but_not_tokens` — 验证 `max_tokens`（计数）不会被脱敏，而 `access_token`（认证）会被脱敏

**JSONL 脱敏测试**（`tests/common/logging.rs` 测试模块）：
- `test_redaction` — 验证 `Authorization` 请求头值被替换为 `[REDACTED]`
- `redaction_case_insensitive_key_matching` — 验证大小写不敏感匹配
- `redaction_partial_key_match` — 验证子串匹配（例如 `x-api-key-header`）
- `redact_json_value_all_sensitive_key_patterns` — 验证全部 10 种键模式

### 安全调试工作流

发生认证故障时，运维人员可通过以下方式安全调试：

1. **错误诊断码**：`AuthDiagnosticCode` wire 字符串（例如 `auth.missing_api_key`）可在不暴露凭据的情况下标识故障类别
2. **修复文案**：每个诊断码都有静态修复字符串（例如 "Set the provider API key env var or run `/login <provider>`."），指引解决方案
3. **VCR 磁带回放**：已录制的磁带所有敏感字段已预先脱敏。通过 `VCR_MODE=playback` 回放可复现精确的故障路径，无需在线凭据
4. **JSONL 日志检查**：测试日志包含结构化类别（`setup`、`action`、`verify`、`error`），其上下文映射已自动脱敏，可安全地与协作者共享
5. **提供方环境变量检查**：`echo $PROVIDER_API_KEY | wc -c` 可在不泄露值的情况下确认密钥已设置

### 认证故障的 CI 产物保留与回放

当认证回归在 CI 中失败时，请基于保留产物进行调试，而非盲目重跑。

必读引用：
- 产物契约：`docs/provider_e2e_artifact_contract.json`
- 回放手册：`docs/ci-operator-runbook.md`

最小回放路径：

```bash
# 复现在先前 CI 运行中捕获的所有失败
./scripts/e2e/run_all.sh --rerun-from tests/e2e_results/<timestamp>/summary.json

# 直接回放保留与回放契约检查
cargo test --test ci_artifact_retention -- --nocapture
cargo test --test e2e_replay_bundles -- --nocapture
cargo test --test e2e_replay_bundle_validation -- --nocapture
```

认证专项分流序列：
1. 打开 `tests/e2e_results/<timestamp>/replay_bundle.json` 并运行 `one_command_replay`。
2. 对每个失败的提供方套件，运行 `failed_suites[].targeted_replay`。
3. 确认 `failure_digest.json` 包含认证类根因及回放指针。
4. 通过以下检查验证保留日志（`test-log.jsonl`、`artifact-index.jsonl`）中的脱敏仍完好：
   - `tests/e2e_artifact_retention_triage.rs::log_redacts_api_keys`
   - `tests/e2e_artifact_retention_triage.rs::log_redacts_authorization_headers`

### 反模式（切勿为之）

| 反模式 | 为何危险 | 安全替代方案 |
|-------------|-------------------|-----------------|
| 记录完整 HTTP 请求体 | 可能包含请求体字段中的 API 密钥 | 记录前使用 `redact_json()` 脱敏 |
| 在错误消息中包含 `Authorization` 请求头 | 暴露 bearer 令牌 | 仅记录状态码 + 诊断码 |
| 在修复文案中回显 API 密钥 | 将密钥打印到终端 | 仅打印环境变量*名称*，而非值 |
| 存储未脱敏的原始磁带 | 磁带文件可能被提交到 git | 始终通过自动脱敏的 `VcrRecorder` 录制 |
| 为调试而禁用脱敏 | 密钥可能残留在日志文件中 | 使用 `find_unredacted_keys()` 断言验证 |

## 相关产物

- 提供方元数据：`src/provider_metadata.rs`
- 错误提示系统：`src/error.rs::provider_hints()`
- 认证诊断码：`src/error.rs::AuthDiagnosticCode`
- 认证解析：`src/auth.rs`
- 契约测试：`tests/provider_native_contract.rs::failure_taxonomy`
- 一致性磁带：`tests/fixtures/vcr/verify_*_error_auth_401.json`
- 提供方 gap 测试矩阵：`docs/provider-gaps-test-matrix.json`
- 长尾证据：`docs/provider-longtail-evidence.md`
