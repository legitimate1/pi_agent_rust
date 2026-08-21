# 提供方配置示例

**Bead:** bd-3uqg.9.2
**Updated:** 2026-02-13

开箱即用的各提供方家族配置示例。
每节均展示最小化配置、高级选项、别名及已知陷阱。

---

## 内置原生提供方

这些提供方拥有专用的 Rust 实现，完整支持流式、工具调用和推理能力。

### Anthropic

```bash
export ANTHROPIC_API_KEY="sk-ant-..."

pi --provider anthropic --model claude-sonnet-4-5
```

**端点(Endpoint)**: `https://api.anthropic.com/v1/messages`
**认证(Auth)**: 通过 `ANTHROPIC_API_KEY` 以 `x-api-key` 请求头认证
**API 家族(API family)**: `anthropic-messages`
**模型(Models)**: claude-opus-4-5, claude-sonnet-4-5, claude-haiku-4-5, claude-sonnet-4-20250514
**支持(Supports)**: 文本 + 图像输入、推理(thinking)、工具调用、流式

**高级(Advanced)**: 自定义基础 URL（例如企业代理）：
```bash
pi --provider anthropic --model claude-sonnet-4-5 --base-url "https://proxy.corp.example.com/anthropic"
```

### OpenAI

```bash
export OPENAI_API_KEY="sk-..."

pi --provider openai --model gpt-4o
```

**端点(Endpoint)**: `https://api.openai.com/v1`
**认证(Auth)**: 通过 `OPENAI_API_KEY` 的 Bearer Token
**API 家族(API family)**: `openai-responses`（原生）、`openai-completions`（兼容）
**模型(Models)**: gpt-5.1-codex, gpt-4o, gpt-4o-mini
**支持(Supports)**: 文本 + 图像输入、推理、工具调用、流式

### Google Gemini

```bash
# 任意环境变量均可
export GOOGLE_API_KEY="AIza..."
# 或
export GEMINI_API_KEY="AIza..."

pi --provider google --model gemini-2.5-pro
# 或使用别名
pi --provider gemini --model gemini-2.5-flash
```

**端点(Endpoint)**: `https://generativelanguage.googleapis.com/v1beta`
**认证(Auth)**: 通过 `GOOGLE_API_KEY`（首选）或 `GEMINI_API_KEY`（备用）的 API 密钥
**API 家族(API family)**: `google-generative-ai`
**模型(Models)**: gemini-2.5-pro, gemini-2.5-flash, gemini-1.5-pro, gemini-1.5-flash
**别名(Aliases)**: `google`, `gemini`
**支持(Supports)**: 文本 + 图像输入、推理、工具调用、流式

### Google Vertex AI

```bash
# 任意环境变量均可
export GOOGLE_CLOUD_API_KEY="..."
# 或
export VERTEX_API_KEY="..."

pi --provider google-vertex --model gemini-2.5-pro
# 或使用别名
pi --provider vertexai --model gemini-2.5-pro
```

**端点(Endpoint)**: 基于区域（例如 `https://us-central1-aiplatform.googleapis.com/...`）
**认证(Auth)**: 通过 `GOOGLE_CLOUD_API_KEY`（首选）或 `VERTEX_API_KEY`（备用）的 API 密钥
**API 家族(API family)**: `google-vertex`
**别名(Aliases)**: `google-vertex`, `vertexai`
**上下文窗口(Context window)**: 最长 1,000,000 tokens

**注意事项(Caveat)**: 基础 URL 由区域和项目动态构建。如需覆盖请使用 `--base-url`：
```bash
pi --provider google-vertex --model gemini-2.5-pro \
  --base-url "https://europe-west4-aiplatform.googleapis.com/v1/projects/my-project/locations/europe-west4/publishers/google/models"
```

### Cohere

```bash
export COHERE_API_KEY="..."

pi --provider cohere --model command-r-plus
```

**端点(Endpoint)**: `https://api.cohere.com/v2`
**认证(Auth)**: 通过 `COHERE_API_KEY` 的 Bearer Token
**API 家族(API family)**: `cohere-chat`
**支持(Supports)**: 仅文本输入（不支持图像）、推理、工具调用、流式

