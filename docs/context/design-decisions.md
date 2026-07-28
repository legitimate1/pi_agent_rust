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

## D14: 移除 find 工具的 CWD 路径限制（2026-07-17）

**决策**：移除 `find` 工具的 `enforce_cwd_scope()` 调用，允许在任意绝对路径下搜索文件。

**涉及改动**：
1. `src/tools/find.rs` → 移除 `enforce_cwd_scope(&search_path, &self.cwd, "find")?`

**理由**：
- 与 D9（移除 write/edit 路径限制）和 D11（移除 read 路径限制）一致 — find 不应比 pwsh/bash 等 shell 工具有更多路径限制
- 验证报错 `"Cannot find outside the working directory"` 阻止了在 CWD 之外搜索必要文件（如 `~/.pi/agent/skills/` 下的技能文件、Home 目录下的配置文件）
- Agent 工作流（如 think-mode 技能的步骤 ⑩）要求在 `~/` 下搜索文件

**不选 B 的原因**：
- 保留限制并改进错误提示——仍然只能在 CWD 内搜索
- 改为白名单模式——增加了不必要的配置复杂度，与 D9/D11 的决策不一致

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

## D15: 移除 grep/ls 工具的 CWD 路径限制（2026-07-17）

**决策**：移除 `grep` 和 `ls` 工具的 `enforce_cwd_scope()` 调用，允许在任意绝对路径下使用。

**涉及改动**：
1. `src/tools/grep.rs` → 移除 `enforce_cwd_scope(&search_path, &self.cwd, "grep")?`
2. `src/tools/ls.rs` → 移除 `enforce_cwd_scope(&dir_path, &self.cwd, "list")?`

**理由**：
- 与 D14（移除 find 限制）、D9/D11 保持一致 — 所有文件系统工具已全部放开 CWD 限制
- grep/ls 不应比 pwsh/bash 等 shell 工具有更多路径限制

**不选 B 的原因**：
- 保留限制——与已放开的其他工具不一致，增加用户心智负担

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

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

## D16: RPC 进程侧主动会话持久化（2026-07-19）

**决策**：在 `pi --mode rpc` 的 event handler 中捕获 `TurnEnd` 事件，通过背景线程（`RpcSessionPersister`）将已完成的 assistant/tool_result 消息实时追加写入 JSONL 文件。

**涉及改动**：
1. `src/rpc.rs` → 新增 `RpcSessionPersister` 结构体 + `writer_thread` 背景线程；`rpc_agent_event_handler` 新增 `session_persister` 参数；`run_prompt_with_retry` 在重试前创建 persister（仅 `save_enabled` 且有文件路径时）

**理由**：
- Obsidian 崩溃或卡死时，正在进行的会话数据全部丢失
- 当前持久化完全依赖 pidian（Obsidian 插件侧）触发 `saveConversation()`，整个 turn 的数据只在 turn 结束后一次性写入
- `RpcSessionPersister` 是独立进程的机制，Obsidian 崩溃不影响它，重启后可恢复已落盘的消息
- 最终 `persist_new_messages` 的标准重写会覆盖我们的条目，不影响 session 的规范 ID

**不选 B 的原因**：
- 在 agent 核心循环中加中间落盘——侵入性强，影响所有模式
- 周期性定时器 flush session——但 turn 运行中的消息在 `self.agent.messages()` 内存中，不在 session 对象里，flush 无效果
- 提高 pidian 的 `saveConversation()` 频率——仍然依赖客户端触发，Obsidian 崩溃时没用

**何时重新考虑**：如果以后不再依靠 JSONL 格式（如完全迁移到 SQLite 后端），可移除或替换。

## D17: 项目级 `.pi/SYSTEM.md` 覆盖（2026-07-20）

**决策**：在 `build_system_prompt()` 中新增项目级 `.pi/SYSTEM.md` 检查，优先级介于 `--system-prompt` 与 `~/.pi/agent/SYSTEM.md` 之间。

**涉及改动**：
1. `src/app.rs` → 将原来的 `system_md_override`（只查 `global_dir/SYSTEM.md`）拆为 `project_system_md` + `user_system_md`，优先检查 `cwd/.pi/SYSTEM.md`

**理由**：
- 允许项目所有者锁定系统提示词内容，不被用户级 SYSTEM.md 意外覆盖
- 避免团队项目中不同成员使用不同的 SYSTEM.md 导致行为不一致
- 优先级链清晰：`--system-prompt > .pi/SYSTEM.md > ~/.pi/agent/SYSTEM.md > default`

