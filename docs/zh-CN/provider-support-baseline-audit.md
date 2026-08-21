# 提供方支持基线审计 (`bd-3uqg.11.1`)

生成时间（UTC）：`2026-02-13T04:48:33Z`

机器可读产物：`docs/provider-baseline-audit.json`

> 历史快照：下列计数与执行指引描述的是 2026-02-13 `bd-3uqg.11` 规划基线，并非当前的注册表或发布证据。请以 `src/provider_metadata.rs`、`src/providers/mod.rs` 以及提供方元数据/工厂测试为准查看实时树。

## 摘要

- 上游并集提供方：**90**
- 矩阵行（含显式用户别名）：**92**
- 元数据中 Pi 规范提供方：**87**

### 当前 Pi 状态计数

| 状态 | 数量 |
|---|---:|
| `alias->native-implemented` | 4 |
| `alias->oai-compatible-preset` | 3 |
| `native-adapter-required-unimplemented` | 2 |
| `native-implemented` | 8 |
| `oai-compatible-preset` | 75 |

### 风险计数

| 风险 | 数量 |
|---|---:|
| `high` | 7 |
| `low` | 14 |
| `medium` | 71 |

## 用户请求的提供方解析

| 提供方 | 规范 | 当前状态 | 目标状态 | 风险 |
|---|---|---|---|---|
| `alibaba` | `alibaba` | `oai-compatible-preset` | `promote-to-provider-specific-runtime-path-and-complete-test-doc-evidence` | `high` |
| `cerebras` | `cerebras` | `oai-compatible-preset` | `promote-to-provider-specific-runtime-path-and-complete-test-doc-evidence` | `high` |
| `groq` | `groq` | `oai-compatible-preset` | `promote-to-provider-specific-runtime-path-and-complete-test-doc-evidence` | `high` |
| `kimi` | `moonshotai` | `alias->oai-compatible-preset` | `promote-to-provider-specific-runtime-path-and-complete-test-doc-evidence` | `high` |
| `moonshotai` | `moonshotai` | `oai-compatible-preset` | `promote-to-provider-specific-runtime-path-and-complete-test-doc-evidence` | `high` |
| `openrouter` | `openrouter` | `oai-compatible-preset` | `promote-to-provider-specific-runtime-path-and-complete-test-doc-evidence` | `high` |
| `qwen` | `alibaba` | `alias->oai-compatible-preset` | `promote-to-provider-specific-runtime-path-and-complete-test-doc-evidence` | `high` |

## 执行指引

- 仅用于历史 `bd-3uqg.11` 复原时使用此冻结矩阵。
- 优先处理 `high` 风险行，其次处理阻碍对等完备性的 `medium` 行。
