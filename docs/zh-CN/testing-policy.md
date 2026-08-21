# 测试策略：套件分类与执行 / Testing Policy: Suite Classification and Enforcement

本文档定义 Pi 的测试套件边界、分类标准与执行规则。

## 对等测试 + 日志契约（Parity Test + Logging Contract，DROPIN-171）

本策略文档是 `pi.parity.test_logging_contract.v1` 的规范归属（normative home）。

该契约将三者绑定在一起：
- 测试套件分类（`unit`、`vcr`、`e2e`）
- 这些套件使用的结构化日志/事件制品
- 确定性回放/调试所需的失败分流元数据

### 已契约模式（Contracted Schemas）

| 领域（Domain） | 模式 ID（Schema ID） | 来源（Source） |
|--------|-----------|--------|
| 测试日志 JSONL 记录 | `pi.test.log.v2` | `tests/common/logging.rs` |
| 制品索引 JSONL 记录 | `pi.test.artifact.v1` | `tests/common/logging.rs` |
| 证据契约包 | `pi.qa.evidence_contract.v1` | `docs/evidence-contract-schema.json` |
| 按套件失败摘要 | `pi.e2e.failure_digest.v1` | `docs/evidence-contract-schema.json` |
| 回放包 | `pi.e2e.replay_bundle.v1` | `tests/e2e_replay_bundles.rs` + 运行制品 |
| 扩展整改待办 | `pi.qa.extension_remediation_backlog.v1` | `tests/qa_certification_dossier.rs` + `tests/full_suite_gate/extension_remediation_backlog.json` |
| 用户感知 SLI + UX 矩阵 | `pi.perf.sli_ux_matrix.v1` | `docs/perf_sli_matrix.json` |

> 路径锚点保留英文：`docs/evidence-contract-schema.json`、`tests/common/logging.rs`、`docs/perf_sli_matrix.json`、`tests/full_suite_gate/extension_remediation_backlog.json`

### 关联模型（Correlation Model）

| 字段（Field） | 范围（Scope） | 要求（Requirement） |
|-------|-------|-------------|
| `correlation_id` | 运行级聚合制品 | 证据/回放摘要中必需 |
| `trace_id` | 按套件/按测试日志流 | `pi.test.log.v2` 记录中必需 |
| `span_id` | 嵌套操作追踪 | 可选，但存在时必须为字符串 |
| `parent_span_id` | Span 层级 | 可选，但存在时必须为字符串 |
| `ci_correlation_id` | 跨分片 CI 关联 | 可选，但存在时必须为字符串 |

### 失败分流元数据要求（Failure Triage Metadata Requirements）

对于证据制品中每个失败套件条目：
- `root_cause_class` 必须为已声明分类中的值
- `first_failing_assertion` 必须记录为非空字符串
- `remediation_pointer.replay_command` 应当产出
- `suite_replay_command` 与 `targeted_test_replay_command` 在可用时应当产出

### 套件绑定要求（Suite Binding Requirements）

| 套件（Suite） | 最低契约绑定（Minimum Contract Binding） |
|-------|--------------------------|
| `unit` | 使用测试 harness 日志时必须保持模式合法的 JSONL 日志 |
| `vcr` | 必须保持确定性回放 + 模式合法的日志/制品记录 |
| `e2e` | 必须产出满足上述模式集的证据/回放/失败摘要制品，且每个工作流必须经由 `docs/perf_sli_matrix.json` 映射到一个或多个面向用户的 SLI |
| `certification` | 重新生成认证时必须同步产出认证案卷制品与扩展整改待办制品 |

### 模式演进策略（Schema Evolution Policy）

`pi.parity.test_logging_contract.v1` 采用增量、版本化、严格 fail-closed 的演进：

- `pi.test.log.v2` 为新测试输出的当前必需日志模式。
- `pi.test.log.v1` 仅对历史/回填校验可读，被 `validate_jsonl_v2_only` 拒绝。
- `pi.test.artifact.v1` 在后继者被显式批准前保持为规范制品索引模式。
- 新模式版本必须附带：
  - `tests/common/logging.rs` 中的校验器更新
  - 覆盖新旧接受与拒绝边界的回归测试
  - 本文档与 `docs/qa-runbook.md` 中的运行手册/策略更新
