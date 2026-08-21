# 会话

Pi 将对话历史存储在会话文件中。

## 当前存储模型 (V1)

### 文件格式

会话以 JSONL (JSON Lines) 文件形式存储。

### 位置

会话按项目目录分组：
`~/.pi/agent/sessions/--encoded-project-path--/`

文件名格式：`YYYY-MM-DDTHH-MM-SS.sssZ_id.jsonl`

### 结构

1. Header：第一行始终为包含元数据（ID、时间戳、CWD、初始设置）的 `SessionHeader` 对象。
2. Entries：后续行均为表示对话中事件的 `SessionEntry` 对象。

### 条目类型

- `message`：用户或助手消息。
- `model_change`：用户切换了模型。
- `thinking_level_change`：用户更改了思考级别设置。
- `compaction`：为节省 token 对上下文进行了总结压缩。
- `branch_summary`：分支点摘要（分叉时）。
- `session_info`：会话重命名等更新。
- `label`：条目上的元数据标签分配。
- `custom`：扩展定义的结构化负载。

### 树结构

Pi 支持对话分支。每个条目都有一个 `id` 和可选的 `parent_id`。

- 线性对话：`A -> B -> C`
- 分支：
  ```
  A -> B -> C
       \ -> D
  ```

当你导航到一条历史消息并回复时，Pi 会创建一个新分支。

### 管理

#### 恢复 (`/resume`, `pi -r`)

打开会话选择器以在会话之间切换。

- 选择：Enter
- 删除：Ctrl+D（需要确认）

#### 树导航器 (`/tree`)

可视化当前会话的分支结构。

- 导航：上/下
- 切换：Enter（将活动上下文切换到所选节点）

#### 分叉 (`/fork`)

从当前点（或所选点）开始创建一个新的会话文件。当你想探索一个显著不同的方向而不污染当前会话文件时，此功能很有用。

#### 压缩 (`/compact`)

手动触发上下文压缩。Pi 也会根据 `settings.json` 中的 `compaction` 设置自动进行压缩。

## ADR：会话存储 V2 + 线格式契约

- ADR ID：`ADR-SESSION-STORE-V2`
- Bead：`bd-3ar8v.3.1`
- 状态：已接受，将在 Phase 2 中实现
- 日期：2026-02-15

### 背景

V1 JSONL 会话稳健且简单，但在保存、恢复和维护工作流期间，大型长期运行的会话会产生很高的读写放大。Phase 2 的性能目标要求：

1. 在超大历史记录下的追加路径可扩展性。
2. 稳态下恢复行为为 `O(index + tail)`。
3. 来自 V1 存储的确定性迁移与回滚。
4. 显式的损坏检测与有界恢复路径。

### 决策

引入围绕以下能力构建的会话存储 V2 布局：

1. 用于会话条目的分段追加日志。
2. 用于直接条目寻址的伴生偏移索引。
3. 用于非阻塞维护与恢复的单调检查点。
4. 带有显式切换与回滚证据的迁移账本。

V1 JSONL 仍可读取以用于迁移和回滚，但不再是高规模路径的目标架构。

### V2 布局（规范）

逻辑上的 V2 会话容器为：

```text
<session-id>.v2/
  manifest.json
  segments/
    0000000000000001.seg
    0000000000000002.seg
  index/
    offsets.jsonl
  checkpoints/
    0000000000000001.json
  migrations/
    ledger.jsonl
  tmp/
```

### 线格式契约（规范）

机器可读的 schema：`docs/schema/session_store_v2_contract.json`

所有序列化的 JSON 成员名称使用小驼峰命名（`entrySeq`、`sourceFormat`、`migrationEvents`）。Rust 标识符和解释性 prose 可以使用下划线命名，但线上传输不接受下划线命名的成员名。诸如 `pre_migration` 和 `native_v2` 的枚举值保持与 schema 中所示完全一致。已持久化的契约文档拒绝未知成员；仅 `segment_frame.payload` 对象有意对条目特定字段开放。

空存储被显式表示：`entriesTotal`、`segmentCount`、`head.segmentSeq` 和 `head.entrySeq` 均为零，`head.entryId` 为空，且 segment/index/checkpoint 数组可以为空。非空存储要求 head 与 segment 值为正数且具有契约有效的条目 ID。检查点始终要求非空的 head。

契约 schema ID：

1. `pi.session_store_v2.contract.v1`（ bundle 级验证产物）
2. `pi.session_store_v2.manifest.v1`
3. `pi.session_store_v2.segment_frame.v1`
4. `pi.session_store_v2.offset_index.v1`
5. `pi.session_store_v2.checkpoint.v1`
6. `pi.session_store_v2.migration_event.v1`

必需的契约属性：

1. 严格连续的 `entrySeq` 值和单调递增的 `segmentSeq` 值。
2. 来自 index/checkpoint/migration 记录的稳定 `entryId` 引用。
3. manifest 与检查点中的哈希链完整性材料。
4. 显式的迁移关联 ID 与分类结果。
5. 具有失效关闭（fail-closed）验证的确定性状态转换。

### 状态机与不变量

规范状态：

1. `CLEAN`
2. `DIRTY`
3. `SEGMENT_SEALED`
4. `INDEXED`
5. `CHECKPOINTED`
6. `MIGRATION_STAGING`
7. `MIGRATED`
8. `ROLLED_BACK`
9. `FAILED`

允许的转换是刻意收窄的，并由 schema + 测试强制执行：

