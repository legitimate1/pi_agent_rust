# 集群活动账本

集群活动账本是一个经脱敏的 JSONL 流，用于在不存储提示正文或密钥的情况下重建多智能体运行期间发生的事情。

每行使用 schema `pi.swarm.activity_ledger.v1` 并携带：

- `sequence`：生产者本地单调递增顺序。
- `timestamp_ms`：用于时间线重建的 Unix 毫秒时间戳。
- `kind`：`bead_status`、`agent_mail`、`file_reservation`、`rch_job`、`verification`、`git_commit`、`recovery` 或 `note` 之一。
- `ids.correlation_id`：用于关联相关 Beads、Agent Mail、RCH、验证与 Git 事件的必填稳定 ID。
- bead、Agent Mail 线程/消息、智能体、文件预留、RCH 任务、验证运行与 git SHA 的可选 ID。
- `details`：默认已脱敏的结构化元数据。
- `redaction`：序列化前已脱敏的计数与字段名。

该模块刻意以库优先设计。操作员或后续 CLI 层面可在工作发生时追加事件，用 `SwarmActivityLedger::to_jsonl` 导出 JSONL，并用 `timeline_from_jsonl` 重建确定性时间线。时间线重建按 `timestamp_ms`、再按 `sequence`、再按 `correlation_id` 排序，因此即使行以乱序收集，事件回顾仍保持确定性。

脱敏对常见敏感字段采用 fail-closed 策略。键名包含 `prompt`、`body`、`transcript`、`token`、`secret`、`password`、`authorization`、`bearer`、`cookie` 或 `key` 的序列化为 `[REDACTED]`；形似 bearer token、API 密钥或 password/token 赋值的取值也会被脱敏。应存储命令名、制品路径、状态码与 ID，而非提示、模型输出或原始凭据。

## 有界摘要

`SwarmActivityLedger::summarize` 与 `SwarmActivitySketch` 从原始行派生 schema `pi.swarm.activity_summary.v1`，而不替换或改变它们。原始 JSONL 仍是审计来源；摘要是面向仪表盘、交接说明与集群健康检查的有界内存视图。

摘要保留事件总数、已脱敏条目、已脱敏字段与按活动类型计数的精确合计。热点列表在智能体、Beads、验证 ID、工具、提供方/模型及选定详情键/值对上分别有界。并列按计数降序再按键升序确定性排序。较长的热点键在保留前会被截断，以免单个大详情值主导内存。

名为 `latency_ms`、`duration_ms` 或 `elapsed_ms` 的延迟详情会进入有界采样概要。摘要报告保留的样本数、最小值、p50、p95、p99、最大值及保守的排名误差界。概要可在多次运行间合并；接收方概要保持其配置的容量并将合并的延迟样本下采样回请求的界限。

## 集群摘要

`SwarmActivityLedger::digest`、`digest_from_jsonl` 与 `SwarmActivityDigest::to_text` 派生 schema `pi.swarm.activity_digest.v1`，用于交接与饱和度检查。摘要是在已有账本行之上的有界脱敏视图，而非新的事实来源。

摘要包括：

- 按事件计数的活跃智能体。
- 最近的 Beads 状态变更。
- 最近的 Agent Mail 事件。
- 文件预留活动。
- 来自验证、RCH 与 git 事件的验证证据。
- 按稳定归一化指纹分组的重复阻塞热点。
- 从最新代表事件起计量的陈旧 Agent Mail 线程。
- 针对新提交 bug 过少、重复工作、重复阻塞、对已关闭 bead 表面的重复编辑、从未转化为声明或预留的陈旧引入、在低提交/验证吞吐下的协调密集窗口以及陈旧线程的饱和信号。

JSON 形式对自动化保持稳定。文本形式对交接说明保持确定性，且仅使用已脱敏的摘要与选定详情字段。提示正文、记录稿、token、API 密钥、cookie、authorization 头及其他敏感值在到达任一输出前仍保持脱敏。

重复阻塞指纹是证据键，而非原始日志。对于 Cargo、Clippy、测试或 RCH 失败等诊断条目，动态路径、target 目录、PID、端口、时间戳、时长、长数字 ID 与十六进制 ID 在哈希前会被归一化，使多个智能体观察到的同一失败归为一组。每个重复阻塞仍携带来自已脱敏账本条目的有界 `sample` 摘录，以便操作员在不暴露提示正文或密钥的情况下识别原始失败。

