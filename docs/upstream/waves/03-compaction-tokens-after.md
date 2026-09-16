# 波次 03：compaction tokensAfter result contract

## 分析目的

确认上游 `S..v0.3.0` 中 `tokensAfter` compaction 结果变化是否构成可以独立解释和验证的功能闭包，并核对当前 `custom` 的对应接入点。

本波次已完成只读语义分析，并在用户确认后按当前 `custom` 结构完成手动适配和验证。未执行整段上游 merge 或 cherry-pick。

## 分析边界

宏观研究窗口：

```text
S = 226a876425a856f657b2a5d7c7ac6f0ca1ad25f1
T = v0.3.0 = e23c4622f8bc4038a5e061ee3640a0e9206ec5cc
```

本波次功能范围：

```text
129cf9fe88598439b4717b17002d6110266c03b3^
..
129cf9fe88598439b4717b17002d6110266c03b3
```

核心提交：

```text
129cf9fe88598439b4717b17002d6110266c03b3
feat(compaction): report tokensAfter post-compaction context estimate
```

该提交最终差异为 10 个文件、约 `+162 / -6`。固定 SHA 仍可复核；当前 `upstream/main` 已经继续前进，不能将移动分支当作本波次终点。

Wave 1 的 fsqlite session storage 和 Wave 2 的 provider usage/quota 已排除。Wave 2 的 custom 适配、conformance fixture 和 coverage matrix 登记也不属于本波次。

## 一句话结论

上游新增的是**compaction 完成后下一次上下文规模的结果契约**：在 compaction 后估算 `tokensAfter`，并将其传播到 Agent/RPC/SDK 的 compaction result payload。当前 `custom` 已按自身 session replay、Agent、RPC 和 SDK 边界完成手动适配；保留当前依赖基线和 session 持久化 schema，不原样 cherry-pick 上游提交。

## 上游最终变化

### Compaction 结果

上游在 compaction 完成后估算压缩后的上下文规模，并把结果作为 `tokensAfter` 暴露给调用方。它回答的是：

```text
本次 compaction 完成后，下一次 provider request 预计还剩多少上下文 token
```

它不是 provider 账户 quota，也不是本次响应已经消耗的 token usage；因此与 Wave 2 usage/quota 和现有 `model::Usage` 保持不同语义。

### RPC/SDK payload

变化穿过以下最小路径：

```text
compaction result
    ↓
RPC compact result
    ↓
SDK RpcCompactionResult
    ↓
调用方可读取 tokensAfter
```

上游还为旧 payload 的反序列化提供默认行为，避免新增字段使已有协议数据无法读取。具体字段名称和序列化形状应以固定提交 `129cf9fe...` 的 `src/rpc.rs`、`src/sdk.rs` 为准，不把当前移动分支的实现当作本波次证据。

### 同提交中的依赖变化

该提交还改变了 `Cargo.toml`/`Cargo.lock`，引入或保留了以下搜索相关依赖：

```text
globset
grep-regex
grep-searcher
```

当前证据尚不能确认这些依赖是否是 `tokensAfter` 的直接前置，还是同一时期并行搜索能力的混入。依赖图变化不能随功能字段自动接受，后续实现前必须先完成因果拆分或单独评估。

## 功能闭包

本波次的最小闭包是：

```text
compaction 后上下文估算
    ↓
Agent/RPC 成功结果传播
    ↓
SDK RpcCompactionResult 序列化/反序列化
    ↓
旧 payload 默认值兼容
    ↓
focused compaction / Agent / RPC / SDK 测试
```

本波次采用 wire-only 方案，不把 `tokens_after` 加入 `CompactionResult` 或 session entry。该闭包可以单独解释为“报告 compaction 后下一次 provider request 的上下文规模”，不要求同时迁移 provider streaming、session backend、interactive UI、Agent turn recovery 或完整 RPC 协议。

## 当前 custom 的对应实现

Wave 3 已按当前 `custom` 结构完成实现，核心提交为：

```text
c73154c0d feat: add post-compaction token estimator
53753e629 fix: align post-compaction token estimation
93bc41e14 feat: expose tokensAfter in RPC compaction results
a0ff33cd3 feat: add tokensAfter to SDK compaction result
1fcca8dfa test: cover Agent tokensAfter payload
5fffd7162 test: cover RPC tokensAfter payload
```

