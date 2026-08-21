# 扩展样本集 (bd-ic9)

本文档概述了 `docs/extension-sample.json` 中定义的 **已冻结样本集**。JSON 是事实来源（source of truth）；本文件是人类可读的概览。

## 快照

- **来源**：commit `df5b0f76c026b35fdd7f0fb78cb0dbaaf939c1b5` 处的 `pi-mono`
- **样本大小**：16（最小 12，最大 20）
- **选型**：已满足 CONFORMANCE.md + EXTENSION_SAMPLING_MATRIX 配额，清单 rationale 中注明了显式替换。
- **校验和**：`checksum.sha256` 已为每个条目填充（见 **制品与校验和**）。

## 制品与校验和

为使一致性可在离线环境下复现，我们为样本集托管（vendor）扩展源文件：

- **制品路径**：`tests/ext_conformance/artifacts/<id>/`
- **溯源**：从 commit `df5b0f76c026b35fdd7f0fb78cb0dbaaf939c1b5` 处的 `legacy_pi_mono_code/pi-mono` 复制（MIT 许可）。
- **校验和存储**：`docs/extension-sample.json` → `items[].checksum.sha256`
- **校验和定义**：制品文件树的仅内容 `sha256`，与平台文件权限/mtime 无关。
  - 递归枚举 `tests/ext_conformance/artifacts/<id>/` 下的所有常规文件。
  - 按规范化相对路径（POSIX `/` 分隔符）排序。
  - 哈希流：对每个文件计算 `b\"file\\0\" + path + b\"\\0\" + bytes + b\"\\0\"`。

## 覆盖率摘要

**运行时层级**
- legacy-js: 8
- multi-file: 4
- pkg-with-deps: 2
- provider-ext: 2

**交互标签**
- tool_only: 5
- slash_command: 4
- event_hook: 6
- ui_integration: 7
- provider: 2
- input_transform: 1

**复杂度**
- small: 3
- medium: 7
- large: 6

**I/O 模式**
- fs-heavy: 6
- network-heavy: 3
- ui-centric: 6
- cpu-heavy: 2
- os-heavy: 4

## 已选扩展

| ID | 路径 | 运行时 | 交互 | 能力 | 复杂度 | I/O |
|---|---|---|---|---|---|---|
| permission-gate | `packages/coding-agent/examples/extensions/permission-gate.ts` | legacy-js | event_hook, ui_integration | exec, env | medium | ui-centric, os-heavy |
| protected-paths | `packages/coding-agent/examples/extensions/protected-paths.ts` | legacy-js | event_hook | read, write | small | fs-heavy |
| todo | `packages/coding-agent/examples/extensions/todo.ts` | legacy-js | tool_only, slash_command, ui_integration | read, write | medium | fs-heavy, ui-centric |
| hello | `packages/coding-agent/examples/extensions/hello.ts` | legacy-js | tool_only | env | small | ui-centric |
| antigravity-image-gen | `packages/coding-agent/examples/extensions/antigravity-image-gen.ts` | legacy-js | tool_only | http, write | medium | network-heavy |
| plan-mode | `packages/coding-agent/examples/extensions/plan-mode` | multi-file | slash_command, ui_integration | read | large | ui-centric |
| status-line | `packages/coding-agent/examples/extensions/status-line.ts` | legacy-js | event_hook, ui_integration | env | small | ui-centric |
| doom-overlay | `packages/coding-agent/examples/extensions/doom-overlay` | multi-file | ui_integration | env | large | cpu-heavy, ui-centric |
| sandbox | `packages/coding-agent/examples/extensions/sandbox` | pkg-with-deps | event_hook, slash_command, ui_integration | exec, read | large | os-heavy, fs-heavy |
| inline-bash | `packages/coding-agent/examples/extensions/inline-bash.ts` | legacy-js | input_transform | exec | medium | os-heavy |
| dynamic-resources | `packages/coding-agent/examples/extensions/dynamic-resources` | multi-file | event_hook | read | medium | fs-heavy |
| custom-provider-anthropic | `packages/coding-agent/examples/extensions/custom-provider-anthropic` | provider-ext | provider | http | large | network-heavy |
| custom-provider-qwen-cli | `packages/coding-agent/examples/extensions/custom-provider-qwen-cli` | provider-ext | provider | exec, http | large | network-heavy |
| with-deps | `packages/coding-agent/examples/extensions/with-deps` | pkg-with-deps | tool_only, slash_command | read, write | medium | fs-heavy |
| subagent | `packages/coding-agent/examples/extensions/subagent` | multi-file | tool_only, ui_integration | exec, read | large | cpu-heavy, os-heavy |
| git-checkpoint | `packages/coding-agent/examples/extensions/git-checkpoint.ts` | legacy-js | event_hook | exec | medium | fs-heavy |

