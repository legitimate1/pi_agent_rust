# Step 1 — RPC contract foundation and pure tree query — 实现计划

> **Build Target:** `rpc-session-tree/step-1/rpc-contract-and-tree-query`
> **来源决策：** `docs/design/rpc-session-tree/decision.md` — `rpc-session-tree-v1`
> **来源设计：** `docs/design/rpc-session-tree/README.md`
> **计划总览：** `docs/design/rpc-session-tree/README.md`
> **当前步骤：** Step 1
> **当前任务记录：** 待用户确认；只规划，未授权执行

## 计划身份

- **计划目的：** 建立 tree RPC 的公共错误码承载和只读 session-tree snapshot，使 Pidian 可以查询当前 live session 的完整 entry graph，而不触发 session mutation 或 entry ID 生成。
- **目标项目：** `C:\Users\m\Project\pi_agent_rust`
- **设计权威：** `docs/design/rpc-session-tree/README.md` — RPC Session Tree Surface
- **计划权威范围：** 本计划只决定 Step 1 的 RPC envelope 扩展、tree read model、纯读查询、preview/resubmit 映射和查询契约测试；不决定或实现 `tree_navigate` mutation。

## 实现边界

### 本次目标

完成后，RPC 客户端可以发送：

```json
{"id":"tree-1","type":"get_session_tree"}
```

并获得一个来自同一 live-session snapshot 的：

```text
sessionId + activeLeafId + entries
```

其中 `entries` 按 `Session.entries` 的 canonical/persistent order 返回，entry ID 已存在且稳定；tree 查询不会生成 ID、修改内存 entry、写入 JSONL 或返回部分树。tree 相关预期错误可以在通用 response envelope 顶层携带稳定 `code`。

### 包含

- 在 RPC 层增加 `RpcErrorCode` 或等价的专用错误码映射，不复用 `Error::category_code()`。
- 扩展通用 RPC error response，使 `code` 为可选顶层字段，同时保持旧命令缺少 `code` 的兼容性。
- 将 `get_session_tree` 注册为只读 RPC command，不把它加入会改变 live session 的 advancement/transition 路径。
- 在同一个 outer `AgentSession` / inner `Session` 读锁边界内构造原子 tree snapshot。
- 校验每个可返回 entry 都有稳定 ID；查询不得调用 `ensure_entry_ids()`，不得临时编号，不得静默持久化。
- 返回所有当前 session entries，而不是只返回 active path；保留 `parentId` 和 canonical array order。
- 为 message、compaction、branch summary、model/settings、custom 等当前 entry 类型定义开放 `kind` 和安全 bounded single-line `preview` 映射。
- 对 `SessionMessage::User` 和 `SessionMessage::Custom` 返回完整可编辑 `resubmitText`；对未知或不可重提交类型不返回可操作的 resubmit text。
- 为 `get_session_tree` 的 invalid/session-unavailable/quarantined 等预期失败接入稳定 code；`unknown_target` 留给 Step 2。
- 增加 Step 1 的单元/集成契约测试。

### 不包含

- `tree_navigate` command。
- `targetLeafId`、`expectedSessionId`、`expectedLeafId` 的 mutation 校验。
- `Session::navigate_to()`、`Session::reset_leaf()` 的 RPC 接入。
- persistence mutation、Agent context replacement、session-switch lifecycle event。
- branch summary 生成、`tree_resubmit`、subscription/revision protocol。
- Pidian 客户端代码；本计划只保证其 RPC-facing response contract。
- 修改 JSONL/session persistence 格式。
- 查询时为缺失 ID 的 legacy entry 生成迁移 ID。

### 前置条件

- `decision.md` 已确认，公共 tree RPC 语义已冻结。
- `README.md` 已记录 `RpcErrorCode`、纯读 entry ID 边界和 v1 error taxonomy。
- 已核查现有 `src/rpc.rs` command loop、common response helpers、transition authority、`src/session.rs` entry model，以及 `src/interactive/tree.rs` 的 preview/resubmit 语义。
- 当前工作区已有设计文档改动必须保留，不得 stash、reset、clean 或覆盖。

## 当前代码事实

