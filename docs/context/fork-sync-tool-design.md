# Fork 同步工具设计

> 状态：设计已确认，已实现
> 适用分支：`custom-next` 及其后续二开分支
> 上游决策：[`ADR-001`](decisions/ADR-001-fork-sync-state-and-tooling.md)
> 实现：`scripts/fork-sync.ps1`
> 状态文件：`upstream-sync-state.json`
> 最近审阅：2026-10-02

本文定义并记录本 Fork 上游同步工具的行为、命令边界、状态生命周期和验证规则。脚本已经实现；本文是实现契约和使用说明，不是待实现计划。

## 1. 目标与背景

本 Fork 需要持续同步快速变化的上游，同时保留少量有意的二开差异。手工复制 Git SHA、手工维护同步状态和一次性处理所有复核结果，容易导致：

- 上游比较基线和本地整合提交混淆；
- 尚未完成的同步被误记为已完成；
- 同步中断后丢失已经完成的逐项复核；
- 状态文档和 Git 实际历史发生漂移；
- Agent 无法可靠判断下一步应该同步、复核还是记录。

工具的目标是让机器负责客观 Git 状态，让 Agent 或用户负责二开差异的语义判断。

## 2. 已实现对象

当前实现文件：

```text
scripts/fork-sync.ps1
```

当前状态文件：

```text
docs/context/upstream-sync-state.json
```

状态文件分为两层：

```text
last_completed_sync
    最近一次完整完成的同步检查点

pending_review
    当前尚未完成的同步事务和逐项复核进度
```

正式状态和临时状态必须共存于同一个 JSON 文件中，但临时状态不能覆盖正式检查点。

## 3. 范围与非目标

### 3.1 包含范围

工具负责：

- 读取当前 Git 分支和工作区状态；
- 获取 `upstream/main`；
- 读取和校验 Git SHA；
- 判断提交祖先关系；
- 计算上游变更范围；
- 创建、恢复和更新 `pending_review`；
- 保存 Agent/用户提供的二开语义复核；
- 在完整条件满足时，将 pending 提升为正式状态；
- 对状态文件进行原子写入；
- 通过退出码和人类/JSON 输出报告状态。

### 3.2 不包含范围

第一版工具不负责：

- 自动切换分支；
- 自动 merge 或 fast-forward；
- 自动解决冲突；
- 自动选择上游代码或本地代码；
- 自动判断二开是否具有产品价值；
- 自动提交或推送；
- 删除文件、分支或提交；
- `git reset`、`git clean`、stash 或其他回退/清理操作；
- 运行所有项目测试并替用户宣称质量完成。

分支同步、冲突解决、验证范围和远程更新仍由 Agent/用户按照项目规则执行。

## 4. 状态文件模型

状态文件路径：

```text
docs/context/upstream-sync-state.json
```

第一版预期结构：

```json
{
  "$schema": "./upstream-sync-state.schema.json",
  "schema_version": 1,
  "kind": "fork-upstream-sync-state",
  "upstream": {
    "repository": "https://github.com/Dicklesworthstone/pi_agent_rust.git",
    "remote": "upstream",
    "ref": "main"
  },
  "fork": {
    "upstream_mirror_branch": "main",
    "development_branch": "custom-next"
  },
  "last_completed_sync": {
    "status": "complete",
    "upstream_baseline_sha": "<自动读取>",
    "main_integration_commit": "<自动读取>",
    "custom_next_sync_commit": "<自动计算>",
    "custom_next_sync_kind": "base",
    "validation_status": "passed",
    "validation_note": "<验证说明>",
    "recorded_at": "<自动生成>"
  },
  "customization_reviews": {},
  "pending_review": null
}
```

### 4.1 正式检查点

`last_completed_sync` 只表示已经完成以下工作的同步：

1. 已抓取目标上游；
2. 本地 `main` 已整合候选上游提交；
3. `custom-next` 已接收更新后的 `main`；
4. 相关冲突已经处理；
5. 受影响的二开差异已经复核；
6. 相关验证已经完成并明确通过。

在 `record` 成功前，正式检查点不得更新。

### 4.2 临时同步事务

`pending_review` 保存当前同步事务，例如：

