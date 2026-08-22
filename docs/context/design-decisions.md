# 设计决策

## D1: 扩展工具同名覆盖内置工具（2026-07-14）

**决策**：当扩展注册的工具与内置工具同名时，扩展工具替换内置工具。

**理由**：旧版 Node.js Pi Agent 支持此行为，用户生态依赖；原先直接追加导致 `tools` 数组重复定义，被上游 Provider 拒绝（HTTP 400）。

**不选 B 的原因**：强制扩展改名（破坏兼容）；跳过重复工具（Provider 不接受重复定义）；扩展收集侧过滤（无法感知哪些是内置工具）。

**何时重新考虑**：如果 Provider API 规范明确要求支持同名工具，可改为保留双份。

## D2: 扩展系统使用 QuickJS 沙箱（内置）

**决策**：JS/TS 扩展在 QuickJS 沙箱中运行，不经 Node.js 转译。

**理由**：消除 Node.js 依赖、加速扩展加载、增强安全性。

## D3: 运行时禁用内置工具配置（2026-07-14）

**决策**：在 `settings.json` 中增加 `disabledTools` 字段，启动时从启用列表过滤掉禁用的工具。

**理由**：用户需要在 Windows 上禁用 `bash` 工具；此前只能通过 `--tools` CLI 参数或 shell alias 绕过；配置化后改 `settings.json` 即可，无需重新编译。

**不选 B 的原因**：直接改 CLI 默认值（影响所有用户）；改源码硬编码跳过（每次加/减都要重新编译）。

**何时重新考虑**：如果未来有更细粒度的工具权限管理（per-tool policy），可合并。

## D4: 内置 pwsh 工具（2026-07-14）

**决策**：新增 `PwshTool` 作为内置工具，通过 `pwsh -NoProfile -Command` 执行 PowerShell 命令。

**理由**：Windows 上 `bash` 不可用或兼容性差，PowerShell 7 是标准 shell；JS 扩展版 pwsh 受 QuickJS 沙箱限制，无法使用 `child_process.spawn`。

**不选 B 的原因**：JS 扩展方式（沙箱限制 exec）；MCP 方式（引入外部进程通信，复杂度高）；保持只有 bash（Windows 用户没有可用的 shell 工具）。

**何时重新考虑**：如果 QuickJS 沙箱放开 `exec` 能力限制，可考虑回退到扩展方式。

**2026-08-01 修订**：Windows 上 `Command::new("pwsh")` 解析 PATH 时带空格条目被按空格截断，改为优先绝对路径 `%PROGRAMFILES%\PowerShell\7\pwsh.exe`，否则 fallback PATH。通用结论：Windows spawn 程序不要依赖裸命令名解析 PATH，用绝对路径 + 扩展名最稳。

## D5: `~/.pi/agent/SYSTEM.md` 覆盖默认系统提示词（2026-07-15）

**决策**：当 `~/.pi/agent/SYSTEM.md` 存在时，替代 `default_system_prompt()` 作为 system prompt 的基础内容。

**理由**：原版 Node.js Pi Agent 支持此约定，用户生态依赖；允许完全自定义人格指令，无需每次传 `--system-prompt`；后续追加内容仍正常注入。

**不选 B 的原因**：要求用户改用 `--system-prompt` 参数——每次都需手动传入，无法持久化。

**何时重新考虑**：如果未来提供更细粒度的提示词分层机制（per-task prompt overlay），可合并。

## D6: 工具描述外部化（2026-07-15）

**决策**：支持通过 `settings.json` 的 `toolDescriptions` 字段，在运行时覆盖内置工具的 `description()`。

**理由**：用户需要调整工具描述（如中文本地化）但不希望修改源码重新编译；settings.json 是已有配置入口。

**不选 B 的原因**：新增独立 `tools.json` 文件（增加配置入口）；环境变量（描述文本太长，不适合 env var）。

**何时重新考虑**：如果未来工具数量大幅增长，可考虑拆为独立文件。

## D7: 工具和技能提示词中文本地化（2026-07-15）

**决策**：内置工具的 `description()` 和 skills prompt 提示文本改为中文。

**理由**：用户使用中文交互，工具描述和技能提示使用中文更自然。

**不选 B 的原因**：保留英文——用户已确认系统提示词由 SYSTEM.md 接管，工具描述通过 API 发送，中文更一致。

**何时重新考虑**：如果未来需要多语言支持，可考虑 i18n 框架。

## D9: 移除 write/edit 工具的 CWD 路径限制（2026-07-15）

**决策**：移除 `write` 和 `edit` 工具的 CWD 路径限制，允许写入任意绝对路径。

**理由**：CWD 限制在沙盒进程环境下反而产生误导——os error 5 让用户误以为是权限问题；write 不应比 pwsh/bash 等 shell 工具有更多路径限制；用户需要写入项目目录之外的路径。

**不选 B 的原因**：保留限制并改进错误提示（功能不足）；改为白名单模式（不必要的配置复杂度）。

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

## D10: Edit 工具写入改为直接写（Windows 句柄冲突）（2026-07-15）

**决策**：`EditTool` 的写入路径从 tempfile 原子重命名改为 `std::fs::write()` 直接写入。

**理由**：Windows 上 async 读操作可能未及时释放文件句柄，导致后续 `MoveFileEx` 报 `ERROR_ACCESS_DENIED`；`WriteTool` 不先读文件不受影响；文件已通过前置权限检查确认可写。

