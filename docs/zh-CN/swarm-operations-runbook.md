# 集群操作手册（Swarm Operations Runbook）

启动、监控、限流、恢复与交接大规模 Pi 智能体集群的实用工作流。

本手册为操作员指引。它不替代 Beads 作为工作账本、Agent Mail 作为预约/消息账本、`pi doctor` 作为实时诊断界面，或发布证据收口闸门作为声明权威。

## 真实来源（Source Of Truth）

| 界面 | 权威 | 命令或制品 |
|---------|-----------|---------------------|
| 工作归属 | Beads issue 状态与评论 | `br ready --json`, `br show <id>`, `br update <id> --claim --actor "$AGENT_NAME"` |
| 跨智能体协同 | Agent Mail 消息、预约与构建槽位 | MCP Agent Mail `macro_start_session`, `file_reservation_paths`, `fetch_inbox` |
| 实时集群就绪度 | Doctor 集群诊断 | `pi doctor --only swarm --format json` |
| Cargo/RCH 准入 | Cargo 余量预检 | `scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets` |
| 远程构建状态 | RCH 队列与工作器状态 | `rch status`, `rch queue`, `rch doctor` |
| 交接包 | 操作员 runpack | `python3 scripts/build_swarm_operator_runpack.py --capture-current ...` |
| 进度态势 | 只读进度 SLO 报告 | `pi swarm-progress --input <progress-slo-input.json> --out-json <progress-slo.json>` |
| 队列收敛 | 只读空队列收敛报告 | `python3 scripts/report_empty_queue_convergence.py --json` |
| 试运行自愈指引 | Runpack 行动计划与工作准入门禁 | `python3 scripts/build_swarm_operator_runpack.py --out-action-plan-json ... --out-work-admission-gate-json ...` |
| 证据续期态势 | 陈旧证据续期队列 | `python3 scripts/build_stale_evidence_renewal_queue.py --out-json ...` |
| 饱和度与时间线证据 | 脱敏后的集群活动账本 | `docs/swarm-activity-ledger.md`, schema `pi.swarm.activity_digest.v1` |
| 确定性回放证据 | 集群飞行记录仪 | `docs/swarm-flight-recorder.md`, schema `pi.swarm.flight_recorder.report.v1` |
| 离线回放策略对比 | 集群回放操作员工作流 | `docs/swarm-replay-operator-workflow.md`, `pi swarm-replay-preview --trace <trace.json>` |

## 启动检查清单（Startup Checklist）

在多智能体会话中认领工作前执行下列检查：

```bash
export AGENT_NAME="${AGENT_NAME:-$(whoami)}"
export PI_CARGO_AGENT_SUFFIX="$AGENT_NAME"
export CARGO_TARGET_DIR="/data/tmp/pi_agent_rust_cargo/${AGENT_NAME}/target"
export TMPDIR="/data/tmp/pi_agent_rust_cargo/${AGENT_NAME}/tmp"
capture_dir="${PI_SWARM_CAPTURE_DIR:-/data/tmp/pi_swarm_runpack/${AGENT_NAME}}"
mkdir -p "$CARGO_TARGET_DIR" "$TMPDIR" "$capture_dir"

git status --short --branch
br ready --json
bv --recipe actionable --robot-plan
if curl -fsS http://127.0.0.1:8765/health > "$capture_dir/agent-mail-health.json"; then
  agent_mail_arg=(--agent-mail-health-json "$capture_dir/agent-mail-health.json")
else
  agent_mail_arg=()
fi
python3 scripts/report_empty_queue_convergence.py --json \
  --beads-jsonl .beads/issues.jsonl \
  "${agent_mail_arg[@]}"
# When available, add:
#   --validation-broker-json <validation-broker-status-or-plan.json>
pi doctor --only swarm --format json > "$capture_dir/doctor.json"
scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets \
  --decision-json "$capture_dir/cargo-admission.json"
rch status
rch queue
```

绿色启动意味着：

- `git status --short --branch` 没有来自本智能体的未提交改动。
- `br ready --json` 存在真实的待办 issue，而非墓碑或已删除项。
- `scripts/report_empty_queue_convergence.py --json` 在认领新 Bead 前报告
  `status=ready_work_available`，或当仅剩延期路线图/规划史诗且应创建新的/细化后的子 Beads 时报告 `status=work_to_plan`，或仅当无待办/进行中工作剩余且无延期规划史诗仍需子待办时报告 `status=queue_clean`。
- 若提供了 `--validation-broker-json`，陈旧槽位、饱和槽位态势、格式错误的 JSON 与重复的高开销 cargo 门禁机会将作为建议性操作员上下文出现。提供的 broker JSON 格式错误时将以警告闭环失败；缺失 broker JSON 时仍为可选。
- 若提供了 Agent Mail 健康 JSON，降级或模式损坏的 Agent Mail 将作为显式的 `use_beads_fallback` 动作出现并附带精确的恢复动作，同时待办 Beads 工作仍保持可见。若健康捕获失败，保留报告中的 `agent_mail_status=unavailable` 警告并以 Beads 认领作为软锁。
- `pi doctor --only swarm --format json` 没有表示必须停止新集群工作的红色发现。
- `scripts/cargo_headroom.sh --runner rch --admit-only ...` 返回
  `decision=allow` 且 `admission_action=allow`。`admission_action=defer` 表示门禁必须等待，`admission_action=fallback` 表示该命令仅因显式允许回退才会在本地运行。
- `rch queue` 未显示会使更多 cargo 工作变得不负责任的饱和或陈旧的重型构建。

若任一检查处于降级，保留原始命令输出并从下方的恢复表中选择响应。不要将降级的协同或 RCH 状态转化为含糊的“测试失败”备注。

## 认领 Bead（Claim A Bead）

使用 `bv` 进行优先级排序，使用 `br` 执行实际认领：

```bash
bv --recipe actionable --robot-plan
br ready --json
br show <issue-id>
br update <issue-id> --claim --actor "$AGENT_NAME"
br comments add <issue-id> --author "$AGENT_NAME" --message \
  "Claimed by $AGENT_NAME. Scope: <files/modules>. Validation: <commands>. Coordination: <Agent Mail status>."
```

编辑前，在 Agent Mail 中预约最窄且实用的文件集合：

```text
file_reservation_paths(
  project_key="/data/projects/pi_agent_rust",
  agent_name="$AGENT_NAME",
  paths=["src/module.rs", "tests/module_tests.rs"],
  ttl_seconds=3600,
  exclusive=true,
  reason="<issue-id>"
)
```

若因 MCP 数据库不可用或损坏导致 Agent Mail 写入失败，在 Beads 评论中记录失败并以 Beads 认领作为软锁继续。存在可用的非重叠工作时，不要在仅协同的循环中等待。

## Cargo 与测试策略（Cargo And Test Policy）

CPU 密集型 Rust 命令必须经由 RCH：

```bash
env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" TMPDIR="$TMPDIR" \
  rch exec -- cargo check --all-targets

env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" TMPDIR="$TMPDIR" \
  rch exec -- cargo clippy --all-targets -- -D warnings

env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" TMPDIR="$TMPDIR" \
  rch exec -- cargo test <focused-filter> -- --nocapture
```

仅对非重型检查使用本地命令：

```bash
cargo fmt --check
git diff --check
timeout 60s ubs --staged --only=rust .
python3 scripts/check_ubs_staged_delta.py
./scripts/reconcile_beads_ledger.sh
```

若 `timeout 60s ubs --staged --only=rust .` 超时或被全文件基线噪声主导，运行 `python3 scripts/check_ubs_staged_delta.py`。仅当增量门禁在已暂存变更行上报告无警告或严重发现时，增量门禁才视为通过。在交接中保留原始超时或基线噪声摘要。

若本地 pre-commit 钩子看起来在执行全仓库扫描而非已暂存 UBS 契约，运行
`python3 scripts/check_ubs_staged_delta.py --check-pre-commit-hook --json`。
该审计为只读：它报告 `.git/hooks/pre-commit` 漂移而不编辑钩子。

## 远程验证证明账本（Remote Validation Proof Ledger）

远程验证证明由
`docs/contracts/remote-validation-proof-ledger-contract.json` 治理，账本 schema 为
`pi.remote_validation.proof_ledger.v1`。该账本仅为操作员证据；
它不是发布性能证据、基准支持、严格 drop-in 认证证据，也不是 RCH、`cargo_headroom.sh`、CI、UBS、Beads、Agent Mail 或声明完整性门禁的替代品。

每个证明条目必须标识命令、命令类别、运行器要求、已解析运行器、RCH 工作器/任务、远程/本地回退状态、起止时间戳、退出状态、`CARGO_TARGET_DIR`、`TMPDIR`、远程 target/tmp 路径、制品取回状态、已变更与已覆盖路径、警告以及最终证据分类。Cargo、脚本自检、已暂存 UBS 与 Beads 账本对账命令被归一化为同一账本形态。

解读规则：

- `clean_remote_proof=true` 要求 RCH 远程运行、命令成功退出、无本地回退，且制品取回干净或不适用。
- `local_fallback=observed` 永远不是远程证明。
- `local_fallback=refused` 是 RCH 必需门禁的正确闭环阻断，不是放行。
- 队列退避必须记录为被阻断的证明条目，而非转化为跳过的绿色门禁。
- 制品取回警告必须在交接中保持可见。远程命令可能退出码为 0 但制品取回已降级；这不是干净的远程证明。
- `authoritative_for_bead=true` 要求一个通过的证明其覆盖路径包含所有已声明变更路径。显式声明权威但遗漏已声明路径的证明必须以 `proof_claim_coverage_mismatch` 闭环失败。
- 无关 worktree 阻断与 RCH 工作器工作区影子（workspace-shadow）失败必须表示为被阻断的证明条目，而非源码回归。

当证明账本嵌入于操作员 runpack 中时，在关闭任何 RCH 必需的 Bead 前检查这些字段：