```json
{
  "status": "in_progress",
  "candidate_upstream_baseline_sha": "<自动读取>",
  "prepared_at": "<自动生成>",
  "prepared_from_upstream_sha": "<上一次正式 SHA>",
  "prepared_from_custom_next_commit": "<自动读取>",
  "main_integration_commit": null,
  "custom_next_sync_commit": null,
  "custom_next_sync_kind": null,
  "reviews": {},
  "validation": {
    "status": "not-run",
    "note": null,
    "validated_at": null
  }
}
```

`pending_review` 是可恢复的临时状态，不代表同步已经完成。

如果新的候选上游 SHA 与已有 pending 不同，工具不得静默覆盖旧 pending，必须报告 `STALE_PENDING_REVIEW`，由用户/Agent 显式完成旧事务或使用 `abandon` 放弃它。

### 4.3 SHA 字段的语义

| 字段 | 语义 | 来源 |
|---|---|---|
| `upstream_baseline_sha` | 二开差异的上游语义比较基线 | 自动读取 `upstream/main` |
| `main_integration_commit` | 上游变化在本地 `main` 中的整合位置 | 自动读取本地 `main` |
| `custom_next_sync_commit` | 同步结果进入 `custom-next` 的位置 | 自动计算 `git merge-base` 或初始化基线 |
| `custom_next_sync_kind` | 同步位置是 `base`、`merge` 还是 `fast-forward` | 工具根据 Git 关系确定 |

所有 SHA 都由 Git 或工具生成，命令参数不接受手写 SHA。

`custom-next` 当前 tip 不写入正式同步状态；`status` 实时读取并显示它。

## 5. 命令设计

## 5.1 `status`

调用：

```powershell
pwsh -File scripts/fork-sync.ps1 status
pwsh -File scripts/fork-sync.ps1 status -Json
```

行为：

- 只读；
- 不执行 fetch；
- 不切换分支；
- 不修改状态文件；
- 不执行 merge、commit 或 push。

检查并显示：

- 当前分支；
- 工作区是否有改动；
- 本地已抓取的 `upstream/main` SHA；
- 本地 `main` SHA；
- 本地 `custom-next` SHA；
- 正式同步检查点；
- pending 状态；
- 分支祖先关系；
- 待复核的二开差异；
- 下一步建议。

建议状态值：

```text
UNINITIALIZED
READY
OUT_OF_DATE
PENDING_REVIEW
STALE_PENDING_REVIEW
BLOCKED
INVALID
DIRTY
```

说明：

- `UNINITIALIZED`：状态文件不存在；
- `READY`：正式检查点完整，且没有待处理同步；
- `OUT_OF_DATE`：当前上游已前进，但尚未创建同步事务；
- `PENDING_REVIEW`：存在未完成同步事务；
- `STALE_PENDING_REVIEW`：pending 针对的上游 SHA 已经过期；
- `BLOCKED`：分支关系或完成条件不满足；
- `INVALID`：JSON、SHA 或状态关系无效；
- `DIRTY`：工作区存在会阻塞当前操作的未处理改动。

`status` 发现提醒时可以返回退出码 `10`，不应把正常的“需要维护”误报为工具错误。

## 5.2 `prepare`

调用：

```powershell
pwsh -File scripts/fork-sync.ps1 prepare
pwsh -File scripts/fork-sync.ps1 prepare -Json
```

行为：

1. 执行 `git fetch upstream main`；
2. 读取正式检查点中的旧上游 SHA；
3. 读取新的 `upstream/main` SHA；
4. 计算新增上游提交数量和变更文件；
5. 检查已有 pending 是否存在或过期；
6. 在没有冲突事务时创建或更新 `pending_review`；
7. 输出同步计划。

`prepare` 不会：

- 切换到 `main` 或 `custom-next`；
- 合并上游；
- 解决冲突；
- 更新正式检查点；
- 自动判断二开影响。

如果已有 pending 且候选 SHA 相同，保留已有 `reviews` 和 `validation`，不能清空进度。如果已有 pending 但候选 SHA 不同，拒绝覆盖并报告 `STALE_PENDING_REVIEW`。

第一版可以列出上游变更文件作为复核导航，但不根据路径重叠自动断言某个二开受到影响。保守策略是要求所有 `active` 二开在本次完整同步中获得明确复核。

## 5.3 `review`

调用：

