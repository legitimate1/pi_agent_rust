# Rust 版本与原版之间的基准对比__GPT

生成时间：2026-02-19
工作区：`/data/projects/pi_agent_rust`

## 0) 历史加固后状态更新（2026-02-17）

本节记录 2026-02-17 加固后的扩展兼容性检查点。属于历史证据，非当前认证状态。

- 该检查点中扩展一致性矩阵在本地已完全通过：`224/224` 通过，`0` 失败，`0` 跳过。
- 该结果取代了本文档中早前引用 `223` 语料条目且存在非零失败的部分兼容性快照。
- 本更新周期内运行的验证命令：
  - `cargo test --test ext_conformance_generated --features ext-conformance -- conformance_sharded_matrix --nocapture --exact`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## 0.0.1) 当前认证现状核查（2026-05-01）

当前已入库的认证证据已将阻塞性必须通过扩展与信息性全清单健康状况分离：

- 必须通过扩展门禁：`123/123` 通过，`0` 失败，`0` 跳过。
- 信息性扩展集：`98/101` 通过，`3` 失败，`0` 跳过。
- 全清单健康状况：`221/224` 通过（`98.7%`），`0` 回归，相对 2026-02-07 基线修复 `34` 项。
- 来源产物：
  - `tests/ext_conformance/reports/gate/must_pass_gate_verdict.json`（`generated_at=2026-05-01T03:20:54.460Z`）
  - `tests/ext_conformance/reports/health_delta/health_delta_report.json`（`generated_at=2026-05-01T04:10:28.479Z`）

更早的 `224/224` 兼容性检查点仍可作为有用历史证据，但严格的当前声明必须引用上述 2026-05-01 的必须通过/全清单分离口径。

## 0.1) 压缩后安全路径验证（2026-02-18）

- 安全/性能护栏：特性/安全默认值已恢复并保留（未翻转以抬高基准的默认值）：
  - `src/extension_dispatcher.rs` 中 `DUAL_EXEC_DEFAULT_SAMPLE_PPM` 恢复为原始默认值（`25_000`）。
  - `src/hostcall_amac.rs` 中 AMAC `from_env()` 默认行为恢复为默认启用语义。
- 分发器中的快速路径优化仍保持安全：
  - 能力拒绝检查在快速路径上仍于宿主调用完成前执行。
  - 未移除任何策略/风险/配额控制。
- 原生运行时热路径清理已落地于活跃脚手架（`src/extensions.rs`）：
  - `RwLock` 守卫的锁作用域收紧与丢弃时机修复，
  - 工具/命令/快捷方式/提供方流中的分配/克隆削减，
  - clippy 驱动的正确性/风格重写（`option_if_let_else`、`manual_let_else`、`significant_drop_tightening`）。
- 本轮门禁状态：
  - `cargo fmt --check` ✅
  - `cargo check --all-targets` ✅
  - `cargo clippy --all-targets -- -D warnings` ✅
  - `cargo test --test event_loop_conformance --test lab_runtime_extensions --test extensions_event_wiring` ✅（`12 + 15 + 133` 项测试通过）

## 0.2) 实时提供方 Release 二进制检查点（2026-02-19）

- 开发首批门禁（debug 二进制，本流程中任何新 release 构建前的必需项）：
  - 产物：`tests/ext_conformance/reports/release_binary_e2e/ollama_firstset_dev_20260219_jobs10_timeout600.json`
  - 运行：`release-e2e-20260219T032439Z`
  - 提供方/模型：`ollama` / `qwen2.5:0.5b`
  - 结果：`20/20` 通过（`0` 失败，`0` 超时）
- 完整优化 release 二进制全量扫描（开发门禁通过后）：
  - 产物：`tests/ext_conformance/reports/release_binary_e2e/ollama_full_release_20260219_jobs10_timeout600.json`
  - 运行：`release-e2e-20260219T033502Z`
  - 提供方/模型：`ollama` / `qwen2.5:0.5b`
  - 结果：`224/224` 通过（`0` 失败，`0` 超时）
- 该通道聚焦兼容性（真实 `pi` 二进制 + 实时提供方路径），而非付费提供方吞吐基准。

## 0.3) 全量性能编排检查点（2026-02-19）

- 全套件编排运行：
  - 命令：`./scripts/perf/orchestrate.sh --profile full --skip-build --no-rch --output-dir /data/tmp/pi_agent_rust/codex/perf/full_local_skipbuild_retry_20260219T0650Z`
  - 关联：`fullbench-local-skipbuild-retry-20260219T0650Z`
  - 清单：`/data/tmp/pi_agent_rust/codex/perf/full_local_skipbuild_retry_20260219T0650Z/manifest.json`
- 运行汇总：
  - 套件：`11` 总计，`9` 通过，`2` 失败，`0` 跳过
  - 时长：`1,601,650ms`
  - 通过的套件：
    - `perf_comparison`
    - `perf_bench_harness`
    - `perf_baseline_variance`
    - `ext_bench_harness`
    - `bench_schema`
    - `bench_scenario`
    - `criterion_tools`
    - `criterion_system`
    - `criterion_extensions`
  - 失败的套件：
    - `perf_budgets`（`exit=101`）：因在预期规范路径下缺失/陈旧的证据产物（criterion/pijs/release-binary 输入）导致的严格 fail-closed 预算契约失败。
    - `perf_regression`（`exit=101`）：`binary_size_check` 失败，因严格模式下 release 二进制路径不存在（`/data/tmp/pi_agent_rust/codex/perf/release/pi`）。
- 解读：
  - 两项失败为证据路径/前置条件失败，并非已证实的运行时延迟回归。
  - 同次运行中，`perf_regression` 内度量的启动护栏仍为绿色（`--help` P95 `3.8ms`、`--version` P95 `3.6ms`）。

## 1) 核心摘要（请勿埋没）

1. 2026-02-18 的全新安全路径重跑加上 2026-02-19 的全量编排检查点延续了同一趋势：相较于最后验证的旧版基线，Rust 在本报告 harness 中度量的 `1M`/`5M` 匹配状态与真实工作负载总耗时上获胜。
2. Rust 在匹配状态与真实流程中**内存占用显著更小**，保留了可观的 RSS 优势。
3. 当前扩展认证在阻塞性必须通过通道上为通过状态，全清单健康状况另行跟踪：
   - 必须通过门禁：`123/123` 通过（`0` 失败，`0` 跳过）
   - 全清单健康状况：`221/224` 通过（`98.7%`），相对基线 `0` 回归
   - 历史 release 二进制端到端检查点（ollama，优化二进制）：`224/224` 通过（`0` 失败，`0` 超时）
4. 相较于旧版编码智能体 CLI，Rust 已大幅扩展一等能力表面（命令、策略解释器、提供方元数据/控制、风险/配额/安全埋点）。
5. 最大的实际优化目标仍是高 token 量与大历史下的会话追加/保存行为；这是实现大幅提速的最佳杠杆。
6. 启动/就绪延迟在本快照中明显偏向 Rust：全新 Rust `--help`/`--version` 均值约为 `3.02ms`/`2.77ms`；旧版直接重跑当前受阻，但此前已验证的旧版均值仍约为 `1.0s`（Node）与 `0.73s`（Bun）。
7. 扩展微 harness 反转已在真实原生运行时通道上实现：在全新的 `pijs_workload` release 运行中，原生运行时单次调用比 QuickJS 快 `~17.74x`（`0.4678us` vs `8.2980us`）。

