# 主题

Pi 的交互式 TUI 支持 **JSON 主题文件**以及若干内置主题。

如果本文描述与你实际看到的不一致，请查看 `src/theme.rs` 与主题工作流（`bd-22p`）—— 主题体验仍在演进中。

## 内置主题

- `dark`
- `light`
- `solarized`

## 主题发现（自定义主题）

Pi 通过扫描以下目录下的 `*.json` 文件来发现自定义主题：

- 全局：`~/.pi/agent/themes/`
- 项目：`<cwd>/.pi/themes/`

发现仅依据文件扩展名；Pi 会加载每个 JSON 文件并使用其中的 `name` 字段。

## 选择主题

### 交互式命令

- ` /theme `（无参数）：列出已发现的主题
- ` /theme <name> `：切换主题

注意：`/settings` 中包含会打开选择器的 Theme 条目。`/theme` 仍可用于快速切换，直接编辑 `settings.json` 也同样有效。

### 设置文件

在设置 JSON 中设置 `theme`：

- 全局：`~/.pi/agent/settings.json`
- 项目：`<cwd>/.pi/settings.json`

示例：

```json
{
  "theme": "solarized"
}
```

如果已配置的主题无法加载，Pi 会回退到 `dark` 并记录一条警告。

## 主题文件格式（JSON）

主题 JSON 文件在加载时会被校验。所有颜色均为 `#RRGGBB` 格式的**十六进制字符串**。

最小示例：

```json
{
  "name": "my-theme",
  "version": "1.0",
  "colors": {
    "foreground": "#e6e6e6",
    "background": "#0b0f14",
    "accent": "#38bdf8",
    "success": "#22c55e",
    "warning": "#f59e0b",
    "error": "#ef4444",
    "muted": "#94a3b8"
  },
  "syntax": {
    "keyword": "#38bdf8",
    "string": "#22c55e",
    "number": "#a78bfa",
    "comment": "#94a3b8",
    "function": "#f59e0b"
  },
  "ui": {
    "border": "#1f2937",
    "selection": "#111827",
    "cursor": "#e6e6e6"
  }
}
```

### 字段含义（概览）

- `colors.*`：主要 UI 颜色（文本/背景 + 语义化颜色）
- `syntax.*`：用于代码/标记渲染的颜色
- `ui.*`：边框/选区/光标颜色

## 与旧版 pi-mono 的当前差距

旧版 pi-mono 支持更多主题发现机制（包、`themes[]` 设置路径、CLI `--theme`、热重载、更多令牌）。Rust 移植版目前有意保持更小规模。

进度请跟踪 `bd-22p`。
