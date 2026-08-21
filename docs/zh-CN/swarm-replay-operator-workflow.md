# 集群回放操作员工作流

目的：说明操作员与开发者如何在不将离线回放证据视为实时协调事实或发布证据的情况下，捕获、预览与解读离线集群回放证据。

本指南涵盖 `bd-in57w` 下交付的回放实验室：`src/swarm_replay.rs` 中的只读轨迹摄取器、`pi swarm-replay-preview` CLI 界面、操作员运行包集成以及无模拟端到端证据 harness。面向需要理解集群中发生了什么、比较建议策略并交接下一个安全动作的多智能体操作员，而无需改变 Beads、Agent Mail、git、RCH 或实时构建槽位。

## 什么是回放

集群回放是对已捕获协调制品的离线分析工具。

它可以：

- 将 Beads、Agent Mail 归档快照、预留记录、RCH 队列/状态事实、运行包交接数据、git 状态、验证制品、活动账本行与飞行记录器行归一化为 `pi.swarm.replay_trace.v1` 轨迹。
- 将这些事件回放为确定性快照。
- 在这些快照上评估内置的建议策略。
- 为审计发出 JSON、文本、对比、清单与 JSONL 事件证据。
- 将回放预览馈入 `scripts/build_swarm_operator_runpack.py`，使交接包可展示策略对比上下文。

它不能：

- 认领或重新打开 Beads。
- 发送 Agent Mail 消息或预留文件。
- 取消、启动或优先处理 RCH 任务。
- 暂存、提交、推送、贮藏、重置、清理或编辑 git 状态。
- 替代 `pi doctor --only swarm`、Beads、Agent Mail、RCH、CI 或发布证据闸门。
- 证明面向发布的性能、严格的 drop-in 认证或实时集群就绪度。

将回放输出视为可复现的操作员证据。以源系统为准。

## 源边界

| 问题 | 事实来源 | 回放角色 |
|------|----------|----------|
| 现在谁拥有任务？ | `br show`、`br list --status=in_progress --json` | 展示捕获时轨迹观察到的内容。 |
| 预留是否活跃？ | Agent Mail 文件预留（Mail 健康时） | 展示已捕获的预留与冲突，包括降级或缺失 Mail 的证据。 |
| RCH 现在是否饱和？ | `rch status`、`rch queue`、`scripts/cargo_headroom.sh --runner rch --admit-only ...` | 展示已捕获的队列压力与建议策略差异。 |
| 是否可作发布声明？ | 声明完整性、认证与性能证据闸门 | 永远不对发布声明具权威性。 |
| 下一位操作员应检查什么？ | Beads、Doctor、Agent Mail、RCH、git 与运行包源制品 | 对建议策略排序并高亮缺失数据。 |

## 捕获输入

对于完整的操作员交接，先捕获当前源事实：

```bash
capture_dir="/data/tmp/pi_swarm_replay/${AGENT_NAME:-agent}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$capture_dir"

br list --json > "$capture_dir/beads.json"
br ready --json > "$capture_dir/beads-ready.json"
git status --short --branch > "$capture_dir/git-status.txt"
rch status > "$capture_dir/rch-status.txt"
rch queue > "$capture_dir/rch-queue.txt"

pi doctor --only swarm --format json > "$capture_dir/doctor-swarm.json"
scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets \
  --decision-json "$capture_dir/cargo-admission.json"
```

Agent Mail 可能不可用或部分降级。若 MCP 写入失败，保留可用的只读证据：收件箱快照、智能体列表、预留导出文件或 Doctor 集群发现。不要虚构绿色的 Mail 状态来让回放看起来完整。

在产生运行包证据时，将回放制品与运行包捕获放在一起：

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --capture-current \
  --capture-dir "$capture_dir/runpack" \
  --project-root /data/projects/pi_agent_rust \
  --agent-name "${AGENT_NAME:-agent}" \
  --out-json "$capture_dir/operator-runpack.json" \
  --out-md "$capture_dir/operator-runpack.md"
```

运行包是对源制品的已脱敏索引，而非新的事实来源。

## 预览轨迹

在验证 CLI 界面时使用已检入的黄金轨迹：

```bash
pi swarm-replay-preview \
  --trace tests/golden_corpus/swarm_replay_trace/normalized_trace.json \
  --format json
