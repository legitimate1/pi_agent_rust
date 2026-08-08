# Verify 内置工具 — 架构与扩展指南

> 面向 Agent 的独立子系统文档。**什么时候读**：修改 verify 行为、新增静态检查工具（checker）、排查 verify 相关回归（如 `[verify:FAILED|...]`）时，先读本文，不必探索源码。

## 1. 定位

**verify** 是 `edit` / `hashline_edit` / `write` 工具的 `verify` 参数触发的**编辑后轻量语法/格式检查**：文件写入后自动检测文件类型并运行对应 checker，结果附在工具输出 `details.verify`，**不阻断流程**（失败只报告，不阻止写入）。

| 入口                  | 位置                                                                                                           |
| :-------------------- | :------------------------------------------------------------------------------------------------------------- |
| 唯一实现              | `src/tools/verify.rs`                                                                                          |
| 调用方（verify=true） | `src/tools/edit.rs`、`src/tools/hashline.rs`、`src/tools/write.rs`                                             |
| 结果序列化            | `verify_result_to_json()` → `details.verify`（字段：`passed` / `checker` / `fileType` / `timeMs` / `message`） |
| 功能清单条目          | `docs/context/features.md` → "编辑后轻量验证"                                                                  |

## 2. 架构

```
verify_file(path, abort)
├── detect_file_type(path)       扩展名 → FileType
├── FileType::Json/Toml          → verify_json/verify_toml    进程内解析（serde_json/toml）
└── FileType::Rust/TypeScript/Markdown → verify_external(&CHECKER)  spawn_blocking_io 包装
                                     └── run_external_checker(共享执行器, 失败时走 checker.fallback 链)
```

### 两类 checker

| 类别                     | 例子                                                          | 特点                                                                                                      |
| :----------------------- | :------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------- |
| **进程内**（internal）   | `.json` → serde_json、`.toml` → toml                          | 无子进程、无超时、错误自带行列号、无限大小                                                                |
| **外部进程**（external） | `.rs` → rustfmt、`.ts`/`.md` → prettier（全局直调，npx 回退） | 1MB 阈值、10s 超时、程序名经 `resolve_program` 解析、失败消息规范化（ANSI 剥离 + diff + fix hint + 截断） |

### 外部进程 checker 是表驱动声明式的

所有外部 checker 共享一个执行器 `run_external_checker`（`src/tools/verify.rs`），每个 checker 只声明自己的差异：

```rust
struct ExternalChecker {
    name: &'static str,                    // checker 显示名
    program: &'static str,                 // 裸程序名（resolve_program 解析 .exe/.cmd）
    version_args: &'static [&'static str], // 可用性探测（如 --version）
    not_found_hint: &'static str,          // 程序缺失时的提示
    check_args: &'static [&'static str],   // check 命令参数（路径由执行器追加）
    fix_hint: &'static str,                // 失败修复提示（<file> 占位符自动替换）
    format_args: Option<&'static [&'static str]>, // 能输出规范化文本→失败自动生成 diff
    classify_failure: Option<fn(i32, &str) -> Option<String>>, // 软失败分类
    fallback: Option<&'static ExternalChecker>, // 主程序缺失时回退的 checker（如 prettier → npx）
}
```

现有实例：`RUSTFMT_CHECKER`（无回退）、`PRETTIER_CHECKER`（**直调全局 prettier**，`fallback: NPX_PRETTIER_CHECKER`）、`NPX_PRETTIER_CHECKER`（npx 包装，仅作回退）— 均在 `src/tools/verify.rs` 静态区。

### 失败消息规范化（`run_external_checker` 硬失败路径）

1. **ANSI 剥离** — `strip_ansi()`（`\x1b\[[0-9;]*[A-Za-z]`，OnceLock 缓存正则）
2. **stdout/stderr 合并** — stderr 总是保留；stdout 仅当 `looks_like_diff()`（含 `Diff in` / `@@` / `-`/`+` 行）时追加（rustfmt 的 diff 在 stdout，prettier 的 warning 在 stderr）
3. **diff 追加** — 若 checker 声明了 `format_args`，失败时多跑一次 `<program> <format_args> <path>` 拿规范化文本，用 `similar` 库生成 unified diff（`format_diff()`，上限 6000 字符）
4. **fix hint** — `<file>` 替换为实际路径
5. **截断** — 总 message 上限 8192 字符（`truncate_message()`，UTF-8 边界安全）

## 3. 如何新增一个静态检查工具（checker）

> 目标：**只写差异，不写样板**。以下步骤适用外部进程型（大多数格式/语法工具）。进程内型（如纯解析库）参考 `verify_json` 模式，跳过第 2 步。

### 外部进程型（推荐）

