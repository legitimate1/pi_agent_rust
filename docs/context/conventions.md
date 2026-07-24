# 命名规范与隐含假设

## 命名规范

| 类别 | 规范 | 示例 |
|:-----|:-----|:-----|
| 内置工具名 | 小写 | `read` `write` `edit` `bash` |
| Provider 名 | 小写 | `anthropic` `openai` |
| 扩展 ID | `ext.*` 前缀 | `ext.tools_fs` |
| Rust 源码 | `snake_case` | `extension_tools.rs` |
| 测试函数 | `snake_case` | `test_tool_override` |

## 隐含假设

- Rust 2024 nightly 编译（见 `rust-toolchain.toml`）
- 依赖 `asupersync`（结构化并发运行时）和 `rich_rust`（终端 UI）
- 扩展 JS/TS 在 QuickJS 沙箱中运行，无 Node/Bun 依赖
- Node.js 内置模块（`node:fs`、`node:path` 等）通过 QuickJS 垫片实现
- `@mariozechner/pi-coding-agent` 等 npm 包为虚拟模块
- **Cargo 不自动清理旧编译产物** — 每次 `cargo build`/`cargo test` 生成新 hash 的文件（.exe/.pdb/.rlib），旧文件永久保留。Windows 上 .pdb 文件尤为庞大。需用 `cargo-sweep` 主动管理。

## 子进程管理约定

所有 spawn 子进程的内置工具**必须**使用 `ProcessGuard`（`src/tools/mod.rs`）管理子进程生命周期，不得直接使用裸 `std::process::Child`。

### 必须使用 ProcessGuard

```rust
// ❌ 错误：裸 Child，Drop 时不 kill，abort 时变孤儿进程
let mut child = cmd.spawn()?;
child.try_wait()...;

// ✅ 正确：ProcessGuard 封装，Drop 时自动 kill + wait 回收
let mut guard = ProcessGuard::new(child, ProcessCleanupMode::ChildOnly);
guard.wait_with_cancellation(timeout_secs).await?;
```

### 选择 cleanup 模式

| 模式 | 适用场景 | 说明 |
|:-----|:---------|:------|
| `ChildOnly` | grep、find、pwsh | 只 kill 直接子进程，不涉及子进程组 |
| `ProcessGroupTree` | bash | kill 整个进程组树，包括 shell 启动的后台进程 |

### 标准化 wait 方法

优先使用 `guard.wait_with_cancellation(timeout_secs)`，它内置：
- **Ambient cancellation** — 通过 `AgentCx::checkpoint()` 检测 abort
- **超时 kill** — 超时后自动 kill 子进程
- **适应性 sleep** — 接近超时时缩短轮询间隔

```rust
// 推荐：一行涵盖 timeout + cancellation
let exit_code = guard.wait_with_cancellation(timeout_secs).await
    .ok()
    .flatten()
    .unwrap_or(-1);
```

### 需要规避

- ❌ 直接使用 `std::process::Child` 裸对象
- ❌ 手动实现 wait 循环时遗漏 cancellation 检查
- ❌ wait 循环中用 `std::thread::sleep` 阻塞异步运行时（用 `asupersync::time::sleep` 代替）

## 反模式

| ❌ 不要 | ✅ 应该 | 原因 |
|:--------|:--------|:------|
| 用脚本批量改代码 | 手动逐处修改 | 正则替换容易引入 Bug |
| 创建 `main_v2.rs` 等变体 | 原地修改原文件 | 文件膨胀导致混乱 |
| 使用不安全的 `unsafe` 代码 | 纯 safe Rust | 项目 `forbid(unsafe_code)` |
| 放任 `target/` 无限膨胀 | 定期 `cargo sweep --file` 清理旧产物 | Cargo 永不删除旧文件，debug .pdb 和增量缓存可累积到数百 GB |
| 用裸 `std::process::Child` | 用 `ProcessGuard` 封装 | Drop 时不 kill 子进程，abort 后变孤儿 |
