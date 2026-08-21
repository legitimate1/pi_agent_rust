# 一致性 Harness 运维手册

本手册介绍如何运行、调试与解读扩展一致性测试套件。它是在本地或 CI 中运维一致性 harness 的主要参考。

## 快速开始

```bash
# 运行快速一致性检查（5 个官方扩展）
PI_OFFICIAL_MAX=5 cargo test --test ext_conformance_diff \
  --features ext-conformance -- --nocapture

# 运行完整的 223 扩展全量任务
cargo test --test ext_conformance_generated conformance_full_report \
  --features ext-conformance -- --nocapture

# 运行场景一致性
cargo test --test ext_conformance_scenarios \
  --features ext-conformance scenario_conformance_suite -- --nocapture

# 生成运行时 API 矩阵报告
cargo test --test ext_conformance_matrix \
  generate_runtime_api_matrix_report -- --nocapture
```

## 前置条件

### Bun 安装

TS 语义对照需要 Bun 1.3.8+。harness 期望其位于 `/home/ubuntu/.bun/bin/bun`。

```bash
# 安装 Bun
curl -fsSL https://bun.sh/install | bash

# 或为已有安装创建软链接
ln -sf $(which bun) /home/ubuntu/.bun/bin/bun
```

### pi-mono 依赖

TS 语义对照通过遗留 pi-mono TypeScript 运行时加载扩展。其 npm 依赖必须已安装：

```bash
cd legacy_pi_mono_code/pi-mono
npm ci
```

### 功能开关

多数一致性测试需要 `ext-conformance` cargo 特性：

```bash
cargo test --features ext-conformance --test ext_conformance_diff
```

## 环境变量

| 变量 | 默认值 | 说明 |
|----------|---------|-------------|
| `PI_TEST_MODE` | 未设置 | 设为 `1` 时启用确定性时间戳与 CWD 归一化 |
| `PI_CONFORMANCE_SEED` | 未设置 | 确定性一致性差异运行的种子（如 `42`） |
| `PI_EXT_RANDOM_SEED` | `42` | `ext_random_trials` 确定性选取的种子 |
| `PI_EXT_RANDOM_N` | `1` | 有界的 `ext_random_trials` 抽样大小；仅在显式批量运行时提高 |
| `PI_EXT_RANDOM_FILTER` | 未设置 | 可选的 `ext_random_trials` 过滤器，如 `tier:1-3` 或 `source:community` |
| `PI_EXT_RANDOM_IDS` | 未设置 | 显式的、以逗号分隔的 `ext_random_trials` 扩展 id |
| `PI_EXT_RANDOM_OUTPUT_DIR` | `$TMPDIR/pi_agent_rust/ext_conformance/random_trials` | 覆盖 random-trial JSONL 与清单输出目录 |
| `PI_TS_ORACLE_TIMEOUT_SECS` | `30` | TS 语义对照对单个扩展的超时 |
| `PI_OFFICIAL_MAX` | 未设置 | 限制测试的官方扩展数量（如快速检查用 `5`） |
| `PI_DETERMINISTIC_CWD` | 自动 | 覆盖确定性工作目录 |
| `PI_DETERMINISTIC_HOME` | 自动 | 覆盖确定性 home 目录 |
| `PI_DETERMINISTIC_TIME_MS` | 自动 | 确定性输出的固定时间戳 |
| `PI_DETERMINISTIC_TIME_STEP_MS` | 自动 | 每次 `Date.now()` 调用的时间增量 |
| `PI_DETERMINISTIC_RANDOM` | 自动 | 固定随机值（覆盖种子） |
| `PI_DETERMINISTIC_RANDOM_SEED` | 自动 | 确定性 PRNG 的种子 |
| `RUST_TEST_THREADS` | `1` | 设为 `1` 以确定性串行执行 |
| `CARGO_TARGET_DIR` | `target` | 在多智能体环境中隔离构建产物（按智能体） |

## 测试套件

### 差异化一致性 (`ext_conformance_diff`)

核心一致性测试。在 TypeScript 语义对照（Bun + pi-mono）与 Rust QuickJS 运行时中分别运行每个扩展，然后比较输出。

```bash
# 全部官方扩展
cargo test --test ext_conformance_diff --features ext-conformance -- --nocapture

# 单个扩展（按测试名）
cargo test --test ext_conformance_diff diff_official_hello -- --nocapture

# 社区扩展（默认被忽略）
cargo test --test ext_conformance_diff --features ext-conformance -- --ignored --nocapture
```

