# 扩展语料刷新清单

刷新扩展语料的分步流程：发现新扩展、验证扩展、运行一致性与性能测试，并更新所有下游制品。任何工程师无需额外上下文即可执行。

---

## 前置条件

- Rust nightly 工具链（用于 `cargo test --features ext-conformance`）
- Bun 1.3+（`~/.bun/bin/bun`）用于 TS oracle
- `jq` 用于 JSON 处理
- 访问 GitHub 与 npm registry 以执行发现扫描

---

## 阶段 1：发现（新候选）

### 1.1 上游同步

```bash
# Pull latest pi-mono upstream examples
cd legacy_pi_mono_code/pi-mono
git pull origin main
```

在以下路径下检查新扩展：
- `packages/coding-agent/examples/extensions/`
- `.pi/extensions/`

### 1.2 GitHub 发现扫描

在 GitHub 上搜索新的 Pi 扩展：

```bash
# Keyword searches (adjust date range)
gh search repos "pi extension" --language=typescript --sort=updated
gh search repos "pi-agent extension" --sort=updated
gh search code "registerTool" "pi.registerTool" --language=typescript
```

将新候选记录到 `docs/EXTENSION_CANDIDATES.md`。

### 1.3 npm registry 扫描

```bash
# Search npm for Pi extension packages
npm search pi-extension
npm search @anthropic/pi
```

### 1.4 与现有语料去重

使用规范源键（canonical source key）+ 内容校验和策略，将新候选与 `docs/extension-master-catalog.json` 对比（见 EXTENSIONS.md 第 1C.3 节）。

移除重复项。将真正新的候选加入候选池。

---

## 阶段 2：获取（下载与整理）

### 2.1 下载新扩展

将源文件置于对应的语料目录下：

| 来源层级 | 目录 |
|---|---|
| `official-pi-mono` | `tests/ext_conformance/artifacts/plugins-official/` |
| `community` | `tests/ext_conformance/artifacts/community/` |
| `npm-registry` | `tests/ext_conformance/artifacts/npm/` |
| `third-party-github` | `tests/ext_conformance/artifacts/plugins-community/` |

### 2.2 记录溯源

为每个新扩展记录：
- 源 URL / 仓库 / npm 包
- 版本 / commit 哈希
- 许可证
- 文件数量与总字节数

使用新条目更新 `docs/extension-artifact-provenance.json`。

---

## 阶段 3：TS Oracle 验证（基准真值）

### 3.1 在新扩展上运行 TS oracle

```bash
cd tests/ext_conformance/ts_oracle

# Single extension
bun run load_extension.ts /path/to/new_extension.ts

# Batch (all new)
bash batch_load.sh /path/to/new_extensions_dir/
```

oracle 会记录：加载成功/失败、已注册的工具、命令、钩子、标志、提供方、快捷键。

### 3.2 更新 VALIDATED_MANIFEST.json

将 oracle 输出合并到 `tests/ext_conformance/VALIDATED_MANIFEST.json`。

每个条目需要：
- `id`：稳定的扩展标识符
- `source_tier`：溯源层级
- `entry_path`：指向扩展源文件的相对路径
- `expected_snapshot`：oracle 的注册输出（工具、命令等）

### 3.3 验证清单完整性

```bash
# Count entries
jq '.extensions | length' tests/ext_conformance/VALIDATED_MANIFEST.json

# Check for duplicate IDs
jq '[.extensions[].id] | group_by(.) | map(select(length > 1)) | length' \
  tests/ext_conformance/VALIDATED_MANIFEST.json
# Should output: 0
```

---

## 阶段 4：一致性测试（Rust 运行时）

### 4.1 重新生成一致性测试文件

如果 `tests/ext_conformance_generated.rs` 中的 `conformance_test!` 宏条目需要更新（已添加新扩展），请从清单重新生成。

### 4.2 运行全部一致性测试

```bash
cargo test --test ext_conformance_generated --features ext-conformance -- --nocapture
```

### 4.3 生成完整一致性报告

```bash
cargo test --test ext_conformance_generated conformance_full_report \
  --features ext-conformance -- --nocapture
```

此命令会生成：
- `tests/ext_conformance/reports/conformance_events.jsonl`
- `tests/ext_conformance/reports/conformance_summary.json`
- `tests/ext_conformance/reports/CONFORMANCE_REPORT.md`

### 4.4 更新一致性基线