将饱和度用作停止并重定向信号，而非性能声明。当摘要报告已关闭表面的 churn、陈旧引入或高闲聊低吞吐时，停止在同一评审循环上启动更多智能体，转为更窄的实现 bead、对某一子系统的更深审计或显式的阻塞清理。`saturation.signals` 字段列出活跃的类型化信号，`saturation.evidence_pointers` 命名导致每个信号的已脱敏智能体、bead、线程或窗口计数，以便操作员在不读取提示正文的情况下验证决策。

当饱和度活跃时，摘要还可能发出 `recommendations`：仅从已脱敏的 `saturation` 字段派生的确定性建议分流提示。这些建议命名下一个工作模式，如窄实现 bead、`testing-golden-artifacts`、`testing-conformance-harnesses`、`mock-code-finder`、`deadlock-finder` 或性能分析，并附带置信度与支持该提示的已脱敏证据指针。它们不会调用工具、分配工作或覆盖 Beads/Agent Mail 归属；操作员必须将其视为选择下一个更窄动作的起点，而非强制指令。

## 尾延迟机制守卫

`src/resource_governor.rs` 中的 `TailLatencyRegimeGuard` 消费实时 p99、p999、队列深度与资源压力样本，以检测集群何时偏离已校准的运行机制。它在进入保守回退前要求连续违规样本，在返回校准模式前要求连续恢复样本，因此短暂尖峰不会使控制器抖动。

机制决策发出 schema `pi.resource_governor.tail_latency_regime.v1`，其中包含活跃机制、回退状态、滞后 streak、实时样本及回退原因如 `p99_latency`、`p999_latency`、`queue_depth`、`resource_pressure` 或 `hysteresis_hold`。当回退活跃时，调用方可将决策应用于 `HostResourceBudgets`，在准入检查前降低输出、队列深度、进程、文件描述符、负载与 RSS 预算。

## 容量规划器

`src/resource_governor.rs` 中的 `plan_swarm_capacity_from_jsonl` 将会话工作负载矩阵 `swarm_metrics` JSONL 行与 `SwarmHostInventory` 转为 schema `pi.resource_governor.capacity_plan.v1`。生成的计划包含活跃智能体并发、工具并发、扩展宿主调用通道、RCH 验证扇出、内存压力阈值、退避窗口、`HostResourceBudgets` 与 `TailLatencyRegimeConfig` 的保守初始值。

当不存在完整的 `swarm_metrics` 证据、所需嵌套字段缺失、主机清单为零，或延迟/RSS/队列值无法解析为有限非负数时，规划器 fail-closed。不含 `swarm_metrics` 的行会被忽略，因此混合 harness JSONL 仍可处理；声明 `swarm_metrics` 却省略必填字段的行会被拒绝。

使用 `SwarmCapacityPlan::what_if` 针对更小的 CPU/RAM 清单重放同一证据摘要。这适用于快速的操作员预算，例如在将这些预算接入 `ResourceGovernor` 之前，检查 64 核/256GiB 证据运行在 16 核/1GiB 受限主机上是否会建议更低的智能体与 RCH 扇出。

容量建议是起点，而非安全最大值的证明。证据稀疏、主机容量不匹配、报告 CPU 使用率为零、队列深度下限与 RSS 余量压力时，置信度会下降或发出不确定性。文件描述符限制仍以保守的内置默认值界定，因为当前集群 harness 记录 CPU/RAM 清单但不记录主机 fd 限制。

`generate_operator_budget_profiles_from_jsonl` 将一次已验证的容量证据运行重放为常见大型主机起点的 schema `pi.resource_governor.operator_budget_profiles.v1`：

- `cpu16_mem64gib`：16 个逻辑 CPU，64 GiB RAM。
- `cpu32_mem128gib`：32 个逻辑 CPU，128 GiB RAM。
- `cpu64_mem256gib`：64 个逻辑 CPU，256 GiB RAM。

每个配置携带智能体并发、工具并发、扩展宿主调用通道、RCH 验证扇出、内存压力阈值、退避窗口、`HostResourceBudgets`、尾延迟守卫设置、置信度与注意事项。从不同源清单派生的配置会从高置信度降级为中置信度并包含源证据注意事项。每个配置还包含 `starting_point_not_release_performance_claim`，以免操作员预算被误认为基准或发布声明。

