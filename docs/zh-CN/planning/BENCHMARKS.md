# 性能基准

> **目的：** 跟踪并验证 pi_agent_rust 的性能预算。

## 用户可感知的 SLI 契约

Phase-0 规范 UX/SLI 契约位于 `docs/perf_sli_matrix.json`（`schema: pi.perf.sli_matrix.v1`）。

- 首要的发布决策指标是用户可见的端到端/响应性 SLI。
- 本文件中的微基准为诊断性/辅助指标。
- 场景到 SLI 的映射以 `docs/e2e_scenario_matrix.json` 工作流 ID 为键。
- 下游 PERF-3X 验证 Beads 必须直接从契约产物消费 SLI 结果。

## 快速开始

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench "truncation"
cargo bench "sse_parsing"
cargo bench "ext_policy"
cargo bench "ext_js_runtime"

# Run with baseline comparison
cargo bench -- --save-baseline main
cargo bench -- --baseline main
```

## 性能预算

以下为目标性能指标。超出这些阈值的回归应被调查。

### 核心指标（硬预算）

| Benchmark | Budget | Current | Status |
|-----------|--------|---------|--------|
| **startup/version** | <100ms (p95) | ~11ms | ✅ |
| **startup/help** | <150ms (p95) | ~15ms | ✅ |
| **startup/list_models** | <200ms (p95) | ~25ms | ✅ |
| **binary/size_mb** | <20MB | ~7.6MB | ✅ |
| **memory/version_peak** | <50MB RSS | TBD | ⬜ |

### 微基准

| Benchmark | Budget | Current | Status |
|-----------|--------|---------|--------|
| truncate_head (10K lines) | <1ms | ~250μs | ✅ |
| truncate_tail (10K lines) | <1ms | ~250μs | ✅ |
| sse_parse (100 events) | <100μs | ~50μs | ✅ |
| ext_policy/evaluate | <1μs | ~20ns | ✅ |
| ext_dispatch/decision | <10μs | ~100ns | ✅ |
| ext_protocol/parse | <100μs | ~5μs | ✅ |
| ext_js_runtime/cold_start | <200ms | ~308μs | ✅ |
| ext_js_runtime/warm_eval_noop | <25ms | ~3.50μs | ✅ |
| ext_js_runtime/warm_run_pending_jobs_empty | <1μs | ~84ns | ✅ |
| ext_js_runtime/tool_call_roundtrip | <500μs | ~43.9μs | ✅ |

### 扩展运行时（基线：2026-02-07，debug 构建，103 个扩展）

| Benchmark | Budget | Current (debug) | Status |
|-----------|--------|-----------------|--------|
| ext_cold_load_simple_p95 (100 extensions) | p95 < 200ms | 106ms | ✅ |
| ext_cold_load_per_ext_p99 (worst ext) | p99 < 100ms | 134ms (hjanuschka-plan-mode) | ⬜* |
| ext_warm_load_p95 (100 extensions) | p95 < 100ms | 734μs | ✅ |
| ext_warm_load_per_ext_p99 (worst ext) | p99 < 100ms | 926μs (jyaunches-pi-canvas) | ✅ |
| event_dispatch_p99 (AgentStart, PR mode) | p99 < 5ms | 616μs | ✅ |

*单扩展冷加载 P99 在 debug 模式下超出预算，但在 release 模式下预期可通过（release 冷加载通常约 5-10ms）。预算断言仅在 release 模式下生效。

基线数据：`tests/perf/reports/ext_bench_baseline.json`
离群分析：`tests/perf/reports/BASELINE_REPORT.md`

### 扩展运行时预算定义

这些预算面向**扩展开销**，而非端到端 LLM 延迟。

- **冷启动：** 进程首次创建/初始化扩展运行时（冷缓存）。
- **热启动：** 扩展运行时已初始化（热缓存）；衡量稳态开销。
- **钩子开销：** 通过无操作扩展钩子路由工具调用所增加的延迟。
- **宿主调用分发：** 跨连接器边界调用一次宿主调用的成本（无操作负载）。

### 度量方法（bd-1ii）

- **硬件类别：** GitHub Actions `ubuntu-latest` 运行器（x86_64）。将数值视为 *CI 预算*；本地机器会有差异。
- **分位数：** 预算以 **p95/p99** 指定，以避免在共享 CI 运行器上仅拟合中位数结果。
- **基准：** 扩展基准将位于 `benches/extensions.rs`（规划中），应报告：
  - 冷启动与热启动耗时分别统计
  - 无扩展基线与无操作扩展增量的钩子开销对比
  - 足够样本以在 CI 上使分位数报告有意义

## 基准结果

### 截断性能

文本截断操作的吞吐：

```
truncation/head/1000    time:   [32 µs]     thrpt:  [2.3 GiB/s]
truncation/head/10000   time:   [251 µs]    thrpt:  [3.0 GiB/s]
truncation/head/100000  time:   [2.3 ms]    thrpt:  [3.3 GiB/s]

