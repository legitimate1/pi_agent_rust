# 开发指南

## 构建

Pi 的 release 构建使用 `rust-toolchain.toml` 中锁定的 `nightly-2026-07-05` 工具链。锁定的依赖图要求 Rust 1.95 或更高版本；请使用仓库内固定的版本以保证编译器与 Clippy 结果可复现。

```bash
# 构建开发版二进制
rch exec -- cargo build

# 构建发布版二进制（优化）
rch exec -- cargo build --release
```

## 兄弟 Crate（已发布 vs 本地开发）

默认情况下，`pi_agent_rust` 依赖兄弟库的 **已发布 crates.io 版本**：
- `asupersync`
- `rich_rust`
- `charmed-*`（bubbletea/lipgloss/bubbles/glamour）
- `sqlmodel-*`（core/sqlite）

若需在本地同步修改这些仓库（联动开发），请使用仅本地生效的 Cargo patch。假设兄弟仓库与 `pi_agent_rust` 并列检出（如 `../asupersync`、`../rich_rust` 等），请在**本地检出**中添加以下内容（请勿提交）：

```toml
[patch.crates-io]
asupersync = { path = "../asupersync" }
rich_rust = { path = "../rich_rust" }
charmed-bubbletea = { path = "../charmed_rust/crates/bubbletea" }
charmed-lipgloss = { path = "../charmed_rust/crates/lipgloss" }
charmed-bubbles = { path = "../charmed_rust/crates/bubbles" }
charmed-glamour = { path = "../charmed_rust/crates/glamour" }
sqlmodel-core = { path = "../sqlmodel_rust/crates/sqlmodel-core" }
sqlmodel-sqlite = { path = "../sqlmodel_rust/crates/sqlmodel-sqlite" }
```

## 测试

我们对核心逻辑执行严格的“无模拟（no-mock）”策略。测试使用真实的文件系统操作（在临时目录中）以及面向 HTTP 交互的 VCR 风格录制。

### 单元测试与集成测试

```bash
# 运行全部测试
rch exec -- cargo test

# 运行指定模块
rch exec -- cargo test config
rch exec -- cargo test session
```

对于多智能体（multi-agent）会话，请将 `rch exec --` 视为编译命令的必选项。使用 `./scripts/smoke.sh --require-rch` 与 `./scripts/ext_quality_pipeline.sh --require-rch` 以避免意外的本地编译风暴。对于临时的 Cargo 闸门，优先使用 headroom 包装器，因为它会在运行前先输出 JSON 准入决策：

```bash
# 探测重型闸门是否可安全启动（不实际运行）
./scripts/cargo_headroom.sh --runner auto --admit-only clippy --all-targets -- -D warnings

# 通过 rch 运行，并将 target/tmp 目录置于仓库之外
PI_CARGO_AGENT_SUFFIX="$USER" ./scripts/cargo_headroom.sh --runner rch clippy --all-targets -- -D warnings
```

在 `--runner auto` 模式下，包装器仅在以下情况下回退到本地：安全的本地命令（如 `cargo fmt`）或操作者传入 `--allow-local-fallback` / `PI_CARGO_ALLOW_LOCAL_FALLBACK=1`。若 `rch` 缺失、饱和或对重型命令不健康，包装器将返回机器可读的 `backoff` 决策，而不是静默启动广义的本地 Cargo 运行。

在启动集群或重型的全目标（all-target）闸门前，请检查宿主资源预算：

```bash
pi doctor --only swarm --format json
```

`pi.doctor.swarm_resource_preflight.v1` 检查结果会报告 cgroup CPU 配额、cpuset 大小、NUMA 节点、cgroup 内存限制以及 `CARGO_TARGET_DIR` 与 `TMPDIR` 的暂存空间余量。若 `status = fail` 或 `critical_failures` 列表非空，则视为硬性阻断，直至两个目录均指向 `/data/tmp/pi_agent_rust_cargo/<agent>/` 且剩余空间充足。当检查通过时，请将 `recommended_budgets` 作为操作者对智能体扇出、工具并发、扩展宿主调用通道、RCH 验证扇出、队列深度与 RSS 预算的上限。

在由 RCH 支撑的闸门消费已检入的测试制品或产出报告包之前，请运行制品同步预检：

```bash
python3 scripts/check_rch_artifact_sync.py --json
```

该预检是对 `.rchignore` 的演练（dry run）。当必需的制品路径（如 `tests/ext_conformance/artifacts/`）会被排除在工作器镜像之外时预检失败，JSON 输出会报告每个必需路径、匹配规则以及导致失败的具体忽略行。根制品排除必须保持锚定为 `/artifacts/` 与 `/artifacts/**`，以免隐藏嵌套的测试自有制品目录。

对于生成已检入证据的 RCH 闸门，还需用生成制品后置条件包裹远程命令：

```bash
before_manifest="/data/tmp/pi_agent_rust_cargo/${USER:-agent}/must-pass-before.json"
python3 scripts/check_rch_artifact_sync.py --mode postcondition \
  --generated-artifact tests/ext_conformance/reports/gate/must_pass_gate_verdict.json \
  --write-before-manifest "$before_manifest" --json
rch exec -- cargo test --test ext_conformance_generated --features ext-conformance -- conformance_must_pass_gate --nocapture --exact
python3 scripts/check_rch_artifact_sync.py --mode postcondition \
  --generated-artifact tests/ext_conformance/reports/gate/must_pass_gate_verdict.json \
  --before-manifest "$before_manifest" --json
```

后置条件会比较执行前后的 mtime 与校验和。当远端生成器已完成但本地证据文件未变更时，后置条件将失效关闭（fail closed），并指明陈旧的制品及建议的本地重跑或 RCH 取回/回写修复方案。