- `src/rpc.rs#run`：RPC 命令按 stdin 顺序处理；command type 经 `normalize_command_type()` 后进入统一 match；只读 command 不应通过 `command_can_advance_rpc_session()` 的 session-advancement 保护路径。
- `src/rpc.rs#response_ok`、`response_error`、`response_error_with_hints`：当前 response envelope 有 `type`、`command`、`success`、`error` 和可选 `errorHints`，没有顶层稳定 request-level `code`。
- `src/rpc.rs#rpc_session_transition_snapshot_from_guard`：现有 inner session lock 模式可作为 snapshot 读取的锁边界参考，但 tree query 不应把 transition baseline 当作完整 tree DTO。
- `src/rpc.rs#rpc_session_transition_blocker` 与 `acquire_rpc_session_transition`：属于 Step 2/3 的 mutation authority，本计划只保持其行为不变，不接入 tree query。
- `src/session.rs#Session`：`header.id`、`entries`、`leaf_id()` 和 `SessionEntry` 元数据可用于构造 read model；`entries` 保留 session store 的持久化顺序。
- `src/session.rs#SessionEntry`、`EntryBase`、`SessionMessage`：entry ID 和 parent ID 是可选字段类型；正常加载路径会 finalization，但查询不能调用 `ensure_entry_ids()` 进行补 ID。
- `src/session.rs#Session::ensure_entry_ids` / `finalize_loaded_entries`：会就地补缺失 ID 并重建缓存，不是纯读 helper；不得在 `get_session_tree` handler 中使用。
- `src/interactive/tree.rs#build_display_nodes` / `describe_entry`：已有 user/custom 完整文本提取和 preview 分离语义，可作为 RPC 映射参考，但 TUI 的展示层级、过滤和固定 60 宽度不直接成为 RPC 契约。
- `tests/e2e_rpc.rs#assert_ok` / `assert_err`：已有 RPC response assertions 和 TestHarness/send_recv 测试入口，可扩展 tree command 的端到端 envelope 验证。

## 不变量映射

### I1 — Tree query 是同一 session snapshot 的原子读

- **设计来源：** `README.md` 的 “Frozen v1 contract / get_session_tree”、Invariants。
- **代码落点：** `src/rpc.rs#run` 的 `get_session_tree` 分支；必要时新增 `src/rpc.rs` 内部 tree snapshot builder 或 `src/session.rs` 的窄只读 helper。
- **证据测试：** `tests/e2e_rpc.rs` 的 tree query snapshot 测试；必要时 `src/rpc.rs` 的 read-model unit test。
- **本次保护方式：** 在同一 outer/inner session lock 生命周期内读取 `header.id`、`leaf_id()` 和完整 `entries`，再统一序列化 response；不得分别跨锁读取。
- **失败处理：** 锁失败返回带稳定 `session_unavailable` 或 `invalid_request` code 的失败 envelope，不返回混合或部分 snapshot。

### I2 — 查询不会生成、修改或持久化 entry ID

- **设计来源：** `decision.md` Key choice 11；`README.md` Stable entry ID boundary。
- **代码落点：** tree snapshot builder；`src/session.rs#Session` 的只读 ID/integrity 检查（如需要新增窄 `pub(crate)` helper）。
- **证据测试：** 构造带缺失 ID entry 的 session，调用 query 后确认 response 失败、原 session entry 仍未被补 ID、未发生 save；正常加载的稳定 ID session 查询成功。
- **本次保护方式：** 只读取已有 `base_id()`；发现缺失 ID 时按现有 session-integrity 状态返回 `session_unavailable` 或 `quarantined`，不返回部分树。
- **失败处理：** 禁止 fallback 到临时 ID、数组下标或自动 save；实现若发现加载边界无法证明 ID 稳定，必须暂停并回到设计阶段。

### I3 — Wire order 和 parent relation 不被展示逻辑替换

- **设计来源：** `README.md` 的 canonical/persistent order 与 flat-on-wire 选择。
- **代码落点：** tree entries serializer；`Session.entries` 遍历顺序和 `entry.base().parent_id` 映射。
- **证据测试：** 线性和分支 session query 验证所有 entry 都返回、数组顺序保持 session order、parentId 关系保持原值。
- **本次保护方式：** 不在 server 端构造 depth-first tree，不按 active path 过滤，不按 UI filter 过滤。
- **失败处理：** 无法安全读取 entry metadata 时返回 session error，不擅自重排或丢弃 entry。

### I4 — preview 安全有界，resubmitText 保留完整文本

- **设计来源：** `README.md` 的 Preview and resubmission text；Pidian client selection semantics。
- **代码落点：** `src/rpc.rs` 或窄 read-model 模块中的 preview helper、`SessionMessage` 到 wire entry 的映射。
- **证据测试：** 多行、超长 user/assistant/tool/bash/unknown 内容的 preview 单行和有界测试；user/custom 完整文本不被 preview 截断；不可重提交 entry 不带可操作 resubmitText。
- **本次保护方式：** preview 使用命名常量和统一安全 helper；resubmitText 单独从原始可编辑内容提取，不复用 preview 截断。
- **失败处理：** 如果某一 entry 类型无法安全生成 bounded preview，则使用安全 fallback/`other`，不得返回无限制 payload；如果无法保留 user/custom 完整文本，暂停并返回设计阶段。

### I5 — Error code 是 RPC 层请求语义，旧 RPC 保持兼容