## 1.1) 刷新增量（2026-02-18）

本次运行中全新度量的项：

- `pijs_workload` release 三通道对比（`quickjs`、`native-rust-runtime`、`native-rust-preview`），`50,000` 次迭代
- Rust release 启动/就绪（`--help` 与 `--version` 的 `hyperfine`）及一次性 `/usr/bin/time` 占用
- 通过 `session_workload_bench` 的 Rust 长会话基准：
  - 匹配状态（`prepare` + `workload`）在 `1M` 与 `5M` 目标 token 下
  - 真实场景（`prepare-realistic` + `workload-realistic`）在 `1M` 与 `5M` 目标 token 下
- 通过 `ext_workloads` 的 Rust 扩展工作负载矩阵及热点矩阵 + trace 产物
- 定向扩展/运行时套件（`event_loop_conformance`、`extensions_event_wiring`、`lab_runtime_extensions`）
- 本地严格质量门禁（`cargo fmt --check`、`cargo check --all-targets`、`cargo clippy --all-targets -- -D warnings`）

复用项（仓库内已有证据，方法不变）：

- LOC 与可调用清单（Rust + 旧版范围）
- CLI 差异（`pi --help` vs 旧版 `dist/cli.js --help`）
- 提供方 ID 差异（Rust 规范表 vs 旧版运行时提供方注册表）
- 来自此前已验证运行的跨运行时启动对比表（Node/Bun）
- 一次性启动占用快照（`/usr/bin/time` RSS/user/sys）
- 真实场景延迟矩阵
- 匹配状态 10 条消息追加占用矩阵
- 真实场景 1M/5M 占用矩阵
- 扩展工作负载微基准（`ext_workloads` 与 `bench_legacy_extension_workloads.mjs`）
- 历史 223 扩展已入库一致性 + 失败分类（保留作基线上下文；已被后续检查点证据与上述当前 2026-05-01 认证分离所取代）

构建/重新生成说明：

- 本轮中 `cargo build --release --bin pi` 成功，并用于全新的 Rust 启动数值。
- 本工作区中旧版 Node/Bun 直接重跑当前因 `legacy_pi_mono_code/pi-mono` 中缺失依赖表面而受阻：
  - Node 启动：`chalk` 的 `ERR_MODULE_NOT_FOUND`
  - Bun 启动：缺失 `node_modules/chalk`
  - 旧版扩展工作负载（Node）：缺失 `@mariozechner/jiti`
  - 旧版扩展工作负载（Bun）：缺失 `@sinclair/typebox` / `proper-lockfile`
  因此旧版对比行在无法直接重跑处继续使用此前已验证产物。

---

## 2) 范围与对比模式

### 2.1 等价对比范围

- Rust 目标：本仓库（`pi_agent_rust`）
- 旧版目标：`legacy_pi_mono_code/pi-mono/packages/coding-agent`

### 2.2 非等价范围（完整旧版运行时上下文）

- 旧版聚合目标：`packages/{ai,agent,coding-agent,tui}`
- 目的：纳入旧版卸载至兄弟包的行为（提供方栈、UI/运行时服务），使对比不至于过窄。

### 2.3 包含的基准

- 匹配状态长会话基准：恢复 + 追加相同的 10 条用户消息。
- 真实端到端基准：恢复 + 追加 + 扩展类活动 + 斜杠类状态变更 + 分叉 + 导出 + 压缩。
- 扩展微基准：真实扩展加载与真实工具/事件分发。
- 扩展语料一致性：完整已入库/未入库兼容性报告。
- Release 二进制实时提供方扩展端到端：开发首批门禁（`20` 用例）随后完整优化扫描（`224` 用例）。
- 这些套件旨在作为实用的系统级回归 harness，而非仅合成微基准快照。

### 2.4 提供方 API 成本控制

- 本报告**未**为基准矩阵使用付费外部 API 调用。
- 包含使用本地 `ollama`（`qwen2.5:0.5b`）的实时提供方扩展兼容性运行，以验证非模拟运行时行为。
- 此处未包含付费提供方吞吐基准。
- 如需新增提供方调用吞吐基准，优先使用 `ollama` 以控制成本。

---

## 3) 代码库规模与复杂度

## 3.1 LOC（生产 vs 测试）

方法：按语言作用域的 `tokei`（Rust 仓库为 `Rust`，旧版范围为 `TypeScript`）。

| Scope | Production LOC | Test LOC |
|---|---:|---:|
| Rust (`src`, 仅 Rust) | 224,348 | 224,212 (`tests`, 仅 Rust) |
| 旧版仅 coding-agent (`src/test`, 仅 TS) | 27,412 | 8,871 |
| 旧版全栈 (`ai+agent+coding-agent+tui`, 仅 TS) | 55,313 | 21,779 |

比例：

- Rust vs 旧版 coding-agent：生产 `8.18x`，测试 `25.27x`
- Rust vs 旧版全栈：生产 `4.06x`，测试 `10.29x`

## 3.2 函数/可调用清单

方法说明：

- 此处 Rust 可调用数使用 `fn` 令牌签名清单（`\\bfn\\s+...`）加测试属性清单；对宏/trait 形式仍为近似。
- 此处旧版可调用数使用 TypeScript AST 遍历（函数声明、方法、构造器、访问器、变量赋值的箭头/函数表达式）；仍为可执行行为的近似。

Rust（签名清单）：

- `src` 函数签名：`10,417`
- `tests` 函数签名：`9,459`
- 测试属性总计：`11,976`（`src=5,474`，`tests=6,502`）

旧版 AST 可调用清单：

- coding-agent `src`：`1,315`
- coding-agent `test`：`93`
- 全栈 `src`：`1,907`
- 全栈 `test`：`247`

## 3.3 测试覆盖基线（Rust）

来自 `docs/coverage-baseline-map.json`：

- 行覆盖：`79.08%`（`95,706 / 121,018`）
- 函数覆盖：`78.01%`（`8,545 / 10,954`）
- 分支覆盖：`51.95%`（因部分文件上 llvm-cov 导出 SIGSEGV，记录为下界）

---

## 4) 已验证的功能/能力差异

本节列出在本工作区快照中**已验证为 Rust 一等表面**且旧版编码智能体 CLI 缺失的能力。

## 4.1 CLI 表面差异（直接 help 对比）

仅 Rust 的顶层命令：

- `doctor`
- `help`
- `info`
- `migrate`
- `search`
- `update-index`

仅 Rust 的标志：

- `--extension-policy`
- `--explain-extension-policy`
- `--repair-policy`
- `--explain-repair-policy`
- `--list-providers`
- `--theme-path`
- `--session-durability`
- `--no-migrations`

仅旧版的标志：

- `--plan`

## 4.2 仅 Rust 的主要能力领域（含复杂度提示）

