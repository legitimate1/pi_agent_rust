# C1 功能全景矩阵（v0.3.0 锚点，只分析不合入）

## 职责与边界

- 职责：为 C1 上游同步建立功能级全景索引，供后续 C1 试合并与分域移植决策使用，不做移植实现。
- 不修改源码，不解决 merge 冲突，不推进 integration 线，不改 custom 与 main 分支状态。
- 上游锚点：C1 为 v0.3.0 对应提交 e23c4622f8bc4038a5e061ee3640a0e9206ec5cc。
- 共同基线：226a876425a856f657b2a5d7c7ac6f0ca1ad25f1。
- C1 区间规模（机器快照，来自 merge-checkpoints）：相对共同基线 317 commits，其中 310 非 merge；351 files，118 源码文件；行变化约 +93550 / -4141。
- 证据边界：只用 C1 CHANGELOG v0.3.0 与 v0.2.0 相关段落、C1 区间机器清单摘要、已有 inventories 与 probe 报告中的路径与行数摘要、各功能域首个代表性提交号与 issue 号。未读 317 提交全文，未读全仓源码，未做三方冲突归因。
- 本文档为决策索引，不是合并结论。采纳 / 暂缓 / 冻结 / 需设计均为初步建议，最终以真实冲突与验证为准。
- 表述要求：本文件禁用 Markdown 表格，全部用列表块记录；不复述 CHANGELOG 原文，只做归纳转述。

## 证据等级说明

- 强证据：CHANGELOG 明确给出提交号与 issue 号的功能。
- 中证据：CHANGELOG 有段落但提交号只覆盖代表性提交，后续修复波次未展开。
- 弱证据：仅在 probe 或 inventory 中出现路径与行数，无逐条 beads Why / Design / Acceptance 核对。
- 本次 .beads issue 正文未逐条拉取，只引用 CHANGELOG 与已有文档中已出现的 epic 号与 issue 号，缺口见文末不确定项。

## A 域：LSP / DAP / eval / AST

### A1 lsp 工具

- 功能名称：lsp 工具，子进程语言服务器统一入口
- 解决问题：缺少结构化代码智能入口，跨语言跳转与诊断依赖外部手工操作
- 主要能力：多服务器注册管理，基于 stdio JSON-RPC，重命名与定义与诊断等多操作，配置可叠加默认
- 上游证据：代表性提交 912a4650；归属 v0.3.0 代码智能段
- 对 custom 的初步价值：高，可补齐结构化编辑检索能力，与现有 grep find read 形成互补
- 是否值得独立移植：值得，需单独评估进程管理与配置面
- 依赖的其他功能：bash 执行与进程树管理，工具注册与分级 schema，workspace 约束
- 当前建议：需设计

### A2 debug 工具 DAP

- 功能名称：debug 工具，DAP 调试适配器统一入口
- 解决问题：缺少可编排调试能力，断点与单步与变量检查无法由 agent 驱动
- 主要能力：多操作调试面，适配器自动选择覆盖常见语言栈
- 上游证据：代表性提交 2db8f6b1；归属 v0.3.0 代码智能段
- 对 custom 的初步价值：中高，调试场景明确但使用频率低于 lsp
- 是否值得独立移植：值得，但优先级低于 lsp
- 依赖的其他功能：进程执行与 PTY，工具权限与审批，jobs 管理
- 当前建议：暂缓

### A3 eval 工具

- 功能名称：eval 工具，常驻 Python 与 QuickJS 内核
- 解决问题：一次性脚本执行无状态，跨步计算与中间表示难以复用
- 主要能力：cell 语义，尾表达式求值回显，工具桥接白名单，顶层 await
- 上游证据：代表性提交 ee942ba2；归属 v0.3.0 代码智能段
- 对 custom 的初步价值：高，可加速数据处理与原型验证类任务
- 是否值得独立移植：值得
- 依赖的其他功能：工具沙箱与 allowlist，bash 与文件工具，会话持久化
- 当前建议：需设计

