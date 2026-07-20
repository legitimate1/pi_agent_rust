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

## 反模式

| ❌ 不要 | ✅ 应该 | 原因 |
|:--------|:--------|:------|
| 用脚本批量改代码 | 手动逐处修改 | 正则替换容易引入 Bug |
| 创建 `main_v2.rs` 等变体 | 原地修改原文件 | 文件膨胀导致混乱 |
| 使用不安全的 `unsafe` 代码 | 纯 safe Rust | 项目 `forbid(unsafe_code)` |
| 放任 `target/` 无限膨胀 | 定期 `cargo sweep --file` 清理旧产物 | Cargo 永不删除旧文件，debug .pdb 和增量缓存可累积到数百 GB |