| Capability area | Primary Rust implementation | Approx LOC | Approx fn count |
|---|---|---:|---:|
| 扩展运行时 + 策略 + 宿主集成 | `src/extensions.rs` | 38,379 | 1,517 |
| QuickJS 桥接 + 宿主调用管道 + 运行时适配器 | `src/extensions_js.rs` | 19,284 | 449 |
| 协议/宿主调用集成的分发器 | `src/extension_dispatcher.rs` | 11,745 | 404 |
| 提供方规范元数据 + 别名路由 | `src/provider_metadata.rs` | 2,645 | 60 |
| 扩展索引/搜索/信息/更新流水线 | `src/extension_index.rs` | 1,469 | 98 |
| 环境 + 兼容性诊断（`doctor`） | `src/doctor.rs` | 1,475 | 69 |
| 运行时风险台账/回放/校准工具 | `src/extensions.rs`, `src/bin/ext_runtime_risk_ledger.rs` | large integrated surface | integrated |
| 单扩展配额执行引擎 | `src/extensions.rs` | integrated in core runtime | integrated |

## 4.3 提供方广度差异

- Rust 规范提供方 ID：`87`
- Rust 别名 ID：`34`
- 旧版提供方 ID（运行时 `@mariozechner/pi-ai` `getProviders()`）：`22`
- 精确规范 ID 重叠（Rust vs 旧版集合）：`16`
- Rust 规范 ID 中不在旧版精确 ID 集合中的数量：`71`
- 旧版独有精确 ID（相对 Rust 规范集合）：`6`（`azure-openai-responses`、`google-antigravity`、`google-gemini-cli`、`kimi-coding`、`openai-codex`、`vercel-ai-gateway`）

完整的 Rust 规范 ID（不在旧版精确 ID 集合中）见 **附录 B**。

## 4.4 综合的仅 Rust 功能清单（当前快照）

已验证为 Rust CLI/运行时中的一等能力，且在旧版编码智能体 CLI 快照中未等效暴露：

1. 扩展策略选择与解释：
- `--extension-policy`、`--explain-extension-policy`（`src/extensions.rs`）

2. 扩展自动修复策略选择与解释：
- `--repair-policy`、`--explain-repair-policy`（`src/extensions.rs`）

3. 提供方注册表内省：
- `--list-providers` 及规范/别名元数据层（`src/provider_metadata.rs`、`src/models.rs`）

4. 扩展索引生命周期命令：
- `search`、`info`、`update-index` 命令路径（`src/extension_index.rs`、`src/main.rs` 中的 CLI 接线）

5. 环境诊断命令：
- `doctor`（`src/doctor.rs`）

6. 会话持久化与迁移控制：
- `--session-durability`、`--no-migrations`、`migrate` 表面（`src/main.rs`、会话/存储模块）

7. 大型集成扩展运行时控制：
- 能力门控的宿主调用分发、策略门控、配额/风险埋点、运行时垫片（`src/extensions.rs`、`src/extensions_js.rs`、`src/extension_dispatcher.rs`）

8. 运行时风险台账与回放/校准工具：
- 集成于扩展运行时及专用工具入口（`src/extensions.rs`、`src/bin/ext_runtime_risk_ledger.rs`）

9. 扩展的提供方覆盖：
- Rust 中 87 个规范提供方 + 34 个别名 vs 旧版运行时中 22 个提供方 ID（附录 B）

10. 仓库内一等基准与一致性可执行文件：
- `src/bin/ext_full_validation.rs`
- `src/bin/ext_workloads.rs`
- `src/bin/session_workload_bench.rs`

主要仅 Rust 表面的复杂度锚点见 **第 4.2 节** 与 **附录 C**。

---

## 5) 基准方法（真实 + 极限）

所有主要基准类别在可能的情况下按运行时保持相同工作负载结构运行。

## 5.1 真实端到端工作负载语义

真实模式执行：

- 恢复/打开已有长会话
- 追加新用户+助手轮次
- 插入工具结果消息
- 扩展自定义条目活动
- 斜杠类状态变更（模型、思考级别、会话信息、标签）
- 压缩条目
- 分叉模拟（`branch` 汇总操作）
- 导出生成（HTML）
- 最终保存/索引更新

真实矩阵参数：

- `messages=5000`
- `append=10`
- `compactions=12`
- `extension_ops=40`
- `slash_ops=40`
- `forks=8`
- `exports=2`
- token 级别：`100k`、`200k`、`500k`、`1M`、`5M`
- 每单元运行次数：`3`

---

## 6) 性能结果

解读说明：

- 第 6.1-6.3 节包含此前已验证的跨运行时基线矩阵。
- 第 6.3.1 节新增 2026-02-18 的 Rust 全新重跑，并与此前已验证的旧版基线对比（直接旧版重跑受阻处）。

## 6.0 冷启动/就绪（响应时间）

命令级就绪基准（`hyperfine`，无网络调用）：

| Probe | Rust mean | Legacy Node mean | Legacy Bun mean | Node/Rust | Bun/Rust |
|---|---:|---:|---:|---:|---:|
| `--help` | 3.02 ms | 1,045.10 ms | 726.28 ms | 346.22x | 240.60x |
| `--version` | 2.77 ms | 1,024.75 ms | 729.70 ms | 370.17x | 263.59x |

一次性基线占用快照（`/usr/bin/time`）：

| Probe | Runtime | RSS KB | User s | Sys s | Elapsed |
|---|---|---:|---:|---:|---:|
| `--help` | rust | 6,912 | 0.00 | 0.00 | 0:00.00 |
| `--help` | legacy_node | 156,720 | 1.11 | 0.20 | 0:01.02 |
| `--help` | legacy_bun | 195,820 | 0.91 | 0.22 | 0:00.71 |
| `--version` | rust | 6,400 | 0.00 | 0.00 | 0:00.00 |
| `--version` | legacy_node | 156,560 | 1.11 | 0.21 | 0:01.03 |
| `--version` | legacy_bun | 194,624 | 0.96 | 0.20 | 0:00.73 |

解读：

- 对于初始 CLI 就绪，Rust 明显更快且基线进程占用显著更轻。
- 这些探针仅隔离启动/路径初始化；不包含会话恢复或扩展工作负载执行。

## 6.1 真实端到端延迟（p50, ms）

| Runtime | Token level | Open | Append/Ops | Save | Total |
|---|---:|---:|---:|---:|---:|
| legacy_bun | 100k | 24.63 | 143.84 | 0.00 | 168.47 |
| legacy_node | 100k | 47.20 | 220.70 | 0.00 | 267.91 |
| rust | 100k | 36.84 | 219.06 | 64.64 | 320.71 |
| legacy_bun | 200k | 29.55 | 196.99 | 0.00 | 226.70 |
| legacy_node | 200k | 58.77 | 303.60 | 0.00 | 362.37 |
| rust | 200k | 40.42 | 397.48 | 113.92 | 552.70 |
| legacy_bun | 500k | 39.01 | 375.75 | 0.00 | 415.27 |
| legacy_node | 500k | 76.68 | 607.04 | 0.00 | 684.64 |
| rust | 500k | 51.22 | 925.65 | 250.27 | 1,226.71 |
| legacy_bun | 1M | 50.83 | 649.51 | 0.00 | 700.52 |
| legacy_node | 1M | 119.76 | 1,117.65 | 0.00 | 1,238.67 |
| rust | 1M | 68.86 | 1,846.67 | 482.81 | 2,401.35 |
| legacy_bun | 5M | 155.63 | 2,801.90 | 0.00 | 2,959.42 |
| legacy_node | 5M | 396.41 | 5,578.20 | 0.00 | 5,974.67 |
| rust | 5M | 204.35 | 9,266.76 | 2,359.30 | 11,828.14 |

