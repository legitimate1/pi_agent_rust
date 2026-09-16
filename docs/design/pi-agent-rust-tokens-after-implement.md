# IMPLEMENT.md — Wave 3 `tokensAfter`

## 1. 实现边界（Implementation Contract）

Source: `docs/upstream/waves/03-compaction-tokens-after.md`

### Goal

在 compaction 成功写入 session 后，沿用 custom 现有的 chars/3 heuristic，估算下一次 provider request 将看到的上下文规模，并将其作为 `tokensAfter` 暴露给 Agent/RPC/SDK 调用方。

### In scope

- 新增与当前 session replay 边界一致的 post-compaction token estimator。
- Agent 自动 compaction 成功结果增加 `tokensAfter`。
- Agent extension hook compaction 成功结果增加 `tokensAfter`。
- RPC `compact` 成功响应增加 `tokensAfter`。
- RPC auto-compaction 成功事件增加 `tokensAfter`。
- SDK `RpcCompactionResult` 增加 `tokens_after`，并映射为外部 `tokensAfter`。
- 旧 SDK payload 缺少 `tokensAfter` 时默认反序列化为 `0`。
- 补充 compaction、Agent/RPC payload 和 SDK serde 的针对性测试。

### Out of scope

- 不修改 `CompactionResult` 的 pre-apply 生命周期。
- 不修改 `CompactionEntry`、session JSONL 持久化 schema 或 `SessionMessage::CompactionSummary`。
- 不迁移 tokenizer/BPE，不引入新依赖。
- 不修改 `model::Usage`、provider streaming、session backend、FTUI、turn recovery、MCP、workspace trust 或搜索后端。
- 不引入或升级 `globset`、`grep-regex`、`grep-searcher`。
- 不 cherry-pick 或 merge 上游 `129cf9fe...`。

### Assumptions

- `tokensAfter` 是 heuristic estimate，不是 provider 精确 token count 或计费数据。
- 估算必须遵循 `Session::to_messages_for_current_path()` 的 compaction replay 边界：最新 summary 加上保留的 current-path entries，不包括已压缩旧历史或 sibling branch。
- 估算复用 `src/compaction.rs` 现有 `estimate_tokens`，并忽略保留 assistant message 中的旧 provider usage。
- 新 producer 对成功 compaction 始终输出数值 `tokensAfter`；旧 producer 的缺失字段仅在 SDK 读取时默认成 `0`，不回退到 `tokensBefore`。

### Design delta

上游固定提交是在 session apply 后直接构造 wire payload；custom 现有 `CompactionResult` 在 `compact()` 返回时已经创建，而此时尚未 append 到 session。因此采用 wire-only 方案：不把 `tokens_after` 加入 `CompactionResult`，而是在 append 后计算并写入 Agent/RPC payload。

---

## 2. 文件变更清单（Change Manifest）

| ID  | 路径                                            | 操作 | 用途                           | 主要改动                                                                                                            |
| :-: | :---------------------------------------------- | :--- | :----------------------------- | :------------------------------------------------------------------------------------------------------------------ |
| C1  | `src/compaction.rs`                             | 修改 | 提供 post-compaction estimate  | 新增只读 helper，按最新 compaction、first-kept 边界和现有 `estimate_tokens` 计算 `u64`；增加边界和 stale usage 测试 |
| C2  | `src/agent.rs`                                  | 修改 | Agent auto-compaction 结果传播 | 让 apply 路径在 append 后返回估算值；normal、synchronous 和 extension hook 成功 payload 写入 `tokensAfter`          |
| C3  | `src/rpc.rs`                                    | 修改 | RPC compact 结果传播           | 手动 `compact` 和 auto-compaction 在 append 后计算并输出 `tokensAfter`                                              |
| C4  | `src/sdk.rs`                                    | 修改 | SDK wire model                 | `RpcCompactionResult` 增加 `#[serde(default)] pub tokens_after: u64`                                                |
| C5  | `tests/sdk_unit.rs`                             | 修改 | SDK 兼容性测试                 | 增加新字段 round-trip 和 legacy payload 默认值测试                                                                  |
| C6  | `tests/compaction.rs` / 相关 Agent/RPC 测试位置 | 修改 | 对外行为回归                   | 按现有测试组织补充估算和成功 payload 断言；不新增真实网络依赖                                                       |

> 以实际符号定位为准。如果现有 Agent/RPC 测试没有可复用的稳定 harness，只在最接近的现有单元测试位置增加纯 payload/estimator 断言，不为了测试而重构 RPC runtime。

---

## 3. 依赖关系（Dependency Plan）

### Dependencies

- C1：无；先确定 estimator 的输入和 replay 边界。
- C2：依赖 C1；调用 estimator 并调整 Agent apply/payload 顺序。
- C3：依赖 C1；调用 estimator 并调整 RPC payload 顺序。
- C4：无；SDK 类型可与 C1 并行，但字段契约必须与 C2/C3 的 wire spelling 一致。
- C5：依赖 C4；验证新字段和旧 payload 默认值。
- C6：依赖 C1、C2、C3、C4；按最终调用链补齐回归断言。

### Phase

- Phase 0：确认当前 custom 工作区干净，并提交实现前 checkpoint。
- Phase 1：C1；完成 post-compaction estimator 和 compaction 单元测试。
- Phase 2：C2、C3、C4；不同文件可并行，禁止同一文件并行编辑。
- Phase 3：C5、C6；完成 SDK、Agent/RPC payload 回归测试。
- Phase 4：运行 targeted tests、clippy 和 fmt；全部改动完成后再按项目收尾规则运行全量门禁。

