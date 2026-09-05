# C1 综合移植计划：v0.3.0（e23c4622）

> 性质：独立、保守的计划文档。只分析，不修改源码，不解 merge 冲突。
> 状态：试合并进行中。`integration/c1` 已创建，初始 82 未解决路径，按确定性规则处理后剩余 33 个带标记文件；无验证结果，无任何功能迁移完成。
> 方向：不做 C1 全量 merge；先做一个完整功能闭包独立移植试点，首选 LSP。

## 0. 本计划的依据与非依据

依据（仅以下材料）：

- `docs/upstream/README.md`：同步工作区入口，`main` / `custom` / 临时 integration 线关系，Fork 政策
- `docs/upstream/merge-checkpoints.md`：C1 定义（317 commits，其中 310 非 merge；351 files；118 源码文件；+93,550 / -4,141），C1–C6 关系，每阶段停止条件，模型上下文边界
- `docs/upstream/fork-merge-sop.md`：改造式 Fork SOP（merge 是唯一载体但选择性采纳；试 merge 必须走 worktree；依赖漂移大于源码冲突；生成物批量 `--ours`；冻结清单模板）
- `docs/upstream/semantic-map.md`：2026-08-16 的 52 文件交集决策底稿，用作候选风险索引，不是当前精确冲突清单
- `docs/upstream/probe-report-2026-08-29.md`：全量试合归档结论（121 冲突可清零但 297 编译缺口太大，本次不全量合流，改走最小闭包移植）
- 本目录三张矩阵：
  - `feature-matrix.md`：C1 功能全景归纳（A–G 域，采纳/暂缓/冻结初步建议；C1 功能分组已由此重建，不再是占位）
  - `dependency-matrix.md`：依赖与 API 漂移决策底稿（类别 A 功能必要 vs 类别 B 整体状态；默认 `Cargo.toml`/`lock` 取 `--ours`）
  - `overlap-risk-matrix.md`：分域重叠与冲突风险（含首个低风险候选讨论；其 §13 与本计划首选存在反例，理由见第 4 节）
- integration 线实测（机器级事实查询，未读冲突块全文）：
  - 当前分支 `integration/c1`（worktree `HEAD` 为 `ref: refs/heads/integration/c1`）
  - `MERGE_HEAD` 为 `e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`，`ORIG_HEAD` 为 `d1786d8b9658ac2553a1a46a3bca5414b1bf9b67`，`MERGE_MODE` 为 `no-ff`
  - 工作树为 `C:\Users\m\AppData\Local\Temp\pi-c1-integration`
  - 无 shell 入口，未执行 `git diff --name-only --diff-filter=U` 与 `git status --short`；初始数以 `MERGE_MSG` 的 `# Conflicts:` 清单计数 82 为准
  - 剩余数以全树 `<<<<<<< HEAD` 标记扫描推导 33 为准（已排除 `src/commit_split.rs` 与 `tests/commit_split.rs` 字符串字面量、`docs/upstream/probe-report-2026-08-29.md` 命令引用；工作树变化时以实测为准）

非依据（明确不做）：

- 不读取 317 个提交全文
- 不把区间文件数、行数当作冲突数或工作量精确估计
- 不把测试快照、证据包、生成物差异当作语义差异
- 不运行 `cargo`，不做源码改动，不解任何语义冲突
- 不虚构任何功能迁移完成；试点未启动前所有采纳结论均为待验证

## 1. 总策略：先收敛冲突地图，再做独立移植试点

禁止把 C1 当作一次全量 merge 直接吞下。C1 全量 merge 已停止，改为两层走。

第一层：冲突地图收敛中（部分完成）。

- 临时线已建：从 `custom` 起 `integration/c1`，只合入 C1 提交 `e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`，使用 `--no-commit --no-ff`
- 初始 82 路径中确定性规则已处理：
  - `.beads/` 已移除（工作树无该目录；初始清单 4 个 `.beads/` 路径已移出视野）
  - `custom` 已删除冻结文件已按保留 custom 处理（初始清单中 `src/tools.rs` 旧路径、`src/extensions/tests/` 等不再带标记）
  - 部分生成物/测试已按保留 custom 或移出处理（初始清单中 `docs/` 证据、`tests/perf/reports/`、`tests/ext_conformance/reports/` 等不再带标记）