- 跨运行对比工具必须使用稳定字段投影（schema/type/level/category/message/context）与场景/组件过滤，以避免来自计时/关联字段的误报差异。

## 套件（Suites）

全部测试恰属三大套件之一：

### 套件 1：单元测试（Unit，无模拟、无夹具）

**测试内容：** 纯逻辑、数据转换、解析、序列化、状态机。

**规则：**
- 无 VCR 磁带、无夹具文件、无 HTTP 服务器（真实或模拟）。
- 无 `MockHttp*`、`RecordingSession`、`RecordingHostActions`、`DummyProvider`，或任何以 `Mock`、`Fake`、`Stub` 开头的结构体（由 CI 强制）。
- 经由 `tempfile` 的临时文件系统允许（真实 I/O，非模拟）。
- 自定义仅测试类型（如 `DeterministicClock`、`SharedBufferWriter`）在以受控输入演练真实逻辑而非替换依赖时允许。
- `NullSession` 与 `NullUiHandler` 在本套件中**不允许**（它们是抑制真实行为的空操作桩）。

**如何运行：**
```bash
cargo test --all-targets --lib          # 仅内联 #[cfg(test)] 模块
cargo test --all-targets --test model_serialization --test config_precedence \
  --test session_conformance --test error_types     # 精选集成子集

# 针对宿主调用队列并发不变量的可选 loom 模型检查。
rch exec -- cargo test --features loom-tests --test hostcall_queue_loom
rch exec -- cargo test --features loom-tests --lib hostcall_queue::tests::loom_
```

**识别该套件中的测试：** 测试位于 `src/*.rs` 内的 `#[cfg(test)]` 模块，或 `tests/suite_classification.toml` 的 `[suite.unit]` 节中列出的 `tests/` 文件。

### 套件 2：VCR / 夹具回放（VCR / Fixture Replay）

**测试内容：** 提供方流式、HTTP 客户端行为、协议一致性、基于录制或预构建数据的扩展注册。

**规则：**
- VCR 磁带（`VcrRecorder`、`VcrMode::Playback`）为主要数据源。
- 允许 JSON 夹具文件（一致性比较器、扩展日志）。
- `MockHttpServer` 仅在 VCR 无法表达测试数据时允许（如原始非法 UTF-8 字节注入）。每次使用必须在下文允许清单中记录。
- `RecordingSession` 与 `RecordingHostActions` 在无需完整会话的会话/扩展 API 表面测试中允许。
- 测试必须确定性：相同磁带/夹具，相同结果。不稳定测试即缺陷。

**如何运行：**
```bash
cargo test --all-targets                          # 默认：含 VCR 支撑测试
cargo test --features ext-conformance             # + 扩展一致性
VCR_MODE=playback cargo test --all-targets        # 强制回放（CI 默认）
```

**识别测试：** `tests/suite_classification.toml` 的 `[suite.vcr]` 中列出的文件，或任何从 `pi::vcr` 导入 / 引用 `cassette_root()` / 加载 JSON 夹具的测试文件。

### 套件 3：实时端到端（Live E2E）

**测试内容：** 含真实提供方、真实网络、真实终端（tmux）的全系统行为。

**规则：**
- 需要实时 API 密钥、网络访问和/或 tmux。
- 测试必须对可用性设门：若提供方/工具缺失则优雅跳过。
- 必须产出 JSONL 日志与制品索引（按 bd-4u9）。
- 成本预算：每次测试运行必须保持在可配置的 token/美元限额内。

**如何运行：**
```bash
# 使用实时提供方（需要 API 密钥）
PI_E2E=1 cargo test --test e2e_cli --test e2e_tui --test e2e_tools

# 基于 VCR 的 E2E（确定性，无需 API 密钥）
VCR_MODE=playback cargo test --test e2e_provider_streaming --test agent_loop_vcr
```

