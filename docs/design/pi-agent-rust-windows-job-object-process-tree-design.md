# pi_agent_rust — Windows Job Object 进程树生命周期

## 目标与背景

Windows 上的 `bash`/`pwsh` 工具目前使用 `ProcessGuard` 和基于 `sysinfo` 父子关系的进程树清理。Windows 没有 Unix 的 POSIX process group；当 shell 启动后台子进程后退出或被终止，后台进程可能被重新挂载到其他父进程，后续树遍历无法找到它。

已观察到的结果是：`bash` 超时后，后台孙进程仍然存活并继续向工作区写文件。现有回归场景因此在 Windows 上被跳过，而不是验证通过。

本设计将 `bash`/`pwsh` 定义为**工具调用作用域的进程执行器**：一次调用创建的根进程及其正常 Win32 后代都属于该调用；调用完成、超时、取消、abort 或宿主销毁时，不保留这些后台后代。

## 成功标准

1. Windows 上内置 bash timeout 后，shell 创建的后台后代不能继续写入测试工作区。
2. Windows 上 pwsh timeout/abort 后，`Start-Process` 等正常 Win32 创建的后代被终止。
3. Windows 上 RPC `run_bash_rpc` 使用与内置 bash 相同的 Job 生命周期，不保留后台后代。
4. Job 归属在子进程开始执行前建立，避免 `CreateProcess` 后再 `AssignProcessToJobObject` 的启动窗口。
5. 正常短命令的 stdout、stderr、退出码、截断和增量输出行为保持不变。
6. 取消、超时和 Drop 仍然能够回收根 `Child`，输出读取线程不会因后代继承 pipe handle 而永久等待。
7. Job 创建/附加失败时 fail closed：不返回“已受管”的假成功，也不留下已经启动的进程。
8. Unix 和非 Windows 平台继续使用现有 process group/tree 逻辑，不引入平台条件之外的行为变化。

## 范围

### In scope

- Windows 内置 `bash`、`pwsh` 和 RPC `run_bash_rpc` 的受管创建路径。
- Windows Job Object 的创建时 attachment。
- `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 生命周期语义。
- shell 专用 `WindowsProcessGuard`，以及与现有 reader/polling 循环的适配。
- bash/pwsh/RPC bash 的 Windows 超时、取消、Drop 和后台后代回归测试。
- `Cargo.toml` / `Cargo.lock` 的 Windows-only `windows-spawn` 依赖。
- 必要的 shell spawn helper 与错误回滚接口。

### Out of scope

- 将 Job Object 扩展到 verify、grep、find、扩展进程或其他 RPC 子进程。
- 把现有通用 `ProcessGuard<std::process::Child>` 改造成跨两种 Child 类型的全局枚举或泛型抽象。
- 为 bash/pwsh 增加 daemon、后台任务注册表或 detach API。
- 进程沙箱、文件系统权限隔离、网络隔离或资源配额。
- 阻止通过 Windows 服务、计划任务、WMI、独立 broker 或显式 Job breakaway 创建完全不属于本 Job 的进程。
- 继续支持 Windows 7 及更早版本的专门兼容路径。项目目标以现代 Windows/MSVC 环境为准；若运行时不支持创建时 Job attachment，必须显式报错，不能静默宣称树清理已生效。
- 手写本项目内的 Windows Job Object unsafe FFI；系统调用由 `windows-spawn` 隔离。

## 生命周期契约

### 工具调用作用域

bash/pwsh 的每次调用拥有一个独立 Job。Job owner 的生命周期至少覆盖：

1. 根进程创建与启动；
2. 根进程等待；
3. stdout/stderr pump 读取和最终 drain；
4. `ProcessGuard` 完成 wait 或终止。

Job 不得因根 shell 先退出而提前丢失。根进程退出后仍在 Job 中的后代继续属于本次调用，直到 drain/收尾完成，随后 Job 被关闭并终止残余后代。

### 正常返回

正常返回也关闭 Job。后台命令不能通过 shell 的 `&`、PowerShell `Start-Process` 等方式隐式转化为宿主外的长期服务。

未来若需要长期服务，应提供显式后台任务生命周期 API，而不是改变内置 bash/pwsh 的调用作用域语义。

### 超时、取消和 abort

超时、ambient cancellation 和显式 abort 均进入现有 `ProcessGuard` 终止路径。Windows 上终止 Job 是树清理的主操作；根 `Child::kill` 作为补充和回收保障。终止一旦开始，不允许被后续取消检查打断。

### Drop/crash 边界

正常 Rust Drop 会释放 Job owner，`KILL_ON_JOB_CLOSE` 终止仍在 Job 中的后代。宿主进程被操作系统强制终止时，Windows 会关闭宿主持有的 Job handle，Job 的 kill-on-close 机制负责终止成员进程。

这不是完整的操作系统沙箱：已经显式脱离 Job，或通过服务/计划任务/WMI 等系统组件创建的独立进程不在保证范围内。

## 技术方案

### Job 配置

每次 Windows bash/pwsh spawn 创建 unnamed Job，并设置：

- `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
- 不设置 `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`
- 不设置 `JOB_OBJECT_LIMIT_BREAKAWAY_OK`

