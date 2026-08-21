# CI 操作员手册：失败特征到回放命令（CI Operator Runbook: Failure Signatures to Replay Commands）

将常见 CI 失败特征映射到精确的回放命令、关键产物路径与修复步骤。

**Bead:** bd-1f42.8.9
**策略（Policy）：** [docs/testing-policy.md](testing-policy.md)
**QA 手册：** [docs/qa-runbook.md](qa-runbook.md)

---

## 快速参考：从任意失败回放（Quick Reference: Replay from Any Failure）

```bash
# 1. 从先前的端到端运行回放所有失败套件
./scripts/e2e/run_all.sh --rerun-from tests/e2e_results/<ts>/summary.json

# 2. 回放单个套件
cargo test --test <suite_name> -- --nocapture

# 3. 回放单个测试函数
cargo test --test <suite_name> <test_name> -- --nocapture

# 4. 带调试输出回放
RUST_LOG=debug RUST_BACKTRACE=1 cargo test --test <suite_name> -- --nocapture

# 5. 回放 CI 门禁失败
cargo test --test ci_full_suite_gate -- full_suite_gate --nocapture --exact
```

---

## 失败特征映射（Failure Signature Map）

### 完成度审计收口闸门失败（Completion audit closeout gate failure）

**特征（Signature）：** `Completion audit closeout gate self-test` 失败或 `scripts/check_completion_audit_gate.py` 对收口产物以非零退出。

**产物（Artifacts）：**
- 来自 `scripts/build_completion_audit.py` 的完成度审计 JSON
- `tests/fixtures/completion_audit_gate/scenarios.json`
- `tests/fixtures/completion_audit_gate/goldens/*.json`

**回放（Replay）：**
```bash
python3 scripts/check_completion_audit_gate.py --self-test \
  --generated-at 2026-01-02T03:04:05+00:00

python3 scripts/check_completion_audit_gate.py \
  --audit-json docs/evidence/<completion-audit>.json
```

**修复（Remediation）：**
1. 读取门禁 JSON 中 `blockers[*].kind` 与 `operator_next_actions`。
2. 对于 `missing_push`，推送收口提交并重新生成审计。
3. 对于 `failed_command`，修复失败后重新运行精确命令并附上通过的转录。
4. 对于 `proxy_only_evidence`，将叙述性或间接证明替换为直接的命令、产物、git 或 Beads 证据。
5. 对于 `missing_artifact`，创建或修正产物路径并重新运行审计。
6. 对于 `unresolved_gap`，在收口前关闭缺口或创建归属明确的 Bead。

该门禁有意保持轻量：仅读取既有 JSON。它不得运行 Cargo、调用在线提供方、变更 Beads、发送 Agent Mail、启动 RCH 或删除文件。审计引用的任何未来重量级校验必须使用仓库的 RCH 支持命令形态。

### 无模拟合规门禁失败（Non-mock compliance gate failure）

**特征：** `non_mock_compliance_gate ... FAILED`

**产物：**
- `docs/non-mock-rubric.json`（评分阈值）
- `docs/test_double_inventory.json`（当前清单）

**回放：**
```bash
cargo test --test non_mock_compliance_gate -- --nocapture
```

**修复：**
1. 检查哪个模块低于其下限阈值。
2. 查看 `docs/non-mock-rubric.json` 中受影响模块的下限值。
3. 将 mock/stub 用法迁移至 VCR 或真实实现。
4. 审批流程参见 `docs/testing-policy.md`“允许清单例外（Allowlisted Exceptions）”。

---

### 扩展一致性门禁失败（Extension conformance gate failure）

**特征：** `conformance_must_pass_gate ... FAILED`

**产物：**
- `tests/ext_conformance/reports/gate/must_pass_gate_verdict.json`
- `tests/ext_conformance/reports/conformance_summary.json`

**回放：**
```bash
cargo test --test ext_conformance_generated --features ext-conformance \
  -- conformance_must_pass_gate --nocapture --exact
```

**修复：**
1. 在 `conformance_summary.json` 中查看通过/失败/不适用计数。
2. 查找摘要中新增的失败扩展。
3. 常见原因：缺少 node 垫片、新增宿主调用未分发、QuickJS 模块解析。
4. 调试工作流参见 `docs/conformance-operator-playbook.md`。

