# ═══ AGENTS.md 项目上下文 ═══

# pi_agent_rust — Pi CLI 编程智能体

高性能 AI 编程智能体 CLI，Rust 移植版。提供交互式终端界面、流式响应、工具执行、会话持久化。

**技术栈**: Rust 2024 nightly · asupersync · rich_rust · serde · clap · rquickjs

---

> 在此 Rust 代码库中工作的 AI 编程智能体指南。

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
> Release profile 契约（opt-level/thin LTO/panic/strip + 校验 + 预算门禁）见 `docs/context/commands.md`「构建与部署配置」。

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

## 接手时查阅

### 核心必读（每次接手先读这 3 个）

| 内容                                         | 文档                           |
| :------------------------------------------- | :----------------------------- |
| 完整功能清单（每条功能→文件映射）            | `docs/context/features.md`     |
| 详细架构（工具系统、扩展加载流程、模块关系） | `docs/context/architecture.md` |
| 命名规范、隐含假设、反模式                   | `docs/context/conventions.md`  |

### 按需查阅（仅特定场景需要）

| 需要什么时读                                                                                                        | 文档                               |
| :------------------------------------------------------------------------------------------------------------------ | :--------------------------------- |
| 做架构级改动、理解历史决策背景                                                                                      | `docs/context/design-decisions.md` |
| 修改运行配置/机制（profile 契约、部署机制、RPC 协议、CLI 参考、低频验证命令）                                       | `docs/context/commands.md`         |
| 新增静态检查工具 / 优化 verify 检查逻辑                                                                             | `docs/context/verify-tool.md`      |
| 症状排查 / 回归调试（provider/session/extension/安装器改动后测试失败；含症状路由表、调试 playbook、安装器补丁模式） | `docs/context/debugging.md`        |