---

## 原生适配器提供方

这些提供方需要超出通用 OpenAI 兼容层之外的专用协议或认证处理。

### Amazon Bedrock

```bash
# AWS 凭证（IAM 或 SSO）
export AWS_ACCESS_KEY_ID="AKIA..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_SESSION_TOKEN="..."       # 如使用临时凭证
export AWS_REGION="us-east-1"

# 或使用 bearer token
export AWS_BEARER_TOKEN_BEDROCK="..."

pi --provider amazon-bedrock --model anthropic.claude-sonnet-4-20250514-v1:0
# 或使用别名
pi --provider bedrock --model anthropic.claude-sonnet-4-20250514-v1:0
```

**端点(Endpoint)**: AWS 区域端点（由 `AWS_REGION` 构造）
**认证(Auth)**: 通过 `AWS_ACCESS_KEY_ID`+`AWS_SECRET_ACCESS_KEY` 的 AWS SigV4 或 bearer token
**API 家族(API family)**: `bedrock-converse-stream`
**别名(Aliases)**: `amazon-bedrock`, `bedrock`

**注意事项(Caveats)**:
- 无单一基础 URL；端点取决于区域
- 模型 ID 使用 Bedrock 格式（例如 `anthropic.claude-sonnet-4-20250514-v1:0`）
- 同时支持通过 `AWS_PROFILE` 进行命名配置文件认证
- 仅文本输入（不支持图像透传）

### Azure OpenAI

```bash
export AZURE_OPENAI_API_KEY="..."

pi --provider azure-openai --model gpt-4o
# 或使用别名
pi --provider azure --model gpt-4o
```

**认证(Auth)**: 通过 `AZURE_OPENAI_API_KEY` 的 API 密钥
**别名(Aliases)**: `azure-openai`, `azure`, `azure-cognitive-services`

**注意事项(Caveats)**:
- 需要针对部署的端点配置
- 端点格式：`https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`
- 模型 ID 映射到部署名称，而非 OpenAI 模型 ID
- 通过 `models.json` 或 `--base-url` 配置：
```bash
pi --provider azure --model my-gpt4o-deployment \
  --base-url "https://my-resource.openai.azure.com/openai/deployments/my-gpt4o-deployment/chat/completions?api-version=2024-02-15-preview"
```

### SAP AI Core

```bash
# 使用 service key
export AICORE_SERVICE_KEY='{"clientid":"...","clientsecret":"...","url":"...","serviceurls":{"AI_API_URL":"..."}}'

# 或单独凭证
export SAP_AI_CORE_CLIENT_ID="..."
export SAP_AI_CORE_CLIENT_SECRET="..."
export SAP_AI_CORE_TOKEN_URL="https://..."
export SAP_AI_CORE_SERVICE_URL="https://..."

pi --provider sap-ai-core --model gpt-4o
# 或使用别名
pi --provider sap --model gpt-4o
```

**认证(Auth)**: 通过 service key 或独立环境变量的 OAuth2 客户端凭证
**别名(Aliases)**: `sap-ai-core`, `sap`

**注意事项(Caveats)**:
- 需要订阅了 AI Core 服务的 SAP BTP
- 使用客户端凭证自动完成 Token 交换
- 模型可用性取决于你的 SAP AI Core 资源组配置

### GitHub Copilot

```bash
export GITHUB_COPILOT_API_KEY="..."
# 或
export GITHUB_TOKEN="ghp_..."

pi --provider github-copilot --model gpt-4o
# 或使用别名
pi --provider copilot --model gpt-4o
```

**认证(Auth)**: 通过 `GITHUB_COPILOT_API_KEY` 或 `GITHUB_TOKEN` 的 Token
**别名(Aliases)**: `github-copilot`, `copilot`, `github-copilot-enterprise`

**注意事项(Caveats)**:
- 需要有效的 GitHub Copilot 订阅
- 每次会话前会与 GitHub API 进行 Token 交换
- 企业版有独立的 Token 处理逻辑

### GitLab Duo

```bash
export GITLAB_TOKEN="glpat-..."
# 或
export GITLAB_API_KEY="..."

pi --provider gitlab --model claude-sonnet-4
# 或使用别名
pi --provider gitlab-duo --model claude-sonnet-4
```

