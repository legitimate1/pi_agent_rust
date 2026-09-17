# Turn recovery 子系统

## 定位

Turn recovery 负责在 provider 正常结束 assistant turn、但结果表现为明显未完成时，有限次数地自动继续当前工作。它位于共同 Agent loop 内，覆盖 interactive、print、RPC、SDK 和 ACP 入口。

它不是 provider/network 错误重试，也不是隐藏的请求重放。错误重试仍由各入口现有的 retry wrapper 和 `revert_incomplete_response` 负责。

## 模块关系

- **`src/turn_recovery.rs`** — 纯分类、模式门控、logical-run budget 和 nudge 文本生成；不访问 provider、session 或入口协议。
- **`src/agent.rs`** — 持有 `AgentConfig.turn_recovery`，在共同 Agent loop 的 turn 边界调用分类器，并把 action 转换成普通 user message；`LogicalRunScope` 由调用方持有。
- **`src/config.rs`** — 读取 `settings.json` 的 `turn_recovery`，执行 global/project merge，并在缺省时解析为 Conservative。
- **`src/main.rs` / `src/rpc.rs`** — 每个独立 prompt 创建 scope；同一高层 provider retry/resume 将 scope 传给后续 continuation。
- **`src/sdk.rs` / `src/acp.rs`** — 将最终 mode 传入 AgentConfig；独立宿主 prompt/continue 使用新的 logical-run scope。
- **`src/interactive/agent.rs`** — interactive 解构 AgentSession 使用裸 Agent 时，显式为独立 continue 创建 scope。
- **`src/session.rs`** — 仍是 session JSONL 的唯一 Pi-owned 写入边界；recovery 不新增 session entry 或专用 flush。

## 模式契约

配置值为：

```text
off          → TurnRecoveryMode::Off
conservative → TurnRecoveryMode::Conservative
aggressive   → TurnRecoveryMode::Aggressive
```

缺少 `turn_recovery` 时解析为 `Conservative`。未知值必须沿现有 serde settings 错误路径失败，不得静默回退或启用更激进模式。

- **Off** — 不自动续跑。
- **Conservative** — 处理 provider `Length` 和明确未闭合结构。
- **Aggressive** — 在 Conservative 基础上处理 semantic promise heuristic。

## 分类契约

分类器输入是 provider stop reason 和 assistant 的可见文本。文本由 Agent 层只提取 `ContentBlock::Text`，多个 text block 以换行连接；thinking、redacted thinking、image、tool call 和 tool arguments 不参与分类。

分类规则：

- `StopReason::Length` → `BudgetTruncated`。
- `StopReason::Stop` 加奇数个 code fence delimiter → `UnclosedStructure`。
- `StopReason::Stop` 加末尾裸 `-`、`*`、`+` 或数字点号列表项 → `UnclosedStructure`。
- `StopReason::Stop` 加末尾 semantic promise → `SemanticPrematureStop`。
- 普通 Stop → `CleanStop`。
- `ToolUse`、`Error`、`Aborted` 和未来未识别 stop reason → `CleanStop`。

promise heuristic 只检查 assistant 文本末尾 240 个字符，先去除末尾空白并转小写，再查找固定短语：

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

短语后若存在明显后续句子或段落结束，则不分类为 semantic premature stop。该判断是启发式，不是 Markdown parser 或语义模型。

## Agent loop 契约

共同循环的顺序必须保持：

1. provider 正常返回 assistant message。
2. assistant message 和既有 message/event 生命周期完成。
3. 若存在 tool call，先执行 tool；该 turn 不进入 recovery。
4. 发出 `TurnEnd`。
5. drain steering；有 steering 时 steering 优先，不追加 recovery nudge。
6. 仅在无 tool call 且无 pending steering 时调用 recovery evaluate。
7. 有 action 时注入普通 `Message::User(UserMessage)`，然后进入下一 provider turn。
8. recovery 完成后才进入 idle follow-up staging。

provider `Error`、`Aborted`、provider stream error、abort 和 tool execution abort 继续使用现有错误、取消和 retry/failover 路径。不能把失败 provider attempt 当作正常 assistant turn 分类，也不能回放已经执行的 tool call。

## Scope 与 budget

`LogicalRunScope` 是一次高层逻辑 prompt/continue 的内部所有权边界，不能只放在底层 `run_loop` 局部变量，也不能作为 Agent 的长期全局字段。

同一个 scope 覆盖：

```text
普通 provider attempt
→ recovery continuation
→ provider error retry/resume
→ 同一高层调用内的后续 continuation
```

新建 scope 的场景：

- 新的用户 prompt；
- 新的宿主 prompt；
- SDK/interactive/ACP 的独立 continue；
- 新的 print/RPC prompt wrapper。