```

使用显式输出路径写入可复现的预览制品：

```bash
pi swarm-replay-preview \
  --trace tests/golden_corpus/swarm_replay_trace/normalized_trace.json \
  --policy conservative_manual \
  --policy rch_fanout_limited \
  --policy build_slot_protective \
  --out-json "$capture_dir/swarm-replay-preview.json" \
  --out-text "$capture_dir/swarm-replay-preview.txt"
```

预览命令拒绝覆盖已请求的输出文件。请选择全新的捕获目录，或仅在已获显式删除许可时自行移除陈旧的临时制品。

仅在 JSON 已存在后才将预览馈入运行包：

```bash
python3 scripts/build_swarm_operator_runpack.py \
  --capture-current \
  --capture-dir "$capture_dir/runpack" \
  --project-root /data/projects/pi_agent_rust \
  --agent-name "${AGENT_NAME:-agent}" \
  --swarm-replay-preview-json "$capture_dir/swarm-replay-preview.json" \
  --out-json "$capture_dir/operator-runpack.json" \
  --out-md "$capture_dir/operator-runpack.md"
```

运行包为交接投射预览摘要。它不替代回放轨迹、策略报告或源证据。

## 策略差异

内置策略集是确定性的：

| 策略 ID | 预期偏向 | 典型信号 |
|-----------|---------------|--------------|
| `conservative_manual` | 偏好等待、人工复核与低变更风险。 | 证据稀疏或降级时有用。 |
| `existing_autopilot` | 建模当前自动驾驶风格的下一步动作行为。 | 适合作比较新策略想法的基线。 |
| `rch_fanout_limited` | 当可见 RCH 压力时减少重量级验证。 | 队列深度或本地回退风险较高时有用。 |
| `stale_bead_reclaiming` | 在证据复核后回收明显陈旧的进行中工作。 | 当陈旧 Beads 阻塞就绪工作且属主活动已久时有用。 |
| `build_slot_protective` | 保护活跃构建槽位并避免重叠的高成本闸门。 | 在编译风暴或共享 target/TMPDIR 压力期间有用。 |

将策略排名解读为建议差异：

- `rank` 与 `score` 仅在单一轨迹内比较策略。
- 当源数据缺失、畸形、陈旧或过薄时 `confidence` 下降。
- `missing_data` 列出回放抑制而非猜测的声明。
- `rationale` 解释驱动评分的证据。
- 高分策略不授权实时变更。它告诉操作员接下来应检查哪些源系统。

若两条策略不一致，优先选择保持实时系统不变的那条，直到 Beads、Agent Mail、RCH 与 git 事实已刷新。

## 缺失或畸形数据

回放必须 fail-closed。常见的降级状态是预期的：

| 缺失事实 | 回放行为 | 操作员响应 |
|--------------|---------------|--------------|
| Agent Mail 不可用 | 标记 Mail 源不可用并抑制实时预留确定性。 | 以 Beads 受托人/状态作软锁并保持文件范围狭窄。 |
| RCH 队列畸形 | 抑制队列深度指标并避免乐观的扇出建议。 | 在启动 cargo 前运行 `rch status` 与 `rch queue`。 |
| 运行包缺失 | 省略运行包建议与交接字段。 | 若交接上下文重要，则从源制品重建运行包。 |
| 脏 git 状态缺失 | 避免声称干净工作区就绪。 | 运行 `git status --short --branch` 并仅暂存自有文件。 |
| 策略未产生决策 | 将策略决策覆盖标记为缺失。 | 不要将该策略排名为可操作胜者。 |

缺失数据是有用的发现。不要用占位符绕过它。

## 隐私与脱敏

回放制品应携带 ID、状态、schema 名、命令标签、制品路径、计数与已脱敏摘要。不应携带提示正文、提供方记录稿、API 密钥、bearer token、cookie、密钥或原始私密消息正文。

添加来源时使用以下规则：

- 存储稳定标识符如 bead ID、消息 ID、预留 ID、RCH 任务 ID、验证 ID 与 git SHA。
- 存储命令名与退出状态，而非完整的携带密钥的环境。
- 脱敏形如 `prompt`、`body`、`transcript`、`token`、`secret`、`password`、`authorization`、`bearer`、`cookie` 或 `key` 的字段。
- 保留带计数与字段名的脱敏摘要，以便复核者知道已移除内容。
- 优先使用制品路径而非嵌入大型源载荷。

若脱敏状态未知，则将该制品视为尚未就绪不宜广泛交接。

## 大主机预算配置

回放在运行于拥有 64 核以上与 256 GiB 以上 RAM 主机的集群中常见。大型机器仍需显式准入限制，因为 RCH 槽位、target 目录、文件描述符、终端渲染与扩展宿主调用通道可能在原始 CPU 或内存耗尽前就已饱和。

在提升扇出前使用此序列：

```bash
pi doctor --only swarm --format json > "$capture_dir/doctor-swarm.json"
scripts/cargo_headroom.sh --runner rch --admit-only check --all-targets \
  --decision-json "$capture_dir/cargo-admission.json"
