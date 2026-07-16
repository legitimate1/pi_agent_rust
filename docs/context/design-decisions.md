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

## D5: `~/.pi/agent/SYSTEM.md` 覆盖默认系统提示词（2026-07-15）

**决策**：当 `~/.pi/agent/SYSTEM.md` 存在时，替代 `default_system_prompt()` 作为 system prompt 的基础内容。

**涉及改动**：
1. `src/app.rs` → `build_system_prompt()` 新增 `system_md_override` 逻辑，在 `--system-prompt` 未提供时检查 `global_dir/SYSTEM.md`

**理由**：
- 原版 Node.js Pi Agent 支持此约定，用户生态依赖此行为
- 允许用户完全自定义 LLM 的人格指令，而不需要每次通过 CLI 传入 `--system-prompt`
- 后续的追加内容（`--append-system-prompt`、project context files、skills prompt、日期/CWD）仍然正常注入

**不选 B 的原因**：
- 要求用户改用 `--system-prompt` 参数——每次都需手动传入，无法持久化

**何时重新考虑**：如果未来提供更细粒度的提示词分层机制（per-task prompt overlay），可合并。

## D6: 工具描述外部化（2026-07-15）

**决策**：支持通过 `settings.json` 的 `toolDescriptions` 字段，在运行时覆盖内置工具的 `description()`。

**涉及改动**：
1. `src/config.rs` → 新增 `tool_descriptions: Option<HashMap<String, String>>` 字段
2. `src/tools/mod.rs` → `ToolRegistry` 新增 `description_overrides` 字段，从 Config 提取
3. `src/agent.rs` → `build_context()` 构建 `ToolDef` 时优先使用 override

**理由**：
- 用户需要调整工具描述（如中文本地化）但不希望修改源码重新编译
- settings.json 是已有配置入口，用户已熟悉

**不选 B 的原因**：
- 新增独立 `tools.json` 文件——增加配置入口数量，用户需额外学习
- 环境变量——描述文本太长，不适合 env var

**何时重新考虑**：如果未来工具数量大幅增长，可考虑拆为独立文件。

## D7: 工具和技能提示词中文本地化（2026-07-15）

**决策**：内置工具的 `description()` 和 skills prompt 提示文本改为中文。

**涉及改动**：
1. `src/tools/*.rs` → 9 个工具的 `description()` 全部改为中文
2. `src/resources.rs` → `format_skills_for_prompt()` 的引导文本改为中文

**理由**：
- 用户使用中文交互，工具描述和技能提示使用中文更自然

**不选 B 的原因**：
- 保留英文——用户已确认系统提示词由 SYSTEM.md 接管，工具描述通过 API 发送，中文更一致

**何时重新考虑**：如果未来需要多语言支持，可考虑 i18n 框架。

## D9: 移除 write/edit 工具的 CWD 路径限制（2026-07-15）

**决策**：移除 `write` 和 `edit` 工具的 `enforce_cwd_scope()` 调用，允许写入任意绝对路径。

**涉及改动**：
1. `src/tools/write.rs` → 移除 `enforce_cwd_scope(&path, &self.cwd, "write")`
2. `src/tools/edit.rs` → 移除 `enforce_cwd_scope(&absolute_path, &self.cwd, "edit")`

**理由**：
- CWD 路径限制在沙盒进程环境下反而产生误导——os error 5（拒绝访问）让用户误以为是权限问题，而实际是路径越界校验
- write 工具不应比 pwsh/bash 等 shell 工具有更多路径限制
- 用户需要写入项目目录之外的路径（如生成配置文件到 `~/.pi/`）

**不选 B 的原因**：
- 保留限制并改进错误提示——仍然无法写入 CWD 之外，功能不足
- 改为白名单模式——增加了不必要的配置复杂度

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

## D10: Edit 工具写入改为直接写（Windows 句柄冲突）（2026-07-15）

**决策**：`EditTool` 的写入路径从 `tempfile::NamedTempFile::persist()`（原子重命名）改为 `std::fs::write()`（直接写入）。

**涉及改动**：
1. `src/tools/mod.rs` → 新增 `persist_with_readonly_handling()` 辅助函数，处理 Windows `FILE_ATTRIBUTE_READONLY` 导致 `MoveFileEx` 失败的问题（诊断式：先 persist，失败后检查 readonly，清除后重试）
2. `src/tools/edit.rs` → `spawn_blocking_io` 内的写入替换为 `std::fs::write`

