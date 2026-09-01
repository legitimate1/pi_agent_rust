# IMPLEMENT.md — 可续命 Subagent（sessionful continuable）

## 1. 实现边界（Implementation Contract）

Source: scout 调研（src/subagents.rs:1401/1350, src/agent_hub.rs:279/336, src/session.rs:1336, src/worktree_iso.rs:147） + planner 最小方案

Goal: 让 Done 后的 subagent 可通过 `hubId + 新 task` 在同一 Session/Worktree 上追加一条 user 消息继续执行，原理即“同一 session push user 消息继续 run”。

In scope:
- `subagent` Tool 增量参数 `continue/hubId/task`（兼容老调用）
- 去掉 `child_args` 硬编码 `--no-session`，改为 `--session <global_dir/sessions/subagents/<hubId>.jsonl>` 持久化
- `agent_hub.rs` 新增 `Idle` 态（可 steer），收窄 `settled()`，新增 `continue_task` 替代 `revive` 16KB 拼文本
- `worktree_iso.rs` 由一次性 `tmp/pi-iso-<id>` 改 `global_dir/worktrees/subagents/<hubId>` 持久化，`cleanup=false` 时保留
- 返回值带 `hubId/sessionId/status`，供主 Agent 链式 `continue`

Out of scope:
- 跨主会话/跨进程 PID 恢复（hub 目录仍 `<global_dir>/agent-hub/<pid>`，父进程退出即回收）
- 进程常驻 RPC/PTY（本期不做长驻子进程，仅做会话重放续跑；Idle 即已 settle 后重建进程但复用 Session+Worktree）
- 新的鉴权/限流、分布式调度

Assumptions:
- `Session::new/open/push_entry` 已支持 JSONL 落盘（src/session.rs），复用即可
- `--session/--session-dir` 旗标已存在（src/cli.rs），`global_dir` 来自 `Config::global_dir()`
- `agent.rs` 的 `run_loop/drain_steering_messages` 已具备续写能力，无需改动

Design delta:
- Planner 曾设想“同一 Child 进程保持 Idle 轮询”，为控制复杂度本期降为“进程退出 + Session 重放”语义：Done 后进程仍回收，但 Session+Worktree 保留，下一次 `continue` 起新进程 `Session::open + 复用 worktree path` 继续跑。效果等价于“同一上下文追加消息”，实现更简单、资源不泄漏。若后续需要真常驻，再升级为 Idle 进程池。

---

## 2. 文件变更清单（Change Manifest）

| ID | 路径 | 操作 | 用途 | 主要改动 |
|:--:|:-----|:----|:-----|:---------|
| C1 | `src/subagents.rs` | 修改 | Tool 契约 + 会话落盘分支 | `SubagentRequest/SubagentTask` 新增 `continue/hubId`（Option, serde default）；`parameters()` JSON Schema 增量；`execute/run_request/run_one` 分流：`continue=true` 校验 hubId+task 走 `continue_task`，否则走新建；`child_args` 去 `--no-session` 改 `--session <path>`（C1a）；`SubagentResult` 新增 `hubId` 透出；返回 `details` 带 `hubId` |
| C2 | `src/agent_hub.rs` | 修改 | Hub 可续命状态机 | `ChildStatus` 新增 `Idle`（或复用 `Done` 但 `settled()` 收窄）；`ChildEntry` 新增 `session_path/worktree_path`；`settled()` 仅 `Failed/Cancelled/Killed` 为终态（或新增 `is_continuable()`）；`steer` 门禁放宽允许 `Idle/Done` 投递；新增 `continue_task(hubId, prompt)`：校验存在→ `Session::open` → 追加 user 消息 → 返回新 run；`revive` 标记 deprecated 转调 `continue_task`；`cleanup_session_files` 保留 subagent worktree/sessions |
| C3 | `src/worktree_iso.rs` | 修改 | Worktree 持久化复用 | `isolate` 改 `global_dir/worktrees/subagents/<hubId>` 持久路径；新增 `reopen(hubId)`/`path_for_hub` 复用；`Drop/collect_diff` 在 `Persistent` 且 `!cleanup` 时保留不删；新增 `cleanup` 显式回收 |
| C4 | `src/session.rs` | 修改（小） | 暴露便捷追加 | 确认/新增 `append_user_message(session_path, text)` 薄封装供 C2 调用；无大改 |
| C5 | `tests/`（如 `tests/e2e_cli.rs`/`tests/conformance/*`） | 修改（小） | 适配新契约 | `defaults.json` 的 `tools` 枚举若校验严格则增补；新增 1-2 个 continuable 单测（新建→continue 能读到历史） |

---

## 3. 依赖关系（Dependency Plan）

Dependencies:
- C1 依赖：无（先定契约，后端才能接）
- C4 依赖：C1（参数定后才知会话路径）
- C2 依赖：C1, C4（需 hubId/session_path）
- C3 依赖：C1（需 hubId）
- C5 依赖：C1, C2, C3

并行分组建议：
- Phase 1：C1（Tool 契约 + child_args 去 no-session）
- Phase 2：C2 + C3 可并行（Hub 状态机与 Worktree 持久化互不写同一文件）
- Phase 3：C4 收口 + C5 测试/文档

冲突说明：
- `src/subagents.rs` 与 `src/agent_hub.rs` 由不同 worker 并行时不冲突；若单 worker 串行则顺序 C1→C2→C3
- `worktree_iso.rs` 独立，无冲突

---

## 4. 验证计划（Validation Plan）

自动化检查：
- `cargo clippy --lib -- -D warnings && cargo fmt --check`（日常轻量）
- `cargo test --lib` + `cargo test --test e2e_cli` 定向（subagent 相关单测）
- 手动 e2e：`spawn(task1: "在 /tmp/foo 写 a=1") -> 记 hubId -> continue(hubId, task2: "读取你之前写的 a 并追加 b=2") -> 校验文件含 a+b 且子进程 output 含历史`

预期结果：
- 老调用 `subagent(agent, task)` 无 `continue` 时行为不变（新建 Session 落盘，返回 hubId）
- 新调用 `subagent(continue=true, hubId, task)` 在同一 Session/Worktree 上增量执行，历史消息对子进程可见
- `steer` 在 Done/Idle 后不再 `cannot steer`，而走 continue 路径
- Worktree 在多次 continue 间保留脏改动，仅 `cleanup=true` 或超时才回收

人工检查：
- `global_dir/sessions/subagents/<hubId>.jsonl` 存在且第二条 user 消息已追加
- `global_dir/worktrees/subagents/<hubId>` 在 continue 间保留