### A4 ast_grep 与 ast_edit

- 功能名称：ast_grep 与 ast_edit 结构化搜索改写
- 解决问题：纯文本搜索改写误伤率高，重构类编辑缺结构锚点
- 主要能力：语法感知搜索与改写，适合批量重构与精确 patch
- 上游证据：代表性提交 1952d084；早期 inventory 已标记为零冲突白拿候选
- 对 custom 的初步价值：高，与 verify 与 edit 链路天然衔接
- 是否值得独立移植：值得，适合做首批小闭包
- 依赖的其他功能：read edit 工具链，工具 schema 分级
- 当前建议：采纳

## B 域：bash / jobs / hub / subagents

### B1 bash 执行加固与 mediation

- 功能名称：bash 生成前分类与 PTY 与后台执行
- 解决问题：危险命令直达执行，isatty 依赖命令失败，长任务阻塞主回合
- 主要能力：生成前按策略分级处置，tty 按需分配，后台执行返回 job 标识
- 上游证据：代表性提交 ca14c3ab，da62aebf，b8e39600；归属 v0.3.0 执行段
- 对 custom 的初步价值：高，custom 有 abort 与进程树定制，正交但易冲突
- 是否值得独立移植：值得，但须先对齐 abort 语义
- 依赖的其他功能：jobs 工具，hub 监督，权限审批，进程回收
- 当前建议：需设计

### B2 jobs 工具

- 功能名称：jobs 后台任务管理
- 解决问题：后台 bash 无统一查询与回收入口
- 主要能力：任务列表与状态与回收，支撑长运行命令
- 上游证据：与 B1 同波次，probe 报告记录 jobs.rs 约五千行规模
- 对 custom 的初步价值：高，是 hub 最小闭包的必要组成
- 是否值得独立移植：值得，但不建议单独拆出，应随 hub 闭包走
- 依赖的其他功能：bash 后台执行，hub，RPC 与 TUI 钩子，会话状态
- 当前建议：暂缓，等待 hub 闭包方案

### B3 hub 监督与任务编排

- 功能名称：hub PTY 服务监督与 readiness 门控
- 解决问题：多任务与常驻服务缺统一监督面
- 主要能力：服务注册与就绪门控，与 jobs 联动
- 上游证据：probe 报告记录 hub.rs 千行级，agent_hub.rs 五百行级；C2 才收束，C1 仅起点
- 对 custom 的初步价值：高，但 C1 阶段尚不完整
- 是否值得独立移植：不值得在 C1 单独移植，应按 C1 加 C2 联合观察
- 依赖的其他功能：jobs，subagents，app cli config rpc session 注入点
- 当前建议：冻结，留待 C2 后再定

### B4 subagents 原生编排

- 功能名称：原生 subagent 编排，第九内置工具形态
- 解决问题：复杂任务缺有界分解执行手段
- 主要能力：单子代理，有界并行，串行链，子进程继承环境与模型与工具表，结构化进度，可取消回收
- 上游证据：v0.2.0 功能段；issue 132 144 145；probe 记录 subagents.rs 两千余行
- 对 custom 的初步价值：高，custom 在该域改动小，重叠低
- 是否值得独立移植：值得，是最小移植候选核心
- 依赖的其他功能：工具执行 arity 变更，模型路由，RPC 与 TUI 进度面，会话与 compaction
- 当前建议：需设计，重点解 Tool execute 参数漂移与依赖增量

## C 域：web / URL / MCP / GitHub / import

### C1 web_search 工具

- 功能名称：web_search 多 rung 搜索链
- 解决问题：外部知识检索单一源脆弱，缺熔断与去重
- 主要能力：keyed 与 keyless 多源排序链，逐 rung 熔断，站点与时间过滤，规范 URL 去重
- 上游证据：代表性提交 45181b06；归属 v0.3.0 web 段
- 对 custom 的初步价值：中高，取决于 key 配置与合规
- 是否值得独立移植：值得
- 依赖的其他功能：HTTP 客户端，工具 schema，secrets 管理
- 当前建议：暂缓

