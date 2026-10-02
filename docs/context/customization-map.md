# Fork 二开差异清单

> 状态：`custom-next` 引导阶段清单
> 最近审阅：2026-10-02

本文是本 Fork 与上游之间有意差异的当前清单。它是状态文档，不是旧 Fork 历史的复制品。

## 基线和同步状态

本 Fork 必须把“上游比较基线”和“本地同步位置”分开记录：

| 字段 | 值 |
|---|---|
| 上游仓库 | `https://github.com/Dicklesworthstone/pi_agent_rust.git` |
| 上游引用 | `main` |
| 上游比较基线 SHA | `1e4548aa745ecfbc40b66bdbae04baa2d09717f9` |
| `main` 整合提交 | `aef52107134a6e8e1b197bbb935265aa485932b1` |
| Fork 基线分支 | `custom-next` |
| `custom-next` 基线提交 | `aef52107134a6e8e1b197bbb935265aa485932b1` |
| `custom-next` 当前 tip | `80a4050f8b3b7447d05b18b76b332b3313744291` |
| 最近同步时间 | `2026-10-02` |
| 引导阶段同步说明 | `custom-next` 直接从更新后的本地 `main` 创建，尚无单独的 `custom-next` 合并提交 |
| 首个 `custom-next` 治理提交 | `80a4050f8` |
| 旧分支 | `custom` —— 仅作历史参考，尚未迁移 |

精确的上游 SHA 是代码基线的事实来源。像 `upstream/main` 这样的可移动分支名，单独不能作为基线证据。

- **上游比较基线（`upstream baseline SHA`）**：用于比较二开差异的上游精确提交；二开条目的语义基线只能使用它。
- **`main` 整合提交（`main integration commit`）**：上游提交在本地 `main` 中的整合位置，可能包含 Fork 治理例外。
- **`custom-next` 集成/基线提交（`custom-next integration/base commit`）**：同步进入二开主线的位置；直接从 `main` 创建时记录基线提交，不虚构合并提交。
- **`custom-next` 当前 tip**：二开主线当前提交，只表示分支位置，不等于上游比较基线。

推荐在差异清单中使用明确字段名：

```text
upstream_baseline_sha
main_integration_commit
custom_next_integration_commit 或 custom_next_base_commit
current_custom_next_tip
```

其中，上游 SHA 是“比较基线”，本地 `main` 和 `custom-next` 提交是“可复查的整合位置”。

上游同步后，必须更新全局同步状态；但只有实际检查过的二开条目，才能更新该条目的“最近复核上游 SHA”。未检查的条目必须保留旧复核 SHA，并标记为 `needs-review`。

## 当前差异摘要

- 已迁移到 `custom-next` 的产品级旧二开：**当前没有登记**。
- Fork 治理差异：**2 项有效**。
- 未登记差异：**必须先调查，不能直接视为有意差异**。
- 旧 Fork 的 `docs/context/` 内容：**不属于新基线，尚未重建**。

## 当前有效差异

### FORK-GOV-001 — 禁用上游 Dependabot 自动化

- **分类：** `governance`
- **状态：** `active`
- **分支/文件：** `main` 及其后代；`.github/dependabot.yml`
- **目的：** 防止上游 Dependabot 自动创建噪声更新任务，或把本 Fork 当成上游项目运行。
- **与上游不同的原因：** 本 Fork 有自己的维护和同步策略。依赖更新机器人不能静默地制造 Fork 工作。
- **验证：** 检查实际生效的 `.github/dependabot.yml`，确认上游更新条目仍然处于禁用状态。
- **最近复核上游 SHA：** `1e4548aa745ecfbc40b66bdbae04baa2d09717f9`
- **最近复核的 `custom-next` 提交：** `80a4050f8b3b7447d05b18b76b332b3313744291`
- **最近一次同步影响：** `unaffected`（该差异属于本 Fork 的治理策略，本次基线变化未改变其目的）
- **冲突规则：** 上游同步时保留此例外，除非用户明确改变 Fork 政策。
- **移除条件：** 只有在本 Fork 明确建立替代性的依赖自动化策略，并且用户授权重新启用时，才允许移除。
- **相关提交：** `6ce84030b`、`aef521071`

### FORK-GOV-002 — 忽略本地 Codegraph 和 Cargo Sweep 生成物

- **分类：** `governance`
- **状态：** `active`
- **分支/文件：** `custom-next`；`.gitignore`
- **目的：** 防止本地搜索索引和本地清理标记进入版本控制。
- **忽略路径：** `/.codegraph/`、`/sweep.timestamp`，以及项目原有的其他本地生成物规则。
- **与干净工作树不同的原因：** Codegraph 是 Agent 使用的本地搜索索引，`cargo-sweep` 会留下本地时间戳标记。两者都不是源码、项目配置或可复现的项目产物。
- **验证：** `git check-ignore -v .codegraph sweep.timestamp`
- **最近复核上游 SHA：** `1e4548aa745ecfbc40b66bdbae04baa2d09717f9`
- **最近复核的 `custom-next` 提交：** `80a4050f8b3b7447d05b18b76b332b3313744291`
- **最近一次同步影响：** `unaffected`（这些路径属于本地工具生成物，与上游业务代码无关）
- **移除条件：** 只有当工具不再生成这些路径，或本 Fork 有意开始版本化对应的可复现产物时，才允许移除。
- **相关提交：** `80a4050f8`

## 尚未迁移的内容

旧 `custom` 分支可能包含许多产品修改、上下文文档、CI 工作流或实验。除非每一项都单独完成以下工作，否则不能把它们视为 `custom-next` 的有效差异：

1. 根据用户可见行为或维护目的识别它；
2. 对照新的上游基线检查它；
3. 重新实现，或明确决定不迁移；
4. 在适用时添加测试并完成验证；
5. 使用稳定 ID 和生命周期状态登记到本文。

不要通过复制旧提交列表来填充本节。应按行为层面的迁移记录处理。

## 新增差异的登记要求

每个新条目应包含：

- 稳定 ID；
- 分类和生命周期状态；
- 用户需求或维护目的；
- 精确文件和接入边界；
- 验证方式和测试位置；
- 上游等价实现（如果存在）；
- 冲突风险；
- 移除或替换条件；
- 相关提交或问题编号。

建议使用以下生命周期状态：

- `active`
- `pending-upstream`
- `upstream-equivalent`
- `obsolete`
- `blocked`
- `needs-review`

处于 `needs-review` 或 `blocked` 状态，不代表可以永远保留该差异；它表示一个已经被公开记录的维护义务。
