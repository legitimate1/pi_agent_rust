# QA 运行手册与故障分流手册 / QA Runbook and Failure Triage Playbook

运行测试套件、解读失败与复现问题的参考手册。

**Bead：** bd-1f42.7.4
**策略（Policy）：** [docs/testing-policy.md](testing-policy.md)（路径保留英文：`docs/testing-policy.md`）
**评分细则（Rubric）：** [docs/non-mock-rubric.json](non-mock-rubric.json)（`docs/non-mock-rubric.json`）
**覆盖率基线：** [docs/coverage-baseline-map.json](coverage-baseline-map.json)（`docs/coverage-baseline-map.json`）

---

## 契约清单（Contract Checklist，DROPIN-171）

`docs/testing-policy.md` 定义了规范契约 `pi.parity.test_logging_contract.v1`。
使用本节校验该契约保持完整。

### 必需的跨套件保障（Required Cross-Suite Guarantees）

| 保障（Guarantee） | 校验来源（Validation Source） |
|-----------|-------------------|
| 套件分类显式（`unit`/`vcr`/`e2e`） | `tests/suite_classification.toml` |
| 测试日志模式保持 `pi.test.log.v2` | `tests/common/logging.rs` 校验器 |
| 制品索引模式保持 `pi.test.artifact.v1` | `tests/common/logging.rs` 校验器 |
| 模式演进策略保持显式且 fail-closed | `docs/testing-policy.md` + `tests/common/logging.rs` v2-only 校验器 |
| 证据契约模式保持 `pi.qa.evidence_contract.v1` | `docs/evidence-contract-schema.json` + 模式测试 |
| 失败摘要分类 + 回放元数据保持稳定 | `docs/evidence-contract-schema.json` |

### 契约校验命令（Contract Verification Commands）

```bash
# 证据契约模式 + 合成样本检查
cargo test --test validate_e2e_artifact_schema -- evidence_contract_schema --nocapture
cargo test --test validate_e2e_artifact_schema -- synthetic_evidence_contract --nocapture

# 日志/制品 JSONL 模式校验
cargo test --test e2e_artifact_retention_triage -- jsonl --nocapture

# 跨运行场景/组件过滤 + 稳定字段对比辅助
cargo test --test rpc_session_connector -- common::logging::tests::filter_log_records_by_scenario_and_component --nocapture
cargo test --test rpc_session_connector -- common::logging::tests::compare_log_streams_by_filter_ignores_trace_and_timing --nocapture

# 可选：完整契约门禁
cargo test --test validate_e2e_artifact_schema -- --nocapture
```

## 快速开始（Quick Start）

### 快速冒烟检查（< 60 秒）

```bash
./scripts/smoke.sh                  # lint + unit + VCR 冒烟目标
./scripts/smoke.sh --skip-lint      # 跳过 fmt/clippy（更快内循环）
./scripts/smoke.sh --only unit      # 仅 unit 冒烟
./scripts/smoke.sh --json           # 机器可读摘要输出到 stdout
```

制品：`tests/smoke_results/<timestamp>/smoke_summary.json`

### 完整校验（Full verification）

```bash
./scripts/e2e/run_all.sh                        # 完整：lint + lib + all targets
./scripts/e2e/run_all.sh --profile ci            # CI 配置：确定性
./scripts/e2e/run_all.sh --profile quick         # 快速：lint + lib + unit only

# 刷新认证案卷 + 整改待办制品
cargo test --test qa_certification_dossier -- certification_dossier --nocapture --exact
```

制品：`tests/e2e_results/<timestamp>/summary.json`

### 按套件运行（Suite-specific runs）

```bash
# 仅单元测试（无模拟、无夹具、无 VCR）
cargo test --all-targets --lib

# VCR/夹具回放测试
VCR_MODE=playback cargo test --all-targets

# 单个测试文件
cargo test --test provider_contract

# 单个测试函数
cargo test --test non_mock_rubric_gate -- rubric_has_required_top_level_keys
```

## 性能工作流：快速循环 vs 确定性基准（Performance Workflow: Fast Loop vs Definitive Benchmarks）

采用双速工作流，使智能体可快速推进，同时不将部分数据视为发布主张。