正常 Win32 子进程默认继承 Job，因此 shell 后代即使发生 parent reparent，也仍由 Job 记录和终止。

`SILENT_BREAKAWAY_OK` 不采用。它会允许 Job 成员创建不属于该 Job 的子进程，与“整个工具调用作用域由 Job 托管”的目标相反。

### 创建时 attachment

优先使用支持 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 的 Windows 创建路径，使进程在初始线程运行前进入 Job。这样可以消除：

```text
CreateProcess -> child starts -> child creates grandchild -> AssignProcessToJobObject
```

之间的竞态。

仓库继续在 Unix 和非 Windows 平台使用 `std::process::Command`。Windows 的 `bash`、`pwsh` 和 RPC bash 通过 `windows-spawn` 的 Windows-only API 创建：

```rust
let mut command = windows_spawn::Command::new(program);
command
    .args(args)
    .current_dir(cwd)
    .stdin(windows_spawn::Stdio::null())
    .stdout(windows_spawn::Stdio::piped())
    .stderr(windows_spawn::Stdio::piped());
let child = command.spawn_with(
    windows_spawn::SpawnOptions::new()
        .drop_policy(windows_spawn::DropPolicy::KillTree),
)?;
```

`DropPolicy::KillTree` 由 crate 内部创建 unnamed Job、设置 `KILL_ON_JOB_CLOSE`，并把 Job 句柄作为 returned Child 的 owner。不能把一个临时外部 `Job` 传给 `SpawnOptions::job` 后立即 drop；`SpawnOptions::job` 只借用 Job，且 kill-on-close 需要 owner 持续到 I/O drain 完成。

### WindowsProcessGuard 边界

`windows-spawn::Child` 不是 `std::process::Child` 的别名。它提供 `try_wait`、`wait`、`kill` 和实现 `Read` 的 stdout/stderr，但字段、句柄和 Drop 语义不同。因此不修改现有通用 `ProcessGuard` 的 `Option<std::process::Child>` 字段，也不把所有调用方改造成枚举。

新增 Windows-only `WindowsProcessGuard`（初步位于 `src/tools/windows_process.rs`），仅由 bash、pwsh 和 RPC bash 使用。它持有 `windows_spawn::Child`，并提供当前 shell 路径需要的最小接口：

- `try_wait_child() -> io::Result<Option<ExitStatus>>`；
- `kill()`：先请求根进程终止，再 drop Child 关闭其私有 Job，最后异步或同步回收根进程；
- `wait()`：等待根进程并保留 Child 到调用方完成 pipe drain；
- `id()`：返回创建时捕获的 PID，供诊断使用。

`WindowsProcessGuard` 的 Child 必须一直活到 bash/pwsh/RPC bash 完成 stdout/stderr drain。正常返回时，即使根 shell 已退出，也必须 drop guard/Child，使 Job 终止残余后代。终止路径不能只调用 `windows_spawn::Child::kill()` 后把 Child 丢失到后台，因为 Job owner 的释放和 reader EOF 必须由同一个生命周期路径控制。

