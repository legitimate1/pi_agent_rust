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
    • RPC 模式: 额外有 RpcSessionPersister 背景线程实时追加写入
       (TurnEnd/ToolResult 等消息无需等 turn 结束即可落盘)
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

    ┌──────────────────────────────────────────────┐
    │  ProcessGuard 子进程生命周期管理              │
    │  (tools/mod.rs:3337)                         │
    │                                              │
    │  spawn_managed() → 便捷构造器                │
    │  wait_with_cancellation() → 标准 wait 循环    │
    │    • ambient cancellation (cx.checkpoint)     │
    │    • abort 信号检查 (signal.is_aborted)       │
    │    • 超时 kill                                │
    │  Drop → spawn 线程 kill + wait 回收子进程     │
    │                                              │
    │  被 pwsh / bash / grep / find 使用            │
    │  全部使用 ProcessGroupTree + isolate 进程组   │
    └──────────────────────────────────────────────┘
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
| `abort.rs` | 共享 AbortHandle/AbortSignal 原语，打破 agent ↔ tools 循环依赖 |
| `app.rs` | 系统提示词构建（SYSTEM.md 加载、default_system_prompt、project context files） |
| `tools/` 模块目录 | ToolRegistry + 9 内置工具模块 + verify 内部验证引擎 |
| `tools/verify.rs` | 编辑后轻量验证引擎：文件类型检测→检查器映射（.rs/.json/.toml/.ts/.md）→进程内/外部进程执行 |
| `agent.rs` | Agent 循环（工具迭代、扩展合并、ToolDef 构建） |
| `extensions.rs` | 扩展管理器、能力策略、生命周期 |
| `extensions_js.rs` | QuickJS 运行时、虚拟模块、HostcallKind |
| `extension_tools.rs` | 扩展工具包装器 + 收集函数 |
| `rpc.rs` | RPC/stdin 服务器模式、RPC 方法分发（get_commands/get_tree/get_version 等）、RpcSessionPersister（进程侧主动会话持久化） |
| `providers/mod.rs` | Provider 工厂 + 扩展 stream-simple 桥接 |
| `models.rs` | 内置 + models.json 模型注册表 |
| `session.rs` | JSONL 会话持久化 |

## Agent 循环中的 abort 传播

```
RPC "abort" → AbortHandle::abort()
              ↓
AbortSignal (AtomicBool + Notify)
              ↓
         ┌────┴──────────────────────────┐
         │                                │
   execute_tool_batch              stream_assistant_response
         │                          select(abort_fut, stream)
   传递给 execute_tool_owned               │
         │                          "Aborted" StopReason
   传递给 execute_tool                     │
         │                          立刻返回
   传递给 execute_tool_without_hooks
         │
   传递给 tool.execute(id, input, on_update, abort)
         │
    ┌────┴────┐
    ▼         ▼
Rust 工具   扩展工具
bash/pwsh   execute_extension_tool
循环检查     │
abort +      await_js_task 循环检查
guard.kill() │
            ├─ 首次 abort → __pi_abort_task(task_id)
            │   → JS AbortController.abort()
            │   → 扩展 signal 事件触发（优雅退出）
            │   → 仍 pending → request_interrupt() 硬中断
            │
            QuickJS interrupt hook
            强制停止 JS 执行

工具执行返回后 → run_loop 立即发送 TurnEnd + AgentEnd(error:"Aborted")
```

| 路径 | 机制 | 延迟 |
|:-----|:-----|:-----|
| bash/pwsh 工具 | 循环中 `signal.is_aborted()` → `guard.kill()` → `taskkill /F /T` | ≤100ms (轮询间隔) |
| 扩展 JS 工具（优雅路径） | `await_js_task` 检测 abort → `__pi_abort_task(task_id)` → JS `AbortController.abort()` → 扩展 signal abort 事件 | ≤1 pump 周期 |
| 扩展 JS 工具（硬中断兜底） | 通知后下一轮仍 pending → `request_interrupt()` → QuickJS 中断 | 1 轮 pump + ≤1ms interrupt |
| agent_end 发送 | 工具返回后 run_loop 检测 abort 标记，立即发送 TurnEnd + AgentEnd | 立即 |