### C2 URL 感知 read 与内部 URL 方案

- 功能名称：read 支持 http 与内部 scheme 路由
- 解决问题：外部文档与内部资源入口割裂
- 主要能力：http 返回阅读模式 markdown，SSRF 默认拒绝私网目标，内部 skill prompt pr issue ssh 统一走同一工具
- 上游证据：代表性提交 f5e2987a，a0d31499；归属 v0.3.0 web 段
- 对 custom 的初步价值：高，read 是 custom 高频定制区
- 是否值得独立移植：值得，但冲突面偏高
- 依赖的其他功能：read 工具，HTTP 安全策略，扩展与会话上下文
- 当前建议：需设计

### C3 MCP 客户端与 slash 入口

- 功能名称：MCP 统一注册与多传输与信任生命周期
- 解决问题：外部工具生态接入缺统一面，配置来源分散
- 主要能力：原生与外部多配置源合并，指纹绑定信任，stdio 与 streamable HTTP，工具名包装，slash 状态查询
- 上游证据：代表性提交 952fb3bd；v0.1.23 已有 slash 状态前身，v0.3.0 成完整客户端
- 对 custom 的初步价值：高，但与 custom 扩展工具定制交织
- 是否值得独立移植：值得，分阶段更稳
- 依赖的其他功能：扩展系统，工具注册表，workspace trust，传输层
- 当前建议：需设计

### C4 github 工具

- 功能名称：github 工具，gh 后端的操作封装
- 解决问题：PR 与 issue 与 CI 运行操作缺 agent 原生入口
- 主要能力：PR 与 issue 与 run 等操作透出
- 上游证据：代表性提交 506a76ae；归属 v0.3.0 集成段
- 对 custom 的初步价值：中，依赖外部 gh 可用性
- 是否值得独立移植：值得
- 依赖的其他功能：bash 执行，auth，工具权限
- 当前建议：暂缓

### C5 外部会话 import

- 功能名称：pi import 外部会话导入
- 解决问题：跨工具会话迁移手工成本高
- 主要能力：从外部 agent 会话做幂等导入
- 上游证据：代表性提交 ddc075d9；归属 v0.3.0 集成段
- 对 custom 的初步价值：中低，偏迁移一次性能力
- 是否值得独立移植：值得，但优先级低
- 依赖的其他功能：会话存储，会话索引
- 当前建议：暂缓

## D 域：model routing / failover / dialect / advisor

### D1 model roles 与 titling 与 task-role 子代理

- 功能名称：模型角色路由与廉价标题与任务级子代理
- 解决问题：大小模型混用缺路由抽象，标题生成成本高
- 主要能力：角色配置，快捷 flag 切换，角色查看，低成本自动标题，任务角色子代理
- 上游证据：代表性提交 d5b8cf72；归属 v0.3.0 路由段
- 对 custom 的初步价值：高，与 custom 思考链与 provider 定制相邻
- 是否值得独立移植：值得
- 依赖的其他功能：模型目录，credential 解析，subagents，会话头
- 当前建议：需设计

### D2 failover chains 与 key rings

- 功能名称：故障转移链与多 key 轮换
- 解决问题：瞬时故障直接失败整回合，单 key 限流无退路
- 主要能力：分类瞬时故障后同回合换模型继续，冷却与每回合上限，auth 失败不转移， plural key 环带退避轮换
- 上游证据：代表性提交 bbc68341；归属 v0.3.0 韧性段
- 对 custom 的初步价值：高，直接提升弱网与限流体验
- 是否值得独立移植：值得
- 依赖的其他功能：provider 错误分类，重试，credential 管理，会话恢复
- 当前建议：采纳

### D3 dialect repair 弱模型兼容