---

### 跨平台矩阵失败（Cross-platform matrix failure）

**特征：** `cross_platform_matrix ... FAILED`

**产物：**
- `tests/cross_platform_reports/linux/platform_report.json`

**回放：**
```bash
cargo test --test ci_cross_platform_matrix -- cross_platform_matrix --nocapture --exact
```

**修复：**
1. 读取平台报告以确定哪些检查失败。
2. 常见原因：缺失系统依赖、路径分隔符问题、权限差异。
3. 修复平台相关代码并重新运行。

---

### 证据包校验失败（Evidence bundle validation failure）

**特征：** `build_evidence_bundle ... FAILED`

**产物：**
- `tests/evidence_bundle/index.json`

**回放：**
```bash
cargo test --test ci_evidence_bundle -- build_evidence_bundle --nocapture --exact
```

**修复：**
1. 证据包校验所有必需产物是否存在且格式良好。
2. 检查缺失的产物文件（summary.json、environment.json 等）。
3. 确保 `scripts/e2e/run_all.sh` 已完成所有后处理阶段。

---

### 认证修复待办缺失或陈旧（Certification remediation backlog missing or stale）

**特征：** 认证/就绪检查因缺失 `extension_remediation_backlog.json` 或模式不匹配而失败。

**产物：**
- `tests/full_suite_gate/certification_dossier.json`
- `tests/full_suite_gate/extension_remediation_backlog.json`
- `tests/full_suite_gate/extension_remediation_backlog.md`

**回放：**
```bash
cargo test --test qa_certification_dossier -- certification_dossier --nocapture --exact
```

**修复：**
1. 通过上述命令一次性重新生成认证产物与待办。
2. 验证待办模式为 `pi.qa.extension_remediation_backlog.v1`。
3. 当存在一致性失败时，确保待办摘要/条目非空。
4. 产物刷新后重新运行依赖门禁。

---

### 套件分类守卫失败（Suite classification guard failure）

**特征：** `suite_classification` 门禁失败

**产物：**
- `tests/suite_classification.toml`

**回放：**
```bash
cargo test --test ci_full_suite_gate -- full_suite_gate --nocapture --exact
```

**修复：**
1. `tests/` 中新增的测试文件未在 `tests/suite_classification.toml` 中列出。
2. 将文件分类到 `[suite.unit]`、`[suite.vcr]` 或 `[suite.e2e]`。
3. 在每个套件内保持条目按字母排序。

---

### 豁免生命周期失败（Waiver lifecycle failure）

**特征：** `waiver_lifecycle_audit ... FAILED` 或 `waiver_lifecycle` 门禁失败

**产物：**
- `tests/full_suite_gate/waiver_audit.json`
- `tests/suite_classification.toml`（豁免条目）

**回放：**
```bash
cargo test --test ci_full_suite_gate -- waiver_lifecycle_audit --nocapture --exact
```

**修复：**
1. 在 `waiver_audit.json` 中检查过期或无效豁免。
2. 过期豁免必须续期（新的 `expires` 日期，最长 +30 天）或移除。
3. 无效豁免缺少必填字段；补充全部 7 个字段。
4. 完整模式参见 `docs/qa-runbook.md`“豁免生命周期（Waiver Lifecycle）”。

---

### 提供方流式回归（Provider streaming regression）

**特征：** `provider_streaming` 或 `e2e_provider_streaming` 测试失败

**产物：**
- `tests/fixtures/vcr/`（VCR 磁带(cassette)）

**回放：**
```bash
# VCR 支持
VCR_MODE=playback cargo test --test provider_streaming -- --nocapture

# 端到端
cargo test --test e2e_provider_streaming -- --nocapture
```

**修复：**
1. 检查 VCR 磁带是否陈旧（模型 ID 变更、API 格式更新）。
2. 验证 `StreamOptions` 中 `api_key: Some("vcr-playback".to_string())`。
3. 对于 URL 不匹配：VCR 使用严格 URL 匹配；确保测试中模型 ID 与磁带一致。

---

### 端到端 TUI 测试失败（E2E TUI test failure）

**特征：** `e2e_tui` 测试失败

**产物：**
- 端到端结果目录

**回放：**
```bash
cargo test --test e2e_tui -- --nocapture
```

