# 模型配置

Pi 从内置注册表与可选的用户定义 `models.json` 加载可用模型。

## 位置

| 路径 | 描述 |
|------|-------------|
| `~/.pi/agent/models.json` | 用户定义的模型覆盖与自定义提供方 |
| `~/.pi/agent/models.fetched.json` | 生成的 v2 实时目录成员；仅由 `--persist-models` 管理 |

不要手动编辑 `models.fetched.json`。其提供方/模型 ID 通过非敏感指纹与时间戳绑定到抓取端点与传输形态。该指纹排除凭证值、URL 查询值与标头值，因此不能作为离线凭证验证器。它绑定已识别的凭证查询有序名称/存在形态（包括精确的查询名称大小写）与大小写不敏感的标头名称/存在形态。当前路由下识别的凭证通道之外的非空查询/标头值可能选择租户或部署；因此 Pi 会拒绝为该路由持久化，并忽略此类当前路由下的旧版生成成员。Pi 会忽略不匹配的生成行并要求你刷新它们。由于指纹有意排除凭证值，仅切换账户而不改变端点/传输形态不会自动使已保存的成员失效；在切换后请重新运行 `--fetch-models <provider> --refresh-models --persist-models`。推理请求仍会解析当前账户的凭证，只有可选保存的模型列表可能保持陈旧。手写的 `models.json` 保持权威性。

旧版 `pi.models.fetched.v1` 文件缺乏 v2 所需的端点与传输溯源，会被保留而不会自动覆盖。请将旧版文件移至 `models.fetched.v1.backup.json`，然后运行经验证的实时 `--fetch-models <provider> --refresh-models --persist-models` 命令以创建 v2 目录。

## 模式

根对象包含 `providers` 映射。

```json
{
  "providers": {
    "openai": { ... },
    "anthropic": { ... },
    "ollama": { ... }
  }
}
```

### 提供方配置

| 字段 | 类型 | 描述 |
|-------|------|-------------|
| `baseUrl` | string | 基础 API URL（例如 `https://api.openai.com/v1`） |
| `api` | string | 协议适配器（例如 `openai-completions`、`openai-responses`、`anthropic-messages`、`google-generative-ai`、`google-vertex`） |
| `apiKey` | string | 回退 API 密钥、环境变量名或在常规运行时凭证解析之后的 shell 命令（见敏感信息解析） |
| `models` | object[] | 模型列表。若省略，则该提供方的提供方设置会覆盖该提供方的内置配置。 |
| `headers` | object | 自定义 HTTP 标头 |
| `authHeader` | boolean | 若为 true，则在 `Authorization: Bearer <key>` 中发送密钥 |
| `compat` | object | 兼容性标志 |

若提供了 `models`，则该提供方的内置模型将被 `models.json` 中的列表替换。

### 模型配置

| 字段 | 类型 | 描述 |
|-------|------|-------------|
| `id` | string | 发送至 API 的模型 ID |
| `name` | string | 显示名称 |
| `contextWindow` | number | 上下文窗口大小（以 token 计） |
| `maxTokens` | number | 最大输出 token 数 |
| `reasoning` | boolean | 若模型支持扩展思考则为真 |
| `input` | string[] | `["text", "image"]` |
| `cost` | object | 每百万 token 的成本 |

### 兼容性标志（`compat`）

| 字段 | 描述 |
|-------|-------------|
| `supportsStore` | 启用 OpenAI `store` 参数（在支持处） |
| `supportsDeveloperRole` | 使用 `developer` 角色而非 `system`（OpenAI o1/o3） |
| `supportsReasoningEffort` | 发送 `reasoning_effort` 参数（OpenAI） |
| `supportsUsageInStreaming` | 期望在流式响应中包含 usage 字段 |
| `maxTokensField` | 覆盖参数名（例如 `max_completion_tokens`） |
| `openRouterRouting` | OpenRouter 路由元数据（JSON 对象） |
| `vercelGatewayRouting` | Vercel 网关路由元数据（JSON 对象） |

## 内置提供方注册表

内置提供方、别名、认证环境键与引导模式的规范列表，由 `src/provider_metadata.rs` 中的 `PROVIDER_METADATA` 生成。请勿手动编辑表格——使用以下命令重新生成：`PI_BLESS_MODELS_DOC=1 cargo test --test provider_metadata_comprehensive docs_models_provider_table_matches_registry`。

<!-- PROVIDER_TABLE:BEGIN -->

