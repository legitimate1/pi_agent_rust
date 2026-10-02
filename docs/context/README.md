# Fork Agent 上下文

本目录是本 Fork 的治理、Agent 导航和按需建立的项目上下文/事实文档层。它与上游项目的普通用户文档和开发者文档分开；上游文档仍保留在 `docs/` 的原位置。

## 当前范围

本目录承载两类内容：

- Fork 如何维护：分支、同步、差异登记和 Agent 执行规则；
- Fork 已经按需核实的项目上下文：例如 `rpc.md` 记录当前 RPC 行为及其源码入口。

项目上下文文档不替代源码和测试，也不表示已经描述完整项目架构或功能清单。没有完成核实的内容不应写成当前行为。

## 阅读顺序

进行任何 Fork 维护工作时：

1. 读取根目录 `AGENTS.md`，了解当前生效的 Agent 执行规则。
2. 读取本文，了解文档边界。
3. 读取 `upstream-sync-and-customization-charter.md`，了解维护契约。
4. 读取 `customization-map.md`，了解当前 Fork 差异和基线。
5. 如果要比较或改造上游 Agent 工作流，读取 `agent-workflow-adoption.md`。
6. 如果要设计或修改 Fork 的长期维护机制，读取 `decisions/` 下相关 ADR；当前同步状态机制见 `decisions/ADR-001-fork-sync-state-and-tooling.md`。
7. 如果要实现或修改 Fork 上游同步工具，读取 `fork-sync-tool-design.md`。
8. 根据具体任务，按需读取源码、测试和上游文档。

## 文档说明

### `upstream-sync-and-customization-charter.md`

定义分支、上游同步、二开修改、冲突处理、Agent 执行和例外处理的稳定维护规则。

### `customization-map.md`

记录当前 Fork 专属差异的动态状态，包括差异原因、代码位置、验证方式、生命周期和移除条件。

### `agent-workflow-adoption.md`

记录如何研究上游 `AGENTS.md`，以及哪些上游 Agent 实践被直接采用、本地化采用、暂缓采用或明确拒绝。

### `fork-sync-tool-design.md`

记录 `scripts/fork-sync.ps1` 的命令、状态生命周期、退出码和安全边界。脚本已经实现；本文档描述当前实现应遵守的设计契约。

### `upstream-sync-state.json`

机器生成的最近一次完整上游同步检查点和二开复核状态。不要手工编辑 SHA、时间或复核记录；使用 `fork-sync.ps1` 管理它们。

## 上游同步工具使用

所有命令都从仓库根目录、`custom-next` 分支执行：

```powershell
# 查看实时状态；只读，不 fetch、不写状态
pwsh -NoProfile -File scripts/fork-sync.ps1 status -Json

# 第一次初始化 custom-next 的同步事务
pwsh -NoProfile -File scripts/fork-sync.ps1 bootstrap -Json

# 上游有新提交时，抓取并创建 pending_review
pwsh -NoProfile -File scripts/fork-sync.ps1 prepare -Json

# 上游同步到 custom-next 后，逐项登记语义复核
pwsh -NoProfile -File scripts/fork-sync.ps1 review FORK-GOV-001 -Impact unaffected
pwsh -NoProfile -File scripts/fork-sync.ps1 review FORK-GOV-002 -Impact unaffected

# 验证通过后，将 pending 提升为正式同步检查点
pwsh -NoProfile -File scripts/fork-sync.ps1 record `
  -ValidationStatus passed `
  -ValidationNote "说明实际执行过的验证"

# 放弃过期或不再继续的 pending；不会回退 Git
pwsh -NoProfile -File scripts/fork-sync.ps1 abandon `
  -Reason "说明放弃原因"
```

### Agent 操作规则

- 每次接手 Fork 维护任务，先运行 `status -Json`。
- `UNINITIALIZED` 只表示状态文件尚未初始化；在确认当前 `main` 和 `custom-next` 基线正确后运行 `bootstrap`。
- `OUT_OF_DATE` 时运行 `prepare`，但工具不会自动 merge；Agent 必须按宪章先更新 `main`，再将 `main` 合入 `custom-next`。
- `PENDING_REVIEW` 时不要重复 `prepare` 清空进度；先完成分支同步、逐项 `review` 和验证。
- `STALE_PENDING_REVIEW` 时不能静默覆盖事务；继续原事务，或使用带明确原因的 `abandon` 后重新 `prepare`。
- `review` 的 `-Impact` 是人工语义判断；SHA、同步位置和时间由工具自动记录。
- `record` 是唯一的正式状态提升入口。成功前不得把同步描述为完成。
- `record` 不会自动 commit 或 push；状态文件应作为单独的治理工具状态提交。
- 不要手工编辑 `upstream-sync-state.json` 中的动态状态。若状态损坏，先保留现场并报告，不要用猜测值修复。

状态退出码：

```text
0   成功或状态正常
10  可读取，但存在待同步、待复核或未初始化提醒
20  被阻塞，无法继续或无法完成 record
2   参数错误、状态错误或工具执行错误
```

## 文档边界

| 位置 | 权威性和用途 |
|---|---|
| `AGENTS.md` | 本 Fork 根目录实际生效的 Agent 指令 |
| `docs/context/` | Fork 治理规则、Agent 导航和已核实的 Fork 项目上下文 |
| `docs/` | 上游/项目文档；按任务读取对应文档，保持上游文档原位 |
| `docs/upstream/` | 未来可用于存放上游同步参考资料；引导阶段暂不创建 |
| 源码和测试 | 当前可执行行为的最终证据 |

治理文档和项目上下文文档不能替代源码与测试。涉及当前运行时行为时，源码和测试是证据；上下文文档负责导航、记录已核实事实和说明 Fork 维护边界。

## 引导阶段状态

- 治理层：已为 `custom-next` 建立。
- 项目架构和功能上下文：尚未重建。
- 旧 `custom` 的功能：尚未迁移；没有证据时，不得写成已经存在于 `custom-next`。
- 上游基线和当前 Fork 差异：已记录在 `customization-map.md`。
