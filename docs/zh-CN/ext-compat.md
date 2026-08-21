# 扩展运行时兼容性矩阵

本文档描述了 Pi 扩展在 PiJS（基于 QuickJS）运行时中可用的 Node.js 与 Bun API 表面。扩展作者可通过此矩阵判断其扩展是否可无需修改直接运行。

---

## 一致性快照

| 表面 | 结果 | 详情 |
|---|---:|---|
| 扩展语料库（223 个扩展） | 205/223 通过（91.9%） | Tier 1 100%，Tier 2 95.4% |
| 场景一致性 | 24/25 通过（96.0%） | 注册、事件、工具、会话 |
| Node API 矩阵 | 13/13 通过（100%） | 全部关键 Node 内置模块已覆盖 |
| Bun API 矩阵 | 7/7 通过（已打桩） | `connect`/`listen` 已打桩（无网络 I/O） |

---

## 1. 兼容性层级

| 层级 | 描述 | 语料库覆盖率 | 策略 |
|------|---------|---:|--------|
| T1 | 简单单文件扩展 | 38/38 (100%) | 静默运行 |
| T2 | 多重注册（工具 + 事件 + 标志） | 83/87 (95.4%) | 静默运行 |
| T3 | 含本地导入的多文件扩展 | 79/90 (87.8%) | 运行 + 相对导入失败时告警 |
| T4 | 含 npm 依赖的扩展 | 1/3 (33.3%) | 除非存在虚拟存根否则阻断 |
| T5 | 使用 exec/网络 API 的扩展 | 4/5 (80.0%) | 能力门控 |

---

## 2. Node.js 内置模块

### 完全支持

这些模块已在 PiJS 中打桩（shim），其 API 子集行为与 Node.js 对应实现一致，覆盖扩展实际使用的范围。

| 模块 | 关键 API | 测试覆盖 | 备注 |
|--------|----------|---:|-------|
| `node:path` | `join`、`resolve`、`dirname`、`basename`、`extname`、`sep`、`posix`、`win32` | 完整 | POSIX 语义 |
| `node:fs` | `readFileSync`、`writeFileSync`、`statSync`、`mkdirSync`、`readdirSync`、`unlinkSync`、`rmSync`、`copyFileSync`、`renameSync`、`appendFileSync`、`accessSync`、`existsSync`、`realpathSync`、`readlinkSync`、`createReadStream`、`createWriteStream`、`chmodSync`、`chownSync` | 39 项测试 | 默认限定于扩展目录；权限变更为路径检查空操作 |
| `node:fs/promises` | `readFile`、`writeFile`、`stat`、`mkdir`、`readdir`、`unlink`、`rm`、`access`、`copyFile`、`rename`、`chmod`、`chown`、`utimes` | 包含在上 | fs 打桩的异步版本；权限变更为路径检查空操作 |
| `node:crypto` | `createHash`、`createHmac`、`randomUUID`、`randomBytes`、`randomInt`、`timingSafeEqual`、`getHashes`、Ed25519 `sign`/`verify` | 56 项测试 | SHA-256/SHA-384/SHA-512、SHA-1、MD5、HMAC、KDF；cipher 与 RSA/ECDSA 失效关闭 |
| `node:buffer` | `Buffer.from`、`Buffer.alloc`、`Buffer.concat`、`Buffer.isBuffer`、`Buffer.byteLength`、`.toString()`、`.slice()`、`.subarray()`、`.compare()`、`.equals()`、`.indexOf()`、`.copy()` | 41 项测试 | 完整的 Buffer 协议 |
| `node:child_process` | `spawnSync`、`execSync`、`execFileSync`、`spawn`、`exec`、`execFile` | 53 项测试 | 能力门控（`exec`） |
| `node:http` | `request`、`get`、`createServer`、`STATUS_CODES`、`METHODS`、`Agent` | 40 项测试 | `createServer` 抛错（沙箱） |
| `node:https` | `request`、`get` | 与 http 共享 | 同 `node:http` |
| `node:events` | `EventEmitter`、`on`、`emit`、`once`、`removeListener`、`removeAllListeners`、`listenerCount` | 26 项测试 | 完整的 EventEmitter 模式 |
| `node:os` | `platform`、`hostname`、`tmpdir`、`homedir`、`cpus`、`arch`、`type`、`release`、`userInfo`、`EOL` | 包含在内 | 返回宿主值 |
| `node:url` | `URL`、`URLSearchParams`、`parse`、`format`、`resolve` | 6 项测试 | WHATWG URL 标准 |
| `node:process` | `env`、`argv`、`cwd`、`exit`、`platform`、`arch`、`version`、`pid`、`hrtime` | 24 项测试 | `exit` 已沙箱化 |
| `node:util` | `format`、`inspect`、`inherits`、`deprecate`、`debuglog`、`types`、`TextEncoder`、`TextDecoder`、`stripVTControlCharacters` | 包含在内 | 标准工具函数 |
| `node:stream` | `Readable`、`Writable`、`Transform`、`Duplex`、`PassThrough`、`pipeline`、`finished` | 包含在内 | Stream 构造器与辅助函数 |
| `node:stream/promises` | `pipeline`、`finished` | 包含在内 | 基于 Promise 的 stream 辅助 |
| `node:querystring` | `parse`、`stringify`、`encode`、`decode` | 包含在内 | 查询字符串工具 |
| `node:assert`、`node:assert/strict` | `ok`、`strictEqual`、`deepStrictEqual`、`throws`、`rejects`、`fail` | 包含在内 | 测试断言辅助 |
| `node:string_decoder` | `StringDecoder` | 包含在内 | UTF-8 字符串解码 |
| `node:module` | `createRequire` | 包含在内 | 模块系统兼容 |