```bash
# Review changes
diff <(jq . tests/ext_conformance/reports/conformance_baseline.json) \
     <(jq . /tmp/new_baseline.json)
```

使用新的计数与失败分类更新 `tests/ext_conformance/reports/conformance_baseline.json`。

### 4.5 对新增失败进行分类

对于每个新增失败，确定根本原因并添加到基线中对应的分类：

| 分类 | 描述 |
|---|---|
| `manifest_registration_mismatch` | Oracle 与 Rust 注册的条目不一致 |
| `missing_npm_package` | 扩展导入了我们未打桩的 npm 包 |
| `multi_file_dependency` | 扩展从同级目录导入 |
| `runtime_error` | JS 在加载/注册期间抛出异常 |
| `test_fixture` | 非真实扩展（测试基础设施） |

在编辑基线之前，先生成确定性的分流报告：

```bash
python3 scripts/summarize_ext_conformance_failures.py \
  --out-json /tmp/ext-conformance-triage.json \
  --out-md /tmp/ext-conformance-triage.md
```

该报告会合并重复的失败签名，标注已知基线与新增/未跟踪失败，标记陈旧的基线，并包含可直接用于 Beads 的标题、标签、正文以及 RCH 复现命令，以便后续修复。

---

## 阶段 5：性能基准测试

### 5.1 运行 PR 基准（快速检查）

```bash
PI_BENCH_MODE=pr cargo test --test ext_bench_harness \
  --features ext-conformance -- --nocapture
```

### 5.2 运行 nightly 基准（全量语料）

```bash
PI_BENCH_MODE=nightly PI_BENCH_MAX=200 PI_BENCH_ITERATIONS=10 \
  cargo test --test ext_bench_harness --features ext-conformance -- --nocapture
```

此命令会生成：
- `tests/perf/reports/ext_bench_baseline.json`
- `tests/perf/reports/BASELINE_REPORT.md`
- `tests/perf/reports/budget_summary.json`

### 5.3 检查预算合规性

```bash
cargo test --test perf_budgets --features ext-conformance -- --nocapture
```

`tests/perf_budgets.rs` 中的所有预算必须通过。关键阈值：

| 预算 | 阈值 |
|---|---|
| 冷加载 P95（跨扩展） | < 200ms |
| 热加载 P95 | < 100ms |
| 事件分发 P99 | < 5ms |

### 5.4 调查回归

如果某项预算未通过，对比上一版基线：

```bash
# Side-by-side P95 comparison
jq '.scenarios.cold_load.p95_us' tests/perf/reports/ext_bench_baseline.json
```

检查是否存在异常缓慢的新扩展（冷加载中的离群点）。

---

## 阶段 6：目录与文档更新

### 6.1 更新扩展目录

更新 `docs/extension-catalog.json` 以包含新扩展。每个条目需要（依据 `docs/extension-catalog.schema.json`）：

**必填字段：**
- `id`、`name`、`source_tier`、`source`（git/npm/url 引用）
- `runtime_tier`（`legacy-js` / `multi-file` / `pkg-with-deps`）
- `interaction_tags`、`capabilities`、`io_pattern`、`complexity`
- `file_count`、`total_bytes`、`checksum.sha256`

**可选字段（从测试结果填充）：**
- `compatibility_notes.conformance_status`（`pass` / `fail`）
- `compatibility_notes.conformance_tier`（1-5）
- `compatibility_notes.failure_category`（如失败）
- `perf_budgets.cold_load_ms`（来自基准测试基线）

### 6.2 根据 schema 验证目录

```bash
# Use ajv or similar JSON Schema validator
npx ajv validate -s docs/extension-catalog.schema.json \
  -d docs/extension-catalog.json
```

### 6.3 更新 COMPATIBILITY_SUMMARY.md

使用一致性与性能运行的新数据重新生成 `tests/ext_conformance/reports/COMPATIBILITY_SUMMARY.md`。

### 6.4 更新 EXTENSIONS.md

使用新的通过/失败计数更新 EXTENSIONS.md 第 1C.5 节中的“已达成覆盖率”表格。

### 6.5 更新 README.md

如果整体通过率发生显著变化，更新 README.md 中的扩展章节。

---

## 阶段 7：提交与验证

### 7.1 验证无回归

```bash
# Regression check: no previously-passing extension should now fail
# Compare old baseline pass list against new results
```

一致性基线包含 `regression_thresholds`：
- Tier 1（简单）：必须保持 100%
- Tier 2（多注册）：必须保持 >= 95%
- 整体：必须保持 >= 80%
- 每个刷新周期最多新增 3 个失败