Rust 总计 p50 相对旧版的比例：

| Token level | Rust/Node | Rust/Bun |
|---|---:|---:|
| 100k | 1.20x | 1.90x |
| 200k | 1.53x | 2.44x |
| 500k | 1.79x | 2.95x |
| 1M | 1.94x | 3.43x |
| 5M | 1.98x | 4.00x |

关键解读：

- Rust 的 open 阶段具竞争力，在更高规模下常优于 Node。
- 当前 Rust 瓶颈是大型长会话高 churn 下的追加/保存行为。

## 6.2 匹配状态合成基准（相同会话状态，恢复 + 10）

这是“相同状态后追加相同 10 条消息”的直接对比。

| Runtime | Token level | Open ms | Append ms | Save ms | Total ms | RSS KB | User s | Sys s | FS out |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rust | 1M | 68.78 | 254.76 | 432.47 | 756.01 | 32,092 | 0.86 | 0.04 | 112 |
| legacy_node | 1M | 126.37 | 170.57 | 0.00 | 296.94 | 167,752 | 0.76 | 0.16 | 0 |
| legacy_bun | 1M | 52.85 | 90.51 | 0.00 | 143.36 | 184,492 | 0.31 | 0.17 | 0 |
| rust | 5M | 210.30 | 1,282.08 | 2,124.94 | 3,617.31 | 129,836 | 3.84 | 0.38 | 112 |
| legacy_node | 5M | 399.61 | 1,395.80 | 0.00 | 1,795.41 | 411,372 | 1.95 | 0.63 | 0 |
| legacy_bun | 5M | 156.24 | 405.62 | 0.00 | 561.86 | 481,852 | 0.56 | 0.42 | 0 |

匹配状态下的比例：

- 1M：Rust 延迟 `2.55x` Node、`5.27x` Bun；Rust 内存比 Node 小 `5.23x`、比 Bun 小 `5.75x`。
- 5M：Rust 延迟 `2.01x` Node、`6.44x` Bun；Rust 内存比 Node 小 `3.17x`、比 Bun 小 `3.71x`。

## 6.3 真实占用（相同真实操作，1M/5M）

| Runtime | Token level | Open ms | Append/Ops ms | Save ms | Total ms | RSS KB | User s | Sys s | FS out | Wall |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rust | 1M | 163.91 | 2,654.88 | 505.34 | 3,324.14 | 76,240 | 3.31 | 0.21 | 112 | 0:03.36 |
| legacy_node | 1M | 199.10 | 1,508.55 | 0.00 | 1,707.65 | 820,380 | 1.44 | 1.16 | 0 | 0:02.26 |
| legacy_bun | 1M | 92.71 | 810.69 | 0.00 | 903.40 | 875,092 | 0.63 | 0.79 | 0 | 0:01.21 |
| rust | 5M | 674.47 | 13,224.33 | 2,460.56 | 16,359.37 | 274,832 | 15.79 | 1.06 | 112 | 0:16.40 |
| legacy_node | 5M | 793.81 | 8,018.42 | 0.00 | 8,812.23 | 2,173,096 | 4.77 | 5.57 | 0 | 0:09.54 |
| legacy_bun | 5M | 325.28 | 3,882.84 | 0.00 | 4,208.12 | 3,057,908 | 1.67 | 3.42 | 0 | 0:04.75 |

解读：

- 在此基线表中，Rust 在真实端到端上延迟更慢。
- 内存仍显著更小（这些真实运行中 RSS 低 `~7.9x` 至 `~11.5x`）。

## 6.3.1 安全路径刷新（2026-02-18，全新 Rust 重跑）

全新 Rust 重跑（release 二进制，同 harness 家族）结果如下：

匹配状态（`resume + append same 10`）：

| Runtime | Token level | Open ms | Append ms | Save ms | Total ms | RSS KB | FS out |
|---|---:|---:|---:|---:|---:|---:|---:|
| rust (fresh) | 1M | 17.49 | 47.54 | 22.63 | 87.66 | 45,140 | 15,496 |
| rust (fresh) | 5M | 58.03 | 233.92 | 73.57 | 365.52 | 146,040 | 77,224 |

真实场景（`append + compactions + extension/slash/fork/export ops`）：

| Runtime | Token level | Open ms | Append/Ops ms | Save ms | Total ms | RSS KB | FS out |
|---|---:|---:|---:|---:|---:|---:|---:|
| rust (fresh) | 1M | 17.59 | 210.35 | 22.36 | 250.29 | 67,572 | 88,840 |
| rust (fresh) | 5M | 58.68 | 1,225.95 | 97.49 | 1,382.12 | 268,844 | 438,104 |

与本报告中此前已验证旧版基线的对比：

- 匹配 1M：Rust 总计 `87.66ms` vs Node `296.94ms` 与 Bun `143.36ms`（`Rust/Node=0.295x`，`Rust/Bun=0.611x`）。
- 匹配 5M：Rust 总计 `365.52ms` vs Node `1,795.41ms` 与 Bun `561.86ms`（`Rust/Node=0.204x`，`Rust/Bun=0.651x`）。
- 真实 1M：Rust 总计 `250.29ms` vs Node `1,238.67ms` 与 Bun `700.52ms`（`Rust/Node=0.202x`，`Rust/Bun=0.357x`）。
- 真实 5M：Rust 总计 `1,382.12ms` vs Node `5,974.67ms` 与 Bun `2,959.42ms`（`Rust/Node=0.231x`，`Rust/Bun=0.467x`）。

重要可比性说明：

- 本工作区中直接旧版重跑因缺失运行时依赖而受阻（见第 1.1 节）。
- 因此上述比例使用全新 Rust 重跑对比本报告中此前已验证的旧版基线。

## 6.4 会话保存热路径优化更新（2026-02-17）

已应用优化：

- 修改 `src/session.rs` 中 `Session::should_full_rewrite`，使压缩条目不再无条件强制全文件重写。
- 保留的全量重写触发条件：首次保存、头部脏、检查点间隔，以及防御性持久化计数不匹配。
- 更新保存路径单元测试以断言压缩路径使用增量追加：
  - `session::tests::test_compaction_entry_uses_incremental_append`

选择该杠杆的原因：

- 真实工作负载包含压缩事件。
- 此前策略将这些工作负载转换为重复的 O(file-size) 重写。

同等真实基准参数下优化前后对比（release 构建）：

| Token level | Metric | Before | After | Delta | Speedup |
|---|---|---:|---:|---:|---:|
| 1M | `save_ms` | 26.42 | 31.31 | +4.89 | 0.84x |
| 1M | `total_ms` | 275.59 | 264.00 | -11.59 | 1.04x |
| 5M | `save_ms` | 359.63 | 114.46 | -245.17 | 3.14x |
| 5M | `total_ms` | 1,636.38 | 1,412.39 | -223.99 | 1.16x |

资源计数器（`/usr/bin/time`）：