### 部分支持

这些模块仅暴露 Node.js API 的子集。缺失的函数会抛错，并附带清晰的错误信息指明不支持的调用。

| 模块 | 已支持 | 未支持 | 备注 |
|--------|-----------|-------------|-------|
| `node:net` | `createConnection`（存根）、`Socket`（存根） | `createServer` | 存根套接字（无网络 I/O）；请使用 `pi.http()` |
| `node:readline` | `createInterface`、`promises.createInterface` | 完整交互式 readline | 可用时使用 `pi.ui('input')`；非交互式提示解析为空字符串 |

### 已阻断

这些模块因需要超出扩展沙箱的能力而被阻断。

| 模块 | 原因 | 替代方案 |
|--------|--------|-------------|
| `vm` | 任意代码执行 | 直接使用扩展 API |
| `worker_threads` | 线程创建 | 单线程运行时 |
| `cluster` | 进程派生 | 单进程运行时 |
| `dgram` | 原始 UDP 套接字 | 使用 `pi.http()` 进行网络通信 |
| `tls` | 原始 TLS 套接字 | 改用 `node:https` |

---

## 3. Bun API

Pi 通过 `globalThis.Bun` 与 `import "bun"` 提供面向 Bun 的兼容表面。

| API | 状态 | 备注 |
|-----|--------|-------|
| `Bun.argv` | 已支持 | 进程参数 |
| `Bun.file(path)` | 已支持 | 返回含 `exists()`、`text()`、`arrayBuffer()`、`json()` 的对象 |
| `Bun.write(path, data)` | 已支持 | 写入文件 |
| `Bun.which(command)` | 已支持 | 在 PATH 上定位可执行文件 |
| `Bun.spawn(...)` | 已支持 | 能力门控（`exec`） |
| `Bun.connect(...)` | 已打桩（无网络） | 内存套接字发射器；真实网络请使用 `pi.http()` 或 `node:http` |
| `Bun.listen(...)` | 已打桩（无网络） | 内存服务器发射器；真实网络请使用 `pi.http()` 或 `node:http` |

---

## 4. 虚拟 npm 模块存根

导入热门 npm 包的扩展会获得虚拟存根，这些存根暴露对应包的公共 API 形态。即使未安装真实包，扩展也能加载并完成注册而不会出现运行时错误。

### Pi 框架模块

