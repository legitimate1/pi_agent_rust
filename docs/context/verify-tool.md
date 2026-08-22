# Verify 内置工具 — 架构与扩展指南

> 面向 Agent 的独立子系统文档。**什么时候读**：修改 verify 行为、新增静态检查工具（checker）、排查 verify 相关回归（如 `[verify:FAILED|...]`）时，先读本文，不必探索源码。

## 1. 定位

**verify** 是 `edit` / `hashline_edit` / `write` 工具的 `verify` 参数触发的**编辑后轻量语法/格式检查**：文件写入后自动检测文件类型并运行对应 checker，**诊断直写 `content` 正文，不进 `details`**，**不阻断流程**（失败只报告，不阻止写入）。

- **唯一实现**：`src/tools/verify.rs`
- **调用方（verify=true）**：`src/tools/edit.rs`、`src/tools/hashline.rs`、`src/tools/write.rs`（各一行复用 `append_verify_to_output`）
- **正文输出**：`append_verify_to_output()` / `append_verify_error_to_output()` → 在 `Successfully ...` 下追加 `\n[verify:STATUS|checker|timeMs]\n{message}`（`message` 原样追加，无围栏/列表/JSON 包裹，去重后的头部已含 `checker`/`time`/`PASSED|FAILED`）
- **序列化保留**：`verify_result_to_json()` 仅供单测/文档保留，不再参与正文路径
- **details 现状**：`write` → `details=None`；`edit`/`hashline_edit` → 仅保留 `diff`/`firstChangedLine`，不再有 `details.verify`
- **二进制依赖**：`C:\Users\m\.pi\agent\bin\oxfmt.exe` / `oxlint.exe` / `ruff.exe` / `gofmt.exe`（`PATH` 已含该目录，`resolve_program` 按 `exe→cmd→bat` 优先命中 `exe` 直调，离线可用；`oxfmt/oxlint` 未命中时回退 `npx --yes`）
- **功能清单条目**：`docs/context/features.md` → "编辑后轻量验证"

## 2. 架构

```
verify_file(path, abort)
├── detect_file_type(path)       扩展名 → FileType
│     .rs → Rust | .json → Json | .toml → Toml
│     .ts/.tsx → TypeScript | .js/.jsx/.mjs/.cjs → JavaScript
│     .py/.pyi → Python | .go → Go | .md/.markdown → Markdown
├── FileType::Json/Toml          → verify_json/verify_toml    进程内解析（serde_json/toml）
├── FileType::Rust               → verify_external(&RUSTFMT_CHECKER)
├── FileType::TypeScript/JavaScript → verify_js_ts_parallel()   oxfmt --check + oxlint --deny-warnings 并行（futures::join）
├── FileType::Python             → verify_python_parallel()    ruff format --check + ruff check 并行
├── FileType::Go                 → verify_go()                 gofmt -l（stdout 非空即需格式化，diff 由 gofmt 合成）
└── FileType::Markdown           → verify_external(&PRETTIER_CHECKER)  spawn_blocking_io 包装
                                     └── run_external_checker(共享执行器, 失败时走 checker.fallback 链)
```

### 分层模型

`Language → Checker[ capability=Format | Lint ]`（`Format`/`Lint` 是 Checker 身上的标签，不升为独立层）：

- `Language` 层：`detect_file_type` 决定 `FileType`
- `Checker` 层：`FileType → Vec<Checker>`，`Format` 与 `Lint` 并行（只读、无锁、独立子进程），`passed = &&`，`message` 按 `format→lint` 固定顺序拼接，`checker` 聚合名为 `oxfmt+oxlint` / `ruff`

### 两类 checker

- **进程内（internal）**
  - 例子：`.json` → serde_json、`.toml` → toml
  - 特点：无子进程、无超时、错误自带行列号、无限大小
- **外部进程（external）**
  - 例子：`.rs` → rustfmt、`.py` → ruff、`.go` → gofmt、`.ts/.js` → oxfmt/oxlint、`.md` → prettier
  - 特点：1MB 阈值、10s 超时、程序名经 `resolve_program` 解析、失败消息规范化（ANSI 剥离 + diff + fix hint + 截断）

### 外部进程 checker 是表驱动声明式的

所有外部 checker 共享一个执行器 `run_external_checker`（`src/tools/verify.rs`），每个 checker 只声明自己的差异：

