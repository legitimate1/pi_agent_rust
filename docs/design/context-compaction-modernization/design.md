# Pi Agent Rust — 上下文压缩现代化设计

> 状态：草案，待设计审核与用户确认
> 设计范围：将上游已验证的上下文压缩改进拆分为可独立验收、可独立回滚的实现阶段
> 设计权威：本文定义目标行为与边界；`README.md` 定义实现阶段总览；当前阶段的详细执行计划见 `plan1.md`

## 目标与背景

当前项目已经具备基础的上下文压缩能力：按 context window 阈值计算切点，调用 provider 生成摘要，将 `CompactionEntry` 追加到 session，并在 replay 时使用摘要与近期消息。当前实现还存在四类可改进点：

1. 本地 token fallback 使用 chars/3，代码与工具输出密集场景的误差较大；
2. 所有自动压缩默认依赖 LLM，工具结果占据大量空间时仍会产生不必要的请求；
3. provider quota、后台 worker cooldown 或 attempt limit 可能让超大 session 长期无法压缩；
4. 后台压缩任务结果没有绑定启动时的 session/model/branch 身份，worker 的 timeout 和 attempt quota 生命周期也不够严密。

上游依据：

- `5a4908afb` / `c287e660d`：BPE token counting；
- `2a6607bfb` / `8a9f63e4a`：shake 与 shake-first；
- `f34a24957` / `7cd028277`：provider-free local fallback 及 2x window gate；
- `2962434f5` / `bdd2400a5`：worker timeout、origin 与成功应用后的 quota 管理；
- `5e85bdc56`：snapcompact 实验模式，本设计暂不纳入核心交付。

## 当前实现事实

以下事实以当前 `custom` 分支源码为准：

- `src/compaction.rs:62-107`：`ResolvedCompactionSettings` 只有 enabled、context window、reserve、keep-recent；`CompactionResult` 没有压缩模式或渲染 payload。
- `src/compaction.rs:835-900`：本地消息估算使用 chars/3，并对图片使用固定估算。
- `src/compaction.rs:1681-1740`：`compact()` 直接生成 LLM 摘要；provider 错误向调用方传播；没有 shake 或 local fallback API。
- `src/compaction_worker.rs:169-342`：pending task 只保存 join、abort sender、启动时间；`try_recv()` 负责轮询 timeout；attempt count 在启动时增加，没有“成功持久化应用后重置”的状态。
- `src/agent.rs:9517-9657`：`maybe_compact()` 先应用后台结果，再根据 worker quota 启动新任务；worker 不可启动时直接返回。
- `src/agent.rs:9755-9791`：成功结果通过 `Session::append_compaction` 进入统一 session 写入路径。
- `src/session.rs`：session entry 不物理删除，provider replay 由最新 compaction entry 的 cut point 决定；这是必须保持的持久化边界。
- `docs/context/context-compaction.md`：当前 Agent-facing 契约要求保留 branch path、tool chain、split turn、session autosave、事件和 tokensAfter 语义。

## 设计目标与行为规格

### G1：后台压缩结果必须属于启动时上下文

后台任务启动时创建不可变的 `CompactionOrigin`：

```text
session_id
provider_id
model_id
snapshot_leaf_id
compaction_generation
```

其中 `compaction_generation` 是 `AgentSession` 作用域内单调递增的失效代数。它不是 session entry 的持久化字段，也不是时间戳；任何 context switch、session replacement、branch navigation、provider/model transition 或显式 invalidation 都必须递增它。这样即使上下文之后通过 ABA 方式恢复到相同的 session/provider/model/leaf 四元组，旧任务也不能重新通过校验。

任务完成后，Agent 只有在 origin 与当前 AgentSession 的 session、provider、model、leaf 和 generation 全部一致时，才允许应用结果。否则：

- 等待任务已经 quiescent 后丢弃结果，不追加 `CompactionEntry`；
- 发送 `auto_compaction_end`，按事件矩阵标记为 stale/aborted；
- 不重置 worker attempt quota，因为结果没有被当前 session 应用；
- 记录可诊断的 origin mismatch 原因。

Phase 1 的承诺范围收窄为：`AgentSession` 自己拥有的后台自动压缩路径、其直接的 provider/model transition、同步 compact 前的 invalidation，以及供外层 session surface 调用的显式 invalidation API。RPC/interactive 的 session replacement、fork/resume/tree surface 不在 Phase 1 的全覆盖承诺内；它们必须在后续 surface closure 阶段接入该 API 后，才能宣称全系统 stale-result 安全。

