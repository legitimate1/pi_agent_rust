# pi_agent_rust Windows Job Object Implementation

## 1. 实现边界（Implementation Contract）

Source: [`docs/design/pi-agent-rust-windows-job-object-process-tree-design.md`](pi-agent-rust-windows-job-object-process-tree-design.md)

Goal:

- 将 Windows 内置 bash、pwsh 和 RPC bash 接入 `windows-spawn = "=0.1.0"` 的 `DropPolicy::KillTree` 创建路径，保持现有输出、轮询、取消和截断契约，并保证工具调用结束时不遗留受管进程后代。

In scope:

- 增加 Windows-only `WindowsProcessGuard` 和共享 shell managed-spawn helper。
- 让 bash/pwsh/RPC bash 在 Windows 使用 `windows_spawn::Command::spawn_with(SpawnOptions::new().drop_policy(DropPolicy::KillTree))`。
- 将 `windows_spawn::Child` 的公开 stdout/stderr reader 接入现有 pump thread；退出状态直接使用标准库 `ExitStatus`。
- 调整三个调用方的 `try_wait`、kill、wait、drain 顺序，覆盖根进程先退出、Job 中仍有后代、重复 kill/wait 与失败收尾。
- 恢复 Windows bash descendant 回归，并补充 Windows pwsh、RPC bash、正常 I/O、正常返回清理、并发 Job 独立和 reader EOF 场景。
- 更新 Cargo manifest/lock、子进程约定文档和相关测试。

Out of scope:

- 不迁移 grep、find、verify、扩展进程或其他 RPC 子进程。
- 不修改通用 `ProcessGuard<Option<std::process::Child>>` 的类型结构。
- 不在主仓库手写 Windows unsafe FFI，不设置 breakaway limit，不使用临时外部 Job + spawn 后 attachment，不自动 release build/deploy。

Assumptions:

- `windows-spawn 0.1.0` 的 Windows 10 1809+运行时契约由依赖负责；Windows 创建/attachment 失败直接返回错误，不静默退回旧树遍历。
- `windows_spawn::Child` 的 `DropPolicy::KillTree` 私有 Job owner 必须在输出 reader 收尾前由 guard 持有；终止路径通过消费 Child 触发 Job close，reader 继续独立 drain 到现有 deadline。
- `windows_spawn::Child::{try_wait,wait}` 返回 `std::process::ExitStatus`，`Child::kill` 仅作为根进程终止请求，Child Drop 才是整棵 Job 清理动作。

Design delta:

- 相对已确认设计无方向变化。根据实际 API，guard 会保存 PID 和可选 exit status；pipes 通过 guard 的明确 `take_stdout`/`take_stderr` 方法取出。Windows 终止会立即消费 Child 关闭 private Job，以避免 descendant 在现有 timeout grace 期间继续写文件；超时文本和结果 schema不变。

---

## 2. 文件变更清单（Change Manifest）

| ID  | 路径                                   | 操作 | 用途                              | 主要改动                                                                                                                                                                      |
| :-: | :------------------------------------- | :--- | :-------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `Cargo.toml`                           | 修改 | 添加平台限定依赖                  | 增加 `[target.'cfg(windows)'.dependencies] windows-spawn = "=0.1.0"`。                                                                                                        |
| C2  | `Cargo.lock`                           | 修改 | 锁定依赖解析结果                  | 由 Cargo 生成 `windows-spawn 0.1.0` 及其 Windows-only 依赖记录。                                                                                                              |
| C3  | `src/tools/windows_process.rs`         | 新建 | Windows shell 进程生命周期边界    | 定义 `WindowsProcessGuard`；提供共享 managed spawn、`take_stdout`/`take_stderr`、`try_wait_child`、`id`、`kill`、`wait` 和 Drop/重复收尾语义。                                |
| C4  | `src/tools/mod.rs`                     | 修改 | 注册并导出平台 helper             | 声明 Windows-only 模块并 re-export；保持通用 `ProcessGuard` 及其他调用方不变。                                                                                                |
| C5  | `src/tools/bash.rs`                    | 修改 | 接入 Windows managed bash         | Windows 分支通过共享 helper 创建；移除 Windows 对通用 Child 字段和 process-group fallback 的依赖；保留增量 bounded channel、timeout 文本、spill/truncation 和非 Windows路径。 |
| C6  | `src/tools/pwsh.rs`                    | 修改 | 接入 Windows managed pwsh         | Windows 分支通过共享 helper 创建；保持 stdout/stderr pump、stderr 过滤、cwd、UTF-8、退出码和截断行为；终止后先让 Job close，再按现有 deadline drain。                         |
| C7  | `src/rpc.rs`                           | 修改 | 接入 RPC bash managed spawn       | `run_bash_rpc` 使用与 bash 相同 helper；保持 abort/ambient cancellation、bounded output、spill/truncation、RPC 错误细节和非 Windows路径。                                     |
| C8  | `tests/dropin_tool_io_differential.rs` | 修改 | 恢复 bash Windows descendant 回归 | 移除 Windows skip；在 Windows 与 Unix 都断言 timeout 后延迟 writer 不能写 `leaked.txt`，平台差异只保留必要 shell 可用性门控。                                                 |
| C9  | `src/tools/tests.rs`                   | 修改 | shell 和 guard 回归               | 补充/恢复 Windows bash/pwsh 正常 I/O、cwd、UTF-8、退出码、timeout/abort descendant、正常返回清理、继承 pipe EOF、并发 Job 独立测试；仅在可用 shell 环境执行。                 |
| C10 | `src/rpc.rs`（现有测试模块）           | 修改 | RPC bash Windows 生命周期回归     | 扩展现有 RPC bash 测试，覆盖 abort/关闭后 descendant 文件写入、正常输出和 reader drain；非 Windows process-group 回归继续保留。                                               |
| C11 | `docs/context/conventions.md`          | 修改 | 更新子进程管理约定                | 说明 bash/pwsh/RPC bash 在 Windows 使用 `WindowsProcessGuard`/Job Object，禁止 silent/breakaway flags，并明确 Job/reader 生命周期及外部 breakaway边界。                       |

