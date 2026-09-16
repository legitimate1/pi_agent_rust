# IMPLEMENT：Wave 2 provider usage/quota 适配

## 1. 实现边界（Implementation Contract）

Source: `docs/upstream/waves/02-provider-usage-quota.md` — Wave 2 处理结论与功能闭包

Goal:
- 将上游 provider usage/quota 新功能按当前 `custom` 的认证、HTTP、CLI 和 interactive 结构手动适配进来。

In scope:
- 新增 provider usage/quota 领域模块，支持上游已确认的 OpenRouter、Moonshot/Kimi、GitHub Copilot reader 语义。
- 复用当前 `AuthStorage`、`Config::auth_path()` 和 HTTP client，不升级主依赖。
- 接入 `pi usage` CLI 子命令，支持文本/JSON 输出及 refresh 选项。
- 接入 interactive `/usage` 命令及帮助文本。
- 保留上游的超时、内存缓存、stale fallback、Unavailable/Error 状态和 credential 脱敏语义。
- 为新增公共接口、reader、缓存/失败边界和命令接入补充针对性测试。

Out of scope:
- 不处理 Wave 1 fsqlite SQLite 后端迁移。
- 不执行整段上游历史 merge，也不带入 beads 收尾提交。
- 不修改 `Cargo.toml`/`Cargo.lock`，不升级 asupersync、HTTP/TLS 或其他主依赖。
- 不改 provider streaming/token usage、retry/failover、session persistence、RPC/MCP、extensions 或 FTUI。
- 不为没有公开 quota endpoint 的 provider 猜测或伪造额度。
- 本阶段不新增 RPC usage 协议；上游闭包没有 RPC 文件变化。

Assumptions:
- 当前 custom 的 `AuthStorage`、`Config::auth_path()`、异步 HTTP client、CLI dispatch 和 interactive event 投递结构可作为适配接入点。
- provider endpoint 和响应 schema 以固定上游实现为初始参考；无法在本地无网络验证的部分必须保留为测试/风险说明，不伪称已验证。
- 使用当前 custom 的既有依赖和错误处理边界；如发现必须引入依赖或改变公共协议，停止并报告，不扩大范围。

Design delta:
- 不直接采用上游提交在 `src/interactive/commands.rs`、`src/interactive/perf.rs`、`src/lib.rs` 中的 patch 上下文；改为按 custom 当前结构手动插入。
- 上游的 `src/usage.rs` 只作为语义和 reader 行为参考，实际实现需核对 custom HTTP/Auth API 后调整。
- 上游闭包原有的 `interactive/perf.rs` 改动只保留 usage 展示所需的最小接入，不带入 checkpoint、undo/redo 等相邻功能上下文。

---

## 2. 文件变更清单（Change Manifest）

| ID | 路径 | 操作 | 用途 | 主要改动 |
|:--:|:-----|:----|:-----|:---------|
| C1 | `src/usage.rs` | 新建 | provider usage/quota 领域闭包 | 统一结果类型、provider readers、超时、缓存、stale fallback、文本/JSON 渲染及局部测试 |
| C2 | `src/lib.rs` | 修改 | 注册新领域模块 | 在当前模块列表中加入 `pub mod usage;` |
| C3 | `src/cli.rs` | 修改 | CLI 命令声明 | 在当前 `Commands` 中加入 `Usage { format, refresh }` 及 root subcommand 识别 |
| C4 | `src/main.rs` | 修改 | CLI 分发 | 加载当前 auth，调用 usage gather/render，处理格式和 refresh |
| C5 | `src/interactive/commands.rs` | 修改 | 交互命令解析与分发 | 加入 `/usage`、`/usage refresh` 解析、帮助和命令处理 |
| C6 | `src/interactive/perf.rs` | 修改 | 交互展示接入 | 按 custom 当前 event/transcript/status 模式展示 usage 结果；仅保留必要变更 |
| C7 | `tests/` 或相关已有测试模块 | 修改/新建 | 回归验证 | 覆盖 reader 解析、Unavailable/Error、缓存/refresh、渲染和 CLI/interactive 接入；具体位置以现有测试组织为准 |

不得修改：`Cargo.toml`、`Cargo.lock`、`src/session*.rs`、`src/rpc.rs`、provider streaming 主流程及 Wave 1 文件。

---

## 3. 依赖关系（Dependency Plan）

Dependencies:
- C1 依赖：当前 `AuthStorage`、HTTP client API 的定向核对；无新 Cargo 依赖。
- C2 依赖：C1。
- C3 依赖：C1 的公开命令调用接口。
- C4 依赖：C1、C3。
- C5 依赖：C1。
- C6 依赖：C1、C5；需沿 custom 当前交互事件模式适配。
- C7 依赖：C1、C3、C4、C5、C6 的实际接口完成后统一补齐。

执行顺序：
- Phase 0：核对 custom 的 Auth/HTTP/CLI/interactive 局部接口和上游 usage 实现。
- Phase 1：实现 C1，并在模块内完成核心纯逻辑/reader 测试。
- Phase 2：实现 C2、C3、C4、C5、C6；涉及相同交互文件的改动必须串行处理。
- Phase 3：由主 Agent 统一按 `add-tests` 流程补测试、运行针对性验证和静态检查。
- Phase 4：更新 Wave 2 文档的实际处理结论和验证摘要；确认后再提交。

冲突说明：
- `src/interactive/commands.rs`、`src/interactive/perf.rs`、`src/lib.rs` 已证明不能直接 cherry-pick，必须由同一实现上下文手术式修改。
- 不允许并行 Agent 同时编辑上述文件。
- 实现 worktree 与父工作区隔离；只有验证通过后才应用变更。

---

## 4. 验证计划（Validation Plan）

自动化检查：
- 针对 usage 模块和新增/修改测试运行最小 Cargo 测试命令（Windows 下通过 `pwsh`）。
- `cargo fmt --check`。
- `cargo clippy --lib -- -D warnings`。
- 若修改了独立集成测试文件，运行该测试文件直到通过；不在日常阶段运行 `--all-targets`。

预期结果：
- usage reader 的成功、Unavailable、错误、缓存回退和 refresh 行为有测试证据。
- `pi usage` 与 `/usage` 接入能够通过编译和针对性测试。
- 不产生 Cargo 依赖变化，不影响现有 provider token usage、session、RPC 或 extension 行为。
- `git diff --check` 通过，父工作区在隔离 worktree 应用前保持干净。

人工检查：
- provider credential 不出现在日志和渲染错误中。
- 没有公开 endpoint 的 provider 不伪造数据。
- 只带入 usage 语义，不带入上游相邻功能、beads 元数据或冲突上下文。
- 对 provider endpoint 未做网络验证的部分在波次文档中明确标记为未知。
