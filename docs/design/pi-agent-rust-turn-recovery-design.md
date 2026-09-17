# pi-agent-rust — Turn recovery 自动续跑机制设计

## 目标与背景

上游在提交 `1ba1e39d0d91ebf3c7b0c97647d148b7401bb459` 中引入了 turn recovery：当 provider 正常结束了一次 assistant turn，但结果表现为 token budget 截断或明显未完成时，在同一次 Agent run 内注入一条有限次数的自动续跑提示，让模型从停止位置继续完成任务。

当前 `custom` 已有两类相邻但不同的能力：

- provider/network error 的 retry/failover；
- `run_continue_with_abort` 与 `revert_incomplete_response` 提供的失败请求恢复。

本设计只吸收“正常 `Stop`/`Length` 结果的有限自动续跑”，不把它混入错误重试，也不改变已完成的 Wave 1/2/3：

- fsqlite session storage；
- provider usage/quota；
- compaction `tokensAfter`。

设计依据：

```text
上游 ref：upstream/main = 82b7c0c8e72c7fed5cc1ab74ebccfe378419c9e6
核心实现：upstream/main:src/turn_recovery.rs
核心提交：1ba1e39d0d91ebf3c7b0c97647d148b7401bb459
当前 custom：6ece4b7f3ffec0df6ce7645f623df858c08369f1
```

⚠️ 本文是设计，不包含源码实现、merge 或 cherry-pick 计划。上游提交不能直接 cherry-pick，因为 `custom` 的 Agent loop、AgentConfig、session persistence 和交互层结构不同。

## 设计目标

1. 对正常结束的 assistant turn 识别有限且可解释的未完成信号。
2. 在单次 Agent run 内最多自动续跑两次，避免无限循环和请求费用失控。
3. 保留已经产生的 assistant 输出，并把续跑提示作为普通 user message 进入现有消息、事件和 session persistence 路径。
4. 让 interactive、print、RPC、SDK、ACP 复用同一个 Agent 层行为，不为每个入口复制 recovery 逻辑。
5. 明确隔离以下路径：

```text
provider/network Error 或 Aborted → 现有 retry/failover/revert 流程
正常 Stop/Length 但疑似未完成 → turn recovery
```

6. 不增加 session JSONL schema、RPC wire result、SDK compaction contract 或新的 Cargo 依赖。

## 非目标与边界

以下内容明确不在本设计内：

- 不处理 provider/network error 的自动续跑；
- 不重试 `Error`、`Aborted` 或取消的 provider request；
- 不回放已经执行过的 tool call；
- 不新增 `PauseTurn`、`Refusal` 等 StopReason 变体；
- 不修改 `CompactionResult`、`CompactionEntry` 或 `tokensAfter`；
- 不新增独立的 `AutoTurnRecoveryStart/End` AgentEvent；
- 不迁移 FTUI、MCP、workspace trust 或 session backend；
- 不把上游混合的 Files API、Gemini thinking config 或其他 provider 变化带入；
- 不在 recovery 内重新设计 follow-up、steering、tool approval 或 ACP permission 协议。

## 语义契约

### Recovery mode

新增 `TurnRecoveryMode`，其配置/编译契约固定为：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnRecoveryMode {
    Off,
    Conservative,
    Aggressive,
}

