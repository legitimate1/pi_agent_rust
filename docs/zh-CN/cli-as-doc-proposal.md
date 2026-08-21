# 议题：CLI 即文档 vs 静态 .md 文档

> **📜 历史探索文档** — 本文是 CLI-as-Doc 概念的早期探索记录，保留了最初的讨论框架和备选方案。当前已收敛的设计在 [`docs/design/cli-as-doc/`](./design/cli-as-doc/) 中推进，以本文的讨论为前置背景。如需了解当前设计，请直接阅读设计文档。

## 背景

pi_agent_rust 有一个面向 QuickJS 扩展开发者的开发指南（`docs/extension-developer-guide.md`，约 20KB，13 节）。

问题：文档写的是 API 规格（registerTool 的参数、事件列表、宿主调用方法等），但这些信息在 Rust 代码中其实已经存在（struct 定义、enum 变体、JS 对象属性注册表）。代码一改，文档就过时。我们不想维护两份真相。

## 提出方案

将文档拆为两层：

### 1. 静态骨架（`docs/` 保留）
不变的或变化极慢的内容：
- 概念解释（什么是宿主调用、能力策略）
- 架构图（系统怎么拼的）
- 设计指南（工具命名、描述怎么写——经验法则，非 API 规格）
- 学习路径

### 2. CLI 生成的血肉（新增 `pi developer-guide <topic>`）
```bash
pi developer-guide events          # 列出所有事件名 + 说明
pi developer-guide register-tool   # registerTool 参数规格 + JSON Schema
pi developer-guide hostcall        # pi.tool/pi.exec/pi.http 等
pi developer-guide node-compat     # Node.js shim 兼容性矩阵
```

关键：**输出内容从代码中的数据结构自动生成**，而非手写文本。

| 主题 | 潜在数据源 | 自动同步？ |
|------|-----------|-----------|
| events | `ExtensionEventName` enum + doc comment + 元数据 | ✅ 加事件自动出现 |
| register-tool | `ExtensionToolDef` struct + `__pi_register_tool` 的验证代码 | ✅ 改签名自动更新 |
| hostcall methods | `pi` 对象的 JS 方法注册表（`extensions_js.rs`） + `HostcallKind` enum | ✅ 加宿主调用自动出现 |
| node-compat | Node.js shim 注册表（内置模块列表） | ✅ 加 shim 自动出现 |

## 需要讨论的问题

### 实现层面
1. 数据从哪里来？—— enum 变体、struct 字段的描述信息在 Rust 中怎么暴露给 CLI？
   - 方案 A：从 Rust 代码的 doc comment 或 derive 宏中提取
   - 方案 B：维护一份接近代码的 metadata 注册表（比手写文档更接近真相但仍然要维护）
   - 方案 C：在 QuickJS 运行时内省 pi 对象的方法列表和 tool 定义
2. 输出格式？—— 纯文本 / 带格式 / JSON（供其他工具消费）？
3. 翻译问题？—— 中文输出还是英文？

### 架构层面
4. 这个 CLI 子命令应该由 Rust 实现（编译进二进制），还是由一个 QuickJS 扩展实现？
   - Rust：强类型、可以直接访问所有内部类型定义、零额外依赖
   - QuickJS 扩展：可以迭代更快、但需要从运行时获取元数据
5. 如何防止这个 CLI 本身变成另一份需要维护的“代码文档”？

### 成本效益
6. 收益最明显的 topic 是哪几个？（可以先做 2-3 个验证模式）
7. 静态 .md 还保留哪些内容？（界定「骨架」的边界）
8. 对于已经存在的 extension-developer-guide.md，是一次性拆解还是逐步迁移？

---

> 这个文档是决策辅助用，不是实现方案。帮我分析可行性和 trade-off，不需要直接写代码。
