# Fork Agent 上下文

本目录是本 Fork 的治理和 Agent 上下文层。它与上游项目的普通用户文档和开发者文档明确分开。

## 当前范围

本目录目前定义的是 Fork 应该如何维护，**不**声称已经描述当前项目的架构、功能清单、命令表面或各子系统行为。这些项目事实文档将在另一个任务中，基于新的上游基线重新建立。

## 阅读顺序

进行任何 Fork 维护工作时：

1. 读取根目录 `AGENTS.md`，了解当前生效的 Agent 执行规则。
2. 读取本文，了解文档边界。
3. 读取 `upstream-sync-and-customization-charter.md`，了解维护契约。
4. 读取 `customization-map.md`，了解当前 Fork 差异和基线。
5. 如果要比较或改造上游 Agent 工作流，读取 `agent-workflow-adoption.md`。
6. 如果要设计或修改 Fork 的长期维护机制，读取 `decisions/` 下相关 ADR；当前同步状态机制见 `decisions/ADR-001-fork-sync-state-and-tooling.md`。
7. 根据具体任务，按需读取源码、测试和上游文档。

## 文档说明

### `upstream-sync-and-customization-charter.md`

定义分支、上游同步、二开修改、冲突处理、Agent 执行和例外处理的稳定维护规则。

### `customization-map.md`

记录当前 Fork 专属差异的动态状态，包括差异原因、代码位置、验证方式、生命周期和移除条件。

### `agent-workflow-adoption.md`

记录如何研究上游 `AGENTS.md`，以及哪些上游 Agent 实践被直接采用、本地化采用、暂缓采用或明确拒绝。

## 文档边界

| 位置 | 权威性和用途 |
|---|---|
| `AGENTS.md` | 本 Fork 根目录实际生效的 Agent 指令 |
| `docs/context/` | Fork 治理规则和 Agent 导航 |
| `docs/` | 上游/项目文档；按任务读取对应文档，默认不要复制到本目录 |
| `docs/upstream/` | 未来可用于存放上游同步参考资料；引导阶段暂不创建 |
| 源码和测试 | 当前可执行行为的证据 |

治理文档不能静默变成项目事实文档。如果某条规则依赖当前源码路径、命令、工作流或子系统行为，就必须对仓库进行核实，或者明确标记为待确认。

## 引导阶段状态

- 治理层：已为 `custom-next` 建立。
- 项目架构和功能上下文：尚未重建。
- 旧 `custom` 的功能：尚未迁移；没有证据时，不得写成已经存在于 `custom-next`。
- 上游基线和当前 Fork 差异：已记录在 `customization-map.md`。