impl Default for TurnRecoveryMode {
    fn default() -> Self {
        Self::Conservative
    }
}
```

字符串映射固定为：

```text
off          → TurnRecoveryMode::Off
conservative → TurnRecoveryMode::Conservative
aggressive   → TurnRecoveryMode::Aggressive
```

未知配置值必须沿用现有 settings/serde 错误路径返回错误，不得静默回退到其他模式。

语义：

```text
Off          不自动续跑
Conservative 只恢复 token budget 截断和明确的结构未闭合
Aggressive   在 Conservative 基础上，额外恢复明显的“即将执行但没有执行”
```

建议配置键与上游保持一致：

```json
{
  "turn_recovery": "conservative"
}
```

配置值：

```text
off
conservative
aggressive
```

⚠️ 上游默认是 `Conservative`，本设计采纳该默认值。缺少 `turn_recovery` 字段的旧 settings 文件解析为 `Conservative`；用户可以显式配置 `"off"` 恢复关闭行为。该选择会改变部分旧配置的请求次数和 transcript，必须通过配置兼容性测试固定。

### 分类器

新增纯函数分类器，只根据 stop reason 与 assistant 可见文本进行判断：

```rust
pub fn classify(stop_reason: StopReason, text: &str) -> RecoveryClass
```

分类结果：

```rust
pub enum RecoveryClass {
    CleanStop,
    BudgetTruncated,
    UnclosedStructure,
    SemanticPrematureStop,
}
```

规则：

| 输入                                                                                   | 分类                    |
| -------------------------------------------------------------------------------------- | ----------------------- |
| `StopReason::Length`                                                                   | `BudgetTruncated`       |
| `StopReason::Stop` + 普通文本                                                          | `CleanStop`             |
| `StopReason::Stop` + 奇数个 code fence delimiter                                       | `UnclosedStructure`     |
| `StopReason::Stop` + 末尾悬空列表项（`-`、`*`、`+`、`2.`）                             | `UnclosedStructure`     |
| `StopReason::Stop` +未履行的执行承诺                                                   | `SemanticPrematureStop` |
| `StopReason::ToolUse`、`StopReason::Error`、`StopReason::Aborted` 或其他 custom reason | `CleanStop`             |

assistant 文本只取 `ContentBlock::Text`，忽略 thinking、tool call 和 tool arguments；多个文本 block 以换行连接。这样分类器依赖的是模型可见文本，而不是 provider 私有 wire payload。

未闭合结构和 promise 判断是启发式，不是 Markdown parser 或语义模型判断：

- code fence 使用 delimiter 奇偶判断；
- 列表只检查最后一行；
- promise 只检查文本末尾有限窗口和固定短语；
- promise 仅在 `Aggressive` 模式开启。

### Mode gating

| 分类                    |    Off | Conservative | Aggressive |
| ----------------------- | -----: | -----------: | ---------: |
| `CleanStop`             | 不续跑 |       不续跑 |     不续跑 |
| `BudgetTruncated`       | 不续跑 |         续跑 |       续跑 |
| `UnclosedStructure`     | 不续跑 |         续跑 |       续跑 |
| `SemanticPrematureStop` | 不续跑 |       不续跑 |       续跑 |

### State 生命周期与上限

新增 recovery state，并由一次高层逻辑 prompt/continue 调用的内部 scope 所有：

```rust
pub(crate) struct LogicalRunScope {
    pub(crate) recovery: TurnRecoveryState,
    // provider retry 可有独立计数，但不得重置 recovery。
    pub(crate) provider_retry: RetryState,
}
```

`LogicalRunScope` 是实现契约，不要求成为公开 API。它必须覆盖同一次高层调用中的：

```text
普通 provider attempt
→ recovery continuation turn
→ provider/network error retry
→ run_continue_with_abort / resume
```

约束：

- 每次新的**逻辑 prompt run**创建一个新的 `LogicalRunScope` 和 recovery budget；
- 同一次逻辑 run 中由 recovery 产生的后续内部 turn，以及因 provider/network error 触发的 retry/`run_continue_with_abort`，共享同一个 recovery budget；
- 底层的 provider retry attempt 不是新的逻辑 run，不得重置 recovery budget；
- provider retry 本身不增加 `continuations`，但必须复用同一个 scope；
- 一次新的、由用户或宿主独立发起的 prompt/continue 调用创建新的 `LogicalRunScope`；
- `continuations` 不写入 session，不跨独立逻辑 run 保留；
- 最多产生 `2` 条自动续跑 nudge；
- clean stop 不消耗额度；
- 达到上限后当前 assistant 结果作为最终结果；
- 第一次 recovery 后，结构启发式不应在每个新 assistant message 上无限重复触发；后续 provider 明确返回 `Length` 仍可使用剩余额度。

实现上，`TurnRecoveryState` 不能只以底层 `run_loop` 的局部变量表达。custom 的 `AgentSession`、interactive 裸 `Agent` 调用、print/RPC 外层 retry 和 SDK/ACP continue 入口都必须明确创建、传递或结束 `LogicalRunScope`：

```text
同一高层逻辑调用的 retry/resume → 复用 scope
新的独立用户/宿主 continue   → 创建新 scope
```

`run_continue_with_abort` 的调用语义必须由调用方显式标注：

```text
provider retry/resume → 接收现有 LogicalRunScope
用户或宿主主动 continue → 创建新的 LogicalRunScope
```

禁止通过“是否调用了 `run_continue_with_abort`”这一方法名自动推断 scope 类型。

### Nudge 消息

续跑提示使用普通 `Message::User(UserMessage)`，格式保持上游语义：

```text
[auto-continue {n}/2: {reason}] Continue from exactly where you stopped. Do not repeat content you already produced; finish the remaining work.
```

它不是隐藏 retry，也不是新的 session entry 类型。预期消息顺序为：

```text
原始 user prompt
assistant 截断/未完成输出
auto-continue user nudge
assistant continuation
```

因此 nudge 会：

- 经过现有 `MessageStart` / `MessageEnd` 和 `TurnStart` / `TurnEnd` 生命周期；
- 出现在 Agent transcript；
- 在 AgentSession 或 interactive 的正常完成路径中，被现有的新增消息批量持久化逻辑保存；
- 计入后续 provider request 的上下文；
- 间接影响未来 compaction 的上下文规模。

⚠️ custom 当前采用 run 结束时的 eventual persistence，而不是 nudge 注入后立即 flush：若进程在 nudge 注入后、整个 AgentSession/interactive 调用返回前崩溃，尚未批量保存的 nudge 或 continuation 可能丢失。本阶段不为 recovery 单独引入 session I/O 耦合；必须用成功、provider error、abort 和中途崩溃边界测试固定该语义。

不新增 recovery 专属事件。若调用方需要区分 recovery，只能依据 nudge 文本或后续新增的可选 observability 方案；后者不属于本阶段。

## Agent loop 集成

### 插入点

custom 的共同 Agent loop 位于：

```text
src/agent.rs
```

实现应围绕现有的：

```text
provider stream 完成
→ assistant message 完成
→ tool call 检测/执行
→ TurnEnd
→ steering drain
→ follow-up staging
→ AgentEnd
```

新增顺序建议为：

```text
provider 正常返回 assistant
→ 完成现有 assistant message/event
→ 若存在 tool call，继续现有 tool execution，不执行 recovery
→ 发出 TurnEnd
→ 优先处理 steering
→ 只有没有 pending steering 且没有 tool call 时 evaluate recovery
→ 若有 RecoveryAction，排入 synthetic user nudge
→ nudge 走普通下一轮 message flow
→ 下一次 provider request
→ recovery 完成后才进入 idle follow-up staging
```

核心规则：

```text
有 tool call       → 先走 tool execution
有 steering        → steering 优先，不额外插入 recovery nudge
无 tool/steering   → 才允许 recovery evaluate
无 recovery action → 继续现有 follow-up/AgentEnd
```

外层错误重试的边界：

```text
同一逻辑 prompt run 内 provider Error
→ 现有 revert/retry/run_continue_with_abort
→ 复用该逻辑 run 的 recovery budget
→ 不把 Error 本身交给 classifier