对于空配置集、零 CPU/RAM 清单、缺失 `swarm_metrics`、无效延迟/RSS/队列证据或畸形主机类别清单，配置生成器 fail-closed。在生产主机上提升上限前，将默认配置用作初始集群准入输入，然后从新鲜本地证据重新生成它们。

## 实时准入控制器

`SwarmAdmissionController` 将已验证的 `SwarmCapacityPlan`、`ResourceGovernor` 与 `TailLatencyRegimeGuard` 组合为 schema `pi.resource_governor.swarm_admission_controller.v1`。每个决策接收请求、实时主机样本、实时 p99/p999/队列/资源压力样本及当前集群负载计数，然后返回最终的 `admit`、`backpressure` 或 `deny` 动作。

控制器使用计划的资源预算进行主机压力检查，使用计划的尾延迟阈值进行保守回退，并使用计划的活跃智能体/工具/RCH/扩展通道建议作为实时容量上限。容量压力可使决策比主机资源决策更严格，因此即使主机看似健康，当集群已达计划并发预算时仍会背压或拒绝。

## 准入重放

`src/resource_governor.rs` 中的 `replay_swarm_admission_from_jsonl` 针对预验证的 `SwarmCapacityPlan` 与已捕获的 `SwarmAdmissionReplaySample` 值重放 schema `pi.swarm.activity_ledger.v1` 行。报告 schema 为 `pi.resource_governor.swarm_admission_replay.v1`。

重放是离线事件分析，而非实时 doctor 输出。它从不采样当前主机、Agent Mail、Beads 或 RCH。每个决策均派生自已脱敏的账本行与已捕获的资源样本，因此在实时机器状态已改变后仍可确定性地重放旧事件。

可重放的账本类型为 `bead_status`、`agent_mail`、`file_reservation`、`rch_job` 与 `verification`。行按 `timestamp_ms`、再按 `sequence`、再按 `correlation_id` 排序，与时间线重建一致。可选详情字段可细化请求：

- `request_operation` 或 `operation`：`tool`、`exec`、`http`、`session`、`ui`、`events`、`log` 或 `unknown` 之一。
- `request_capability` 或 `capability`：附加到重放请求的能力标签。
- `estimated_tool_output_bytes` 或 `tool_output_bytes`：请求输出预算输入。
- `queue_depth`：请求队列深度输入。
- `expected_action`、`expected_admission_action` 或 `admission_action`：用于分歧标记的可选比较值。

每份报告包含决策时间线、每个重放决策的主导容量压力，以及针对重复 correlation ID、陈旧或缺失样本、无效 expected-action 详情与 expected-action 不匹配的分歧标记。缺失的可选请求字段对账本类型使用确定性默认值。缺失或陈旧的资源样本为 fail-closed：报告状态变为 `fail_closed`，受影响事件不会获得乐观决策。

`assert_swarm_digest_admission_replay_alignment` 产生独立的 schema `pi.resource_governor.swarm_admission_replay_digest_alignment.v1` 断言报告，用于比较源自记录稿的摘要与重放报告。摘要对记录稿汇总与饱和度证据（`saturation.reasons` 与 `saturation.evidence_pointers`）保持权威。重放报告对已捕获的主机资源、尾延迟、实时负载与准入决策证据保持权威。对齐断言仅检查这些制品在严重程度上是否一致。

当摘要饱和时，重放必须显示背压、拒绝或 fail-closed 重放状态，操作员才会将该运行视为不安全不宜扩容。当摘要无饱和信号时，重放必须保持安全，操作员才会将记录稿与准入证据视为对齐。任何不匹配都会发出 `status = fail_closed` 并附带可操作断言，如暂停新的智能体启动、检查摘要证据或在改变扇出预算前刷新已捕获的重放样本。这在为夹具提供从记录稿饱和度到准入预期的确定性桥梁的同时，保持重放 schema 向后兼容。

## 无模拟集群冒烟 Harness

`scripts/run_swarm_smoke_harness.py` 针对真实本地协调面演练操作员工作流，在保留的临时项目中进行。它创建一次性的 Beads 工作区，验证 claim-to-close 状态转换，初始化带有无关脏文件的临时 git 仓库，针对已分配文件运行并发模拟智能体，检查脏文件未被移除或改变，通过 MCP HTTP 端点注册三个 Agent Mail 身份，发送夹具线程消息，预留并释放真实文件预留，强制制造预留冲突，将进行中的 bead 扫描为陈旧，并为实时 RCH 态势记录 `scripts/cargo_headroom.sh --admit-only` 决策，外加一个 RCH 不可用的隔离 PATH。

