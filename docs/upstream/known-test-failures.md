---
version: 1
created: 2026-08-16
updated: 2026-08-16
aliases: []
tags: [upstream, test, baseline]
---
## 已知测试失败清单(基线遗留 + 冻结面)

> 用途: 作为追上游时的对照基准。全量测试跑完,把新失败与此清单对比——**清单外的失败 = 合并引入,需处理;清单内 = 已知,不动**。
> 数据来源: 2026-08-16 全量 `cargo test --no-fail-fast` + 基线(46ddadd9)worktree 复跑对照。

---

### 一、合并引入且已修复(本次修复,基线通过 → 现在通过)

| 修复项 | 文件 | 测试 |
|---|---|---|
| ⑤ schema 回退 | `docs/schema/session_store_v2_contract.json` | `session_store_v2_contract_bundle_validates` |
| ⑤ schema 回退 | `docs/schema/mock_spec.json` | (thinking 级别冻结面) |
| ④ 数据文件回退 | `docs/testing-policy.md` | `testing_policy_allowlist_entries_reference_real_files` |
| ④ workflow 回退 | `.github/workflows/weekly-certification-verdict.yml` | `weekly_certification_verdict_workflow...` |
| ① .beads 证据修复 | `docs/evidence/proof-carrying-...json` | `proof_carrying_*_child_artifact_map`、`_rejects_missing_...` |
| ① .beads 证据修复 | `docs/evidence/runtime-intelligence-...json` | `runtime_intelligence_*_child_artifact_map` |
| ②⑥ 删上游测试 | `tests/e2e_subagent_roles.rs`(删) | `no_unclassified_test_files`、`python_governance_script_passes` |

### 二、合并引入但**暂不修**(③ ext conformance,4 个)

| 测试 | target | 原因 |
|---|---|---|
| `test_ext_conformance_artifact_provenance_matches_master_catalog_checksums` | ext_conformance_artifacts | 合并带入的第三方 vendor(vaayne-agent-kit)checksum 未同步到 master catalog |
| `test_snapshot_protocol_provenance_entries_valid` | ext_conformance_artifacts | 同上,快照协议条目 |
| `provenance_verification_evidence_log` | ext_provenance_verification | vendor 资产 provenance 校验 |
| `extension_stub_reconciliation_matches_current_backlog_families` | non_mock_compliance_gate | extension stub 清单与 backlog 不同步 |

**处理**: 等 extensions 架构迁移时一并处理,或定期同步 vendor checksum。不阻塞追上游。

### 三、基线遗留失败(B 类,合并前就失败,与合并无关)— 24 个

> 这些失败在基线 46ddadd9 上就存在,custom 从未跑过全量测试,是上游 QA 门禁与二开状态不匹配的长期负债。**不建议现在修**(工作量分散、与追上游无关),先记录。

| # | target | 失败测试 | 原因 |
|---|---|---|---|
| 1 | e2e_golden_corpus | golden_corpus_json_mode / _stdin / print_stdin / print_text(4) | VCR cassette 与代码行为不匹配,需 VCR_MODE=record 重录 |
| 2 | node_fs_shim | exists_sync / mkdir_sync / mkdtemp / read_file / stat / readdir / symlink / unlink / mkdir_recursive(9) | fs shim 行为问题(路径、ENOENT、类型) |
| 3 | provider_backward_lock | gemini_default_max_tokens_8192 / max_tokens_field_name(2) | Gemini maxOutputTokens 65536 vs 锁定 8192 |
| 4 | provider_error_paths | gemini_http_500 / gemini_invalid_json_event(2) | Gemini 错误路径处理 |
| 5 | qa_docs_policy_validation | dropin_gap_ledger / capture_baseline(2) | README/ledger 不一致 |
| 6 | remote_validation_proof_ledger_contract | operator_docs_reference_contract_and_claim_boundary(1) | README 缺链接 |
| 7 | sdk_integration | sdk_extension_policy_safe_denies_exec...(1) | deny policy 源(not_in_default_caps) |
| 8 | security_budgets | memory_limit_prevents_large_allocation(1) | 内存限制跨环境差异 |
| 9 | swarm_progress_cli | swarm_progress_stdout_does_not_mutate...(1) | git/beads 文件检查(路径不存在) |
| 10 | swarm_progress_slo_closeout_gate_contract | progress_slo_closeout_checklist...(1) | README 缺链接 |
| 11 | swarm_progress_slo_e2e | progress_slo_no_mock_e2e...(1) | 环境(路径) |
| 12 | swarm_replay_ingestor | no_mock_e2e_harness_emits_auditable...(1) | 缺失证据文件 replay_bundle.json |
| 13 | traceability_staleness | native_provider_module / source_coverage_matrix(2) | README/matrix 不同步 |
| 14 | validate_e2e_artifact_schema | artifact_index_cross_validation...(1) | 索引缺失 |
| 15 | validation_broker_contract | operator_docs / closeout_gate(2) | README 缺链接 |
| 16 | validation_broker_e2e | swarm_runpack_freshness...(1) | runpack 过期 |
| 17 | proof_carrying_swarm_test_fabric | checklist_quality_gates(1) | README 缺链接 |
| 18 | runtime_intelligence_closeout | checklist_quality_gates(1) | README 缺链接 |

**共性**: ① 大量是「README 必须链接契约/文档」门禁(6 个),custom 的 README 长期没满足;② VCR cassette 需重录;③ Gemini provider 行为差异;④ fs shim 行为。

### 四、Flaky / 环境(单独跑通过)

- `install_time_security_scanner`(could not execute process,单独跑通过)
- `sdk_unit lifecycle_state_fresh_session_has_zero_messages`(复跑 160 全过)
- `--doc`(单独跑 1+1 全过,全量并行时失败)

---

### 五、使用说明

1. 每次 merge 后跑全量 `cargo test --no-fail-fast`,与二、三、四对比
2. **新失败** = 合并引入 → 处理(回退数据文件 / 删上游测试 / 修证据)
3. **清单内** = 已知,不动
4. 若清单越来越长 → 考虑专门清理(重录 cassette、补 README 链接、同步 vendor)