新的独立 prompt/continue 调用
→ 创建新的 recovery budget
```

⚠️ 插入时必须保持现有 abort、hook、tool approval 和 event 顺序。不能把 recovery 放入 provider error 分支，也不能在 `revert_incomplete_response` 之后复用同一个恢复入口。provider retry 只负责恢复失败的 provider attempt；它不能重置同一逻辑 prompt 的 recovery budget，也不能把一次失败 assistant attempt 当作 recovery 可分类的正常 assistant turn。

### assistant 文本提取

在 `src/agent.rs` 增加局部 helper，等价于：

```rust
assistant_text_content(content: &[ContentBlock]) -> String
```

只拼接 assistant 的文本 content。该 helper 不改变模型消息结构，也不改变 session serialization。

### AgentConfig 注入

`AgentConfig` 增加：

```rust
pub turn_recovery: TurnRecoveryMode,
```

并更新：

- `AgentConfig::default()`；
- `Debug` 输出；
- 所有显式 `AgentConfig { ... }` 构造点；
- 测试 builder / fixture helper；
- `examples/pi_debug.rs`；
- `src/agent.rs`、`src/sdk.rs`、`src/acp.rs` 内部测试；
- `tests/**/*.rs` 中的完整 struct literal。

入口传递：

```text
Config::turn_recovery_mode()
    → main 创建 AgentConfig
    → SDK 创建 AgentConfig
    → ACP 创建 AgentConfig
    → common Agent loop
