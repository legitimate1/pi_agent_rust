# Asupersync 利用清单

- Bead: `bd-xdcrh.1`
- 日期: 2026-03-10
- 范围: 仅识别那些更充分利用 `asupersync` 有望改善取消正确性、截止时间传播、静默关闭或运行时归属的位点。

这不是一份重写愿望清单。Pi 已在运行时引导、提供方 HTTP/TLS 栈、扩展预算模型以及众多工具/测试路径中从 `asupersync` 获得了实实在在的收益。本文的目标是留下一张精确的地图，标出剩余的高投入产出比位点，使下游 Beads 能从具体的代码位置出发，而不必重复考古。

## 决策图例

- `keep`：当前形态已符合运行时意图，或改动很可能仅具美观价值。
- `refactor`：置信度高、影响面可控的改进。
- `defer`：未来可能的工作，但需在更高价值的上下文/线程归属修复之后，或在有实测证据支撑时再进行。

## 集群 1：扩展/会话辅助层仍在创建全新的请求上下文

- 建议: `refactor`
- 原因: 这些辅助函数可从已拥有有意义的当前 `Cx` 或管理器级超时的扩展宿主调用与交互式事件流中触达。在辅助边界处创建全新的请求作用域，会使会话加锁与即时保存工作与调用方的截止时间/取消语义脱节。
- 最小化定向重构:
  - 在通过 trait 传递 `&AgentCx` 较为别扭的 trait/辅助边界处，使用 `AgentCx::for_current_or_request()` 或 `Cx::current().unwrap_or_else(Cx::for_request)`。
  - 若辅助函数已处于内部 API 边界，优先直接接受 `&AgentCx`，而非在内部重新创建。
- 精确模块与函数:
  - `src/session.rs`
    - `impl ExtensionSession for SessionHandle::{get_state,get_messages,get_entries,get_branch,set_name,append_message,append_custom_entry,set_model,get_model,set_thinking_level,get_thinking_level,set_label}`
  - `src/interactive/ext_session.rs`
    - `InteractiveExtensionHostActions::append_to_session`
    - `impl ExtensionSession for InteractiveExtensionSession::{get_state,get_messages,get_entries,get_branch,set_name,append_message,append_custom_entry,set_model,get_model,set_thinking_level,get_thinking_level,set_label}`
  - `src/agent.rs`
    - `AgentSessionHostActions::append_to_session`
- 保持不变:
  - `ExtensionSession` 与 `ExtensionHostActions` 之间的 trait 划分
  - 扩展消息当前的空闲 vs 流式入队语义
- 下游 Beads:
  - `bd-xdcrh.2.1`
  - `bd-xdcrh.5`

## 集群 2：Agent/RPC 辅助层已拥有活跃上下文，却仍重建请求作用域

- 建议: `refactor`
- 原因: 顶层异步流程已具备自然的请求生命周期，但若干嵌套辅助函数仍重新创建 `AgentCx::for_request()` 而非继承该作用域。这会削弱未来的截止时间/检查点工作，并使取消行为的可解释性低于应有水平。
- 最小化定向重构:
  - 在每条 run/turn 路径顶部附近创建单一 `AgentCx`，并向下传递克隆或引用。
  - 保留已有的显式 `cx` 模式，并在其下方的嵌套辅助函数中扩展该模式。
- 精确模块与函数:
  - `src/rpc.rs`
    - `run`
    - `run_prompt_with_retry` 已接受 `cx: AgentCx`；保留该模式
    - `run_extension_command` 已接受 `cx: AgentCx`；保留该模式
    - `apply_thinking_level`
    - `apply_model_change`
    - `maybe_auto_compact`
    - `rpc_emit_extension_ui_request`
  - `src/agent.rs`
    - `AgentSession::{set_provider_model,sync_runtime_selection_from_session_header,maybe_compact,apply_compaction_result,compact_synchronous,enable_extensions_with_policy,save_and_index,persist_session,revert_last_user_message}`
  - `src/main.rs`
    - `run` 中的会话快照/模型历史/关闭刷盘加锁区段
- 保持不变:
  - `src/rpc.rs::run_prompt_with_retry` 携带显式 `AgentCx` 参数
  - `src/rpc.rs::run_extension_command` 携带显式 `AgentCx` 参数
  - CLI/引导与模式特定执行之间的现有运行时划分
- 下游 Beads:
  - `bd-xdcrh.2.2`
  - `bd-xdcrh.2.3`
  - `bd-xdcrh.3`
  - `bd-xdcrh.5`