**不选 B 的原因**：继续排查 asupersync 句柄释放时序（框架层修复周期长）；保留原子重命名并增加 retry（编辑场景原子性非必须）。

**何时重新考虑**：如果 `asupersync` 底层修复 Windows 句柄释放问题，或 edit 改为同步读文件，可恢复原子重命名。

## D11: 移除 read 工具的 CWD/agent-dir 路径限制（2026-07-15）

**决策**：移除 `read` 工具的路径限制，允许读取任意绝对路径。

**理由**：与 D9 一致——read 不应比 shell 工具有更多路径限制；验证报错阻止了读取项目目录之外的必要文件（配置、构建产物）；工作流需要跨项目引用。

**不选 B 的原因**：保留限制并改进错误提示（功能不足）；白名单模式（与 D9 决策不一致）。

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

## D12: run_extension_command 总是发送 agent_end（2026-07-15）

**决策**：`run_extension_command` 执行完毕后无论成功或失败，始终发送 `agent_end` 事件。

**理由**：RPC 客户端（pidian）依赖 `agent_end` 触发 `finalizeStreaming()`；先前仅错误时发送，成功时客户端 `isStreaming=true` 永不终结。

**不选 B 的原因**：成功分支补发但保持时序不变（写法冗余）；pidian 侧加超时 fallback（治标不治本，掩盖 pi 事件缺失）。

**何时重新考虑**：如果 RPC 协议改用更明确的流式生命周期消息替代 `agent_end`，可整体重构。

## D13: RPC 模式扩展交互 UI 传递 hasUI 上下文（2026-07-16）

**决策**：`execute_command()` 构建 JS 上下文时，使用 snapshot 的 `has_ui` 字段，替代硬编码的空对象。

**理由**：RPC 模式下扩展管理器已配置 UI 通道，但传空对象导致 `hasUI = false`，`ctx.ui.select/confirm/input` 静默返回 `undefined`，扩展 handler 误报「已取消」。

**不选 B 的原因**：硬编码 `true`（headless 场景无 UI 通道，应返回 undefined）；手动传参（调用方繁多，遗漏风险大）。

**何时重新考虑**：如果未来 UI 通道模型改为每个连接独立配置，可改为从调用上下文动态推导。

## D14: 移除 find 工具的 CWD 路径限制（2026-07-17）

**决策**：移除 `find` 工具的 CWD 路径限制，允许在任意绝对路径下搜索文件。

**理由**：与 D9/D11 一致；验证报错阻止了在 CWD 之外搜索必要文件（技能文件、Home 下配置）；Agent 工作流要求在 `~/` 下搜索。

**不选 B 的原因**：保留限制并改进错误提示（功能不足）；白名单模式（与 D9/D11 决策不一致）。

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

## D15: 移除 grep/ls 工具的 CWD 路径限制（2026-07-17）

**决策**：移除 `grep` 和 `ls` 工具的 CWD 路径限制，允许在任意绝对路径下使用。

**理由**：与 D14/D9/D11 保持一致——所有文件系统工具已全部放开 CWD 限制；不应比 shell 工具有更多路径限制。

**不选 B 的原因**：保留限制——与已放开的其他工具不一致，增加用户心智负担。

**何时重新考虑**：如果未来引入细粒度工具安全策略（per-tool allowlist），可重新加入路径限定。

## D8: Release 构建 — 栈溢出问题（2026-07-15）

**问题**：Debug 构建的 `pi.exe` 在 Windows 上启动即崩溃，报 `thread 'main' has overflowed its stack`。Release 构建正常。

**原因**：Debug 模式下 Rust 不优化尾递归、增加栈安全检测，启动初始化链路超出默认栈大小（Windows 1MB）。Release 模式（LTO + 优化）消除了中间栈帧。

**对策**：日常使用始终 `cargo build --release`；Debug 构建调试需增大栈大小（`.cargo/config.toml` 加 `link-args=/STACK:4194304`）；release profile 已从激进体积优化改为速度优先。

## D16: RPC 进程侧主动会话持久化（2026-07-19）

**决策**：`pi --mode rpc` 捕获 `TurnEnd` 事件，背景线程 `RpcSessionPersister` 将已完成的消息实时追加写入 JSONL。

**理由**：Obsidian 崩溃时正在进行的会话数据全部丢失；持久化原完全依赖 pidian 触发 `saveConversation()`，只在 turn 结束后一次性写入。

**不选 B 的原因**：agent 核心循环中加中间落盘（侵入性强，影响所有模式）；定时器 flush（turn 运行中的消息在内存中，不在 session 对象里）；提高 pidian 保存频率（仍依赖客户端，Obsidian 崩溃时没用）。

**何时重新考虑**：如果以后完全迁移到 SQLite 后端，可移除或替换。

## D17: 项目级 `.pi/SYSTEM.md` 覆盖（2026-07-20）

**决策**：新增项目级 `.pi/SYSTEM.md` 检查，优先级链：`--system-prompt > .pi/SYSTEM.md > ~/.pi/agent/SYSTEM.md > default`。

**理由**：允许项目所有者锁定系统提示词，不被用户级 SYSTEM.md 意外覆盖；团队项目中不同成员行为一致。

**不选 B 的原因**：复用全局文件并覆盖（影响所有项目，粒度太粗）；要求改用 `--system-prompt`（无法持久化）。

