# C1 上游同步重叠与冲突风险矩阵

- 生成时间：2026-09-05
- 目标 checkpoint：C1 v0.3.0，提交 e23c4622f8bc4038a5e061ee3640a0e9206ec5cc
- 共同基线：226a876425a856f657b2a5d7c7ac6f0ca1ad25f1
- 当前分支：custom
- 试合并工作树：C:\Users\m\AppData\Local\Temp\pi-c1-integration
- 上游末端参考：e403485b3116e6c97e9af7026ec9445f30312c7d（仅用于定位 C1 在区间中的位置，不作为本次合并内容）
- 输入文档：
  - docs/upstream/merge-checkpoints.md（C1 区间统计）
  - docs/upstream/semantic-map.md（2026-08-16，52 交集文件决策底稿）
  - docs/upstream/probe-report-2026-08-29.md（121 到 0 试合与 297 编译缺口分类）
  - docs/upstream/fork-merge-sop.md（改造式 fork 合并 SOP）
  - docs/upstream/custom-changes-inventory.md（custom 二开面）
  - docs/upstream/upstream-changes-inventory.md（上游 0.1.22 到 0.2.0 主题聚类）
  - docs/upstream/plan-hub-minimal.md（含 2026-08-29 落地证据 A1）
- 方法约束：
  - 只分析，不修改源码，不解决 merge 冲突
  - 只取机器统计和已有风险索引，不读取所有冲突块
  - 必要时只看少量代表性冲突块，本次未读取任何冲突块内容，仅依据文件清单与已有报告归因
  - 不给出未经证据支持的源码级结论，凡推断均标注为推断
  - 本文档不使用 Markdown 表格，全部以列表记录，避免宽表误读

## 0. 证据边界与可信度声明

- 本次 Worker 环境无 shell 执行入口，未能执行试合并工作树的 git status 与 git diff --name-only --diff-filter=U。
  - 因此冲突路径清单的精确 UU 数在本矩阵中留空，待 Main Agent 在工作树中补取。
  - 补取命令（Main Agent 执行，不在本任务内执行）：
    - git diff --name-only --diff-filter=U 统计冲突路径
    - git status --short 统计 A M D U 分桶
    - git log --oneline 基线到 C1 范围核对 317 commits 区间
- 已确认的机器统计（来自 merge-checkpoints.md C1 节）：
  - 相对共同基线：317 commits，其中 310 个非 merge
  - 区间变化：351 files，其中 118 个源码文件（src 与 crates）
  - 行变化：加 93550，减 4141
  - 行数不能直接当作模型上下文大小
- 已确认的历史试合统计（来自 probe-report-2026-08-29.md，仅作类比，不直接等同 C1）：
  - 当时基线 main 92e5884a，custom ae3f4b78，上游 b7b5988b
  - 差距 main 到 upstream 为 1411 commits 与 1482 files
  - 试 merge 初始 UU 为 121，经冻结后 46，再冻后 36，并行清零
  - 白拿 A 从 836 到 339，其中 497 为证据噪音 rm --cached
  - cargo check --lib 第三轮为 297 errors 与 19 warnings
  - 结论为源码冲突是纸老虎，依赖与 API 漂移是主成本
- 工作树文件存在性观察（仅 ls，未读内容）：
  - 试合并工作树 src 下同时存在 lsp 目录与 lsp.rs，eval 目录与 eval.rs，debug 目录与 dap 相关文件，hub.rs 与 jobs.rs 与 agent_hub.rs 与 subagents.rs，extensions.rs 与大量 extension 前缀文件，tools 目录与 ast_tools.rs，interactive 目录与 interactive_ftui.rs 与 tui.rs，workspace.rs 与 workspace_trust.rs 与 semantic_workspace_graph.rs，rpc.rs 与 sdk.rs 与 sse.rs，providers 目录与 models.rs
  - 以上只证明 C1 区间引入或保留了这些模块路径，不证明其内部语义
- custom 定制边界（来自 custom-changes-inventory.md 与 plan-hub-minimal.md 落地证据）：
  - 扩展系统约 20 commit，工具系统 tools.toml 外部化与 verify 子系统与 pwsh 工具，RPC 新增端点与会话持久化，模型层思考链与方言，进程 abort 全链路，TUI 流式修复，配置启动平台定制，workspace 拆分到 pi-core 与 tools.rs 模块化
  - hub 全闭包 A1 已落 custom 4eeb3bf7，加 11258 减 78，含 hub.rs 1134 行与 jobs.rs 5354 行与 agent_hub.rs 585 行与 subagents.rs 2515 行与 secrets.rs 437 行与 worktree_iso.rs 657 行
- 上游主题聚类（来自 upstream-changes-inventory.md，基准为 0.1.22 到 0.2.0 的 212 commits，仅作主题参考，不等同 C1 的 317 commits 全集）：
  - extensions 大重构约 50 commit，models 目录 v2 与认证约 25 commit，v0.2.0 发布加固约 40 commit，session 硬化约 15 commit，subagents 与 roles 与 TUI 约 15 commit，ast_grep 与 ast_edit 约 10 commit，compaction 与 prompt-cache 约 20 commit，pijs VFS 隔离约 10 commit