**修复：**
1. TUI 测试需要 tmux。验证 `tmux` 已安装且可访问。
2. 设置 `PI_TEST_MODE=1` 以获得确定性渲染。
3. VCR 磁带提供提供方响应；检查磁带新鲜度。

---

### 不稳定测试（Flaky test，本地通过、CI 失败）

**特征：** 同一提交上多次运行结果不一致（时而通过时而失败）。

**回放：**
```bash
# 使用与 CI 相同的并行度运行
cargo test --test <suite> -- --nocapture --test-threads=1

# 多次运行以检测不稳定性
for i in $(seq 1 5); do
    cargo test --test <suite> -- <test_name> --exact --nocapture || echo "FAIL on run $i"
done
```

**修复：**
1. 按分类法对不稳定用例分流（FLAKE-TIMING/ENV/NET/RES/EXT/LOGIC）。
2. 在 `tests/suite_classification.toml` 中添加隔离条目。
3. 完整生命周期参见 `docs/testing-policy.md`“不稳定测试隔离（Flaky-Test Quarantine）”。

---

## 一致性事件响应（Parity Incident Response，DROPIN-162）

本节定义威胁严格 drop-in 声明的一致性回归的操作员工作流。

### 事件触发条件（立即建档）

- `tests/e2e_results/<ts>/triage_diff.json` 中 `status = "regression"` 或 `summary.regression_count > 0`。
- `tests/full_suite_gate/full_suite_verdict.json` 显示影响一致性/测试日志证据的阻塞门禁失败（`e2e_log_contract`、`suite_classification`、`conformance_pass_rate`、`evidence_bundle` 或其他阻塞门禁）。
- `docs/evidence/dropin-certification-verdict.json` 缺失、`overall_verdict != CERTIFIED`、未命名干净发布源提交，或当发布文案需要严格 drop-in 措辞时最终发布引用包含该源提交之后的任意非证据后代。
- CI 一致性套件门禁失败（`.github/workflows/ci.yml` 中 `PARITY GATE FAIL`）。

### 严重级别与响应目标

| 严重级别 | 判定标准 | 响应目标 |
|----------|----------|----------|
| `SEV-1` | `main` 或发布切分路径上的阻塞性一致性回归 | 30 分钟内指派负责人并发布事件上下文 |
| `SEV-2` | PR/分支中新增回归，当前无发布阻塞 | 4 小时内指派负责人并发布上下文 |
| `SEV-3` | 无活跃行为回归的证据/文档漂移 | 1 个工作日内指派负责人并发布上下文 |

### 每次一致性事件的证据包（Evidence bundle for every parity incident）

收集并将以下产物附加到事件 Bead 与 Agent Mail 线程：

- `tests/e2e_results/<ts>/summary.json`
- `tests/e2e_results/<ts>/triage_diff.json`
- `tests/e2e_results/<ts>/replay_bundle.json`
- `tests/e2e_results/<ts>/failure_diagnostics_index.json`
- `tests/full_suite_gate/full_suite_verdict.json`
- `tests/full_suite_gate/full_suite_events.jsonl`
- `tests/full_suite_gate/full_suite_report.md`
- `tests/evidence_bundle/index.json`
- `docs/contracts/dropin-certification-contract.json`
- `docs/evidence/dropin-certification-verdict.json`（若该次运行中存在）

### 响应流程

1. 捕获可复现的基线差异：
```bash
./scripts/e2e/run_all.sh --profile ci \
  --diff-from tests/e2e_results/<baseline-ts>/summary.json
```

2. 对失败通道运行门禁回放命令：
```bash
cargo test --test ci_full_suite_gate -- full_suite_gate --nocapture --exact
cargo test --test ci_full_suite_gate -- preflight_fast_fail --nocapture --exact
cargo test --test ci_full_suite_gate -- full_certification --nocapture --exact
```

3. 从裁决中提取每个门禁的精确修复命令：
```bash
python3 - <<'PY'
import json
from pathlib import Path
p = Path("tests/full_suite_gate/full_suite_verdict.json")
if not p.exists():
    raise SystemExit("missing full_suite_verdict.json")
data = json.loads(p.read_text(encoding="utf-8"))
for gate in data.get("gates", []):
    if gate.get("status") == "fail":
        print(f"{gate['id']}: {gate.get('reproduce_command', 'N/A')}")
PY
```

