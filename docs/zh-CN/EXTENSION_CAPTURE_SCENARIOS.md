# 扩展捕获场景套件 (bd-2qd)

本文档为 `docs/extension-sample.json` 中冻结的扩展样本集定义了一份**场景规约**。
其目标是以**确定性**、**可审计**的预期驱动捕获与一致性测试框架工作（传统 `pi-mono` → Rust `pi_agent_rust`）。

样本集的制品已归档于 `tests/ext_conformance/artifacts/<id>/`（从锁定的传统快照复制而来；参见 `docs/EXTENSION_SAMPLE.md`）。

---

## 目标

- 对于每个已采样的扩展：
  - 枚举其支持的功能类别（工具 / 斜杠命令 / 事件钩子 / 输入变换 / UI 集成 / 提供方 / 标志）
  - 为每个支持的类别定义**至少一个**确定性场景
  - 记录所需的配置/密钥（尽可能使用**已模拟**或**无环境变量**路径）

本规约有意保持**与实现无关**：它描述要运行什么、要断言什么，而不描述测试框架如何实现。

---

## 约定

### 场景 ID

所有场景均具有稳定的 ID：

```
<extension-id>/<category>/<name>
```

示例：
- `hello/tool/basic`
- `permission-gate/event_hook/dangerous_bash_blocked_no_ui`

### 模式

- **interactive（交互模式）**：`ctx.hasUI = true` 且 UI 调用可脚本化（select/confirm/custom/key 输入）
- **headless（无界面模式）**：`ctx.hasUI = false`（print / json / rpc / 非 tty 运行器）

### 确定性规则

优先选择**不依赖**以下条件的场景：
- 网络调用
- 真实的 OAuth 登录
- CI 中不保证存在的外部二进制

对于重度依赖网络/认证的扩展，需定义：
- 一个**离线**场景，用于断言正确的错误/诊断路径，和/或
- 一个 **VCR 回放**场景（已录制的 HTTP 请求，已做密钥脱敏），待该能力可用时补充。

### 标准测试工作区（推荐）

除非场景另有说明：
- 在每个场景独立的临时目录中运行（名称唯一且确定，由 scenario_id 派生）
- 设置 `TZ=UTC`
- 避免断言精确时间戳；仅断言存在性/结构
- 对于 git 场景：在临时目录内初始化一个新仓库

---

## 样本集

ID（16 个）：

- `permission-gate`
- `protected-paths`
- `todo`
- `hello`
- `antigravity-image-gen`
- `plan-mode`
- `status-line`
- `doom-overlay`
- `sandbox`
- `inline-bash`
- `dynamic-resources`
- `custom-provider-anthropic`
- `custom-provider-qwen-cli`
- `with-deps`
- `subagent`
- `git-checkpoint`

---

## 场景

### permission-gate

**来源：** `tests/ext_conformance/artifacts/permission-gate/permission-gate.ts`  
**功能类别：** event_hook（tool_call）、UI 集成（select）  
**备注：** 拦截危险的 `bash` 命令（匹配 `rm -r*`、`sudo`、`chmod/chown 777`）。

场景：

- `permission-gate/event_hook/dangerous_bash_blocked_no_ui` (headless)
  - 前置条件：
    - 启用该扩展运行
    - 确保 `ctx.hasUI = false`
  - 步骤：
    - 触发一条 `bash` 工具调用，其命令匹配危险模式（使用临时本地路径），例如 `rm -rf ./tmp-to-delete`
  - 预期：
    - 工具调用被拦截
    - 原因包含 `Dangerous command blocked (no UI for confirmation)`

- `permission-gate/ui_integration/dangerous_bash_prompt_denied` (interactive)
  - 前置条件：脚本化的 UI 选择返回 `"No"`
  - 步骤：同样的危险 `bash` 工具调用
  - 预期：
    - 被拦截
    - 原因包含 `Blocked by user`

- `permission-gate/ui_integration/dangerous_bash_prompt_allowed` (interactive)
  - 前置条件：
    - 脚本化的 UI 选择返回 `"Yes"`
    - 工作目录包含由前置条件创建的 `./tmp-to-delete`
  - 步骤：`bash` 工具调用执行 `rm -rf ./tmp-to-delete`
  - 预期：
    - 工具调用未被扩展拦截
    - 命令执行且退出码为 `0`（或等效的成功信号）
    - `./tmp-to-delete` 已不存在

---

### protected-paths

**来源：** `tests/ext_conformance/artifacts/protected-paths/protected-paths.ts`  
**功能类别：** event_hook（tool_call）、UI 集成（notify）  
**备注：** 拦截对包含 `.env`、`.git/`、`node_modules/` 路径的 `write` 和 `edit` 操作。

