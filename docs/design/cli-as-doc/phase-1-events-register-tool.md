# CLI as Doc — Phase 1：events + register-tool 契约闭环

## 目标与背景

### 问题

当前 `docs/extension-developer-guide.md` 手写维护：

- 事件列表（名称、payload 说明、触发时机）
- `registerTool` 参数规格（字段名、类型、必填性）

而这些信息的物理来源分散在三处：

1. **Rust 类型定义**：`ExtensionEvent`（extension_events.rs）、`ExtensionEventName`（extensions.rs）、`ExtensionToolDef`（extensions_js.rs）
2. **嵌入 JS 验证逻辑**：`__pi_register_tool`（extensions_js.rs）的运行时检查和规整
3. **运行时事实**：`pi.on()` 实际接受的事件名列表、`__pi_register_tool` 实际接受和拒绝的调用

三者之间没有强制同步机制。新增枚举变体、字段或验证规则后，文档不会自动更新，也没有 CI 阻止静默过期。

### Phase 1 目标

1. 为 `events` 和 `register-tool` 各建立一个结构化契约
2. 证明新增事件或修改字段时，契约、运行时和 reference 输出三者不能静默漂移
3. 开发者可以通过以下方式查询当前安装版本的精确参考：

   ```bash
   pi developer-guide events
   pi developer-guide events tool_call
   pi developer-guide register-tool
   pi developer-guide register-tool --json
   ```

4. 输出风格与架构统一，不给后续 phase 增加迁移负担

### Phase 1 不覆盖

- `node-compat` 兼容语义（Phase 2）
- `hostcall` 方法参考（Phase 3）
- 静态 Markdown 生成器
- 旧文档中教程、概念和设计说明的完整迁移
- 国际化和翻译

> **重叠期处理**：Phase 1 完成后，在 `docs/extension-developer-guide.md` 顶部添加醒目注释：
> ```
> > ⚠️ API 参考（事件列表、registerTool 参数规格）已迁移到 `pi developer-guide`
> > 子命令。本文档中对应章节可能过时，以 CLI 输出为准。
> ```
> 这是唯一需要在 Phase 1 中修改旧文档的地方。不提前删除旧规格内容。

---

## 事件契约（Events Contract）

### 当前状态分析

当前代码中事件信息分布在两个不同的 enum 中：

| enum | 位置 | 变体数 | 用途 |
|:-----|:------|:-------|:-----|
| `ExtensionEvent` | `extension_events.rs:23` | 10 | 携带 payload 的 dispatch 事件，被序列化为 JSON 发送给 JS hooks |
| `ExtensionEventName` | `extensions.rs:16080` | 28 | 生命周期事件名称，用于 dispatch_event 路由和 is_informational 分类 |

两者关系：

- `ExtensionEventName` 更完整（28 vs 10），包含 `Input`、`MessageStart/Update/End`、`ToolExecutionStart/Update/End`、`SessionStart/Switch/Fork/Compact/Shutdown`、`ResourcesDiscover`、`ModelSelect`、`UserBash`、`SessionBeforeTree`、`SessionTree` 等**仅用于通知**的事件
- `ExtensionEvent` 是**可携带 payload 的事件子集**，通过 dispatch_with_context/dispatch 发给 JS
- 两者通过字符串名称关联——`ExtensionEvent::event_name()` 返回字符串，与 `ExtensionEventName::Display` 输出一致（不完全相同？需要进一步验证）
- `pi.on()` 当前**不验证**事件名是否合法——任意字符串都可注册 hook

### 契约设计

#### 事实（机械生成的）

以下信息必须由代码派生，不允许在契约中手写：

```rust
// 目标：事件契约条目
struct EventContractEntry {
    /// 事件名（蛇形），也是 pi.on() 和 dispatch 使用的字符串
    name: &'static str,

    /// 是否有可序列化的 payload（rust 侧定义的 ExtensionEvent 变体）
    has_payload: bool,

    /// 是否在 ExtensionEvent 中有对应的 dispatch 入口
    has_dispatch: bool,

    /// 是否在 ExtensionEventName 中有对应的生命周期路由
    has_lifecycle_name: bool,

    /// 事件分类：informational（仅通知）/ actionable（可阻断或修改）
    kind: EventKind,
}
```

**权威来源**：契约在 Rust 侧声明，是**实现与文档之间的可验证投影层**。契约条目通过以下机制与实现保持一致性：