- 当前剩余 33 个带标记文件：
  - 根配置 2 个：`Cargo.toml`、`Cargo.lock`
  - 示例 1 个：`examples/pi_debug.rs`
  - `src/` 30 个：`agent.rs`、`agent_hub.rs`、`app.rs`、`auth.rs`、`cli.rs`、`compaction.rs`、`compaction_worker.rs`、`config.rs`、`extensions.rs`、`extensions_js.rs`、`hub.rs`、`interactive/commands.rs`、`interactive/tests.rs`、`jobs.rs`、`lib.rs`、`main.rs`、`models.rs`、`package_manager.rs`、`perf_build.rs`、`providers/bedrock.rs`、`providers/model_fetch.rs`、`rpc.rs`、`sdk.rs`、`secrets.rs`、`semantic_workspace_graph.rs`、`session.rs`、`session_picker.rs`、`session_sqlite.rs`、`subagents.rs`、`worktree_iso.rs`
- 结构观察（仅 `ls`，未读块）：
  - `src/lsp/` 存在 5 文件（`client.rs`、`edits.rs`、`jsonrpc.rs`、`registry.rs`、`text.rs`）
  - `src/tools/` 为目录形态 13 文件；`src/extensions/` 目录不存在（仍为单文件形态）
  - `src/eval/`、`src/debug/`、`src/hub.rs`、`src/jobs.rs`、`src/subagents.rs` 存在
- 本层不改代码，不升级依赖，不做语义融合；剩余 33 的语义归因仍待试点 worktree 复核

第二层：按完整功能闭包做独立移植试点（未启动）。

- 功能闭包定义：一个可独立说清动机、可独立验证、可独立采纳或冻结的上游变化集合，含其直接依赖与 custom 接入点
- 不按文件逐个跟随上游；不按提交逐个 cherry-pick（SOP 已确认 cherry-pick 在改造式 Fork 上不可行）
- 不在当前 `integration/c1` 工作树上直接解 33 个冲突；另建短路径干净 worktree 做试点，避免继承冲突与 Windows 长路径
- 每个试点走同一五段格式：动机 → 依赖 → custom 接入边界 → 针对性验证 → 采纳/冻结
- 一个试点没有走完五段，就不算有结论；没有结论的默认状态是冻结，不是采纳
- 本计划第 3 节保留 P0–P7 结构；推荐顺序见第 4 节（已收敛为 LSP 首试点）

## 2. C1 功能分组重建说明

- 初步重建已由 `feature-matrix.md` 完成：按 C1 CHANGELOG v0.3.0 与 v0.2.0 相关段落归纳为 A–G 域，不再是占位
- 但以下边界仍成立：
  - 未读 317 提交全文，未拉取 `.beads` issue 全文，未做三方冲突归因
  - 各域代表性提交之后是否有同域修复与回滚未收敛，特别是 hub 在 C2 才收束、FTUI 在 C3 才默认切换，C1 结论不可外推到 C2 以后
  - 具体文件归属仍以冲突地图校准为准；第 3 节路径是候选索引，不是定稿
  - 若某个分组完全由证据、文档、内部工具组成，直接归入噪音桶，不立闭包
- 重建后调整允许，但调整必须记录理由

## 3. C1 候选功能闭包（每个闭包五段，P0–P7 结构保留）

### 闭包 P0：噪音剥离与冲突地图定稿（部分完成，未定稿）

- 动机：把不产生语义的差异先移出视野，让后续试点只看真实源码冲突；这是 SOP 与探针都验证过的最低成本步骤
- 依赖：无源码依赖；依赖 Git 实测冲突清单（初始 82 已有，剩余 33 为标记扫描推导，仍需 `git diff --name-only --diff-filter=U` 与 `git status --short` 实测复核）
- custom 接入边界：
  - 上游内部数据（例如 `.beads/` 类）：删除，不带入（已执行，工作树无 `.beads/`）
  - 测试快照、证据包、perf 生成物、conformance artifacts：默认保留 custom（`--ours`），重跑生成，不手工逐个解（初始清单中对应路径已不再带标记）
  - `benches/`、`docs/` 生成物：同上，批量处理
- 验证：
  - `git diff --name-only --diff-filter=U` 复核剩余冲突数（待执行；当前 33 为替代方法推导）
  - 全仓搜索冲突标记为零（例如 `<<<<<<<`；待执行；当前扫描仍有 33 文件带标记）
  - 记录白拿新增文件（`A` 状态）中属于证据噪音的部分并移出（`rm --cached` 路线按 SOP 执行；待执行）