**何时重新考虑**：如果未来提供更细粒度的提示词分层机制（per-task prompt overlay），可合并。

## D18: 项目技能加载模式 `skill_mode`（2026-07-20）

**决策**：`.pi/settings.json` 新增 `skill_mode: "project_only"` 配置，项目跳过全局技能，只加载项目技能。

**理由**：携带大量专属技能的项目会被全局技能干扰；减少 token 消耗；仅影响技能，不影响 extensions/prompts/themes；默认行为不变。

**不选 B 的原因**：`--no-skills` 全局关闭（粒度太粗）；技能清单逐个 disable（维护成本高）；package 机制白名单（对非 package 的自动发现技能无效）。

**何时重新考虑**：如果未来引入更细粒度的技能可见性策略（per-skill scope、标签路由等），可合并。

## D19: `global_skills` 白名单 — 选择加载特定全局技能（2026-07-20）

**决策**：`.pi/settings.json` 新增 `global_skills` 数组，只加载列表中的全局技能，其余被过滤。

**理由**：有些项目只需要少数几个全局技能；与 `project_only` 叠加（先排除全部，再按白名单恢复）；空数组 = 不过滤，向前兼容；项目技能不受影响。

**不选 B 的原因**：`skills` 数组逐个 disable（需先知道全局技能列表）；将技能移到项目目录（破坏全局统一管理）；环境变量（不适合传列表、不易持久化）。

**何时重新考虑**：如果未来引入基于标签/分类的技能可见性系统，可合并或废弃。

## D20: RPC 队列管理命令 + `queue_update` 事件 + 消息 ID（2026-07-21）

**决策**：RPC 协议新增 3 个队列管理命令（`remove_from_queue`/`clear_queue`/`get_queue`），新增 `queue_update` 事件实时推送队列状态，每条入队消息携带 `messageId`。

**理由**：pidian 需在 UI 展示队列并支持精确取消；原先客户端无法感知队列当前内容；`messageId` 是精确操作的手柄。

**不选 B 的原因**：用 `Message` 本身作引用（序列化开销大、语义不清）；仅用索引（并发下索引漂移误删）；客户端维护镜像（状态可能不一致）；`replace_queue` 整体替换（过度设计）。

**何时重新考虑**：如果 RPC 协议升级为双向流式 IDL（如 gRPC），可重新设计队列同步机制。

## D21: OpenAI 兼容推理模型的 reasoning_effort 支持（2026-07-24）

**决策**：所有非 DeepSeek 的 OpenAI 兼容推理模型（`reasoning: true`）在请求中发送 `reasoning_effort` 字段（`low`/`medium`/`high`），而非静默丢弃。

**理由**：原先只在 DeepSeek 路径发送思考控制字段，其他推理模型走 `None` 分支不发任何字段；Lucis `gpt-5.6-sol` 实测支持 `reasoning_effort` 但从未收到。

**不选 B 的原因**：非 DeepSeek 模型送入 DeepSeek 路径（错误发送 `thinking` 对象，可能被拒绝）；只改 models.json（`thinkingLevelMap` 在 `ModelConfig` 中不存在，读不到）。

**何时重新考虑**：如果 OpenAI 官方原生支持其他思考参数，可统一所有推理模型到一个分支。

## D22: 编辑后轻量验证系统（Verify）（2026-07-27 → 2026-07-28 default 改为 true）

**决策**：`edit`/`hashline_edit`/`write` 增加可选 `verify` 参数（默认 `true`），编辑成功后自动运行轻量语法/格式检查。结果附在 `details.verify`，不阻断编辑流程。

**理由**：Agent 编辑文件后缺少自动格式/语法验证环节，增加迭代来回；进程内检查器（JSON/TOML）零额外开销；文件类型直接映射策略确定性强。

**不选 B 的原因**：默认 `false`（2026-07-28 重评估：单文件验证 <50ms，延迟可忽略，改默认 `true`）；暴露为独立 LLM-visible tool（已有 pwsh 可手动检查）；自动修正（违反「只报告不修」铁律）。

**何时重新考虑**：需要支持更多文件类型时，扩展映射表即可。

## D23: set_model/set_thinking_level 增加 persist 参数（2026-07-29）

**决策**：`set_model` 和 `set_thinking_level` 请求体增加可选 `persist: bool`（默认 `true`）。`persist=false` 时仅内存切换 provider/model/thinking，不写会话文件。

**理由**：pidian 需要切换模型/思考等级但不污染默认配置；原方案走重启进程（`--model` 启动参数），体验差。

**不选 B 的原因**：不加控制全部持久化（客户端需额外清理）；全局开关持久化（粒度太粗）；仅限 RPC 路径（Extension/ACP 也需要此能力）。

**何时重新考虑**：如果未来引入会话级配置系统，可合并到统一配置管理。

## D24: 扩展工具 abort 两阶段机制（2026-08-02）

**决策**：扩展工具执行时，检测到 abort 后先通知 JS 侧 `AbortController`（扩展可优雅退出），下一轮仍 pending 则 `request_interrupt()` 硬中断兜底；真实 signal 作为第 4 参数传给扩展 `execute`。

**理由**：此前 signal 恒为 `undefined`，abort 信号在包装器处即被丢弃，中断轮询从未生效；扩展生态依赖 `signal.addEventListener('abort')` 实现可取消长任务，硬中断会跳过清理逻辑；先通知后中断行为不退化。

