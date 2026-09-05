# 上游同步工作区

> 这是 `docs/upstream/` 的当前入口。用于在低上下文成本下恢复上游同步状态；不替代 Git 历史，也不复制源码 diff。

## 当前结论

- `main` 是上游镜像基线：先同步 `upstream/main`，再把 `main` 合入 `custom`。
- `custom` 是 Fork 的二开主线；当前大跨度同步不直接落到 `custom`。
- 合并采用临时 integration 线，按 `merge-checkpoints.md` 中的累计 checkpoint 逐阶段推进。
- 每个阶段先由 Git 完成 merge，再按真实冲突和验证失败决定是否需要模型。
- 阶段失败可以停止；不得为了追平上游而继续叠加未知变化。

## 当前快照

- 分叉基线：`226a876425a856f657b2a5d7c7ac6f0ca1ad25f1`
- `custom` 当前提交：`d1786d8b9`，已推送到 `origin/custom`
- `main` 当前提交：`195ca9464`，已在本地合入最新 `upstream/main`，尚未推送到 `origin/main`
- `upstream/main` 当前提交：`e403485b3`
- 相对共同基线：`custom` 独有 330 个提交；`main` 独有 1303 个提交
- `custom` 与 `main` 的净差异：657 个文件；两侧共同改动 229 个文件，其中 `src/` 交集 87 个

这些数字是同步前的机器快照，不应被当作永久事实；下一次同步前重新计算。

## 固定边界

- `main` 不运行上游机器人。
- 上游同步不得把会自动运行的上游 bot 配置或相关自动化无条件带入 `main`。
- 尤其保持 `.github/dependabot.yml` 中的 Dependabot 更新项禁用。
- 这条 Fork 政策记录在 `custom` 分支的 `AGENTS.md`，不写入 `main` 的上游规则文件。
- 未经用户明确授权，不启用或合入上游 Dependabot 配置。
- 上述政策文件属于合并时的 Fork 约束，不是普通源码冲突。

## 工作流

```text
upstream/main → main
                  ↓
              临时 integration 线
                  ↓
              C1 → C2 → ... → C6
                  ↓
              验证通过后再合回 custom
```

integration 线只用于本次同步，不作为长期产品分支。每个阶段应记录：

- 上游 checkpoint 和提交范围；
- 实际冲突文件；
- 保留 custom 的区域；
- 采纳 upstream 的区域；
- 冻结或延期的区域；
- 针对性验证命令和结果。

## 文档路由

优先读取：

- `merge-checkpoints.md`：当前 checkpoint、区间规模和阶段状态。
- `fork-merge-sop.md`：改造式 Fork 的长期合并 SOP；其中部分数量和案例是历史快照。
- `semantic-map.md`：旧基线下的高风险交集决策底稿，适合作为候选风险索引，不是当前精确冲突清单。

历史快照：

- `custom-changes-inventory.md`：2026-08-16 的 custom 改动清单。
- `upstream-changes-inventory.md`：2026-08-16 的上游改动清单。
- `probe-report-2026-08-29.md`：特定日期的探测报告。

验证与专项材料：

- `known-test-failures.md`：已知测试失败对照资料。
- `upstream-qa-bead-swarm-guide.md`：上游 QA/Beads 流程参考。
- `plan-hub-minimal.md`：特定计划材料。

现有文档暂不改写；当前入口中的快照标注用于防止把旧数字误当作当前状态。

## 下一步

1. 从 `custom` 创建临时 integration 线。
2. 只合入 C1：`v0.3.0` 对应的 `e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`。
3. 先处理 Git 能确定的内容，不提前让模型读取全量历史。
4. 统计真实冲突，再决定针对性验证和模型上下文。
5. C1 验证通过后，才进入 C2。

> 当前文档只记录计划和事实，不表示任何 checkpoint 已经合入 `custom`。
