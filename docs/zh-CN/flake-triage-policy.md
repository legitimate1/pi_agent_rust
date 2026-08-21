# 不稳定用例分流与重跑策略

## 目的

即使是确定性的测试套件也会遇到环境性失败（资源耗尽、文件系统争用、超时抖动）。本策略定义了一致性 CI 如何对这些失败进行分类、重试与不稳定用例分流，且不掩盖真实的回归。

## 失败分类

每次测试失败都会被归入以下三类之一：

| 类别 | 判定标准 | 处置 |
|--------|----------|--------|
| **确定性** | 在干净检出上本地可复现 | 立即修复；阻塞合并 |
| **瞬时性** | 重试后不可复现；匹配已知不稳定模式 | 自动重试一次；记录以便追踪 |
| **环境性** | 基础设施层面（OOM、磁盘已满、网络超时） | 带退避重试；重复出现时告警 |

## 已知不稳定模式

以下模式被识别为瞬时性，符合自动重试条件：

| 模式 | 正则 | 分类 |
|---------|-------|----------|
| TS oracle 超时 | `oracle.*timed?\s*out\|bun.*timed?\s*out` | `oracle_timeout` |
| 资源耗尽 | `out of memory\|ENOMEM\|Cannot allocate` | `resource_exhaustion` |
| 文件系统争用 | `EBUSY\|ETXTBSY\|resource busy` | `fs_contention` |
| 端口冲突 | `EADDRINUSE\|address already in use` | `port_conflict` |
| 临时目录清理竞态 | `No such file or directory.*tmp` | `tmpdir_race` |
| QuickJS GC 压力 | `out of memory.*quickjs\|allocation failed` | `js_gc_pressure` |

## 重试策略

### CI 重试规则

1. **最大重试次数**：每次运行中每个测试目标自动重试 1 次。
2. **重试范围**：仅重试失败的测试目标，不重试整个矩阵行。
3. **重试间隔**：尝试之间间隔 5 秒（避免立即重试导致的资源争用）。
4. **第二次尝试仍失败**：报告为真实失败。

### 本地重试（`scripts/e2e/run_all.sh`）

使用 `--rerun-from <summary.json>` 仅重新执行上一次运行中失败的套件。此方式使用相同的分类逻辑。

## 隔离契约（CI 强制执行）

隔离元数据位于 `tests/suite_classification.toml` 的 `[quarantine.<test_stem>]` 小节中，并由 CI 校验。

每个条目的必填字段：

- `category` (`FLAKE-TIMING`, `FLAKE-ENV`, `FLAKE-NET`, `FLAKE-RES`, `FLAKE-EXT`, `FLAKE-LOGIC`)
- `owner`
- `quarantined`
- `expires`
- `bead`
- `evidence`（CI 运行 URL 或产物路径）
- `repro`（精确的复现命令）
- `reason`
- `remove_when`（客观的退出条件）

策略边界：

- 最大隔离窗口：14 天（`quarantined` → `expires`）
- 过期条目会立即使 CI 失败
- 2 天内即将过期的条目会以升级告警的形式提示

CI 产生的审计输出：

- `tests/quarantine_report.json`（机器可读的汇总 + 升级状态）
- `tests/quarantine_audit.jsonl`（便于追加的逐条目审计记录）

## 不稳定预算

- **单目标不稳定预算**：每个测试目标在滚动 30 天窗口内最多允许出现 3 次不稳定现象，超出则需介入调查。
- **全局不稳定预算**：所有目标的不稳定总数必须保持在总测试执行次数的 5% 以下。
- **超出预算**：当某个目标超出其不稳定预算时，将从“瞬时性”升级为“确定性”，必须修复或作为已知限制进行文档化。

## 分流工作流

1. **CI 在一致性任务中检测到失败**。
2. **分类器**将失败输出与已知不稳定模式进行比对。
3. 若匹配：重试一次，并将 `flake_event` 记录到 JSONL。
4. 若无匹配或重试仍失败：标记为**确定性失败**。
5. **每周复盘**：聚合不稳定事件。若有目标超出预算，则创建一个 Beads 进行调查。

## 证据产物

每次一致性运行都会产生：

| 产物 | 格式 | 内容 |
|----------|--------|---------|
| `conformance-*.log` | Text | 完整测试输出 |
| `flake_events.jsonl` | JSONL | 已分类的不稳定事件 |
| `conformance_summary.json` | JSON | 通过/失败/跳过计数 |
| `retry_manifest.json` | JSON | 哪些目标被重试及结果 |
| `quarantine_report.json` | JSON | 隔离策略状态与升级情况 |
| `quarantine_audit.jsonl` | JSONL | 逐条目的 owner/过期时间/证据/repro 轨迹 |

## 与质量流水线集成

`scripts/ext_quality_pipeline.sh` 脚本**不**执行自动重试。它按原样报告失败，以提供确定性的本地反馈。重试逻辑仅存在于 CI 中（`.github/workflows/conformance.yml`）。

## 配置

| 变量 | 默认值 | 说明 |
|----------|---------|-------------|
| `PI_CONFORMANCE_MAX_RETRIES` | `1` | 每个目标的最大自动重试次数 |
| `PI_CONFORMANCE_RETRY_DELAY` | `5` | 重试尝试之间的秒数 |
| `PI_CONFORMANCE_FLAKE_BUDGET` | `3` | 单目标 30 天不稳定预算 |
| `PI_CONFORMANCE_CLASSIFY_ONLY` | `0` | 设为 `1` 则仅分类而不重试 |
