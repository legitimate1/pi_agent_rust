## 分层扩展采样矩阵

该矩阵为扩展后的扩展语料定义了一套确定性的、分层选择策略，基于：

- `docs/EXTENSION_POPULARITY_CRITERIA.md`（评分 + 门禁）、
- `docs/extension-candidate-pool.json`（原始候选）以及
- `docs/extension-individual-enumeration.json`（单个扩展覆盖率统计）。

它刻意采用**分层**设计：

- `Tier-0`：官方基线（全部官方 pi-mono 扩展）、
- `Tier-1`：必过语料（**>= 200** 个未经修改的扩展）、
- `Tier-2`：额外的长尾/延伸覆盖。

该矩阵并非对 `docs/extension-sample.json` 中已冻结的 16 扩展运行时采样的替代；该文件仍作为精简的对等性 harness 基线保留。

> **注意：** 下述标签均**推断**自 README/描述。应在最终选型前通过静态扫描进行校验与调整。

---

## 1) 采样维度与配额（分层语料）

### 1.1 选型规模目标

| 层级     | 目标                           |
| -------- | ------------------------------ |
| `tier-0` | 包含全部官方 pi-mono 扩展      |
| `tier-1` | 选择 **>= 200** 个扩展（必过） |
| `tier-2` | 选择全部其他符合条件的长尾条目 |

### 1.2 配额公式（确定性 + 可行）

对于每个分桶：

`required(bucket) = min(available(bucket), ceil(tier1_target * ratio(bucket)))`

其中：

- `tier1_target = 200`
- `available(bucket)` 由最新候选快照计算得出
- 下述比例编码了期望的行为覆盖度

### 1.3 Tier-1 源层级最小值（绝对值）

使用 `docs/extension-individual-enumeration.json` 可用性快照（`total = 214`）：

| 源层级               | 可用数 | Tier-1 最小值 |
| -------------------- | -----: | ------------: |
| `official-pi-mono`   |     60 |            60 |
| `community`          |     55 |            50 |
| `third-party-github` |     59 |            50 |
| `npm-registry`       |     37 |            37 |
| `agents-mikeastock`  |      3 |             3 |

这些最小值合计为 **200**。

### 1.4 Tier-1 行为分桶目标

行为目标受配额管控，并以可用性为上限：

| 分桶                       |      比例目标 | 可用性来源                                            |
| -------------------------- | ------------: | ----------------------------------------------------- |
| `event_hook`               |          0.40 | `by_capability.event_hook`                            |
| `registerTool`             |          0.25 | `by_capability.registerTool`                          |
| `registerShortcut`         |          0.07 | `by_capability.registerShortcut`                      |
| `registerFlag`             |          0.04 | `by_capability.registerFlag`                          |
| `registerProvider`         |          0.02 | `by_capability.registerProvider`                      |
| `exec_api`                 |          0.12 | `by_capability.exec_api`                              |
| `session_api`              |          0.05 | `by_capability.session_api`                           |
| `ui_header` / `ui_overlay` | 0.25 combined | `by_capability.ui_header`, `by_capability.ui_overlay` |

若 `available(bucket) < ceil(200 * ratio)`，则在选型输出中将该分桶标记为 `availability_limited=true`，并包含缺口数量。

---

## 2) 候选标签映射（全部候选）

**图例：**  
交互标签 = `tool_only`, `slash_command`, `event_hook`, `ui_integration`, `provider`, `input_transform`  
能力 = `read`, `write`, `exec`, `http`, `env`  
运行时 = `legacy-js`, `multi-file`, `pkg-with-deps`, `provider-ext`, `gist`, `pi-package`  
I/O = `fs-heavy`, `network-heavy`, `ui-centric`, `cpu-heavy`, `os-heavy`

### A) pi‑mono 示例扩展

| 候选                     | 运行时     | 交互                          | 能力        | 复杂度 | I/O        |
| ------------------------ | ---------- | ----------------------------- | ----------- | ------ | ---------- |
| `permission-gate.ts`     | legacy-js  | event_hook, ui_integration    | exec, env   | medium | ui-centric |
| `protected-paths.ts`     | legacy-js  | event_hook                    | write, read | small  | fs-heavy   |
| `confirm-destructive.ts` | legacy-js  | slash_command, ui_integration | env         | small  | ui-centric |
| `dirty-repo-guard.ts`    | legacy-js  | event_hook                    | exec        | small  | fs-heavy   |
| `sandbox/`               | multi-file | event_hook                    | exec        | large  | os-heavy   |

