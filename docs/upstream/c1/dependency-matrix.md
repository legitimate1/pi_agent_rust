# C1 依赖与 API 漂移矩阵（只分析，不改源码）

> 目标：为 C1 上游同步建立依赖漂移决策底稿。
> 本文档只分析，不修改源码，不解决 merge 冲突，不运行 cargo。
> 不使用 Markdown 表格，所有条目均用列表表达，避免表格解析歧义。

## 0. 上下文边界与方法约束

- C1 锚点：`e23c4622f8bc4038a5e061ee3640a0e9206ec5cc`（上游 `v0.3.0`，第一段 release 状态）
- 共同基线：`226a876425a856f657b2a5d7c7ac6f0ca1ad25f1`
- 当前 custom HEAD（本次从当前分支直接读取）：`d1786d8b9658ac2553a1a46a3bca5414b1bf9b67`
- 当前 main（本地已合入最新 upstream，未推 origin）：`195ca9464101c4862607c909951bf9467baf245c`
- 当前 upstream/main 快照：`e403485b3116e6c97e9af7026ec9445f30312c7d`
- integration/c1 当前指向：`d1786d8b9658ac2553a1a46a3bca5414b1bf9b67`（与 custom 一致，尚未开始合 C1）
- C1 区间规模（来自 merge-checkpoints.md，非本次实测）：相对共同基线 317 commits，其中 310 非 merge；351 files，118 源码文件；+93,550 / -4,141
- 当前 custom 与 main 净差异（来自 docs/upstream/README.md 快照）：custom 独有 330 提交，main 独有 1303 提交；净差异 657 文件，两侧共同改动 229 文件，其中 src 交集 87 个

方法约束（本次严格遵守）：

- 只读取和分析 Cargo.toml 的依赖和 features 声明冲突，以及 Cargo.lock 的机器可提取的版本行差异
- 没有读取整个 Cargo.lock，只用 grep 按 crate 名抽取版本行及其上下文 2 行
- 没有运行 cargo，没有执行 cargo check/test/add/update
- 没有修改 Cargo.toml / Cargo.lock，没有修改任何 src/crates 源码
- 没有做真实 merge，因此没有真实冲突文件清单；凡涉及“C1 侧版本”而无直接证据处，一律标注为待 integration 线复核

证据来源清单（本矩阵每项均回指以下之一）：

- E1：当前 custom `Cargo.toml` 全文（已读，版本 0.1.93）
- E2：当前 custom `Cargo.lock` 的按名 grep 片段（asupersync、digest、filetime、fs4、glob、globset、ignore、jsonschema、portable-pty、rquickjs、rustix、swc_common、swc_ecma_parser、sysinfo 双版本、wasmtime、win32job、zstd；未读全文件）
- E3：`crates/pi-core/Cargo.toml` 与 `crates/pi-provider-core/Cargo.toml`（asupersync 0.3.6 双 crate 声明）
- E4：`rust-toolchain.toml`（nightly-2026-07-05 pin 及其注释：sysinfo 0.39.6 需要 Rust 1.95）
- E5：`docs/upstream/probe-report-2026-08-29.md` §4（297 错误分类，Cargo/API 漂移桶）
- E6：`docs/upstream/plan-hub-minimal.md` §7 落地证据（hub A1 实际 Cargo 增量）
- E7：`docs/upstream/fork-merge-sop.md`（依赖漂移陷阱：digest/swc，上游依赖自动合入，默认回退）
- E8：`docs/upstream/merge-checkpoints.md`（C1 区间规模与停止条件）
- E9：`docs/upstream/README.md`（当前快照与工作流：integration 线逐阶段推进）
- E10：当前 custom `src/` 的按名 grep（rquickjs、swc、wasmtime、sysinfo、portable-pty、fs4、jsonschema、filetime、glob、ignore 的 use 与调用点；tiktoken/fsqlite/ftui/pprof/htmd 在 src 中零命中或仅注释）

冲突等级定义（本矩阵统一用语）：

- 等级：已落地：custom 已含该版本或该依赖，C1 若同版本则无动作
- 等级：低：新增 optional 或 dev-only 依赖，或上游单边新增，custom 可按需单加
- 等级：中：既有 crate 小版本/补丁漂移，或 feature 加法，需小适配
- 等级：高：大版本/工具链/运行时主干漂移，或 API arity 变化牵连几十文件，默认不跟
- 等级：冻结：已知上游整体状态，与 C1 功能闭包无关，本次明确不跟

功能必要性二分法（每项必标其一）：

- 类别 A：某个功能的直接必要依赖（缺了该功能编译不过或行为缺失）
- 类别 B：C1 上游整体状态（工具链/传递依赖/发布管道顺带抬升，不是任一功能点名要的）

## 1. 当前 custom 基线快照（E1+E2+E3+E4）