## 1. C1 区间机器统计摘要

- C1 类型：release 锚点，上游第一段 release 状态
- C1 目的：建立第一个可验证的上游累计状态
- C1 区间：共同基线到 e23c4622 为 317 commits，351 files，118 src files，加 93550 减 4141
- C1 在全区间中的位置：
  - C1 到 C2 为 253 commits，761 files，66 src files
  - C2 到 C3 为 279 commits，165 files，51 src files
  - C3 到 C4 为 275 commits，109 files，57 src files
  - C4 到 C5 为 98 commits，172 files，76 src files
  - C5 到 C6 为 78 commits，148 files，41 src files
- 推断（待冲突清单验证）：
  - 351 files 中必然混杂测试快照、证据包、Beads 数据、发布管道文件，不能按文件总数估算语义冲突
  - 118 src files 为语义风险上限，不是确认冲突数
  - 真实 UU 数以试合并工作树 git diff --name-only --diff-filter=U 为准，本矩阵不预填

## 2. 功能域：LSP 与 DAP 与 eval

- C1 功能：
  - 工作树存在 src/lsp 目录（含 registry.rs 与 text.rs 与 edits.rs 与 jsonrpc.rs 与 client.rs）与 src/lsp.rs
  - 工作树存在 src/debug 目录（含 session.rs 与 adapters.rs 与 dap.rs）与 src/debug.rs
  - 工作树存在 src/eval 目录（含 js_kernel.rs）与 src/eval.rs
  - 按路径判断 C1 区间包含 LSP 客户端与协议脚手架、DAP 适配脚手架、eval 内核脚手架的新增或大幅演进，具体语义未读块，不下结论
- custom 现状与定制边界：
  - custom-changes-inventory 未列出 LSP 与 DAP 与 eval 为 custom 主定制域
  - semantic-map 未将上述文件列入 52 交集高密度区
  - 因此判定 custom 在此域基本无结构性定制，属可白拿面（推断，待 custom 分支文件差异核对确认）
- 涉及文件或模块：
  - src/lsp.rs 与 src/lsp/client.rs 与 src/lsp/jsonrpc.rs 与 src/lsp/registry.rs 与 src/lsp/text.rs 与 src/lsp/edits.rs
  - src/debug.rs 与 src/debug/dap.rs 与 src/debug/adapters.rs 与 src/debug/session.rs
  - src/eval.rs 与 src/eval/js_kernel.rs
- 重叠类型：新增
  - 理由：custom 无同名语义实现，上游为新能力
- 风险等级：低
  - 理由：无 custom 锚点可撞，冲突预期为自动合入或白拿 A
  - 残留风险仅为依赖缺失（若 eval 与 LSP 引用新 crate）与 QuickJS 沙箱策略差异（见 extensions 域）
- 最小集成闭包：
  - 必选：上述 LSP 与 DAP 与 eval 文件原样移植评估
  - 可选：package_manager 与 extension 桥接中的 LSP 钩子（若有引用才纳入，否则冻结）
  - 不纳入：src/extensions 重构面，src/tools 拆分面
- 建议动作：独立移植
  - 不走全量 merge，另起 worktree 以 git show 方式单域评估
- 验证入口：
  - cargo check --lib 只看新增模块缺失 crate 报错
  - 若存在 conformance 或单测脚手架，针对性跑单文件测试，不跑全量
  - 人工冒烟以 LSP 初始化握手与 eval 空内核加载为通过门，不深入语义

## 3. 功能域：Hub 与 Jobs 与 Subagents

- C1 功能：
  - 工作树存在 src/hub.rs 与 src/jobs.rs 与 src/agent_hub.rs 与 src/subagents.rs
  - C1 v0.3.0 区间包含 hub 史诗早期累计状态，但 C1 是否等同 probe 报告中的 hub 史诗 bd-cv653 的 8965 行规模未经块级确认
  - 另有相关文件 src/checkpoint.rs 与 src/handoff.rs 与 src/github.rs 与 src/btw.rs 与 src/secrets.rs 与 src/worktree_iso.rs 在工作树存在
- custom 现状与定制边界：
  - custom 已在 2026-08-29 落地 hub 全闭包 A1（plan-hub-minimal.md 第 7 节证据）
  - 改动为加 11258 减 78，含 hub 与 jobs 与 agent_hub 与 subagents 与 secrets 与 worktree_iso，以及 Cargo 增量与 tools/mod.rs 105 行适配
  - Cargo 保持 asupersync 0.3.9 与 rust 1.85 与 digest 0.10 基线，仅增 portable-pty 与 rustix 与 win32job 与 jsonschema 依赖调整
  - 因此 custom 在此域已有完整可用闭环，不缺 hub 基础能力
