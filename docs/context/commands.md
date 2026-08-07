# 开发命令

## 构建

```bash
cargo check              # 快速检查编译错误
cargo build              # Debug 构建（LLD 链接器 + sccache 缓存）
cargo build --release    # 发布构建（opt-level=3 + thin LTO + panic=abort）
```

### 部署

构建完成后，用脚本一键停进程+覆盖（脚本还会自动更新 cargo-sweep 时间戳，为下次清理做准备）：

```powershell
.\scripts\deploy-release.ps1
```

### target/ 磁盘空间管理

Cargo 设计上永不删除旧编译产物，每次构建（`cargo build`/`cargo test`）生成新的 hash 文件，日积月累会导致 `target/` 膨胀到数百 GB。使用 `cargo-sweep` 管理：

```bash
# 安装
cargo install cargo-sweep

# 标记当前构建时间戳（部署时自动执行）
cargo sweep --stamp

# 清理所有比标记时间戳旧的产物（回收几十~几百 GB）
cargo sweep --file

# 或按天数清理
cargo sweep --time 30
```

部署脚本 `deploy-release.ps1` 末尾自动执行 **`cargo sweep --file` → `cargo sweep --stamp`**（先清理上次部署前的旧产物，再更新基线），无需手动清理。首次部署（无 stamp）自动跳过清理只打标记。

### 构建优化配置（`.cargo/config.toml`）

- **LLD 链接器**：`lld-link.exe` 替代 MSVC `link.exe`，链接速度快 3-5x
- **sccache**：编译器缓存，`cargo clean` / 切换分支后大幅加速冷启动
- **Dev profile**：`debug = "line-tables-only"`，减少 debug 信息量以加速编译
- **Defender 排除**：项目 `target/` 已加入 Windows Defender 排除列表

## 测试

```bash
cargo test               # 全部测试
cargo test -- --nocapture  # 带输出
cargo test conformance   # 一致性测试
cargo test --lib sse::tests    # 特定 lib 模块单元测试
cargo test --lib tools::verify::tests  # 特定内部模块（非集成测试 target）
```

### 统一验证运行器（含证据产物）

```bash
./scripts/e2e/run_all.sh --profile focused   # 聚焦验证
./scripts/e2e/run_all.sh --profile ci        # CI 全量验证
./scripts/e2e/run_all.sh --rerun-from tests/e2e_results/<timestamp>/summary.json --skip-unit

# 快速冒烟 / 扩展质量（强制远程执行）
./scripts/smoke.sh --require-rch
./scripts/ext_quality_pipeline.sh --require-rch
```

### 磁盘余量保护（cargo_headroom.sh）

```bash
./scripts/cargo_headroom.sh build             # Debug（远程卸载）
./scripts/cargo_headroom.sh build --release   # Release
./scripts/cargo_headroom.sh test --all-targets
./scripts/cargo_headroom.sh clippy --lib --bins -- -D warnings
./scripts/cargo_headroom.sh clippy --tests -- -D warnings
```

`CARGO_TARGET_DIR`/`TMPDIR` 未设置时默认 `/data/tmp/pi_agent_rust_cargo/<agent>/...`，拒绝 repo 根 target，磁盘余量不足提前失败。`PI_CARGO_RUNNER=local` 本地执行，`PI_CARGO_BUILD_ROOT=<dir>` 指定大卷，`PI_CARGO_HEADROOM_MIN_FREE_MB=<mb>` 调整阈值。重型 all-targets 门禁前先跑 `pi doctor --only swarm --format json`。

### 扩展验证管道（examples）

