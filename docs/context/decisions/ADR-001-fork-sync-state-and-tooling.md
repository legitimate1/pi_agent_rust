# ADR-001 — Fork 上游同步状态与自动记录机制

- **状态：** Accepted
- **日期：** 2026-10-02
- **适用范围：** `custom-next` 及其后续二开维护
- **决策类型：** Fork 治理 / 同步状态 / Agent 工具协作

## 1. 背景

本 Fork 需要长期跟踪快速变化的上游项目，同时保留少量明确的二开行为。仅靠人工在 `customization-map.md` 中复制和填写 Git SHA，容易造成以下问题：

- 手工复制 SHA 出错；
- 把上游提交误写成本地提交，或反过来；
- 忘记更新某个同步字段；
- 把 `custom-next` 当前 tip 误当成上游比较基线；
- 尚未完成的同步被误记为已完成；
- 同步中断后无法恢复已经完成的逐项复核；
- Markdown 中的动态状态与真实 Git 历史逐渐漂移。

因此，需要把客观的 Git 状态交给机器管理，把二开差异的语义判断保留给 Agent 或用户。

## 2. 决策

采用一个机器可读的同步状态文件：

```text
docs/context/upstream-sync-state.json
```

并设计 Fork 专用同步工具：

```text
scripts/fork-sync.ps1
```

状态文件只保存**最近一次完整完成的上游同步检查点**，同时在同一个 JSON 文件中保存当前未完成同步事务的 `pending_review` 区块。

### 2.1 正式检查点

正式检查点存放在：

```json
"last_completed_sync": {}
```

它只能表示已经完成以下工作的同步：

1. 上游已抓取；
2. 本地 `main` 已整合上游；
3. `custom-next` 已接收更新后的 `main`；
4. 相关冲突已处理；
5. 受影响的二开差异已复核；
6. 相关验证已完成并明确通过。

在 `record` 成功前，正式检查点保持不变。未完成或被阻塞的同步不得覆盖它。

### 2.2 临时同步事务

当前同步事务存放在：

```json
"pending_review": {}
```

它保存：

- 候选上游 SHA；
- 本地 `main` 的整合位置；
- `custom-next` 的同步位置；
- 已完成的逐项二开复核；
- 验证状态；
- 准备时间和复核时间。

`pending_review` 是可恢复的临时状态，不代表同步已经完成。

如果发现新的上游候选 SHA，而旧的 `pending_review` 尚未完成，工具不得静默覆盖旧事务。必须明确报告旧事务过期，或通过显式操作放弃它。

### 2.3 SHA 的语义分工

状态文件和工具必须区分以下提交：

- **`upstream_baseline_sha`**：上游精确提交，是二开差异的语义比较基线；
- **`main_integration_commit`**：上游变化在本地 `main` 中的整合位置，可能包含 Fork 治理例外；
- **`custom_next_sync_commit`**：这次同步结果进入 `custom-next` 的位置；
- **`custom_next_sync_kind`**：说明同步位置是直接创建的 `base`，还是后续合并产生的 `merge`。

所有 SHA 必须由 Git 或同步工具读取。Agent 和用户不得手工复制填写 SHA。

`custom-next` 当前 tip 不写入正式同步状态，因为普通二开提交会使它持续变化。当前 tip 由工具在 `status` 中实时读取。

### 2.4 机器与人类的职责

机器负责：

- 读取 Git SHA；
- 获取上游；
- 判断提交是否存在及其祖先关系；
- 计算同步位置和变更文件；
- 记录时间；
- 保存状态；
- 校验是否满足完整记录条件。

Agent 或用户负责：

- 执行或授权分支切换、合并和推送；
- 解决上游与 Fork 的代码冲突；
- 判断某项二开是否受上游影响；
- 判断上游是否提供了等价能力；
- 决定采用 `unaffected`、`adapted`、`upstream-equivalent` 或 `unknown`；
- 决定是否保留、适配或退役二开差异。

机器不能代替人类做产品语义判断。

### 2.5 工具命令边界

工具提供以下四个核心命令：

```powershell
pwsh -File scripts/fork-sync.ps1 status
pwsh -File scripts/fork-sync.ps1 prepare
pwsh -File scripts/fork-sync.ps1 review FORK-GOV-001 -Impact unaffected
pwsh -File scripts/fork-sync.ps1 record -ValidationStatus passed
```

- `status`：只读显示当前 Git 状态、正式检查点、候选同步和待复核事项；
- `prepare`：抓取 `upstream/main`，计算候选同步并创建或更新 `pending_review`；
- `review`：接收 Agent/用户的语义影响判断，由工具自动填入对应 SHA 和时间；
- `record`：校验同步、复核和验证均已完成，将 pending 状态提升为正式检查点。