- 涉及文件或模块：
  - src/hub.rs 与 src/jobs.rs 与 src/agent_hub.rs 与 src/subagents.rs
  - src/secrets.rs 与 src/worktree_iso.rs
  - 注入点候选：src/app.rs 与 src/cli.rs 与 src/config.rs 与 src/rpc.rs（probe Tier1 已定位为 hub 注入点）
  - 可选外围：src/checkpoint.rs 与 src/handoff.rs 与 src/github.rs 与 src/btw.rs
- 重叠类型：重复
  - 理由：两边均有 hub 与 jobs 实现，custom 版为上游 hub 史诗在 ae3f4b78 时点的全量覆盖，C1 为另一时点的累计状态，属同一功能的时间分叉
- 风险等级：高（若走全量 merge），低（若冻结并走增量观察）
  - 全量 merge 会把已落地的 A1 闭包重新置为冲突，注入点 Tier1 的 12 文件（agent 与 app 与 cli 与 config 与 rpc 与 session 系列与 compaction 系列）会被上游大段覆盖，custom 的 touched_files 与 asupersync 适配会被标 TODO 回补
  - 冻结则无新增风险
- 最小集成闭包：
  - 本次为零文件，不做任何 hub 重搬
  - 下次增量观察仅对比 upstream C1 到 C2 的 hub 差分，按需小移植 roster 与 jobs 钩子
  - 不碰 Cargo 大版本，不碰 asupersync 升级
- 建议动作：保留 custom，冻结
  - C1 的 hub 部分不合，custom 现有 A1 在三个月内无需再动（沿用 plan-hub-minimal 结论）
- 验证入口：
  - hub roster 与 jobs 空表冒烟（plan-hub-minimal 第 4 节门禁）
  - cargo check --lib 零 error 为基线门，不为本次新增门

## 4. 功能域：extensions（含 extensions_js 与 pijs）

- C1 功能：
  - 工作树存在 src/extensions.rs 与 src/extensions_js.rs 与 src/extension_dispatcher.rs 与 src/extension_tools.rs 与 src/extension_events.rs，以及 extension 前缀系列（conformance_matrix 与 inclusion 与 index 与 license 与 popularity 与 preflight 与 replay 与 scoring 与 validation）
  - 上游主题聚类显示 extensions de-monolith 与 compatibility contracts 与 protocol 与 fs_connector 与 exec_mediation 等结构性重构，以及 pijs VFS 隔离与 bridge secrets 与 owner-isolated shards
  - C1 是否完整包含上述全部重构未经块级确认，按路径存在判断至少包含扩展多文件化开端
- custom 现状与定制边界：
  - custom 最大定制域，约 20 commit
  - 含 manifest-aware 加载与 sibling-index 误发现修复与混合包 root 收集，node:fs 落盘持久化（renameSync 与 unlinkSync 与 rmSync 与 rmdirSync 与 writeFileSync 与 appendFileSync），exec hostcall 经 AMAC 交错，abort 桥接 QuickJS，replaceInput 参数重写，ctx.modelRegistry 暴露，execute_command hasUI 上下文，run_extension_command agent_end，legacy runner fail closed
  - 关键锚点在旧单文件结构 src/extensions.rs 与 src/extensions_js.rs
  - probe 报告将 src/extensions 目录 18 个冲突文件列为冻结面，以 git rm -rf 方式整体冻结
- 涉及文件或模块：
  - src/extensions.rs 与 src/extensions_js.rs
  - src/extension_dispatcher.rs 与 src/extension_tools.rs 与 src/extension_events.rs 与 src/package_manager.rs
  - src/extensions 目录（若 C1 已拆分）与 pijs 相关 VFS 隔离代码
- 重叠类型：结构冲突为主，兼语义冲突
  - 结构冲突：上游单体拆多文件，custom 定制锚在旧结构，重命名检测可能失败
  - 语义冲突（推断，需块级确认）：VFS 隔离要挡，custom fs 持久化要放，两者文件系统可达性语义可能直接对立
  - 另有重复子项：manifest-aware 与目录聚类两边均有，需留一判断，但本次未读块，不指定留哪一
- 风险等级：最高
  - 理由：semantic-map 明确标为最高风险，probe 列为冻结面，SOP 列为单独迁移项目
  - 全量 merge 会触发大文件破裂风险（probe 教训：大于 1500 行文件并行合并易丢右大括号，perf.rs 与 semantic_workspace_graph.rs 曾破裂）
- 最小集成闭包：
  - 本次为零文件，整体冻结
  - 未来迁移项目闭包另立设计，不在本矩阵内展开，仅记录需先读上游新架构再决定 custom 定制挂载点
- 建议动作：冻结，另立设计
  - 不在 C1 阶段解 extensions 任何冲突
  - 沿用 probe 冻结手法（git rm -rf src/extensions 上游新文件，单文件取 ours），但本次不执行，仅记录手法来源
