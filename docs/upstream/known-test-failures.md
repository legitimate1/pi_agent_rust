---
version: 3
created: 2026-08-16
updated: 2026-08-26
aliases: []
tags: [upstream, test, baseline]
---

## 已知测试失败清单（当前基线）

> 用途：追上游时的对照基准。全量 `cargo test --no-fail-fast` 跑完，将新失败与此清单对比——**清单外的失败 = 本次改动引入，需处理；清单内 = 已知，不动**。  
> 数据来源：`2026-08-26` `my-check` Linux 全量验证（`4740fb12` / Run `32947095297` `4/4 PASS`）+ `2026-08-22` Windows 复核基线（`58187fb8` `4 failed`）。  
> 约束：本文件只记**当前**仍失败的用例；已修复的不保留（历史修复见 `git log`，如 `285e66b6`/`16b4fafd`/`41488c9b`/`58187fb8`/`ebae0ee8`/`719b6070`/`4740fb12`）。

---

### 当前已知失败（按平台区分）

#### Linux (`my-check` `ubuntu-latest`，`4740fb12` Run `32947095297`) — 0 个

> `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + `cargo nextest run --all-targets`（`VCR_MODE=playback`，4 分片）`4/4 PASS`。`2026-08-22` 的 `ext_conformance` 3 例（`70c002e8… → a5a45c18…`）已在 `ebae0ee8`/`5b313075`/`36cf2e9e` 同步并在本次验证中自愈；`tui_snapshot` 版本横幅漂移已在 `719b6070` hermetic 隔离；`security_budgets` 1MB 阈值已在 `4740fb12` 抬至 4MB 后通过。

#### Windows（本地全量，`2026-08-22` 基线延续）— 1 个

| # | target | 失败用例 | 根因 | 处置 |
|---|---|---|---|---|
| 1 | `provider_smoke_matrix` | `smoke_openai_completions_matrix` | `WSAEWOULDBLOCK (os error 10035)` — `mock_http` 非阻塞套接字在全量并行高负载下偶发 `requests=0`；单跑 `cargo test --test provider_smoke_matrix smoke_openai_completions_matrix` 两次均 `1/1 ok` | 环境 Flaky。单跑通过即忽略；全量 `10035` 不追阈值，后续若稳定复现再调 `mock_http` 重试/串行 |

> 校验：Linux `0 failed` 已由 `32947095297` 验证；`b 17 → c 11 → d 5 → e 4 → 2026-08-26 Linux 0` 收敛路径：`A 类 6`（`16b4fafd`）+ `B 可修 6`（`41488c9b` 5 项 + `58187fb8` 1 项）+ `C 校验 3`（`ebae0ee8` 等）+ `D 快照/阈值 3`（`719b6070`/`4740fb12`）已治愈，不再列入；Windows 1 例为平台特有，未在 Linux 验证范围内。

---

### 使用说明

1. 每次 `merge` / 大改后以 `my-check`（Linux）为准跑全量，与本清单对比；Windows 单跑 `cargo test --test provider_smoke_matrix smoke_openai_completions_matrix` 复核。
2. **新失败**（清单外）= 本次引入 → 回退数据文件 / 修证据 / 调阈值。
3. **清单内** = 已知，不动。
4. 若清单持续增长 → 单独立项清理（重录 `VCR`、`PROVENANCE_VERIFICATION.json` 重算、`mock_http` 治 `10035`）。
