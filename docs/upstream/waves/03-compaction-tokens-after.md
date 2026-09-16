# 波次 03：compaction tokensAfter result contract

## 分析目的

确认上游 `S..v0.3.0` 中 `tokensAfter` compaction 结果变化是否构成可以独立解释和验证的功能闭包，并核对当前 `custom` 的对应接入点。

本波次完成只读语义分析，不执行源码移植、merge 或 cherry-pick。是否进入实现，等待用户决定。

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

上游新增的是**compaction 完成后下一次上下文规模的结果契约**：在 compaction 后估算 `tokensAfter`，并将其传播到 RPC/SDK 的 compaction result payload。当前 `custom` 已有 compaction 结果、RPC compact 路径和 SDK 结果类型，但只保留 `tokens_before`；因此该主题具备独立的低风险候选闭包，推荐作为 Wave 3 继续进行实现决策。不过，上游提交同时带入了若干 grep/search 依赖，是否属于该功能仍未确认，不能原样 cherry-pick。

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
CompactionResult 增加 tokens_after
    ↓
RPC compact result 传播
    ↓
SDK RpcCompactionResult 序列化/反序列化
    ↓
旧 payload 默认值兼容
    ↓
focused compaction / SDK 单元测试
```

该闭包不要求同时迁移 provider streaming、session backend、interactive UI、Agent turn recovery 或完整 RPC 协议。

## 当前 custom 的对应实现

当前 `custom` 已有可直接对照的 compaction 结构：

```text
src/compaction.rs
  CompactionResult 当前包含 tokens_before，但没有 tokens_after

src/rpc.rs
  compact result 当前主要传播 tokensBefore

src/sdk.rs
  RpcCompactionResult 当前没有 tokens_after
```

依据当前 `custom` 工作树的定位：

- `src/compaction.rs:100-106`：`CompactionResult` 的现有字段；
- `src/compaction.rs:774-817`：当前上下文估算路径；
- `src/rpc.rs:1985-1990`、`src/rpc.rs:4676-4680`：RPC compact result 的现有输出路径；
- `src/sdk.rs:589-597`：`RpcCompactionResult` 的现有 SDK 类型。

这些位置说明当前不是完全没有 compaction result，而是缺少 post-compaction estimate 的结果字段和跨边界传播。

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
Wave 3 只读分析完成。
推荐：进入 tokensAfter 的实现设计/实现决策。
当前：未移植、未 merge、未 cherry-pick，等待用户确认。
```

如果用户确认实现，下一阶段应先处理依赖因果和 custom 协议差异，再建立最小实现计划；不应直接执行：

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
- tokensAfter 候选范围、路径和 custom 接入点调查。

本波次未执行：

- 源码修改；
- merge 或 cherry-pick；
- provider/compaction 真实网络验证；
- cargo 测试、clippy、fmt；
- release 构建或部署。

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
status: analysis-complete
requires_user_decision: true
implementation_started: false
```
