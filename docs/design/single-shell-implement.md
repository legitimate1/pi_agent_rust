# IMPLEMENT.md

## 1. 实现边界（Implementation Contract）

Source: `docs/design/single-shell-design.md` — 全量（重点：§工具契约 §实现方案 §验证策略），Review 定稿版（已采纳 14/16，PS 5.1 与 AGENTS.md 不改）

Goal:

- 将 `bash` / `pwsh` 双工具收口为单一 `shell(shell, command, timeout?)`，薄转发复用现有 `BashTool`/`PwshTool`，默认单工具列表，`PI_ENABLE_LEGACY_SHELL` 带外可逆逃生

In scope:

- `src/tools/shell.rs` 新建：极简契约 `shell: bash|pwsh 必填, command 必填, timeout?: integer minimum:0`，`command.trim().is_empty()` / `shell` 非枚举 / `timeout <0` 均 `validation` 拒绝，透传 `cwd`+`command`+`timeout`+`abort` 到底层 `run_*_command`
- `src/tools/mod.rs` 修改：`mod shell; pub use shell::ShellTool;`，`ToolRegistry::new` 常驻 `shell`，`PI_ENABLE_LEGACY_SHELL=1|true|yes|on`（`trim`+`to_ascii_lowercase`）时追加注册 `bash`/`pwsh`，保留 `bash.rs`/`pwsh.rs` 文件不删
- 5 单测（`#[cfg(test)]` 于 `shell.rs` 内）：`forwards_to_pwsh_and_bash` / `invalid_shell(含大小写)` / `timeout_validation(None/0/<0/string)` / `empty_command` / `timeout_forwarded_to_backend`
- 文档：`docs/context/features.md` 合并条目、`docs/context/conventions.md` 反模式 1 条、`docs/context/design-decisions.md` 新增 `D52`
- 薄转发耦合注释与 Flag 判定抽取 `is_legacy_shell_enabled()`

Out of scope:

- `AGENTS.md` — 用户明确不改（保留原 Windows 构建约束）
- `auto` 自动路由 / `workdir` / `.pi/tools.toml` 预置护栏（V1 先不建，观测 1-2 会话或首次 `cargo→bash` 错即补）
- `Cargo.toml` 依赖新增 / `forbid(unsafe_code)` 变更 / 单二进制分发變更
- `src/sdk.rs` 的 `create_all_tools` 切换（V1 仅 CLI `ToolRegistry`，SDK 保持现状，待后续按需对齐）
- `drop-in` / `tools.toml` 契约快照集成测试

Assumptions:

- `pwsh` = PowerShell 7 (`C:\Program Files\PowerShell\7\pwsh.exe`)，已支持 `&&`，不兼容 PS 5.1
- `bash` = Git Bash（`resolve_bash_shell`），Unix 走 `/bin/bash` 家族
- `ToolRegistry::new` 的 `cwd` 锚定即 `Agent cwd == shell cwd`，跨目录靠 `cd dir && cmd` 前缀
- `timeout` 底层语义：`None`→`120s`，`Some(0)`→禁用，`timeout=0` 视为受控长阻塞（风险接受，不封顶）
- Rust 2024 nightly + `forbid(unsafe_code)`，薄转发 bug 面 `1 match + 1 enum + timeout/command 校验`

Design delta:

- `AGENTS.md` 工具链简化与 Flag 说明移除（与 Review 建议 15 相反，按用户终审不改）
- 其余按 Review 定稿：`timeout minimum:0` + `Option<i64>` 接收、`command` 空串校验、`shell` 错误文案 `Expected shell to be "bash" or "pwsh"`、Flag `trim+lowercase+on`、Mermaid 拆行、`shell.rs` 顶部耦合注释、验证 2→5 例、注意事项补 `cd` 授权与 `timeout=0` 风险接受与观测阈值

---

## 2. 文件变更清单（Change Manifest）

