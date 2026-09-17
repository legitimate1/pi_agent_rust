# pi-agent-rust — Turn recovery 实现计划

## 1. 实现边界（Implementation Contract）

Source: `docs/design/pi-agent-rust-turn-recovery-design.md`

Goal:

- 在 custom 的共同 Agent loop 中实现正常 `Stop`/`Length` assistant turn 的有限自动续跑，并让 interactive、print、RPC、SDK、ACP 共享同一行为。

In scope:

- 新增 `TurnRecoveryMode`、`RecoveryClass`、`TurnRecoveryState`、`RecoveryAction` 及纯分类/预算/nudge 逻辑。
- 在 Agent turn 完成、tool/steering 优先级处理之后接入 recovery evaluate；nudge 使用普通 `Message::User(UserMessage)`。
- 以一次高层逻辑 prompt/continue 调用作为 `LogicalRunScope` 边界；同一逻辑 run 的 recovery continuation 与 provider retry/resume 复用 scope，独立用户/宿主调用创建新 scope。
- 将 settings 的 `turn_recovery` 解析、合并和默认值传入所有现有 `AgentConfig` 构造点。
- 保持现有 AgentEvent、session JSONL、RPC/ACP wire、SDK compaction/`tokensAfter` 和 eventual persistence 契约。
- 同步补充纯机制测试、Agent loop 负路径/优先级测试、配置兼容性测试、入口最小回归和必要 fixture 迁移。

Out of scope:

- 不直接 merge 或 cherry-pick 上游 turn recovery 提交 `1ba1e39d0d91ebf3c7b0c97647d148b7401bb459`。
- 不新增 `PauseTurn`、`Refusal`、recovery 专属 AgentEvent、RPC/ACP wire 字段、SDK result 字段或 session entry/schema。
- 不将 provider/network `Error`、`Aborted`、取消、tool call turn 或 `revert_incomplete_response` 路径交给 recovery classifier。
- 不修改 `CompactionResult`、`CompactionEntry`、`tokensAfter`、session schema、Cargo 依赖、TUI recovery 专用渲染或中途主动 compaction。
- 不把 provider retry 计数器与 recovery continuation cap 混用，也不让 retry 重置同一 logical run 的 recovery state。

Assumptions:

- `src/agent.rs` 是所有入口复用的共同 Agent loop；`AgentSession` 的 `persist_new_messages`/interactive 自己的保存代码继续承担已有的批量持久化。
- `Config` 继续使用 `Option` 字段、`#[serde(default)]` 和 `Config::merge(base, other)` 的 `other.or(base)` 规则。
- `run_continue_with_abort` 的现有调用者必须按语义迁移到显式的“复用已有 scope”或“创建独立 scope”入口，不能以方法名自动推断。
- 新增代码保持 safe Rust、Rust 2024 nightly 和现有项目风格；不增加第三方依赖。

Design delta:

- 无。实现必须遵守定稿设计；如果实际调用链无法满足 scope、事件顺序或持久化边界，应停止编码并报告，不自行改变语义。

---

## 2. 文件变更清单（Change Manifest）