- 主包版本：0.1.93，edition 2024，rust-version 声明 1.85（注意与 toolchain pin 的 nightly-2026-07-05 并存，E4 注释称锁定的 asupersync/sysinfo 图需要 Rust 1.95 语言支持）
- 默认 features：sqlite-sessions + tui；full 聚合 image-resize、jemalloc、clipboard、wasm-host、sqlite-sessions、syntax-highlighting、tui、rich_rust/full
- tui feature 聚合：dep:crossterm、dep:bubbletea、dep:lipgloss、dep:bubbles、dep:glamour、dep:unicode-width、dep:textwrap
- wasm-host feature：仅聚合 dep:wasmtime，wasmtime 本体 optional
- asupersync：主 Cargo.toml 为 0.3.9（default-features=false，features tls-webpki-roots + test-internals）；pi-core 与 pi-provider-core 均为 0.3.6 同 feature 集；Cargo.lock 当前含 asupersync 0.3.9
- rquickjs：主依赖 0.11（features futures + loader）；freebsd 与 android target 覆盖同版本加 bindgen；Cargo.lock 含 rquickjs 0.11.0
- SWC 系列：swc_common 18.0.1、swc_ecma_codegen 23.0.0、swc_ecma_ast 20.0.1、swc_ecma_parser 34.0.0、swc_ecma_transforms_base 37.0.0、swc_ecma_transforms_typescript 41.0.0、swc_ecma_visit 20.0.0；Cargo.lock 含 swc_common 18.0.1 与 swc_ecma_parser 34.0.0
- wasmtime：41.0.3 optional，features component-model；Cargo.lock 含 wasmtime 41.0.4（补丁位漂移 1 位，属 lockfile 正常浮动）
- sysinfo：Cargo.toml 声明 0.38.2；Cargo.lock 同时含 sysinfo 0.38.4 与 sysinfo 0.39.6 双版本（直接证明传递依赖已引入新大版本，但直连声明仍钉旧版）
- fs4：Cargo.toml 与 Cargo.lock 均为 1.1.0（hub A1 已从 0.13 升上来，见 E6）
- portable-pty：Cargo.toml 直连 0.9；Cargo.lock 0.9.0（hub A1 新增，见 E6）
- rustix：Cargo.toml 直连 1.1.4（features fs + process）；Cargo.lock 1.1.4（hub A1 新增，见 E6）
- win32job：windows target 直连 2.0.3；Cargo.lock 2.0.3（hub A1 新增，见 E6）
- jsonschema：主 dependencies 0.42.0（default-features=false）与 dev-dependencies 同版本各一条；Cargo.lock 0.42.2（hub A1 从 dev 提升到主依赖，见 E6）
- glob：直连 0.3；ignore：直连 0.4；Cargo.lock 含 glob 0.3.3、globset 0.4.18、ignore 0.4.25（注意 Cargo.toml 无 globset 直连，但 lock 已有 globset，说明为 ignore 的传递依赖）
- filetime：仅 dev-dependencies 0.2.27；Cargo.lock 0.2.27
- sqlmodel-sqlite 与 sqlmodel-core：均为 0.2.2 直连
- digest：Cargo.lock 同时含 digest 0.10.7 与 digest 0.11.2 双版本（直接证明上游新依赖树已部分进入 lock，但 custom 直连代码仍用旧 API，见 E5 与 E7）
- 外部工具链：MSVC cl.exe（ring、libsqlite3-sys、rquickjs-sys、tree-sitter 含 C/C++，AGENTS.md 约束 Windows 必须 pwsh+vcvars64）；freebsd/android 的 rquickjs bindgen 需要系统 clang；build-dependencies 仅 vergen-gix 9.1.0

## 2. C1 区间可确认的漂移信号（E5+E6+E7+E8，未做真实 merge）

- E5 明确记录：在 Cargo.toml 取 --ours 保 custom 基线时，hub/jobs 新代码缺依赖；在取 --theirs 跟上游时，asupersync 0.3.9→0.4.4、rust 1.85→1.95、digest 0.10→0.11、fs4 0.13→1.1 会引入 400+ 错误量级（与 SOP 423 同量级）
- E5 的 missing crate 桶点名：globset、fsqlite 0.3.5（native+fts5）、portable-pty 0.9、tiktoken-rs 0.12（optional）、pprof、rustix、htmd、jsonschema
- E5 的 missing fn/type 桶与 Tool::execute 4→5 参、ExtensionSession/HostActions 2→3 参同列，说明依赖缺失与 API arity 漂移是同一批编译失败的不同表现，不可只加依赖不看 API
- E6 落地证据确认 hub A1 已消化其中一部分：fs4 0.13→1.1、新增 portable-pty 0.9、rustix 1.1.4、win32job 2.0.3（windows）、jsonschema 提升到主依赖；刻意未搬 tiktoken/pprof/ftui
- E7 定性：上游依赖升级会自动合入 Cargo.toml（无冲突），custom 代码用旧 API 会爆几百编译错误；默认决策是主依赖回退 custom 版，dev-deps 可跟上游
- E8 约束：C1 模型不得读取 317 提交全文，只处理真实冲突和首个验证失败；出现依赖升级导致大面积错误且无适配计划就停止当前阶段

