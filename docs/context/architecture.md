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

- **`abort.rs`** → 共享 AbortHandle/AbortSignal 原语，打破 agent ↔ tools 循环依赖
- **`app.rs`** → 系统提示词构建（SYSTEM.md 加载、default_system_prompt、project context files）
- **`tools/` 模块目录** → ToolRegistry + 9 内置工具模块 + verify 内部验证引擎
- **`tools/verify.rs`** → 编辑后轻量验证引擎：文件类型检测→检查器映射（.rs/.json/.toml/.ts/.md）→进程内/外部进程执行
- **`agent.rs`** → Agent 循环（工具迭代、扩展合并、ToolDef 构建）
- **`extensions.rs`** → 扩展管理器、能力策略、生命周期
- **`extensions_js.rs`** → QuickJS 运行时、虚拟模块、HostcallKind
- **`hostcall_amac.rs`** → AMAC 批量调度器：hostcall 按类型分组，stall 遥测驱动并发/串行决策
- **`extension_tools.rs`** → 扩展工具包装器 + 收集函数
- **`rpc.rs`** → RPC/stdin 服务器模式、RPC 方法分发（get_commands/get_tree/get_version 等）、RpcSessionPersister（进程侧主动会话持久化）
- **`providers/mod.rs`** → Provider 工厂 + 扩展 stream-simple 桥接
- **`providers/`（12 个模块）** → 12 个 native provider 实现：anthropic/openai/openai_responses/gemini/cohere/azure/bedrock/vertex/copilot/gitlab/cursor/model_fetch
- **`provider_metadata.rs`** → Provider 元数据：别名、认证键、本地 provider（ollama/llamacpp/mistralrs/lmstudio）
- **`auth.rs`** → 凭据管理：API Key / OAuth / AWS / Bearer，auth.json 文件锁
- **`models.rs`** → 内置 + models.json 模型注册表
- **`session.rs`** → JSONL 会话持久化
- **`session_store_v2.rs`** → V2 sidecar：分段日志 + 偏移索引 + 检查点回滚
- **`autocomplete.rs`** → `@` 文件引用 + `/` 命令补全索引（WalkBuilder + 30s 后台刷新）
- **`resources.rs`** → Skills / prompt templates / themes / extensions 资源加载
- **`package_manager.rs`** → 包安装/移除/更新（pi install 等）
- **`doctor.rs`** → 环境诊断（config/dirs/auth/sessions/swarm preflight）

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

- **bash/pwsh 工具**：循环中 `signal.is_aborted()` → `guard.kill()` → `taskkill /F /T`；≤100ms (轮询间隔)
- **扩展 JS 工具（优雅路径）**：`await_js_task` 检测 abort → `__pi_abort_task(task_id)` → JS `AbortController.abort()` → 扩展 signal abort 事件；≤1 pump 周期
- **扩展 JS 工具（硬中断兜底）**：通知后下一轮仍 pending → `request_interrupt()` → QuickJS 中断；1 轮 pump + ≤1ms interrupt
- **agent_end 发送**：工具返回后 run_loop 检测 abort 标记，立即发送 TurnEnd + AgentEnd；立即

## 运行时不变量

1. **Turn 作用域的 agent 生命周期** — 主循环按稳定顺序发 `AgentStart → TurnStart → TurnEnd → AgentEnd`；工具递归由 `max_tool_iterations`（默认 50）封顶
2. **Abort/超时行为显式化** — abort 检查在 turn 边界与工具执行处；bash 超时走升级路径（终止进程树 → 宽限期 → 硬杀）
3. **会话写入 crash-resilient** — JSONL 保存经临时文件 + 原子 persist；会话索引用 SQLite WAL + 锁文件协调多实例
4. **Compaction 阈值驱动、边界感知** — 触发条件为估算 token 超 `context_window - reserve_tokens`；cut-point 优先 user turn 边界，保留近期上下文预算
5. **能力策略 fail-closed、优先级明确** — 解析顺序：per-extension deny → 全局 deny → per-extension allow → 默认 caps → 模式回退
6. **流式解析器容忍真实网络分块** — CR/LF 变体、多行 `data:`、UTF-8 部分尾部、EOF flush

## Extension Hostcall 协议

扩展（QuickJS 沙箱）通过 hostcall 请求与宿主通信，`ExtensionDispatcher` 检查能力策略后分派到 ToolRegistry / HTTP / session 等：

- **`pi.tool(...)`**：`read`/`write`/`exec` 等；read/write 否，exec 是
- **`pi.http(request)`**：`http`；否
- **`pi.exec(cmd, args)`**：`exec`；是
- **`pi.env(key)`**：`env`；是
- **`pi.session(op, ...)`**：`session`；否
- **`pi.ui(op, ...)`**：`ui`；否
- **`pi.log(entry)`**：`log`；否（始终允许）

关键机制：

