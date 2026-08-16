# custom vs 上游改动清单

> 生成时间: 2026-08-16
> 基准: `main...custom`(custom 相对上游 0.1.22 快照的全部改动)
> 用途: 合并上游前的情报底稿;理解二开面

## 总览

| 指标 | 数值 |
|---|---|
| custom 总 commit | 164 个(含 1 个 merge upstream) |
| 改动文件总数 | 306 (+58,275 / -19,783 行) |
| 其中真源码(src/ + crates/) | **82 个** |
| 测试/快照/生成物 | 136 个 |
| vendor/ | 26 个 |
| docs/ | 37 个 |
| 其他(配置/脚本/CI) | 25 个 |

**结论:二开本质是"改面广,但源码增量集中"** —— 源码 82 个文件里,相当一部分是 3 次大手笔重构(workspace 拆分、tools.rs 模块化、clippy 清扫)带来的"路过式修改"。

## 功能域分类

### 1. 扩展系统(最大域,~20 commit)

上游扩展机制的深度定制,包括:

- **manifest-aware 加载**:扩展入口按 manifest 加载,修复 sibling-index 误发现,混合包收集 root extension(#35 系列)
- **QuickJS 层持久化**:`node:fs` 的 renameSync/unlinkSync/rmSync/rmdirSync、writeFileSync/appendFileSync 真正落到真实文件系统
- **执行语义**:exec hostcall 通过 AMAC 决策真正交错(#47);abort 信号桥接到 QuickJS 扩展工具;`replaceInput` hook 支持参数重写
- **API 暴露**:`ctx.modelRegistry.getAll()/getAvailable()/getProviderNames()`;execute_command 传递 hasUI 上下文;run_extension_command 总是发 agent_end
- **drop-in 模式**:legacy runner 不可用时 fail closed,诚实降级
- **适配**:support current Pi package APIs、平台 sibling 目录排除

关键文件:`src/extensions.rs`、`src/extensions_js.rs`、`src/extension_dispatcher.rs`、`src/extension_tools.rs`、`src/extension_events.rs`、`src/package_manager.rs`

### 2. 工具系统(src/tools/ 全家)

- **tools.toml 外部化**:内置工具描述与参数通过 tools.toml 暴露/覆盖(#49),支持 tool_overrides
- **移除 CWD 路径限制**:grep/ls/find/read/write/edit 不再被 cwd 约束
- **verify 验证系统**(全新子系统):edit 后轻量验证;table-driven external checkers;npx.cmd shim(Windows);prettier direct-call;null stdin 防 cmd-shim 挂死;超时杀进程树;支持 verify .md
- **pwsh 内置工具**:输出截断、stderr 分离、PATH 空格截断修复、UTF-8
- **read 工具**:cache key 修复(排除 offset/limit/head/tail/hashline)
- 上限 1000→10000

关键文件:`src/tools/mod.rs`、`src/tools/verify.rs`(全新)、`src/tools/pwsh.rs`、`src/tools/read.rs`、`src/tools/edit.rs`、`src/tools/hashline.rs`、`src/tools/bash.rs`、`src/tool_overrides.rs`

### 3. RPC 层与会话持久化

- **新增端点**:get_system_prompt(可带 tools)、get_tree、get_version、estimate_tokens、queue 管理(remove_from_queue/clear_queue/get_queue + queue_update 事件)、append_custom_entry
- **持久化**:进程侧主动会话持久化(RpcSessionPersister),去掉每条消息 fsync、仅 AgentEnd 落盘;Windows 文件竞争重试
- **健壮性**:截断 SSE 分类为 transient 自动重试;RPC persister 链根修复

关键文件:`src/rpc.rs`、`src/session.rs`、`src/resource_governor.rs`、`src/compaction.rs`

### 4. 模型/提供商层

- **gemini 思考链**:thinkingConfig 发送 + thought part 接收(带实测证据)
- **DeepSeek 方言**:honor compat.thinkingFormat,修复 opencode-go 空思考
- **reasoning_effort**:全 OpenAI-compatible 提供商支持
- **xhigh**:supports_xhigh 检查 compat.thinkingLevelMap;session_state 暴露 thinkingLevelMap
- **persist 参数**:set_model/set_thinking_level 支持持久化
- opencode-go provider 新增

关键文件:`src/providers/*`(openai.rs、gemini.rs、vertex.rs 为主)、`src/models.rs`、`src/model.rs`

### 5. 进程/执行/abort

- **ProcessGuard + wait_with_cancellation**:修复 abort RPC 后 pwsh 子进程孤儿(#24)
- **abort 全链路**:Phase A-D 终止工具进程;abort/error 时清理 dangling tool call
- **AMAC exec interleave**:exec hostcall 通过 AMAC 决策交错执行(#47)
- fsync 拒绝容忍(virtiofs/FUSE)

关键文件:`src/abort.rs`、`src/subprocess_handle.rs`、`src/hostcall_amac.rs`、`src/agent.rs`(部分)

### 6. TUI / 交互 / print

- 流式帧间残留字符清除;Windows Terminal 全屏闪烁修复
- print 模式会话持久化(--session-dir/--session)
- get_system_prompt RPC 返回 tools

关键文件:`src/interactive*`、`src/theme.rs`

### 7. 配置 / 启动 / 平台

- **SYSTEM.md**:项目级 .pi/SYSTEM.md,优先级高于用户级
- **技能模式**:skill_mode=project_only 跳过全局技能;global_skills 白名单;project_only 与 global_skills 叠加过滤
- 临时目录注入 system prompt;disabledTools 配置;中文本地化
- Windows:doctor/sdk 平台修复、bash 修复

关键文件:`src/config.rs`、`src/app.rs`、`src/cli.rs`、`src/main.rs`、`src/auth.rs`、`src/package_manager.rs`

### 8. 架构重构(基础性,牵涉面广)

- **workspace 拆分**:src → pi-core + pi-provider-core 双 crate(6667784b),大量文件"搬家式"修改
- **tools.rs 模块化**:拆成 src/tools/ 目录(985c3962)
- **clippy 清扫**:3 次大规模修 lint(约 70+13 个预存问题)
- 构建:LLD linker + sccache + debug profile opt、balanced release profile、cargo-sweep

### 9. 文档与工程化

- docs/context 体系重构(AGENTS.md 瘦身、tables→lists、中文化)
- .githooks/post-commit 自动推送;LF 行尾统一(.gitattributes);dependabot 禁用
- deploy-release.ps1 部署脚本
- 版本号 bump 系列(0.1.22 → 0.1.70)

## 合并冲突风险地图

### 高冲突风险:两边都动过的文件(52 个)

| 域 | 文件数 | 代表文件 |
|---|---|---|
| providers | 9 | openai.rs, gemini.rs, vertex.rs, anthropic.rs... |
| interactive | 8 | commands.rs, ext_session.rs, tree_ui.rs... |
| 核心编排 | 7 | agent.rs, rpc.rs, sdk.rs, acp.rs, app.rs |
| 扩展 | 6 | extensions.rs, extensions_js.rs, package_manager.rs... |
| 配置/启动 | 5 | config.rs, cli.rs, main.rs, auth.rs |
| 会话 | 4 | session.rs, session_sqlite.rs, session_store_v2.rs |

### 低风险:custom 独有改动(31 个 src/crates 文件)

上游没碰过,合并时基本自动通过:
`src/tools/`(verify.rs、pwsh.rs、read.rs、edit.rs 等大部分)、`src/tool_overrides.rs`、`src/error.rs`、`crates/pi-core/src/*`(大部分)

### 合并工作量预估

- **必人工判断**:52 个交集文件(但多数是小区域冲突,可快速解)
- **批量处理**:tests/ 快照(96 个 .snap/.jsonl)+ 生成物(57 个 .json)→ 重新生成,不手工 merge
- **核心难点**:`src/agent.rs`、`src/rpc.rs`、`src/extensions.rs` —— 两边都高频改动,且语义可能重叠(如 abort/exec 相关上游也有改动)

## 合并建议

1. **先做这次大合并**(52 个交集文件一次清),之后按 Release 节点高频小步(上游 0.1.x 节奏),把每次合并的增量控制在个位数文件
2. **merge 而非 cherry-pick**:上游 200+ commit 相互依赖,且你的改动是"改造式"(不是"叠加式"),三方合并才能保留语义;cherry-pick 只用于上游孤立的安全修复
3. **生成物冲突一律以本地为准 + 重跑生成**:tests/ 快照、evidence 目录、golden corpus 不手工解
4. 合并顺序建议:先解 52 个交集中的 providers → interactive → 扩展 → 核心编排(由易到难)