```

RPC 不新增独立配置字段：它继续复用当前 RPC session 所使用的 AgentSession/AgentConfig。RPC retry wrapper 仍只处理错误重试。

## 配置合并与兼容性

在 `src/config.rs` 中增加：

```rust
turn_recovery: Option<TurnRecoveryMode>
turn_recovery_mode(&self) -> TurnRecoveryMode
```

缺少该字段时统一解析为：

```text
TurnRecoveryMode::Conservative
```

遵循现有 settings 合并方向：显式配置覆盖低优先级配置；缺失字段使用上述默认模式。

兼容约束：

- 既有 settings 文件缺少该字段时必须仍可读取；
- 非 recovery 配置不受影响；
- 未识别的模式值应沿用项目现有配置错误处理，不静默启用 aggressive；
- 不修改 RPC result schema；
- 不修改 SDK compaction result 或 `tokensAfter` schema；
- 不修改 session JSONL entry schema。

## 各入口行为

### Interactive / print

二者都通过 common Agent loop 获得 recovery。无需在 UI 层复制 classifier。

interactive 当前会将 `AgentSession` 解构为裸 `Agent` 使用，因此不能假设 `AgentSession` 自动持有 scope；`src/interactive/agent.rs` 必须显式创建、传递和结束 `LogicalRunScope`，或调用一个等价的内部 scope API。

预期：

- recovery nudge 可在 Agent internal transcript 和现有 event callback 中观察到；
- 最终 assistant continuation 沿现有输出路径展示；
- 当前 TUI 不额外渲染 recovery 专用提示，也不新增 UI 状态；
- nudge 不要求出现在用户屏幕上的独立对话气泡中；若未来需要该行为，另立 UI 议题；
- 不产生 `AutoRetryStart/End`；
- provider error 仍使用原有 retry/failover 行为。

### RPC

RPC 继续复用现有 AgentSession 调用路径：

```text
RPC prompt/continue
→ AgentSession
→ common Agent loop
→ recovery nudge/event
→ RPC event stream
```

不新增 `ask_request` 等协议面，也不新增 recovery result 字段。需要确认并测试 RPC 客户端能正确接受同一次 prompt 中多个 turn 的既有事件序列。

### SDK

SDK 只需要把配置值传入 AgentConfig；不新增 SDK wire result。SDK 的一次独立用户/宿主 `continue` 创建新的 `LogicalRunScope`；同一高层调用内部的 provider retry/resume 复用既有 scope，不得仅因调用 `run_continue_with_abort` 就重置 recovery budget。

### ACP

ACP session 创建时传入同一配置。内部 Agent 层仍可产生多组 `TurnStart` / `TurnEnd`，但当前 ACP wire 层只暴露既有 message/tool `session/update` 序列；不新增 `TurnStart` / `TurnEnd` wire 字段、recovery 字段或新的 StopReason。ACP 的一次独立 prompt 创建新的 `LogicalRunScope`，同一 prompt 内的 provider retry/resume 复用该 scope。

## 与 retry/failover 的隔离

| 维度           | 现有 retry/failover                         | 本设计 turn recovery                    |
| -------------- | ------------------------------------------- | --------------------------------------- |
| 触发           | provider/network `Error`、transient failure | 正常 `Stop` / `Length`                  |
| 目的           | 重发失败请求                                | 让模型完成未完成工作                    |
| assistant 残留 | 可能回滚 incomplete response                | 保留已完成 assistant 输出               |
| 消息           | 通常不是新的 user 请求                      | 追加普通 user nudge                     |
| 计数           | retry counter                               | per-run continuation counter            |
| 事件           | 现有 retry event（如适用）                  | 不新增专属事件                          |
| tool call      | 避免重复执行                                | 只在无 tool call 的 assistant turn 触发 |

实现禁止：

```text
把 recovery evaluate 放入 provider Err 分支
把 recovery 计数器复用 retry_max_retries
对 assistant StopReason::Error 生成 nudge
对已执行 tool call 的 turn 直接生成 nudge
```

## Compaction / tokensAfter 关系

本设计不修改 Wave 3 的实现：

- 不增加 `tokens_after` 到 `CompactionResult`；
- 不改 `CompactionEntry`；
- 不改 `SessionMessage::CompactionSummary`；
- 不在 recovery 内主动触发 compaction；
- 不新增 `tokensAfter` wire 字段。

recovery continuation 在当前 AgentSession 调用中不重新进入入口级 `maybe_compact`。因此 nudge 后的下一次 provider request 如果因新增上下文触发 context overflow，应沿现有 provider error / retry / final error 路径处理；不得因为该错误再次生成 recovery nudge。

间接影响是：nudge 和 continuation 会进入后续 transcript，并可能在未来 session run 的 compaction 输入中出现。该影响通过现有 replay/estimate 逻辑自然体现，不在本设计中引入特殊处理。

## 文件变更清单

| ID  | 路径                                    | 操作 | 用途                                                                                                                                                                                              |
| :-: | :-------------------------------------- | :--- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1  | `src/turn_recovery.rs`                  | 新增 | mode、分类器、state、nudge 和单元测试                                                                                                                                                             |
| C2  | `src/agent.rs`                          | 修改 | AgentConfig、LogicalRunScope、assistant 文本提取、loop 插入点和 recovery message 注入                                                                                                             |
| C3  | `src/config.rs`                         | 修改 | settings 字段、合并和最终 mode 解析                                                                                                                                                               |
| C4  | `src/lib.rs`                            | 修改 | 导出 `turn_recovery` 模块                                                                                                                                                                         |
| C5  | `src/main.rs`                           | 修改 | print/主入口 AgentConfig 传递、provider retry 与 LogicalRunScope 复用                                                                                                                             |
| C6  | `src/sdk.rs`                            | 修改 | SDK AgentConfig 传递、独立 continue 的 scope 边界和内部测试 literal                                                                                                                               |
| C7  | `src/acp.rs`                            | 修改 | ACP AgentConfig 传递、内部事件/wire 边界和测试 literal                                                                                                                                            |
| C8  | `src/interactive/agent.rs`              | 修改 | 裸 Agent 调用的 LogicalRunScope 传递、事件回归和 eventual persistence 回归                                                                                                                        |
| C9  | `src/agent.rs`、相关测试                | 修改 | Agent loop、session persistence、retry/resume、tool/steering/follow-up focused tests                                                                                                              |
| C10 | `tests/**/*.rs`、`examples/pi_debug.rs` | 修改 | 所有完整 `AgentConfig { ... }` literal、测试 fixture/builder、VCR/golden/event transcript 与各入口回归；与 recovery 无关的旧测试显式使用 `Off`，recovery 专用测试使用 `Conservative`/`Aggressive` |

fixture 迁移规则：

- 先定位全部 `AgentConfig { ... }` literal 和 fixture/builder，再逐一补齐字段；
- 与 recovery 无关、原本只验证既有行为的测试显式使用 `TurnRecoveryMode::Off`，避免默认 Conservative 改变其调用次数和 transcript；
- recovery 专用测试显式使用 `Conservative` 或 `Aggressive`；
- 仅在行为确实改变时更新 VCR、golden、event transcript 和调用次数断言，不对所有 fixture 无条件增加 nudge；
- 纯配置测试覆盖缺少字段、显式 `off`、`conservative`、`aggressive` 和未知值错误。

不修改：

```text
Cargo.toml
Cargo.lock
src/session.rs 的 schema/entry 类型
src/compaction.rs 的 tokensAfter contract
src/rpc.rs 的 wire protocol（除非测试需要最小 harness 调整）
```

## 实现依赖与阶段

### Phase 0：已确定的设计决策

以下决策已在本轮设计评审中采纳，不再作为实现前的开放问题：

1. 默认模式为 `Conservative`；显式 `"off"` 可关闭；
2. nudge 是可见、可持久化的普通 user message，接受 custom 的 eventual persistence 时点；
3. 首版实现 `Length` + `UnclosedStructure`，同时保留 upstream 的 `SemanticPrematureStop` 逻辑，并仅由显式 `Aggressive` 配置启用；
4. recovery 覆盖 interactive、print、RPC、SDK、ACP 全部入口；
5. steering/follow-up 优先级按 Agent loop 中的既有顺序固定；
6. recovery budget 以一次高层逻辑 prompt/continue 调用为边界：内部 recovery turn 与 provider retry/resume 共享 cap，新的独立调用重置 state；
7. recovery 不在 Agent loop 内主动触发 compaction；nudge 后 context overflow 走现有 error/retry/final error 路径。

💡 实现阶段仍需根据实际符号位置确定具体测试文件，但不得改变以上语义决策。

### Phase 1：纯机制模块

C1 先完成：

- enum 与默认值；
- classify；
- state evaluate；
- nudge 格式；
- 分类、模式和 cap 单测。

C1 不依赖 Agent loop，可独立验证。

### Phase 2：Agent loop

C2 与 C4：

- 将模块接入 lib；
- 修改 AgentConfig；
- 在无 tool/steering 的 turn-end 边界调用 evaluate；
- 注入普通 user nudge；
- 保持已有 event/persistence 路径。

### Phase 3：入口配置与回归

C3、C5、C6、C7、C8、C10：

- settings merge 与默认值；
- main、SDK、ACP 配置传递；
- interactive 裸 Agent 的 LogicalRunScope 传递和 nudge internal transcript 回归；
- 补充 interactive/print/RPC/SDK/ACP 的最小行为测试；
- 更新 `examples/pi_debug.rs`；
- 更新 `src/agent.rs`、`src/sdk.rs`、`src/acp.rs` 内部测试，以及 `tests/**/*.rs` 中所有完整 `AgentConfig { ... }` literal、测试 fixture 和 builder；
- 与 recovery 无关的旧测试显式使用 `TurnRecoveryMode::Off`，recovery 专用测试显式使用 `Conservative`/`Aggressive`；
- 仅在行为确实改变时更新 VCR、golden、event transcript 和调用次数断言。

### Phase 4：质量验证

日常验证：

```pwsh
cargo test --lib turn_recovery
cargo test --lib <Agent recovery focused test>
cargo test --test <受影响的单个测试文件>
cargo clippy --lib -- -D warnings
cargo fmt --check
```

全部实现完成后按项目收尾规则验证：

```pwsh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Windows 下所有 Cargo 命令必须通过 `pwsh` 执行。不自动构建、不自动部署。

