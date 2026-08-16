---
version: 1
created: 2026-08-16
updated: 2026-08-16
aliases: []
tags: [upstream, swarm, beads, qa, agent-army]
---
## 上游 QA/bead/swarm 工作流门禁体系解读

> 目的: 读懂上游(pi_agent_rust 原作者)的 QA/bead/swarm 门禁体系 —— 它是什么、为什么存在、哪些值得二开保留、以及如何借鉴它来组建自己的 Agent 大军。
> 阅读前提: 已通读 2026-08-16 追上游复盘 + 快赢合并清单。

---

### 一、一句话总结

**上游不是"一个人写代码",而是一支 AI agent 大军在并行开发同一个仓库。这套体系是为"多 agent 并行 + 远程构建 + 证据化收尾"设计的治理层。** 二开是单人作战,所以天然不匹配 —— 但其中的**证据化收尾**与**任务台账**哲学,是单人也能(且应该)借鉴的工程纪律。

---

### 二、全景:四层体系

```
┌─────────────────────────────────────────────────────┐
│  ① Beads(任务台账)     —— 一切工作的源头             │
│      br CLI + .beads/issues.jsonl                    │
│      "每个任务一个 bead,关闭必须带证据"               │
├─────────────────────────────────────────────────────┤
│  ② 协调层(并行不打架)                               │
│      Agent Mail: MCP 协议,消息 + 文件预约(reservation)│
│      RCH: 远程构建队列,CPU 重活统一排队               │
├─────────────────────────────────────────────────────┤
│  ③ 证据层(出了事能复盘)                             │
│      activity ledger: 脱敏 JSONL 事件流              │
│      flight recorder: 确定性 E2E 证据                │
│      replay: 离线重放分析                            │
│      capacity planner: 资源准入                      │
├─────────────────────────────────────────────────────┤
│  ④ QA 门禁(收尾必须证明自己)                        │
│      契约(contract JSON) + 证据(evidence JSON)      │
│      + 测试(三对齐检查) + testing-policy(宪法)       │
└─────────────────────────────────────────────────────┘
```

**核心哲学:claim-integrity(声明完整性)** —— 任何"我完成了 X"的声明,必须附**直接证据**(命令输出、产物、git commit、beads 记录),拒绝叙述性证据("我觉得好了"不算)。这是整套体系的灵魂。

---

### 三、一次典型生命周期(从任务到收尾)

```
1. 创建 bead        br create --title "实现 X" --priority 0
                    → 写入 .beads/issues.jsonl

2. 认领任务          br update <id> --claim --actor "agent-7"
                    → 防止多 agent 抢同一任务

3. 预约文件          Agent Mail file_reservation_paths
                    → 防止两个 agent 改同一文件

4. 干活 + 验证       本地改代码 → rch exec -- cargo check
                    → 编译重活进远程队列,不阻塞别人

5. 记录证据          activity ledger 记录每个动作(脱敏)
                    flight recorder 记录 E2E 运行

6. 收尾关 bead       必须有: 验证命令 + 证据文件 + commit
                    + close_reason + 契约检查通过
                    → 防止"看起来完成了"的幻觉

7. 门禁三对齐        契约(要求) ↔ 证据(结果) ↔ README(挂链接)
                    + testing-policy(分类正确)
                    → 任一不对齐,门禁测试红
```

---

### 四、关键文件地图(速查)

| 文件 | 作用 | 二开要用吗 |
|---|---|---|
| `docs/testing-policy.md` | 测试宪法:suite 分类(unit/vcr/e2e/certification)+ 契约 | ✅ 保留(通用纪律) |
| `tests/suite_classification.toml` | 每个测试文件必须在且只在 1 个 suite | ✅ 保留 |
| `docs/qa-runbook.md` | 失败排查手册:签名→重放命令 | ✅ 保留 |
| `docs/contracts/*.json`(48 个) | 每个收尾门禁的"要求定义" | ⚠️ 大部分是上游特定,挑通用保留 |
| `docs/evidence/*.json` | 每个门禁的"结果证据" | ⚠️ 二开不生成,维护成本高 |
| `docs/swarm-operations-runbook.md`(1439 行) | 多 agent 操作手册 | ❌ 二开用不上(单人) |
| `docs/swarm-activity-ledger.md` | 脱敏事件账本 | ❌ 同上 |
| `docs/swarm-flight-recorder.md` | E2E 证据回放 | ❌ 同上 |
| `docs/swarm-replay-operator-workflow.md` | 离线重放 | ❌ 同上 |
| `docs/context-intelligence.md` | 语义工作区图 | ⚠️ 二开有简化版,可借鉴 |
| `docs/ci-operator-runbook.md` | CI 故障签名→重放 | ✅ 保留(通用) |
| `docs/conformance-operator-playbook.md` | 扩展一致性测试 | ⚠️ 二开用得到(有 ext 测试) |

---

### 五、门禁测试为什么红(对 B 类 24 个的根源解释)

| 失败类型 | 根源 | 修法 |
|---|---|---|
| README 缺链接类(6 个) | 上游每个新契约都要在 README 挂链接,custom 的 README 没挂全 | 补链接(纯文档,零风险) |
| 证据文件缺失(3 个) | replay_bundle.json 等由脚本生成,上游定期重生成,二开没跑过 | 重生成 |
| VCR cassette(4 个) | 上游录的请求快照,行为变了要重录 | VCR_MODE=record 重录 |
| Gemini 锁 8192(2 个) | 上游故意锁死默认值防漂移(backward lock),二开代码用 65536 | 决策:改代码 or 改锁 |
| fs shim 等(9 个) | 扩展沙箱行为测试,与二开改动有关 | 逐个诊断 |
| ext conformance(4 个) | 合并带入的 vendor 资产 checksum 未同步 | 同步 checksum |