```bash
# 1) 拉取未 vendor 扩展源语料
cargo run --example ext_unvendored_fetch_run -- run-all --workers 8 --no-probe

# 2) 端到端验证编排（conformance shards + provider 矩阵 + 场景套件 + 自动修复）
cargo run --example ext_full_validation --

# 3) 发布前 dev-firstset live-provider 门禁（必须 pass=20/total=20）
rch exec -- cargo build --bin pi --bin ext_release_binary_e2e
PI_HTTP_REQUEST_TIMEOUT_SECS=0 rch exec -- \
  cargo run --example ext_release_binary_e2e -- \
  --pi-bin target/debug/pi --provider ollama --model qwen2.5:0.5b \
  --jobs 10 --timeout-secs 600 --max-cases 20 --extension-policy balanced

# 4) 全量 release-binary live-provider E2E（步骤 3 通过后）
rch exec -- cargo build --release --bin pi --bin ext_release_binary_e2e
PI_HTTP_REQUEST_TIMEOUT_SECS=0 target/release/ext_release_binary_e2e \
  --pi-bin target/release/pi --provider ollama --model qwen2.5:0.5b \
  --jobs 10 --timeout-secs 600 --extension-policy balanced

# 扩展工作负载热点剖析（六阶段成本分解 + hotspot matrix）
cargo run --example ext_workloads -- \
  --out artifacts/perf/ext_workloads.jsonl \
  --matrix-out artifacts/perf/ext_hostcall_hotspot_matrix.json

# 运行时风险台账取证（verify/replay/calibrate）
rch exec -- cargo run --example ext_runtime_risk_ledger -- verify --input path/to/runtime_risk_ledger.json
rch exec -- cargo run --example ext_runtime_risk_ledger -- replay --input path/to/runtime_risk_ledger.json
rch exec -- cargo run --example ext_runtime_risk_ledger -- calibrate --input path/to/runtime_risk_ledger.json --objective balanced_accuracy
```

产物集中在 `tests/ext_conformance/reports/`（pipeline/gate/health_delta/journeys/release_binary_e2e）。

### 安装器回归

```bash
bash tests/installer_regression.sh
```

## Release & Publishing

- Tag 格式：`vX.Y.Z`（`vX.Y.Z-rc.N` 预发布允许但跳过 crates.io publish）
- Tag 版本**必须**等于 `Cargo.toml` 的 `package.version`
- 依赖发布顺序：`asupersync` → `rich_rust` → `charmed-*`（lipgloss/bubbletea/bubbles/glamour）→ `pi_agent_rust`
- `.github/workflows/publish.yml` 在 `CARGO_REGISTRY_TOKEN` 设置时处理 crates.io 发布

## Coverage

```bash
cargo install cargo-llvm-cov --locked   # 一次性安装
rustup component add llvm-tools-preview

cargo llvm-cov --all-targets --workspace --summary-only  # 摘要（最快）
CI=true VCR_MODE=playback VCR_CASSETTE_DIR=tests/fixtures/vcr \
  cargo llvm-cov --all-targets --workspace --lcov --output-path lcov.info  # LCOV（CI/产物）
cargo llvm-cov --all-targets --workspace --html  # HTML 报告
```

## Cargo features

```toml
default = ["sqlite-sessions", "tui"]   # JSONL 仍为默认会话存储
full    = [image-resize, jemalloc, clipboard, wasm-host, sqlite-sessions, syntax-highlighting, tui]
```

- `--features full`：一次构建全部可选用户面 extras
- `--no-default-features --features clipboard`：无 SQLite 会话后端的最小子集
- jemalloc 仅 benchmark 变体使用（`BENCH_ALLOCATORS_CSV=system,jemalloc ./scripts/bench_extension_workloads.sh`），默认不启用

## 静态检查

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

> `cargo clippy` 已内含编译检查，无需单独跑 `cargo check`。详情见 AGENTS.md 的「静态检查」节。

## Release 构建

默认 profile：

```toml
opt-level = 3       # 速度优化
lto = "thin"        # 薄 LTO，兼顾编译速度与代码质量
panic = "abort"
strip = true
```

## CLI 用法

```bash
pi [OPTIONS] [ARGS]...
pi --print "message"         # 非交互模式
pi --model <MODEL> "..."     # 指定模型
pi --thinking <LEVEL>        # 思考级别
pi --no-tools                # 禁用全部内置工具
pi --tools read,write,edit   # 仅启用指定工具
pi --provider <PROVIDER>     # 指定 Provider
```