### 7.2 提交制品

暂存并提交以下文件：

```bash
# Core artifacts
git add tests/ext_conformance/VALIDATED_MANIFEST.json
git add tests/ext_conformance_generated.rs
git add docs/extension-catalog.json

# Reports (tracked via .gitignore negation rules)
git add tests/ext_conformance/reports/conformance_baseline.json
git add tests/ext_conformance/reports/conformance_summary.json
git add tests/ext_conformance/reports/CONFORMANCE_REPORT.md
git add tests/ext_conformance/reports/COMPATIBILITY_SUMMARY.md

# Documentation
git add EXTENSIONS.md README.md

git commit -m "chore(extensions): refresh corpus (N new, M total, X% pass)"
```

### 7.3 推送

```bash
git push origin main && git push origin main:master
```

---

## 退出标准

当以下所有条件均为真时，刷新完成：

- [ ] 所有新候选均已通过 TS oracle
- [ ] `VALIDATED_MANIFEST.json` 已使用新条目更新
- [ ] 所有一致性测试均已运行（无测试基础设施失败）
- [ ] 一致性基线已使用新计数更新
- [ ] 无回归：此前通过的扩展仍保持通过
- [ ] 性能基准已运行；预算通过（或已记录回归）
- [ ] `docs/extension-catalog.json` 包含带有一致性状态的新条目
- [ ] 目录通过 `docs/extension-catalog.schema.json` 校验
- [ ] `COMPATIBILITY_SUMMARY.md` 已反映新数据
- [ ] `EXTENSIONS.md` 第 1C.5 节覆盖率表格已更新
- [ ] 所有变更已提交并推送

---

## 每次刷新更新的制品

| 制品 | 用途 | 位置 |
|---|---|---|
| Validated manifest | 基准真值注册信息 | `tests/ext_conformance/VALIDATED_MANIFEST.json` |
| Generated tests | Rust 一致性测试用例 | `tests/ext_conformance_generated.rs` |
| Conformance baseline | 通过/失败计数与失败分类 | `tests/ext_conformance/reports/conformance_baseline.json` |
| Conformance summary | 机器可读摘要 | `tests/ext_conformance/reports/conformance_summary.json` |
| Conformance report | 人类可读的逐扩展结果 | `tests/ext_conformance/reports/CONFORMANCE_REPORT.md` |
| Compatibility summary | 组合的一致性与性能概览 | `tests/ext_conformance/reports/COMPATIBILITY_SUMMARY.md` |
| Perf baseline | 逐扩展加载时间分位数 | `tests/perf/reports/ext_bench_baseline.json` |
| Budget summary | 预算通过/失败结果 | `tests/perf/reports/budget_summary.json` |
| Extension catalog | 富化元数据（223+ 条目） | `docs/extension-catalog.json` |
| Artifact provenance | 源跟踪与许可证 | `docs/extension-artifact-provenance.json` |
| EXTENSIONS.md | 含覆盖率表格的架构文档 | `EXTENSIONS.md` |
| README.md | 面向用户的扩展状态 | `README.md` |

---

## 刷新节奏与触发条件

### 计划节奏

**季度刷新**（推荐）：每季度运行一次完整流水线。这在新鲜度与工程成本之间取得平衡。扩展生态变化不够快，不足以证明每月运行的合理性。

建议时间表：
- Q1（1 月）：节后生态补齐
- Q2（4 月）：年中扫描
- Q3（7 月）：会前季
- Q4（10 月）：年末稳定化

### 触发事件（非计划刷新）

当发生以下任一情况时，立即运行刷新：

| 触发条件 | 范围 | 原因 |
|---|---|---|
| **Pi 上游大版本发布** | 全量刷新 | 新 API 可能破坏或启用扩展 |
| **QuickJS 运行时升级** | 仅一致性 | 引擎变更可能影响垫片行为 |
| **新增 Node API 垫片** | 仅一致性 | 此前失败的扩展现在可能通过 |
| **安全事件** | 定向（受影响的扩展） | 必须验证语料中无恶意负载 |
| **扩展生态事件** | 发现与验证 | 大量新的社区扩展 |
| **CI 中检测到回归** | 定向调查 | 预算失败或一致性下降 |

### 紧急刷新标准