Windows spawn adapter 返回的 stdout/stderr 类型统一按 `Read + Send + 'static` 交给现有 pump thread；不使用 `windows-spawn::Child::wait_with_output()`，以保留增量输出、bounded channel、超时/abort 轮询、截断和 spill 文件行为。

### ProcessGuard 边界

现有 `ProcessGuard` 继续服务于 std 子进程调用方（grep/find、扩展、RPC 其他路径等），其 Unix process group/tree 和 Windows 旧 fallback 语义不在本次全局改造范围内。Windows Job owner 不注入现有 `ProcessGuard`，避免改变未纳入测试的调用方。

`ProcessGuard::new` 保持现有不可失败构造器。Windows shell 的 Job 初始化和创建失败通过 `WindowsProcessGuard::spawn` 返回 `io::Result`，在返回 guard 前完成所有资源准备和失败回滚。

### 输出与退出状态

不改变 bash 的 bounded channel/backpressure，也不改变 pwsh 的 stdout/stderr pump。Job handle 必须在最后一次 pipe drain 之前保持有效；否则孙进程仍可能持有写端，导致读线程 EOF 时序发生变化。

根进程退出和 Job 终止后的 exit code 仍按现有工具契约转换。超时文本、cancellation details、truncation details 保持现有 schema，不新增破坏性字段。

## 错误与降级策略

| 场景                          | 处理                                                                                         |
| :---------------------------- | :------------------------------------------------------------------------------------------- |
| Job 创建失败                  | `windows-spawn` 返回工具 spawn 错误；命令不会以不受管方式启动                                |
| 创建时 Job attachment 失败    | `windows-spawn` 的 spawn transaction 回滚已创建进程和临时句柄，返回工具错误                  |
| Windows spawn 参数验证失败    | 返回原始 `io::Error`，不尝试改用 `std::process::Command`                                     |
| 根进程终止请求失败            | 继续 drop Child 触发 kill-on-close，并记录首个可诊断错误                                     |
| 根进程已退出但 Job 仍有后代   | 保持 `WindowsProcessGuard` 到 drain 完成，再 drop Child 终止残余后代                         |
| pipe reader 仍未结束          | 先确保 Child/Job 已关闭，再按现有 drain deadline 收尾；不能无限 join                         |
| 宿主已处于外部 Job            | 允许 Windows nested Job 语义；若系统拒绝 requested Job，fail closed，不设置 silent breakaway |
| 子进程主动创建 breakaway/服务 | 不声称覆盖；测试和文档明确边界                                                               |

不提供静默降级到旧 `sysinfo` 树遍历的路径。否则调用方无法区分“Job 保护生效”和“仅做 best-effort 父子扫描”。

## 模块影响

```text
bash.rs / pwsh.rs / rpc.rs::run_bash_rpc
        |
        v
WindowsProcessGuard + shell spawn adapter (tools/windows_process.rs)
        |
        v
windows-spawn::Child
        |
        v
PROC_THREAD_ATTRIBUTE_JOB_LIST
        |
        v
Windows Job Object (KILL_ON_JOB_CLOSE)
```

初步文件边界：

- `src/tools/windows_process.rs`：Windows-only shell spawn adapter、`WindowsProcessGuard`、`windows-spawn::Child` 到现有 `Read`/`ExitStatus` 接口的适配。
- `src/tools/mod.rs`：声明并 re-export Windows-only helper；保持现有通用 `ProcessGuard` 和 Unix process group/tree 实现不变。
- `src/tools/bash.rs`：按平台选择 managed shell spawn，保持现有输出循环、超时和 cancellation 语义。
- `src/tools/pwsh.rs`：按平台选择 managed pwsh spawn，保持现有 stdout/stderr pump 和 wait 语义。
- `src/rpc.rs`：`run_bash_rpc` 按平台选择同一 managed bash spawn，不能继续独立裸 spawn。
- `tests/dropin_tool_io_differential.rs`：移除 Windows bash descendant skip，恢复 bash 后代清理断言，新增 pwsh 作用域场景。
- `tests/...`：新增或扩展 RPC bash 的 Windows 后代清理覆盖，具体文件按现有 RPC 测试布局确定。
- `Cargo.toml`：增加精确版本的 Windows-only `windows-spawn = "=0.1.0"`。
- `Cargo.lock`：记录 `windows-spawn` 及其 Windows-only `windows-sys` 依赖。
- `docs/context/conventions.md`：更新 Windows shell cleanup 语义、`WindowsProcessGuard` 使用规则和 `SILENT_BREAKAWAY_OK` 禁止项。