```bash
jq '.remote_validation_proof_ledger.summary' runpack.json
jq '.remote_validation_proof_ledger.entries[] | {
  command: .command.rendered,
  class: .command_class,
  resolved: .runner.resolved_runner,
  remote: .runner.remote_execution,
  local_fallback: .runner.local_fallback,
  artifacts: .artifact_retrieval.status,
  coverage: .evidence_classification.coverage.coverage_status,
  authoritative: .evidence_classification.coverage.authoritative_for_bead,
  clean: .evidence_classification.clean_remote_proof,
  status: .evidence_classification.status,
  warnings: [.warnings[].warning_id]
}' runpack.json
```

黄金示例位于
`tests/golden_corpus/remote_validation_proof_ledger/examples.json`，涵盖干净远程通过、本地回退拒绝、队列退避与制品取回警告。

### 远程验证证明复用门禁（Remote Validation Proof Reuse Gate）

证明复用门禁由
`docs/contracts/remote-validation-proof-reuse-gate-contract.json` 治理，并发出
`pi.validation.proof_reuse_gate.v1`。它是只读准入辅助，用于判断既有远程验证证明是否可覆盖当前精确的命令、git head、已暂存路径、运行器要求、`CARGO_TARGET_DIR` 与 `TMPDIR` 上下文。

仅将其用作闭环预检：

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --run-proof-reuse-gate \
  --proof-ledger-json runpack-proof-ledger.json \
  --proof-reuse-context-json current-proof-context.json \
  --print-proof-reuse-gate
```

`reuse_allowed=true` 表示所选证明匹配了所有必需上下文字段并覆盖了所有当前变更路径。`reuse_allowed=false` 表示需通过 RCH 重新运行验证。任何陈旧的 git head、脏 worktree 不匹配、已暂存路径覆盖缺口、缺失 RCH 溯源、命令指纹不匹配、target/tmp 漂移或当前 `Cargo.lock` / `rust-toolchain.toml` 变更都会使复用失效。

该门禁本身永远不会跳过验证，也不会改变 Beads、git、Agent Mail、RCH 工作器、源文件或临时制品。

### 验证证明记忆索引（Validation Proof-Memory Index）

验证证明记忆索引由
`docs/contracts/validation-proof-memory-index-contract.json` 治理，并发出
`pi.validation.proof_memory_index.v1`。它是对已核验远程验证证明夹具与证明复用决策的只读索引，覆盖当前命令、git head、已暂存路径、RCH 溯源、`CARGO_TARGET_DIR`、`TMPDIR` 与制品取回上下文。

将其用作操作员审计制品，而非验证跳过器：

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --run-validation-proof-memory-index \
  --print-validation-proof-memory-index
```

当前夹具制品为
`docs/evidence/validation-proof-memory-index.json`。它必须包含一个可复用远程证明，以及针对陈旧 git head、陈旧源码时间、缺失制品、本地回退、脏 worktree 不匹配、命令指纹不匹配、路径覆盖不匹配、非权威覆盖与失败的收口/runpack 新鲜度输入的闭环夹具。任何不可复用类别均表示需在收口前通过相应门禁重新运行或刷新验证。

该索引永远不会改变 RCH、Agent Mail、Beads、git、源文件、临时制品或运行时调度策略。它不授权发布性能、基准、容量或严格 drop-in 声明。

### 操作员工作推荐器（Operator Work Recommender）

操作员工作推荐器由
`docs/contracts/operator-work-recommendation-contract.json` 治理，并发出
`pi.swarm.operator_work_recommendation.v1`。它消费事件回放与验证证明记忆制品，然后对健康待办 Beads、无待办工作、Agent Mail 损坏、RCH 饱和、陈旧证明刷新、重复工作风险与脏 worktree 准入拒绝等场景的建议性下一步工作决策进行排序。

在认领工作前用它检查下一个安全的操作员姿态：

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --run-operator-work-recommendation \
  --print-operator-work-recommendation
```

当前夹具制品为
`docs/evidence/operator-work-recommendation.json`。每条推荐都引用精确的证据路径、列出被拒绝的不安全替代项、给出置信度分数并包含面向操作员的解释。缺失、陈旧、矛盾、未脱敏或权威混淆的源证据将闭环失败为 `refresh_or_surface_operator_blocker`。

该推荐器为只读。它从不认领 Beads、写入 Agent Mail 预约、启动 RCH、运行 cargo、改变 git、删除文件或替代源系统。操作员仍需通过常规的 Beads、Agent Mail、RCH、git 与验证工作流执行任何选定动作。

### 操作员平滑度 SLO（Operator Smoothness SLO）

操作员平滑度 SLO 由
`docs/contracts/operator-smoothness-slo-contract.json` 治理，并发出
`pi.operator.smoothness_slo.v1`。它使用针对提供方流式增量、RPC 输出压力、TUI 帧渲染、工具更新合并与会话写入压力的确定性大流量夹具。

用它在合成集群输出压力下检查语义可见性：

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --run-operator-smoothness-slo \
  --print-operator-smoothness-slo
```

当前夹具制品为 `docs/evidence/operator-smoothness-slo.json`。
每个用例均包含带 p50/p95/p99 可见性计数器、语义里程碑计数、低价值合并计数、积压预算与失败日志的界面指标。针对语义可见性延迟、非单调时间线、失控帧积压与缺失界面覆盖的负向对照将闭环失败。计数器仅为工程夹具证据；它们不授权基准、容量、发布性能、严格 drop-in 或运行时变更声明。

### 扩展资源防火墙矩阵（Extension Resource Firewall Matrix）

扩展资源防火墙矩阵由
`docs/contracts/extension-resource-firewall-matrix-contract.json` 治理，并从确定性扩展压力夹具发出
`pi.ext.resource_firewall_matrix.v1`。它覆盖廉价读取洪泛、大负载发射、被拒能力抖动、慢速宿主调用、重复失败与稳定对等端进展。

使用聚焦的压力测试切片生成目标/perf 证据：

```bash
export CARGO_TARGET_DIR="/data/tmp/pi_agent_rust_cargo/${USER:-agent}/target"
export TMPDIR="/data/tmp/pi_agent_rust_cargo/${USER:-agent}/tmp"
mkdir -p "$CARGO_TARGET_DIR" "$TMPDIR"
rch exec -- env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" TMPDIR="$TMPDIR" cargo test --test extensions_stress resource_firewall_matrix -- --nocapture
```

该测试在已配置的 `CARGO_TARGET_DIR` 的 `perf/` 目录下写入 `resource_firewall_matrix.json`。每行均包含资源类别、扩展角色、宿主调用类别、预算、观测单位、准入决策、拒绝模式、回退行为、负载脱敏状态、能力边界状态、对等端进展保留与操作员可见计数器。针对缺失计数器、缺失对等端进展与未脱敏负载体的负向对照将闭环失败。该矩阵扩展宿主调用成本归因证据；它不替代运行时强制、能力策略、RCH 验证、Agent Mail、Beads、UBS、CI 或基准/容量/发布声明。

## 临时制品清单（Temp Artifact Inventory）

集群 runpack 包含 schema 为 `pi.swarm.temp_artifact_inventory.v1` 的 `temp_artifact_inventory`。这是通过 cargo 准入、RCH 证明条目、冒烟测试制品、验证输出捕获与捕获清单临时制品观测到的暂存与证据路径的只读清单。

该清单永远不会执行清理也不会发出删除命令。每条目均记录删除策略：

- `retain_active`：必须保留的已拥有或活跃路径。
- `requires_explicit_operator_approval`：仍需书面批准方可删除的已知归属陈旧候选项。
- `deletion_protected_unknown_owner`：未知归属路径，始终受保护。

操作员可使用如 `stat` 与 `du -sh` 等发出的检查命令来检查压力，但删除文件或目录仍需在 runpack 之外获得显式书面许可。

## 监控活跃集群（Monitor An Active Swarm）

在工作进行中使用此状态循环：

```bash
git status --short --branch
br list --status=in_progress --json
br ready --json
rch status
rch queue
pi doctor --only swarm --format json
```

关注：

- 多个智能体在无 Agent Mail 预约或 Beads 评论的情况下编辑同一文件。
- `br list --status=in_progress --json` 中 `updated_at` 时间戳陈旧且无近期评论的条目。
- `rch queue` 中进度停滞、制品取回反复失败或槽位压力的条目。
- `pi doctor --only swarm --format json` 中关于 Agent Mail 构建槽位、预约冲突、cgroup 内存压力、target/TMPDIR 余量或 RCH 分类器失败的发现。
- 你已认领文件集之外的脏 worktree 条目。

不要回退无关的脏文件。将其视为另一智能体的工作，除非所属 Bead 或用户另有明确说明。

## 进度 SLO 操作员工作流（Progress SLO Operator Workflow）

`pi swarm-progress` 从归一化的 `ProgressSloEvaluationInput` 快照对集群是否取得进展进行分类。它是只读建议性证据。它不读取实时 Beads、不发送 Agent Mail、不预约文件、不启动或取消 RCH 任务、不改变 git、不关闭 Beads、不豁免验证门禁，也不支撑面向发布的速度、容量、基准或严格 drop-in 声明。

先捕获源事实，再从这些制品构建或复用归一化输入：

```bash
capture_dir="/data/tmp/pi_swarm_progress/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$capture_dir"

br list --json > "$capture_dir/beads.json"
br ready --json > "$capture_dir/beads-ready.json"
br list --status=in_progress --json > "$capture_dir/beads-in-progress.json"
git status --short --branch > "$capture_dir/git-status.txt"
rch status > "$capture_dir/rch-status.txt"
rch queue > "$capture_dir/rch-queue.txt"
PI_SWARM_PROGRESS_SLO_JSON="$capture_dir/progress-slo.json" \
  pi doctor --only swarm --format json > "$capture_dir/doctor-swarm.json"
```

评估已准备好的归一化输入并保留机器与人类可读制品：

```bash
pi swarm-progress \
  --input "$capture_dir/progress-slo-input.json" \
  --since HEAD~1 \
  --out-json "$capture_dir/progress-slo.json" \
  --out-text "$capture_dir/progress-slo.txt"
```