- 采纳/冻结：
  - 采纳：噪音处理手法本身（批量脚本与清单；已部分验证）
  - 冻结：无；本闭包不冻结源码语义，只做地图
- 当前状态：确定性部分完成；剩余 33 未定稿，不得引用为决策结果

### 闭包 P1：根配置与依赖基线（待试点 worktree 复核）

- 动机：依赖漂移是最大隐藏成本，且会自动合入而无冲突提示；必须先定基线再看试点，否则试点验证全是误报
- 依赖：P0 剩余 33 复核后才能看清根配置是否真实冲突（当前 `Cargo.toml`、`Cargo.lock` 均带标记，属实）
- custom 接入边界：
  - `Cargo.toml` / `Cargo.lock`：默认保留 custom 基线；上游依赖升级（digest、swc、asupersync、Rust 版本要求等类型）不得自动跟随（按 `dependency-matrix.md` §5 默认 `--ours`）
  - `.cargo/config.toml`、`build.rs`、嵌入资源相关文件：默认保留 custom；上游引用新 crate 的构建逻辑不自动合入
  - `AGENTS.md`、`README.md`、`.gitignore`、`examples/`：按 Fork 政策处理，`.github/dependabot.yml` 禁用项保持禁用
- 验证：
  - 记录 `Cargo.toml` / `Cargo.lock` 在试合并中是自动合入还是冲突（当前为冲突，带标记；自动合入部分必须人工复核依赖增量）
  - 不运行 `cargo`（本计划阶段禁止）；只记录需要试点验证的依赖增量清单（`dependency-matrix.md` §6 的 10 个未知项）
  - 复核 Fork 政策文件没有被上游自动化无条件覆盖
- 采纳/冻结：
  - 采纳：仅限 custom 基线确认与政策确认（待试点确认）
  - 冻结：所有上游依赖升级默认冻结，进入试点时按需逐个评估，不批量跟进
- 当前状态：未完成；试点启动前必须先定基线

### 闭包 P2：Provider 与模型目录小增量（冻结评估，未启动）

- 动机：semantic-map 显示 `src/providers/openai.rs`、`src/models.rs`、`src/providers/anthropic.rs`、`src/provider_metadata.rs` 一带以上游为主或语义重复；`feature-matrix.md` D5 判极高价值但须语义去重
- 依赖：P1（依赖基线先定，否则 provider 增量可能引用新依赖）；可能依赖模型目录数据文件
- custom 接入边界：
  - custom 在模型 thinking 级别、credential 处理上有自己的定制；以上游官方实现为默认去向，但 custom 独特价值（例如 xhigh 特例、Windows 相关、credential precedence 定制）必须保留
  - 目录驱动大改走融合路线：保留上游目录，补 custom 特例，而不是整文件二选一
  - 当前剩余 33 中 `models.rs`、`providers/bedrock.rs`、`providers/model_fetch.rs` 带标记，属本闭包候选观察面（未读块，不下结论）
- 验证：
  - 针对性单模块测试与 provider 元数据快照测试（命令在试点 worktree 确定，本计划阶段只记录需要跑哪些，不执行）
  - 确认 credential precedence 行为没有被意外反转
- 采纳/冻结：
  - 采纳条件：冲突块小、依赖无新增、行为差异可解释（待验证）
  - 冻结条件：出现目录 schema 升级、credential 语义反转、或需要新依赖；冻结时保留 custom 现状
- 当前状态：冻结评估；不在首试点内

### 闭包 P3：Session、持久化、RPC、SDK、ACP 小增量互补区（冻结评估，未启动）

- 动机：semantic-map 显示 `src/session.rs`、`src/sdk.rs`、`src/acp.rs`、`src/rpc.rs` 一带有互补空间；`feature-matrix.md` E 系多项判值得但需设计
- 依赖：P1；与 P2 无硬依赖，但 credential 与 session 语义要一起看
- custom 接入边界：
  - custom 的持久化、Windows 竞争重试、自定义 RPC 端点是必须保留的二开语义
  - 上游新增端点与 custom 新增端点可能重名：重名必须逐个确认行为，不自动覆盖
  - `src/rpc.rs` 与 `src/session*.rs` 是 Hub 注入高发区（探针 Tier1），本闭包只收小增量，Hub 相关大段不收
  - 当前剩余 33 中 `session.rs`、`session_sqlite.rs`、`session_picker.rs`、`rpc.rs`、`sdk.rs` 带标记，属本闭包候选观察面（未读块，不下结论）
