# 上游合并 Checkpoint

> 当前策略：release 锚点 + 结构性边界。所有 checkpoint 都是 `upstream/main` 历史中的累计状态；不使用 cherry-pick。

## 基线

- custom/main 共同基线：`226a876425a856f657b2a5d7c7ac6f0ca1ad25f1`
- 基线提交：`Bridge ctx.compact() to the native engine with the session's credentials.`
- 当前上游：`e403485b3116e6c97e9af7026ec9445f30312c7d`
- 当前上游主题：nightly toolchain pin（`2026-08-31`）

区间统计来自 Git 的累计差异。文件数包含源码、测试、文档、生成物和配置；源码文件数只统计 `src/` 与 `crates/`。新增/删除行数不能直接当作模型上下文大小。

## 主 checkpoint

### C1 — `v0.3.0`

- 提交：`e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`
- 类型：release 锚点
- 主题：上游第一段 release 状态
- 相对共同基线：317 commits；其中 310 个非 merge
- 区间变化：351 files；118 个源码文件
- 行变化：+93,550 / -4,141
- 目的：建立第一个可验证的上游累计状态
- 风险：区间仍较大，模型不得读取 317 个提交全文；只处理真实冲突和首个验证失败
- 状态：待执行

### C2 — `c87618f3e3479851935758ec78d9afa94f24bf41`

- 类型：结构性/功能波次边界
- 提交主题：完成 OMP-ADOPT 多个父级功能域
- 相对 C1：253 commits；其中 238 个非 merge
- 区间变化：761 files；66 个源码文件
- 行变化：+92,205 / -1,299
- 目的：在主要工具、浏览器、运行时功能域收束后建立观察点
- 风险：文件变化中包含大量测试、证据和 Beads 数据，不能按文件总数估算语义冲突
- 状态：待执行

### C3 — `9b2851e40a03fbc1859ff014e93b9d62cb568129`

- 类型：结构性边界
- 提交主题：FrankenTUI 默认切换完成并验证
- 相对 C2：279 commits；其中 278 个非 merge
- 区间变化：165 files；51 个源码文件
- 行变化：+24,723 / -6,117
- 目的：把 TUI 迁移从实验路径切到默认路径，形成独立行为验证边界
- 风险：交互、TUI、扩展桥接和会话持久化可能存在行为重叠
- 状态：待执行

### C4 — `1ddafe8957464c55a497a435e96b206d276a0f11`

- 类型：功能波次边界
- 提交主题：扩展 background jobs、models catalog 和 resource routing
- 相对 C3：275 commits；其中 274 个非 merge
- 区间变化：109 files；57 个源码文件
- 行变化：+60,404 / -8,208
- 目的：把 jobs、models、resources 的关联变化作为一个可验证状态
- 风险：可能与 custom 的 RPC、session、provider 和资源路由定制重叠
- 状态：待执行

### C5 — `v0.4.0`（`5bd3e3537de6509fb9f3bfbed4059e10fbf914da`）

- 类型：release 锚点
- 主题：上游 `v0.4.0` release 状态
- 相对 C4：98 commits；全部为非 merge
- 区间变化：172 files；76 个源码文件
- 行变化：+49,029 / -5,461
- 目的：建立第二个 release 级验证状态
- 风险：发布、性能证据、依赖和 FTUI 收尾变化可能混杂
- 状态：待执行

### C6 — `upstream/main`（`e403485b3116e6c97e9af7026ec9445f30312c7d`）

- 类型：当前上游末端增量
- 主题：proxy、RPC 测试任务 pin、MCP/RPC/扩展收尾、nightly toolchain
- 相对 C5：78 commits；其中 76 个非 merge
- 区间变化：148 files；41 个源码文件
- 行变化：+16,300 / -5,586
- 目的：在 release 状态稳定后再吸收当前上游增量
- 风险：HTTP proxy 与 `src/http/client.rs` 的变化需要单独验证
- 状态：待执行

## 区间总览

```text
共同基线
  → C1 v0.3.0                 317 commits / 351 files / 118 src files
  → C2 OMP-ADOPT 收束点       253 commits / 761 files / 66 src files
  → C3 FrankenTUI 默认切换    279 commits / 165 files / 51 src files
  → C4 jobs/models/resources   275 commits / 109 files / 57 src files
  → C5 v0.4.0                  98 commits / 172 files / 76 src files
  → C6 当前 upstream/main       78 commits / 148 files / 41 src files
```

## 自适应细分

C4 到 C5 如果出现无法归因的源码冲突或验证失败，再插入以下备用点之一：

- `b80676410917855ac0a5e679a263e8b189be09f9`：MCP streamable HTTP 合同测试落地。
- `b2d894b909a47322dcf6a003ddd51fce2e157f32`：checkpoint/retry 兄弟分支拓扑落地。

备用点不是默认阶段，只有在实际 integration 试合并显示 C4→C5 过于混杂时启用。

## 每阶段停止条件

阶段完成必须同时满足：

- Git 无未解决冲突；
- 针对性测试和必要的静态检查通过；
- 未出现未解释的依赖漂移；
- Fork 政策文件没有被上游自动化无条件覆盖；
- 已记录保留、采纳、冻结三类决策。

出现以下任一情况就停止当前阶段：

- 需要大规模结构迁移但尚未形成设计；
- 测试失败无法区分基线问题和本阶段引入的问题；
- 依赖升级导致大面积错误且没有适配计划；
- 上游机器人或发布自动化违反 Fork 政策；
- 模型需要读取整个仓库才能继续判断。

## 模型上下文边界

模型每次只接收：

- 当前 checkpoint 与上一个 checkpoint 的范围摘要；
- Git 实际产生的冲突文件和冲突块；
- 相关 custom 二开意图；
- 首个编译或测试失败；
- 当前阶段已经确认的短决策摘要。

模型不接收：

- 全部上游提交全文；
- 全部 657 个 custom 差异文件；
- 测试快照和生成物的完整差异；
- 已经由 Git 或验证工具确定的事实。

## 当前执行状态

- checkpoint 已筛选，尚未创建 integration 线。
- C1 是下一次试合并目标。
- 本文件记录候选和规则，不表示任何 checkpoint 已经合入 `custom`。
