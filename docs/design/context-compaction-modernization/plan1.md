# Phase 1 — Worker 生命周期与 stale-result 安全实现计划

> Build Target: `context-compaction-modernization/phase-1/worker-lifecycle-origin-safety`
> 来源设计：`docs/design/context-compaction-modernization/design.md`
> 计划总览：`docs/design/context-compaction-modernization/README.md`
> 当前步骤：Phase 1
> 当前任务记录：待用户确认

## 计划身份

- 计划目的：修复 `AgentSession::maybe_compact` 后台 compaction worker 的任务生命周期和结果归属，不改变摘要算法、token 估算或压缩模式。
- 目标项目：`C:\Users\m\Project\pi_agent_rust`
- 设计权威：`docs/design/context-compaction-modernization/design.md`，G1、G2、G2a、G6、事件终态矩阵与 I1–I4、I11–I13。
- 计划权威范围：只决定 AgentSession 自己拥有的后台自动压缩路径、其直接 provider/model transition、同步 compact 前 invalidation、generation API 和对应测试；不决定 RPC/interactive session replacement、fork/resume/tree surface 的完整接入，也不决定后续 local fallback、BPE、shake 或 snapcompact 的公共契约。

## 实现边界

### 本次目标

完成后应观察到：

1. `AgentSession::maybe_compact` 启动的后台任务带有启动时的 session/provider/model/leaf origin 和 generation；
2. stale、缺失 origin、generation mismatch 或已失效 context 的结果不会写入当前 AgentSession session；
3. timeout、abort、panic 和 provider error 都不会产生 compaction entry，且错误发布前任务已经 quiescent；
4. worker 自己执行 timeout，`try_recv` 只负责非阻塞回收 quiescent 结果；
5. attempt count 在启动时递增，但只有 apply contract 判定为成功后才重置；
6. apply/persistence failure 不会错误重置 quota，并明确报告内存/持久化状态；
7. Phase 1 覆盖的 AgentSession context switch、同步 compact 和 provider/model transition 可以取消并失效 pending worker，同时重置该 AgentSession 的 worker generation/quota；
8. RPC/interactive 的 session replacement、fork/resume/tree surface 不在本 Phase 的全覆盖承诺内，但可复用的 invalidation/generation API 已形成；
9. 默认摘要内容和当前 compaction entry/replay 格式不变；每个后台任务最多产生一个 terminal compaction event。

### 本次包含

- `CompactionOrigin` 和 pending task origin 存储；
- AgentSession 作用域内单调递增的 `compaction_generation`/失效代数；
- worker 的 bound result receive API；
- worker 内部 timeout、abort quiescence、panic 转换；
- `mark_applied_success` 或等价的 apply-contract 成功后状态 API；
- AgentSession 当前 compaction origin 构造、generation 比较和 invalidation API；
- stale/generation mismatch 的事件和丢弃路径；
- Phase 1 明确覆盖的 provider/model transition、同步 compact 前 invalidation；
- apply/persistence failure 的状态分类、quota 保持和回归测试；
- worker、AgentSession 与 event terminal matrix 的回归测试。

### 本次不包含

- RPC/interactive 的 session replacement/fork/resume/tree/navigation 完整接入；
- 通过业务 origin 四元组猜测 ABA 安全；generation 是本 Phase 的必需部分；
- BPE/tiktoken 或 `src/token_count.rs`；
- `compact_local`、2x window gate 或 deterministic fallback summary；
- `compact_shake`、shake-first、aggressive mode；
- snapcompact、MediaContent、图像 replay；
- provider admission permit / 全局并发协议；
- 改变 `CompactionDetails` 的持久化 schema；
- 改变 `Session::to_messages_for_current_path()` 的 replay 规则；
- 全量上游 `agent.rs`/`rpc.rs`/`session.rs` 移植。

### 前置条件

- 设计文档和本总览获用户确认；
- 当前工作区的预期外删除状态不属于本计划，不得顺手恢复或删除；
- 实现前重新读取当前 `src/compaction_worker.rs`、`src/agent.rs` 的相关符号，确认本计划中的行号只作导航而非编辑锚点。

## 当前代码事实