`--since` 为可选，但提供时必须与 `input.time_window.comparison_baseline` 匹配。该命令拒绝覆盖已有输出文件；请使用全新捕获目录而非删除旧证据。

使用 `jq` 检查操作员通常需要的字段：

```bash
jq '{schema, status, confidence, reason_ids, next_actions}' \
  "$capture_dir/progress-slo.json"

jq '.saturation_summary | {
  coordination_saturation,
  build_saturation,
  validation_saturation,
  queue_convergence,
  recommended_operator_posture
}' "$capture_dir/progress-slo.json"

jq '.source_statuses[] | {
  id: .source_id,
  kind: .source_kind,
  availability,
  freshness_state,
  redaction_state,
  degraded_reason
}' "$capture_dir/progress-slo.json"

jq '.redaction_summary | {
  redacted_source_count,
  unsafe_to_emit_source_count,
  suppressed_claims
}' "$capture_dir/progress-slo.json"
```

保守地解读状态：

| 状态 | 典型原因 | 操作员动作 |
| --- | --- | --- |
| `progressing` | 窗口内已关闭 Beads、已推送提交与验证通过发生移动。 | 继续当前集群，但仍使用 Beads 与 Agent Mail 进行归属管理。 |
| `converged_no_open_work` | 无待办或进行中工作剩余。 | 停止认领新实现工作；仅为具体未覆盖缺口新建 Bead。 |
| `quiet_blocked` | 存在待办工作但待办工作被阻塞。 | 使用 `br show <id> --json` 检查依赖并解除命名源 issue 的阻塞。 |
| `coordination_degraded` | Agent Mail 为红、损坏、只读或缺失。 | 以 Beads 状态/评论作为软锁，保持文件范围狭窄，并记录确切的 Mail 错误。 |
| `build_saturated` | RCH 或验证代理压力较高。 | 停止启动重量级 Cargo 任务；继续文档、源码检查或非重型修复，直至 RCH 恢复。 |
| `stalled` | 进行中 Beads 看似陈旧且无可见有效进展。 | 在重新打开前审查 `br show`、评论、git 历史与 Agent Mail 证据；切勿仅凭时间判断重新打开。 |
| `malformed_source_degraded` | 所需源格式错误或矛盾。 | 修复或重新生成源制品；不要对报告的乐观部分采取行动。 |
| `insufficient_evidence_degraded` | 所需源数据缺失、陈旧或不安全无法发出。 | 刷新源制品并重新运行；将报告视为阻断而非放行。 |

当报告应出现在 Doctor 或操作员 runpack 中时，显式传入 JSON：

```bash
PI_SWARM_PROGRESS_SLO_JSON="$capture_dir/progress-slo.json" \
  pi doctor --only swarm --format json \
  | jq '.findings[] | select(.id == "progress_slo_current_posture")'

python3 scripts/build_swarm_operator_runpack.py \
  --capture-current \
  --capture-dir "$capture_dir/runpack" \
  --project-root /data/projects/pi_agent_rust \
  --agent-name "${AGENT_NAME:-agent}" \
  --progress-slo-json "$capture_dir/progress-slo.json" \
  --out-json "$capture_dir/operator-runpack.json" \
  --out-md "$capture_dir/operator-runpack.md"
```

隐私边界：

- 存储 bead id、源 id、schema 名称、计数、命令标签、退出状态、文件路径、源哈希与脱敏摘要。
- 不要嵌入提示体、提供方转录、原始 Agent Mail 消息体、bearer token、cookie、API 密钥、机密或完整环境转储。
- 若某源报告 `redacted`、`sensitive_omitted` 或 `unsafe_to_emit`，保持被抑制声明可见，避免将缺失原始数据视为绿色证据。
- 进度 SLO 报告仅对其源窗口与源哈希有效。为新交接重建报告，而非沿用陈旧状态。

## 第四波自愈工作流（Fourth-Wave Self-Healing Workflow）

第四波自愈制品帮助操作员在集群嘈杂时选择下一个安全动作。它们仅为试运行指引。它们不认领 Beads、不预约文件、不终止进程、不隔离扩展、不重新生成证据、不覆盖输出、不启动或取消 RCH 工作、不推送提交，也不授权严格 drop-in 发布措辞。

按此顺序使用工作流：

1. 捕获源事实：Beads、git 状态、Doctor 集群输出、RCH 状态、cargo 余量、可用时的验证代理状态，以及 runpack 将汇总的任何源证据。
2. 构建试运行诊断：陈旧证据续期队列、可选预算租约模拟、可选扩展隔离演练、runpack、自动驾驶输入包、自动驾驶计划、行动计划与工作准入门禁。
3. 在开始新工作前读取 `work-admission-gate.json`。若其表示 `wait`、`renew_evidence`、`pause_escalate` 或任何非准入决策，停止准入新实现智能体，直至人工操作员执行命名安全命令或显式记录覆盖。
4. 当计划推荐变更现实世界的动作时，将命令作为提议命令复制到交接中，而非已执行动作。在变更 Beads 归属、Agent Mail 预约、扩展配置、git 引用或证据文件前需显式人工确认。

仅在空捕获目录下写入新文件的示例捕获：

```bash
capture_dir="/data/tmp/pi_swarm_fourth_wave/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$capture_dir"

br ready --json > "$capture_dir/beads-ready.json"
br list --status=in_progress --json > "$capture_dir/beads-in-progress.json"
git status --short --branch > "$capture_dir/git-status.txt"
pi doctor --only swarm --format json > "$capture_dir/doctor-swarm.json"
scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets \
  --decision-json "$capture_dir/cargo-admission.json"
rch status > "$capture_dir/rch-status.txt"
rch queue > "$capture_dir/rch-queue.txt"

python3 scripts/build_stale_evidence_renewal_queue.py \
  --source-root /data/projects/pi_agent_rust \
  --freshness-hours 336 \
  --max-items 25 \
  --out-json "$capture_dir/stale-evidence-renewal.json"

python3 scripts/build_swarm_operator_runpack.py \
  --capture-current \
  --capture-dir "$capture_dir/runpack-sources" \
  --project-root /data/projects/pi_agent_rust \
  --agent-name "$AGENT_NAME" \
  --stale-evidence-renewal-json "$capture_dir/stale-evidence-renewal.json" \
  --out-json "$capture_dir/operator-runpack.json" \
  --out-md "$capture_dir/operator-runpack.md" \
  --out-autopilot-input-pack-json "$capture_dir/autopilot-input-pack.json" \
  --out-autopilot-plan-json "$capture_dir/autopilot-plan.json" \
  --out-action-plan-json "$capture_dir/action-plan.json" \
  --out-work-admission-gate-json "$capture_dir/work-admission-gate.json"
```

可选演练保持试运行，应与 runpack 一并捕获：

```bash
python3 scripts/simulate_swarm_budget_leases.py \
  --fixture-id rch_saturation \
  --out-json "$capture_dir/budget-lease-simulation.json"

python3 scripts/rehearse_extension_quarantine.py \
  --fixture-id startup_crash_loop_quarantine \
  --out-json "$capture_dir/extension-quarantine-rehearsal.json"
```

保守地解读第四波输出：

| 输出 | 操作员用途 | 边界 |
| --- | --- | --- |
| `action-plan.json` | 从已捕获源对下一个最安全操作员动作进行排序。 | 仅为建议；命令需操作员执行。 |
| `work-admission-gate.json` | 决定是否准入新实现工作、续期证据、等待或暂停；其 `dry_run_executor` 将计划项分类为 `would_execute`、`blocked`、`requires_operator` 或 `never_execute`。 | 仅为闭环门禁；它不强制运行时限流也不改变 Beads、Agent Mail、RCH、git 或文件。 |
| runpack 中的 `turn_pressure_ledger` | 显示提示、工具、提供方、TUI 与会话写入压力，不含原始负载体。 | 仅为诊断；非基准或发布证据。 |
| `budget-lease-simulation.json` | 在饱和下推荐公平的按智能体预算分配与降低扇出。 | 不预约容量也不改变 Agent Mail、Beads、RCH 或进程。 |
| `extension-quarantine-rehearsal.json` | 从夹具或已捕获扩展事实演练隔离或回滚决策。 | 本身不编辑扩展配置或隔离任何内容。 |
| `stale-evidence-renewal.json` | 列出陈旧、缺失、契约漂移或 RCH 阻断证据及有界续期命令。 | 不重新生成或覆盖证据，也不弱化 drop-in 声明门禁。 |
| 交接摘要 | 向下一操作员提供脱敏源状态、所选建议动作与被阻断/降级原因。 | 它们不是真实来源证据，也不替代 Beads、Agent Mail、Doctor、RCH、CI、UBS 或发布门禁。 |

试运行执行器是准入证明，而非执行器。它可能将只读探针标记为 `would_execute`，但 Beads 归属变更、制品写入与其他变更命令仍为 `requires_operator`。Agent Mail 变更、RCH 执行或变更、本地重量级 Cargo、删除请求与 Beads 归属绕过为带稳定原因码的 `never_execute` 条目。

声明边界不变：通过 Beads 认领，健康时通过 Agent Mail 预约，Mail 损坏或只读时以 Beads 评论作为软锁。工作准入门禁可推荐 `use_beads_soft_lock`，但无法证明未预约文件是空闲的。操作员在触碰文件族前仍需检查 `br show`、近期评论、`git log -- <file>` 与脏 worktree。

当以下任一为真时，停止准入新实现工作：

- `work-admission-gate.json` 包含 `admit_new_implementation=false`。
- `action-plan.json` 选择 `renew_stale_evidence`、`wait_for_pressure` 或 `pause_or_surface_blocker`。
- `stale-evidence-renewal.json` 列出认领所需但被阻断或需续期的条目。
- 扩展演练推荐隔离或回滚且无人工操作员批准实际配置变更。
- 预算租约模拟报告下一个智能体或重型验证命令所需资源类别的饱和。

安全交接措辞：

```text
Fourth-wave artifacts are advisory dry-run outputs. Next recommended action:
<decision from action-plan/work-admission-gate>. Proposed commands require
explicit operator execution. No Beads, Agent Mail, extension config, git refs,
evidence files, RCH jobs, or release claims were mutated by these artifacts.
```