## 测试设计

### 纯分类器

覆盖：

- `Length` → `BudgetTruncated`；
- 空文本/普通 `Stop` → `CleanStop`；
- 奇偶 code fence；
- `-`、`*`、`+`、数字点号悬空项；
- 完整列表项不触发；
- promise heuristic 的具体契约：仅检查 assistant 文本末尾 240 个字符，先 `trim_end`、转小写，再查找以下短语的最后一次出现：

```text
i will now
i'll now
let me now
i am going to
i'm going to
next, i will
next, i'll
now i will
now i'll
proceeding to
```

- promise phrase 后只剩标点、空格或换行，且没有明显后续句号/段落结束时，分类为 `SemanticPrematureStop`；
- `StopReason::ToolUse`、`StopReason::Error`、`StopReason::Aborted` 不触发；
- 三种 mode gating。

### State / cap

覆盖：

- 两次 recovery 后第三次不再续跑；
- clean stop 不消耗额度；
- 第一次结构 recovery 后，后续结构判断不会无限重复；
- 剩余额度允许明确 `Length` recovery；
- 新逻辑 prompt/continue run 的 state 从零开始；
- recovery → provider Error → retry → recovery 的组合路径仍共享同一 logical-run cap；
- provider retry 不会把同一逻辑 run 的 recovery 次数重置；
- 新的独立 prompt/continue 调用会创建新的 recovery budget。