- `src/compaction_worker.rs:169-173`：`PendingCompaction` 目前只有 join、abort sender、started_at，没有 origin 或 generation。
- `src/compaction_worker.rs:279-306`：`try_recv()` 同时负责 timeout 判断和结果 join。
- `src/compaction_worker.rs:308-342`：`start()` 直接 spawn `run_compaction_task`，只在启动时增加 attempt count。
- `src/compaction_worker.rs:382-415`：provider future 有 panic catch，但 worker timeout 不在 task 内执行，abort 后没有保证 join/quiescence 后再发布 timeout/abort。
- `src/agent.rs:3523-3546`：`AgentSession` 持有 compaction worker、session、agent 和 compaction runtime；Phase 1 需要增加 generation/invalidation 状态，但不替换外层 session surface 的生命周期。
- `src/agent.rs:9517-9657`：`maybe_compact()` 应用 worker 结果、准备新 compaction、dispatch hook，然后启动 worker；这是 Phase 1 的主要生产调用路径。
- `src/agent.rs:9755-9791`：`apply_compaction_result()` 将结果转换为 details，并调用统一 session append/autosave 路径；当前没有显式区分 durable apply 与 persistence failure。
- `src/agent.rs:9168-9449`：当前 runtime provider/model 切换路径；Phase 1 只核对并接入 AgentSession 直接 transition，不宣称覆盖 RPC/interactive 的 session replacement/fork/resume/tree surface。
- `src/session.rs:2552-2554`：session header 有稳定 session id。
- `src/session.rs:3327`：已有 `Session::leaf_id()` 只读 accessor，可用于 origin。
- `src/session.rs`：现有 session append/autosave/replay 责任不在本计划中改变；持久化失败语义必须通过测试暴露，而不是新增旁路 writer。

## 不变量映射

### I1 — session entry 是唯一 Pi-owned session 写入路径

- 设计来源：G6、I1。
- 代码落点：`src/agent.rs::apply_compaction_entry`、`apply_compaction_result`、`src/session.rs::append_compaction`。
- 证据测试：现有 `apply_compaction_result_emits_structured_result_payload` 及 session compaction persistence 测试；Phase 1 新增 stale/timeout entry-count 断言。
- 本次保护方式：只在 origin 验证通过后调用现有 apply；不新增 worker/RPC 直接写 session。
- 失败处理：origin 不匹配或 worker outcome 非成功时丢弃/报告，不追加 entry。

### I2 — stale background result 不能写入当前 AgentSession

- 设计来源：G1、I2、I11。
- 代码落点：`src/compaction_worker.rs::CompactionOrigin`、generation state；`src/agent.rs` 的 origin/generation compare、invalidation 与 `maybe_compact` result handling。
- 证据测试：session id 变化、leaf 变化、provider/model 变化、generation mismatch，以及“切走后切回相同四元组”的 ABA 测试。
- 本次保护方式：应用前做 origin + generation 全字段匹配；不匹配时 terminal event 标记 stale/aborted。
- 失败处理：等待任务 quiescent 后丢弃结果；不重置 attempt count；不重试旧 origin。

### I3 — pending/timeout/abort/panic/provider error 不产生部分 entry

- 设计来源：G2、G2a、G6、I3。
- 代码落点：`run_compaction_task`、worker bound receive、AgentSession result handling 和 apply outcome classification。
- 证据测试：never-completing provider timeout、abort flag、panic provider、provider error、apply rejected、persistence failure 后 session compaction count/replay 状态断言。
- 本次保护方式：任务结束并返回错误前完成 quiescence；Agent 只对 origin 匹配且 apply contract 成功的结果进入 quota reset。
- 失败处理：按事件矩阵报告；persistence failure 不吞错，也不伪装成“未发生”。

### I4 — attempt quota 只在 apply contract 判定成功后重置

- 设计来源：G2、G2a、I4、I12。
- 代码落点：`src/compaction_worker.rs::mark_applied_success`；`src/agent.rs` 在 apply outcome 判定为成功后调用。
- 证据测试：provider task success 但 origin stale、apply reject、save/flush failure 时 attempt 不变；正常 apply 成功后 attempt 归零；session switch invalidation 后 generation 和 quota reset。
- 本次保护方式：禁止在 `try_recv`、provider future 完成或 stale result 丢弃时重置。
- 失败处理：持久化失败按 apply contract 报告并保留 quota；如果当前实现无法区分内存 mutation 与持久化失败，暂停该部分。

### I11 — generation 失效优先于业务 origin 四元组

- 设计来源：G1、I2、I11。
- 代码落点：`AgentSession` generation、worker pending origin、context-switch invalidation API。
- 证据测试：同一 session/provider/model/leaf 切换后再切回时，旧结果仍被拒绝。
- 本次保护方式：generation 单调递增，不能从 session entry replay 推导，也不能使用时间戳替代。
- 失败处理：generation 无法可靠递增或跨 transition 传递时暂停 origin 安全实现。

### I12 — persistence failure 不得伪装成 durable success

- 设计来源：G2a、I12。
- 代码落点：`apply_compaction_entry`/`apply_compaction_result` 的 apply outcome 与现有 autosave/flush。
- 证据测试：持久化失败后的 entry/replay/错误状态和 quota；save-disabled 模式单独标注非 durable。
- 本次保护方式：只把明确成功的 apply outcome 交给 `mark_applied_success`。
- 失败处理：必须报告状态不确定或已隔离；不得通过简单返回 `Ok` 掩盖 flush failure。

### I13 — 每个后台任务最多一个 terminal compaction event

