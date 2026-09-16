# 上游语义波次索引

> 本文只负责导航：指出当前上游分析窗口、当前工作位置和波次文档路径。不在这里写波次的完整分析、冲突细节或验证过程。
> 更新日期：2026-09-16

## 当前工作位置

第一执行子窗口的只读分析已完成，结论是：fsqlite session storage 是已有 SQLite session 后端的替换，不直接 merge；如需采用，应另立 SQLite 后端迁移波次。

波次 02 的 provider usage/quota 已完成语义分析、隔离 cherry-pick 探针、基于 custom 的手动适配和局部回归验证；结论是：不原样 merge 上游提交，保留 custom 结构完成适配，当前实现和局部验证已完成。

波次 03 已完成 `tokensAfter` compaction result contract 的只读分析；结论是：上游新增 compaction 后上下文规模估算并传播到 RPC/SDK，当前 custom 有对应 compaction/RPC/SDK 接入点但缺少 `tokens_after`，推荐进入后续实现决策；同提交中的 grep/search 依赖仍需单独核对，不直接 cherry-pick。

当前详细记录：

- `waves/01-fsqlite-session-storage.md`
- `waves/02-provider-usage-quota.md`
- `waves/03-compaction-tokens-after.md`

当前窗口边界：

- 起点：`226a876425a856f657b2a5d7c7ac6f0ca1ad25f1`
- 宏观终点：`v0.3.0` / `e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`
- 第一执行子窗口终点：`8f12352e174ea06c7d8d66cde15768cecfccccf3`
- 窗口详情：`analysis-window.md`

## 波次文档索引

### 已记录波次

#### 01 — fsqlite session storage

- 主题：上游把 `sqlmodel-*` SQLite session 后端替换为 `fsqlite 0.3.4`，并同步改变连接线程、错误、侧车和权限契约。
- 结论：不直接 merge；暂时冻结为独立的 SQLite 后端迁移议题。
- 详细文档：`waves/01-fsqlite-session-storage.md`
- 当前处理：不修改源码，不执行 merge。

### 当前波次

Wave 3 已完成 `tokensAfter` compaction result contract 的只读语义分析；当前未开始源码迁移。推荐先由用户决定是否进入实现设计，再核对同一上游提交中混入的 grep/search 依赖。

当前详细记录：

- `waves/03-compaction-tokens-after.md`

### 已记录波次

#### 01 — fsqlite session storage

- 主题：上游把 `sqlmodel-*` SQLite session 后端替换为 `fsqlite 0.3.4`，并同步改变连接线程、错误、侧车和权限契约。
- 结论：不直接 merge；暂时冻结为独立的 SQLite 后端迁移议题。
- 详细文档：`waves/01-fsqlite-session-storage.md`
- 当前处理：不修改源码，不执行 merge。

#### 02 — provider usage/quota surface

- 主题：上游新增 OpenRouter、Moonshot/Kimi、GitHub Copilot 等 provider 的账户额度/余额查询，并通过 `pi usage` 与 `/usage` 展示。
- 结论：不原样 merge；已按当前 custom 的认证、HTTP、CLI 和 interactive 边界完成手动适配。
- 详细文档：`waves/02-provider-usage-quota.md`
- 当前处理：本地适配和局部验证已完成；真实 provider 网络验证及全量质量门禁尚未执行。

### 后续候选

#### SQLite 后端迁移

- 主题：评估从 `sqlmodel-*` 到 `fsqlite` 的完整迁移。
- 详细文档：待创建。
- 前置：确认是否愿意承担 fsqlite 依赖树、线程模型、错误类型、侧车文件、只读行为、并发验证和发布体积变化。

#### FTUI foundation

- 主题：FrankenTUI 初始迁移、启动路径和交互基础。
- 详细文档：待创建。
- 说明：属于结构性变化，不与 SQLite 后端迁移混合。

#### MCP / RPC protocol

- 主题：MCP client、RPC、SSE 和 extension host protocol 的协同变化。
- 详细文档：待创建。
- 说明：需要单独核对协议、生命周期和 custom 接入点。

#### 其他功能闭包

- tools、providers、compaction、workspace、Hub 增量以及依赖迁移，待后续按语义拆分。
- 每个真正开始分析的主题都建立独立波次文档，再在这里登记路径。

## 历史记录入口

以下文件不是当前波次，也不放入波次索引；它们保存过去的事实和验证依据：

- `plan-hub-minimal.md`：Hub 最小闭包的完成记录。
- `probe-report-2026-08-29.md`：全量 merge 探针及其失败原因。
- `known-test-failures.md`：带日期的测试基线。

## 波次切换规则

完成一个波次的分析或处理后：

1. 在对应的 `waves/<wave-name>.md` 中补充处理结论、实际结果和验证摘要。
2. 将本文的“当前波次”改为下一个波次，并登记原波次的路径和一句话结论。
3. 不删除原波次文档，也不因为完成而移动到其他目录。
4. 只有验证材料很长、需要独立复用或由 CI/工具生成时，才另建独立证据文件。

> 波次文档是语义理解档案；本文是当前工作索引。两者都不以 Markdown 表格承载信息。