## 集群 3：会话持久化与发现仍依赖原生线程加 oneshot 会合

- 建议: `refactor`
- 原因: 原生线程包装确实隔离了阻塞工作，但等待侧也重新创建了全新的请求上下文。这意味着外部取消与关闭预算不会自然传播到等待路径，且工作线程归属位于运行时之外，即使调用方本身已处于异步上下文。
- 最小化定向重构:
  - 将 JSONL 扫描/打开/保存包装迁移至 `asupersync::runtime::spawn_blocking_io` 或小型的运行时拥有的阻塞辅助。
  - 为等待路径接受继承的 `&AgentCx` 或 `&Cx`，而非在内部构造新的请求上下文。
  - 对于 SQLite 辅助，将调用方上下文透传下去，而非在辅助内部新建。
- 精确模块与函数:
  - `src/session.rs`
    - `Session::resume_with_picker`
    - `Session::continue_recent_in_dir`
    - `Session::open_v2_with_diagnostics`
    - `Session::open_jsonl_with_diagnostics`
    - `Session::save_inner`
    - `scan_sessions_on_disk`
  - `src/session_sqlite.rs`
    - `load_session`
    - `load_session_meta`
    - `save_session`
    - `append_entries`
- 保持不变:
  - JSONL 原子化临时文件持久化行为
  - 会话索引协调启发式
  - SQLite 模式/布局选择
- 下游 Beads:
  - `bd-xdcrh.2.1`
  - `bd-xdcrh.4.1`
  - `bd-xdcrh.5`

## 集群 4：后台压缩是在线程拥有的运行时上运行的异步工作

- 建议: `refactor`
- 原因: 压缩是提供方/网络相关工作，而非单纯的阻塞 I/O。如今它在专用 OS 线程上启动，每次尝试都会构建全新的 current-thread 运行时。这保留了前台响应性，但也放弃了结构化任务归属与直接的取消继承。
- 最小化定向重构:
  - 保留两阶段工作线程状态机与配额逻辑。
  - 将实际的压缩 future 重新托管到现有的多线程运行时上，通过受控的任务句柄与显式的关闭/中止路径管理。
- 精确模块与函数:
  - `src/compaction_worker.rs`
    - `CompactionWorkerState::start`
    - `run_compaction_thread`
  - `src/agent.rs`
    - `AgentSession::maybe_compact`
    - `AgentSession::compact_synchronous`
    - `AgentSession::apply_compaction_result`
- 保持不变:
  - 冷却/尝试次数配额策略
  - “先应用已完成结果，再决定是否发起下一次压缩”的非阻塞模型
- 下游 Beads:
  - `bd-xdcrh.4.2`
  - `bd-xdcrh.3`
  - `bd-xdcrh.5`

## 集群 5：管道泵与子进程清理线程目前基本保持原样即可

- 建议: 暂定 `keep`，仅对基于实测的后续工作做小范围 `defer`
- 原因: 这些线程与阻塞式 OS 管道及短生命周期子进程清理绑定。 preemptively 将其替换为通用运行时阻塞通道未必更优，且其中若干路径已正确地将超时与显式的 kill/清理逻辑结合。
- 精确模块与函数:
  - `src/tools/mod.rs`
    - `BashTool::execute`
    - `GrepTool::execute`
    - `FindTool::execute`
    - `ProcessGuard::drop`
  - `src/extensions.rs`
    - 扩展 exec/tool 子进程泵路径
  - `src/extension_dispatcher.rs`
    - 扩展 exec/command 子进程泵路径
- 保持不变:
  - `src/tools/mod.rs::{EditTool::execute,WriteTool::execute,HashlineEditTool::execute}` 使用 `spawn_blocking_io` 进行原子化写入
  - 工具轮询循环中的 `AgentCx::for_current_or_request()` 计时器用法
  - 当前的 `TERM -> 宽限期 -> KILL` 与进程树清理语义
- 仅在出现证据时再推迟处理:
  - 并行子进程负载下的线程激增
  - 关闭挂起或孤儿清理回归
  - 运行时拥有的阻塞通道能明显解决的可度量争用
- 下游 Beads:
  - `bd-xdcrh.4.3`

## 集群 6：长循环/检查点工作应等待继承式上下文修复落地后再进行