- **设计来源：** `decision.md` Key choice 9；`README.md` Error contract。
- **代码落点：** `src/rpc.rs#response_error*` 和新增 `RpcErrorCode` mapping。
- **证据测试：** tree query 预期失败包含顶层 `code`；既有非-tree RPC response 仍满足当前 envelope assertions；`error`/`errorHints` 仍可读。
- **本次保护方式：** `code` 仅在有明确映射时输出，旧命令不被强制改写；Pidian 可兼容缺失 code。
- **失败处理：** 禁止以错误字符串作为 code；发现现有 helper 不能在不破坏旧测试的情况下扩展时，暂停并回到设计阶段。

## 当前工作单元

本计划只包含一个工作单元：Step 1 的 RPC contract foundation and pure tree query。

### W0 — Pure `get_session_tree` read model and RPC error-code foundation

- **目标：** 新增可独立验收的 `get_session_tree` 查询和通用错误 `code` 承载；完成后 Pidian 能获得完整、稳定、纯读的当前 session tree snapshot。
- **前置依赖：** 无；设计确认和源码核查已完成。
- **可修改范围：** `src/rpc.rs`；必要时 `src/session.rs` 的窄只读 entry-ID/integrity helper；`tests/e2e_rpc.rs` 和/或 `src/rpc.rs` 的相关测试模块；不修改 Pidian 仓库。
- **禁止扩大：** 不实现 `tree_navigate`，不调用 transition mutation，不修改 JSONL 格式，不新增 summary/subscription，不改变 TUI behavior。
- **相关不变量：** I1、I2、I3、I4、I5。
- **验证：** 针对 tree query、error code、entry ID purity、preview/resubmit text 和 branched ordering 的 targeted tests；随后运行 `cargo fmt --check` 和本计划列出的最小 Cargo test 集合。
- **完成条件：** query success/expected failure envelope 与设计示例一致；snapshot 三元组同锁读取；所有 entry 均来自当前 session order；缺失 ID 不被补写；preview/resubmit 规则有测试保护；旧 RPC 测试不回归；没有引入 Step 2 mutation 代码。
- **并行关系：** 无。W0 内部步骤必须按 S1 → S2 → S3 → S4 顺序完成。

内部执行顺序：

```text
W0 / S1 先写 error-code 与 pure-query 契约测试
  ↓
W0 / S2 实现 RpcErrorCode/envelope 扩展和 tree read model/preview helper
  ↓
W0 / S3 接入 get_session_tree command，并核对 stable-ID/integrity failure
  ↓
W0 / S4 运行 targeted test、旧 RPC 回归和格式检查
```

## 文件与符号变更清单

- **C1 — `src/rpc.rs`：修改**
  - **符号：** `response_error` / `response_error_with_hints` 相关 helper；新增 RPC 层 `RpcErrorCode` 或等价 mapping；`normalize_command_type`/command dispatch；tree snapshot/read-model helper。
  - **用途：** 承载顶层可选 `code`，注册 `get_session_tree`，在单一 session snapshot 内构造 wire DTO，并实现 preview/resubmit 映射。
  - **关键改动：** 新增 `get_session_tree` success/failure responses；为 tree query 失败提供 `invalid_request`、`session_unavailable` 或 `quarantined`；只读取稳定 entry IDs；返回 canonical order。
  - **预期保持：** 既有 command normalization、旧 response shape、transition authority、prompt/streaming/recovery 行为不变。

- **C2 — `src/session.rs`：条件修改，仅在需要时**
  - **符号：** `Session` 的窄只读 entry-ID/integrity 查询 helper，或现有可证明等价的公开/`pub(crate)`访问边界。
  - **用途：** 让 RPC 判断稳定 ID/session integrity，而无需调用会补 ID 的 `ensure_entry_ids()` 或暴露无关持久化细节。
  - **关键改动：** 只提供读取；不得改变 entry、缓存或 persistence 状态。
  - **预期保持：** session load/save、entry ID finalization 和 JSONL 格式保持不变。

- **C3 — `tests/e2e_rpc.rs`：修改**
  - **符号：** 新增 `get_session_tree` success/error tests 及 response-code assertions；复用现有 `TestHarness`、`send_recv`、`assert_ok`/`assert_err`。
  - **用途：** 验证 Pidian-facing wire contract、canonical order、branch completeness、stable IDs、preview/resubmit text 和 failure envelope。
  - **关键改动：** 增加线性/分支 session fixture、missing-ID purity、invalid/quarantined query failure 和 optional legacy code compatibility assertions。
  - **预期保持：** 既有 RPC e2e tests 和 command response semantics 不变。

