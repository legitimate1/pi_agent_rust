# 提供方选型与迁移指南 (bd-3uqg.11.12.3)

如何在多个提供方之间进行选型并安全地迁移配置。

生成时间：2026-02-13

## 提供方选型指南

### 按使用场景选型

| 使用场景 | 推荐提供方 | 原因 |
|---|---|---|
| 通用对话 | openai, anthropic | 模型选择最广，质量最佳 |
| 快速推理 | groq, cerebras | 硬件加速，低延迟 |
| 成本优化 | deepinfra, togetherai | 开放模型定价有竞争力 |
| 开放模型 | huggingface, nvidia, togetherai | 可访问 Llama、Mistral 等 |
| 模型聚合 | openrouter | 单个 API 密钥接入多个提供方 |
| 欧盟数据驻留 | stackit | 欧盟托管端点 |
| 中文模型 | alibaba (qwen), moonshotai (kimi) | 可访问 Qwen、Kimi 模型 |
| 自托管 | ollama (local) | 私有化，数据不出本机 |
| 代码专用 | mistral | Codestral 及代码优化模型 |
| 编程智能体 | kimi-for-coding | 通过 Anthropic Messages API 接入 Kimi K2.5 |

### 按优先级选型

1. **可靠性**：anthropic、openai、google（原生适配器，测试最充分）
2. **性能**：groq、cerebras（专用硬件）
3. **灵活性**：openrouter（模型市场，300+ 模型）
4. **成本**：deepinfra、togetherai、huggingface（开放模型托管）
5. **隐私**：ollama（完全本地）

### Gap 提供方对比矩阵

| 特性 | Groq | Cerebras | OpenRouter | Kimi (moonshotai) | Qwen (alibaba) |
|---|:---:|:---:|:---:|:---:|:---:|
| API 类型 | openai-completions | openai-completions | openai-completions | openai-completions | openai-completions |
| 工具调用 | 是 | 部分支持（仅 3 个模型） | 是 | 是（K2+） | 部分支持 |
| 流式 | 是 | 是 | 是 | 是 | 是 |
| 流式 + 工具 | 是 | 是（非推理模型） | 是 | 是 | 否（旧模型） |
| 最大上下文 | 128K | 131K | 取决于模型 | 262K | 1M |
| temperature 范围 | 0-2 | 0-1.5 | 取决于模型 | 0-1 | 0-2 |
| `n` 参数 | 仅 n=1 | 仅 n=1 | 是 | 是 | 有限支持 |
| 并行工具调用 | 是 | 有限支持 | 是 | 是（K2+） | 是 |
| 免费层级速率限制 | 30 RPM | 30 RPM | 因套餐而异 | 3 RPM | 因套餐而异 |
| 区域变体 | 无 | 无 | 无 | .ai（全球）/ .cn（中国） | intl / cn |

## 在提供方之间迁移

### 切换提供方

所有 OpenAI 兼容提供方共享相同的线上传输格式。在它们之间切换只需更改环境变量和提供方标志：

```bash
# From Groq to Cerebras
# Before:
export GROQ_API_KEY="gsk_..."
pi --provider groq --model llama-3.3-70b-versatile

# After:
export CEREBRAS_API_KEY="csk-..."
pi --provider cerebras --model llama-3.3-70b
```

```bash
# From direct provider to OpenRouter
# Before:
export GROQ_API_KEY="gsk_..."
pi --provider groq --model llama-3.3-70b-versatile

# After:
export OPENROUTER_API_KEY="sk-or-v1-..."
pi --provider openrouter --model meta-llama/llama-3.3-70b-instruct
# Note: OpenRouter uses org/model format for model IDs
```

```bash
# Between Kimi regional endpoints
# Before (global):
export MOONSHOT_API_KEY="sk-global-key"
pi --provider moonshotai --model kimi-k2.5

# After (China):
export MOONSHOT_API_KEY="sk-china-key"  # Different key!
pi --provider moonshotai-cn --model kimi-k2.5
# WARNING: Keys are NOT interchangeable between .ai and .cn endpoints
```

### Model ID 差异

同一模型在不同提供方上使用不同的 model ID 格式：