- 验证入口：
  - extension_conformance_matrix 与 ext_conformance 单测（针对性跑，不跑全量）
  - manifest 加载回归与 fs 持久化回归为 custom 必保项，VFS 隔离语义需新设计评审后才设门

## 5. 功能域：tools 拆分（含 ast 工具与 verify）

- C1 功能：
  - 工作树存在 src/tools 目录（含 mod.rs 与 verify.rs 与 pwsh.rs 与 read.rs 与 edit.rs 与 hashline.rs 与 bash.rs 与 grep.rs 与 ls.rs 与 find.rs 与 write.rs 与 shell.rs 与 touched_files.rs 与 tests.rs）与 src/ast_tools.rs 与 src/tool_overrides.rs
  - 上游主题聚类显示 ast_grep 与 ast_edit 结构化工具为全新能力（bd-cv653.1.3 约 10 commit）
  - probe 显示 Tool execute 从 4 参到 5 参（新增 abort）导致约 130 个跨 20 个 impl 的 arity 漂移
- custom 现状与定制边界：
  - custom 已将 tools.rs 单文件模块化为 src/tools 目录（985c3962）
  - 新增 verify 验证子系统与 pwsh 内置工具与 read 缓存键修复与上限放宽，tools.toml 外部化与 tool_overrides
  - semantic-map 明确指出 custom 已拆分而上游仍改单文件，上游改动需映射到新结构
  - probe 将 src/tools.rs 以 rm 方式保 tools 目录拆分
- 涉及文件或模块：
  - src/tools/mod.rs 与 src/tools/verify.rs 与 src/tools/pwsh.rs 与 src/tools/read.rs 与 src/tools/edit.rs 与 src/tools/hashline.rs 与 src/tools/bash.rs 与 src/tools/touched_files.rs
  - src/ast_tools.rs 与 src/tool_overrides.rs
  - 上游 Tool trait 定义侧（具体文件未经块级确认，不点名行号）
- 重叠类型：结构冲突兼依赖冲突
  - 结构冲突：目录拆分对单文件，git 难以自动映射
  - 依赖冲突：Tool execute arity 漂移，ExtensionSession 与 HostActions arity 漂移（probe 计约 12 处）
  - 新增子项：ast_grep 与 ast_edit 属新增，可分离
- 风险等级：高
  - 理由：改动面广（20 个 impl），机械改量大，但单点语义简单
  - 若全量 merge，会同时引入结构映射与 arity 漂移，编译错误呈数百量级（probe 类比 297，非 C1 实测）
- 最小集成闭包：
  - 独立移植候选：ast_grep 与 ast_edit 两个工具文件及其 tools 注册项
  - 适配层：沿用 plan-hub-minimal 的 shim 思路，为 custom 的 4 参 impl 写 adapt 桥接，不改 20 个 impl
  - 不纳入：上游对旧 tools.rs 单文件的全量改动
- 建议动作：保留 custom 结构，独立移植新增工具
  - tools 目录结构不动，Cargo 不跟上游全量依赖
- 验证入口：
  - cargo check --lib 首个 arity 报错为引导信号
  - cargo test 针对 tools 单文件或单模块，不跑全量
  - ast 工具以结构查询空跑通为门，不深入规则语义

## 6. 功能域：session 与 storage（含 compaction）

- C1 功能：
  - 工作树存在 src/session.rs 与 src/session_store_v2.rs 与 src/session_sqlite.rs 与 src/session_index.rs 与 src/session_metrics.rs 与 src/session_import.rs 与 src/session_picker.rs，以及 src/compaction.rs 与 src/compaction_worker.rs 与 src/file_lock.rs 与 src/migrations.rs
  - 上游主题聚类显示 session 与 session-store 硬化约 15 commit，含 Windows 工件目录 pin 与 durable clean 与 dirty 状态与 fsync 拒绝容忍与 file_lock 心跳续期与 fsqlite 迁移计划，以及 compaction 多处重构与 prompt-cache 调整
- custom 现状与定制边界：
  - custom 有 RPC 会话持久化（RpcSessionPersister，进程侧主动持久化，去每条消息 fsync，仅 AgentEnd 落盘）与 Windows 文件竞争重试与截断 SSE 分类重试与 persister 链根修复
  - semantic-map 将 session.rs 判为互补（上游 Bearer 恢复与 role 字段与 ModelChange 对 custom Windows 重试与 persist，无冲突，各自保留）
  - 但 probe Tier1 将 agent 与 app 与 cli 与 config 与 rpc 与 session 系列 5 文件与 compaction 系列 2 文件列为 hub 注入重冲突区，需取 upstream 全量保编译并标 TODO 回补
  - 因此 session 域存在双面性：语义互补，但注入点结构重叠
- 涉及文件或模块：
  - src/session.rs 与 src/session_store_v2.rs 与 src/session_sqlite.rs 与 src/session_index.rs 与 src/session_metrics.rs
  - src/compaction.rs 与 src/compaction_worker.rs
  - src/file_lock.rs 与 src/migrations.rs 与 src/resource_governor.rs