- 验证：
  - RPC 与 session 相关针对性测试（具体文件在试点 worktree 点名）
  - 确认 custom 端点与持久化语义回归通过
- 采纳/冻结：
  - 采纳：互补小增量，且 custom 语义完整保留（待验证）
  - 冻结：任何与 Hub/Jobs 编排耦合的变化移出本闭包，改入 P6 评估
- 当前状态：冻结评估；不在首试点内

### 闭包 P4：tools 结构映射（custom 已拆分 vs 上游单文件；冻结，未启动）

- 动机：custom 已把 `src/tools.rs` 拆成 `src/tools/` 目录；当前工作树 `src/tools/` 为目录形态 13 文件；直接 merge 会产生整文件级冲突或错误锚定，必须做映射迁移，不能自动合
- 依赖：P0、P1；需要试点 worktree 确认上游到底改了单文件的哪些段（当前剩余 33 无 `src/tools/` 下文件带标记，但有旧路径 `src/tools.rs` 在初始清单，映射仍待确认）
- custom 接入边界：
  - 默认保留 custom 目录结构；上游单文件改动必须映射到新结构，不整文件覆盖
  - `Tool::execute` 参数数量、abort 信号等 trait 形状变化统归本闭包评估，不在本闭包之外顺手改（按 `dependency-matrix.md` 未知 9，首个 check 错误数可判定）
- 验证：
  - 结构映射表（上游段落到 custom 新路径的对应关系；待试点产出）
  - 受影响工具的针对性测试（点名待定）
- 采纳/冻结：
  - 默认冻结：除非映射表完整且验证通过，否则整个上游 tools 增量冻结，保留 custom 现状
  - 不得因为单个工具看起来独立就单独 cherry-pick
- 当前状态：冻结；不在首试点内

### 闭包 P5：extensions 与 extensions_js 重构面（冻结，未启动）

- 动机：已知最高风险区；`feature-matrix.md` G8 判冻结留待扩展专题；`overlap-risk-matrix.md` §4 判最高风险
- 依赖：P0、P1；与 P4 同属结构面，但独立决策
- custom 接入边界：
  - 默认保留 custom 结构与语义；上游新文件默认不自动带入
  - 即使试点显示某些子文件无冲突，也必须先判断新架构能否承载 custom 定制，再谈采纳
  - 当前剩余 33 中 `extensions.rs`、`extensions_js.rs` 带标记，`src/extensions/` 目录不存在，属本闭包观察面（未读块，不下结论）
- 验证：
  - 承载性判断记录（能承载/不能承载/待迁移设计；待试点产出）
  - 扩展相关针对性测试（点名待定）
- 采纳/冻结：
  - 默认冻结：本闭包在 C1 的默认结论是冻结，不是采纳
  - 采纳只允许在有独立迁移设计并验证通过后发生，不在试合并中顺手完成
- 当前状态：冻结

### 闭包 P6：Hub/Jobs/subagents 编排闭包（仅评估，不自动跟随；冻结，未启动）

- 动机：探针显示上游 subagent 已演进为 hub 史诗；`feature-matrix.md` B3 判冻结留待 C2 后再定；custom 已有 hub 全闭包 A1
- 依赖：P0、P1、P3；需要先知道 C1 到底带入了多少 hub 相关新增文件（当前剩余 33 中 `hub.rs`、`jobs.rs`、`agent_hub.rs`、`subagents.rs`、`worktree_iso.rs`、`secrets.rs` 均带标记，属本闭包观察面，未读块）
- custom 接入边界：
  - custom 有 `touched_files`、asupersync runtime、交互层定制，与 hub 注入点正面重叠
  - 只做最小闭包评估：识别 hub 最小文件集合与注入点位置，不做全量合流
  - trait 形状变化优先用适配层思路评估，不批量改 20+ 实现
- 验证：
  - 新增文件清单与注入点清单（只记录，不合入）
  - 依赖增量清单（例如新增 crate 需求只记录，不 `cargo add`）
- 采纳/冻结：
  - 默认冻结：C1 阶段对 Hub/Jobs 的默认结论是冻结或延期，不采纳
  - 采纳只允许走独立的最小移植项目，不在 C1 integration 线上直接完成
