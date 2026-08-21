> 本文为英文原文的中文翻译，源文件：`docs/sec_traceability_matrix.md`

# SEC 工作流单元测试追溯矩阵 / SEC Workstream Unit-Test Traceability Matrix

版本（Version）: 1.0.0 | 数量生成（Counts generated）: 2026-02-14 | 定位归属刷新（Locator ownership refreshed）: 2026-08-06 | Bead: bd-2jkio (SEC-6.5)

## 概览（Overview）

本追溯矩阵（traceability matrix）将每个 SEC 实现 bead 映射到其具体的单元测试与集成测试目标。
机器可读版本位于 `docs/sec_traceability_matrix.json`。

**总计：1,155 项 SEC 相关测试**，覆盖 18 个 Bead 与 29 个集成测试文件。已认证的数量仍以 2026-02-14 基线为准；机器可读矩阵中源码级测试归属在扩展（extension）运行时分解后已刷新。

---

## WS2: 供应链与溯源 / Supply-Chain and Provenance

| Bead | 标题（Title） | 单元（Unit） | 集成（Integration） | 总计（Total） | 类别（Categories） |
|------|-------|------|-------------|-------|------------|
| SEC-2.1 (bd-f0huc) | 扩展清单 v2（Extension manifest v2） | 12 | 23 | 35 | success, failure, edge-case |
| SEC-2.2 (bd-3br2a) | 锁文件 + 溯源（Lockfile + provenance） | 8 | 0 | 8 | success, failure |
| SEC-2.3 (bd-21vng) | 安装时扫描器（Install-time scanner） | 15 | 49 | 64 | success, failure, edge-case, determinism |
| SEC-2.4 (bd-21nj4) | 隔离至信任（Quarantine-to-trust） | 10 | 37 | 47 | success, failure, edge-case |

**集成测试文件（Integration test files）:**
- `tests/ext_preflight_analyzer.rs` (7 tests) - SEC-2.1
- `tests/e2e_workflow_preflight.rs` (16 tests) - SEC-2.1
- `tests/install_time_security_scanner.rs` (49 tests) - SEC-2.3
- `tests/extension_trust_promotion.rs` (37 tests) - SEC-2.4

---

## WS3: 运行时异常检测 / Runtime Anomaly Detection

| Bead | 标题（Title） | 单元（Unit） | 集成（Integration） | 总计（Total） | 类别（Categories） |
|------|-------|------|-------------|-------|------------|
| SEC-3.1 (bd-2a9ll) | 宿主调用遥测（Hostcall telemetry） | 20 | 1 | 21 | success, edge-case |
| SEC-3.2 (bd-153pv) | 基线建模（Baseline modeling） | 15 | 14 | 29 | success, edge-case, determinism |
| SEC-3.3 (bd-3f1ab) | 风险评分器（Risk scorer） | 30 | 17 | 47 | success, failure, edge-case, determinism |
| SEC-3.4 (bd-3tb30) | 执行状态机（Enforcement state machine） | 29 | 16 | 45 | success, anti-flapping, determinism |
| SEC-3.5 (bd-3i9da) | 哈希链账本（Hash-chained ledger） | 25 | 15 | 40 | success, determinism, tamper-detection |

**集成测试文件（Integration test files）:**
- `tests/e2e_runtime_risk_telemetry.rs` (1 test) - SEC-3.1
- `tests/runtime_risk_quantile_validation.rs` (8 tests) - SEC-3.2
- `tests/runtime_risk_quantile_evidence.rs` (6 tests) - SEC-3.2
- `tests/risk_scorer_golden_fixtures.rs` (7 tests) - SEC-3.3
- `tests/accuracy_performance_sec63.rs` (10 tests) - SEC-3.3/6.3
- `tests/enforcement_state_machine_sec34.rs` (16 tests) - SEC-3.4
- `tests/ledger_calibration_sec35.rs` (15 tests) - SEC-3.5

### SEC-3.4 断言清单 / Assertion Checklist