```rust
struct ExternalChecker {
    name: &'static str,                    // checker 显示名
    program: &'static str,                 // 裸程序名（resolve_program 解析 .exe/.cmd）
    version_args: &'static [&'static str], // 可用性探测（如 --version，gofmt 用 --help）
    not_found_hint: &'static str,          // 程序缺失时的提示
    check_args: &'static [&'static str],   // check 命令参数（路径由执行器追加）
    fix_hint: &'static str,                // 失败修复提示（<file> 占位符自动替换）
    format_args: Option<&'static [&'static str]>, // 能输出规范化文本→失败自动生成 diff
    classify_failure: Option<fn(i32, &str) -> Option<String>>, // 软失败分类
    fallback: Option<&'static ExternalChecker>, // 主程序缺失时回退的 checker（如 prettier → npx）
}
```

现有实例（均在 `src/tools/verify.rs` 静态区）：

- `RUSTFMT_CHECKER`（无回退，`rustfmt --check --edition 2024`）
- `OXFMT_CHECKER` → `NPX_OXFMT_CHECKER`（`oxfmt --check` → `npx --yes oxfmt --check`）
- `OXLINT_CHECKER` → `NPX_OXLINT_CHECKER`（`oxlint --deny-warnings` → `npx --yes oxlint --deny-warnings`）
- `RUFF_FORMAT_CHECKER`（`ruff format --check`）/ `RUFF_CHECKER`（`ruff check`，无回退，`bin/ruff.exe` 直调）
- `GOFMT_CHECKER`（`gofmt -l`，`version_args=--help`，特殊：`exit 0` 仍需判 `stdout` 非空，diff 由 `gofmt <file>` 合成）
- `PRETTIER_CHECKER`（**直调全局 prettier**，`fallback: NPX_PRETTIER_CHECKER`）、`NPX_PRETTIER_CHECKER`（npx 包装，仅作回退，仅用于 `.md`）

### 正文输出（去重）

`append_verify_to_output` 已精简为：

```rust
pub fn append_verify_to_output(output: &mut String, r: &VerifyResult) {
    write!(output, "\n[verify:{}|{}|{}ms]", status, r.checker, r.time_ms);
    if let Some(msg) = &r.message { write!(output, "\n{msg}"); } // 原样追加，无围栏/列表
}
```

`[verify:FAILED|prettier|425ms]` 已含 `checker`/`time`/`PASSED|FAILED`，不再重复输出 `- checker/fileType/passed/timeMs` 四行。

### 失败消息规范化（`run_external_checker` 硬失败路径）

1. **ANSI 剥离** — `strip_ansi()`（`\x1b\[[0-9;]*[A-Za-z]`，OnceLock 缓存正则）
2. **stdout/stderr 合并** — stderr 总是保留；stdout 仅当 `looks_like_diff()`（含 `Diff in` / `@@` / `-`/`+` 行）时追加（rustfmt 的 diff 在 stdout，prettier 的 warning 在 stderr）
3. **diff 追加** — 若 checker 声明了 `format_args`，失败时多跑一次 `<program> <format_args> <path>` 拿规范化文本，用 `similar` 库生成 unified diff（`format_diff()`，上限 6000 字符）；`gofmt` 固定用 `run_formatter(&[], path)` 合成 diff
4. **fix hint** — `<file>` 替换为实际路径
5. **截断** — 总 message 上限 8192 字符（`truncate_message()`，UTF-8 边界安全）

### 为什么不需要为格式化工具写忽略文件（如 `.prettierignore`）

- **verify 是单文件直调**：`verify_file(path)` 对给定绝对路径跑 `<program> <check_args> <path>`（如 `prettier --check <file>`），不走 `prettier .` 批量扫描；是否校验只看 `detect_file_type`（扩展名），不看 ignore 文件
- **Agent 只动源码**：Agent 的编辑范围是 `src/` / `docs/` / `scripts/` 等源码，不会动 `target/` / `node_modules/` / `dist/` 等构建产物
  - 实测：`prettier 3.x` 默认 `--ignore-path=[.gitignore,.prettierignore]`，`prettier --check target/...` 会被 `.gitignore` 的 `/target/` 静默跳过（exit 0 误通过）
  - 但该路径永远不会被 `edit` / `write` / `hashline_edit` 的 `verify` 触发，故无需修复