| Provider ID | Name | Aliases | Auth env keys | Onboarding |
|---|---|---|---|---|
| `anthropic` | Anthropic | - | `ANTHROPIC_API_KEY` | native |
| `openai` | OpenAI | - | `OPENAI_API_KEY` | native |
| `openai-codex` | OpenAI Codex (ChatGPT) | codex, chatgpt-codex | - | native adapter |
| `google` | Google Gemini | gemini | `GOOGLE_API_KEY`, `GEMINI_API_KEY` | native |
| `google-gemini-cli` | Google Cloud Code Assist | gemini-cli | - | native adapter |
| `google-antigravity` | Google Antigravity | antigravity | - | native adapter |
| `cohere` | Cohere | - | `COHERE_API_KEY` | native |
| `cursor` | Cursor | cursor-agent | `CURSOR_API_KEY`, `CURSOR_ACCESS_TOKEN` | native |
| `groq` | Groq | - | `GROQ_API_KEY` | openai-compatible preset |
| `deepinfra` | Deep Infra | deep-infra | `DEEPINFRA_API_KEY` | openai-compatible preset |
| `cerebras` | Cerebras | - | `CEREBRAS_API_KEY` | openai-compatible preset |
| `atlascloud` | Atlas Cloud | atlas-cloud, atlas | `ATLASCLOUD_API_KEY`, `ATLAS_CLOUD_API_KEY` | openai-compatible preset |
| `openrouter` | OpenRouter | open-router | `OPENROUTER_API_KEY` | openai-compatible preset |
| `mistral` | Mistral AI | mistralai | `MISTRAL_API_KEY` | openai-compatible preset |
| `moonshotai` | Moonshot AI | moonshot, kimi | `MOONSHOT_API_KEY`, `KIMI_API_KEY` | openai-compatible preset |
| `alibaba` | Alibaba (Qwen) | dashscope, qwen | `DASHSCOPE_API_KEY`, `QWEN_API_KEY` | openai-compatible preset |
| `deepseek` | DeepSeek | deep-seek | `DEEPSEEK_API_KEY` | openai-compatible preset |
| `fireworks` | Fireworks AI | fireworks-ai | `FIREWORKS_API_KEY` | openai-compatible preset |
| `togetherai` | Together AI | together, together-ai | `TOGETHER_API_KEY`, `TOGETHER_AI_API_KEY` | openai-compatible preset |
| `perplexity` | Perplexity | pplx | `PERPLEXITY_API_KEY` | openai-compatible preset |
| `xai` | xAI (Grok) | grok, x-ai | `XAI_API_KEY` | openai-compatible preset |
| `302ai` | 302.AI | - | `302AI_API_KEY` | openai-compatible preset |
| `abacus` | Abacus AI | - | `ABACUS_API_KEY` | openai-compatible preset |
| `aihubmix` | AIHubMix | - | `AIHUBMIX_API_KEY` | openai-compatible preset |
| `bailing` | Bailing | - | `BAILING_API_TOKEN` | openai-compatible preset |
| `berget` | Berget | - | `BERGET_API_KEY` | openai-compatible preset |
| `chutes` | Chutes | - | `CHUTES_API_KEY` | openai-compatible preset |
| `cortecs` | Cortecs | - | `CORTECS_API_KEY` | openai-compatible preset |
| `fastrouter` | FastRouter | - | `FASTROUTER_API_KEY` | openai-compatible preset |
| `firmware` | Firmware | - | `FIRMWARE_API_KEY` | openai-compatible preset |
| `friendli` | Friendli | - | `FRIENDLI_TOKEN` | openai-compatible preset |
| `helicone` | Helicone | - | `HELICONE_API_KEY` | openai-compatible preset |
| `huggingface` | Hugging Face | hf, hugging-face | `HF_TOKEN` | openai-compatible preset |
| `iflowcn` | iFlow | - | `IFLOW_API_KEY` | openai-compatible preset |
| `inception` | Inception | - | `INCEPTION_API_KEY` | openai-compatible preset |
| `inference` | Inference | - | `INFERENCE_API_KEY` | openai-compatible preset |
| `io-net` | io.net | - | `IOINTELLIGENCE_API_KEY` | openai-compatible preset |
| `jiekou` | Jiekou | - | `JIEKOU_API_KEY` | openai-compatible preset |
| `lucidquery` | LucidQuery | - | `LUCIDQUERY_API_KEY` | openai-compatible preset |
| `moark` | Moark | - | `MOARK_API_KEY` | openai-compatible preset |
| `morph` | Morph | - | `MORPH_API_KEY` | openai-compatible preset |
| `nano-gpt` | NanoGPT | nanogpt | `NANO_GPT_API_KEY` | openai-compatible preset |
| `nova` | Nova | - | `NOVA_API_KEY` | openai-compatible preset |
| `novita-ai` | Novita AI | novita | `NOVITA_API_KEY` | openai-compatible preset |
| `nvidia` | NVIDIA NIM | nim, nvidia-nim | `NVIDIA_API_KEY` | openai-compatible preset |
| `poe` | Poe | - | `POE_API_KEY` | openai-compatible preset |
| `privatemode-ai` | PrivateMode AI | - | `PRIVATEMODE_API_KEY` | openai-compatible preset |
| `requesty` | Requesty | - | `REQUESTY_API_KEY` | openai-compatible preset |
| `submodel` | Submodel | - | `SUBMODEL_INSTAGEN_ACCESS_KEY` | openai-compatible preset |
| `synthetic` | Synthetic | - | `SYNTHETIC_API_KEY` | openai-compatible preset |
| `vivgrid` | Vivgrid | - | `VIVGRID_API_KEY` | openai-compatible preset |
| `vultr` | Vultr | - | `VULTR_API_KEY` | openai-compatible preset |
| `wandb` | Weights & Biases | - | `WANDB_API_KEY` | openai-compatible preset |
| `xiaomi` | Xiaomi | - | `XIAOMI_API_KEY` | openai-compatible preset |
| `alibaba-cn` | Alibaba China | - | `DASHSCOPE_API_KEY` | openai-compatible preset |
| `alibaba-us` | Alibaba US | - | `DASHSCOPE_API_KEY`, `QWEN_API_KEY` | openai-compatible preset |
| `kimi-for-coding` | Kimi for Coding | kimi-coding, kimi-code | `KIMI_API_KEY` | openai-compatible preset |
| `minimax` | MiniMax | - | `MINIMAX_API_KEY` | openai-compatible preset |
| `minimax-cn` | MiniMax China | - | `MINIMAX_CN_API_KEY` | openai-compatible preset |
| `minimax-coding-plan` | MiniMax Coding Plan | - | `MINIMAX_API_KEY` | openai-compatible preset |
| `minimax-cn-coding-plan` | MiniMax China Coding Plan | - | `MINIMAX_CN_API_KEY` | openai-compatible preset |
| `modelscope` | ModelScope | - | `MODELSCOPE_API_KEY` | openai-compatible preset |
| `moonshotai-cn` | Moonshot AI China | - | `MOONSHOT_API_KEY` | openai-compatible preset |
| `nebius` | Nebius | - | `NEBIUS_API_KEY` | openai-compatible preset |
| `ovhcloud` | OVHcloud | - | `OVHCLOUD_API_KEY` | openai-compatible preset |
| `scaleway` | Scaleway | - | `SCALEWAY_API_KEY` | openai-compatible preset |
| `stackit` | STACKIT | - | `STACKIT_API_KEY` | openai-compatible preset |
| `siliconflow` | SiliconFlow | silicon-flow | `SILICONFLOW_API_KEY` | openai-compatible preset |
| `siliconflow-cn` | SiliconFlow China | - | `SILICONFLOW_CN_API_KEY` | openai-compatible preset |
| `upstage` | Upstage | - | `UPSTAGE_API_KEY` | openai-compatible preset |
| `venice` | Venice AI | - | `VENICE_API_KEY` | openai-compatible preset |
| `zai` | Zai | - | `ZHIPU_API_KEY` | openai-compatible preset |
| `zai-coding-plan` | Zai Coding Plan | - | `ZHIPU_API_KEY` | openai-compatible preset |
| `zhipuai` | Zhipu AI | zhipu, glm | `ZHIPU_API_KEY` | openai-compatible preset |
| `zhipuai-coding-plan` | Zhipu AI Coding Plan | - | `ZHIPU_API_KEY` | openai-compatible preset |
| `baseten` | Baseten | - | `BASETEN_API_KEY` | openai-compatible preset |
| `llama` | Meta Llama | - | `LLAMA_API_KEY` | openai-compatible preset |
| `lmstudio` | LM Studio | lm-studio | `LMSTUDIO_API_KEY` | openai-compatible preset |
| `ollama` | Ollama | - | - | openai-compatible preset |
| `llamacpp` | llama.cpp | llama-cpp, llama.cpp, llama-server | - | openai-compatible preset |
| `mistralrs` | mistral.rs | mistral.rs, mistral-rs | - | openai-compatible preset |
| `ollama-cloud` | Ollama Cloud | - | `OLLAMA_API_KEY` | openai-compatible preset |
| `opencode` | OpenCode | opencode-zen | `OPENCODE_API_KEY` | openai-compatible preset |
| `vercel` | Vercel AI | vercel-ai-gateway | `AI_GATEWAY_API_KEY` | openai-compatible preset |
| `zenmux` | ZenMux | - | `ZENMUX_API_KEY` | openai-compatible preset |
| `gmi` | GMI Cloud | gmi-cloud, gmi-serving | `GMI_API_KEY` | openai-compatible preset |
| `coreweave` | CoreWeave Serverless Inference | coreweave-serverless | `COREWEAVE_API_KEY`, `WANDB_API_KEY` | openai-compatible preset |
| `sakana` | Sakana AI | sakana-ai | `SAKANA_API_KEY`, `FUGU_API_KEY` | openai-compatible preset |
| `wafer` | Wafer Serverless | wafer-serverless | `WAFER_SERVERLESS_API_KEY` | openai-compatible preset |
| `qianfan` | Qianfan (Baidu) | baidu-qianfan | `QIANFAN_API_KEY` | openai-compatible preset |
| `umans` | Umans AI Coding Plan | umans-ai | `UMANS_AI_CODING_PLAN_API_KEY` | openai-compatible preset |
| `kilo` | Kilo Gateway | kilo-gateway, kilo-ai | `KILO_API_KEY` | openai-compatible preset |
| `opencode-go` | OpenCode Go | - | `OPENCODE_API_KEY` | openai-compatible preset |
| `cloudflare-ai-gateway` | Cloudflare AI Gateway | - | `CLOUDFLARE_API_TOKEN` | openai-compatible preset |
| `cloudflare-workers-ai` | Cloudflare Workers AI | - | `CLOUDFLARE_API_TOKEN` | openai-compatible preset |
| `google-vertex` | Google Vertex AI | vertexai, google-vertex-anthropic | `GOOGLE_CLOUD_API_KEY`, `VERTEX_API_KEY` | native |
| `amazon-bedrock` | Amazon Bedrock | bedrock | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `AWS_BEARER_TOKEN_BEDROCK`, `AWS_PROFILE`, `AWS_REGION` | native adapter |
| `sap-ai-core` | SAP AI Core | sap | `AICORE_SERVICE_KEY`, `SAP_AI_CORE_CLIENT_ID`, `SAP_AI_CORE_CLIENT_SECRET`, `SAP_AI_CORE_TOKEN_URL`, `SAP_AI_CORE_SERVICE_URL` | native adapter |
| `v0` | v0 by Vercel | - | `V0_API_KEY` | native adapter |
| `azure-openai` | Azure OpenAI | azure, azure_openai, azure-cognitive-services, azure-openai-responses | `AZURE_OPENAI_API_KEY` | native adapter |
| `github-copilot` | GitHub Copilot | copilot, github-copilot-enterprise | `GITHUB_COPILOT_API_KEY`, `GITHUB_TOKEN` | native adapter |
| `gitlab` | GitLab Duo | gitlab-duo | `GITLAB_TOKEN`, `GITLAB_API_KEY` | native adapter |