不修改 `src/tools/grep.rs`、`src/tools/find.rs`、`src/tools/verify.rs` 的 spawn 类型；它们继续走现有通用 `ProcessGuard`/专用清理路径。

## 测试策略

### 回归测试

恢复并强化现有 `bash_timeout_kills_descendant`：

```text
sh -c 'sleep 2; printf leaked > leaked.txt' & sleep 5
```

timeout 1 秒后等待足够时间，`leaked.txt` 必须不存在。

新增 Windows pwsh 场景，使用 `Start-Process` 启动延迟写入器；timeout 或 abort 后，写入文件必须不存在。

新增 Windows RPC bash 场景，使用 RPC 的 abort/关闭路径启动延迟 writer；调用结束后 writer 不能继续写入。

### 生命周期测试

- 短命令正常输出仍成功。
- 根 shell 先退出但 descendant 持有 stdout/stderr 写端时，终止后 drain 能结束。
- 两个并发调用各自使用独立 Job，互不终止。
- managed spawn 初始化失败时，测试确认没有残留 writer/process。
- 正常调用返回后，后台后代不能继续写文件，验证“调用作用域”语义。
- `WindowsProcessGuard::try_wait_child`、`kill`、`wait` 分别覆盖运行中、已退出和重复收尾状态。
- Windows shell command 的 cwd、stdin=null、stdout/stderr capture、UTF-8 输出和带空格路径保持现有行为。

### 平台门禁

Windows 专属行为使用 `#[cfg(windows)]`；Unix 测试继续验证 process group 语义。Windows 测试不依赖固定安装路径以外的 bash/pwsh 探测，缺少对应 shell 时按仓库已有约定跳过环境依赖场景。Cargo 编译验证至少覆盖 Windows MSVC target，因为非 Windows target 不会编译 `windows-spawn` 的公开 API。

## 注意事项

1. Job attachment 必须发生在初始线程运行前，除非验证后明确记录为仅临时探针方案。
2. 不把 `taskkill /T /F` 当作 Job 的等价实现；它可保留为失败回滚或兼容诊断手段，但不是正式归属机制。
3. 不要在 `isolate_command_process_group` 的 Windows 空实现中塞入隐式全局 Job；Job 必须是每次调用独立 owner，不能跨调用共享。
4. 不修改默认 bash/pwsh timeout 数值和输出 schema。
5. Job owner 和 pipe reader 的 Drop 顺序必须通过测试证明，不能只依赖字段声明顺序的直觉。
6. 不能因为实现了 Job Object 就把 Pi 描述为进程沙箱或严格 drop-in 替代品。

## 待讨论

- `windows-spawn 0.1.0` 的 Cargo metadata 已验证：MIT/Apache-2.0、Rust 1.75、仅依赖 `windows-sys 0.61`；正式实现仍需在 Windows MSVC target 做编译和行为验证。
- `windows-spawn::Child` 与 `std::process::Child` API 不同，已决定采用独立 `WindowsProcessGuard`，不把现有 `ProcessGuard` 改造成跨两种 Child 的大型枚举。
- RPC bash 已纳入本阶段；verify/grep/find/扩展进程的 Job 迁移保留为后续独立决策。
- 如果实测 `windows-spawn` 的 Windows 版本契约与项目发布支持矩阵冲突，需在实现前复审最低 Windows 版本，而不是静默降级到旧树遍历。

## 外部依据

- Microsoft Learn: [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- Microsoft Learn: [AssignProcessToJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject)
- Raymond Chen: [A more direct and mistake-free way of creating a process in a job object](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812)
- `windows-spawn` API: [docs.rs](https://docs.rs/windows-spawn/latest/windows_spawn/)