truncation/tail/1000    time:   [~32 µs]    thrpt:  [~2.3 GiB/s]
truncation/tail/10000   time:   [~251 µs]   thrpt:  [~3.0 GiB/s]
truncation/tail/100000  time:   [~2.3 ms]   thrpt:  [~3.3 GiB/s]
```

**关键观察：**

- 无论输入大小，吞吐均稳定在 2.3-3.3 GiB/s
- 头部与尾部截断性能相近
- 对于典型文件大小（10K 行）远低于 1ms 预算

### SSE 解析性能

Server-Sent Events 解析吞吐：

```
sse_parsing/parse/100   time:   [50.129 µs 50.315 µs 50.504 µs]
                         thrpt:  [1.9800 Melem/s 1.9875 Melem/s 1.9949 Melem/s]

sse_parsing/parse/1000  time:   [495.54 µs 495.96 µs 496.40 µs]
                         thrpt:  [2.0145 Melem/s 2.0163 Melem/s 2.0180 Melem/s]
```

## 基准结构

```
benches/
├── bench_env.rs      # Shared environment validation and fingerprinting
├── tools.rs          # Core operation benchmarks
│   ├── truncation    # Text truncation (head/tail)
│   ├── sse_parsing   # SSE event parsing
│   ├── sse_stream    # Streaming SSE parsing at various chunk sizes
│   └── streaming_clone  # Arc<AssistantMessage> vs deep clone
├── extensions.rs     # Connector dispatch + policy / protocol parsing
│   ├── ext_policy
│   ├── ext_required_capability
│   ├── ext_dispatch
│   ├── ext_protocol
│   ├── ext_js_runtime     # QuickJS cold/warm start + no-op eval
│   ├── hostcall_*         # Hostcall conversion, hashing, dispatch
│   └── js_serde_bridge    # JS↔Rust serialization roundtrip
├── system.rs         # System-level benchmarks (process spawn)
│   ├── startup       # Startup time (version, help, list_models)
│   ├── memory        # RSS memory measurement
│   └── binary        # Binary size tracking
├── tui_perf.rs       # TUI rendering benchmarks (PERF-8)
│   ├── build_conversation_content
│   ├── view          # Full TUI render
│   ├── viewport_operations
│   └── markdown_rendering
└── session_save.rs   # Session clone benchmarks
scripts/
└── bench_env_setup.sh  # OS-level benchmark environment standardization
```

## 新增基准

1. 在 `benches/tools.rs` 中添加基准函数：

```rust
fn bench_new_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("new_operation");

    // Test with different input sizes
    for size in [100, 1000, 10000] {
        let input = generate_input(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("name", size),
            &input,
            |b, input| {
                b.iter(|| pi::module::function(black_box(input)));
            },
        );
    }

    group.finish();
}

