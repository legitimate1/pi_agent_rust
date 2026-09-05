# C1 入口：v0.3.0 上游累计状态

> 范围：只覆盖 C1。不修改源码，不解 merge 冲突，不代表任何功能已迁移。

## Checkpoint

- 锚点：`v0.3.0`，提交 `e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`
- 性质：release 锚点，上游第一段累计可验证状态
- 相对共同基线 `226a876425a856f657b2a5d7c7ac6f0ca1ad25f1`：317 commits，其中 310 个非 merge
- 区间规模：351 files，118 个源码文件（`src/` 与 `crates/` 口径）
- 行变化：+93,550 / -4,141（只表示区间大小，不表示语义冲突量，不表示模型上下文大小）
- 状态：试合并进行中（`integration/c1` 已创建，`custom` 未动，无任何功能迁移完成）

## integration 线实测事实

- 当前分支：`integration/c1`（以 worktree 的 `HEAD` 为准，内容为 `ref: refs/heads/integration/c1`）
- 合并对端：`MERGE_HEAD` 为 `e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`，`ORIG_HEAD` 为 `d1786d8b9658ac2553a1a46a3bca5414b1bf9b67`，`MERGE_MODE` 为 `no-ff`
- 试合并工作树：`C:\Users\m\AppData\Local\Temp\pi-c1-integration`
- 初始未解决路径：82 个（以 `MERGE_MSG` 中 `# Conflicts:` 清单计数为准）
- 确定性规则已处理：
  - `.beads/` 已移除（工作树 `ls` 无该目录；初始清单中 4 个 `.beads/` 路径已移出视野）
  - `custom` 已删除冻结文件与部分生成物/测试已按保留 custom 或移出处理（初始清单中 `docs/` 证据、`tests/perf/reports/`、`tests/ext_conformance/reports/`、`src/extensions/tests/`、`src/tools.rs` 旧路径等不再带标记）
  - 未读任何冲突块全文，未解任何语义冲突
- 当前剩余：33 个带冲突标记文件（以标记扫描为准；工作树状态变化时以实测为准）
  - 扫描方法：无 shell 入口，未执行 `git diff --name-only --diff-filter=U` 与 `git status --short`；改用 `MERGE_MSG` 计数初始值，加全树 `<<<<<<< HEAD` 标记扫描推导剩余数；`src/commit_split.rs` 与 `tests/commit_split.rs` 内字符串字面量与 `docs/upstream/probe-report-2026-08-29.md` 内命令引用已排除，不计入冲突
  - 剩余类别：根配置 2 个（`Cargo.toml`、`Cargo.lock`）；示例 1 个（`examples/pi_debug.rs`）；`src/` 30 个（`agent.rs`、`agent_hub.rs`、`app.rs`、`auth.rs`、`cli.rs`、`compaction.rs`、`compaction_worker.rs`、`config.rs`、`extensions.rs`、`extensions_js.rs`、`hub.rs`、`interactive/commands.rs`、`interactive/tests.rs`、`jobs.rs`、`lib.rs`、`main.rs`、`models.rs`、`package_manager.rs`、`perf_build.rs`、`providers/bedrock.rs`、`providers/model_fetch.rs`、`rpc.rs`、`sdk.rs`、`secrets.rs`、`semantic_workspace_graph.rs`、`session.rs`、`session_picker.rs`、`session_sqlite.rs`、`subagents.rs`、`worktree_iso.rs`）
  - 结构观察（仅 `ls`，未读块）：`src/lsp/` 5 文件存在；`src/tools/` 为目录形态（13 文件）；`src/extensions/` 目录不存在（仍为单文件 `extensions.rs` 形态）；`src/eval/`、`src/debug/`、`src/hub.rs`、`src/jobs.rs`、`src/subagents.rs` 存在
- 临时 integration 线只作冲突地图，不作为 `custom` 落点

## 分析边界

- 只依据以下已定材料：`docs/upstream/README.md`、`merge-checkpoints.md`、`fork-merge-sop.md`、`semantic-map.md`、`probe-report-2026-08-29.md`，以及本目录三张矩阵
- C1 功能分组已由 `feature-matrix.md` 从 CHANGELOG 归纳重建，不再是占位；但未读 317 提交全文，未拉取 `.beads` issue 全文，未做三方冲突归因
- 不把文件数、行数当作冲突数
- 不把测试快照、证据包、生成物的差异当作语义差异
- 模型每次只允许接触：当前 checkpoint 范围摘要、Git 实际冲突文件和冲突块、相关 custom 二开意图、首个编译或测试失败、已确认的短决策摘要
- 不运行 `cargo`，不做任何源码改动