实际实现文件：

```text
src/compaction.rs
src/agent.rs
src/rpc.rs
src/sdk.rs
tests/sdk_unit.rs
```

实现边界：

- `src/compaction.rs` 按实际 provider replay 后的消息内容估算 tokensAfter，复用现有 chars/3 heuristic；summary、branch summary、bash execution 使用 replay 转换后的文本，`excludeFromContext` 和 stale assistant usage 按既有语义处理；
- Agent normal、synchronous 和 extension hook compaction 成功结果均在 append 后生成 `tokensAfter`；
- RPC 手动 `compact` 和 auto-compaction 成功结果均输出 `tokensAfter`；
- SDK `RpcCompactionResult` 增加 `tokens_after`，通过 camelCase 映射为 `tokensAfter`，旧 payload 缺失字段时默认 `0`；
- 不修改 `CompactionResult` 的 pre-apply 生命周期，不修改 `CompactionEntry`、session JSONL schema、`Cargo.toml` 或 `Cargo.lock`；
- 未引入上游同提交中与搜索后端相关的 `globset`、`grep-regex`、`grep-searcher` 依赖。

## 语义差异与适配边界

### 可以局部复用的部分

- 当前 custom 已有 compaction 执行和结果类型；
- 当前已有 RPC compact result 输出路径；
- 当前已有 SDK serde 类型和相关单元测试入口；
- 当前 custom 的 runtime、session 和 provider streaming 不需要为该字段改变；
- 新字段可以作为 additive result field 处理，前提是确认旧 payload 的默认/缺省语义。

### 必须单独核对的部分

1. `tokensAfter` 的估算口径：是字符估算、tokenizer 估算，还是下一次 request 的特定上下文估算。
2. 上游字段的 serde 命名：Rust 的 `tokens_after` 与 RPC/SDK 外部的 `tokensAfter` 是否通过既有 rename 规则映射。
3. 缺少字段时的默认值：应区分“旧客户端没有该字段”和“估算失败/不可用”。
4. RPC 与 SDK 是否必须同步修改，还是 custom 的 SDK 边界已有不同协议版本。
5. 同提交 grep/search 依赖是否可以不带入，或需要另立依赖迁移议题。
6. 测试是否覆盖旧 payload 反序列化、字段缺省、compact result 序列化和估算结果传播。

### 明确排除

本波次不纳入：

- BPE/tokenizer 全面迁移：`b91f6a3c0054312f57439794d38010a368b4f921`、`5a4908afb0250fffd0884c05c60dcbda69050ac3` 相关变化；
- Wave 1 fsqlite session storage；
- Wave 2 provider usage/quota；
- turn recovery；
- FTUI transcript/rendering；
- MCP/RPC transport 或协议生命周期重构；
- workspace trust；
- provider streaming 和 session persistence；
- 未经因果核对的 `globset`、`grep-regex`、`grep-searcher` 依赖迁移。

## 相邻候选与选择理由

在同一宏观窗口中还确认了几个候选，但本波次不纳入：

- turn recovery：`1ba1e39d...^..d256c241...`，会改变 Agent turn 终止和自动续跑状态机；
- workspace trust：`17faf856...^..17faf856...`，跨配置、包解析和扩展自动发现安全边界；
- typed subagent results：`072583e3...^..7c99674...`，当前 custom 已有大量相邻实现，应先做相对差异审计；
- MCP client：`952fb3bd...^..5a9cc051...`，涉及协议、传输、认证、信任和生命周期；
- FTUI foundation：`41dfdb7e...^..7afc9655...`，约 212 个文件，是结构性 UI/runtime 迁移。

相比这些主题，`tokensAfter` 的代码闭包更小、当前 custom 对照点更明确，也不直接改变 Agent 状态机或安全边界。因此推荐它作为 Wave 3 的下一决策主题。

## 处理结论

```text
Wave 3 已实现并完成验证。
处理方式：按 custom 结构手动适配，不执行整段上游 merge 或 cherry-pick。
依赖：保留 custom Cargo.toml/Cargo.lock 基线，未引入搜索依赖。
```

本波次实际落地：