**不选 B 的原因**：
- 复用 `~/.pi/agent/SYSTEM.md` 并覆盖——全局文件修改影响所有项目，粒度太粗
- 要求用户改用 `--system-prompt` 参数——每次都需手动传入，无法持久化

**何时重新考虑**：如果未来提供更细粒度的提示词分层机制（per-task prompt overlay），可合并。

## D18: 项目技能加载模式 `skill_mode`（2026-07-20）

**决策**：在 `.pi/settings.json` 中新增 `skill_mode: "project_only"` 配置，项目可跳过全局技能，只加载项目技能。

**涉及改动**：
1. `src/package_manager.rs` → 新增 `SkillMode` 枚举（`All` / `ProjectOnly`）；`SettingsSnapshot` 增加 `skill_mode` 字段；`read_settings_snapshot()` 解析 `"skill_mode"`；`resolve_with_roots()` 在汇总后过滤掉 `PackageScope::User` 的技能

**理由**：
- 某些项目携带了大量专属技能（如文档辅助、领域特定指令），全局技能反而干扰
- 避免全局技能污染项目上下文，减少 token 消耗
- 仅影响技能（skills），不影响 extensions/prompts/themes
- 默认行为不变（`All`），只有显式配置才切换，不影响现有用户

**不选 B 的原因**：
- 用 `--no-skills` 全局关闭——粒度太粗，影响所有技能
- 在技能清单中逐个 disable——维护成本高，容易遗漏
- 通过 package 机制做白名单——对非 package 的自动发现技能无效

**何时重新考虑**：如果未来引入更细粒度的技能可见性策略（per-skill scope、标签路由等），可合并。

## D19: `global_skills` 白名单 — 选择加载特定全局技能（2026-07-20）

**决策**：在 `.pi/settings.json` 中新增 `global_skills` 字符串数组配置，项目可指定只加载列表中的全局技能，其余全局技能被过滤。

**涉及改动**：
1. `src/package_manager.rs` → `SettingsSnapshot` 新增 `global_skills` 字段；`read_settings_snapshot()` 解析 `"global_skills"` 数组；新增 `skill_name_from_resolved_path()` 辅助函数（从路径父目录名提取技能名）；`resolve_with_roots()` 中合并到技能过滤逻辑，与 `skill_mode` 叠加

**理由**：
- 有些项目只需要少数几个全局技能（如 `add-tests`），不需要全部加载
- 与 `skill_mode: "project_only"` 叠加使用：先排除全部全局技能，再按白名单恢复特定技能
- `global_skills` 为空时表示"不过滤"，非空时才生效，向前兼容
- 项目技能始终不受影响

**不选 B 的原因**：
- 通过 settings.json 的 `skills` 数组逐个 disable——需要先知道全局技能列表，维护成本高
- 将技能移到项目目录——破坏全局技能的统一管理，无法复用更新
- 环境变量——不适合传递列表，且不易持久化

**何时重新考虑**：如果未来引入基于标签/分类的技能可见性系统，可合并或废弃。

## D20: RPC 队列管理命令 + `queue_update` 事件 + 消息 ID（2026-07-21）

**决策**：在 RPC 协议中新增 3 个队列管理命令（`remove_from_queue`、`clear_queue`、`get_queue`），新增 `queue_update` 事件实时推送队列状态，并为每条入队消息携带 `messageId`。

**涉及改动**：
1. `src/rpc.rs` → 新增 `QueuedMessage` 结构体（含 `message_id` + `Message`）；`RpcSharedState` 队列从 `VecDeque<Message>` 升级为 `VecDeque<QueuedMessage>`；新增 `remove_by_message_id()`、`clear()` 方法
2. `src/rpc.rs` → 新增 3 个 RPC 命令 handler（`remove_from_queue`/`clear_queue`/`get_queue`）
3. `src/rpc.rs` → 新增 `build_queue_snapshot()` 函数 + 在 `steer`/`follow_up`/`prompt` 入队成功后发射 `queue_update`
4. `src/rpc.rs` → `steer`/`follow_up`/`prompt` 从请求中读取可选 `messageId`，未提供时 `uuid::Uuid::new_v4()` 生成

**理由**：
- pidian（Obsidian 插件）需要在 UI 上展示队列中每条消息并支持 `✕` 按钮精确取消
- 原先队列只有入队/出队，客户端无法感知队列当前内容，也无法精确操作某条消息
- `queue_update` 事件让客户端在所有队列变更时同步到 UI，无需轮询
- `messageId` 作为消息的持久引用 ID，是 `remove_from_queue` 的操作手柄