### Agent loop

使用 fake provider 验证：

```text
第一次 Length → 第二次 Stop
```

断言：

- provider 调用次数为 2；
- transcript 包含一条 auto-continue user message；
- assistant 原始输出和 continuation 都保留；
- 最终结果按第二次 assistant turn 结束。

再验证 `Length` 连续返回时最多 2 次 nudge，不能无限请求。

### 优先级与负路径

覆盖：

- 有 tool call：不直接 recovery；
- 有 steering：steering 优先，不额外插入 nudge；
- follow-up：recovery 先完成，再进入 idle follow-up；
- provider error：仍走 retry/failover，不生成 nudge；
- abort before/during provider：不生成 nudge；
- tool execution abort：不生成 nudge；
- mode `Off`：行为与现有 custom 保持一致。

### Event / persistence

验证现有事件序列中出现普通 nudge message，而不是新事件：

```text
assistant turn end
→ next turn start
→ nudge MessageStart/MessageEnd
→ continuation assistant turn
```

验证 session replay 顺序：

```text
user
→ assistant partial
→ auto-continue user
→ assistant continuation
```

验证 session persistence 的实际时点和异常边界：

- 成功完成整个 AgentSession/interactive 调用后，nudge 与 continuation 都被批量保存；
- provider error、abort 后按现有错误/结束路径检查已保存消息；
- 在 nudge 注入后、调用返回前发生中断时，确认并记录 eventual persistence 允许丢失尚未 flush 的消息；
- 不引入 recovery 专用 session flush。