第四波收口闸门发出
`pi.swarm.fourth_wave_self_healing.closeout_gate.v1`，由
`docs/contracts/fourth-wave-self-healing-closeout-gate-contract.json` 治理；当前制品为
`docs/evidence/fourth-wave-self-healing-closeout-gate.json`。该门禁在路线图可关闭前将每个 `bd-63x3v.7` 子 Bead 映射到源路径、docs/contracts/evidence、验证命令、已推送引用与建议性声明边界。

## 限流或暂停（Throttle Or Pause）

当以下任一为真时，退避新认领：

| 信号 | 命令 | 动作 |
|--------|---------|--------|
| RCH 准入拒绝或退避 | `scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets` | 停止启动重型 cargo 任务。继续文档、源码检查或小型非 cargo 修复。 |
| 本地 cargo/rustc 进程压力较高 | `scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets` | 等待本地进程压力下降，或仅对显式批准的覆盖使用 `--force-admit`。 |
| 队列压力较高 | `rch queue` | 等待活跃任务完成后再启动更多 cargo。 |
| Agent Mail 预约冲突 | `pi doctor --only swarm --format json` 或 Agent Mail 预约响应 | 缩小文件集、选择不同 Bead，或与持有者协调。 |
| Beads 存在陈旧进行中工作 | `br list --status=in_progress --json` | 在陈旧 issue 上评论，核实无近期归属活动后，仅在明确废弃时重新打开。 |
| Drop-in 或发布证据陈旧 | `scripts/report_swarm_claim_readiness.py` | 不要做出面向发布的声明。提交或处理证据缺口。 |
| Worktree 在你范围外为脏 | `git status --short --branch` | 忽略无关变更并保持提交狭窄暂存。 |

## 停滞 Bead 恢复（Stalled Bead Recovery）

仅对明确废弃的工作使用：

```bash
br show <issue-id>
br comments list <issue-id>
git log --oneline --decorate --all -- <claimed-file>
br update <issue-id> --status open --assignee "" --actor "$AGENT_NAME"
br comments add <issue-id> --author "$AGENT_NAME" --message \
  "Reopened as stale: no recent owner activity found; no file changes reverted."
```

不要仅因 Agent Mail 降级就重新打开进行中 Bead。当前 Beads 评论、近期提交或活跃文件预约已足以证明另一智能体可能仍拥有它。

## 恢复演练（Recovery Drills）

### Agent Mail 降级（Agent Mail Degraded）

1. 运行 `pi doctor --only swarm --format json` 并保存发现。
2. 尝试 MCP 注册/读取路径：`macro_start_session` 或 `register_agent`，然后 `fetch_inbox` 或 `list_agents`。保留精确的健康错误，例如 `database schema missing required tables`。
3. 使用 `file_reservation_paths` 尝试一次狭窄预约写入。若因 Mail 为红、只读或模式损坏导致写入失败，不要在编码前要求 Agent Mail 预约。
4. 以 Beads 作为协同记录：

   ```bash
   br show <issue-id> --json
   br update <issue-id> --status in_progress --assignee "$AGENT_NAME"
   ```

5. 在狭窄文件集上继续工作，并在最终交接中提及降级的 Mail 状态。
6. 通过 Beads 与 git 收口：

   ```bash
   br close <issue-id> --reason "Completed with Agent Mail unavailable; Beads used as soft lock"
   br sync --flush-only
   git add .beads/ <changed-files>
   git commit -m "<summary>"
   git push origin main
   ```

最终交接措辞应包含：`Agent Mail unavailable: <exact error>; Beads assignee/status used as soft lock; reservations/messages were not trusted for this bead.`

### RCH 取回或磁盘压力（RCH Retrieval Or Disk Pressure）

1. 运行 `rch status` 与 `rch queue`。
2. 运行 `scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets`。
3. 若分类器指向本地 target/TMPDIR 余量，将 `CARGO_TARGET_DIR` 与 `TMPDIR` 移至 `/data/tmp/pi_agent_rust_cargo/$AGENT_NAME` 下。
4. 若远程命令失败，仅在原始 RCH 输出识别出该类别后才将其视为代码或远程构建失败。

### 脏 Worktree（Dirty Worktree）

1. 运行 `git status --short --branch`。
2. 仅暂存当前 Bead 的文件。
3. 不要使用 `git reset --hard`、`git clean`、`git checkout --` 或 `rm` 清理命令。
4. 若无关脏文件阻断命令，记录精确阻断并请求指引。

### 饱和评审循环（Saturated Review Loop）

1. 构建或读取 `docs/swarm-activity-ledger.md` 中描述的集群活动摘要。
2. 若饱和原因显示重复工作、陈旧引入、重复阻断或验证吞吐偏低，停止启动宽泛评审智能体。
3. 选择一个狭窄实现 Bead、聚焦的 `testing-*` 技能，或来自摘要推荐的具体跟进 Bead。

## 交接包（Handoff Bundle）

在结束集群轮班前捕获交接包：

```bash
capture_dir="/data/tmp/pi_swarm_runpack/${AGENT_NAME}-$(date +%Y%m%dT%H%M%S)"
mkdir -p "$capture_dir"

python3 scripts/plan_semantic_validation_route.py \
  --from-git \
  --source-bead <issue-id> \
  --pretty \
  --out "$capture_dir/semantic-route-plan.json"

python3 scripts/build_swarm_operator_runpack.py \
  --capture-current \
  --capture-dir "$capture_dir" \
  --project-root /data/projects/pi_agent_rust \
  --agent-name "$AGENT_NAME" \
  --semantic-route-plan-json "$capture_dir/semantic-route-plan.json" \
  --progress-slo-json "$capture_dir/progress-slo.json" \
  --out-json "$capture_dir/operator-runpack.json" \
  --out-md "$capture_dir/operator-runpack.md" \
  --out-predictive-telemetry-ledger-json "$capture_dir/predictive-telemetry-ledger.json" \
  --out-validation-scheduler-plan-json "$capture_dir/validation-scheduler-plan.json" \
  --out-autopilot-input-pack-json "$capture_dir/autopilot-input-pack.json" \
  --out-autopilot-plan-json "$capture_dir/autopilot-plan.json"
```

