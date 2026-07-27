# IMPLEMENT.md — 编辑后轻量验证系统（Verify）

## 1. 实现边界（Implementation Contract）

**Source**: `docs/design/pi-edit-verify-design.md`

**Goal**: 为 `edit`/`hashline_edit`/`write` 三个工具增加可选 `verify` 参数，编辑完成后自动对文件运行轻量语法/格式检查。结果附在 `details` 字段中，不阻断编辑流程。

**In scope**:
- 新建 `src/tools/verify.rs` 内部验证引擎（文件类型检测 → 检查器映射 → 执行）
- 三个编辑工具各增加 `#[serde(default)] verify: bool` 输入参数
- `toml` crate 从 dev-dependencies 提升为正式依赖
- 工具参数 JSON Schema 增加 `verify` 字段描述
- 检查结果以 `details.verify` 格式输出

**Out of scope**:
- 不新增 LLM-visible tool
- 不新增 CLI 子命令
- 不自动修正（铁律）
- 不添加 `.yaml`/`.md` 等扩展的检查器

**Assumptions**:
- `rustfmt` 在 PATH 中（随 Rust 工具链安装）
- `npx` 在 PATH 中（仅当需要验证 `.ts` 文件时）
- 项目已有 `serde_json` 在正式依赖中

**Design delta**:
- `.toml` 从 Phase 2 提升到 MVP（Review 建议）
- `npx --no-install` 替代 `npx`（避免首次自动下载）

---

## 2. 文件变更清单（Change Manifest）

| ID | 路径 | 操作 | 用途 | 主要改动 |
|:--:|:-----|:----|:-----|:---------|
| C1 | `Cargo.toml` | 修改 | 将 `toml` 从 dev-deps 提升为正式 deps | `toml = "1.0.3"` 移到 `[dependencies]` 区段 |
| C2 | `src/tools/verify.rs` | 新建 | 内部验证引擎 | `FileType` 枚举、`VerifyResult` 结构体、`verify_file()` 函数、文件类型→检查器映射、进程内 JSON/TOML 验证、外部 rustfmt/prettier 调用 |
| C3 | `src/tools/edit.rs` | 修改 | 增加 `verify` 参数 | `EditInput` 加 `verify: bool`、parameters JSON Schema 加 `verify`、execute 结束时条件调用 `verify_file()` |
| C4 | `src/tools/hashline.rs` | 修改 | 增加 `verify` 参数 | `HashlineEditInput` 加 `verify: bool`、parameters JSON Schema 加 `verify`、execute 所有 edits 完成后条件调用验证 |
| C5 | `src/tools/write.rs` | 修改 | 增加 `verify` 参数 | `WriteInput` 加 `verify: bool`、parameters JSON Schema 加 `verify`、execute 结束时条件调用验证 |
| C6 | `src/tools/mod.rs` | 修改 | 注册 verify 模块 | `mod verify;` 声明（`pub(crate)` 不对外暴露） |

---

## 3. 依赖关系（Dependency Plan）

**Dependencies**:
- C1 依赖：无（独立改动）
- C2 依赖：C1（`toml` 作为正式依赖后才能 import）
- C3 依赖：C2（调用 `verify::verify_file()`）
- C4 依赖：C2
- C5 依赖：C2
- C6 依赖：C2（模块注册）

**并行分组**：
- **Phase 1**：C1（toml 提升依赖）
- **Phase 2**：C2（verify 引擎，核心模块）
- **Phase 3**：C3、C4、C5 可并行（三个工具的 verify 参数，模式相同）
- **Phase 4**：C6（mod.rs 注册）

**冲突说明**：C3/C4/C5 虽然可并行，但改动量小且模式一致，建议主 Agent 串行完成。

---

## 4. 验证计划（Validation Plan）

**自动化检查**：
```bash
cargo clippy --all-targets -- -D warnings
cargo test -- --nocapture
cargo fmt --check
```

**预期结果**：
- clippy 无 warning
- 所有现有测试通过
- 格式正确

**人工检查**：
- 确认 `verify` 参数在三个工具的 JSON Schema 中正确显示
- 确认 `details.verify` 在 TUI 中可读