- **编译期约束**：穷尽 match 确保新增枚举变体必须在契约构造中显式处理——这是契约/实现一致性层的组成部分，而非真相的唯一来源。
- **一致性测试**：CI 中运行的测试验证契约投影与实际运行时行为对齐（见「运行时探针」和「分层门禁」）。
- **语义描述**：人维护的摘要、触发时机、稳定性等，在契约条目中与机械事实并列——运行时无法推断这些信息。

> **Phase 1 决策**：契约数据不从 enum 自动生成，enum 也不从契约数据生成。不修改现有 enum 定义。编译期检查和运行时测试构成双向一致性网，而非单一的「同步」关系。

**设计选择**：辅助函数 + 穷尽 match + 一致性测试。不引入过程宏或 `include!`，不修改现有 enum 定义。

#### 语义（人工维护，绑定到条目）

> **为什么语义必须人工维护**：Rust 契约承载的公共语义——稳定性承诺、能力要求、使用限制、精炼的人类描述——是运行时无法推断的。类型系统能表达什么是可调用的，但无法表达什么是推荐的、什么是废弃的、什么需要特定权限。这些信息必须在契约中与机械事实并列。

每个事件条目附带以下信息，**不和实现代码放在一起，但紧邻契约**：

```rust
struct EventSemantics {
    /// 中文摘要
    summary: &'static str,

    /// 触发时机说明
    when: &'static str,

    /// payload 的 JSON 结构说明（非精确 schema，而是人类可读的描述）
    ///
    /// 约束：当 entry.has_payload == false 时，此值必须为 None；
    /// 当 entry.has_payload == true 时，此值必须为 Some。
    /// 由构造器或测试断言验证此约束。
    payload_description: Option<&'static str>,

    /// handler 返回值说明（如果有）
    return_description: Option<&'static str>,

    /// 稳定性：stable / experimental / deprecated
    stability: &'static str,
}
```

### 验收标准

1. `ExtensionEventName` 新增变体 → 穷尽匹配强制要求补充到契约表 → 否则编译失败
2. `ExtensionEvent` 新增变体 → 同上
3. CLI `events` 输出只从契约表投影，renderer 不手写任何事件名
4. JSON `--json` 输出所有契约条目
5. 每个声明的事件必须通过运行时探针证明其 Rust dispatch 路径可观测

> **Phase 1 关于 `pi.on()` 验证的决策**：保持 `pi.on()` 宽松（任意字符串均可注册），不对未知事件名做运行时错误。每个声明的事件必须通过运行时探针证明：注册 handler 后触发对应的 Rust dispatch，探针能捕获到执行。文档定义的事件集以**实际可分派/可观测**的事件为准，而非 `pi.on()` 接受的字符串集合。如果当前代码中的事件计数/映射关系存在不确定之处，保守表述，不凭空声称行为。

---

## 工具注册契约（Register-Tool Contract）

### 当前状态分析

`registerTool` 的契约横跨：

| 来源 | 内容 |
|:-----|:------|
| `ExtensionToolDef` struct | `name: String`, `label: Option<String>`, `description: String`, `parameters: Value` |
| `__pi_register_tool` JS 验证 | `spec` 必须是对象、`name` 必填且非空、`execute` 必须是函数、`description` 转为字符串默认为空、`parameters` 默认 `{ type: 'object', properties: {} }`、`label` 仅在 string 时写入 |
| 扩展工具包装器 | `extension_tools.rs` 中验证、收集、处理后作为 Tool trait |
| 现有指南 | 手写 `name`/`description`/`label`/`parameters`/`execute` 解释和注意事项 |

**关键问题**：

- Rust struct 用 `#[serde(default)]` 表达可选性，但 JS 侧有自己的默认逻辑
- `execute` 回调是 JS 函数，不出现在 Rust struct 中（本质上是注册的行为）
- `parameters` 的类型是 `serde_json::Value`，没有强约束——JS 侧默认填充一个基础 JSON Schema，但实际上由开发者自由传入

### 契约设计

#### 事实（机械生成）

```rust
struct RegisterToolContractEntry {
    /// 字段名
    field: &'static str,

    /// 类型描述（人类可读）
    field_type: &'static str,

    /// 是否必填
    required: bool,

    /// 默认值（字符串描述）
    default: Option<&'static str>,

    /// 验证约束描述
    validation: Option<&'static str>,
}
```

**权威来源**：字段注册表在 Rust 侧定义，与 `ExtensionToolDef` 的反序列化规则同源。`__pi_register_tool` 的 QuickJS 验证行为通过运行时探针验证：对每个字段的接受/默认/拒绝路径，测试通过 QuickJS 运行时实际调用 `__pi_register_tool`，以实际行为作为验证目标。契约元数据（字段名、类型、默认值说明）自身不能作为行为一致性的充分证据——运行时探针是关键的安全网。