<!-- PROVIDER_TABLE:END -->

## 示例

### 1. 覆盖 OpenAI Base URL（例如用于 Groq）

```json
{
  "providers": {
    "openai": {
      "baseUrl": "https://api.groq.com/openai/v1",
      "apiKey": "gsk_...",
      "models": [
        {
          "id": "llama3-70b-8192",
          "name": "Groq Llama 3 70B",
          "contextWindow": 8192
        }
      ]
    }
  }
}
```

### 2. Azure OpenAI

Azure 需要资源特定的 URL 与 `api-key` 标头而非 Bearer 令牌。

```json
{
  "providers": {
    "azure-openai": {
      "api": "openai-completions",
      "baseUrl": "https://my-resource.openai.azure.com/openai/deployments/my-deployment",
      "apiKey": "...",
      "authHeader": false,
      "headers": {
        "api-key": "..."
      },
      "models": [
        {
          "id": "gpt-4",
          "contextWindow": 128000
        }
      ]
    }
  }
}
```

### 3. 本地 LLM（Ollama）

```json
{
  "providers": {
    "ollama": {
      "api": "openai-completions",
      "baseUrl": "http://localhost:11434/v1",
      "apiKey": "ollama",
      "models": [
        {
          "id": "llama3",
          "contextWindow": 8192
        }
      ]
    }
  }
}
```

