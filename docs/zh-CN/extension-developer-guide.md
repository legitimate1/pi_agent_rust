# Pi QuickJS 扩展开发指南

> 本文档面向希望在 pi_agent_rust 中编写 QuickJS 扩展的开发者。
> 如果你想**使用**现有扩展（安装、配置），请运行 `pi help` 或查看相关 CLI 文档。

---

## 目录

1. [概述](#1-概述)
2. [架构速览](#2-架构速览)
3. [快速开始：Hello World](#3-快速开始hello-world)
4. [扩展结构](#4-扩展结构)
5. [生命周期与入口](#5-生命周期与入口)
6. [核心 API：注册能力](#6-核心-api注册能力)
7. [核心 API：宿主调用](#7-核心-api宿主调用)
8. [事件系统](#8-事件系统)
9. [工具设计指南](#9-工具设计指南)
10. [兼容层：Node.js 与 npm](#10-兼容层nodejs-与-npm)
11. [调试与测试](#11-调试与测试)
12. [从 TypeScript 迁移到 QuickJS](#12-从-typescript-迁移到-quickjs)
13. [参考与下一步](#13-参考与下一步)

---

## 1. 概述

Pi QuickJS 扩展允许你用 **JavaScript 或 TypeScript** 编写代码，向 pi_agent_rust 注册工具（Tool）、斜杠命令（Slash Command）、事件钩子（Event Hook）、快捷方式（Shortcut）、提供方（自定义模型）等能力。

### 适用场景

| 你想做什么 | 方式 |
|------------|------|
| 添加一个自定义工具（如 `my_search`） | QuickJS 扩展 |
| 监听智能体生命周期事件 | QuickJS 扩展 |
| 注册斜杠命令 | QuickJS 扩展 |
| 添加自定义模型提供方 | QuickJS 扩展 |
| 集成一个外部 API | QuickJS 扩展 |
| 实现高性能、低延迟的原子操作 | Rust 内置工具（见 `src/tools/`） |
| 需要完整的 Node.js 生态（大量 npm 依赖） | 不推荐；QuickJS 不支持裸 npm 包 |

### 前提条件

- pi_agent_rust 已编译运行
- 了解 JavaScript（扩展可用 TS，但运行时会编译为 JS）
- （可选）熟悉 [QuickJS](https://bellard.org/quickjs/) 沙箱约束

---

## 2. 架构速览

```
 扩展 JS/TS 源码（不受信任）          Rust 宿主（受信任）
┌──────────────────────────┐         ┌───────────────────────────────────┐
│                          │         │                                   │
│  export default function │         │  SWC 编译器                       │
│  activate(pi) {          │ ──────► │  (TS → JS 编译)                   │
│    pi.registerTool(...)  │         │                                   │
│    pi.on("startup", ...) │         │  模块解析器                       │
│    pi.tool("read", ...)  │         │  (相对路径 + node: 内置模块)      │
│  }                       │         │                                   │
│                          │ ◄────── │  QuickJS 运行时                   │
│                          │         │  (ES2020, 单线程, 沙箱化)         │
└──────────────────────────┘         │                                   │
       ↕  宿主调用通道               │  宿主调用桥                      │
       (Promise 式 RPC)               │  (ToolRegistry · subprocess ·     │
                                      │   HttpConnector · Session · UI)   │
                                      │                                   │
                                      │  能力策略 (capability policy)     │
                                      │  (safe / balanced / permissive)   │
                                      └───────────────────────────────────┘
```

### 关键概念

- **QuickJS**: 嵌入式 JavaScript 引擎，pi_agent_rust 用它运行扩展代码。不支持 `#` 私有字段、`worker_threads`、Web Worker。
- **SWC 编译**: 扩展入口若为 `.ts`，pi 在加载时会用 SWC 实时编译为 JS，无需预编译。
- **宿主调用**: 扩展通过 `pi.tool()`、`pi.exec()` 等方法向 Rust 宿主发起请求。每次宿主调用返回一个 `Promise`。
- **能力策略**: 扩展的敏感操作（exec、http、跨根 fs）由策略控制，防止恶意或错误代码造成破坏。

---

## 3. 快速开始：Hello World

### 3.1 创建目录结构

在 `~/.pi/agent/extensions/` 下创建你的扩展目录：

```
~/.pi/agent/extensions/
└── hello-world/
    ├── index.ts              # 入口文件
    └── package.json          # 扩展信息（可选但推荐）
```

### 3.2 编写扩展

`index.ts`:

```typescript
export default function activate(pi: any) {
  pi.registerTool({
    name: "hello_world",
    description: "向用户打个招呼",
    parameters: {
      type: "object",
      properties: {
        name: {
          type: "string",
          description: "要问候的人名",
        },
      },
      required: [],
    },
    execute: async (input: { name?: string }) => {
      const name = input?.name || "World";
      return {
        content: [{ type: "text", text: `Hello, ${name}!` }],
      };
    },
  });
}
```

`package.json`:

```json
{
  "name": "hello-world",
  "version": "0.1.0",
  "description": "Hello World 示例扩展",
  "pi": {
    "extensions": "index.ts"
  }
}
```

### 3.3 加载扩展

pi_agent_rust 启动时自动扫描 `~/.pi/agent/extensions/` 下的扩展目录，无需手动安装。如果扩展写在工作区目录下的 `.pi/extensions/`，也会被自动发现。

### 3.4 验证加载

```bash
pi doctor ~/.pi/agent/extensions/hello-world
```

或者查看智能体日志中的加载信息：

```
TRACE event="pijs.load_extension" extension_id="hello-world" entry="index.ts"
```

然后在对话中让 LLM 调用 `hello_world`：

> 「帮我调用 hello_world 工具，名字叫小明」

---

## 4. 扩展结构

### 4.1 标准目录布局

```
my-extension/
├── index.ts              # 入口（可被 SWC 实时编译）
├── package.json          # 扩展元信息
├── src/                  # 源码目录（可选）
│   ├── my-tool.ts
│   └── utils.ts
├── README.md
└── LICENSE
```

### 4.2 入口文件

pi 在加载扩展时按以下顺序查找入口：

1. `index.ts` → 用 SWC 编译为 JS 后执行
2. `index.js` → 直接由 QuickJS 执行
3. `index.mjs` / `index.cjs` / `index.jsx` / `index.tsx` / `index.mts` / `index.cts`

### 4.3 模块解析

扩展内可以相对导入自己的模块：

```typescript
// index.ts — 从 src/ 目录导入本地模块
import { helper } from "./src/helper.js";

// 简单扩展也可以直接在同文件全部写完
export default function activate(pi) {
  pi.registerTool({ name: "my_tool", ... });
}
```

模块解析规则：

| 写法 | 处理方式 |
|------|----------|
| `./path/to/file.js` | 相对路径解析 |
| `./path/to/file.ts` | 相对路径，SWC 编译 |
| `node:fs` | 映射到内置 Node.js 兼容 shim |
| `npm-package-name` | 映射到虚拟 stub（如果存在）或报错 |

**重要**: 不支持裸 npm 包导入——即 `import axios from "axios"` 会失败，除非该包有虚拟 stub。

### 4.4 package.json 字段

| 字段 | 说明 |
|------|------|
| `name` | 扩展名，建议用 `@scope/my-extension` 格式 |
| `version` | 语义化版本号 |
| `description` | 简单描述 |
| `pi.extensions` | 可选的入口声明，pi 实际**总是用 `index.{ts,js,...}`** |
| `files` | 发布时包含的文件列表 |

pi 目前不使用 `pi.extensions` 字段决定入口——入口总是 `index.ts`/`index.js`/等。`package.json` 主要用于元信息展示。

---

## 5. 生命周期与入口

### 5.1 activate 函数

每个扩展必须导出一个 `default` 函数作为激活入口：

```typescript
export default function activate(pi: PiAPI) {
  // 在这里注册工具、命令、钩子
}
```

`activate` 可以是 `async`：

```typescript
export default async function activate(pi: PiAPI) {
  const config = await loadConfig();
  pi.registerTool({ /* ... */ });
}
```

### 5.2 导出形状兼容（自动修复）

pi 运行时支持多种导出模式，以保证与各种遗留扩展的兼容性：

| 源写法 | 运行时自动处理 |
|--------|---------------|
| `export default function activate(pi) { }` | ✅ 标准形式 |
| `export function activate(pi) { }` | ✅ 识别命名导出 `activate` |
| `module.exports = { activate(pi) {} }` | ✅ CJS 形式，提取 `.activate` |
| `export default { activate(pi) {} }` | ✅ 对象形式的 `.activate` |
| 嵌套 `default` | ✅ 自动解包 `mod.default.default` |
| `init` / `initialize` / `setup` / `register` / `plugin` / `main` | ✅ 命名回退链 |

### 5.3 运行阶段

```
加载顺序：
1. SWC 编译入口（如果是 .ts）
2. 模块解析 + 依赖导入
3. 调用 activate(pi)
4. 扩展进入就绪状态
5. 触发 "startup" 事件

卸载顺序：
1. 清除注册的工具/命令/钩子
2. 释放资源
3. 扩展不可用
```

---

## 6. 核心 API：注册能力

所有的注册 API 都在 `activate(pi)` 中调用，接受 `pi` 对象上的方法。

### 6.1 `pi.registerTool(spec)`

注册一个工具（工具是 Pi 智能体的核心抽象，LLM 可以调用它）。

```typescript
pi.registerTool({
  name: "weather",                      // 工具名，全小写+下划线
  description: "获取指定城市的天气信息",  // 工具描述，LLM 选工具的依据
  parameters: {                         // JSON Schema
    type: "object",
    properties: {
      city: {
        type: "string",
        description: "城市名，如 北京、上海",
      },
    },
    required: ["city"],
  },
  execute: async (input) => {
    // input.city → "北京"
    return {
      content: [{ type: "text", text: "北京的天气是..." }],
    };
  },
});
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | `string` | 工具名，需唯一。全小写字母、数字、下划线，64 字符以内 |
| `label`? | `string` | 可读标签（用于 UI 展示） |
| `description` | `string` | 工具描述——这是 LLM 理解工具用途的唯一文档，务必写清楚 |
| `parameters` | `object` | [JSON Schema](https://json-schema.org/) 定义入参 |
| `execute` | `(input) => Promise<Result>` | 工具的执行函数 |

**返回值格式**：

```typescript
// 标准返回
{ content: [{ type: "text", text: "结果内容" }] }

// 带结构化数据
{ content: [{ type: "text", text: JSON.stringify(data, null, 2) }] }

// 空返回（执行成功但无有用信息应避免）
{ content: [{ type: "text", text: "" }] }
```

### 6.2 `pi.registerCommand(spec)`

注册斜杠命令（用户在输入框输入 `/` 时触发）。

```typescript
pi.registerCommand({
  name: "/my-command",
  description: "执行我的命令",
  handler: async (args: string) => {
    return {
      type: "text",
      text: `你传了参数: ${args}`,
    };
  },
});
```

### 6.3 `pi.registerProvider(spec)`

注册自定义模型提供方。实现 `streamSimple` 方法后，Pi 就可以用你的提供方作为模型后端。

```typescript
pi.registerProvider({
  name: "my-llm",
  models: [
    { id: "my-model-1", name: "我的模型" },
  ],
  async streamSimple(model, context, options) {
    // model: 模型 ID 字符串
    // context: 消息数组 [{role, content}]
    // options: { maxTokens, temperature, stream }
    // 返回 AsyncIterable<StreamEvent>
  },
});
```

### 6.4 `pi.registerShortcut(spec)`

注册快捷键。

```typescript
pi.registerShortcut({
  name: "my-shortcut",
  key: "ctrl+k",
  run: async () => {
    // 快捷键触发时的行为
  },
});
```

### 6.5 `pi.registerFlag(spec)`

注册功能开关。

```typescript
pi.registerFlag({
  name: "my-feature",
  description: "启用我的功能",
  default: false,
});
```

### 6.6 `pi.on(eventName, handler)`

注册生命周期钩子。详见[第 8 节](#8-事件系统)。

```typescript
const unsubscribe = pi.on("startup", async (event) => {
  console.log("扩展启动完成");
});
// 返回 unsubscribe() 函数可取消注册
```

### 6.7 `pi.registerMcpServer(spec)`

注册 MCP 服务器（Model Context Protocol）连接。

```typescript
pi.registerMcpServer({
  name: "my-mcp-server",
  command: "node",
  args: ["path/to/server.js"],
});
```

### 6.8 `pi.registerMessageRenderer(spec)`

注册自定义消息渲染器。

```typescript
pi.registerMessageRenderer({
  name: "my-renderer",
  render(message) {
    // 返回渲染后的内容
  },
});
```

### 6.9 辅助 API

| API | 说明 |
|-----|------|
| `pi.getFlag(name)` | 读取开关状态 |
| `pi.setActiveTools(names[])` | 设置当前启用的工具列表 |
| `pi.getActiveTools()` | 获取当前启用的工具列表 |
| `pi.getModel()` | 获取当前模型 |
| `pi.setModel(modelId)` | 切换模型 |
| `pi.getThinkingLevel()` | 获取思考级别 |
| `pi.setThinkingLevel(level)` | 设置思考级别 |
| `pi.getSessionName()` | 获取会话名称 |
| `pi.setSessionName(name)` | 设置会话名称 |
| `pi.setLabel(key, value)` | 设置状态栏标签 |
| `pi.sendMessage(content)` | 发送消息 |
| `pi.sendUserMessage(content)` | 模拟用户发送消息 |
| `pi.appendEntry(entry)` | 追加会话条目 |
| `pi.env.get(key)` | 获取环境变量（受 SecretBrokerPolicy 过滤） |

---

## 7. 核心 API：宿主调用

宿主调用是扩展向 Rust 宿主发起请求的通道，所有宿主调用返回 `Promise`。

### 7.1 `pi.tool(name, input)`

调用内置工具或其他扩展注册的工具。

```typescript
// 调用 read 工具读取文件
const result = await pi.tool("read", { path: "test.txt" });
console.log(result.content[0].text);

// 调用 grep 搜索
const grepResult = await pi.tool("grep", {
  pattern: "TODO",
  path: "./src",
});
```

| 工具 | 说明 |
|------|------|
| `read` | 读取文件 |
| `write` | 写入文件 |
| `edit` | 编辑文件（替换文本） |
| `bash` | 执行 shell 命令（需要 `exec` 能力） |
| `pwsh` | 执行 PowerShell 命令（需要 `exec` 能力） |
| `grep` | 搜索文件内容 |
| `find` | 按 glob 查找文件 |
| `ls` | 列出目录 |
| `hashline_edit` | 按行号+哈希精确编辑 |

### 7.2 `pi.exec(cmd, args, options?)`

执行外部命令。

```typescript
// 简单执行
const result = await pi.exec("node", ["-e", "console.log('hello')"]);
console.log(result.stdout);

// 流式执行（实时接收输出）
await pi.exec("npm", ["install"], {
  stream: true,
  onChunk: (chunk) => {
    console.log("进度:", chunk);
  },
});
```

**需要 `exec` 能力**，由扩展策略控制。

### 7.3 `pi.http(request)`

发起 HTTP 请求。

```typescript
const response = await pi.http({
  method: "GET",
  url: "https://api.example.com/data",
  headers: { Authorization: "Bearer xxx" },
});
console.log(response.data);

// 流式 HTTP
await pi.http({
  method: "GET",
  url: "https://api.example.com/stream",
  stream: true,
  onChunk: (chunk) => { /* 逐块处理 */ },
});
```

**需要 `http` 能力**，由扩展策略控制。

### 7.4 `pi.session(op, args?)`

读取和修改会话状态。

```typescript
const state = await pi.session("get_state", {});
const messages = await pi.session("get_messages", {});
const name = await pi.session("get_name", {});
```

### 7.5 `pi.ui(op, args?)`

与终端 UI 交互。

```typescript
await pi.ui("set_status", { text: "处理中..." });
```

### 7.6 `pi.events(op, args?)`

与事件系统交互。除了直接调用外还有 `pi.events.emit()` 和 `pi.events.on()` 便捷方法。

```typescript
// 触发一个事件（广播给其他扩展）
pi.events.emit("my-event", { data: "hello" });

// 监听其他扩展的事件
pi.events.on("my-event", (payload) => {
  console.log("收到事件:", payload);
});
```

### 7.7 `pi.log(entry)`

输出结构化日志。

```typescript
pi.log({
  level: "info",
  event: "my-extension.doSomething",
  message: "开始执行",
});
```

日志级别：`trace` / `debug` / `info` / `warn` / `error`。

### 7.8 `pi.sleep(ms)`

延迟（非宿主调用，纯 JS 侧）。

```typescript
await pi.sleep(1000); // 等待 1 秒
```

### 7.9 路径与进程工具

```typescript
// 路径操作
pi.path.join("a", "b", "c");    // "a/b/c"
pi.path.basename("/a/b/c.txt"); // "c.txt"
pi.path.normalize("a/../b");    // "b"

// 进程信息
pi.process.cwd;   // 当前工作目录
pi.process.args;  // 启动参数数组（只读）
```

---

## 8. 事件系统

Pi 提供两层事件机制：**生命周期钩子**和**扩展间事件总线**。

### 8.1 生命周期事件（`pi.on()`）

```typescript
pi.on("startup", async (event) => {
  // 扩展加载完毕后触发
});

pi.on("input", async (event) => {
  // 用户输入时触发
  console.log(event.data.payload);
});

pi.on("agent_start", async (event) => {
  // 智能体开始处理
});

pi.on("agent_end", async (event) => {
  // 智能体处理结束
});
```

**完整事件列表**：

| 事件名 | 触发时机 | 是否可阻塞 |
|--------|----------|-----------|
| `startup` | 扩展加载完成 | 否 |
| `input` | 用户输入 | 否 |
| `before_agent_start` | 智能体开始处理前 | 是 |
| `context` | 构建上下文时 | 是（可修改消息） |
| `agent_start` | 智能体开始处理 | 否 |
| `agent_end` | 智能体结束处理 | 否 |
| `turn_start` | 回合开始 | 否 |
| `turn_end` | 回合结束 | 否 |
| `message_start` | 消息开始 | 否 |
| `message_update` | 消息流式更新 | 否（合并） |
| `message_end` | 消息结束 | 否 |
| `tool_call` | 工具被调用前 | 是 |
| `tool_result` | 工具返回结果 | 是（可修改结果） |
| `tool_execution_start` | 工具执行开始 | 否 |
| `tool_execution_update` | 工具执行中 | 否（合并） |
| `tool_execution_end` | 工具执行结束 | 否 |
| `session_start` | 会话开始 | 否 |
| `session_switch` | 会话切换 | 否 |
| `session_fork` | 会话派生 | 否 |
| `session_compact` | 会话压缩 | 否 |
| `session_shutdown` | 会话关闭 | 否 |
| `model_select` | 模型切换 | 否 |
| `user_bash` | 用户执行 bash | 否 |

> 💡 **可阻塞**事件可以通过修改 `event.data` 来影响智能体行为。
> **合并**事件在短时间内会被批处理，不会每次触发。

### 8.2 扩展间事件总线（`pi.events.on()` / `pi.events.emit()`）

```typescript
// 扩展 A：发布事件
pi.events.emit("data-ready", { count: 42 });

// 扩展 B：订阅事件
const unsub = pi.events.on("data-ready", (payload) => {
  console.log("数据已就绪:", payload.count);
});

// 取消订阅
unsub();
```

事件总线的事件名**可以任意定义**，不做限制。但建议用带命名空间的事件名避免冲突，如 `my-ext.data-updated`。

---

## 9. 工具设计指南

工具的设计质量决定了 LLM 能否正确使用它。以下原则来自实际经验。

### 9.1 命名

- 全小写 + 下划线：`search_code`、`get_weather`
- 64 字符以内
- 动宾结构：`create_issue`、`list_users`，不要 `issue_creator`
- 避免缩写：`retrieve_document` 而非 `get_doc`

### 9.2 描述

描述是 LLM 决定**是否调用、何时调用**的唯一依据。

```typescript
// ❌ 差
{ description: "搜索代码" }

// ✅ 好
{ description: "在项目中搜索代码或文本。支持正则表达式和 glob 过滤。返回匹配行及其行号。不能做文件替换——如果用户要替换请告诉他们用 edit 工具。" }
```

好的描述准则：

- **告诉工具做什么**：匹配用户意图
- **告诉工具返回什么**：让 LLM 知道是否还需要额外调用
- **告诉工具不做什么**：防止 LLM 将这个工具误用于相近任务

### 9.3 参数

参数 Schema 直接嵌入 LLM 的上下文，写得越精确，LLM 传参越准确。

```typescript
// ❌ 松 —— LLM 可能传错
{ type: "string" }

// ✅ 紧——LLM 传对的概率更高
{ type: "string", enum: ["open", "closed", "all"] }
{ type: "number", minimum: 1, maximum: 100, default: 20 }
{ type: "string", pattern: "^usr_[a-z0-9]{12}$" }
```

**每个参数都要写 `description`**：

```typescript
properties: {
  limit: {
    type: "number",
    description: "返回结果数量上限（默认 20，最大 100）",
    default: 20,
    minimum: 1,
    maximum: 100,
  },
}
```

### 9.4 返回值

```typescript
// ✅ 结构化数据用 JSON
return {
  content: [{ type: "text", text: JSON.stringify(result, null, 2) }],
};

// ✅ 简洁确认
return {
  content: [{ type: "text", text: "已创建 issue #123" }],
};

// ✅ 截断提示
return {
  content: [{ type: "text", text: "显示了 10 条结果，共 847 条。请缩小搜索范围。" }],
};
```

**不要**：

- 返回裸 HTML
- 返回数 MB 未过滤的原始数据
- 返回没有标识符的 "ok"（LLM 后续无法引用）

### 9.5 工具数量

| 数量 | 建议 |
|------|------|
| 1–15 | 一个工具一个操作。最佳区间。 |
| 15–30 | 检查是否有可以合并的近似工具 |
| 30+ | 改用 搜索+执行 模式，只暴露 top 3–5 为独立工具 |

每个工具的 schema 都会消耗 LLM 上下文的 token。

### 9.6 错误处理

工具应返回错误信息，而不是抛出异常：

```typescript
execute: async (input) => {
  try {
    const result = await doSomething(input);
    return { content: [{ type: "text", text: JSON.stringify(result) }] };
  } catch (err) {
    return {
      content: [{ type: "text", text: `操作失败: ${err.message}` }],
      isError: true,
    };
  }
},
```

---

## 10. 兼容层：Node.js 与 npm

### 10.1 Node.js 内置模块

QuickJS 不运行 Node.js，但 Pi 为常用 Node.js 模块提供了兼容 shim：

| 模块 | 支持程度 |
|------|----------|
| `node:path` | ✅ 完整（POSIX 语义） |
| `node:fs` | ✅ `readFileSync`、`writeFileSync`、`statSync`、`mkdirSync`、`readdirSync` 等 19 个 API |
| `node:fs/promises` | ✅ 对应异步版本 |
| `node:crypto` | ✅ SHA-256/384/512、SHA-1、MD5、HMAC、randomUUID、Ed25519 |
| `node:buffer` | ✅ 完整 Buffer API |
| `node:child_process` | ✅ `spawnSync`、`execSync`、`spawn`、`exec`（需 `exec` 能力） |
| `node:http` / `node:https` | ✅ `request`、`get`（`createServer` 被阻止） |
| `node:events` | ✅ 完整 EventEmitter |
| `node:os` | ✅ `platform`、`hostname`、`tmpdir`、`homedir` 等 |
| `node:url` | ✅ URL、URLSearchParams |
| `node:process` | ✅ `env`、`argv`、`cwd`（`exit` 沙箱化） |
| `node:util` | ✅ `format`、`inspect`、TextEncoder/TextDecoder |
| `node:stream` / `node:stream/promises` | ✅ 流基础 API |
| `node:net` | ⚠️ 部分支持（stub，无网络 I/O） |
| `node:readline` | ⚠️ 部分支持（非交互时解析为空） |
| `node:vm` / `worker_threads` / `cluster` / `dgram` / `tls` | ❌ 被阻止 |

### 10.2 npm 包虚拟 Stub

Pi 为常见 npm 包提供了虚拟 stub，使依赖这些包的扩展可以正常加载（即使包本身未安装）：

| 包名 | 用途 |
|------|------|
| `@mariozechner/pi-coding-agent` | ExtensionAPI 类型定义 |
| `@sinclair/typebox` | 类型验证（`Type.String()` 等） |
| `uuid` | `v4`、`v5`、`v7` |
| `dotenv` | `config`、`parse` |
| `glob` | `glob`、`globSync` |
| `openai` | OpenAI 客户端 |
| `jsdom` | JSDOM |
| `adm-zip` | ZIP 读写 |
| `turndown` | HTML→Markdown |
| `shell-quote` | shell 参数解析 |

完整列表见 `docs/ext-compat.md`。

> 如果扩展用到的 npm 包没有 stub，加载会失败。解决方案：要么用 `pi 提 issue` 请求添加 stub，要么改用内置宿主调用（如 `pi.http()` 替代 `axios`）。

---

## 11. 调试与测试

### 11.1 `pi doctor`

快速检查扩展兼容性：

```bash
# 文本输出
pi doctor ~/.pi/agent/extensions/my-extension

# JSON 输出（适合 CI）
pi doctor ~/.pi/agent/extensions/my-extension --format json

# 按特定策略检查
pi doctor ~/.pi/agent/extensions/my-extension --policy safe
```

输出包含：

- **Verdict**: PASS / WARN / FAIL
- **Confidence**: 0–100 数值
- **Findings**: 逐个问题，含位置和严重级别

### 11.2 日志

Pi 使用 `tracing` 框架输出结构化日志。查看扩展加载和运行时信息：

```bash
# 启用详细日志
RUST_LOG=trace pi

# 只看扩展相关
RUST_LOG=pijs=debug pi
```

关键日志事件：`pijs.load_extension`、`pijs.register_tool`、`pijs.hostcall`、`pijs.repair.*`。

### 11.3 常见加载失败原因

| 症状 | 可能原因 | 解决 |
|------|----------|------|
| `Error: registerTool: spec.name is required` | 注册时传了空 name | 检查工具的 name 字段 |
| `Error: registerTool: tool name collision: xxx` | 工具名冲突 | 改名或在 `disabledTools` 中禁用冲突方 |
| `export default function activate` not found | 入口无默认导出 | 检查导出形状（见 5.2 节） |
| `import "unknown-package"` fails | npm 包无虚拟 stub | 改用宿主调用或添加 stub |

### 11.4 热重载

开发期间，你可以在 Pi 运行中更新扩展文件，然后**重启 pi 会话**即可生效。

目前尚无运行时热重载。如需要此功能可提 issue。

---

## 12. QuickJS 与 Node.js 的差异注意事项

如果你熟悉 Node.js 环境，在 Pi QuickJS 扩展中需要注意以下几点：

### 语法差异

| 项目 | Node.js | QuickJS |
|------|---------|---------|
| 私有字段 | `#field`（原生私有） | ❌ 不支持。用 TS `private` 代替（编译后擦除为普通属性） |
| 异步 | `async/await` | ✅ 完整支持 |
| ES module | `import/export` | ✅ 完整支持 |
| CommonJS | `require()` | ✅ 支持（通过 shim） |
| `worker_threads` | 多线程 | ❌ 单线程运行时，无 Worker |
| `Web Worker` | 浏览器 API | ❌ 不支持 |
| `Proxy` | 完整 | ✅ 支持 |
| `SharedArrayBuffer` | 支持 | ❌ 不支持 |

### 环境差异

| 操作 | Node.js | QuickJS |
|------|---------|---------|
| 文件读写 | `fs.readFileSync(...)` | ✅ 映射到内置 shim，作用域限制在扩展目录 |
| 子进程 | `child_process.spawn()` | ✅ 通过 `pi.exec()` 或 `node:child_process` shim（需 `exec` 能力） |
| 环境变量 | `process.env.X` | ✅ `pi.env.get("X")` 或 `node:process` shim（敏感变量被过滤） |
| HTTP 请求 | `fetch()` 或 `axios` | ✅ 用 `pi.http()` 或 `node:http` shim |
| 当前目录 | `process.cwd()` | ✅ `pi.process.cwd`（只读字符串） |
| 退出进程 | `process.exit()` | ⚠️ 沙箱化——不影响宿主进程 |
| `setTimeout` | 标准 | ✅ 完整支持（由 PiJS 事件循环驱动） |

### 全局变量

QuickJS 运行时中可用的全局变量：`console`、`setTimeout`、`setInterval`、`clearTimeout`、`clearInterval`、`Buffer`（Node.js 兼容 shim）。

不存在的全局：`global`（用 `globalThis` 代替）、`process`（用 `pi.process` 代替）、`fetch`（用 `pi.http()` 代替）、`WebSocket`。

## 13. 参考与下一步

### 参考文档

| 文档 | 内容 |
|------|------|
| `docs/ext-compat.md` | 完整兼容性矩阵（Node.js API、npm stub、能力策略） |
| `docs/extension-architecture.md` | 运行时架构（ExtensionManager、宿主调用路由） |
| `docs/extension-registry.md` | 扩展注册与发现 |
| `docs/context/features.md` | 功能清单 |

### 现有扩展参考

`~/.pi/agent/extensions/` 下的扩展可以作为参考示例：

| 扩展 | 亮点 |
|------|------|
| `read`（内置） | 工具设计参考 |
| `web-tool` | HTTP 调用集成 |
| `todo` | 状态管理和持久化 |
| `subagent` | 子进程管理和并行任务 |
| `proj-mgr` | 资源管理和注册 |
| `ast-grep` | AST 搜索和重写 |
| `login-custom` | 交互式配置引导 |

### 学习路径

1. 先用这个指南写一个简单的 Hello World 扩展
2. 阅读 `docs/ext-compat.md` 了解兼容性边界
3. 翻阅 `~/.pi/agent/extensions/` 下的现有扩展作为参考
4. 如果要发布，考虑提交到 Pi 扩展目录

---

> **遇到问题？**
> - 运行 `pi doctor` 检查扩展
> - 开启 `RUST_LOG=pijs=debug` 查看加载日志
> - 提 issue 到 pi_agent_rust 仓库
