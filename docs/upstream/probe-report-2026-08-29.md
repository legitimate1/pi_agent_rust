# 探针报告 2026-08-29：全量追上游试合 (121→0) 与 297 编译缺口分类

> 状态：**归档 / 不落 custom**。`trial-wt` 已探完弃用，主分支 `custom (ae3f4b78)` 零改动。下次追上游走**最小移植 (hub 闭包)**，不再全量 `merge upstream/main`。

## 1. 探针输入

- 基线：`main = 92e5884a` (v0.1.22 发布)、`custom = ae3f4b78` (v0.1.82)、`upstream/main = b7b5988b` (2026-08-29)
- 差距：`main..upstream/main = 1411 commits / 1482 files`、`main...custom = 562 files / +128661 -29360`
- custom 重构区：`interactive/*、hostcall_*/touched_files、tools/*(split)、extensions/*(删除)、asupersync runtime`
- 上游 subagent 已演进为 **hub 史诗 (bd-cv653)**：`hub.rs 1050 + jobs.rs 5008 + agent_hub.rs 539 + subagents.rs 2368 = 8965 行`，依赖散在 `app/cli/config/rpc/session/compaction/extensions/interactive`
- 方法：`git worktree add --detach /tmp/pi-trial-wt custom && git merge --no-commit --no-ff upstream/main` — 唯一可信冲突数来源（`merge-tree` 预演会骗人）

## 2. 探针结果总览

| 指标                           | 值                                                                  |
| ------------------------------ | ------------------------------------------------------------------- |
| 初始 `UU`                      | **121**                                                             |
| 批量冻结后 `UU`                | **46** → 根配置+examples 再冻后 **36**                              |
| `3×subagent` 并行清 36 后 `UU` | **0**，`<<<<<<<` 全仓 **0**                                         |
| 白拿 `A`                       | `836 → 339`（`497` 证据噪音 `rm --cached`）                         |
| `cargo check --lib`            | 2 处语法破裂后 **297 errors / 19 warnings**                         |
| 结论                           | 源码冲突是纸老虎，**依赖/API 漂移是主成本**，全量合流不值得此刻付费 |

## 3. 121 冲突分类与冻结路径

按 `git diff --name-only --diff-filter=U` 实测：

| 桶                                                                                                                | 数               | 处理                                                                | 复用脚本      |
| ----------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------- | ------------- |
| `.beads/*`                                                                                                        | 5                | `git rm -rf .beads`                                                 | 1             |
| `src/extensions/*`                                                                                                | 18               | `git rm -rf src/extensions/*`（本次冻结面）                         | 1             |
| `src/subagents.rs, src/tools.rs`                                                                                  | 2                | `subagents → --theirs`（要 hub），`tools → rm`（保 `tools/*` 拆分） | 1             |
| `docs/evidence, contracts, perf, artifacts`                                                                       | 497 `A` + 4 `UU` | `git rm --cached` / `--ours`                                        | 1             |
| `tests/*`                                                                                                         | 47 `UU`          | `--ours`                                                            | 1             |
| `benches/*, .github/*, docs/*`                                                                                    | ~10              | `--ours`                                                            | 1             |
| 根配置 `Cargo.toml/lock, .cargo/config.toml, .gitignore, AGENTS.md, README.md, examples/*, benches/extensions.rs` | 8                | `--ours` 回 `custom 0.1.82 / rust 1.85 / asupersync 0.3.9`          | 1             |
| **剩余 `src/*` 内容冲突**                                                                                         | **36**           | **唯一需人看**                                                      | 并行 subagent |

### 3.1 36 的 Tier 拆分（已验 `0 UU`）

- **Tier1 hub/会话 12**：`agent.rs, app.rs, cli.rs, config.rs, rpc.rs, session*.rs×5, compaction*.rs×2` — hub 注入点，重冲突取 `upstream` 全量保编译，custom `touched_files/asupersync` 标 `TODO` 回补
- **Tier2 扩展/TUI 10**：`extension_dispatcher.rs, extensions.rs, extensions_js.rs, interactive*.rs×6, perf.rs` — 保 custom 骨架，仅 `interactive/commands.rs` 植入最小 `hub roster/jobs` 钩子 (`completed_tan_event`)，其余上游大段丢弃
- **Tier3 其他 14**：`auth.rs, lib.rs, main.rs, models.rs(9866↑), package_manager.rs, perf_build.rs, providers×5, resources.rs, sdk.rs, semantic_workspace_graph.rs` — custom 为主、合入上游小增量