// Add to criterion_group!
criterion_group!(benches, ..., bench_new_operation);
```

2. 在本文档中添加性能预算
3. 运行基准：`cargo bench new_operation`

## CI 集成

GitHub Actions 中的性能回归检测：

```yaml
# .github/workflows/bench.yml
name: Benchmarks
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly

      - name: Build release binary
        run: cargo build --release

      - name: Check binary size budget
        run: |
          SIZE_MB=$(stat --printf="%s" target/release/pi | awk '{printf "%.2f", $1/1024/1024}')
          echo "Binary size: ${SIZE_MB}MB"
          if (( $(echo "$SIZE_MB > 20" | bc -l) )); then
            echo "::error::Binary size ${SIZE_MB}MB exceeds 20MB budget"
            exit 1
          fi

      - name: Run benchmarks
        run: |
          cargo bench --bench tools -- --noplot
          cargo bench --bench extensions -- --noplot
          cargo bench --bench system -- --noplot

      - name: Generate PiJS workload perf data (JSONL)
        run: |
          set -euxo pipefail
          BENCH_ALLOCATORS_CSV=system \
          BENCH_PGO_MODE=off \
          ITERATIONS=2000 \
          TOOL_CALLS_CSV=1,10 \
          scripts/bench_extension_workloads.sh

      - name: Perf budget gate
        run: cargo test --test perf_budgets -- --nocapture

      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: target/criterion/
          retention-days: 30
```

### 回归检测（手动）

与已知良好基线对比：

```bash
# Save baseline on main branch
cargo bench -- --save-baseline main

# After changes, compare
cargo bench -- --baseline main

# Look for regressions > 10%
```

### 方差处理

系统基准会派生真实进程，因此方差高于微基准：

- **微基准**（tools.rs、extensions.rs）：使用 criterion 默认值（100+ 样本）
- **系统基准**（system.rs）：使用 20 样本、10s 度量时间
- **CI 运行器**：相比本地机器预期有 2-3 倍方差；关注相对变化
- **分位数**：预算报告 p95/p99，而非仅均值

### 环境标准化（bd-3ar8v.5.4）

所有基准套件使用共享环境模块（`benches/bench_env.rs`），该模块会：

1. 在启动时**验证**执行环境（CPU governor、turbo boost、ASLR、THP）
2. 为每次运行**打指纹**，记录 OS、CPU、核心数、内存、governor、turbo、ASLR、THP 与配置哈希
3. **计算噪声分数**（0 = 最优），并在条件非最优时告警

`scripts/bench_env_setup.sh` 脚本将 OS 标准化以获得低方差结果：

```bash
# Check current environment suitability
./scripts/bench_env_setup.sh validate

# Apply optimal settings (requires root)
sudo ./scripts/bench_env_setup.sh apply

# Run benchmarks with CPU affinity and priority
./scripts/bench_env_setup.sh run cargo bench

# Emit JSON fingerprint for artifact tracking
./scripts/bench_env_setup.sh fingerprint

