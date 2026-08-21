# 集成方迁移与兼容性手册 (DROPIN-163 / bd-2sx56)

生成时间：2026-02-15

## 目的

本手册为下游团队提供一条实用、低风险的路径，用于从 TypeScript 版 Pi（`pi-mono`）迁移至 `pi_agent_rust`，并在无需内部项目上下文的情况下验证兼容性。

适用于以下场景：

- 将现有的 `pi` 集成替换为 Rust 版 Pi，
- 验证自动化/脚本是否仍按预期工作，
- 以可复现的证据记录通过/不通过（go/no-go）决策。

## 兼容性契约输入

迁移前，请将以下产物固定为可信源：

- 基线快照：`docs/dropin-upstream-baseline.json`
- 表面清单：`docs/evidence/dropin-feature-inventory-matrix.json`
- 差距台账：`docs/evidence/dropin-parity-gap-ledger.json`
- 认证门禁：`docs/contracts/dropin-certification-contract.json`
- 认证裁决产物（严格声明门禁）：`docs/evidence/dropin-certification-verdict.json`
- 当前一致性状态快照（参考信息）：`docs/parity-certification.json`

若你的所需工作流在 `docs/evidence/dropin-parity-gap-ledger.json` 中映射到开放的 `critical`/`high` 差距条目，请将迁移视为阻塞状态，直到该差距已关闭或已针对你的环境明确豁免。
若 `docs/evidence/dropin-certification-verdict.json` 缺失或 `overall_verdict` 不为 `CERTIFIED`，请将严格的 drop-in 替代声明视为阻塞状态。

## 迁移完成条件

仅当以下全部满足时，迁移才算完成：

1. Rust 版 Pi 已安装，并可通过预期命令（`pi` 或 `pi-rust`）调用。
2. 所需的执行表面均通过验证（交互式、print、JSON 模式、RPC、已使用的 SDK）。
3. 提供方/认证/配置行为符合你的生产预期。
4. 证据产物已归档，另一位工程师可复现相同结果。

## 阶段 0：迁移前清单

记录当前 TypeScript 版 Pi 的使用全貌：

- 已使用的调用表面：
  - interactive（交互式）
  - print text/json
  - RPC
  - SDK 嵌入
- 自动化所依赖的 CLI 标志/子命令
- 生产环境中使用的提供方/模型
- 会话持久化预期（恢复行为、会话目录使用）
- 扩展使用情况（工具、命令、能力提示）

最小采集模板：

```text
Current pi version:
Execution surfaces in use:
Required flags/subcommands:
Provider + model matrix:
Env vars used:
Extension dependencies:
Session storage expectations:
```

## 阶段 1：安装与命令策略

选用以下发布方案之一：

1. 规范替换（推荐）：Rust 版 Pi 成为 `pi`，旧版保留为 `legacy-pi`。
2. 并排金丝雀：保留 TypeScript 版 `pi`，将 Rust 版安装为 `pi-rust`。

验证命令：

```bash
command -v pi
pi --version
pi --help >/dev/null

# If side-by-side migration is used
command -v legacy-pi || true
command -v pi-rust || true
```

## 阶段 2：配置与凭据迁移

审慎迁移设置与密钥；不要依赖隐式默认值。

### 2.1 设置

检查并协调：

- `~/.pi/agent/settings.json`
- 项目级 `.pi/settings.json`

需关注的一致性敏感领域：

- 默认提供方/模型/思考级别
- 队列模式（`steeringMode`、`followUpMode`）
- 压缩/重试参数
- 扩展策略与修复策略
- 终端/图像行为

### 2.2 凭据

验证工作流所需的所有提供方凭据（例如 `ANTHROPIC_API_KEY`、`OPENAI_API_KEY`、`GOOGLE_API_KEY`、`AZURE_OPENAI_API_KEY`、`COHERE_API_KEY`，以及任意 OpenAI 兼容提供方的密钥）。

参考：

- `docs/provider-auth-crosswalk.json`
- `docs/provider-auth-troubleshooting.md`

## 阶段 3：逐表面兼容性验证

仅运行与你的集成范围相关的检查。

### 3.1 CLI 与交互式