| 模式 | 适用时机 | 命令模式 | 主张强度 |
|------|----------|-----------------|----------------|
| 快速内循环（Fast inner loop） | 活跃编辑期间 | 仅文件级检查（`cargo fmt --check -- <file>`、定向 `cargo test --test ...`） | **非权威**；仅开发者反馈 |
| 确定性基准/认证通过（Definitive benchmark/certification pass） | 集成/决策边界 | 重量级运行通过 `rch exec -- ...` 卸载 + 必需证据制品重新生成 | **权威**，用于 PERF-3X/发布主张 |

### 确定性基准门禁（权威）

在启动长时间基准刷新前，先运行只读 preflight，使缺失输入在前期即以精确的 RCH 命令与预期路径报告：

```bash
python3 scripts/perf/preflight_budget_inputs.py
```

`scripts/perf/orchestrate.sh` 将同一检查持久化为 `results/perf_budget_preflight_before_refresh.json`，然后 perf 预算套件方可刷新 `tests/perf/reports/budget_summary.json`。它还会在制品收集后写入 `results/perf_artifact_staging_manifest.json`（`pi.perf.artifact_staging_manifest.v1`）。除非每个必需契约都有带源路径、已拉取本地路径、mtime、校验和及格式对应模式的新鲜制品，否则将该暂存清单视为阻塞。

Perf 运行还在 `PERF_EVIDENCE_CACHE_DIR`（默认：`$CARGO_TARGET_DIR/perf/evidence_cache`）维护证据缓存。仅当模式、命令、git 提交、构建配置、运行 ID/关联 ID、主机/工具链溯源、校验和与 TTL 校验通过时，缓存条目方可复用。

当 cargo 目标目录位于仓库外时，不要让 RCH 工作线程从该本地路径推断预算输入。应在仓库可见的证据根下暂存所需的 JSON、JSONL、Criterion 与发布二进制证据，并使用 `PERF_EVIDENCE_DIR` 运行报告：

```bash
PERF_EVIDENCE_DIR=tests/perf/reports \
  PI_GENERATE_PERF_BUDGET_REPORT=1 rch exec -- cargo test --test perf_budgets --profile perf generate_budget_report -- --nocapture
```

两个报告生成集成测试在常规测试运行期间为只读。仅在其显式 opt-in 下刷新受跟踪制品：

```bash
PI_GENERATE_PERF_BUDGET_REPORT=1 cargo test --test perf_budgets generate_budget_report -- --nocapture
PI_GENERATE_BENCH_SCHEMA_DOCS=1 cargo test --test bench_schema generate_schema_doc -- --nocapture
```

在 `pi.perf.budget_summary.v2` 中，`performance_claims_authorized=true` 统管 blanket 定量性能文案。因此它对**全部已声明预算**要求严格、源绑定、同运行证据：每个预算必须有数据且为 PASS，含非 CI 强制预算，且不得有数据契约失败。

`tests/perf_budgets.rs` 在 `CARGO_TARGET_DIR` 之前检查 `PERF_EVIDENCE_DIR`/`PERF_EVIDENCE_DIRS`，因此报告可消费 RCH 实际同步的暂存证据。

编排器的 `env_fingerprint.json` 具备 cgroup 感知，记录 cgroup v2 CPU 配额、cpuset 大小、内存限制、NUMA 节点数等。

使用远程卸载运行重量级检查：

```bash
rch exec -- cargo test --test bench_scenario_runner -- --nocapture
rch exec -- cargo test --test perf_budgets -- --nocapture
rch exec -- cargo test --test release_evidence_gate -- --nocapture
rch exec -- cargo test --test ci_full_suite_gate -- full_certification --nocapture --exact
```

仅当全部必需制品存在且模式合法时，基准结果才视为确定性：

- `tests/perf/reports/phase1_matrix_validation.json`（`pi.perf.phase1_matrix_validation.v1`）
- `tests/full_suite_gate/full_suite_verdict.json`
- `tests/full_suite_gate/certification_verdict.json`
- `tests/full_suite_gate/extension_remediation_backlog.json`（`pi.qa.extension_remediation_backlog.v1`）

---

## 测试套件分类（Test Suite Classification）

每个测试文件恰属一个套件。见 `tests/suite_classification.toml` 获取规范映射。

