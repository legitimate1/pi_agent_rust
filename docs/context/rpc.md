# Fork RPC 接口

> 本文档描述 `custom-next` 当前实际提供的 RPC 接口。
>
> 上游协议文档 `docs/rpc.md` 保持不变；本文只记录 Fork 当前源码已经提供、或相对于上游文档需要补充的接口事实。当前行为以 `src/rpc.rs` 和 `src/agent.rs` 为准。

## 1. 启动与传输

启动 RPC 模式：

```bash
pi --mode rpc
```

RPC 使用 **JSON Lines（JSONL）** 通过标准输入和标准输出通信：

- stdin：客户端每行发送一个 JSON 对象；
- stdout：服务端每行返回一个 JSON 对象；
- 请求必须包含字符串类型的 `type` 字段；
- 建议每个请求包含字符串类型的 `id`，用于关联响应；
- 服务端响应和异步事件共享同一 stdout 通道。

RPC 不是 HTTP、WebSocket 或独立 TCP 服务。

## 2. 请求和响应

### 请求

```json
{
  "id": "req-1",
  "type": "get_state"
}
```

### 成功响应

```json
{
  "type": "response",
  "id": "req-1",
  "command": "get_state",
  "success": true,
  "data": {}
}
```

没有返回数据时，响应可能省略 `data`：

```json
{
  "type": "response",
  "id": "req-2",
  "command": "abort",
  "success": true
}
```

### 失败响应

```json
{
  "type": "response",
  "id": "req-3",
  "command": "set_model",
  "success": false,
  "error": "Model not found: provider/model"
}
```

部分运行时错误还会包含 `errorHints`。请求 JSON 无法解析、缺少 `type` 或参数非法时，服务端会返回失败响应，而不是静默丢弃该请求。

## 3. 命令总览

当前 RPC 分发器提供以下内置命令：

### 3.1 对话和 Agent 控制

| 命令 | 参数 | 说明 |
|---|---|---|
| `prompt` | `message`；可选 `images`、`streamingBehavior` | 发送用户消息并启动 Agent；空闲时立即执行，流式期间可按策略排队 |
| `steer` | `message` | 当前生成过程中转向新消息 |
| `follow_up` | `message` | 将消息排队到当前回合之后 |
| `abort` | 无 | 中止当前 Agent 生成 |
| `retry` | 无 | 重新执行最近一次用户回合 |
| `abort_retry` | 无 | 中止自动重试流程 |
| `set_plan_mode` | `mode` | 设置计划模式；通常使用 `on` 或 `off` |
| `approve_plan` | 无 | 批准当前已提交计划 |
| `reject_plan` | 无 | 拒绝当前已提交计划 |

`prompt` 示例：

```json
{
  "id": "prompt-1",
  "type": "prompt",
  "message": "分析这个项目的 RPC 接口",
  "streamingBehavior": "steer"
}
```

`streamingBehavior` 支持 `steer` 和 `follow-up`。当 Agent 正在流式执行时，`prompt` 必须提供该字段；扩展命令不能在流式期间执行。

`prompt.images` 可用于发送图片内容，但具体图片对象格式应与当前模型输入能力匹配；不支持图片的模型会返回参数或模型能力错误。

### 3.2 状态、消息和模型

| 命令 | 参数 | 说明 |
|---|---|---|
| `get_state` | 无 | 获取当前模型、会话、设置、token 使用和运行状态 |
| `get_session_stats` | 无 | 获取当前会话统计信息 |
| `get_messages` | 无 | 获取当前会话当前路径中的消息 |
| `get_last_assistant_text` | 无 | 获取最近一条 Assistant 文本 |
| `get_available_models` | 无 | 获取可用模型列表 |
| `get_commands` | 无 | 获取资源加载器提供的命令列表 |
| `set_model` | `provider`、`modelId` | 切换当前模型 |
| `cycle_model` | 无 | 按可用模型顺序切换模型 |
| `set_thinking_level` | `level` | 设置思考级别 |
| `cycle_thinking_level` | 无 | 在当前模型支持的思考级别之间循环 |
| `set_steering_mode` | `mode` | 设置 steer 队列策略 |
| `set_follow_up_mode` | `mode` | 设置 follow-up 队列策略 |
| `set_auto_compaction` | `enabled` | 开关自动上下文压缩 |
| `set_auto_retry` | `enabled` | 开关自动重试 |
| `set_session_name` | `name` | 设置会话名称 |