- 设计来源：事件终态矩阵、I13。
- 代码落点：`maybe_compact`、worker outcome 分类和 `extensions_is_compacting` guard。
- 证据测试：正常、stale、abort、timeout、provider error、panic、apply reject、persistence failure 各一条 terminal event。
- 本次保护方式：worker result 消费和 stale/invalidation 分支共享终态收口。
- 失败处理：发现重复 terminal event 或 flag 泄漏时停止，不扩大到 Phase 2。

## 当前工作单元

本计划只包含一个工作单元，因为 worker API、AgentSession orchestration 和它们的回归测试共享同一个生命周期契约和回滚边界。

### W0 — 后台 compaction 生命周期与结果归属收口

- 目标：Phase 1 覆盖的 AgentSession worker 结果只对仍匹配的当前上下文生效，并具有可证明的 generation、timeout/quiescence、abort/panic、apply/persistence/quota 语义。
- 前置依赖：设计确认；无其他实现 Phase。
- 可修改范围：
  - `src/compaction_worker.rs`：`PendingCompaction`、`CompactionOrigin`、`CompactionWorkerState`、generation-aware result、`run_compaction_task`、测试辅助。
  - `src/agent.rs`：AgentSession generation/invalidation、origin 构造/比较、`maybe_compact`、Phase 1 直接覆盖的 provider/model transition、同步 compact 前 invalidation、apply outcome 和事件收口、相关测试。
  - 必要时 `src/session.rs`：仅增加缺失的只读 accessor；当前已知 `header.id` 和 `leaf_id()` 足够时不修改。
- 明确不覆盖：RPC/interactive 的 session replacement、fork/resume/tree/navigation 全路径；Phase 1 只提供后续 surface 可复用的 invalidation/generation API。
- 禁止扩大：
  - 不修改 compaction summary 文本或 token estimator；
  - 不增加新的公共配置项；
  - 不改变 session entry JSON 结构；
  - 不引入 provider admission gate；
  - 不重构 RPC event handler 或整个 session transition 系统。
- 相关不变量：I1–I4、I11–I13。
- 验证：worker lifecycle/filter tests；AgentSession apply/stale/event tests；`cargo fmt --check`；`cargo clippy --lib -- -D warnings`。
- 完成条件：所有 W0 测试通过；generation/origin stale outcome 不写 entry；timeout/abort 发布前 task quiescent；apply/persistence failure 不错误清 quota；session switch invalidation 重置 generation/quota；每个任务只有一个 terminal event；默认 compaction tests 不回归；未产生计划外文件或公共 schema 变化。
- 并行关系：无。worker API、AgentSession orchestration 和 apply/event 语义必须顺序协同修改；不并行编辑同一文件。

#### W0 / S1 — 先固定生命周期、generation、apply 和事件契约测试

- 增加/调整测试 double，使 provider 可以：立即成功、返回错误、永久 pending、panic。
- 固定以下测试预期：
  - worker 内部 timeout 不依赖 `try_recv` 轮询，且 timeout/abort 错误发布前 provider/task 已 quiescent；
  - timeout/abort/panic/provider error 不产生 session compaction entry；
  - 同一 session/provider/model/leaf 切走后切回时，旧 generation 结果仍被拒绝；
  - 任务成功只是“生成成功”，不会立即重置 attempt count；
  - apply rejected、persistence failure 或 stale result 不重置 quota；
  - apply contract 判定成功后才重置 quota；
  - 每个 outcome 只发送一个 terminal `AutoCompactionEnd`，且 `extensions_is_compacting` 最终为 false；
  - session switch/invalidation 后 generation 递增、pending 被取消、该 AgentSession worker 的 attempt/cooldown 状态按契约重置。
- 先固定 persistence failure 的测试状态：至少区分 `RejectedBeforeMutation`、`AppliedDurably`；如果当前实现会产生 `PersistenceFailedAfterMutation`，必须断言 entry/replay 状态已被明确报告，不能把它当作 no-op。
- 若当前测试辅助无法注入 apply/persistence failure，优先增加最小的 AgentSession test seam，不改变生产公共 API。

#### W0 / S2 — 扩展 worker state、generation 和 quiescent bound receive

- 为 pending task 增加 `CompactionOrigin`，包含业务 origin 四元组和 captured generation。
- 为 `AgentSession` 增加单调递增的 compaction generation；invalidation 不能通过恢复旧值或重新读取 session entry 来回退。
- 将 `try_recv` 内的 timeout 责任迁移到 `run_compaction_task` 或等价 worker-owned future。
- 增加 bound receive，返回 `(origin, outcome)`；只有 task quiescent 后才允许发布 outcome。
- timeout/abort 路径必须等待 provider future、wrapper task 和取消相关资源结束；如果 runtime API 无法证明 quiescence，停止继续实现并返回设计阶段。
- panic 转换为稳定的 session/worker error，不输出 provider secret。
- 增加 `mark_applied_success()`，只允许 Agent 在 apply contract 判定成功后调用。