- [x] 状态排序（State ordering） (`Allow < Harden < Prompt < Deny < Terminate`)
- [x] Display 实现往返（Display impl roundtrip）
- [x] `EnforcementState` <-> `RuntimeRiskAction` 映射（mapping）
- [x] 按配置分数段分类（Per-profile score band classification）（safe/balanced/permissive）
- [x] 高分时立即升级（Immediate escalation on high scores）
- [x] 多级跃升式升级（Multi-level escalation jumps）
- [x] 迟滞阻止立即降级（Hysteresis prevents immediate de-escalation）
- [x] 降级前需要冷却期（Cooldown required before de-escalation）
- [x] 每次仅降一级（One-level-at-a-time de-escalation）
- [x] Terminate 为终止态（Terminate is terminal）
- [x] 升级时重置冷却期（Cooldown resets on escalation）
- [x] 边界分数下无抖动（No flapping under borderline scores）（10 次评估抖动测试 / 10-evaluation jitter test）
- [x] 所有类型的 Serde 往返（Serde roundtrip for all types）
- [x] 确定性序列复现（Deterministic sequence reproduction）
- [x] 配置对比（Profile comparison）（safe vs permissive）

---

## WS4: 能力策略与中介 / Capability Policy and Mediation

| Bead | 标题（Title） | 单元（Unit） | 集成（Integration） | 总计（Total） | 类别（Categories） |
|------|-------|------|-------------|-------|------------|
| SEC-4.1 (bd-b1d7o) | 资源配额（Resource quotas） | 20 | 19 | 39 | success, failure, edge-case |
| SEC-4.2 (bd-wzzp4) | 文件系统/网络允许清单（FS/network allowlists） | 25 | 169 | 194 | success, failure, path-traversal |
| SEC-4.3 (bd-zh0hj) | 执行与机密中介（Exec + secret mediation） | 20 | 68 | 88 | success, failure, edge-case |
| SEC-4.4 (bd-2vbax) | 策略配置加固（Policy profile hardening） | 23 | 36 | 59 | success, edge-case, audit |

**集成测试文件（Integration test files）:**
- `tests/security_budgets.rs` (19 tests) - SEC-4.1
- `tests/security_fs_escape.rs` (40 tests) - SEC-4.2
- `tests/security_http_policy.rs` (28 tests) - SEC-4.2
- `tests/capability_policy_scoped.rs` (75 tests) - SEC-4.2
- `tests/capability_denial_matrix.rs` (26 tests) - SEC-4.2
- `tests/exec_mediation_integration.rs` (68 tests) - SEC-4.3
- `tests/policy_profile_hardening.rs` (36 tests) - SEC-4.4

### SEC-4.4 断言清单 / Assertion Checklist

- [x] `explain_effective_policy()` 按配置返回正确决策（returns correct decisions per profile）
- [x] 危险能力（capability）在 safe/standard 下默认被阻止（Dangerous capabilities blocked by default in safe/standard）
- [x] 危险能力仅在 permissive 配置下启用（Dangerous capabilities enabled only in permissive profile）
- [x] `DangerousOptInAuditEntry` 记录未阻止的能力（records unblocked capabilities）
- [x] `is_valid_downgrade()` 正确识别严格度变更（correctly identifies strictness changes）
- [x] 配置集成：`allow_dangerous` 时填充审计追踪（Config integration: audit trail populated on `allow_dangerous`）
- [x] 策略解释可序列化为 JSON（Policy explanation serializes to JSON）

---

## WS5: 安全体验、告警与事件响应 / Security UX, Alerts, and Incident Response

| Bead | 标题（Title） | 单元（Unit） | 集成（Integration） | 总计（Total） | 类别（Categories） |
|------|-------|------|-------------|-------|------------|
| SEC-5.1 (bd-qudx1) | 安全告警（Security alerts） | 21 | 30 | 51 | success, filtering, serde |
| SEC-5.2 (bd-ww5br) | Kill-switch + 信任（Kill-switch + trust） | 27 | 13 | 40 | success, failure, lifecycle, audit |
| SEC-5.3 (bd-11mqo) | 事件证据包（Incident evidence bundle） | 5 | 43 | 48 | success, determinism, redaction |

