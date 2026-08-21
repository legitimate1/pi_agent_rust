# 扩展排障指南

本指南涵盖扩展运行时、一致性测试框架与能力策略系统中的常见故障模式。每一节都将症状映射到根因与具体修复方法。

## 宿主调用错误码

当扩展调用失败时，宿主会返回带有以下错误码之一的 `HostcallOutcome::Error`：

| Code              | 含义                                           |
|-------------------|------------------------------------------------|
| `denied`          | 能力策略拒绝了该操作           |
| `invalid_request` | 载荷格式错误、未知操作或参数错误  |
| `timeout`         | 操作超出预算                      |
| `io`              | 文件系统或网络 I/O 失败                  |
| `internal`        | 非预期的宿主错误（缺陷）                        |

## 策略失败

### 症状：`pi.exec()` 或 `pi.env()` 上出现 `denied` 错误

**原因**：默认策略配置（`Standard`）会拒绝 `exec` 与 `env` 能力。运行 shell 命令或读取环境变量的扩展会被阻止。

**修复**：
```toml
# pi.toml
[extensions.policy]
profile = "standard"
allow_dangerous = true
```
或使用 CLI 标志：`--extension-policy permissive`

### 症状：切换到 `Safe` 配置后出现 `denied`

**原因**：`Safe` 配置使用 `Strict` 模式，且 `default_caps` 中仅包含 `read` 与 `write`。`http`、`events`、`session` 等能力不在允许清单中，未经提示即会被拒绝。

**修复**：切换到 `Standard`（允许非危险能力，对危险能力进行提示）或 `Permissive`（允许所有操作并记录审计日志）。

### 症状：一个扩展正常工作，但另一个扩展被拒绝

**原因**：策略中存在按扩展覆盖。检查已解析策略中的 `per_extension` 条目。

**诊断**：
```bash
# 检查生效策略
pi info <extension-id>

# 或检查已解析配置
pi config show | grep -A 20 extensions.policy
```

### 症状：“Allow Always” 未持久化

**原因**：`PermissionStore` 会写入 `~/.pi/agent/extension_permissions.json`。若该文件不可写或目录不存在，则决策仅在当前会话有效。

**修复**：确保 `~/.pi/agent/` 存在且可写。

### 策略优先级（用于调试）

评估某项能力时，5 层链按顺序执行：

1. 按扩展 `deny` 列表 —— 始终拒绝
2. 全局 `deny_caps` —— 始终拒绝
3. 按扩展 `allow` 列表 —— 始终允许
4. 全局 `default_caps` —— 始终允许
5. 模式回退 —— Strict:拒绝，Prompt:提示，Permissive:允许

要诊断哪一层产生了决策，请检查 `PolicyCheck` 中的 `reason` 字段：
- `"extension_deny"` —— 第 1 层
- `"deny_caps"` —— 第 2 层
- `"extension_allow"` —— 第 3 层
- `"default_caps"` —— 第 4 层
- `"mode_strict"` / `"mode_prompt"` / `"mode_permissive"` —— 第 5 层

## 扩展加载失败

### 症状：“是否需要先将 JS 扩展转换为描述符？”

**回答**：不需要。旧版 `.js/.ts` 扩展直接在内嵌的 QuickJS 运行时中运行。对于常规扩展使用，不需要进行描述符转换步骤。

描述符条目（`*.native.json`）是可选的原生 Rust 运行时路径。当前会话一次只能使用一个运行时家族（JS/TS 或原生描述符）。

### 症状：“Extension entry does not exist”

**原因**：`JsExtensionLoadSpec::from_entry_path()` 找不到文件。

**修复**：
- 验证扩展已安装：`ls ~/.pi/agent/extensions/`
- 检查扩展目录中是否存在 `extension.json`
- 确保 `extension.json` 中的 `entry_path` 指向有效的 `.js`/`.ts`

### 症状：扩展已加载但 `activate()` 从未运行

**原因**：入口点未调用 `pi.register()`。QuickJS 运行时会加载文件，但注册需要显式调用。

