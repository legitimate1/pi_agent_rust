# pi_agent_rust — 编辑后轻量验证系统（Verify）

## 目标与背景

Pi Agent 编辑文件后，没有自动的格式/语法验证环节。Agent 需要手动跑检查（如 `cargo clippy`、`rustfmt --check` 等）才能确认文件无问题——每次多一步来回。

本设计为编辑工具增加一个可选的验证环节，让 Agent 在编辑后**立即获得格式检查反馈**，减少迭代轮次。

### 核心约束

- **不自动修正**——所有检查器只报告不修改（铁律）
- **默认关闭**——`verify` 参数默认 `false`，由 Agent 在需要时主动开启
- **零新增对外接口**——不新增 LLM-visible tool，不新增 CLI 子命令
- **仅轻量检查器**——语法/格式化级别，不含编译/类型检查

## 决策记录

以下为本功能的关键决策点及其选择：

| 决策点 | 选择 | 理由 |
|:-------|:-----|:------|
| 功能形态 | 工具参数集成 | `edit`/`hashline_edit`/`write` 增加可选 `verify` 参数 |
| 检查器发现策略 | 文件类型直接映射 | 不扫描项目配置，确定性高、零扫描成本 |
| `.json` 检查器 | 进程内 `serde_json::from_str()` | `serde_json` 已在项目依赖中，零额外开销 |
| `.rs` 检查器 | 外部 `rustfmt --check` | 重量级 crate 不值得嵌入；随 Rust 工具链自带 |
| `.ts` 检查器 | 外部 `npx prettier --check` | 轻量格式化检查；无 Node 环境时报警告并跳过 |
| `.toml` 检查器（MVP） | 进程内 `toml::from_str::<toml::Value>()` | `toml` 作为正式依赖（从 dev-deps 提升） |
| 自动修正 | 禁止（铁律） | 所有检查器只报告不修改 |
| 是否暴露为独立工具/CLI | 否 | LLM 可通过 `pwsh` 做手动批量检查 |
| `verify` 参数默认值 | `false`（关） | 避免中间态误报和批量编辑性能损耗 |
| `effects()` 声明 | 不变更（`WRITE`） | WRITE 本身已是 barrier，调度器不会并发执行写入 |
| 工具路径预检 | 不做 | 报错时给出清晰提示信息即可 |
| 工具自动修正 | 禁止（铁律） | 所有检查器只报告不修改 |

### 文件类型 → 检查器映射表（MVP）

| 扩展名 | 检查器 | 实现方式 | 预期耗时 | 大小阈值 | 降级行为 |
|:-------|:-------|:---------|:---------|:---------|:---------|
| `.rs` | `rustfmt --check` | 外部进程 `Command` | ~0.5s | >1MB 跳过 | 工具不可用 → 报 error |
| `.json` | `serde_json::from_str::<Value>()` | 进程内解析 | ~0.001s | 无限制 | 无降级（零依赖） |
| `.toml` | `toml::from_str::<toml::Value>()` | 进程内解析 | ~0.001s | 无限制 | 无降级（零依赖） |
| `.ts` | `npx --no-install prettier --check` | 外部进程 `Command` | ~0.5s | >1MB 跳过 | 无 Node 或 prettier 未缓存 → 警告并跳过 |

#### 关于 `.ts` 检查器

使用 `npx --no-install prettier --check` 而非 `npx prettier --check`：
- `--no-install` 确保 prettier 未被缓存时不自动下载，避免网络延迟打破「轻量检查」的语义
- prettier 未缓存时，报警告并跳过，不阻断编辑流程

#### 关于大文件阈值

超过 **1MB** 的文件跳过外部进程验证（进程内 JSON/TOML 验证不受限），防止大文件编辑后的验证耗时过长。

## 架构设计

### 模块位置

新增内部模块 `src/tools/verify.rs`（不对外暴露），被现有工具调用。

```
src/tools/
├── mod.rs
├── edit.rs        # + verify 参数
├── hashline.rs    # + verify 参数
├── write.rs       # + verify 参数
├── verify.rs      # 新增：内部验证引擎
└── ...
```

### 内部 API

```rust
// src/tools/verify.rs

/// 文件类型分类
enum FileType {
    Rust,
    Json,
    Toml,
    TypeScript,
}

/// 单文件验证结果
struct VerifyResult {
    path: PathBuf,
    file_type: FileType,
    passed: bool,
    /// 错误/警告信息（passed=true 时为 None）
    message: Option<String>,
    /// 检查器名称（如 "rustfmt" / "serde_json" / "prettier"）
    checker: &'static str,
    /// 验证耗时（毫秒）
    time_ms: u64,
}

/// 对单个文件运行验证（不对外暴露）
pub fn verify_file(path: &Path, abort: Option<AbortSignal>) -> Result<VerifyResult>;
```

### 工具参数变更

三个编辑工具各增加一个可选参数。以 `edit` 为例：

```json
{
  "name": "edit",
  "parameters": {
    "path": { "type": "string" },
    "oldText": { "type": "string" },
    "newText": { "type": "string" },
    "verify": {
      "type": "boolean",
      "description": "编辑后自动运行语法检查（.rs → rustfmt --check, .json/.toml → 进程内 JSON/TOML 解析, .ts → prettier --check）。依赖工具需在 PATH 中可用。文件 >1MB 时跳过外部进程验证。默认 false。",
      "default": false
    }
  }
}
```

对应的 Deserialize 结构体变更：

```rust
// edit.rs
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditInput {
    path: String,
    old_text: String,
    new_text: String,
    #[serde(default)]  // 默认 false
    verify: bool,
}
```

同样的变更应用到 `WriteInput` 和 `HashlineEditInput`。

### 执行流程