- 建议: `defer`，随后以聚焦测试重新审视
- 原因: 若干循环已使用感知计时器驱动的 `now` 计算，但显式的检查点/取消纪律难以在周边辅助层仍在重建全新请求上下文的情况下得到清晰评估。
- 在集群 1-4 之后再回访的精确模块与函数:
  - `src/rpc.rs`
    - `run_prompt_with_retry` 重试退避循环
    - `maybe_auto_compact`
    - `rpc_emit_extension_ui_request`
  - `src/tools/mod.rs`
    - `BashTool::execute`
    - `GrepTool::execute`
    - `FindTool::execute`
  - `src/compaction_worker.rs`
    - `CompactionWorkerState::try_recv`
- 目前保持不变:
  - `src/tools/mod.rs` 与 `src/http/client.rs` 中感知计时器驱动的 `now` 查询
  - 通过 `ExtensionManager::effective_timeout` 进行的扩展管理器超时钳制
- 下游 Beads:
  - `bd-xdcrh.3`
  - `bd-xdcrh.3.1`
  - `bd-xdcrh.5`

## `bd-xdcrh.3.1` 检查点边界地图

本节将集群 6 转化为可直接实施的地图。规则很简单：仅在取消可在完整迭代之间发生、或紧邻下一次阻塞等待之前发生的位置添加显式检查点。不要在会以部分生效状态遗留会话、工具输出或 UI 投递状态的中间状态转换中设置检查点。

### 1. Shell/进程轮询循环

- `src/tools/mod.rs::run_bash_command`
  - 重要性: 这是一个长生命周期的轮询/排空循环，在子进程存活并持续输出时可能运行数分钟。
  - 安全检查点位置: 当前 `rx.try_recv()` 排空完成后、且超时/终止截止时间决策计算完毕后，紧邻开始下一次轮询迭代的 `sleep(now, tick)` 之前。
  - 安全检查点位置: 在退出后排空循环中，仅在 `TryRecvError::Empty` 分支、进入下一次 `sleep(now, tick)` 之前。
  - 非安全区域: `ingest_bash_chunk(...)` 内部，因为该函数会作为一个逻辑单元更新总字节/行数、截断状态以及可选的溢出到临时文件的状态。
  - 非安全区域: `terminate_process_group_tree(pid)` 与后续宽限期 kill 之间；在该处取消会使关闭状态变得含糊。
  - 插入后预期行为: 取消会及时停止后续轮询，但绝不会丢弃半次摄入的块，也不会在已启动的 terminate-then-kill 升级过程中中断。

- `src/rpc.rs::run_bash_rpc`
  - 重要性: 与 `run_bash_command` 形态相同，但用于带显式中止处理与有界内存/尾文件状态的 RPC shell 路径。
  - 安全检查点位置: 在主 `exit_code = loop { ... }` 体底部，当前 `rx.try_recv()` 排空与子进程状态检查之后，紧邻 `sleep(wall_now(), tick)` 之前。
  - 安全检查点位置: 在剩余输出排空循环中，仅在 `TryRecvError::Empty` 分支、进入下一次 `sleep(...)` 之前。
  - 非安全区域: `ingest_bash_rpc_chunk(...)` 内部，原因与工具路径相同。
  - 非安全区域: 在 `abort_rx.try_recv().is_ok()` 已触发并已提交 `guard.kill()` 之后。已开始的 kill 路径应执行到底。
  - 插入后预期行为: 已取消的 RPC shell 工作会迅速停止轮询，但尾文件溢出状态与进程树清理保持内部一致。

- `src/tools/mod.rs::GrepTool::execute`
  - 重要性: 主循环反复从 ripgrep 排空有界的 stdout/stderr 通道并等待进程退出；退出后的 join 循环在背压下也可能运行一段时间。
  - 安全检查点位置: 当前迭代的 `drain_rg_stdout(...)` / `drain_rg_stderr(...)` 完成后，紧邻 `Ok(None)` 分支上的 `sleep(now, tick)` 之前。
  - 安全检查点位置: 在 `while !stdout_thread.is_finished() || !stderr_thread.is_finished()` 循环中，当前排空轮次之后、 `sleep(wall_now(), Duration::from_millis(1))` 之前。
  - 非安全区域: `drain_rg_stdout(...)` 正在将 JSON 行消费到匹配累加器期间；在批次之间停止，而非在批次中间停止。
  - 非安全区域: 在代码判定已达到匹配上限并提交 `guard.kill()` 及通道排空之后。该终止序列应保持不可分割。
  - 插入后预期行为: 取消会停止等待更多 ripgrep 输出，但不会使匹配累加器或显式进程终止处于半应用状态。

