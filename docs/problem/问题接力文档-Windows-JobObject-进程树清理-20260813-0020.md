# 问题接力文档 - Windows 上 bash/pwsh 超时无法杀死后台孤儿进程（Job Object 修复）

## 问题
- 最终目标：给 bash/pwsh 工具加 **Windows Job Object**（把整个进程树塞进 job，超时/取消时关闭 job 全杀），使 Windows 上超时后后台孤儿进程不再存活、不再写文件。
- 现象：Windows 上 bash 工具 timeout 后，后台子进程存活并继续执行（实测写出 `leaked.txt`）；`dropin_tool_io_differential.rs` 的 `bash_timeout_kills_descendant` 场景断言失败。
- 复现步骤：
  1. `cargo test --test dropin_tool_io_differential`（Windows）
  2. 场景 `bash_timeout_kills_descendant`：`sh -c 'sleep 2; printf leaked > leaked.txt' & sleep 5`，`timeout: 1` 秒
  3. 等待 3 秒 → `root/leaked.txt` 出现（预期：不存在）→ 断言失败 `timed-out bash process group leaked a descendant writer`
- 环境：Windows（`C:\Users\m\...`，Win10/11）+ Git for Windows bash（`resolve_bash_shell` 命中 `Program Files/Git/bin/bash.exe`）+ Rust nightly + Node v24.16.0；测试用 `futures::executor` 直接驱动 `BashTool::execute`。

## 已尝试路径
| 尝试路径 | 预期 | 实际结果 | 为何放弃 |
|----------|------|----------|---------|
| `kill_process_group_tree`（src/tools/mod.rs:3535） | 超时杀死整个进程树 | Windows 上 `isolate_command_process_group`（mod.rs:3729）为 `#[cfg(not(unix))]` 空操作；`sysinfo` 树遍历丢失 `sh` 退出后被 reparent 的后台子进程；无进程组 `kill -- -PGID` 兜底（mod.rs:3559 仅 unix）→ `sleep 2` 存活写出文件 | 平台语义限制，Windows 无 POSIX 进程组 |
| 探针测试直接调 `BashTool::execute`（临时 tests/probe_bash_timeout.rs） | 验证泄漏是否真实 | 确认泄漏真实：超时报 `Command timed out after 1 seconds`（清理声明 `process_group_tree_terminated`），3 秒后 `leaked.txt` 出现 | 探针完成使命后删除，问题仍存在 |
| `dropin_tool_io_differential.rs` 该场景 `#[cfg(windows)]` skip | 全量测试转绿 | ✅ 测试通过 | 治标不治本：bash/pwsh 工具运行时仍有孤儿进程写文件风险，仅掩盖症状 |

## 当前推测
- 最可能根因：Windows 没有 POSIX 进程组，现有按进程树/进程组清理机制无法覆盖「shell 退出后 reparent 的后台孙进程」。正确修法是用 **Windows Job Object**：每个 bash/pwsh 子进程创建独立 Job（`CreateJobObjectW` + `AssignProcessToJobObject`），设 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，超时/取消时 `CloseHandle(job)` 全杀；配合 `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` 处理嵌套。Rust 可用 windows-sys / 手工 FFI，或用 `job` crate（如 `job::Object`），封装进 `isolate_command_process_group` / `terminate_process_group_tree` 的 Windows 分支。
- 已否决的根因：
  - 「没 bash 环境」— 否。Git bash 存在，其他 bash 场景全过，仅后台子进程场景失败。
  - 「测试写错」— 否。Linux 上该测试逻辑正确（进程组 kill 兜底有效），是 Windows 平台语义缺失。
  - 「修改测试 = 解决方案」— 否。skip 只让测试变绿，运行时风险（孤儿进程继续写文件/占资源）仍在。

## 关键资源
- `src/tools/bash.rs`：`run_bash_command`（142-381 行），超时路径 `terminate_process_group_tree(pid)`（279 行），`isolate_command_process_group(&mut cmd)`（185 行）
- `src/tools/mod.rs`：`kill_process_group_tree` / `kill_process_tree_with`（3535-3600 行，unix-only 进程组 kill 在 3559），`isolate_command_process_group`（3729-3744 行，Windows 空操作）
- `src/tools/pwsh.rs`：PwshTool，同 BashTool 的进程清理路径（需一并验证/修复）
- `tests/dropin_tool_io_differential.rs`：`bash_timeout_kills_descendant` 场景（约 138-153 行，已加 Windows skip，修复后可移除 skip）
- 实测证据：探针测试输出 `Command timed out after 1 seconds` + `leaked.txt` 出现（详见会话记录 2026-08-12）
- 参考：Windows `Job Objects` 文档（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`），crates.io `job` crate（`job::Object` + `assign_current_process`/`add_process`），或 `windows-sys` 直接 FFI