- 当前状态：冻结

### 闭包 P7：运行时与重型依赖（FrankenSQLite、rquickjs/SWC/wasmtime、FTUI 等；冻结，未启动）

- 动机：这类依赖一旦跟随，会连带 Rust 工具链要求、构建脚本、C 依赖、体积预算一起漂移；探针与 SOP 都确认这是高成本项
- 依赖：P1；独立于 P2–P6 的语义闭包
- custom 接入边界：
  - 一律不得自动跟随；每个重型依赖单独评估（构建约束、体积、平台支持、custom 是否真的需要该能力）
  - Windows MSVC 构建约束必须在评估中显式记录
  - 当前剩余 33 中 `session_sqlite.rs` 带标记但不等同 FrankenSQLite 已出现；rquickjs/SWC/wasmtime/FTUI 是否在 C1 出现仍以试点复核为准，不预判
- 验证：
  - 只记录依赖增量与构建影响面；本计划阶段不运行构建
- 采纳/冻结：
  - 默认冻结：C1 阶段全部冻结
  - 任何采纳必须有单独的适配计划与验证计划，不在 C1 顺手决定
- 当前状态：冻结

## 4. 推荐顺序与理由（已收敛）

推荐顺序：

- P0 收尾 → P1 基线确认 → LSP 独立移植试点 → 其余按需排队（ast 第二、P2/P3 小增量随后、P4/P5/P6/P7 保持冻结评估）
- 不做 C1 全量 merge；当前 `integration/c1` 只到冲突地图，不解 33 个剩余冲突

理由：

- P0 最先：不剥离噪音，后续一切计数都是错的；确定性部分已完成，剩余 33 需实测复核后定稿
- P1 其次：依赖基线不定，所有试点验证都可能是误报；SOP 明确依赖漂移大于源码冲突
- 首试点选 LSP：
  - 闭包完整：`src/lsp/` 五件套（`client.rs`、`registry.rs`、`jsonrpc.rs`、`text.rs`、`edits.rs`）加 `src/lsp.rs`，可独立说清动机与验证门
  - 重叠低：`feature-matrix.md` A1 与 `overlap-risk-matrix.md` §2 均判 custom 在此域基本无结构性定制，冲突预期为白拿或自动合入
  - 接入轻：不牵动 `src/tools/` 结构与 `Tool::execute` arity，不碰 Hub 注入点与会话存储格式
  - 验证门清晰：LSP 初始化握手与 eval 空内核加载为通过门，针对性跑单模块，不跑全量
- 矩阵反例说明：
  - `overlap-risk-matrix.md` §13 推荐 ast 优先于 LSP，理由是 ast 为全新零冲突而 LSP 有 QuickJS 沙箱策略间接关联
  - 本计划仍首选 LSP：ast 需先解工具注册与 SWC/分级 schema 前置（`feature-matrix.md` 跨域小结：A 全家依赖 G6 未定则形态不稳定），试点前置成本高于 LSP；ast 作为第二试点，不丢弃
- 试点载体：建短路径干净 worktree，避免继承当前 integration 工作树 33 冲突与 Windows 长路径；以 `git show` 单域评估，不走 `merge upstream/main`
- P4/P5/P6/P7 靠后且冻结：结构面与编排与重型依赖必须单独设计，先把冻结结论定下来再看是否值得投入

如果试点 worktree 显示 LSP 实际为空或前置依赖过大，直接标记并切换到 ast 第二试点，不为了首选而硬上。

## 5. 临时 integration 线的定位

- 临时 integration 线只做冲突地图，不作为 `custom` 落点
- 落点只能是 `custom`，且必须在试点有结论、验证通过后，以受控方式合入；本计划不描述合入命令，只定门槛
- integration 线上允许存在未解决冲突、试验性取舍与 `--ours` / `--theirs` 中间态；这些中间态不得被当作 custom 的决策结果引用（当前 33 即中间态）
- 每次试合并必须在 worktree 中进行，不污染主仓库；主仓库 `custom` 在试点结论出来前保持不动
- 试点另起短路径干净 worktree，不复用当前 `C:\Users\m\AppData\Local\Temp\pi-c1-integration` 的冲突态；只复用手法，不复用分支本身
- integration 线不做长期分支，不跨 C1 带到 C2；C2 开始前重新评估是否复用手法，不复用分支本身