**识别测试：** `tests/suite_classification.toml` 的 `[suite.e2e]` 中列出的文件，或任何以 `e2e_` 为前缀的测试文件。

本套件的规范场景覆盖映射位于：
- `docs/e2e_scenario_matrix.json`（模式 `pi.e2e.scenario_matrix.v2`）
- `docs/perf_sli_matrix.json`（模式 `pi.perf.sli_ux_matrix.v1`）
- 漂移与模式强制：`python3 scripts/check_traceability_matrix.py`

PERF-3X 阶段验证与诊断流程必须消费以 `scenario_id` + `sli_id` 为键的 SLI 输出；仅微基准摘要不足。

### 实时提供方凭证 + 回放策略（bd-1f42.2.7）

本策略适用于 `tests/e2e_live_harness.rs` 及 `tests/common/harness.rs` + `tests/common/logging.rs` 中的共享辅助。

#### 凭证处理与脱敏（Credential handling and redaction）

- API 密钥来源优先级严格：环境（`*_API_KEY`） -> 认证存储 -> `models.json`。
- 凭证**值**绝不得写入日志、JSONL 制品或契约记录。
- 实时 harness 制品仅包含 `credential_source` 元数据（如 `env:OPENAI_API_KEY`）。
- 敏感请求头值在写入运行记录前强制脱敏为 `[REDACTED]`。
- 每个产出的 JSONL 制品（`log`、`artifact index`、原始结果、契约结果、成本契约）必须通过未脱敏密钥扫描（`find_unredacted_keys`）及头对脱敏检查。

#### 配额/限流预算与重试策略

- 成本预算经由 `default_cost_thresholds()` 与 `check_cost_budget()` 按提供方强制：软阈值告警，硬阈值失败。
- 实时提供方调用使用确定性重试策略：
  - `LIVE_E2E_MAX_ATTEMPTS=3`
  - `LIVE_E2E_RETRYABLE_HTTP_STATUS=[408,429,500,502,503,504,529]`
  - `LIVE_E2E_RETRY_BACKOFF_MS=[500,1500]`（毫秒，指数式固定调度）
- 仅对瞬态失败重试（可重试 HTTP 状态或传输超时/重置类错误）。
- 实时提供方结果契约中必需重试遥测（`attempts`、`retry_backoff_ms`）。

#### 确定性回放边界与日志保障

- 实时 harness 执行模式始终为 `live_record`（仅 `VcrMode::Record`）。
- 边界定义：
  - 先发生实时网络调用 + 实时流式事件。
  - 调用后追踪提取从刚录制的磁带读取最新交互。
  - 本套件不允许 VCR 回放。
- 结果契约必须包含：
  - `execution_mode=live_record`
  - `replay_boundary=live_request_then_vcr_trace_extract`
  - `trace_origin=vcr_last_interaction`
- 归一化 JSONL 制品仍必须归一化时间戳/路径并保持脱敏。

---

## 测试替身清单基线（Test-Double Inventory Baseline，bd-1f42.8.1）

机器可读清单制品：
- `docs/test_double_inventory.json`

报告按以下维度标记测试替身使用：
- `file`
- `suite`（`unit`、`vcr`、`e2e`、`unit-inline`、`unclassified`）
- `module`
- 最近的 `test_case`
- `double_identifier` 与 `double_type`
- `risk` 与理由

当前基线快照（来自 `report_id=bd-1f42.8.1-test-double-inventory-v2`，生成于 `2026-02-13T04:24:50Z`）：
- `entry_count`：267
- `module_count`：21
- 套件分布：`unit-inline` 116、`vcr` 73、`unit` 16、`e2e` 26、`unclassified` 36

高风险聚集：
- `src/extension_dispatcher`（86 条，高）
- `src/extensions`（22 条，高）
- `tests/extensions_provider_oauth`（28 条，高）
- `tests/e2e_provider_scenarios`（23 条，高）

