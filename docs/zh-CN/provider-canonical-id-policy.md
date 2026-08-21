# 提供方规范 ID + 别名策略 (`bd-3uqg.1.3`)

生成时间: `2026-02-10T04:38:00Z`
最后复审: `2026-08-06`
依赖: bd-3uqg.1.1（上游快照）、bd-3uqg.1.2（基线审计）

## 规范化算法

当用户提供提供方 ID（CLI 标志、配置、环境变量）时，按以下顺序执行：

1. **去除首尾空白**：去掉首尾空白字符。
2. **匹配**已注册的规范 ID 和别名（不区分大小写）。
3. **规范化**：将匹配项规范化为 `PROVIDER_METADATA` 中存储的规范 ID。
4. 对于未知 ID **不返回内置匹配项**；自定义和扩展路由在该内置查找之外处理已声明的 ID。

**原理**：[`src/provider_metadata.rs`](../src/provider_metadata.rs) 是运行时权威来源。仅当显式注册为别名时才接受下划线拼写（例如 `azure_openai`）；Pi 不会执行通用的下划线转连字符重写。

## 冲突解决规则

1. **运行时元数据优先**：`PROVIDER_METADATA` 定义了 Pi 当前生效的规范 ID 和别名。
2. **已验证的上游别名**：与 Pi 规范 ID 不同的上游拼写，在其路由和认证行为验证通过后可注册为别名。
3. **已退役服务保持退役**：某个 ID 可能保留在历史上游快照中，但不会保留在当前生效的运行时注册表中。
4. **区域变体**：如 `alibaba-cn`、`moonshotai-cn` 这类 ID 是独立的规范 ID，而非别名。
5. **编程计划变体**：如 `minimax-coding-plan` 这类 ID 是独立的规范 ID。
6. **扩展提供方**：已声明 ID 的语义归属于内置元数据查找之外，由扩展侧自行负责。

## 弃用策略

仅接受显式注册的别名。当前没有生效中的规范 ID 弃用。`fireworks` 和 `azure-openai` 是运行时规范 ID；`fireworks-ai` 及 Azure 的其他拼写均为别名。

## 别名对照表

| 别名                                                                          | 规范 ID              | 来源                  |
| ----------------------------------------------------------------------------- | -------------------- | --------------------- |
| `antigravity`                                                                 | `google-antigravity` | Pi runtime            |
| `atlas`, `atlas-cloud`                                                        | `atlascloud`         | Pi runtime            |
| `azure`, `azure_openai`, `azure-cognitive-services`, `azure-openai-responses` | `azure-openai`       | Pi runtime            |
| `bedrock`                                                                     | `amazon-bedrock`     | opencode              |
| `codex`, `chatgpt-codex`                                                      | `openai-codex`       | Pi runtime            |
| `copilot`, `github-copilot-enterprise`                                        | `github-copilot`     | opencode + Pi runtime |
| `cursor-agent`                                                                | `cursor`             | Pi runtime            |
| `dashscope`                                                                   | `alibaba`            | Pi alias              |
| `deep-infra`                                                                  | `deepinfra`          | Pi runtime            |
| `deep-seek`                                                                   | `deepseek`           | Pi runtime            |
| `fireworks-ai`                                                                | `fireworks`          | Pi runtime            |
| `gemini`                                                                      | `google`             | opencode              |
| `gemini-cli`                                                                  | `google-gemini-cli`  | Pi runtime            |
| `gitlab-duo`                                                                  | `gitlab`             | opencode              |
| `glm`, `zhipu`                                                                | `zhipuai`            | Pi runtime            |
| `google-vertex-anthropic`, `vertexai`                                         | `google-vertex`      | models.dev + opencode |
| `grok`, `x-ai`                                                                | `xai`                | Pi runtime            |
| `hf`, `hugging-face`                                                          | `huggingface`        | Pi runtime            |
| `kimi`                                                                        | `moonshotai`         | Pi alias              |
| `kimi-code`, `kimi-coding`                                                    | `kimi-for-coding`    | Pi runtime            |
| `llama-cpp`, `llama.cpp`, `llama-server`                                      | `llamacpp`           | Pi runtime            |
| `lm-studio`                                                                   | `lmstudio`           | Pi runtime            |
| `mistral-rs`, `mistral.rs`                                                    | `mistralrs`          | Pi runtime            |
| `mistralai`                                                                   | `mistral`            | Pi runtime            |
| `moonshot`                                                                    | `moonshotai`         | Pi alias              |
| `nanogpt`                                                                     | `nano-gpt`           | Pi runtime            |
| `nim`, `nvidia-nim`                                                           | `nvidia`             | Pi runtime            |
| `novita`                                                                      | `novita-ai`          | Pi runtime            |
| `open-router`                                                                 | `openrouter`         | Pi runtime            |
| `pplx`                                                                        | `perplexity`         | Pi runtime            |
| `qwen`                                                                        | `alibaba`            | Pi alias              |
| `sap`                                                                         | `sap-ai-core`        | opencode              |
| `silicon-flow`                                                                | `siliconflow`        | Pi runtime            |
| `together`, `together-ai`                                                     | `togetherai`         | Pi runtime            |
| `vercel-ai-gateway`                                                           | `vercel`             | Pi runtime            |