**不选 B 的原因**：
- 用 `Message` 本身作引用——`Message` 包含完整 content，JSON 序列化后发回客户端做匹配，网络开销大且语义不清晰
- 仅用索引引用——并发场景下索引漂移，竞态条件会导致误删
- 客户端自行维护队列镜像——客户端和服务端状态可能不一致，且增加客户端复杂度
- 整体替换队列（`replace_queue`）——使用场景不明确，过度设计

**何时重新考虑**：如果未来 RPC 协议升级为双向流式 IDL（如 gRPC），可重新设计队列同步机制。

## D21: OpenAI 兼容推理模型的 reasoning_effort 支持（2026-07-24）

**决策**：新增 `ReasoningStyle::Standard` 变体，所有非 DeepSeek 的 OpenAI 兼容推理模型（`reasoning: true`）在请求中发送 `reasoning_effort` 字段（`low`/`medium`/`high`），而非静默丢弃。

**涉及改动**：
1. `src/providers/openai.rs` → `ReasoningStyle` 枚举新增 `Standard`；`reasoning_style()` 非 DeepSeek 推理模型返回 `Some(ReasoningStyle::Standard)` 而非 `None`；`build_request()` 新增 `Standard` 分支，读取 `compat.thinkingLevelMap` 映射或使用默认映射
2. `src/providers/openai.rs` → `OpenAIRequest.reasoning_effort` 类型从 `Option<&'static str>` 改为 `Option<String>`，支持动态映射值
3. `C:\Users\m\.pi\agent\models.json` → `gpt-5.6-sol` 的 `thinkingLevelMap` 从模型顶层移入 `compat` 对象内部（`ModelConfig` 无 `thinking_level_map` 字段，serde 静默丢弃；`CompatConfig` 才有）

**理由**：
- 原先 `reasoning_style()` 只在 DeepSeek 路径中返回 `Some(...)`，所有其他 OpenAI 兼容推理模型走 `None` 分支 → `(None, None)` → 不发送任何思考控制字段
- Lucis API 的 `gpt-5.6-sol` 实测支持标准 OpenAI `reasoning_effort` 参数，但 Pi 从未发送该参数

**不选 B 的原因**：
- 把非 DeepSeek 推理模型也送入 DeepSeek 路径——会错误发送 `thinking: {type: "enabled"}` 对象，非 DeepSeek API 可能拒绝或忽略
- 仅改 models.json 不改代码——`thinkingLevelMap` 在 `ModelConfig` 中不存在，无论如何都读不到

**何时重新考虑**：如果未来 OpenAI 官方 Chat Completions API 原生支持 `reasoning_effort` 之外的思考参数，可统一所有推理模型到一个分支。

## D22: 编辑后轻量验证系统（Verify）（2026-07-27 → 2026-07-28 default 改为 true）

**决策**：为 `edit`/`hashline_edit`/`write` 三个工具增加可选 `verify` 参数（默认 `true`），编辑成功后自动对文件运行轻量语法/格式检查。结果附在工具输出的 `details.verify` 字段中，不阻断编辑流程。

**涉及改动**：
1. `src/tools/verify.rs` → 新增内部验证引擎
2. `src/tools/edit.rs` → `EditInput` 增加 `verify: bool` + 编辑成功后条件调用验证
3. `src/tools/hashline.rs` → `HashlineEditInput` 增加 `verify: bool` + 所有 edits 应用完后调用验证
4. `src/tools/write.rs` → `WriteInput` 增加 `verify: bool` + 写入成功后条件调用验证
5. `Cargo.toml` → `toml` 从 dev-deps 提升为正式 deps
6. `src/tools/mod.rs` → 注册 `pub(crate) mod verify`，新增 `default_verify() -> bool`

**理由**：
- Agent 编辑文件后缺少自动格式/语法验证环节，需要额外手动检查，增加迭代来回
- 进程内检查器（JSON/TOML）零额外开销，已在依赖中
- 文件类型直接映射策略确定性强、零扫描成本，适合 agent 编辑循环

**不选 B 的原因**：
- ~~默认 `verify=true` → 中间态误报 + 批量编辑性能损耗~~（2026-07-28 重新评估：单文件验证 <50ms，延迟可忽略；Agent 每次 edit 都是完整改动，中间态极少）
- 暴露为独立 LLM-visible tool → LLM 已有 pwsh 可手动检查
- 自动修正 → 违反"只报告不修"铁律

**默认值变更（2026-07-28）**：初始实现默认 `false`，经验证验证开销可忽略且能减少迭代来回，改为默认 `true`。Agent 可显式传 `verify: false` 跳过。<br>
**何时重新考虑**：需要支持更多文件类型时，扩展映射表即可。