- 重叠类型：互补为主，注入点为结构冲突
  - 互补：持久化策略与硬化手段不同源，可共存
  - 结构冲突：hub 注入点散在 app 与 cli 与 config 与 rpc 与 session 与 compaction，若全量 merge 会被上游大段覆盖
- 风险等级：中高
  - 理由：单文件语义可融合，但注入点需人判，且 fsqlite 与 file_lock 涉及存储格式与并发语义，未经块级确认前不得合
- 最小集成闭包：
  - 挑合候选：fsync 拒绝容忍与 file_lock 心跳与 Windows 工件 pin（若确认为小增量）
  - 冻结：session_store_v2 状态机大改与 fsqlite 大版本迁移
  - 注入点：仅在 hub 增量观察需要时打最小钩子，不搬大段
- 建议动作：保留 custom，挑合小增量
  - 存储格式与锁语义未明前冻结 v2 状态机
- 验证入口：
  - session 持久化单测与 print 模式会话持久化冒烟
  - Windows 文件竞争复现（custom 原有修复的回归门）
  - cargo check --lib 先行，不直接跑全量 session 集成测试

## 7. 功能域：TUI 与 FTUI（含 interactive）

- C1 功能：
  - 工作树存在 src/interactive 目录（含 view.rs 与 tests.rs 与 tool_render.rs 与 perf.rs 与 state.rs 与 keybindings.rs 与 model_selector_ui.rs 与 commands.rs 与 agent.rs 与 tree.rs 与 tree_ui.rs 与 share.rs 与 text_utils.rs 与 ext_session.rs 与 file_refs.rs 与 conversation.rs）与 src/interactive.rs 与 src/interactive_ftui.rs 与 src/tui.rs
  - 上游主题聚类显示 subagents 与 roles 与 TUI 约 15 commit，含 model roles 与 cheap auto-titling 与 task-role subagents
  - merge-checkpoints 显示 FrankenTUI 默认切换在 C3（9b2851e4），因此 C1 应为 TUI 增量早段，非默认切换点（推断，以 C3 主题为据）
- custom 现状与定制边界：
  - custom 在 interactive 有重构区（probe 明确列 interactive 为 custom 重构区之一）
  - custom TUI 定制为流式帧间残留清除与 Windows Terminal 全屏闪烁修复与 print 模式会话持久化，属渲染与平台适配层
  - semantic-map 将 interactive/commands.rs 列为上游 role 命令大改、custom 少量路过
  - probe Tier2 将 extension_dispatcher 与 extensions 系列与 interactive 系列 6 文件与 perf 列为保 custom 骨架、仅植入最小 hub roster 与 jobs 钩子
- 涉及文件或模块：
  - src/interactive/commands.rs 与 src/interactive/ext_session.rs 与 src/interactive/state.rs 与 src/interactive/agent.rs
  - src/interactive/perf.rs（probe 曾破裂，需全量覆盖治法警示）
  - src/interactive_ftui.rs 与 src/tui.rs 与 src/theme.rs 与 src/keybindings.rs
- 重叠类型：互补为主，结构锚点漂移
  - 互补：上游 role 与 titling 对 custom 渲染修复，语义不直接对立
  - 结构风险：custom 重构后 interactive 锚点与上游不一致，大段合并易丢右大括号
- 风险等级：中
  - 若冻结骨架则低，若全量 merge 则因大文件破裂与 TUI 默认路径切换前兆而升为中高
  - C1 阶段不触 FrankenTUI 默认切换（C3 边界），是控制风险的关键
- 最小集成闭包：
  - 可选小钩子：interactive/commands.rs 中的 hub roster 与 jobs 最小钩子（沿用 probe completed_tan_event 思路，但本次不执行）
  - 冻结：FTUI 默认路径、view 与 state 大重构、perf 大文件全量覆盖
- 建议动作：保留 custom 骨架，冻结 FTUI 迁移
  - TUI 新特性另起小移植，不在 C1 全量内解
- 验证入口：
  - interactive 单测文件针对性跑（src/interactive/tests.rs 对应单文件门）
  - 流式残留与 Windows Terminal 闪烁人工目检为 custom 回归门
  - hub roster 钩子以可列空表为通，不深入 TUI 交互

## 8. 功能域：workspace 与 security（含 permissions 与 auth）

- C1 功能：
  - 工作树存在 src/workspace.rs 与 src/workspace_trust.rs 与 src/semantic_workspace_graph.rs（含 6266 行规模警示，probe 曾破裂）与 src/permissions.rs 与 src/auth.rs 与 src/secrets.rs 与 src/validation_broker.rs 与 src/hostcall_amac.rs 系列与 src/resource_governor.rs
  - 上游主题聚类显示 security 加固含 bound auth I/O 与 lock timeouts 与 fail-closed loads
  - 上游另有 Windows 工件 pin 与 resource routing（C4 主题，但 C1 可能已有前兆，未经块级确认）