解读说明：
- `unit-inline` 中的高计数代表严格审计热点，应对照无模拟策略意图审查。
- `tests/common` 有意为仅辅助模块，不属于 `tests/*.rs` 套件分类的直接条目。
- 本文档中的允许清单例外仍为策略事实来源；JSON 报告为可搜索证据索引。

---

## 定义（Definitions）

| 术语（Term） | 定义（Definition） | 在套件 1 中允许？ |
|------|------------|----------------------|
| **Mock** | 以可编程行为替换依赖的对象，可选调用校验。匹配 `Mock*`、`Fake*`、`Stub*` 的标识符。 | 否 |
| **VCR 磁带** | 测试期间回放的已录制 HTTP 交互。 | 否 |
| **夹具文件** | 从磁盘加载的预构建 JSON/文本数据。 | 否 |
| **桩类型** | trait 的空操作或最小实现（`NullSession`、`NullUiHandler`）。 | 否 |
| **测试辅助** | 以受控输入演练真实逻辑的类型（`DeterministicClock`、`SharedBufferWriter`）。 | 是 |
| **Tempfile** | 经由 `tempfile` crate 的真实文件系统 I/O。 | 是 |
| **真实 TCP** | 用于测试 HTTP 客户端代码的本地 `TcpListener`。 | 仅套件 2 |

---

## 允许清单例外（Allowlisted Exceptions）

套件 1 之外的每个 mock/桩使用必须在此显式列入允许清单并附理由：

| 标识符（Identifier） | 位置（Location） | 套件 | 理由（Rationale） | 负责人（Owner） | 替换计划 |
|------------|----------|-------|-----------|-------|------------------|
| `MockHttpServer` | `tests/common/harness.rs` | 2 | 真实本地 TCP；命名有误导（实为真实服务器）。用于 VCR 无法表达的原始字节注入（非法 UTF-8）。 | infra | 永久：VCR 存储 UTF-8 字符串，无法表达原始非法字节。 |
| `MockHttpRequest` | `tests/common/harness.rs` | 2 | `MockHttpServer` 的请求构建器。 | infra | 同 `MockHttpServer` — 永久伴生类型。 |
| `MockHttpResponse` | `tests/common/harness.rs` | 2 | `MockHttpServer` 的响应构建器。 | infra | 同 `MockHttpServer` — 永久伴生类型。 |
| `PackageCommandStubs` | `tests/e2e_cli.rs` | 3 | CLI E2E 的离线 npm/git 桩；已记录至 JSONL。 | infra | 永久：真实 npm/git 操作非确定性。 |
| `RecordingSession` | `tests/extensions_message_session.rs` | 2 | 会话 API 表面测试。 | bd-m9rk | 替换为 `SessionHandle`（真实会话）。多数用例已迁移。 |
| `RecordingHostActions` | `tests/e2e_message_session_control.rs` | 2 | 扩展宿主动作录制；在智能体循环提供宿主动作处需要。 | bd-m9rk | 评估智能体循环集成测试是否可替代录制。 |
| `MockHostActions` | `src/extensions.rs`（单元测试） | 2 | `sendMessage`/`sendUserMessage` 的模块内桩。 | bd-m9rk | 一旦存在完整集成测试则替换为基于真实会话的调度。 |

**新增例外的流程：** 新建 bead 并附理由。获取评审。在本表中添加并带 bead ID。在仓库根的 `.no-mock-allowlist` 中添加 `<path>:<Identifier>` 条目（`.github/workflows/ci.yml` 中的无模拟 CI 门禁读取该文件；无对应允许清单行不允许新增条目）。

### 已批准的非模拟标准（Ratified Non-Mock Standard，bd-1f42.1.3）

本节为测试替身的权威接受/拒绝矩阵。

接受（附显式理由与范围）：
- 保持真实协议行为的真实本地测试基础设施辅助（`MockHttpServer` 族）。
- 用于为契约断言捕获宿主/会话副作用的录制替身（`RecordingSession`、`RecordingHostActions`）。
- E2E 中用于在保持端到端流程断言的同时隔离外部包管理器的 CLI 工作流桩（`PackageCommandStubs`）。