## 敏感信息解析

API 密钥可以是纯字符串、环境变量或 shell 命令。

常规运行时凭证——显式覆盖、提供方环境变量、已存储认证及受支持的外部提供方凭证——优先于提供方路由的 `apiKey`。仅当常规运行时解析未找到非空凭证时，才会使用 `models.json` 的值。

- **环境变量**：若字符串匹配环境变量名（例如 `OPENAI_API_KEY`），则会被解析。
- **Shell 命令**：以 `!` 为前缀以执行命令。

```json
{
  "providers": {
    "openai": {
      "apiKey": "!pass show api/openai"
    }
  }
}
```

Shell 命令在 Unix 上通过 `sh -c` 运行，在 Windows 上通过 `cmd /C` 运行。

### 本地提供方（无需 API 密钥）

`ollama`、`llamacpp`（llama.cpp 的 `llama-server`）、`mistralrs`（mistral.rs）与 `lmstudio` 为已识别的内置**本地**提供方。`ollama`、`llamacpp` 与 `mistralrs` **无需 API 密钥**——它们在 localhost 上暴露 OpenAI 兼容服务器，调用时不带 `Authorization` 标头。无需 `models.json` 条目即可开箱即用：

```bash
# 默认值：llama-server -> http://127.0.0.1:8080/v1，mistral.rs -> http://127.0.0.1:1234/v1
pi --provider llamacpp  --model ggml-org/gemma-4-E4B-it-GGUF -p "hi"
pi --provider mistralrs --model default -p "hi"
```