- custom 现状与定制边界：
  - custom 将部分逻辑移到 crates/pi-core（workspace 拆分 6667784b），大量文件为搬家式修改
  - custom 安全与执行定制含 AMAC exec 交错（hostcall_amac.rs 为 custom 主场，semantic-map 列为低频路过区但注明 AMAC 是 custom 的）与 ProcessGuard 与 wait_with_cancellation 与 abort 全链路与 SYSTEM.md 项目级 prompt 与 skill_mode 过滤与 disabledTools
  - plan-hub-minimal 落地含 permissions.rs 上游回灌与 session.rs lock_exclusive 到 FileExt 适配，说明 permissions 与锁语义已发生过一次融合，需谨慎二次覆盖
- 涉及文件或模块：
  - src/workspace.rs 与 src/workspace_trust.rs 与 src/semantic_workspace_graph.rs
  - src/permissions.rs 与 src/auth.rs 与 src/secrets.rs
  - src/hostcall_amac.rs 与 src/abort.rs 与 src/subprocess_handle.rs
  - crates/pi-core 相关搬家文件（本次未列全量清单，不点名）
- 重叠类型：互补兼依赖冲突
  - 互补：bound I/O 与 fail-closed 对 AMAC 交错，目标一致但机制不同层
  - 依赖冲突：workspace 拆分导致原位置与新 crate 位置对不上，上游改原位置，custom 用新位置
  - 语义敏感：auth 与 permission 为安全边界，留一或融合需设计评审，不可机械取 theirs
- 风险等级：中高
  - 理由：安全边界不可回退，workspace 图大文件有破裂史，crate 搬家导致映射成本
- 最小集成闭包：
  - 挑合候选：bound auth I/O 与 lock timeouts 小修复（若确认为孤立函数）
  - 冻结：workspace 图大重构、resource routing、crates 搬家映射
- 建议动作：保留 custom 安全边界，挑合孤立加固
  - 任何 auth fail-open 风险的改动直接冻结
- 验证入口：
  - auth fail-closed 单测与 doctor 平台检查
  - AMAC 交错回归（custom 原有行为门）
  - semantic_workspace_graph 以全量覆盖为破裂治法储备，但本次不执行

## 9. 功能域：RPC 与 SDK（含 ACP 与 SSE 与 HTTP）

- C1 功能：
  - 工作树存在 src/rpc.rs 与 src/sdk.rs 与 src/acp.rs 与 src/sse.rs 与 src/http 目录（含 mod.rs 与 sse.rs 与 test_api.rs 与 test_asupersync.rs 与 client.rs）与 src/http_shim.rs
  - semantic-map 显示 rpc.rs 为上游新版端点与 parity（7 commit）对 custom 8 自定义端点加持久化（22 commit），判为中高
  - semantic-map 显示 sdk.rs 与 acp.rs 为互补（上游 prompt-cache 默认与会话级扩展提示与推理签名清理，对 custom persist 参数）
  - merge-checkpoints 显示 C6 有 proxy 与 MCP 与 RPC 与扩展收尾，但 C1 是否含 proxy 变化未经确认，http client 变化需单独验证（C6 风险提示，不直接归因 C1）
- custom 现状与定制边界：
  - custom RPC 新增端点含 get_system_prompt（含 tools）与 get_tree 与 get_version 与 estimate_tokens 与 queue 管理（remove_from_queue 与 clear_queue 与 get_queue 加 queue_update 事件）与 append_custom_entry
  - custom 持久化含 RpcSessionPersister 与 SSE transient 自动重试与 persister 链根修复
  - custom 为 RPC 高频改动方，端点命名与行为为 custom 对外契约
- 涉及文件或模块：
  - src/rpc.rs 与 src/sdk.rs 与 src/acp.rs
  - src/sse.rs 与 src/http/client.rs 与 src/http/mod.rs
  - src/app.rs 与 src/cli.rs 与 src/config.rs（hub 注入点，见 Hub 域）
- 重叠类型：互补为主，命名与行为重叠待查
  - 互补：各加端点
  - 待查：端点命名或行为是否重叠未经块级确认，不下重叠结论
- 风险等级：中高
  - 理由：RPC 为对外契约，hub 注入点集中在此，全量 merge 易丢 custom 端点或改变行为
- 最小集成闭包：
  - 保留 custom rpc.rs 为主，合入上游小增量 parity 端点（若命名不冲突）
  - hub 注入点仅打最小初始化钩子，不搬上游大段
  - http client 与 proxy 变化留到 C6 阶段单独验证，本次冻结
- 建议动作：保留 custom，独立移植上游小端点
  - 命名冲突时以 custom 对外契约为准，另立命名映射设计
- 验证入口：
  - RPC parity 测试与 queue 端点冒烟
  - SSE 重试回归（custom 原有 transient 分类门）
  - sdk persist 参数回归

## 10. 功能域：providers 与 search（含 models 目录）