拒绝：
- 套件 1（`unit`）测试中的任何 `Mock*`、`Fake*`、`Stub*`、`DummyProvider`、`NullSession` 或 `NullUiHandler`。
- 套件 1 中任何抑制真实行为而非演练生产逻辑的新空操作 trait 实现。
- 任何无显式负责人、到期日与替换计划的新允许清单条目。

临时允许的强制例外模板（必需）：
- `bead_id`：证明例外的跟踪问题。
- `owner`：单一可问责负责人。
- `expires_at`：硬到期日（UTC）。
- `replacement_plan`：移除替身的具体路径。
- `scope`：允许例外的精确文件/测试。
- `verification`：证明尽管存在临时替身行为仍被覆盖的 CI/测试。

---

## CI 强制（CI Enforcement）

### 已有守护（ci.yml）

1. **无模拟依赖守护：** 若 `Cargo.toml` 或 `Cargo.lock` 中出现 `mockall`、`mockito` 或 `wiremock` 则失败。
2. **无模拟代码守护：** 若 `tests/` 中在允许清单正则之外出现 `Mock*`、`Fake*` 或 `Stub*` 标识符则失败。

### 新守护（本策略）

3. **套件分类守护：** 若任何 `tests/*.rs` 文件未在 `tests/suite_classification.toml` 中列出则失败。确保每个测试文件都有显式套件归属。
4. **VCR 泄漏守护：** 若套件 1 测试导入 `VcrRecorder`、`VcrMode`、`cassette_root` 或从 `tests/fixtures/vcr/` 加载文件则失败。
5. **Mock 泄漏守护：** 守护 #2 的增强版，亦检查套件 1 的 `src/` 测试模块中是否存在 `NullSession`、`NullUiHandler`、`DummyProvider`。

### CI 门禁通道（bd-1f42.8.8.1）

CI 门禁组织为两条评估通道：

**Preflight 快速失败通道：** 仅评估阻塞门禁，遇首个失败即停。用于 PR 快速反馈。命令：
```bash
cargo test --test ci_full_suite_gate -- preflight_fast_fail --nocapture --exact
```

**完整认证通道：** 评估全部门禁（阻塞 + 非阻塞），生成豁免审计，并产出含晋升规则与重跑指引的裁决。命令：
```bash
cargo test --test ci_full_suite_gate -- full_certification --nocapture --exact
```

**Drop-in 契约门禁（bd-35t7i）：** 仅当 `docs/contracts/dropin-certification-contract.json` 评估为全部硬门禁 `pass` 且产出的 `docs/evidence/dropin-certification-verdict.json` 中 `overall_verdict = CERTIFIED` 时，才允许严格 drop-in 发布措辞。操作的事件响应见 `docs/ci-operator-runbook.md` 的 **Parity Incident Response (DROPIN-162)**。

制品：
- `tests/full_suite_gate/preflight_verdict.json`（模式 `pi.ci.preflight_lane.v1`）
- `tests/full_suite_gate/certification_verdict.json`（模式 `pi.ci.certification_lane.v1`）
- `tests/full_suite_gate/waiver_audit.json`（模式 `pi.ci.waiver_audit.v1`）
- `tests/full_suite_gate/replay_bundle.json`（模式 `pi.e2e.replay_bundle.v1`）

### 豁免策略（Waiver Policy，bd-1f42.8.8.1）

CI 门禁可通过 `tests/suite_classification.toml` 中可审计豁免临时绕过。每个豁免需要：`owner`、`created`、`expires`（最长 30 天）、`bead`、`reason`、`scope`、`remove_when`。

规则：
- 最长豁免期：30 天（必须续期或修复）。
- 过期豁免经由 `waiver_lifecycle` 门禁导致 CI 硬失败。
- 距到期 3 天内触发告警。
- 每个豁免的 `gate_id` 必须匹配 `ci_full_suite_gate.rs` 中定义的门禁。

见 `docs/qa-runbook.md`“豁免生命周期”节获取完整模式与示例。

---

## 套件分类文件（Suite Classification File）