| ID  | 路径                                   | 操作 | 用途                                 | 主要改动                                                                                                                                                                                                                                                                                                                                                                     |
| :-: | :------------------------------------- | :--- | :----------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `src/turn_recovery.rs`                 | 新建 | 承载无入口副作用的 recovery 机制     | `TurnRecoveryMode`（serde lower-case/default）、`RecoveryClass`、`TurnRecoveryState`、`RecoveryAction`；`classify`、mode gating、code fence/悬空列表/promise heuristic、最多 2 次 nudge、结构 recovery 防重复、固定 nudge 文本；模块单元测试                                                                                                                                 |
| C2  | `src/agent.rs`                         | 修改 | 接入共同 Agent runtime               | `AgentConfig.turn_recovery` 及 Debug/Default；`LogicalRunScope`（至少持有 recovery state，provider retry 独立保留）；显式 scope 的 run 内部 API；assistant `ContentBlock::Text` 提取 helper；在 assistant 完成→tool call/TurnEnd→steering 之后、follow-up staging 之前 evaluate；注入普通 user nudge并走现有 Message/Turn event/transcript；Agent/AgentSession focused tests |
| C3  | `src/config.rs`                        | 修改 | settings 解析、合并和最终 mode       | `turn_recovery: Option<TurnRecoveryMode>`；`merge` 中 other-over-base；`turn_recovery_mode()` 默认 Conservative；缺失字段兼容、off/conservative/aggressive 和未知值错误测试                                                                                                                                                                                                  |
| C4  | `src/lib.rs`                           | 修改 | 注册机制模块                         | 添加 `turn_recovery` 模块导出，保持当前 public API policy（按现有模块可见性决定公开程度）                                                                                                                                                                                                                                                                                    |
| C5  | `src/main.rs`                          | 修改 | print/CLI 入口和外层 retry scope     | 主入口 `AgentConfig` 传入 `config.turn_recovery_mode()`；`run_print_prompt_with_retry` 为一次独立 prompt 创建 scope，并将 provider retry/resume 使用现有 scope；错误 retry 继续不生成 recovery nudge                                                                                                                                                                         |
| C6  | `src/sdk.rs`                           | 修改 | SDK 配置和 continue 语义             | SDK `AgentConfig` 传入 mode；独立用户/宿主 continue 创建新 `LogicalRunScope`；内部 provider retry/resume 复用已有 scope；更新 SDK 测试 literal、message/event 行为测试                                                                                                                                                                                                       |
| C7  | `src/acp.rs`                           | 修改 | ACP 配置和内部/wire 边界             | ACP session 创建时传入 mode；独立 prompt 创建新 scope、provider retry/resume 复用 scope；验证内部多 turn 仍映射到现有 ACP message/tool `session/update`，不新增 wire 字段；更新测试 literal                                                                                                                                                                                  |
| C8  | `src/interactive/agent.rs`             | 修改 | 裸 Agent 调用的 scope 传递和保存回归 | `submit_continue` 显式建立独立 continue scope或调用等价内部 API；将裸 Agent 运行产生的 nudge/continuation 继续追加到现有内存 transcript并按 `save_enabled` 保存；保持 abort、事件 batcher、extension callback 顺序                                                                                                                                                           |
| C9  | `examples/pi_debug.rs`                 | 修改 | 示例构造点迁移                       | 完整 `AgentConfig` literal 增加 mode，保持示例行为明确                                                                                                                                                                                                                                                                                                                       |
| C10 | `tests/**/*.rs` 及现有 fixture/builder | 修改 | 编译迁移和行为回归                   | 逐一补齐完整 `AgentConfig { ... }` literal；非 recovery 测试显式 `Off`，Recovery 测试显式 Conservative/Aggressive；仅按行为变化更新 VCR/golden/event transcript/调用次数                                                                                                                                                                                                     |
| C11 | 受影响的 `src/*` 内部测试模块          | 修改 | 覆盖入口边界而不引入真实网络         | `src/agent.rs` loop/优先级/persistence/retry 组合；`src/config.rs` settings；`src/sdk.rs` SDK；`src/acp.rs` ACP；必要时 `src/main.rs`/`src/rpc.rs` 既有 fake provider harness 的最小调整                                                                                                                                                                                     |

明确不修改：`Cargo.toml`、`Cargo.lock`、`src/session.rs` schema/entry 类型、`src/compaction.rs` 的 `tokensAfter` contract、RPC wire protocol（除非测试 harness 必须做最小内部调整）。

---

## 3. 依赖关系（Dependency Plan）

### Phase 0 — 实现前 checkpoint 与构造点盘点

- 在开始源码实现前，提交当前工作区状态；当前交接记录的设计文档仍需纳入可追溯提交。
- 盘点范围：`src/`、`tests/`、`examples/` 中所有完整 `AgentConfig { ... }` literal、builder/helper，以及所有 `run_continue_with_abort` 调用。
- 标注每个调用者是：
  - 新的用户/宿主 prompt 或 continue（新建 scope）；
  - 同一逻辑 prompt 的 provider retry/resume（复用 scope）；
  - 仅测试辅助调用（按测试意图显式选择 Off 或 recovery mode）。
- 重点事实：`src/interactive/agent.rs:1472` 的 `submit_continue` 使用裸 `Agent`；`src/agent.rs:10494` 附近的 `AgentSession::run_continue_with_abort` 先重放当前 session path 再持久化新增消息；`src/main.rs:6881` 和 `src/rpc.rs:2571` 的外层 retry 需要复用 scope。