**工作原理：**

1. 通过 Bun 将扩展文件加载到 TS 语义对照中
2. 将同一文件加载到 Rust QuickJS 运行时中
3. 对两份输出做归一化（时间戳、路径、随机值）
4. 逐字段比较输出
5. 差异产生 FAIL 并输出详细 diff

### 生成式一致性 (`ext_conformance_generated`)

对完整的 223 扩展语料库执行加载与注册测试：

```bash
# 完整报告
cargo test --test ext_conformance_generated conformance_full_report \
  --features ext-conformance -- --nocapture

# 生成式 tier 3-5 扩展测试的归属跟踪 opt-in 通道
cargo test --test ext_conformance_generated --features ext-conformance \
  -- --include-ignored --nocapture
```

生成的 `#[ignore]` 用例归属于 `bd-8t27h.17`；它们并非通用的未归档占位符。请将此通道中的失败视为语料库接入工作，并选择归档缺失产物、将扩展保留在带具体原因的跟踪 stretch tier，或提交更窄范围的归属 bead。

### 随机试验冒烟通道 (`ext_random_trials`)

从 Rust-N/A 池中运行确定性的随机子集。默认是单扩展冒烟运行，以使常规 `cargo test` 保持有界，输出默认位于 `TMPDIR` 下。

```bash
export CARGO_TARGET_DIR="/data/tmp/pi_agent_rust_cargo/${USER:-agent}/target"
export TMPDIR="/data/tmp/pi_agent_rust_cargo/${USER:-agent}/tmp"
mkdir -p "$CARGO_TARGET_DIR" "$TMPDIR"

PI_EXT_RANDOM_SEED=42 PI_EXT_RANDOM_N=1 \
  rch exec -- cargo test --test ext_random_trials \
    --features ext-conformance random_trials_batch -- --nocapture
```

如需更大范围的 opt-in 批量，请提高 `PI_EXT_RANDOM_N` 或传入 `PI_EXT_RANDOM_IDS=id-a,id-b`；结果将写入 `$TMPDIR/pi_agent_rust/ext_conformance/random_trials`，除非设置了 `PI_EXT_RANDOM_OUTPUT_DIR`。

### 场景一致性 (`ext_conformance_scenarios`)

测试在 JSON 夹具中定义的具体行为场景：

```bash
cargo test --test ext_conformance_scenarios \
  --features ext-conformance scenario_conformance_suite -- --nocapture
```

每个场景指定：
- 要加载的扩展
- 预期的注册形态（工具、标志、命令、事件钩子）
- 预期的宿主调用行为（exec 调用、会话操作、UI 事件）
- 预期的内容输出

### 运行时 API 矩阵 (`ext_conformance_matrix`)

校验 Node.js 与 Bun API 表面覆盖率：

```bash
# 检查关键条目通过
cargo test --test ext_conformance_matrix \
  runtime_api_matrix_node_critical_entries_pass -- --nocapture

# 生成完整矩阵报告
cargo test --test ext_conformance_matrix \
  generate_runtime_api_matrix_report -- --nocapture
```

### 负向策略测试 (`extensions_policy_negative`)

校验能力策略是否正确拒绝未授权操作：

```bash
cargo test --test extensions_policy_negative -- --nocapture
```

### 能力拒绝矩阵 (`capability_denial_matrix`)

测试策略配置与能力请求的全部组合：

```bash
cargo test --test capability_denial_matrix -- --nocapture
```

## 解读结果

### 状态码

| 状态 | 含义 |
|--------|---------|
| `PASS` | 扩展行为在 TS 语义对照与 Rust 运行时之间一致 |
| `FAIL` | 检测到行为差异（见 diff 输出） |
| `N/A` | 扩展尚未测试（通常为 community/npm/third-party 层级） |
| `SKIP` | 因 harness 能力缺失而跳过测试 |
| `ERROR` | 测试基础设施故障（非扩展问题） |

### 一致性摘要 (`conformance_summary.json`)

```json
{
  "schema": "pi.ext.conformance_summary.v2",
  "counts": { "pass": 56, "fail": 4, "na": 163, "total": 223 },
  "pass_rate_pct": 93.33,
  "per_tier": { ... },
  "evidence": { "golden_fixtures": 16, "parity_logs": 16, ... }
}
```

关键指标：
- `pass_rate_pct`: 计算为 `pass / (pass + fail) * 100`（不含 N/A）
- `per_tier`: 按扩展来源层级的细分
- `evidence`: 生成的证据产物数量