#### 语义（人工维护，绑定到条目）

> **为什么语义必须人工维护**：字段的稳定性承诺、行为约束的含义、与其他 API 的关联——这些是 Rust 类型系统和 QuickJS 运行时都无法推断的公共语义。必须在契约中显式声明。

```rust
struct RegisterToolSemantics {
    /// 中文说明
    summary: &'static str,

    /// 用法注意事项
    notes: Option<&'static str>,

    /// 与其他 API 的关系
    see_also: Option<&'static str>,

    /// 稳定性
    stability: &'static str,
}
```

`execute` 字段特殊处理——它不出现在字段表中，而是在语义层说明其签名和约束：

```text
execute(input, context?) → Promise<object>
```

### 验收标准

1. `ExtensionToolDef` 新增字段 → 必须在字段注册表中补充 → 否则编译失败或其他检验失败
2. `__pi_register_tool` 新增验证规则 → 必须在注册表的 `validation` 中体现 → 否则测试失败
3. CLI 文本输出不手写字段表，只从注册表投影
4. JSON `--json` 输出完整字段注册表
5. 对每个字段至少有一个有效和无效输入测试（测试通过 QuickJS 运行时探针执行，以 `__pi_register_tool` 的实际行为为验证目标）

#### `execute` 字段在 ReferenceEntry 中的表示

`execute` 不出现在 `RegisterToolContractEntry` 字段表中，而是作为独立的 `ReferenceEntry` 存在：

| 属性 | 值 |
|:-----|:----|
| id | `"execute"` |
| signature | `(input, context?) → Promise<object>` |
| fields | 空（不是对象字段，而是回调函数）|
| constraints | `["必须是函数"]` |
| summary | 函数签名的中文说明 |

数据来自 `RegisterToolSemantics`，其字段注册表中不含 `execute`，但 `RegisterToolContract` 中会为它生成一个语义条目。

---

## 统一参考模型（DeveloperReference）

所有 topic 在进入展示层前先投影为统一的参考模型。renderer 只消费此模型，不直接读取领域契约。

```rust
/// @schema_version 1
/// @experimental (schema may change without notice in Phase 1)
struct DeveloperReference {
    schema_version: String,          // "1"
    binary_version: String,          // 从 env!("CARGO_PKG_VERSION") 获取
    binary_sha: Option<String>,       // 从 built-time SHA 获取（如果有）

// 实现注记：binary_sha 的直接来源是 built::git::commit_id() 
// 或 env!("VERGEN_GIT_SHA")。src/rpc.rs 中的 get_version 方法  
// 已有获取 Git SHA 的现成实现，可直接参考。
    language: String,                 // "zh-CN"
    topic: ReferenceTopic,
    entries: Vec<ReferenceEntry>,
}

enum ReferenceTopic {
    Events,
    RegisterTool,
    // 后续：NodeCompat, Hostcall, RegisterCommand, RegisterProvider
}

struct ReferenceEntry {
    /// 唯一标识（topic 内不重复）
    id: String,

    /// 中文摘要
    summary: String,

    /// 签名（如有）
    signature: Option<String>,

    /// 字段列表（如有）
    fields: Vec<ReferenceField>,

    /// 约束/限制
    constraints: Vec<String>,

    /// 兼容性说明
    compatibility: Option<String>,

    /// 稳定性
    stability: String,

    /// payload 描述（events 专用）
    payload_description: Option<String>,
}

struct ReferenceField {
    name: String,
    field_type: String,
    required: bool,
    default: Option<String>,
    summary: String,
    validation: Option<String>,
}
```

### 设计原则

1. **不提前抽象**：模型只覆盖 `events` 和 `register-tool` 的需求。进入 Phase 2 前可以向后兼容扩展，不承诺字段稳定。
2. **不序列化 Rust 内部对象**：`DeveloperReference` 是显式投影，不是 `serde(Serialize)` 注解的领域对象。
3. **schema_version 独立**：JSON 输出始终携带 `schema_version` 字段，Phase 1 标记为 `@experimental`。
4. **language 声明**：`"zh-CN"`，renderer 不根据系统语言切换。

---

## CLI 子命令设计

### 命令树

```bash
pi developer-guide
pi developer-guide <topic>
pi developer-guide <topic> <id>
pi developer-guide <topic> --json
pi developer-guide <topic> <id> --json
```

### 命名映射规则

