# 长尾提供方证据 (bd-3uqg.11.10.7)

快速见效的长尾提供方映射、附带用户影响的显式延期，以及指向通过测试证据的链接，使决策可审计、可复现。

生成时间：2026-02-13

> 历史快照：下方的提供方列表、数量、延期及命令描述的是 2026-02-13 的审计，已为溯源而保留。它们并非当前版本的发布依据。当前的注册表与分发真相位于 `src/provider_metadata.rs`、`src/providers/mod.rs` 及其可执行测试中。

更新于 2026-08-06：原有的 `github-models` 快速见效路径已在 GitHub 于 2026-07-30 退役 GitHub Models 服务后移除。独立的 `github-copilot` 原生提供方仍受支持。

## 快速见效提供方（已实现 + 已测试）

所有快速见效提供方均通过 OpenAI 兼容适配器（`openai-completions`）路由，提供方特定的基础 URL 与认证环境变量定义于 `src/provider_metadata.rs` 中。

### 复制即用配置

每个提供方仅需一个 API 密钥环境变量。基础用例无需 `models.json` 条目。

```bash
# Mistral
export MISTRAL_API_KEY="your-key"
pi --provider mistral --model mistral-large-latest -p "Say hello"

# DeepInfra
export DEEPINFRA_API_KEY="your-key"
pi --provider deepinfra --model meta-llama/Meta-Llama-3.1-70B-Instruct -p "Say hello"

# Together AI
export TOGETHER_API_KEY="your-key"
pi --provider togetherai --model meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo -p "Say hello"

# NVIDIA NIM
export NVIDIA_API_KEY="your-key"
pi --provider nvidia --model meta/llama-3.1-70b-instruct -p "Say hello"

# Hugging Face Inference
export HF_TOKEN="your-key"
pi --provider huggingface --model meta-llama/Meta-Llama-3.1-70B-Instruct -p "Say hello"

# StackIT
export STACKIT_API_KEY="your-key"
pi --provider stackit --model stackit-chat -p "Say hello"

# SiliconFlow
export SILICONFLOW_API_KEY="your-key"
pi --provider siliconflow --model Qwen/Qwen2.5-72B-Instruct -p "Say hello"
```

### 代表性测试覆盖

| 提供方 | 认证环境变量 | 契约测试 | 一致性测试 | 端到端测试 |
|---|---|---|---|---|
| stackit | `STACKIT_API_KEY` | 8 (provider_native_contract.rs) | 7 (provider_native_verify.rs) | E2E_FAMILIES (e2e_provider_scenarios.rs) |
| mistral | `MISTRAL_API_KEY` | 8 | 7 | E2E_FAMILIES |
| deepinfra | `DEEPINFRA_API_KEY` | 8 | 7 | E2E_FAMILIES |
| togetherai | `TOGETHER_API_KEY` | 8 | 7 | E2E_FAMILIES |
| nvidia | `NVIDIA_API_KEY` | 8 | 7 | E2E_FAMILIES |
| huggingface | `HF_TOKEN` | 8 | 7 | E2E_FAMILIES |
| ollama-cloud | `OLLAMA_API_KEY` | 8 | 7 | E2E_FAMILIES |

### 证据产物

- **契约测试**：`tests/provider_native_contract.rs::longtail_contract::*`
  - 56 个按提供方计数的测试（每提供方 8 个 × 7 个提供方）
  - `longtail_provider_metadata` 中 5 个元数据一致性测试
  - 运行：`cargo test --test provider_native_contract -- longtail_contract`
- **一致性测试**：`tests/provider_native_verify.rs::*_conformance`
  - 49 个基于 VCR 的一致性测试（7 个场景 × 7 个提供方）
  - 运行：`cargo test --test provider_native_verify -- stackit_conformance mistral_conformance deepinfra_conformance togetherai_conformance nvidia_conformance huggingface_conformance ollama_cloud_conformance`
- **端到端测试**：`tests/e2e_provider_scenarios.rs`
  - E2E_FAMILIES 中 16 个家族（包含全部 7 个长尾提供方）
  - `e2e_openai_compatible_wave_presets` 中 13 个 wave 预设
  - 运行：`cargo test --test e2e_provider_scenarios`
- **失败分类**：`tests/provider_native_contract.rs::failure_taxonomy`
  - 7 个测试，验证全部 12 个提供方的错误提示覆盖
  - 运行：`cargo test --test provider_native_contract -- failure_taxonomy`
- **注册表护栏**：`tests/provider_registry_guardrails.rs`
  - 防止漂移的测试，确保上游增量可分流
  - 运行：`cargo test --test provider_registry_guardrails`

### 按提供方的失败分类

每个快速见效提供方的错误提示均通过 `src/error.rs::provider_hints()` 验证：