## 3. 按功能域的漂移条目

### 3.1 JS 运行时：rquickjs 0.11 系

- 上游状态：C1 区间大概率仍为 0.11 系（E5 未把 rquickjs 列为升级项；E2 当前 lock 即 0.11.0）
- custom 现状：主依赖 0.11 futures+loader；freebsd/android 加 bindgen；代码使用面极广（E10：crypto_shim 全文件、extensions_js 的 SWC+loader+polyfill 区、pi_wasm 注释提及 wasmtime-backed polyfill）
- 证据来源：E1 Cargo.toml 263 行与 325/334 target 覆盖；E2 lock 0.11.0；E10 grep 命中 crypto_shim、extensions_js
- 影响范围：QuickJS 扩展加载、node:fs shim、node:zlib、crypto hostcalls、wasm polyfill 注入点（extensions_js 19423 行附近注释）
- API/编译影响：0.11 系内 features 变化敏感（bindgen、loader、futures 缺一即 montage 编译失败）；跨大版本则 Func/TypedArray/Object 签名全受影响，但本次无跨大版本证据
- 必要性：类别 A（pijs/扩展运行时的直接必要依赖）
- 与 custom 冲突等级：低（已落地同版本；C1 若只动 features 需逐项核对）
- 建议：保留 custom 版本与 features 组合；C1 若新增 bindgen 覆盖平台，仅按平台 target 采纳，不动主依赖行

### 3.2 TS/JS 结构化处理：SWC 系列

- 上游状态：E7 点名 swc 为隐藏成本代表之一；但 E5 的 297 分类未把 swc 版本号列为独立桶，说明 SWC 更多是“升了就爆几十处”而非“缺包”
- custom 现状：7 个 swc crate 版本钉死（见 §1）；代码使用集中在 extensions_js（resolver/strip/Emitter/JsWriter/Parser，E10）
- 证据来源：E1 Cargo.toml 264-270 行；E2 swc_common 18.0.1、swc_ecma_parser 34.0.0；E10 extensions_js 55-60、1750、1924、7586 行；E7 digest/swc 并列
- 影响范围：扩展 JS 的 TS strip、resolver、codegen；ast_grep/ast_edit 的上游新工具若在 C1 已落地，会与 SWC 解析共用 SourceMap/Globals 语义
- API/编译影响：SWC 跨大版本极易变（Syntax/EsSyntax、ModuleItem、Pass、Mark、Lrc 路径）；即使小版本也可能因 swc_common 与 parser 版本对不齐导致 trait 不匹配
- 必要性：类别 A（extensions_js TS 支持的直接必要依赖）；但“升到哪个大版本”属于类别 B（上游整体状态顺带）
- 与 custom 冲突等级：中到高（版本对齐即中，不对齐即高；custom 已有 10 内置工具含 ast_grep/ast_edit，接 SWC 升级必须同步测扩展加载冒烟）
- 建议：单独升级适配；C1 合入时先 --ours 钉住 7 个版本号，待扩展冒烟通过后再单独立项评估 SWC 大版本，不在 C1 内顺手升

### 3.3 WASM 宿主：wasmtime optional

- 上游状态：wasmtime 在 C1 仍应为 optional（probe 的 missing 桶未点名 wasmtime 缺失；E10 显示 extensions.rs 的 component bindgen 与 HostState 仍在 cfg/feature 门后）
- custom 现状：wasmtime 41.0.3 optional + component-model；wasm-host feature 聚合；lock 为 41.0.4；代码使用在 extensions.rs 13942/14956/15975 区与 pi_wasm.rs 全文件（E10）
- 证据来源：E1 Cargo.toml 271 与 370 行；E2 lock 41.0.4；E10 extensions.rs 与 pi_wasm.rs 命中
- 影响范围：wasm-host feature 开启者（扩展 native_runtime_experimental、component Linker/Store/Engine）；默认构建不影响（optional 未启用则不编译）
- API/编译影响：wasmtime 41 系内 Config/Engine/Store/Linker 稳定；component-model feature 缺失会导致 bindgen 宏不可用，但属显式 feature 错误，易定位
- 必要性：类别 A（wasm 扩展能力的直接必要依赖）；默认关闭时对 C1 主线为类别 B（整体状态，不 blocking）
- 与 custom 冲突等级：低（optional 隔离天然低冲突；41.0.3→41.0.4 补丁位无需动作）
- 建议：保留 custom 版本与 optional 形态；C1 若把 wasmtime 从 optional 转正或加默认 features，必须冻结并单独决策（会直接抬升默认构建体积与 MSVC 链接成本）