任何 context switch 或同步 compact 开始前，Phase 1 已覆盖的 AgentSession 路径必须取消 pending task、递增 generation，并按 session-switch 规则清空该 AgentSession 的 compaction quota 状态。

### G2：worker timeout、abort、panic 和 quota 有明确所有权

目标 worker 生命周期：

```text
admitted
  ↓
started(attempt_count += 1; generation captured)
  ↓
provider task owns timeout + abort race
  ├─ success → quiescent result returned; wait for durable apply
  ├─ provider error → quiescent failed; no entry; no quota reset
  ├─ timeout → abort + await quiescence; no entry; no quota reset
  ├─ abort → await quiescence; no entry; no quota reset
  └─ panic → quiescent task converted to worker error; no entry; no quota reset

origin + generation accepted
  ↓
append/apply outcome classified
  ├─ persistence/apply failed → current Session outcome is explicitly retained or rolled back according to apply contract; quota is not reset
  ├─ save disabled → in-memory apply is successful only under the documented non-durable mode; quota reset is still tied to successful apply, not provider completion
  └─ durable apply succeeded → mark_applied_success(attempt_count = 0)
```

`try_recv` 只负责非阻塞回收已完成且 quiescent 的结果，不再是 timeout 的唯一执行者。timeout/abort 的完成条件是：provider future、wrapper task 和其持有的取消相关资源都已结束；若 runtime API 无法在错误发布前证明这一点，Phase 1 必须暂停该生命周期变更，而不是把“已发送 abort”当成“任务已停止”。

### G2a：Compaction apply 失败语义

Phase 1 不重做 session 存储引擎，但必须把当前 apply 状态写成明确契约：

```text
生成成功 ≠ 应用成功
内存 append 成功 ≠ 持久化成功
持久化失败 ≠ 可以重置 worker quota
```

`apply_compaction_entry` 的结果必须能区分：

1. `AppliedDurably`：entry 已安装并完成当前持久化路径，允许 `mark_applied_success`；
2. `AppliedInMemoryOnly`：仅适用于明确的 `save_enabled = false` 测试/运行模式，必须记录非持久化语义，是否重置 quota 由该模式契约明确；
3. `PersistenceFailedAfterMutation`：内存 Session 已变更但持久化失败；不得重置 quota，必须通过现有错误/隔离语义报告 entry/replay 状态，不能假装“未发生”；
4. `RejectedBeforeMutation`：origin、admission 或 precondition 失败，不能有 compaction entry。

Phase 1 的验收必须覆盖至少 1、3、4；如果当前实现无法在不扩大 session 事务设计的情况下区分 2/3，必须将其列为 blocking 偏差并暂停 quota 语义变更。

### G3：超大 session 必须有 provider-free 进展路径

定义强制本地压缩阈值：

```text
tokens_before >= context_window_tokens × 2
```

以下两种情况触发 deterministic local fallback：

1. 后台 worker 因 session attempt limit、cooldown 或其他 quota 阻塞，且当前 session 达到强制阈值；
2. LLM 摘要请求失败，且当前 preparation 已达到强制阈值。

强制 fallback：

- 不调用 provider；
- 不执行可取消的 `session_before_compact` hook；
- 仍经 AgentSession 的统一 session append/autosave 路径；
- 保留 previous summary；
- 使用有预算的历史头尾 excerpts；
- 对中间消息显式写入省略标记；
- `CompactionDetails.mode = "local"`；
- 通过 `auto_compaction_start.reason` 报告 `forced_local` 和阻塞原因。

强制阈值以下的 provider 错误必须传播给 worker 的 cooldown/attempt 机制，不应立即把一次短暂网络错误永久变成质量较低的摘要。

### G4：token 估算分层且统一

估算优先级保持不变：

```text
有效 provider usage
  ↓ 无法使用
当前 provider 对应的 BPE 估算
  ↓ bpe-tokens feature 未启用
chars/4 或项目现有最终 fallback
```

BPE 计数器：

- Anthropic 使用 Cl100k 类表；
- 其他 provider 默认使用 O200k 类表；
- 图片保留固定估算；
- 未来若引入 MediaContent，再单独增加媒体大小估算；本设计不添加媒体模型。

所有以下路径必须复用同一估算入口：

- `prepare_compaction` 的 `tokens_before`；
- cut point 累计；
- `tokensAfter`；
- RPC/SDK 的 token estimate 表面；
- shake projection。

不能复制第二套 token 算法。