---

### 六、对二开的价值评估(保留 / 降级 / 排除)

#### ✅ 值得保留(通用工程质量,与单人/多 agent 无关)

1. **suite_classification + testing-policy** —— 测试分类清晰,新测试必登记,防"孤儿测试"
2. **backward lock 哲学** —— 关键默认值(如 maxOutputTokens)锁死防漂移,改要显式决策
3. **契约↔证据↔README 三对齐** —— 文档与实现不脱节,单人也有价值
4. **fail-closed 原则** —— 证据缺失就报错,不乐观放行(适合任何规模)

#### ⚠️ 降级使用(理解但不全量跑)

1. **48 个契约 + 证据体系** —— 二开不跑上游流程,不用全量维护;保留与二开核心相关的几个(如 dropin 认证、ext 兼容),其余列入"已知失败"清单
2. **VCR cassette** —— 有价值(离线重放),但维护成本高;二开可选择性重录
3. **context-intelligence** —— 简化版够用

#### ❌ 排除(上游多 agent 专属)

1. **swarm 全家桶**(activity ledger / flight recorder / replay / capacity planner / admission controller)—— 单人不需要
2. **Agent Mail / RCH** —— 单人不需要协调;但**组建 Agent 大军时**参考
3. **runpack / handoff bundle** —— 多 agent 交接用

---

### 七、Agent 大军落地路线图(二开怎么用这套思路)

> 核心转变: 从"我 vs 代码"到"我 + N 个 agent 并行 vs 代码"。不是要照搬上游全套,而是**渐进式引入最值钱的几块**。

#### 阶段 0(现在):单人 + 纪律

- 已有:`known-test-failures.md` 对照基准、fork-merge-sop、suite_classification
- 补:修 README 链接类失败(零风险,恢复 6 个)→ 让全量测试有真实意义
- 收益:每次改动后全量测试 = 真实状态,不靠猜

#### 阶段 1(1-2 周):引入任务台账(借鉴 Beads,轻量版)

- 上游 `.beads/issues.jsonl` 被删了,但**任务台账思想**值得捡回来
- 轻量方案:GitHub Issues 或 `docs/tasks/*.md`(每个任务:状态 + 验证命令 + 证据路径)
- 关键纪律:**任务关闭必须带验证命令 + 结果**,复刻 claim-integrity
- 收益:多 agent 并行时,每个 agent 知道"什么算完成";单人也防"假完成"

#### 阶段 2(1 个月):并行 agent + 隔离

- **worktree 隔离**:每个 agent 一个 `git worktree`,共享 target 目录(已验证可行)
- **文件预约思想**(借鉴 Agent Mail):agent 开工前声明改哪些文件,防冲突
- **任务领取**:一个简单清单(谁在做什么),防两个 agent 做同一件事
- 收益:2-4 个 agent 并行改不同模块,编译产物共享,冲突可预见

#### 阶段 3(按需):远程构建队列(借鉴 RCH)

- 前提:并行 agent 编译成为瓶颈
- 轻量方案:一个"编译队列"脚本(谁要编译,排队,串行重活)
- 上游用 rch 做远程队列,二开本地可用简单串行 + 共享 target
- 收益:避免 N 个 agent 同时 cargo build 打爆机器

#### 阶段 4(可选):证据化收尾(全套)

- 每个功能:契约(1 个 JSON 定义要求)+ 证据(1 个 JSON 记录结果)+ 测试(检查对齐)
- 这是上游最值钱的工程纪律,单人也可用(当前 dropin 认证已经在用)
- 收益:代码、文档、测试三者永不脱节;review 有据可查

---

### 八、修基线决策参考(结合本体系)

基于上述理解,修 B 类 24 个的优先级:

| 优先级 | 批次 | 内容 | 理由 |
|---|---|---|---|
| 1 | 文档门禁 | README 链接类 6 个 | 通用纪律,零风险,恢复最多 |
| 2 | 数据重生成 | replay_bundle / runpack / artifact index 3 个 | 通用,重跑脚本即可 |
| 3 | 决策类 | Gemini 锁 2 个 | 需拍板,但影响真实行为 |
| 4 | 诊断类 | fs shim 9 个 | 可能动核心代码,先诊断再定 |
| 5 | 冻结 | swarm 专属门禁(swarm-progress/replay 等) | 与二开无关,记录即可 |
| 6 | 排除 | ext conformance 4 个(合并引入) | 等 extensions 迁移时处理 |

---

### 九、关键教训(浓缩)

1. **上游的 QA 体系是为"多 agent 并行"设计的**,单人硬扛全量测试 = 维护一个不匹配的治理层
2. **claim-integrity 是最值钱的哲学**:任何"完成"必须有直接证据 —— 单人也能用
3. **backward lock**:关键默认值锁死防漂移,改要显式决策 —— 防"静默回归"
4. **三对齐(契约/证据/README)**:文档与实现不脱节 —— 二开最缺的纪律
5. **B 类失败不是 bug,是"上游流程 vs 二开现实"的落差** —— 理解了体系,修起来才有判断力