- 功能名称：文本形态 tool call 修复
- 解决问题：弱模型不按结构化通道输出调用，导致调用丢失
- 主要能力：裸 JSON 与围栏块与标签形态修复为结构化调用，严格防误报
- 上游证据：代表性提交 89a15c22；归属 v0.3.0 韧性段
- 对 custom 的初步价值：高，对 DeepSeek 与兼容模型尤其有用
- 是否值得独立移植：值得
- 依赖的其他功能：工具执行器，provider 解析，agent 回合控制
- 当前建议：采纳

### D4 advisor 第二模型评审

- 功能名称：advisor 回合评审
- 解决问题：关键回合缺第二视角复核
- 主要能力：第二模型评审，失败封闭隔离，slash 入口
- 上游证据：代表性提交 b0c6dd59；归属 v0.3.0 韧性段
- 对 custom 的初步价值：中，评审质量依赖第二模型可用性
- 是否值得独立移植：值得
- 依赖的其他功能：模型路由，会话上下文构造，TUI 与 RPC 展示
- 当前建议：暂缓

### D5 模型目录与 thinking 与 auth 相关修复束

- 功能名称：目录 v2 与 thinking 尊重与 Bearer 与 prompt cache 修复束
- 解决问题：自定义目录行为与官方目录漂移，思考级别映射丢失，OpenAI 系 Bearer 缺失，缓存成本泄漏
- 主要能力：fetch 与 persist 与 refresh 闭环，thinkingFormat 与 thinkingLevelMap 尊重，credential 优先级，Anthropic 缓存 TTL 与保留期策略
- 上游证据：v0.2.0 目录与 credential 段；v0.3.0 修复段提交 281c2984，d5d8d276，1d817ab0，5e20d9df，9ec0ab34；issue 166 相关
- 对 custom 的初步价值：极高，与 custom gemini 思考链 DeepSeek 方言 xhigh 直接重叠
- 是否值得独立移植：值得，但不可整块硬合
- 依赖的其他功能：providers 全家，models 注册表，SDK，会话头
- 当前建议：需设计，重点做语义去重而非文件级合并

## E 域：checkpoint / undo / recovery / handoff / commit / ask / todo / plan / memory / magic

### E1 checkpoints 与 rewind 与 fresh 与 retry 与 max-time

- 功能名称：检查点与回退与 fresh 与重试与墙钟上限
- 解决问题：长会话试错无安全点，重试破坏拓扑，任务无时间上限
- 主要能力：检查点创建，回退压缩报告，fresh 重开，保留拓扑的重试，墙钟 cap
- 上游证据：代表性提交 b3723e4d；归属 v0.3.0 会话控制段
- 对 custom 的初步价值：高
- 是否值得独立移植：值得
- 依赖的其他功能：会话持久化，会话树，compaction，RPC
- 当前建议：需设计

### E2 undo 与 redo 快照

- 功能名称：文件变更 undo 与 redo
- 解决问题：文件工具误写缺可逆手段
- 主要能力：内容寻址快照，覆盖文件变更工具
- 上游证据：代表性提交 dd77beaa；归属 v0.3.0 会话控制段
- 对 custom 的初步价值：高，与 custom edit verify 链路互补
- 是否值得独立移植：值得
- 依赖的其他功能：文件工具全家，workspace 根，扩展 hostcall 共享 recorder 在 C1 后续才出现本次不计入
- 当前建议：采纳

### E3 turn recovery

- 功能名称：非预期停止分类与有界自续
- 解决问题：停止原因不明时整回合丢弃或无限续写
- 主要能力：确定性分类器，有上限自动继续
- 上游证据：代表性提交 1ba1e39d；归属 v0.3.0 会话控制段
- 对 custom 的初步价值：高
- 是否值得独立移植：值得
- 依赖的其他功能：agent 回合机，provider 错误分类，会话持久化
- 当前建议：采纳

### E4 pi handoff