总计：51 个别名，对应 33 个规范 ID。

## 规范 ID 注册表（94 个生效 ID）

镜像当前生效的运行时注册表。机器可读的来源为 [`provider-canonical-id-table.json`](provider-canonical-id-table.json)；策略 JSON 镜像了相同的 94 个 ID 和 51 个别名。

| 规范 ID                | 是否有别名                                                                  | 来源                  |
| ---------------------- | --------------------------------------------------------------------------- | --------------------- |
| 302ai                  | no                                                                          | models.dev            |
| abacus                 | no                                                                          | models.dev            |
| aihubmix               | no                                                                          | models.dev            |
| alibaba                | yes (dashscope, qwen)                                                       | models.dev            |
| alibaba-cn             | no                                                                          | models.dev            |
| alibaba-us             | no                                                                          | Pi runtime            |
| amazon-bedrock         | yes (bedrock)                                                               | models.dev + opencode |
| anthropic              | no                                                                          | all                   |
| atlascloud             | yes (atlas, atlas-cloud)                                                    | Pi runtime            |
| azure-openai           | yes (azure, azure_openai, azure-cognitive-services, azure-openai-responses) | Pi runtime            |
| bailing                | no                                                                          | models.dev            |
| baseten                | no                                                                          | models.dev            |
| berget                 | no                                                                          | models.dev            |
| cerebras               | no                                                                          | models.dev + opencode |
| chutes                 | no                                                                          | models.dev            |
| cloudflare-ai-gateway  | no                                                                          | models.dev + opencode |
| cloudflare-workers-ai  | no                                                                          | models.dev + opencode |
| cohere                 | no                                                                          | models.dev            |
| cortecs                | no                                                                          | models.dev            |
| cursor                 | yes (cursor-agent)                                                          | Pi runtime            |
| deepinfra              | yes (deep-infra)                                                            | models.dev            |
| deepseek               | yes (deep-seek)                                                             | models.dev            |
| fastrouter             | no                                                                          | models.dev            |
| fireworks              | yes (fireworks-ai)                                                          | Pi runtime            |
| firmware               | no                                                                          | models.dev            |
| friendli               | no                                                                          | models.dev            |
| github-copilot         | yes (copilot, github-copilot-enterprise)                                    | models.dev + opencode |
| gitlab                 | yes (gitlab-duo)                                                            | models.dev + opencode |
| google                 | yes (gemini)                                                                | models.dev + opencode |
| google-antigravity     | yes (antigravity)                                                           | Pi runtime            |
| google-gemini-cli      | yes (gemini-cli)                                                            | Pi runtime            |
| google-vertex          | yes (vertexai, google-vertex-anthropic)                                     | models.dev + opencode |
| groq                   | no                                                                          | models.dev + opencode |
| helicone               | no                                                                          | models.dev            |
| huggingface            | yes (hf, hugging-face)                                                      | models.dev            |
| iflowcn                | no                                                                          | models.dev            |
| inception              | no                                                                          | models.dev            |
| inference              | no                                                                          | models.dev            |
| io-net                 | no                                                                          | models.dev            |
| jiekou                 | no                                                                          | models.dev            |
| kimi-for-coding        | yes (kimi-coding, kimi-code)                                                | models.dev            |
| llama                  | no                                                                          | models.dev            |
| llamacpp               | yes (llama-cpp, llama.cpp, llama-server)                                    | Pi runtime            |
| lmstudio               | yes (lm-studio)                                                             | models.dev + codex    |
| lucidquery             | no                                                                          | models.dev            |
| minimax                | no                                                                          | models.dev            |
| minimax-cn             | no                                                                          | models.dev            |
| minimax-cn-coding-plan | no                                                                          | models.dev            |
| minimax-coding-plan    | no                                                                          | models.dev            |
| mistral                | yes (mistralai)                                                             | models.dev            |
| mistralrs              | yes (mistral-rs, mistral.rs)                                                | Pi runtime            |
| moark                  | no                                                                          | models.dev            |
| modelscope             | no                                                                          | models.dev            |
| moonshotai             | yes (moonshot, kimi)                                                        | models.dev            |
| moonshotai-cn          | no                                                                          | models.dev            |
| morph                  | no                                                                          | models.dev            |
| nano-gpt               | yes (nanogpt)                                                               | models.dev            |
| nebius                 | no                                                                          | models.dev            |
| nova                   | no                                                                          | models.dev            |
| novita-ai              | yes (novita)                                                                | models.dev            |
| nvidia                 | yes (nim, nvidia-nim)                                                       | models.dev            |
| ollama                 | no                                                                          | codex                 |
| ollama-cloud           | no                                                                          | models.dev            |
| openai                 | no                                                                          | all                   |
| openai-codex           | yes (codex, chatgpt-codex)                                                  | Pi runtime            |
| opencode               | no                                                                          | models.dev + opencode |
| openrouter             | yes (open-router)                                                           | models.dev + opencode |
| ovhcloud               | no                                                                          | models.dev            |
| perplexity             | yes (pplx)                                                                  | models.dev            |
| poe                    | no                                                                          | models.dev            |
| privatemode-ai         | no                                                                          | models.dev            |
| requesty               | no                                                                          | models.dev            |
| sap-ai-core            | yes (sap)                                                                   | models.dev + opencode |
| scaleway               | no                                                                          | models.dev            |
| siliconflow            | yes (silicon-flow)                                                          | models.dev            |
| siliconflow-cn         | no                                                                          | models.dev            |
| stackit                | no                                                                          | Pi runtime            |
| submodel               | no                                                                          | models.dev            |
| synthetic              | no                                                                          | models.dev            |
| togetherai             | yes (together, together-ai)                                                 | models.dev            |
| upstage                | no                                                                          | models.dev            |
| v0                     | no                                                                          | models.dev            |
| venice                 | no                                                                          | models.dev            |
| vercel                 | yes (vercel-ai-gateway)                                                     | models.dev + opencode |
| vivgrid                | no                                                                          | models.dev            |
| vultr                  | no                                                                          | models.dev            |
| wandb                  | no                                                                          | models.dev            |
| xai                    | yes (grok, x-ai)                                                            | models.dev + opencode |
| xiaomi                 | no                                                                          | models.dev            |
| zai                    | no                                                                          | models.dev            |
| zai-coding-plan        | no                                                                          | models.dev            |
| zenmux                 | no                                                                          | models.dev + opencode |
| zhipuai                | yes (zhipu, glm)                                                            | models.dev            |
| zhipuai-coding-plan    | no                                                                          | models.dev            |