### 3.4 运行时主干：asupersync + sysinfo + toolchain 链（最危险，见 §5）

- 上游状态：E5/E4 共同指向 asupersync 0.3.9→0.4.4、sysinfo 0.38→0.39、Rust 1.85→1.95 三连（E4 注释明说 sysinfo 0.39.6 声明 Rust 1.95 且用了老 pin 不支持的语言/库特性）
- custom 现状：主 asupersync 0.3.9，双子 crate asupersync 0.3.6；sysinfo 直连 0.38.2 但 lock 已双版本并存；toolchain pin nightly-2026-07-05；代码使用 sysinfo 在 doctor.rs 磁盘卷枚举与 hub.rs 进程树信号（E10）
- 证据来源：E1 主 0.3.9；E3 双 0.3.6；E2 asupersync 0.3.9 + sysinfo 0.38.4/0.39.6 双版本；E4 工具链注释；E10 doctor.rs 10446/10452、hub.rs 724/754；E5 取 --theirs 爆 400+ 错误
- 影响范围：全局运行时（spawn_blocking、TLS、test-internals）、子代理/任务重试、hub 进程树 kill、doctor 磁盘检查；工具链抬升还会触发 clippy pedantic/nursery 新警告（E4 明确警告）
- API/编译影响：asupersync 大版本常伴随 API/行为变化；sysinfo 0.38→0.39 的 Disks/System/ProcessesToUpdate/Pid/Signal 路径已在 custom 落子，混版极易出现“lock 有新版、代码用旧版”或反之；digest 双版本并存是同一链条的下游症状
- 必要性：类别 B（上游整体状态，不是 hub/扩展任一功能点名要的；hub A1 已证明 0.3.9 可跑 hub 全闭包，见 E6）
- 与 custom 冲突等级：高（跨 toolchain + 运行时 + clippy 门禁三重放大）
- 建议：保留 custom 版本；C1 全程 Cargo.toml/lock 对 asupersync/sysinfo/rust-toolchain 取 --ours；若 C1 真实冲突显示上游代码已调用 0.4.4/0.39 新 API，冻结该文件并转“运行时升级”独立项目，不在 C1 内解

### 3.5 文件锁：fs4 0.13→1.1（已落地，C1 无需再付费）

- 上游状态：E5/E6 确认上游已到 fs4 1.1；hub A1 之前 custom 还在 0.13
- custom 现状：已升到 1.1.0（E1/E2）；代码使用在 permissions.rs 与 session.rs 的 FileExt（E10）；E6 记录曾出现 fs_std::FileExt→FileExt、lock_exclusive→lock 的 API 迁移并已修
- 证据来源：E1 214 行；E2 1.1.0；E6 落地证据；E10 permissions.rs 11、session.rs 25
- 影响范围：会话持久化文件锁、权限文件锁、Windows 文件竞争重试
- API/编译影响：1.1 的 FileExt 路径与 lock 命名已在 custom 收敛；C1 若仍有旧调用残留，属单点易修
- 必要性：类别 A（会话/锁的直接必要依赖；且已落地）
- 与 custom 冲突等级：低（已落地；C1 同版本则零动作）
- 建议：保留 custom 版本；C1 若出现旧 API 调用残留，按 E6 既有修法（FileExt 路径与 lock 命名）单点适配

### 3.6 终端 PTY：portable-pty 0.9 + rustix + win32job（已落地，C1 需防 feature 回退）

- 上游状态：E5 把 portable-pty 0.9 列为 hub 最小闭包三件套之一（portable-pty、fsqlite、globset）；rustix 同属该批
- custom 现状：三件套中两件已落地（portable-pty 0.9.0、rustix 1.1.4 fs+process；windows 另有 win32job 2.0.3）；hub.rs 已用 portable_pty CommandBuilder/PtySize/native_pty_system 与 Child/MasterPty trait（E10）
- 证据来源：E1 215-216、340-341 行；E2 三个 lock 版本；E6 增量清单；E10 hub.rs 145/502/505 行
- 影响范围：hub PTY 会话、子代理终端、Windows Job Object 进程树（win32job 封装 Win32 FFI，保持 unsafe-free 约束见 E1 注释）
- API/编译影响：portable-pty 的 MasterPty/Child trait 与 PtySize/CommandBuilder 在 0.9 系内稳定；rustix 的 fs/process features 缺一会导致对应模块编译门失败，但错误信息明确；win32job 仅 windows 生效，Linux CI 不感知
- 必要性：类别 A（hub PTY 的直接必要依赖；win32job 为 Windows 进程树纪律的直接必要依赖）
- 与 custom 冲突等级：低（已落地；风险只在 C1 把 features 改窄或把 win32job 移出 windows target）
- 建议：只新增依赖（若 C1 真实 merge 丢了这三行，按 E6 补回，不接受删除）；features 保持 fs+process 不缩水；win32job 保持 windows target 专属