可选的 calibration 日志只用于诊断，不改变触发判断：

```text
estimated_total
measured_total
estimated_minus_measured
ratio
```

### G5：Shake 提供零 LLM 压缩路径

新增 deterministic `compact_shake()`：

- 用户文本和 assistant 文本原样保留；
- 小型 tool result 原样保留；
- 超过阈值的大型 tool result 替换为一行 stub，包含 tool 名称、成功/失败状态、行数和字节数；
- 保留 tool call 名称和可读的 command/bash/custom/branch 信息；
- 不创建 dangling tool-call/tool-result replay 关系；
- `CompactionDetails.mode = "shake"`；
- 不访问 provider，不要求 API key。

建议的保留阈值：`512` 个字符。该阈值属于可调实现常量，不作为公共配置初始暴露。

shake projection 必须计算：

```text
shake summary estimate + keep_recent estimate
```

不能只计算 summary 本身，否则临界场景会在下一轮立即再次触发压缩。

自动策略：

```text
summary       → 始终执行 LLM 摘要
shake-first   → shake projection 不再触发阈值时直接 shake，否则升级到 LLM
aggressive    → LLM 摘要，但 keep_recent_tokens 减半
```

默认策略保持 `summary`，不改变未配置用户的现有行为。

手动命令目标行为：

```text
/compact                  → 普通 LLM 摘要
/compact shake            → deterministic shake
/compact aggressive       → aggressive LLM 摘要
/compact <instructions>  → 原有 custom instructions 语义
```

解析规则必须避免把普通 custom instructions 中的首词误判为模式；具体命令语法在 Phase 4 实现前定向核对 interactive/RPC/SDK 三个入口。

### G6：压缩持久化、replay 和事件契约不变

所有压缩结果仍然：

1. 生成 `CompactionResult`；
2. 通过 `AgentSession::apply_compaction_result` 或等价统一路径；
3. 追加 `SessionEntry::Compaction`；
4. 使用现有 autosave、sidecar lock、索引和 Windows contention retry；
5. 由 `Session::to_messages_for_current_path()` replay。

禁止：

- 物理删除旧 session entries；
- 在 RPC event handler 中新增 JSONL writer；
- 为 local/shake 建立旁路 session 格式；
- 将 `tokensBefore`/`tokensAfter` 当作 provider 精确 tokenizer 结果。

新增模式只进入 `details.mode` 和事件 payload，不改变旧 entry 缺省字段的读取语义。

### G7：默认安全性和兼容边界

默认配置必须继续是：

```text
自动压缩：summary
渲染：text-only
provider usage 优先
session entry replay 规则不变
```

本轮不引入 snapcompact，不修改 `ContentBlock`，不引入 MediaContent，不迁移整个上游 `agent.rs`/`rpc.rs`/`session.rs`。

## 配置与内部接口目标形态

以下是目标形态示意，不是当前已实现 API：

```rust
pub enum AutoCompactionMode {
    Summary,
    ShakeFirst,
    Aggressive,
}

pub struct ResolvedCompactionSettings {
    pub enabled: bool,
    pub context_window_tokens: u32,
    pub reserve_tokens: u32,
    pub keep_recent_tokens: u32,
    pub auto_mode: AutoCompactionMode,
}

pub struct CompactionDetails {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub mode: Option<String>, // local | shake; None = LLM summary
}

pub struct CompactionResult {
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    pub details: CompactionDetails,
}
```

配置名称建议使用现有 `compaction` 对象内的清晰字段：

```json
{
  "compaction": {
    "enabled": true,
    "reserve_tokens": 16384,
    "keep_recent_tokens": 20000,
    "auto_mode": "summary"
  }
}
```

不为本设计引入与自动策略含义冲突的 `compaction.mode` / render mode 双重语义。snapcompact 若未来启动，另行设计独立的 render 配置键。

## 事件终态矩阵

Phase 1 不新增公共事件枚举，但必须固定现有 `AutoCompactionEnd` 的字段组合和单终态规则：

| outcome | result | aborted | willRetry | error_message | quota reset | session append |
|---|---:|---:|---:|---|---:|---:|
| 正常结果，origin/generation 匹配，apply 成功 | 有 | false | false | None | 是 | 是 |
| stale origin 或 generation mismatch | None | true | false | 含 `stale`/`origin mismatch` | 否 | 否 |
| explicit invalidation/用户取消 | None | true | false | 可为空或含 `cancelled` | 否 | 否 |
| worker timeout，任务已 quiescent | None | false | true/false 依调用方既有重试契约 | 含 `timed out` | 否 | 否 |
| provider error | None | false | false/既有 worker retry 语义 | provider error | 否 | 否 |
| worker panic | None | false | false | 含 `worker panicked` | 否 | 否 |
| apply rejected before mutation | None | false | false | precondition/apply error | 否 | 否 |
| persistence failed after in-memory mutation | None | false | false | persistence error，必须标明状态不确定/已隔离 | 否 | 已发生，按 apply contract 报告 |

