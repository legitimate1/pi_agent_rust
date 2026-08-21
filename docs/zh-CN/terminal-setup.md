# 终端设置

Pi 可在任何现代终端中工作，但某些功能（如图像显示）与按键组合需要终端特定的支持或配置。

## 推荐终端

- **Ghostty**：出色的性能与 Kitty 图形支持。
- **WezTerm**：优秀的跨平台支持与 iTerm 图形协议。
- **iTerm2**：可靠的 iTerm 图形协议支持（macOS）。
- **Kitty**：一流的 Kitty 图形支持。
- **Windows Terminal**：良好的 Unicode 支持，内联图像支持有限。

## 键盘协议说明

某些终端需要启用 **Kitty 键盘协议**以可靠支持组合键（例如 `Shift+Enter`、`Alt+Backspace`）。

### Ghostty

添加到 `~/.config/ghostty/config`：

```
keybind = alt+backspace=text:\x1b\x7f
keybind = shift+enter=text:\n
```

### WezTerm

创建 `~/.wezterm.lua`：

```lua
local wezterm = require 'wezterm'
local config = wezterm.config_builder()
config.enable_kitty_keyboard = true
return config
```

### VS Code（集成终端）

添加到 `keybindings.json`：

```json
{
  "key": "shift+enter",
  "command": "workbench.action.terminal.sendSequence",
  "args": { "text": "\u001b[13;2u" },
  "when": "terminalFocus"
}
```

### Windows Terminal

添加到 `settings.json`：

```json
{
  "actions": [
    {
      "command": { "action": "sendInput", "input": "\u001b[13;2u" },
      "keys": "shift+enter"
    }
  ]
}
```

### IntelliJ IDEA（集成终端）

IntelliJ 的终端无法区分 `Shift+Enter` 与 `Enter`。为获得最佳体验，请使用外部终端。

若想让硬件光标可见，请在运行 `pi` 前设置 `PI_HARDWARE_CURSOR=1`。

## 图像支持

Pi 检测终端能力以在内联显示图像（目前支持 Kitty 兼容终端如 Kitty、WezTerm 与 Ghostty，以及 iTerm2）。对于不支持的终端，Pi 回退到稳定的占位符如 `[image: image/png, 1024x768]`。

要完全屏蔽图像，请设置：

```json
{
  "images": {
    "block_images": true
  }
}
```

你也可以在终端输出中隐藏图像块：

```json
{
  "terminal": {
    "show_images": false
  }
}
```

`terminal.show_images` 控制 Pi 是否在终端工具输出中包含图像块（默认为 `true`）。

`terminal.clear_on_shrink`（默认为 `false`）在终端高度收缩时清除回滚缓冲区，有助于避免调整大小后陈旧行重新出现。

## 快捷键

某些终端会拦截 Pi 所需的按键组合（例如 `Ctrl+Arrow`、`Shift+Enter`）。

- **Windows Terminal**：若 `Shift+Enter` 不可用，请使用 `Ctrl+Enter` 换行。
- **VS Code Terminal**：某些快捷键可能被 VS Code 捕获。请检查你的 `terminal.integrated.commandsToSkipShell` 设置。