`tests/suite_classification.toml` 将每个测试文件映射到其套件：

```toml
[suite.unit]
files = [
    "model_serialization",
    "config_precedence",
    "session_conformance",
    "error_types",
    "bench_schema",
    "compaction",
]

[suite.vcr]
files = [
    "provider_streaming",
    "agent_loop_vcr",
    "auth_oauth_refresh_vcr",
    "provider_error_paths",
    "error_handling",
    "http_client",
]

[suite.e2e]
files = [
    "e2e_cli",
    "e2e_tui",
    "e2e_tools",
    "e2e_provider_streaming",
]
```

> 完整清单以 `tests/suite_classification.toml` 为准；上表为摘录。

---

## 快速本地冒烟套件（Fast Local Smoke Suite，bd-1f42.6.6）

贡献者可在推送前运行快速冒烟检查，无需等待完整 CI 即可捕获常见回归。冒烟套件在开发机上目标 60 秒内完成。

**命令：**
```bash
./scripts/smoke.sh                    # lint + unit + VCR 冒烟目标
./scripts/smoke.sh --skip-lint        # 跳过 cargo fmt/clippy（更快）
./scripts/smoke.sh --only unit        # 仅 unit 冒烟目标
./scripts/smoke.sh --only vcr         # 仅 VCR 冒烟目标
./scripts/smoke.sh --verbose          # 显示完整 cargo test 输出
./scripts/smoke.sh --json             # 向 stdout 发射 JSON 摘要
```

**覆盖内容：**
| 套件 | 目标 | 覆盖领域 |
|-------|---------|---------------|
| Unit | `model_serialization`、`config_precedence`、`session_conformance`、`error_types`、`compaction`、`security_budgets` | 核心数据模型、配置、会话、错误处理 |
| VCR | `provider_streaming`、`error_handling`、`http_client`、`sse_strict_compliance`、`model_registry`、`provider_factory` | 提供方层、HTTP、SSE、模型路由 |

**结构化输出：**
- `smoke_log.jsonl`：按事件 JSONL 日志（模式 `pi.smoke.*.v1`）
- `smoke_summary.json`：机器可读通过/失败摘要（模式 `pi.smoke.summary.v1`）
- `<target>/output.log`：按目标的详细输出

---

## 不稳定测试隔离与升级策略（Flaky-Test Quarantine and Escalation Policy，bd-1f42.6.3）

不稳定测试削弱 CI 信号并侵蚀对测试套件的信任。本节定义不稳定测试的分类、隔离工作流、升级规则与可审计跟踪。

### 不稳定分类（Flake Taxonomy）

每个不稳定测试必须归入恰好一个类别。分类决定隔离层级、自动重试预算与升级时间线。

| 类别 | 代码 | 描述 | 重试预算 | 隔离层级 |
|----------|------|-------------|-------------|-----------------|
| **时序依赖** | `FLAKE-TIMING` | 竞态条件、基于 sleep 的断言、非确定性调度、CI 负载敏感。 | 1 次重试 | 7 天修复窗口 |
| **环境依赖** | `FLAKE-ENV` | 文件系统状态、区域、时区、OS 特定行为、缺失系统依赖。 | 1 次重试 | 7 天修复窗口 |
| **网络依赖** | `FLAKE-NET` | DNS 解析、端口冲突、防火墙规则、VPN 状态、代理设置。 | 1 次重试 | 14 天修复窗口 |
| **资源依赖** | `FLAKE-RES` | OOM、磁盘满、文件描述符耗尽、线程池饱和。 | 1 次重试 | 14 天修复窗口 |
| **外部服务** | `FLAKE-EXT` | 实时 API 限流、提供方宕机、认证令牌过期、配额耗尽。 | 1 次重试 | 14 天修复窗口 |
| **非确定性逻辑** | `FLAKE-LOGIC` | 随机种子、哈希顺序、浮点比较、并发数据结构。 | 1 次重试 | 7 天修复窗口 |

**硬限制：** 最长隔离窗口为 **14 天**，与类别无关。CI 守护拒绝 `expires - quarantined > 14` 的条目。

