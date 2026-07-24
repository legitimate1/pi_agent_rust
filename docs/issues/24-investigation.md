## 调查报告 — Issue #24: abort RPC 后 pwsh/bash 子进程未被杀死

### ✅ 属实性评估

Issue 描述的问题**完全属实**，且根因定位准确。

#### pwsh.rs — 确实存在孤儿进程问题

- **文件**: `src/tools/pwsh.rs`
- `run_pwsh_command()` 函数（第 104 行）使用同步 `std::process::Command`
- 轮询循环（第 189–210 行）**只检查超时**，不检查任何 abort/cancellation 信号
- 当 `agent.rs:execute_tool_batch`（第 2758 行）通过 `select` 匹配到 abort → `Either::Right` → tool future 被 drop
- `std::process::Child` 在 Windows 上 drop 时**不会自动 kill** 子进程 → **孤儿进程**
- stdout/stderr 的读取线程（在第 160/173 行 spawn）在子进程存活时会持续阻塞在 `read()` 上

#### bash.rs — 相对安全但可加固

- **文件**: `src/tools/bash.rs`
- 使用 `ProcessGuard`（`src/tools/mod.rs:3337`）封装子进程
- `ProcessGuard::Drop`（第 3384 行）在 drop 时 spawn 线程执行 `cleanup_child` + `child.kill()` + `child.wait()`
- 主循环中已有 `cx.checkpoint()`（第 229 行）检测 ambient cancellation
- **但**仍缺少显式的 `AbortSignal` 检查，作为额外保险

#### 架构层面的发现

- `Tool::execute` trait（`src/tools/mod.rs:217`）的签名**不包含** `AbortSignal` 参数
- `agent.rs` 通过 `select` 丢弃 future 来实现 abort，工具自身无法感知
- 没有统一的跨工具 abort 传播机制

### 🛠 修改思路

#### 方案 A（推荐 — 双保险，与 Issue 建议一致）

**A1. pwsh 封装 ProcessGuard（Drop 层兜底）**
- 将 pwsh 的 `child` 改为由 `ProcessGuard` 管理（使用 `ChildOnly` 模式）
- 确保 future 被 drop 时自动 kill 子进程

**A2. pwsh 轮询循环增加 cancellation 检查**
- 类似 bash 的做法，通过 `AgentCx::for_current_or_request()` 获取 cx
- 在循环中调用 `cx.checkpoint()`，检测到 cancellation 时 `child.kill()` 并退出

**A3. bash 增加 AbortSignal 检查（可选加固）**
- 在轮询循环中增加 `AbortSignal` 检查

#### 涉及文件

| 文件 | 改动量 | 说明 |
|------|--------|------|
| `src/tools/pwsh.rs` | ~30 行 | 封装 ProcessGuard + 增加 cancellation 检查 |
| `src/tools/bash.rs` | ~10 行 | 可选：增加 AbortSignal 检查 |

### ⚠️ 风险判断

- **低风险**：改动集中在工具内部，不涉及工具接口变更
- 不需要修改 `Tool::execute` trait 签名（利用现有的 `AgentCx` ambient cancellation 机制）
- `ProcessGuard` 已在 bash 中充分使用，直接复用到 pwsh 即可
- 需要确认测试覆盖（现有 pwsh 测试 + 新增 abort 场景测试）

### 结论

建议按 **方案 A1 + A2** 修复 pwsh，bash 作为可选加固。改动量小、风险低、可测试。
