# IMPLEMENT.md — x-opencode-session 请求头

## 1. 实现边界（Implementation Contract）

Source: OpenCode 官方邮件 — `OpenCode Go` 要求请求携带 `x-opencode-session`（每个 conversation 一个稳定 ID），缺头自 09/06 可能直接 error。本地 + `upstream/main` 均无此实现，需自己做。

Goal:
- opencode 系 provider 的每个请求自动带上 `x-opencode-session: <session_id>`，值取现成的 `StreamOptions.session_id`（即 `session.header.id`，天然 per-conversation 稳定）。

In scope:
- `opencode-go`（`https://opencode.ai/zen/go/v1`，邮件点名）注入该头。
- `opencode`（`https://opencode.ai/zen/v1`，同服务 sibling）一并注入——同属 OpenCode 服务，行为一致，无理由区别对待。
- 判定条件：provider 规范 id 为 `opencode` / `opencode-go`，或 `base_url` 含 `opencode.ai`（覆盖用户自定义 provider id 但指向该网关的情形）。
- 仅当 `options.session_id` 为 `Some` 且非空时注入；用户已在 `compat.custom_headers` / `options.headers` 里自定义该头时不覆盖（用户优先级最高）。
- 在 `src/providers/openai.rs` 的 `OpenAIProvider::stream` 里注入（opencode 系均为 `openai-completions` 路由，只走这一条发送路径）。
- 补单元测试：opencode-go 发头、非 opencode 不发、`session_id=None` 不发、用户自定义头不被覆盖。

Out of scope:
- `openai_responses.rs`（codex 路径）不动——opencode 系不走 responses 路由。
- 环境变量覆盖开关（`PI_xxx`）——邮件没要求，保持 YAGNI；用户真要定制可用 models.json `custom_headers`。
- 非流式 `complete` 路径——`OpenAIProvider` 只有 `stream` 一个发送点，无第二路径。
- compaction（`compaction.rs:1319` 的 `session_id` 为 `None`）——按规则自动不发，不专门补。

Assumptions:
- `StreamOptions.session_id` 在主链路（`app.rs:994`）已填 `session.header.id`，稳定且 per-conversation；ACP（`acp.rs:1394`）同样已填。
- 自研 `http::client::RequestBuilder::header` 为 upsert 语义（同名覆盖），注入顺序决定优先级——自动注入放前面，用户头放后面自然覆盖。

Design delta:
- 无设计文档，本实现即设计。写法模仿两处现成模式：
  - `openai.rs:442-460` 的 openrouter 默认头注入（先默认值、后让 custom 覆盖）。
  - `openai_responses.rs:278-280` 的 codex `session_id` 头注入（`if let Some(session_id) = &options.session_id`）。

---

## 2. 文件变更清单（Change Manifest）

| ID | 路径 | 操作 | 用途 | 主要改动 |
|:--:|:-----|:----|:-----|:---------|
| C1 | `src/providers/openai.rs` | 修改 | 注入 `x-opencode-session` + 配套单测 | `stream()` 内加判定+注入（约15行）；`mod tests` 加4个单测，复用 `run_stream_and_capture_headers` 系 harness |

单文件闭合，无跨模块依赖：判定用已 import 的 `canonical_provider_id`，`session_id` 取 `options` 现成字段。

---

## 3. 依赖关系（Dependency Plan）

Dependencies:
- C1 依赖：无（实现+测试同文件，原子提交）。

并行分组建议：
- Phase 1：C1，主 Agent 自己串行做（简单任务：单文件、约20行实现+约60行测试）。

冲突说明：
- 无他人并行改动该文件（custom 分支当前干净）。

---

## 4. 验证计划（Validation Plan）

自动化检查：
- `cargo test --lib providers::openai`（针对性：新4单测 + 存量 openai 单测全过）
- `cargo clippy --lib -- -D warnings && cargo fmt --check`（日常轻量门禁）

预期结果：
- 新单测：opencode-go（provider 名判定 + base_url 判定两路）请求头含 `x-opencode-session: <session_id>`；openai 普通 provider 不含；`session_id=None` 不含；用户预置同名头时保留用户值。
- 存量单测零回归；clippy/fmt 全绿。

人工检查：
- 无（header 为纯加法；VCR 回放不受影响——新头只在真实 opencode 请求出现，fixture 走 loopback 不命中判定）。
