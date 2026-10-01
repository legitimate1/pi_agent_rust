# context-compaction-modernization — 实现计划总览

> 来源设计：`docs/design/context-compaction-modernization/design.md`
> 计划类型：multi-phase / architecture-roadmap
> 当前计划入口：`./plan1.md`
> 当前状态：待用户确认；未授权执行

## 整体目标

在不破坏 session entry replay、统一 autosave、扩展事件和现有默认行为的前提下，分阶段吸收上游上下文压缩改进：

1. 先让后台压缩任务具备正确的生命周期、origin 校验和 quota 应用时序；
2. 再为超大 session 增加 provider-free deterministic local fallback；
3. 统一本地 token 估算并引入 BPE fallback；
4. 增加 shake 与 shake-first 自动策略；
5. 最后完成 Agent/RPC/SDK/interactive 的契约和观测收口。

本路线不默认引入 snapcompact、MediaContent 或 provider-specific native compaction。

## 步骤总览

| 步骤 | 名称 | 依赖 | 状态 | 计划入口 |
|---|---|---|---|---|
| Phase 1 | AgentSession 后台 Worker 生命周期与 stale-result 安全 | 无 | 进行中（计划待确认） | [plan1.md](./plan1.md) |
| Phase 2 | 超大 session deterministic local fallback | Phase 1 | 待设计/待开始 | 具体计划待生成 |
| Phase 3 | 统一 token counting 与 BPE fallback | Phase 1 | 待设计/待开始 | 具体计划待生成 |
| Phase 4 | Shake 与 shake-first 自动策略 | Phase 3；复用 Phase 1 生命周期 | 待设计/待开始 | 具体计划待生成 |
| Phase 5 | RPC/interactive/session transition 表面收口、观测和契约测试 | Phase 2–4 | 待设计/待开始 | 具体计划待生成 |
| Deferred A | Snapcompact | 另行设计 | 延后 | 不生成当前计划 |
| Deferred B | Provider native compaction | 另行设计 | 延后 | 不生成当前计划 |
| Deferred C | MediaContent token 估算 | MediaContent 先决 | 延后 | 不生成当前计划 |

## 步骤依赖

```text
Phase 1 Worker 生命周期
   ├──> Phase 2 Local fallback
   └──> Phase 3 BPE/token estimator
                    ↓
              Phase 4 Shake-first
                    ↓
              Phase 5 Surface closure
```

Phase 2 与 Phase 3 在架构上可以并行，但建议先完成 Phase 2 的故障安全路径，再切入 token estimator，避免在 worker 仍可能 stale/泄漏时扩大压缩行为变化。

## 全局边界与不变量

- `SessionEntry::Compaction` 仍是压缩的持久化事实；禁止物理删除历史 entry。
- 所有 session 写入仍通过 `Session` autosave、sidecar lock、索引和 Windows contention retry。
- 后台结果只有在 origin 与当前 AgentSession 的 session/provider/model/leaf 以及 compaction generation 全部匹配时才能应用；外层 RPC/interactive/session replacement 等 surface 在 Phase 5 接入前不视为已覆盖。
- worker timeout、abort、panic 或 provider error 不产生部分 compaction entry。
- attempt quota 只有在 compaction entry 已成功持久化并安装到当前 session 后才重置。
- provider usage、BPE 和最终 fallback 的估算优先级必须统一，不能复制多套算法。
- 默认行为保持 LLM summary + text-only；新增策略必须显式选择或配置。
- `tokensAfter` 继续表示当前 branch replay 的本地估算，不是 provider 精确 usage 或账户 quota。
- 任何会改变公共配置、session details schema、provider admission、扩展 hook 取消语义或 Phase 1 外层 surface 覆盖范围的偏差，必须返回设计阶段确认。
- Phase 1 的 generation/invalidation API 必须设计成后续 RPC、interactive、fork/resume/tree surface 可复用的 crate-private/内部能力；Phase 1 不为这些 surface 伪造已完成覆盖。

## 当前状态

- 设计文档：已创建，待审核与用户确认。
- Phase 1 详细计划：已创建，待审核与用户确认。
- 生产代码：未修改。
- 自动化验证：未执行；本轮只产生设计与计划文档。
- 后续计划：只有用户确认设计并明确“开始执行”后，才进入 `implement-mode` 执行 `plan1.md`。

## 设计闸门

在进入后续 Phase 前，分别确认：

1. 配置键采用 `compaction.auto_mode`，还是严格兼容上游命名；
2. `CompactionDetails.mode` 是否作为稳定 RPC/SDK 字段公开；
3. 是否引入上游完整的 provider admission permit；当前路线默认不引入新的全局 provider 并发协议；
4. 手动 shake 是否需要同时覆盖 RPC/SDK 显式 mode 参数；
5. 是否另行启动 snapcompact 设计，而不是把它混入通用压缩核心。