| 候选                       | 运行时     | 交互                                     | 能力        | 复杂度 | I/O           |
| -------------------------- | ---------- | ---------------------------------------- | ----------- | ------ | ------------- |
| `todo.ts`                  | legacy-js  | tool_only, slash_command, ui_integration | write, read | medium | fs-heavy      |
| `hello.ts`                 | legacy-js  | tool_only                                | env         | small  | ui-centric    |
| `question.ts`              | legacy-js  | tool_only, ui_integration                | env         | small  | ui-centric    |
| `questionnaire.ts`         | legacy-js  | tool_only, ui_integration                | env         | medium | ui-centric    |
| `tool-override.ts`         | legacy-js  | event_hook, tool_only                    | read, write | medium | fs-heavy      |
| `truncated-tool.ts`        | legacy-js  | tool_only                                | exec        | medium | fs-heavy      |
| `antigravity-image-gen.ts` | legacy-js  | tool_only                                | http, write | medium | network-heavy |
| `ssh.ts`                   | legacy-js  | tool_only                                | exec, http  | large  | network-heavy |
| `subagent/`                | multi-file | tool_only                                | exec        | large  | cpu-heavy     |

| 候选                      | 运行时     | 交互                          | 能力  | 复杂度 | I/O           |
| ------------------------- | ---------- | ----------------------------- | ----- | ------ | ------------- |
| `preset.ts`               | legacy-js  | slash_command, ui_integration | env   | medium | ui-centric    |
| `plan-mode/`              | multi-file | slash_command, ui_integration | read  | large  | ui-centric    |
| `tools.ts`                | legacy-js  | slash_command, ui_integration | env   | medium | ui-centric    |
| `handoff.ts`              | legacy-js  | slash_command                 | write | medium | fs-heavy      |
| `qna.ts`                  | legacy-js  | slash_command, ui_integration | env   | small  | ui-centric    |
| `status-line.ts`          | legacy-js  | ui_integration                | env   | small  | ui-centric    |
| `widget-placement.ts`     | legacy-js  | ui_integration                | env   | small  | ui-centric    |
| `model-status.ts`         | legacy-js  | event_hook, ui_integration    | env   | small  | ui-centric    |
| `snake.ts`                | legacy-js  | ui_integration                | env   | large  | cpu-heavy     |
| `space-invaders.ts`       | legacy-js  | ui_integration                | env   | large  | cpu-heavy     |
| `send-user-message.ts`    | legacy-js  | slash_command                 | env   | small  | ui-centric    |
| `timed-confirm.ts`        | legacy-js  | ui_integration                | env   | small  | ui-centric    |
| `rpc-demo.ts`             | legacy-js  | ui_integration                | env   | medium | ui-centric    |
| `modal-editor.ts`         | legacy-js  | ui_integration                | env   | large  | ui-centric    |
| `rainbow-editor.ts`       | legacy-js  | ui_integration                | env   | medium | ui-centric    |
| `notify.ts`               | legacy-js  | event_hook, ui_integration    | exec  | medium | os-heavy      |
| `titlebar-spinner.ts`     | legacy-js  | ui_integration                | env   | small  | ui-centric    |
| `summarize.ts`            | legacy-js  | slash_command, tool_only      | http  | medium | network-heavy |
| `custom-footer.ts`        | legacy-js  | ui_integration                | env   | small  | ui-centric    |
| `custom-header.ts`        | legacy-js  | ui_integration                | env   | small  | ui-centric    |
| `overlay-test.ts`         | legacy-js  | ui_integration                | env   | medium | ui-centric    |
| `overlay-qa-tests.ts`     | legacy-js  | ui_integration                | env   | large  | ui-centric    |
| `doom-overlay/`           | multi-file | ui_integration                | exec? | large  | cpu-heavy     |
| `shutdown-command.ts`     | legacy-js  | slash_command                 | env   | small  | ui-centric    |
| `interactive-shell.ts`    | legacy-js  | event_hook                    | exec  | medium | os-heavy      |
| `inline-bash.ts`          | legacy-js  | input_transform               | exec  | medium | os-heavy      |
| `bash-spawn-hook.ts`      | legacy-js  | event_hook                    | exec  | small  | os-heavy      |
| `input-transform.ts`      | legacy-js  | event_hook                    | env   | small  | ui-centric    |
| `system-prompt-header.ts` | legacy-js  | event_hook                    | env   | small  | ui-centric    |

| 候选                     | 运行时    | 交互       | 能力 | 复杂度 | I/O      |
| ------------------------ | --------- | ---------- | ---- | ------ | -------- |
| `git-checkpoint.ts`      | legacy-js | event_hook | exec | medium | fs-heavy |
| `auto-commit-on-exit.ts` | legacy-js | event_hook | exec | medium | fs-heavy |

