# 一致性测试策略

> **目的：** 说明 pi_agent_rust 如何验证与 Pi Agent（TypeScript）的行为兼容性。

## 概述

pi_agent_rust 必须在所有可观测行为上与 TypeScript 参考实现保持一致。本文档描述用于验证该兼容性的一致性测试方法。

## 测试架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Test Layers                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐  │
│   │   Unit Tests    │   │  Conformance    │   │  Integration    │  │
│   │   (src/*.rs)    │   │  Tests          │   │  Tests          │  │
│   │                 │   │  (fixtures)     │   │  (E2E)          │  │
│   └────────┬────────┘   └────────┬────────┘   └────────┬────────┘  │
│            │                     │                     │            │
│   Tests internal        Tests observable       Tests full          │
│   logic in isolation    behavior vs fixtures   agent workflow      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## 测试分类

### 1. 单元测试（`cargo test --lib`）

位置：`src/*.rs` 内联 `#[cfg(test)]` 模块

**覆盖范围：**
- 消息类型序列化/反序列化
- SSE 解析器边界情况
- 截断算法
- 路径解析
- 提供方消息转换
- 包管理器源解析/标识 + 设置更新
- 技能加载器 + 提示模板展开

**数量：** 35+ 项测试

### 2. 一致性测试（基于夹具）

位置：`tests/conformance/`

**目的：** 验证工具行为与 TypeScript 参考实现一致。

**夹具格式：**
```json
{
  "version": "1.0",
  "tool": "read",
  "description": "Conformance tests for the read tool",
  "cases": [
    {
      "name": "read_simple_file",
      "setup": [
        {"type": "create_file", "path": "test.txt", "content": "hello"}
      ],
      "input": {"path": "test.txt"},
      "expected": {
        "content_exact": "hello",
        "details_none": true
      }
    }
  ]
}
```

**夹具文件：**
| 工具 | 文件 | 用例数 |
|------|------|-------|
| read | `read_tool.json` | 5+ |
| bash | `bash_tool.json` | 10+ |
| edit | `edit_tool.json` | 8+ |
| write | `write_tool.json` | 6+ |
| grep | `grep_tool.json` | 8+ |
| find | `find_tool.json` | 5+ |
| ls | `ls_tool.json` | 5+ |
| truncation | `truncation.json` | 10+ |

### 3. 集成测试

位置：`tests/*.rs`

**目的：** 对智能体工作流进行端到端测试。

**覆盖范围（当前）：**
- `tests/rpc_mode.rs`：RPC 协议自检（get_state、prompt 流式事件、get_session_stats）
- `tests/e2e_cli.rs`：无头 CLI 冒烟测试（print 模式、选择路径）
- `tests/provider_streaming.rs`：基于 VCR 的提供方流式回放（Anthropic/OpenAI/Gemini/Azure）
- `tests/compaction.rs`：使用脚本化提供方的压缩引擎行为

**计划中：**
- 基于夹具的 RPC 一致性工具，对比 Rust RPC 响应/事件与 TypeScript 参考实现（`legacy_pi_mono_code/pi-mono/packages/coding-agent/docs/rpc.md`）。

### 4. 扩展一致性（差异预言机）

位置：`tests/ext_conformance_diff.rs` + `tests/ext_conformance/`

**目的：** 通过在 TS 预言机（Bun + jiti）和 Rust QuickJS 运行时中运行**同一**扩展并比较注册快照，来验证扩展运行时行为（注册/事件/宿主调用）与 TypeScript 参考实现的一致性。

**结果（2026-02-05）：**

| 语料库 | 通过 | 总数 | 通过率 | 备注 |
|--------|--------|-------|------|-------|
| Official | 60 | 60 | 100% | 全部通过，在 CI 中运行 |
| Built-in | 4 | 4 | 100% | Pi-mono 内置扩展（diff、files、prompt-url-widget、redraws） |
| Community | 53 | 58 | 91.4% | 53/53 可测通过；5 个 TS 预言机环境失败 |
| npm | 47 | 63 | 74.6% | 16 个失败：13 个缺失 npm 依赖，3 个环境问题 |
| Third-party | 19 | 23 | 82.6% | 19/19 可测通过；4 个已知不可修复已跳过 |

**社区 TS 预言机失败（环境问题，非 Rust 缺陷）：**
- `nicobailon-interactive-shell`：需要原生 `pty.node` 模块
- `nicobailon-interview-tool`：缺失 `form/index.html` 文件
- `qualisero-background-notify`：缺失 `../../shared` 模块
- `qualisero-pi-agent-scip`：缺失 `./dist/extension.js`
- `qualisero-safe-git`：缺失 `../../shared` 模块

**第三方已知不可修复失败（外部依赖，非 Rust 缺陷）：**
- `kcosr`：readFileSync 相邻 `.md` 文件（VFS 仅内存实现）
- `marckrenn`：导入 `@marckrenn/pi-sub-shared`（私有 npm 包）
- `ogulcancelik`：readFileSync 相邻 `.html` 文件（VFS 仅内存实现）
- `qualisero`：导入 `@sourcegraph/scip-typescript`（外部 npm 包）

**内置 pi-mono 扩展（source_tier `built-in-pi-mono`）：**
- `diff`：斜杠命令 `/diff` 在 TUI 中展示 git diff（使用 `pi.exec`、`ctx.ui.custom`）
- `files`：斜杠命令 `/files` 列出会话文件操作（使用 `ctx.sessionManager`、`ctx.ui.custom`、`pi.exec`）
- `prompt-url-widget`：用于 PR/issue 链接的微件，带 `gh` 元数据（使用 `pi.on`、`pi.exec`、`ctx.ui.setWidget`、`pi.getSessionName`、`pi.setSessionName`）
- `redraws`：斜杠命令 `/tui` 展示 TUI 重绘统计（使用 `ctx.ui.custom`、`ctx.ui.notify`）

**实现一致性的关键运行时特性：**
- 内存虚拟文件系统（`__pi_vfs`）用于 `node:fs`
- 用于 CommonJS 扩展的 CJS-to-ESM 转换垫片
- `createRequire` 解析实际内置模块
- 虚拟模块存根：`shell-quote`、`vscode-languageserver-protocol`、`@modelcontextprotocol/sdk`、`glob`、`uuid`、`diff`、`just-bash`、`bunfig`、`dotenv`
- `registerCommand` 同时接受 `spec.handler` 与 `spec.fn`（PiCommand 兼容）
- QuickJS 的全局 `URL` polyfill
- 完整的 node polyfill：`fs`、`path`、`os`、`crypto`、`url`、`process`、`buffer`、`child_process`

当前构建块：
- 差异测试运行器（`tests/ext_conformance_diff.rs`）
- TS 预言机工具链（`tests/ext_conformance/ts_harness/run_extension.ts`）
- 供应商制品（`tests/ext_conformance/artifacts/*`）
- 确定性 PiJS 调度器一致性（`tests/event_loop_conformance.rs`）

### 4A. 扩展一致性矩阵 + 测试计划（bd-2kyq）

本节将**扩展分类**（见 `EXTENSIONS.md` §1B）转化为具体的一致性矩阵与测试计划。目标是确保**每一种扩展形态**都有**明确、可测试的通过/失败标准**与**夹具覆盖**。

#### 一致性矩阵（形态 × 能力 × 预期行为）

| 扩展形态 | 入口/配置 | 所需能力 / I/O | 预期行为（通过/失败） | 覆盖率（当前 / 计划） |
|---|---|---|---|---|
| **PiJS (JS/TS)** | `extension.json`（`pi.ext.manifest.v1`）或包清单；入口 `.ts/.js` | `tool`（→ `read/write/exec`）、`http`、`session`、`ui`、`log` | **通过** 条件：注册一致（工具/命令/标志/快捷键/提供方）；派生能力与宿主调用方法匹配（见 `EXTENSIONS.md` §3.2A）；按调度器契约确定性事件排序；固定规约下模拟输出确定；错误映射到分类（`timeout/denied/io/invalid_request/internal`）。 | **当前：** `tests/e2e_extension_registration.rs`、`tests/extensions_registration.rs`、`tests/ext_conformance.rs`、`tests/event_loop_conformance.rs`、`tests/ext_conformance/event_payloads/event_payloads.json`、`tests/ext_conformance/mock_specs/*`、`tests/ext_conformance_fixture_schema.rs`。**计划：** 差异化 TS↔Rust 运行器（`bd-21dv`）。 |
| **WASM 组件** | `extension.json` 含 `runtime="wasm"`；入口 `.wasm` 组件 | WIT 宿主调用 → 与 PiJS 相同的能力集合 | **通过** 条件：注册 + 宿主调用行为符合 PiJS 契约；能力推导与 JS 完全一致；日志确定；错误分类一致。 | **计划：** WASM 宿主一致性 + 对等套件（`bd-nom`、`bd-320`）。 |
| **MCP Server** | MCP 配置或 CLI 参数（stdio/http/sse） | MCP 协议（工具列表 + 工具调用/响应）；策略门控连接器 | **通过** 条件：工具 schema 可发现；工具调用在确定性模拟下执行；策略拒绝以 MCP 错误形式呈现；超时得到处理。 | **当前：** `tests/ext_conformance_scenarios.rs` 中的场景工具链 + 夹具 `tests/ext_conformance/fixtures/minimal_mcp.json`。 |
| **技能包** | `SKILL.md` + 资源 | 仅文件加载（无宿主调用） | **通过** 条件：frontmatter 合法；名称/描述解析正确；注入到系统提示；技能解析优先级正确。 | **当前：** `tests/resource_loader.rs`、`tests/e2e_cli.rs`（技能发现路径）。 |
| **提示模板** | `.md` 提示文件（可选 frontmatter） | 仅文件加载 | **通过** 条件：模板解析成功；参数确定性替换；`/template` 调用正确展开。 | **当前：** `tests/resource_loader.rs`、`tests/e2e_cli.rs`（模板路径）。 |
| **主题** | `.json` 主题文件 | 仅文件加载 | **通过** 条件：JSON schema 合法；主题解析/加载成功；TUI 应用无 panic。 | **当前：** `tests/tui_snapshot.rs` + 主题加载器覆盖。 |
| **包源** | 列出资源的包清单 | 取决于所含资源 | **通过** 条件：资源发现解析正确；冲突确定性解决；包优先级得到遵守。 | **当前：** `tests/package_manager.rs`、`tests/resource_loader.rs`、`tests/e2e_cli.rs`（包流程）。 |

#### 测试计划（夹具 → 工具链 → 断言）

1. **夹具 schema**
   - 校验事件载荷夹具：`tests/ext_conformance/event_payloads/event_payloads.json`
   - 校验模拟规约：`tests/mock_spec_schema.rs` + `tests/mock_spec_validation.rs`

2. **注册对等性**
   - Rust 运行时：`tests/extensions_registration.rs` + `tests/e2e_extension_registration.rs`
   - 输出：工具/命令/标志/快捷键/提供方必须与预期快照一致

3. **事件一致性**
   - 使用 `tests/ext_conformance/event_payloads/event_payloads.json` 驱动事件钩子
   - 校验调度/确定性：`tests/event_loop_conformance.rs`

4. **宿主调用 + 能力映射**
   - 使用模拟规约演练 `tool_call` / `tool_result` / `pi.http` / `pi.exec`
   - 断言派生能力符合分类（见 `EXTENSIONS.md` §3.2A）

5. **差异化 TS ↔ Rust（预言机模式）**
   - TS 工具链：`tests/ext_conformance/ts_harness/run_extension.ts`
   - Rust 工具链：`tests/ext_conformance.rs` + 一致性比较器
   - 计划中的运行器：`bd-21dv`（逐扩展对比 + 报告）

6. **资源包**
   - 技能/提示/主题/包：`tests/resource_loader.rs` + `tests/e2e_cli.rs`

7. **通过/失败标准汇总**
   - **通过** = 注册对等 + 确定性输出 + 错误分类合规
   - **失败** = 注册、能力推导或归一化输出差异中任一不匹配
   - **跳过** = 不支持的能力/形态（必须附带理由 + 跟踪 bead）

### 扩展日志（JSONL）

所有扩展相关日志必须符合 **ext.log.v1** schema（见 `EXTENSIONS.md`）。一致性工具链按场景记录 JSONL 日志：

- **工具链输出：** `target/ext_conformance/logs/<scenario_id>.jsonl`
- **捕获输出：** `tests/ext_conformance/capture/<ext>/<scenario>/extension.log.jsonl`

**确定性差异归一化：**
- 将 `ts`、`pid`、`host`、`run_id`、`session_id`、`artifact_id`、`trace_id`、`span_id` 替换为占位符。
- 将绝对路径归一化为 `<cwd>/...`。

**确定性运行时控制（TS 预言机 + Rust PiJS）：**
- 已打补丁的全局对象：`Date`/`Date.now`、`Math.random`、`process.cwd`、`process.env.HOME`、`pi.time.nowMs`。
- 环境变量：`PI_DETERMINISTIC_TIME_MS`、`PI_DETERMINISTIC_TIME_STEP_MS`、`PI_DETERMINISTIC_RANDOM`、`PI_DETERMINISTIC_RANDOM_SEED`、`PI_DETERMINISTIC_CWD`、`PI_DETERMINISTIC_HOME`。

**CI 消费：**
- 将 `target/ext_conformance/logs/**` 作为 CI 制品归档。
- 差异应按 `event` 与 `correlation` ID 分组以加速分流。

### NPM Registry 一致性（bd-3dd7）

大多数 npm 扩展为 **tier 3+**，因此默认 `#[ignore]`。要尝试全部 npm registry 扩展，需包含被忽略的测试：

```bash
CARGO_TARGET_DIR=/tmp/pi_target cargo test --test ext_conformance_generated ext_npm_ -- --include-ignored
```

**快照（2026-02-05）：**
- 尝试的 npm 扩展：63
- 通过：28
- 失败：35
- 自包含子集（`conformance_tier <= 2` 且 `has_npm_deps = false`）：14/17 通过（82.4%）

**失败汇总（每行一个失败扩展）：**

| 扩展 | 分类 | 详情 |
|---|---|---|
| `npm/aliou-pi-guardrails` | `missing_npm_dependency` | @aliou/pi-utils-settings |
| `npm/aliou-pi-linkup` | `missing_global_console` | console is not defined |
| `npm/aliou-pi-processes` | `relative_import_resolution` | ../components/processes-component |
| `npm/aliou-pi-synthetic` | `manifest_mismatch` | expected command 'synthetic:quotas' not found in actual commands: [] |
| `npm/aliou-pi-toolchain` | `missing_npm_dependency` | @aliou/sh |
| `npm/benvargas-pi-ancestor-discovery` | `missing_node_shim_export` | Could not find export 'isAbsolute' in module 'node:path' |
| `npm/imsus-pi-extension-minimax-coding-plan-mcp` | `missing_node_shim_export` | Could not find export 'readFile' in module 'node:fs' |
| `npm/juanibiapina-pi-files` | `missing_npm_dependency` | @juanibiapina/pi-extension-settings |
| `npm/lsp-pi` | `missing_npm_dependency` | vscode-languageserver-protocol/node.js |
| `npm/marckrenn-pi-sub-bar` | `missing_npm_dependency` | @marckrenn/pi-sub-shared |
| `npm/marckrenn-pi-sub-core` | `missing_npm_dependency` | @marckrenn/pi-sub-shared |
| `npm/permission-pi` | `missing_npm_dependency` | shell-quote |
| `npm/pi-agentic-compaction` | `missing_npm_dependency` | just-bash |
| `npm/pi-amplike` | `manifest_mismatch` | manifest says it registers tools, but no tool defs were captured |
| `npm/pi-bash-confirm` | `manifest_mismatch` | expected command 'demo-bash-confirm' not found in actual commands: ["bash-confirm"] |
| `npm/pi-brave-search` | `missing_npm_dependency` | @mozilla/readability |
| `npm/pi-ghostty-theme-sync` | `missing_node_shim_export` | Could not find export 'createHash' in module 'node:crypto' |
| `npm/pi-mermaid` | `missing_npm_dependency` | beautiful-mermaid |
| `npm/pi-messenger` | `missing_node_shim_export` | Could not find export 'isAbsolute' in module 'node:path' |
| `npm/pi-multicodex` | `missing_virtual_module_export` | Could not find export 'getApiProvider' in module '@mariozechner/pi-ai' |
| `npm/pi-repoprompt-mcp` | `missing_npm_dependency` | @modelcontextprotocol/sdk |
| `npm/pi-screenshots-picker` | `missing_npm_dependency` | glob |
| `npm/pi-search-agent` | `missing_npm_dependency` | dotenv |
| `npm/pi-session-ask` | `runtime_error` | not a function |
| `npm/pi-shadow-git` | `missing_node_shim_export` | Could not find export 'isAbsolute' in module 'node:path' |
| `npm/pi-super-curl` | `missing_npm_dependency` | uuid |
| `npm/pi-telemetry-otel` | `missing_npm_dependency` | @opentelemetry/api |
| `npm/pi-wakatime` | `missing_node_builtin` | node:stream |
| `npm/pi-watch` | `missing_npm_dependency` | chokidar |
| `npm/pi-web-access` | `missing_npm_dependency` | @mozilla/readability |
| `npm/ralph-loop-pi` | `missing_virtual_module_export` | Could not find export 'AssistantMessageComponent' in module '@mariozechner/pi-coding-agent' |
| `npm/vaayne-agent-kit` | `missing_npm_dependency` | @modelcontextprotocol/sdk/client/index.js |
| `npm/vaayne-pi-mcp` | `missing_npm_dependency` | @modelcontextprotocol/sdk/client/index.js |
| `npm/vaayne-pi-web-tools` | `missing_npm_dependency` | jsdom |
| `npm/zenobius-pi-dcp` | `missing_npm_dependency` | bunfig |

---

## 测试日志（JSONL + 制品索引）

为使 E2E 与集成测试可审计、可 diff，测试会发出**结构化 JSONL 日志**与 **JSONL 制品索引**。这些用于 CI 制品捕获与确定性 diff，并与归一化夹具并列使用。

### 日志 Schema：`pi.test.log.v1`

每条日志为一行一个 JSON 对象：

```json
{
  "schema": "pi.test.log.v1",
  "type": "log",
  "test": "e2e_cli_help_flag",
  "seq": 1,
  "ts": "2026-02-03T03:01:02.123Z",
  "t_ms": 123,
  "level": "info",
  "category": "setup",
  "message": "Created test directory",
  "context": {
    "path": "/tmp/pi-test-123/workspace",
    "size": "42 bytes"
  }
}
```

**字段说明：**
- `ts` 为 ISO-8601 UTC；`t_ms` 为相对工具链启动的相对时间。
- `test` 可选；存在时为单个字符串。
- `context` 为扁平字符串映射（敏感键已脱敏）。

### 制品索引 Schema：`pi.test.artifact.v1`

每条制品为一行一个 JSON 对象：

```json
{
  "schema": "pi.test.artifact.v1",
  "type": "artifact",
  "test": "e2e_cli_help_flag",
  "seq": 1,
  "ts": "2026-02-03T03:01:05.000Z",
  "t_ms": 3000,
  "name": "stdout.txt",
  "path": "/tmp/pi-test-123/stdout.txt",
  "size_bytes": 2048,
  "sha256": "sha256:deadbeef..."
}
```

### 归一化（确定性 Diff）

归一化的 JSONL 将非确定性值替换为稳定占位符，使 diff 稳定：
- `ts` → `<TIMESTAMP>`
- `t_ms` → `0`
- 绝对项目路径 → `<PROJECT_ROOT>/...`
- 临时/测试路径 → `<TEST_ROOT>/...`
- 字符串中的 UUID/run ID → `<UUID>` / `<RUN_ID>`
- URL 中的本地端口 → `<PORT>`

归一化输出与原始日志并列写入，后缀为 `.normalized.jsonl`。


## 夹具 Schema

### 测试用例字段

| 字段 | 类型 | 必填 | 说明 |
|-------|------|----------|-------------|
| `name` | string | 是 | 唯一测试标识 |
| `description` | string | 否 | 人类可读描述 |
| `setup` | array | 否 | 初始化测试环境的步骤 |
| `input` | object | 是 | 工具输入参数 |
| `expected` | object | 是 | 预期结果 |
| `expect_error` | bool | 否 | 测试是否应失败 |
| `error_contains` | string | 否 | 预期错误子串 |
| `tags` | array | 否 | 用于过滤的分类 |

### 前置步骤

| 类型 | 字段 | 说明 |
|------|--------|-------------|
| `create_file` | `path`、`content` | 创建带内容的文件 |
| `create_dir` | `path` | 创建目录 |
| `run_command` | `command` | 执行 shell 命令 |

### 预期结果

| 字段 | 类型 | 说明 |
|-------|------|-------------|
| `content_exact` | string | 内容必须精确匹配 |
| `content_contains` | array | 内容必须包含全部子串 |
| `content_not_contains` | array | 内容必须不包含任一子串 |
| `content_regex` | string | 内容必须匹配正则 |
| `details` | object | details 必须包含指定键（可选校验值） |
| `details_exact` | object | details 必须精确匹配 |
| `details_none` | bool | details 必须为 None |

---

## 参考捕获流程

### 阶段 1：手动夹具创建（当前）

夹具创建方式：
1. 使用特定输入运行 TypeScript 工具
2. 捕获输出与元数据
3. 在 JSON 中编码预期行为

### 阶段 2：自动化捕获（计划中）

未来通过 TypeScript 捕获工具链自动化：

```bash
# 运行 TypeScript 参考实现并捕获输出
cd pi-mono
node capture-fixtures.js --tool read --output fixtures/read_tool.json

# 针对相同夹具运行 Rust 实现
cd ../pi_agent_rust
cargo test --test conformance_fixtures
```

---

## 运行一致性测试

### 全部测试
```bash
cargo test
```

### 仅库测试
```bash
cargo test --lib
```

### 仅一致性测试
```bash
cargo test --test tools_conformance
cargo test --test conformance_fixtures
```

### 带输出
```bash
cargo test -- --nocapture
```

### 指定工具
```bash
cargo test read_tool
cargo test bash_tool
```

---

## 添加新的一致性测试

### 1. 针对已有工具

向对应的 `tests/conformance/fixtures/<tool>_tool.json` 添加用例：

```json
{
  "name": "new_test_case",
  "description": "Test some edge case",
  "setup": [...],
  "input": {...},
  "expected": {...}
}
```

### 2. 针对新工具

1. 创建夹具文件：`tests/conformance/fixtures/<tool>_tool.json`
2. 向 `tests/tools_conformance.rs` 添加测试模块
3. 为该工具实现夹具运行器

### 3. 对照 TypeScript 验证

添加夹具前，先验证预期行为：

```bash
# 在 pi-mono 中
echo '{"path": "test.txt"}' | node -e "
  const tool = require('./tools/read');
  process.stdin.on('data', async (d) => {
    const result = await tool.execute(JSON.parse(d));
    console.log(JSON.stringify(result, null, 2));
  });
"
```

---

## 行为契约

### 工具输出结构

所有工具返回：
```rust
struct ToolResult {
    content: Vec<ContentBlock>,  // 主要输出
    details: Option<Value>,      // 元数据（截断信息等）
    is_error: bool,              // 错误标志
    error_type: Option<String>,  // 错误分类
}
```

### 截断行为

| 常量 | 值 | 使用方 |
|----------|-------|---------|
| `DEFAULT_MAX_LINES` | 2000 | read、bash、grep |
| `DEFAULT_MAX_BYTES` | 50KB | read、bash、grep、find、ls |
| `GREP_MAX_LINE_LENGTH` | 500 | grep |

截断消息格式：
```
[N more lines in file. Use offset=M to continue.]
```

### 路径解析

1. 绝对路径按原样使用
2. `~` 展开为家目录
3. 相对路径基于工作目录解析
4. 读取时跟随符号链接，写入时不跟随

### 错误处理

工具应对以下情况返回错误（而非 panic）：
- 文件未找到
- 权限被拒绝
- 非法路径
- 超时
- 非法输入参数

---

## 测试失败分流

### 常见原因

| 症状 | 可能原因 | 修复 |
|---------|--------------|-----|
| 内容不匹配 | 换行符处理不同 | 检查 `\n` vs `\r\n` |
| 详情不匹配 | 额外/缺失元数据 | 更新夹具或代码 |
| 超时 | 异步处理差异 | 检查 spawn/wait 逻辑 |
| 顺序不匹配 | 非确定性输出 | 对比前排序 |

### 调试

```bash
# 带调试输出运行指定测试
RUST_LOG=debug cargo test test_name -- --nocapture

# 手动对比输出
cargo run -- -p 'read test.txt' > rust_output.txt
node pi-mono/cli.js -p 'read test.txt' > ts_output.txt
diff rust_output.txt ts_output.txt
```

---

## 覆盖率目标

| 分类 | 目标 | 当前 |
|----------|--------|---------|
| 核心类型 | 100% | ~95% |
| 工具 | 100% | ~80% |
| 提供方 | 流式路径 | ~70% |
| 会话 | JSONL 格式 | ~60% |
| CLI | 参数解析 | ~40% |

---

## 未来工作

1. **TypeScript 参考工具链**：从 pi-mono 自动生成夹具
2. **会话格式测试**：JSONL 兼容性验证
3. **CLI 参数测试**：标志解析一致性
4. **流式测试**：SSE 事件序列校验
5. **性能基准**：延迟与吞吐对比

---

## 相关文档

- [FEATURE_PARITY.md](FEATURE_PARITY.md)：实现状态跟踪
- [README.md](README.md)：项目概览
- [AGENTS.md](AGENTS.md)：智能体指令