#### W0 / S3 — AgentSession origin/generation 校验、invalidation 与 apply outcome

- 在启动 worker 前，从同一 AgentSession 的 session header、leaf、active provider/model 和 generation 创建 origin；计划执行时必须核对读取顺序和锁边界，避免 Agent provider 与 Session header 不一致时生成混合快照。
- worker result 应用前重新读取同一来源的当前 origin/generation；全部匹配才调用 `apply_compaction_result`。
- stale/generation mismatch：
  - 等待/确认 worker 已 quiescent；
  - 不调用 session append；
  - 发送事件矩阵规定的 stale terminal event；
  - 不执行 `mark_applied_success()`。
- 对 Phase 1 明确覆盖的 synchronous compact、AgentSession provider/model transition 和 context invalidation 入口，递增 generation、取消 pending、重置该 AgentSession worker quota。
- 不把 RPC/interactive session replacement、fork/resume/tree/navigation 标记为已覆盖；这些 surface 只在后续阶段接入当前 API。
- 明确 `apply_compaction_entry` 的 outcome：成功 durable apply、save-disabled in-memory apply、before-mutation rejection、persistence failure after mutation；只有允许 quota reset 的成功 outcome 才调用 `mark_applied_success()`。
- 保持 `extensions_is_compacting` 在取消、stale、错误和正常完成时最终复位；优先沿用 RAII guard，避免 cancellation 泄漏 true 状态。
- 按事件矩阵收口所有 terminal path，避免 worker error、stale 分支和 apply error 重复发送 `AutoCompactionEnd`。

#### W0 / S4 — 集成验证与边界复核

- 运行 Phase 1 相关 lib tests。
- 运行 `cargo fmt --check` 和 `cargo clippy --lib -- -D warnings`。
- 检查 `git status --short`，确认只有计划允许的源文件和测试改动；不得删除或恢复用户已有的无关文件。
- 对照 I1–I4、I11–I13 和事件终态矩阵做一次实现/测试映射复核。
- 明确记录 Phase 1 未覆盖的 RPC/interactive/session replacement/fork/resume/tree 入口，作为 Phase 5 的输入。

## Checkpoint Protocol v1 — W0 执行协议

### 1. 协议目的与权威性

Checkpoint 是能够让新的 Main Agent 或新的 Worker 在不读取完整主会话的情况下，安全继续当前工作单元的最小持久化状态。它记录执行状态，不改变设计目标、工作范围、完成标准或公共契约。

Checkpoint 的权威层级如下：

```text
design.md
  ↓
README.md
  ↓
plan1.md 的范围、依赖与完成标准
  ↓
当前 canonical checkpoint
  ↓
Worker 的自然语言交接
```

Main Agent 是 canonical checkpoint 的唯一写入者。Worker 只返回角色专属交接结果，不直接关闭 W0 或 Phase 1。当前阶段的 checkpoint 直接维护在本文件“当前任务记录”中，不新增独立 checkpoint 文件；后续 Phase 使用各自的计划文件保存同类状态。

### 2. 身份与版本

每个 checkpoint 必须关联以下身份：

- `protocol: checkpoint-v1`
- `plan_revision`：当前计划/设计版本；设计、Phase 边界或完成标准改变时递增。
- `phase`、`work_unit`、`slice`：当前执行位置；本 Phase 使用 `Phase 1`、`W0`、`S1`–`S4`。
- `checkpoint_id`：推荐格式 `CCM-P1-W0-S<slice>-G<generation>-A<attempt>-C<seq>`。
- `checkpoint_kind`：`dispatch`、`continuation`、`blocked`、`failed`、`accepted` 或 `phase-gate`。
- `assignment_generation`：当前工作单元的语义代数；目标、范围、验收标准或上游方案改变时递增。
- `attempt`：同一 assignment 的执行尝试次数；失败重试或 session 重建时递增，不能用来替代 generation。
- `checkpoint_seq`：canonical checkpoint 的单调递增序号。
- `owner_role`：`main`、`worker` 或 `reviewer`。
- `hub_id`：当前可继续的 Agent session；不可用时允许从 checkpoint 重建。
- `external_task_ref`：只有使用 `agent-collab` 时填写 ticket/handoff 引用。

同一 assignment 的普通 continuation、上下文压缩或 hub session 继续不递增 `assignment_generation`。目标、范围或验收标准变化时，旧结果视为 stale，不得直接应用。

### 3. Checkpoint 内容

每个 canonical checkpoint 至少记录：

- 当前目标与本次允许/不允许的范围；
- 已冻结的上游决策；
- 已完成、正在进行、尚未开始的内容；
- 已尝试但失败的路径及原因；
- 事实、判断、未知及其代码/测试/命令证据；
- 预期修改文件、实际修改文件和计划外文件；
- 已运行、未运行和失败的验证；
- 阻塞原因及解除条件；
- 下一步唯一的首个动作；
- 恢复路由；
- 恢复时不应重复调查或重做的内容；
- Main Agent 的 implementation、verification、acceptance 三类验收状态。