### 3.7 结构校验：jsonschema 0.42（已从 dev 提升到主依赖）

- 上游状态：E5 同时在 missing 桶点名 jsonschema，说明上游 hub/扩展校验已依赖它
- custom 现状：主+dev 双 0.42.0 default-features=false；lock 0.42.2；extensions.rs 已用 Validator 与 draft202012 options（E10）
- 证据来源：E1 244 与 287 行；E2 0.42.2；E6 提升记录；E10 extensions.rs 32006/32121 行
- 影响范围：扩展 manifest/协议校验、工具参数 schema 校验
- API/编译影响：0.42 的 Validator/draft202012 路径稳定；default-features=false 是体积与编译时间的关键约束，打开默认会引入多余后端
- 必要性：类别 A（扩展校验的直接必要依赖）
- 与 custom 冲突等级：低（已落地；C1 若改 default-features 需警惕）
- 建议：保留 custom 版本与 default-features=false；C1 若新增校验调用点，只新增调用，不升级版本

### 3.8 文件发现：glob / globset / ignore / grep 系列

- 上游状态：E5 点名 globset 缺失；ignore 体系是 ast_grep/文件发现的上游基础
- custom 现状：直连 glob 0.3 + ignore 0.4；无 globset 直连但 lock 已有 globset 0.4.18（ignore 传递）；代码使用 glob::Pattern（app.rs）、ignore::WalkBuilder 与 overrides（autocomplete.rs、ast_tools.rs）（E10）
- 证据来源：E1 217-218 行；E2 glob 0.3.3、globset 0.4.18、ignore 0.4.25；E10 app.rs 11、autocomplete.rs 21/666、ast_tools.rs 207/211/227/270；E5 missing 桶
- 影响范围：自动补全文件漫游、ast 工具 override、tools 的 grep/ls/find/read 路径
- API/编译影响：globset 缺失是显式编译错误（E5 已归类为低成本按需 cargo add）；ignore 的 WalkBuilder/OverrideBuilder 在 0.4 系内稳定；若 C1 把 globset 从传递变直连，属加法，无破坏
- 必要性：类别 A（hub 最小闭包三件套之一的 globset + 文件发现的直接必要依赖）
- 与 custom 冲突等级：低（hub A1 计划加但 E6 落地未明确 globset 是否已加；lock 有传递版打底）
- 建议：只新增依赖；C1 真实 merge 若报 globset 缺失，单加 globset 直连，不跟上游其他发现层重构；grep 系列若上游新增 ripgrep 封装 crate，同理单加评估

### 3.9 会话存储：fsqlite / fsqlite（C1 最高优先级待确认项）

- 上游状态：E5 明确写 fsqlite 0.3.5（native+fts5）；上游 session/session-store 硬化与 fsqlite 0.3.2 迁移计划相关（见旧上游清单）；C1 作为 v0.3.0 release 状态，极可能已含 fsqlite 新调用
- custom 现状：Cargo.toml 无 fsqlite/fsqlute 直连；Cargo.lock 无命中；src 无命中（E10）；当前会话靠 sqlmodel-sqlite/sqlmodel-core 0.2.2 + fs4 锁 + 自研 persister
- 证据来源：E1 无直连（sqlmodel 221-222 行可作对照）；E2 grep fsqlite 零命中；E10 src 零命中；E5 missing 桶点名；E6 明确本次不跟 fsqlite 之外的多余依赖
- 影响范围：若 C1 合入，冲击 session/session_store_v2/sqlite 会话、jobs artifact、hub Done 条目持久化；且与 custom 的 RPC 会话持久化、Windows 竞争重试正面重叠
- API/编译影响：native+fts5 features 缺一即全文检索或本地文件后端编译失败；open_regular_file_for_write/path_entry_exists/open_private_directory/ArtifactWriteMode 等 helpers（E5 missing fn 桶）很可能就是 fsqlite 伴生 API，需连带移植
- 必要性：类别 A（若要 hub/jobs/session 全量语义，必须；若只要 hub roster/jobs 最小闭环，可暂缓，E6 已证明最小闭环可不带 fsqlite 跑通）
- 与 custom 冲突等级：中到高（单加依赖是低，但语义重叠区是高；session_store_v2 在旧语义地图即上游单边为主区）
- 建议：暂不采纳为默认；C1 先 --ours 保持无 fsqlite，若真实冲突显示 jobs/session 新文件强引用 fsqlite，再按“只新增依赖”（0.3.5 native+fts5）单加，并把 helpers 缺失作为 API 适配子项另行评估，不在依赖矩阵内顺手迁 session 语义