runpack 的 schema 由 `docs/contracts/swarm-operator-runpack-contract.json` 治理。runpack 是对既有证据的脱敏索引，不是发布性能声明，也不是源制品的替代品。
语义路由计划的 schema 为 `pi.validation.semantic_route_plan.v1`。使用 `scripts/plan_semantic_validation_route.py --from-git --source-bead <issue-id> --out "$capture_dir/semantic-route-plan.json"` 生成它，并通过 `--semantic-route-plan-json` 传入 runpack。该路由仅为建议：它汇总变更路径分桶、证明记忆/缓存热度、RCH 支持的命令模板、协同准入与合并顺序，但操作员仍必须通过 Beads 认领、健康时通过 Agent Mail 预约，并通过 RCH 运行重量级 Cargo 验证。它不得执行命令、启动 RCH、改变 Beads、改变 Agent Mail、改变 git、删除文件、跳过验证，或支撑发布、基准、容量、性能、严格 drop-in 或声明就绪断言。
预测遥测账本的 schema 由 `docs/contracts/predictive-swarm-telemetry-ledger-contract.json` 治理；已检入的建议性夹具证据位于 `docs/evidence/predictive-swarm-telemetry-ledger.json`。它仅从既有 runpack 信号对验证、协同、工作队列、轮次上下文、瓶颈源与证据新鲜度压力进行排序，且不得用作发布性能、容量、Agent Mail、RCH、调度器、Beads、git 或声明就绪权威。
验证调度器计划的 schema 由 `docs/contracts/validation-scheduler-plan-contract.json` 治理；已检入的建议性夹具证据位于 `docs/evidence/validation-scheduler-plan.json`。它从 runpack 的 git、预测遥测、RCH 准入、远程证明与 target 缓存信号对精确脚本与 RCH 支持的 cargo 命令字符串进行排序。它为只读：不执行 cargo、不预约工作器、不改变 Agent Mail 或 Beads、不删除临时制品，也不允许在 RCH 不可用时将重型 cargo 回退到本地执行。
自动驾驶输入包的 schema 由 `docs/contracts/swarm-autopilot-input-pack-contract.json` 治理。它为试运行规划器归一化源状态，但仍为建议性，永远不替代 Doctor、Beads、Agent Mail、RCH、git 或源制品本身。
自动驾驶计划的 schema 由 `docs/contracts/swarm-autopilot-plan-contract.json` 治理。它将输入包映射为有序试运行动作，如 `claim_ready_bead`、`wait_for_rch`、`adjust_swarm_budget`、`use_beads_soft_lock`、`reopen_stale_bead_candidate`、`run_docs_only_work`、`capture_handoff` 或 `stop_and_surface_blocker`。
当命令发出配套输入包与计划时，runpack 还包含 schema 为 `pi.swarm.autopilot_handoff.v1` 的 `autopilot_handoff`。该节命名输入包与计划的 schema、制品路径、所选建议动作与源溯源，以便新智能体可在不将 runpack 视为新真实来源的情况下检查单个交接包。
依赖交接包前，运行 `python3 scripts/check_swarm_runpack_freshness.py "$capture_dir/operator-runpack.json" --source-root /data/projects/pi_agent_rust`。新鲜度守卫为只读，当 runpack 或类收口证据引用缺失、占位符、哈希不匹配、更新或陈旧源制品时将闭环失败。
对于收口证据分流，run 在新鲜度审计存在后执行 `python3 scripts/check_closeout_gate_freshness.py --operator-summary markdown`。该摘要对当前制品、缺失契约、陈旧源、缺失提交、哈希漂移、README 漂移与格式错误源失败进行分组，然后对只读检查命令与仅 Beads 刷新归属指引进行排序。它仅为建议性操作员上下文；不替代新鲜度 JSON、Beads、Agent Mail、RCH、git、源制品、UBS 或声明完整性门禁。
该计划还包含针对待办 Beads 的 `work_partitions`。这些条目推荐预约 glob、需避免的可能碰撞面、备选文件族、置信度与降级注意事项。它们仅为诊断；操作员在健康时仍通过 Beads 认领并通过 Agent Mail 预约。
输入包与计划还携带 schema 为 `pi.swarm.budget_drift.v1` 的 `budget_drift` 证据。它将上次接受的集群资源预检配置与实时 cgroup、内存、暂存路径、RCH 队列与活跃归属观测进行比较。状态 `stable` 保持当前上限，`degraded` 在带迟滞的情况下推荐降低扇出，`deny_new_work` 推荐在实时信号恢复前不准入新智能体或重量级 RCH 验证。
该计划还包含针对常见操作阻断的 `failure_actions`。这些条目对 RCH 制品取回、本地 Cargo target/TMPDIR 压力、远程编译器失败、Agent Mail 模式/只读降级、Beads JSONL 漂移、陈旧 Beads 归属与未知操作失败使用稳定目录 ID。未知条目以脱敏原始摘录与安全检查命令闭环失败，而非猜测根因。
工作准入门禁包含 schema 为 `pi.swarm.work_admission_dry_run_executor.v1` 的 `dry_run_executor`。它消费自动驾驶计划加上 Beads/RCH/Agent Mail/git/余量信号，将只读探针分类为 `would_execute`，将真实来源变更路由至 `requires_operator`，以显式原因阻断不安全准入，并将删除请求、Agent Mail 变更、RCH 执行/变更、本地重量级 Cargo 与 Beads 归属绕过永久拒绝为 `never_execute`。
无模拟自动驾驶端到端 harness 发出 `pi.swarm.autopilot_e2e.v1` 加 `pi.swarm.autopilot_e2e.event.v1` JSONL 事件。它在安全处使用临时 Beads 与临时 git 工作区，在实时变更不安全处使用夹具捕获的降级 Agent Mail 与 RCH 输入，并验证健康认领、空队列、删除请求拒绝、Beads 软锁回退、饱和 RCH、陈旧 Bead 审查、无关脏 worktree 与格式错误源闭环场景。这仅为操作员准入证据；不是发布速度、drop-in 或基准声明。
最终收口闸门发出 `pi.swarm.autopilot_decision_gate.v1`，由 `docs/contracts/swarm-autopilot-decision-gate-contract.json` 治理。它将已交付输入包、规划器、工作分区、失败动作目录、预算漂移观察器、端到端/日志证据、runpack 交接、安全守卫、已推送提交与质量门与提示到制品清单进行比较。失败门禁发出 `follow_up_beads` 与 `decision=file_follow_up_beads_before_closing_epic`；通过的门禁仍仅为覆盖 Beads、git、RCH、Doctor、Agent Mail 与源制品的收口证据，而非新真实来源。
自适应执行收口闸门发出 `pi.swarm.adaptive_execution.closeout_gate.v1`，由 `docs/contracts/adaptive-execution-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/adaptive-execution-closeout-gate.json`。它仅为建议性收口证据，不替代 Beads、git、RCH、Agent Mail、CI、UBS、发布认证或源文件。
扩展兼容性收口闸门发出 `pi.ext.compatibility_closeout_gate.v1`，由 `docs/contracts/extension-compatibility-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/extension-compatibility-closeout-gate.json`。它仅为建议性收口证据，不替代扩展一致性运行、Beads、git、RCH、Agent Mail、CI、UBS、发布认证或源文件。
集群回放收口闸门发出 `pi.swarm.replay_closeout_gate.v1`，由 `docs/contracts/swarm-replay-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/swarm-replay-closeout-gate.json`。它仅为建议性收口证据，不替代回放夹具、Beads、git、RCH、Agent Mail、CI、UBS、发布认证或源文件。
上下文智能收口闸门发出 `pi.context_intelligence.closeout_gate.v1`，由 `docs/contracts/context-intelligence-closeout-gate-contract.json` 治理。它将每个 `bd-ircr3` 子 Bead 映射到代码、测试、文档或证据、命令、关闭原因与提交哈希；然后检查图契约、图构建器、新鲜度与声明门禁、打包规划器、脱敏与失效、预览界面、提示注入、无模拟端到端、性能预算、Doctor/runpack 态势、操作员文档、README 新鲜度、已推送提交、已暂存 UBS 与 Beads 账本对账。通过的上下文门禁仅为收口证据，不替代 Beads、git、RCH、Doctor、runpack 或源文件。
验证代理收口闸门发出 `pi.validation_broker.closeout_gate.v1`，由 `docs/contracts/validation-broker-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/validation-broker-closeout-gate.json`。它将每个 `bd-gusp4` 实现子 Bead 映射到代码、测试、文档或证据、命令、关闭原因与提交哈希；然后检查源边界契约、租约存储、源归一化、准入策略、CLI 租约流、故障语料、Doctor/runpack 投影、无模拟端到端覆盖、压力预算证据、操作员文档、README 新鲜度、已推送提交、已暂存 UBS 与 Beads 账本对账。通过的验证代理门禁仅为收口证据，不替代 Beads、git、RCH、Doctor、runpack、Agent Mail、CI、UBS、`cargo_headroom.sh` 或源文件。
进度 SLO 收口闸门发出 `pi.swarm.progress_slo.closeout_gate.v1`，由 `docs/contracts/swarm-progress-slo-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/swarm-progress-slo-closeout-gate.json`。它将每个 `bd-wzri8` 实现子 Bead 映射到代码、测试、文档或证据、命令、关闭原因与提交哈希；然后检查进度 SLO 契约、确定性评估器、只读 CLI、Doctor/runpack 投影、无模拟端到端证据、合成压力预算、操作员文档、README 新鲜度、已推送提交、已暂存 UBS、Beads 账本对账与源边界检查。通过的进度 SLO 门禁仅为收口证据，不替代 Beads、git、RCH、Doctor、runpack、Agent Mail、CI、UBS、声明完整性门禁或源文件。
运行时智能收口闸门发出 `pi.runtime_intelligence.closeout_gate.v1`，由 `docs/contracts/runtime-intelligence-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/runtime-intelligence-closeout-gate.json`。它将每个 `bd-h66tp` 实现子 Bead 映射到代码、测试、文档或证据、命令、关闭原因与提交哈希；然后检查压缩准入、工具输出制品、提供方路由、调度公平性、帧预算遥测、取消清理、扩展安全溯源、文档/证据、源边界、已推送引用、已暂存 UBS、Beads 账本对账与 RCH 支持的质量门。通过的运行时智能门禁仅为收口证据，不替代 Beads、git、RCH、Doctor、runpack、Agent Mail、CI、UBS、声明完整性门禁或源文件。
带证明集群测试织物收口闸门发出 `pi.swarm.proof_carrying_test_fabric.closeout_gate.v1`，由 `docs/contracts/proof-carrying-swarm-test-fabric-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/proof-carrying-swarm-test-fabric-closeout-gate.json`。它将每个 `bd-zeccr` 实现子 Bead 映射到源路径、测试或夹具、证据制品、验证命令、关闭原因、已推送提交与负向对照；然后检查无模拟生命周期端到端、跨界面一致性、操作员证据黄金集、结构感知模糊/属性覆盖、蜕变回放等价、源边界、已推送引用、已暂存 UBS、Beads 账本对账与 RCH 支持的质量门。通过的带证明测试织物门禁仅为收口证据，不替代 Beads、git、RCH、Agent Mail、UBS、CI、声明完整性门禁、子证据或源文件。
预测性运营收口闸门发出 `pi.swarm.predictive_operations.closeout_gate.v1`，由 `docs/contracts/predictive-operations-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/predictive-operations-closeout-gate.json`。它将每个 `bd-63x3v.11` 实现子 Bead 映射到源路径、测试或夹具、证据制品、验证命令、关闭原因、已推送提交与声明边界文本；然后检查预测遥测融合、验证调度、语义压缩质量、宿主调用成本归因、操作员感知延迟、冗余智能体工作检测、源边界、已推送引用、已暂存 UBS、Beads 账本对账与未跟踪跟进。通过的预测性运营门禁仅为收口证据，不替代 Beads、git、RCH、Agent Mail、UBS、CI、声明完整性门禁、子证据、已生成 target/perf 输出或源文件。
第九波事件回放与证明记忆收口闸门发出 `pi.swarm.incident_replay_proof_memory.closeout_gate.v1`，由 `docs/contracts/ninth-wave-incident-replay-proof-memory-closeout-gate-contract.json` 治理；当前制品为 `docs/evidence/ninth-wave-incident-replay-proof-memory-closeout-gate.json`。它将每个 `bd-9yq7i` 子 Bead 映射到源路径、测试或夹具、证据制品、验证命令、关闭原因、已推送提交、负向对照与声明边界文本；然后检查事件语料、事件回放、验证证明记忆、操作员工作推荐、操作员平滑度 SLO、扩展资源防火墙矩阵、事件回放端到端、源边界、已推送引用、已暂存 UBS、Beads 账本对账与未跟踪跟进。通过的第九波门禁仅为收口证据，不替代 Beads、git、RCH、Agent Mail、UBS、CI、声明完整性门禁、子证据、已生成 target/perf 输出、先前波次证据或源文件。
操作员感知延迟追踪发出 `pi.operator.perceived_latency_trace.v1`，由 `docs/contracts/operator-perceived-latency-trace-contract.json` 治理；当前夹具制品为 `docs/evidence/operator-perceived-latency-trace.json`。它联接提供方流、RPC 输出、TUI 帧、工具更新与操作员可见语义里程碑，同时证明低价值合并不会隐藏语义输出。该追踪仅为建议性夹具证据，不替代提供方/RPC/TUI 背压证据也不授权基准、容量、发布性能或严格 drop-in 声明。
操作员平滑度 SLO 发出 `pi.operator.smoothness_slo.v1`，由 `docs/contracts/operator-smoothness-slo-contract.json` 治理；当前夹具制品为 `docs/evidence/operator-smoothness-slo.json`。它以确定性 p50/p95/p99 可见性计数器、语义里程碑计数、积压预算、失败日志与针对可见性延迟、非单调时间线、失控帧积压与缺失界面覆盖的闭环对照，覆盖提供方流式增量、RPC 输出压力、TUI 帧渲染、工具更新合并与会话写入压力。该 SLO 仅为建议性工程夹具证据，不替代聚焦界面测试也不授权基准、容量、发布性能、严格 drop-in、运行时变更、RCH、cargo、git 或 Beads 声明。
扩展资源防火墙矩阵发出 `pi.ext.resource_firewall_matrix.v1`，由 `docs/contracts/extension-resource-firewall-matrix-contract.json` 治理；聚焦 `extensions_stress` 运行在 target/perf 下写入 `resource_firewall_matrix.json`。它以预算、观测计数器、准入决策、拒绝模式、回退行为、负载脱敏、能力边界保留与针对缺失计数器、缺失对等端进展与未脱敏负载体的闭环负向对照，覆盖廉价读取洪泛、大负载发射、被拒能力抖动、慢速宿主调用、重复失败与稳定对等端进展行。该矩阵仅为建议性压力证据，不替代运行时强制、宿主调用成本归因、RCH 验证、Agent Mail、Beads、UBS、CI 或基准/容量/发布声明。
集群事件语料发出 `pi.swarm.incident_corpus.v1`，由 `docs/contracts/swarm-incident-corpus-contract.json` 治理；当前夹具制品为 `docs/evidence/swarm-incident-corpus.json`。它为 Agent Mail 模式损坏、RCH 饱和/本地回退拒绝、陈旧证据、重复工作风险、脏 worktree 准入拒绝、格式错误源制品与删除或实时变更拒绝等场景捕获确定性操作员事件，外加针对缺失源、不安全未脱敏体、矛盾状态与不安全授权尝试的闭环负向对照。该语料仅为建议性夹具证据，不替代发布性能、drop-in 认证、Agent Mail、RCH、Beads、git、源制品或破坏性动作权威。
集群事件回放 harness 发出 `pi.swarm.incident_replay.v1`，由 `docs/contracts/swarm-incident-replay-contract.json` 治理；当前夹具制品为 `docs/evidence/swarm-incident-replay.json`。它消费事件语料并以按步骤断言与脱敏摘录重构源捕获、Agent Mail 降级、RCH 准入、Beads 归属、脏 worktree 状态、验证结果与最终推荐阶段。负向对照对乱序事件、缺失源、未脱敏敏感内容以及将回放输出视为真实来源权威的情况闭环失败。回放仅为建议性夹具证据，不替代实时 Agent Mail、RCH、Beads、git、源制品或破坏性动作权威。
集群事件回放端到端 harness 发出 `pi.swarm.incident_replay_e2e.v1`，由 `docs/contracts/swarm-incident-replay-e2e-contract.json` 治理；当前夹具制品为 `docs/evidence/swarm-incident-replay-e2e.json`，JSONL 事件位于 `docs/evidence/swarm-incident-replay-e2e-events.jsonl`。它结合真实临时 Beads 与 git 工作区与夹具捕获的降级 Agent Mail/RCH 输入，以演练健康回放、Beads 软锁回退、RCH 证明刷新退避、重复工作风险、脏 worktree 拒绝、陈旧证明记忆刷新、扩展资源防火墙失败与平滑度 SLO 失败。该端到端制品仅为建议性操作员证据，不授权实时源变更、本地重量级 Cargo 回退、发布、基准、容量或 drop-in 声明。
验证证明记忆索引发出 `pi.validation.proof_memory_index.v1`，由 `docs/contracts/validation-proof-memory-index-contract.json` 治理；当前夹具制品为 `docs/evidence/validation-proof-memory-index.json`。它从已核验远程验证证明夹具对可复用、陈旧、缺失制品、本地回退、脏 worktree 不匹配、命令不匹配、路径覆盖不匹配与非权威验证证明条目进行分类。证明记忆仅为建议性夹具证据，不跳过验证也不替代 RCH、Agent Mail、Beads、git、源制品或声明完整性门禁。