**不选 B 的原因**：仅硬中断（扩展无法感知取消，清理逻辑被跳过）；仅 JS signal 通知不硬中断（恶意/死循环扩展永远不响应，abort 失效）。

**何时重新考虑**：如果 QuickJS 运行时支持 Promise 级取消，可简化为一阶段优雅取消。

## D25: verify 的 .ts 检查直调全局 prettier，回退 npx（2026-08-03）

**决策**：verify 的 TypeScript 检查从 `npx --no-install prettier --check` 改为直调全局 `prettier --check`，无全局安装时回退 npx 包装。超时/abort 改为杀整棵进程树。

**理由**：npx 包装层触网（registry 探测），一次网络挂起即触发 verify 超时；直调全局 prettier 是纯本地，实测 ~270ms vs npx ~1.2s；fallback 保证无全局环境零回归。

**不选 B 的原因**：放宽超时（只延后不改根因）；项目内安装 prettier（verify 是宿主侧能力，不应要求项目侧安装）；缓存 npx 解析结果（仍可能触网，只缓解不根治）。

**何时重新考虑**：如果目标环境普遍无全局 prettier 且 npx 不再触网，可重新评估以 npx 为主。

> 注：后续 D26 修正根因分析——#32 超时实为 cmd shim 的 stdin 继承问题，网络只是放大因素；D25 的直调决策本身仍成立。

## D26: verify 子进程 stdin 置 null，防宿主管道挂起（2026-08-03）

**决策**：`run_external_process` spawn 检查器子进程时显式 `stdin(Stdio::null())`，与全库其他 spawn 一致。

**理由**：verify 只读不消费输入；继承宿主 stdin（Obsidian = 活跃 JSONL 管道，写端常开）时，`cmd.exe` 包装的 shim（`prettier.cmd`/`npx.cmd`）等待该管道不退出 → 超时；rustfmt 是 `.exe` 直连无 cmd 层，不受影响。probe（stdin=null）从不超时，同程序仅差 stdin 处理，定位根因。

**不选 B 的原因**：放宽超时（不改根因）；手动包装 stdin 管道并主动 EOF（verify 永不读 stdin，无收益且复杂化）。

**何时重新考虑**：若未来 verify 需要读取 stdin 输入，需重新设计管道生命周期。

## D27: 扩展 sibling 发现排除 auto-discovery 根（2026-08-04）

**决策**：`discover_sibling_index_entries` 在 parent 与 cluster_root 任一层目录名为 `extensions` 时直接返回空，与姊妹函数既有 guard 对齐。

**理由**：`~/.pi/agent/extensions/`（或 `.pi/extensions/`）下每个目录是独立扩展，不是同一 bundle 的多入口；缺 guard 时 2 个以上扩展共存即触发误判，非 primary 入口的相对 import 报 `Unsupported module specifier`（真实用户环境 8 扩展必现）。

**不选 B 的原因**：调用方过滤（发现函数应自己保证语义正确）；放宽多入口加载容错（掩盖误判根因，根注册扩散/命令冲突）。

**何时重新考虑**：若未来引入「extensions 根下多目录 bundle」合法场景，应改为按 manifest 判 bundle，而非目录结构启发式。

## D28: manifest-aware 扩展加载——有 extension.json 时禁用 sibling 发现（2026-08-04）

**决策**：`discover_related_extension_entries` 在 primary 所在目录存在 `extension.json`（或 package.json 的 `pi.ext.manifest.v1` schema）时，只返回 manifest 声明的 entrypoint，跳过全部 sibling 启发式发现。

**理由**：manifest 是唯一权威入口；启发式发现会把目录内模块文件和子目录门面误判为额外入口，逐个加载耗尽 hostcall budget；误判的子目录被注册为 root，导致合法相对 import 被误判为逃逸。

**不选 B 的原因**：收紧 flat-entry 启发式（启发式永远有盲区）；放宽 `detect_monorepo_escape` 判定（破坏 monorepo 不跨包引用的安全语义）。

**何时重新考虑**：若未来支持「manifest 多入口」，本 guard 应改为信任 manifest 的完整入口列表。

## D29: 会话保存 Windows 文件竞争重试 + RPC 持久化补全（2026-08-07）

**决策**：会话 JSONL 保存遇 Windows 文件竞争错误（os error 5 / 32）退避重试；append 后 fsync 的 PermissionDenied 降级为警告；修复 RPC persister 链根（header id 作首条 entry parentId）并补写 user 消息；新增 `append_custom_entry` RPC 端点。

**理由**：`rename` 遇无 `FILE_SHARE_DELETE` 持有者报「拒绝访问」，与用户高频报错一字不差（Defender/编辑器/并行实例为毫秒级瞬态）；RPC persister 是防崩溃设计，数据到页缓存已够；CustomEntry 消除 pidian 双写。

**不选 B 的原因**：移除 fsync 完全放弃崩溃安全（checkpoint/关闭仍 fsync）；pidian 继续直接写 JSONL（双写导致热力图虚高、链交错断裂）；加 `session/save` RPC（多一层往返无收益）。

**何时重新考虑**：若 pi 迁移 SQLite 会话后端，JSONL 重试与 persister 均随格式废弃。

## D30: Specification-First 移植方法论（2026-08-07）

**决策**：从 TypeScript 原版移植时按「提取行为 → 文档化 spec → 按 spec 实现 → 一致性测试」流程，而非逐行翻译。