| 套件（Suite） | 测试内容 | 执行命令 |
|-------|---------------|-------------------|
| **unit** | 纯逻辑、解析、序列化、状态机。无模拟、夹具或 VCR。 | `cargo test --lib` + 精选 `--test` 目标 |
| **vcr** | 基于录制数据的提供方流式、HTTP 客户端、一致性。 | `VCR_MODE=playback cargo test` |
| **e2e** | 含真实提供方、网络或 tmux 的全系统。 | `PI_E2E=1 cargo test --test e2e_*` |

---

## 制品位置（Artifact Locations）

| 制品（Artifact） | 位置（Location） | 内容 |
|----------|----------|---------|
| 冒烟摘要 | `tests/smoke_results/<ts>/smoke_summary.json` | 按目标的通过/失败、时长 |
| 冒烟事件日志 | `tests/smoke_results/<ts>/smoke_log.jsonl` | 按事件的结构化日志 |
| E2E 摘要 | `tests/e2e_results/<ts>/summary.json` | 完整运行摘要 |
| E2E 证据 | `tests/e2e_results/<ts>/evidence_contract.json` | 证据契约 |
| E2E 回放包 | `tests/e2e_results/<ts>/replay_bundle.json` | 合并的回放命令 + 环境上下文 |
| E2E 失败诊断 | `tests/e2e_results/<ts>/failure_diagnostics_index.json` | 按套件的摘要索引 |
| 按套件失败摘要 | `tests/e2e_results/<ts>/<suite>/failure_digest.json` | 根因 + 回放命令 |
| 按套件失败时间线 | `tests/e2e_results/<ts>/<suite>/failure_timeline.jsonl` | 有序失败事件 |
| E2E 分流差异 | `tests/e2e_results/<ts>/triage_diff.json` | 基线 vs 当前对比 |
| E2E 场景矩阵 | `docs/e2e_scenario_matrix.json` | 规范工作流到套件的覆盖映射 |
| 一致性报告 | `tests/ext_conformance/reports/conformance_summary.json` | 扩展一致性 |
| CI 门禁裁决 | `tests/full_suite_gate/full_suite_verdict.json` | 全套件门禁结果 |
| CI preflight 裁决 | `tests/full_suite_gate/preflight_verdict.json` | preflight 快速失败结果 |
| CI 认证裁决 | `tests/full_suite_gate/certification_verdict.json` | 完整认证结果 |
| 扩展整改待办 | `tests/full_suite_gate/extension_remediation_backlog.json` | 未通过扩展的整改队列（`pi.qa.extension_remediation_backlog.v1`） |
| CI 豁免审计 | `tests/full_suite_gate/waiver_audit.json` | 豁免生命周期审计 |
| CI 回放包 | `tests/full_suite_gate/replay_bundle.json` | 门禁失败回放命令 |
| 合规报告 | `target/compliance-report.json` | 模块合规（设置 `COMPLIANCE_REPORT=1`） |
| 覆盖率基线 | `docs/coverage-baseline-map.json` | 按模块的行/函数覆盖率 |
| VCR 磁带 | `tests/fixtures/vcr/` | 录制的 HTTP 交互 |
| 测试失败日志 | `target/test-failures.jsonl` | 结构化失败诊断 |

---

## 故障分流手册（Failure Triage Playbook）

### 步骤 1：识别失败类别

| 特征（Signature） | 可能类别 | 下一步 |
|-----------|-------------|-------------|
| `assertion failed` 在 `provider_*` 测试中 | 提供方回归 | 检查 VCR 磁带新鲜度；核验 `StreamOptions` 中 `api_key` 已设置 |
| `missing Start event` | 流式认证失败 | 确保 VCR 测试的 `StreamOptions` 中 `api_key: Some("vcr-playback".to_string())` |
| `request URL mismatch` 在 VCR 中 | 模型 ID 漂移 | VCR 使用严格 URL 匹配，确保测试中的模型 ID 与磁带 URL 路径一致 |
| `connection refused` | 缺少测试基础设施 | 检查 mock 服务器或 VCR 是否配置；核验 `VCR_MODE` 环境变量 |
| `DummyProvider` / `NullSession` 在单元测试中 | 策略违规 | 将测试移至 VCR 套件或用真实实现替换替身 |
| `SIGSEGV` 在 `llvm-cov` 中 | LLVM 分支覆盖大文件 bug | 使用按文件 `llvm-cov export -sources FILE -summary-only` 变通。63/107 文件可用；44 文件 SIGSEGV。见 `docs/coverage-baseline-map.json` |
| `thread panicked` 在扩展测试中 | 扩展调度器问题 | 检查 `src/extension_dispatcher.rs`；审查 mock 桩使用 |
| 不稳定：本地通过、CI 失败 | 非确定性 | 按不稳定分类（FLAKE-TIMING/ENV/NET/RES/EXT/LOGIC）归类 |
| `No such file or directory` 对应磁带 | 缺少 VCR 夹具 | 录制新磁带或检查磁带命名约定 |
| `too many open files` | 资源耗尽 | 提升 `ulimit -n`；检查泄漏的文件描述符 |