**修复**：确保扩展入口点调用了：
```js
pi.register({
  name: "my-extension",
  version: "1.0.0",
  apiVersion: "1.0",
  capabilities: ["read", "session"],
  tools: [...],
  eventHooks: [...]
});
```

### 症状：Node 内置模块报 "Module not found"

**原因**：扩展导入的 Node 模块在 QuickJS 中未被垫片化。

**已垫片化模块**：`node:fs`、`node:path`、`node:os`、`node:crypto`、`node:child_process`、`node:events`、`node:buffer`、`node:url`、`node:http`、`node:net`、`node:readline`、`node:util`、`node:stream`

**修复**：若模块不在上述列表中，请检查是否存在虚拟模块存根。若不存在，该扩展可能需要兼容性补丁或需要新增垫片。

### 症状：npm 包报 "Module not found"

**原因**：QuickJS 没有 node_modules 解析器。npm 包必须被打包进扩展，或作为虚拟模块存根提供。

**已存根的包**：`glob`、`uuid`、`jsonwebtoken`、`shell-quote`、`chalk`、`chokidar`、`jsdom`、`turndown`、`node-pty`、`@opentelemetry/*`、`@xterm/*`、`vscode-languageserver-protocol`、`@sinclair/typebox`、`@mariozechner/pi-ai`

**修复**：若该包用于核心功能，则需要真实垫片。若用于可选功能（遥测、IDE 集成），无操作存根可能就已足够。

## 一致性测试框架失败

### 症状：测试显示 `N/A` 而非 `PASS`/`FAIL`

**原因**：该场景需要测试框架尚未实现的能力。常见缺失能力：
- `mock_http` —— 为发起请求的扩展提供 HTTP 响应模拟
- `mock_model_registry` —— 为提供方测试提供模型注册表模拟
- `mock_exec` —— 子进程输出模拟

**诊断**：检查对等日志中的 `skip_reason` 字段：
```json
{"status":"skip","skip_reason":"requires mock_http"}
```

**修复**：这些被跟踪为一致性证据缺口。完整分类见 `tests/ext_conformance/reports/CONFORMANCE_REPORT.md`。

### 症状：一致性差异显示误报

**原因**：非确定性输出（时间戳、路径、随机值）在 TS 基准与 Rust 运行时之间不同。

**修复**：
- 设置 `PI_TEST_MODE=1` 以稳定时间戳与 CWD
- 设置 `PI_CONFORMANCE_SEED=42` 以获得确定性的一致性差异
- 设置 `PI_EXT_RANDOM_SEED=42 PI_EXT_RANDOM_N=1` 以进行有界随机试验冒烟运行
- 使用路径规范化断言（后缀匹配，而非精确匹配）
- 查看 `docs/extension-architecture.md` 了解归一化细节

### 症状：TS 基准超时

**原因**：TypeScript 基准（基于 Bun）对每个扩展默认超时 30 秒。复杂扩展或较慢的机器可能超出此时限。

**修复**：
```bash
export PI_TS_ORACLE_TIMEOUT_SECS=60
```

测试框架对不稳定的基准超时包含重试逻辑。

### 症状：一致性测试找不到 Bun

**原因**：测试框架期望 Bun 位于 `/home/ubuntu/.bun/bin/bun`。

**修复**：
```bash
# 安装 Bun
curl -fsSL https://bun.sh/install | bash

# 或为现有安装创建软链接
ln -sf $(which bun) /home/ubuntu/.bun/bin/bun
```

### 症状："npm ci" 在 legacy_pi_mono_code 中失败

**原因**：TS 基准依赖 `legacy_pi_mono_code/pi-mono/` 已安装 npm 依赖。

**修复**：
```bash
cd legacy_pi_mono_code/pi-mono
npm ci
```

## 会话与状态失败

### 症状：`pi.session("setLabel")` 返回 null

**原因**：`Session::add_label` 要求目标条目存在于会话中。若 `target_id` 与任何条目都不匹配，则返回 `None`。

