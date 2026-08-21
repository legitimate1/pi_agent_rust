# 故障排查

本页收录常见故障与实用修复方法。若某行为仍在实现中，会列出对应的 bead ID 以便跟踪。

## API 密钥与认证

**现象：** `Missing API key` 或提供方认证错误。

**修复：**
- 使用环境变量：`ANTHROPIC_API_KEY`、`OPENAI_API_KEY`、`GOOGLE_API_KEY` 等。
- 或每次运行时设置 `--api-key`。
- 或通过 `/login`（Anthropic OAuth）将凭据存储到 `~/.pi/agent/auth.json`。

**配置优先级（高 → 低）：**
1. `--api-key`
2. `auth.json` 中未过期的 OAuth/bearer 凭据
3. 提供方元数据声明顺序中的提供方专属环境变量
4. `auth.json` 中存储的 API 密钥
5. 受支持的外部编程 CLI 凭据（仅全局认证存储）
6. 选中模型附带时的内联 `models.json` 提供方 `apiKey` 回退

提供方托管的凭据是通用 API 密钥链的例外：Bedrock 在请求时解析 AWS 凭据链/SigV4，SAP AI Core 则用客户端凭据交换 bearer 令牌，而非将客户端密钥作为 API 令牌发送。

## 提供方错误（401/429/5xx）

**401/403：** 密钥缺失或无效。请确认提供方与密钥是否正确。

**429：** 限流。若设置中 `retry.enabled` 为 true，Pi 会重试。

**5xx/网络：** 提供方临时故障或网络不稳定。请重试或切换模型。

**重试配置**位于 `~/.pi/agent/settings.json`：
```json
{
  "retry": {
    "enabled": true,
    "maxRetries": 3,
    "baseDelayMs": 1000,
    "maxDelayMs": 30000
  }
}
```

## VCR 模式（测试）

演练提供方流式传输的测试使用录制的磁带以保证确定性。

环境变量：
- `VCR_MODE=record|playback|auto`
- `VCR_CASSETTE_DIR=tests/fixtures/vcr`（默认值）

常见修复：
- 缺少磁带：先以 `VCR_MODE=record` 运行一次，然后提交磁带。
- CI 无网络：使用 `VCR_MODE=playback`。
- 非法的 `VCR_MODE`：仅接受 `record`、`playback` 或 `auto`。

## 包与扩展

**现象：** 未找到扩展或技能。

**修复：**
- 通过 `pi list` 检查包来源。
- 确认 `~/.pi/agent/settings.json` 或 `.pi/settings.json` 中的设置。
- 添加来源后重新运行 `pi update`。

扩展发现由 **bd-1e0** 跟踪（安装 + 解析）。若扩展加载失败，随着该 bead 落地，诊断信息将会改进。

## 会话（持久化 + 恢复）

会话位于：
```
~/.pi/agent/sessions/
```

覆盖：
- `PI_CODING_AGENT_DIR`（全局基目录）
- `PI_SESSIONS_DIR`（会话根目录）

**损坏恢复：**
- 使用 `--no-session` 运行以绕过持久化。
- 将有问题的 `.jsonl` 文件移出会话目录。

`/resume`、`/tree`、`/fork` 的交互式 UX 对等由 **bd-14cc** 跟踪。

## 按键绑定与热键

按键绑定加载自：
```
~/.pi/agent/keybindings.json
```

若快捷键未按预期工作：
- 删除/重命名该文件以回退到默认值。
- 确认终端未拦截按键。

完整按键绑定对等（含 `/hotkeys`）由 **bd-3ip** 跟踪。

## 终端差异

某些终端会占用按键组合（尤其在 Windows 上）：
- `Ctrl+Enter` / `Alt+Enter` 可能被拦截。
- 不同终端的粘贴事件可能不同。

若快捷键未触发，请尝试更换终端或重新映射按键。交互式编辑器对等（自动补全/bang/粘贴）由 **bd-1iwi** 跟踪。

## 缺少系统依赖

`find` 工具需要 `fd`：
```bash
# Ubuntu/Debian
apt install fd-find

# macOS
brew install fd

# 二进制可能名为 fdfind
ln -s $(which fdfind) ~/.local/bin/fd
```

`rg`（ripgrep）可选但推荐，以加快搜索。

## 工具输出被截断

大型工具输出会被截断以保护上下文窗口。请按指定范围请求（例如“读取该文件的 2000-4000 行”）。