## 矩阵链接

- 根目录入口与规则：
  - `docs/upstream/README.md`：上游同步工作区当前入口，含 `main` / `custom` / integration 线关系与 Fork 政策
  - `docs/upstream/merge-checkpoints.md`：C1–C6 checkpoint 定义、区间规模、每阶段停止条件、模型上下文边界
  - `docs/upstream/fork-merge-sop.md`：改造式 Fork 合并 SOP，含试 merge 流程、冲突分类、依赖漂移陷阱、冻结清单模板
  - `docs/upstream/semantic-map.md`：2026-08-16 的 52 文件交集决策底稿，用作候选风险索引，不是当前精确冲突清单
  - `docs/upstream/probe-report-2026-08-29.md`：全量试合探针归档结论（121 冲突归零过程、297 编译缺口分类、本次不全量合流、改走最小闭包移植）
- C1 本目录新增三张矩阵：
  - `feature-matrix.md`：C1 功能全景矩阵（v0.3.0 锚点，只分析不合入；A–G 域归纳与采纳/暂缓/冻结初步建议）
  - `dependency-matrix.md`：C1 依赖与 API 漂移矩阵（只分析不改源码；类别 A/B 二分与默认 `--ours` 策略）
  - `overlap-risk-matrix.md`：C1 重叠与冲突风险矩阵（含 Hub/扩展/tools/会话/TUI/RPC/providers 分域风险与首个低风险候选讨论）
- C1 本目录：
  - `integration-plan.md`：C1 综合移植计划正文（P0–P7 结构，状态已按 integration 实测校准；推荐顺序已收敛到独立移植试点）
  - 本文件：只做入口，不重复正文的闭包细节

## 状态

- C1 试合并进行中：`integration/c1` 已创建，初始 82 路径，按确定性规则处理后剩余 33 个带标记文件；无验证结果，无任何功能迁移完成
- 本目录文档只记录计划、约束与实测校准，不表示 C1 已经合入 `custom`
- 临时 integration 线即使推进，也只做冲突地图，不作为 `custom` 落点（详见 `integration-plan.md`）
- 高风险项现阶段仍冻结：Hub/Jobs/Subagents 编排、extensions/tools 结构重构、FrankenSQLite、rquickjs/SWC/wasmtime、FTUI、主依赖和工具链升级

## 决策规则

- 挑着合，不一次吞；C1 不做全量 merge
- cherry-pick 不可用；merge 是载体，但只做选择性采纳
- 冲突处理优先级（按 SOP 与语义地图）：
  - 生成物、测试快照、证据包、内部数据：批量保留 custom 或删除，不手工逐个解
  - 上游内部数据（例如 `.beads/` 类噪音）：删除，不带入
  - 根配置与依赖：人工合并，默认保留 custom 基线，依赖升级必须有适配计划才跟进
  - 语义重复：留一，默认优先上游官方实现，除非 custom 有上游没有的独特价值
  - 语义重叠：逐冲突人工解，以保留 custom 语义为主，融合上游小增量
  - 结构性差异（文件拆分、目录重构）：不自动合，必须单独做迁移设计
- 依赖漂移优先于源码冲突处理：`Cargo.toml` / `Cargo.lock` 的自动合入最危险，无冲突合入也必须复核
- Fork 政策是硬约束：`main` 不运行上游机器人；不得把会自动运行的上游 bot 配置或相关自动化无条件带入；`.github/dependabot.yml` 保持禁用；未经用户明确授权不启用上游 Dependabot 配置
- 每个闭包必须记录保留、采纳、冻结三类决策，否则该闭包不算完成

## 下一步

- 不做 C1 全量 merge
- 先选一个完整功能闭包做独立移植试点，首选 LSP（详见 `integration-plan.md` 第 4 节；矩阵反例说明：`overlap-risk-matrix.md` §13 认为 ast 风险更低，LSP 仍首选的理由见该节）
- 建短路径干净 worktree，避免继承当前 integration 工作树冲突与 Windows 长路径
- 每个试点走“动机 → 依赖 → custom 接入边界 → 针对性验证 → 采纳/冻结”五段并留记录
- 任一停止条件触发即停，不进入 C2
- C1 全部闭包有结论且验证通过后，才讨论 C2；否则冻结或延期
