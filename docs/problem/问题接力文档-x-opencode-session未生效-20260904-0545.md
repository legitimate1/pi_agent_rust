# 问题接力文档 - x-opencode-session 请求头未生效

## 问题
- 最终目标：opencode-go 请求自动携带 `x-opencode-session`（每 conversation 稳定 ID），满足 OpenCode 09/06 强制要求。
- 现象：已实现并合入（`60a8ce2a`，my-check 全绿），但实际未生效（待补充具体表现：请求根本没带头 / 带了但服务端仍报错 / 09/06 后 error）。
- 复现步骤：（待补充：如何验证的——抓包看请求头 / 服务端回执 / 所用 provider+model）。
- 环境：Windows + Rust nightly-2026-07-05，分支 `custom`（`origin/custom` 已同步）。

## 已尝试路径
- 尝试路径：在 `src/providers/openai.rs` 的 `OpenAIProvider::stream()` 注入请求头
  - 预期：opencode 系请求带上 `x-opencode-session: <session.header.id>`。
  - 实际结果：单测全过（5/5）、`cargo test --lib providers::openai` 76 passed、clippy+fmt 全绿、my-check 云端 success；但真实请求未生效。
  - 为何放弃：代码逻辑在测试环境成立，问题应在测试覆盖不到的真实链路，需下个会话继续定位。

## 当前推测
- 最可能根因：（待定位。候选方向见下方“关键资源”。）
- 已否决的根因：上游已有实现（本地 + `upstream/main` 均 grep 确认无此头，需自己做）。

## 关键资源
- 实现 commit：`60a8ce2a feat(provider): send x-opencode-session on opencode gateway requests`
- 实现文档：`docs/design/pi-agent-rust-x-opencode-session-implement.md`
- 决策：`docs/context/design-decisions.md` D64
- 核心代码：`src/providers/openai.rs`
  - `OPENCODE_SESSION_HEADER` 常量 + `provider_targets_opencode()`（规范 id / 原始名 / base_url 三路判定）
  - `stream()` 内注入点（`Accept`/`Authorization` 之后、`compat.custom_headers` 之前）
  - 测试：`opencode_session_header_targets_opencode_gateway` + 4 个 `test_stream_opencode_*`
- 下个会话优先排查方向：
  1. 真实请求是否真走 `OpenAIProvider::stream`——opencode-go 的 `api` 字段若不是 `openai-completions`（如配成 responses），路由会进 `openai_responses.rs`，那条路没加头。
  2. `StreamOptions.session_id` 在真实链路是否为 `None`——主链路 `app.rs:994` 有填，但若走 compaction（`compaction.rs:1319` 为 `None`）或扩展 provider（`extensions.rs:33054` 为 `None`），按规则自动不发。
  3. `self.provider` / `self.base_url` 在真实 `ModelEntry` 里到底是什么值——`provider_targets_opencode` 三路都没命中则静默跳过；建议先打一条 `tracing::debug!` 看实际值。
  4. header 大小写/下划线——自研 `http::client` 对 header 名做 sanitize（`client.rs:871`），确认 `x-opencode-session` 原样上 wire（单测走 loopback 已验证上 wire，但 VCR/代理链路未验）。
