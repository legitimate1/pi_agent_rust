# 扩展运行时兼容性矩阵

本文档为 Pi 扩展运行时提供全面的兼容性矩阵。涵盖 Node.js 内置模块垫片、Bun API 覆盖、npm 包存根以及 Pi SDK——扩展作者需要了解的关于在基于 QuickJS 的运行时中哪些功能可用的全部内容。

## Node.js 内置模块垫片

扩展运行时为标准的 Node.js 内置模块提供垫片。每个垫片都在 QuickJS 沙箱内运行，并通过由能力门控的宿主调用接口路由 I/O。

### 覆盖摘要

| 模块 | 覆盖率 | 关键 API |
|--------|----------|----------|
| `node:path` | 完整 | `join`, `dirname`, `resolve`, `basename`, `relative`, `isAbsolute`, `extname`, `normalize`, `parse`, `format`, `sep`, `delimiter`, `posix` |
| `node:os` | 完整 | `homedir`, `tmpdir`, `hostname`, `platform`, `arch`, `type`, `release`, `cpus`, `totalmem`, `freemem`, `uptime`, `loadavg`, `networkInterfaces`, `userInfo`, `endianness`, `EOL`, `devNull`, `constants` |
| `node:fs` | 部分 | `readFileSync`, `writeFileSync`, `existsSync`, `readdirSync`, `statSync`, `mkdirSync`, `rmdirSync`, `unlinkSync`, `renameSync`, `symlinkSync`, `readlinkSync`, `realpathSync`, `copyFileSync`, `appendFileSync`, `chmodSync`/`chownSync` 路径检查, `accessSync`, `lstatSync`, `openSync`, `closeSync`, `readSync`, `writeSync`, `fstatSync`, `createReadStream`, `createWriteStream`, `promises.*`, 回调变体 |
| `node:buffer` | 完整 | `Buffer.from`, `Buffer.alloc`, `Buffer.allocUnsafe`, `Buffer.concat`, `Buffer.isBuffer`, `Buffer.byteLength`, `toString(encoding)` (utf8, hex, base64, latin1, ascii) |
| `node:events` | 完整 | `EventEmitter` 类，包含 `on`, `once`, `off`, `emit`, `removeListener`, `removeAllListeners`, `listenerCount`, `listeners`, `prependListener` |
| `node:util` | 完整 | `inspect`, `promisify`, `callbackify`, `format`, `deprecate`, `inherits`, `debuglog`, `stripVTControlCharacters`, `types.*`, `TextEncoder`, `TextDecoder` |
| `node:child_process` | 完整 | `spawn`, `spawnSync`, `execSync`, `exec`, `execFile`, `execFileSync`, `fork` (存根) |
| `node:crypto` | 部分 | `randomBytes`, `randomUUID`, `createHash` (SHA-256/SHA-384/SHA-512, SHA-1, MD5), `createHmac`, `timingSafeEqual`, `getHashes`, `pbkdf2`/`scrypt`, AES-128-GCM/AES-256-GCM `createCipheriv`/`createDecipheriv`, Ed25519 `sign`/`verify` |
| `node:http` | 部分 | `request`, `get`, `createServer` (存根), `STATUS_CODES`, `METHODS`, `IncomingMessage`, `ClientRequest` -- 通过 `pi.http()` 宿主调用路由 |
| `node:url` | 部分 | `URL` (globalThis), `URLSearchParams`, `parse`, `format`, `resolve`, `fileURLToPath`, `pathToFileURL` |
| `node:stream` | 部分 | `Readable`, `Writable`, `Transform`, `PassThrough`, `Duplex`, `pipeline`, `finished` |
| `node:net` | 存根 | `createConnection` (存根), `Socket` (存根); `createServer` 抛出异常 -- 沙箱外的网络 API |
| `node:readline` | 部分 | `createInterface`, `promises.createInterface` -- 当可用时提问使用 `pi.ui('input')`，否则解析为空字符串 |

### `node:fs` 详情