- 功能名称：pi handoff 交接包
- 解决问题：跨会话与跨人交接缺标准载体且易泄密
- 主要能力：版本化 schema，secret 过滤
- 上游证据：代表性提交 d59bffa7；归属 v0.3.0 会话控制段
- 对 custom 的初步价值：中高
- 是否值得独立移植：值得
- 依赖的其他功能：会话序列化，secrets 扫描
- 当前建议：暂缓

### E5 pi commit

- 功能名称：pi commit 依赖序原子拆分提交
- 解决问题：大改动一锅提交难以评审回滚
- 主要能力：按依赖排序拆分原子提交
- 上游证据：代表性提交 644a077d；归属 v0.3.0 会话控制段
- 对 custom 的初步价值：中，偏工程提效
- 是否值得独立移植：值得
- 依赖的其他功能：git 面，bash，workspace 状态
- 当前建议：暂缓

### E6 ask 工具默认启用

- 功能名称：ask 结构化中途选项卡
- 解决问题：中途决策只靠自由文本追问，结构弱
- 主要能力：默认启用的结构化选项卡，跨 TUI RPC SDK
- 上游证据：代表性提交 bba4345f；归属 v0.3.0 agent 体验段
- 对 custom 的初步价值：高
- 是否值得独立移植：值得
- 依赖的其他功能：工具 schema，交互面，审批模式
- 当前建议：采纳

### E7 todo 工具默认启用

- 功能名称：todo 会话任务表与进度脚注
- 解决问题：多步任务状态散在对话中不可跟踪
- 主要能力：持久会话任务表，脚注进度行
- 上游证据：代表性提交 6ccdea44；归属 v0.3.0 agent 体验段
- 对 custom 的初步价值：高
- 是否值得独立移植：值得
- 依赖的其他功能：会话持久化，TUI 渲染
- 当前建议：采纳

### E8 plan mode 与 approval gate

- 功能名称：只读计划模式与提交计划与审批门
- 解决问题：高风险操作缺先审后做路径
- 主要能力：只读计划，submit plan，审批门，RPC 命令，approval mode 与 yolo 开关
- 上游证据：代表性提交 be2a71c2，af47798d；归属 v0.3.0 agent 体验段
- 对 custom 的初步价值：高，但与权限模型耦合深
- 是否值得独立移植：值得
- 依赖的其他功能：权限系统，工具执行门，RPC，TUI
- 当前建议：需设计

### E9 memory bank 可选本地 store

- 功能名称：memory bank 项目级记忆库
- 解决问题：跨会话知识缺项目级沉淀
- 主要能力：默认关闭，本地 SQLite 加 FTS，retain recall reflect 等工具，受管 skill 分级
- 上游证据：代表性提交 3be3a829；归属 v0.3.0 agent 体验段
- 对 custom 的初步价值：中，价值依赖长期使用习惯
- 是否值得独立移植：值得，但不紧急
- 依赖的其他功能：会话存储，工具注册，扩展 host
- 当前建议：暂缓

### E10 magic keywords 与 stream rules

- 功能名称：magic 关键词与流式规则
- 解决问题：高频编排意图表达冗长，流中干预缺手段
- 主要能力：散文级触发词，语法感知分词，流中 abort 与注入规则面
- 上游证据：代表性提交 3aac814f，441ffaa6；归属 v0.3.0 agent 体验段
- 对 custom 的初步价值：中低，偏品味与习惯
- 是否值得独立移植：不值得优先移植
- 依赖的其他功能：agent 输入管线，流事件机，规则管理命令
- 当前建议：冻结

### E11 compaction shake mode

- 功能名称：compact shake 零 LLM 回收
- 解决问题：超限上下文每次都走 LLM 总结成本高
- 主要能力：确定性丢弃超大工具结果，先 shake 后升级总结
- 上游证据：代表性提交 2a6607bf；归属 v0.3.0 agent 体验段
- 对 custom 的初步价值：高，custom 有 compaction 定制
- 是否值得独立移植：值得
- 依赖的其他功能：compaction，BPE 计量，会话裁剪
- 当前建议：需设计

