# 测试覆盖率矩阵（当前源码清单） / Test Coverage Matrix (Current Source Inventory)

> 最后重新生成：2026-05-10
> 负责人 bead：`bd-8t27h.1`

本文档是 `src/**/*.rs` 的当前源码文件覆盖率清单。它不是 drop-in 认证制品，也不覆盖 `docs/evidence/dropin-certification-verdict.json`。

> 路径锚点保留英文：`docs/traceability_matrix.json`、`docs/TEST_COVERAGE_MATRIX.md`、`docs/coverage-baseline-map.json`

### 重新生成证据（Regeneration Evidence）

- `rg --files src -g '*.rs' | sort` -> 111 个当前源码文件。
- `rg --files tests -g '*.rs' | wc -l` -> 304 个 `tests/` 下的 Rust 测试文件。
- `rg -n '#\[cfg\(test\)|mod tests' src -g '*.rs'` -> 源码内单元测试清单，用于下文 `Unit` 状态。
- `python3 scripts/check_traceability_matrix.py` 通过，分类追溯覆盖率 100.00%，E2E 场景覆盖率 100.00%（经 `e2e_swarm_flight_recorder` 与 `rch_artifact_sync_preflight` 清单刷新后）。更广义的语义追溯扩展由 `bd-8t27h.3` 跟踪。
- `docs/coverage-baseline-map.json` 为 2026-02-14 的历史覆盖率证据，覆盖 107 个源码文件；本 Markdown 清单现已反映 111 文件的当前树。
- 漂移守护：`cargo test --test traceability_staleness source_coverage_matrix_matches_current_src_inventory`。

### 当前漂移检查（Current Drift Check）

- 当前 `src/` 清单：111 个文件。
- 下表源码文件行数：111。
- 本文档遗漏的源码文件：0。
- 拆分模块、提供方扩展模块、宿主调用调度/队列模块、PiWasm、会话 v2/SQLite、资源、资源管控器及调度器/准入表面均已显式呈现，并经由 `resource_scheduler_admission` 制品清单通道关联。
- 机器可读追溯仍由 `docs/traceability_matrix.json`、`tests/suite_classification.toml`、`docs/e2e_scenario_matrix.json` 与 `scripts/check_traceability_matrix.py` 治理。

### 图例（Legend）

- **Unit**：源码文件内存在 `#[cfg(test)]` 或 `mod tests`。
- **Integration/E2E/Conformance**：覆盖该表面的代表性测试文件或受治理制品。
- **Waived glue**：重导出或测试支撑模块；在此呈现以防静默消失。
- **Gap owner**：拥有该行已知薄弱点的 bead。

---

## 1) 源码文件覆盖率矩阵

