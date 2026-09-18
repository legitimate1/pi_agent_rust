# Verify 输出策略 — Markdown 跳过自动验证与 oxfmt 噪音过滤

## 目标与背景

当前 `edit`、`hashline_edit` 和 `write` 在 `verify=true` 时会对已识别的文件运行自动验证。对于 Markdown，Prettier 的全文件格式检查经常产生大量表格对齐、空格和换行 diff。这些 diff 通常不反映文档语义，也不能说明本次局部编辑是否正确，反而会污染 Agent 的工具反馈。

对于 JavaScript/TypeScript，`oxfmt --check` 的失败输出中还包含两类 informational 日志：

```text
No config found, using defaults. Please add `.oxfmtrc.json` if needed.
Finished in 1ms on 1 files using 24 threads.
```

这两类信息不是格式错误、lint 错误或修复依据。它们被原样合并到 verify 输出后，会增加 Agent 需要解析的噪音。

本设计只处理两个问题：

1. Markdown 不再被三个文件修改工具自动运行 Prettier；
2. 自动 verify 的 oxfmt 输出过滤无助于修复的 informational 行。

⚠️ 本设计不把 Markdown 的格式问题等同于语义问题，也不引入自动格式化。

## 设计结论

### Markdown 不参与自动 verify

对以下路径统一采用相同策略：

```text
edit(.md / .markdown)

hashline_edit(.md / .markdown)

write(.md / .markdown)
```

当调用方传入 `verify=true` 时：

```text
执行文件修改
→ 保留并返回实际写入/编辑结果
→ 不运行 Markdown Prettier
→ 输出一条 SKIPPED 通知
```

通知固定为：

```text
[verify:SKIPPED|markdown] Markdown 默认跳过自动验证。
```

当调用方传入 `verify=false` 时，不输出 `SKIPPED` 通知；现有跳过验证行为保持不变。

`SKIPPED` 不得伪装成 `PASSED`：

```text
PASSED  = 已执行检查且通过
FAILED  = 已执行检查但失败
SKIPPED = 本次没有执行检查
```

`.md` 和 `.markdown` 都属于该规则。其他不支持自动验证的扩展名继续保持现有行为，不额外输出 `SKIPPED`。

### Markdown checker 的保留边界

`FileType::Markdown` 和现有 Prettier checker 暂时保留，作为未来显式 Markdown 检查或格式化能力的基础；但它们不再从 `edit`、`hashline_edit`、`write` 的自动 verify 路径调用。

本设计不新增 `verify=auto/always/never` 参数，也不在本阶段设计新的显式 Markdown 工具入口。未来若需要主动检查或格式化 Markdown，应作为用户明确请求的独立动作设计。

### oxfmt 输出过滤

只对以下两个 checker 启用相同的 oxfmt 专属过滤规则：

```text
OXFMT_CHECKER
NPX_OXFMT_CHECKER
```

需要过滤的行：

```text
No config found...
Finished in ... on ... files using ... threads.
```

建议按结构而不是按完整版本文案识别。当前规则依据本地 `oxfmt 0.64.0` 的实际输出；官方文档只保证 `--check` 会显示检查统计，不把完整日志文案、标点或 stdout/stderr 归属视为稳定协议。

```rust
fn is_oxfmt_informational_line(line: &str) -> bool {
    let line = line.trim();

    line.starts_with("No config found, using defaults.")
        || (line.starts_with("Finished in ")
            && line.contains(" on ")
            && line.contains(" files using ")
            && line.ends_with(" threads."))
}
```

过滤器只负责移除已确认的 informational 行，不采用“只保留白名单”的激进策略，以避免未来 oxfmt 或其他诊断文本被误删。若未来 oxfmt 改变文案，应新增精确匹配规则和回归夹具，不扩大为全局 informational 黑名单。

当前本地观测到的输出流为：

```text
stdout: Checking formatting... / Format issues / Finished in ...
stderr: No config found, using defaults. ...
```