# Restore original settings
sudo ./scripts/bench_env_setup.sh restore
```

**控制项：**

| Setting | Optimal | Why |
|---------|---------|-----|
| CPU governor | `performance` | Fixed frequency eliminates DVFS variance |
| Turbo boost | disabled | Prevents thermal-dependent frequency shifts |
| ASLR | disabled | Reproducible memory layouts |
| THP | `never` | Avoids latency spikes from page coalescing |

**环境变量：**

| Variable | Default | Description |
|----------|---------|-------------|
| `BENCH_CORES` | `0,1` | CPU cores for `taskset` affinity |
| `BENCH_GOVERNOR` | `performance` | CPU frequency governor to set |
| `BENCH_NICE` | `-20` | Nice priority for bench processes |

**噪声分数解读：**

| Score | Meaning |
|-------|---------|
| 0 | Optimal — all settings applied |
| 1-2 | Minor — THP or ASLR not ideal |
| 3-5 | Moderate — governor or turbo not controlled |
| 6-7 | High — multiple sources of variance |

CI 在基准测试前自动应用环境设置。基准 stderr 输出中的 `[bench-env]` 横幅包含每次运行的噪声分数。

## 分析提示

### bd-1pb：Profile 驱动的优化闭环

本工作流采用严格的 **基线 → 分析 → 验证 → 实现 → 复验** 闭环。

#### 1) 基线

- 使用 Criterion 生成稳定的微基准产物：`cargo bench --bench extensions -- ext_js_runtime`。
- 对端到端 CLI 路径使用 `hyperfine`（如已安装）：

```bash
hyperfine --warmup 3 --runs 10 'target/release/pi --version'
```

- 使用 PiJS 工作负载 harness 进行确定性扩展往返：

```bash
scripts/bench_extension_workloads.sh
```

#### 历史诊断基线采集（2026-02-05）

这些 200 次迭代的直连二进制采集早于发布证据契约。它们仅为诊断用途，不具备回归门禁或发布声明资格。请通过上述规范 harness 生成权威 PiJS 证据，该 harness 绑定源提交、运行标识、精确的发布特性集、分配器、Cargo 指纹、可执行路径与可执行校验和。

命令：

```bash
hyperfine --warmup 3 --runs 10 'target/perf/pijs_workload --iterations 200 --tool-calls 1'
hyperfine --warmup 3 --runs 10 'target/perf/pijs_workload --iterations 200 --tool-calls 10'
```

汇总（时间单位 ms）：

| Scenario | Mean ± σ | Min / Max | per_call_us | calls/sec |
|----------|----------|-----------|-------------|-----------|
| pijs_workload_200x1 | 16.96 ± 0.98 | 15.78 / 19.00 | 44 | 22,716 |
| pijs_workload_200x10 | 97.09 ± 4.27 | 93.08 / 105.57 | 43 | 22,883 |

JSONL 日志（hyperfine + workload）：

```jsonl
{"tool":"hyperfine","scenario":"pijs_workload_200x1","command":"target/perf/pijs_workload --iterations 200 --tool-calls 1","mean_ms":16.96,"stddev_ms":0.98,"min_ms":15.78,"max_ms":19.00}
{"tool":"hyperfine","scenario":"pijs_workload_200x10","command":"target/perf/pijs_workload --iterations 200 --tool-calls 10","mean_ms":97.09,"stddev_ms":4.27,"min_ms":93.08,"max_ms":105.57}
{"schema":"pi.perf.workload.v1","tool":"pijs_workload","scenario":"tool_call_roundtrip","iterations":200,"tool_calls_per_iteration":1,"total_calls":200,"elapsed_ms":8,"per_call_us":44,"calls_per_sec":22716,"build_profile":"perf"}
{"schema":"pi.perf.workload.v1","tool":"pijs_workload","scenario":"tool_call_roundtrip","iterations":200,"tool_calls_per_iteration":10,"total_calls":2000,"elapsed_ms":87,"per_call_us":43,"calls_per_sec":22883,"build_profile":"perf"}
```

本地原始产物：

- `target/perf/perf/hyperfine_pijs_workload_200x1_perf.json`
- `target/perf/perf/hyperfine_pijs_workload_200x10_perf.json`
- `target/perf/perf/pijs_workload_perf.jsonl`

#### 2) 分析

- CPU 热点：`cargo flamegraph --bench extensions`（需 `cargo install flamegraph`）。
- 分配：`heaptrack cargo bench --bench extensions`（Linux）。
- Flamegraph 运行（2026-02-05）：`cargo flamegraph --bench extensions -- ext_js_runtime --noplot` 成功编译了 benches，随后在采样期间失败，原因是在此主机上 `perf_event_paranoid=4`（无 perf 权限）。请在具备 `CAP_PERFMON`（或降低 `perf_event_paranoid`）的主机上重试，并将生成的 SVG 作为 flamegraph 产物保留。

来自 Criterion `new/estimates.json`（均值点估计）的热点快照：

| Benchmark | Mean (ns) | Mean (μs) | Relative cost vs `warm_eval_noop` |
|-----------|-----------|-----------|------------------------------------|
| `ext_js_runtime/cold_start` | 307,950.60 | 307.95 | 88.0× |
| `ext_js_runtime/tool_call_roundtrip` | 43,915.12 | 43.92 | 12.6× |
| `ext_js_runtime/warm_eval_noop` | 3,498.12 | 3.50 | 1.0× |
| `ext_js_runtime/warm_run_pending_jobs_empty` | 84.45 | 0.08 | 0.02× |

#### 3) 验证（无“静默回归”）

- 保持输出可复现：记录环境（`benches/extensions.rs` 发出的 `[bench-env] ... config_hash=...`）。
- 将基准产物存储于 `target/criterion/`（Criterion JSON + 报告）。
- 使用 `--save-baseline` / `--baseline` 对比进行回归检测。

#### 4) 机会矩阵（已按优先级排序）

| Opportunity | Evidence | Expected impact | Confidence | Effort | Score | Notes |
|-------------|----------|-----------------|------------|--------|-------|-------|
| Cache compiled extension setup program across repeated loads | `ext_js_runtime/cold_start` = 307.95μs dominates runtime hotspot table | -150μs to -220μs cold-start cost on repeated extension loads | 4 | 3 | 5.33 | Keep module hash keyed by source+runtime config; preserve deterministic teardown semantics |
| Reduce JSON bridge overhead in hostcall tool path | `ext_js_runtime/tool_call_roundtrip` = 43.92μs and `pijs_workload` steady-state per-call = 43–46μs | -8μs to -15μs per roundtrip | 3 | 2 | 4.50 | Target serialization/path allocation churn first; validate with criterion baseline diff |
| Keep `run_pending_jobs` empty fast path as invariant | `ext_js_runtime/warm_run_pending_jobs_empty` = 84.45ns | Avoid regressions in scheduler idle overhead | 5 | 1 | 5.00 | No optimization work needed; treat as guardrail metric in future PRs |

### 使用 perf 进行 CPU 分析

```bash
# Record profile
cargo bench -- --profile-time 10
perf record -g target/release/deps/tools-*