**理由**：TS 习语不能直接映射到 Rust（所有权/trait/enum）；按 spec 实现产出更符合 Rust 习语的代码，fixture 一致性测试可对照原版验证。

**不选 B 的原因**：逐行翻译产出「穿着 Rust 外衣的 JS」，对抗语言特性。

**何时重新考虑**：无。新功能开发沿用同一流程。

## D31: 单二进制分发 + 内置 QuickJS（2026-08-07）

**决策**：分发模型为单个 Rust 静态二进制（`pi`），扩展用嵌入 QuickJS 运行（无 Node/Bun 依赖），而非 npm 包 + Node 运行时。

**理由**：消除 Node 运行时启动开销（<100ms vs 500ms+）与运行时依赖管理；扩展经 `node:` 垫片保持生态兼容。

**不选 B 的原因**：Node 嵌入式运行时引入 100MB+ 体积与 JIT 启动延迟；放弃扩展兼容则破坏既有生态。

**何时重新考虑**：若 QuickJS 沙箱无法覆盖未来扩展 API 需求，可评估渐进式外部运行时桥接。

## D32: asupersync 结构化并发运行时替换 Node event loop（2026-08-07）

**决策**：异步基座用 asupersync（结构化并发 + 内置 HTTP/TLS/SQLite），`AgentCx` 在 agent/tools/session/rpc 边界显式传递能力作用域。

**理由**：取消语义显式化（父任务取消 → 子任务干净取消，无孤儿 future）；I/O 能力经 `Cx` 作用域化，测试确定性；HTTP/TLS 内置避免 OpenSSL 依赖地狱。

**不选 B 的原因**：tokio 生态虽大但取消靠约定不靠结构；Node 事件循环 + Promise 约定无法提供能力作用域。

**何时重新考虑**：若 asupersync 生态停滞或出现结构性缺陷，可评估迁移 tokio（代价高，需重写取消边界）。

## D33: 自研 SSE 解析器（2026-08-07）

**决策**：流式响应用自研 SSE 状态机（`src/sse.rs`），不用现成 crate。

**理由**：需处理多 provider 的分块差异（CR/LF、多行 data:、UTF-8 部分尾部、TCP 分块跨界）；状态机可按字节增量处理、零拷贝、错误不崩流。

**不选 B 的原因**：现成 SSE crate 面向通用场景，无法按需内联事件类型、控制缓冲策略，且多 provider 适配成本更高。

**何时重新考虑**：若协议复杂度超出维护成本，可评估基于成熟 crate 的封装。

## D34: 能力门控扩展安全模型（2026-08-07）

**决策**：扩展无 ambient 系统访问权；所有 hostcall（tool/exec/http/session/ui/env/log）经能力策略门控 + 审计日志，exec 再加命令级调解。

**理由**：原版扩展模型文档化为全系统访问，安全风险不可审计；门控后策略可解释、确定性、fail-closed，支持 trust 生命周期与 kill switch。

**不选 B 的原因**：维持全系统访问 + 事后审计（无法在 spawn 前拦截危险命令）；OS 级沙箱（部署复杂、牺牲性能）。

**何时重新考虑**：若扩展生态出现合法需要 ambient 能力的工作负载，可评估按扩展细粒度授权（仍保留审计）。

## D35: 截断类 SSE parse error 分类为瞬时错误自动重试（2026-08-08）

**决策**：`openai.rs` 解析 SSE chunk 失败时按 `serde_json` 错误分类分流：`Category::Eof`（数据不完整）→ 瞬时错误可重试；其他 parse error（语法/类型错误）保持不可重试。

**理由**：第三方网关（opencode.ai）长思考后实测发送截断 chunk 并关闭连接；`EofWhileParsing*` 只能由「数据不完整」产生，瞬时性成立；此前映射到不可重试，整个响应直接断流。

**不选 B 的原因**：全部 parse error 设为可重试（语法/类型错误是确定性失败，重试重复计费并掩盖客户端 bug）；改正则匹配错误消息文本（不如在错误源头用 typed 分类精确）。

**何时重新考虑**：若上游修复截断问题、或 Pi 侧改为流式解析，此分类可撤销或降级。

## D36: Gemini 思考链支持（2026-08-11）

**决策**：gemini/vertex provider 支持 Gemini 3.x 思考链——发送侧 `thinkingConfig.thinkingLevel`（Pi 级别映射：`off→minimal`、`xhigh→high`、其余同名，per-model `thinkingLevelMap` 优先），接收侧 `thought` part 映射为思考事件，`maxOutputTokens` 固定用满官方上限 65536。

**理由**：Gemini 3 系列不支持完全关闭思考（官方文档明确）；`high` 是官方最高档，`xhigh` 降级为 `high`；思考 token 与输出共享额度，固定 65536 避免深度思考 + 长回答被截断。

**不选 B 的原因**：`off` 不传 thinkingConfig（模型用默认 medium，语义上 off 名存实亡且更贵）；继续忽略 thought part（思考强度不可控）。

**实测结论**：`thinkingLevel: high` 确认生效（thoughtsTokenCount 847→1149）。但 Google 3.x 不返回思考文本 part（只有 thoughtSignature + 计数），接收侧思考文本分支目前不触发——保留为防御性实现。

**何时重新考虑**：若 Google 提供真正的 thinking-off 档位或开放思考文本返回，可调整映射表。