- **不要额外维护 `.prettierignore` / `.rustfmt.toml` 等**：
  - 单写 `!target/` / `!target/**` 否定不生效（需先有肯定才有否定，且默认两份 ignore 叠加）
  - 空 `.prettierignore` 仍会读 `.gitignore`（`prettier --file-info` 仍 `ignored:true`）
  - 要绕过需显式 `prettier --check --ignore-path=.prettierignore <file>` / `--ignore-path=NUL` 等参数 —— 为单文件模型引入额外复杂度不值得
  - 结论：verify 保持零配置，不在仓库根添加仅为 verify 服务的忽略文件
- **何时再考虑**：只有当 verify 改为批量扫描（如 `prettier --check .`）或确实需要在构建产物上校验时，再评估 `check_args` 中追加 `--ignore-path` / `--no-ignore` 等选项

## 3. 如何新增一个静态检查工具（checker）

> 目标：**只写差异，不写样板**。以下步骤适用外部进程型（大多数格式/语法工具）。进程内型（如纯解析库）参考 `verify_json` 模式，跳过第 2 步。

### 外部进程型（推荐）

1. **`FileType` 加变体** — `src/tools/verify.rs` 枚举（如 `Python`）
2. **`detect_file_type()` 加扩展名映射** — `.py` → `FileType::Python`
3. **`verify_file()` 的 match 加分支** — `FileType::Python => verify_external(&PYTHON_CHECKER, &path, abort).await?`；若需 `Format+Lint` 并行，新增 `verify_python_parallel()` 并用 `futures::join`
4. **声明 checker 常量** — 照 `RUSTFMT_CHECKER` 模板：
   - `program`：裸命令名（Windows `.cmd` shim 由 `resolve_program` 自动处理，**不要**硬编码 `.exe`）
   - `check_args`：check 命令参数（路径自动追加在最后）
   - `format_args`：能输出规范化文本就设 `Some(...)` → **免费获得失败 diff**
   - `classify_failure`：有"非格式问题退出码"（如模块未缓存）时提供软失败分类函数
   - `fix_hint`：给 Agent 的修复命令（用 `<file>` 占位）
   - `fallback`：主程序缺失时回退的 checker（如 `PRETTIER_CHECKER → NPX_PRETTIER_CHECKER`）；无回退设 `None`
5. **补测试** — `src/tools/verify.rs` tests：
   - `detect_file_type` 新扩展名断言
   - 有差异逻辑（classify / diff）时补纯函数单测
6. **验证** — `cargo test --lib tools::verify` + `cargo clippy --lib -- -D warnings` + `cargo fmt --check`

### 进程内型

1. 同样加 `FileType` 变体 + `detect_file_type` + `verify_file` match
2. 写一个 `verify_xxx(path) -> Result<(bool, Option<String>, &'static str)>` 函数（读文件 → 解析 → 返回），照 `verify_json` 模板
3. 补测试 + 验证

### 新增 checker 时必须人工判断的固有差异（无法表驱动）

- **退出码语义**
  - 例子：prettier exit 2 = 模块未缓存（软失败）；exit 1 = 格式问题；gofmt `exit 0 + stdout非空` = 需格式化；oxlint 默认 warning `exit 0`，需 `--deny-warnings` 才 `exit 1`
  - 影响：`classify_failure` / `verify_go` 的 `stdout` 判空
- **diff 输出位置**
  - 例子：rustfmt → stdout；prettier `--check` → 无 diff；oxfmt/ruff 无 `format_args` diff
  - 影响：`looks_like_diff` 合并逻辑
- **ANSI 色码**
  - 例子：rustfmt diff 带色码
  - 影响：`strip_ansi`（自动）
- **规范化文本来源**
  - 例子：prettier 无 `--check` 时输出 stdout；gofmt `<file>` 输出格式化后文本
  - 影响：`format_args` / `run_formatter`
- **探测命令**
  - 例子：rustfmt `--version`；gofmt `--help`；npx `--version`
  - 影响：`version_args`

## 4. 已知坑（遇到类似症状先查这里）

- **坑：Windows 上 prettier/rustfmt verify 恒失败**
  - 症状：`[verify:FAILED|prettier|0ms]`，message "npx not found"
  - 原因/处理：npm 只装 `npx.cmd` 无 `npx.exe`，CreateProcess 不解析裸 `.cmd`。已由 `resolve_program` 修复（PATH 扫 `name.exe`→`name.cmd`→`name.bat`）