- C1 功能：
  - 工作树存在 src/providers 目录（含 mod.rs 与 openai.rs 与 openai_responses.rs 与 gemini.rs 与 vertex.rs 与 anthropic.rs 与 bedrock.rs 与 azure.rs 与 cohere.rs 与 copilot.rs 与 cursor.rs 与 gitlab.rs 与 model_fetch.rs）与 src/models.rs 与 src/model.rs 与 src/provider.rs 与 src/provider_metadata.rs 与 src/model_routing.rs 与 src/model_selector.rs 与 src/failover.rs 与 src/web_search.rs 与 src/dialects.rs
  - 上游主题聚类显示 models 目录 v2 与 credential order 与 fetched catalog 与 thinkingLevelMap 与 thinkingFormat 尊重（gh 166）与 GPT-5.6 家族种子
  - semantic-map 显示 openai.rs 为语义重复（上游 DeepSeek thinkingFormat 对 custom 同名功能，决策留一优先上游），models.rs 为融合（上游目录驱动加 custom xhigh 特例）
- custom 现状与定制边界：
  - custom 模型层含 gemini 思考链（thinkingConfig 发送加 thought part 接收，带实测证据）与 DeepSeek 方言（honor compat.thinkingFormat）与 reasoning_effort 全 OpenAI-compatible 支持与 xhigh（supports_xhigh 检查 compat.thinkingLevelMap）与 persist 参数（set_model 与 set_thinking_level）与 opencode-go provider 新增
  - custom-changes-inventory 将 providers 列为 9 个交集文件的高冲突风险域之首
  - probe Tier3 将 models.rs（9866 行增量警示）与 providers 5 文件列为 custom 为主、合入上游小增量
- 涉及文件或模块：
  - src/providers/openai.rs 与 src/providers/gemini.rs 与 src/providers/vertex.rs 与 src/providers/anthropic.rs 与 src/providers/bedrock.rs 与 src/providers/model_fetch.rs
  - src/models.rs 与 src/model.rs 与 src/provider_metadata.rs
  - src/web_search.rs 与 src/model_routing.rs（search 与路由外围）
- 重叠类型：重复兼互补
  - 重复：thinkingFormat 与 thinkingLevelMap 两边均实现，需留一或融合
  - 互补：上游目录 v2 更全面，custom xhigh 与实测思考链为特例补充
  - 新增：opencode-go 与 GPT-5.6 种子等目录数据为增量
- 风险等级：中
  - 理由：语义清晰，可按 semantic-map 留一与融合规则解，不涉及结构拆分
  - 残留风险为 models.rs 大文件增量（9866 行警示）与 credential order 注入方式，未经块级确认前不全量跟
- 最小集成闭包：
  - 融合候选：上游目录驱动（thinking 尊重与 credential order）加 custom xhigh 特例
  - 留一候选：DeepSeek thinkingFormat 优先上游官方实现，删 custom 同名实现（沿用 semantic-map 决策，但需块级复核后执行）
  - 不纳入：models catalog 全量 fetch 与 persist 与 refresh CLI 大闭环（按需拆小）
- 建议动作：融合，独立移植
  - 不走全量 merge，按文件逐个小移植
- 验证入口：
  - conformance_fixtures 集成测试文件针对性跑（cargo test --test conformance_fixtures）
  - provider-canonical-id 与 thinking 相关单测（若存在则针对性跑，不跑全量）
  - DeepSeek 空思考与 gemini thought part 实测证据为 custom 回归门

## 11. 剩余冲突类别（待 Main Agent 以工作树实测补数）

- 以下类别为基于 SOP 与 probe 的预期分桶，不是 C1 实测数，实测以 git diff --name-only --diff-filter=U 为准：
  - 生成物与测试快照类：tests 快照与 docs 证据与 contracts 与 perf 工件与 artifacts，预期批量 ours 加重跑生成，不手工解
  - 上游内部数据类：.beads 目录，预期 git rm -rf，不合入
  - 冻结域类：src/extensions 多文件重构面与 src/tools.rs 单文件旧路径（若仍以冲突形式出现），预期冻结手法处理
  - 根配置类：Cargo.toml 与 Cargo.lock 与 .cargo/config.toml 与 .gitignore 与 AGENTS.md 与 README.md 与 examples 与 benches/extensions.rs，预期回 custom 基线（rust 1.85 与 asupersync 0.3.9 与 digest 0.10），dev 依赖按需跟
  - 源码内容类：Tier1 hub 与会话注入点约十余文件，Tier2 扩展与 TUI 约十文件，Tier3 其他约十余文件（数量级沿用 probe 36 的分法作参考，不等同 C1）
  - 依赖漂移类：Tool execute arity 与 ExtensionSession arity 与缺失 crate（globset 与 fsqlite 与 portable-pty 与 tiktoken-rs 可选与 pprof 与 rustix 与 htmd 与 jsonschema）与缺失 helper 函数，预期以 cargo check --lib 首错为引导逐个收敛
  - 大文件破裂类：大于 1500 行文件（perf.rs 与 semantic_workspace_graph.rs 与 models.rs）在并行解冲突时易丢右大括号，治法储备为 git show 上游版本全量覆盖，但本次不执行
