# RPC 协议（RPC Protocol）

> 本文为英文原文的中文翻译，源文件：`docs/rpc.md`

Pi 支持无头 RPC 模式，用于与 IDE 及其他工具集成。

## 用法

以 RPC 模式启动 Pi：

```bash
pi --mode rpc
```

通信通过 **JSON Lines** 经由 stdin/stdout 进行。每行必须是合法的 JSON 对象。

## 消息格式

### 请求（Request）

```json
{
  "id": "req-1",
  "type": "command_name",
  "param": "value"
}
```

### 响应（Response）

```json
{
  "id": "req-1",
  "type": "response",
  "command": "command_name",
  "success": true,
  "data": { ... },
  "error": "Error message if success is false"
}
```

### 事件（Events，服务端推送）

```json
{
  "type": "event_name",
  "data": "..."
}
```

## 命令

### 对话

- **prompt**：发送用户消息。
  - 参数：`message`（字符串）、`images`（可选数组）、`streamingBehavior`（"steer" 或 "follow-up"）。
- **steer**：中断当前生成并转向。
  - 参数：`message`。
- **follow_up**：将消息排队至当前轮次之后。
  - 参数：`message`。
- **abort**：停止生成。

### 会话（Session）

- **new_session**：开始全新会话。
  - 参数：`parentSession`（可选路径）。
- **switch_session**：加载会话文件。
  - 参数：`sessionPath`。
- **set_session_name**：重命名会话。
  - 参数：`name`。
- **export_html**：导出对话。
  - 参数：`outputPath`。
- **compact**：触发上下文压缩。
  - 参数：`customInstructions`（可选）、`reserveTokens`（可选）、`keepRecentTokens`（可选）。
- **fork**：从某条消息分叉。
  - 参数：`entryId`。

### 状态与配置

- **get_state**：获取当前模型、设置、token 使用情况。
- **get_messages**：获取对话历史。
- **get_available_models**：列出可用模型。
- **set_model**：切换模型。
  - 参数：`provider`、`modelId`。
- **set_thinking_level**：设置思考预算。
  - 参数：`level`（"off"、"low" 等）。
- **set_steering_mode**："one-at-a-time" 或 "all"。
- **set_follow_up_mode**："one-at-a-time" 或 "all"。

### 扩展 UI

- **extension_ui_response**：回复待处理的扩展 UI 请求。
  - 参数：`requestId`（首选）或旧别名 `id`，另加以下之一：
    - `confirmed`（布尔值）用于 `confirm`
    - `value`（字符串/布尔值，取决于方法）用于 `select`/`input`/`editor`
    - `cancelled`（`true`）表示取消

## 事件

- `agent_start`：智能体开始工作。
- `text_delta`：助手文本输出片段。
- `thinking_delta`：助手思考输出片段。
- `tool_execution_start`：工具执行开始。
- `tool_execution_update`：流式工具输出。
- `tool_execution_end`：工具执行结束。
- `extension_ui_request`：扩展请求宿主 UI 交互（confirm/select/input/editor/notify 等）。
- `agent_end`：轮次完成。
- `auto_retry_start` / `auto_retry_end`：瞬时错误重试。
- `auto_compaction_start` / `auto_compaction_end`：自动压缩状态。
- `extension_error`：扩展事件分发/运行时错误。