### Validation Broker Operator Workflow（验证代理操作工作流）

验证代理是对高开销验证工作的建议性协同辅助。它帮助智能体决定是立即运行门禁、等待活跃槽位、复用等效证据、收窄命令，还是恢复陈旧槽位。它不认领 Beads、不预约文件、不调度 RCH 任务、不豁免 CI，也不将陈旧数据转化为绿色验证结果。
简言之：它不认领 Beads，不替代 RCH，也不跳过必需门禁。

仅在常规归属检查可见后使用代理：

1. 使用 `br ready --json`、`br show <id> --json` 与 `br list --status=in_progress --json` 检查 Beads 中的可执行工作与陈旧归属。
2. 当 Mail 数据库健康时通过 Agent Mail 预约文件。若 Mail 为红、只读或模式损坏，使用 Beads 受托人作为软锁并在 Bead 或交接中记录 Mail 阻断。
3. 在重量级门禁前运行 `pi doctor --only swarm --format json` 与 `scripts/cargo_headroom.sh --admit-only ...`，使暂存空间、cgroup、CPU、内存与 RCH 态势保持显式。
4. 在启动重复或宽泛验证命令前向代理请求计划。将结果视为建议，而非跳过必需门禁的许可。

典型只读状态捕获：

```bash
pi validation-broker status \
  --store "$PI_VALIDATION_BROKER_STORE" \
  --format json \
  --out-json "$capture_dir/validation-broker-status.json"
```

典型计划请求：

```bash
pi validation-broker plan \
  --request "$capture_dir/validation-request.json" \
  --inputs "$capture_dir/validation-inputs.json" \
  --store "$PI_VALIDATION_BROKER_STORE" \
  --policy "$capture_dir/validation-policy.json" \
  --format json \
  --out-json "$capture_dir/validation-broker-plan.json"
```

保守地解读决策：

| 决策 | 操作员动作 |
| --- | --- |
| `allow` | 通过已声明运行器运行所请求门禁并仍记录实际命令结果。 |
| `wait` | 不要启动重复重量级门禁；等待活跃归属者或请求更新。 |
| `coalesce` | 仅复用其命令、git head、target/TMPDIR、运行器、特性标志与哈希与请求匹配的命名制品。 |
| `narrow` | 将宽泛命令替换为代理要求的更窄动作，然后如实验证该更窄范围。 |
| `deny_local_fallback` | 不要让 RCH 必需命令开放式回退到本地构建。浮现 RCH 或余量阻断。 |
| `stale_recover` | 可见地标记陈旧槽位，在溯源不匹配后打开非重叠槽位或重新运行，且不终止进程。 |
| `degraded_block` | 停止并浮现缺失、陈旧、格式错误或不可用的源行。 |

获取、续期与释放仅改变追加式槽位存储：

```bash
pi validation-broker acquire \
  --request "$capture_dir/validation-request.json" \
  --store "$PI_VALIDATION_BROKER_STORE" \
  --started-at "$started_at_utc" \
  --expires-at "$expires_at_utc"

pi validation-broker renew \
  --store "$PI_VALIDATION_BROKER_STORE" \
  --slot-id "$slot_id" \
  --owner "$AGENT_NAME" \
  --heartbeat-at "$heartbeat_at_utc" \
  --expires-at "$expires_at_utc"

pi validation-broker release \
  --store "$PI_VALIDATION_BROKER_STORE" \
  --slot-id "$slot_id" \
  --owner "$AGENT_NAME" \
  --at "$released_at_utc" \
  --reason "gate completed and artifacts recorded"
```

代理的可复用证据路径为闭环。仅当代理命名槽位且其溯源与当前请求匹配时复用才有效。来自另一 git head、目标目录、TMPDIR、运行器、特性集、脏路径范围或制品哈希的相似命令是被拒绝的可复用槽位，而非放行。

隐私与脱敏边界：

- 代理状态、计划、runpack 与自动驾驶摘要应携带 schema ID、源可用性、源哈希、降级原因与有界摘录，而非原始提示体、邮箱令牌、命令日志或机密。
- 动态路径、PID、端口、时间戳、时长、长数字 ID 与十六进制 ID 在跨智能体比较阻断指纹前应归一化。
- Agent Mail 健康与预约事实仅为协同证据。若 Mail 不可用，不要推断无人拥有文件；回退到 Beads 与可见交接备注。
- 合成压力制品如 `docs/evidence/validation-broker-stress-budgets.json` 仅为工程预算证据。它们不是发布性能证据，不支撑 README 速度、容量或严格 drop-in 声明。

验证代理故障排查：

| 症状 | 操作员响应 |
| --- | --- |
| Agent Mail 模式损坏、为红或只读 | 以 Beads 受托人与可见交接备注作为软锁；不要推断缺席预约也不要在协同悬空等待。 |
| RCH 必需门禁将开放式回退到本地 | 将 `deny_local_fallback` 视为硬阻断，浮现 RCH 状态或队列证据，仅在远程执行可用时重新运行。 |
| 暂存空间、目标目录或 TMPDIR 余量偏低 | 运行文档化的 cargo 余量预检，在允许时切换到隔离的高容量 target/TMPDIR 对，且在余量显式前不启动宽泛门禁。 |
| 槽位存储缺失、格式错误或不可用 | 将代理态势视为降级，避免合并证据，并在交接中记录格式错误源路径或缺失制品。 |
| 可复用制品溯源不匹配 | 拒绝复用并为当前命令、git head、运行器、特性、目标目录、TMPDIR 与制品哈希运行所需门禁。 |

即使代理计划显示 `allow` 或 `coalesce`，代码变更后提交前这些命令仍为强制：

```bash
cargo fmt --check
git diff --check
rch exec -- cargo check --all-targets
rch exec -- cargo clippy --all-targets -- -D warnings
ubs --staged --only=rust .
./scripts/reconcile_beads_ledger.sh
```

关闭自动驾驶史诗时，收集实际命令结果并传给最终门禁：