**集成测试文件（Integration test files）:**
- `tests/security_alert_integration.rs` (30 tests) - SEC-5.1
- `tests/trust_onboarding_killswitch_sec52.rs` (13 tests) - SEC-5.2
- `tests/incident_evidence_bundle.rs` (30 tests) - SEC-5.3
- `tests/incident_evidence_bundle_sec53.rs` (13 tests) - SEC-5.3

### SEC-5.1 断言清单 / Assertion Checklist

- [x] 6 种告警源的工厂方法（Factory methods for 6 alert sources）（策略拒绝 policy denial、执行中介 exec mediation、机密脱敏 secret redaction、异常 anomaly、隔离 quarantine、执行态转换 enforcement transition）
- [x] `SecurityAlertAction::from_enforcement()` 往返（roundtrip）
- [x] 所有变体的 `SecurityAlertAction::as_str()`（for all variants）
- [x] 告警序列化/反序列化往返（Alert serialization/deserialization roundtrip）
- [x] 按类别、严重级别、扩展、时间戳过滤（Filter by category, severity, extension, timestamp）
- [x] 类别与严重级别计数聚合（Category and severity count aggregation）
- [x] `emit_security_alert()` 记录并触发 tracing（records + emits tracing）
- [x] `sha256_short()` 确定性（determinism）

### SEC-5.2 断言清单 / Assertion Checklist

- [x] `kill_switch()` 将信任状态设为 Killed（sets trust state to Killed）
- [x] `kill_switch()` 在运行时风险控制器中隔离（quarantines in runtime risk controller）
- [x] `kill_switch()` 触发 Critical 安全告警（emits Critical security alert）
- [x] `kill_switch()` 记录带溯源（provenance）的审计条目（records audit entry with provenance）
- [x] `kill_switch()` 幂等，已处于 killed 时无操作（is idempotent (no-op when already killed)）
- [x] `kill_switch()` 在 Acknowledged 与 Trusted 状态下均可生效（works from Acknowledged and Trusted states）
- [x] `lift_kill_switch()` 恢复至 Acknowledged（restores to Acknowledged）
- [x] `lift_kill_switch()` 清除隔离与 consecutive_unsafe 计数器（clears quarantine + consecutive_unsafe counter）
- [x] `lift_kill_switch()` 触发 Info 级别告警（emits Info-level alert）
- [x] `lift_kill_switch()` 记录停用审计条目（records deactivation audit entry）
- [x] 若当前未处于 killed 则 `lift_kill_switch()` 失败（fails if not currently killed）
- [x] `is_killed()` 在状态转换中返回正确状态（returns correct state through transitions）
- [x] 未知扩展的默认信任状态为 Pending（Default trust state is Pending for unknown extensions）
- [x] 信任引导接受 -> Acknowledged（Trust onboarding accept -> Acknowledged）
- [x] 信任引导拒绝 -> Killed + 隔离（Trust onboarding reject -> Killed + quarantine）
- [x] 信任引导记录带风险等级的决策（records decision with risk level）
- [x] `promote_trust()` 从 Acknowledged -> Trusted（from Acknowledged -> Trusted）
- [x] `promote_trust()` 从 Pending 或 Killed 时无操作（no-op from Pending or Killed）
- [x] 完整生命周期：Pending -> Acknowledged -> Trusted -> Killed -> Acknowledged -> Trusted（Full lifecycle: Pending -> Acknowledged -> Trusted -> Killed -> Acknowledged -> Trusted）
- [x] Kill-switch 审计保留操作员溯源（Kill-switch audit preserves operator provenance）
- [x] 多个扩展拥有独立的信任状态（Multiple extensions have independent trust states）
- [x] `ExtensionTrustState` Display 实现（Display impl）
- [x] 告警序列 ID 单调递增（Alert sequence IDs are monotonically increasing）
- [x] Kill -> lift -> 再次 kill 循环正常工作（Kill -> lift -> kill again cycle works correctly）

---

## WS6: 验证与确定性测试 / Validation and Determinism Testing