# Analyze
perf report
```

### 使用 heaptrack 进行内存分析

```bash
heaptrack cargo bench
heaptrack_gui heaptrack.tools.*.gz
```

### 火焰图

```bash
cargo install flamegraph
cargo flamegraph --bench tools
```

## 与 TypeScript 的对比

Rust vs TypeScript 的目标指标：

| Operation | TypeScript | Rust Target | Rust Actual |
|-----------|------------|-------------|-------------|
| Startup | ~200ms | <100ms | 11.2ms ✅ |
| 10K line truncate | ~10ms | <1ms | 250μs ✅ |
| 100 SSE events | ~5ms | <100μs | 50.3μs ✅ |
| Binary size | N/A (Node) | <20MB | 7.6MB ✅ |
| Memory (idle) | ~80MB | <50MB | TBD |

### 扩展加载时间：Rust vs 旧版 TS（bd-uah）

所有 60 个官方扩展的单扩展加载时间对比。两类运行时加载相同的未修改 `.ts` 文件。TS 使用 Bun/jiti（基于原生 V8 的求值）。Rust 使用带 SWC 转译的 QuickJS。

| Metric | Rust (QuickJS) | TS (Bun/jiti) |
|--------|---------------|---------------|
| Mean load time | 103ms | 2ms |
| Min load time | 96ms | 1ms |
| Max load time | 131ms | 51ms |

**已知回归：** Rust 中扩展加载慢约 50-100 倍，原因：

1. 每次加载时的 SWC TypeScript 到 JavaScript 转译
2. QuickJS 字节码编译（无 JIT）
3. 虚拟模块系统解析开销

**为何可接受：** 加载成本是每个会话的一次性冷启动。稳态操作在 Rust 中快数个数量级：

- 工具调用往返：44μs（Rust）vs ~5ms（TS）
- 策略求值：20ns（Rust）
- 事件钩子分发：亚 50μs（Rust）

**计划缓解：** 编译后字节码缓存（见上文机会矩阵）以跨会话分摊冷启动。

完整单扩展数据：`tests/ext_conformance/reports/performance_comparison.json`

重新生成：`cargo test --test performance_comparison generate_performance_comparison -- --nocapture`

## 扩展基准 Harness（bd-20s9 / bd-2mb1）

统一基准 harness（`tests/ext_bench_harness.rs`）以单扩展超时、预算检查与完整环境指纹运行扩展加载与事件分发场景。

### 运行 Harness

```bash
# PR mode — diverse 10-extension subset, 10 iterations, ~3-4s in debug
PI_BENCH_MODE=pr cargo test --test ext_bench_harness --features ext-conformance -- --nocapture

# Nightly mode — full safe corpus, 50 iterations
PI_BENCH_MODE=nightly cargo test --test ext_bench_harness --features ext-conformance -- --nocapture