场景：

- `protected-paths/event_hook/allow_safe_write` (headless)
  - 步骤：工具调用 `write` 写入 `notes.txt`
  - 预期：未被拦截

- `protected-paths/event_hook/block_env_write` (headless)
  - 步骤：工具调用 `write` 写入 `.env`
  - 预期：
    - 被拦截
    - 原因包含 `Path ".env" is protected`

- `protected-paths/ui_integration/notify_on_block` (interactive)
  - 步骤：工具调用 `edit` 编辑 `node_modules/x.txt`
  - 预期：
    - 被拦截，原因包含 `is protected`
    - UI 收到一条警告通知，内容包含 `Blocked write to protected path:`

---

### hello

**来源：** `tests/ext_conformance/artifacts/hello/hello.ts`  
**功能类别：** tool  
**工具：** `hello(name: string)`

场景：

- `hello/tool/basic` (headless)
  - 步骤：
    - 以 `{ "name": "World" }` 调用工具 `hello`
  - 预期：
    - 工具结果文本包含 `Hello, World!`
    - 工具结果详情包含 `{ "greeted": "World" }`

- `hello/tool/param_validation` (headless)
  - 步骤：调用工具 `hello` 时缺失 `name` 参数
  - 预期：暴露校验错误（schema/类型错误），错误类别稳定

---

### todo

**来源：** `tests/ext_conformance/artifacts/todo/todo.ts`  
**功能类别：** tool、slash_command、event_hook（session_*）、UI 集成（custom UI）  
**工具：** `todo(action, text?, id?)`  
**命令：** `/todos`

场景：

- `todo/tool/smoke_add_list_toggle_clear` (headless)
  - 步骤：
    1. `todo { action: "add", text: "first" }`
    2. `todo { action: "add", text: "second" }`
    3. `todo { action: "list" }`
    4. `todo { action: "toggle", id: 1 }`
    5. `todo { action: "clear" }`
  - 预期：
    - add 返回 `Added todo #<n>: <text>` 并更新 details.todos/nextId
    - list 返回类似 `[ ] #1: first` 的行
    - toggle 返回 `Todo #1 completed`（或 `uncompleted`）
    - clear 返回 `Cleared <n> todos` 并重置 `nextId = 1`

- `todo/tool/error_missing_text` (headless)
  - 步骤：`todo { action: "add" }`
  - 预期：
    - 工具文本包含 `Error: text required for add`
    - details.error 包含 `text required`

- `todo/tool/error_missing_id` (headless)
  - 步骤：`todo { action: "toggle" }`
  - 预期：
    - 工具文本包含 `Error: id required for toggle`
    - details.error 包含 `id required`

- `todo/slash_command/requires_ui` (headless)
  - 步骤：执行 `/todos`
  - 预期：错误通知包含 `/todos requires interactive mode`

- `todo/ui_integration/render_list_and_close` (interactive)
  - 前置条件：
    - 已通过工具调用至少创建 1 条 todo
    - 脚本化按键 `Escape` 用于关闭 UI
  - 步骤：执行 `/todos`
  - 预期：
    - UI 浮层渲染包含 `Todos` 和 `#<id>` 的列表
    - 按 Escape 后 UI 关闭

- `todo/event_hook/session_fork_reconstructs_state` (interactive)
  - 前置条件：
    - 在主分支上创建 2 条 todos
    - 在较早的条目处（第二条 todo 之前）分叉会话
  - 步骤：
    - 在分叉分支中运行 `/todos`
  - 预期：
    - 列表反映分支历史（仅包含分叉点之前创建的 todos）

---

### inline-bash

**来源：** `tests/ext_conformance/artifacts/inline-bash/inline-bash.ts`  
**功能类别：** input_transform、event_hook（input）、UI 集成（notify）

场景：

- `inline-bash/input_transform/expand_echo` (headless)
  - 步骤：
    - 用户输入：`Value is !{echo 123}`
  - 预期：
    - 输入被变换为包含 `Value is 123`
    - 无需额外的工具调用（扩展内部执行 exec）

- `inline-bash/input_transform/preserve_bang_command` (headless)
  - 步骤：用户输入：`!echo 123`
  - 预期：动作为 `continue`（不进行内联展开）

- `inline-bash/input_transform/error_substitution` (headless)
  - 步骤：用户输入：`Bad is !{false}`
  - 预期：变换后的文本包含 `[error:` 或包含退出码说明（实现相关），但存在稳定的失败标识

