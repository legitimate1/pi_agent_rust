# IMPLEMENT.md — subagent 网络重试（复用现有机制）

## 1. 实现边界（Implementation Contract）

Source: 会话讨论 `subagent 网络中断无重试` — 复用现有 `is_retryable_error + 指数退避` 能力

Goal:
- 为 `subagent` 的外进程编排层补上瞬断自动重试，复用既有分类器与退避公式，重试环单独适配子进程模型。

In scope:
- `src/subagents.rs` 增加网络瞬断重试：`is_retryable_error` 判定 + `retry_delay_ms` 指数退避 + `AgentCx::checkpoint` 取消点
- 复用 `Config::retry{enabled,max_retries,base_delay_ms,max_delay_ms}` 配置，`ToolRegistry` 注入路径可读
- `chain`/`parallel`/`single` 三模式的 per-task 重试语义保持首错即断/有序
- `worktree` 隔离下每次重试新建隔离环境（当前 `run_child_process` 已做）；`none` 隔离下允许重试并记录

Out of scope:
- 不改 `crates/pi-provider-core/src/error.rs` 与 `src/error.rs` 的 `is_retryable_error` 正则本身
- 不改 `agent_hub` roster 与 `worktree_iso` 核心逻辑（仅复用其现有 `Keep` 语义）
- 不引入新的 `Error::Transient` 变体或 provider 内流内重试

Assumptions:
- 子代理失败形态为字符串 `error/output/stderr/exit_code`，无 `Error::is_transient` 链路，仅靠 `is_retryable_error` 判定
- 重试幂等性由调用方保证：`worktree` 天然安全，`none` 需业务侧幂等
- 配置缺失时回退 `enabled=true, max_retries=3, base=2000, max=60000`

Design delta:
- 无独立设计文档，本实现即设计；重试次数与主链路 `rpc::run_prompt_with_retry` 对齐 `maxRetries=3`

---

## 2. 文件变更清单（Change Manifest）

| ID | 路径 | 操作 | 用途 | 主要改动 |
|:--:|------|------|------|----------|
| C1 | `src/subagents.rs` | 修改 | 子代理重试主体 | 新增 `retry_delay_ms` helper + `SubagentResult::flatten_error_text` + `ChildRunner::run_one_with_network_retry` 包装；在 `run_one` 外层加 `0..=max_retries` 循环、命中 `is_retryable_error` 才退避重试；接入 `AgentCx::checkpoint` 与 `asupersync::time::sleep` |
| C2 | `src/config.rs` | 修改（可选） | 复用退避公式 | 若抽公用则新增 `pub fn retry_delay_ms(&self, attempt: u32) -> u32` 供 `rpc.rs`/`main.rs`/`subagents.rs` 共用；否则 `subagents.rs` 内联同款公式并注释溯源 |
| C3 | `src/tools/mod.rs` | 修改 | 注入 Config | `ToolRegistry::new` 构造 `SubagentTool` 时传入 `Config` 的 `retry` 视图（或 `SubagentTool::with_retry_config`），无 Config 时 fallback 默认值 |
| C4 | `src/subagents.rs` (tests) | 修改 | 回归覆盖 | 新增 `#[cfg(unix)]` 用 `two-phase-child.sh` 伪造 `fetch failed / other side closed` 失败首跑、第二跑成功的重试用例；另覆盖 `strict/permissive` 不误重试与 `max_retries` 上限 |

---

## 3. 依赖关系（Dependency Plan）

Dependencies:
- C2 依赖：无（可独立先做，若不抽则跳过）
- C1 依赖：C2（若抽公用）/ C3（配置注入）
- C3 依赖：C2
- C4 依赖：C1

并行分组建议：
- Phase 1：C2 + C3（配置与注入，可并行；C2 可省略）
- Phase 2：C1（核心重试环）
- Phase 3：C4（测试）

冲突说明：
- 仅 `src/subagents.rs` 为主冲突点，单 Agent 串行改，避免与 `rpc.rs` 退避公式并发编辑冲突

---

## 4. 验证计划（Validation Plan）

自动化检查：
- `cargo fmt --check`
- `cargo clippy --lib -- -D warnings`
- `cargo test --test subagents` 或 `cargo test subagents`（新增用例必绿）
- `cargo test --lib error::tests::is_retryable` 保持绿（分类器未动）

预期结果：
- 瞬断文案（`fetch failed`/`connection reset`/`stream ended without done`）触发重试并最终 `completed`，`schemaValid` 不受影响
- 非重试文案（`prompt is too long`/`invalid api key`）不重试直接 `failed`
- `retry.enabled=false` 或 `max_retries=0` 时不重试
- 超过 `max_retries` 后以最后一次失败为准

人工检查：
- `parallel` 下并发重试不串扰；`chain` 首失败仍断链