# Custom mode — tune all parameters
PI_BENCH_MODE=custom PI_BENCH_MAX=25 PI_BENCH_ITERATIONS=20 PI_BENCH_EVENT_COUNT=100 \
  cargo test --test ext_bench_harness --features ext-conformance -- --nocapture
```

### 环境变量

| Variable | Default | Description |
|----------|---------|-------------|
| `PI_BENCH_MODE` | `pr` | Mode: `pr`, `nightly`, or `custom` |
| `PI_BENCH_MAX` | 10 (pr) / 200 (nightly) / 20 (custom) | Max extensions to benchmark |
| `PI_BENCH_ITERATIONS` | 10 (pr) / 50 (nightly) / 20 (custom) | Iterations per extension per scenario |
| `PI_BENCH_EVENT_COUNT` | 50 (pr) / 200 (nightly) / 100 (custom) | Event dispatch iterations |
| `PI_BENCH_TIMEOUT_SECS` | 30 | Per-extension timeout (skips slow extensions) |

### PR 子集选择策略

PR 模式选择多样化的代表性子集以最大化 API 表面覆盖：

- 2 个官方扩展（1 个带工具注册，1 个带事件订阅）
- 2 个社区扩展（1 个带 commands+events，1 个带 tools+commands+flags）
- 2 个 npm-registry 扩展（1 个带 commands，1 个带 events）
- 剩余槽位按清单顺序从安全池填充

这确保每次运行都覆盖工具、命令、标志与事件钩子。

### 场景

| Scenario | What it measures | Method |
|----------|-----------------|--------|
| `cold_load` | Fresh runtime + context creation per iteration | New `ExtensionManager` + `JsExtensionRuntimeHandle::start()` + `load_js_extensions()` |
| `warm_load` | Repeated load on shared runtime (cache-hit path) | Single runtime, repeated `load_js_extensions()` after warmup |
| `event_dispatch` | Event hook dispatch latency across loaded extensions | `dispatch_event(AgentStart, payload)` on loaded corpus |

### 预算检查

| Budget | Threshold | Enforced |
|--------|-----------|----------|
| `ext_cold_load_simple_p95` | 200ms | Release builds only |
| `event_dispatch_p99` | 5ms | Release builds only |
| `ext_warm_load_p95` | 100ms | Release builds only |

预算断言在 **debug 构建中跳过**（debug 模式天然慢 5-10 倍）。

### 输出产物

所有输出位于 `target/perf/`：

| File | Format | Content |
|------|--------|---------|
| `ext_bench_harness.jsonl` | JSONL | One `pi.ext.rust_bench.v1` record per extension per scenario |
| `ext_bench_harness_report.json` | JSON | Full report with env, config, summaries, budget checks |
| `BENCH_HARNESS_REPORT.md` | Markdown | Human-readable summary with tables |

### 解读结果

- **P50/P95/P99** 按扩展从原始微秒样本计算
- **冷加载**时间包含 QuickJS 运行时创建（debug 中约 70ms，release 中约 5ms）
- **热加载**时间仅度量 `load_js_extensions()` 调用（约 300-800us）
- **事件分发**度量 `dispatch_event()` 延迟（约 40-700us，取决于已加载扩展）
- 聚合预算检查使用所有单扩展 P95 值的 P95

### 更新基线

如需有意更新基线阈值：

1. 在 release 模式下运行 harness 以获得准确数值：
   ```bash
   cargo test --release --test ext_bench_harness --features ext-conformance -- --nocapture
   ```
2. 复核 `target/perf/ext_bench_harness_report.json` 中的实际 P95/P99 值
3. 更新 `tests/ext_bench_harness.rs` 中 `check_budgets()` 的阈值常量
4. 在提交信息中记录调整理由

### 区分噪声与真实回归

- 运行 harness 3 次并比较 P95 值
- 多次运行间方差 > 20% 表示环境噪声
- 多次运行间一致的 P95 增长 > 50% 表示真实回归
- 检查 JSONL 中的 `env` 指纹以确保硬件/构建配置相同

## 说明

- 基准在启用 LTO 的 release 模式下运行
- 时间在标准 CI 硬件（GitHub Actions）上度量
- 吞吐以 GiB/s 或 elements/sec 度量
- 使用 `--save-baseline` 与 `--baseline` 进行回归检测