工具第一版不负责：

- 自动解决冲突；
- 自动替用户选择上游或本地代码；
- 自动提交；
- 自动推送；
- 自动删除文件或分支；
- 自动判断二开是否具有产品价值。

## 3. 生命周期

```text
最近一次完整状态
        │
        ├── prepare
        ▼
待复核的 pending_review
        │
        ├── 分支同步
        ├── review
        ├── 验证
        ▼
record
        │
        ▼
新的完整状态，pending_review 清空
```

### 3.1 普通二开提交

普通二开提交不会修改：

```text
last_completed_sync
```

也不会因为普通功能开发而伪造新的上游同步检查点。

### 3.2 同步中断

同步中途失败时：

- `last_completed_sync` 保持上一次完整值；
- `pending_review` 保留已经完成的临时进度；
- 工具报告当前同步尚未完成；
- 在所有必要复核和验证完成前，不允许执行成功的 `record`。

### 3.3 完成登记

`record` 成功后：

1. 将 `pending_review` 中的候选同步信息提升为 `last_completed_sync`；
2. 将临时二开复核提升为正式 `customization_reviews`；
3. 清空 `pending_review`；
4. 原子写入 `upstream-sync-state.json`；
5. 不自动 commit 或 push。

## 4. 未采用的方案

### 4.1 每次手工填写 SHA

不采用。手工复制容易出错，也无法可靠区分上游基线和本地整合位置。

### 4.2 用本地 `main` SHA 作为二开比较基线

不采用。`main` 可能包含 Fork 的 Dependabot 隔离等治理例外；二开差异的语义比较基线必须是上游精确 SHA。

### 4.3 把 `custom-next` 当前 tip 作为同步基线

不采用。普通二开提交会改变当前 tip，它不代表上游同步进入二开主线的位置。

### 4.4 状态文件只保存一份动态 Markdown

不采用。机器状态与人类语义混在同一份 Markdown 中，容易出现重复字段和内容漂移。

### 4.5 使用独立的 pending 文件

当前不采用。临时状态和正式状态都属于同一个同步事务，将 `pending_review` 放入同一个 JSON 可以保持单一状态入口，并支持中断恢复。

### 4.6 工具自动 merge、resolve、commit 或 push

不采用。合并冲突、提交范围和远程推送都涉及工作区安全、代码语义或用户授权，不应由第一版同步工具静默完成。

### 4.7 工具自动判断二开语义影响

不采用。文件重叠或调用关系只能产生候选范围，不能证明上游行为已经等价替代本 Fork 的产品差异。

### 4.8 不持久化 pending 复核

不采用。一次性传入全部复核结果无法在中断后恢复，也无法保留已经完成的逐项判断。

## 5. 后果

### 正面后果

- 减少手工复制 SHA 的错误；
- 明确区分上游比较基线和本地整合位置；
- 正式完成状态不会被未完成同步污染；
- 中断后的同步可以恢复；
- 机器事实和人工语义判断边界清晰；
- 更适合 Agent 持续维护和审计。

### 代价

- 增加 `upstream-sync-state.json` 状态文件；
- 增加 PowerShell 同步工具及其校验逻辑；
- 需要处理过期的 `pending_review`；
- 需要保证状态文件原子写入；
- 需要防止机器状态与 `customization-map.md` 的语义记录脱节；
- 完整同步仍需要人工处理冲突和二开影响。

## 6. 实现边界与待定项

本 ADR 确认的是长期机制和职责边界，不代表工具已经实现。

以下内容可以在工具设计或实现阶段细化，而不需要修改本 ADR 的核心决定：

- `upstream-sync-state.json` 的完整 JSON Schema；
- PowerShell 参数的最终大小写和帮助文本；
- 命令退出码；
- Git 祖先关系的具体校验实现；
- 状态文件中字段的可选性和兼容迁移；
- `pending_review` 的显式放弃命令。

如果未来改变以下核心决定，应新增 ADR 或明确替代本 ADR：

- 不再使用同一个 JSON 保存 `pending_review`；
- 不再以上游 SHA 作为二开语义比较基线；
- 让工具自动解决冲突或自动提交推送；
- 让机器自动替代 Agent/用户进行二开语义判断；
- 让状态文件记录完整同步历史而不再只是当前检查点。

## 7. 相关文档

- `AGENTS.md`：本 Fork 的活动 Agent 规则；
- `docs/context/README.md`：治理层文档入口；
- `docs/context/upstream-sync-and-customization-charter.md`：上游同步和二开维护规则；
- `docs/context/customization-map.md`：当前二开差异和复核状态；
- `docs/context/upstream-sync-state.json`：待实现的机器同步状态文件；
- `scripts/fork-sync.ps1`：待实现的同步工具。
