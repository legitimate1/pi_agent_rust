# 波次 02：provider usage/quota surface

## 分析目的

确认上游在 `S..v0.3.0` 中新增的 provider usage/quota 查询能力是否构成独立功能闭包，核对当前 `custom` 的接入基础，并判断后续应直接采纳、局部适配还是冻结。

本波次先完成只读语义分析，随后在用户确认后按当前 `custom` 结构完成手动适配；不执行整段上游 merge，也不修改 Cargo 配置。

## 分析边界

宏观研究窗口仍为：

```text
S = 226a876425a856f657b2a5d7c7ac6f0ca1ad25f1
T = v0.3.0 = e23c4622f8bc4038a5e061ee3640a0e9206ec5cc
```

第一执行子窗口 `T1 = 8f12352e174ea06c7d8d66cde15768cecfccccf3` 的 fsqlite session storage 分析已在波次 01 完成，本波次不重复分析它。

provider usage/quota 功能闭包固定为：

```text
起点：f6be31dad19a7a82239c1a367c5c7eb897d8e9a9 的父提交
终点：d7d1c31177299c8ead788897ad6fece1b498edd6
可复核范围：f6be31dad19a7a82239c1a367c5c7eb897d8e9a9^..d7d1c31177299c8ead788897ad6fece1b498edd6
```

功能核心提交：

```text
f6be31dad19a7a82239c1a367c5c7eb897d8e9a9
feat(usage): provider usage/quota readers and /usage surface (bd-cv653.7.4)
```

闭合提交：

```text
d7d1c31177299c8ead788897ad6fece1b498edd6
chore(beads): close bd-cv653.7.4; log gel2u class-1 addendum
```

该功能范围统计为 10 个变更文件、约 `+650 / -4`；其中核心实现文件为 `src/usage.rs`、`src/cli.rs`、`src/interactive/commands.rs`、`src/interactive/perf.rs`、`src/lib.rs` 和 `src/main.rs`，其余主要是 `.beads` 跟踪元数据。

该闭包位于 `T1` 之后、`v0.3.0` 之前。它没有因 fsqlite 而改变 session storage，也没有把同时间段的其他提交自动纳入本波次。

## 一句话结论

上游新增的是**provider 账户额度/余额查询闭包**：在已有认证和 HTTP 基础上，为 OpenRouter、Moonshot/Kimi、GitHub Copilot 等 provider 查询 credits、balance 或 entitlement/quota，并通过 `pi usage` 与 `/usage` 展示。当前 `custom` 已通过手动适配获得这项能力，同时保留了 custom 的认证、HTTP、交互事件和依赖边界；Wave 2 不采用上游提交的直接 Git 合并。

处理建议：

```text
已适配
```

这表示上游 usage/quota 语义已经以本地手动适配的方式进入 `custom`，不是对上游提交执行原样 merge。provider endpoint 的真实网络兼容性仍需单独验证。

## 上游最终变化

### 独立 usage 模块

上游终点新增 `src/usage.rs`，形成统一的只读 usage reader 层。关键符号包括：

```text
USAGE_SCHEMA = "pi.usage.v1"
USAGE_CACHE_TTL = 60 seconds
USAGE_FETCH_TIMEOUT = 8 seconds

ProviderUsage
UsageStatus
UsageReader

OpenRouterUsageReader
MoonshotUsageReader
CopilotUsageReader

readers_from_auth(...)
gather_usage(...)
render_usage_text(...)
render_usage_json(...)
```

上游的 provider-specific 行为是：

- OpenRouter 读取 credits；
- Moonshot/Kimi 读取 balance；
- GitHub Copilot 读取 entitlement/quota；
- 没有公开 quota endpoint 的 provider 返回 `Unavailable`，不猜测或伪造额度；
- 单个读取有 8 秒请求超时；
- 结果在内存中缓存 60 秒；
- 请求失败或过慢时，可以回退到带年龄信息的缓存结果；
- 结果支持文本和 JSON 渲染；
- credential 不写入日志。

这些行为说明该模块是查询和呈现层，不是新的 provider streaming、session persistence 或 quota 持久化系统。

相关证据位置：

```text
d7d1c311...:src/usage.rs:22
  usage schema

d7d1c311...:src/usage.rs:26
  cache TTL

d7d1c311...:src/usage.rs:29
  fetch timeout

d7d1c311...:src/usage.rs:31
  usage types

d7d1c311...:src/usage.rs:57
  reader abstraction

d7d1c311...:src/usage.rs:84
  provider readers

d7d1c311...:src/usage.rs:361
  auth-to-reader construction

d7d1c311...:src/usage.rs:418
  gather path

d7d1c311...:src/usage.rs:471
  rendering path
```