### 3.10 Token 计数：tiktoken-rs 0.12 optional（明确不跟）

- 上游状态：E5 标注 optional；E6 明确本次不搬
- custom 现状：Cargo.toml 无直连；lock 无命中；src 无命中（E10）
- 证据来源：E1 无直连；E2 零命中；E10 零命中；E5/E6
- 影响范围：仅 token 估算/队列预算/模型上下文裁剪的精度优化，不 blocking 编译（optional 未启用则无影响）
- API/编译影响：无（不加即无影响；加了才需适配分词器 API）
- 必要性：类别 B（上游整体状态/优化项，非 C1 功能闭环必要）
- 与 custom 冲突等级：冻结（本次不跟，等级不适用；若未来要 estimate_tokens 精度再立项）
- 建议：暂不采纳；C1 即使出现上游调用点，也优先用现有 estimate_tokens/RPC 端点语义保留 custom 行为，不为单个优化项引入新分词依赖

### 3.11 测试时间工具：filetime dev-only（保持 dev，不上主）

- 上游状态：dev 依赖常见项；E5 未点名 filetime 缺失，说明非主线 blocking
- custom 现状：dev 0.2.27；jobs.rs 测试用 set_file_mtime/from_unix_time 做 artifact rotation 固件（E10）
- 证据来源：E1 285 行；E2 0.2.27；E10 jobs.rs 3412/3463/3503 行
- 影响范围：仅 cargo test/bench 的 mtime 固件，不进 release 产物
- API/编译影响：无主线影响；dev 版漂移只影响测试编译
- 必要性：类别 B（测试固件便利，非功能必要）
- 与 custom 冲突等级：低
- 建议：保留 custom 版本；C1 的 dev-deps 变化可跟上游（按 E7 dev-deps 可跟），但 filetime 不得提升到主 dependencies

### 3.12 性能/格式/文本：ftui、pprof、htmd 及同类可选（全部冻结）

- 上游状态：E5 把 pprof、htmd 与 tiktoken 并列为“按需 cargo add，勿全跟 upstream 0.3.0”；ftui 在任务要求中点名但 E5/E6 均未将其列为 hub 闭包必要（E6 明确多余 tiktoken/pprof/ftui 本次不跟）
- custom 现状：三者均无 Cargo.toml 直连、无 lock 命中、无 src 命中（E1/E2/E10）；C5/C6 才涉及 FTUI 收尾（见 merge-checkpoints），C1 不应提前付费
- 证据来源：E1 零直连；E2 pprof/ftui/htmd 零命中（仅 zstd 有命中但与此无关）；E10 零命中；E6 冻结清单；merge-checkpoints C5 风险提示 FTUI 收尾混杂
- 影响范围：pprof 影响 profiling/bench 门；htmd 影响 HTML→Markdown 转换；ftui 影响 FrankenTUI/交互默认路径（C3 才切换默认，C1 更不应动）
- API/编译影响：现在跟会提前引入 TUI 默认路径与性能证据门噪音，与 C1 的 v0.3.0 锚点无关
- 必要性：类别 B（上游整体状态/后段 checkpoint 功能，非 C1 必要）
- 与 custom 冲突等级：冻结
- 建议：暂不采纳；C1 全程忽略这三者的新增调用（若真实 merge 带入新文件引用，先回退该文件到 custom HEAD，按 SOP 自动合入文件处理，而不是加依赖去迁就）

### 3.13 哈希/签名链：digest 双版本并存（症状，非目标）

- 上游状态：digest 0.10→0.11 是 E5 取 --theirs 爆错四件套之一；E7 把 digest 列为自动合入陷阱
- custom 现状：lock 同时含 0.10.7 与 0.11.2（E2），说明依赖树已混版；custom 直连代码仍按旧 API（crypto_shim 的 sha2/sha1/md-5/hmac/pbkdf2/scrypt 组合见 E1 236-241 行）
- 证据来源：E2 双版本；E1 236-241 行；E5/E7
- 影响范围：扩展签名、auth I/O、包校验、crypto hostcalls
- API/编译影响：digest 大版本 API 不兼容是几百错误的放大器之一，但 custom 不应直接依赖 digest（应通过 sha2/hmac 等上层 crate 间接），因此版本钉死权不在本矩阵，而在上层 crate 的 req 上
- 必要性：类别 B（传递依赖整体状态）
- 与 custom 冲突等级：高（若跟上游 --theirs 会连带爆；若保持 --ours 则 lock 双版本可容忍，cargo 会按需选）
- 建议：保留 custom 版本（即不主动 cargo update digest）；C1 若出现 digest 新 API 调用，一律视为“自动合入文件引用新 API”，按 SOP 回退该文件到 HEAD，不升级 digest

