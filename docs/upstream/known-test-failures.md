---
version: 2
created: 2026-08-16
updated: 2026-08-22
aliases: []
tags: [upstream, test, baseline]
---

## 已知测试失败清单（当前基线）

> 用途：追上游时的对照基准。全量 `cargo test --no-fail-fast` 跑完，将新失败与此清单对比——**清单外的失败 = 本次改动引入，需处理；清单内 = 已知，不动**。  
> 数据来源：`2026-08-22` 全量 `cargo test --no-fail-fast` `e` 基线（`4 failed`，`58187fb8`）+ `Windows` 复核。  
> 约束：本文件只记**当前**仍失败的用例；已修复的不保留（历史修复见 `git log`，如 `285e66b6`/`16b4fafd`/`41488c9b`/`58187fb8`）。

---

### 当前已知失败（4 个 targets，4 个用例）

| # | target | 失败用例 | 根因 | 处置 |
|---|---|---|---|---|
| 1 | `ext_conformance_artifacts` | `test_ext_conformance_artifact_provenance_matches_master_catalog_checksums` + `test_snapshot_protocol_provenance_entries_valid` | `npm/vaayne-agent-kit` vendor checksum `70c002e8… → a5a45c18…` 漂移（`PROVENANCE_VERIFICATION.json` 与本地实际不一致） | 冻结面（③ ext conformance）。等 extensions 架构迁移时重算校验和或定期同步 vendor；不阻塞追上游 |
| 2 | `ext_provenance_verification` | `provenance_verification_evidence_log` | 同上 provenance 链 | 同上 |
| 3 | `non_mock_compliance_gate` | `extension_stub_reconciliation_matches_current_backlog_families` | 同上 vendor 校验传播 | 同上 |
| 4 | `provider_smoke_matrix` | `smoke_openai_completions_matrix` | `Windows` `WSAEWOULDBLOCK (os error 10035)` — `mock_http` 非阻塞套接字在全量并行高负载下偶发 `requests=0`；单跑 `cargo test --test provider_smoke_matrix smoke_openai_completions_matrix` 复现两次均 `1/1 ok` | 环境 Flaky。单跑通过即忽略；全量 `10035` 不追阈值，后续若稳定复现再调 `mock_http` 重试/串行 |

> 校验：`e` 基线 `4 failed` 全命中此表；`b 17 → c 11 → d 5 → e 4` 收敛路径：`A 类 6`（`16b4fafd`）+ `B 可修 6`（`41488c9b` 5 项 + `58187fb8` 1 项）已治愈，不再列入。

---

### 使用说明

1. 每次 `merge` / 大改后跑全量 `cargo test --no-fail-fast`，与本清单对比。
2. **新失败**（清单外）= 本次引入 → 回退数据文件 / 修证据 / 调阈值。
3. **清单内** = 已知，不动。
4. 若清单持续增长 → 单独立项清理（重录 `VCR`、`PROVENANCE_VERIFICATION.json` 重算、`mock_http` 治 `10035`）。