行号属于上游终点快照，后续上游移动后应以固定 SHA 重新核对，不把行号当作永久接口。

### CLI surface

上游在 `src/cli.rs` 增加 usage 子命令：

```text
Commands::Usage {
    format: String,
    refresh: bool,
}
```

`src/main.rs::handle_subcommand` 的处理路径为：

```text
AuthStorage::load(Config::auth_path())
    ↓
usage::gather_usage(&auth, refresh)
    ↓
render_usage_json(...) 或 render_usage_text(...)
```

支持强制刷新，以及文本/JSON 两种输出方向。该入口不改变 Agent turn、session message 或 provider 主请求流程。

上游证据位置：

```text
d7d1c311...:src/cli.rs:2201-2212
  Usage 子命令定义

d7d1c311...:src/main.rs:2368-2375
  CLI dispatch

d7d1c311...:src/lib.rs:284
  pub mod usage
```

### Interactive surface

上游在 `src/interactive/commands.rs` 增加：

```text
SlashCommand::Usage
/usage parser branch
/usage [refresh] help text
handle_slash_usage(...)
```

交互式路径异步取得结果后，将文本投递到 transcript/status 展示；它不直接追加 session 消息，不改变 Agent turn 状态机。

上游证据位置：

```text
d7d1c311...:src/interactive/commands.rs:16
  SlashCommand 枚举

d7d1c311...:src/interactive/commands.rs:99
  命令解析

d7d1c311...:src/interactive/commands.rs:138
  帮助文本

d7d1c311...:src/interactive/commands.rs:906
  命令处理辅助路径

d7d1c311...:src/interactive/commands.rs:2305
  usage 交互处理
```

当前证据只确认 CLI 和 interactive surface；本波次不扩大为 RPC usage 协议。

## 功能闭包

本波次纳入的最小闭包是：

```text
AuthStorage credential 读取
    ↓
provider-specific quota/credit readers
    ↓
ProviderUsage / UsageStatus 统一结果
    ↓
超时、缓存、stale fallback、Unavailable/Error 分类
    ↓
文本/JSON 渲染
    ↓
pi usage CLI
    ↓
/usage interactive command
    ↓
usage reader 和展示路径测试
```

该闭包可以单独解释为“读取并展示 provider 账户额度”，不要求同时迁移 provider streaming、agent loop、session backend 或扩展运行时。

## 当前 custom 的对应实现

Wave 2 已按当前 `custom` 结构完成手动适配。实际实现位于：

```text
src/usage.rs
src/lib.rs
src/cli.rs
src/main.rs
src/interactive/commands.rs
src/interactive/agent.rs
```

实现提交：

```text
32ca3f25d feat: adapt provider usage quota
bc395caae fix: harden provider usage integration
f8db80205 test: stabilize provider usage cache coverage
```

其中：

- `src/usage.rs` 提供三个 provider reader、统一状态、超时、缓存、stale fallback 和 text/JSON renderer；
- `src/cli.rs` 和 `src/main.rs` 提供 `pi usage [--format text|json] [--refresh]`；
- `src/interactive/commands.rs` 提供 `/usage` 和 `/usage refresh`；
- 交互结果使用 `PiMsg::SystemNote`，不会把异步 usage 结果当成 Agent 完成事件，也不会重置正在处理的 Agent 状态；
- cache identity 使用 provider 与 credential 的不可逆 SHA-256 摘要，避免不同账号共享 fresh/stale quota；
- CLI 和 interactive 的 auth 文件读取使用 `AuthStorage::load_async`，不在 runtime 中直接执行同步读取；
- `src/interactive/agent.rs` 增加了 `SystemNote` 保持 Processing 状态的回归测试。

### 可复用的底层接入点

当前 `custom` 已具备与 usage 闭包相邻的底层能力：

```text
src/auth.rs:339
  AuthStorage

src/auth.rs:497
  AuthStorage::api_key(...)

src/auth.rs:592
  AuthStorage::resolve_api_key(...)

src/config.rs:418
  Config::auth_path(...)

src/http/client.rs:206
  Client

src/http/client.rs:245
  impl Client
```

这些接入点覆盖认证文件读取、provider credential 查找、统一认证路径和异步 HTTP client；本次适配已实际复用它们完成 usage/quota 接入。

### 已实现闭包

当前 `custom` 已具备 usage/quota 闭包。实现不等于上游提交原样合入，而是保留 custom 结构后完成的本地适配。

已实现的接口包括：

```text
Commands::Usage
SlashCommand::Usage
handle_slash_usage(...)
pub mod usage
```

当前 CLI 定义和分发入口仍分别位于：