| Token level | Metric | Before | After | Delta |
|---|---|---:|---:|---:|
| 1M | `elapsed_s` | 0.36 | 0.27 | -0.09 |
| 1M | `rss_kb` | 68,800 | 67,852 | -948 |
| 5M | `elapsed_s` | 1.76 | 1.43 | -0.33 |
| 5M | `rss_kb` | 268,692 | 268,788 | +96 |

解读：

- 这显著削减了大会话保存瓶颈（尤其在 5M token 时）。
- 聚合运行时在高 token 数下仍由追加/操作阶段主导，因此这是重要但非最终的优化。

---

## 7) 扩展运行时设计与兼容性状态

## 7.1 Rust 扩展架构（深度剖析）

Rust 扩展处理以能力门控的 QuickJS 宿主运行时为中心，具备显式宿主调用分发与策略执行。

核心属性：

- 连接器模型而非环境化 Node/Bun 权限（`tool`、`exec`、`http`、`session`、`ui`、`events`、`log`）。
- 策略优先分发（`allow/prompt/deny`）及可解释配置与 CLI 解释器。
- 确定性事件循环桥接（微任务排空 + 宿主完成调度纪律）。
- 结构化生命周期控制与有界执行区域。
- 对高价值 Node/Bun 表面的兼容性垫片，而非完整运行时仿真。
- 运行时风险评分 + 哈希链台账 + 回放/校准产物。
- 单扩展配额执行集成于共享宿主调用分发。

设计/实现重点领域：

- `src/extensions.rs`
- `src/extensions_js.rs`
- `src/extension_dispatcher.rs`
- `EXTENSIONS.md`（运行时契约 + 一致性流程）

## 7.2 真实扩展执行基准（Rust vs 旧版）

使用的基准产物：

- Rust：`.tmp_windyelk/ext_workloads_rust_gpt.jsonl`
- 旧版 Node：`.tmp_windyelk/ext_workloads_legacy_node_gpt.jsonl`
- 旧版 Bun 运行时：`.tmp_windyelk/ext_workloads_legacy_bun_gpt.jsonl`

| Scenario | Extension | Rust | Legacy (Node) | Legacy (Bun runtime) | Rust/Node | Rust/Bun |
|---|---|---:|---:|---:|---:|---:|
| `ext_load_init/load_init_cold` | hello | 7.96 ms (p50) | 22.29 ms (p50) | 22.25 ms (p50) | 0.36x | 0.36x |
| `ext_load_init/load_init_cold` | pirate | 7.74 ms (p50) | 11.97 ms (p50) | 19.01 ms (p50) | 0.65x | 0.41x |
| `ext_tool_call/hello` | hello | 16.80 us/call | 1.37 us/call | 0.87 us/call | 12.26x slower | 19.37x slower |
| `ext_event_hook/before_agent_start` | pirate | 17.51 us/call | 1.71 us/call | 1.00 us/call | 10.27x slower | 17.52x slower |

解读：

- 在这些代表性扩展上，Rust 冷加载已明显具竞争力/更快。
- 单次调用分发开销在 Rust 中仍显著更高，仍是首要的扩展运行时优化目标。

### 7.2.1 增量优化更新（2026-02-17）

在 `src/extensions.rs` 中进行定向扩展热路径变更后（上下文负载缓存复用、跨运行时命令通道的 `Arc<Value>` 上下文传输、任务 ID 分配开销削减，以及 `await_js_task` 快速路径处理），我们重新运行了 release 版 `ext_workloads`。

产物：

- `.tmp_codex/ext_workloads_after_arc_release.jsonl`
- `.tmp_codex/ext_workloads_after_arc_release_matrix.json`
- `.tmp_codex/ext_workloads_after_arc_release_trace.jsonl`
重复采样：

- `.tmp_codex/ext_workloads_release_rep1.jsonl`
- `.tmp_codex/ext_workloads_release_rep2.jsonl`
- `.tmp_codex/ext_workloads_release_rep3.jsonl`

相对本报告中此前数值的已更新仅 Rust 增量：

| Scenario | Prior Rust (report baseline) | Updated Rust (release) | Change |
|---|---:|---:|---:|
| `ext_load_init/load_init_cold` (hello, p50) | 7.96 ms | 6.93 ms | 1.15x faster (`~13.0%`) |
| `ext_load_init/load_init_cold` (pirate, p50) | 7.74 ms | 6.48 ms | 1.19x faster (`~16.3%`) |
| `ext_tool_call/hello` | 16.80 us/call | 11.88 us/call | 1.41x faster (`~29.3%`) |
| `ext_event_hook/before_agent_start` | 17.51 us/call | 15.02 us/call | 1.17x faster (`~14.2%`) |

复现说明：

- 3 次即时重复 release 运行显示出一定主机争用方差（工具调用 `~12.18-13.25us`，事件钩子 `~15.41-17.05us`），但仍显著优于此前基线。

### 7.2.2 QuickJS vs 原生 Rust 预览（内部微 harness）

我们重新运行 `pijs_workload` 以隔离最小工具往返的运行时引擎开销：

| Runtime engine | Command | Result |
|---|---|---:|
| QuickJS | `target/release/pijs_workload --iterations 50000 --runtime-engine quickjs` | `per_call_us_f64 = 8.29795126` (`per_call_ns_f64 = 8297.95126`) |
| Native Rust runtime (real handle path) | `target/release/pijs_workload --iterations 50000 --runtime-engine native-rust-runtime` | `per_call_us_f64 = 0.46778008` (`per_call_ns_f64 = 467.78008`) |
| Native Rust preview | `target/release/pijs_workload --iterations 50000 --runtime-engine native-rust-preview` | `per_call_us_f64 = 0.00777448` (`per_call_ns_f64 = 7.77448`) |

重要提示：

- `native-rust-preview` 为合成且一致性不完整。
- `native-rust-runtime` 为真实运行时句柄路径，在此 harness 中单次调用比 QuickJS 快 `~17.74x`。
- 预览仍远更快（相对 QuickJS `~1067.33x`），表明当前真实运行时实现之外仍有额外提升空间。
- 此微 harness 不取代第 6 节中更大的真实会话基准；它仅隔离扩展调用运行时开销。

### 7.2.2.1 全新扩展工作负载重跑（2026-02-18）

全新 Rust 重跑产物：

- `target/perf/secure_path_refresh_20260218/rust_extension_workloads.jsonl`
- `target/perf/secure_path_refresh_20260218/rust_ext_hotspot_matrix.json`
- `target/perf/secure_path_refresh_20260218/rust_ext_trace.jsonl`

全新 Rust 扩展指标：

- `ext_load_init/load_init_cold`（`hello`）：`p50=7.02ms`、`p95=8.52ms`
- `ext_load_init/load_init_cold`（`pirate`）：`p50=6.75ms`、`p95=6.78ms`
- `ext_tool_call/hello`：`13.10us/call`（`~76.3k calls/s`）
- `ext_event_hook/before_agent_start`：`15.37us/call`（`~65.1k calls/s`）
- `ext_hostcall_bridge/long_session_real_corpus`（8 个真实扩展）：`15.53us/call`（`~64.4k calls/s`）

与本报告中此前已验证旧版扩展基线的对比：