过滤器仍须对 stdout 和 stderr 都生效，因为不同版本或包装路径可能改变输出流。过滤实现必须删除完整 informational 行，同时原样保留其他内容及其行结束符：

```rust
fn filter_output_preserving_newlines(
    text: &str,
    suppress: fn(&str) -> bool,
) -> String {
    let mut out = String::with_capacity(text.len());

    for chunk in text.split_inclusive('\n') {
        let body = chunk.strip_suffix('\n').unwrap_or(chunk);
        let line = body.strip_suffix('\r').unwrap_or(body);

        if !suppress(line) {
            out.push_str(chunk);
        }
    }

    out
}
```

禁止使用 `lines().collect::<Vec<_>>().join("\n")` 作为实现，因为它会改变 CRLF、末尾换行和空输出边界。

## 处理流程

### Markdown 自动 verify

三个调用方在执行自动 verify 前，按以下顺序处理：

```text
verify=false
  → 保持现有行为，不输出通知

verify=true + Markdown
  → 统一自动验证分派函数返回 SkipMarkdown
  → append SKIPPED 通知
  → 不调用 verify_file

verify=true + 其他已支持类型
  → 统一自动验证分派函数返回 RunVerify
  → 继续调用 verify_file

verify=true + 不支持类型
  → 统一自动验证分派函数返回 NoVerify
  → 保持现有行为，不输出 unsupported 错误
```

三个工具必须共用同一分派规则，避免未来出现某个工具仍对 Markdown 启动 checker 的分支漂移。测试应能直接断言 Markdown 返回 `SkipMarkdown`，其他可验证类型返回 `RunVerify`，`verify=false` 返回 `NoVerify`；工具级测试再验证最终 `SKIPPED` 文本、实际 diff 和写入字节数。

实现应避免把 Markdown 先交给 `verify_file` 再丢弃结果，因为这样仍会产生无意义的进程启动和潜在超时。

### oxfmt 外部输出

在共享外部 checker 执行器中，处理顺序固定为：

```text
读取 stdout/stderr
→ 分别 strip ANSI
→ 仅对 checker 声明的输出过滤器逐行过滤
→ 根据现有规则合并 stderr/stdout
→ 追加格式化 Diff（如有）
→ 追加 fix hint
→ 截断最终消息
```

过滤必须在 stdout/stderr 合并前完成，原因是：

- 可以明确保证只影响 oxfmt；
- 不会误伤 `oxlint` 聚合消息；
- `oxfmt` 与 `npx-oxfmt` 可以共享同一个过滤函数；
- 不改变已有 Diff 识别和消息聚合逻辑。

建议在 `ExternalChecker` 中增加可选的逐行过滤能力，例如：

```rust
suppress_output_line: Option<fn(&str) -> bool>,
```

只有 `OXFMT_CHECKER` 和 `NPX_OXFMT_CHECKER` 设置该字段，其他 checker 设置为 `None`。

保留以下有效信息：

```text
Checking formatting...
Format issues found ...
Diff / unified diff
--- oxlint ---
具体 lint 诊断
Run `oxfmt ...` to fix.
```

预期输出示例：

```text
[verify:FAILED|oxfmt+oxlint|138ms]
Checking formatting...
Format issues found in above 1 files. Run without `--check` to fix.
@@ -1 +1,3 @@
-export function hello( name : string ) : string{return...}
+export function hello(name: string): string {
+  return `hello ${name}`;
+}

Run `oxfmt C:\path\to\file.ts` to fix.
--- oxlint ---
4:7 warning eslint(no-unused-vars): ...
```

## 影响范围

### 代码范围

预计修改：

```text
src/tools/edit.rs
src/tools/hashline.rs
src/tools/write.rs
src/tools/verify.rs
src/tools/tests.rs
```

其中：

- 三个文件修改工具增加 Markdown 自动跳过和 `SKIPPED` 通知路径，并同步工具参数 Schema 描述；
- `verify.rs` 提供 Markdown 路径识别/通知辅助，以及 oxfmt 输出过滤能力；
- `tests.rs` 增加三个工具的 Markdown 行为验收和 checker 输出回归测试；
- 现有 `verify_file` 的其他 checker 分支保持不变；
- Markdown checker 保留但不再由自动编辑路径调用。