| 包 | 关键导出 |
|---------|-------------|
| `@mariozechner/pi-coding-agent` | `ExtensionAPI`、`Tool`、`SlashCommand`、`EventHook` |
| `@mariozechner/pi-ai` | `AI`、`Message`、`StreamEvent` |
| `@mariozechner/pi-tui` | `TUI`、`Widget`、`Layout` |
| `@sinclair/typebox` | `Type`、`Static`、`TSchema` |

### 协议与框架模块

| 包 | 关键导出 |
|---------|-------------|
| `@modelcontextprotocol/sdk/*` | MCP 客户端/服务端/传输类型 |
| `vscode-languageserver-protocol/*` | LSP 类型与协议定义 |
| `jsonwebtoken` | `decode`、HS256/HS384/HS512 `sign`/`verify` |
| `uuid` | `v4`、`v5`、`v7`、`NIL` |
| `dotenv` | `config`、`parse` |
| `shell-quote` | `parse`、`quote` |
| `ms` | 时长解析 |
| `diff` | `diffChars`、`diffLines`、`createPatch` |
| `glob` | `glob`、`globSync` |

`jsonwebtoken` 支持有意限定于 HMAC JWT。RSA/ECDSA 算法与不支持的校验选项将失效关闭并给出明确诊断，而不会静默接受令牌。

### 运行时 API 兼容模块

| 包 | 关键导出 |
|---------|-------------|
| `openai` | `OpenAI`、默认 `OpenAI`、`chat.completions.create` |
| `adm-zip` | 默认 `AdmZip`、`getEntries`、`readAsText`、`extractAllTo`、`addFile`、`writeZip` |
| `linkedom` | `parseHTML`，含语料库扩展所用的 document/window 形态 |
| `@sourcegraph/scip-typescript` | `scip.Index`、默认 `{ scip }` |
| `@sourcegraph/scip-typescript/dist/src/scip.js` | `scip.Index`、默认 `{ scip }` |
| `@sourcegraph/scip-typescript/dist/src/main.js` | `main`、`run`、默认 `main` |

### 终端与 UI 模块

| 包 | 关键导出 |
|---------|-------------|
| `node-pty` | `spawn`（返回 PTY 存根） |
| `chokidar` | `watch`（返回 watcher 存根） |
| `@xterm/headless` | `Terminal` |
| `@xterm/addon-serialize` | `SerializeAddon` |
| `turndown` | `TurndownService` |
| `turndown-plugin-gfm` | `gfm`、`tables`、`strikethrough` |
| `@mozilla/readability` | `Readability`、`isProbablyReaderable` |
| `beautiful-mermaid` | `render` |
| `jsdom` | `JSDOM` |

### 可观测性模块

| 包 | 关键导出 |
|---------|-------------|
| `@opentelemetry/api` | `trace`、`context`、`propagation`、`SpanStatusCode` |
| `@opentelemetry/sdk-trace-base` | `BasicTracerProvider`、`SimpleSpanProcessor` |
| `@opentelemetry/resources` | `Resource` |
| `@opentelemetry/exporter-trace-otlp-http` | `OTLPTraceExporter` |
| `@opentelemetry/semantic-conventions` | `SEMRESATTRS_*` 常量 |

---

## 5. 扩展 API 表面（Pi 协议）

核心 Pi 扩展 API 已完全支持，这是扩展使用的主要 API。

### 注册

```javascript
export default function activate(pi) {
  // 注册工具
  pi.tool({ name: "my-tool", description: "...", schema: {}, run: async (input) => { ... } });

  // 注册斜杠命令
  pi.slashCommand({ name: "/my-cmd", description: "...", run: async (args) => { ... } });

  // 注册事件钩子
  pi.on("onMessage", async (event) => { ... });
  pi.on("onToolResult", async (event) => { ... });

  // 注册标志
  pi.flag({ name: "my-flag", description: "...", default: false });

  // 注册快捷键
  pi.shortcut({ name: "my-shortcut", key: "ctrl+k", run: async () => { ... } });

  // 注册提供方
  pi.registerProvider({ name: "my-provider", models: [...], streamSimple: async (model, context) => { ... } });
}
```

### 会话与状态 API