- `src/tools/mod.rs::FindTool::execute`
  - 重要性: 基于 fd 的搜索循环与 grep/bashtool 轮询镜像，只是使用批量 stdout 读取器而非逐行 JSON 解析。
  - 安全检查点位置: 当前子进程状态检查之后、 `sleep(now, tick)` 之前。
  - 非安全区域: 除现有的 `try_wait` / 超时状态更新外，循环体内无额外非安全区域；这是较干净的候选者之一，因为每次迭代已具备单一等待边界。
  - 插入后预期行为: 取消会以有界延迟停止后续轮询，且不影响已缓冲的 stdout/stderr 收集。

### 2. 重试 / 流式 / UI 投递循环

- `src/rpc.rs::run_prompt_with_retry`
  - 重要性: 该重试循环可跨越多次模型调用与显式退避延迟。
  - 安全检查点位置: 下一次重试迭代顶部，在创建下一对 `AbortHandle` / `AbortSignal` 之前。
  - 安全检查点位置: 发出 `auto_retry_start` 之后、进入 `delay_ms` 的退避休眠之前。
  - 非安全区域: 当前尝试持有 `abort_handle_slot` 或会话互斥锁并正在执行 `run_text_with_abort(...)` / `run_with_content_with_abort(...)` 期间。
  - 非安全区域: 已完成的尝试与将槽位重置为 `None` 的 `abort_handle_slot` 清理之间。
  - 插入后预期行为: 取消会阻止下一次重试或退避等待开始，但绝不会遗留陈旧的 abort 句柄，或以内部不一致的状态中断正在进行的模型尝试。

- `src/agent.rs::stream_assistant_response`
  - 重要性: 这是提供方输出的核心流式事件循环，包含中止与错误合成。
  - 安全检查点位置: 紧邻等待下一个 `stream.next()` / abort-select 结果之前。
  - 安全检查点位置: 完整 `StreamEvent` 分支完成后、循环即将回到下一次迭代之前。
  - 非安全区域: 在任意单个 `StreamEvent` 分支内修改部分助手消息期间，尤其是在 `MessageStart` 发出与对应状态更新之间。
  - 非安全区域: 在中止处理分支中，已合成中止消息并开始发出 `MessageUpdate` 之后。
  - 插入后预期行为: 取消在提供方事件之间中止，而非在将单个事件应用到助手 transcript 的中途中止。

- `src/rpc.rs` 扩展 UI 请求桥接
  - 精确循环: `while let Ok(request) = extension_ui_rx.recv(&cx).await`
  - 重要性: 该队列串行化需要响应的扩展 UI 请求，并在互斥锁下执行 active/queued 转换。
  - 安全检查点位置: 紧邻下一次 `extension_ui_rx.recv(&cx).await` 之前。
  - 安全检查点位置: 在 `ui_state` 互斥锁释放后、为下一个请求调用 `rpc_emit_extension_ui_request(...)` 之前。
  - 非安全区域: 持有 `ui_state.lock(&cx)` 并修改 `active` / `queue` 期间；出队、溢出取消与下一个 active 提升各自是单一逻辑转换。
  - 插入后预期行为: 取消会停止接受新的 UI 请求或延迟下一次发出，但绝不会使 `active` 与 `queue` 在归属上出现分歧。

- `src/interactive/agent.rs::flush_ui_stream_batcher_with_backpressure`
  - 重要性: 该循环在刷新分离的 UI 增量批次时，可能花费时间等待通道容量。
  - 条件安全检查点位置: 已完成的 `sender.send(&cx, msg).await` 调用之间，但仅在明确接受“取消可能丢弃剩余分离尾部”的前提下。
  - 非安全区域: 若调用方仍期望完整刷新保证，则在 `pending` 已从批处理器取出之后。目前分离批次没有取消时重入队路径。
  - 当前建议: 暂不添加检查点。先明确关闭/取消应保留还是丢弃未发送的 UI 增量；安全边界取决于该策略选择。

### 3. 会话与持久化循环

- `src/session.rs::scan_sessions_on_disk`
  - 重要性: 这是主要的磁盘会话发现循环，可能扫描大量会话文件。
  - 工作线程迁移后的候选安全检查点位置: 已完成的 `for entry in dir_entries` 迭代之间，在完成单个文件的元数据/加载决策后、下一个文件开始前。
  - 非安全区域: 在复用已知条目期间，或在为单个文件执行 `load_session_meta(...)` 的中途；每个文件决策应保持不可分割。
  - 当前建议: 不要在当前的原生线程体中插入检查点。正确的第一步是在等待侧继承调用方上下文，和/或将扫描移至运行时管理的阻塞工作（`bd-xdcrh.4.1`），然后在文件之间添加协作式检查点。

