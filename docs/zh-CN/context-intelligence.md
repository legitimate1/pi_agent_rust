# 上下文智能（Context Intelligence）

上下文智能是 Pi 的顾问式语义工作区图谱与打包（bundle）规划器。它为代码、测试、文档、证据、Beads、提供方（provider）表面与验证命令建立索引，使智能体（agent）轮次能够获得聚焦的导航上下文。它不替代 Beads 的工作状态管理、Agent Mail 的协作、Doctor 或 runpack 的运维姿态、RCH 的验证、README 新鲜度检查或 drop-in 认证闸门。

## 配置

已发布的面向用户的入口为 `pi context-preview`。该命令为只读：不执行任何提供方调用、不写入、不变更 Beads、不进行 Agent Mail 预留。

```bash
pi context-preview \
  --format text \
  --bead bd-ircr3.11 \
  --changed-path scripts/build_swarm_operator_runpack.py \
  --failing-command "rch exec -- cargo test --test semantic_workspace_graph_builder context" \
  --max-items 24 \
  --max-bytes 32768 \
  context intelligence closeout docs gate
```

相关的打包控制参数为：

- `--format text|json`：text 面向运维人员，JSON 面向机器证据。
- `--bead`：用于 Beads 及相邻制品评分的任务锚点。
- `--changed-path`：处于活跃编辑状态文件的可重复路径锚点。
- `--failing-command`：用于测试与命令节点的验证命令锚点。
- `--max-items` 与 `--max-bytes`：硬性打包上限。
- 末尾查询文本：用于相关性评分的自由形式任务意图。

AgentSession 的提示词注入通过 Rust 代码内的 `SemanticContextBundleInjection` 按需启用。CLI 预览不会自动将打包附加到正在进行的提供方轮次。

## 预览工作流

1. 使用至少一个信号运行 `pi context-preview --format text ...`：查询文本、`--bead`、`--changed-path` 或 `--failing-command`。
2. 检查已选条目、已排除条目、陈旧证据抑制情况、建议的验证命令、脱敏状态与缓存 TTL。
3. 仅将已选路径作为导航提示使用，编辑前直接阅读权威源文件。
4. 当建议的验证命令涉及 Rust 编译时，使用 RCH 执行。
5. 需要证据时捕获 JSON：

```bash
pi context-preview --format json \
  --bead bd-ircr3.11 \
  --changed-path docs/context-intelligence.md \
  --changed-path scripts/build_swarm_operator_runpack.py \
  context intelligence operator docs final gate
```

## 失效模式

上下文智能采用失效关闭（fail closed）策略。缺失、不可读、格式错误、陈旧、历史性、未认证或新鲜度未知的证据会被分类处理，而不会被静默提升为当前上下文。已关闭和已墓碑化的 Beads 可作为引用节点，但永远不会成为可执行的任务。

常见的降级原因：

- `semantic_graph_missing_inputs`：预期的源路径缺失。
- `semantic_graph_malformed_inputs`：JSON 或结构化证据无法解析。
- `context_bundle_empty`：没有候选项通过评分与策略过滤。
- `context_bundle_partial_coverage`：已选打包缺少足够的关联测试或验证命令。
- `stale_or_unsafe_evidence_suppressed`：陈旧或不安全的证据被省略。
- `selected_code_without_test_link`：选中了代码上下文但缺少关联的测试信号。
- `context_cache_pressure`：在当前工作区、分支、会话或 TTL 下缓存指纹不可复用。

## 隐私姿态

图谱仅存储元数据与脱敏摘要，不存储原始提示词或提供方载荷。敏感密钥与路径类别会被抑制或脱敏，包括 API 密钥、令牌、Cookie、鉴权头、原始用户提示词、原始模型响应、Agent Mail 注册令牌、VCR HTTP 正文与会话转录。

提示词渲染器会重复声明陈旧、未认证或不安全的证据不是当前的发布证据。若脱敏状态达到 `unsafe_to_emit`，则该打包会排除对应条目，而不会将其暴露给提供方。

## 示例

排查失败的上下文规划器测试：

```bash
pi context-preview --format text \
  --failing-command "rch exec -- cargo test --test semantic_workspace_graph_builder context" \
  --changed-path src/semantic_workspace_graph.rs \
  semantic bundle planner deterministic replay
```

为本项目准备收口审查：