### Surface 回归

至少为 common Agent loop、RPC event stream、SDK session 和 ACP session 做一条配置/行为回归；如果现有测试 harness 无法稳定驱动真实 provider，则使用 fake provider 或纯配置/事件断言，不引入真实网络依赖。

## 注意事项

⚠️ 这是启发式自动行为，不是 provider 精确判断。必须保留清晰的模式开关和每次 run 的上限。

⚠️ nudge 是真实 transcript 内容。它可能增加上下文 token、请求成本和未来 compaction 负担，不能宣传为“零成本 retry”。

⚠️ 不要用 `StopReason::Length` 作为错误重试信号；它在本设计中表示一次正常完成但可能被 token budget 截断的 assistant turn。

💡 首次实现采用 `Conservative` 语义；`Aggressive` promise heuristic 保留为显式配置，不默认启用。

💡 上游没有为 recovery 增加独立事件。保持这一点可以避免 RPC/SDK/ACP 协议扩张，但必须通过事件顺序测试固定可观察行为。

## 实现阶段边界（非开放项）

本设计的核心语义已定稿。以下仅记录实现阶段需要保持一致的边界，不是待用户选择的方案：

1. **首版分类范围**：实现 `Length` + `UnclosedStructure`；保留 `SemanticPrematureStop`，但仅由显式 `Aggressive` 配置启用；
2. **nudge 可见性与持久化**：接受 nudge 出现在 Agent internal transcript、RPC event stream、SDK message list 和持久化 session 中，采用 custom 的 eventual persistence；当前 TUI 不额外渲染 recovery 专用提示；
3. **覆盖面**：统一放在 common Agent loop，覆盖 interactive、print、RPC、SDK、ACP，不做入口差异化；interactive 裸 Agent 调用显式传递 `LogicalRunScope`；
4. **配置暴露**：沿用 settings + 已有入口配置对象，不新增 CLI flag、RPC wire surface 或 SDK result 字段；
5. **compaction 交互**：recovery 不主动触发中途 compaction；nudge 后 context overflow 走现有 error/retry/final error 路径；
6. **logical-run scope**：provider retry/resume 复用同一 scope，provider retry 不增加 recovery `continuations`；新的独立用户/宿主 prompt/continue 创建新的 scope；
7. **持久化条件**：`save_enabled=true` 时在高层调用结束的既有批量阶段保存 nudge/continuation；`save_enabled=false` 时只保留内存 transcript、provider context 和 event stream，不写 session JSONL；
8. **ACP 事件边界**：内部 Agent 层可以产生多组 `TurnStart`/`TurnEnd`，ACP wire 层只沿用既有 message/tool `session/update`，不新增 turn/recovery wire 字段。

实现过程中如果发现这些已定语义与 custom 的实际调用链存在冲突，应停止实现并报告，而不是自行改变设计契约。

## 设计状态

```text
status: final
implementation: not started
review: round 1 issues incorporated; final review completed by main agent
upstream endpoint: 82b7c0c8e72c7fed5cc1ab74ebccfe378419c9e6
```