- `src/session.rs::save_inner`
  - 重要性: 该函数包含最明显的持久化循环（全量重写期间的 `for entry in &entries`，以及增量追加序列化期间的 `for entry in new_entries`）。
  - 安全检查点位置: 在派生阻塞工作线程之前，以及完全接收工作线程结果并应用到内存计数器之后。
  - 非安全区域: 将头部 + 条目写入临时文件的全量重写循环。
  - 非安全区域: 增量追加序列化循环及随后的加锁 `file.write_all(&serialized_buf)` 追加路径。
  - 当前建议: 将这些写入阶段视为有意不可分割。在中间插入检查点对取消延迟的改善，远不及对原子性语义的削弱。

- `src/session.rs` 选择器 / 最近会话合并循环
  - 精确循环: `continue_recent_in_dir` 与 `resume_with_picker` 中的 `for entry in entries.into_iter().chain(scanned.into_iter())` 合并
  - 重要性: 这些循环可能触及大量已索引 + 已扫描条目，但在阻塞的索引/扫描工作完成后，它们是纯内存合并逻辑。
  - 安全检查点位置: 若这些循环经实测证明足够大，则在已完成的 `by_path` 合并迭代之间。
  - 当前建议: 推迟。更大的取消收益在阻塞工作线程等待路径上，而非这些短小的内存合并中。

### 4. 非候选或保持原样的表面

- `src/compaction_worker.rs::CompactionWorkerState::try_recv`
  - 非检查点候选。它是单一非阻塞轮询辅助，而非长生命周期循环。

- `src/rpc.rs` stdio 泵线程（`read_line` / stdout 写入循环）
  - 暂保持原样。这些是专用阻塞 I/O 线程；若将来迁移，自然的检查点边界应在处理完完整一行并进入下一次阻塞读取或背压重试之前，但那首先是运行时归属工作，而非检查点工作。

### `bd-xdcrh.3.2` 的推荐实施顺序

1. `src/tools/mod.rs::run_bash_command`
2. `src/rpc.rs::run_bash_rpc`
3. `src/tools/mod.rs::GrepTool::execute`
4. `src/rpc.rs::run_prompt_with_retry`
5. `src/agent.rs::stream_assistant_response`
6. `src/rpc.rs` 扩展 UI 请求桥接
7. 仅在工作线程/等待路径重构落地后，再回访 `scan_sessions_on_disk`
8. 保持 `save_inner` 与 `flush_ui_stream_batcher_with_backpressure` 不变，直至其不可分割性/丢弃策略问题得到明确回答

### `bd-xdcrh.3.2` 实施说明

当前实现已落地本图中最具置信度的四个位点：

- `src/tools/mod.rs::run_bash_command`
- `src/rpc.rs::run_bash_rpc`
- `src/rpc.rs::run_prompt_with_retry`
- `src/agent.rs::stream_assistant_response`

两个剩余候选被有意推迟而非遗忘：

- `src/tools/mod.rs::GrepTool::execute` 与 `src/tools/mod.rs::FindTool::execute`
  - 轮询边界在机制上是安全的，但这些工具 API 目前未暴露显式的已取消结果形态。在此添加检查点将迫使对“取消应返回部分搜索结果还是新的工具错误”做出独立的契约决策。

- `src/rpc.rs` 扩展 UI 请求桥接
  - `RpcUiBridgeState` 下的队列转换目前内部是一致的，但在互斥锁释放后、发出前的检查点，仅在取消同时明确 active 请求是被取消、重入队还是保持待处理时才是安全的。该归属策略应与桥接后续工作一并落地，而非作为本 bead 的隐式副作用。

## 高置信度保持面

这些是 Pi 已在以正确方式使用 `asupersync` 且应被视为基线模式、而非清理目标的位置：

- `src/http/client.rs`
  - `RequestBuilder::send`
  - 查询 `Cx::current()` 以获取计时器驱动时间的响应超时路径
- `src/extensions.rs`
  - `cx_with_deadline`
  - `ExtensionManager::{with_budget,set_budget,extension_cx,effective_timeout,dispatch_event*,execute_command,execute_shortcut}`