Checkpoint 只保存结论、证据位置和下一步，不复制完整主会话、完整日志、完整 diff、无关工具输出、已否决的长篇推理、完整 prompt 或敏感信息。

### 4. 生命周期与写入时机

W0 按以下生命周期推进：

```text
dispatch
  → continuation / blocked / failed
  → candidate result
  → Main Agent 验收
  → accepted
  → phase-gate
```

必须在以下时机写入或更新 canonical checkpoint：

1. 派发一个新的 W0 切片前：`dispatch`；
2. 一个切片完成但 W0 未完成时：`continuation`；
3. 上下文不足、session 即将交接或需要暂停时：`continuation`；
4. 外部依赖、用户决策、环境或设计边界阻塞时：`blocked`；
5. 本次执行失败但尚未决定重试、换角色或重规划时：`failed`；
6. Main Agent 检查 Worker 交接、修改范围和证据后：`accepted`；
7. W0 全部完成并通过 Phase 1 验收时：`phase-gate`。

“Worker 已生成结果”不等于“结果已接受”；“实现完成”不等于“验证通过”；“W0 accepted”不等于“Phase 1 phase-gate”。

### 5. 恢复路由

按以下优先级恢复：

```text
原工作单元仍有效 + hub_id 可用
  → continue_same_hub

原工作单元仍有效 + hub_id 不可用，但 checkpoint 和代码状态完整
  → resume_from_checkpoint

代码已修改但只缺验证
  → verify_existing_changes

Worker 已产生候选产物但尚未验收
  → apply_candidate_result

阻塞条件尚未解除
  → wait_for_unblock

目标、方案、范围或验收标准发生变化
  → replan_before_continue

当前 Phase 的所有工作单元和验证均通过
  → close_phase
```

恢复时不得因 hub session 不可用而无条件重新调查或重做已完成部分。若只是暂时缺少外部服务、权限或环境，保留原 checkpoint，阻塞解除后继续原 assignment。若 assignment generation 已变化，旧 Agent 的晚到结果只能记录为 stale，不能写入当前结果。

### 6. W0 切片映射

W0 使用一个逻辑 Worker 工作单元，S1–S4 是上下文边界和恢复边界，不是四个可并行的独立工作单元：

```text
S1  固定生命周期、generation、apply 和事件契约测试
  → continuation checkpoint
S2  扩展 worker state、generation 和 quiescent bound receive
  → continuation checkpoint
S3  接入 AgentSession origin/generation/invalidation/apply outcome
  → continuation checkpoint
S4  集成验证、边界复核和 Phase 1 输入收口
  → phase-gate checkpoint
```

W0 不并行编辑同一文件。默认同时只有一个活动 Worker；优先继续同一个 `hub_id`。只有原 session 不可用时，才使用同一 assignment 从最近 checkpoint 重建 Worker session，并递增 `attempt`。

### 7. agent-orchestration 与 agent-collab 的使用边界

- 当前项目内的连续实现、切片交接和 hub session 恢复使用 `agent-orchestration`、本文件 checkpoint 和 W0 计划范围。
- 不为每个 S 切片创建独立 ticket；S1–S4 共享同一个生命周期契约和验收边界。
- 需要跨项目调查/实现、长期异步等待或当前 Main Agent 无法继续而必须持久化外部交接时，才使用 `agent-collab`。
- 使用 `agent-collab` 时，在 checkpoint 记录 `external_task_ref`；解除阻塞或收到结果后继续原 assignment，不创建重复任务。
- handoff/ticket 的自然语言报告是输入材料，不替代本文件中的 canonical checkpoint 和 Phase Gate。

### 8. Phase 1 Gate 与协议验收

只有同时满足以下条件，才能写入 `phase-gate`：

- W0/S1–S4 均有对应的 canonical checkpoint，且每个切片的范围和验收状态明确；
- stale/generation mismatch 不写入 session entry；
- timeout/abort 错误发布前能够证明 task quiescent；
- provider error、panic、timeout、abort 不产生 compaction entry；
- apply/persistence failure 不错误重置 quota；
- 正常成功路径仍追加 compaction entry；
- 每个后台任务最多一个 terminal event，`extensions_is_compacting` 最终为 false；
- 定向测试、`cargo fmt --check` 和 `cargo clippy --lib -- -D warnings` 通过；
- 没有计划外文件或公共 schema 变化；
- Phase 1 未覆盖的 RPC/interactive/session replacement/fork/resume/tree 入口已明确记录，作为 Phase 5 输入。

协议自身在当前阶段的冻结结果是：checkpoint-v1 已确认；实现、自动化验证和 Phase 1 执行授权仍按本计划的用户授权边界处理。


## 文件与符号变更清单

