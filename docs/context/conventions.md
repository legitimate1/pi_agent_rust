# 命名规范与隐含假设

## 命名规范

| 类别        | 规范         | 示例                         |
| :---------- | :----------- | :--------------------------- |
| 内置工具名  | 小写         | `read` `write` `edit` `bash` |
| Provider 名 | 小写         | `anthropic` `openai`         |
| 扩展 ID     | `ext.*` 前缀 | `ext.tools_fs`               |
| Rust 源码   | `snake_case` | `extension_tools.rs`         |
| 测试函数    | `snake_case` | `test_tool_override`         |

## 隐含假设

- Rust 2024 nightly 编译（见 `rust-toolchain.toml`）
- 依赖 `asupersync`（结构化并发运行时）和 `rich_rust`（终端 UI）
- 扩展 JS/TS 在 QuickJS 沙箱中运行，无 Node/Bun 依赖
- Node.js 内置模块（`node:fs`、`node:path` 等）通过 QuickJS 垫片实现
- `@mariozechner/pi-coding-agent` 等 npm 包为虚拟模块
- **扩展自动发现根（`~/.pi/agent/extensions/`、`.pi/extensions/`）下的每个目录/文件是独立扩展，不是 bundle 入口** — sibling 扩展发现逻辑（`discover_sibling_extension_entries` / `discover_sibling_index_entries`）必须排除该根，否则多个独立扩展会被误判为同一包的多入口（#35）
- **Cargo 不自动清理旧编译产物** — 每次 `cargo build`/`cargo test` 生成新 hash 的文件（.exe/.pdb/.rlib），旧文件永久保留。Windows 上 .pdb 文件尤为庞大。需用 `cargo-sweep` 主动管理。

## 子进程管理约定

所有 spawn 子进程的内置工具**必须**使用 `ProcessGuard`（`src/tools/mod.rs`）管理子进程生命周期，不得直接使用裸 `std::process::Child`。

### 必须使用 ProcessGuard

```rust
// ❌ 错误：裸 Child，Drop 时不 kill，abort 时变孤儿进程
let mut child = cmd.spawn()?;
child.try_wait()...;

// ✅ 正确：ProcessGuard 封装，Drop 时自动 kill + wait 回收
let mut guard = ProcessGuard::new(child, ProcessCleanupMode::ProcessGroupTree);
guard.wait_with_cancellation(timeout_secs, abort.as_ref()).await?;
```

### 清理模式：统一 ProcessGroupTree

所有 spawn 子进程的工具（bash/pwsh/grep/find）统一使用 `ProcessGroupTree`：

| 工具 | 清理模式                                             | 说明                                             |
| :--- | :--------------------------------------------------- | :----------------------------------------------- |
| bash | `ProcessGroupTree` + `isolate_command_process_group` | shell 启动的后台进程一律被 `taskkill /F /T` 终止 |
| pwsh | `ProcessGroupTree` + `isolate_command_process_group` | PowerShell 命令的子进程树被完整清理              |
| grep | `ProcessGroupTree` + `isolate_command_process_group` | ripgrep 及其子进程被清理                         |
| find | `ProcessGroupTree` + `isolate_command_process_group` | fd 及其子进程被清理                              |

> `ChildOnly` 模式已废弃。新增 spawn 子进程的工具必须使用 `ProcessGroupTree` + `isolate_command_process_group`。

> **例外：verify 引擎**（`src/tools/verify.rs`）不使用 ProcessGuard — 它在 `spawn_blocking_io` 同步上下文中运行自己的轮询循环（`run_external_process`），50ms 轮询 + 10s wall-clock 超时 + abort 检查。超时/abort 时用 `terminate_process_tree`（Windows `taskkill /T /F` 杀整棵进程树，防 cmd 外壳泄漏 node 孤儿）而非 ProcessGuard 的 cleanup。新增 verify 子进程逻辑时沿用此模式，不要强行套 ProcessGuard（异步上下文不兼容）。子进程 spawn 必须显式 `stdin(Stdio::null())`（#34：继承宿主 JSONL 管道会让 cmd 包装的 shim 挂起）。

### 标准化 wait 方法

优先使用 `guard.wait_with_cancellation(timeout_secs, abort.as_ref())`，它内置：

- **Ambient cancellation** — 通过 `AgentCx::checkpoint()` 检测 abort
- **Abort 信号** — 通过 `AbortSignal::is_aborted()` 检测外部取消
- **超时 kill** — 超时后自动 kill 子进程
- **适应性 sleep** — 接近超时时缩短轮询间隔

```rust
// 推荐：一行涵盖 timeout + cancellation + abort
let exit_code = guard.wait_with_cancellation(timeout_secs, abort.as_ref()).await
    .ok()
    .flatten()
    .unwrap_or(-1);
```

### 需要规避

- ❌ 直接使用 `std::process::Child` 裸对象
- ❌ 手动实现 wait 循环时遗漏 cancellation 检查
- ❌ wait 循环中用 `std::thread::sleep` 阻塞异步运行时（用 `asupersync::time::sleep` 代替）
- ❌ `Tool::execute()` 实现中忽略 abort 参数（应至少传 `None` 给 trait 定义）

## 扩展 abort 约定

扩展工具（扩展注册的自定义工具）的 abort 传播路径不同于内置工具，采用**两阶段**机制：

1. `ExtensionToolWrapper::execute` 收到 abort → 经 `execute_tool_ref` → `JsRuntimeCommand::ExecuteTool.abort` 字段（mpsc channel）转发给 runtime worker
2. `execute_extension_tool` 将 `task_id` 传给 JS 侧 `__pi_execute_tool`，JS 侧为每次调用创建 `AbortController` 并作为第 4 参数 `signal` 传给扩展 `execute`（此前恒为 `undefined`）
3. `await_js_task` 循环检查 `abort.is_aborted()`：
   - **首次** → 调 JS `__pi_abort_task(task_id)` → 触发该调用的 `AbortController.abort()` → 扩展的 `signal.addEventListener('abort')` 事件生效（优雅路径）
   - **下一轮仍 pending** → `runtime.request_interrupt()` → QuickJS interrupt handler 硬中断兜底
