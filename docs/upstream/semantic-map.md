# 语义地图:custom × 上游 交集文件冲突决策底稿

> 生成时间: 2026-08-16
> 用途: merge upstream/main 到 custom 时的去重/融合/保留决策表
> 数据: 52 个两边都改过的 src/crates 文件,按改动密度排序

## 一、高密度区(双方都大改,最需要决策)

### 🔴 语义重复(两边实现了同一功能 → merge 时留一)

| 文件 | 上游实现 | custom 实现 | 决策 |
|---|---|---|---|
| `src/providers/openai.rs` | `82630422` DeepSeek thinkingFormat(gh #166) | `a60e67fc` 同名功能 | **留一**:优先上游(gh #166 是官方目录驱动),删 custom 的 |
| `src/models.rs` | `7d10cb18`/`281c2984` thinkingLevelMap 尊重(gh #166) | `d2b26aea` supports_xhigh + thinkingLevelMap | **融合**:上游目录驱动更全面,保留上游 + 补 custom 的 xhigh 特例 |
| `src/session.rs` | `d5d8d276` Bearer 恢复;role 字段;ModelChange | `9df21068` Windows 竞争重试;persist | **互补**:无冲突,各自保留 |
| `src/sdk.rs` | `9ec0ab34` prompt-cache 默认 short;`edf3e894` 会话级扩展提示 | `d3b42f58` persist 参数 | **互补**:无冲突 |
| `src/acp.rs` | `5e20d9df` 推理签名清理 | `d3b42f58` persist;`4d7e872a` clippy | **互补** |

### 🟠 语义重叠区(两边改同一区域但目的不同 → 需人判)

| 文件 | 上游 | custom | 决策 |
|---|---|---|---|
| `src/extensions.rs` | **结构性重构** de-monolith(#130) | manifest/fs/abort 深度定制(21 commit) | **最高风险**。上游把单体拆成 `src/extensions/` 多文件,你的定制锚在旧结构。**决策:先看新架构能否承载你的定制,能则迁,不能则保留旧结构 + 手工合并上游修复** |
| `src/extensions_js.rs` | pijs VFS 隔离(26 commit) | QuickJS fs 持久化(13 commit) | **高风险**。VFS 隔离 vs fs 直写真实系统,语义可能冲突(隔离要挡,你要放)。需逐段看 |
| `src/agent.rs` | compact bridge、ctx seed、compaction(14) | tools.toml、abort、replaceInput(14) | **高风险**。两边都在编排层加钩子,需逐冲突解 |
| `src/rpc.rs` | 新版端点、parity(7) | 8 个自定义端点 + 持久化(22) | **中高**。上游加端点,你加端点,可能有命名/行为重叠 |

### 🟡 单边为主区(一方大改,另一方路过 → 大概率保留主方)

| 文件 | 主方 | 说明 |
|---|---|---|
| `src/models.rs` | 上游(18) | 上游目录 v2 大改,你只有 1 个路过 |
| `src/tools.rs` | 上游(17) | ⚠️ **注意:custom 已拆成 src/tools/ 目录,此文件在 custom 里已被拆分**。上游还在改单文件,merge 时可能整文件冲突,需把上游改动映射到新结构 |
| `src/session_store_v2.rs` | 上游(13) | 上游 v2 状态机大改,你只有 1 个路过 |
| `src/interactive/commands.rs` | 上游(8) | role 命令等,你是 3 个路过 |
| `src/providers/anthropic.rs` | 上游(10) | prompt-cache TTL 等,你 2 个路过 |
| `src/provider_metadata.rs` | 上游(7) | 目录驱动,你 1 个路过 |
| `src/main.rs` | 上游(11) | 版本/发布,你 6 个(版本 bump) |

### 🟢 低频路过区(双方都只动 1-2 次 → 低风险,自动合并为主)

`src/permissions.rs`、`src/error.rs`、`src/doctor.rs`、`src/hostcall_amac.rs`(你 2,上游 1,AMAC 是你的)、`src/interactive/tree_ui.rs`、`src/interactive/share.rs`、`src/resource_governor.rs` 等

## 二、结构性差异提醒(merge 时会出现的特殊冲突)

1. **src/tools.rs 拆分**:custom 拆成 `src/tools/` 目录,上游仍改单文件。上游 17 个 commit 的改动需要映射到新结构,git 重命名检测可能失败 → **这些改动 merge 后需人工迁移**
2. **extensions 拆分**:上游拆 `src/extensions.rs` → `src/extensions/` 目录,你的定制锚在旧单文件 → 同构风险
3. **workspace 拆分**:custom 把部分逻辑移到 crates/pi-core,上游可能改原位置

## 三、merge 决策规则

```
冲突处理优先级:
1. 生成物/测试快照 (.snap/.jsonl/.json) → git checkout --ours,重跑生成
2. 单边为主的源码 → 保留主方(上游重构就收上游,你的定制就收你的)
3. 语义重复 → 留一(优先官方/上游实现,除非你的实现有上游没有的价值)
4. 语义重叠 → 逐冲突人工解,保留双方语义融合
5. 结构性差异 → 最耗时,需人工迁移(工具拆分/扩展拆分)
```

## 四、预估工作量(按决策类型)

| 类型 | 数量 | 耗时 |
|---|---|---|
| 生成物批量 | ~190 | 30min(脚本) |
| 单边为主源码 | ~20 | 1h |
| 语义重复(留一) | ~3 | 30min |
| 语义重叠(人工) | ~10 | 2-3h |
| 结构性迁移(tools.rs 拆分) | 1-2 | 2h |

**总计:一个工作日内可完成。** 高风险集中在 extensions 两个文件 + tools.rs,建议这三个单独规划。