4. 创建/更新归属 Bead 并在线程内通知集群（`thread_id = bead id`），内容包括：失败门禁 ID、`triage_diff.status`、靠前的 `ranked_diagnostics` 与一键回放命令。

5. 应用修复并重新运行：
```bash
./scripts/e2e/run_all.sh --rerun-from tests/e2e_results/<ts>/summary.json
cargo test --test ci_full_suite_gate -- full_suite_gate --nocapture --exact
```

6. 仅当所有退出条件为真时关闭：
   - `triage_diff.status` 不为 `regression`。
   - 阻塞性全套件门禁通过。
   - 面向发布声明的 drop-in 措辞守卫已满足（`overall_verdict = CERTIFIED`）。
   - Bead + Agent Mail 线程包含产物链接与最终修复说明。

### 升级路径（Escalation path）

- 若超出响应目标仍未解决：在同一 Bead 线程内升级至维护者。
- 若发布列车进行中且 `SEV-1` 持续：冻结严格 drop-in 文案直至一致性事件关闭。
- 仅在短期应急控制下使用回滚模式（`CI_GATE_PROMOTION_MODE=rollback`）；在事件 Bead 中记录原因与过期时间，修复后恢复 `strict`。

### PERF-3X 门禁事件附录（bd-3ar8v.6.4）

当事件影响性能认证（不仅是 drop-in 措辞）时，同时应用此闭环失败清单：

1. 将缺失/陈旧的 PERF-3X 产物视为阻塞失败：
   - `tests/full_suite_gate/perf3x_bead_coverage_audit.json`
   - `tests/full_suite_gate/practical_finish_checkpoint.json`
   - `tests/perf/reports/budget_summary.json`
   - `tests/perf/reports/perf_comparison.json`
   - `tests/perf/reports/stress_triage.json`
   - `tests/perf/reports/parameter_sweeps.json`
2. 附加 `tests/full_suite_gate/certification_events.jsonl` 及 perf 事件流：
   - `tests/perf/reports/budget_events.jsonl`
   - `tests/perf/reports/perf_comparison_events.jsonl`
   - `tests/perf/reports/stress_events.jsonl`
   - `tests/perf/reports/parameter_sweeps_events.jsonl`
3. 对归因与回放定位使用 `docs/qa-runbook.md` 中 **PERF-3X 回归分流（bd-3ar8v.6.4）** 下的日志查询手册。
4. 在 Bead 线程中以产物链接记录检测、归因、缓解与验证全部完成前，不要关闭事件。

### PERF-3X 特征：`parameter_sweeps_integrity` 门禁失败

**特征：** `full_suite_verdict.json` 中门禁 `parameter_sweeps_integrity` 的 `status = "fail"` 且详情提及 `parameter_sweeps.*` 模式/就绪/源契约漂移。

**产物：**
- `tests/perf/reports/parameter_sweeps.json`
- `tests/perf/reports/parameter_sweeps_events.jsonl`
- `tests/perf/reports/phase1_matrix_validation.json`
- `tests/full_suite_gate/full_suite_verdict.json`

**回放：**
```bash
rch exec -- cargo test --test release_evidence_gate -- \
  parameter_sweeps_contract_links_phase1_matrix_and_readiness --nocapture --exact
rch exec -- cargo test --test ci_full_suite_gate -- full_suite_gate --nocapture --exact
```

**修复：**
1. 强制产物模式 `pi.perf.parameter_sweeps.v1`。
2. 强制 `source_identity` 契约（`source_artifact = "phase1_matrix_validation"` 且 `source_artifact_path` 引用 `phase1_matrix_validation.json`）。
3. 强制就绪不变量：
   - `status = ready` -> `ready_for_phase5 = true` 且 `blocking_reasons = []`
   - `status = blocked` -> `ready_for_phase5 = false` 且 `blocking_reasons` 非空
4. 确保 `selected_defaults` 为正整数且 `sweep_plan.dimensions` 包含所需旋钮。
5. 重新运行全套件门禁并重新附加更新后的 `parameter_sweeps` 产物与事件流。

### PERF-3X 特征：`practical_finish_checkpoint` 就绪漂移