约束：

- 每个后台任务最多发送一个 terminal `AutoCompactionEnd`；
- `AutoCompactionStart` 已发送但任务 stale/失败时必须有对应 terminal event；
- `extensions_is_compacting` 在所有 terminal path 后最终为 false；
- `willRetry` 不得用来掩盖 stale result，也不得在没有实际重试计划时设置为 true；
- Phase 1 复用现有公共事件结构，Phase 5 再评估是否需要稳定的 `reason` 字段。

## 阶段拆分与独立交付边界

### Phase 1 — AgentSession 后台 Worker 生命周期与 stale-result 安全

目标：不改变摘要内容，仅修复 `AgentSession::maybe_compact` 后台任务的 timeout、abort、panic、generation/origin 和 quota 应用时序。

明确范围：

- 覆盖 AgentSession 自己拥有的后台自动压缩路径；
- 覆盖其直接的 provider/model transition、同步 compact 前 invalidation 和显式 worker invalidation API；
- 不承诺覆盖 RPC 的 session replacement/fork/resume、interactive session replacement、tree/navigation 等外层 session surface；这些入口必须在 Phase 5 接入统一 invalidation/generation API 后，才能宣称全系统 stale-result 安全；
- Phase 1 不引入新的全局 ProviderAdmissionGate/permit。

独立验收：worker 测试能够证明任务 quiescent timeout；generation/origin mismatch 不会写 session；session switch 规则会失效 pending 并重置该 AgentSession 的 worker quota；只有 apply contract 判定成功后 attempt 才重置；apply/persistence failure 不会错误重置 quota；事件矩阵中的每个后台任务只有一个 terminal event。

独立回滚：只回滚 AgentSession 后台 worker/orchestration 改动，不影响 token 算法和压缩模式。

当前详细计划：`plan1.md`。

### Phase 2 — 超大 session 的 deterministic local fallback

依赖：Phase 1。

目标：增加 `compact_local`、fallback summary、2x window gate，并接入 Agent background quota-blocked 路径和 LLM error 路径。

独立验收：provider 永不被调用；超过阈值可追加 entry；低于阈值仍传播 provider error；fallback 结果经过统一 autosave/replay。

### Phase 3 — 统一 token counting 与 BPE fallback

依赖：Phase 1；与 Phase 2 可在设计上并行，但建议先完成 Phase 2 的故障安全路径。

目标：新增 `token_count` 抽象，接入 BPE、provider table 选择、feature-off fallback、tokensAfter 和 calibration tests。

独立验收：BPE reference vectors、feature-off tests、provider usage 优先级、cut point/tokensAfter 一致性。

### Phase 4 — Shake 与 shake-first 自动策略

依赖：Phase 3 的统一 token estimator；Phase 1 的可靠 worker 生命周期。

目标：增加 `AutoCompactionMode`、`compact_shake`、`compact_auto`、手动模式解析和配置路由。

独立验收：shake 零 provider call；tool result stub/keep 行为；projection 包含 keep-recent；shake-first 正确升级/不升级；默认 summary 行为不变。

### Phase 5 — 观测、契约和完整表面收口

依赖：Phase 2–4。

目标：补齐 Agent/RPC/SDK/interactive 的事件和 payload 断言、文档、配置说明、模式指标和跨入口测试。

独立验收：四个表面使用同一模式/估算语义；事件序列一致；session replay 和 tokensAfter 一致。

### Deferred A — Snapcompact

不属于当前核心交付。需要另行确认图像 payload 大小预算、vision capability、session details schema、provider 兼容性和质量评估后再启动。

### Deferred B — Provider native compaction

OpenAI Responses 原生 `/responses/compact` 属于 provider-specific/extension-specific 功能，不纳入通用压缩核心。若用户明确需要，再单独设计安全的 request sanitizer、认证边界和结果 replay 契约。

### Deferred C — MediaContent token 估算

当前分支模型层尚未引入 MediaContent；不在本轮通过修改 compaction 单独添加媒体支持。

## 跨阶段不变量

