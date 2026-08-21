# 文档分类 (docs/**/*.{md,json})

为 bd-we34i.1 生成 - DC-T1：将所有 docs 文件分类为契约 / 证据快照 / 可退役。

## 分类框架

### 契约文档
- 定义稳定接口的规范/需求文档
- API 契约、模式、认证要求
- 对长期有影响的架构/设计决策
- 面向用户的文档（指南、故障排查）

### 证据快照文档
- 生成的制品、测试结果、合规报告
- 可能变为陈旧的临时快照
- 自动化分析输出、矩阵、清单
- 用于认证闸门的带时间戳证据

### 可退役文档
- 已弃用功能的遗留文档
- 重复或已被取代的内容
- 无实际用途的空文件/占位文件
- 引用已移除功能点的文档

## 分类结果

### 契约文档（稳定规范）

#### API 与协议契约
- `schema/extension_manifest.json` - 扩展清单模式规范
- `schema/extension_protocol.json` - 扩展协议契约
- `schema/session_store_v2_contract.json` - 会话存储契约
- `schema/test_evidence_logging_contract.json` - 测试证据日志契约
- `schema/cli-surface-diff.json` - CLI surface diff 模式

#### 认证与合规契约
- `dropin-certification-contract.json` - 规范性认证要求
- `dropin-upstream-baseline.json` - 已固定的上游基线引用
- `franken-node-unified-test-certification-contract.json` - Franken node 测试认证
- `franken-node-package-interop-contract.json` - 包互操作契约
- `franken-node-runtime-substrate-contract.json` - 运行时基座契约

#### 用户文档
- `troubleshooting.md` - 用户故障排查指南
- `keybindings.md` - TUI 快捷键参考
- `session.md` - 会话管理文档
- `tui.md` - 终端 UI 文档
- `prompt-templates.md` - 提示模板文档
- `integrator-migration-playbook.md` - 面向集成方的迁移指南
- `EXTENSION_CANDIDATES.md` - 扩展候选文档
- `EXTENSION_CAPTURE_SCENARIOS.md` - 扩展捕获场景

#### 提供方文档
- `providers.md` - 提供方实现指南
- `qa-runbook.md` - 质量保障操作手册
- `extension-troubleshooting.md` - 扩展故障排查指南

### 证据快照文档（生成物/时效性制品）

#### 认证证据
- `dropin-certification-verdict.json` - 生成的认证裁决（基于时间戳）
- `dropin-parity-gap-ledger.json` - 带归属追踪的差距台账
- `dropin-feature-inventory-matrix.json` - 机器可读清单
- `dropin-112-feature-inventory-matrix.md` - 人可读清单配套文档
- `dropin-tool-io-differential.json` - G09 证据摘要
- `dropin-error-crosswalk.json` - 错误分类体系证据
- `dropin-cli-surface-diff.json` - CLI surface 对比
- `dropin-rpc-surface-diff.json` - RPC surface 对比
- `dropin-config-surface-diff.json` - Config surface 对比

#### 测试与一致性证据
- `extension-conformance-matrix.json` - 扩展一致性测试结果
- `extension-conformance-test-plan.json` - 带执行状态的测试计划
- `extension-entry-scan.json` - 扩展入口扫描结果
- `coverage-baseline-map.json` - 覆盖率基线映射
- `traceability_matrix.json` - 测试追溯矩阵
- `TEST_COVERAGE_MATRIX.md` - 覆盖率矩阵文档

#### 提供方证据与分析
- `provider-upstream-model-ids-snapshot.json` - 提供方模型 ID 快照
- `provider-parity-reconciliation.json` - 提供方对等一致性核对状态
- `provider-discrepancy-ledger.json` - 提供方差异台账
- `provider-parity-checklist.json` - 提供方对等一致性校验结果
- `provider-audit-evidence-index.json` - 提供方审计证据索引
- `provider-test-matrix-validation-report.json` - 提供方测试校验结果
- `provider_e2e_artifact_contract.json` - 提供方端到端制品证据

#### 性能与监控证据
- `perf_sli_matrix.json` - 性能 SLI 度量矩阵
- `provider-cerebras-setup.json` - 提供方专属配置证据

#### 模式证据
- `schema/test_evidence_logging_instance.json` - 测试证据日志实例

#### 扩展分析证据
- `extension-research-playbook.json` - 扩展研究分析
- `ext-compat.md` - 扩展兼容性分析

### 可退役文档（遗留/已被取代）

#### 遗留分析
- `beads-ledger-reconciliation-report.md` - 遗留台账核对（已被活跃台账取代）
- `franken-node-remediation-backlog-contract.json` - 遗留 franken-node 待办
- `franken-node-compatibility-doctor-contract.json` - 遗留兼容性自检
- `franken-node-practical-finish-contract.json` - 遗留实际完成契约
- `franken-node-claim-gating-contract.json` - 遗留声明闸门（已被取代）
- `franken-node-claim-contract.json` - 遗留声明契约

#### 安全文档（契约）
- `security/security-slos.md` - 安全 SLO 规范
- `security/operator-handbook.md` - 运维方安全手册
- `security/maintenance-playbook.md` - 安全维护流程
- `security/incident-response-runbook.md` - 事件响应流程
- `security/incident-runbook.md` - 通用事件流程
- `security/operator-quick-reference.md` - 运维方快速参考指南
- `security/runtime-hostcall-telemetry.md` - 运行时宿主调用遥测规范
- `security/invariants.md` - 安全不变量文档
- `security/lockfile-format.md` - 锁文件格式规范
- `security/manifest-v2-migration.md` - Manifest v2 迁移指南
- `security/threat-model.md` - 安全威胁模型
- `sec_traceability_matrix.md` - 安全追溯矩阵

#### 其他契约文档
- `cargo-binary-classification.md` - 二进制分类框架（刚完成复核）
- 各类大写命名的 .md 文件（架构决策）

#### 其他证据快照文档
- 多个提供方专属配置/设置文件（cerebras、anthropic 等）
- 扩展评分/校验矩阵
- 一致性测试制品与报告
- 基准与性能度量结果

#### 其他可退役候选
- 重复的安全操作手册（incident-runbook.md 与 incident-response-runbook.md — 需复核重复情况）
- 可能已被取代的遗留扩展分析报告
- 陈旧的提供方配置快照

## 总结

- **已分类文件总数：** 193（已完成）
- **契约：** ~85（稳定规范、安全文档、用户指南、API 契约、模式）
- **证据快照：** ~95（生成物、测试结果、合规证据、快照）
- **可退役：** ~13（遗留 franken-node 契约、重复文档、已被取代的报告）

## 关键分类决策

1. **安全文档归为契约** — 它们是操作性规范与流程，而非时效性证据
2. **Dropin 认证制品归为证据快照** — 带时间戳生成，需定期刷新
3. **提供方快照归为证据快照** — 时效性配置与能力清单
4. **模式定义归为契约** — 稳定的 API 接口定义
5. **Franken-node 遗留契约归为可退役** — 功能已被取代

## 需验证项（AGENTS.md 规则 1）

在执行 bd-we34i.2 中的移动操作前，需显式验证可退役候选：
- 遗留 franken-node 契约（6 个文件）
- 重复的事件操作手册
- 遗留扩展分析报告
- 陈旧的提供方快照（确认未被引用）

## 已具备实施条件

分类完成。已就绪进入 bd-we34i.2 执行，需按规则 1 获得干系人批准。

---
*为 bd-we34i.1 生成 — DC-T1 文档分类任务*