- `inline-bash/ui_integration/notify_expansions` (interactive)
  - 前置条件：`ctx.hasUI = true`
  - 步骤：包含两个展开式的用户输入
  - 预期：UI 信息通知包含 `Expanded 2 inline command(s):`

---

### plan-mode

**来源：** `tests/ext_conformance/artifacts/plan-mode/index.ts`  
**功能类别：** slash_command、flags、event_hook（tool_call/context/before_agent_start/turn_end/agent_end）、UI 集成  
**命令：** `/plan`、`/todos`  
**标志：** `--plan`  
**行为：** 切换可用工具集；在 plan 模式下拦截非允许清单内的 `bash` 命令；跟踪 `[DONE:n]`。

场景：

- `plan-mode/slash_command/toggle_plan_mode` (interactive)
  - 步骤：
    1. 执行 `/plan`（启用）
    2. 执行 `/plan`（禁用）
  - 预期：
    - UI 通知包含 `Plan mode enabled`，随后包含 `Plan mode disabled`
    - 状态/小组件相应更新

- `plan-mode/event_hook/block_destructive_bash` (interactive)
  - 前置条件：已启用 plan 模式
  - 步骤：智能体尝试以 `rm -rf ./x` 发起 `bash` 工具调用
  - 预期：
    - 被拦截，原因包含 `Plan mode: command blocked (not allowlisted)`

- `plan-mode/slash_command/todos_empty` (interactive)
  - 步骤：在尚未提取任何计划时运行 `/todos`
  - 预期：UI 通知包含 `No todos. Create a plan first with /plan`

- `plan-mode/event_hook/extract_plan_and_track_done` (interactive)
  - 前置条件：已启用 plan 模式，UI 可用
  - 步骤：
    1. 智能体输出：
       ```
       Plan:
       1. Do thing A
       2. Do thing B
       ```
    2. 用户选择执行路径（实现相关的 UI）
    3. 智能体稍后回复 `[DONE:1]`
  - 预期：
    - 计划步骤被提取到 todo 列表中
    - 状态/小组件显示完成度 `1/2`

---

### status-line

**来源：** `tests/ext_conformance/artifacts/status-line/status-line.ts`  
**功能类别：** event_hook（session_start/turn_start/turn_end/session_switch）、UI 集成（setStatus）

场景：

- `status-line/ui_integration/turn_progress_status` (interactive)
  - 步骤：
    1. 启动新会话
    2. 发送一条无需工具即可完成的提示
  - 预期：
    - 会话启动时状态键 `status-demo` 被设为 `Ready`
    - 在 turn_start 时更新为包含 `Turn 1...`
    - 在 turn_end 时更新为包含 `Turn 1 complete`

---

### git-checkpoint

**来源：** `tests/ext_conformance/artifacts/git-checkpoint/git-checkpoint.ts`  
**功能类别：** event_hook（turn_start/session_before_fork/tool_result/agent_end）、UI 集成（select/notify）、外部进程（git）

场景：

- `git-checkpoint/event_hook/creates_stash_refs` (headless)
  - 前置条件：
    - 在临时目录中初始化一个 git 仓库
    - 至少创建一个提交
  - 步骤：运行触发 `turn_start` 的单轮智能体回合
  - 预期：
    - 扩展调用 `git stash create`（可通过 exec 日志或桩 exec 观测）
    - 内部检查点映射已更新（若有扩展可见的诊断信息则据此断言）

- `git-checkpoint/ui_integration/restore_on_fork_yes` (interactive)
  - 前置条件：
    - 仓库存在未提交的变更
    - 脚本化的 UI 选择为 `Yes, restore code to that point`
  - 步骤：
    1. 运行一轮以记录检查点
    2. 在该条目处分叉会话（触发 `session_before_fork`）
  - 预期：
    - 已执行 `git stash apply <ref>`
    - UI 通知包含 `Code restored to checkpoint`

- `git-checkpoint/ui_integration/restore_on_fork_no` (interactive)
  - 前置条件：脚本化选择为 `No, keep current code`
  - 预期：未执行 `stash apply`

---

### dynamic-resources

**来源：** `tests/ext_conformance/artifacts/dynamic-resources/index.ts`  
**功能类别：** event_hook（resources_discover）

场景：

- `dynamic-resources/event_hook/returns_paths` (headless)
  - 步骤：触发 `resources_discover`
  - 预期：
    - 返回的负载包含：
      - `skillPaths` 包含 `SKILL.md`
      - `promptPaths` 包含 `dynamic.md`
      - `themePaths` 包含 `dynamic.json`