1. `CLEAN -> DIRTY | MIGRATION_STAGING`
2. `DIRTY -> SEGMENT_SEALED | FAILED`
3. `SEGMENT_SEALED -> INDEXED | FAILED`
4. `INDEXED -> CHECKPOINTED | DIRTY | FAILED`
5. `CHECKPOINTED -> DIRTY | MIGRATION_STAGING | ROLLED_BACK | FAILED`
6. `MIGRATION_STAGING -> MIGRATED | ROLLED_BACK | FAILED`
7. `MIGRATED -> DIRTY | FAILED`
8. `ROLLED_BACK -> DIRTY | FAILED`
9. `FAILED -> DIRTY | ROLLED_BACK`

不变量 ID（除非状态为 `FAILED`，否则必须成立）：

1. `INV-001`：父链接是闭合的（`parentEntryId` 为 null 或已知）。
2. `INV-002`：`entrySeq` 在整个存储中严格递增 1。
3. `INV-003`：索引行可解析为界内的 `(segmentSeq, frameSeq, byteOffset, byteLength)` 范围，且 segment 覆盖完整、无间隙。
4. `INV-004`：检查点 head 在创建时与 manifest head 一致。
5. `INV-005`：哈希链从首个 segment 帧到当前 head 连续。
6. `INV-006`：活动上下文引用的分支 head 已被索引。
7. `INV-007`：迁移切换是原子的：manifest 指针与活动存储标记一同移动。

### 稳态恢复验证

恢复通过有界的水合（hydration）保持诚实，而不将其视为完整审计。它读取 JSONL 头部和完整的 V2 偏移索引，然后验证索引文档、连续的条目/帧/字节范围、segment 元数据以及完整的 segment 文件覆盖。此结构化检查为 O(index rows + segment files)，不会扫描每个帧主体。manifest 必须通过自哈希和声明的不变量校验，必须与索引派生的条目、字节、segment、head 和 last-CRC 事实一致，且消息、分支和压缩计数器必须保持在由已索引条目数推导的边界内。

水合使用同一份已验证的索引快照，且仅读取选定的完整、活动路径或尾部行。在获取的帧被使用之前，读取器会强制执行其配置的大小限制、精确的已索引字节范围、CRC32C、尾随 LF、schema 与索引坐标、条目与父 ID、条目类型、时间戳以及负载字节数和 SHA-256。因此，尾部恢复为 O(index rows + segment files + selected frames)。获取的父 ID 必须存在于索引中，且在获取集合内完全可见的环会被拒绝；前向父引用仍然有效，因为迁移保留了权威的 JSONL 顺序。未选中帧主体中的损坏仅在获取该帧时才会被检测到，而非由有界的结构化检查发现。

完整性与迁移验证仍然刻意更强：`validate_integrity`、`validate_session_integrity` 以及 manifest/store 审计路径会扫描每个帧，验证父图和哈希链，并检查检查点证据。所选帧中或 manifest/JSONL 身份封套中的恢复时失败将失效关闭，并从允许该修复的权威 JSONL 源调用修复。

### 失败语义与恢复行为

#### 追加失败

如果追加在 segment fsync 之前失败，则不会提交任何索引更新，状态保持为 `DIRTY`。恢复时使用相同的逻辑条目负载重试追加。

#### 段封存失败

如果段封存在数据写入之后但在 manifest/index 提交之前失败，该段被视为待定，并在打开时通过重放尾部校验和进行协调。

#### 索引更新失败

如果索引写入在持久化的 segment 写入之后失败，打开路径会记录警告并从 segment 尾部重建缺失的索引行。迁移账本仍保留用于迁移和回滚事件。

#### 检查点失败

检查点文件在临时文件中暂存并以原子重命名方式发布，不会替换已有的最终检查点。崩溃遗留的常规临时文件会在重试时被覆盖；链接或特殊文件类型的临时占位会被拒绝。最后一次有效发布的检查点保持权威性。

#### 迁移切换失败

如果切换在提交标记之前失败，源 V1 保持活动状态。如果切换在提交标记之后但在验证最终化之前失败，状态变为 `FAILED`，在提供写入服务之前需要确定性回滚。

### 迁移与回滚契约

前向迁移（`jsonl_v3|sqlite_v1 -> native_v2`）要求：

1. 创建迁移事件 `phase=planned`。
2. 在暂存路径中构建 V2 segments/index/checkpoints。
3. 验证完整性（`entryCountMatch`、`hashChainMatch`、`indexConsistent`）。
4. 通过更新活动 manifest 指针原子地提交切换。
5. 发出 `phase=completed` 且 `outcome=ok`。

格式回退（`native_v2 -> jsonl_v3|sqlite_v1`）是迁移层的操作。它要求：

1. 保留源快照 ID 和迁移关联 ID。
2. 原子地恢复先前的活动指针。
3. 在重新开放写入之前验证回滚目标的完整性。
4. 发出带有显式原因和结果的回滚事件。

检查点回滚是独立的、原地的 V2 历史操作，由 `SessionStoreV2::rollback_to_checkpoint` 实现。它不会切换活动格式或恢复 V1 指针。在更改活动索引或 segment 集合之前，它会持久记录有界的回滚意图和校验和绑定的暂存索引。恢复会幂等地重放该意图，在截断或隔离 segment 尾部之前发布保留的索引，协调 manifest，隔离比目标更新的检查点，验证结果存储，并仅在持久的成功证据被记录后移除意图。

从部分迁移中恢复是确定性的：

1. 无提交标记：继续使用源格式。
2. 带失败验证的提交标记：强制走回滚路径。
3. 缺失回滚目标：以 `FAILED` 硬失败，需要操作员介入。

### 可测试性承诺

当契约测试能够证明以下几点时，该 ADR 被视为已实现：

1. 契约示例通过 `docs/schema/session_store_v2_contract.json` 验证。
2. 无效转换和缺失关键字段会失效关闭。
3. 迁移和回滚记录符合 schema 且关联链接正确。
4. 下游实现 beads（`3.2`、`3.3`、`3.7`）直接消费此契约而无需重新解释。