当出现以下情况时，需要进行紧急（当日）刷新：
- 此前通过的官方扩展现在失败（T1/T2 回归）
- 在语料扩展中发现安全漏洞
- 整体通过率降至 80% 以下（回归阈值）

### 归属

触发刷新的工程师需负责直至完成。其必须：
1. 端到端遵循本清单
2. 不得跳过退出标准
3. 在提交信息中记录任何偏离

对于计划内刷新，至少在目标日期前 1 周分配负责人，以便为发现扫描留出时间。

---

## 扩展提案接收

在刷新周期之间，应系统化跟踪新的扩展候选，而非临时添加。

### 提案模板

提议将新扩展纳入语料时，请记录：

```
Extension: <name>
Source: <URL or package reference>
Source tier: <official / community / npm / third-party>
Reason: <why add this — unique API surface, popular, covers gap in coverage>
Evidence: <link to repo, npm page, or usage data>
Priority: <high = covers uncovered capability / low = incremental>
```

### 分流规则

- **高优先级**：覆盖当前语料中未充分代表的能力或交互标签（检查 `docs/extension-catalog.json` 中的缺口）。
- **中优先级**：热门扩展或来自新来源层级的扩展。
- **低优先级**：在行为/能力上与现有扩展相似。
- **拒绝**：重复、已废弃（12 个月以上无提交）或使用被禁止 API 的扩展（见 EXTENSIONS.md §2A.4）。

### 将提案移入刷新

在下一次计划刷新（阶段 1）期间，审查所有待处理提案，并将高/中优先级的提案纳入发现扫描。低优先级提案顺延至下一周期，除非语料仍有容量。

---

## 自动化钩子

### CI 一致性门禁

添加到 `.github/workflows/ci.yml`（或等效文件）以在每个 PR 上捕获回归：

```yaml
# Extension conformance (PR subset)
- name: Extension conformance check
  run: |
    cargo test --test ext_conformance_generated --features ext-conformance \
      -- --test-threads=1 -q
  env:
    PI_TEST_MODE: "1"
```

这会运行全部 223 个一致性测试。在快速 CI 运行器上耗时约 2 分钟（debug 构建）。如需更快反馈，仅运行 Tier 1+2 测试：

```bash
cargo test --test ext_conformance_generated "tier_[12]_" \
  --features ext-conformance -- -q
```

### CI 性能预算门禁

```yaml
# Extension performance budgets
- name: Extension perf budget check
  run: |
    PI_BENCH_MODE=pr cargo test --test ext_bench_harness \
      --features ext-conformance -- --nocapture
    cargo test --test perf_budgets --features ext-conformance -- -q
```

### 陈旧检测

一致性基线记录了 `generated_at` 时间戳。一个简单的陈旧检查：

```bash
#!/bin/bash
# check_extension_staleness.sh
BASELINE="tests/ext_conformance/reports/conformance_baseline.json"
GENERATED=$(jq -r '.generated_at' "$BASELINE")
DAYS_OLD=$(( ($(date +%s) - $(date -d "$GENERATED" +%s)) / 86400 ))

if [ "$DAYS_OLD" -gt 90 ]; then
  echo "WARNING: Extension conformance baseline is ${DAYS_OLD} days old."
  echo "Consider running a refresh (see docs/EXTENSION_REFRESH_CHECKLIST.md)."
  exit 1
fi
echo "Extension baseline is ${DAYS_OLD} days old (within 90-day window)."
```

在 CI 中按周计划运行此脚本，以便在语料接近陈旧时获得早期预警。

### 按需刷新

要在计划节奏之外触发刷新：

```bash
# 1. Run conformance to see current state
cargo test --test ext_conformance_generated conformance_full_report \
  --features ext-conformance -- --nocapture

# 2. Run benchmarks
PI_BENCH_MODE=nightly PI_BENCH_MAX=200 PI_BENCH_ITERATIONS=10 \
  cargo test --test ext_bench_harness --features ext-conformance -- --nocapture

# 3. Follow the full checklist from Phase 1
# See docs/EXTENSION_REFRESH_CHECKLIST.md
```

### 回归告警

当 CI 检测到一致性或预算失败时，责任工程师应：

1. 检查 `tests/ext_conformance/reports/conformance_events.jsonl` 中失败的扩展。
2. 判断失败是代码变更（我们的回归）还是测试基础设施问题。
3. 如果是我们的回归：修复代码，重新运行一致性测试，验证通过。
4. 如果是测试基础设施：更新清单或夹具，并在提交信息中记录变更。