## D37: 中断/错误消息 strip dangling tool calls（2026-08-12）

**决策**：`build_abort_message`/`build_error_message` 在克隆 partial 消息后删除未完成的 `ToolCall` content blocks，再持久化。

**理由**：流式输出 tool_call 期间中断时，partial 里已有 `ToolCall` 块但没有对应 tool 响应；持久化后下次请求被 provider 拒绝（`tool_calls must be followed by tool messages` 400），会话从此卡死。迭代上限路径早有同样的 strip，abort/error 路径是补漏。

**不选 B 的原因**：保留 tool_call 重放（provider 校验严格，请求发不出去）；用 `revert_incomplete_response` 整体回退（只在 retry 路径使用，且会丢中断前已输出的文本）。

**何时重新考虑**：若 provider 允许未完成的 tool_call，可保留文本、仅标记 tool_call 无效。

## D38: Windows 磁盘余量探测用 sysinfo 卷枚举（2026-08-12）

**决策**：`disk_available_kb` 的 Windows 分支用 sysinfo 枚举磁盘卷、按挂载点前缀匹配路径所在卷；Unix 分支保持 `df -Pk` 不变。

**理由**：Windows 没有 `df`，swarm 预检此前恒为 `None`，headroom 判定失败、测试全挂；sysinfo 是项目已有依赖，零新增成本。

**不选 B 的原因**：`std::fs::canonicalize` 在 Windows 返回 `\\?\C:\...` verbatim 形式，`starts_with` 不匹配，不能用于挂载点匹配；调 PowerShell `Get-PSDrive`（进程开销大、解析脆弱、破坏「doctor 不依赖 shell」架构）。

**何时重新考虑**：若引入 windows crate 的 `GetDiskFreeSpaceExW` 直调，可替换 sysinfo。

## D39: workspace bundle 发现要求父目录声明 pi.extensions（2026-08-13）

**决策**：`discover_workspace_bundle_entries` 只把「父目录自身 `package.json` 声明了 `pi.extensions`」的目录当作 bundle 根；否则直接返回空，不做任何兄弟目录扫描。

**理由**：e2e 实测聚合仓库（40 个兄弟目录各有独立 package.json，恰好卡在阈值内）被无脑当 bundle 根扫描，无关扩展全被卷为入口互相污染；这是 D27/D28 的补全——前两轮只挡了「目录名=extensions」的根，本决策把「何为 bundle 根」收敛为「父 package.json 显式声明」。

**不选 B 的原因**：调整 `MAX_BUNDLE_CLUSTER_DIRS` 阈值（阈值永远有边界案例）；调用方过滤（D27 已否决）；收紧 flat-entry 启发式（D28 已否决）。

**何时重新考虑**：manifest 多入口声明天然兼容本 guard；「无声明但确为 bundle」的合法场景应要求显式声明而非再放宽启发式。

## D40: drop-in 认证诚实降级（2026-08-13）

**决策**：`overall_verdict` 从 `CERTIFIED` 降为 `NOT_CERTIFIED`，差分测试在 runner 不可用时 fail-closed（skip 而非失败）。

**理由**：legacy pi-mono 快照故意残缺（git 全历史核心 blob 数 = 0），差分 runner 永远无法执行——此前的 `CERTIFIED` 是无法复现的声明；原作者已转向契约测试，不再维护 legacy 差分。

**不选 B 的原因**：保留 `CERTIFIED` 但标注「不可复现」（证据文件要求可复现）；彻底删除 dropin 测试/认证体系（破坏与上游的契约对齐，恢复成本低但未来重新 provision 快照可恢复差分）。

**何时重新考虑**：若重新拉取完整快照并跑通差分，可将 verdict 恢复；或彻底放弃 drop-in 声明后移除认证体系。

## D41: print 模式会话落盘 opt-in（2026-08-13）

**决策**：print 模式保持默认不落盘（一次性输出即弃）；显式 `--session-dir` 或 `--session` 时 opt-in 持久化（成功与失败都落盘）；显式 `--no-session` 优先级最高。

**理由**：无人值守场景（`pi -p @prompt`）失败时需要完整会话上下文供诊断；默认不落盘保护 print 高频临时调用，不污染默认 `sessions/` 目录。

**不选 B 的原因**：print 总是落盘（目录膨胀且行为破坏）；复用 `--no-session` 现有语义（无法表达「我要落盘」，参数被静默忽略是 bug）。

**何时重新考虑**：若 print 模式普遍需要持久化，可考虑环境变量/配置默认开启。

## D42: Exec hostcall 并发执行 + AMAC Interleave 决策落地（2026-08-14）

**决策**：`Exec` 组加入 AMAC interleave 白名单，且让 Interleave 决策真正落地——interleave 组用有界并发（自适应宽度）执行并保序收集，不再顺序 await；逃生开关 `PI_HOSTCALL_AMAC_EXEC_INTERLEAVE=0` 强制 exec 串行。

**理由**：subagent 扩展 `Promise.all` + `spawn` 并行拉起 4 个子进程被强制串行（4 任务 1m32s vs bash 直启 7s）；原实现 Interleave 决策只写日志、从未消费；Exec 有副作用，保留逃生开关应对潜在顺序依赖。

**不选 B 的原因**：仅把 Exec 加白名单（决策从未被消费，并发不生效，已实证）；扩展侧 longLived 绕过（LSP 专用通道，短生命周期命令语义不匹配）；Exec 恒并发不加开关（无自适应、无回退手段）。