| 模型 | Groq | Cerebras | DeepInfra | Together AI | NVIDIA | OpenRouter |
|---|---|---|---|---|---|---|
| Llama 3.3 70B | llama-3.3-70b-versatile | llama-3.3-70b | meta-llama/Meta-Llama-3.3-70B-Instruct | meta-llama/Llama-3.3-70B-Instruct-Turbo | meta/llama-3.3-70b-instruct | meta-llama/llama-3.3-70b-instruct |
| Qwen 3 32B | -- | qwen-3-32b | Qwen/Qwen3-32B | Qwen/Qwen3-32B | -- | qwen/qwen3-32b |

### 迁移安全清单

切换提供方前，请核对：

1. **认证环境变量**：每个提供方使用各自的环境变量（`GROQ_API_KEY`、`CEREBRAS_API_KEY`、`OPENROUTER_API_KEY`、`MOONSHOT_API_KEY`、`DASHSCOPE_API_KEY` 等）
2. **Model ID**：不同提供方的模型名称不同（见上表）
3. **工具调用支持**：并非所有提供方/模型都支持工具调用
   - Cerebras：仅 `gpt-oss-120b`、`qwen-3-32b`、`zai-glm-4.7`
   - Qwen：旧模型上无法同时使用流式 + 工具
   - Kimi：不支持 `tool_choice="required"`
4. **temperature 范围**：限制在提供方允许范围内
   - Groq：0-2（标准）
   - Cerebras：0-1.5
   - Kimi：0-1（大于 1 的值会被拒绝）
   - Qwen：0-2（标准）
5. **速率限制**：重度使用前请检查提供方层级限制
6. **区域端点**：Kimi（.ai 与 .cn）和 Qwen（intl 与 cn）的密钥不可互换
7. **不支持的参数**：部分 OpenAI 参数会被静默忽略或拒绝
   - Cerebras：`frequency_penalty`、`presence_penalty`、`logit_bias` 会返回 400
   - Groq：`n`、`logprobs`、`logit_bias` 会被静默忽略
   - OpenRouter：不支持的参数可能被上游静默忽略

### 提供方特定的迁移注意事项

**迁移至 Groq**：
- `temperature=0` 会在服务端被归一化为 `1e-8`
- `n` 必须为 1（不支持多条补全）
- 消息的 `.name` 字段会被静默忽略

**迁移至 Cerebras**：
- 非标准的速率限制头（`x-ratelimit-*-day`、`x-ratelimit-*-minute`）
- 响应中包含 `time_info`（WSE 计时数据）—— 额外字段，可安全忽略
- `frequency_penalty` 和 `presence_penalty` 会导致 HTTP 400

**迁移至 OpenRouter**：
- Model ID 需使用 `org/model` 格式（例如 `openai/gpt-4o-mini`，而非 `gpt-4o-mini`）
- 实际服务的模型可能与请求的模型不同（请检查 `response.model`）
- 流式传输中途错误以 SSE 负载形式到达，`finish_reason='error'`（HTTP 200）
- SSE 注释帧（`: OPENROUTER PROCESSING`）必须忽略

**迁移至 Kimi (moonshotai)**：
- Pi 中有三项：`moonshotai`（全球）、`moonshotai-cn`（中国）、`kimi-for-coding`（Anthropic API）
- 密钥在 `.ai` 和 `.cn` 端点之间不可互换
- `kimi-for-coding` 使用 `anthropic-messages` API，而非 `openai-completions`
- temperature 必须为 0-1（而非 0-2）
- 不支持 `tool_choice="required"` — 请改用 `"auto"`

**迁移至 Qwen (alibaba)**：
- 旧模型上工具调用无法与流式结合使用
- 两种不同的 429 错误类型：`qps`（可重试）与 `quota`（不可重试）
- 在当前 OpenAI 兼容示例中 `system_fingerprint` 返回 null，请勿依赖它
- `logprobs` 始终返回 null

## 相关文档

- 配置示例：`docs/provider-config-examples.json`
- 认证问题排查：`docs/provider-auth-troubleshooting.md`
- 长尾证据：`docs/provider-longtail-evidence.md`
- 各提供方配置文档：`docs/provider-{groq,cerebras,openrouter,kimi,qwen}-setup.json`
