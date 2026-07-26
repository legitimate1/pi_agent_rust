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

### 关键依赖

| Crate | 用途 |
|-------|------|
| `asupersync` | 结构化并发异步运行时 |
| `rich_rust` | 终端 UI 渲染（标记语法）|
| `serde` + `serde_json` | JSON 序列化 |
| `clap` | CLI 参数解析 |
| `crossterm` | 底层终端控制 |
| `thiserror` | 错误类型定义 |

## 代码编辑规范

- **不要用脚本批量改代码** — 手动逐处改。批量简单修改用并行 subagent。
- **不要创建文件变体** — 没有 `main_v2.rs`、没有 `main_improved.rs`。原地改。新文件只给真正的新功能。
- **不向后兼容** — 没用户，不留技术债。不要兼容垫片，直接改。

## Drop-In 声明

除非 `docs/contracts/dropin-certification-contract.json` 的硬性条件满足，否则不要将 Pi Rust 描述为严格的 drop-in 替代品。`docs/evidence/dropin-certification-verdict.json` 的 `overall_verdict = CERTIFIED` 才是发布闸门。

## 静态检查（质量门禁）

改代码后必须跑：

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

## 构建与部署

### 规则
1. 每次构建前升版本号：`cargo set-version --bump patch`
2. 构建和部署分离 — 构建后等用户指令再部署
3. 不得私自构建或部署

### 流程

```
用户说「构建」→ cargo set-version --bump patch → git add + commit → cargo build --release → 停下
用户说「部署」→ .\scripts\deploy-release.ps1
```

## 测试

```bash
cargo test                    # 全部
cargo test -- --nocapture     # 带输出
cargo test conformance        # 一致性测试
cargo test sse::tests         # 特定模块
```

### 一致性测试

基于 JSON 测试夹具，验证内置工具行为符合预期：

```json
{
  "version": "1.0",
  "tool": "tool_name",
  "cases": [
    {
      "name": "test_name",
      "setup": [{"type": "create_file", "path": "...", "content": "..."}],
      "input": {"param": "value"},
      "expected": {
        "content_contains": ["..."],
        "content_regex": "...",
        "details_exact": {"key": "value"}
      }
    }
  ]
}
```

## 第三方库使用

如果你不是 100% 确定如何使用某个第三方库，上网搜索最新的文档和最佳实践，不要猜。

## 功能域→文件快速定位

| 功能域 | 入口文件 |
|:-------|:---------|
| CLI 入口 + 子命令 | `src/main.rs` |
| Agent 主循环 | `src/agent.rs` |
| Abort 信号原语 | `src/abort.rs` |
| Provider 层（10 个实现模块） | `src/providers/` |
| 内置工具（9 个） | `src/tools/` |
| 交互式 TUI | `src/interactive.rs` + `src/tui.rs` |
| RPC/stdin 模式 | `src/rpc.rs` |
| 扩展体系（协议 + QuickJS 桥接） | `src/extensions.rs` + `src/extensions_js.rs` |
| 会话持久化 | `src/session.rs` + `src/session_index.rs` |
| 配置加载 | `src/config.rs` |
| 模型注册表 | `src/models.rs` |
| 系统提示词构建 | `src/app.rs` |

## 会话结束

1. 为未完成的工作创建问题
2. 跑测试、静态检查、构建
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

| 内容 | 文档 |
|:-----|:-----|
| 完整功能清单（每条功能→文件映射） | `docs/context/features.md` |
| 详细架构（工具系统、扩展加载流程、模块关系） | `docs/context/architecture.md` |
| 命名规范、隐含假设、反模式 | `docs/context/conventions.md` |

### 按需查阅（仅特定场景需要）

| 需要什么时读 | 文档 |
|:------------|:-----|
| 做架构级改动、理解历史决策背景 | `docs/context/design-decisions.md` |
| 更新构建/测试/部署流程（日常开发看 AGENTS.md 的「构建」+「测试」+「会话结束」即可） | `docs/context/commands.md` |