```bash
final_gate_dir="/data/tmp/pi_swarm_autopilot_final_gate/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$final_gate_dir"

python3 scripts/build_swarm_operator_runpack.py \
  --run-autopilot-final-gate \
  --out-autopilot-final-gate-json "$final_gate_dir/summary.json" \
  --quality-gate-result "py_compile=pass:python3 -m py_compile scripts/build_swarm_operator_runpack.py" \
  --quality-gate-result "runpack_self_test=pass:python3 scripts/build_swarm_operator_runpack.py --self-test" \
  --quality-gate-result "autopilot_e2e=pass:python3 scripts/build_swarm_operator_runpack.py --run-autopilot-e2e" \
  --quality-gate-result "json_contracts=pass:python3 -m json.tool docs/contracts/swarm-autopilot-decision-gate-contract.json" \
  --quality-gate-result "cargo_fmt=pass:cargo fmt --check" \
  --quality-gate-result "cargo_check_all_targets_rch=pass:CARGO_TARGET_DIR=$CARGO_TARGET_DIR TMPDIR=$TMPDIR rch exec -- cargo check --all-targets" \
  --quality-gate-result "cargo_clippy_all_targets_rch=pass:CARGO_TARGET_DIR=$CARGO_TARGET_DIR TMPDIR=$TMPDIR rch exec -- cargo clippy --all-targets -- -D warnings" \
  --quality-gate-result "staged_ubs=pass:timeout 60s ubs --staged --only=rust ." \
  --quality-gate-result "beads_ledger_reconcile=pass:./scripts/reconcile_beads_ledger.sh"
```

关闭上下文智能史诗时，收集实际命令结果并传给最终门禁：

```bash
final_gate_dir="/data/tmp/pi_context_intelligence_final_gate/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$final_gate_dir"

python3 scripts/build_swarm_operator_runpack.py \
  --run-context-intelligence-final-gate \
  --out-context-intelligence-final-gate-json "$final_gate_dir/summary.json" \
  --quality-gate-result "py_compile=pass:python3 -m py_compile scripts/build_swarm_operator_runpack.py" \
  --quality-gate-result "runpack_self_test=pass:python3 scripts/build_swarm_operator_runpack.py --self-test" \
  --quality-gate-result "json_contracts=pass:python3 -m json.tool docs/contracts/context-intelligence-closeout-gate-contract.json" \
  --quality-gate-result "semantic_context_graph_contract_rch=pass:rch exec -- cargo test --test semantic_context_graph_contract -- --nocapture" \
  --quality-gate-result "semantic_workspace_graph_contract_rch=pass:rch exec -- cargo test --test semantic_workspace_graph_contract -- --nocapture" \
  --quality-gate-result "semantic_workspace_graph_builder_rch=pass:rch exec -- cargo test --test semantic_workspace_graph_builder context" \
  --quality-gate-result "context_intelligence_e2e_rch=pass:rch exec -- cargo test --test e2e_agent_loop context_intelligence_no_mock_harness -- --nocapture" \
  --quality-gate-result "doctor_context_intelligence_rch=pass:rch exec -- cargo test --test doctor_swarm_temp_dir_json context_intelligence -- --nocapture" \
  --quality-gate-result "context_perf_budgets_rch=pass:rch exec -- cargo test --test perf_budgets context_intelligence" \
  --quality-gate-result "context_intelligence_closeout_gate_contract_rch=pass:rch exec -- cargo test --test context_intelligence_closeout_gate_contract -- --nocapture" \
  --quality-gate-result "cargo_fmt=pass:cargo fmt --check" \
  --quality-gate-result "cargo_check_all_targets_rch=pass:CARGO_TARGET_DIR=$CARGO_TARGET_DIR TMPDIR=$TMPDIR rch exec -- cargo check --all-targets" \
  --quality-gate-result "cargo_clippy_all_targets_rch=pass:CARGO_TARGET_DIR=$CARGO_TARGET_DIR TMPDIR=$TMPDIR rch exec -- cargo clippy --all-targets -- -D warnings" \
  --quality-gate-result "staged_ubs=pass:timeout 60s ubs --staged --only=rust ." \
  --quality-gate-result "beads_ledger_reconcile=pass:./scripts/reconcile_beads_ledger.sh"
```

更多细节参见 `docs/swarm-operations-runbook.md#validation-broker-operator-workflow`。

## 完成检查清单（Completion Checklist）

关闭 Bead 前：

```bash
env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" TMPDIR="$TMPDIR" \
  rch exec -- cargo check --all-targets
env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" TMPDIR="$TMPDIR" \
  rch exec -- cargo clippy --all-targets -- -D warnings
cargo fmt --check
git diff --check
git add <changed-files> .beads/issues.jsonl
timeout 60s ubs --staged --only=rust .
python3 scripts/check_ubs_staged_delta.py
./scripts/reconcile_beads_ledger.sh
br close <issue-id> --reason "<completed evidence>"
br sync --flush-only
git add .beads/issues.jsonl
AGENT_NAME="$AGENT_NAME" git commit -m "<type>: <summary>"
git pull --rebase
git push
# Mirror the legacy compatibility branch per AGENTS.md after pushing main.
git status --short --branch
```

对于仅文档变更，使用面向文档的验证而非强制 cargo：

```bash
command -v git br bv rch cargo jq python3
python3 scripts/build_swarm_operator_runpack.py --self-test
python3 scripts/check_swarm_runpack_freshness.py --self-test
python3 scripts/check_swarm_runpack_freshness.py --run-runpack-smoke
python3 scripts/report_empty_queue_convergence.py --self-test
e2e_dir="/data/tmp/pi_swarm_autopilot_e2e/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
python3 scripts/build_swarm_operator_runpack.py \
  --run-autopilot-e2e \
  --capture-dir "$e2e_dir" \
  --out-autopilot-e2e-json "$e2e_dir/summary.json" \
  --out-autopilot-e2e-events-jsonl "$e2e_dir/events.jsonl"
python3 -m json.tool docs/contracts/swarm-operator-runpack-contract.json >/dev/null
python3 -m json.tool docs/contracts/validation-scheduler-plan-contract.json >/dev/null
python3 -m json.tool docs/contracts/swarm-autopilot-input-pack-contract.json >/dev/null
python3 -m json.tool docs/contracts/swarm-autopilot-plan-contract.json >/dev/null
python3 -m json.tool docs/contracts/swarm-autopilot-decision-gate-contract.json >/dev/null
cargo fmt --check
git diff --check
./scripts/reconcile_beads_ledger.sh
```

## 模式示例（Schema Examples）

Doctor 集群预检证据：

```json
{
  "schema": "pi.doctor.swarm_resource_preflight.v1",
  "status": "pass",
  "effective_cpu_cores": 64,
  "memory_limit_bytes": 274877906944,
  "recommended_budgets": {
    "agent_fanout": 8,
    "rch_verification_fanout": 4
  }
}
```

Cargo/RCH 准入证据：

```json
{
  "schema": "pi.cargo_headroom.admission.v1",
  "decision": "admit",
  "requested_runner": "rch",
  "resolved_runner": "rch",
  "cargo_command": "cargo check --all-targets",
  "rch_queue_forecast": {
    "schema": "pi.cargo_headroom.rch_queue_forecast.v1",
    "recommended_action": "proceed"
  }
}
```

操作员 runpack 证据：

```json
{
  "schema": "pi.swarm.operator_runpack.v1",
  "purpose": "operator_handoff_not_release_performance_claim",
  "status": "ready",
  "autopilot_handoff": {
    "schema": "pi.swarm.autopilot_handoff.v1",
    "status": "ready",
    "input_pack": {
      "schema": "pi.swarm.autopilot_input_pack.v1",
      "artifact_path": "/data/tmp/pi_swarm_runpack/<run>/autopilot-input-pack.json"
    },
    "plan": {
      "schema": "pi.swarm.autopilot_plan.v1",
      "selected_action": "claim_ready_bead",
      "artifact_path": "/data/tmp/pi_swarm_runpack/<run>/autopilot-plan.json"
    },
    "source_provenance": {
      "source_statuses": [
        {
          "id": "beads_ready",
          "status": "ok"
        }
      ],
      "command_count": 5
    }
  },
  "swarm_scale_safety_scorecard": {
    "schema": "pi.swarm.safety_scorecard.v1",
    "overall_status": "ready"
  }
}
```

自动驾驶输入包证据：

```json
{
  "schema": "pi.swarm.autopilot_input_pack.v1",
  "purpose": "dry_run_swarm_autopilot_input_not_source_of_truth",
  "status": "degraded",
  "normalized_inputs": {
    "agent_mail": {
      "status": "degraded",
      "fallback_action": "use_beads_soft_lock"
    },
    "budget_drift": {
      "schema": "pi.swarm.budget_drift.v1",
      "status": "deny_new_work",
      "signals": [
        {
          "id": "rch_queue_saturated",
          "severity": "critical",
          "recommendation": "deny new heavyweight work until RCH queue pressure clears"
        }
      ],
      "recommended_adjustments": {
        "admit_new_agents": 0,
        "rch_verification_fanout": 0,
        "reason": "deny_new_work until critical budget drift clears"
      }
    }
  },
  "planner_guards": {
    "dry_run_only": true,
    "no_prose_scraping": true
  }
}
```

自动驾驶计划证据：

```json
{
  "schema": "pi.swarm.autopilot_plan.v1",
  "purpose": "dry_run_swarm_autopilot_plan_not_source_of_truth",
  "status": "ready",
  "actions": [
    {
      "rank": 1,
      "action": "claim_ready_bead",
      "evidence_paths": [
        "normalized_inputs.beads_ready.candidates",
        "work_partitions"
      ],
      "commands": [
        {
          "purpose": "Inspect ready bead before claiming",
          "command": "br show <issue-id> --json"
        }
      ]
    }
  ],
  "planner_guards": {
    "dry_run_only": true,
    "commands_require_operator_execution": true
  }
}
```

降级自动驾驶计划证据：