### 隔离生命周期

```
检测 ──► 分类 ──► 隔离条目 ──► 修复/变通 ──► 恢复 ──► 验证
```

#### 步骤 1：检测

当以下情况时测试被怀疑为不稳定：
- 在 CI 上失败但重试通过（同一提交、同一 runner OS）。
- 本地通过但在 CI 上间歇失败。
- 在同一提交的多次运行中以不同错误信息失败。

**证据要求：** 检测主张必须包含提交 SHA、CI 运行 URL 或日志摘录、同一提交上至少一次通过运行、runner OS 及相关环境变量。

#### 步骤 2：分类

分配分类并记录 `category`、`evidence_url`、`reproduction_command`。

#### 步骤 3：隔离条目

在 `tests/suite_classification.toml` 的 `[quarantine]` 节中添加测试：

```toml
[quarantine.example_flaky_test]
category = "FLAKE-TIMING"
owner = "AgentName"
quarantined = "2026-02-10"
expires = "2026-02-17"          # 距 quarantined 最长 14 天
bead = "bd-XXXX"                # 修复的跟踪 bead
evidence = "https://ci.example.com/run/12345"
repro = "cargo test example_flaky_test -- --nocapture"
reason = "Intermittent timeout on CI due to thread scheduling variance"
remove_when = "Two consecutive green CI runs on Linux/macOS/Windows"
```

**隔离含义：**
- 测试仍被编译与运行，但失败在 CI 中**非阻塞**。
- 隔离测试失败在独立的 CI 摘要节中报告。
- 测试保留其原始套件分类（unit/vcr/e2e）。
- 在标记为隔离失败前按类别重试预算自动重试。

#### 步骤 4：修复或变通

负责人必须在隔离层级的修复窗口内修复根因或应用确定性变通。可接受修复包括消除非确定性源、增加正确同步、对环境可用性设门、或从实时转为 VCR 支撑（针对 `FLAKE-EXT` 与 `FLAKE-NET`）。

#### 步骤 5：恢复

修复落地后：从 `tests/suite_classification.toml` 的 `[quarantine]` 中移除条目；在 3 次连续 CI 运行中验证通过；以链接修复提交与 CI 证据的评论关闭跟踪 bead。

#### 步骤 6：到期强制

若隔离测试在 `expires` 日期前**未修复**：隔离条目转为 CI **硬失败**；负责人必须延期（最多一次）或禁用测试。

### 自动重试策略

| 设置 | 值 |
|---------|-------|
| 最大自动重试 | 1 |
| 重试延迟 | 5 秒 |
| 重试范围 | 仅失败目标 |
| 二次失败策略 | 视为确定性失败 |

非隔离测试获得 **零重试**。若非隔离测试失败，即为真实失败。

### CI 隔离守护

隔离守护作为 CI（`.github/workflows/ci.yml`）的一部分运行并校验全部 9 个必填字段、类别合法性、隔离跨度不超过 14 天等，并产出 `tests/quarantine_report.json` 与 `tests/quarantine_audit.jsonl`。

### 隔离决策模板

```
标题: [FLAKE] <test_name>: <简述>
类型: bug
优先级: P1（层级 1）或 P2（层级 2-3）

类别: FLAKE-TIMING | FLAKE-ENV | FLAKE-NET | FLAKE-RES | FLAKE-EXT | FLAKE-LOGIC
负责人: <智能体或人名>
隔离于: <YYYY-MM-DD>
到期: <YYYY-MM-DD>（距隔离最长 14 天）
证据: <CI 运行 URL 或制品路径>
复现: <精确命令>
移除条件: <隔离移除的客观退出条件>

根因分析:
  <何种原因导致测试非确定性？>

拟议修复:
  <如何恢复确定性？>

验证计划:
  <如何确认修复有效？（如 3 次干净 CI 运行）>
```

---

> 术语对照：`provider → 提供方`、`extension → 扩展`、`VCR cassette → VCR 磁带`、`Beads → Beads`（保留英文）、路径字符串保留英文。