不删除文件；不创建代码变体文件。

---

## 3. 依赖关系（Dependency Plan）

Dependencies:

- C1：无。
- C2：C1；只由 Cargo 更新。
- C3：C1；先以 crate 实际公开 API 实现 guard 和 helper。
- C4：C3；注册模块和导出 helper。
- C5：C3、C4；bash 需同时改平台 spawn、pipe take、PID/终止分支。
- C6：C3、C4；pwsh 需同时改平台 spawn 和 wait/terminate 生命周期。
- C7：C3、C4；RPC 需同时改平台 spawn 和 abort/drain 生命周期。
- C8：C5；恢复现有 differential 场景。
- C9：C3、C5、C6；补充 shell/guard 行为回归。
- C10：C3、C7；补充 RPC 生命周期回归。
- C11：C3、C5、C6、C7；实现完成后同步记录使用边界。

并行分组建议:

- Phase 1：C1、C3（依赖/API基础，可先独立准备；Cargo lock 在 C1 后由 Cargo 更新）。
- Phase 2：C4；随后 C5、C6、C7 可按文件分开实现，但三者共享 C3 接口，整合时由主 Agent统一检查。
- Phase 3：C8、C9、C10（实现接入后补测试；测试文件不可并行编辑同一文件）。
- Phase 4：C11、Cargo lock 整理、格式化、针对性验证。

冲突说明:

- `src/tools/mod.rs`、`src/rpc.rs` 和 `Cargo.toml` 是共享整合点，不安排多个 agent 同时编辑。
- `src/tools/bash.rs`、`src/tools/pwsh.rs`、`src/rpc.rs` 的生命周期语义必须使用同一个 helper，不能各自复制 Job 创建逻辑。
- `src/tools/windows_process.rs` 的 Child 消费/Drop 顺序是关键接口；调用方修改必须在其基础上进行。

---

## 4. 验证计划（Validation Plan）

自动化检查（Windows 下均通过 `pwsh` 执行 Cargo）:

- `cargo test --test dropin_tool_io_differential`
- `cargo test --lib tools`
- `cargo test --lib rpc::tests::run_bash_rpc`
- `cargo clippy --lib -- -D warnings`
- `cargo fmt --check`
- 全部实现和针对性测试通过后：`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`

预期结果:

- Windows bash timeout、pwsh timeout/abort、RPC bash abort/关闭和正常返回后的延迟 descendant 均不能写入测试文件。
- 正常 shell 命令 stdout/stderr、cwd、UTF-8、退出码和现有 truncation/spill/cancellation details 不变。
- descendant 继承 stdout/stderr pipe handle 时，Job close 后 reader 能在既有 drain deadline 内结束，不永久阻塞。
- 两个并发 shell 调用使用独立 Job；一个调用的终止不会影响另一个。
- managed spawn/Job attachment 失败只返回错误，不启动不受管成功路径或遗留进程。
- Unix 原有 process-group/tree 回归继续通过。

人工检查:

- 检查 `windows-spawn::DropPolicy::KillTree` 确实只用于本阶段三个 Windows shell路径。
- 检查没有 `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`、`JOB_OBJECT_LIMIT_BREAKAWAY_OK`、主仓库 unsafe FFI、临时外部 Job attachment 或静默旧树 fallback。
- 检查 guard Drop、重复 kill/wait、根进程先退出和 pipe reader 的所有权/关闭顺序。
- 检查未把项目描述为严格 drop-in 或完整进程沙箱；外部 breakaway/service/任务计划边界仍在文档中。