### 冲突说明

- `src/compaction.rs` 是 C1 的唯一实现入口；C1 完成前不要让 C2/C3 猜测 helper 签名。
- `src/agent.rs` 的 normal/hook/synchronous compaction 分支必须由同一任务统一修改，避免 payload 字段不一致。
- `src/rpc.rs` 的手动 compact 和 auto-compaction 必须同时更新。
- 不修改 `Cargo.toml`、`Cargo.lock`、`src/session.rs` 的持久化结构。

---

## 4. 实现要点

### C1：post-compaction estimator

在 `src/compaction.rs` 新增 `pub(crate)` helper，名称可采用：

```rust
estimate_post_compaction_context_tokens
```

或等价的 `estimate_entries_context_tokens`。

要求：

1. 输入为 append 后的 current-path entries 或等价的、已确定 replay 边界的 entries。
2. 只计算 `Session::to_messages_for_current_path()` 会发送给 provider 的消息。
3. 最新 `CompactionEntry` 只贡献一次 summary。
4. 从 `first_kept_entry_id` 开始计算保留 entries。
5. 找不到 first-kept ID 时遵循当前 replay fallback，不把压缩前历史重复算入。
6. 排除 metadata 和 sibling branch。
7. 对转换出的 `SessionMessage` 调用既有 `estimate_tokens`。
8. 不调用 `estimate_context_tokens`，不使用 retained assistant 的 provider usage。
9. 使用 `u64::saturating_add`。
10. 不引入 tokenizer 或 Cargo 依赖。

如发现 helper 为了完全复用 session replay 必须跨模块抽取大量逻辑，先停止并报告，不扩大为 session 重构。

### C2：Agent

在 `src/agent.rs` 的 compaction apply 路径中：

1. `append_compaction` 完成后，在同一 session 状态边界内计算 post-compaction estimate。
2. 让 apply helper 返回该 `u64`，或以等价方式把结果传回 payload 构造点。
3. normal、synchronous 和 extension hook compaction 都必须在 apply 后再构造成功 payload。
4. `auto_compaction_result_payload` 增加 `tokens_after` 参数并输出 camelCase `tokensAfter`。
5. 失败、取消、缺少 API key 的结果不伪造成功 `tokensAfter`。
6. 不把 `tokensAfter` 写入 session entry。

### C3：RPC

在 `src/rpc.rs`：

- 手动 `compact`：append 后计算，再将 `tokensAfter` 写入成功 JSON response。
- `maybe_auto_compact`：append 后计算，再将 `tokensAfter` 写入 `AutoCompactionEnd.result`。
- 两处都保持当前 `to_messages_for_current_path()` 和 session persist/agent replace 顺序。
- 如能用小型私有 payload builder 消除两处字段漂移可以复用；不得借机重构 RPC 协议。

### C4/C5：SDK 和兼容性

```rust
#[serde(rename_all = "camelCase")]
pub struct RpcCompactionResult {
    pub tokens_before: u64,
    #[serde(default)]
    pub tokens_after: u64,
    // details...
}
```

`#[serde(default)]` 表示旧 payload 没有该字段时读取为 `0`。不使用 `tokens_before` 作为 fallback，不把 `0` 写回 session schema。

---

## 5. 验证计划（Validation Plan）

### 自动化检查

Windows 下所有 Cargo 命令通过 `pwsh` 执行。

#### 先运行新增/受影响测试

```pwsh
cargo test --lib compaction::tests
cargo test --lib apply_compaction_result_emits_structured_result_payload
cargo test --test sdk_unit rpc_compaction_result_serde
cargo test --test compaction
```

测试名称以最终实际新增名称为准。若 RPC 没有稳定的纯单元测试入口，不伪造命令；改为运行实际覆盖该 payload 的现有测试目标。

#### 日常静态检查

```pwsh
cargo clippy --lib -- -D warnings
cargo fmt --check
```

#### 全部改动完成后的收尾门禁

```pwsh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

不运行 `release-max`，不自动构建或部署。

### 测试覆盖目标

- post-compaction estimate 不包含压缩前旧历史。
- summary 和 kept current-path entries 被计算。
- sibling branch 不被计算。
- retained assistant 的 stale usage 不影响 estimate。
- normal/hook/auto Agent 成功 payload 包含 `tokensAfter`。
- RPC manual/auto 成功 payload 包含 `tokensAfter`。
- 失败/取消路径不生成伪造成功 result。
- SDK 新 payload round-trip 使用 camelCase `tokensAfter`。
- legacy SDK payload 缺字段仍可读取并得到 `tokens_after == 0`。
- session `CompactionEntry` persistence 结构保持不变。

### 预期结果

- 所有 focused tests 通过。
- clippy 无 warning。
- fmt 无差异。
- 收尾阶段全量测试、all-targets clippy 和 fmt 通过后，提醒用户进行构建；不自动构建。

### 人工检查

- `tokensAfter` 只在 compaction 成功结果中出现。
- payload 使用 `tokensAfter`，不出现 `tokens_after`。
- 没有 Cargo manifest/lock 依赖变化。
- 没有 session JSONL schema 变化。
- 没有把旧 payload 默认 `0` 误写成真实上下文大小。