接受提供方别名：`llama.cpp` / `llama-cpp` / `llama-server` -> `llamacpp`，以及 `mistral.rs` / `mistral-rs` -> `mistralrs`。

要指向非默认主机/端口，请添加 `models.json` 条目（无需 `apiKey`）：

```json
{
  "providers": {
    "llamacpp": {
      "baseUrl": "http://127.0.0.1:9090/v1",
      "models": [ { "id": "my-model" } ]
    }
  }
}
```

## 用户模型覆盖（扩展内置快照）

Pi 随附 `docs/provider-upstream-model-ids-snapshot.json` 处每个提供方发现端点的快照。该快照在发布前重新生成，但提供方的新模型（例如 Anthropic 发布新的 Opus 版本）在下一次发布之前对 `/model` 不可见。

在 `<config_dir>/pi/models-override.json` 处放置一个 JSON 文件即可在运行时扩展快照。该文件使用与内置快照相同的形态：

```json
{
  "anthropic": ["claude-opus-4-7"],
  "openrouter": ["anthropic/claude-opus-4-7"]
}
```

`<config_dir>` 为 `dirs::config_dir()` 报告的任何位置——Linux 上为 `~/.config`，macOS 上为 `~/Library/Application Support`，Windows 上为 `%APPDATA%`。在环境中设置 `PI_MODELS_OVERRIDE=/path/to/file.json` 可让 pi 指向标准配置目录之外的文件。

行为：

- **仅追加。**覆盖条目与内置快照取并集。无法通过覆盖文件*移除*内置模型；提供方的下一次刷新将重新引入你删除的任何内容。
- **跨升级保留。**覆盖文件位于你的用户配置目录，而非 pi 二进制文件中，因此你添加的模型条目会在发布间保持，直到内置快照追上——之后会自动去重。
- **容错。**缺失或格式错误的覆盖文件会记录 debug/warning 行并被视为空，因此拼写错误永远不会破坏 pi 启动。
- **提供方 ID 必须匹配规范名称。**使用 `anthropic`、`openai`、`openrouter` 等（你在 `docs/provider-upstream-model-ids-snapshot.json` 中看到的键）。

该覆盖仅影响 `/model` 自动补全目录。要实际调用 pi 尚未内置路由的模型，还需在 `models.json` 中配置提供方（如上节所述）——无论 ID 是否在快照中，pi 都会通过 Anthropic API 路由任何 `anthropic/<id>` 值。