## 6. 不得自动跟随的高风险项（现阶段仍冻结）

以下项目在 C1 一律不得自动跟随，无论 Git 是否显示无冲突或自动合入：

- Hub/Jobs/Subagents 编排体系（含 `hub.rs`、`jobs.rs`、`agent_hub.rs`、`subagents.rs` 及相关注入点）
- extensions / tools 结构重构（含 `src/extensions/` de-monolith、`src/tools/` 拆分映射、`extensions_js` VFS 语义）
- FrankenSQLite 相关（`fsqlite`、FTS、native 依赖与持久化语义）
- rquickjs / SWC / wasmtime 相关运行时与工具链升级
- FTUI / FrankenTUI 相关交互默认路径切换
- 任何 Rust 工具链版本要求提升、asupersync 大版本升级、digest / fs4 等主依赖升级
- 发布管道与证据门（embedded_assets、LZSS、性能 claim、release hardening）
- 上游机器人、发布自动化、Dependabot 相关配置（按 Fork 政策，本来就不带入）

自动合入不等于已决策。无冲突合入的上述文件必须人工回查，必要时回退到 custom 基线。

## 7. 每个试点（闭包）的固定记录格式

每个试点结论必须包含以下条目，缺一即视为未完成：

- 试点名称与覆盖的上游范围（提交主题聚类或文件清单；LSP 首试点为 `src/lsp.rs` 与 `src/lsp/` 五件套）
- 动机：一句话说明为什么值得看
- 依赖：直接依赖的闭包与外部依赖增量（按 `dependency-matrix.md` 类别 A/B 标注）
- custom 接入边界：保留 custom 的区域、采纳 upstream 的区域、需要融合的区域
- 验证：针对性命令与结果（本计划阶段只列命令，不执行；执行结果由后续验证记录补）
- 采纳/冻结/延期：三选一，且冻结必须写明解冻条件
- 风险与未知：仍不能判断的事项，移入矩阵确认清单

## 8. 停止条件

出现以下任一情况，C1 停止并冻结，不进入下一个试点，不进入 C2：

- 需要大规模结构迁移但尚未形成设计（尤其 P4、P5、P6）
- 测试失败无法区分是 custom 基线自带还是本阶段引入
- 依赖升级导致大面积错误且没有适配计划
- 上游机器人或发布自动化违反 Fork 政策
- 模型需要读取整个仓库或全部 317 提交才能继续判断
- 冲突地图显示 C1 实际规模远超 checkpoint 记载且无法归因
- 针对性验证命令无法确定（不知道该跑什么即停止，不盲跑全量）

C1 完成必须同时满足：

- Git 无未解决冲突（以试点与 integration 线实测为准；当前 33 未清零，不满足）
- 针对性测试与必要的静态检查通过（结果有记录，不是口头通过；当前无结果，不满足）
- 无未解释的依赖漂移（当前 `Cargo.toml`/`Cargo.lock` 带标记，不满足）
- Fork 政策文件没有被上游自动化无条件覆盖
- 每个非空试点都有保留、采纳、冻结三类决策记录（当前无试点结论，不满足）

## 9. 仍需试点确认的事项（本计划不预判）

- C1 冲突地图定稿：`git diff --name-only --diff-filter=U` 清单、`git status --short` 的冲突/新增类别统计、白拿新增文件清单（当前 33 为标记扫描推导，需实测复核）
- P2、P3 是否非空：取决于 C1 是否包含 provider/session/RPC 增量，不能按 semantic-map 的旧基线断言（当前带标记文件只证明冲突存在，不证明语义）
- Hub/Jobs 在 C1 的形态：是完全不存在、雏形、还是已具规模，必须由试点 worktree 确认（当前带标记只证明文件级重叠，不证明规模）
- FrankenSQLite、FTUI、rquickjs/SWC/wasmtime 在 C1 是否已经出现，必须由试点 worktree 确认（当前剩余 33 不能直接等同出现）
- custom 基线本身是否全绿：试点前必须先确认 `custom` HEAD 的基线验证状态，否则无法归因
- 每个试点的针对性验证命令：必须在试点 worktree 点名，不能提前写死
- LSP 首试点前置检查：`src/lsp/` 是否引用新 crate、是否触 QuickJS 沙箱策略、是否依赖 G6 分级 schema；任一不满足则切换到 ast 第二试点