**认证(Auth)**: 通过 `GITLAB_TOKEN`（首选）或 `GITLAB_API_KEY`（备用）的 Token
**别名(Aliases)**: `gitlab`, `gitlab-duo`

**注意事项(Caveats)**:
- 端点为你的 GitLab 实例 URL（通过 `--base-url` 配置）
- 返回非流式的 done 事件（流式行为可能不同）
- 模型可用性取决于你的 GitLab 订阅级别

---

## OpenAI 兼容预设提供方（旗舰）

以下均使用 `openai-completions` API 家族，并通过 OpenAI 兼容适配器路由。设置对应提供方的 API 密钥即可使用。

### Groq

```bash
export GROQ_API_KEY="gsk_..."

pi --provider groq --model llama-3.3-70b-versatile
```

**端点(Endpoint)**: `https://api.groq.com/openai/v1/chat/completions`
**模型(Models)**: llama-3.3-70b-versatile, llama-3.1-8b-instant, mixtral-8x7b-32768

**注意事项(Caveats)**:
- 温度 0 会在服务端被归一化为 1e-8
- `logprobs`、`logit_bias`、`messages[].name` 会被静默忽略

### DeepSeek

```bash
export DEEPSEEK_API_KEY="sk-..."

pi --provider deepseek --model deepseek-chat
```

**端点(Endpoint)**: `https://api.deepseek.com`
**模型(Models)**: deepseek-chat, deepseek-coder, deepseek-reasoner
**上下文窗口(Context window)**: 128,000 tokens

### Cerebras

```bash
export CEREBRAS_API_KEY="csk-..."

pi --provider cerebras --model llama-3.3-70b
```

**端点(Endpoint)**: `https://api.cerebras.ai/v1/chat/completions`
**模型(Models)**: llama-3.3-70b, llama-3.1-8b, qwen-3-32b

**注意事项(Caveats)**:
- 仅 `gpt-oss-120b`、`qwen-3-32b`、`zai-glm-4.7` 支持工具调用
- 非标准的速率限制响应头（按天和按分钟）

### OpenRouter

```bash
export OPENROUTER_API_KEY="sk-or-..."

pi --provider openrouter --model openai/gpt-4o-mini
```

**端点(Endpoint)**: `https://openrouter.ai/api/v1/chat/completions`

**高级(Advanced)**: 通过 `provider/model` 格式访问任意模型：
```bash
pi --provider openrouter --model anthropic/claude-sonnet-4
pi --provider openrouter --model meta-llama/llama-3.3-70b-instruct
```

**注意事项(Caveats)**:
- 模型 ID 使用 `org/model` 格式
- 流中错误使用 HTTP 200 + SSE 错误载荷（非标准错误码）
- 实际服务的模型可能与请求的模型不同（回退路由）

### Mistral

```bash
export MISTRAL_API_KEY="..."

pi --provider mistral --model mistral-large-latest
```

**端点(Endpoint)**: `https://api.mistral.ai/v1/chat/completions`
**模型(Models)**: mistral-large-latest, mistral-medium-latest, open-mistral-7b

### Moonshot AI (Kimi)

```bash
# 任意环境变量均可
export MOONSHOT_API_KEY="sk-..."
# 或
export KIMI_API_KEY="sk-..."

# 全球端点
pi --provider moonshotai --model moonshot-v1-128k
# 中国端点
pi --provider moonshotai-cn --model moonshot-v1-128k
# 面向编程（使用 Anthropic API）
pi --provider kimi-for-coding --model kimi-k2.5
```

**端点(Endpoint)**: `https://api.moonshot.ai/v1/chat/completions`（全球）
**别名(Aliases)**: `moonshotai`, `moonshot`, `kimi`

**注意事项(Caveats)**:
- 三个独立条目：`moonshotai`（.ai 全球）、`moonshotai-cn`（.cn 中国）、`kimi-for-coding`（Anthropic API）
- 密钥在 `.ai` 和 `.cn` 端点之间不可互换
- `kimi-for-coding` 使用 `anthropic-messages` API，而非 `openai-completions`
- 温度范围 0-1（而非像 OpenAI 的 0-2）

