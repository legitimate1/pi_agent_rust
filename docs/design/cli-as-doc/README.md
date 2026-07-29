# CLI as Doc — 多阶段设计总览

## 决策回溯

本设计基于 [[2026-07-29-cli-as-doc-决策记录]] 的结构化决策推进。

核心定位：

- **不追求「CLI 即文档」**，而是建立**契约驱动的开发者参考体系**
- **CLI 是版本绑定的查询入口**，不是新的维护负担
- **内容按「事实—语义—教学」三层边界**分层，机械事实由实现派生或共用

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