### 步骤 2：本地复现

```bash
# 运行指定失败测试并输出
cargo test --test <test_file> -- <test_name> --nocapture

# 强制 VCR 回放
VCR_MODE=playback cargo test --test <test_file> -- <test_name> --nocapture

# 带调试日志
RUST_LOG=debug cargo test --test <test_file> -- <test_name> --nocapture

# 带回溯
RUST_BACKTRACE=1 cargo test --test <test_file> -- <test_name> --nocapture
```

### 步骤 3：检查 VCR 磁带完整性

若失败涉及提供方/流式测试：

```bash
# 列出某提供方的磁带
ls tests/fixtures/vcr/verify_<provider>_*.json

# 校验磁带为合法 JSON
python3 -m json.tool tests/fixtures/vcr/<cassette>.json > /dev/null

# 检查磁带中的请求 URL 是否匹配测试期望
python3 -c "
import json
with open('tests/fixtures/vcr/<cassette>.json') as f:
    d = json.load(f)
    for i in d['interactions']:
        print(i['request']['url'])
"
```

### 步骤 4：审查合规状态

```bash
# 生成合规报告
COMPLIANCE_REPORT=1 cargo test --test non_mock_compliance_gate

# 检查细则完整性
cargo test --test non_mock_rubric_gate
```

---

## 回放工作流（Replay Workflow）

E2E harness 支持确定性回放以复现失败。

### 一键回放（来自 summary.json）

```bash
# 仅重跑上次运行中失败的套件
./scripts/e2e/run_all.sh --rerun-from tests/e2e_results/<ts>/summary.json

# 将当前运行与基线对比
./scripts/e2e/run_all.sh --diff-from tests/e2e_results/<baseline>/summary.json
```

`--rerun-from` 标志读取摘要中的 `failed_names`，仅重跑这些套件，并自动将 `--diff-from` 设为源摘要以生成分流差异。

### 回放包制品（Replay bundle artifact）

运行完成后，harness 在 `summary.json` 旁产出 `replay_bundle.json`（模式 `pi.e2e.replay_bundle.v1`）。该包合并：
- **`one_command_replay`**：复现全部失败的单条命令。
- **`environment`**：配置、分片上下文、VCR 模式、rustc 版本、git SHA、OS。
- **`failed_suites`**：按套件条目，含 runner、cargo 及定向回放命令与失败摘要路径。
- **`failed_unit_targets`**：按目标的 cargo 回放命令。

### 按套件失败摘要

每个失败套件还会得到 `failure_digest.json`（模式 `pi.e2e.failure_digest.v1`），包含：
- `remediation_pointer.replay_command`（runner 级）
- `remediation_pointer.suite_replay_command`（cargo test）
- `remediation_pointer.targeted_test_replay_command`（单测）
- 根因分类与首个失败断言

### 分流差异（Triage diff）

当提供基线（经 `--diff-from` 或 `--rerun-from` 自动设置）时，harness 生成 `triage_diff.json`（模式 `pi.e2e.triage_diff.v1`），包含回归、新增失败、已修复及未解决失败等。

### 认证待办刷新

当认证制品刷新时，在同一运行中重新生成扩展整改待办，使诊断与发布门禁消费同一证据集：

```bash
cargo test --test qa_certification_dossier -- certification_dossier --nocapture --exact
```

产出制品：
- `tests/full_suite_gate/certification_dossier.json`
- `tests/full_suite_gate/certification_dossier.md`
- `tests/full_suite_gate/extension_remediation_backlog.json`
- `tests/full_suite_gate/extension_remediation_backlog.md`

---

## CI 门禁通道（CI Gate Lanes）

全套件 CI 门禁以两通道运行（bd-1f42.8.8.1）：