4. `InterruptBudget.external_trigger` 确保外部 abort 优先级高于 interrupt budget；`with_ctx` 每次入口自动 `reset()`，外部 trigger 一次性有效

```rust
// await_js_task 中的两阶段 abort 检查模式
let mut js_abort_notified = false;
loop {
    if let Some(signal) = abort {
        if signal.is_aborted() {
            if js_abort_notified {
                runtime.request_interrupt();
                return Err(Error::extension("Extension tool aborted by user request"));
            }
            js_abort_notified = true;
            // 通知 JS 侧 AbortController（best-effort）
            let _ = runtime.with_ctx(|ctx| {
                let abort_fn: rquickjs::Function<'_> = ctx.globals().get("__pi_abort_task")?;
                let _: rquickjs::Value<'_> = abort_fn.call((js_task,))?;
                Ok::<(), rquickjs::Error>(())
            }).await;
        }
    }
    pump_js_runtime_once(runtime, host).await?;
    // ... 检查 task 状态
}
```

扩展工具 execute 的 5 参签名：`(toolCallId, input, onUpdate, signal, ctx)`——第 4 位 `signal` 是真实的 AbortController signal（支持 `aborted` / `addEventListener('abort')` / `throwIfAborted()`）。

## 反模式

| ❌ 不要                                                         | ✅ 应该                                                                                | 原因                                                                                                                                                                                                                        |
| :-------------------------------------------------------------- | :------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 用脚本批量改代码                                                | 手动逐处修改                                                                           | 正则替换容易引入 Bug                                                                                                                                                                                                        |
| 创建 `main_v2.rs` 等变体                                        | 原地修改原文件                                                                         | 文件膨胀导致混乱                                                                                                                                                                                                            |
| 使用不安全的 `unsafe` 代码                                      | 纯 safe Rust                                                                           | 项目 `forbid(unsafe_code)`                                                                                                                                                                                                  |
| 放任 `target/` 无限膨胀                                         | 定期 `cargo sweep --file` 清理旧产物                                                   | Cargo 永不删除旧文件，debug .pdb 和增量缓存可累积到数百 GB                                                                                                                                                                  |
| 用裸 `std::process::Child`                                      | 用 `ProcessGuard` 封装                                                                 | Drop 时不 kill 子进程，abort 后变孤儿                                                                                                                                                                                       |
| 扩展 sibling 发现逻辑忽略 auto-discovery 根（`extensions/`）    | 发现函数先检查 parent/cluster_root 目录名是否为 `extensions`，是则跳过 sibling 发现    | 根下每个目录是独立扩展，不是 bundle 多入口；漏查会把全部独立扩展误判为同一包，多文件扩展的相对 import 因 root 检查失败报 `Unsupported module specifier`（#35，`discover_sibling_index_entries` 曾漏掉 cluster_root 层检查） |
| 对带 `extension.json` 的扩展做 sibling 启发式发现               | 有 manifest 时只加载声明 entrypoint（`discover_related_extension_entries` guard，D28） | 目录内模块文件（含 `pi.registerCommand`）和子目录门面（`fusion/index.ts`）会被误判为额外入口，逐个加载超 hostcall budget；子目录被注册为 root 导致扩展内合法相对 import 误判逃逸（#35 follow-up）                           |
| 包收集逻辑在发现资源子目录后忽略包根扩展入口                    | has_any_dir 时仍复用 `resolve_extension_entries(package_root)` 收集根扩展入口          | `-e <目录>` 静默成功是假象：`prompts/` 等资源目录存在时根 `index.ts` 扩展根本没加载（#35 follow-up，`collect_package_resources`）                                                                                           |
| spawn 子进程不显式设置 stdin（依赖默认继承）                    | 显式 `.stdin(Stdio::null())` 或 `Stdio::piped()`                                       | 默认继承父进程 stdin；GUI 宿主（Obsidian/Electron）下父进程 stdin 是活跃 JSONL 管道，cmd 包装的 shim（`prettier.cmd`/`npx.cmd`）等待该管道挂起，verify 10s 超时（#34）。全库 20+ 处 spawn 均显式 null，唯 verify 遗漏       |
| 用 `Path::is_absolute()` 判断"绝对形式路径"                     | 同时检查 `components()` 是否含 `RootDir`/`Prefix`                                      | **Windows 上 `/tmp/x` 这类 root-relative 路径（无盘符）`is_absolute()` 返回 false**，会漏判导致路径逃逸校验失效（#33 及同类问题的根因，已在 conformance/mod.rs、logging.rs 修复）                                           |
| 测试 fixture 用 `format!("cwd:\"{}\"", path.display())` 拼 JSON | 用 `serde_json::to_string(&path.to_string_lossy())`                                    | Windows 路径反斜杠未转义会生成非法 JSON（migrations 测试根因）                                                                                                                                                              |
| 用 `PathBuf::push` 拼接 `C:` 盘符与相对段                       | 显式 `format!("{drive}\\{path}")`                                                      | Windows 上 `PathBuf::from("C:").push("Users")` 得到 `C:Users`（丢分隔符，auth home_dir 根因）                                                                                                                               |
| `cargo test <模块名>` 定位单元测试                              | `cargo test --lib <模块名>`                                                            | 不加 `--lib` 会编译所有集成测试 target（e2e 等），大幅延长等待时间                                                                                                                                                          |
