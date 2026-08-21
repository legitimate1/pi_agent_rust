# Windows 说明

Pi 可在 Windows 上原生运行，但有一些平台特定的差异需要注意。

## Shell 要求（bash 工具）

Pi 的 `bash` 工具（以及 `!command` 快捷方式）需要 **POSIX shell**。在 Windows 上，请安装以下之一：

1. Git Bash（推荐）：`C:\Program Files\Git\bin\bash.exe`
2. `PATH` 上的 MSYS2/Cygwin bash
3. WSL bash（若已暴露在 `PATH` 上）

你也可以在设置中指定自定义 shell：

```json
{
  "shell_path": "C:\\Program Files\\Git\\bin\\bash.exe"
}
```

## 按键绑定

### Windows Terminal

- **换行**：在编辑器中使用 `Ctrl+Enter` 插入换行（而非 Linux/macOS 上常见的 `Shift+Enter`）。`Enter` 用于提交消息。

## 剪贴板

Pi 会尝试为 `/copy` 和图片粘贴使用系统剪贴板。

- 如果通过远程会话（如 SSH）运行，请确保所在终端支持剪贴板访问。
- 如果剪贴板操作失败，Pi 通常会回退为打印内容或忽略粘贴。

## 路径

- Pi 同时支持正斜杠 `/` 与反斜杠 `\` 路径。
- 在 JSON 中配置路径时（如 `settings.json`），请记得转义反斜杠：`C:\Users\Name\.pi`。
- 如需跨平台兼容，`settings.json` 中请尽量使用正斜杠（`C:/Users/Name/.pi`）。

## Shell 命令

- 在 `bash` 工具和 `!command` 快捷方式中，若可用，Pi 会尝试使用 `sh`（Git Bash 或类似）。
- 若配置了 `shell_path`，请指向你偏好的 shell 可执行文件（如 `bash.exe`、`powershell.exe`、`pwsh.exe`）。
- `models.json` 中的密钥解析使用 `cmd /C` 来执行 `!commands`。
