# 设置

Pi 读取 JSON 设置并按清晰的优先级规则应用。

## 位置

Pi 从（最多）两个文件加载设置：

| 位置 | 范围 |
|------|------|
| `~/.pi/agent/settings.json` | 全局（所有项目） |
| `.pi/settings.json` | 项目（当前目录） |

你可以通过 `PI_CONFIG_PATH` 完全覆盖路径（见下文）。

运行 `pi config` 打印生效路径与优先级。

## 优先级（最高 → 最低）

1. CLI 标志
2. 环境变量
3. 项目设置（`.pi/settings.json`）
4. 全局设置（`~/.pi/agent/settings.json`）
5. 内置默认值

## `PI_CONFIG_PATH`（单文件模式）

若设置了 `PI_CONFIG_PATH`，Pi 仅加载该文件并跳过全局/项目合并。

## 合并行为（全局 vs 项目）

项目设置按字段覆盖全局设置。

重要细节：像 `compaction`、`retry`、`images`、`terminal`、`branch_summary` 和 `thinking_budgets` 这样的嵌套对象被视为*单个*字段。若 `.pi/settings.json` 包含 `compaction` 对象，它将替换整个全局 `compaction` 对象。

在单个文件内，缺失的嵌套键在访问时回退到内置默认值（见 `src/config.rs`）。

示例：

```json
// ~/.pi/agent/settings.json（全局）
{ "compaction": { "enabled": false, "reserve_tokens": 16384 } }
```

```json
// .pi/settings.json（项目）
{ "compaction": { "reserve_tokens": 8192 } }
```

结果行为：
- `compaction.reserve_tokens` 变为 `8192`
- `compaction.enabled` 不会从全局继承 `false`，而是回退到其内置默认值

## 支持的设置（snake_case JSON 键）

### 外观

- `theme`（string）：要应用的主题。内置：`dark`、`light`、`solarized`；也接受已发现的主题名或主题 JSON 文件路径。`light/dark`、`auto` 或 `system` 通过 `COLORFGBG` 环境变量从终端背景自动检测深色/浅色（不可用时为深色）。若未设置，则以同样方式自动检测。
- `hide_thinking_block`（bool）：在交互式输出中隐藏思考块。默认 `false`。
- `show_hardware_cursor`（bool）：显示终端硬件光标。默认 `false`，除非 `PI_HARDWARE_CURSOR=1`。

### 模型选择

- `default_provider`（string）
- `default_model`（string）
- `default_thinking_level`（string）
- `enabled_models`（model 模式数组）

示例：

```json
{
  "default_provider": "anthropic",
  "default_model": "claude-sonnet-4-20250514",
  "default_thinking_level": "medium",
  "enabled_models": ["claude-*", "gpt-*"]
}
```

### 消息投递（队列模式）

- `steering_mode`（string）：`one-at-a-time` 或 `all`（默认 `one-at-a-time`）。
- `follow_up_mode`（string）：`one-at-a-time` 或 `all`（默认 `one-at-a-time`）。

旧版别名：`steeringMode`、`followUpMode`。

```json
{
  "steering_mode": "one-at-a-time",
  "follow_up_mode": "one-at-a-time"
}
```

### 交互式体验 / 编辑器

- `double_escape_action`（string）：`tree`、`fork` 或 `none`（默认 `tree`）。别名：`doubleEscapeAction`。使用 `none` 禁用双击 Escape 快捷键。
- `editor_padding_x`（u32）：编辑器水平内边距（限制为 0–3）。默认 `0`。
- `autocomplete_max_visible`（u32）：自动补全最大可见行数（限制为 3–20）。默认 `5`。
- `session_picker_input`（u32）：非交互式会话选择器选项（从 1 开始）。别名：`sessionPickerInput`。
- `quiet_startup`（bool）：抑制启动头。
- `collapse_changelog`（bool）：存在时压缩“What's New”输出。

### 压缩（默认值）

访问器默认值：
- `compaction.enabled`：`true`
- `compaction.reserve_tokens`：`16384`
- `compaction.keep_recent_tokens`：`20000`

```json
{
  "compaction": {
    "enabled": true,
    "reserve_tokens": 16384,
    "keep_recent_tokens": 20000
  }
}
```

### 分支摘要

- `branch_summary.reserve_tokens`（u32）：默认为 `compaction.reserve_tokens`。

### 重试（默认值）

访问器默认值：
- `retry.enabled`：`true`
- `retry.max_retries`：`3`
- `retry.base_delay_ms`：`2000`
- `retry.max_delay_ms`：`60000`

```json
{
  "retry": {
    "enabled": true,
    "max_retries": 3,
    "base_delay_ms": 2000,
    "max_delay_ms": 60000
  }
}
```

### Shell

- `shell_path`（string）：Shell 二进制路径。默认 `/bin/bash`。
- `shell_command_prefix`（string）：默认 `set -e`。
- `gh_path`（string）：覆盖 `/share` 的 `gh` 路径。别名：`ghPath`。

```json
{
  "shell_path": "/bin/bash",
  "shell_command_prefix": "set -e"
}
```

### 子智能体

- `subagent_structured_results`（bool）：默认 `false`。别名：`subagentStructuredResults`。为 `true` 时，`subagent` 工具会在结果文本后追加机器可读的 `<subagent-structured-result>` 块：一个紧凑的 JSON 数组，每项对应一个子智能体（`agent`、`step`、`status`、`exitCode`、`output`、`error` —— 与 `pi.subagent.result.v1` 详情 schema 字段名相同）。`output`/`error` 各截断至 2 KiB，整个块上限 16 KiB；当必须丢弃条目时，数组最后一项为 `{"truncated": true, "omittedResults": N}`。默认 `false` 时工具输出与旧版本保持字节一致。

```json
{
  "subagent_structured_results": true
}
```

### 图像

- `images.auto_resize`（bool）：默认 `true`。
- `images.block_images`（bool）：默认 `false`。

```json
{
  "images": {
    "auto_resize": true,
    "block_images": false
  }
}
```

### 终端显示

- `terminal.show_images`（bool）：默认 `true`。为 `false` 时，Pi 在终端工具输出中隐藏图像块（图像仍存储于会话/导出中）。
- `terminal.clear_on_shrink`（bool）：默认 `false`。为 `true` 时，Pi 在终端高度收缩时清除回滚缓冲区，以避免调整大小后陈旧行重新出现。

### 思考预算（token）

- `thinking_budgets.minimal`：默认 `1024`
- `thinking_budgets.low`：默认 `2048`
- `thinking_budgets.medium`：默认 `8192`
- `thinking_budgets.high`：默认 `16384`
- `thinking_budgets.xhigh`：默认 `32768`
- `thinking_budgets.max`：默认 `65536`

### 包与资源

- `packages`（array）：包来源（string 或 `{ source, local, kind }`）。
- `extensions`、`skills`、`prompts`、`themes`（arrays）：资源过滤器。
- `enable_skill_commands`（bool）：默认 `true`。

## 完整参考

`src/config.rs` 是支持字段与默认值行为的权威清单。