- `dynamic-resources/harness/resources_loaded` (headless)
  - 前置条件：启用该扩展运行资源加载器
  - 预期：
    - 技能列表包含来自该扩展的动态技能
    - 提示模板包含 `dynamic.md`
    - 主题列表包含动态主题 JSON

---

### with-deps

**来源：** `tests/ext_conformance/artifacts/with-deps/index.ts`  
**功能类别：** tool  
**备注：** 真实运行时要求扩展目录中存在 npm 依赖。一致性测试应验证依赖解析。

场景：

- `with-deps/tool/parse_duration_valid` (headless)
  - 步骤：调用 `parse_duration { duration: "1h" }`
  - 预期：文本包含 `1h = 3600000 milliseconds`

- `with-deps/tool/parse_duration_invalid` (headless)
  - 步骤：调用 `parse_duration { duration: "not-a-duration" }`
  - 预期：
    - `isError = true`
    - 文本包含 `Invalid duration:`

---

### subagent

**来源：** `tests/ext_conformance/artifacts/subagent/index.ts`  
**功能类别：** tool、UI 集成（confirm）、外部进程（派生 `pi` 子进程）、文件系统（临时提示文件）  
**工具：** `subagent(...)`

场景（优先确定性）：

- `subagent/tool/invalid_params_mode_count` (headless)
  - 步骤：同时传入 `agent+task` 与 `tasks` 调用 `subagent`
  - 预期：
    - 工具结果文本包含 `Invalid parameters. Provide exactly one mode.`
    - 工具结果详情包含 `{ mode: "single" | ... }` 且结果为空

- `subagent/tool/unknown_agent` (headless)
  - 前置条件：确保不存在所请求名称的智能体
  - 步骤：`subagent { agent: "does-not-exist", task: "hi" }`
  - 预期：
    - 结果包含 `Unknown agent: does-not-exist`
    - 详情中 `exitCode = 1`

- `subagent/ui_integration/deny_project_agents` (interactive)
  - 前置条件：
    - 在 `.pi/agents/` 中存在项目级智能体
    - 以 `agentScope: "project"` 和 `confirmProjectAgents: true` 调用 subagent
    - 脚本化 confirm 返回 `false`
  - 预期：
    - 工具结果文本包含 `Canceled: project-local agents not approved.`

VCR 回放（未来）：

- `subagent/tool/single_smoke` (headless, VCR)
  - 步骤：运行仅执行 `ls/read` 的简单用户智能体（`scout`）
  - 预期：确定性的 JSON 模式输出被捕获并汇总

---

### antigravity-image-gen

**来源：** `tests/ext_conformance/artifacts/antigravity-image-gen/antigravity-image-gen.ts`  
**功能类别：** tool、网络/认证（OAuth）、文件系统（可选保存）  
**工具：** `generate_image(prompt, model?, aspectRatio?, save?, saveDir?)`

确定性场景：

- `antigravity-image-gen/tool/missing_credentials` (headless)
  - 前置条件：不存在 Google Antigravity 凭证（未执行 `/login`，无已存储密钥）
  - 步骤：调用 `generate_image { prompt: "a cat" }`
  - 预期：稳定的错误消息，包含 `Missing Google Antigravity OAuth credentials`

- `antigravity-image-gen/tool/save_mode_custom_without_dir` (headless)
  - 步骤：在未设置 `PI_IMAGE_SAVE_DIR` 的情况下调用 `generate_image { prompt: "a cat", save: "custom" }`
  - 预期：暴露确定性错误或 saveError（具体消息可能不同；需断言存在 `save`/`custom` 及缺失目录的指示）

VCR 回放（未来）：

- `antigravity-image-gen/tool/vcr_generate_image` (headless, VCR)
  - 前置条件：已录制的 HTTP 流式响应，其中嵌入 base64 图像；密钥已脱敏
  - 预期：
    - 工具结果包含带有 mimeType 的 `image` 内容块
    - 汇总文本包含提供方/模型 + aspectRatio

---

### doom-overlay

**来源：** `tests/ext_conformance/artifacts/doom-overlay/index.ts`  
**功能类别：** slash_command、UI 集成（overlay）、网络/文件系统（自动下载 WAD）  
**命令：** `/doom-overlay`

确定性场景：

- `doom-overlay/slash_command/requires_ui` (headless)
  - 步骤：执行 `/doom-overlay`
  - 预期：UI 通知错误包含 `DOOM requires interactive mode`

- `doom-overlay/slash_command/wad_download_failure` (interactive, offline)
  - 前置条件：无网络可用（或 WAD URL 被拦截）
  - 步骤：执行 `/doom-overlay`
  - 预期：错误通知包含 `Failed to download DOOM WAD file`

---

