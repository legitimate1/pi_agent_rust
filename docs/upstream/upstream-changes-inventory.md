# 上游 0.1.22 → 最新(212 commits)改动清单

> 生成时间: 2026-08-16
> 基准: `92e5884a..upstream/main`
> 用途: 判断是否值得合并进 custom

## 总览

| 指标 | 数值 |
|---|---|
| commit 总数 | 212 |
| 类型分布 | fix 62, chore 38, feat 37, test 22, docs 22, beads 8, style 6, refactor 6, 其余 11 |
| 版本跨度 | 0.1.22 → **0.2.0**(含 v0.2.0 release,大量打包/认证/发布加固) |

## 上游在做的事(按主题聚类)

### 1. **extensions 扩展系统大重构**(最大域,~50 commit)
- `refactor(extensions): de-monolith behind stable façade (#130)`, `extract extension compatibility contracts (#130)`
- **新增 src/extensions/ 目录**:compatibility.rs、protocol.rs、extension_manager_impl.rs、fs_connector.rs、exec_mediation.rs、native_runtime_experimental.rs、permission_drift.rs、event_coalescer_impl.rs
- 大量行为修复:目录聚类、manifest-aware、scanner 兼容、UI 超时、wasm 能力
- **这是对你影响最大的域**——你的 custom 也大改了 extensions(manifest-aware、fs 持久化、abort 桥接),上游这次是**结构性重构**,冲突会非常深

### 2. **models 目录 v2 + 认证体系**(~25 commit)
- `feat(models): provenance-bearing fetched catalog v2 and credential order`
- `models.fetched.json` 加载优先级、fetch/persist/refresh CLI、credential resolver 注入、Bearer 修复、thinkingLevelMap/thinkingFormat 尊重(gh #166)、GPT-5.6 家族目录种子
- 与你的 gemini 思考链、DeepSeek 方言、xhigh 改动**高度重叠**

### 3. **v0.2.0 发布加固**(~40 commit)
- LZSS 编译期嵌入、gzip 大文本资产、MSVC 链接预检、安装器加固、release 证据门、认证门、performance-claim 门
- 大量 `[skip actions]` 标记,几乎全是发布管道

### 4. **session/session-store 硬化**(~15 commit)
- Windows 工件目录 pin、durable clean/dirty 状态、fsync 拒绝容忍、file_lock 心跳续期、fsqlite 0.3.2 迁移计划
- 与你的 RPC 会话持久化、Windows 竞争重试**有重叠**

### 5. **subagents / roles / TUI**(~15 commit)
- `feat(tui): /model roles, cheap auto-titling, task-role subagents`
- role_model_spec 线程化、结构化子代理结果、子代理结果 fence 转义修复
- 你的 custom 没大动 subagents,重叠小

### 6. **新工具:ast_grep / ast_edit**(~10 commit)
- `feat(tools): add ast_grep and ast_edit structural tools (bd-cv653.1.3)`
- 全新功能,与你无关(不冲突,但值得要)

### 7. **compaction / prompt-cache / provider 细节**(~20 commit)
- `refactor(compaction): force_local_compaction_if_oversized only needs &self`
- Anthropic 1h prompt-cache TTL、默认 short retention、缓存最后 user block
- 与你的 compaction 改动**中度重叠**

### 8. **pijs VFS 隔离**(~10 commit)
- `fix(pijs): end-to-end VFS isolation, bridge secrets, owner-isolated shards`
- QuickJS 层 VFS 隔离、FS escape 修复——**与你的 extensions_js.rs fs 持久化改动强相关**

## 与 custom 的重叠分析

| 域 | 上游改动 | custom 改动 | 冲突风险 |
|---|---|---|---|
| extensions | 结构性重构(新目录+协议) | manifest/fs/abort 定制 | 🔴 **极高** |
| models/providers | 目录 v2、credential、thinking 尊重 | 思考链、DeepSeek、xhigh | 🔴 **高** |
| session/session-store | Windows pin、状态硬化 | 持久化、竞争重试 | 🟠 **中** |
| compaction | 多处重构 | 少量 | 🟠 **中** |
| pijs/extensions_js | VFS 隔离 | fs 持久化 | 🟠 **中** |
| subagents/TUI | role 系统、auto-titling | 少量 | 🟢 低 |
| ast_grep/ast_edit | 全新 | 无 | 🟢 低(白拿) |
| v0.2.0 发布加固 | 管道 | 无关 | 🟢 低(但会带来大量测试/证据文件冲突) |

## 值得合的部分(强烈推荐)

1. **ast_grep / ast_edit 工具** —— 全新能力,零冲突
2. **security(auth) 加固** —— 安全相关,`bound auth I/O, lock timeouts, fail-closed loads`,上游专门的安全 commit
3. **models catalog v2 的 thinking 尊重**(gh #166 系列)—— 与你 xhigh/思考链互补
4. **fsync 拒绝容忍 / file_lock 心跳** —— 稳定性修复
5. **extensions 的 bug 修复**(聚类、scanner、UI 超时)—— 不是重构部分的话很值
6. **Windows 工件 pin / session 硬化** —— 你在 Windows 上二开,直接受益

## 可能不值得合的部分

1. **v0.2.0 发布加固全家桶**(LZSS、gzip 嵌入、MSVC 预检、release 证据门)—— 这是"上游要发版"的管道,你二开用不到,却会带来大量证据/测试文件冲突
2. **OMP-ADOPT 史诗**(beads,~50 commit)—— 上游在把 oh-my-pi 能力移植进来,量大且与你的二开无关,可以等它稳定
3. **性能 claim 门 / certification 门** —— 纯上游发布流程

## 结论

**值得合,但要挑着合** —— 不是一次性吞 212 个。

- **必合**:安全修复、bug 修复、新工具、thinking 尊重、Windows 修复(约 60-80 commit,分布在 extensions/models/session 域)
- **可选**:extensions 重构(v0.2.0 的新架构,合了能少走弯路,但冲突最深)
- **缓合**:发布管道、OMP-ADOPT、证据/认证体系(等上游稳定再合,或干脆不合)

**最难的抉择在 extensions 域**:上游把它从单体拆成多文件结构(stable façade),你的 custom 深度定制了旧结构。**合 = 把定制迁移到新架构,一次大手术;不合 = 每次上游动 extensions 都冲突,且拿不到新能力。** 我的判断:这个域值得合,但应该单独规划,作为"extensions 架构迁移"项目,而不是塞进一次普通 merge。

## 建议路径(修正版)

```
阶段1: 快赢合并 (低风险)
   cherry-pick 或分域 merge: 安全修复 + bug 修复 + ast 工具 + thinking 尊重 + Windows 修复
   预期冲突: 低-中

阶段2: extensions 架构迁移 (单独项目)
   先读上游新架构,再决定 custom 定制如何挂到新架构
   预期冲突: 高,但可规划

阶段3: 高频小步
   之后每周 merge,只处理增量
```

**不推荐一次性大 merge**(212 个),因为上游 v0.2.0 的发布管道和 OMP-ADOPT 你不需要,却会引入几百个测试/证据文件冲突,纯噪音。