安全自测：

```bash
python3 scripts/run_swarm_smoke_harness.py --self-test
```

带固定制品目录的操作员运行：

```bash
python3 scripts/run_swarm_smoke_harness.py \
  --correlation-id bd-2zcs5.26-smoke \
  --out-dir /data/tmp/pi_swarm_smoke_artifacts/bd-2zcs5.26
```

harness 写入 schema `pi.swarm.smoke_harness.v1` 摘要与 `pi.swarm.smoke_harness.event.v1` JSONL 事件。每个事件包含 correlation ID、命令运行时的命令计时、脱敏元数据以及相关的智能体名、bead ID、预留 ID 或 RCH 准入决策。Agent Mail 注册 token 与敏感外观的命令输出在到达制品包前被脱敏。冒烟夹具默认将任何进行中的临时 bead 视为陈旧；传入 `--stale-after-seconds` 以测试更长的操作员阈值。若请求的输出目录中已存在 `events.jsonl` 或 `summary.json`，harness 将失败而非覆盖证据。

harness 不会删除或重置生产文件。生成的夹具项目与制品有意保留在 `TMPDIR` 或 `/data/tmp` 下，以便操作员在冒烟运行失败后检查。若实时 RCH 态势降级，RCH 准入场景会记录退避决策而非强制本地重型 cargo 回退。

## 操作员运行包封装器

`scripts/build_swarm_operator_runpack.py` 从现有源制品为单一操作员交接视图组装 schema `pi.swarm.operator_runpack.v1`。它接受来自 `pi doctor --only swarm --format json`、`scripts/report_swarm_claim_readiness.py`、`scripts/run_swarm_smoke_harness.py`、`scripts/cargo_headroom.sh --admit-only`、Beads JSON、git porcelain 输出及最新 `pi.swarm.activity_digest.v1` 摘要的已捕获 JSON。脚本仅读取传入给它的文件，脱敏敏感字段与 token 形态取值，并在提供的源畸形时拒绝而非发出部分乐观证据。

运行包包含 schema `pi.swarm.safety_scorecard.v1` 作为 `swarm_scale_safety_scorecard`。其七个维度涵盖协调健康、cargo/RCH 态势、性能证据新鲜度、脏工作区容忍度、停滞 Bead 卫生、资源管控就绪度与冒烟测试覆盖度。除非所需源制品成功加载且维度保留回汇总运行包字段的带点证据路径，否则维度无法评为绿色。

安全自测：

```bash
python3 scripts/build_swarm_operator_runpack.py --self-test
```

示例操作员捕获：

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --doctor-json /data/tmp/pi_swarm_runpack/doctor.json \
  --claim-readiness-json /data/tmp/pi_swarm_runpack/claim-readiness.json \
  --smoke-summary-json /data/tmp/pi_swarm_runpack/smoke-summary.json \
  --activity-digest-json tests/full_suite_gate/swarm_activity_digest.json \
  --cargo-admission-json /data/tmp/pi_swarm_runpack/cargo-admission.json \
  --beads-json /data/tmp/pi_swarm_runpack/beads.json \
  --git-status-file /data/tmp/pi_swarm_runpack/git-status.txt \
  --out-json /data/tmp/pi_swarm_runpack/operator-runpack.json \
  --out-md /data/tmp/pi_swarm_runpack/operator-runpack.md
```

运行包刻意不是新的事实来源。Beads 对任务归属保持权威，Agent Mail 对预留与消息保持权威，doctor 对实时集群诊断保持权威，claim-readiness 对面向发布的证据状态保持权威。将运行包视为有界、已脱敏的索引，它告诉下一位操作员应首先检查哪些源制品。

示例行：

```json
{"schema":"pi.swarm.activity_ledger.v1","sequence":0,"timestamp_ms":1778223600000,"kind":"verification","summary":"cargo check completed","ids":{"correlation_id":"bd-2zcs5.17:verify:1","bead_id":"bd-2zcs5.17","agent_name":"CopperOx","rch_job_id":"29832517041259999","verification_id":"check-all-targets"},"details":{"command":"cargo check --all-targets","status":"passed"},"redaction":{"redacted_count":0}}
```

使用账本进行事件回顾与交接。它补充 Beads 与 Agent Mail，而非作为事实来源替代它们。