```text
src/cli.rs:1889
  pub enum Commands

src/main.rs:1828
  handle_subcommand(...)

src/interactive/commands.rs:14
  pub enum SlashCommand

src/interactive/commands.rs:1673
  handle_slash_command(...)
```

### 不应混淆的现有能力

当前 custom 已有 provider response 中的 token 使用量统计，例如：

```text
src/model.rs:219
  model::Usage

src/interactive/commands.rs:1651-1652
  total token/cost 展示
```

这类数据回答“本次请求消耗了多少 token/cost”，而 usage/quota 模块回答的是：

```text
账户 credits
账户 balance
entitlement
remaining quota
```

两者的来源、生命周期和失败语义不同。当前 custom 同时保留原有的 `model::Usage` token 统计和新增的 usage/quota 账户查询，不能将二者混为一谈。

## 语义差异与适配边界

### 可以局部复用的部分

以下部分已在本次适配中核对并完成：

- `AuthStorage` 的 credential 读取；
- 当前 HTTP client 的异步请求、headers、JSON body 和请求级 timeout；
- CLI 子命令注册和 `handle_subcommand` 分发；
- interactive slash command 的解析、帮助和展示结构；
- serde/JSON 输出基础；
- usage cache 的 credential-scoped identity 与 stale fallback；
- async runtime 中的 auth 文件读取。

仍需保留为外部或后续边界的核对项：

- OpenRouter、Moonshot/Kimi、GitHub Copilot endpoint 和响应格式的真实线上可用性；
- 是否需要未来单独暴露 RPC usage surface；上游本闭包没有 `src/rpc.rs` 变化，因此当前不纳入。

### 应明确排除

本波次不纳入：

- `model::Usage` 的 token 消耗解析；
- provider streaming 主流程改造；
- retry/failover 和 turn recovery；
- session persistence、session index 和 picker；
- extensions、MCP、VFS 和核心 runtime 结构；
- fsqlite 或任何 SQLite 后端迁移；
- provider quota 持久化数据库；
- TLS/HTTP 主依赖代际升级；
- 对没有公开 endpoint 的 provider 猜测性实现。

## 与相邻候选的关系

### tokens-after compaction result

`129cf9fe88598439b4717b17002d6110266c03b3` 的 subject 是：

```text
feat(compaction): report tokensAfter post-compaction context estimate
```

该提交是 `T1` 的祖先，且与 fsqlite 同日但无 session 后端因果关系。它的范围为 compaction、RPC/SDK payload 和相关测试，未来可作为独立的 compaction result contract 波次，但不属于本波次。

当前 custom 的 `RpcCompactionResult` 仍主要包含 `tokens_before`，不能把手工测试数据中的 `tokensAfter` 字符串当作完整实现证据。该事实不改变它被排除出波次 02 的时间和语义边界。

### turn-recovery

上游候选范围为：

```text
1ba1e39d0d91ebf3c7b0c97647d148b7401bb459^..
d256c241a37fd1ad9cc64bd27b73a80d788f3b11
```

核心行为是 unexpected-stop 分类和最多两次自动续跑，涉及 `src/turn_recovery.rs`、`src/agent.rs`、`src/config.rs`、`src/main.rs`、`src/sdk.rs`、`src/acp.rs` 及测试。

它直接改变 Agent turn 终止/继续语义，并与 custom 已有的 `run_continue_with_abort`、`revert_incomplete_response`、retry/failover 和消息持久化边界相交。因此暂不优先，建议冻结为后续 Agent 状态机恢复闭包，不与 usage 混合。

### workspace trust

`17faf856a0f4749d08a34b6799b95ea63e6cb13c` 的 subject 是：

```text
feat(security): workspace trust-on-first-use gate for project-local .pi config
```

它控制 project settings merge、package resolution、`.pi/extensions` 自动发现、非交互 fail-closed、trust digest 和 CLI `--trust`，跨越配置、包管理和扩展加载安全边界。虽然可以独立立项，但范围和风险高于 usage，暂不作为波次 02。

### typed subagent results

上游 typed subagent results 的核心范围位于：

```text
072583e35d61d4aeae563188f21173ff51d74181
..
7c99674a87d132422d4c0bdf336bef71aa9959ce
```

当前 custom 的 `src/subagents.rs` 已有 `output_schema`、`SchemaMode`、schema validation、retry 和 structured result 相关实质实现。虽然尚未在本波次重新完成全部相对差异审计，但它不是优先于 usage 的新闭包；后续只需比较相对 custom 的新增差异。

### FTUI、MCP/RPC 和 plan mode

FTUI foundation 属于结构性 UI/runtime 迁移；MCP/RPC 属于协议、生命周期和传输闭包；plan mode 交叉 approval、agent、RPC 和 FTUI 命令路径。它们均不应作为普通 usage 功能顺手吸收。