- 当前缺口：
  - C1 试合并工作树的 UU 总数未知
  - UU 按路径分桶未知
  - 白拿 A 数未知
  - cargo check 首错未知
  - 以上四项必须由 Main Agent 在工作树中实测后回填，本矩阵不编造

## 12. 当前 C1 全量 merge 应停止的原因

- 区间过大，违反模型上下文边界：
  - C1 为 317 commits 与 351 files 与加 93550 减 4141，模型不得读取 317 个提交全文，只能处理真实冲突与首个验证失败
  - 全量 merge 会把发布管道与证据包与 Beads 数据等噪音一并带入，偏离 merge-checkpoints 每阶段停止条件中的 Fork 政策与依赖漂移门
- 依赖漂移大于源码冲突：
  - 沿用 SOP 与 probe 结论，上游依赖升级会自动合入 Cargo.toml 而无冲突提示，custom 代码用旧 API 会产生数百编译错误
  - C1 全量若切 theirs 会触发 asupersync 与 rust 版本与 digest 与 fs4 等大版本漂移，若保 ours 则 hub 与 jobs 新代码缺依赖，两头皆需数万 token 级机械适配
  - 在无适配计划前继续全量 merge，违反每阶段停止条件中的依赖漂移项
- 结构性迁移尚未形成设计：
  - extensions 单体拆多文件与 tools 单文件拆目录与 workspace 搬 crates 三处结构冲突，均需先读新架构再定挂载点
  - SOP 明确此类需单独迁移项目，merge-checkpoints 停止条件亦规定需大规模结构迁移但尚未形成设计时停止当前阶段
  - C1 全量会同时引爆三处，不符合挑着合原则
- hub 已落地，全量无增益：
  - custom 已有 hub 全闭包 A1，全量 merge 只是把同一功能的时间分叉重新冲突一遍，注入点还会被上游大段覆盖并标 TODO 回补
  - 下次应走 C1 到 C2 的 hub 增量观察，而非重合 C1 全量
- 安全边界不可机械合并：
  - auth 与 permissions 与 AMAC 与 workspace_trust 为安全边界，fail-open 代价高，需设计评审
  - 全量 merge 默认取 theirs 或 ours 皆有丢语义风险，违反保留采纳冻结三类决策需逐项记录的要求
- 综上：C1 全量 merge 应在矩阵完成后续停止，不进入解冲突与编译修复循环，转入首个低风险候选的独立移植

## 13. 首个低风险候选

- 首个候选：ast_grep 与 ast_edit 结构化工具独立移植
  - 备选（同级低风险）：LSP 与 DAP 与 eval 新增脚手架独立移植
  - 本矩阵推荐 ast 工具优先于 LSP，因为上游主题聚类明确将其标注为全新且与 custom 无关，而 LSP 仍有 QuickJS 沙箱策略的间接关联
- 候选依据：
  - 来自 upstream-changes-inventory 值得合部分第一条：ast_grep 与 ast_edit 全新能力零冲突
  - custom-changes-inventory 未列出同名实现，semantic-map 52 交集未覆盖，工作树路径 src/ast_tools.rs 可作为移植锚点（仅路径存在性证据，不含语义保证）
- 最小闭包（推断，需块级确认）：
  - 必选：src/ast_tools.rs 及其在 tools 注册中的挂载项
  - 适配：若 Tool execute arity 不匹配，以 shim 桥接，不改 20 个 impl
  - Cargo：按需最小增量，不跟 tiktoken 与 pprof 与 ftui 可选依赖
  - 冻结：extensions 与 session 与 TUI 与 workspace 全部不动
- 建议动作：独立移植
  - 另起 worktree 以 git show 方式单域评估，不走 merge upstream/main
- 验证入口：
  - cargo check --lib 零新增 error
  - cargo clippy --lib 加 -D warnings 与 cargo fmt --check（日常门禁，不跑 --all-targets）
  - cargo test 针对 tools 或 ast 单模块，不跑全量
  - 空查询冒烟通过即算首个候选打通，不深入规则语义
- 不做事项：
  - 不升 Cargo 大版本，不碰 asupersync，不碰 extensions 重构，不搬 session 存储格式

---

- 矛盾与待澄清：
  - C1 是否已含 extensions de-monolith 全量、models catalog v2 全量、VFS 隔离全量，未经冲突块确认，本矩阵按主题存在性记录为可能包含，不作定量断言
  - custom 分支自 plan-hub-minimal 落地后是否新增 TUI 或 session 定制，未在本任务内核对 custom log，需 Main Agent 以 git log 复核
  - 工作树 UU 与 A 分桶待实测回填后，本矩阵的风险等级可能需下调或上调，尤其是 Tier1 注入点的中高项