### 3.14 features 与 default features 变化（高敏区，默认全 --ours）

- 上游可能变化：tui 默认开/关、wasm-host 是否进默认、sqlite-sessions 语义、syntax-highlighting/image-resize/clipboard/jemalloc 的 full 聚合、rquickjs bindgen 平台覆盖、jsonschema default-features 开关、wasmtime component-model 开关
- custom 现状：default 为 sqlite-sessions+tui；wasm-host/image/jemalloc/clipboard 保持 opt-in；rquickjs 主 features 为 futures+loader，freebsd/android 额外 bindgen；jsonschema 明确 default-features=false；asupersync 明确 default-features=false（见 §1）
- 证据来源：E1 343-382 行 features 全段；E1 167/249/263/271/244 行各 features；E4 clippy 门禁对 features 变化敏感
- 影响范围：default 变化会同时改变 pi 二进制形态（tui 门控 src/main.rs）、release 体积预算、Linux jemalloc 后端、Windows clipboard/wasmtime 链接
- API/编译影响：wasmtime/rquickjs/jsonschema 的 features 缺失是显式编译错误但易修；tui 从默认移除则 pi 主二进制不产出（required-features=tui），属发布行为突变，必须拦截；jemalloc 默认开启会改变 Linux 内存分配器与基准
- 必要性：混合（tui/wasm-host/sqlite 为类别 A 的形态控制；其余 full 聚合多为类别 B）
- 与 custom 冲突等级：中到高（加法 feature 为中，动 default 为高）
- 建议：保留 custom 版本（features 全段 --ours）；只接受“加法且 optional”的上游新 feature（例如新增 ext-conformance 这类空 feature），拒绝任何把 optional 转默认、把 default-features=false 改 true、把 required-features=tui 摘掉的变化；jemalloc 保持 opt-in，不进默认

### 3.15 外部工具依赖（非 crates.io，但会卡 C1 验证）

- MSVC 工具链：ring/rquickjs-sys/tree-sitter/libsqlite3-sys 含 C/C++，Windows 必须 pwsh+vcvars64+sccache/lld-link（AGENTS.md 约束）；C1 若新增同类含 C crate，本地验证必须同环境，否则 build-script-build 失败会被误判为依赖漂移
- clang/bindgen：freebsd/android 的 rquickjs bindgen 需要系统 clang；C1 若把 bindgen 扩大到更多平台或更多 crate，Linux CI 也要装 libclang，否则 repro 不出来
- git/vergen：build-dependencies 的 vergen-gix 需要 git 元数据；worktree/integration 线的 commit trailer 与 shallow fetch 会影响 build 脚本输出，不属依赖版本问题，排查时先排除
- Node/npx/prettier/oxfmt/ruff/gofmt：verify 与格式化外部 checkers（custom 的 verify 子系统语境）；C1 若新增外部 checker 调用，缺失时表现为测试失败而非编译失败，不应在依赖矩阵里用加 crate 去修，应按“外部工具缺失”单列验证前置
- 证据来源：E1 276-277 build-deps、322-335 target 覆盖、337-341 windows 覆盖；AGENTS.md 工具链节；E10 未涉及但约束成立
- 必要性：类别 B（环境整体状态）
- 与 custom 冲突等级：中（环境不对会伪装成依赖失败）
- 建议：暂不采纳任何新增系统依赖；C1 验证前先声明环境（pwsh+MSVC/clang/npx 可用性），环境不齐的失败不计入依赖漂移

## 4. C1 整体状态 vs 功能直接必要的判定汇总

- 判定为类别 A（功能直接必要，缺了编不过或语义缺失）：
- rquickjs 0.11（pijs 运行时）、SWC 七件套（TS strip/resolver/codegen）、wasmtime optional（wasm 宿主）、fs4 1.1（文件锁，已落地）、portable-pty/rustix/win32job（hub PTY/进程树，已落地）、jsonschema 0.42（扩展校验，已落地）、globset 直连（hub/发现闭包，待单加）、fsqlite 0.3.5 native+fts5（jobs/session 全量语义，可选单加）
- 判定为类别 B（上游整体状态，C1 不应顺手跟）：
- asupersync 0.3.9→0.4.4、sysinfo 0.38→0.39、Rust 1.85→1.95、digest 0.10→0.11、tiktoken-rs optional、pprof、htmd、ftui、SWC 大版本抬升、full/default features 重排、外部工具链新增
- 关键结论：hub A1 已证明“hub 全闭包可在 custom 旧运行时上跑通”（E6：check/clippy/fmt 全绿并 push），因此 C1 的依赖策略不应为“跟上游整体状态”，而应为“保旧运行时、按需单加功能缺件”