| 源码文件（Source file） | 领域（Area） | 覆盖证据 / 状态（Coverage evidence / status） |
|---|---|---|
| `src/acp.rs` | ACP 协议 | 单元测试；`tests/sdk_api.rs`、`tests/sdk_integration.rs`、`tests/sdk_unit.rs`。 |
| `src/abort.rs` | 中止信号原语 | 单元测试。 |
| `src/agent.rs` | 智能体循环 | 单元测试；`tests/agent_loop_vcr.rs`、`tests/agent_loop_reliability.rs`、`tests/e2e_agent_loop.rs`、`tests/rpc_mode.rs`。 |
| `src/agent_cx.rs` | 智能体上下文 | 单元测试；经由智能体/RPC 套件覆盖。 |
| `src/app.rs` | 应用编排 | 单元测试；`tests/e2e_cli.rs`、`tests/e2e_rpc.rs`、`tests/main_cli_selection.rs`。 |
| `src/auth.rs` | 认证与 OAuth | 单元测试；`tests/auth_oauth_refresh_vcr.rs`、`tests/extensions_provider_oauth.rs`。 |
| `src/autocomplete.rs` | 提示自动补全 | 单元测试；交互式覆盖经由 `tests/tui_state.rs`。 |
| `src/bin/pi_legacy_capture.rs` | 旧版捕获工具 | 单元测试；可选捕获工具，非默认用户路径。 |
| `src/buffer_shim.rs` | Node buffer 垫片 | `tests/node_buffer_shim.rs`；分支导出基线标记为 branch-SIGSEGV 回退。 |
| `src/cli.rs` | CLI 解析 | 单元测试；`tests/main_cli_selection.rs`、`tests/cli_edge_cases.rs`、`tests/e2e_cli.rs`。 |
| `src/compaction.rs` | 会话压缩 | 单元测试；`tests/compaction.rs`、`tests/compaction_bug.rs`。 |
| `src/compaction_worker.rs` | 压缩工作线程 | 单元测试；由压缩套件覆盖。 |
| `src/config.rs` | 配置加载 | 单元测试；`tests/config_precedence.rs`、`tests/config_edge_cases.rs`。 |
| `src/conformance.rs` | 一致性运行器 | 单元测试；`tests/conformance_*.rs`、`tests/tools_conformance.rs`。 |
| `src/conformance_shapes.rs` | 一致性模式 | 单元测试；`tests/ext_conformance_shapes.rs`。 |
| `src/connectors/http.rs` | HTTP 连接器 | `tests/pi_connector_shims.rs`；连接器覆盖仍需在 `bd-8t27h.3` 下扩展机器可读追溯。 |
| `src/connectors/mod.rs` | 连接器注册表 | 单元测试；`tests/rpc_session_connector.rs`、`tests/pi_connector_shims.rs`。 |
| `src/crypto_shim.rs` | Node crypto 垫片 | 单元测试；`tests/node_crypto_shim.rs`。 |
| `src/doctor.rs` | 诊断与 doctor | 单元测试；`tests/doctor_swarm_temp_dir_json.rs`、`tests/franken_node_compatibility_doctor_contract.rs`。 |
| `src/error.rs` | 错误类型 | 单元测试；`tests/error_types.rs`、`tests/error_handling.rs`。 |
| `src/error_hints.rs` | 错误修复提示 | 单元测试；`tests/error_handling.rs`。 |
| `src/extension_conformance_matrix.rs` | 扩展矩阵 | 单元测试；`tests/ext_conformance_matrix.rs`。 |
| `src/extension_dispatcher.rs` | 扩展调度器 | 单元测试；`tests/event_dispatch_latency.rs`、`tests/extensions_event_wiring.rs`；计时忽略测试负责人 `bd-8t27h.11`。 |
| `src/extension_events.rs` | 扩展事件 | 单元测试；`tests/extensions_event_wiring.rs`、`tests/extensions_event_cancellation.rs`、`tests/extensions_repair_events.rs`。 |
| `src/extension_inclusion.rs` | 扩展包含清单 | 单元测试；`tests/ext_inclusion_list.rs`。 |
| `src/extension_index.rs` | 扩展索引/搜索 | 单元测试；`tests/ext_entry_scan.rs`、`tests/extension_code_search.rs`。 |
| `src/extension_license.rs` | 扩展许可证审计 | 单元测试；`tests/extension_license.rs`；报告证据位于 `docs/extension-license-report.json`。 |
| `src/extension_popularity.rs` | 扩展热度评分 | 单元测试；覆盖以制品/报告为主，需在 `bd-8t27h.3` 中追溯。 |
| `src/extension_preflight.rs` | 扩展 preflight | 单元测试；`tests/ext_preflight_analyzer.rs`、`tests/e2e_workflow_preflight.rs`。 |
| `src/extension_replay.rs` | 扩展回放 | 单元测试；`tests/e2e_replay_bundle_validation.rs`、`tests/e2e_replay_bundles.rs`。 |
| `src/extension_scoring.rs` | 扩展评分 | 单元测试；`tests/extension_scoring.rs`、`tests/extension_scoring_ope.rs`。 |
| `src/extension_tools.rs` | 扩展工具 | 单元测试；`tests/e2e_extension_registration.rs`；分支导出基线标记为回退。 |
| `src/extension_validation.rs` | 扩展校验 | 单元测试；`tests/extension_validation.rs`、`tests/extension_lockfile_provenance.rs`。 |
| `src/extensions.rs` | 扩展协议/运行时 | 单元测试；`tests/extensions_*.rs`、`tests/ext_conformance*.rs`、`tests/e2e_extension_registration.rs`。 |
| `src/extensions_js.rs` | QuickJS 桥接 | 单元测试；`tests/event_loop_conformance.rs`、`tests/js_runtime_ordering.rs`、`tests/e2e_ts_extension_loading.rs`。 |
| `src/flake_classifier.rs` | 不稳定用例分类器 | 单元测试；模式由 `scripts/ci_conformance_retry.sh` 镜像。 |
| `src/file_lock.rs` | DirLock 目录协议锁 | 单元测试。 |
| `src/hostcall_amac.rs` | 宿主调用 AMAC | 单元测试；`tests/streaming_hostcall.rs`。 |
| `src/hostcall_io_uring_lane.rs` | 宿主调用 io_uring 通道 | 单元测试；`tests/streaming_hostcall.rs`。 |
| `src/hostcall_queue.rs` | 宿主调用队列 | 单元测试；`tests/hostcall_queue_ebr.rs`、`tests/hostcall_queue_loom.rs`；loom 可选负责人 `bd-8t27h.6`。 |
| `src/hostcall_rewrite.rs` | 宿主调用重写 | 单元测试；`tests/streaming_hostcall.rs`。 |
| `src/hostcall_s3_fifo.rs` | 宿主调用 S3 FIFO | 单元测试；`tests/hostcall_s3_fifo_policy.rs`。 |
| `src/hostcall_superinstructions.rs` | 宿主调用超级指令 | 单元测试；`tests/streaming_hostcall.rs`。 |
| `src/hostcall_trace_jit.rs` | 宿主调用 trace JIT | 单元测试；`tests/streaming_hostcall.rs`。 |
| `src/http/client.rs` | HTTP 客户端 | 单元测试；`tests/http_client.rs`；分支导出基线标记 `src/http/*.rs` 为回退。 |
| `src/http/mod.rs` | HTTP 模块胶水 | 豁免胶水：重导出/测试模块装配。 |
| `src/http/sse.rs` | HTTP SSE | 单元测试；`tests/repro_sse_flush.rs`。 |
| `src/http/test_api.rs` | HTTP 测试支撑 | 豁免测试支撑模块；仅测试编译。 |
| `src/http/test_asupersync.rs` | HTTP 测试支撑 | 豁免测试支撑模块；仅测试编译。 |
| `src/http_shim.rs` | Node HTTP 垫片 | `tests/node_http_shim.rs`；分支导出基线标记为回退。 |
| `src/interactive.rs` | TUI 根 | 单元测试模块装配；`tests/tui_snapshot.rs`、`tests/tui_state.rs`、`tests/e2e_tui.rs`。 |
| `src/interactive/agent.rs` | TUI 智能体通道 | 单元测试；`tests/e2e_tui.rs`、`tests/tui_state.rs`。 |
| `src/interactive/commands.rs` | 交互式命令 | 单元测试；`tests/interactive_commands_unit.rs`、`tests/interactive_extension_ui.rs`。 |
| `src/interactive/conversation.rs` | 对话模型 | 单元测试；`tests/tui_state.rs`。 |
| `src/interactive/ext_session.rs` | 扩展会话 UI | 单元测试；`tests/interactive_extension_ui.rs`；分支导出基线标记为回退。 |
| `src/interactive/file_refs.rs` | 文件引用 | 单元测试；`tests/tui_state.rs`。 |
| `src/interactive/keybindings.rs` | 交互式按键绑定 | 单元测试；`tests/tui_state.rs`。 |
| `src/interactive/model_selector_ui.rs` | 模型选择器 UI | 单元测试；`tests/model_selector_cycling.rs`、`tests/tui_state.rs`。 |
| `src/interactive/perf.rs` | TUI 性能遥测 | 单元测试；`tests/e2e_tui_perf.rs`、`tests/perf_regression.rs`。 |
| `src/interactive/share.rs` | 分享/导出 UI | 单元测试；经由交互式状态与命令测试覆盖。 |
| `src/interactive/state.rs` | 交互式状态 | 单元测试；`tests/tui_state.rs`。 |
| `src/interactive/tests.rs` | 交互式测试模块 | 豁免测试模块，由 `src/interactive.rs` 包含。 |
| `src/interactive/text_utils.rs` | 文本工具 | 经由交互式状态/视图测试覆盖；若增长应补充直接单元行。 |
| `src/interactive/tool_render.rs` | 工具渲染 | 单元测试；`tests/tui_snapshot.rs`、`tests/tui_state.rs`。 |
| `src/interactive/tree.rs` | 对话树 | 经由 `tests/tui_state.rs` 与会话/导航测试覆盖；直接追溯应在 `bd-8t27h.3` 扩展。 |
| `src/interactive/tree_ui.rs` | 树 UI | 经由 `tests/tui_snapshot.rs` 与 `tests/tui_state.rs` 覆盖。 |
| `src/interactive/view.rs` | 视图渲染 | 单元测试；`tests/tui_snapshot.rs`、`tests/e2e_tui.rs`。 |
| `src/keybindings.rs` | 按键绑定配置 | 单元测试；交互式/TUI 测试。 |
| `src/lib.rs` | Crate 导出 | 豁免胶水：导出模块表面由全部目标编译；无独立行为行。 |
| `src/main.rs` | CLI 入口 | 单元测试；`tests/e2e_cli.rs`、`tests/e2e_rpc.rs`、`tests/main_cli_selection.rs`；分支导出基线标记为回退。 |
| `src/migrations.rs` | 迁移 | 单元测试；SQLite/会话迁移覆盖经由 `tests/session_sqlite.rs`。 |
| `src/model.rs` | 消息/内容模型 | 单元测试；`tests/model_serialization.rs`。 |
| `src/model_routing.rs` | 模型路由辅助 | 单元测试。 |
| `src/model_selector.rs` | 模型选择器 | 单元测试；`tests/model_selector_cycling.rs`。 |
| `src/models.rs` | 模型注册表 | 单元测试；`tests/model_registry.rs`。 |
| `src/package_manager.rs` | 包管理器 | 单元测试；`tests/package_manager.rs`、`tests/e2e_cli.rs`。 |
| `src/perf_build.rs` | 性能构建元数据 | 单元测试；`tests/perf_bench_harness.rs`、`tests/perf_budgets.rs`。 |
| `src/semantic_workspace_graph.rs` | 语义工作区图 | 单元测试；`tests/semantic_workspace_graph_builder.rs`。 |
| `src/subprocess_handle.rs` | 子进程句柄 | 单元测试。 |
| `src/swarm_progress_slo.rs` | 集群进度 SLO | 单元测试；`tests/swarm_progress_cli.rs`。 |
| `src/swarm_replay.rs` | 集群回放摄取/预览 | 单元测试；`tests/swarm_replay_ingestor.rs`、`tests/swarm_replay_preview_cli.rs`。 |
| `src/permissions.rs` | 能力权限 | 单元测试；`tests/capability_policy_model.rs`、`tests/capability_policy_scoped.rs`。 |
| `src/pi_wasm.rs` | PiWasm 运行时 | 单元测试；`tests/lab_runtime_extensions.rs`；不支持的导入 fail-closed。 |
| `src/platform.rs` | 平台辅助 | 单元测试。 |
| `src/provider.rs` | 提供方 trait/模式 | 单元测试；`tests/provider_factory.rs`、`tests/provider_contract.rs`。 |
| `src/provider_metadata.rs` | 提供方元数据 | 单元测试；`tests/provider_metadata_comprehensive.rs`、`tests/provider_registry_guardrails.rs`。 |
| `src/providers/anthropic.rs` | Anthropic 提供方 | 单元测试；`tests/provider_streaming/anthropic.rs`、`tests/e2e_provider_streaming.rs`。 |
| `src/providers/azure.rs` | Azure 提供方 | 单元测试；`tests/provider_streaming/azure.rs`。 |
| `src/providers/bedrock.rs` | Bedrock 提供方 | 单元测试；提供方原生/契约套件。 |
| `src/providers/cohere.rs` | Cohere 提供方 | 单元测试；`tests/provider_streaming/cohere.rs`。 |
| `src/providers/copilot.rs` | Copilot 提供方 | 单元测试；提供方原生/契约套件。 |
| `src/providers/cursor.rs` | Cursor 提供方 | 单元测试；提供方原生/契约套件。 |
| `src/providers/gemini.rs` | Gemini 提供方 | 单元测试；`tests/provider_streaming/gemini.rs`。 |
| `src/providers/gitlab.rs` | GitLab Duo 提供方 | 单元测试；提供方原生/契约套件。 |
| `src/providers/model_fetch.rs` | 动态模型目录拉取 | 单元测试。 |
| `src/providers/mod.rs` | 提供方工厂 | 单元测试；`tests/provider_factory.rs`、`tests/provider_native_verify.rs`；分支导出基线部分回退。 |
| `src/providers/openai.rs` | OpenAI chat 提供方 | 单元测试；`tests/provider_streaming/openai.rs`。 |
| `src/providers/openai_responses.rs` | OpenAI Responses 提供方 | 单元测试；`tests/provider_streaming/openai_responses.rs`。 |
| `src/providers/vertex.rs` | Vertex 提供方 | 单元测试；提供方原生/契约套件。 |
| `src/resource_governor.rs` | 资源管控器 | 单元测试；`tests/cargo_headroom_admission.rs`、`tests/resource_edge_cases.rs`；追溯通道 `resource_scheduler_admission`。 |
| `src/resources.rs` | 资源加载 | 单元测试；`tests/resource_loader.rs`、`tests/resource_edge_cases.rs`。 |
| `src/rpc.rs` | RPC/stdin 模式 | 单元测试；`tests/rpc_mode.rs`、`tests/rpc_protocol.rs`、`tests/e2e_rpc.rs`。 |
| `src/scheduler.rs` | 调度器/准入 | 单元测试；`tests/scheduler_repro.rs`、`tests/cargo_headroom_admission.rs`。 |
| `src/sdk.rs` | SDK API | 单元测试；`tests/sdk_api.rs`、`tests/sdk_integration.rs`。 |
| `src/session.rs` | 会话 JSONL/树 | 单元测试；`tests/session_conformance.rs`、`tests/e2e_session_persistence.rs`。 |
| `src/session_index.rs` | 会话索引 | 单元测试；`tests/session_index_tests.rs`。 |
| `src/session_metrics.rs` | 会话指标 | 单元测试；`tests/provider_session_coverage.rs` 及会话证据套件。 |
| `src/session_picker.rs` | 会话选择器 UI | 单元测试；`tests/session_picker.rs`。 |
| `src/session_sqlite.rs` | SQLite 会话后端 | 单元测试；`tests/session_sqlite.rs`、`tests/fault_injection_persistence.rs`。 |
| `src/session_store_v2.rs` | 会话存储 v2 | 单元测试；`tests/session_store_v2.rs`、`tests/session_store_v2_contract.rs`。 |
| `src/session_test.rs` | 会话测试辅助 | 豁免测试支撑模块；由会话测试编译。 |
| `src/sse.rs` | SSE 解析器 | 单元测试；`tests/sse_strict_compliance.rs`、`tests/repro_sse_flush.rs`。 |
| `src/swarm_activity_ledger.rs` | 集群活动账本 | 单元测试；证据文档位于 `docs/swarm-activity-ledger.md`。 |
| `src/swarm_flight_recorder.rs` | 集群飞行记录器 | 单元与 E2E；`tests/e2e_swarm_flight_recorder.rs` 覆盖确定性多智能体回放制品。 |
| `src/terminal_images.rs` | 终端图像 | 单元测试；交互式/TUI 渲染测试。 |
| `src/theme.rs` | 主题加载 | 单元测试；`tests/tui_snapshot.rs`、交互式 UI 测试。 |
| `src/tools/bash.rs` | Bash 工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/edit.rs` | Edit 工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/find.rs` | Find 工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/grep.rs` | Grep 工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/hashline.rs` | Hashline 编辑工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/ls.rs` | LS 工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/mod.rs` | 工具注册表与辅助 | 单元测试；`tests/tools_conformance.rs`、`tests/e2e_tools.rs`。 |
| `src/tools/pwsh.rs` | PowerShell 工具 | 单元测试；`tests/tools_conformance.rs`。 |
| `src/tools/read.rs` | Read 工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/tests.rs` | 工具测试工具 | 豁免测试支撑模块。 |
| `src/tools/verify.rs` | 语法/格式校验 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/tools/write.rs` | Write 工具 | 单元测试；`tests/tools_conformance.rs`、`tests/tools_hardened.rs`。 |
| `src/validation_broker.rs` | 校验经纪人 | 单元测试；`tests/validation_broker_cli.rs`、`tests/validation_broker_e2e.rs`。 |
| `src/tui.rs` | 终端渲染器 | 单元测试；`tests/tui_snapshot.rs`、`tests/tui_state.rs`、`tests/e2e_tui.rs`。 |
| `src/vcr.rs` | VCR 回放/录制 | 单元测试；`tests/vcr_parity_validation.rs`、`tests/vcr_redaction_scan.rs`。 |
| `src/version_check.rs` | 版本检查 | 单元测试；跨平台与发布就绪测试覆盖周边行为。 |