- 冷加载在 Rust 中仍更快（`hello`：`0.315x` Node、`0.316x` Bun；`pirate`：`0.564x` Node、`0.355x` Bun）。
- 单次扩展分发仍更慢（`tool_call`：`~9.56x` Node、`~15.06x` Bun；`event_hook`：`~8.99x` Node、`~15.37x` Bun）。

本工作区中直接旧版扩展重跑当前受阻：

- Node22 运行：缺失 `@mariozechner/jiti`
- Bun 运行：缺失 `@sinclair/typebox` / `proper-lockfile`

### 7.2.3 QuickJS 移除计划（性能反转路径）

为真正反转扩展开销（Rust 单次调用快于旧版），基准数据支持分阶段替换：

1. 优先对热路径钩子/工具采用原生运行时层级（`tool_call`、`tool_result`、高频事件钩子）。
2. 在迁移期间仅将 QuickJS 保留为显式兼容性/测试 harness 基础设施（本树中生产运行时选择现为原生强制）。
3. 引入扩展的预先降低（清单 + 类型化宿主调用 IR），使分发绕过已验证扩展的 JS 编组。
4. 在原生分发器中保留现有策略/配额/风险护栏，但将其移至预验证的类型化结构以消除重复 JSON 解码。
5. 以现有的一致性语料与性能 SLI 门禁作为发布门禁：
   - 已入库通过率无回归，
   - `ext_tool_call/hello` 与 `ext_event_hook/before_agent_start` 必须超越当前旧版基线。

近期可度量目标（基于当前数据）：

- 在保持一致性的同时，将 `ext_tool_call/hello` 从约 `11.9-12.3us` 驱动至 `<1.3us`，将 `ext_event_hook/before_agent_start` 从约 `15.0-15.5us` 驱动至 `<1.7us`。

## 7.3 语料一致性（当前 2026-02-18）

来源：

- `tests/ext_conformance/reports/sharded/shard_0_report.json`（`generated_at=2026-02-18T23:43:48Z`）
- `tests/ext_conformance/reports/scenario_conformance.json`（`generated_at=2026-02-18T23:11:57Z`）
- `tests/ext_conformance/reports/parity/triage.json`（`generated_at=2026-02-18T23:12:13Z`）
- `tests/ext_conformance/reports/release_binary_e2e/ollama_firstset_dev_20260219_jobs10_timeout600.json`（`runId=release-e2e-20260219T032439Z`）
- `tests/ext_conformance/reports/release_binary_e2e/ollama_full_release_20260219_jobs10_timeout600.json`（`runId=release-e2e-20260219T033502Z`）

已入库矩阵状态：

- 清单计数：`224`
- 已测：`224`
- 通过：`224`
- 失败：`0`
- 跳过：`0`
- 总体通过率：`100%`

场景一致性套件：

- 总计：`25`
- 通过：`25`
- 失败：`0`
- 错误：`0`
- 跳过：`0`

一致性分流样本：

- 总计：`25`
- 匹配：`22`
- 不匹配：`0`
- 跳过：`3`
- rust_error：`0`
- ts_error：`0`

## 7.3.1 Release 二进制实时提供方端到端（2026-02-19）

本检查点的执行顺序：

1. 在 debug 二进制上进行开发首批门禁（`max_cases=20`）以在 release 构建前验证行为。
2. 在完整已入库清单上进行完整优化 release 二进制全量扫描。

结果：

- 开发首批门禁（`ollama_firstset_dev_20260219_jobs10_timeout600.json`）：
  - 总计：`20`
  - 通过：`20`
  - 失败：`0`
  - 超时：`0`
- 完整 release 扫描（`ollama_full_release_20260219_jobs10_timeout600.json`）：
  - 总计：`224`
  - 通过：`224`
  - 失败：`0`
  - 超时：`0`
  - missing_extension：`0`
  - process_error：`0`

解读：

- 扩展兼容性声明现同时具备 harness 级一致性证据与真实 `target/release/pi` 实时提供方执行证据。
- 这是强运行时兼容性验证；并非提供方质量的吞吐/延迟基准。

## 7.4 历史基线（2026-02-14，已被取代）

以下 TSV 为来自更早 `223` 条目基线运行的审计历史保留，已被后续检查点证据取代。对于当前认证表述，请使用本文档顶部的 2026-05-01 必须通过/全清单分离口径。

列：`id`、`status`、`verdict`、`failure_category`、`reason`、`suggested_fix`

```tsv
agents-mikeastock/extensions.fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
community/nicobailon-interview-tool	fail	extension_problem	extension_load_error	Extension expects local assets/files unavailable at runtime.	Bundle required assets or extend missing_asset auto-repair policy.
community/prateekmedia-lsp	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
doom-overlay	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/@verioussmith/pi-openrouter	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/agentsbox	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/aliou-pi-linkup	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/aliou-pi-synthetic	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/lsp-pi	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-bar	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-core	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/mitsupi	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/oh-my-pi-anthropic-websearch	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-exa	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-lsp	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-pi-git-tool	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-subagents	pending	needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/pi-amplike	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-bash-confirm	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-messenger	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/pi-package-test	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-search-agent	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-shell-completions	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/shitty-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/tmustier-pi-arcade	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/vaayne-agent-kit	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/vaayne-pi-mcp	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
third-party/aliou-pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/ben-vargas-pi-packages	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/charles-cooper-pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/kcosr-pi-extensions	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/marckrenn-pi-sub	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/openclaw-openclaw	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/pasky-pi-amplike	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/w-winter-dot314	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
```

## 7.5 当前差距状态

1. 当前已认证的必须通过一致性无未通过条目（`123/123` 通过）。
2. 当前全清单健康状况有三条未通过的扩展性条目（`221/224` 通过）；在当前认证门禁中为信息性且非阻塞。
3. 当前场景套件无未解决失败（`25/25` 通过）。
4. 差异一致性分流当前在采样用例中显示 `0` 不匹配（`22` 匹配，`3` 跳过）。
5. 剩余工作为回归预防：将必须通过、全清单健康状况、场景与一致性通道作为发布门禁证据，并立即调查任何未来漂移。

---

## 8) 测试表面对比（单元 + 端到端）

Rust：

- Rust 测试文件：`257`
- Rust 端到端前缀测试文件：`35`
- Rust 测试属性：`11,976`

旧版代理（正则调用点计数）：

- coding-agent 测试文件：`49`
- coding-agent 测试调用点：`604`（`it(` + `test(` 出现次数）
- 全栈测试文件（`ai+agent+coding-agent+tui`）：`107`
- 全栈测试调用点：`1,413`（`it(` + `test(` 出现次数）

说明：本工作区中的旧版树未提供类似 `docs/coverage-baseline-map.json` 的等效整合覆盖率 JSON 产物以供直接百分比对比。

---

## 9) 安全/可靠性/asupersync 影响

## 9.1 安全

- Rust 扩展路径为能力门控且按宿主调用可审计。
- 策略解释器 + 显式 deny/prompt/allow 语义为一等能力。
- 风险与配额控制已集成且具备测试埋点。

## 9.2 可靠性与正确性

- 结构化并发基础（`asupersync`）降低异步生命周期不确定性。
- 确定性取消/资源作用域提升长驻 CLI 会话的健壮性。
- 哈希链风险台账 + 回放/校准工具提升事后可复现性。