| Bead | 标题（Title） | 单元（Unit） | 集成（Integration） | 总计（Total） | 类别（Categories） |
|------|-------|------|-------------|-------|------------|
| SEC-6.4 (bd-1a2cu) | 兼容性一致性 + CI 门禁（Compatibility conformance + CI gates） | 0 | 31 | 31 | success, conformance, regression |

**集成测试文件（Integration test files）:**
- `tests/sec_compatibility_conformance.rs` (31 tests) - SEC-6.4

### SEC-6.4 断言清单 / Assertion Checklist

- [x] 良性能力（read/write/http/events/session）在所有配置下均允许（Benign capabilities allowed in all profiles）
- [x] 危险能力（exec/env）在 safe/standard 下拒绝，在 permissive 下允许（Dangerous capabilities denied in safe/standard, allowed in permissive）
- [x] 单扩展覆盖不能绕过 deny_caps（Per-extension override cannot bypass deny_caps）
- [x] 单扩展拒绝覆盖默认允许（Per-extension deny overrides default allow）
- [x] 策略解释覆盖所有能力及原因（Policy explanation covers all capabilities with reasons）
- [x] 配置转换校验（Profile transition validation）（降级/升级检测 downgrade/upgrade detection）
- [x] 兼容性扫描器：良性扩展通过，危险扩展被标记（Compatibility scanner: benign extensions pass, dangerous flagged）
- [x] 信任生命周期：Pending → Acknowledged → Trusted → Killed（Trust lifecycle）
- [x] Kill-switch 触发安全告警与审计条目（emits security alert and audit entry）
- [x] 解除 kill-switch 触发额外告警（Lift kill-switch emits additional alert）
- [x] 引导接受/拒绝记录决策（Onboarding accept/reject records decision）
- [x] 豁免（waiver）格式与时长校验（format and duration validation）
- [x] 跨配置一致性矩阵（Cross-profile consistency matrix）
- [x] 所有配置的 Serde 往返（Serde roundtrip for all profiles）
- [x] CI 门禁产物（`sec_conformance_verdict.json`）以 95% 阈值生成（CI gate artifact generated with 95% threshold）

---

## WS7: 发布与运维 / Rollout and Operations

| Bead | 标题（Title） | 单元（Unit） | 集成（Integration） | 总计（Total） | 类别（Categories） |
|------|-------|------|-------------|-------|------------|
| SEC-7.1 (bd-2teqs) | 影子模式（Shadow mode） | 3 | 0 | 3 | success, failure |

---

## 跨领域测试文件 / Cross-Cutting Test Files

以下文件覆盖多个 SEC bead：

| 文件（File） | 测试数（Tests） | 覆盖（Covers） |
|------|-------|--------|
| `tests/capability_policy_model.rs` | 34 | SEC-4.2, SEC-4.4 |
| `tests/capability_prompt.rs` | 46 | SEC-4.2, SEC-4.3 |
| `tests/extensions_policy_negative.rs` | 38 | SEC-4.2, SEC-4.3, SEC-4.4 |
| `tests/e2e_high_risk_workflows.rs` | 23 | SEC-3.3, SEC-3.4, SEC-5.1 |
| `tests/qa_docs_policy_validation.rs` | 61 | SEC-4.2, SEC-4.3, SEC-4.4 |

---

## 维护协议 / Maintenance Protocol

1. **新增 SEC bead 时（When adding a new SEC bead）**：同时在 `docs/sec_traceability_matrix.json` 与本文件中添加条目（Add an entry to both `sec_traceability_matrix.json` and this file）。
2. **新增/移除测试时（When adding/removing tests）**：更新测试数量与断言清单（Update the test count and assertion checklist）。
3. **变更行为时（When changing behavior）**：验证对应测试断言仍匹配（Verify that the corresponding test assertions still match）。
4. **命名约定（Naming convention）**：集成测试文件应尽可能包含 SEC ID（例如 `_sec34.rs`、`_sec52.rs`）（Integration test files should include the SEC ID where possible (e.g., `_sec34.rs`, `_sec52.rs`)）。
5. **黄金夹具（Golden fixtures）**：归属 SEC-3.3（`risk_scorer_golden_fixtures.rs`）。变更需重新验证（Owned by SEC-3.3. Changes require re-validation）。