队列模式通常为 `one-at-a-time` 或 `all`。`set_model` 的 `provider` 和 `modelId` 必须能匹配当前可用模型，且模型所需凭据必须已经配置。

### 3.3 会话和上下文管理

| 命令 | 参数 | 说明 |
|---|---|---|
| `new_session` | 可选 `parentSession` | 创建并切换到新会话 |
| `switch_session` | `sessionPath` | 加载并切换到已有会话；相对路径会限制在会话目录内 |
| `fork` | `entryId` | 从指定用户消息处分叉新会话 |
| `get_fork_messages` | 无 | 获取当前会话路径上的可用于 fork 的消息 |
| `fresh` | 无 | 重置当前会话的 provider cache 和流状态 |
| `compact` | 可选 `customInstructions`、`reserveTokens`、`keepRecentTokens` | 手动压缩当前上下文 |
| `checkpoint` | 可选 `name`、`note` | 创建会话检查点 |
| `rewind` | 可选 `name` | 回退到指定检查点 |
| `export_html` | 可选 `outputPath` | 将当前会话导出为 HTML |

示例：

```json
{
  "id": "checkpoint-1",
  "type": "checkpoint",
  "name": "before-refactor",
  "note": "重构前状态"
}
```

会话切换、fork、compact 等会改变会话状态的命令，通常要求 Agent 处于空闲状态。`new_session` 和 `switch_session` 还会触发扩展生命周期事件；扩展可以取消切换。

### 3.4 Shell 执行

| 命令 | 参数 | 说明 |
|---|---|---|
| `bash` | `command` | 执行 Bash 命令，并将结果返回到 RPC 响应 |
| `abort_bash` | 无 | 请求中止当前 Bash 命令 |

示例：

```json
{
  "id": "bash-1",
  "type": "bash",
  "command": "git status --short"
}
```

`bash` 响应数据可能包含：

- `output`
- `exitCode`
- `cancelled`
- `truncated`
- `fullOutputPath`
- `persisted`
- `persistenceStatus`

同一 RPC 会话同时只能运行一个 Bash 命令。

### 3.5 扩展 UI 和 Ask Tool

| 命令 | 参数 | 说明 |
|---|---|---|
| `extension_ui_response` | `requestId` 或 `id`、`requestGeneration`，以及 `confirmed`/`value`/`cancelled` | 回复扩展发出的 UI 请求 |
| `ask_response` | `requestId` 或 `id`，以及 `answers` 或 `dismissed` | 回复 Ask Tool 发出的提问 |

`ask_response` 只有启用 `ask` 工具时才可用。`ask_response` 示例：

```json
{
  "id": "answer-1",
  "type": "ask_response",
  "requestId": "ask-123",
  "answers": [
    {
      "questionId": "q1",
      "selected": ["Option A"]
    }
  ]
}
```

取消整个提问：

```json
{
  "id": "answer-2",
  "type": "ask_response",
  "requestId": "ask-123",
  "dismissed": true
}
```

`extension_ui_response` 必须回显匹配的 `requestGeneration`。该字段用于防止旧请求的迟到响应解析后续复用相同公开 ID 的新请求。

## 4. 命令别名

以下别名会在 RPC 层规范化为标准命令名：