三种上下文使用不同的命名风格，映射在 topic 路由层集中处理：

| 上下文 | 风格 | 示例 |
|:-------|:-----|:------|
| CLI 参数 | kebab-case | `register-tool` |
| Rust enum 变体 | PascalCase | `RegisterTool` |
| JSON `topic` 字段 | snake_case | `"register_tool"` |

映射在 `developer_guide` 模块的 topic 路由层（`mod.rs`）集中处理，不分散到各 renderer。Phase 1 所有 topic 的映射在此集中定义。

### Topic 列表

| topic | 对应契约 | Phase |
|:------|:---------|:------|
| `events` | EventContract | 1 |
| `register-tool` | RegisterToolContract | 1 |
| `register-command` | 待定 | 未来 |
| `node-compat` | 待定 | 2 |
| `hostcall` | 待定 | 3 |

### 文本输出示例

```
$ pi developer-guide events

事件列表（共 28 个）

工具调用前触发 (tool_call)
  触发时机：在每次工具执行之前触发。handler 可以返回
  { block: true, reason: "..." } 来阻止本次执行。
  稳定性：stable
  Payload: { toolName, toolCallId, input }
  返回值：{ block?: boolean, reason?: string }

工具执行后触发 (tool_result)
  触发时机：在每次工具执行之后触发。handler 可以返回
  修改后的 content 来覆盖原始结果。
  稳定性：stable
  Payload: { toolName, toolCallId, input, content, details, isError }
  返回值：{ content?: ContentBlock[], details?: any }

...
```

```
$ pi developer-guide events tool_call

工具调用前触发 (tool_call)

触发时机：在每次工具执行之前触发。handler 可以返回
{ block: true, reason: "..." } 来阻止本次执行。

稳定性：stable
有 Payload：是
可通过 pi.on 监听：是
有 dispatch 入口：是

Payload 字段：
  toolName    string  必填  工具名称
  toolCallId  string  必填  调用 ID
  input       any     必填  工具输入参数

返回值：
  block    boolean  可选  如果为 true，阻止工具执行
  reason   string   可选  阻止原因
```

```
$ pi developer-guide register-tool

registerTool(spec)

注册一个扩展工具。必须在 activate 阶段调用。

参数字段：
  name         string  必填  工具名称（蛇形命名，全局唯一）
  description  string  可选  工具描述（默认为空）
  label        string  可选  人类可读的显示名称
  parameters   object  可选  JSON Schema 定义参数（默认 { type: "object", properties: {} }）
  execute      function 必填  工具执行函数

验证规则：
  - spec 必须是对象
  - name 不能为空
  - execute 必须是函数
  - 同名的工具如果属于不同扩展则抛出碰撞错误

稳定性：stable
```

```
$ pi developer-guide register-tool --json

{
  "schema_version": "1",
  "binary_version": "0.35.0",
  "language": "zh-CN",
  "topic": "register_tool",
  "entries": [
    {
      "id": "name",
      "summary": "工具名称",
      "signature": "string",
      "fields": [],
      "constraints": ["必填", "不能为空字符串"],
      "stability": "stable"
    }
  ]
}
```

### 实现策略

**不做**：

- 不内置复杂分页或交互式导航
- 不根据终端宽度自适应换行（文本 renderer 按 ~80 字符习惯输出，不由 TUI 承担）
- renderer 不存储领域逻辑

**做的方式**：

- `developer_guide` 子命令实现在一个独立的模块（`src/developer_guide.rs` 或 `src/cli_ref/`）
- 领域契约（`EventContract`、`RegisterToolContract`）各自独立模块
- `DeveloperReference` 模型和投影逻辑独立一个模块
- 文本 renderer 和 JSON renderer 各一个模块

#### 模块结构

```text
src/
├── developer_guide/
│   ├── mod.rs               # 子命令入口，topic 路由
│   ├── contract/
│   │   ├── mod.rs
│   │   ├── events.rs        # EventContract + projection to DeveloperReference
│   │   └── register_tool.rs # RegisterToolContract + projection
│   ├── model.rs              # DeveloperReference 结构体
│   ├── text_renderer.rs      # 文本输出
│   └── json_renderer.rs      # JSON 输出
```

---

## 一致性门禁

根据节点 8 决策，采用分层门禁：

### 第 0 层：编译期约束（最强制）

- `ExtensionEventName` 的 `match`（`is_informational`）已经做到穷尽
- 事件契约表的构造应当利用同样的穷尽性——用 `match` 或宏确保新增变体必须补充条目
- `RegisterToolContract` 检入时验证 `ExtensionToolDef` 的字段列表与契约表一致