- C1 — `src/compaction_worker.rs`：修改
  - 符号：`PendingCompaction`、新增/规划中的 `CompactionOrigin`、`CompactionWorkerState`、generation-aware bound receive、`start`、`run_compaction_task`、测试辅助。
  - 当前/目标标记：上述 origin、generation-aware receive、`mark_applied_success` 是 planned symbols；当前只有 `PendingCompaction`、`try_recv`、`start`、`run_compaction_task`。
  - 用途：实现 worker-owned timeout、quiescence、origin/generation 携带、abort/panic join 和 apply-contract quota API。
  - 关键改动：增加任务实例失效代数；从“生成成功即可能清 quota”改为“apply contract 成功后清 quota”。
  - 预期保持：默认 cooldown/timeout/max attempts 数值不变；现有 signal-based compaction admission reason 语义不变；不新增全局 provider gate。

- C2 — `src/agent.rs`：修改
  - 符号：`AgentSession` generation/invalidation state、`maybe_compact`、planned origin compare helper、Phase 1 直接覆盖的 provider/model transition、同步 compact 前 invalidation、apply outcome/event helper、相关测试。
  - 当前/目标标记：`AgentSession`、`maybe_compact`、`apply_compaction_entry`、`apply_compaction_result` 是 current symbols；generation/origin/apply outcome/event helper 是 planned symbols。
  - 用途：将 AgentSession worker 结果与当前上下文绑定，并把 worker 状态更新放在 apply contract 成功之后。
  - 关键改动：`try_recv` 结果变为 origin+generation-aware；stale result fail-closed；apply/persistence failure 不错误清 quota；每个任务只发送一个 terminal event。
  - 预期保持：正常成功压缩仍追加一个 compaction entry；事件名称和现有基本 payload 字段不变；不宣称覆盖 RPC/interactive session replacement。

- C3 — `src/session.rs`：仅在必要时修改
  - 符号：现有 `Session::leaf_id()` 或最小只读 accessor。
  - 用途：提供无副作用的 session identity/leaf identity；不承担 generation 所有权。
  - 关键改动：优先不改；若编译/可见性阻塞，只增加最小只读方法和单测。
  - 预期保持：session JSONL、SQLite/sidecar、replay 和 autosave 行为不变。

- C4 — `src/compaction_worker.rs` / `src/agent.rs` 内测试区域：修改
  - 符号：worker lifecycle tests、quiescence tests、generation/ABA tests、AgentSession stale/apply/persistence/event tests。
  - 用途：将 I1–I4、I11–I13 和事件矩阵变成可重复的自动化证据。
  - 关键改动：增加 provider double、origin/generation matrix、entry/replay state、attempt count、terminal event count 和 flag reset 断言。
  - 预期保持：不依赖真实网络、不读取完整 CI 日志、不改变生产配置；明确标注 Phase 1 未覆盖的外层 session surfaces。

## 验证计划

### 工作单元验证

- W0/S1：运行 worker 与 AgentSession 的定向测试，证明生命周期、generation/ABA、apply outcome 和事件终态矩阵。
- W0/S2：运行 `cargo test --lib compaction_worker`，确认 timeout quiescence、abort、panic、origin/generation wrapper 和 quota 相关测试通过。
- W0/S3：运行 `cargo test --lib compaction` 与 AgentSession 相关过滤器，确认 stale/generation mismatch 不追加 entry，apply/persistence failure 不错误清 quota，正常结果仍产生 entry。
- W0/S4：运行：

```pwsh
cargo fmt --check
cargo clippy --lib -- -D warnings
cargo test --lib compaction_worker
cargo test --lib compaction
```

Windows 下这些 Cargo 命令必须通过 PowerShell 执行。Phase 1 不运行全量 `cargo test`、`--all-targets` clippy 或 release-max build。

### 集成验证

- session apply contract 成功后重新进入下一轮，worker attempt count 可再次 admission；
- generation 失效后即使 session/provider/model/leaf 四元组恢复相同，旧 worker 结果仍不会改变当前 session entry 数量；
- Phase 1 覆盖的 AgentSession transition 后，pending task 已取消且新 generation/quota 状态符合契约；
- RPC/interactive session replacement、fork/resume/tree/navigation 暂不作为 Phase 1 的通过证据，必须列入 Phase 5 覆盖清单；
- worker timeout/abort 在没有前台 `try_recv` 的情况下也能结束 task，并在错误发布前验证 task quiescent；
- compaction entry 的 replay 和 `tokensAfter` 仍沿现有路径运行；
- 正常、stale、abort、timeout、provider error、panic、apply reject、persistence failure 各最多产生一个 terminal event；
- `extensions_is_compacting` 在所有 terminal path 后为 false。

### 当前环境人工检查

- 检查测试输出中没有真实 provider 网络请求；
- 检查 stale/timeout/abort/persistence 日志不包含 API key 或完整 prompt；
- 检查 `extensions_is_compacting` 在失败、取消和 stale 路径结束后为 false；
- 检查 `git status --short` 没有计划外文件；
- 检查 Phase 1 文档仍明确区分现有 signal-based compaction admission 与未引入的全局 provider admission permit。

