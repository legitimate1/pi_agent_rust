# Android (Termux) 说明

Pi 可通过 [Termux](https://termux.dev/) 在 Android 上运行，但部分功能会受移动环境限制。

## 前置条件

1. 从 F-Droid 或 GitHub 安装 **Termux**（Play Store 构建已弃用）。
2. 安装 **Termux:API**（剪贴板集成所需）。
3. 在 Termux 中执行：
   ```bash
   pkg update && pkg upgrade
   pkg install termux-api git
   ```

当标准剪贴板访问失败时，Pi 会检测 `termux-clipboard-get` 和 `termux-clipboard-set`。

> 注意：官方源码构建使用 `rust-toolchain.toml` 中钉住的确切 nightly 版本。如果 Termux 难以安装该工具链，请在桌面端构建二进制文件后复制过去。

## 剪贴板支持

- **文本剪贴板**：通过 `termux-clipboard-get` / `termux-clipboard-set` 可用。
- **图片剪贴板**：在 Termux 上**不支持**（`Ctrl+V` 粘贴图片流程将无操作）。

## 终端差异

- 如果方向键或快捷键行为异常，请在 Termux 设置中配置**额外按键行**。
- 某些终端会将 `Ctrl+Enter` 当作 `Shift+Enter` 发送，用于“插入换行”行为。

## 存储

- 会话位于 `~/.pi/agent/sessions`。
- 如需访问共享存储（Downloads/Documents），执行一次：
  ```bash
  termux-setup-storage
  ```

## 故障排查

### 剪贴板无法工作

请确保已安装以下两个应用：
1. Termux（来自 F-Droid/GitHub）
2. Termux:API

然后安装 CLI 工具：
```bash
pkg install termux-api
```

### 共享存储权限被拒绝

执行一次以授予存储权限：
```bash
termux-setup-storage
```
