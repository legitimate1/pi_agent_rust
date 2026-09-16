# 上游分析窗口

> 本文定义当前上游研究窗口的边界，不表示已经把分析终点合入 `custom`。
> 更新日期：2026-09-16

## 当前窗口

本轮采用“两层边界”：

```text
宏观分析窗口：
S ─────────────────────────────► v0.3.0

第一执行子窗口：
S ─► fsqlite session storage cutover
```

宏观终点用于观察第一个完整上游演进周期的净变化；执行子窗口用于控制第一次实际语义分析的规模。两者都不是自动 merge 范围。

## 固定边界

```text
上游起点 S：226a876425a856f657b2a5d7c7ac6f0ca1ad25f1
起点含义：custom 与 upstream/main 的共同祖先，也是 custom 上次上游同步所对应的上游父提交

宏观终点：v0.3.0
宏观终点 SHA：e23c4622f8bc4038a5e061ee3640a0e9206ec5cc
宏观终点用途：建立第一个 minor/release 周期的观察边界

第一执行子窗口终点 T1：8f12352e174ea06c7d8d66cde15768cecfccccf3
T1 标题：chore(beads): close bd-oc1wu (fsqlite cutover landed); file bd-2ohw2 size-reclaim follow-up
T1 用途：分析 fsqlite session storage 后端替换闭包
```

`S..T1` 包含 16 个提交、69 个变更文件，净变化为 `+3412 / -1119`。这 16 个提交中只有一部分直接属于 session storage；其他工具、扩展、压缩、测试和文档变化不因处于同一时间区间就自动纳入第一波。

## 当前引用快照

本次 Wave 3 候选分析在刷新远程引用后的本地快照：

- 当前工作分支：`custom`
- 当前 `custom`：`6d65193a628dabe1f3136bd76bc4a9e998f302db`
- `main`：`195ca9464101c4862607c909951bf9467baf245c`
- `origin/main`：`fffc80db497ddb755c7af23052c6c553ab598ac4`
- `origin/custom`：`6d65193a628dabe1f3136bd76bc4a9e998f302db`
- 刷新后的本地 `upstream/main`：`c32541d0bdf431f7fdfe5a7b628662b05b4b7ab1`
- 本地 `v0.3.0` annotated tag：`ecdb899c573bce7a0cd3e2168d2d33e07e865998`
- `v0.3.0^{}`：`e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`

本次执行了 `git fetch --prune upstream main`、`git fetch --prune origin main custom` 和 `git fetch --prune --tags upstream`。`upstream/main` 相对文档原快照 `e403485b3116e6c97e9af7026ec9445f30312c7d` 已前进；固定的 `S`、`T1` 和 `v0.3.0` 对象仍存在，`S..v0.3.0` 仍可复核。刷新只移动远程跟踪引用，没有切换分支或合并上游。

这些 SHA 只说明本次分析使用的 Git 对象。上游分支会移动，后续研究前仍需重新读取并重新固定终点；本次 Wave 3 仍以固定 `v0.3.0` 为边界，不自动扩展到当前 `upstream/main`。

## 为什么先用 T1，而不是直接分析 v0.3.0

`S..v0.3.0` 已有约 302 个 first-parent 提交和 351 个变更文件，适合做宏观地图，不适合作为一个语义实现批次。

`T1` 在第一个明确的 session storage 后端切换闭包处收束，且早于 FTUI 大规模迁移，能够先验证“最终净变化 → 功能闭包 → custom 对照 → 处理结论”的分析流程。

## 分析方法

```text
固定 S 与宏观终点、执行子窗口终点
        ↓
读取执行窗口的最终净差异
        ↓
识别真正的功能闭包
        ↓
只对闭包相关提交回看历史
        ↓
与当前 custom 实现对照
        ↓
决定采纳、适配、冻结、排除或无需迁移
        ↓
把结论写入 waves/<wave-name>.md
```

最终 diff 用来回答“终点最终留下了什么”；定向历史用来回答“为什么这样变化、哪些提交必须一起理解”。不把提交数量、文件数量或同日提交自动当作语义边界。

## 下一步边界

第一执行子窗口已经完成分析，结论见：

```text
waves/01-fsqlite-session-storage.md
```

结论不是直接 merge，而是将 fsqlite 后端替换记录为独立迁移议题。后续若继续处理，应先创建独立的 SQLite 后端迁移波次；在此之前不把 `S..T1` 或 `S..v0.3.0` 作为全量 merge 范围。

当前后续执行边界：

```text
Wave 3：tokensAfter compaction result contract
固定范围：129cf9fe88598439b4717b17002d6110266c03b3^..129cf9fe88598439b4717b17002d6110266c03b3
状态：已完成只读分析，等待用户决定是否进入实现设计
```

Wave 3 仍不改变宏观窗口终点，也不自动吸收当前 `upstream/main` 在 `v0.3.0` 之后的变化。