文件系统垫片使用由内存中 `Map` 支持的虚拟文件系统（VFS）。宿主机文件系统访问通过回退机制提供，该机制将 `readFileSync` 和 `statSync` 调用通过宿主调用边界路由（由 `read` 能力门控）。

| API | 实现 | 备注 |
|-----|---------------|-------|
| `readFileSync` | 真实 | VFS 优先，对真实文件回退到宿主机 |
| `writeFileSync` | 真实 | 仅 VFS（沙箱化） |
| `existsSync` | 真实 | VFS + 宿主机回退 |
| `statSync` | 真实 | VFS + 宿主机回退 |
| `readdirSync` | 真实 | 仅 VFS |
| `mkdirSync` | 真实 | 仅 VFS，支持 `recursive` |
| `promises.readFile` | 真实 | 同步方法的异步包装 |
| `promises.writeFile` | 真实 | 同步方法的异步包装 |
| `promises.stat` | 真实 | 同步方法的异步包装 |
| `promises.mkdir` | 真实 | 同步方法的异步包装 |
| `promises.readdir` | 真实 | 同步方法的异步包装 |
| `promises.access` | 真实 | 同步方法的异步包装 |
| `promises.rm` | 真实 | 同步方法的异步包装 |
| `promises.rename` | 真实 | 同步方法的异步包装 |
| `promises.chmod`/`promises.chown`/`promises.utimes` | 部分 | 路径检查的无操作；缺失路径以 `ENOENT` 失败 |
| `openSync`/`closeSync`/`readSync`/`writeSync` | 真实 | 文件描述符 API |
| `readlink`/`readlinkSync` | 真实 | VFS 符号链接目标查找 |
| `chmodSync`/`chownSync` | 部分 | 路径检查的无操作；缺失路径以 `ENOENT` 失败 |
| `createReadStream` | 真实 | 返回 Readable 流 |
| `createWriteStream` | 真实 | 返回 Writable 流 |
| `watch`/`watchFile` | 存根 | 无真实文件监听；已存在路径返回无操作监听器外观，缺失路径以 `ENOENT` 失败 |
| 回调变体 (`readFile`, `writeFile` 等) | 真实 | 9 个回调风格函数 |

### `node:crypto` 详情

| API | 实现 | 备注 |
|-----|---------------|-------|
| `randomBytes(n)` | 真实 | 使用 QuickJS 随机源 |
| `randomUUID()` | 真实 | v4 UUID 生成 |
| `createHash(algo)` | 真实 | 通过原生宿主调用实现 SHA-256、SHA-384、SHA-512、SHA-1、MD5 |
| `createHmac(algo, key)` | 真实 | HMAC-SHA256、HMAC-SHA384、HMAC-SHA512、HMAC-SHA1、HMAC-MD5 |
| `timingSafeEqual(a, b)` | 真实 | 常量时间比较 |
| `getHashes()` | 真实 | 返回支持的算法列表 |
| `pbkdf2`/`scrypt` | 真实 | 通过原生宿主调用进行密钥派生 |
| `sign`/`verify` | 部分 | 使用 PKCS#8 私钥和 SPKI 公钥的 Ed25519；RSA/ECDSA 失效关闭 |
| `createCipheriv`/`createDecipheriv` | 部分 | 带 12 字节 IV、16 字节认证标签、可选 AAD 的缓冲式 AES-128-GCM 和 AES-256-GCM，不支持的算法失效关闭 |
| `createCipher`/`createDecipher` | 缺失 | 已弃用的基于密码派生的加密 API 未实现 |

### `node:child_process` 详情

所有子进程 API 都通过 `pi.exec()` 宿主调用路由，该调用需要 `exec` 能力。`exec` 能力在 `Standard` 和 `Safe` 策略配置中默认被拒绝。