**特征：** 门禁 `practical_finish_checkpoint` 失败，详情类似 `technical PERF-3X issue(s) still open` 或 `Fail-closed practical-finish source read error`。

**产物：**
- `tests/full_suite_gate/practical_finish_checkpoint.json`
- `.beads/issues.jsonl`（或回退 `.beads/beads.base.jsonl`）
- `tests/full_suite_gate/full_suite_verdict.json`
- `tests/full_suite_gate/certification_events.jsonl`

**回放：**
```bash
rch exec -- cargo test --test ci_full_suite_gate -- \
  practical_finish_report_fails_when_technical_open_issues_remain --nocapture --exact
rch exec -- cargo test --test release_readiness -- practical_finish_checkpoint_ -- --nocapture
rch exec -- cargo test --test ci_full_suite_gate -- full_suite_gate --nocapture --exact
```

**修复：**
1. 验证 `practical_finish_checkpoint.json` 模式为 `pi.perf3x.practical_finish_checkpoint.v1`。
2. 确保必需契约字段一致：`status`、非空 `detail`、`technical_completion_reached`、`residual_open_scope`，以及计数相等性（`open_perf3x_count = technical_open_count + docs_or_report_open_count`）。
3. 关闭或重新界定剩余技术性 PERF-3X issue；仅允许 docs/report 残留。
4. 重新运行全套件门禁并在关闭前附加刷新后的检查点产物与认证事件。

### FrankenNode 声明特征：`claim_tier_order_drift`

**特征：** 声明契约校验报告以下层级顺序漂移（或缺失规范序列）：
- `TIER-1-EXTENSION-HOST-PARITY`
- `TIER-2-TARGETED-RUNTIME-PARITY`
- `TIER-3-FULL-NODE-BUN-REPLACEMENT`

**产物：**
- `docs/franken-node-claim-gating-contract.json`
- `tests/full_suite_gate/franken_node_claim_verdict.json`
- `tests/full_suite_gate/practical_finish_checkpoint.json`

**回放：**
```bash
rch exec -- cargo test --test franken_node_claim_contract -- \
  franken_node_claim_contract_declares_expected_tier_order -- --nocapture
rch exec -- cargo test --test release_evidence_gate -- \
  franken_node_claim_contract_is_present_and_valid --nocapture --exact
```

**修复：**
1. 在 `claim_tiers` 中恢复规范层级顺序 Tier-1 -> Tier-2 -> Tier-3。
2. 确保每层仍携带非空 `required_evidence`、`allowed_claim_language` 与 `forbidden_claim_language`。
3. 保持严格替换门禁为闭环失败（需 `overall_verdict = CERTIFIED`）并在事件关闭前重新生成 `franken_node_claim_verdict.json`。

### FrankenNode 内核边界特征：`kernel_boundary_drift`

**特征：** 在清单校验输出中检测到内核抽取边界契约/报告漂移，尤其是模块归属覆盖缺失、归属重复或禁用跨边界对回归。

**产物：**
- `docs/franken-node-kernel-extraction-boundary-manifest.json`
- `tests/full_suite_gate/franken_node_kernel_boundary_drift_report.json`
- `tests/full_suite_gate/practical_finish_checkpoint.json`

**回放：**
```bash
rch exec -- cargo test --test franken_node_kernel_extraction_boundary_manifest -- \
  kernel_boundary_manifest_ -- --nocapture
rch exec -- cargo test --test qa_docs_policy_validation -- \
  franken_node_mission_contract_tier_mapping_declares_required_checks_and_phase6_beads -- --nocapture
```

**修复：**
1. 确保漂移报告检查保持存在且为闭环失败：`kernel_boundary.all_modules_mapped_or_deferred`、`kernel_boundary.no_duplicate_domain_ownership` 与 `kernel_boundary.banned_cross_boundary_pairs_absent`。
2. 在任务契约中恢复严格层级证据链接令牌：`docs/franken-node-kernel-extraction-boundary-manifest.json` 与 `tests/full_suite_gate/franken_node_kernel_boundary_drift_report.json`。
3. 重新运行回放命令并在清除事件前附加刷新后的产物。

### FrankenNode 兼容 harness 特征：`node_runtime_unavailable_or_shimmed`