## 9.3 asupersync“正确性优先”影响

- 工作限定于显式生命周期，降低隐藏后台任务泄漏与孤立异步工作。
- 取消成为一等控制流原语而非尽力而为约定，降低会话卡死与关闭竞态风险。
- 确定性运行时模式使失败复现与取证回放更可信（尤其配合扩展宿主调用/风险台账）。
- 主要权衡是更严格的执行模型，相较于松散结构化异步图可能增加工程/协调开销。
- 在此刷新快照中，正确性/可控性收益仍在，且在安全路径上大会话延迟相较此前基线已大幅改善；扩展单次调用开销仍是主要剩余延迟差距。

## 9.4 本快照中的性能权衡

- Rust 启动/就绪仍显著更快。
- 全新 Rust 长会话/匹配状态重跑在 `1M` 与 `5M` 下快于报告中此前已验证的旧版基线。
- Rust 仍在内存占用上大幅获胜。
- 剩余主要差距：相对旧版 Node/Bun 扩展运行时的扩展单次调用分发开销。

---

## 10) 极限优化优先级（以达成下一阶段 5-10 倍）

这些是从已度量瓶颈中预期价值最高的目标：

1. 会话追加/保存热路径：
- 最小化重复的全历史序列化工作。
- 为大型会话文件引入增量持久化。
- 降低追加/更新例程中的分配抖动与拷贝放大。

2. JSON 解析/序列化快速路径：
- 消除热循环中可避免的中间 `Value` 转换。
- 在关键路径上优先类型化反序列化。
- 在安全且可度量的前提下使用零拷贝/借用解析。

3. 扩展单次调用开销：
- 降低宿主调用编组开销与临时分配。
- 为高频调用批量或预计算不变的策略/风险元数据。
- 优化热连接器分发路径（`tool`/`events`）。

4. 多核与局部性：
- 将昂贵分析与索引工作从前台会话循环中分区 offload。
- 改善会话条目扫描/索引更新中的缓存局部性。
- 尽可能保持保存/索引更新为追加导向而非全量重建。

5. 回归护栏：
- 将真实场景 100k/200k/500k/1M/5M 矩阵作为阻塞性性能 CI 轨道保留。
- 按提交序列跟踪 p50/p95、RSS 与 FS I/O 增量。

---

## 11) 附录 A — 历史已入库扩展列表快照（223，2026-02-14）

列：`id`、`sourceTier`、`candidateStatus`、`conformanceStatus`、`verdict`、`conformanceFailureCategory`、`classificationReason`、`suggestedFix`

```tsv
agents-mikeastock/extensions	agents-mikeastock	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
antigravity-image-gen	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
auto-commit-on-exit	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
bash-spawn-hook	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
bookmark	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
claude-rules	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/ferologics-notify	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-clipboard	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-cost-tracker	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-flicker-corp	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-funny-working-message	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-handoff	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-loop	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-memory-mode	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-oracle	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-plan-mode	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-resistance	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-speedreading	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-status-widget	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-ultrathink	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/hjanuschka-usage-bar	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/jyaunches-canvas	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-answer	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-control	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-cwd-history	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-files	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-loop	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-notify	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-review	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-todos	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-uv	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/mitsuhiko-whimsical	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-interactive-shell	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-interview-tool	community	vendored	fail	extension_problem	extension_load_error	Extension expects local assets/files unavailable at runtime.	Bundle required assets or extend missing_asset auto-repair policy.
community/nicobailon-mcp-adapter	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-powerline-footer	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-rewind-hook	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/nicobailon-subagents	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/ogulcancelik-ghostty-theme-sync	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-checkpoint	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-lsp	community	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
community/prateekmedia-permission	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-ralph-loop	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-repeat	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/prateekmedia-token-rate	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-background-notify	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-compact-config	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-pi-agent-scip	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-safe-git	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-safe-rm	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-session-color	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/qualisero-session-emoji	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-agent-guidance	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-mario-not	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-picman	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-ping	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-spice-invaders	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-arcade-tetris	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-code-actions	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-files-widget	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-ralph-wiggum	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-raw-paste	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-tab-status	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
community/tmustier-usage-extension	community	vendored	pass	pass		Extension passed conformance without requiring repair.	
confirm-destructive	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-compaction	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-footer	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-header	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-provider-anthropic	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-provider-gitlab-duo	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
custom-provider-qwen-cli	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
dirty-repo-guard	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
doom-overlay	official-pi-mono	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
dynamic-resources	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
event-bus	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
file-trigger	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
git-checkpoint	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
handoff	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
hello	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
inline-bash	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
input-transform	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
interactive-shell	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
mac-system-theme	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
message-renderer	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
modal-editor	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
model-status	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
notify	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/@verioussmith/pi-openrouter	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/agentsbox	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/aliou-pi-extension-dev	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/aliou-pi-guardrails	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/aliou-pi-linkup	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/aliou-pi-processes	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/aliou-pi-synthetic	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/aliou-pi-toolchain	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/benvargas-pi-ancestor-discovery	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/benvargas-pi-antigravity-image-gen	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/benvargas-pi-synthetic-provider	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/checkpoint-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/imsus-pi-extension-minimax-coding-plan-mcp	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/juanibiapina-pi-extension-settings	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/juanibiapina-pi-files	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/juanibiapina-pi-gob	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/lsp-pi	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-bar	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/marckrenn-pi-sub-core	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/mitsupi	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/ogulcancelik-pi-sketch	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/oh-my-pi-anthropic-websearch	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-basics	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/oh-my-pi-exa	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-lsp	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-pi-git-tool	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/oh-my-pi-subagents	npm-registry	vendored		needs_review		Vendored candidate is missing from VALIDATED_MANIFEST.json.	Regenerate or repair VALIDATED_MANIFEST.json.
npm/permission-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-agentic-compaction	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-amplike	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-annotate	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-bash-confirm	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-brave-search	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-command-center	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-ephemeral	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-extensions	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-ghostty-theme-sync	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-interactive-shell	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-interview	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-mcp-adapter	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-md-export	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-mermaid	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-messenger	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/pi-model-switch	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-moonshot	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-multicodex	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-notify	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-package-test	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-poly-notify	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-powerline-footer	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-prompt-template-model	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-repoprompt-mcp	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-review-loop	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-screenshots-picker	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-search-agent	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/pi-session-ask	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-shadow-git	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-shell-completions	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/pi-skill-palette	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-subdir-context	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-super-curl	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-telemetry-otel	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-threads	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-voice-of-god	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-wakatime	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-watch	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/pi-web-access	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/qualisero-pi-agent-scip	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/ralph-loop-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/repeat-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/shitty-extensions	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/tmustier-pi-arcade	npm-registry	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
npm/token-rate-pi	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/vaayne-agent-kit	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/vaayne-pi-mcp	npm-registry	vendored	fail	needs_review	extension_load_error	Extension load failure could not be cleanly mapped to limitation vs extension bug.	Inspect failure dossier and reproduce command.
npm/vaayne-pi-subagent	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/vaayne-pi-web-tools	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/vpellegrino-pi-skills	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/walterra-pi-charts	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/walterra-pi-graphviz	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
npm/zenobius-pi-dcp	npm-registry	vendored	pass	pass		Extension passed conformance without requiring repair.	
overlay-qa-tests	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
overlay-test	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
permission-gate	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
pirate	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
plan-mode	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
preset	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
protected-paths	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
qna	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
question	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
questionnaire	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
rainbow-editor	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
rpc-demo	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
sandbox	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
send-user-message	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
session-name	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
shutdown-command	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
snake	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
space-invaders	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
ssh	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
status-line	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
subagent	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
summarize	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
system-prompt-header	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/aliou-pi-extensions	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/ben-vargas-pi-packages	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/charles-cooper-pi-extensions	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/cv-pi-ssh-remote	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/graffioh-pi-screenshots-picker	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/graffioh-pi-super-curl	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/jyaunches-pi-canvas	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/kcosr-pi-extensions	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/limouren-agent-things	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/lsj5031-pi-notification-extension	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/marckrenn-pi-sub	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/michalvavra-agents	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/ogulcancelik-pi-sketch	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/openclaw-openclaw	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/pasky-pi-amplike	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/qualisero-pi-agent-scip	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/raunovillberg-pi-stuffed	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/rytswd-direnv	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/rytswd-questionnaire	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/rytswd-slow-mode	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/vtemian-pi-config	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
third-party/w-winter-dot314	third-party-github	vendored	fail	harness_gap	registration_mismatch	Observed registration output diverges from manifest expectations.	Refresh expected snapshot from TS oracle and re-validate.
third-party/zenobi-us-pi-dcp	third-party-github	vendored	pass	pass		Extension passed conformance without requiring repair.	
timed-confirm	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
titlebar-spinner	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
todo	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
tool-override	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
tools	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
trigger-compact	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
truncated-tool	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
widget-placement	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
with-deps	official-pi-mono	vendored	pass	pass		Extension passed conformance without requiring repair.	
```