**理由**：
- `EditTool` 在写入前使用 `asupersync::fs::File::open` 读取文件内容，Windows 上 async 读操作可能未及时释放文件句柄，导致后续 `MoveFileEx`（persist 的底层调用）报 `ERROR_ACCESS_DENIED`
- `WriteTool` 不受影响，因为它不先读取文件内容
- 直接写入绕过 tempfile 的重命名链路，消除句柄冲突
- 文件已通过前置权限检查确认可写，直接写入安全

**不选 B 的原因**：
- 继续排查 asupersync 的句柄释放时序——框架层修复周期长，且可能影响其他模块
- 保留原子重命名并增加 retry——编辑场景下写入目标唯一，原子性非必须

**何时重新考虑**：如果 `asupersync` 底层修复了 Windows 句柄释放问题，或 edit 改为同步读文件，可恢复原子重命名。

## D11: 移除 read 工具的 CWD/agent-dir 路径限制（2026-07-15）

**决策**：移除 `read` 工具的 `enforce_read_scope()` 调用，允许读取任意绝对路径。

**涉及改动**：
1. `src/tools/read.rs` → 移除两处 `enforce_read_scope(&path, &self.cwd)?` 调用（主读取路径 + diff 目标路径）

**理由**：
- 与 D9（移除 write/edit 路径限制）一致——read 不应比 pwsh/bash 等 shell 工具有更多路径限制
- 验证报错 `"Cannot read outside the working directory or agent dir"` 阻止了读取项目目录之外的必要文件（如 `~/.pi/` 配置文件、`/data/tmp/` 下的构建产物）
- 用户工作流中需要读取 CWD 之外的文件（如跨项目引用、系统配置文件）

**不选 B 的原因**：
- 保留限制并改进错误提示——仍然只能读 CWD/agent-dir 内的文件
- 改为白名单模式——增加了不必要的配置复杂度，与 D9 的决策不一致

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

## D12: run_extension_command 总是发送 agent_end（2026-07-15）

**决策**：`run_extension_command` 执行完毕后无论成功或失败，始终发送 `agent_end` 事件通知 RPC 客户端。

**涉及改动**：
1. `src/rpc.rs:2491-2501` → `agent_end` payload 移出 `if let Err` 分支，成功时不含 `error` 字段，失败时附带错误信息

**理由**：
- RPC 客户端（pidian）依赖 `agent_end` 事件触发 `finalizeStreaming()` 来终结流式渲染状态
- 先前仅在错误时发送 `agent_end`，成功时客户端 `isStreaming=true` 永远不终结，消息内容无法 finalize
- TUI 交互模式不受影响，因为它不依赖 `agent_end` 事件

**不选 B 的原因**：
- 在成功分支也补发 `agent_end` 但保持 `isStreaming=false` 的时序不变——逻辑等价，只是写法更冗余
- 让 pidian 侧增加超时 fallback——治标不治本，掩盖了 pi-rust 的事件缺失问题

**何时重新考虑**：如果未来 RPC 协议改用更明确的流式生命周期消息替代 `agent_end`，可整体重构。

## D13: RPC 模式扩展交互 UI 传递 hasUI 上下文（2026-07-16）

**决策**：`ExtensionManager::execute_command()` 构建 JS 上下文时，使用 snapshot 的 `has_ui` 字段，替代硬编码的空对象 `{}`。

**涉及改动**：
1. `src/extensions.rs:29959` → `Arc::new(json!({}))` 改为 `let ctx = json!({ "hasUI": self.read_snapshot().has_ui })`

**理由**：
- RPC 模式下 `ExtensionManager` 已配置 UI 通道（`ui_sender`），snapshot 的 `has_ui` 为 `true`
- 但 `execute_command` 直接传空 `{}` 给 JS，导致 `__pi_make_extension_ctx` 中 `hasUI = false`
- `hasUI = false` 时 JS 侧 `ctx.ui.select/confirm/input` 静默返回 `undefined/false`，不调用 `pi.ui()`，扩展 handler 收到空值后输出"已取消"
- 使用 snapshot 的 `has_ui` 字段可准确反映当前是否有 UI 通道

**不选 B 的原因**：
- 硬编码 `true`——纯 headless 场景无 UI 通道，`select` 等操作无法完成，应返回 undefined
- 手动传参——调用方繁多，遗漏风险大

**何时重新考虑**：如果未来 UI 通道模型改为每个连接独立配置，可改为从调用上下文动态推导。

## D8: Release 构建 — 栈溢出问题（2026-07-15）

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