| 候选                   | 运行时    | 交互          | 能力 | 复杂度 | I/O        |
| ---------------------- | --------- | ------------- | ---- | ------ | ---------- |
| `pirate.ts`            | legacy-js | event_hook    | env  | small  | ui-centric |
| `claude-rules.ts`      | legacy-js | event_hook    | read | medium | fs-heavy   |
| `custom-compaction.ts` | legacy-js | event_hook    | env  | medium | ui-centric |
| `trigger-compact.ts`   | legacy-js | slash_command | env  | small  | ui-centric |

| 候选                  | 运行时     | 交互           | 能力 | 复杂度 | I/O        |
| --------------------- | ---------- | -------------- | ---- | ------ | ---------- |
| `mac-system-theme.ts` | legacy-js  | event_hook     | env  | small  | os-heavy   |
| `dynamic-resources/`  | multi-file | event_hook     | read | medium | fs-heavy   |
| `message-renderer.ts` | legacy-js  | ui_integration | env  | medium | ui-centric |
| `event-bus.ts`        | legacy-js  | event_hook     | env  | medium | ui-centric |
| `session-name.ts`     | legacy-js  | event_hook     | env  | small  | ui-centric |
| `bookmark.ts`         | legacy-js  | event_hook     | env  | small  | ui-centric |

| 候选                          | 运行时       | 交互     | 能力       | 复杂度 | I/O           |
| ----------------------------- | ------------ | -------- | ---------- | ------ | ------------- |
| `custom-provider-anthropic/`  | provider-ext | provider | http       | large  | network-heavy |
| `custom-provider-gitlab-duo/` | provider-ext | provider | http       | large  | network-heavy |
| `custom-provider-qwen-cli/`   | provider-ext | provider | exec, http | large  | network-heavy |

| 候选              | 运行时        | 交互       | 能力        | 复杂度 | I/O      |
| ----------------- | ------------- | ---------- | ----------- | ------ | -------- |
| `with-deps/`      | pkg-with-deps | mixed      | read, write | medium | fs-heavy |
| `file-trigger.ts` | legacy-js     | event_hook | read        | small  | fs-heavy |

### B) 仓库本地 `.pi/extensions`

| 候选                                  | 运行时    | 交互                          | 能力 | 复杂度 | I/O           |
| ------------------------------------- | --------- | ----------------------------- | ---- | ------ | ------------- |
| `.pi/extensions/diff.ts`              | legacy-js | slash_command, ui_integration | exec | medium | fs-heavy      |
| `.pi/extensions/files.ts`             | legacy-js | slash_command, ui_integration | read | small  | fs-heavy      |
| `.pi/extensions/prompt-url-widget.ts` | legacy-js | ui_integration                | http | medium | network-heavy |
| `.pi/extensions/redraws.ts`           | legacy-js | ui_integration                | env  | small  | ui-centric    |

### C) badlogic gists

| 候选                   | 运行时 | 交互                          | 能力  | 复杂度 | I/O      |
| ---------------------- | ------ | ----------------------------- | ----- | ------ | -------- |
| `review-extension*.ts` | gist   | slash_command, ui_integration | write | medium | fs-heavy |
| `diff.ts`              | gist   | slash_command, ui_integration | exec  | medium | fs-heavy |

### D) 社区 / npm / git 软件包

| 候选        | 运行时     | 交互           | 能力       | 复杂度 | I/O           |
| ----------- | ---------- | -------------- | ---------- | ------ | ------------- |
| `agentsbox` | pi-package | tool_only      | exec, http | medium | network-heavy |
| `pi-doom`   | pi-package | ui_integration | exec       | large  | cpu-heavy     |

---

## 3) 如何应用该矩阵

1. 使用可执行评分规则（`src/extension_scoring.rs`）计算候选得分，并持久化排序后的输出（`pi.ext.scoring.v1`）。
2. 应用硬性门禁（`provenance_pinned`、许可证再分发、确定性场景、未经修改兼容性）。被排除的候选不计入配额。
3. 优先分配 Tier-0（全部官方 pi-mono），然后按得分顺序填充 Tier-1 至 `>=200`。
4. 使用 §1.2 中的公式强制执行源层级最小值与行为分桶配额。
5. 当配额无法满足时，记录 `availability_limited` 与缺口元数据。
6. 发布一份机器可消费的选型产物，其中包含：
   - 按候选的得分明细、
   - 所选层级（`tier-0|tier-1|tier-2|excluded`）、
   - 配额满足情况摘要、
   - 显式的手动覆盖（如有）。