**特征：** 语义兼容性 harness 失败或硬跳过，因为真实 Node 运行时不可用，或 Bun 的 `node` 垫片被错误视为 Node。典型信号包括 `Node.js not found` 与 `SKIP: generate_compatibility_matrix requires both Node.js and Bun`。

**产物：**
- `tests/franken_node_compat_harness.rs`
- `tests/franken_node_compat/fixtures/`
- `tests/full_suite_gate/full_suite_verdict.json`

**回放：**
```bash
rch exec -- cargo test --test franken_node_compat_harness -- \
  node_detection_rejects_bun_node_shim_when_present -- --nocapture
rch exec -- cargo test --test franken_node_compat_harness -- \
  generate_compatibility_matrix -- --nocapture
```

**修复：**
1. 保持 `find_node()` 与 `is_real_node()` 与闭环失败检测一致：Bun 的 `/home/ubuntu/.bun/bin/node` 垫片不得作为真实 Node 通过。
2. 当 Node/Bun 不可用时保留确定性跳过诊断：`SKIP: Node.js not found on this machine`、`SKIP: Bun not found on this machine` 与 `SKIP: generate_compatibility_matrix requires both Node.js and Bun`。
3. 在运行时可用性纠正后，重新运行 harness 回放命令并在清除事件前附加刷新后的裁决产物。

---

## 证据产物解读（Evidence Artifact Interpretation）

### summary.json

主要运行摘要。关键字段：

| 字段 | 含义 |
|-------|---------|
| `failed_names` | 失败的端到端套件名称列表 |
| `failed_unit_names` | 失败的单元目标名称列表 |
| `passed_suites` / `total_suites` | 端到端套件通过率 |
| `replay_bundle.one_command_replay` | 回放所有失败的一键命令 |
| `triage_diff` | 基线对比（若使用了 `--diff-from`） |

### replay_bundle.json

合并的回放命令与环境上下文：

| 字段 | 含义 |
|-------|---------|
| `one_command_replay` | 复现所有失败的单一命令 |
| `environment.profile` | 运行配置（quick/focused/ci/full） |
| `environment.vcr_mode` | 运行期间的 VCR 模式 |
| `environment.git_sha` | 该次运行的 Git 提交 |
| `failed_suites[].cargo_replay` | 按套件的 cargo test 命令 |
| `failed_suites[].targeted_replay` | 单测 cargo 命令 |
| `failed_suites[].digest_path` | 按套件失败摘要路径 |

### failure_digest.json

按套件失败分析：

| 字段 | 含义 |
|-------|---------|
| `root_cause_class` | 分类：assertion_failure、timeout、panic 等 |
| `impacted_scenario_ids` | 失败测试名称列表 |
| `first_failing_assertion` | 首个失败的位置与消息 |
| `remediation_pointer.replay_command` | 运行器级回放 |
| `remediation_pointer.suite_replay_command` | 套件级 cargo test |
| `remediation_pointer.targeted_test_replay_command` | 单测 cargo test |

### triage_diff.json

基线对比用于回归检测：

| 字段 | 含义 |
|-------|---------|
| `status` | `regression`、`stable` 或 `known_failures_only` |
| `summary.regression_count` | 相对基线的新增失败数 |
| `ranked_diagnostics` | 按严重度排序的变更列表 |
| `recommended_commands.runner_repro_command` | 回放所有问题目标 |
| `recommended_commands.ranked_repro_commands` | 按优先级排序的按目标命令 |

---

## 分片工作流（Shard Workflow）

CI 运行器支持分片以实现并行执行：

```bash
# 为端到端套件运行 3 分片中的第 0 片
./scripts/e2e/run_all.sh --profile ci --shard-kind suite --shard-index 0 --shard-total 3

# 为单元目标运行 4 分片中的第 1 片
./scripts/e2e/run_all.sh --profile ci --shard-kind unit --shard-index 1 --shard-total 4
```

分片上下文捕获于：
- `environment.json`：`shard.kind`、`shard.index`、`shard.total`
- `summary.json`：相同的分片字段
- `replay_bundle.json`：`environment.shard_kind`、`shard_index`、`shard_total`

要回放特定分片的失败，请对该分片的 `summary.json` 使用 `--rerun-from` 标志。