### 文档范围

实现完成后同步更新：

```text
src/tools/edit.rs
src/tools/hashline.rs
src/tools/write.rs
src/tools/verify.rs
docs/context/verify-tool.md
docs/context/features.md
```

文档与工具 Schema 必须明确：

- Markdown 的 Prettier checker 仍可保留，但不是三个文件修改工具的自动 verify；
- `verify=true` 对 Markdown 输出 `SKIPPED`，不是 `PASSED`；
- `oxfmt` 的两类 informational 行会在 verify 输出中被过滤；
- 过滤只作用于 oxfmt/npx-oxfmt，不是全局 stdout/stderr 过滤；
- `edit`、`hashline_edit`、`write` 的参数描述必须与上述实际行为一致。

## 测试与验收标准

### Markdown 行为

必须覆盖：

```text
- edit 写入 .md 时 verify=true 输出 SKIPPED
- edit 写入 .markdown 时 verify=true 输出 SKIPPED
- hashline_edit 对 .md/.markdown 输出 SKIPPED
- write 对 .md/.markdown 输出 SKIPPED
- verify=false 不输出 SKIPPED
- Markdown 路径不启动 Prettier
- 实际编辑/写入 diff 和字节数仍正常返回
```

### oxfmt 过滤

至少补充纯函数或 checker 级测试覆盖：

```text
- "No config found, using defaults." 被过滤
- 不同版本中已确认的默认配置提示变体被过滤
- "Finished in ... files using ... threads." 被过滤
- 带 ANSI 的上述两类行被过滤
- LF、CRLF 和无末尾换行均保持边界
- 噪音位于开头、中间、末尾以及全部内容均为噪音时行为正确
- stdout 中的噪音被过滤
- stderr 中的噪音被过滤
- Checking formatting 保留
- Format issues 保留
- Diff 行和修复提示保留
- oxlint 聚合诊断保留
- OXFMT_CHECKER 和 NPX_OXFMT_CHECKER 的字段接线均启用
- OXLINT、rustfmt、ruff、prettier、serde_json、toml、gofmt 的过滤字段均未启用且输出不受影响
```

### 自动 verify 分派

必须验证分派层本身，而不只断言最终输出：

```text
- verify=true + .md/.markdown → SkipMarkdown，verify_file 调用次数为 0
- verify=false + .md/.markdown → NoVerify，verify_file 调用次数为 0，且无 SKIPPED
- verify=true + 其他已支持类型 → RunVerify
- 工具级测试仍验证 edit/hashline_edit 的实际 diff 和 write 的字节数
```

### 静态验证

实现阶段运行：

```text
cargo test --lib tools::verify
cargo test --lib tools::tests
cargo clippy --lib -- -D warnings
cargo fmt --check
```

不要使用不存在或无法命中当前测试布局的 `tools::edit`、`tools::hashline`、`tools::write` selector 代替 `tools::tests`。按项目日常开发约束，不在本阶段运行全量 `--all-targets` 检查；全部改动收尾时再按项目规则执行全量门禁。

## 不在本次范围内

以下事项明确不纳入本设计：

```text
- 不自动运行 prettier --write
- 不对 Markdown 做语义正确性判断
- 不新增 Markdown 全文语义检查器
- 不新增 verify=auto/always/never 参数
- 不改用 oxfmt --list-different
- 不对所有 checker 采用统一 informational 日志黑名单
- 不改变 verify=false 行为
- 不改变 Markdown checker 以外的 rustfmt、ruff、prettier、JSON/TOML 解析等既有检查逻辑
- 不修改保留的 Markdown Prettier checker 本身；本次只移除它在三个文件修改工具自动路径中的调用
```

## 注意事项

⚠️ `verify:SKIPPED` 只能表示“自动 verify 未执行”，不能被记录或文档描述为 Markdown 已通过检查。