### Preflight 快速失败通道

仅评估**阻塞门禁**，遇首个失败即停。用于 PR 检查的快速反馈。

```bash
cargo test --test ci_full_suite_gate -- preflight_fast_fail --nocapture --exact
```

制品：`tests/full_suite_gate/preflight_verdict.json`（模式 `pi.ci.preflight_lane.v1`）

### 完整认证通道

评估**全部门禁**（阻塞 + 非阻塞），生成豁免审计，并产出含晋升规则与重跑指引的综合裁决。

```bash
cargo test --test ci_full_suite_gate -- full_certification --nocapture --exact
```

制品：
- `tests/full_suite_gate/certification_verdict.json`
- `tests/full_suite_gate/certification_events.jsonl`
- `tests/full_suite_gate/certification_report.md`

### 最终 >=3x Go/No-Go 决策工作流（bd-3ar8v.6.5）

仅在确定性发布决策边界使用。将缺失或陈旧证据视为 `NO-GO`（fail-closed），永不视为告警。

1. 重新生成权威认证证据（卸载执行）：
```bash
rch exec -- cargo test --test ci_full_suite_gate -- full_certification --nocapture --exact
rch exec -- cargo test --test release_evidence_gate -- --nocapture
rch exec -- cargo test --test qa_certification_dossier -- certification_dossier --nocapture --exact
```
2. 确认必需制品存在且为最新（见原文列表）。
3. 从 `full_suite_verdict.json` 强制最终门禁通过标准：`perf3x_bead_coverage = pass` 等。
4. 验证 practical-finish 输出为 docs/report-only 残留范围。
5. 决策：`GO` 需全部通过且制品新鲜；否则 `NO-GO`。

### Drop-in 认证契约门禁（bd-35t7i）

在发布文案可声称严格 drop-in 对等前，需评估 `docs/contracts/dropin-certification-contract.json` 并产出 `docs/evidence/dropin-certification-verdict.json`（`pi.dropin.certification_verdict.v1`）。

阻塞规则：若 `overall_verdict != CERTIFIED`，发布语言不得声称严格 drop-in 替代。

该门禁的对等事件处理定义于 `docs/ci-operator-runbook.md` 的 **Parity Incident Response (DROPIN-162)**。将发布路径上的 `overall_verdict != CERTIFIED` 或缺失裁决制品视为对等事件，而非仅文档告警。

---

## PERF-3X 回归分流（bd-3ar8v.6.4）

用于永久性能门禁事件的流程。缺失或陈旧证据为失败条件，而非告警。

### 检测（fail-closed 制品检查）

运行完整认证通道后，以下制品必须存在且匹配预期模式：
- `tests/full_suite_gate/full_suite_verdict.json`（`pi.ci.full_suite_gate.v1`）
- `tests/full_suite_gate/certification_verdict.json`（`pi.ci.certification_lane.v1`）
- `tests/perf/reports/budget_summary.json`（`pi.perf.budget_summary.v2`）等，详见原文。

### 用户侧诊断工作流（durability/resume/extension/build-profile）

用于用户侧“变慢”事件的诊断，保持场景、回放命令与制品指针一并可复现。

#### 集群协调排障
- Preflight 命令：`pi doctor --only swarm --format json`
- 确认：Beads JSONL 健康、陈旧 `in_progress` 工作、Agent Mail 状态等。
- 准入解读：在 JSON 输出中检查 `data.schema` 为 `pi.doctor.swarm_admission.v1` 的 swarm 发现项。

---

## 豁免生命周期（Waiver Lifecycle）

CI 门禁可通过可审计豁免临时绕过（bd-1f42.8.8.1）。

### 添加豁免

在 `tests/suite_classification.toml` 中添加 `[waiver.<gate_id>]` 条目：

```toml
[waiver.ext_must_pass]
owner = "AgentName"
created = "2026-02-13"
expires = "2026-02-27"            # 距 created 最长 30 天
bead = "bd-XXXX"
reason = "Blocked by upstream QuickJS bug"
scope = "both"                    # "full"、"preflight" 或 "both"
remove_when = "QuickJS fix merged and all blocking extension conformance gates pass"
```

### 豁免规则