```bash
pi context-preview --format json \
  --bead bd-ircr3.11 \
  --changed-path docs/context-intelligence.md \
  --changed-path docs/contracts/context-intelligence-closeout-gate-contract.json \
  --changed-path scripts/build_swarm_operator_runpack.py \
  context intelligence closeout final gate child artifact map
```

通过 Doctor 检查集群姿态：

```bash
pi doctor --only swarm --format json
```

当可从当前工作区构建图谱时，Doctor 输出包含 `pi.doctor.context_intelligence_posture.v1`。集群 runpack 在 `doctor_swarm.context_intelligence` 下投射相同的姿态。

## 故障排查

- 若预览提示未提供上下文信号，请添加查询文本、`--bead`、`--changed-path` 或 `--failing-command`。
- 若相关代码缺失，请从仓库根目录重新运行并包含更具体的变更路径。
- 若证据因陈旧或未认证被抑制，请刷新源证据或仅将其视为历史上下文。
- 若制品因脱敏被抑制，请在本地检查源文件，在制品可安全摘要之前避免将其用于面向提供方的打包。
- 若出现缓存压力，请在分支、工作区或会话身份稳定后重建预览。
- 若 Agent Mail 降级，仍以 Beads 作为工作状态的真实来源；上下文智能不会推断预留。

## 收口闸门

最终项目收口闸门发出 `pi.context_intelligence.closeout_gate.v1`，受 `docs/contracts/context-intelligence-closeout-gate-contract.json` 治理。该闸门将从 `bd-ircr3.1` 到 `bd-ircr3.10` 的每个子 Bead 映射到代码路径、测试、文档或证据路径、验证命令、关闭原因与提交哈希。它还会检查运维文档、README 新鲜度、已暂存的 UBS、Bead 账本对账、聚焦的 RCH 测试、广义的 RCH Cargo 闸门以及已推送的 `origin/main` 与 `origin/master` 状态。

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --run-context-intelligence-final-gate \
  --out-context-intelligence-final-gate-json docs/evidence/context-intelligence-closeout-gate.json \
  --quality-gate-result "py_compile=pass:python3 -m py_compile scripts/build_swarm_operator_runpack.py" \
  --quality-gate-result "runpack_self_test=pass:python3 scripts/build_swarm_operator_runpack.py --self-test" \
  --quality-gate-result "json_contracts=pass:python3 -m json.tool docs/contracts/context-intelligence-closeout-gate-contract.json" \
  --quality-gate-result "semantic_context_graph_contract_rch=pass:rch exec -- cargo test --test semantic_context_graph_contract -- --nocapture" \
  --quality-gate-result "semantic_workspace_graph_contract_rch=pass:rch exec -- cargo test --test semantic_workspace_graph_contract -- --nocapture" \
  --quality-gate-result "semantic_workspace_graph_builder_rch=pass:rch exec -- cargo test --test semantic_workspace_graph_builder context" \
  --quality-gate-result "context_intelligence_e2e_rch=pass:rch exec -- cargo test --test e2e_agent_loop context_intelligence_no_mock_harness -- --nocapture" \
  --quality-gate-result "doctor_context_intelligence_rch=pass:rch exec -- cargo test --test doctor_swarm_temp_dir_json context_intelligence -- --nocapture" \
  --quality-gate-result "context_perf_budgets_rch=pass:rch exec -- cargo test --test perf_budgets context_intelligence" \
  --quality-gate-result "context_intelligence_closeout_gate_contract_rch=pass:rch exec -- cargo test --test context_intelligence_closeout_gate_contract -- --nocapture" \
  --quality-gate-result "cargo_fmt=pass:cargo fmt --check" \
  --quality-gate-result "cargo_check_all_targets_rch=pass:CARGO_TARGET_DIR=$CARGO_TARGET_DIR TMPDIR=$TMPDIR rch exec -- cargo check --all-targets" \
  --quality-gate-result "cargo_clippy_all_targets_rch=pass:CARGO_TARGET_DIR=$CARGO_TARGET_DIR TMPDIR=$TMPDIR rch exec -- cargo clippy --all-targets -- -D warnings" \
  --quality-gate-result "staged_ubs=pass:timeout 60s ubs --staged --only=rust ." \
  --quality-gate-result "beads_ledger_reconcile=pass:./scripts/reconcile_beads_ledger.sh"
```

失败的闸门会发出 `follow_up_beads` 并判定 `decision=file_follow_up_beads_before_closing_epic`。通过的闸门仅作为收口证据，不作为新的真实来源。