## 为什么不直接 merge

当前不直接 merge 的理由不是 Git 冲突数量，而是需要保持功能归因和 custom 边界：

1. 上游 usage 是新功能，但当前 custom 的认证、HTTP、CLI 和 interactive 结构不一定与上游接口相同。
2. provider endpoint 和响应格式属于外部契约，不能只凭 Git patch 认定在当前环境仍然可用。
3. 现有 token usage 与账户 quota 是不同语义，直接复用错误的数据模型会掩盖失败、不可用和缓存状态。
4. 上游本闭包没有依赖升级，适配可以保留 custom 的依赖基线；没有必要把它和其他依赖迁移绑在一起。
5. 未来是否需要 RPC usage surface 尚未决定，直接 merge 上游 CLI/interactive 形状可能越过 custom 的协议边界。

## 处理结论

```text
已适配
```

Wave 2 已按当前 custom 的认证、HTTP、CLI 和 interactive 结构完成手动移植，并修复了两个适配阶段发现的边界问题：

- usage cache 改为按 provider + credential 的不可逆摘要隔离，避免账号之间复用 fresh/stale quota；
- interactive usage 结果改用不改变 Agent 状态的 `PiMsg::SystemNote`，认证文件读取改用 `AuthStorage::load_async`。

本次处理没有执行上游 `git merge` 或保留未解决的 cherry-pick；没有引入 Cargo 依赖变化，也没有纳入 Wave 1、beads 元数据或相邻功能。

仍未完成的不是本地适配闭包，而是 provider endpoint 的真实网络验证和项目收尾阶段的全量质量门禁。

## 验证摘要

本波次已完成上游 Git 分析、custom 结构对照、隔离 cherry-pick 探针、手动源码适配和局部回归验证。

### Git 与适配证据

已确认：

- 工作区当前分支为 `custom`，适配前后均未将上游整段历史 merge 进来；
- `v0.3.0` 当前指向 `e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`；
- usage 上游功能核心范围为 `f6be31da^..d7d1c311`；
- 直接 cherry-pick 上游功能提交在隔离 worktree 中对 `src/interactive/commands.rs`、`src/interactive/perf.rs`、`src/lib.rs` 产生内容冲突，因此改为手动适配；
- 当前 custom 的本地适配提交为：

```text
32ca3f25d feat: adapt provider usage quota
bc395caae fix: harden provider usage integration
f8db80205 test: stabilize provider usage cache coverage
```

- 实际变更未修改 `Cargo.toml`、`Cargo.lock`、session、RPC、extensions、FTUI 或 provider streaming 主流程；
- `src/interactive/perf.rs` 未被修改，usage 展示复用了 custom 已有的 `PiMsg::SystemNote` 路径。

### 已执行验证

以下命令均通过，且均在 Windows `pwsh` 中执行：

```text
cargo test --lib usage::tests
  10 passed

cargo test --lib parse_usage
  3 passed

cargo test --lib slash_usage
  3 passed

cargo test --lib system_note_does_not_change_processing_state
  1 passed

cargo check --lib
  passed

cargo clippy --lib -- -D warnings
  passed

cargo fmt --check
  passed

git diff --check
  passed
```

测试覆盖 provider reader 解析、HTTP 错误脱敏、Unavailable/error 分类、credential-scoped cache、refresh、stale fallback、文本/JSON 渲染、CLI 参数和 interactive slash parser，以及 `SystemNote` 不改变 Processing 状态的回归。

### 未执行验证

- 未运行 provider 真实网络请求；
- 未运行全量 `cargo test`；
- 未运行 `cargo test --all-targets`；
- 未运行 `cargo clippy --all-targets`；
- 未运行 release-max 构建。

因此“已适配”表示本地实现和局部验证完成，不表示 provider 线上 endpoint 或完整发布门禁已经认证通过。

## 证据入口

- 宏观窗口：`docs/upstream/analysis-window.md`
- 波次索引：`docs/upstream/wave-plan.md`
- 跨波次决策：`docs/upstream/upstream-decisions.md`
- 波次 01：`docs/upstream/waves/01-fsqlite-session-storage.md`
- usage 核心提交：`f6be31dad19a7a82239c1a367c5c7eb897d8e9a9`
- usage 闭合提交：`d7d1c31177299c8ead788897ad6fece1b498edd6`
- 上游 usage 实现：`d7d1c311...:src/usage.rs`
- custom 认证：`src/auth.rs`、`src/config.rs`
- custom HTTP：`src/http/client.rs`
- custom CLI/interactive：`src/cli.rs`、`src/main.rs`、`src/interactive/commands.rs`
- custom token usage：`src/model.rs`、`src/interactive/commands.rs`