### CLI 子命令

```bash
# 包管理
pi install <source> [-l|--local]    # 安装包源并加入 settings
pi remove <source> [-l|--local]     # 从 settings 移除包源
pi update [source]                  # 更新全部（或单个）非 pinned 包
pi list                             # 列出 user + project 包

# 配置
pi config                           # 显示 settings 路径 + 优先级

# 扩展目录索引 + 发现
pi update-index                     # 刷新扩展索引元数据
pi search "git"                     # 搜索扩展元数据
pi info pi-search-agent             # 查看扩展详情

# 环境与扩展诊断
pi doctor                           # 检查 config/dirs/auth/shell/sessions/swarm/扩展兼容性
pi doctor --only sessions --format json
pi doctor --only swarm --format json   # 报告 cgroup CPU/内存、NUMA、target/tmp 余量、并发预算
pi doctor ./path/to/extension --policy safe --fix

# 只读 swarm 进度 SLO 评估
pi swarm-progress --input progress-slo-input.json --format json

# 会话存储迁移（JSONL -> v2 sidecar）
pi migrate ~/.pi/agent/sessions --dry-run
pi migrate ~/.pi/agent/sessions
```

### 其他常用选项

| 选项                         | 说明                                 |
| :--------------------------- | :----------------------------------- |
| `-c, --continue`             | 继续最近会话                         |
| `-r, --resume`               | 打开会话选择器 UI                    |
| `--session <PATH>`           | 打开指定会话文件                     |
| `--session-dir <DIR>`        | 覆盖会话存储目录                     |
| `--no-session`               | 不持久化会话                         |
| `-p, --print`                | 单次响应，无交互                     |
| `--mode text                 | json                                 | rpc`        | 输出/协议模式    |
| `--extension-policy safe     | balanced                             | permissive` | 扩展能力配置文件 |
| `--repair-policy off         | suggest                              | auto-safe   | auto-strict`     | 扩展自动修复策略 |
| `--list-models [PATTERN]`    | 列出可用模型（可选模糊过滤）         |
| `--list-providers`           | 列出 provider ID、别名、认证 env key |
| `--export <PATH>`            | 导出会话为 HTML                      |
| `--no-migrations`            | 跳过启动迁移检查                     |
| `--explain-extension-policy` | 打印生效的能力决策并退出             |
| `--explain-repair-policy`    | 打印生效的修复策略解析并退出         |

## 工具配置

### 运行时禁用工具

在 `settings.json` 中设置 `disabledTools` 数组，启动时自动过滤：

```json
{
  "disabledTools": ["bash"]
}
```

支持 `camelCase`（`disabledTools`）和 `snake_case`（`disabled_tools`）两种格式。

### `--tools` CLI 参数

覆盖启用的工具列表（逗号分隔）：

```bash
pi --tools read,write,edit,grep,find,ls,hashline_edit,pwsh
```

## RPC 协议

### `append_custom_entry`（会话注入）

客户端注入自定义 entry（如 pidian 苏格拉底消息）经 pi 会话管理落盘：

```
请求: { "type": "request", "id": "req_1", "command": "append_custom_entry",
        "customType": "socratic", "data": { "kind": "challenge", ... } }
响应: { "type": "response", "id": "req_1", "command": "append_custom_entry",
        "success": true, "data": { "entryId": "<8-hex>" } }
```

- `customType` 必填且非空；`data` 可选任意 JSON
- 落盘格式：`{"type":"custom","id":...,"parentId":...,"timestamp":...,"customType":...,"data":...}`（camelCase）
- CustomEntry **不进入 API 消息链路**（`append_model_message_for_entry` 忽略），不影响 LLM 请求
- 完整 RPC 方法清单见 `features.md`「会话管理」域

## 版本迁移注意事项

- `asupersync` 和 `rich_rust` 为外部依赖（sibling 项目）
- 从 TypeScript 迁移时参考 `docs/sdk.md` 的迁移映射表