| 失败类别 | 错误模式 | 修复建议 |
|---|---|---|
| 缺少 API 密钥 | "missing api key" | 设置环境变量（按提供方区分） |
| 认证失败 (401) | "401", "unauthorized" | 校验 API 密钥，检查组织权限 |
| 禁止访问 (403) | "403", "forbidden" | 检查模型访问权限与账户权限 |
| 速率限制 (429) | "429", "too many requests" | 等待后重试，降低请求速率 |
| 配额超限 | "insufficient_quota" | 校验计费/余额 |
| 过载 (529) | "529", "overloaded" | 稍后重试 |
| 超时 | "request timed out" | 重试，检查网络连接 |

## 额外的 OpenAI 兼容提供方（仅元数据）

这些提供方在 `src/provider_metadata.rs` 中有元数据条目，但不在代表性测试集合中。它们均使用由上述代表性集合所验证的同一 OpenAI 兼容适配器路径。配置遵循相同模式：设置环境变量，使用 `--provider <id>`。

| 提供方 | 基础 URL | 认证环境变量 |
|---|---|---|
| 302ai | `https://api.302.ai/v1` | `302AI_API_KEY` |
| abacus | `https://routellm.abacus.ai/v1` | `ABACUS_API_KEY` |
| aihubmix | `https://aihubmix.com/v1` | `AIHUBMIX_API_KEY` |
| berget | `https://api.berget.ai/v1` | `BERGET_API_KEY` |
| chutes | `https://llm.chutes.ai/v1` | `CHUTES_API_KEY` |
| cortecs | `https://api.cortecs.ai/v1` | `CORTECS_API_KEY` |
| friendli | `https://api.friendli.ai/serverless/v1` | `FRIENDLI_TOKEN` |
| helicone | `https://ai-gateway.helicone.ai/v1` | `HELICONE_API_KEY` |
| inference | `https://inference.net/v1` | `INFERENCE_API_KEY` |
| nano-gpt | `https://nano-gpt.com/api/v1` | `NANO_GPT_API_KEY` |
| novita-ai | `https://api.novita.ai/openai` | `NOVITA_API_KEY` |
| poe | `https://api.poe.com/v1` | `POE_API_KEY` |
| requesty | `https://router.requesty.ai/v1` | `REQUESTY_API_KEY` |
| siliconflow | `https://api.siliconflow.com/v1` | `SILICONFLOW_API_KEY` |
| venice | `https://api.venice.ai/api/v1` | `VENICE_API_KEY` |
| vultr | `https://api.vultrinference.com/v1` | `VULTR_API_KEY` |
| wandb | `https://api.inference.wandb.ai/v1` | `WANDB_API_KEY` |

## 已延期提供方（共 53 个）

此处列出的提供方在 `provider-parity-checklist.json` 中有元数据条目，但已从快速见效批次中显式延期。每个条目均包含延期原因与用户影响评估。

### 需要原生适配器（无 OpenAI 兼容路径）

这些提供方使用专有协议或认证流程，无法通过 OpenAI 兼容适配器路由。需要专用的适配器模块。

| 提供方 | 原因 | 用户影响 |
|---|---|---|
| v0 | 无已验证的协议/认证路径 | 用户无法通过 Pi 访问 v0 模型 |
| gitlab | 专有认证流程（GitLab Duo） | GitLab Duo 用户必须使用 GitLab 自有工具 |
| llama | 无已确认的 API/认证契约 | 暂不支持 Meta 的 Llama API |
| lmstudio | 仅本地回环，无云端 API | 通过 `models.json` 的 base_url 覆盖可在本地使用 |
| ollama (local) | 无需认证，仅本地回环 | 通过预设可在本地使用；无远程测试 |

### 区域性 CN 变体

中国区变体需要单独的端点与认证验证。全局父提供方可正常工作；CN 变体暂不支持。

| 提供方 | 原因 | 用户影响 |
|---|---|---|
| alibaba-cn | alibaba 的区域性 CN 变体 | CN 用户应使用 alibaba 提供方并覆盖 CN 端点 |
| moonshotai-cn | moonshotai 的区域性 CN 变体 | CN 用户应使用 moonshotai 并覆盖 CN 端点 |
| siliconflow-cn | siliconflow 的区域性 CN 变体 | CN 用户应使用 siliconflow 并覆盖 CN 端点 |
| minimax-cn | 区域性端点/认证未验证 | CN 用户暂无法使用 minimax CN 变体 |
| minimax-cn-coding-plan | CN 编码计划变体未验证 | 暂不可用 |

### 编码计划变体

不同的编码计划风格 ID 目前未在运行时路由中体现。基础提供方可正常工作；专用变体不支持。

| 提供方 | 原因 | 用户影响 |
|---|---|---|
| kimi-for-coding | 路由中无编码计划 ID | 用户可使用 moonshotai 提供方及标准模型 |
| minimax-coding-plan | 编码计划变体未验证 | 用户可使用 minimax 基础模型 |
| zai-coding-plan | 编码计划变体未验证 | 暂不可用 |
| zhipuai-coding-plan | 编码计划变体未验证 | 暂不可用 |

### 无运行时/认证证据

这些提供方出现在上游目录中，但缺乏已验证的协议、认证或端点契约。一旦收集到证据，它们可能会作为快速见效项接入。

