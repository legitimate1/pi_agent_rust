# LSP 独立移植实现契约

> 状态：待确认。本文只定义 LSP 试点的实现边界，不表示 C1 已合入，也不接受 C1 全量 merge。

## 1. 实现边界

Source:

- `docs/upstream/c1/feature-matrix.md` 的 A1 LSP 条目
- `docs/upstream/c1/dependency-matrix.md` 的低依赖候选结论
- `docs/upstream/c1/overlap-risk-matrix.md` 的 LSP 风险条目
- 上游首个功能提交 `912a4650e0ca39246114d3082ba92099d973f852`
- 上游任务 `bd-cv653.1.1`

Goal:

- 在当前 `custom` 架构中，以独立功能闭包提供 LSP 代码智能工具，不接受上游 C1 的整体依赖升级和结构重构。

In scope:

- 子进程语言服务器的 Content-Length JSON-RPC 通信。
- 语言服务器注册、按 workspace 选择和生命周期管理。
- 诊断、定义、引用、悬停、符号、重命名、文件重命名、代码操作、类型定义、实现、状态、重载、能力查询和原始请求等 LSP 操作；最终以 custom 接口与可验证测试为准。
- WorkspaceEdit 的路径校验、原子应用和失败回滚。
- 接入 custom 当前 `Tool` trait 与 `src/tools/mod.rs` 的 `ToolRegistry`。
- 增加 LSP 配置的最小字段和 CLI 工具选择支持。
- LSP 单元测试、无语言服务器时的可诊断失败测试，以及条件化的真实 `rust-analyzer` 验证。

Out of scope:

- C1 全量 merge 或继续解决 `integration/c1` 的 33 个源码冲突。
- `src/tools.rs` 单文件到 `src/tools/` 的结构迁移。
- extensions / extensions_js 重构、VFS 语义、Hub/Jobs/Subagents、FTUI、FrankenSQLite。
- `asupersync`、`rquickjs`、SWC、`wasmtime`、Rust toolchain 等主依赖升级。
- 自动安装语言服务器、网络下载、编辑器插件协议适配。
- 引入或重建上游 `xdev` 系统；第一版使用 custom 已有工具注册路径，工具是否默认启用须单独确认。

Assumptions:

- custom 当前 `Tool` trait 的取消信号、工具效果声明和文件触碰记录必须继续生效。
- LSP 服务器是外部可执行文件，缺失时返回安装提示，而不是静默降级到 grep。
- 所有由 LSP 触发的文件写入仍经过 custom 的路径约束和原子写入边界。
- 首个实现优先验证 rust-analyzer；其他语言服务器只验证配置模型和错误分类，不承诺逐一实测。

Design delta:

- 不直接照搬上游修改 `src/tools.rs`、`src/xdev.rs` 和上游配置结构；先映射到 custom 当前目录化 tools 与配置模型。
- 不把 LSP 默认暴露策略预先视为已决定：建议第一版先通过显式 `--tools lsp` 或等价配置启用，完成验证后再决定是否加入默认工具集合。
- 不为适配 LSP 而升级主依赖；若编译暴露 custom 现有 API 缺口，先停止并记录，不扩大到 C1 依赖迁移。

## 2. 文件变更清单

### 新建

- `src/lsp.rs`：LSP 工具外层、输入动作、结果映射和工具契约。
- `src/lsp/client.rs`：服务器进程、请求/响应关联、超时和取消。
- `src/lsp/jsonrpc.rs`：Content-Length framing 与 JSON-RPC 消息处理。
- `src/lsp/registry.rs`：服务器配置、语言/扩展名匹配和 workspace 选择。
- `src/lsp/edits.rs`：WorkspaceEdit 校验、排序、原子应用和回滚。
- `src/lsp/text.rs`：行列位置、UTF-16 映射和文本范围转换。
- `tests/lsp.rs`：协议、注册、编辑原子性和错误分类测试。
- `tests/e2e_lsp.rs`：条件化 rust-analyzer 端到端测试；缺少服务器时必须可解释地跳过或失败。
- `scripts/e2e/run_lsp.sh`：聚焦 E2E 入口（若 custom 的现有 E2E 调度约定要求该脚本）。

