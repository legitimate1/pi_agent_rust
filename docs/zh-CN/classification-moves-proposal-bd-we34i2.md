# 分类移动提案 — bd-we34i.2

为 DC-T2 生成：经用户按 AGENTS.md 规则 1 显式批准后执行分类移动

## 规则 1 合规：需批准的可退役候选

### 遗留 Franken-Node 契约 (6 个文件)
**申请批准后移除：**
- `docs/franken-node-remediation-backlog-contract.json` — 已被取代的遗留待办
- `docs/franken-node-compatibility-doctor-contract.json` — 遗留兼容性检测
- `docs/franken-node-practical-finish-contract.json` — 遗留实际收尾
- `docs/franken-node-claim-gating-contract.json` — 已被取代的遗留声明门控
- `docs/franken-node-claim-contract.json` — 遗留声明契约
- `docs/franken-node-unified-test-certification-contract.json` — 可能仍活跃 — 需核实

### 潜在重复（需复核）
**申请批准后合并：**
- `docs/security/incident-runbook.md` vs `docs/security/incident-response-runbook.md` — 检查是否重复
- 遗留扩展分析报告 — 复核是否已被取代的版本

## 安全的组织性移动（无需规则 1）

### 证据归集
```bash
# 为快照创建 evidence 子目录
mkdir -p docs/evidence/
# 已移动: docs/evidence/dropin-certification-verdict.json
git mv docs/dropin-parity-gap-ledger.json docs/evidence/
git mv docs/dropin-feature-inventory-matrix.json docs/evidence/
git mv docs/dropin-*-diff.json docs/evidence/
git mv docs/provider-*-snapshot.json docs/evidence/
```

### 契约归集
```bash
# 为规约创建 contracts 子目录
mkdir -p docs/contracts/
git mv docs/dropin-certification-contract.json docs/contracts/
git mv docs/dropin-upstream-baseline.json docs/contracts/
```

## 实施计划

**阶段 1：安全移动（立即执行）**
- 将证据文件归集至 docs/evidence/
- 将契约文件归集至 docs/contracts/
- 更新文档中的内部引用

**阶段 2：等待批准**
- 移除遗留 franken-node 契约（待批准）
- 合并重复文件（待复核）

## 提交策略
1. 执行安全的组织性移动
2. 以 "organize docs structure" 为信息提交
3. 提出可退役候选以获取规则 1 批准
4. 获显式批准后执行移除

---
**状态**: 就绪，可执行安全移动 + 申请规则 1 批准