| 编号 | 不变量 | 保护方式 |
|---|---|---|
| I1 | session entry 是唯一 Pi-owned session JSONL 写入路径 | 所有模式复用现有 apply/autosave；测试检查 reopen/replay |
| I2 | stale background result 不能写入当前 AgentSession | origin + generation compare；Phase 1 stale-result 测试；外层 session surface 在 Phase 5 接入后再扩展覆盖 |
| I3 | pending/timeout/abort/panic 不产生部分 compaction entry | worker quiescence + session entry count 断言 |
| I4 | attempt quota 只在 apply contract 判定成功后重置 | worker + Agent apply/persistence failure 测试 |
| I5 | provider usage、BPE、最终 fallback 的优先级稳定 | token estimator 单元测试 |
| I6 | tool-call/result replay 不被 cut point 或 shake 破坏 | cut point、shake adjacency、replay 测试 |
| I7 | local fallback 只在 2x window 等严重条件下绕过 provider | threshold matrix 测试 |
| I8 | 默认用户仍使用 LLM summary + text-only | default config/integration tests |
| I9 | `tokensAfter` 表示当前 path replay 的本地估算，不是账户 quota | Agent/RPC/SDK payload tests |
| I10 | 扩展 hook 不能阻塞强制 local forward progress | forced-local path test；明确不调用 before hook |
| I11 | generation 失效优先于业务 origin 四元组 | session switch ABA 测试；generation mismatch 必须 fail-closed |
| I12 | persistence failure 不得伪装成 durable success | append/flush failure 测试；quota 不重置并报告状态 |
| I13 | 每个后台任务最多一个 terminal compaction event | timeout/stale/abort/provider/apply outcome 事件矩阵测试 |

## 风险与取舍

### Token 估算误差

BPE 仍然不是每个 provider 的精确 tokenizer。它比 chars/3 更适合作为本地 fallback，但不能取代 provider usage。必须保留“estimated/reported”区分。

### Local fallback 的语义损失

local fallback 不进行语义理解，只保存 previous summary 与受预算限制的 excerpts。因此必须显式标记 degraded/local，且只作为严重超窗的 forward-progress 机制。

### Shake 对重要工具输出的损失

shake 会丢弃大型 tool result。默认不启用，只在用户显式选择或配置 `shake-first` 时使用；stub 必须告诉模型可以重新运行工具。

### Origin 判断的状态粒度

当前分支的 session/model 生命周期比上游简单，但 Phase 1 必须覆盖 session id、provider/model、current leaf 和 AgentSession generation。业务 origin 四元组只能描述快照，不能替代 generation；如果现有 Session API 无法无副作用取得 leaf，Phase 1 需要先补一个只读 accessor。不能用全量 session hash、时间戳或“切换后四元组恢复”来替代单调失效代数。Phase 1 只承诺 AgentSession 直接拥有的后台路径；外层 RPC/interactive session replacement、fork/resume/tree surface 留给 Phase 5。

### 配置命名

本设计建议新增 `compaction.auto_mode`，而不是照搬上游同时使用多个 `mode` 字段。若用户要求严格上游配置兼容，需在进入 Phase 4 前重新确认公共配置契约。

## 验证总原则

每个 Phase 必须先写针对性测试，再实现调用方迁移，最后执行轻量静态检查：

```text
cargo test --lib <相关过滤器>
cargo fmt --check
cargo clippy --lib -- -D warnings
```

修改测试文件时，必须运行对应测试直到通过。全部阶段完成后才执行项目收尾门禁：全量 `cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。本设计不授权构建、部署或提交。

## 待确认事项

以下事项不阻塞 Phase 1 的 worker 设计，但在对应阶段开始前必须确认：

1. 是否采用本设计建议的 `compaction.auto_mode` 配置名，还是要求严格跟随上游配置命名；
2. local fallback 的 `CompactionDetails.mode` 是否公开为稳定 RPC/SDK 字段，还是仅作为内部详情字段；
3. Phase 1 是否同时引入 provider admission permit；当前分支没有上游完整的 `ProviderAdmissionGate`，默认先不扩大到新的全局 provider 并发协议；
4. Phase 4 的手动 `/compact shake` 是否只覆盖 interactive，还是同时为 RPC/SDK 增加显式 mode 参数；
5. 是否在核心四阶段完成后再单独启动 snapcompact 设计，而不是将其作为本轮默认路线。

这些问题不影响本轮计划总览和 Phase 1 的 worker 生命周期设计；它们会在进入对应 Phase 前作为设计闸门重新确认。`