**何时重新考虑**：若发现调用方依赖 exec 顺序 → 设逃生开关；若冷启动首轮 batch 串行影响明显 → 调低遥测门槛。

## D43: Exec 组跳过 AMAC 遥测门槛（Rule 3/4）+ 门槛可配置（2026-08-14）

**决策**：`decide_toggle` 新增 Rule 2c——Exec 组跳过遥测数量门槛与 stall 比率门槛，宽度确定性取 `min(batch_size, max_width)`；`AmacBatchExecutorConfig` 新增 `min_telemetry` 字段（env `PI_HOSTCALL_AMAC_MIN_TELEMETRY`，默认 64，`0` 关闭保护）。

**理由**：TUI 会话中 telemetry 只来自 QuickJS hostcall（原生工具不贡献），每轮仅 4–5 次观察，数量门槛永久锁死 Exec → 体验等价于永远串行；快速调用稀释 stall 比率，比率门槛同样锁死；Exec 是秒级进程阻塞，并发收益确定性，stall 检测对其无信息量。

**不选 B 的原因**：按组独立统计小阈值（额外状态机，收益不明确）；首次 batch 并发 + 观察起步（与跳过门槛语义重叠）；扩展侧 longLived 绕过（LSP 专用，语义不匹配）。

**何时重新考虑**：若并发后调用方出现顺序依赖 → 逃生开关；若 Http/Tool 组在 TUI 场景也需要冷启动并发 → `PI_HOSTCALL_AMAC_MIN_TELEMETRY=0`。

## D44: tools.toml 工具可见信息覆盖（2026-08-15）

**决策**：新增 `tools.toml`（用户级 `~/.pi/agent/tools.toml` + 项目级 `.pi/tools.toml`，项目 key 优先）逐工具覆盖「LLM 可见信息」：`description` 与 `parameters`（内嵌 JSON 文本整体替换 JSON Schema）。description 同时作用于提示词文字层（`Available tools:` 列表）与 API tool schema 层，两层保持一致。未列出的工具保持内置默认，删除条目/文件即恢复默认（不冻结核心更新）。`settings.json` 的 `toolDescriptions` 保留兼容，tools.toml 优先。

**理由**：用户需要外放所有 LLM 可见描述（此前只有 description 且仅 schema 层可改、提示词文字层与参数层完全硬编码）；TOML 逐工具覆盖保留「只显示启用工具」的过滤逻辑与默认跟随；项目级提供团队一致性，与 SYSTEM.md 双层语义对称。

**不选 B 的原因**：纯文本整段接管 `tools.md`（破坏 enabled 过滤、冻结默认）；parameters 部分字段嵌套覆盖（schema 合并语义复杂，收益低）；只做用户级（丢失项目级团队锁定能力）。

**何时重新考虑**：如果出现「只改某个参数 description」的高频诉求，可增加部分字段合并覆盖。

## D45: 系统提示词注入当前临时目录（2026-08-15）

**决策**：`build_system_prompt` 在 cwd 注入旁新增 `Current temporary directory: {path}`，经 `std::env::temp_dir()` 跨平台解析（Windows `%TEMP%` / Unix `$TMPDIR`/`/tmp`），test_mode 下用 `<TEMP>` 占位符，跟随 `include_cwd` 开关。

**理由**：Agent 需要临时目录时无需自行查询；temp 路径会话内稳定但跨机器不同（Windows 含用户名、macOS `$TMPDIR` 随机），与 cwd 同属「运行时环境事实」，放核心保证确定性（扩展注入发生在核心构建之后，感知 test_mode 占位符会把约定泄漏到扩展层）；跟随 `--hide-cwd-in-prompt` 隐私语义（Windows temp 路径含 OS 用户名）。

**不选 B 的原因**：用扩展注入（扩展需感知 `PI_TEST_MODE` + 事件 ctx 无 temp 通道，需核心加字段，等于没去核心化）。

**何时重新考虑**：若核心推出「事实注入 vs 业务注入」的提示词分层机制（事实类保留核心、业务类外放扩展），本条随之调整。

## D46: 依赖升级冻结——快赢合并时回退上游依赖（2026-08-16）

**决策**：合并上游时，Cargo.toml/Cargo.lock 的依赖升级（digest 0.11、swc 26、asupersync 0.4、rquickjs 0.12、base64 0.23 等）**整体回退到 custom 基线**，不随合并引入；build.rs 与引用新 API 的自动合入文件同样回退。升级留待与 extensions 架构迁移绑定做独立项目。

**理由**：上游为升级依赖同步改了几百个 commit 的适配代码；custom 代码基于旧 API，直接升级 = 423 个编译错误（实测），等于重写适配层。快赢合并的目标是拿功能（ast 工具等），不是升级依赖；依赖升级是独立工程，应单独规划、单独验证。

**不选 B 的原因**：逐个文件适配新 API（工作量 = 上游适配全量，且与 extensions 迁移重复）；部分升级（版本分裂，编译选一行为漂移）。

**何时重新考虑**：做 extensions 架构迁移时（新架构依赖新依赖，强相关），连同依赖升级一起做。

## D47: 上游合并策略——快赢合并 + 冻结面（2026-08-16）