| API | 描述 |
|-----|-------------|
| `pi.session.getState()` | 获取当前会话状态 |
| `pi.session.getMessages()` | 获取会话消息 |
| `pi.session.getName()` / `setName()` | 会话名称 |
| `pi.session.getModel()` / `setModel()` | 活跃模型 |
| `pi.session.setLabel(key, value)` | 设置状态行标签 |
| `pi.session.getThinkingLevel()` / `setThinkingLevel()` | 思考级别 |
| `pi.events(op, payload)` | 分发生命周期事件 |

### 宿主工具

| API | 描述 |
|-----|-------------|
| `pi.tool(name, input)` | 调用内置工具（read/write/edit/bash/grep/glob/ls） |
| `pi.exec(command, args, options)` | 执行命令（能力门控） |
| `pi.http(request)` | HTTP 客户端（受策略控制） |
| `pi.log({level, event, message})` | 结构化日志 |

---

## 6. 能力策略

扩展对敏感 API 的访问由能力策略管控。

| 策略 | `exec` | `http` | `fs`（根外） | `env` |
|--------|--------|--------|---------------------|-------|
| `safe` | 拒绝 | 拒绝 | 拒绝 | 拒绝 |
| `balanced`（默认） | 允许 | 允许 | 告警 | 允许 |
| `permissive` | 允许 | 允许 | 允许 | 允许 |

使用 `pi doctor <path> --policy <profile>` 检查扩展在特定策略下的兼容性。

---

## 7. 预检分析

`pi doctor` 命令会在加载前对扩展进行静态分析：

```bash
# 文本输出（默认）
pi doctor /path/to/extension

# JSON 供自动化使用
pi doctor /path/to/extension --format json

# Markdown 供文档使用
pi doctor /path/to/extension --format markdown

# 针对特定策略检查
pi doctor /path/to/extension --policy safe
```

报告包含：
- **结论**：PASS / WARN / FAIL
- **置信度分数**：0-100 数值评分
- **风险横幅**：人类可读的摘要
- **发现项**：按类别、严重级别、消息与行号的逐项问题

---

## 8. 已知限制

### 模块解析

- **裸包说明符**（`import foo from "some-package"`）需要虚拟存根条目。使用未列入清单的 npm 包的扩展将无法加载。
- **相对导入**（`import ./utils`）在已打包的扩展内可用，但对于未打包的多文件扩展可能会失败。
- **网络导入**（`import "https://..."`）会被拒绝。

### 运行时约束

- **单线程**：无 `worker_threads`，无并行执行。
- **无原生插件**：无法加载 C/C++ Node 插件。请改用宿主调用或 WASM。
- **沙箱边界**：`createServer`、`listen` 等服务端 API 被阻断。扩展是客户端而非服务端。
- **文件系统作用域**：默认情况下，文件系统访问限定于扩展目录。目录外的读取需要显式的能力授予。

### 剩余失败分类（18/223）

| 类别 | 数量 | 根因 |
|----------|------:|------------|
| 多文件相对说明符 | 4 | 未打包的多文件扩展 |
| 包模块说明符 | 5 | 缺少虚拟存根的 npm 包 |
| 宿主读取策略拒绝 | 4 | 扩展根外的读取访问 |
| 运行时形态/加载错误 | 4 | 扩展结构不匹配 |
| 测试夹具制品 | 1 | 非真实扩展 |

---

## 9. 验证兼容性

### 面向扩展作者

```bash
# 检查你的扩展
pi doctor /path/to/your-extension

# 使用严格策略检查
pi doctor /path/to/your-extension --policy safe

# 查看支持的策略模式
pi --explain-extension-policy
```

### 面向 CI 集成

```bash
# 用于自动化检查的 JSON 输出
pi doctor /path/to/extension --format json | jq '.verdict'

# 退出码始终为 0（结论在输出中，而非退出码）
# 解析 JSON 的 verdict 字段以判定通过/失败
```

---

## 10. 问题反馈

若你的扩展加载失败且你认为其应当兼容：

1. 运行 `pi doctor /path/to/extension --format json` 并捕获输出。
2. 检查 findings 数组中的具体错误信息。
3. 若失败原因为缺失模块存根或垫片缺口，则可能属于 PiJS 运行时的修复范畴。