- **双执行通道**：fast lane（已知安全模式，省分配/解析）与 compatibility lane（兜底），两者走同一能力策略；`forced_compat_*_kill_switch` 可全局/单扩展强制兼容通道
- **Shadow dual execution**：采样小部分只读 hostcall 双通道执行并对指纹，分歧超预算自动回退 fast lane
- **AMAC 批量调度**：批量 hostcall 按类型分组（session 读/写、events 读/写、tool、exec、http、ui、log），stall 遥测（EMA）驱动每组决策——并发组（session 读/tool/exec/http/log）按自适应宽度真正并行执行并保序收集，串行组（session 写/events 写/ui）严格保序；**Exec 组例外**：秒级子进程阻塞的并发收益是确定性的，跳过遥测门槛（Rule 3/4）冷启动即并发、宽度 = min(batch, max_width)；`PI_HOSTCALL_AMAC_EXEC_INTERLEAVE=0` 强制串行（逃生通道），`PI_HOSTCALL_AMAC_MIN_TELEMETRY=0` 关闭全局冷启动保护
- **Trust 生命周期**：`pending → acknowledged → trusted → killed`；kill switch 隔离扩展并写审计记录，lift 需显式操作
- **命令级 exec 调解**：spawn 前分类 command+arg 签名，默认阻断关键危险类（递归删除、磁盘/设备写入、反弹 shell），safe/strict 策略可阻断 shutdown/杀进程/改凭据文件
- **决策引擎**：CUSUM+BOCPD 负载体制检测、Conformal 预测包络（自适应异常阈值）、PAC-Bayes 安全界（不确定时否决激进优化）、IPS/WIS/DR + ESS 离策略评估（样本不足/高后悔 fail-closed）、VOI 实验选择、OCO 后悔跟踪回滚

## Compaction 算法

```
全量对话 → 找合法 turn 边界 cut point → LLM 摘要旧消息
        → 存 Compaction entry 到 JSONL → 下次调用用 [summary] + 近期消息
```

- 触发：每 agent turn 后，估算 token 超 `context_window - reserve_tokens`
- Token 估算：保守 `chars ÷ 4` 启发 + 图片固定 1200 token；assistant `usage` 字段存在时优先
- Cut point：优先完整 user-assistant turn 边界；被迫中途切时包含前缀消息保上下文连贯
- 文件操作追踪：被摘要消息里的 read/write/edit 路径提取进摘要提示词（`<read-files>`/`<modified-files>`）
- 手动触发：`/compact`（交互）或 RPC `compact`

## 会话索引 + Store V2

**SQLite 索引**（`session-index.sqlite`）：会话元数据表（path/id/cwd/timestamp/message_count/last_modified/size_bytes），保存后 upsert；`pi -c` 按 `cwd + last_modified DESC` 查询；WAL + 锁文件串行化并发写入；索引过期触发全量重扫。

**Store V2 sidecar**（`session_store_v2.rs`）：分段追加日志 + 偏移索引行（直接 seek/快速尾读）+ 周期检查点 + 迁移台账。恢复路径：sidecar 新鲜 → 从索引+分段打开；过期 → 回退 JSONL；索引损坏但分段有效 → 重建。完整性：帧带 payload+chain hash，索引行 CRC32C，截断尾帧可恢复，非 EOF 帧损坏 fail-closed。`pi migrate` 做 JSONL→V2 迁移。

## TUI 架构（交互模式）

Elm 架构（Model-Update-View），`crossterm` + `bubbletea` 家族：

```
Terminal (crossterm raw mode / alt screen)
   ↓
bubbletea Program Loop (Init → Update(Msg) → View)
   ↓
PiApp (Model) ── TextArea(editor) + Viewport(convo) + Spinner(status)
   ↓  overlay 栈：Model Selector / Session Picker / /tree / Settings / Capability Prompt
   ↓  mpsc 异步通道
Agent Async Task（asupersync 运行时，流式 provider 响应 + 执行工具）
```

- 异步/同步桥：agent 在独立线程跑 asupersync 运行时，经 mpsc 把每个流事件（text delta/tool start/tool update/agent done）作为 `PiMsg` 投递到 `PiApp::update()`
- 视口自动跟随：用户未上滚时跟随流尾部；上滚禁用，按 `End` 或发消息恢复
- 渲染缓存：按消息缓存 markdown→ANSI 渲染，仅主题/尺寸变化失效；`RenderBuffers` 预分配复用；帧时序遥测（>16ms 慢帧按阶段分类）；内存压力三级（Normal/Pressure/Critical 渐进降级）

## SSE 解析器

自研状态机（`src/sse.rs`）：`Bytes → Line Accumulator → Event Parser → Typed StreamEvent`。

- 12 种事件变体（MessageStart/ContentBlockStart/Delta/Stop/MessageDelta/MessageStop/Ping/Error + thinking 事件）
- 处理多行 `data:`、TCP 分块跨界事件、`event:` 在 `data:` 前后、CRLF/LF 混用、UTF-8 部分尾部缓冲
- 错误事件记录不崩流；固定大小滚动缓冲防无界增长

## 认证与凭据管理

凭据存 `~/.pi/agent/auth.json`（文件锁防并发损坏）。存储值可为字面量、`$ENV:VAR` 引用、`$CMD:shell command`（请求时解析 trimmed stdout）。

- **API Key**：Anthropic/OpenAI/Gemini/Cohere 等；环境变量或 settings
- **OAuth**：Anthropic/OpenAI Codex/Gemini CLI/Antigravity/Kimi/Copilot/GitLab/扩展定义；PKCE + 自动刷新；Kimi 用 device flow
- **AWS 凭据**：Bedrock；access key + secret + 可选 session token
- **Service Key**：SAP AI Core；client id/secret 换 bearer
- **Bearer Token**：自定义 provider；静态 token

`pi config` 报告各 provider 凭据状态：`Missing` / `ApiKey` / `OAuthValid`（含过期倒计时）/ `OAuthExpired` / `AwsCredentials` / `BearerToken`。认证失败返回机器可读诊断码（MissingApiKey/InvalidApiKey/QuotaExceeded/OAuthTokenRefreshFailed 等）。
