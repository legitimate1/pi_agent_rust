# Beads ↔ 台账对账报告

**生成时间**: 2026-04-22T19:05:00Z  
**对账脚本**: `scripts/reconcile_beads_ledger.sh`  
**任务**: `bd-wz2sg.2` (TL-T2: 为每个开放的 critical/high 台账条目创建对应的文件 Beads 实现初始对账)

## 摘要

对账脚本识别出 **3 个孤儿台账缺口**，需要创建对应的 Beads 以防止 bead 完成假象。

## 需要创建 Beads 的孤儿缺口

### 1. gap-json-auto-lifecycle-events
- **严重级别**: high
- **领域**: json-mode
- **预期归属 Bead**: `bd-2my4b` (缺失/已关闭)
- **描述**: JSON 模式自动生命周期事件（压缩、重试）需要与 TS 实现保持 schema 与时序一致
- **影响**: 消费 JSON 流的自动化可能错误处理生命周期转换

**需创建的 Bead**:

```bash
br create --title "JSON-T1: Auto lifecycle events parity — close gap-json-auto-lifecycle-events" \
  --priority 0 --type task --labels json-mode,dropin,lifecycle,g05 \
  "JSON mode auto lifecycle events (compaction, retry) need schema and ordering parity with TS implementation. Automation consuming JSON streams can mis-handle lifecycle transitions. References gap-json-auto-lifecycle-events in dropin-parity-gap-ledger.json."
```

### 2. gap-json-tool-and-extension-ui-events
- **严重级别**: high
- **领域**: json-mode
- **预期归属 Bead**: `bd-359pl` (缺失/已关闭)
- **描述**: 工具执行与扩展 UI 事件一致性校验
- **影响**: 扩展驱动的 UX 流程可能在 JSON/RPC 集成中出现分歧或失败

**需创建的 Bead**:

```bash
br create --title "JSON-T2: Tool and extension UI events parity — close gap-json-tool-and-extension-ui-events" \
  --priority 0 --type task --labels json-mode,dropin,extension-ui,g05 \
  "Tool execution event names and extension_ui_request/response round-trips need validated parity with TS. Extension-driven UX flows can diverge or fail in JSON/RPC integrations. References gap-json-tool-and-extension-ui-events in dropin-parity-gap-ledger.json."
```

### 3. gap-tool-io-limit-divergence
- **严重级别**: high
- **领域**: tools
- **预期归属 Bead**: `bd-2xalc` (缺失/已关闭)
- **描述**: 工具 I/O 限制默认值需要与 TS 行为对齐
- **影响**: 相同的工具调用可能以不同方式截断或失败，破坏自动化假设

**需创建的 Bead**:

```bash
br create --title "TOOLS-T1: I/O limit divergence alignment — close gap-tool-io-limit-divergence" \
  --priority 0 --type task --labels tools,dropin,limits,g05 \
  "Tool I/O limit defaults need alignment with TS DEFAULT_MAX_BYTES behavior. Same tool call can truncate or fail differently, breaking automation assumptions. References gap-tool-io-limit-divergence in dropin-parity-gap-ledger.json."
```

## 对账状态

- **开放的 Critical/High 缺口总数**: 6
- **已匹配 Beads 的缺口**: 3
- **孤儿缺口**: 3 (需创建 bead)
- **Bead 孤儿**: 0

## 后续动作

1. 执行上述三条 `br create` 命令以创建缺失的 Beads
2. 使用新的 bead id 更新台账条目的 `owner_issue_primary`
3. 重新运行对账脚本以验证孤儿数为 0
4. 关闭 `bd-wz2sg.2` 任务

## 验证命令

创建 bead 后：

```bash
./scripts/reconcile_beads_ledger.sh
# 预期: 退出码 0 并显示 "no orphans found"
```

---

*本报告通过确保所有 critical/high 缺口均有对应的活跃 Beads 进行跟踪，来消除 bead 完成假象。*