## 向样本集添加新扩展 (bd-1rm)

这是用于扩展已冻结样本集的“新贡献者路径”。

### 0) 开始前

- 决定你是要**添加**新的样本条目还是**更新**现有条目。
- 确认溯源与再分发：当前样本集从固定 commit（MIT 许可）的 `pi-mono` 托管源文件。如果你想引入 gist/npm/社区扩展，**请先停下来验证许可/再分发策略**（不要托管无法再分发的制品）。

### 1) 挑选候选（并说明理由）

1. 从原始清单开始：`docs/EXTENSION_CANDIDATES.md`。
2. 应用配额与选型依据：`docs/EXTENSION_SAMPLING_MATRIX.md`。
3. 优先选择能补充缺失覆盖率的候选（运行时层级、交互标签、I/O 模式、能力）。

### 2) 更新清单（`docs/extension-sample.json`）

1. 在 `items[]` 下添加或修改条目：
   - `id`、`name`、`source`（仓库 + commit + 路径），以及层级/标签元数据。
2. 为新的 `extension_id` 在 `scenario_suite.items[]` 下添加捕获场景：
   - 每个场景必须具有稳定的 `id`（`scn-<ext>-<nnn>`）并声明 `kind` 及相关选择器（`tool_name`、`command_name` 或 `event_name`）。
3. 若样本集构成发生变化，请保持清单的 rationale（理由说明）为最新。

### 3) 托管制品（`tests/ext_conformance/artifacts/<id>/`）

将扩展源文件托管到 `tests/ext_conformance/artifacts/<id>/` 下的新目录中。

- 对于 `pi-mono` 示例，从 `legacy_pi_mono_code/pi-mono/...` 下固定的检出副本复制。
- 保留原始文件名（例如 `todo.ts`、`package.json`、`index.ts`）。

### 4) 计算制品校验和并写入清单

校验和为上文所述的仅内容树摘要（文件路径 + 字节，按 POSIX 路径排序）。

一种计算方式（在仓库根目录执行）：

```bash
python - <<'PY'
import hashlib, os

ext_id = "todo"  # <-- set this
root = os.path.join("tests", "ext_conformance", "artifacts", ext_id)

files = []
for dirpath, dirnames, filenames in os.walk(root):
    dirnames.sort()
    for name in sorted(filenames):
        path = os.path.join(dirpath, name)
        rel = os.path.relpath(path, root).replace(os.sep, "/")
        files.append((rel, path))

h = hashlib.sha256()
for rel, path in sorted(files):
    h.update(b"file\0")
    h.update(rel.encode("utf-8"))
    h.update(b"\0")
    with open(path, "rb") as f:
        h.update(f.read())
    h.update(b"\0")

print(h.hexdigest())
PY
```

将该值写入 `docs/extension-sample.json` 的 `items[].checksum.sha256`。

### 5) 重新运行旧版捕获并重新生成夹具

1. 运行捕获流水线：
   - 全量套件：`cargo run --features internal-legacy-capture --bin pi_legacy_capture`
   - 单个场景：`cargo run --features internal-legacy-capture --bin pi_legacy_capture -- --scenario-id scn-<ext>-<nnn>`
2. 确认夹具已存在：
   - `tests/ext_conformance/fixtures/<extension_id>.json`

### 6) 验证（测试）

提交前运行以下命令：

```bash
cargo test ext_conformance_artifacts_match_manifest_checksums
cargo test ext_conformance_pinned_sample_compat_ledger_snapshot
```

然后运行常规项目质量门禁：

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
```

## 后续步骤

1. 在 `docs/extension-sample.json`（`scenario_suite`）中定义每个扩展的捕获场景（bd-2qd）。
2. 实现旧版捕获流水线以运行场景并记录输出（bd-3on），然后对路径/时间/随机性进行归一化（bd-1oz）。
3. 将此清单 + 制品作为一致性与基准运行的规范样本。

### 旧版捕获归一化 (bd-1oz)

`pi_legacy_capture` 会在原始捕获输出旁写入归一化制品：
- `stdout.normalized.jsonl`
- `meta.normalized.json`
- `capture.normalized.log.jsonl`

归一化规则（去除非确定性，保留语义）：
- 将 RFC3339 时间戳字符串替换为 `<TIMESTAMP>`，并将数值型 `timestamp` 字段替换为 `0`。
- 将仓库下的绝对路径重写为 `<PROJECT_ROOT>`，将旧版仓库根重写为 `<PI_MONO_ROOT>`。
- 将 `run-<uuid>` 重写为 `<RUN_ID>`，将裸 UUID 重写为 `<UUID>`。
- 将模拟 OpenAI 基础 URL 重写为 `http://127.0.0.1:<PORT>/v1`。
- 将 `Total output lines: N` 重写为 `Total output lines: <N>`。