### sandbox

**来源：** `tests/ext_conformance/artifacts/sandbox/index.ts`  
**功能类别：** tool 覆盖（bash）、flags、slash_command、event_hook（session_start/user_bash/session_shutdown）、UI 集成（notify/setStatus）  
**标志：** `--no-sandbox`  
**命令：** `/sandbox`

确定性场景（无需外部沙箱运行时）：

- `sandbox/flags/no_sandbox_disables` (interactive)
  - 前置条件：以 `--no-sandbox` 运行
  - 步骤：启动会话
  - 预期：警告通知包含 `Sandbox disabled via --no-sandbox`

- `sandbox/slash_command/sandbox_when_disabled` (interactive)
  - 前置条件：沙箱已禁用（通过标志或配置）
  - 步骤：执行 `/sandbox`
  - 预期：通知包含 `Sandbox is disabled`

尽力而为（依赖环境）：

- `sandbox/tool/bash_still_works_without_sandbox` (headless)
  - 前置条件：沙箱已禁用
  - 步骤：运行 bash 工具 `echo ok`
  - 预期：输出包含 `ok`

VCR/CI 专用（未来）：

- `sandbox/event_hook/initializes_and_sets_status` (interactive, 需 bubblewrap/沙箱运行时)
  - 预期：状态包含 `🔒 Sandbox:` 且通知包含 `Sandbox initialized`

---

### custom-provider-anthropic

**来源：** `tests/ext_conformance/artifacts/custom-provider-anthropic/index.ts`  
**功能类别：** 提供方注册、OAuth（`/login`）、流式实现（`streamSimple`）  
**提供方：** `custom-anthropic`  
**环境变量：** `CUSTOM_ANTHROPIC_API_KEY`  
**API 标识：** `custom-anthropic-api`

场景：

- `custom-provider-anthropic/provider/models_listed` (headless)
  - 步骤：列出模型
  - 预期：包含：
    - `custom-anthropic/claude-opus-4-5`
    - `custom-anthropic/claude-sonnet-4-5`

- `custom-provider-anthropic/provider/missing_api_key_errors` (headless)
  - 前置条件：无 `CUSTOM_ANTHROPIC_API_KEY`，无 OAuth 凭证
  - 步骤：尝试以提供方 `custom-anthropic` 流式传输最小提示
  - 预期：关于缺失 API 密钥/凭证的确定性错误

VCR 回放（未来）：

- `custom-provider-anthropic/provider/vcr_streaming_smoke` (headless, VCR)
  - 前置条件：已录制的流式响应（SSE），密钥已脱敏
  - 预期：
    - 发出文本增量
    - 若启用工具，工具调用可正确往返

---

### custom-provider-qwen-cli

**来源：** `tests/ext_conformance/artifacts/custom-provider-qwen-cli/index.ts`  
**功能类别：** 提供方注册、OAuth 设备流（`/login`）、OpenAI 兼容 API  
**提供方：** `qwen-cli`  
**环境变量：** `QWEN_CLI_API_KEY`  
**API 标识：** `openai-completions`

场景：

- `custom-provider-qwen-cli/provider/models_listed` (headless)
  - 步骤：列出模型
  - 预期：包含 `qwen-cli/qwen3-coder-plus` 和 `qwen-cli/qwen3-coder-flash`

- `custom-provider-qwen-cli/provider/missing_api_key_errors` (headless)
  - 前置条件：无 `QWEN_CLI_API_KEY`，无 OAuth 凭证
  - 步骤：尝试以提供方 `qwen-cli` 流式传输提示
  - 预期：关于缺失凭证 / API 密钥的确定性错误

VCR 回放（未来）：

- `custom-provider-qwen-cli/provider/vcr_openai_compat_smoke` (headless, VCR)
  - 预期：
    - 请求使用 OpenAI 兼容的负载/标头
    - 响应解析产生稳定的智能体输出

---

### plan-mode / sandbox / doom-overlay（重度 UI 扩展）

这些扩展已在上文通过**无界面错误路径** + **交互式脚本化 UI** 场景覆盖。
测试框架应优先在 CI 中运行无界面路径，并将较重的交互/网络路径置于可选运行器之后。

---

## 待解决问题（面向测试框架实现者）

- 如何在 Rust 测试框架中以与传统行为一致的方式表示 `ctx.hasUI` / UI 脚本化？
- 对于提供方/网络场景，是否在提供方与扩展之间标准化一种 VCR 格式（SSE 录制），以便共享？
- 用于确定性断言 `pi.exec(...)` 行为的规范“命令执行记录”格式是什么（stdout/stderr/exitCode + 时序）？