| API | 实现 | 备注 |
|-----|---------------|-------|
| `spawn(cmd, args, opts)` | 真实 | 返回带 stdout/stderr 流的 ChildProcess |
| `spawnSync(cmd, args, opts)` | 真实 | 通过 `__pi_exec_sync_native` 同步执行 |
| `execSync(cmd, opts)` | 真实 | Shell 执行，返回 stdout |
| `exec(cmd, opts, cb)` | 真实 | 异步 Shell 执行 |
| `execFile(file, args, opts, cb)` | 真实 | 异步文件执行 |
| `execFileSync(file, args, opts)` | 真实 | 同步文件执行 |
| `fork(modulePath)` | 存根 | 沙箱中不支持 |

## Bun API 覆盖

运行时提供 `Bun` 全局对象，以兼容面向 Bun 运行时的扩展。

| API | 状态 | 备注 |
|-----|--------|-------|
| `Bun.argv` | 已支持 | 进程参数数组 |
| `Bun.file(path)` | 已支持 | 返回包含 `exists()`, `text()`, `arrayBuffer()`, `json()` 的对象 |
| `Bun.write(dest, data)` | 已支持 | 通过 `node:fs` 垫片写入文件 |
| `Bun.which(cmd)` | 已支持 | 通过 `which` 执行定位命令 |
| `Bun.spawn(cmd, opts)` | 已支持 | 通过 `node:child_process` 垫片 |
| `Bun.connect(...)` | 不支持 | 沙箱外的网络连接 |
| `Bun.listen(...)` | 不支持 | 沙箱外的服务端监听器 |

## npm 包存根

导入 npm 包的扩展会获得虚拟模块存根。这些存根提供足够的 API 覆盖，使扩展能够无错误地加载和注册。部分存根是功能性的（例如 `uuid`、`shell-quote`），而其他存根则对可选功能为无操作。

### 功能性存根

| 包 | 覆盖率 | 关键 API |
|---------|----------|----------|
| `uuid` | 功能性 | `v4()`, `v7()`, `v1()`, `v3()`, `v5()`, `validate()`, `version()` |
| `shell-quote` | 功能性 | `parse(cmd)`, `quote(args)` |
| `diff` | 功能性 | `createTwoFilesPatch`, `createPatch`, `diffLines`, `diffChars`, `diffWords` |
| `dotenv` | 功能性 | `config(opts)`, `parse(src)` |
| `ms` | 功能性 | 时长解析 (`ms("2h")` -> `7200000`) |
| `glob` | 部分 | `globSync`, `glob`, `Glob` 类（对 VFS 已知文件的基本 `*`, `?`, `**`） |
| `jsonwebtoken` | 部分 | `decode()`, HS256/HS384/HS512 `sign()`/`verify()`；非对称算法失效关闭 |

### 无操作存根（扩展可加载，功能不可用）

| 包 | 存根 API | 原因 |
|---------|-------------|--------|
| `chalk` | 透传（无颜色） | 终端颜色在 QuickJS 中不适用 |
| `chokidar` | `watch()` 返回无操作 | 文件监听不可用 |
| `jsdom` | `JSDOM` 类（空） | DOM 不可用 |
| `turndown` | `TurndownService` (透传) | HTML 转 Markdown 不可用 |
| `node-pty` | `spawn()` 返回无操作 | 沙箱中 PTY 不可用 |
| `@opentelemetry/*` | 无操作 span/metrics | 不收集遥测数据 |
| `@xterm/*` | 无操作终端 | 终端仿真不可用 |
| `vscode-languageserver-protocol` | 仅类型常量 | 为兼容性提供的 LSP 类型 |
| `@sinclair/typebox` | `Type` 模式构建器 | JSON Schema 构建 |
| `@modelcontextprotocol/sdk` | `Client`、传输类 | MCP 客户端存根 |
| `c12` (config loader) | `define()`, `loadConfig()` | 配置加载存根 |
| `execa` | `bash()` 返回空 | 改为通过宿主调用执行进程 |
| `@anthropic-ai/sdk` | `Anthropic` 类 | API 客户端存根 |
| `@anthropic-ai/bedrock-sdk` | `SandboxManager` | 沙箱管理器存根 |
| `openai` | `OpenAI` 类 | API 客户端存根 |
| `adm-zip` | `AdmZip` 类 | Zip 处理存根 |
| `linkedom` | `parseHTML()` | DOM 解析器存根 |
| `@sourcegraph/scip-typescript` | `scip.Index` 类 | 索引器存根 |