| ID  | 路径                               | 操作 | 用途                        | 主要改动                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| :-: | ---------------------------------- | ---- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1  | `src/tools/shell.rs`               | 新建 | 单一 shell 薄转发核心与校验 | `ShellTool { cwd: PathBuf }`；`ShellInput { shell: String, command: String, timeout: Option<i64> }`；`name()="shell"`，中文极简 `description`；`parameters()` 含 `shell enum[bash,pwsh] 必填 + command 必填 + timeout integer minimum:0`；`execute` 内 `command.trim().is_empty()` / `shell` 枚举 / `timeout<0` validation，`match { bash => run_bash_command(&self.cwd,...), pwsh => run_pwsh_command(&self.cwd,...) }` 透传 `abort`；文件头 `// BashTool/PwshTool 签名变更必须同步更新此处 match`；`#[cfg(test)]` 5 例 |
| C2  | `src/tools/mod.rs`                 | 修改 | 注册收口与 Flag 逃生        | `mod shell; pub use shell::ShellTool;`；`ToolRegistry::new` 中 `tools.push(ShellTool::new(cwd))` 常驻；`is_legacy_shell_enabled()` 判定 `env PI_ENABLE_LEGACY_SHELL trim+lowercase in [1,true,yes,on]` 时追加 `BashTool`/`PwshTool`；保留既有 `bash`/`pwsh` match 分支仅在 Flag 下生效                                                                                                                                                                                                                                   |
| C3  | `docs/context/features.md`         | 修改 | 功能目录收口                | `bash` + `pwsh` 两条合并为 `shell(shell, command, timeout?) — 统一 shell，显式方言，当前 cwd，薄转发，Flag 逃生` 单条，指向 `shell.rs` + `mod.rs`                                                                                                                                                                                                                                                                                                                                                                        |
| C4  | `docs/context/conventions.md`      | 修改 | 反模式收口                  | 新增 `双壳选错→收口到 shell` 或在“子进程 stdin 显式 null”旁加注，指向 `shell(shell=bash                                                                                                                                                                                                                                                                                                                                                                                                                                  | pwsh, ...)` |
| C5  | `docs/context/design-decisions.md` | 修改 | 决策沉淀                    | 新增 `D52: 单一 shell 抽象（显式方言 + Flag 逃生）`，含 Why/Why not/何时重考虑（`auto/workdir/tools.toml` 阈值）                                                                                                                                                                                                                                                                                                                                                                                                         |

---

## 3. 依赖关系（Dependency Plan）

Dependencies:

- C1 依赖：无（可独立新建，`bash.rs`/`pwsh.rs` 的 `run_*_command` 已存在）
- C2 依赖：C1（需 `ShellTool` 类型与 `is_legacy_shell_enabled`）
- C3 依赖：C1, C2（文案依赖契约定稿）
- C4 依赖：C1, C2
- C5 依赖：C1, C2

并行分组建议：

- Phase 1：C1（新建 `shell.rs`，含 5 单测骨架）
- Phase 2：C2（`mod.rs` 注册与 Flag，需 C1 完成才可编译）
- Phase 3：C3, C4, C5（文档 3 项可并行，均只依赖 Phase 2 定稿）

冲突说明:

- `C1` 与 `C2` 无同文件冲突（`shell.rs` vs `mod.rs`），但 `C2` 编译期依赖 `C1`
- `C3/C4/C5` 3 个文档文件互不冲突，可并行编辑
- `C2` 的 `mod.rs` 为热点文件，Phase 2 内串行，避免并发编辑同一文件

---

## 4. 验证计划（Validation Plan）

自动化检查:

- `cargo clippy --lib -- -D warnings` — 0 新增警告
- `cargo fmt --check` — 通过（`shell.rs` 需 `prettier` 不适用，仅 `cargo fmt`）
- `cargo test --lib shell -- --nocapture` — 5 例全绿：`forwards_to_pwsh_and_bash` / `invalid_shell_rejected` / `timeout_validation` / `empty_command_rejected` / `timeout_forwarded_to_backend`
- 可选：`cargo test --lib -- --nocapture` 冒烟（`bash`/`pwsh` 既有单测不受影响）

预期结果:

- `ToolRegistry::new(&["shell"], cwd, ...)` 日常仅含 `shell` 1 项；`PI_ENABLE_LEGACY_SHELL=1` 时含 `shell`+`bash`+`pwsh` 3 项（人肉 `PI_ENABLE_LEGACY_SHELL=1 cargo test -- --nocapture` 或 `pi --help` 看 tools）
- `shell(shell="fish")` / `shell="Bash"` / `command=""` / `timeout=-1` 均返回 `validation` 错误，不起进程
- `shell(shell="pwsh", command="cargo --version")` 与 `shell(shell="bash", command="echo hi")` 各 1 次人肉透传成功，`is_error==false`

人工检查:

- `docs/design/single-shell-design.md` 的契约与实现方案描述与 `shell.rs` 实际校验一致（`minimum:0`、`trim`、`lowercase`）
- `docs/context/design-decisions.md` 的 `D52` 与 `deliberate` 11 节点一致且含观测阈值“1-2 会话或首次 `cargo→bash` 错即补 `.pi/tools.toml`”