> 破裂教训：`>1500 行` 大文件的并行合并易丢 `}`。`perf.rs 2955` 与 `semantic_workspace_graph.rs 6266` 均破裂，治法：`git show upstream/main:src/xxx > src/xxx` 全量覆盖（已在 trial-wt 执行，模式可复用）。

### 3.2 白拿 `A` 清单

- 保留 `76 src/*` 含 `hub.rs, jobs.rs, agent_hub.rs, subagents.rs, github.rs, btw.rs, checkpoint.rs, handoff.rs, mcp.rs, lsp/*, eval/*` 等
- 已 `rm --cached`：`docs/evidence/*, docs/contracts/*, tests/perf/*, tests/ext_conformance/artifacts/*, tests/evidence_bundle/*` 共 `497`
- 剩 `339A` 中 `scripts/* 37, tests/* ~120, src/* 76, examples/* 11, other 50`

## 4. Cargo/API 漂移：297 分类（第三轮 `cargo check --lib`）

`Cargo.toml --ours` 保 `custom` 基线，但让 `hub/jobs` 新代码找不到依赖；切 `--theirs` 则 `asupersync 0.3.9→0.4.4 / rust 1.85→1.95 / digest 0.10→0.11 / fs4 0.13→1.1` 引 400+ 错（与 SOP 423 同量级）。当前为前者：

| 桶                                    | 数   | 样例                                                                                                                                                                                                                                                                                                            | 修法                                      | 成本         |
| ------------------------------------- | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- | ------------ |
| `Tool::execute 4→5 参`                | ~130 | `src/tools/mod.rs` 已合为 `abort: Option<AbortSignal>`，但 `computer.rs, debug.rs, eval.rs, github.rs, lsp.rs, mcp.rs, media_tools.rs×3, memory.rs×4, plan.rs, security_scan.rs, todo.rs, web_search.rs, xdev.rs` 等 20+ `impl Tool` 仍 4 参                                                                    | 机械加参 `, _abort: Option<AbortSignal>`  | 低 token、广 |
| `ExtensionSession/HostActions 2→3 参` | ~12  | `ext_session.rs/session.rs` 保留 `origin: Option<SessionActionOrigin>`，`extensions.rs` 取 upstream 无 `origin` 版，trait arity 不配                                                                                                                                                                            | 二选一：全回 custom 版或删 `origin`       | 中           |
| `missing crate`                       | ~15  | `globset, fsqlite 0.3.5(native+fts5), portable-pty 0.9, tiktoken-rs 0.12(optional), pprof, rustix, htmd, jsonschema…`                                                                                                                                                                                           | 按需 `cargo add`，勿全跟 `upstream 0.3.0` | 低           |
| `missing fn/type`                     | ~30  | `extensions.rs: js_runtime_remaining_timeout×9, remaining_js_task_timeout, is_likely_flat_extension_entry, discover_auxiliary_example_entries` / `extensions_js.rs: log_host_fs_decision×2` / `session_store_v2.rs: ArtifactWriteMode, open_regular_file_for_write, path_entry_exists, open_private_directory…` | 补 helpers 或 `extensions.rs` 取 custom   | 中           |
| `2 处语法破裂`                        | 2    | `perf.rs:1384 spawn_with_cx(`, `semantic:5982`                                                                                                                                                                                                                                                                  | 已全量覆盖治好                            | —            |

## 5. 决策：本次不全量合流

- **已得**：冻结脚本沉淀、`36→0` 可 `3 subagent` 零主上下文复刻、`hub` 最小闭包定位完成
- **待付**：`~297` 跨 `40+` 文件机械改 + `6` crate + `30` helper — 估 `30k token`，且与 `hostcall/touched_files/asupersync` 重构区正面撞
- 符合 `docs/upstream/fork-merge-sop.md` 结论：**挑着合，不一次吞；依赖漂移 > 源码冲突**