## Pi SDK (`@mariozechner/pi-coding-agent`)

Pi SDK 虚拟模块提供主要的扩展 API 覆盖。

| 导出 | 类型 | 描述 |
|--------|------|-------------|
| `keyHint(action, fallback)` | 函数 | 键盘提示显示辅助 |
| `compact(prep, model, key, instr, signal)` | 函数 | 上下文压缩 |
| `completeSimple(model, prompt, opts)` | 函数 | 简单 LLM 补全 |
| `fuzzyMatch(query, text, opts)` | 函数 | 模糊字符串匹配（返回 `{score, positions}`） |
| `fuzzyFilter(query, items, opts)` | 函数 | 按模糊匹配分数过滤条目 |
| `Text`, `Container`, `Markdown`, `Spacer` | 类 | TUI 渲染组件 |
| `Editor`, `Box`, `SelectList`, `Input` | 类 | TUI 输入组件 |
| `Image`, `DynamicBorder`, `CancellableLoader` | 类 | TUI 显示组件 |
| `Key` | 对象 | 按键绑定常量 |
| `CURSOR_MARKER` | 字符串 | 光标位置标记 |
| `truncateToWidth`, `visibleWidth`, `wrapTextWithAnsi` | 函数 | 文本渲染工具 |
| `getEditorKeybindings` | 函数 | 编辑器按键绑定配置 |
| `VERSION`, `DEFAULT_MAX_LINES`, `DEFAULT_MAX_BYTES` | 常量 | 运行时常量 |
| `truncateHead`, `truncateTail` | 函数 | 内容截断 |
| `parseSessionEntries`, `convertToLlm`, `serializeConversation` | 函数 | 会话数据工具 |
| `createBashTool`, `createReadTool`, `createWriteTool` 等 | 函数 | 工具工厂函数 |
| `getAgentDir`, `copyToClipboard` | 函数 | 系统工具 |
| `highlightCode`, `getLanguageFromPath` | 函数 | 代码显示辅助 |
| `AssistantMessageComponent`, `ToolExecutionComponent`, `UserMessageComponent` | 类 | 消息渲染 |
| `SessionManager` | 类 | 会话状态管理 |

### Pi AI SDK (`@mariozechner/pi-ai`)

| 导出 | 类型 | 描述 |
|--------|------|-------------|
| `StringEnum(values)` | 函数 | 枚举类型构建器 |
| `calculateCost()` | 函数 | 令牌成本计算（使用 `model.cost` × 使用令牌数） |
| `getEnvApiKey(provider)` | 函数 | 经能力过滤的环境密钥查找 |
| `getOAuthApiKey(provider)` | 函数 | 在 PiJS 中不支持；失效关闭 |
| `complete(model, messages, opts)` | 函数 | 当 `model.api` 匹配时通过已注册的 API 提供方运行；否则使用提供方宿主桥接，若不可用则失效关闭 |
| `completeSimple(model, prompt, opts)` | 函数 | 当 `model.api` 匹配时通过已注册的 API 提供方运行；否则使用提供方宿主桥接，若不可用则失效关闭 |
| `stream(model, context, opts)` | 函数 | 返回已注册 API 提供方的助手消息事件流，或为没有 API ID 的模型返回宿主桥接流 |
| `streamSimple(model, context, opts)` | 函数 | 返回已注册 API 提供方的简单助手消息事件流，或为没有 API ID 的模型返回宿主桥接流 |
| `createAssistantMessageEventStream()` | 函数 | 面向扩展提供方的异步可迭代助手事件流工厂 |
| `streamSimpleAnthropic()` | 函数 | 在没有提供方宿主桥接的情况下于 PiJS 中不支持；失效关闭 |
| `streamSimpleOpenAIResponses()` | 函数 | 在没有提供方宿主桥接的情况下于 PiJS 中不支持；失效关闭 |
| `streamSimpleOpenAICompletions()` | 函数 | 在没有提供方宿主桥接的情况下于 PiJS 中不支持；失效关闭 |
| `getProviders()`, `getModel(provider, modelId)`, `getModels(provider)` | 函数 | 对内置注册表中已打包提供方元数据的同步查找；目前包含扩展提供方镜像所需的 OpenAI Codex 模型 |
| `registerApiProvider(provider, sourceId)` | 函数 | 为兼容模型路由注册扩展自有的 `{api, stream, streamSimple}` 提供方 |
| `unregisterApiProviders(sourceId)` | 函数 | 仅移除调用扩展使用该源 ID 注册的 API 提供方 |
| `getApiProvider(api)`, `getApiProviders()` | 函数 | 返回已注册的 API 提供方；已知的内置 API ID 保留其同步宿主桥接回退 |
| `getModel()`, `getApiProvider()`, `getModels()` 无查找参数 | 函数 | 会话/模型宿主上下文辅助；当未配置宿主桥接时失效关闭 |