### 失败分桶

常见失败类别及其根因：

| 分桶 | 根因 | 修复 |
|--------|-----------|-----|
| `multi_file_dependency` | 扩展通过相对路径导入同级文件 | 实现相对说明符解析 |
| `runtime_error` | 扩展在加载/激活期间抛出 | 检查缺失的 shim 或 API 表面 |
| `host_read_policy_denial` | `readFileSync` 被宿主读取回退拦截 | 扩展读取了允许根之外的路径 |
| `package_module_specifier` | `require("some-npm-pkg")` 未被桩 | 添加虚拟模块桩 |
| `test_fixture` | 非真实扩展（测试基础设施产物） | 忽略 |

### 阅读 Diff 输出

当测试失败时，输出会显示逐字段差异：

```
DIFF: extension "hello" field "tools[0].description"
  TS oracle:  "Say hello to someone"
  Rust:       "Say hello"
```

请判断差异属于：
1. 真实行为缺口（需代码修复）
2. 归一化问题（需确定性设置）
3. TS 语义对照缺陷（少见，但可能）

## CI 配置

### PR (快速)

在拉取请求上触发。运行子集以快速反馈：

- `ext_conformance_diff` 配合 `PI_OFFICIAL_MAX=5`
- `ext_conformance_generated`（生成式 tier 1-2）
- `extensions_policy_negative`
- `capability_denial_matrix`

### Nightly (全量)

每日 02:00 UTC 运行：

- 完整的 `ext_conformance_diff`（全部 66 个官方）
- 完整的 `ext_conformance_generated`（含被忽略项）
- `ext_conformance_scenarios`
- `ext_conformance_fixture_schema`
- `ext_conformance_artifacts`
- `conformance_report` 生成

### Weekly (扩展)

周六 02:00 UTC 运行：

- 社区、npm 与第三方扩展
- 包含全部被忽略测试的完整语料库

## CI 门禁阈值

CI 门禁 (`ci.yml`) 强制最低质量标准：

| 指标 | 阈值 | 当前 |
|--------|-----------|---------|
| 通过率 | >= 80% | 93.3% |
| 最大失败数 | <= 36 | 4 |
| 最大 N/A 数 | <= 170 | 163 |
| 证据契约 | `pass` | `pass` |

门禁模式：
- `strict`（默认）：阈值违规时使构建失败
- `rollback`：告警但允许构建继续

## 调试工作流

### 调试单个扩展失败

```bash
# 1. 运行该扩展的差异测试
cargo test --test ext_conformance_diff diff_official_hello \
  --features ext-conformance -- --nocapture 2>&1 | tee /tmp/debug.log

# 2. 单独检查 TS 语义对照输出
cd legacy_pi_mono_code/pi-mono
bun run packages/coding-agent/src/core/extensions/runner.ts \
  --extension /path/to/extension.ts

# 3. 检查 Rust 运行时输出
cargo test --test ext_conformance_scenarios \
  --features ext-conformance -- hello --nocapture
```

### 调试 TS 语义对照超时

若 TS 语义对照超时：

```bash
# 增大超时
export PI_TS_ORACLE_TIMEOUT_SECS=60

# 或检查 Bun 是否正确安装
/home/ubuntu/.bun/bin/bun --version

# 检查 pi-mono 依赖
cd legacy_pi_mono_code/pi-mono && npm ci
```

harness 对不稳定的语义对照超时包含重试逻辑（最多 3 次重试）。

### 调试 "Module Not Found" 错误

```bash
# 1. 检查缺失的模块
cargo test --test ext_conformance_diff diff_official_<name> \
  --features ext-conformance -- --nocapture 2>&1 | grep "Module not found"

# 2. 检查模块是否已做 shim
grep "node:<module>" src/extensions_js.rs

# 3. 检查虚拟模块桩
grep "<package-name>" src/extensions_js.rs
```

### 改进后更新基线

修复缺陷或添加 shim 后：

```bash
# 1. 运行全量任务
cargo test --test ext_conformance_generated conformance_full_report \
  --features ext-conformance -- --nocapture

# 2. 测试会在以下位置自动生成更新后的报告：
#    tests/ext_conformance/reports/conformance_summary.json
#    tests/ext_conformance/reports/CONFORMANCE_REPORT.md

# 3. 更新基线
cp tests/ext_conformance/reports/conformance_summary.json \
   tests/ext_conformance/reports/conformance_baseline.json

# 4. 运行兼容性校验包
python3 tests/ext_conformance/build_inventory.py
```

