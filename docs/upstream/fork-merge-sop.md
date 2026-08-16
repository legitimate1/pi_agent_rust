# Fork 追上游合并 SOP(改造式 fork 专用)

> 来源:pi_agent_rust 2026-08-16 快赢合并实战
> 适用:custom 深度二次开发(结构重构过)+ 需要追上游的场景

## 核心结论(实战验证)

1. **cherry-pick 在改造式 fork 上不可行**——custom 拆分过文件结构后,上游 commit 的改动锚点已不存在,即使"单文件独立修复"也会冲突(实测 9 个全部失败)
2. **merge 是唯一可行载体**——但必须"选择性采纳",不是一次性吞全部
3. **依赖漂移是最大隐藏成本**——上游依赖升级(digest/swc 等)会自动合入 Cargo.toml,custom 代码未适配 → 几百个编译错误
4. **生成物/测试冲突是纸老虎**——批量 --ours + 重跑生成即可,不手工解

## 标准流程

### 阶段 0:决策情报(纯读,零风险)

```
1. 生成 custom 改动清单     → git diff main...custom 分类(功能/适配/测试)
2. 生成上游改动清单         → git log 92e5884a..upstream/main 按主题聚类
3. 语义地图                → 52 个交集文件对比双方改动,分"重复/互补/唯一"
4. 判断合并价值            → 值得合(安全修复/新工具/你缺的修复)vs 缓合(大重构)vs 不合(发布管道)
```

**决策原则:挑着合,不一次吞。** 上游的发布管道、内部工具(beads)、无关史诗,合并只会带来噪音冲突。

### 阶段 1:试 merge(实验,零风险)

```bash
git worktree add --detach /tmp/trial-wt custom   # 必须 worktree!主仓库有 index 幽灵问题
cd /tmp/trial-wt
git checkout -b trial-merge
git merge --no-commit --no-ff upstream/main       # 只看冲突,不提交
git diff --name-only --diff-filter=U              # 冲突清单
```

**验证:冲突数 vs 语义地图预测是否一致,决定值不值得正式合。**

### 阶段 2:正式 merge(执行)

```bash
git worktree add --detach /tmp/merge-wt custom
cd /tmp/merge-wt && git checkout -b merge-non-ext
git merge --no-commit --no-ff upstream/main
```

**冲突分类处理(按优先级):**

| 类型 | 处理 | 依据 |
|---|---|---|
| tests/ 快照、docs/ 生成物 | `git checkout --ours` 批量 | 生成物重跑再生 |
| 上游内部数据(.beads/) | `git rm -rf` 删除 | 无关噪音 |
| 冻结域(extensions 重构等) | `git rm -rf` 上游新文件 + `--ours` 单文件 | 单独规划迁移 |
| 根配置(Cargo.toml 等) | 人工合并 | 版本号保留 custom,依赖看情况 |
| 语义重复(两边实现同一功能) | 留一,优先上游官方实现 | 语义地图 |
| 语义重叠 | 逐冲突人工解 | 保留 custom 语义为主 |
| custom 独有功能 | `--ours` 保留 | 你的定制 |

**依赖漂移陷阱(Cargo.toml):**
- 上游依赖升级会**自动合入**(无冲突!),custom 代码用旧 API → 几百编译错误
- 决策:主依赖回退 custom 版(`git checkout HEAD -- Cargo.toml Cargo.lock`),dev-deps 跟上游
- build.rs 同理:上游 build.rs 引用新 crate → 回退 custom 版
- 引用新 API 的自动合入文件(如 embedded_assets)→ 回退 + 删除

**编译修复循环:**
```bash
cargo check --all-targets
# 报错 → 判断是 merge 引入还是 custom 基线自带(基线自带需单独修)
# 自动合入文件引用新 API → 回退该文件到 HEAD
# 反复直到 Finished
```

### 阶段 3:验证与落地

```bash
cargo check --all-targets                          # 必须全绿
git commit -m "merge: upstream/main into custom (快赢合并)"
# 主仓库:
git merge --ff-only merge-non-ext                  # custom 快进
git push origin custom
git worktree remove /tmp/merge-wt --force
git branch -D merge-non-ext
```

## 冻结清单模板(记录本次没合的东西)

```
冻结 (TODO 迁移):
- src/extensions/ 重构 (de-monolith, 待架构迁移)
- 上游依赖升级 (digest 0.11/swc 26 等, custom 代码未适配)
- v0.2.0 发布管道 (embedded_assets/LZSS/认证门)
- subagent roles/TUI 新特性
- src/tools.rs 单文件改动 (custom 已拆分结构)
```

每次合并后更新,冻结项积累到一定程度单独做"迁移项目"。

## 关键教训

1. **先试 merge 再正式合**——merge-tree 预演的数据会骗人(248 vs 实际 83),真实冲突以试 merge 为准
2. **worktree 是唯一安全环境**——主仓库的 index 幽灵问题(checkout 被拒)用 worktree 绕开
3. **Windows 反斜杠坑**:.cargo/config.toml 的 `\\` 转义,heredoc/JSON/正则三层都会吞反斜杠,用 python bytes 级写入 + 字节数验证
4. **post-commit hook 自动推送**:在 worktree 提交会推到新分支,注意清理
5. **基线验证先行**:合并前先确认 custom HEAD 本身 `--all-targets` 能过,否则把基线错误误判为 merge 引入
6. **依赖漂移 > 源码冲突**:源码冲突 83 个只花 1 小时,依赖漂移 423 个错误花了 3 小时——Cargo.toml 的决策要快(回退是默认)

## 后续高频小步节奏

- 每周 `git merge upstream/main`,冲突应该控制在个位数(冻结面之外)
- 冻结面(extensions 等)在迁移完成前,每次 merge 用同样的"冻结"手法
- 上游发 0.2.x release 时,评估是否值得做 extensions 架构迁移(独立项目)