## 一致性状态

针对 223 个真实扩展语料测试：

| 来源层级 | 通过 | 失败 | 不适用 | 总计 | 通过率 |
|-------------|------|------|-----|-------|-----------|
| 官方 (pi-mono) | 56 | 4 | 6 | 66 | 93.3% |
| 社区 | 58 | 0 | 0 | 58 | 100%* |
| npm Registry | 63 | 12 | 0 | 75 | 84.0%* |
| 第三方 GitHub | 20 | 3 | 0 | 23 | 87.0%* |
| 智能体 | 1 | 0 | 0 | 1 | 100%* |

*社区/npm/第三方扩展通过兼容性验证包测试，该验证包使用更广泛的加载并注册测试，而非完整的差异预言机。

### 剩余失败分类

| 分类 | 数量 | 根因 |
|----------|-------|------------|
| 多文件依赖解析 | 4 | 跨文件的相对说明符尚未支持 |
| 缺失的 npm 包说明符 | 5 | 扩展导入了未存根的真实 npm 包 |
| 宿主读取策略拒绝 | 4 | 扩展读取了允许根之外的文件 |
| 运行时形态/加载错误 | 4 | 杂项加载时错误 |
| 测试夹具（非真实扩展） | 1 | `base_fixtures` 为测试基础设施 |

### 运行时 API 矩阵

| 覆盖面 | 通过 | 失败 | 总计 |
|---------|------|------|-------|
| Node.js API | 13 | 0 | 13 |
| Bun API | 5 | 2 | 7 |

失败的 Bun API：`Bun.connect` 和 `Bun.listen`（扩展沙箱外的网络服务端/客户端 API）。

## 不支持的功能

这些功能在扩展沙箱中有意不予支持：

| 功能 | 原因 | 替代方案 |
|---------|--------|-------------|
| 原生 Node 插件 (`.node`) | 二进制模块无法在 QuickJS 中运行 | 使用宿主调用或 WASM |
| 直接网络套接字 | 安全边界强制 | 使用 `pi.http()` 宿主调用 |
| 直接文件系统访问 | 为安全起见由能力门控 | 使用 `pi.tool("Read", ...)` 或 `node:fs` 垫片 |
| 服务端监听器 (`net.createServer`) | 扩展是客户端而非服务端 | 不适用 |
| 工作线程 | QuickJS 为单线程 | 不适用 |
| `node:cluster` | 进程集群不可用 | 不适用 |
| `node:dgram` | 沙箱外的 UDP 套接字 | 不适用 |
| `node:tls` | TLS 由宿主机 HTTP 客户端处理 | 使用带 HTTPS 的 `pi.http()` |

## 版本信息

- QuickJS 运行时：通过 `rquickjs` crate 嵌入
- Node.js API 目标：Node 18+ 兼容子集
- Bun API 目标：Bun 1.x 兼容子集
- 扩展 API 版本：`1.0`