复用 scope 的场景：

- 同一高层 prompt wrapper 中的 provider/network retry；
- `revert_incomplete_response` 后对失败 provider request 的 resume。

每个 scope 最多产生两条 recovery nudge。clean stop 不消耗额度。第一次 recovery 之后，结构和 semantic heuristic 不再重复触发；剩余额度只允许 provider 明确返回 `Length` 时使用。provider retry 不增加 recovery continuation，也不得重置 scope 中的 recovery budget。

## Nudge、事件和持久化

nudge 的固定格式为：

```text
[auto-continue {n}/2: {reason}] Continue from exactly where you stopped. Do not repeat content you already produced; finish the remaining work.
```

它是普通 user message，因此会：

- 进入 Agent 内存 transcript；
- 经过既有 `MessageStart` / `MessageEnd`；
- 出现在后续 provider context；
- 随既有 `TurnStart` / `TurnEnd` 序列产生后续 turn；
- 在正常高层调用结束时沿既有 persistence 路径保存。

不新增：

- recovery 专属 AgentEvent；
- RPC/ACP recovery wire 字段；
- session JSONL entry 类型或 schema 字段；
- SDK result 字段；
- recovery 专用 session flush。

`save_enabled=true` 时，nudge 和 continuation 在 AgentSession 或 interactive 的既有批量阶段保存。`save_enabled=false` 时，它们只保留在内存 transcript、provider context 和 event stream 中，不写 session JSONL。进程若在既有批量保存前崩溃，尚未 flush 的消息可能丢失，这是当前 eventual persistence 契约的一部分。

## 入口边界

- **Interactive / print** — 使用共同 Agent loop；当前 TUI 不渲染 recovery 专用提示，最终 continuation 沿现有输出路径展示。
- **RPC** — 使用已有 AgentEvent/message/turn event 流；客户端必须接受同一 prompt 内多个既有 turn 的事件序列；事件 handler 不直接写 session JSONL。
- **SDK** — prompt 由 AgentSession 管理 fresh scope；公开独立 continue 显式创建 fresh scope；不扩展 SDK wire/result。
- **ACP** — session 创建时接收最终 mode；内部可以产生多组 turn 生命周期，但 wire 继续沿用既有 message/tool `session/update` 映射；不增加 recovery 字段或新的 stop reason。

## 已知陷阱

- **不要把 `Length` 当成错误重试** — `Length` 是正常 provider result 的可能截断信号，应走 recovery classifier；network/provider error 才走 retry wrapper。
- **不要用 `run_continue_with_abort` 方法名推断 scope** — 该方法可能表示独立 continue 或 retry resume；调用方必须选择 fresh scope 或显式 shared-scope API。
- **不要把 state 放进 Agent 字段** — Agent 会跨多个独立 prompt 存活，持久 state 会导致新 prompt 错误继承旧 budget。
- **不要在 tool call turn 追加 nudge** — 已执行工具不能重放；tool call turn 必须先完成既有工具路径。
- **不要让 steering 被 recovery 插队** — steering 是实时用户/宿主输入，必须在 recovery evaluate 前消费。
- **不要让 follow-up 提前消费** — recovery action 注入后要先完成 continuation，再进入 idle follow-up。
- **不要在 RPC handler 中第二次写 session** — nudge 由 Agent transcript 和 Session autosave 统一落盘，避免双写和索引/锁语义分裂。
- **不要主动在 recovery 中触发 compaction** — nudge 导致的 context overflow 走既有 error/retry/final error 路径，不生成新的 recovery nudge。
- **不要宣传为零成本 retry** — nudge 是真实上下文内容，会增加 token、请求成本和未来 compaction 负担。
- **不要把启发式当成确定语义** — code fence、列表和 promise 判断都可能误报或漏报，模式开关和两次 cap 是必要的资源边界。

## 扩展指南

新增 recovery 分类时：

1. 先在 `RecoveryClass` 和 `TurnRecoveryState` 中定义分类、mode gating 和 cap 行为。
2. 为 stop reason、可见文本、模式和连续 run 边界增加纯单元测试。
3. 只有无 tool/steering 的正常 assistant turn 才能接入 Agent loop。
4. 复用普通 Message/event/transcript/persistence 路径，不新增 wire 或 session schema。
5. 为 provider retry、abort、tool call、steering、follow-up 和独立 scope 增加负路径回归。
6. 更新 `features.md` 指针、`architecture.md` 摘要和本文件的契约/坑表；若新增命令、规则或路由，再更新 `AGENTS.md`。

如果未来需要 recovery 专属 observability，必须单独设计事件/协议边界，不能通过隐式字段改变当前普通 message contract。
