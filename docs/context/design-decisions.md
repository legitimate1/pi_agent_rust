# 设计决策

## D1: 扩展工具同名覆盖内置工具（2026-07-14）

**决策**：当扩展注册的工具与内置工具同名时，扩展工具替换内置工具。

**涉及改动**：
1. `src/tools/mod.rs` → 新增 `ToolRegistry::retain()` 方法，支持按谓词移除工具
2. `src/agent.rs` → 重写 `Agent::extend_tools()`，先收集扩展工具名，移除同名的内置工具，再追加扩展工具

**理由**：
- 旧版 Node.js Pi Agent 支持扩展覆盖同名内置工具，用户生态依赖此行为
- Rust 版原先直接追加，导致 `tools` 数组出现重复定义，上游 Provider 拒绝（HTTP 400）
- 用户在 `extensions/` 放置 `tools-fs`（含 `read`/`write`/`edit` 等与内置同名的工具）时触发

**不选 B 的原因**：
- 强制扩展工具改名——破坏与旧版的兼容性，所有现有扩展需修改工具名
- 跳过重复工具不做处理——Provider 侧 API 不接受重复 tool 定义
- 在扩展收集侧过滤——无法感知哪些是内置工具

**何时重新考虑**：如果 Provider API 规范明确要求支持同名工具（如 OpenAI 的未来版本），可改为保留双份。

## D2: 扩展系统使用 QuickJS 沙箱（内置）

**决策**：JS/TS 扩展在 QuickJS 沙箱中运行，不经 Node.js 转译。

**理由**：消除 Node.js 依赖、加速扩展加载、增强安全性。

## D3: 运行时禁用内置工具配置（2026-07-14）

**决策**：在 `settings.json` 中增加 `disabledTools` 字段，启动时从启用列表过滤掉禁用的工具。

**涉及改动**：
1. `src/config.rs` → 新增 `disabled_tools: Option<Vec<String>>` 字段
2. `src/main.rs:1393-1398` → 读取配置并过滤 `enabled_tools`

**理由**：
- 用户需要在 Windows 上禁用 `bash` 工具（兼容性问题）
- 之前只能通过 `--tools` CLI 参数或 shell alias 绕过，不够方便
- 配置化后改 `settings.json` 即可，无需重新编译

**不选 B 的原因**：
- 直接改 CLI 默认值——影响所有用户，不灵活
- 改源码硬编码跳过——每次加/减都要重新编译

**何时重新考虑**：如果未来有更细粒度的工具权限管理（per-tool policy），可合并。

## D4: 内置 pwsh 工具（2026-07-14）

**决策**：新增 `PwshTool` 作为第 9 个内置工具，通过 `pwsh -NoProfile -Command` 执行 PowerShell 命令。

**涉及改动**：
1. `src/tools/pwsh.rs` → 新增 `PwshTool` 结构体 + `Tool` trait 实现 + `run_pwsh_command()` 函数
2. `src/cli.rs` → 默认工具列表增加 `pwsh`

**理由**：
- Windows 上 `bash` 不可用或兼容性差，PowerShell 7（pwsh）是标准 shell
- JS 扩展版 pwsh 受 QuickJS 沙箱限制，无法使用 `child_process.spawn`
- 内置工具不走扩展沙箱，直接调用 `std::process::Command`，无策略限制

**不选 B 的原因**：
- JS 扩展方式——QuickJS 沙箱限制 `exec` 能力，pwsh 无法执行命令
- MCP 方式——引入外部进程通信，复杂度高
- 保持现状（只有 bash）——Windows 用户没有可用的 shell 工具

**何时重新考虑**：如果 QuickJS 沙箱放开 `exec` 能力限制，可考虑回退到扩展方式。

## D5: Release 构建 — 栈溢出问题（2026-07-15）

**问题**：Debug 构建的 `pi.exe` 在 Windows 上启动即崩溃，报 `thread 'main' has overflowed its stack`。Release 构建正常。

**原因**：Debug 模式下 Rust 不优化尾递归、增加栈安全检测（`__chkstk`），启动时的初始化链路在 Debug 模式超出默认栈大小（Windows 1MB）。Release 模式（LTO + 优化）消除了中间栈帧，绕过该问题。

**对策**：
- 日常使用始终用 `cargo build --release`
- 如需 Debug 构建调试，需增大栈大小：
  ```bash
  # 通过 rustflag 设置 4MB 栈
  # .cargo/config.toml: [target.'cfg(windows)'].rustflags = ["-C", "link-args=/STACK:4194304"]
  ```
- 当前 release profile 已从激进体积优化改为速度优先（`opt-level = 3`, `lto = "thin"`），编译速度与运行效率平衡