## F 域：self-update / completions / usage / FTUI

### F1 pi self-update

- 功能名称：pi self-update 自升级
- 解决问题：二进制分发后升级链路缺闭环
- 主要能力：失败封闭校验，原子替换回滚，包管理器感知
- 上游证据：代表性提交 f16763f2；归属 v0.3.0 运维面
- 对 custom 的初步价值：中，取决于 custom 发布方式是否走官方通道
- 是否值得独立移植：值得
- 依赖的其他功能：发布资产命名，校验链，平台适配
- 当前建议：暂缓

### F2 shell completions 与 complete 协议

- 功能名称：shell 补全与模型会话候选协议
- 解决问题：命令行补全缺活数据
- 主要能力：多 shell 补全生成，live 模型与会话候选
- 上游证据：代表性提交 09e8dcc4；归属 v0.3.0 运维面
- 对 custom 的初步价值：中，偏体验加分
- 是否值得独立移植：值得
- 依赖的其他功能：CLI 图，模型注册表，会话索引
- 当前建议：暂缓

### F3 pi usage 与 slash usage

- 功能名称：用量与额度读取
- 解决问题：多 provider 花费与额度不可见
- 主要能力：主流 provider 额度读取，CLI 与 slash 双入口
- 上游证据：代表性提交 f6be31da；归属 v0.3.0 运维面
- 对 custom 的初步价值：中
- 是否值得独立移植：值得
- 依赖的其他功能：provider 适配，auth
- 当前建议：暂缓

### F4 FrankenTUI preview

- 功能名称：FrankenTUI 实验 TUI
- 解决问题：经典 TUI 滚动与 picker 与回滚体验瓶颈
- 主要能力：实验开关，尾随滚动，模态 picker，扩展桥接，inline 保滚动
- 上游证据：代表性提交 41dfdb7e；归属 v0.3.0 运维面；注意默认切换在 C3，本 C1 仅 preview
- 对 custom 的初步价值：高，但 custom 有 TUI 流式与 Windows 修复，冲突面大
- 是否值得独立移植：C1 阶段不值得单独移植
- 依赖的其他功能：交互栈，扩展桥接，会话持久化，主题
- 当前建议：冻结，留待 C3 专题处理

### F5 theme auto 与 TUI 可靠性波

- 功能名称：theme auto 与 TUI 加固波
- 解决问题：明暗检测缺失，日志盖 transcript，输出转义与换行与滚动异常
- 主要能力：终端明暗检测，日志分流，输出净化，工具头标注，换行与滚动与内存 tier 修复，tmux 与 VCR 覆盖
- 上游证据：代表性提交 6b7ac35c，41e97d31，1d723625，f6df955f，c98ac8fd，9d184467，28a798d4，7c391723，9b0f2841，8b43a2bf；归属 v0.3.0 TUI 可靠性段
- 对 custom 的初步价值：高，custom 同样面向 Windows Terminal 与流式渲染
- 是否值得独立移植：值得挑着移植
- 依赖的其他功能：TUI 渲染，主题，工具输出管线，测试夹具
- 当前建议：需设计，先拆渲染修复与测试夹具两类

## G 域：BPE / FrankenSQLite / workspace trust / secrets / extension 行为变化

### G1 BPE 真实 token 计量

- 功能名称：BPE 表驱动 token 计量
- 解决问题：chars 除 4 启发式导致 compaction 阈值漂移
- 主要能力：默认启用的 BPE 表，可回退旧启发式
- 上游证据：代表性提交 b91f6a3c；Breaking Changes 段
- 对 custom 的初步价值：高，影响 compaction 与会话裁剪基线
- 是否值得独立移植：值得，但属于基线行为变更
- 依赖的其他功能：compaction，会话存储，模型上下文窗
- 当前建议：需设计，不建议静默合入