### Alibaba (Qwen / DashScope)

```bash
# 任意环境变量均可
export DASHSCOPE_API_KEY="sk-..."
# 或
export QWEN_API_KEY="sk-..."

pi --provider alibaba --model qwen-plus
# 或使用别名
pi --provider qwen --model qwen-turbo
```

**端点(Endpoint)**: `https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions`
**别名(Aliases)**: `alibaba`, `dashscope`, `qwen`
**中国区域(China region)**: 使用 `alibaba-cn` 提供方 ID

**注意事项(Caveats)**:
- 工具调用不能与流式同时使用
- 两种不同的 429 分类：`qps`（可重试） vs `quota`（不可重试）
- `QWEN_API_KEY` 回退仅适用于 `alibaba`（国际版），不适用于 `alibaba-cn`

### Fireworks AI

```bash
export FIREWORKS_API_KEY="..."

pi --provider fireworks --model accounts/fireworks/models/llama-v3p1-70b-instruct
# 或使用别名
pi --provider fireworks-ai --model accounts/fireworks/models/llama-v3p1-70b-instruct
```

**端点(Endpoint)**: `https://api.fireworks.ai/inference/v1`
**别名(Aliases)**: `fireworks`, `fireworks-ai`

### Perplexity

```bash
export PERPLEXITY_API_KEY="pplx-..."

pi --provider perplexity --model sonar-pro
```

**端点(Endpoint)**: `https://api.perplexity.ai`
**模型(Models)**: sonar-pro, sonar, sonar-reasoning

### xAI (Grok)

```bash
export XAI_API_KEY="xai-..."

pi --provider xai --model grok-2
```

**端点(Endpoint)**: `https://api.x.ai/v1`
**模型(Models)**: grok-2, grok-2-mini

### Together AI

```bash
export TOGETHER_API_KEY="..."

pi --provider togetherai --model meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo
```

**端点(Endpoint)**: `https://api.together.xyz/v1/chat/completions`

### DeepInfra

```bash
export DEEPINFRA_API_KEY="..."

pi --provider deepinfra --model meta-llama/Meta-Llama-3.1-70B-Instruct
```

**端点(Endpoint)**: `https://api.deepinfra.com/v1/openai/chat/completions`

---

## 区域与专用预设

### NVIDIA

```bash
export NVIDIA_API_KEY="nvapi-..."

pi --provider nvidia --model meta/llama-3.1-70b-instruct
```

**端点(Endpoint)**: `https://integrate.api.nvidia.com/v1/chat/completions`

### Hugging Face

```bash
export HF_TOKEN="hf_..."

pi --provider huggingface --model meta-llama/Meta-Llama-3.1-70B-Instruct
```

**端点(Endpoint)**: `https://router.huggingface.co/v1/chat/completions`

### STACKIT (EU)

```bash
export STACKIT_API_KEY="..."

pi --provider stackit --model <model-id>
```

**端点(Endpoint)**: `https://api.openai-compat.model-serving.eu01.onstackit.cloud/v1/chat/completions`
**备注(Note)**: 欧盟托管，符合数据驻留合规要求。

### Ollama Cloud

```bash
export OLLAMA_API_KEY="..."

pi --provider ollama-cloud --model llama3.1:70b
```

**端点(Endpoint)**: `https://ollama.com/v1/chat/completions`

---

## 验证

配置任意提供方后，验证其是否可用：

```bash
# 快速冒烟测试
pi --provider <provider-id> --model <model-id> -m "Hello, respond with just OK"

# 预期：返回包含 "OK" 或类似确认的响应
```

常见认证问题及修复方法：[provider-auth-troubleshooting.md](provider-auth-troubleshooting.md)

---

## 相关文档

- 认证排错：[provider-auth-troubleshooting.md](provider-auth-troubleshooting.md)
- 长尾证据：[provider-longtail-evidence.md](provider-longtail-evidence.md)
- 接入 playbook：[provider-onboarding-playbook.md](provider-onboarding-playbook.md)
- 提供方元数据源：`src/provider_metadata.rs`