### Phase 1 — 纯机制模块

- C1 无跨模块运行时依赖，可先完成并独立测试。
- C3 的 enum 类型会被配置层引用；若 C1 类型需要先注册模块，C4 可作为最小编译配套随 Phase 1 完成。
- 验收：分类规则、mode gating、结构启发式、promise 末尾 240 字符窗口、2 次 cap、结构 recovery 防重复、nudge 格式全部由单元测试固定。

### Phase 2 — 共同 Agent loop 与显式 scope

- C2 依赖 C1/C4。
- 先确定内部 API 形状：公共 `run`/`run_with_*` 代表新的逻辑 run；provider retry/resume 使用接收既有 `LogicalRunScope` 的内部/`pub(crate)` API；独立 continue 使用新建 scope 的入口。不要保留一个无法表达语义的隐式 `run_continue_with_abort` 调用约定。
- `LogicalRunScope` 的生命周期必须覆盖：普通 provider attempt → recovery continuation → provider error retry/resume → 同一高层调用内的后续 continuation。
- loop 插入点必须位于：assistant message 完成、tool call 检查和 `TurnEnd` 之后；steering 已检查且为空时才 evaluate；recovery action 注入后不得先进入 idle follow-up。
- recovery 只接收正常 assistant message 的 stop reason 和从 `ContentBlock::Text` 拼接出的可见文本；`ToolUse`/`Error`/`Aborted`、tool call、steering、abort/error 分支均不生成 nudge。
- nudge 通过既有普通 user message 路径进入 `self.messages`、`new_messages`、MessageStart/MessageEnd、后续 provider context 和 TurnStart/TurnEnd；不新增专属事件。
- 验收：fake provider `Length → Stop` 调用 2 次、transcript 保留原始 assistant+nudge+continuation；连续 `Length` 最多 2 条 nudge；tool/steering/follow-up/error/abort 负路径保持契约。

### Phase 3 — 配置与入口迁移

- C3 依赖 C1；C5/C6/C7/C8/C9/C10/C11 依赖 C2 和 C3。
- C3：按照现有 `Config::merge` 显式加入字段，最终 accessor 默认 Conservative；serde 缺省字段必须成功，未知枚举值沿配置错误路径失败。
- C5：一次 print prompt 的外层 retry loop 创建并持有 scope；所有 `run_continue_with_abort` retry/resume 调用明确标记为复用该 scope。
- C6：SDK 构造 `AgentConfig` 时传 mode；独立 continue 与 provider retry/resume 语义分开；不扩展 SDK wire/result schema。
- C7：ACP session 配置传递和内部 scope；只验证既有 wire 映射，不添加字段。
- C8：interactive 不假设 `AgentSession` 自动持有 scope；裸 Agent task 显式创建/传递 scope；保存逻辑维持 `save_enabled=false` 不写 JSONL、`true` 在既有批量阶段保存。
- C9/C10：逐项迁移完整 literal 和 fixture，不对 `AgentConfig::default()` 使用点做无意义改写；只在测试意图需要时显式 Off。
- C11：补充入口最小回归，优先 fake provider/纯配置/事件断言，不引入真实网络。

### Phase 4 — 集成质量验证

- 所有生产代码和测试迁移完成后，主 Agent 集成阶段加载 `add-tests` 技能，检查新增公共/核心接口、cap、边界分支和回归覆盖。
- 先跑针对性测试和轻量静态检查；全部改动完成后再跑项目收尾门禁。
- 通过后才创建 Wave 4 文档、更新 `docs/upstream/wave-plan.md`，并按用户指令提交/推送和触发 `my-check.yml`。

### 并行与冲突说明

- C1 可独立于构造点盘点进行，但源码写入阶段 C2/C3/C4 不应和入口迁移并行，避免接口尚未稳定。
- C5、C6、C7、C8、C9、C10 在 C2/C3 完成后可按文件并行，但不得同时编辑 `src/agent.rs`；C2 和 C11 也不能并行修改同一测试模块。
- `src/agent.rs` 是冲突热点：AgentConfig、LogicalRunScope、loop、AgentSession persistence 和大量内部测试应由同一串行任务完成或分阶段锁定。
- 不使用 Subagent review；交接记录已注明设计/实现 review 由主 Agent 自行完成，除非用户重新授权。

---

## 4. 验证计划（Validation Plan）