### 用户发布后验证

- 无。Phase 1 设计上可完全由本地单元测试和 session 内存测试验收；真实 provider soak 属于后续发布/CI 验证，不在本计划授权范围内。

## 偏差与停止规则

### 允许自行调整

- 将 `try_recv_bound` 命名为其他等价的 crate-private API；
- 将 timeout race 封装在 `run_compaction_task` 或独立的 worker future helper；
- 在不改变行为的前提下增加 RAII flag guard；
- 测试 helper 的文件内位置、provider double 名称和测试 fixture 结构。

### 必须记录的局部偏差

- 当前 worker 的 runtime API 无法安全 await abort 后 join/quiescence：记录具体限制，暂停 timeout/abort 生命周期变更和 durable apply quota 语义变更；
- 某些 Phase 1 直接覆盖的 AgentSession provider/model transition 入口无法接入 generation invalidation：列出具体入口和风险，不宣称 AgentSession 范围全覆盖；
- RPC/interactive/session replacement/fork/resume/tree/navigation 入口未在 Phase 1 接入：记录为 Phase 5 deferred surface，不将其作为 Phase 1 失败；
- 现有事件枚举无法表达 stale：复用事件矩阵规定的 `aborted + error_message`，记录为兼容性偏差，不新增公共事件枚举；
- 当前 apply 路径只能观察到 persistence failure after mutation、无法回滚内存 Session：必须记录实际 entry/replay 状态，且不得重置 quota。

### 必须暂停并返回设计阶段

- 需要修改 session JSONL/SQLite/sidecar entry schema；
- 需要让 RPC event handler 成为新的 session writer；
- 需要引入全局 provider admission gate 或改变其所有权；
- 需要改变 `AutoCompactionEnd` 公共事件字段语义；
- 需要将 attempt quota 在 provider task 成功但 apply contract 未完成时重置；
- 需要为兼容 stale result 而恢复或物理删除旧 session entries；
- 发现 generation 无法在 AgentSession transition 中可靠递增或传递；
- 发现 timeout/abort 错误发布前无法证明 task quiescent；
- 发现 persistence failure 状态无法被测试和事件清晰区分。

## 未决项

- provider admission permit：`deferred`；Phase 1 默认不引入，若 timeout/quiescence 证明必须持有全局 permit，返回设计阶段；当前已有 `CompactionAdmissionSignals` 仍保留，不与新的全局 permit 混淆。
- stale event 的公共字段：`assumption`；优先复用事件矩阵规定的 `aborted + error_message`，Phase 5 再评估是否需要稳定 stale reason 字段。
- Phase 1 覆盖范围：`resolved`；只覆盖 AgentSession 自己拥有的后台自动压缩、直接 provider/model transition、同步 compact 前 invalidation 和可复用 generation API；RPC/interactive session replacement、fork/resume/tree/navigation 留给 Phase 5。
- generation 所有权：`resolved`；由 AgentSession 作用域持有，单调递增，不持久化为 session entry，不从 session replay 推导。
- apply outcome：`blocking-before-implementation`；执行前必须确认当前 `apply_compaction_entry` 能否区分 durable success、before-mutation rejection 与 persistence failure after mutation；无法区分时暂停 quota reset 变更。
- future `CompactionOrigin` 的字段公开性：`non-blocking`；默认 crate-private，仅用于 AgentSession/worker；generation 不能暴露为用户配置。
- timeout quiescence：`blocking-before-implementation`；必须用测试或 runtime API 证明错误发布前 provider/task 已停止。

## 当前任务记录

- 计划：已确认 `checkpoint-v1`，并已将协议落入本 Phase 1 计划。
- 协议状态：已冻结；canonical checkpoint 维护在本文件“当前任务记录”中，后续 Phase 使用各自计划文件维护同类状态。
- 当前 canonical checkpoint：`CCM-P1-W0-S1-G1-A1-C02`
- 当前 checkpoint_kind：`continuation`
- plan_revision：`P1-r1`
- phase / work_unit / slice：`Phase 1` / `W0` / `S1`
- assignment_generation：`1`
- attempt：`1`
- checkpoint_seq：`2`
- owner_role：`main`
- hub_id：未创建（scout 只读预检 session 已完成）
- external_task_ref：无

### 当前目标

完成 W0/S1 的实现前事实对齐，并在用户明确授权后进入 `implement-mode`；当前只允许保存预检结论，不修改生产代码、测试或配置。

### W0/S1 当前状态

- 已完成：
  - checkpoint-v1 协议确认并写入本计划；
  - scout 对 `src/compaction_worker.rs`、`src/agent.rs` 及最小相关 runtime、provider、session、测试代码完成只读预检；
  - 当前源码事实与计划基本一致，预检结论为 `ready_for_w0_s1`。