## 凭据解析优先级

`AuthStorage::resolve_api_key` 按以下顺序执行：

1. 显式覆盖；
2. 已存储且未过期的 OAuth 访问令牌或 `BearerToken`；
3. 按元数据顺序排列的提供方环境变量；
4. 已存储的 `ApiKey`；
5. 仅当使用 Pi 全局认证存储时，从其他本地编程 CLI 自动检测到的受支持凭据。

规范 ID 与别名共享已存储和外部凭据查找。别名解析不作为独立的优先级层级。若认证解析器未返回凭据，应用层的模型选择仍可使用内联 `models.json` 中的 `apiKey` 回退。

## 已退役的提供方 ID

`github-models` 不是生效中的规范 ID 或别名。GitHub 已于 2026-07-30 退役该服务。这不会移除或重命名 `github-copilot`，后者是独立的受支持原生提供方。

## 实现指引

请使用现有的运行时元数据函数，而非另行维护一套规范化实现：

```rust
use pi::provider_metadata::{canonical_provider_id, provider_auth_env_keys};

let canonical = canonical_provider_id(user_input);
let auth_env_keys = provider_auth_env_keys(user_input);
```

提供方入口在选择内置路由或凭据之前，应通过 `provider_metadata()` 或 `canonical_provider_id()` 进行解析。自定义和扩展路由另行处理无内置匹配的情况。