⚠️ oxfmt 过滤规则必须尽量窄。若未来出现新的 informational 输出，应先确认它不包含修复所需的诊断，再扩展过滤规则。

💡 设计保留 Prettier checker，是为了把“自动编辑反馈”和“用户主动格式化”分开，而不是承诺 Markdown 永久不需要格式化。

## 待讨论

本设计中的核心行为已由用户确认。实现阶段如发现以下情况，只能做兼容性修正，不得自行改变核心方向：

```text
- oxfmt 实际输出格式与当前过滤模式存在版本差异；
- 现有测试要求某个内部 verify 辅助函数具有不同可见性；
- 过滤实现必须保持 LF/CRLF、末尾换行和其他诊断边界；
- 自动验证分派层需要在三个调用方之间共享，以确保 Markdown 永不调用 verify_file。
```

第 7 项关于 `.MD` / `.MARKDOWN` 大小写扩展名不纳入本设计，继续沿用现有小写扩展名识别规则。

外部参考：

- Oxfmt CLI 文档：https://oxc.rs/docs/guide/usage/formatter/cli.html
- Oxfmt 配置与默认行为：https://oxc.rs/docs/guide/usage/formatter/config

## 实现

### 实现边界

来源：本文件的设计部分

目标：

- 让 Markdown 在 `edit`、`hashline_edit`、`write` 的自动 verify 路径中明确显示 `SKIPPED`，且不启动 Prettier。
- 过滤 `oxfmt`/`npx-oxfmt` 失败输出中的两类 informational 行，同时保留可修复诊断。

包含：

- 在共享自动验证分派处区分 `NoVerify`、`SkipMarkdown` 和 `RunVerify`，三个文件修改工具复用同一规则。
- 同步三个工具的 `verify` 参数 Schema 描述。
- 为 `ExternalChecker` 增加 checker 专属输出过滤器，仅接线到 `OXFMT_CHECKER` 和 `NPX_OXFMT_CHECKER`。
- 使用保留 LF/CRLF、末尾换行和其他内容边界的逐行过滤实现。
- 增加分派、SKIPPED、过滤器、checker 接线和工具行为回归测试。
- 同步 `verify.rs` 模块文档、`docs/context/verify-tool.md` 和 `docs/context/features.md`。

不包含：

- 不新增 `verify=auto/always/never` 参数。
- 不修改或删除保留的 Markdown Prettier checker。
- 不新增 Markdown 语义检查器或自动格式化入口。
- 不支持 `.MD` / `.MARKDOWN` 大小写扩展名。
- 不切换到 `oxfmt --list-different`。

### 当前代码事实

- `src/tools/edit.rs`、`src/tools/hashline.rs`、`src/tools/write.rs` 当前在 `verify=true` 且文件类型受支持时直接调用 `verify_file`。
- `verify_file` 当前把 Markdown 分派到 `PRETTIER_CHECKER`；该 checker 保留，但自动编辑路径需要在调用前短路。
- `src/tools/tests.rs` 是三个工具行为测试的实际注册模块；测试不按 `tools::edit`、`tools::hashline`、`tools::write` 分模块注册。
- `ExternalChecker` 由 `run_external_checker_resolved` 统一执行，stdout/stderr 在失败路径中分别读取、合并，并追加格式化 Diff 与 fix hint。
- 过滤必须发生在 ANSI 清理后、stdout/stderr 合并前；verify 的外部进程 runner 保持现有 `stdin=null`、超时和进程树终止行为。

### 文件变更清单

- C1 — `src/tools/verify.rs`：修改
  - 用途：集中定义自动 verify 分派、Markdown SKIPPED 辅助、oxfmt 输出过滤和相关测试。
  - 关键改动：增加共享分派结果；增加保留换行的 checker 输出过滤；扩展 `ExternalChecker` 的可选过滤器字段；只为 oxfmt 两个 checker 接线；同步模块级契约说明。