- 已确认的关键事实：
  - 当前 timeout 仍由 `try_recv()` 执行，发送 abort 后没有等待 join/quiescence；
  - 当前没有 `CompactionOrigin`、generation 或 apply-success quota reset API；
  - 当前 append 发生在 flush 前，persistence failure 可能是 mutation 后失败，且 apply error 当前可能遗漏 terminal event；
  - 现有 provider double 不足以覆盖成功、error、永久 pending、panic 四类真实 worker 行为，但 `Provider::stream()` 和 `CompactionWorkerState::start()` 提供了最小测试注入点；
  - runtime 的 `JoinHandle` 可用于 worker-owned timeout 后等待 wrapper task 结束；当前实现尚未证明该 quiescence。
- 正在进行：无；尚未进入 `implement-mode`。
- 尚未开始：W0/S1 契约测试和测试 seam 修改；所有生产代码、测试和配置均未因本次预检修改。
- 已尝试但失败：无。

### 当前验证状态

- 已运行：scout 只读源码/API 调查；未运行测试、构建、fmt 或 clippy。
- 预检结果：`ready_for_w0_s1`；没有发现按 plan1 停止规则必须立即返回设计阶段的 blocking 偏差。
- 尚未运行：所有 Phase 1 实现测试、`cargo fmt --check`、`cargo clippy --lib -- -D warnings`。
- 设计/计划文档变更：`plan1.md` 已更新；Markdown 自动语法检查跳过。

### 当前阻塞与解除条件

- 阻塞：生产代码和测试实现尚未获用户明确授权。
- 非 blocking 风险：W0/S1–S3 必须用测试和实现证明 timeout/abort 后 quiescence；必须显式分类 apply/persistence outcome；必须覆盖当前 apply error 的 terminal event 和 compacting flag 语义。若实现时无法证明这些契约，再按停止规则返回设计阶段。
- 解除条件：用户明确要求开始执行 Phase 1 / W0，随后进入 `implement-mode`；实现前仍需由 Worker 按当前源码重新核对符号，scout 报告只作为预检输入，不能替代实现前的局部读取。

### 下一步首动作

> 用户授权执行后，进入 `implement-mode`，由一个逻辑 Worker 从 W0/S1 开始：先基于本 checkpoint 读取当前相关符号，固定 provider 四类行为、timeout/abort quiescence、apply/persistence failure、quota 和 terminal event 的契约测试；确认没有新 blocking 偏差后再修改测试 seam。

### 恢复路由

- 当前选择：`wait_for_unblock`
- 用户授权且当前上下文仍有效：`continue_same_hub`（若后续已创建 Worker session）。
- Worker session 不可用但本 checkpoint 与代码状态仍有效：`resume_from_checkpoint`，同一 assignment 下递增 `attempt`。
- 若源码事实、公共契约或 quiescence/apply 证明改变计划假设：`replan_before_continue`，不得直接扩大实现范围。
- 当前 scout 报告已完成；恢复时不得重新派发同一只读预检，除非代码或计划版本发生变化。

### 恢复时不要重复

- 不要重新设计 checkpoint-v1。
- 不要重新执行已经完成的 `src/compaction_worker.rs` / `src/agent.rs` 只读预检。
- 不要重新调查已经固定的 W0/S1 范围、Phase 1 排除项和计划完成标准。
- 不要把 scout 的 `ready_for_w0_s1` 当作 W0/S1 实现完成、W0 accepted 或 Phase 1 phase-gate。
- 不要把计划文档更新或源码预检当作生产代码实现完成。

### Main Agent 验收

- implementation：`not_started`
- verification：`partial`（完成源码/API 预检；未运行实现测试）
- acceptance：`pending`
- 验收结论：已接受 scout 的只读预检作为 W0/S1 执行前输入；预检结论为 `ready_for_w0_s1`，没有确认的 blocking 偏差；生产代码未修改，仍等待实现授权。

### 实际变更与计划偏差

- 实际变更：本轮仅更新 `docs/design/context-compaction-modernization/plan1.md` 的 canonical checkpoint；scout 未修改任何文件。
- 计划偏差：无。当前仍处于 Phase 1 实现前预检，不宣称 W0/S1 已实现。

### 验证证据与遗留风险

- 证据：scout 报告确认 `src/compaction_worker.rs:167–415` 的 worker 生命周期、`src/agent.rs:9517–9791` 的后台 apply 路径、`src/session.rs:3002–3026` 的内存 append 与 `flush_autosave` 边界，以及 `asupersync 0.3.9` 的 JoinHandle 行为。
- 遗留风险：实现时仍需用测试证明 timeout/abort 错误发布前 provider/task 已 quiescent；显式区分 `AppliedDurably`、`RejectedBeforeMutation`、`PersistenceFailedAfterMutation`；并补齐 apply error 的 terminal event、quota 和 compacting flag 语义。若无法清晰区分或证明，按停止规则返回设计阶段。