### 修改

- `src/lib.rs`：声明并导出 `lsp` 模块。
- `src/tools/mod.rs`：在当前目录化 `ToolRegistry` 中注册 LSP，不覆盖现有工具实现。
- `src/config.rs`：增加最小 LSP server 配置与 timeout/idle 字段；兼容现有 snake_case/camelCase 解析约定。
- `src/cli.rs`：增加 LSP 的显式工具选择和必要的帮助文本；不默认扩大所有工具列表，除非验证后另行决定。
- 相关测试注册文件：仅在现有 suite 分类机制确实要求时增加 LSP 条目。

### 不修改

- `Cargo.toml`、`Cargo.lock`：首轮预期不增加 Cargo 依赖；若实际代码需要新增直接依赖，必须先回到本契约复审。
- `src/extensions.rs`、`src/extensions_js.rs`、`src/session*.rs`、`src/rpc.rs`、`src/hub.rs`、`src/jobs.rs`：除非编译或接口核查证明是 LSP 的直接必要接入点，否则保持不动。

## 3. 依赖关系

- Phase 1：确认 custom 工具 trait、路径约束、取消信号和测试 harness 的最小接口。
- Phase 2：实现 `jsonrpc`、`text`、`client`、`registry`、`edits`；这些模块之间按协议、文本、进程、配置、编辑的依赖顺序推进。
- Phase 3：实现 `lsp.rs` 工具外层并接入 `src/tools/mod.rs`、`src/lib.rs`、`src/config.rs`、`src/cli.rs`。
- Phase 4：增加单元测试和条件化 E2E，完成最小验证闭环。

不可并行编辑的边界：

- `src/tools/mod.rs`、`src/config.rs`、`src/cli.rs` 由同一集成任务收口，避免注册名、配置和 CLI 默认值不一致。
- `src/lsp/edits.rs` 与 custom 的现有写入/路径约束接口必须在同一轮确认，不能先实现绕过约束的临时写入。
- 测试文件可独立补充，但不能在实现接口未稳定前固化上游字段名称。

## 4. 验证计划

自动化检查：

- `cargo test --test lsp`
- `cargo test --test e2e_lsp`（无 `rust-analyzer` 时按测试契约给出明确 skip；强制模式必须失败）
- `cargo test --lib lsp::`
- `cargo clippy --lib -- -D warnings`
- `cargo fmt --check`
- 若修改了 suite 分类，再运行对应的分类/清单测试。

预期结果：

- JSON-RPC framing 能正确处理多条消息、空响应、错误响应和超时。
- 服务器缺失、协议错误、超时和取消均返回可诊断错误，不永久阻塞 Agent。
- WorkspaceEdit 多文件应用满足全成全败；部分失败不会留下半写状态。
- 路径访问不能绕过 custom workspace 约束。
- LSP 工具可从 custom 的 ToolRegistry 构造并执行；不引入 C1 主依赖漂移。
- 在真实 rust-analyzer 可用时，至少完成 diagnostics 或 definition 一条真实链路；完整 14 操作作为后续验收目标，不因服务器缺失而伪造通过。

人工检查：

- 语言服务器缺失时提示可执行的安装/配置动作。
- 取消和 idle shutdown 不遗留子进程。
- rename/rename_file 的文件修改可被 custom 的 touched-files、路径审计和后续 undo 语义观察到；若无法接入，停止而不是绕过。
- 明确记录本闭包的保留 custom、采纳 upstream、冻结/延期三类决策。

## 5. 试点停止条件

- 需要升级 `asupersync` 或其他主依赖才能编译。
- 需要修改 extensions、Hub、session 或工具拆分结构才能继续，但尚未形成独立设计。
- WorkspaceEdit 无法满足 custom 的路径约束或原子性要求。
- 测试失败无法区分 LSP 引入问题与 custom 基线问题。
- 需要读取整个仓库或 C1 全部提交才能继续判断。

> 本契约需要用户确认后才进入代码实现。
