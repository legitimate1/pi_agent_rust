# 架构骨架

## 核心数据流

```
CLI (clap) → main/app/config/resources → Agent Session
                         ↓
Provider Layer (10 native provider modules + extension providers)
                         ↓
Tool Registry (built-ins + extension tools) ↔ Extension Runtime (QuickJS + capability policy)
                         ↓
Surfaces: Interactive TUI + RPC/stdin modes
                         ↓
Session persistence + index (JSONL, optional SQLite)
```

## 工具系统架构

```
                    ┌─────────────────────┐
                    │   ToolRegistry       │
                    │   Vec<Box<dyn Tool>> │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                 ▼
    ┌─────────────────┐ ┌──────────────┐ ┌──────────────────┐
    │ Built-in tools   │ │ Extension    │ │ Collection point │
│ (read/write/...) │ │ tool wrappers│ │ (agent.rs)       │
│ tools/mod.rs     │ │ extension_   │ │                  │
│                  │ │ tools.rs:100 │ │ extend_tools()   │
    └─────────────────┘ └──────────────┘ │ → dedup by name  │
                                         │ → replace builtin│
                                         │ → append new     │
                                         └──────────────────┘
```

## 扩展加载流程

1. `main.rs:1392` → `cli.enabled_tools()` 获取默认工具列表
2. `main.rs:1393-1398` → **过滤 `disabledTools` 配置**中列出的工具（如 `bash`）
3. `main.rs:1434` → `ToolRegistry::new()` 创建内置工具
4. `main.rs:1502` → `agent_session.enable_extensions_with_policy()`
5. `agent.rs:9093` → `collect_extension_tool_wrappers()` 收集扩展工具
6. `agent.rs:9095` → `self.agent.extend_tools(wrappers)` → **同名扩展工具覆盖内置工具**

## 模块关系

| 模块 | 职责 |
|:-----|:------|
| `tools/` 模块目录 | ToolRegistry + 9 内置工具模块（read/bash/pwsh/edit/write/grep/find/ls/hashline） |
| `agent.rs` | Agent 循环（工具迭代、扩展合并） |
| `extensions.rs` | 扩展管理器、能力策略、生命周期 |
| `extensions_js.rs` | QuickJS 运行时、虚拟模块、HostcallKind |
| `extension_tools.rs` | 扩展工具包装器 + 收集函数 |
| `providers/mod.rs` | Provider 工厂 + 扩展 stream-simple 桥接 |
| `models.rs` | 内置 + models.json 模型注册表 |
| `session.rs` | JSONL 会话持久化 |