```powershell
pwsh -File scripts/fork-sync.ps1 review FORK-GOV-001 -Impact unaffected
pwsh -File scripts/fork-sync.ps1 review FORK-GOV-001 -Impact adapted -Note "已完成适配并通过针对性验证"
```

参数：

```text
<ID>
    二开差异 ID，例如 FORK-GOV-001

-Impact
    unaffected
    adapted
    upstream-equivalent
    unknown

-Note
    可选的语义判断说明
```

影响值含义：

- `unaffected`：已检查，确认上游变化不影响该二开；
- `adapted`：上游变化影响该二开，但适配已经完成；
- `upstream-equivalent`：上游提供了等价能力，当前差异可能可以退役；
- `unknown`：尚未能确定影响。

`review` 自动写入：

- 当前候选上游 SHA；
- 当前 `custom-next` 同步位置；
- 复核时间；
- 复核状态。

Agent/用户不提供 SHA。

默认前置条件：

- 存在 `pending_review`；
- 当前 `upstream/main` 等于候选 SHA；
- `main` 已包含候选上游 SHA；
- `custom-next` 已包含 `main`。

这样可以避免在尚未同步到新代码的旧分支上提前登记复核结论。

## 5.4 `record`

调用：

```powershell
pwsh -File scripts/fork-sync.ps1 record -ValidationStatus passed
pwsh -File scripts/fork-sync.ps1 record -ValidationStatus passed -ValidationNote "针对性测试和 fmt 已通过"
```

`record` 是唯一能够把 pending 提升为正式检查点的命令。

### 自动读取

`record` 自动读取：

- `upstream/main`；
- `main`；
- `custom-next`；
- pending 中的复核结果；
- 当前时间。

### 必须满足的条件

1. 存在 `pending_review`；
2. pending 候选 SHA 等于当前 `upstream/main`；
3. `main` 包含候选上游 SHA；
4. `custom-next` 包含更新后的 `main`；
5. 所有有效二开差异都已复核；
6. 没有 `unknown`、`needs-review` 或 `blocked`；
7. 验证状态为 `passed`；
8. 所有必要 SHA 都是有效提交；
9. 不存在与本次同步无关、会被误纳入判断的未提交代码或配置改动。

工作区不要求绝对干净，因为状态文件本身可能是本次事务产生的改动；但无关源码、配置或其他工作必须阻塞 `record`，不能被工具静默忽略。

### 提交位置计算

`main_integration_commit`：

```text
git rev-parse main
```

它表示本次记录时本地 `main` 的真实 tip，允许包含已批准的 Fork 治理例外。

`custom_next_sync_commit`：

```text
git merge-base main custom-next
```

它表示 `custom-next` 与 `main` 的共同同步位置，而不是二开主线当前 tip。

初始化时，如果 `custom-next` 直接从更新后的 `main` 创建，则使用：

```text
custom_next_sync_kind = base
custom_next_sync_commit = main_integration_commit
```

后续同步根据实际 Git 历史记录：

```text
base
merge
fast-forward
```

不能可靠判断时应报告阻塞，而不是猜测类型。

### 成功后的写入

成功后：

1. 将 pending 候选信息提升为 `last_completed_sync`；
2. 将 pending 中的复核结果合入 `customization_reviews`；
3. 清空 `pending_review`；
4. 使用临时文件加原子替换写入 JSON；
5. 不自动 commit；
6. 不自动 push。

## 5.5 `bootstrap`

调用：

```powershell
pwsh -File scripts/fork-sync.ps1 bootstrap
```

用途：第一次为新的 `custom-next` 创建状态事务。

只允许在 `custom-next` 上执行，并检查：

- `main` 存在；
- `custom-next` 存在；
- `main` 包含当前 `upstream/main`；
- `custom-next` 包含 `main`；
- 状态文件尚不存在。

它创建初始 `pending_review`，而不是直接伪造完整状态：

```json
{
  "status": "bootstrap",
  "candidate_upstream_baseline_sha": "<自动读取>",
  "reviews": {}
}
```

之后仍需执行：

```powershell
pwsh -File scripts/fork-sync.ps1 review FORK-GOV-001 -Impact unaffected
pwsh -File scripts/fork-sync.ps1 record -ValidationStatus passed
```

## 5.6 `abandon`

调用：

```powershell
pwsh -File scripts/fork-sync.ps1 abandon -Reason "上游再次更新，旧同步事务重新规划"
```

