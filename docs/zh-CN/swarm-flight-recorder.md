# 集群飞行记录器

集群飞行记录器是面向多智能体运行的确定性端到端证据 harness。它将来自真实 `AgentSession` 执行、内置工具调用、JS 扩展钩子、会话持久化快照与外部协调标记的已脱敏运行时事件记录为 schema 为 `pi.swarm.flight_recorder.event.v1` 的 JSONL 行。

该 harness 设计为无需实时提供方凭据即可重放。测试使用确定性的进程内提供方与真实的 Pi 运行时组件，因此操作员无需依赖 OpenAI、Anthropic 或其他提供方账号即可检查时序与协调行为。

## 制品

- `swarm_flight_recorder.jsonl`：追加式事件行，包含 `correlationId`、`agentName`、`component`、`eventKind`、脱敏摘要与已脱敏载荷。
- `swarm_flight_recorder_report.json`：schema 为 `pi.swarm.flight_recorder.report.v1` 的摘要报告，包含重放命令、主导延迟组件、组件计数与协调失败。

每个 JSONL 行由 `validate_swarm_flight_recorder_jsonl` 校验当前 schema、单调序列号与必填身份字段。诸如 token、提示、API 密钥、cookie、密钥、记录稿与消息内容等敏感载荷键会被替换为 `[REDACTED]`，行中记录哪些键被脱敏。

## 重放

运行聚焦的确定性重放：

```bash
rch exec -- cargo test --test e2e_swarm_flight_recorder -- --exact multi_agent_flight_recorder_bundle_replays_without_credentials --nocapture
```

报告嵌入不带 `rch exec --` 前缀的相同 cargo test 命令，以便本地制品读取器可看到底层重放目标。在本仓库中智能体仍须对 CPU 密集型验证使用 `rch exec --`。

## 该 Harness 证明了什么

- 多个 Pi 会话可在单一场景中针对隔离的临时工作区运行。
- 内置工具执行是真实的，而非合成夹具。
- 会话持久化通过真实的 `Session` 状态得到演练。
- JS 扩展生命周期钩子可观察智能体、轮次、工具调用与工具结果活动。
- Agent Mail 或其他协调失败可作为非阻塞标记被捕获，同时 Beads 仍作为软锁回退。
- 摘要报告从事件包中识别主导的实测延迟贡献者与协调失败。