- `src/tools/mod.rs`
  - `EditTool::execute`
  - `WriteTool::execute`
  - `HashlineEditTool::execute`
  - 广泛使用 `asupersync::fs` 与 `spawn_blocking_io`
- `src/interactive.rs`
  - TUI 核心本身不是杠杆问题；可操作的上下文问题在 `src/interactive/ext_session.rs`

## 下游工作的测试义务

已有较强的示例:

- `src/extensions.rs` 预算/超时测试
- `src/http/client.rs` 确定性超时/流式测试
- `src/tools/mod.rs` 广泛的 `asupersync::test_utils::run_test` 覆盖

随代码落地值得填补的覆盖缺口:

1. 会话/扩展会话辅助调用应继承当前截止时间，而非静默创建无界请求作用域。
2. 会话 JSONL/SQLite 工作线程包装应在调用方预算在等待中途过期时证明其取消与关闭行为。
3. 压缩工作线程变更应证明后台工作遵守关闭预算且不会在所属运行时之外存活。
4. RPC 重试/自动压缩路径应证明截止时间在嵌套辅助调用间的传播。

## 推荐执行顺序

1. `bd-xdcrh.2.1`: 首先修复会话与扩展会话辅助的继承。
2. `bd-xdcrh.2.2`: 通过 RPC 控制/模型/思考辅助透传继承的 `AgentCx`。
3. `bd-xdcrh.2.3`: 通过智能体与交互式后台辅助透传继承的 `AgentCx`。
4. `bd-xdcrh.4.1`: 将会话 JSONL/SQLite 工作线程孤岛移至运行时拥有的阻塞执行之下。
5. `bd-xdcrh.4.2`: 将后台压缩归属迁移至运行时。
6. `bd-xdcrh.3` 与 `bd-xdcrh.3.1`: 在继承式上下文工作就位后，再回访检查点边界。
7. `bd-xdcrh.4.3`: 仅在实测证据表明当前线程模型确有问题时，再回访子进程管道泵。
8. `bd-xdcrh.5`: 随每个切片一并落地确定性测试，而非留到最后。
9. `bd-xdcrh.6` 与 `bd-xdcrh.6.1`: 仅在窄域修复后仍存在生命周期隐患时，再升级至更广的监管/AppSpec 工作。
10. `bd-xdcrh.6.2`: 推迟广域监管或 AppSpec 采纳

## `bd-xdcrh.6.2` 监管 / AppSpec 决策

- 建议: `defer` 广域监管或 AppSpec 采纳
- 为何现在这样决策是正确的:
  - Pi 已拥有可用的归属主干: `src/main.rs` 拥有多线程 `asupersync` 运行时，随后将运行时句柄透传至 `AgentSession`，而后者已为提供方状态、会话状态、扩展区域与后台压缩集中了按会话的归属。
  - 最高价值的工作线程孤岛修复已在 `bd-xdcrh.4.2` 中落地，因此后台压缩不再是 Pi 缺乏运行时归属的证据。
  - 剩余的生命周期边缘是模式特定的桥接边界，尤其是交互式与 RPC 切换点，外加显式的 `ExtensionRegion::shutdown()` 契约，而非缺少仓库级监管框架。
  - 已知的原生线程孤岛仍是有意为之。RPC stdio 循环与子进程管道泵与硬阻塞 I/O 及后代管道行为绑定，在这些位置专用线程仍比通用的 AppSpec 形态抽象更简单、更安全。
- 替代做法应继续:
  - 在异步工作跨越 UI 或阻塞边界处使关闭边缘显式化
  - 保留现有的 `main` -> `AgentSession` -> 模式桥接归属模型
  - 将专用阻塞线程视为仅在出现实测失败时才有罪，而非因审美上过时就定罪
- 非目标:
  - 不要仅为更直观地镜像 `asupersync` 概念就引入新的顶层监管树
  - 不要在没有具体待解失败模式的情况下，用广域 AppSpec 层包裹现有交互式或 RPC 流程
  - 不要借此 bead 重启已关闭的工作线程生命周期决策，例如压缩运行时归属
- 仅在出现新证据时再回访:
  - 桥接任务被证实存活于所属模式之外，且无法通过局部关闭或中止契约修复
  - 显式扩展关闭在不止一个窄域边界上变得难以管理
  - 原生线程孤岛出现实测的关闭挂起、孤儿工作或线程激增，且运行时拥有的结构能明显改善
