# 修复: QuickJS execSync 找不到 PATH 中的 CLI 工具（.cargo/bin）

## 根因分析

经过代码审查，发现两个根因：

### 1. `allow_unsafe_sync_exec` 默认为 `false`

`PiJsRuntimeConfig` 的 `allow_unsafe_sync_exec` 字段默认是 `false`，导致所有 `execSync`/`spawnSync` 在策略能力检查之前就被无条件拒绝，返回 "sync child_process APIs are disabled by default"。

生产环境中创建 `PiJsRuntimeConfig` 的代码（`src/agent.rs:7916`、`src/main.rs:1203`）都使用 `..Default::default()`，没有覆盖该字段。

### 2. JS 包装器硬编码 `"sh"` 作为 shell

`execSync` 和 `exec`（异步）的 JS 包装器都硬编码 `"sh"` 作为 shell 命令。但：
- Windows 上 `sh` 不是标准命令（需要 Git Bash/MSYS2）
- Node.js 的 `execSync` 在 Windows 上使用 `cmd.exe /c`，Unix 上使用 `/bin/sh -c`

## 改动内容

### 修改 1: `src/extensions.rs` — 策略联动

在 `start_inner()` 中，当策略不拒绝 `exec` 能力时，自动启用 `allow_unsafe_sync_exec`：

```rust
if !policy.deny_caps.contains(&"exec".to_string()) {
    config.allow_unsafe_sync_exec = true;
}
```

遵循与现有 `deny_env` 一致的策略联动模式。这样：
- 策略允许 `exec` → 同步执行可用（仍受 per-extension 能力和 exec_mediation 检查）
- 策略拒绝 `exec` → 同步执行被拒绝（纵深防御）

### 修改 2: `src/extensions_js.rs` — `execSync` 平台检测

`execSync` 函数现在检测 `process.platform`：
- `win32` → 使用 `cmd.exe /d /s /c <command>`
- 其他平台 → 使用 `sh -c <command>`（保持向后兼容）

### 修改 3: `src/extensions_js.rs` — `exec` 平台检测

异步 `exec` 函数同样检测平台：
- `win32` → `spawn("cmd.exe", ["/d", "/s", "/c", cmdStr])`
- 其他平台 → `spawn("sh", ["-c", cmdStr])`

## 测试

- `cargo check --all-targets` ✅
- `cargo clippy -p pi_agent_rust --all-targets` ✅（pi-core 的预存在警告不受影响）
- `cargo fmt --check` ✅
- `cargo test extension` → 2084 通过，8 失败（预存在，Windows 缺少 python3/sh）
- `cargo test exec_sync` → 3 通过（denied、mediation、empty），4 失败（预存在，缺少 python3/sh）