- C2 — `src/tools/edit.rs`：修改
  - 用途：让文本局部编辑遵循 Markdown 跳过策略。
  - 关键改动：使用共享分派；Markdown `verify=true` 追加固定 SKIPPED 通知；其他可验证类型保持原 verify 路径；更新工具 Schema 描述。

- C3 — `src/tools/hashline.rs`：修改
  - 用途：让 hashline 局部编辑与 `edit` 保持一致。
  - 关键改动：使用共享分派；追加 Markdown SKIPPED 通知；更新工具 Schema 描述。

- C4 — `src/tools/write.rs`：修改
  - 用途：让整文件写入也不再自动运行 Markdown Prettier。
  - 关键改动：使用共享分派；追加 Markdown SKIPPED 通知；更新工具 Schema 描述。

- C5 — `src/tools/tests.rs`：修改
  - 用途：覆盖三个工具的行为契约。
  - 关键改动：增加 `.md` / `.markdown`、`verify=true/false`、实际 diff/字节数和不启动 checker 的回归测试。

- C6 — `docs/context/verify-tool.md`：修改
  - 用途：同步 verify 架构和 Markdown 自动路径契约。
  - 关键改动：说明 Markdown checker 保留但自动编辑路径跳过、SKIPPED 语义和 oxfmt 过滤边界。

- C7 — `docs/context/features.md`：修改
  - 用途：同步功能目录中的编辑后验证描述。
  - 关键改动：说明 Markdown 默认跳过自动验证，并保留其他 checker 的自包含诊断说明。

### 依赖与工作单元

- C1：无依赖；先完成共享分派和过滤基础。
- C2、C3、C4：依赖 C1；三个调用方接入同一分派规则和输出辅助。
- C5：依赖 C1–C4；补充并执行工具行为与 verify 回归测试。
- C6、C7：依赖 C1–C4；按最终实现同步长期项目文档。

工作单元：

- W1 — verify 核心：完成共享自动验证分派、SKIPPED 输出辅助、oxfmt 过滤字段/函数/接线及 verify 单测；完成条件是核心纯函数、字段接线和消息边界可验证。
- W2 — 工具接入：修改三个工具的调用分支和 Schema；完成条件是 Markdown 不调用 `verify_file`、`verify=true` 输出 SKIPPED、`verify=false` 无通知且既有其他类型路径不变。
- W3 — 集成测试与文档：在 `src/tools/tests.rs` 补行为回归，并同步两份 context 文档；完成条件是验收项覆盖且文档与实际契约一致。

冲突：

- C1–C4 共享自动 verify 输出契约，不能让三个工具分别发明 Markdown 判断或通知文本。
- C1 的外部 checker 过滤不得扩展到全局 stdout/stderr，也不得改变 oxlint、rustfmt、ruff、prettier、JSON/TOML、gofmt 的现有输出。

### 验证计划

自动化检查：

- `cargo test --lib tools::verify`
- `cargo test --lib tools::tests`
- `cargo clippy --lib -- -D warnings`
- `cargo fmt --check`

预期结果：

- Markdown `.md` / `.markdown` 的 `verify=true` 均输出 `[verify:SKIPPED|markdown] Markdown 默认跳过自动验证。`。
- Markdown 路径不调用 `verify_file`，不启动 Prettier；`verify=false` 不输出 SKIPPED。
- oxfmt informational 行在 stdout/stderr 两侧被过滤，Diff、格式摘要、oxlint 诊断和 fix hint 保留。
- LF、CRLF、无末尾换行和空输出边界保持正确。
- 其他 checker 的既有行为和工具实际修改结果不变。

人工检查：

- 查看工具 Schema 描述、`verify-tool.md`、`features.md` 是否都明确区分 `SKIPPED` 与 `PASSED`。
- 用真实坏格式 TS 文件运行一次 verify，确认不再出现 `No config found...` 和 `Finished in ... threads`。

### 执行状态

- 状态：已完成
- 备注：W1 verify 核心、W2 三个工具接入、W3 测试与项目文档同步均已完成；针对性测试、Clippy（`--lib`）和格式检查通过。