## 6. 下次：最小移植（hub 闭包，约 8-12 文件）

不 `merge upstream/main`，另起 `worktree` 只移植：

```
必选：src/hub.rs, src/jobs.rs, src/agent_hub.rs, src/subagents.rs
可选：src/checkpoint.rs, src/handoff.rs, src/github.rs, src/btw.rs（按需）
注入点：src/app.rs, src/cli.rs, src/config.rs, src/rpc.rs 的 hub 钩子（Tier1 已探出位置）
适配层：为 custom 的 `Tool::execute 4参` 写 shim `fn adapt(abort: Option<AbortSignal>)`，不改 20+ impl
Cargo：仅加 `portable-pty, fsqlite, globset` 3 个，多余 `tiktoken/pprof/ftui` 本次不跟
```

## 7. 复用脚本（一键冻结，pwsh）

在 `trial merge --no-commit` 后执行，再并行解剩余 `src/*`：

```pwsh
$wt="C:\Users\m\AppData\Local\Temp\pi-trial-wt"; Push-Location $wt
git rm -rf .beads
$docs = git diff --name-only --diff-filter=U | Where-Object { $_ -like "docs/*" -or $_ -like "tests/*" -or $_ -like "benches/*" }
foreach($f in $docs){ $o=git checkout --ours -- $f 2>&1; if($LASTEXITCODE -ne 0){ git rm -rf $f } else{ git add $f } }
git rm -rf src/extensions; git rm -rf src/extensions 2>$null
foreach($f in @(".cargo/config.toml",".gitignore","AGENTS.md","Cargo.lock","Cargo.toml","README.md","examples/pi_debug.rs","examples/pijs_workload.rs","docs/TEST_COVERAGE_MATRIX.md","docs/e2e_scenario_matrix.json","docs/provider-canonical-id-table.json","docs/traceability_matrix.json","benches/extensions.rs")){ if(git diff --name-only --diff-filter=U | Where-Object {$_ -eq $f}){ git checkout --ours -- $f; git add $f } }
git checkout --theirs -- src/subagents.rs; git add src/subagents.rs
git rm -f src/tools.rs 2>$null; if(Test-Path src/tools.rs){ Remove-Item src/tools.rs -Force }
# 去 497 证据噪音
$rm = git status --short | Where-Object {$_ -match "^A "} | ForEach-Object {($_ -split "\s+",2)[1]} | Where-Object {$_ -like "docs/evidence/*" -or $_ -like "docs/contracts/*" -or $_ -like "tests/perf/*" -or $_ -like "tests/ext_conformance/artifacts/*"}
foreach($f in $rm){ git rm --cached -r --quiet $f 2>$null }
git diff --name-only --diff-filter=U  # 应 36
Pop-Location
```

大文件破裂后修复：

```pwsh
git show upstream/main:src/interactive/perf.rs > src/interactive/perf.rs; git add src/interactive/perf.rs
git show upstream/main:src/semantic_workspace_graph.rs > src/semantic_workspace_graph.rs; git add src/semantic_workspace_graph.rs
```

## 8. 证据与清理

- `trial-wt`：`C:\Users\m\AppData\Local\Temp\pi-trial-wt`，`36 UU → 0` 已验，`cargo check` 三轮日志在本报告 §4
- 冻结清单（本次未合）：`src/extensions/* 重构、Cargo 1.95/asupersync 0.4.4 升级、docs/evidence/contracts/perf 证据包、tiktoken/pprof/ftui 可选依赖`
- 操作：本报告落 `docs/upstream/` 后 `git worktree remove /tmp/pi-trial-wt --force`，主 `custom` 保持 `ae3f4b78`

---

_探针执行：2026-08-29 22:00-01:00 (pwsh, MSVC, worktree)；复核：`git diff --name-only --diff-filter=U = 0`, `grep <<<<<<< = 0`, `cached src 36-2(MISS bedrock/model_fetch 因无 diff)==0`。_