1. **`FileType` 加变体** — `src/tools/verify.rs` 枚举（如 `Python`）
2. **`detect_file_type()` 加扩展名映射** — `.py` → `FileType::Python`
3. **`verify_file()` 的 match 加分支** — `FileType::Python => verify_external(&PYTHON_CHECKER, &path, abort).await?`
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
6. **验证** — `cargo test --lib tools::verify` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`

### 进程内型

1. 同样加 `FileType` 变体 + `detect_file_type` + `verify_file` match
2. 写一个 `verify_xxx(path) -> Result<(bool, Option<String>, &'static str)>` 函数（读文件 → 解析 → 返回），照 `verify_json` 模板
3. 补测试 + 验证

### 新增 checker 时必须人工判断的固有差异（无法表驱动）

| 维度           | 例子                                                      | 影响                       |
| :------------- | :-------------------------------------------------------- | :------------------------- |
| 退出码语义     | prettier exit 2 = 模块未缓存（软失败）；exit 1 = 格式问题 | `classify_failure`         |
| diff 输出位置  | rustfmt → stdout；prettier `--check` → 无 diff            | `looks_like_diff` 合并逻辑 |
| ANSI 色码      | rustfmt diff 带色码                                       | `strip_ansi`（自动）       |
| 规范化文本来源 | prettier 无 `--check` 时输出 stdout                       | `format_args`              |
| 探测命令       | rustfmt `--version`；npx `--version`                      | `version_args`             |

## 4. 已知坑（遇到类似症状先查这里）

| 坑                                                                             | 症状                                                                  | 原因/处理                                                                                                                                                                                                                                                                                                                                     |
| :----------------------------------------------------------------------------- | :-------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows 上 prettier/rustfmt verify 恒失败                                      | `[verify:FAILED\|prettier\|0ms]`，message "npx not found"             | npm 只装 `npx.cmd` 无 `npx.exe`，CreateProcess 不解析裸 `.cmd`。已由 `resolve_program` 修复（PATH 扫 `name.exe`→`name.cmd`→`name.bat`）                                                                                                                                                                                                       |
| prettier verify 偶发 10s 超时（Windows）                                       | `[verify:ERROR\|...npx.cmd timed out after 10s]`                      | **npx 包装触网**（npm registry 探测，`fetch-timeout=300s` + retries=2 → 一次网络挂起即超 10s）。已改**直调全局 prettier.cmd**（纯本地 ~270ms，无网络依赖），无全局安装时自动回退 npx。超时/abort 杀进程树（taskkill /T），避免 node 孤儿                                                                                                      |
| 宿主内（Obsidian）verify 稳定 10s 超时，.ts/.md 均复现，rustfmt .exe 不受影响  | `[verify:ERROR\|...prettier.cmd timed out after 10s]`（3 次连续复现） | **cmd shim 检查器 + stdin 继承宿主管道**：`prettier.cmd`/`npx.cmd` 经 cmd.exe 包装，而 verify 子进程 stdin 未设置 = 继承宿主的 JSONL 管道（Obsidian 持有活跃写端）→ cmd 等待管道不退出。修复：`run_external_process` spawn 显式 `stdin=null`（#34）。注：#32 的 npx 超时根因实为此（网络只是放大因素）；rustfmt 是 .exe 直连无 cmd 层，故不挂 |
| prettier 对某文件恒返回 exit 0（"All matched files use Prettier code style!"） | verify 误通过                                                         | **prettier 3.x 无 `.prettierignore` 时回退用 `.gitignore`**，gitignored 路径（如 `target/`）下的文件被静默跳过。验证时文件要放在非 gitignore 路径                                                                                                                                                                                             |
| rustfmt 失败但 message 只有 fix hint 没有 diff                                 | 旧版行为                                                              | 已修复：rustfmt diff 在 stdout，`looks_like_diff` 合并                                                                                                                                                                                                                                                                                        |
| message 超长                                                                   | 工具输出刷屏                                                          | `truncate_message` 8192 字符上限，UTF-8 边界安全                                                                                                                                                                                                                                                                                              |
| 程序探测失败误报                                                               | 恒 "not found"                                                        | 探测用 `version_args`，失败才报 not_found_hint；确认程序在 PATH                                                                                                                                                                                                                                                                               |

## 5. 输出格式示例（details.verify）

```json
{
  "passed": false,
  "checker": "prettier",
  "fileType": "typescript",
  "timeMs": 42,
  "message": "[warn] ...bad.ts\n[warn] Code style issues found...\n\n\n@@ -1,2 +1,2 @@\n-const x:number=1;\n+const x: number = 1;\n\n\nRun `npx prettier --write ...bad.ts` to fix."
}
```

- `passed=false` 时 Agent 应直接按 `message` 中的 diff 修正文件（无需再手动跑格式化工具）
- `message` 结尾的 fix hint 是建议的修复命令