**修复**：在打标签前确保消息/条目存在。使用 `pi.session("getEntries")` 验证目标 ID。

### 症状：会话操作失败并提示 "no session"

**原因**：`ExtensionManager` 未附加会话。发生在以下情况：
- 未调用 `set_session()` 的测试环境
- 非交互式 CLI 模式（`--print` 模式）

**修复**：对于测试，附加一个真实会话：
```rust
let session = SessionHandle(Arc::new(Mutex::new(Session::create())));
manager.set_session(Arc::new(session) as Arc<dyn ExtensionSession>);
```

## 文件系统逃逸模式

这些是经安全测试的失败模式（见 `tests/security_fs_escape.rs`）：

| 攻击                  | 控制措施                                |
|-------------------------|----------------------------------------|
| `../../etc/passwd`      | 路径规范化 + 根检查     |
| 符号链接到 `/etc`       | `canonicalize()` 解析真实路径    |
| `//server/share`        | UNC 路径检测                     |
| `/dev/null` 读取        | 设备文件排除                  |
| 超长路径          | 路径长度限制检查                |

扩展无法通过 `host_read_fallback` 机制读取工作目录根之外的文件。

## 结构化并发失败

### 症状：会话结束时扩展清理挂起

**原因**：`ExtensionRegion` 关闭预算（默认 5 秒）可能不足以完成含长时间运行操作的扩展。

**修复**：
```rust
ExtensionRegion::with_budget(manager, Duration::from_secs(15))
```

### 症状：会话结束后宿主调用失败并提示 "shutdown"

**原因**：`JsRuntimeHost` 持有 `Weak<Mutex<ExtensionManagerInner>>` 引用。在 `ExtensionManager` 被丢弃后，弱引用无法升级，所有宿主调用均以原因 `"shutdown"` 返回 `Deny`。

**修复**：此为设计使然。扩展应优雅地处理关闭，不应在清理期间发起宿主调用。

### 症状："Budget exceeded" 错误

**原因**：`effective_timeout()` 会将管理器的剩余预算与单次操作超时取交集。若管理器预算几近耗尽，即使较短的操作也可能超时。

**诊断**：检查 `extension_budget` 剩余时间与操作超时的对比。

## 提供方扩展失败

### 症状：自定义 `streamSimple` 提供方返回空响应

**原因**：JS `streamSimple()` 函数必须返回 `AsyncIterable<string>`。若返回 `undefined` 或不可迭代对象，Rust 侧会将其解释为空流。

**修复**：确保 `streamSimple` 为异步生成器：
```js
async function* streamSimple(model, context, options) {
  yield "Hello ";
  yield "world";
}
```

### 症状：OAuth 令牌刷新失败

**原因**：`refresh_extension_oauth_token()` 函数要求 `ModelEntry` 上具有有效的 `OAuthConfig`。缺少 `token_url` 或 `client_id` 会导致刷新失败。

**修复**：验证提供方注册包含完整的 OAuth 配置：
```js
pi.events("registerProvider", {
  name: "my-provider",
  models: [{ id: "model-1", oauth: {
    authUrl: "...",
    tokenUrl: "...",
    clientId: "...",
    scopes: ["read"]
  }}]
});
```

## 快速参考

| 错误                         | 可能原因            | 首要步骤                    |
|-------------------------------|-------------------------|-------------------------------|
| `denied`                      | 策略阻止了能力| 检查配置 + deny_caps     |
| `invalid_request`             | 载荷/操作名错误     | 检查 JS 调用参数            |
| `timeout`                     | 预算耗尽        | 增大超时/预算       |
| Module not found              | 缺少垫片            | 检查已垫片化模块列表     |
| N/A in conformance            | 测试框架功能缺失 | 检查日志中的 skip_reason      |
| Session op returns null       | 缺少会话/条目   | 附加会话，验证 ID     |
| Extension not loading         | 缺少 extension.json  | 检查安装目录       |
| Cleanup hangs                 | 预算不足     | 增大 ExtensionRegion 预算|