## 5. 默认依赖策略（C1 integration 线直接执行，无需再议）

- 主依赖 Cargo.toml 与 Cargo.lock 默认取 --ours（保留 custom：asupersync 0.3.9、sysinfo 0.38.2、rquickjs 0.11、SWC 七件套、wasmtime 41.0.3 optional、fs4 1.1、portable-pty 0.9、rustix 1.1.4、jsonschema 0.42 default-features=false、digest 不主动升）
- dev-dependencies 可跟上游（按 E7），但 filetime 不得上主，jsonschema 主/dev 双轨保持同版本
- 只新增依赖的白名单（出现真实 missing crate 才加，不预加）：
- 缺 globset 即加 globset 直连（hub/发现闭包）
- 缺 fsqlite 且 jobs/session 新文件强引用才加 fsqlite 0.3.5 native+fts5（否则不加，见 §3.9）
- 其余 missing crate 一律先回退引用文件到 HEAD，不加依赖去迁就自动合入
- 暂不采纳的黑名单（C1 全程冻结，即使上游文件引用）：
- tiktoken-rs、pprof、htmd、ftui、asupersync 0.4.4、sysinfo 0.39、digest 0.11、SWC 大版本、wasm-host 转默认、jemalloc 进默认、tui 默认变更、jsonschema 打开默认 features
- features 全段默认 --ours；只接受新增 optional 空 feature 这类加法；任何动 default、动 required-features、动 default-features=false 的变化一律拦截并转决策
- 自动合入文件引用新 API（embedded_assets、build.rs 新 crate 引用、digest/swc 新 API）按 SOP 回退该文件到 HEAD，不升级依赖去适配
- 外部工具不新增系统依赖；Windows 验证必须 pwsh+MSVC，freebsd/android bindgen 需 clang，npx 类失败先判环境再判依赖
- 停止条件（来自 E8，依赖侧重述）：一旦出现依赖升级导致大面积错误且无适配计划，C1 当场停止，不叠加 C2

## 6. 仍需编译验证的未知项（本次未运行 cargo，故全部待 integration 线首个 cargo check 确认）

- 未知 1：C1 的 Cargo.toml 是否已把 wasmtime 从 optional 转正或塞进 default；需真实 merge 后检查 features 冲突块，默认 --ours 可拦截但需人眼确认
- 未知 2：C1 的 rquickjs features 是否新增 bindgen 之外的项（如 parallel/serde 等）；需检查 target 覆盖冲突块，避免主依赖行被顺手改
- 未知 3：C1 的 SWC 七件套具体目标版本；当前判“单独升级适配”成立，但若 C1 只是补丁位浮动则可直接收，需 lock 差异复核
- 未知 4：fsqlite 是否为 C1 jobs/session 编译的强引用；当前判“暂不采纳”依赖于“最小闭环可不带 fsqlite”，若 C1 新文件在首个 cargo check 即报 fsqlite 缺失，则转“只新增依赖”
- 未知 5：globset 是否已被 hub A1 遗漏但 C1 真实需要；lock 有传递版打底，首个 check 若报 globset 缺失则单加直连
- 未知 6：tiktoken/pprof/htmd/ftui 在 C1 是否从 optional 变为强引用；当前判冻结，若首个 check 显示新 src 强引用其中之一，需按“回退文件”优先而非“加依赖”处理，并记录冻结例外申请
- 未知 7：digest 双版本在 C1 merge 后是否三版本并存或被收敛；需 lock 差异复核，但策略不变（不主动 update）
- 未知 8：asupersync 0.4.4/sysinfo 0.39 的新 API 是否已被 C1 的 app/cli/config/rpc/session/compaction 注入点调用；若是，冻结该注入点并转运行时升级项目，不在 C1 内解
- 未知 9：Tool::execute 4→5 参与 ExtensionSession/HostActions 2→3 参是否在 C1 已成定局；属 API 漂移而非依赖版本，但会决定是否需要为旧 custom impl 写 shim，首个 check 的错误数可直接判定（E5 基准约 130+12 量级）
- 未知 10：外部工具（clang/MSVC/npx）在 C1 验证环境的可用性；环境缺失导致的失败不得计入依赖漂移，需先在验证记录中声明环境

## 7. 落档说明

- 本次只写本文档，未改源码，未动 Cargo.toml/Cargo.lock，未解任何 merge 冲突
- 下一步 integration 线操作（非本次执行）：从 custom 起临时线，只合 C1 提交，先处理 Git 可确定的 Cargo/features 冲突（默认 --ours），再以首个 cargo check 复核 §6 的 10 个未知项，最后把复核结论回写到本文档 §3 各条目的“复核”附注
- 若首个 check 即出现 §5 停止条件（大面积依赖错误无适配计划），按 merge-checkpoints 停止当前阶段，不得为追平上游叠加变化