用途：显式放弃过期或不再继续的 pending 事务。

行为：

- 要求非空的 `-Reason`；
- 显示即将放弃的 pending 信息；
- 保持 `last_completed_sync` 不变；
- 清空 `pending_review`；
- 不删除任何 Git 分支或提交；
- 不执行 reset、clean、stash、回退或覆盖源码。

`abandon` 只处理状态文件中的临时事务，不代表放弃 Git 中已经存在的同步或二开代码。

## 6. 状态转换

```text
UNINITIALIZED
    │
    └── bootstrap
          ▼
PENDING_REVIEW (bootstrap)
    │
    ├── prepare
    ├── review
    ├── 分支同步
    ├── 验证
    └── record
          ▼
READY

READY
    │
    └── 上游前进 / prepare
          ▼
PENDING_REVIEW

PENDING_REVIEW
    │
    ├── 候选 SHA 变化
    ▼
STALE_PENDING_REVIEW
    │
    └── abandon
          ▼
READY 或 OUT_OF_DATE
```

`last_completed_sync` 在进入 `READY` 之前保持旧值。

## 7. 不变量与安全约束

### 7.1 提交关系

正式记录时必须满足：

```text
candidate upstream SHA ⊆ main
main ⊆ custom-next
```

对应校验：

```text
git merge-base --is-ancestor <candidate-upstream-sha> main
git merge-base --is-ancestor main custom-next
```

### 7.2 状态关系

```text
status = needs-review 或 blocked
    → 不能完成 record

status = reviewed
    → 必须有复核 SHA、影响值和复核时间

impact = unknown
    → 不能完成 record

pending_review 候选 SHA ≠ 当前 upstream/main
    → 不能继续写入 review 或 record
```

### 7.3 写入安全

状态文件必须使用：

1. 读取并解析现有 JSON；
2. 在内存中完成校验和修改；
3. 写入同目录临时文件；
4. 完成 JSON 解析/格式验证；
5. 原子替换目标文件。

写入失败时不得留下半截的正式状态文件。

### 7.4 工作区保护

工具不得处理、隐藏或覆盖无关的未提交工作。发现不确定来源的改动时，应报告阻塞，而不是 stash、revert、reset 或 clean。

## 8. 退出码

建议统一使用：

```text
0   成功，或状态正常
10  状态可读取，但存在待同步/待复核提醒
20  被阻塞，无法继续或无法完成 record
2   参数错误、配置错误、JSON 错误或工具执行错误
```

例如：

```text
status 发现上游有新提交  → 10
record 仍有未复核差异    → 20
状态文件损坏             → 2
```

## 9. 验证与验收标准

设计完成后的实现至少需要覆盖：

### 状态读取

- 状态文件不存在时返回 `UNINITIALIZED`；
- 状态文件损坏时返回 `INVALID`；
- 正常状态能够区分正式检查点和当前 tip。

### Pending 生命周期

- `bootstrap` 能创建初始 pending；
- `prepare` 能创建 pending；
- 相同候选 SHA 的 `prepare` 保留已有 review；
- 新候选 SHA 不会静默覆盖旧 pending；
- `abandon` 只清空 pending，不改变正式检查点；
- 中断后可以继续已有 pending。

### 复核和记录

- `review` 自动写入机器读取的 SHA 和时间；
- `unknown` 不能完成 `record`；
- 未合入新 `main` 的 `custom-next` 不能完成 `record`；
- 验证未通过不能完成 `record`；
- `record` 成功后 pending 被清空、正式状态被更新。

### 安全边界

- 工具不执行自动 merge、commit、push；
- 工具不执行删除、reset、clean、stash 或回退；
- 无关工作区改动不会被覆盖或纳入状态判断；
- JSON 写入失败不会破坏旧的正式状态。

## 10. 关联文档

- `AGENTS.md`：本 Fork 的活动 Agent 规则；
- `docs/context/README.md`：同步工具的 Agent 使用入口；
- `docs/context/upstream-sync-and-customization-charter.md`：维护宪章；
- `docs/context/customization-map.md`：二开差异语义和生命周期；
- `docs/context/upstream-sync-state.json`：机器维护的当前同步状态；
- `docs/context/decisions/ADR-001-fork-sync-state-and-tooling.md`：为什么采用当前机制。