### 一致性测试

一致性测试会验证 Pi 在工具、扩展与核心逻辑上与旧版 TypeScript 实现行为完全一致。测试按层级组织：

#### 快速：策略 + 工具一致性（无外部依赖）

```bash
# 工具一致性夹具
cargo test conformance

# 扩展策略负向测试（51 项：跨模式的拒绝/允许）
cargo test --test extensions_policy_negative

# 夹具模式校验
cargo test --test ext_conformance_fixture_schema

# 制品校验和校验
cargo test --test ext_conformance_artifacts
```

#### 完整：差分 TS-Rust 预言机（需要 Bun + pi-mono）

这些测试在旧版 TypeScript 运行时与 Rust QuickJS 运行时中运行相同的未修改扩展，然后比较注册快照。

**前置条件：**
- Bun 1.3.8 位于 `/home/ubuntu/.bun/bin/bun`（或在 PATH 上）
- 已安装 pi-mono 的 npm 依赖：`cd legacy_pi_mono_code/pi-mono && npm ci`

```bash
# 官方扩展（60 个）- 差分一致性
cargo test --test ext_conformance_diff --features ext-conformance -- --nocapture

# 限制为前 N 个官方扩展（更快迭代）
PI_OFFICIAL_MAX=5 cargo test --test ext_conformance_diff --features ext-conformance -- --nocapture

# 场景执行（工具调用、命令、事件）
cargo test --test ext_conformance_scenarios --features ext-conformance -- --nocapture

# 自动生成的按扩展测试
cargo test --test ext_conformance_generated --features ext-conformance -- --nocapture

# 社区 + npm + 第三方（CI 中每周运行，使用 --ignored）
cargo test --test ext_conformance_diff --features ext-conformance -- --ignored --nocapture

# Npm-registry 差分通道（ignored 按需启用，默认限定为 5 个）
rch exec -- env PI_NPM_FILTER=aliou-pi-extension-dev PI_NPM_MAX=1 \
  cargo test --test ext_conformance_diff --features ext-conformance diff_npm_manifest -- \
  --include-ignored --nocapture
```

**环境变量：**

| 变量 | 默认值 | 用途 |
|----------|---------|---------|
| `PI_OFFICIAL_MAX` | （全部） | 限制测试的官方扩展数量 |
| `PI_NPM_FILTER` | （无） | 按 `dir/entry` 子串过滤 npm-registry 扩展 |
| `PI_NPM_MAX` | 5 | 将被忽略的 npm-registry 差分通道限制为确定性的有界样本 |
| `PI_TS_ORACLE_TIMEOUT_SECS` | 30 | TS 预言机进程超时 |
| `PI_DETERMINISTIC_TIME_MS` | 1700000000000 | 用于确定性的固定墙钟时间 |
| `PI_DETERMINISTIC_RANDOM_SEED` | 1337 | 固定的随机种子 |

**报告：** 测试结果以 JSONL 与 JSON 格式写入 `tests/ext_conformance/reports/`。

#### 生成一致性报告

运行一致性测试后，生成按扩展汇总的综合报告：

```bash
cargo test --test conformance_report generate_conformance_report -- --nocapture
```

这会在 `tests/ext_conformance/reports/` 下产生三个输出文件：
- `CONFORMANCE_REPORT.md` — 人类可读的按层级表格，含通过/失败/N/A 状态
- `conformance_summary.json` — 带按层级细分的机器可读摘要
- `conformance_events.jsonl` — 每个扩展一行，含完整指标

#### CI 集成

| 触发条件 | 套件 | 命令 |
|---------|-------|---------|
| 每个 PR | 快速（5 个官方 + 负向 + 已生成） | `conformance.yml` / `conformance-fast` |
| 每夜 | 全量官方 + 场景 + 模式 + 制品 | `conformance.yml` / `conformance-full` + `conformance-full-scenario` |
| 每周 | 社区 + npm + 第三方 | `conformance.yml` / `conformance-weekly` |
| 每次推送 | 全部非特性门控测试 | `ci.yml` / `cargo test --all-targets` |

CI 会将一致性日志与报告作为可下载制品上传。

### 性能报告冒烟测试

性能/报告生成器在普通的 `cargo test` 运行期间不应重写已检入的制品。其冒烟测试模式默认写入 `TMPDIR`，而有意的证据刷新必须传入显式的输出根目录：

```bash
PERF_EVIDENCE_DIR=tests/perf/reports \
  rch exec -- cargo test --test perf_comparison generate_perf_comparison -- --nocapture
```

### VCR 模式

提供方测试使用已录制的“磁带（cassette）”来避免网络调用并确保确定性。

- **回放（默认）**：重放已录制的响应。若磁带缺失则失败。
- **录制**：发起真实的 API 调用并保存磁带。

```bash
# 在回放模式下运行（CI 默认）
VCR_MODE=playback cargo test

# 录制新的磁带（需要 API 密钥）
export ANTHROPIC_API_KEY=...
VCR_MODE=record cargo test provider_streaming
```

## 质量门

提交 PR 前，请确保所有闸门均通过：

```bash
# 格式检查
cargo fmt --check

# Lint 检查（拒绝警告）
rch exec -- cargo clippy --all-targets -- -D warnings

# 测试
rch exec -- cargo test --all-targets
```

## 项目结构

- `src/`：核心 Rust 源码
- `tests/`：集成测试与一致性测试
- `docs/`：面向用户与开发者的文档
- `legacy_pi_mono_code/`：来自原始 TypeScript 实现的参考代码