---

## 2) 测试套件清单指针

完整 Rust 测试清单过大，本 Markdown 表不再作为事实来源。2026-05-10 当前计数：

| 清单 | 数量 | 事实来源 |
|---|---:|---|
| 源码文件 | 111 | `rg --files src -g '*.rs' \| sort` |
| Rust 测试文件 | 304 | `rg --files tests -g '*.rs' \| sort` |
| 已分类顶层测试文件 | 280 | `tests/suite_classification.toml` |
| 追溯矩阵引用的已分类测试 | 280 | `docs/traceability_matrix.json` 经由 `scripts/check_traceability_matrix.py` |
| 已分类但未追溯的测试 | 0 | `scripts/check_traceability_matrix.py` |

代表性高信号套件：

| 套件族 | 代表性文件 | 主要表面 |
|---|---|---|
| Agent/RPC/CLI | `tests/agent_loop_vcr.rs`、`tests/e2e_agent_loop.rs`、`tests/rpc_mode.rs`、`tests/e2e_cli.rs` | `agent`、`app`、`main`、`rpc`、CLI 选择 |
| 提供方 | `tests/provider_streaming/*.rs`、`tests/provider_factory.rs`、`tests/provider_metadata_comprehensive.rs` | 原生提供方模块、元数据、工厂路由 |
| 扩展 | `tests/ext_conformance*.rs`、`tests/extensions_*.rs`、`tests/e2e_extension_registration.rs` | 扩展协议、QuickJS 桥接、策略、一致性 |
| TUI | `tests/tui_snapshot.rs`、`tests/tui_state.rs`、`tests/e2e_tui.rs` | 交互式状态、视图、渲染、按键绑定 |
| 会话 | `tests/session_conformance.rs`、`tests/session_index_tests.rs`、`tests/session_sqlite.rs` | JSONL/树/索引/sqlite/store v2 持久化 |
| 工具 | `tests/tools_conformance.rs`、`tests/e2e_tools.rs` | 内置工具与工具 I/O 契约 |
| 资源/调度器 | `tests/resource_loader.rs`、`tests/scheduler_repro.rs` | 资源、资源管控器、调度器/准入 |
| 垫片 | `tests/node_buffer_shim.rs`、`tests/node_crypto_shim.rs` | Node 兼容垫片 |