- **C4 — `src/rpc.rs` 测试模块：条件修改**
  - **符号：** preview helper、kind mapping、`RpcErrorCode` serialization/mapping 的 unit tests。
  - **用途：** 将纯函数边界与端到端 session fixture 分离，减少 e2e 测试对内容构造的重复。
  - **关键改动：** 测试单行化、有界 preview、开放 kind fallback 和 code 字符串稳定性。
  - **预期保持：** 不新增独立公共模块或改变非-tree RPC 行为。

## 验证计划

### 工作单元验证

- **W0 / S1：** 先添加失败的契约测试或最小测试骨架，明确 response `code`、原子三元组、稳定 ID 和 preview/resubmit 断言。
- **W0 / S2：** 运行 RPC/session 相关 unit tests，确认 helper 的 code mapping、kind mapping 和 preview bounds。
- **W0 / S3：** 运行新增 tree e2e tests，确认：
  - linear query 返回 sessionId、activeLeafId、所有 entries；
  - branched query 返回 inactive branches；
  - canonical order 与 parentId 保持；
  - user/custom 返回完整 resubmitText；
  - missing ID 不修改内存、不写盘；
  - expected query errors 带稳定 code。
- **W0 / S4：** 运行 `cargo fmt --check`；运行本计划新增测试和受影响的现有 `e2e_rpc`/`rpc` 测试集合。

### 集成验证

- `cargo test --lib rpc` 或项目中等价的 RPC library test filter：PASS 预期。
- `cargo test --test e2e_rpc <tree-test-filters>`：PASS 预期。
- `cargo test --test e2e_rpc <受影响的既有 RPC filters>`：PASS 预期。
- `cargo fmt --check`：PASS 预期。
- Step 1 不默认运行全量测试、`cargo clippy --all-targets` 或构建/打包/发布；如执行阶段发现需要扩大验证，必须记录原因。

### 当前环境人工检查

- 检查 success response 的 `sessionId` 不暴露 session file path 或 UI tab ID。
- 检查 tree response 中不出现 session JSONL 直接导出、无限制 tool payload 或临时 entry ID。
- 检查旧 RPC 错误仍可缺少 `code`，而 tree expected failures 不依赖错误字符串。

### 用户发布后验证

- 无。Step 1 不包含发布或部署；Pidian 联调属于后续集成验证范围。

## 偏差与停止规则

### 允许自行调整

- 在 `src/rpc.rs` 内将 read model helper 拆为若干纯函数或窄内部结构。
- 复用现有 session/message text helper，只要不改变设计中的 wire semantics。
- 调整测试 fixture 和测试文件位置，只要仍覆盖本计划的符号级边界。
- 选择一个命名的 bounded preview 常量，只要不泄露无限制输出、保持单行化，并在测试中以该常量验证上限；若需要将具体上限提升为公共契约，必须返回设计阶段确认。

### 必须记录的局部偏差

- 测试 harness 无法直接制造 missing-ID live session，改用 session unit fixture 或加载边界测试。
- 现有 quarantine 状态只能通过 mutation admission 获取，query 侧采用等价的 session-integrity error 映射。
- 为保持旧 RPC 完全兼容，新增 `response_error_with_code` 而不是改变所有旧 helper 的调用点。

### 必须暂停并返回设计阶段

- 需要在 query 中调用 `ensure_entry_ids()`、写 JSONL 或修改 live session 才能实现稳定 ID。
- 需要把 `code` 设为所有旧 RPC response 的强制字段，破坏已确认兼容策略。
- 需要改变 `sessionId`、entry ID、parentId、kind、preview 或 resubmitText 的公共语义。
- 发现无法在同一 snapshot 内读取 `sessionId`、`activeLeafId` 和 `entries`。
- 发现 user/custom 完整文本无法安全获得，且只能通过截断、持久化格式变化或新的安全边界解决。
- 需要实现 `tree_navigate`、summary、subscription 或其他 Step 2/3 内容才能完成当前目标。

## 未决项

- **具体 preview 上限数值** — 类型：`assumption`；处理：使用命名实现常量并以测试锁定有界/单行行为；如果要把具体数值作为跨客户端公共契约，先返回设计阶段确认。
- **quarantined query 的现有读取入口** — 类型：`non-blocking`；处理：优先复用现有 session-integrity/provider-admission 状态；若源码证明 query 无法观察该状态，记录为实现偏差并保持不返回部分树。
- **C2 是否需要新增 Session helper** — 类型：`non-blocking`；处理：执行时先复用现有公开只读字段/方法，只有无法保持纯读边界时才增加最窄 `pub(crate)` helper。
- **Pidian adapter 的具体错误类实现** — 类型：`deferred`；处理：本 Step 只冻结并输出 RPC envelope；客户端代码留给后续跨项目集成任务。

## 当前任务记录

- **计划：** 待用户确认
- **实现：** 未开始
- **自动化验证：** 未执行
- **实际变更与计划偏差：** 待执行后记录
- **验证证据与遗留风险：** 待执行后记录