rch status
rch queue
```

然后将回放指引与当前 Doctor 与 RCH 事实对比。`rch_fanout_limited` 与 `build_slot_protective` 旨在当已捕获压力已较高时引导操作员避免启动更多重量级检查。它们不设定永久限制；仅在获得新鲜本地证据前提供保守起点。

## Agent Mail 中断

Agent Mail 健康可能是部分可用的：读取可能正常而 bootstrap、发送或预留写入失败。在此状态下：

1. 尝试 `health_check`、`fetch_inbox` 与 `list_agents`。
2. 若收件箱读取正常，处理需 ack 的消息。
3. 若写入失败，在交接中记录数据库错误。
4. 通过 Beads 认领并保持文件面狭窄。
5. 提及 Beads 受托人状态是软锁。

回放应反映降级状态，而非假装 Mail 预留具权威性。

## RCH 竞争

本仓库中所有 CPU 密集型 Cargo 工作必须使用 `rch exec -- ...` 或强制 RCH 的仓库封装器。回放有助于解释验证动作被延迟的原因，但不授予跳过必需闸门的许可。

将已捕获的回放用作警告，然后刷新实时状态：

```bash
rch status
rch queue
env CARGO_TARGET_DIR="/data/tmp/pi_agent_rust_cargo/${AGENT_NAME:-agent}/target" \
  TMPDIR="/data/tmp/pi_agent_rust_cargo/${AGENT_NAME:-agent}/tmp" \
  rch exec -- cargo check --all-targets
```

若 RCH 已饱和，继续文档、源码检查或非重量级工作。不要强制本地全目标构建来让 bead 看起来完成。

## 故障排查

| 症状 | 可能原因 | 响应 |
|---------|--------------|----------|
| `swarm-replay-preview requires --trace` | 缺少轨迹路径。 | 传入 `--trace <pi.swarm.replay_trace.v1 JSON>`。 |
| `requires trace schema ...` | 错误的 JSON 制品。 | 使用归一化轨迹，而非运行包、预览或策略报告。 |
| `unsupported swarm-replay-preview policy` | 拼写错误或不支持的策略 ID。 | 使用上文列出的五个内置策略 ID 之一。 |
| 输出路径已存在 | CLI 拒绝覆盖证据。 | 使用新的捕获路径；仅在获显式许可时删除。 |
| 预览置信度低 | 缺失或畸形的源事实。 | 刷新源制品并重新运行预览。 |
| 运行包省略回放章节 | 未传入预览 JSON 或未通过 schema 检查。 | 在生成预览 JSON 后传入 `--swarm-replay-preview-json <path>`。 |

## 验证

对于仅改本文档的变更，运行：

```bash
python3 scripts/check_docs_purpose_headers.py
python3 -m json.tool docs/contracts/swarm-replay-trace-contract.json >/dev/null
python3 -m json.tool docs/schema/swarm_replay_preview.json >/dev/null
cargo fmt --check
git diff --check
./scripts/reconcile_beads_ledger.sh
```

若示例或 CLI 标志变更，还需通过 RCH 运行聚焦的 CLI 测试：

```bash
env CARGO_TARGET_DIR="/data/tmp/pi_agent_rust_cargo/${AGENT_NAME:-agent}/target" \
  TMPDIR="/data/tmp/pi_agent_rust_cargo/${AGENT_NAME:-agent}/tmp" \
  rch exec -- cargo test --test swarm_replay_preview_cli -- --nocapture
```

在关闭回放 bead 前，暂存文档与 Beads 变更，然后运行 `ubs --staged --only=rust .`。对于仅文档提交仍应记录；它可能报告无已暂存 Rust 文件。