**决策**：改造式 fork 追上游**只用 merge，不用 cherry-pick**（实测：custom 拆分过文件结构后，上游 commit 的改动锚点不存在，连单文件独立修复也 cherry-pick 失败）。merge 时按「快赢合并 + 冻结面」策略：值得立刻要的（安全修复/bug 修复/新工具）合入；结构大改（extensions 重构）、依赖升级、发布管道、无关新特性（subagent roles 等）**冻结**——上游新文件删除、单文件保留 custom 版，留待独立迁移项目。流程与冻结清单见 `.pi/upstream-reports/fork-merge-sop.md`。

**理由**：上游 3 天 85 commits 的活跃度下，一次性吞全部 = 每次 merge 处理几百个冲突文件（含大量无价值的发布管道/内部工具数据）。挑着合把冲突面从 248 文件降到 83（其中源码真冲突仅 11 个，68% 集中在冻结的 extensions 两文件）。冻结面是「延迟决策」而非「拒绝」——每次上游动冻结域都重新评估一次。

**不选 B 的原因**：一次性全量 merge（引入不需要的发布管道噪音 + 依赖漂移灾难）；放弃追上游（失去安全修复与上游生态演进）。

**何时重新考虑**：extensions 架构迁移完成后，冻结面消失，可回归普通周频 merge；上游发大版本（0.x.0）时评估一次是否继续追。

---

## D48: Exec AMAC 阈值 — 通用 min_batch=4 前置为 Rule 0 直通

**决策**：decide_toggle 将 Exec 分支提至最前（Rule 0），无视通用 min_batch_size=4 阈值（零新增配置/字段/env）；2 即 Interleave(width=2)，1 仍 Sequential(computed_width_too_low)，Http/Tool 等仍受 min_batch=4 守门。

**理由**：Exec 是秒级阻塞子进程，并发收益确定（2×6s 并行≈6s 串行≈12s），不应套用为 Http/Tool LLC-miss 统计模型设的 4 阈值；原 Rule1 在 Rule2c 之前导致 2~3 被 batch_too_small 强制串行（实测 2× ratio2.8/3× ratio7.3 vs 4× ratio1.4）。

**不选 B 的原因**：全局改 min_batch=2——会让 Http 2× 也无条件 buffered(2)，在低 stall 下负优化；新增 PI_HOSTCALL_AMAC_EXEC_MIN_BATCH=2——把配置负担丢给用户，且 Issue 对照已证 2 即该并行，无需阈值可调。

**何时重新考虑**：若 Exec 出现顺序依赖需调优 2 阈值，再评估可配置化；否则保持零配置直通。

---

## D49: Stream 断流自动重试 — Stream ended without Done 归入 is_retryable_error 正则

**决策**：将 `Stream ended without Done event` 归入 `is_retryable_error` 正则白名单（`src/error.rs` + `crates/pi-provider-core/src/error.rs` 末尾追加 `|stream ended without done`），命中后走现有 `run_prompt_with_retry` 指数退避重试（`revert_incomplete_response` + `run_continue_with_abort`，最多 `retry.maxRetries=3` 次，已执行 tool_call 不重放）。

**理由**：网络抖动/代理掐流导致 SSE 未发 Done 即 FIN，此前为 `Error::Api` 既不走 `is_transient` 也不在正则中，直接 `agent_end{error}`。正则兜底在 `Ok(Error)` 与 `Err` 双路径均生效，且受 `is_context_overflow` 前置排除约束。

**不选 B 的原因**：不新增 `Error::Transient` 变体/flag（破坏 dropin 错误契约）；不在 `is_transient()` 加 `Api` 特判（仅覆盖 `Err` 路径，`Ok(Error)` 的 `is_retryable_prompt_result` 仍漏）；不在 provider 内做流内重试（流已断无 Done，重试应在 turn 级 `run_continue` 语义）。

**何时重新考虑**：若上游网关永久不发 Done（兼容实现缺失）导致 3 次重试仍失败，或 compaction 链路需独立重试策略，再评估是否对 compaction 加重试或对该文案加独立退避。

---

## D50: RPC 斜杠命令超时 — EXTENSION_COMMAND_BUDGET_MS (30s) 替代 EXTENSION_EVENT_TIMEOUT_MS (5s)

**决策**：run_extension_command（pi --mode rpc 的 prompt 分支）在调用 JsExtensionRuntimeHandle::execute_command 时传入 EXTENSION_COMMAND_BUDGET_MS（30s），而非 EXTENSION_EVENT_TIMEOUT_MS（5s）。

**理由**：交互式斜杠命令的等待时间包含 await ctx.ui.select/confirm/input/custom 的用户思考时间；5s 预算使用户 5s 内未完成选择即抛 JS extension runtime command timed out after 5000ms 并以 agent_end{error} 回 pidian。30s 与 extensions.rs:16057 设计一致，覆盖正常交互时长。

**不选 B 的原因**：继续用 5s 事件预算会导致所有带 UI 交互的 RPC 斜杠命令在思考>5s 时 100% 复现，custom 类因 pidian 白名单丢弃更是必超时；30s 仅放宽 RPC 命令路径，不影响 TUI/事件路径，且 rpc_parse_extension_ui_response 对未知 custom 回包格式已做 error 兜底，不会再挂起。

**何时重新考虑**：若 pidian 补齐 custom 等 expects_response 方法的全量转发并提供兜底 Modal，或交互 UI 改为按请求生命周期驱动取消（effective_timeout_ms=None 无限等待），可评估进一步放宽或改为 UI 驱动的取消模型。

