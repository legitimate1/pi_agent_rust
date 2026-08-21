# 依赖升级日志

**日期:** 2026-02-14
**项目:** pi_agent_rust
**语言:** Rust
**清单:** `Cargo.toml`, `fuzz/Cargo.toml`

---

## 概要

| 指标 | 数量 |
|--------|-------|
| **总依赖（直接，已过期）** | 18 |
| **已更新** | 18 |
| **已跳过** | 0 |
| **失败（已回滚）** | 0 |
| **需要关注** | 0 |

---

## 发现

检测到的清单：
- `Cargo.toml`
- `fuzz/Cargo.toml`

检测到的过期直接依赖（当前 -> 最新稳定版）：
- `anyhow` `1.0.100` -> `1.0.101`
- `clap` `4.5.56` -> `4.5.58`
- `clap_complete` `4.5.65` -> `4.5.66`
- `criterion` `0.7.0` -> `0.8.2`
- `ctrlc` `3.5.1` -> `3.5.2`
- `getrandom` `0.2.17` -> `0.4.1`
- `jsonschema` `0.40.2` -> `0.42.0`
- `memchr` `2.7.6` -> `2.8.0`
- `proptest` `1.9.0` -> `1.10.0`
- `regex` `1.12.2` -> `1.12.3`
- `sysinfo` `0.36.1` -> `0.38.1`
- `tempfile` `3.24.0` -> `3.25.0`
- `toml` `0.8.23` -> `1.0.1+spec-1.1.0`
- `uuid` `1.20.0` -> `1.21.0`
- `vergen` `9.0.6` -> `9.1.0` (fuzz)
- `vergen-gix` `1.0.9` -> `9.1.0`
- `wasmtime` `29.0.1` -> `41.0.3`
- `wat` `1.244.0` -> `1.245.1`

---

## 已成功更新

- 根清单（`Cargo.toml`）直接依赖声明已更新：
  - `anyhow = "1.0.101"`
  - `clap = "4.5.58"`
  - `clap_complete = "4.5.66"`
  - `ctrlc = "3.5.2"`
  - `tempfile = "3.25.0"`
  - `uuid = "1.21.0"`
  - `memchr = "2.8.0"`
  - `getrandom = "0.4.1"`
  - `regex = "1.12.3"`
  - `sysinfo = "0.38.1"`
  - `wasmtime = "41.0.3"`
  - `vergen-gix = "9.1.0"`
  - dev-deps: `criterion = "0.8.2"`, `jsonschema = "0.42.0"`, `proptest = "1.10.0"`, `wat = "1.245.1"`, `toml = "1.0.1"`, `tempfile = "3.25.0"`
- Fuzz 清单（`fuzz/Cargo.toml`）构建依赖已更新：
  - `vergen-gix = "=9.1.0"`
  - `vergen = "=9.1.0"`
- 锁文件已使用最新兼容解析结果刷新。

---

## 兼容性 / 后续修复已应用

为在升级后的工具链/依赖集上保持项目可用，需进行额外的代码更新：

- `wasmtime` 41 在 `src/extensions.rs` 与 `src/pi_wasm.rs` 中的 API/宏迁移：
  - `component::bindgen!` 异步配置切换为 `imports/exports` 标志。
  - 已更新 linker 胶水代码以适配 `HasSelf` 泛型用法。
  - 已处理新增的 `Extern::Tag` 变体。
- 事件枚举扩展（`AgentEvent::ExtensionError`）导致多个文件中的现有 `match` 变为非穷尽；已更新所有受影响的匹配点。
- 已修复在 `-D warnings` 下新增的 `clippy` 检查发现，涉及测试/基准测试与辅助代码（文档 Markdown、浮点断言、冗余克隆/闭包、格式化字符串内联等）。

---

## 验证

已执行（因共享的 `/dev/shm` 与 `/tmp` 空间耗尽，构建目录位于 `/var/tmp`）：

```bash
export CARGO_TARGET_DIR="/var/tmp/pi_agent_rust/${USER:-agent}/target"
export TMPDIR="/var/tmp/pi_agent_rust/${USER:-agent}/tmp"
mkdir -p "$TMPDIR"

rch exec -- cargo check --all-targets
rch exec -- cargo clippy --all-targets -- -D warnings
rch exec -- cargo fmt --check
```

结果：
- `cargo check --all-targets` ✅
- `cargo clippy --all-targets -- -D warnings` ✅
- `cargo fmt --check` ✅

---

## 使用的命令

```bash
# Discovery / inventory
cargo metadata --format-version 1 --no-deps
cargo metadata --manifest-path fuzz/Cargo.toml --format-version 1 --no-deps
cargo tree --depth 1 --prefix none
cargo tree --manifest-path fuzz/Cargo.toml --depth 1 --prefix none

# Upgrade + resolve
rch exec -- cargo update
rch exec -- cargo update --manifest-path fuzz/Cargo.toml

# Validation
rch exec -- cargo check --all-targets
rch exec -- cargo clippy --all-targets -- -D warnings
rch exec -- cargo fmt --check
```