```json
{
  "schema": "pi.swarm.autopilot_plan.v1",
  "purpose": "dry_run_swarm_autopilot_plan_not_source_of_truth",
  "status": "degraded",
  "budget_drift": {
    "schema": "pi.swarm.budget_drift.v1",
    "status": "deny_new_work",
    "profile_status": "ok",
    "recommended_adjustments": {
      "admit_new_agents": 0,
      "rch_verification_fanout": 0
    }
  },
  "work_partitions": [
    {
      "issue_id": "bd-provider",
      "surface_ids": [
        "provider_streaming"
      ],
      "suggested_reservation": [
        "src/provider.rs",
        "src/providers/**/*.rs",
        "tests/provider_streaming*.rs"
      ],
      "avoid": [],
      "confidence": "high",
      "degraded_caveats": []
    }
  ],
  "failure_actions": [
    {
      "id": "FAIL-AGENT-MAIL-SCHEMA",
      "catalog_schema": "pi.swarm.failure_action_catalog.v1",
      "category": "agent_mail",
      "title": "Agent Mail database schema is missing required tables",
      "match_confidence": "high",
      "explanation": "Agent Mail coordination cannot be trusted for reservations or inbox state until the mailbox schema is repaired or restored.",
      "evidence_paths": [
        "normalized_inputs.agent_mail"
      ],
      "matched_source": "agent_mail",
      "safe_commands": [
        {
          "purpose": "Preview Agent Mail repair",
          "command": "am doctor repair --dry-run"
        }
      ],
      "escalation": "Continue with Beads soft locks until Mail health is green.",
      "raw_excerpt": "status=degraded issue=database schema missing required tables",
      "redaction_summary": {
        "redacted_count": 0,
        "fields": []
      }
    }
  ],
  "actions": [
    {
      "rank": 1,
      "action": "adjust_swarm_budget",
      "evidence_paths": [
        "normalized_inputs.budget_drift.status",
        "normalized_inputs.budget_drift.signals"
      ],
      "commands": [
        {
          "purpose": "Refresh swarm resource preflight",
          "command": "pi doctor --only swarm --format json"
        }
      ]
    },
    {
      "rank": 2,
      "action": "use_beads_soft_lock",
      "evidence_paths": [
        "normalized_inputs.agent_mail.status"
      ],
      "commands": [
        {
          "purpose": "Inspect active ownership",
          "command": "br list --status=in_progress --json"
        }
      ]
    }
  ]
}
```

自动驾驶无模拟端到端证据：

```json
{
  "schema": "pi.swarm.autopilot_e2e.v1",
  "purpose": "no_mock_swarm_autopilot_e2e_operator_evidence_not_release_claim",
  "status": "pass",
  "required_scenarios": [
    "healthy_ready_claim",
    "empty_ready_queue",
    "degraded_agent_mail_soft_lock",
    "saturated_rch_queue",
    "stale_in_progress_bead",
    "unrelated_dirty_worktree",
    "malformed_source_fail_closed"
  ],
  "events_jsonl": "/data/tmp/pi_swarm_autopilot_e2e/<run>/events.jsonl",
  "guards": {
    "uses_real_temp_beads": true,
    "uses_real_temp_git": true,
    "fixture_captures_degraded_rch_and_agent_mail": true,
    "dangerous_commands_blocked": true,
    "heavy_rust_validation_requires_rch": true
  }
}
```

降级协同 runpack 无模拟端到端证据：

```bash
e2e_dir="/data/tmp/pi_swarm_degraded_coordination_e2e/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
python3 scripts/build_swarm_operator_runpack.py \
  --run-degraded-coordination-e2e \
  --capture-dir "$e2e_dir" \
  --out-degraded-coordination-e2e-json "$e2e_dir/summary.json" \
  --out-degraded-coordination-e2e-events-jsonl "$e2e_dir/events.jsonl"
```

该摘要发出 `pi.swarm.degraded_coordination_runpack_e2e.v1` 与带 `pi.swarm.degraded_coordination_runpack_e2e.event.v1` 的 JSONL 事件。该场景对一个全新进行中 Bead 与一个被阻断待办 Bead 使用真实临时 Beads 工作区、夹具捕获的 Agent Mail 语义就绪失败与 RCH 工作器工作区影子阻断。通过的裁决表示 runpack 推荐 Beads 软锁归属、保持验证为降级而非绿色，且不对临时制品发出清理或删除命令。

事件回放端到端证据：

```bash
e2e_dir="/data/tmp/pi_swarm_incident_replay_e2e/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
python3 scripts/build_swarm_operator_runpack.py \
  --run-swarm-incident-replay-e2e \
  --capture-dir "$e2e_dir" \
  --out-swarm-incident-replay-e2e-json "$e2e_dir/summary.json" \
  --out-swarm-incident-replay-e2e-events-jsonl "$e2e_dir/events.jsonl"
```

该摘要发出 `pi.swarm.incident_replay_e2e.v1` 与带 `pi.swarm.incident_replay_e2e.event.v1` 的 JSONL 事件。它在安全处使用真实临时 Beads 与 git 工作区，在实时变更不安全处使用夹具捕获的 Agent Mail/RCH 失败、已检入事件回放/证明记忆/操作员工作源，以及扩展防火墙加平滑度 SLO 失败证据。通过的裁决证明 harness 闭环失败为显式操作员动作，且不授权清理命令、本地重量级 Cargo 回退、发布、基准、容量、严格 drop-in 或实时源系统声明。

自动驾驶最终决策门禁证据：

```json
{
  "schema": "pi.swarm.autopilot_decision_gate.v1",
  "purpose": "prompt_to_artifact_autopilot_epic_close_gate_not_source_of_truth",
  "status": "pass",
  "required_checks": [
    "child_beads_closed",
    "input_pack_contract",
    "planner_contract",
    "work_partitions",
    "failure_actions",
    "budget_drift",
    "e2e_logging",
    "runpack_handoff",
    "safety_guards",
    "pushed_commits",
    "quality_gates"
  ],
  "missing_checks": [],
  "follow_up_required": false,
  "follow_up_beads": [],
  "decision": "close_final_gate_and_parent_epic",
  "epic_can_close_after_this_commit": true
}
```

上下文智能最终收口门禁证据：

```json
{
  "schema": "pi.context_intelligence.closeout_gate.v1",
  "purpose": "prompt_to_artifact_context_intelligence_closeout_gate_not_source_of_truth",
  "status": "pass",
  "required_checks": [
    "child_beads_closed",
    "graph_contracts",
    "graph_builder",
    "freshness_claim_gates",
    "bundle_planner",
    "redaction_invalidation",
    "preview_surface",
    "prompt_injection",
    "no_mock_e2e",
    "perf_budgets",
    "doctor_runpack",
    "operator_docs",
    "readme_freshness",
    "pushed_commits",
    "quality_gates"
  ],
  "missing_checks": [],
  "follow_up_required": false,
  "follow_up_beads": [],
  "decision": "close_final_gate_and_parent_epic",
  "epic_can_close_after_this_commit": true
}
```

验证代理最终收口门禁证据：

```json
{
  "schema": "pi.validation_broker.closeout_gate.v1",
  "purpose": "prompt_to_artifact_validation_broker_closeout_gate_not_source_of_truth",
  "status": "pass",
  "required_checks": [
    "child_beads_closed",
    "contract_and_source_inventory",
    "lease_store_schema",
    "source_normalization",
    "admission_policy",
    "cli_surface",
    "fault_corpus_stale_recovery",
    "doctor_runpack",
    "no_mock_e2e",
    "stress_budgets",
    "operator_docs_privacy",
    "readme_freshness",
    "source_boundaries",
    "pushed_commits",
    "quality_gates"
  ],
  "missing_checks": [],
  "remaining_follow_ups": [],
  "follow_up_required": false,
  "follow_up_beads": [],
  "decision": "close_final_gate_and_parent_epic",
  "epic_can_close_after_this_commit": true
}
```

进度 SLO 最终收口门禁证据：

```json
{
  "schema": "pi.swarm.progress_slo.closeout_gate.v1",
  "purpose": "prompt_to_artifact_swarm_progress_slo_closeout_gate_not_source_of_truth",
  "status": "pass",
  "required_checks": [
    "child_beads_closed",
    "contract_and_source_inventory",
    "deterministic_evaluator",
    "cli_surface",
    "doctor_runpack_projection",
    "no_mock_e2e",
    "stress_budgets",
    "operator_docs_privacy",
    "readme_freshness",
    "source_boundaries",
    "pushed_commits",
    "quality_gates"
  ],
  "missing_checks": [],
  "remaining_follow_ups": [],
  "follow_up_required": false,
  "follow_up_beads": [],
  "decision": "close_final_gate_and_parent_epic",
  "epic_can_close_after_this_commit": true
}
```

集群飞行记录仪报告证据：

```json
{
  "schema": "pi.swarm.flight_recorder.report.v1",
  "event_count": 12,
  "coordination_failures": [],
  "replay_command": "cargo test --test e2e_swarm_flight_recorder -- --exact multi_agent_flight_recorder_bundle_replays_without_credentials --nocapture"
}
```

## 验证记录（Validation Record）

当本手册变更时，至少运行：

```bash
command -v git br bv rch cargo jq python3
python3 scripts/build_swarm_operator_runpack.py --self-test
e2e_dir="/data/tmp/pi_swarm_autopilot_e2e/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
python3 scripts/build_swarm_operator_runpack.py \
  --run-autopilot-e2e \
  --capture-dir "$e2e_dir" \
  --out-autopilot-e2e-json "$e2e_dir/summary.json" \
  --out-autopilot-e2e-events-jsonl "$e2e_dir/events.jsonl"
python3 -m json.tool docs/contracts/swarm-operator-runpack-contract.json >/dev/null
python3 -m json.tool docs/contracts/swarm-autopilot-input-pack-contract.json >/dev/null
python3 -m json.tool docs/contracts/swarm-autopilot-plan-contract.json >/dev/null
python3 -m json.tool docs/contracts/swarm-autopilot-decision-gate-contract.json >/dev/null
cargo fmt --check
git diff --check
./scripts/reconcile_beads_ledger.sh
```

若某验证命令不可用或降级，在 Beads 收口中记录命令、退出码与 stderr，而非声称手册已完全验证。
