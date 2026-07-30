# CLI as Doc — 多阶段设计总览

## 决策回溯

本设计基于 [[2026-07-29-cli-as-doc-决策记录]] 的结构化决策推进。

核心定位：

- **长期模型**：Rust 公共契约声明 + 最小化 QuickJS 运行时探针 + 一致性/行为测试 → `pi developer-guide` 文本/JSON 输出。Markdown 参考文档后续从同一投影生成——手写 Markdown 不是 API 真相来源。
- **契约包含运行时无法推断的语义**：稳定性、能力要求、限制、简洁的人工描述。运行时探针验证 QuickJS 实际暴露的内容；测试是可执行的证据，不是同等真相来源。
- **CLI 是版本绑定的查询入口**，不是新的维护负担。
- **内容按「事实—语义—教学」三层边界**分层，机械事实由实现派生或共用。

## 冻结决策检查点（2026-07-30）

本轮讨论已确定 `pi developer-guide` 的架构方向；在开始实现前，以下结论冻结，不在扩展技能重构讨论中继续展开。

### 已确认

- 公共扩展 API 的长期链路是：**Rust 公共契约声明 + 最小 QuickJS 运行时探针 + 一致性/行为测试**，派生 `pi developer-guide` 的 text/JSON 输出；未来 Markdown reference 也只从同一投影生成。
- Rust 契约只声明运行时无法推断的公共语义：稳定性、能力要求、限制和简洁的人类说明。运行时探针验证 QuickJS 实际暴露面；测试是可执行证据，不是并列的事实来源。
- Phase 1 只覆盖 `events` 和 `register-tool`，提供 text/JSON，不提供静态 Markdown renderer，也不提供面向扩展作者的生产级 runtime introspection API。
- `pi.on()` 在 Phase 1 保持宽松，未知字符串不变成运行时错误。文档中的事件集以“Rust 实际 dispatch 且已注册 handler 可在最小 QuickJS runtime 中观察到”为准。
- `register-tool` 的字段接受、默认和拒绝行为必须通过实际 QuickJS 注册路径验证，不能只由契约元数据声称。
- 旧静态开发指南只保留概念、设计建议、工作流和教学内容；精确 API reference 最终由 `pi developer-guide` 提供。

### 与 `pi-extension-dev` 的接口

在 `pi developer-guide` 尚未实现前，扩展技能中的 API/能力文件只作为证据索引：引导读取对应 Rust 实现和测试，不重复维护完整 API 事实。`pi developer-guide` 可用后，索引改为路由到它的派生输出。

### 暂缓到 B 树恢复时处理

- 最小 QuickJS runtime probe 如何加载 fixture extension 并建立当前扩展上下文。
- `events` 契约与实际 dispatch 路径的完整枚举、计数和映射校验。
- `register-tool` 的精确事实表、契约与 `ExtensionToolDef`/JS 验证逻辑的共享或校验边界。
- `DeveloperReference` 的最终字段与后续 `node-compat`、`hostcall` topic 的演进。

## 阶段总览

| 阶段 | 范围 | 状态 | 文档 |
|:-----|:------|:------|:------|
| Phase 1 | `events` + `register-tool` 契约闭环、CLI 文本/JSON renderer、门禁 | ⏳ **设计中** | [phase-1-events-register-tool.md](./phase-1-events-register-tool.md) |
| Phase 2 | `node-compat` 兼容语义 | 📌 待定 | — |
| Phase 3 | `hostcall` 完善 | 📌 待定 | — |
| 贯穿 | 旧文档渐进迁移、静态 Markdown renderer | 📌 待定 | — |

## 当前状态

**Phase 1 正在进行**。当前设计文档覆盖：

- 事件契约（`events`）的数据模型、权威来源和展示
- 工具注册契约（`register-tool`）的数据模型、权威来源和展示
- 统一的 `DeveloperReference` 公共参考模型
- CLI 文本和 JSON renderer 设计
- 一致性门禁方案

## 注意事项

- Phase 2 依赖 Phase 1 的契约模型稳定后再启动
- Phase 3 依赖 Phase 1 + Phase 2，且涉及 `HostcallKind` 路由，范围较广
- 旧文档迁移在每个 phase 完成 topic 闭环后立即执行，不等到所有 phase 完成
- JSON 输出 schema 在 Phase 1 标记为实验性，不承诺永久兼容