| 提供方 | 原因 | 用户影响 |
|---|---|---|
| 302ai | 无仓库内运行时/认证证据 | 暂不可用；支持后请设置 `302AI_API_KEY` |
| abacus | 无仓库内运行时/认证证据 | 暂不可用 |
| aihubmix | 无仓库内运行时/认证证据 | 暂不可用 |
| bailing | 无运行时/认证证据 | 暂不可用 |
| baseten | 无运行时/认证证据 | 暂不可用 |
| berget | 无运行时/认证证据 | 暂不可用 |
| chutes | 无运行时/认证证据 | 暂不可用 |
| cortecs | 无运行时/认证证据 | 暂不可用 |
| firmware | 无运行时/认证证据 | 暂不可用 |
| friendli | 无运行时/认证证据 | 暂不可用 |
| huggingface | 无已验证的运行时/认证契约 | 作为具名提供方暂不可用 |
| iflowcn | 无运行时/认证证据 | 暂不可用 |
| inception | 无运行时/认证证据 | 暂不可用 |
| inference | 无运行时/认证证据 | 暂不可用 |
| io-net | 无运行时/认证证据 | 暂不可用 |
| jiekou | 无运行时/认证证据 | 暂不可用 |
| lucidquery | 无运行时/认证证据 | 暂不可用 |
| minimax | 无已验证的提供方协议 | 暂不可用 |
| moark | 无运行时/认证证据 | 暂不可用 |
| modelscope | 无已验证的协议/认证路径 | 暂不可用 |
| morph | 无运行时/认证证据 | 暂不可用 |
| nano-gpt | 无运行时/认证证据 | 暂不可用 |
| nebius | 无已验证的协议/认证路径 | 暂不可用 |
| nova | 无运行时/认证证据 | 暂不可用 |
| novita-ai | 无已验证的协议/认证路径 | 暂不可用 |
| nvidia | 无已验证的协议/认证路径 | 已通过代表性集合测试，但不在对等清单中 |
| ovhcloud | 无已验证的协议/认证路径 | 暂不可用 |
| poe | 无已验证的协议/认证路径 | 暂不可用 |
| privatemode-ai | 无运行时/认证证据 | 暂不可用 |
| scaleway | 无已验证的协议/认证路径 | 暂不可用 |
| siliconflow | 无已验证的协议/认证路径 | 仅元数据；使用环境变量可能可用 |
| submodel | 无已验证的协议/认证路径 | 暂不可用 |
| synthetic | 无已验证的协议/认证路径 | 暂不可用 |
| upstage | 无已验证的协议/认证路径 | 暂不可用 |
| venice | 无已验证的协议/认证路径 | 仅元数据；使用环境变量可能可用 |
| vivgrid | 无已验证的协议/认证路径 | 暂不可用 |
| vultr | 无已验证的协议/认证路径 | 仅元数据；使用环境变量可能可用 |
| wandb | 无已验证的协议/认证路径 | 仅元数据；使用环境变量可能可用 |
| xiaomi | 无已验证的协议/认证路径 | 暂不可用 |
| zai | 无已验证的协议/认证路径 | 暂不可用 |
| zhipuai | 无已验证的协议/认证路径 | 暂不可用 |

### 已延期提供方的变通方案

需要已延期提供方的用户，若该提供方支持 OpenAI 兼容端点，可使用 `models.json` 手动配置：

```json
{
  "providers": {
    "custom-provider": {
      "apiKey": "your-key",
      "models": {
        "custom-model": {
          "provider": "custom-provider",
          "api": "openai-completions",
          "base_url": "https://api.example.com/v1",
          "context_window": 128000,
          "max_tokens": 4096
        }
      }
    }
  }
}
```

## CI 集成

- 提供方缺口测试矩阵：`docs/provider-gaps-test-matrix.json`
- CI 门禁：`tests/ci_full_suite_gate.rs` 中的 Gate 12 校验测试矩阵
- 产物保留：`.github/workflows/ci.yml` 中分片产物的 30 天保留期
- 套件分类：`tests/suite_classification.toml` 中列出的所有测试文件
- 注册表护栏：`tests/provider_registry_guardrails.rs` 防止静默的上游漂移
- 对等清单：`docs/provider-parity-checklist.json`（99 条目，53 个已延期）

## 决策审计轨迹

| 决策 | 证据 | Bead |
|---|---|---|
| 60+ 个提供方通过 OpenAI 兼容适配器路由 | `src/provider_metadata.rs` 元数据条目 | bd-3uqg.11.10.2 |
| 7 个代表性长尾提供方已测试 | 56 个契约 + 49 个一致性 + 101 个端到端测试 | bd-3uqg.11.10.5, bd-3uqg.11.10.6 |
| 53 个提供方已延期并附原因 | `docs/provider-parity-checklist.json` 延期条目 | bd-3uqg.11.10.3 |
| 注册表护栏防止静默漂移 | `tests/provider_registry_guardrails.rs` | bd-3uqg.11.10.4 |