- 新增与 `Session::to_messages_for_current_path()` 转换结果一致的 post-compaction token estimator；
- Agent normal、synchronous 和 extension hook compaction 成功结果增加 `tokensAfter`；
- RPC 手动 `compact` 和 auto-compaction 成功结果增加 `tokensAfter`；
- SDK `RpcCompactionResult` 增加 `tokens_after`，旧 payload 缺失字段时默认 `0`；
- 保持 `CompactionResult` 的 pre-apply 生命周期、`CompactionEntry` 和 session JSONL schema 不变；
- 未采纳上游同提交中与 in-process grep/find 相关的依赖变化。

不执行：

```text
git cherry-pick 129cf9fe...
```

## 停止边界

后续设计或实现调查应停止在：

```text
compaction estimate
    → RPC compact result
    → SDK RpcCompactionResult
    → focused serialization/backward-compatibility tests
```

遇到以下任一情况应停止扩大并记录阻塞：

- `tokensAfter` 依赖不可拆分且必须引入主依赖代际变化；
- 上游估算口径与 custom 当前 compaction 语义无法对应；
- RPC/SDK 现有版本边界要求同时迁移更大的协议闭包；
- 失败无法区分为本波次变化还是 custom 原有基线问题。

## 验证摘要

本波次已完成：

- refs 刷新；
- 固定 `S..v0.3.0` 对象复核；
- 宏观净变化和 first-parent 里程碑调查；
- Wave 1/2 排除；
- tokensAfter 候选范围、路径和 custom 接入点调查；
- custom 结构手动实现；
- estimator、Agent、RPC、SDK 的 focused 测试；
- 收尾质量门禁。

### 已通过验证

以下命令均通过，并通过 Windows `pwsh` 执行：

```text
cargo check --lib
cargo clippy --lib -- -D warnings
cargo fmt --check
cargo test --lib compaction::tests
  97 passed
cargo test --lib apply_compaction_result_emits_structured_result_payload
  1 passed
cargo test --lib rpc_compact_success_payload_contains_tokens_after
  1 passed
cargo test --lib rpc_auto_compaction_success_event_contains_tokens_after
  1 passed
cargo test --test sdk_unit rpc_compaction_result_serde
  1 passed
cargo test --test compaction
  139 passed
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### 全量测试结果

全量 `cargo test` 共通过 6475 个测试，1 个 ignored，另有 5 个失败。5 个失败随后逐个以 `cargo test --lib <name> -- --exact` 复跑，均稳定复现，且与本波次修改文件无交集：

```text
jobs::tests::cancel_kills_running_job
  Windows 进程终止升级测试未观察到 KILL escalation

hub::tests::logs_cursor_advances_incrementally
hub::tests::send_text_drives_repl
hub::tests::status_stays_running_for_live_repl
  Windows PTY readiness 失败，日志尾部为 ESC [6n 终端探测序列

hub::tests::restart_after_completion_works
  Windows PTY 子进程状态/退出码观察失败
```

这些失败属于当前 Windows 环境或既有基线行为，不是本波次新增失败；本波次未修改 `src/jobs.rs` 或 `src/hub.rs`，也未为此扩大处理范围。

### 未执行

- provider/compaction 真实网络验证；
- release 构建或部署；
- `release-max` 构建。

## 证据入口

- 宏观窗口：`docs/upstream/analysis-window.md`
- 波次索引：`docs/upstream/wave-plan.md`
- 跨波次决策：`docs/upstream/upstream-decisions.md`
- Wave 1：`docs/upstream/waves/01-fsqlite-session-storage.md`
- Wave 2：`docs/upstream/waves/02-provider-usage-quota.md`
- tokensAfter 核心提交：`129cf9fe88598439b4717b17002d6110266c03b3`
- 当前 custom compaction：`src/compaction.rs`
- 当前 custom RPC：`src/rpc.rs`
- 当前 custom SDK：`src/sdk.rs`

## 状态

```text
status: implemented
requires_user_decision: false
implementation_started: true
implementation_commits:
  - c73154c0d
  - 53753e629
  - 93bc41e14
  - a0ff33cd3
  - 1fcca8dfa
  - 5fffd7162
closeout: focused-pass-full-test-baseline-failures
```