| 别名 | 标准命令 |
|---|---|
| `follow-up`、`followUp`、`queue-follow-up`、`queueFollowUp` | `follow_up` |
| `get-state`、`getState` | `get_state` |
| `set-model`、`setModel` | `set_model` |
| `set-steering-mode`、`setSteeringMode` | `set_steering_mode` |
| `set-follow-up-mode`、`setFollowUpMode` | `set_follow_up_mode` |
| `set-auto-compaction`、`setAutoCompaction` | `set_auto_compaction` |
| `set-auto-retry`、`setAutoRetry` | `set_auto_retry` |
| `set-plan-mode`、`setPlanMode` | `set_plan_mode` |
| `approve-plan`、`approvePlan` | `approve_plan` |
| `reject-plan`、`rejectPlan` | `reject_plan` |

## 5. 服务端事件

服务端事件没有 `command` 字段，使用 `type` 区分。一次 `prompt` 或扩展命令通常会产生以下生命周期事件：

### Agent 生命周期

```text
agent_start
agent_end
```

### Turn 生命周期

```text
turn_start
turn_end
```

### 消息流

```text
message_start
message_update
message_end
```

`message_update` 用于 Assistant 流式输出，可能包含文本增量、思考增量或工具调用增量。

### 工具执行

```text
tool_execution_start
tool_execution_update
tool_execution_end
```

### 自动处理和故障转移

```text
auto_compaction_start
auto_compaction_end
auto_retry_start
auto_retry_end
failover_start
failover_end
provider_error
```

### 扩展和交互

```text
extension_ui_request
extension_error
ask_request
advisor_note
```

`ask_request` 事件的主要字段包括：

```json
{
  "type": "ask_request",
  "id": "ask-123",
  "questions": [
    {
      "id": "q1",
      "question": "选择一个方案",
      "header": "方案",
      "options": [
        {"label": "Option A", "description": "方案 A"}
      ],
      "recommended": true,
      "multi": false
    }
  ],
  "timeoutMs": 300000
}
```

`extension_ui_request` 的具体字段取决于扩展 UI 方法，例如 `confirm`、`select`、`input`、`editor` 或 `notify`。需要回复的请求会包含 `requestGeneration`。

## 6. 错误和并发边界

- RPC 请求按 JSONL 逐行解析；无效 JSON 会返回解析失败响应。
- 缺少 `type` 会返回解析失败响应。
- Agent 正在压缩上下文时，不能执行会推进会话的命令。
- Agent 正在流式执行时，`prompt`、`steer` 和 `follow_up` 可以按规则排队；其他会推进会话的命令通常会被拒绝，客户端应等待 `agent_end`。
- `abort`、状态查询和部分控制命令可以在 Agent 工作期间使用。
- 改变会话的命令在执行前会进行会话状态和持久化状态检查。
- `switch_session` 的相对路径不能逃逸当前会话目录。
- `extension_ui_response` 必须同时匹配活动请求的 ID 和 generation，否则会被拒绝。
- `ask_response` 返回的 `data.resolved` 为 `false` 时，表示对应请求已经超时或不再处于等待状态。

## 7. 扩展命令

除了内置命令，已注册的扩展命令也可以通过 `prompt` 调用：

```json
{
  "id": "extension-1",
  "type": "prompt",
  "message": "/my-extension-command argument"
}
```

只有以 `/` 开头且已经由扩展注册的命令才会按扩展命令执行；否则会作为普通用户消息发送给 Agent。

## 8. 当前源码入口

本文档对应的主要源码入口：

- 命令规范化：`src/rpc.rs` 中的 `normalize_command_type`
- 命令分发：`src/rpc.rs` 中的 RPC command `match`
- 响应封装：`src/rpc.rs` 中的 `response_ok`、`response_error`
- Ask Tool 事件和响应解析：`src/rpc.rs` 中的 `ask_request_rpc_event`、`rpc_parse_ask_response`
- Agent 事件序列化：`src/agent.rs` 中的 `AgentEvent`

如果本文档与当前源码不一致，应以源码和相关测试为准，并同步更新本文档；不要修改上游的 `docs/rpc.md`。