## 12) 附录 B — Rust 规范提供方 ID（不在旧版精确 ID 集合中，71 个）

```text
302ai
abacus
aihubmix
alibaba
alibaba-cn
azure-openai
bailing
baseten
berget
chutes
cloudflare-ai-gateway
cloudflare-workers-ai
cohere
cortecs
deepinfra
deepseek
fastrouter
fireworks
firmware
friendli
github-models
gitlab
helicone
iflowcn
inception
inference
io-net
jiekou
kimi-for-coding
llama
lmstudio
lucidquery
minimax-cn-coding-plan
minimax-coding-plan
moark
modelscope
moonshotai
moonshotai-cn
morph
nano-gpt
nebius
nova
novita-ai
nvidia
ollama
ollama-cloud
ovhcloud
perplexity
poe
privatemode-ai
requesty
sap-ai-core
scaleway
siliconflow
siliconflow-cn
stackit
submodel
synthetic
togetherai
upstage
v0
venice
vercel
vivgrid
vultr
wandb
xiaomi
zai-coding-plan
zenmux
zhipuai
zhipuai-coding-plan
```

## 13) 附录 C — 本报告使用的特性复杂度表

### 13.1 Rust 特性复杂度表

```tsv
file	loc	fn_count
src/extensions.rs	38379	1517
src/extensions_js.rs	19284	449
src/extension_dispatcher.rs	11745	404
src/provider_metadata.rs	2645	60
src/extension_index.rs	1469	98
src/doctor.rs	1475	69
src/session.rs	7294	334
src/session_index.rs	1648	88
src/cli.rs	1330	107
src/main.rs	3632	120
src/providers/mod.rs	2127	105
src/providers/openai.rs	1903	75
src/providers/anthropic.rs	1792	63
src/providers/gemini.rs	1317	59
src/providers/azure.rs	1030	39
src/providers/cohere.rs	1551	51
src/providers/vertex.rs	801	39
src/providers/bedrock.rs	1048	42
src/providers/gitlab.rs	375	22
src/providers/copilot.rs	424	22
src/bin/ext_full_validation.rs	1635	28
src/bin/ext_workloads.rs	4537	120
src/bin/session_workload_bench.rs	435	18
```

### 13.2 旧版特性复杂度表

```tsv
file	loc	callables
packages/coding-agent/src/core/extensions/index.ts	132	0
packages/coding-agent/src/core/extensions/wrapper.ts	85	4
packages/coding-agent/src/core/extensions/runner.ts	615	37
packages/coding-agent/src/core/session-manager.ts	1011	61
packages/coding-agent/src/core/model-registry.ts	432	21
packages/coding-agent/src/cli/args.ts	286	3
packages/coding-agent/src/main.ts	619	16
packages/ai/src/providers/register-builtins.ts	62	2
packages/ai/src/providers/openai-responses.ts	222	8
packages/ai/src/providers/openai-completions.ts	699	15
packages/ai/src/providers/anthropic.ts	637	15
packages/ai/src/providers/google.ts	408	9
packages/ai/src/providers/google-vertex.ts	435	11
packages/ai/src/providers/amazon-bedrock.ts	547	15
packages/ai/src/providers/azure-openai-responses.ts	212	9
packages/ai/src/providers/google-gemini-cli.ts	862	16
packages/ai/src/providers/openai-codex-responses.ts	356	13
```

## 14) 附录 D — 主要原始产物

- 真实端到端延迟 + 匹配状态 + 占用矩阵（本报告复用的基线数据集）
  - `BENCHMARK_COMPARISON_BETWEEN_RUST_VERSION_AND_ORIGINAL__CODEX.md`（源表与矩阵输出）
  - `.bench/pi_session_bench/after_round2_runs.jsonl`
  - `.bench/pi_session_bench/after_round3_runs.jsonl`
  - `.bench/pi_session_bench/after_round4_runs.jsonl`
  - `.bench/pi_session_bench/after_round5_runs.jsonl`
- 扩展执行微基准
  - `.tmp_windyelk/ext_workloads_rust_gpt.jsonl`
  - `.tmp_windyelk/ext_workloads_legacy_node_gpt.jsonl`
  - `.tmp_windyelk/ext_workloads_legacy_bun_gpt.jsonl`
- 冷启动就绪与占用探针
  - `/tmp/startup_help_compare.json`
  - `/tmp/startup_version_compare.json`
- 扩展一致性语料输出
  - `tests/ext_conformance/reports/pipeline/full_validation_report.compat2.json`
  - `tests/ext_conformance/reports/pipeline/full_validation_report.compat2.md`
- 提供方清单/一致性产物
  - `docs/provider-canonical-id-table.json`
  - `docs/provider-parity-reconciliation-report.json`
  - `/tmp/provider_diff.json`
  - `/tmp/help_diff.json`
  - `.tmp_windyelk/rust_provider_extra.txt`
  - `.tmp_windyelk/provider_overlap.txt`
  - `.tmp_windyelk/legacy_provider_extra.txt`
- 覆盖率与测试表面产物
  - `docs/coverage-baseline-map.json`
  - `docs/TEST_COVERAGE_MATRIX.md`
  - `/tmp/ts_counts_coding_agent.json`
  - `/tmp/ts_counts_fullstack.json`
  - `.tmp_windyelk/pi_rust_tokei.json`
  - `.tmp_windyelk/pi_legacy_coding_agent_tokei.json`
  - `.tmp_windyelk/pi_legacy_fullstack_tokei.json`
