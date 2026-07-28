# 调研报告：AI Agent 后台终端能力

> 日期：2026-07-28  
> 目的：全面调研行业主流 AI Agent 工具的后台终端/后台进程管理能力，为 pi_agent_rust 新增「后台终端模式」提供决策依据

---

## 目录

1. [执行摘要](#1-执行摘要)
2. [Claude Code 后台任务系统 — 深度拆解](#2-claude-code-后台任务系统--深度拆解)
   - 2.1 设计哲学与核心接口
   - 2.2 工具接口规范
   - 2.3 BackgroundShellManager 架构
   - 2.4 进程生命周期
   - 2.5 通知系统
   - 2.6 输出管理
   - 2.7 安全与沙箱
   - 2.8 关键设计决策
3. [行业全景对比](#3-行业全景对比)
4. [pi_agent_rust 差距分析](#4-pi_agent_rust-差距分析)
5. [结论与建议](#5-结论与建议)

---

## 1. 执行摘要

**核心结论**：Claude Code 的后台任务系统是当前行业最成熟、最可参考的实现。pi_agent_rust 已有 `ProcessGuard` + `SubprocessHandle` + `SubprocessRegistry` 等基础设施（约 80% 已就位），缺少的是一个面向 Agent 主循环的工具层封装和配套的进程管理工具。

**Claude Code 模式的核心**：
- 在现有 Bash/PowerShell 工具上加 `run_in_background` 布尔标志
- 配套 3 个管理工具：BashOutput（读输出）、KillShell（终止）、ListShells（列表）
- 进程在后台保持运行，Agent 不阻塞，完成时自动通知
- 所有进程 session 域隔离，session 结束自动清理

**pi_agent_rust 的缺口**：
- `pwsh` 工具只有同步执行模式，没有后台标志
- 没有跨工具调用的后台进程注册表（Agent 主循环级别）
- 没有输出增量读取机制
- 没有完成通知机制
- 没有大小看门狗（防进程无限输出）

---

## 2. Claude Code 后台任务系统 — 深度拆解

### 2.1 设计哲学与核心接口

Claude Code 的后台任务不是独立功能，而是**Bash 工具的扩展模式**。核心思想：

> 「对长时间运行的命令（dev server、build、tail -f），不阻塞 Agent 的工具循环，而是后台启动、异步通知完成。」

**三种后台化路径**：

| 路径 | 触发方式 | 决策者 |
|:-----|:---------|:-------|
| 显式后台 | `run_in_background: true` | 模型主动选择 |
| 超时后台 | 命令超过默认超时（~2min） | `shellCommand.onTimeout()` |
| 助理模式自动后台 | 阻塞超过 ~15 秒（仅主 agent） | `setTimeout()` + `ASSISTANT_BLOCKING_BUDGET_MS` |
| 用户手动后台 | 执行时按 `Ctrl+B` | 用户 |

`sleep` 命令被明确排除在自动后台化之外。

### 2.2 工具接口规范

#### BashTool — 后台启动

```typescript
Bash(command: string, run_in_background?: boolean, timeout?: number, description?: string)
```

参数：
- `command` — 要执行的命令
- `run_in_background` — 设为 `true` 时后台执行（默认 `false`）
- `timeout` — 超时毫秒（后台任务同样受限制）
- `description` — 命令描述（用于 UI 显示）

返回结果（后台模式）：
```json
{
  "stdout": "",
  "stderr": "",
  "backgroundTaskId": "bash_42",
  "backgroundedByUser": false,
  "assistantAutoBackgrounded": false,
  "interrupted": false
}
```

> **关键设计**：后台任务立即返回，`stdout`/`stderr` 为空。Agent 通过 BashOutput 工具后续按需读取输出。

#### BashOutput — 读取后台输出

```typescript
BashOutput(shell_id: string, mode?: "incremental" | "all")
```

- `shell_id` — Bash 返回的 `backgroundTaskId`
- `mode` — `"incremental"`（默认）：只返回上次读取后的新输出；`"all"`：返回全部保留缓冲
- 输出会被剥离 ANSI 颜色码和进度条
- 返回 header 包含状态信息：`[bash_42 status=running port=3000]`

#### KillShell — 终止后台进程

```typescript
KillShell(shell_id: string)
```

- 终止整个进程组（`tree-kill` + `SIGKILL`）
- 幂等：对已退出的 shell 返回 `alreadyExited: true`

#### ListShells — 列出后台进程

```typescript
ListShells()
```

- 返回当前 session 的所有后台 shell 列表
- 包含：shell_id、status、command、startedAt、detectedPort、exitCode/signal
- 输出示例：
  ```
  bg_1 running port=3000 npm run dev (2m ago)
  bg_2 exited exit=0 npm test (5m ago)
  bg_3 killed npm run build (10s ago)
  ```

#### PowerShellTool — Windows 等效

完全相同的参数和返回结构，只是执行引擎从 `bash` 换成 `pwsh`。

### 2.3 BackgroundShellManager 架构

这是后台任务系统的核心组件，一个**进程级单例**，与 `asyncAgentRegistry`（子 agent 系统）完全隔离。

```
BackgroundShellManager
├── spawnBackground(opts) → { ok, shellId }
├── get(shellId) → BgShell | undefined
├── listForSession(sessionId) → BgShell[]
├── readOutput(shellId, mode, expectSessionId?) → ReadResult
├── kill(shellId, expectSessionId?) → KillResult
├── killSession(sessionId) → Promise<void>
├── killAll() → Promise<void>
└── reapOrphansFromPidfiles() → PidfileRecord[]
```

**核心设计原则**：

| 原则 | 实现 |
|:-----|:------|
| **Session 域隔离** | 每个 shell 绑定 `sessionId`，session A 无法读取/终止 session B 的进程 |
| **进程组隔离** | spawn 时用 `detached: true`，使 shell 拥有自己的进程组（`sh → npm → node vite` 全树可杀） |
| **与子 agent 系统分离** | `BackgroundShellManager` 不在 `asyncAgentRegistry` 中——`Engine.run` 的等待循环不会等 dev server |
| **Crash 恢复** | pidfile 持久化到 `~/.code-shell/bg-shells/<sessionId>/`，worker 重启后通过 `reapOrphansFromPidfiles()` 发现和清理孤儿进程 |
| **容量控制** | 每 session 最大 16 个 shell (`MAX_SHELLS_PER_SESSION = 16`) |

### 2.4 进程生命周期

```
                    spawnBackground()
                         │
                    ┌────▼────┐
                    │ starting│
                    └────┬────┘
                         │ process group created
                    ┌────▼────┐
              ┌─────│ running │◄────── 大小看门狗超限 → SIGKILL
              │     └────┬────┘
              │          │
         ┌────▼───┐  ┌──▼──────┐
         │exited  │  │killed   │  ← KillShell / killSession / killAll
         └────┬───┘  └────┬────┘
              │            │
              └─────┬──────┘
              通知队列 │ 完成通知
                     ▼
              Agent 下一轮收到
```

**生命周期关键点**：

1. **启动**：`spawnBackground()` 创建 detached 进程组，写入 pidfile
2. **运行中**：stdout/stderr 合并写入同一个 fd（`O_APPEND` 保证时序）；5 秒间隔的大小看门狗监控输出文件大小
3. **退出**：进程自然退出或被杀 → `exit` 事件触发（不用 `close` 事件——`close` 会等孙子进程也关闭 fd）
4. **通知**：退出时将完成通知（一行摘要，不含输出体）推送到 Agent 通知队列
5. **清理**：pidfile 删除（log 文件保留以便后续查看）

### 2.5 通知系统

Claude Code 的通知机制是异步非阻塞的：

```
进程退出
  ↓
[后台系统] enqueueNotification(sessionId, {
    workKind: "shell",
    description: "Background shell bg_42 exited with exit=0. Use BashOutput(\"bg_42\") to inspect.",
    enqueuedAt: ...
  })
  ↓
Agent 下一轮开始时
  ↓
[系统提示] 自动注入通知内容
  ↓
Agent 看到并决定是否采取行动
```

**设计要点**：
- 通知只包含**一行摘要**，不含输出体（避免 token 浪费）
- Agent 如需查看输出，主动调用 `BashOutput` 读取
- Agent**不需要轮询**——"end your turn instead of looping Sleep + BashOutput"

### 2.6 输出管理

**文件模式（默认）**：
```
stdout ─┐
        ├──► 同一个文件 fd (O_WRONLY | O_CREAT | O_APPEND | O_NOFOLLOW)
stderr ─┘
```
- stdout 和 stderr **合并写入同一个文件**，保证时序正确
- 使用 `O_NOFOLLOW` 防止 symlink 攻击
- 后台任务直接写文件 fd，不经过 JS 事件循环（零 overhead）

**内存环形缓冲区**：输出同时保留在内存环形缓冲区中，供 `BashOutput` 增量读取。

**大小看门狗**：每 5 秒检查输出文件大小，超过 `maxOutputBytes` 阈值时 `SIGKILL` 进程（之前发生过 768GB disk fill 事故）。

**ANSI 清理**：读取输出时剥离 ANSI 颜色码和进度条，Agent 看到干净文本。

### 2.7 安全与沙箱

Claude Code 的安全性是多层的：

| 层次 | 技术 | 说明 |
|:-----|:------|:------|
| 应用层 | 命令解析 + 分类 | `bashSecurity.ts` ~2600 行命令风险评估 |
| 权限层 | 通配符模式匹配 | `bashPermissions.ts` ~2500 行 allow/deny 规则 |
| 沙箱层 | macOS `sandbox-exec` / Linux `bubblewrap` | 文件系统 + 网络隔离 |
| 内核层 | seccomp（Linux） | 系统调用过滤 |

后台任务同样受沙箱约束——启动时根据策略选择沙箱模式。

**特殊安全措施**：
- `_simulatedSedEdit` 字段始终对模型隐藏（防止模型绕过权限写文件）
- 设置文件（`settings.json`）写入无条件拒绝（防止沙箱逃逸）
- 裸 git 仓库攻击防护（防止恶意 `HEAD`/`config` 文件触发 git hook 逃逸）

### 2.8 关键设计决策

| 决策 | 选择 | 理由 |
|:-----|:------|:------|
| 后台标志 vs 独立工具 | Bash 工具上加布尔标志 | 最小化 API surface，模型不需要学新工具 |
| stdout/stderr 合并 | 合并到同一个 fd | 保证时序，简化实现 |
| 增量读取 vs 一次性返回 | 增量读取（`BashOutput incremental`） | 长进程输出可能巨大，增量读取节省 token |
| 通知 vs 轮询 | 完成时自动推送通知 | 更自然的 Agent 工作流，避免无用循环 |
| `exit` vs `close` 事件 | 用 `exit` | `close` 等待所有 fd 关闭，可能等孙子进程 |
| SIGTERM vs SIGKILL | SIGKILL（无 grace period） | 后台任务不需要优雅关闭 |
| 进程组 kill vs 单进程 | tree-kill 进程组 | 确保子进程（npm → node）一起清理 |
| Session 隔离 | sessionId 作为桶键 | 安全 + 清晰的所有权 |
| 与 asyncAgentRegistry 分离 | 独立 singleton | 不干扰子 agent 的生命周期等待 |
| Crash 恢复 | 文件系统 pidfile | worker 崩溃后重启可发现孤儿进程 |

---

## 3. 行业全景对比

### 完整矩阵

| 工具 | 后台终端能力 | 实现方式 | 接口模式 | 通知机制 | 跨对话持久 | 进程组管理 |
|:-----|:------------|:---------|:---------|:---------|:----------|:----------|
| **Claude Code** | ✅ 完整 | Bash `run_in_background` | 工具标志 + 管理工具 | 自动推送通知 | ❌ session 域 | tree-kill |
| **Codex CLI** | 🚧 讨论中 | 提议 `--bg` 标志 | proposal | 未定 | 未定 | 未定 |
| **GitHub Copilot CLI** | ✅ 隐式 | 内部 shell manager | 隐式自动 | 状态层 | ❌ | 内置 |
| **Cursor** | ✅ Cloud Agent | 隔离 VM / worktree | Agents Window GUI | 通知栏 | ✅ 云 VM | VM 级 |
| **Windsurf/Cascade** | ⛔ 无 | — | — | — | — | — |
| **Aider** | ⛔ 无 | `/run` 一次性 | — | — | — | — |
| **Devin** | ✅ 沙箱 VM | 默认持久 shell | 无特殊标志 | 日志流 | ✅ VM | VM 级 |

### 细节对比

#### 后台启动方式

| 工具 | 显式启动 | 自动后台 | 用户手动 |
|:-----|:---------|:---------|:---------|
| Claude Code | `run_in_background:true` | >15s 自动 | `Ctrl+B` |
| Copilot CLI | 隐式 | 自行判断 | — |
| Cursor | Agents Window 发任务 | — | — |
| Devin | 全部在 VM 中天然后台 | — | — |

#### 输出读取方式

| 工具 | 增量读取 | 全量读取 | 自动通知 |
|:-----|:---------|:---------|:---------|
| Claude Code | ✅ BashOutput incremental | ✅ BashOutput all | ✅ 完成时推送 |
| Copilot CLI | 通过状态层 | 通过状态层 | 部分 |
| Cursor | Agent Window 实时流 | Agent Window | ✅ 通知栏 |
| Devin | 日志流 | 日志流 | ✅ 通知 |

#### 进程生命周期绑定

| 工具 | 绑定到 | Agent 退出后 | 机器重启后 |
|:-----|:-------|:------------|:----------|
| Claude Code | session | ❌ 进程消失 | ❌ |
| Copilot CLI | session | ❌ | ❌ |
| Cursor Cloud Agent | 云 VM | ✅ 继续 | ✅ |
| Devin | 云 VM | ✅ 继续 | ❌ |

---

## 4. pi_agent_rust 差距分析

### 4.1 已有基础设施（存量）

| 组件 | 文件 | 能力 | 可复用度 |
|:-----|:-----|:------|:---------|
| **ProcessGuard** | `src/tools/mod.rs:3421` | 子进程生命周期、Drop 自动 kill、进程组树清理 | ✅ **完全复用** |
| **ProcessCleanupMode** | `src/tools/mod.rs:3427` | 进程组树 kill | ✅ **完全复用** |
| **SubprocessHandle** | `src/subprocess_handle.rs:55` | spawn、stdin write、pump 线程、输出缓冲区 | ✅ **完全复用**（需增加增量读取 API） |
| **SubprocessRegistry** | `src/subprocess_handle.rs:222` | key → handle 映射、kill_all | ✅ **完全复用**（需增加 session 域） |
| **pwsh 工具执行引擎** | `src/tools/pwsh.rs:108` | Command 构建、pipe 读取、超时、截断 | 🔄 部分复用（重构为可选后台） |
| **Tool trait** | `src/tools/mod.rs:61` | 工具注册接口 | ✅ **完全复用** |
| **ToolRegistry** | `src/tools/mod.rs:2890` | 工具注册表 | ✅ **完全复用** |

### 4.2 需新增/改造的组件（增量）

| 组件 | 描述 | 估算规模 | 优先度 |
|:-----|:------|:---------|:-------|
| **`background.rs`** | 后台终端工具组（`background_start` / `background_list` / `background_read` / `background_stdin` / `background_kill`） | ~400-600 行 | **P0** |
| **`BackgroundRegistry`** | Agent 主循环级的后台进程注册表（类比 Claude Code 的 `BackgroundShellManager`），支持 session 域隔离 | ~200 行 | **P0** |
| **pwsh 改造** | 加 `run_in_background` 参数 → 走 `BackgroundRegistry.spawn()` | ~50 行 | **P0** |
| **完成通知机制** | 进程退出时向 Agent 的通知队列推送事件 | ~100 行 | **P1** |
| **大小看门狗** | 监控后台进程输出大小，超限自动 kill | ~50 行 | **P1** |
| **pidfile 持久化** | 写入文件系统，支持 crash 后恢复 | ~80 行 | **P2** |
| **端口检测** | 从输出中检测 http 端口（bonus） | ~30 行 | **P2** |

### 4.3 功能覆盖度对比

| 功能 | Claude Code | pi_agent_rust 当前 | 备注 |
|:-----|:-----------|:-------------------|:------|
| 后台启动 | ✅ `run_in_background` | ❌ 只有同步 `pwsh` | **P0** |
| 读取输出 | ✅ BashOutput（增量/全量） | ❌ | **P0** |
| 终止进程 | ✅ KillShell | ⚠️ 仅 ProcessGuard.kill() 底层 | **P0** |
| 列出进程 | ✅ ListShells | ❌ | **P0** |
| session 域隔离 | ✅ 跨 session 不可见 | ⚠️ SubprocessRegistry 无 session 概念 | **P0** |
| 完成通知 | ✅ 自动推送到对话 | ❌ | **P1** |
| 大小看门狗 | ✅ 5s 间隔 → SIGKILL | ❌ | **P1** |
| stdout/stderr 合并 | ✅ 同 fd O_APPEND | ⚠️ 分开读 | 可选项 |
| 超时后台化 | ✅ >15s 自动后台 | ❌ | **P2** |
| 端口检测 | ✅ 最佳努力 | ❌ | **P2** |
| crash 恢复 | ✅ pidfile | ❌ | **P2** |
| PTY 支持 | ❌ 不用 PTY | ❌ 不用 PTY | 行业共识 |
| 跨对话持久 | ❌ session 域 | ❌ | 不做 |
| 沙箱集成 | ✅ sandbox-exec/bwrap | ❌ | 未来 |

### 4.4 实现策略总览

```
当前架构:
  pwsh 工具 ──► ProcessGuard ──► 一次性进程

目标架构:
  pwsh 工具 ──┬─► (run_in_background=false) ──► 一次性进程 (不变)
              │
              └─► (run_in_background=true) ──► BackgroundRegistry
                                                   ├── 后台 pump 线程
                                                   ├── 输出缓冲区（增量读取）
                                                   ├── 大小看门狗
                                                   └── 完成时 → 通知队列
```

---

## 5. 结论与建议

1. **Claude Code 是唯一值得全面参考的实现**。其他工具要么不如它成熟（Codex 讨论中），要么定位不同（Cursor/Devin 是云 VM 级），要么不足（Aider/Windsurf 无此功能）

2. **pi_agent_rust 的基础设施已到位 80%**。`ProcessGuard` + `SubprocessHandle` + `SubprocessRegistry` + Tool trait 构成了坚实的底层，缺失的是 Agent 主循环级的工具封装

3. **建议分阶段实施**：
   - **Phase 1（P0）**：`BackgroundRegistry` + `background_start/read/kill/list` 四个工具 + `pwsh` 支持 `run_in_background`
   - **Phase 2（P1）**：完成通知机制 + 大小看门狗
   - **Phase 3（P2）**：pidfile crash 恢复 + 端口检测

4. **不做**：PTY 模拟、跨对话持久化、tmux 集成——这些不在范围内，也与行业共识一致

5. **总量估算**：Phase 1 约 **600-900 行 Rust**，不涉及 provider/TUI/config 层改动，纯新增工具层