---

## 3) Mock / Fake / Stub 审计

本矩阵不认证无模拟合规。请使用专用守护与文档：

- `tests/non_mock_compliance_gate.rs`
- `tests/non_mock_rubric_gate.rs`
- `.github/workflows/ci.yml` 无模拟守护步骤
- `docs/non-mock-rubric.json`

已知允许清单测试替身仍局限于测试 harness，未替代发布路径：`tests/common/harness.rs` 本地 TCP `MockHttpServer`、CLI E2E 测试中的包命令桩、以及扩展消息会话测试的录制宿主/会话 harness。

---

## 4) JSONL / 制品覆盖

结构化证据位置在本文档外治理：

- `docs/traceability_matrix.json`
- `docs/e2e_scenario_matrix.json`
- `tests/e2e_results/*`
- `tests/ext_conformance/reports/*`
- `tests/perf/reports/*`

活跃 JSONL 清单缺口为 `bd-8t27h.9`。

---

## 5) 本次刷新的活跃后续工作

| Bead | 范围 |
|---|---|
| `bd-8t27h.2` | 增加确定性源码覆盖率漂移守护，使该 110 文件清单不会静默陈旧。 |
| `bd-8t27h.3` | 修复追溯矩阵、套件分类、证据日志与 E2E 场景覆盖。 |
| `bd-8t27h.5` | 替换 macOS 被忽略的扩展 OAuth MockHttpServer 测试。 |
| `bd-8t27h.6` | 使宿主调用队列 loom 测试在显式可选 cfg/profile 后可运行。 |
| `bd-8t27h.9` | 扩展高价值套件的 JSONL 制品清单。 |
| `bd-8t27h.10` | 确定化 E2E 黄金语料动态磁带路径。 |
| `bd-8t27h.11` | 将扩展调度器计时回归从墙钟不稳定中移出。 |
| `bd-8t27h.12` | 将手工 perf/报告生成器归一为 tmpdir 感知的冒烟测试。 |
| `bd-8t27h.13` | 记录并测试 PiWasm 不支持导入的 fail-closed 策略。 |
| `bd-8t27h.16` | 将扩展随机试验收敛至确定性冒烟通道。 |
| `bd-8t27h.17` | 接入未供应商化的扩展一致性语料。 |
| `bd-8t27h.18` | 归一化 npm 仓库一致性差异忽略。 |