- **坑：prettier verify 偶发 10s 超时（Windows）**
  - 症状：`[verify:ERROR|...npx.cmd timed out after 10s]`
  - 原因/处理：**npx 包装触网**（npm registry 探测，`fetch-timeout=300s` + retries=2 → 一次网络挂起即超 10s）。已改**直调全局 prettier.cmd**（纯本地 ~270ms，无网络依赖），无全局安装时自动回退 npx。超时/abort 杀进程树（taskkill /T），避免 node 孤儿；`oxfmt/oxlint` 的 `npx --yes` 同理，但 `bin/ox*.exe` 直调已规避
- **坑：宿主内（Obsidian）verify 稳定 10s 超时，.ts/.md 均复现，rustfmt .exe 不受影响**
  - 症状：`[verify:ERROR|...prettier.cmd timed out after 10s]`（3 次连续复现）
  - 原因/处理：**cmd shim 检查器 + stdin 继承宿主管道**：`prettier.cmd`/`npx.cmd` 经 cmd.exe 包装，而 verify 子进程 stdin 未设置 = 继承宿主的 JSONL 管道（Obsidian 持有活跃写端）→ cmd 等待管道不退出。修复：`run_external_process` spawn 显式 `stdin=null`（#34）。注：#32 的 npx 超时根因实为此（网络只是放大因素）；rustfmt 是 .exe 直连无 cmd 层，故不挂；`ruff.exe`/`gofmt.exe`/`oxfmt.exe` 同为 `.exe` 直调不受影响
- **坑：prettier 对某文件恒返回 exit 0（"All matched files use Prettier code style!"）**
  - 症状：verify 误通过
  - 原因/处理：**prettier 3.x 默认 `--ignore-path=[.gitignore,.prettierignore]`**，gitignored 路径（如 `target/`）下的文件被静默跳过。**无需为此写 `.prettierignore`**（见 §2 末“为什么不需要为格式化工具写忽略文件”）：Agent 不会编辑 `target/` 等构建产物；要校验 `target/` 需改 `check_args` 追加 `--ignore-path` / `--no-ignore` 等选项，而非维护忽略文件。验证时文件要放在非 gitignore 路径
- **坑：gofmt 误通过**
  - 症状：未格式化文件仍 `PASSED`
  - 原因/处理：`gofmt -l <file>` 格式化不一致时 `exit 0` 仅 stdout 吐路径，需判 `stdout.trim().is_empty()` 而非 `status.success()`；已由 `verify_go` 修复，并用 `gofmt <file>` 合成 diff
- **坑：rustfmt 失败但 message 只有 fix hint 没有 diff**
  - 症状：旧版行为
  - 原因/处理：已修复：rustfmt diff 在 stdout，`looks_like_diff` 合并
- **坑：message 超长**
  - 症状：工具输出刷屏
  - 原因/处理：`truncate_message` 8192 字符上限，UTF-8 边界安全
- **坑：程序探测失败误报**
  - 症状：恒 "not found"
  - 原因/处理：探测用 `version_args`，失败才报 not_found_hint；确认程序在 PATH（`oxfmt/oxlint/ruff/gofmt` 已在 `C:\Users\m\.pi\agent\bin`，`where.exe` 可验证）

## 5. 输出格式示例（content 正文，非 details）

```text
Successfully wrote 16 bytes to C:\path\to\bad.ts
[verify:FAILED|oxfmt+oxlint|42ms]
Checking formatting...
C:/path/to/bad.ts (0ms)
Format issues found in above 1 files. Run without `--check` to fix.

--- oxlint ---
C:/path/to/bad.ts:1:7: warning eslint(no-unused-vars): Variable 'x' is declared but never used.

Run `oxfmt C:\path\to\bad.ts` to fix.
```

```text
Successfully wrote 15 bytes to C:\path\to\bad.json
[verify:FAILED|serde_json|2ms]
JSON parse error: key must be a string at line 1 column 2
```

- `content` 中 ` [verify:FAILED|...]` 下一行即 `message` 原样（`JSON parse error` / `Diff in` / `[warn]` + `fix hint`），Agent 无需翻 `details` 即可直接按 diff 修正
- `write` 的 `details` 为 `None`，`edit`/`hashline_edit` 的 `details` 仅含 `diff`/`firstChangedLine`，不再有 `details.verify`（`verify_result_to_json()` 仅保留供单测）