### Phase 1 针对性验证

Windows 下通过 `pwsh` 执行：

```pwsh
cargo test --lib turn_recovery
cargo fmt --check
cargo clippy --lib -- -D warnings
```

预期：纯机制模块测试通过，格式和 lib Clippy 无新增诊断。

### Phase 2 Agent loop 验证

```pwsh
cargo test --lib <Agent recovery focused test filter>
cargo test --lib <Agent priority/error/abort focused test filter>
cargo fmt --check
cargo clippy --lib -- -D warnings
```

必须观察：

- `Length → Stop` 只发一条 nudge并最终返回 continuation；
- 连续截断最多两条 nudge；
- clean stop 不消耗预算；结构 recovery 不无限重复，但剩余额度允许明确 `Length` recovery；
- tool call 先执行 tool，不 recovery；steering 优先，不额外 nudge；recovery 完成后才消费 idle follow-up；
- provider `Error`/abort/tool abort 走原有路径，不生成 nudge；
- provider retry/resume 与 recovery 共享同一 logical-run cap，retry 不重置 continuation 次数；独立 continue 从零开始；
- `save_enabled=true` 时 nudge/continuation 在现有高层批量阶段保存；`false` 时不写 session JSONL；不引入 recovery 专属 flush；
- 事件是普通 MessageStart/MessageEnd 与既有 TurnStart/TurnEnd 序列，不新增 recovery 事件。

### Phase 3 配置与入口验证

```pwsh
cargo test --lib config
cargo test --lib <SDK recovery/config focused test filter>
cargo test --lib <ACP recovery/config focused test filter>
cargo test --lib <interactive recovery/persistence focused test filter>
cargo test --test <实际受影响的单个测试文件>
cargo fmt --check
cargo clippy --lib -- -D warnings
```

必须观察：

- settings 缺少 `turn_recovery` → Conservative；显式 `off`/`conservative`/`aggressive` 正确；未知值报配置错误；global/project merge 遵守 project/other 覆盖；
- main、SDK、ACP 的 AgentConfig 都传入最终 mode；RPC 不出现新协议字段；
- interactive 裸 Agent 的独立 continue 有新 scope，nudge 与 continuation 进入内存 transcript/event callback 并遵守保存开关；
- 非 recovery 旧测试显式 Off 后调用次数/transcript 不因默认 Conservative 意外改变；recovery 专用测试明确 mode。

### 全部实现完成后的项目收尾门禁

Windows 下通过 `pwsh` 执行，且只在所有改动完成后执行：

```pwsh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

预期：全量测试、全量 Clippy、格式检查通过。日常开发阶段禁止运行 `cargo test --all-targets`、`cargo clippy --all-targets` 和 `cargo build --profile release-max`；本阶段不自动构建或部署。

### 人工检查

- [ ] classifier 未接触 provider 私有 payload、thinking、tool call 或 tool arguments。
- [ ] recovery 不进入 provider Err 分支，不复用 retry counter，不回滚已完成 assistant 输出。
- [ ] nudge 文本、消息顺序和 persistence 时点符合定稿设计。
- [ ] `run_continue_with_abort` 的每个调用点都有显式 scope 语义，未以方法名隐式判断。
- [ ] 未改 session schema、compaction/`tokensAfter`、RPC/ACP wire 或新增依赖。
- [ ] 用户允许后才进入源码实现；实现完成后按要求提醒更新 `docs/context/`。

---

## 5. 起始交接信息

- 设计依据：`docs/design/pi-agent-rust-turn-recovery-design.md`
- 上游终点：`upstream/main = 82b7c0c8e72c7fed5cc1ab74ebccfe378419c9e6`
- 上游核心实现提交：`1ba1e39d0d91ebf3c7b0c97647d148b7401bb459`
- 当前 custom checkpoint：`6ece4b7f3ffec0df6ce7645f623df858c08369f1`
- 交接文档：`C:\Users\m\Documents\Flow\References\会话交接\handoff-turn-recovery-design-after-20260917-2325.md`
- 当前工作区已知状态：设计文档未跟踪；源码无未提交修改（开始实现前再次以 `git status` 为准）。
- 完成条件：IMPLEMENT.md 经用户确认后，按 Phase 0→4 实现、验证、形成 Wave 4 记录；未确认前不得修改源码。