**实现方式**（Phase 1 评估两种）：

1. **过程宏**：为一个 enum 生成所有变体的列表，与手写表比较——灵活但复杂
2. **辅助函数 + tests**：在 `match` 中显式列出所有变体，测试验证条目数与 enum 变体数一致——更简单，但需要手动追平

**推荐**：Phase 1 先用方案 2（辅助函数 + 测试），根据实际体验决定是否升级为宏。

### 第 1 层：运行时一致性测试（含运行时探针）

```rust
// 每个注册的事件名都能在 ExtensionEventName 中找到
// 每个 ExtensionEventName 变体都有一个契约条目
// 每个 ExtensionEvent 变体的 event_name() 返回值在契约中
// registerTool 的字段表与 ExtensionToolDef 字段匹配
// 运行时探针：每个声明事件注册 handler 后 dispatch 可观测
// 运行时探针：register-tool 字段的实际 QuickJS 行为与契约一致
```

这些测试放置在 `src/developer_guide/` 的 `tests` 子模块或集成测试中。运行时探针在测试中启动最小 QuickJS 运行时，不引入生产级的 introspection API。

### 第 2 层：行为约束测试

```rust
// registerTool 字段验证：必填字段缺失时抛出错误
// registerTool 字段验证：无效类型时抛出错误
// 对每个字段，至少有一个成功路径测试
```

这些测试通过 QuickJS 运行时探针直接调用 `__pi_register_tool`，验证字段级别的接受/默认/拒绝行为与契约声称一致。不依赖契约元数据作为行为充分的证据。

### 第 3 层：Renderer 输出测试

```rust
// 文本 renderer 输出包含预期的事件名
// JSON renderer 输出包含 schema_version
// JSON 输出可反序列化为 DeveloperReference
```

### 门禁优先级

当一个机械事实发生变化时：

1. **共享或生成**优先：尽量让运行时和契约共用同一份数据定义
2. 做不到共享时，用**生成**：从一个来源生成另一个
3. 做不到生成时，用**测试**：CI 运行一致性测试检测漂移

---

## 注意事项

### 实现优先级

1. 先定义领域契约数据结构（EventContract, RegisterToolContract）
2. 再实现 DeveloperReference 模型和投影逻辑
3. 再实现 CLI 子命令和 renderer
4. 再补充一致性测试
5. 最后接入 clap 子命令树

### 不引入的依赖

- 不引入 Rust 反射或 syn/syn 解析来提取 doc comment
- 不引入生成时的构建脚本

### 运行时探针边界

Phase 1 **不提供公开的生产级运行时自省 API**：`pi developer-guide` 的输出来自 Rust 契约投影，而非 QuickJS 运行时枚举。

但测试和 CI 中**可以使用最小 QuickJS 运行时探针**来验证契约投影的真实性：

- **事件**：对每个声明的事件，测试启动运行时、注册 handler、确认对应的 Rust dispatch 路径在探针中可观测。
- **工具注册**：对 `register-tool` 的每个字段，测试通过 QuickJS 实际调用 `__pi_register_tool`，验证接受/默认/拒绝行为与契约声称一致——契约元数据本身不能作为充分证据。
- **门禁定位**：运行时探针是契约的"安全网"，确保投影在 CI 管道中不会与运行时事实漂移。探针不是 `pi developer-guide` 输出的数据源，而是其真实性的验证者。

### Compile time impact

- 事件契约表是少量静态数据（~28 条目），对编译时间影响可以忽略
- 宏未被引入，编译路径不受影响

### 与现有事件 dispatch 的关系

不改变 `dispatch_event` 和 `pi.on()` 的运行时行为。Phase 1 的文档事件集以运行时探针验证的实际 dispatch/可观测性为准；是否收紧未知事件名的注册行为是独立的未来行为变更，不在本阶段预设。

### 与现有 registerTool 的关系

不改变 `ExtensionToolDef` 结构体、`__pi_register_tool` JS 函数。只在 Rust 侧增加契约注册表和一组合性测试。验证逻辑与契约的一致性由测试保证，不强制运行时共享。

---

## 待讨论

1. `ExtensionEvent` 和 `ExtensionEventName` 的字符串名称映射是否需要自动对齐？当前两者有成对的关系，也有一些不在 ExtensionEvent 中却有 ExtensionEventName 的事件。
2. JSON `--json` 的 schema_version 策略：Phase 1 标记为实验性，是否需要在版本号中嵌入 major.minor？