---

## 6) 运行扩展一致性测试

```bash
# 生成的按扩展注册测试，默认 tier 1-2。
cargo test --test ext_conformance_generated --features ext-conformance

# 负责人跟踪的可选通道，用于生成 tier 3-5 扩展测试。
cargo test --test ext_conformance_generated --features ext-conformance -- --include-ignored

# 差异化 TypeScript/Rust 预言。
cargo test --test ext_conformance_diff --features ext-conformance

# 仅官方扩展，有界。
PI_OFFICIAL_MAX=5 cargo test --test ext_conformance_diff --features ext-conformance

# 场景执行测试。
cargo test --test ext_conformance_scenarios --features ext-conformance

# 默认一致性相关测试。
cargo test conformance
cargo test extensions_policy_negative
```

---

## 7) 覆盖率工具

覆盖率报告使用 `cargo-llvm-cov` 生成，见 `README.md`。

当前历史基线证据位于 `docs/coverage-baseline-map.json`：

| 指标 | 值 |
|---|---:|
| 生成于 | 2026-02-14T14:00:00Z |
| 基线中的源码文件 | 107 |
| 当前源码文件 | 110 |
| 行覆盖率 | 79.08% |
| 分支覆盖率 | 51.95% 下界 |
| 函数覆盖率 | 78.01% |
| 可分支度量文件 | 63 |
| 分支导出 SIGSEGV 回退文件 | 44 |

该基线为有用证据，但非当前 HEAD 的全量清单。本文档现提供当前 110 文件清单；`bd-8t27h.2` 负责将其转为强制漂移守护。