## 重新生成旧版夹具 (bd-16n / bd-vbs)

本节是用于复现已提交旧版夹具的“新维护者路径”。

### 生成内容

- **原始捕获制品**（每次场景运行一个目录）：`target/legacy_capture/<scenario_id>/<run_id>/`
  - `stdout.jsonl`、`stderr.txt`、`meta.json`、`capture.log.jsonl`
  - 以及归一化副本：`stdout.normalized.jsonl`、`meta.normalized.json`、`capture.normalized.log.jsonl`
- **黄金夹具输出**（每个扩展一个文件）：`tests/ext_conformance/fixtures/<extension_id>.json`
  - Schema: `pi.ext.legacy_fixtures.v1`
  - 捕获溯源（旧版 pi-mono HEAD、node/npm 版本、清单 commit/校验和等）

### 前置条件

- Rust nightly 工具链（见 `rust-toolchain.toml`）
- PATH 上可用的 Node + npm
- 旧版 pi-mono 工作区已安装依赖（需要 `legacy_pi_mono_code/pi-mono/node_modules/tsx/...`）

若看到 `missing tsx runner`，请运行：

```bash
cd legacy_pi_mono_code/pi-mono
npm install
```

### 验证 Pin

样本集通过以下方式固定旧版参考源：

- `docs/extension-sample.json` → `source_commit`（用于选定样本的 pi-mono 修订）
- `docs/extension-sample.json` → `items[].source.commit` + `items[].checksum.sha256`（每个扩展的溯源）

捕获工具会在每个场景的 `meta.normalized.json` 中记录实际使用的旧版 pi-mono 检出，位于：

- `pi_mono.head`
- `pi_mono.extension_path`
- `pi_mono.manifest_commit`
- `pi_mono.manifest_checksum_sha256`

### 运行全量捕获

在仓库根目录执行：

```bash
cargo run --features internal-legacy-capture --bin pi_legacy_capture
```

默认值（见 `src/bin/pi_legacy_capture.rs`）：

- 清单：`docs/extension-sample.json`
- 旧版根：`legacy_pi_mono_code/pi-mono`
- 原始输出目录：`target/legacy_capture`
- 夹具目录：`tests/ext_conformance/fixtures`
- 确定性/离线：`--no-env`（默认为 true）
- 每个场景超时：`--timeout-secs 20`

### 运行单个场景（调试）

```bash
cargo run --features internal-legacy-capture --bin pi_legacy_capture -- --scenario-id scn-todo-003
```

这在只需修复单个场景而无需重新生成全部内容时很有用。

### 确定性说明

`pi_legacy_capture` 旨在使运行可复现：

- 设置 `TZ=UTC`。
- 针对本地模拟 OpenAI 服务器运行旧版 pi-mono，以获得可预测的流式与工具调用事件。
- 支持来自清单的按场景模拟钩子：
  - `setup.mock_exec`：生成 `node_preload.cjs` 以桩化（stub）`child_process.spawn`。
  - `setup.mock_http`：桩化 `fetch()` 以支持离线的“图像生成”夹具。
  - `setup.session_branch`：写入 `seed_session.jsonl` 以预加载会话历史（例如 toolResult 详情）。

### 故障排查

- **超时/挂起：** 使用更高超时重新运行，并检查 `target/legacy_capture/...` 下场景目录中的 `stderr.txt` + `capture.log.jsonl`。
- **Node preload 未生效：** 确认场景已写入 `node_preload.cjs` 且 `meta.json` 包含 `node_preload`；旧版 pi-mono 应通过 `NODE_OPTIONS=--require <绝对路径>` 接收它。
- **种子会话失败：** 检查 `seed_session.jsonl` 是否存在格式错误的消息；已植入的 `toolResult` 条目必须包含 `toolCallId`、`toolName`、`content`（数组）、`isError` 和数值型 `timestamp`。