```bash
pi --list-models >/dev/null
pi config >/dev/null
pi --model claude-sonnet-4-20250514 -p "ping"
```

验证：

- 预期标志可正常解析，
- 预期子命令存在（`install`、`remove`、`update`、`list`、`config`），
- 团队使用的交互式斜杠命令存在且可用。

### 3.2 Print 与 JSON 模式

```bash
printf 'Hello\n' | pi -p
printf 'Hello\n' | pi --mode json
```

验证：

- stdout 帧与退出码符合预期，
- JSON 事件信封可在现有工具链中解析，
- 事件顺序或字段名未导致下游解析器中断。

### 3.3 RPC 模式

冒烟检查行分隔 JSON 协议：

```bash
pi --mode rpc
```

然后至少发送：

- `prompt`
- `get_state`
- `follow_up`
- `abort`
- `compact`（若客户端使用）

验证：

- 命令处理语义，
- 事件顺序一致性，
- 若客户端依赖则包括工具与扩展 UI 事件。

参考：

- `docs/rpc.md`

### 3.4 SDK（仅嵌入式集成）

若以库表面嵌入 Pi，请在以下位置运行 SDK 迁移检查：

- `docs/sdk.md`
- `docs/dropin-sdk-contract.json`

除非你的使用场景通过这些检查，否则不要声称 SDK 具备 drop-in 兼容性。

## 阶段 4：会话与持久化验证

针对实际会话工作流验证行为：

```bash
pi --continue
pi --session <path-to-known-session>
```

检查：

- 恢复时选中预期的项目会话，
- 消息历史与分支语义得以保留，
- 会话索引行为在你的使用场景下可接受。

参考：

- `docs/session.md`
- `docs/tree.md`

## 阶段 5：扩展与工具链验证（如适用）

若依赖扩展，请验证：

- 扩展加载/发现，
- 能力提示行为，
- 所需的宿主调用（`tool`/`http`/`session`/`ui`），
- 部署模式下的策略行为（`safe`/`balanced`/`permissive`）。

参考：

- `EXTENSIONS.md`
- `docs/extension-architecture.md`
- `docs/capability-prompts.md`

## 阶段 6：CI 证据与通过/不通过门禁

在提升至生产环境前，捕获并归档：

- 迁移检查的命令记录，
- 来自 CI 运行的机器可读测试/日志产物，
- 针对每个所需表面的明确通过/失败结论，
- 未解决的一致性风险（如有）及其负责人与缓解措施。

推荐的门禁策略：

- 若任一所需表面失败，则阻塞发布。
- 若存在影响你工作流的未解决 `critical` 一致性差距，则阻塞发布。
- 除非 `docs/evidence/dropin-certification-verdict.json` 存在且报告 `overall_verdict = CERTIFIED`，否则阻塞严格的 drop-in 替代声明。
- 要求引用用于决策的产物集合进行签收。

## 回滚计划

若金丝雀或生产环境中出现兼容性失败：

1. 将命令别名切回旧版（`legacy-pi` 或 TypeScript 版 `pi`）。
2. 恢复先前的配置快照。
3. 记录失败的命令/事件记录。
4. 将失败映射到一致性差距条目（或新建一条）后再重试迁移。

## 快速检查清单

```text
[ ] Baseline/gap/certification artifacts reviewed (including drop-in verdict)
[ ] Required surfaces identified
[ ] Install strategy selected (canonical vs canary)
[ ] Config + credential migration completed
[ ] CLI/print/JSON/RPC/SDK checks run as applicable
[ ] Session behavior validated
[ ] Extension behavior validated (if used)
[ ] Evidence captured and archived
[ ] Go/No-Go decision documented with rollback path
```

## 相关参考

- `docs/dropin-upstream-baseline.json`
- `docs/evidence/dropin-feature-inventory-matrix.json`
- `docs/evidence/dropin-parity-gap-ledger.json`
- `docs/contracts/dropin-certification-contract.json`
- `docs/evidence/dropin-certification-verdict.json`
- `docs/parity-certification.json`
- `docs/rpc.md`
- `docs/session.md`
- `docs/sdk.md`
- `docs/providers.md`