### G2 FrankenSQLite 会话存储引擎切换

- 功能名称：FrankenSQLite 纯 Rust 会话引擎
- 解决问题：系统 sqlite 依赖与平台构建脆弱
- 主要能力：索引与 SQLite 会话走纯 Rust 引擎，错误类型化，sidecar 家族变化，二进制预算上调
- 上游证据：代表性提交 432c90cc；Breaking Changes 段
- 对 custom 的初步价值：高，但 custom 有会话持久化与 Windows 竞争重试定制
- 是否值得独立移植：不建议在 C1 拆出单移植，应随会话域整体设计
- 依赖的其他功能：session 与 session store，文件锁，fsync 策略，发布预算
- 当前建议：冻结，留待会话专题处理

### G3 workspace trust TOFU

- 功能名称：workspace trust 首次使用信任
- 解决问题：项目本地配置与扩展自动执行风险
- 主要能力：项目本地包与扩展默认不执行，非交互 fail closed，显式信任开关
- 上游证据：代表性提交 17faf856，issue 151；Breaking Changes 段
- 对 custom 的初步价值：高，安全基线变化
- 是否值得独立移植：值得
- 依赖的其他功能：配置加载，扩展发现，CLI 非交互面，automation 兼容
- 当前建议：需设计，需先评估对现有自动化与脚本的影响

### G4 manifest-only 扩展发现

- 功能名称：扩展发现仅 manifest
- 解决问题：启发式发现误加载，行为不可预期
- 主要能力：移除 sibling 与 bundle 聚类与 examples 扫描与 node_modules 回退，未声明即不加载
- 上游证据：代表性提交 3f37f46a；Breaking Changes 段
- 对 custom 的初步价值：中高，与 custom manifest aware 定制方向一致但实现可能冲突
- 是否值得独立移植：值得
- 依赖的其他功能：扩展扫描器，package manager，compat 层
- 当前建议：需设计

### G5 secrets 与 MCP 环境 allowlist

- 功能名称：MCP 子进程环境 allowlist
- 解决问题：环境变量透传导致 token 泄漏面扩大
- 主要能力：stdio 服务器只给 PATH HOME locale temp TERM 白名单， ambient token 须显式配置
- 上游证据：代表性提交 2da01cbc；Breaking Changes 段
- 对 custom 的初步价值：高，安全收益明确
- 是否值得独立移植：值得
- 依赖的其他功能：MCP 传输，配置，secrets 管理
- 当前建议：采纳

### G6 tiered tool schema 与 xdev

- 功能名称：工具 schema 分级与 xdev 调度
- 解决问题：全量工具 schema 挤占上下文
- 主要能力：非 essential 默认不出 provider schema，经 xdev list describe run promote 间接使用，可覆盖
- 上游证据：代表性提交 319f70ef；Breaking Changes 段
- 对 custom 的初步价值：高，但改变 custom tools.toml 与默认工具假设
- 是否值得独立移植：值得，但属于架构决策
- 依赖的其他功能：工具注册表，provider schema 构造，CLI tools 面
- 当前建议：需设计

### G7 扩展 JS host 能力增强

- 功能名称：扩展 host 上下文与 compact 与子代理与 provider hook 增强
- 解决问题：扩展可编排能力不足，压缩与评审与请求干预缺原生钩子
- 主要能力：会话上下文构造，LLM 转换，模型查找，原生 compact，类型化子代理结果，provider 请求前钩子
- 上游证据：代表性提交 cd9c2bfc，9974d79d；issue 167 相关；归属 v0.3.0 互操作段
- 对 custom 的初步价值：高，但 custom 扩展语义定制深
- 是否值得独立移植：值得分批
- 依赖的其他功能：扩展运行时，工具注册，compaction，subagents，provider 路由
- 当前建议：需设计

### G8 扩展 VFS 隔离与 realm 新鲜度与单体拆分

