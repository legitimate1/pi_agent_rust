# pi_agent_rust — Pi CLI 编程智能体

高性能 AI 编程智能体 CLI，Rust 移植版。提供交互式终端界面、流式响应、工具执行、会话持久化。

- **语言/框架** — Rust 2024 nightly（见 `rust-toolchain.toml`），单静态二进制分发
- **关键依赖** — asupersync（结构化并发运行时）、rich_rust（终端 UI）、serde、clap、rquickjs（扩展沙箱）、ast-grep-language/ast-grep-core（AST 结构工具）

---

## 基本规则

- **我的话优先** — 即使与下文冲突，听我的。
- **删文件前先问** — 包括你自己创建的文件。你说过你删错过东西，所以这条不破。
- **禁止危险命令** — `git reset --hard`、`git clean -fd`、`rm -rf` 必须由我明确手写命令才能执行。不确定就问。
- **分支用 `custom`** — 这是当前推送的主要分支。`main` 是 upstream 跟踪的分支，不做直接推送。

## 工具链

- **只认 Cargo**，不用其他包管理器
- Rust 2024 nightly（见 `rust-toolchain.toml`）
- 不安全代码禁止（`#![forbid(unsafe_code)]`）
- **Windows 构建约束** — 依赖 `ring` / `libsqlite3-sys` / `rquickjs-sys` / `tree-sitter` 等含 C/C++ 代码的 crate，编译必须依赖 MSVC 工具链（`cl.exe`）。Windows 下所有 `cargo` 相关命令**必须通过 `pwsh` 执行**（`$PROFILE` 已自动注入 `vcvars64` 并补全 `fd` / `cygpath` / `sccache` / `lld-link` 路径），**禁止在 `bash` / `git bash` 中直接执行 `cargo`**，否则将因缺失 MSVC 环境导致 `build-script-build` 失败；如需绕过自动注入请显式调用 `cargo.exe`

## 代码编辑规范

- **不要用脚本批量改代码** — 手动逐处改。批量简单修改用并行 subagent。
- **不要创建文件变体** — 没有 `main_v2.rs`、没有 `main_improved.rs`。原地改。新文件只给真正的新功能。
- **不向后兼容** — 没用户，不留技术债。不要兼容垫片，直接改。

## Drop-In 声明

除非 `docs/contracts/dropin-certification-contract.json` 硬性条件满足，否则不要把 Pi Rust 描述为严格的 drop-in 替代品；发布闸门是 `docs/evidence/dropin-certification-verdict.json` 的 `overall_verdict = CERTIFIED`。

## 静态检查（质量门禁）

- **日常改动**（轻量，快速反馈）：
  ```pwsh
  cargo clippy --lib -- -D warnings && cargo fmt --check
  ```
- **收尾**（全部改动完成后，与全量测试一起跑）：
  ```pwsh
  cargo clippy --all-targets -- -D warnings && cargo fmt --check
  ```
  `--all-targets` 会编译全部集成测试二进制（Windows 下单个 ~64MB，全套 ~30GB），只允许在收尾时跑。

## 构建与部署

### 规则

1. 每次构建前升版本号：`cargo set-version --bump patch -p pi_agent_rust`（`-p pi_agent_rust` 限仅升主 crate，避免 workspace 成员全部跳版本）
2. 构建和部署分离 — 构建后等用户指令再部署
3. 不得私自构建或部署

### 流程

```
用户说「构建」→ cargo set-version --bump patch -p pi_agent_rust → git add + commit → cargo build --release → 停下
用户说「部署」→ .\scripts\deploy-release.ps1
```

> 构建前**不**重复跑全量测试 — 收尾门禁已验证过。若用户中途要求构建（改动未收尾），先跑针对性测试确认无误再构建。部署脚本自动执行 `cargo sweep --file` + `--stamp` 清理旧产物，无需手动清理。
>
> Release profile 契约（`Cargo.toml` 的 `[profile.release]`：`opt-level = 3` + `lto = "thin"` + `panic = "abort"` + `strip = true`，+ 校验 + 预算门禁）见 `docs/context/commands.md`「构建与部署配置」。release 二进制大小预算由 `BINARY_SIZE_RELEASE_BUDGET_MB` 定义。jemalloc is opt-in via `--features jemalloc`，不要默认启用。

## 测试

- **日常改动**：只跑针对性测试（`cargo test <模块>` / `cargo test --test <文件>`），
  外加轻量静态检查 `cargo clippy --lib -- -D warnings && cargo fmt --check`。
  禁止日常跑全量 `cargo test`（全量编译 ~30GB debug 产物，见「静态检查」）。
- **全部改动完成（收尾）**：跑一次全量 `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`，通过后**提醒用户构建**（不自动构建）。
- 创建/修改测试文件时：必须运行该测试文件直到通过。

常用针对性命令：

```pwsh
cargo test --test conformance_fixtures   # 特定集成测试文件
cargo test conformance                   # 一致性测试
cargo test sse::tests                    # 特定模块
cargo test -- --nocapture                # 带输出
```

> 一致性测试夹具格式（JSON schema）见 `docs/context/commands.md`「测试与验证」。

## 第三方库使用

如果你不是 100% 确定如何使用某个第三方库，上网搜索最新的文档和最佳实践，不要猜。

## 会话结束

1. 为未完成的工作创建问题
2. 跑收尾质量门禁（全量测试 + clippy --all-targets + fmt，通过后提醒用户构建）
3. 更新问题状态
4. **推送到远程（强制）：**
   ```bash
   git pull --rebase
   git add -A
   git commit -m "..."
   git push
   git status  # 必须显示 up to date
   ```
5. 清理暂存区、修剪远程分支
6. 提供上下文给下一次会话

## 快速导航

> 📌 **接手必读** = 接手项目时就要读完
> 🔍 **按需查询** = 开发中遇到具体问题再查，接手时不预读

### 接手必读

- **功能目录（含文件映射）** — `docs/context/features.md`
  - 有什么功能、代码在哪；先 `grep 关键词` 命中即要点，需要全局视角时整读
- **架构骨架** — `docs/context/architecture.md`
  - 核心数据流、工具系统、扩展加载流程、模块关系、运行时不变量
- **命名规范、约定与反模式** — `docs/context/conventions.md`
  - 反模式跨域通用，不可按域切，必读

### 按需查询

- **开发命令（低频/发布/RPC 协议）** — `docs/context/commands.md`
  - 修改运行配置/机制、跑低频命令、发布流程时读；高频命令见本文件上方
- **Windows 开发环境搭建** — `docs/context/windows-setup.md`
  - 新机/重装后首次搭建，或 `cargo` 报 MSVC/linker/sccache 错误时读；含一键搭建、自适配清单与故障排查
- **设计决策（为什么没选 B）** — `docs/context/design-decisions.md`
  - 做架构级改动、理解决策背景时读；过时决策在 `docs/context/design-decisions-archive.md`
- **verify 验证引擎** — `docs/context/verify-tool.md`
  - 新增静态检查工具 / 优化 verify 检查逻辑时读
- **症状排查手册** — `docs/context/debugging.md`
  - 症状已知但根因不明时读：症状路由表、调试 playbook、安装器补丁模式
- **追上游 / 合并上游** — `docs/upstream/`
  - 追上游时读 `fork-merge-sop.md`(SOP)+ `known-test-failures.md`(对照基准)+ `upstream-qa-bead-swarm-guide.md`(体系理解);全量测试跑完对照已知失败清单