- 最长持续：30 天（到期前必须续期或修复）。
- 过期豁免经由 `waiver_lifecycle` 门禁导致 CI 硬失败。
- 距到期 3 天内触发告警。
- `gate_id` 必须匹配 `ci_full_suite_gate.rs` 中的门禁。
- 全部 7 个字段必填。

---

## 扩展失败案卷解读（Extension Failure Dossier Interpretation）

当扩展未通过一致性测试时：

1. **检查一致性摘要：**
   ```bash
   python3 -c "
   import json
   with open('tests/ext_conformance/reports/conformance_summary.json') as f:
       d = json.load(f)
       print(f\"Pass: {d.get('pass_count', '?')}, Fail: {d.get('fail_count', '?')}, N/A: {d.get('na_count', '?')}\")
   "
   ```
2. **审查失败详情：** 失败案卷包含扩展 ID 与版本、所用输入夹具、期望 vs 实际输出、提供方兼容性说明、一键复现等。
3. **常见扩展失败模式：** 模式验证错误、超时、策略拒绝、缺失宿主调用等，详见原文表格。

---

## 冒烟套件使用（Smoke Suite Usage）

冒烟套件（`scripts/smoke.sh`）用于推送前校验：

| 套件 | 目标 | 捕获内容 |
|-------|---------|-----------------|
| Unit | `model_serialization`、`config_precedence`、`session_conformance`、`error_types`、`compaction`、`security_budgets` | 核心数据模型回归 |
| VCR | `provider_streaming`、`error_handling`、`http_client`、`sse_strict_compliance`、`model_registry`、`provider_factory` | 提供方/HTTP/SSE 回归 |

**何时运行：** 每次 `git push` 前、修改 `src/model.rs`、`src/providers/`、`src/sse.rs`、`src/config.rs` 后、更新 VCR 磁带后。

---

## CI 门禁阈值（CI Gate Thresholds）

CI 晋升门禁（`.github/workflows/ci.yml`）评估：

| 指标 | 默认值 | 覆盖变量 |
|--------|---------|-------------------|
| 晋升模式 | `strict` | `CI_GATE_PROMOTION_MODE` |
| 最低通过率 | 80.0% | `CI_GATE_MIN_PASS_RATE_PCT` |
| 最大失败数 | 36 | `CI_GATE_MAX_FAIL_COUNT` |
| 最大 N/A | 170 | `CI_GATE_MAX_NA_COUNT` |

紧急回滚：将 `CI_GATE_PROMOTION_MODE=rollback` 设为仅告警不阻塞。

---

## 按模块覆盖率阈值（Per-Module Coverage Thresholds）

来自 `docs/non-mock-rubric.json`：

| 模块 | 关键度 | 行阈（Floor） | 函数阈 | 行目标 | 函数目标 |
|--------|-------------|------------|----------------|-------------|-----------------|
| agent_loop | critical | 75% | 70% | 85% | 80% |
| tools | critical | 75% | 72% | 85% | 82% |
| providers | critical | 82% | 79% | 90% | 88% |
| extensions | critical | 80% | 69% | 88% | 80% |
| session | high | 76% | 74% | 85% | 82% |
| auth | high | 72% | 70% | 82% | 78% |
| error | high | 72% | 70% | 82% | 78% |
| model | high | 74% | 72% | 84% | 80% |
| sse | high | 76% | 74% | 86% | 82% |
| config | medium | 70% | 68% | 80% | 76% |
| compaction | medium | 70% | 68% | 80% | 76% |
| vcr | medium | 68% | 66% | 78% | 74% |
| rpc | medium | 70% | 68% | 80% | 76% |
| interactive | low | 60% | 58% | 72% | 68% |

**Floor** = 低于此 CI 失败。**Target** = 期望目标。

---

## 隔离工作流（Quarantine Workflow）

不稳定测试在 `tests/suite_classification.toml` 中隔离：

1. **检测**：测试在 CI 上失败但重试通过（同一提交）
2. **分类**：分配 `FLAKE-*` 类别（TIMING/ENV/NET/RES/EXT/LOGIC）
3. **隔离**：在 `[quarantine.<test_stem>]` 中添加条目，含全部 9 个必填字段
4. **修复**：在类别对应窗口（7 或 14 天）内落地修复
5. **恢复**：3 次干净 CI 运行后移除隔离条目

最长隔离窗口：14 天。见 `docs/testing-policy.md` 获取完整升级阶梯。
