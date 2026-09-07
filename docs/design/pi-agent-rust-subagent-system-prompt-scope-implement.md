# Subagent SYSTEM.md 隔离实现文档

## 1. 实现边界（Implementation Contract）

Source: 用户已确认的设计结论

Goal:
- 让 Subagent 完全不加载用户级或项目级 `SYSTEM.md`；`SYSTEM.md` 仅用于 Main Agent 的系统提示词覆盖。

In scope:
- 为系统提示词构建流程增加明确的 Main/Subagent scope 区分。
- Subagent 启动时显式使用 Subagent scope，避免读取任意层级的 `SYSTEM.md`。
- 保留 Subagent 对 `AGENTS.md` / `CLAUDE.md` 等共享项目上下文、Agent 定义角色提示词和 schema 指令的访问。
- 补充 Main/Subagent 的 `SYSTEM.md` 隔离回归测试。

Out of scope:
- 不新增 `COMMON_SYSTEM.md`、`SUBAGENT_SYSTEM.md` 或其他兼容层。
- 不改变父子会话历史、工具白名单、worktree 隔离和 schema 契约。
- 不阻止 Subagent 通过拥有的文件工具主动读取磁盘上的 `SYSTEM.md`；本次只保证其不被自动注入模型上下文。

Assumptions:
- `AGENTS.md` / `CLAUDE.md` 仍是 Main 与 Subagent 共享的项目上下文。
- Agent definition 的 `system_prompt` 继续通过 `--append-system-prompt` 注入子进程。
- 不保持原先 Subagent 自动继承 `SYSTEM.md` 的行为兼容性。

Design delta:
- 将现有 `build_system_prompt` 中无条件按 CLI/cwd/global_dir 解析 `SYSTEM.md` 的逻辑改为仅在 Main scope 执行。
- 子进程通过显式内部 scope 参数启动，而不是依赖深度或环境变量推断。

---

## 2. 文件变更清单（Change Manifest）

| ID | 路径 | 操作 | 用途 | 主要改动 |
|:--:|:-----|:----|:-----|:---------|
| C1 | `src/app.rs` | 修改 | 系统提示词组装 | 增加 prompt scope；Subagent scope 跳过用户级和项目级 `SYSTEM.md`，继续组装共享上下文、skills 和 runtime facts |
| C2 | `src/subagents.rs` | 修改 | 子进程启动参数 | 为子进程传递明确的 Subagent prompt scope，并保持 Agent role prompt/schema 注入 |
| C3 | `src/main.rs` 或 CLI 参数定义所在文件 | 修改 | 子进程参数解析 | 增加内部 scope 参数并将其传递给 system prompt 构建入口；仅允许内部启动路径使用 |
| C4 | `src/app.rs` / `src/subagents.rs` 现有测试区域 | 修改 | 回归保护 | 验证 Main 读取两类 `SYSTEM.md`，Subagent 不读取两类 `SYSTEM.md`，同时保留 `AGENTS.md` 和 role prompt |
| C5 | `docs/context/features.md`、`docs/context/subagent.md` | 修改 | 更新行为契约 | 记录 `SYSTEM.md` 仅属于 Main、Subagent 不自动加载的语义 |

---

## 3. 依赖关系（Dependency Plan）

Dependencies:
- C1 依赖：无
- C2 依赖：C1 的 scope 接口
- C3 依赖：C1 的 scope 接口
- C4 依赖：C1、C2、C3
- C5 依赖：C1、C2、C3

并行分组建议：
- Phase 1：C1、C3（同一参数契约下协调修改，不并行编辑同一文件）
- Phase 2：C2
- Phase 3：C4、C5

冲突说明：
- `src/app.rs`、`src/subagents.rs` 需避免多个 Agent 同时编辑；实现与测试集中串行完成。
- 若实际 CLI 参数定义位置不同，以代码中的真实入口为准，不新增重复参数解析路径。

---

## 4. 验证计划（Validation Plan）

自动化检查：
- 针对 `app` / `subagents` 的现有测试。
- 新增的 `SYSTEM.md` scope 回归测试。
- `cargo clippy --lib -- -D warnings`
- `cargo fmt --check`

预期结果：
- Main scope 可按现有优先级读取用户级/项目级 `SYSTEM.md`。
- Subagent scope 在存在用户级和项目级 `SYSTEM.md` 时，生成 prompt 不包含其内容。
- Subagent scope 仍包含共享 `AGENTS.md` / `CLAUDE.md`、Agent 定义角色提示词和 schema 指令。
- 子进程 argv 包含明确的 Subagent scope；普通 Main 启动行为不受影响。
- 针对性测试、Clippy 和格式检查通过。

人工检查：
- 确认没有引入 `COMMON_SYSTEM.md` / `SUBAGENT_SYSTEM.md` 兼容层。
- 确认 `SYSTEM.md` 隔离是 prompt 加载边界，不误写成文件系统权限隔离。
- 确认文档明确说明这是有意的破坏性行为变化。