- 功能名称：扩展 VFS 端到端隔离与 realm 隔离与 extensions 目录化
- 解决问题：扩展越权访问，realm 复用污染，单体难维护
- 主要能力：tmp 下注册根 host 回退其余私有，缓存与符号链与 fd 逐跳鉴权，reload 用新 realm，extensions 拆多模块 façade 不变
- 上游证据：v0.2.0 可靠性段与 Internal 段；issue 130；probe 与 inventory 已标记 extensions 高冲突
- 对 custom 的初步价值：高，但与 custom fs 持久化定制语义可能正交冲突
- 是否值得独立移植：不建议拆小移植，应做扩展架构专题
- 依赖的其他功能：扩展 fs shim，QuickJS 运行时，权限漂移，事件合并， release 证据
- 当前建议：冻结，留待扩展专题处理

### G9 SDK 结构与 prompt cache 默认与 image 解码收窄

- 功能名称：SDK 结构变更与缓存默认与解码收窄 bundle
- 解决问题：SDK 面缺扩展字段，缓存默认导致每回合全价，image 解码 bundle 膨胀
- 主要能力：AgentConfig 增字段，PiApp 增参，短保留默认，解码限定常见四格式
- 上游证据：代表性提交 9ec0ab34，2f6c227b，9d4872a0；分属 Breaking 与性能段
- 对 custom 的初步价值：中，SDK 面影响下游 embedder
- 是否值得独立移植：值得，但须同步 SDK 调用方
- 依赖的其他功能：SDK，provider，构建 profile
- 当前建议：暂缓

## 跨域依赖小结

- 执行底座：B1 与 B2 与 B3 与 D2 与 E1 与 E3 相互咬合，不宜按单文件移植。
- 交互底座：F4 与 F5 与 E6 与 E8 与 C3 共用 TUI 与 RPC 与审批面，C3 前置变化大。
- 知识底座：A 全家依赖 G6 分级 schema 与 workspace 约束，G6 未定前 A 移植形态不稳定。
- 会话底座：E 全家与 G1 与 G2 与 D5 共用会话与 compaction，G2 未定前 E 不宜大合。
- 安全底座：G3 与 G4 与 G5 与 C3 与 G7 共用信任与鉴权，适合做一个安全专题包。

## 不确定项

- C1 新增与修改路径机器清单只有总量快照：351 files 与 118 源码文件，未在本任务中逐文件展开；当前环境无 shell 可执行 Git 区间 diff，如需逐路径清单须另起只读任务补齐。
- 317 提交未逐条阅读，各功能域代表性提交之后是否还有同域修复与回滚未收敛，特别是 hub 在 C2 才收束，FTUI 在 C3 才默认切换，C1 结论不可外推到 C2 以后。
- .beads issue 的 Why 与 Design 与 Acceptance 未逐条拉取：bd-cv653 epic 与各 GH issue 的设计取舍可能改变移植粒度，当前移植建议只基于 CHANGELOG 归纳。
- custom 与 C1 的真实三方冲突未知：README 快照显示 custom 与 main 净差异 657 文件与 src 交集 87，但那是相对当前上游末端的快照，不是 C1 区间精确交集；C1 真实 UU 须等 integration 试合并。
- 依赖漂移版本未锁定：v0.2.0 提及 Rust 1.95 与 asupersync 与 fsqlite 与 fs4 等，probe 报告另提 Tool execute arity 与 HostActions arity 与缺失 crate，C1 是否全部引入须以 Git 冲突与 cargo check 为准。
- TUI 与证据与发布管线噪音未量化：v0.2.0 发布加固与证据门在 C1 区间占比不明，移植时应默认排除 docs evidence contracts perf 与快照生成物。
- MCP streamable HTTP 与 RPC 改路由在 v0.4.0 才有合同测试落点，C1 的 C3 描述仅为 preview，移植时不应把 C1 后的传输语义提前假设。