### 处理不稳定测试

1. 通过固定种子运行检查失败是否确定性：
   ```bash
   PI_CONFORMANCE_SEED=42 PI_TEST_MODE=1 RUST_TEST_THREADS=1 \
     cargo test --test ext_conformance_diff -- --nocapture
   ```

2. 若 TS 语义对照不稳定，增大超时与重试次数。

3. 对于路径相关失败，确保已设置确定性 CWD/HOME。

## 产物位置

| 产物 | 路径 | 格式 |
|----------|------|--------|
| 一致性摘要 | `tests/ext_conformance/reports/conformance_summary.json` | JSON |
| 一致性报告 | `tests/ext_conformance/reports/CONFORMANCE_REPORT.md` | Markdown |
| 一致性事件 | `tests/ext_conformance/reports/conformance_events.jsonl` | JSONL |
| 一致性基线 | `tests/ext_conformance/reports/conformance_baseline.json` | JSON |
| 兼容性摘要 | `tests/ext_conformance/reports/COMPATIBILITY_SUMMARY.md` | Markdown |
| 校验包 | `tests/ext_conformance/reports/compatibility_validation_pack.json` | JSON |
| 场景结果 | `tests/ext_conformance/reports/scenario_conformance.json` | JSON |
| 场景事件 | `tests/ext_conformance/reports/scenario_conformance.jsonl` | JSONL |
| 冒烟分流 | `tests/ext_conformance/reports/smoke_triage.json` | JSON |
| 清单 | `tests/ext_conformance/reports/inventory.json` | JSON |
| 按扩展日志 | `tests/ext_conformance/reports/extensions/<name>.jsonl` | JSONL |
| 一致性日志 | `tests/ext_conformance/reports/parity/extensions/<name>.jsonl` | JSONL |
| 冒烟日志 | `tests/ext_conformance/reports/smoke/extensions/<name>.jsonl` | JSONL |
| 运行时 API 矩阵 | `tests/ext_conformance/reports/parity/runtime_api_matrix.json` | JSON |
| 已校验清单 | `tests/ext_conformance/VALIDATED_MANIFEST.json` | JSON |
| 端到端结果 | `tests/e2e_results/<timestamp>/` | 混合 |
| CI 门禁裁决 | `tests/e2e_results/<timestamp>/ci_gate_promotion_v1.json` | JSON |

## 统一验证运行器

`scripts/e2e/run_all.sh` 脚本为全部校验提供单一入口：

```bash
# 完整校验（lint + lib 测试 + 全部套件）
./scripts/e2e/run_all.sh

# 快速本地迭代
./scripts/e2e/run_all.sh --profile quick

# CI 配置（确定性）
./scripts/e2e/run_all.sh --profile ci

# 运行指定套件
./scripts/e2e/run_all.sh --suite e2e_extension_registration

# 跳过 lint 门禁
./scripts/e2e/run_all.sh --skip-lint

# 列出可用套件
./scripts/e2e/run_all.sh --list

# 仅重跑上一次运行中的失败项
./scripts/e2e/run_all.sh --rerun-from tests/e2e_results/<timestamp>/summary.json

# 与基线对比
./scripts/e2e/run_all.sh --diff-from tests/e2e_results/<timestamp>/summary.json
```

### 配置

| 配置 | 范围 | 适用场景 |
|---------|-------|----------|
| `full` | Lint + lib + 全部 targets (unit、vcr、e2e) | 发布校验 |
| `quick` | Lint + lib + 仅 unit | 快速本地迭代 |
| `focused` | Lint + lib + 选定集成 | 定向调试 |
| `ci` | Lint + lib + 全部非 e2e + 1 个 e2e | CI 流水线 |

## 多智能体注意事项

在多个智能体并发工作于同一代码库的环境中：

1. **隔离构建产物**: 使用 `CARGO_TARGET_DIR=target-<agent-name>` 以防止构建缓存冲突。

2. **串行测试执行**: 设置 `RUST_TEST_THREADS=1` 以避免 VFS 中的文件系统争用。

3. **确定性设置**: 始终设置 `PI_TEST_MODE=1` 与 `PI_CONFORMANCE_SEED=42` 以获得可复现结果。

4. **检查编译错误**: 其他智能体可能修改 `src/extensions.rs` 等共享文件。若编译失败，请拉取最新变更后重试。