```
Agent 调用 edit(path="x.rs", oldText="...", newText="...", verify=true)
  → edit 尝试修改文件
  │
  ├─ 编辑失败 → 直接返回错误（不验证）
  │
  └─ 编辑成功 → verify=true 检测到，调用 verify::verify_file("x.rs")
       → 检测文件类型：.rs → FileType::Rust
       → 检查文件大小（外部进程验证 >1MB 跳过）
       → 运行对应的检查器：rustfmt --check x.rs
         （传递 abort signal，超时 10 秒）
       → 收集结果 VerifyResult { passed: false, message: "...", checker: "rustfmt" }
       → 将验证结果附加到 ToolOutput.details
       → 返回给 Agent

Agent 看到验证未通过 → 自行决定是否修复
Agent 看到验证通过 → 继续下一步
```

#### hashline_edit 的特殊处理

hashline_edit 接收一个 `edits` 数组，可能操作同一个文件的多个位置。验证在**所有 edits 应用完成后**对最终文件执行一次，而非每个 edit 后都验证。

### 检查结果的输出格式

验证结果附加在工具输出的 `details` 字段中，与现有的 `details.diff` 共存：

```json
{
  "content": [{ "type": "text", "text": "已替换 2 处匹配" }],
  "details": {
    "diff": "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-foo\n+bar",
    "verify": {
      "passed": false,
      "checker": "rustfmt",
      "message": "--- x.rs (original)\n+++ x.rs (formatted)\n@@ -1,4 +1,4 @@\n  fn main() {\n-    println!(\"hello\");\n+    println!(\"hello\");\n  }\n",
      "fileType": "rust",
      "timeMs": 523
    }
  }
}
```

#### 验证通过时

```json
{
  "details": {
    "diff": "...",
    "verify": {
      "passed": true,
      "checker": "serde_json",
      "fileType": "json",
      "timeMs": 0
    }
  }
}
```

`verify.passed=true` 时不携带 `message`，TUI 可以不额外渲染，保持输出简洁。

### effects() 声明的处理

当前 `edit`/`write`/`hashline_edit` 的 effects 返回 `ToolEffects::write()`。WRITE 本身已属于 BARRIER 位掩码，调度器不会在 barrier 操作期间并发执行其他写入。

即便 `verify=true` 时会启动外部进程，verify 也是在编辑操作的**同一个 barrier 内同步执行**的，所以不需要动态变更 effects 声明。

> **结论**：`effects()` 不变更，保持 `WRITE`。调度器语义不受影响。

### 工具缓存

现有 `ToolOutputCache` 的 `is_side_effect_tool_cache_key` 已将 `edit`/`write` 标记为副作用工具并禁用缓存。因此 `verify` 参数的引入不存在缓存一致性问题。

## 实现计划

### Phase 1：内部引擎 + 工具集成

1. **提升 `toml` 到正式依赖** — 将 `Cargo.toml` 中 `toml = "1.0.3"` 从 `[dev-dependencies]` 移至 `[dependencies]`
2. **创建 `src/tools/verify.rs`** — 内部验证引擎：
   - 文件类型检测（扩展名映射）
   - 进程内检查器（JSON/TOML via `serde_json::from_str` / `toml::from_str`）
   - 外部进程检查器（rustfmt / prettier，传递 `AbortSignal`，10 秒超时）
   - 大文件阈值检查（>1MB 跳过外部进程）
   - 工具不可用时（如 PATH 中找不到）→ 清晰错误/警告信息
3. **修改 `edit.rs`** — 增加 `verify` 参数，编辑成功后条件调用 verify
4. **修改 `hashline.rs`** — 增加 `verify` 参数，所有 edits 应用完后调用 verify
5. **修改 `write.rs`** — 增加 `verify` 参数
6. **更新工具参数 JSON Schema 和 description**
7. **更新 `src/tools/mod.rs`** — 注册 verify 模块（不对外暴露，仅 `pub(crate)`）
8. **测试** — 为每个检查器创建单元测试 + 编辑工具的集成测试

### Phase 2：扩展检查器支持（后续独立）

- `.yaml` → `yamllint`
- `.md` → 空白行检测 / 退化无检查器

## 注意事项

- JSON/TOML 检查器使用 `serde_json::from_str::<serde_json::Value>()` 和 `toml::from_str::<toml::Value>()`——仅验证语法，不验证 schema。这是有意的克制
- 外部检查器（rustfmt / prettier）的路径通过 `PATH` 环境变量解析，不写死路径
- 外部进程超时设定为 10 秒（非 `DEFAULT_BASH_TIMEOUT_SECS`），防止挂起，同时传递 `AbortSignal` 支持用户取消
- 无 Node 环境或 prettier 未缓存时的 `.ts` 检查器警告输出到 `details.verify.warning`，不阻断编辑
- 工具不在 PATH 中时输出清晰的提示（如 `"rustfmt not found in PATH，请运行 rustup component add rustfmt"`），不做静默预检
- `npx --no-install` 避免 prettier 首次使用时自动下载造成的网络延迟
- TUI 渲染方面：`details.verify` 内容由现有 JSON 展示机制处理，验证结果本身简短（passed/failed + message），无需特殊 TUI 适配
- SDK (`ToolFactory`) 无影响——verify 是执行时参数，工具注册逻辑不变
- `effects()` 不因 verify 动态变更——WRITE 已是 barrier，verify 在同一个 barrier 内同步完成

## 待讨论

- 后续是否要支持用户配置 `verify` 默认值（如 `.pi/config.toml` 增加 `default_verify = true`）？
- `.rs` 的 `rustfmt` 是否要考虑 `rustfmt.toml` 项目配置？当前方案是不考虑，只跑默认规则
