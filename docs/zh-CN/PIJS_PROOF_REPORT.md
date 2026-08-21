# PiJS 论证报告：在无需 Node/Bun 的前提下实现安全、性能与确定性

> 自包含证据，证明 Pi 扩展运行时（QuickJS + 能力门控连接器）在运行第三方扩展方面比 Node/Bun 更安全、更确定、更高效。

---

## 1. 执行摘要

Pi 在内嵌 QuickJS 运行时中运行 JS/TS 扩展，不依赖 Node.js 或 Bun。这不是限制——而是有意为之的安全与性能优势。

**关键主张，均有可复现证据支撑：**

| 主张 | 证据 |
|---|---|
| 已认证必过扩展集完全通过（`123/123`），全清单健康度为 `221/224`（`98.7%`） | `tests/ext_conformance/reports/gate/must_pass_gate_verdict.json`、`tests/ext_conformance/reports/health_delta/health_delta_report.json` |
| 场景一致性套件完全通过（`25/25`） | `tests/ext_conformance/reports/scenario_conformance.json` |
| 差异对等分流样本零不匹配（`22` 匹配，`0` 不匹配，`3` 跳过） | `tests/ext_conformance/reports/parity/triage.json` |
| 冷加载 P95 = 106ms（debug 构建） | `ext_bench_baseline.json` |
| 热加载 P99 < 1ms | 性能基准（§5） |
| 事件分发 P99 = 616us | 性能基准（§5） |
| 无环境 OS 访问 | 能力模型（§2） |
| 30 项负向安全测试通过 | `ext_conformance_negative.rs` |
| 测试下确定性执行 | LabRuntime + 事件循环规范（§4） |

---

## 2. 安全论证

### 2.1 Node/Bun 在扩展方面的問題

Node.js 与 Bun 默认给予扩展**环境授权**：

- 完整文件系统访问（`fs.*` 无路径限制）
- 无限制网络访问（`http`、`net`、`tls`、`dgram`）
- 进程派生（`child_process.spawn`、`cluster`、`worker_threads`）
- 环境变量访问（`process.env`——包括 API 密钥）
- 原生插件加载（`process.dlopen`、`.node` 文件）
- 调试器/检查器访问

在 Node/Bun 中运行的恶意或有缺陷扩展可读取你的 SSH 密钥、外泄 API 令牌、派生后台进程并修改任意文件。唯一的防御是对扩展作者的信任。

### 2.2 PiJS：能力门控连接器

PiJS 反转了安全模型。扩展具有**零环境授权**。每个有副作用的操作都必须经过显式宿主连接器：

| 操作 | 连接器 | 所需能力 |
|---|---|---|
| 读取文件 | `pi.tool("read")` 或 `pi.fs.read()` | `read` |
| 写入文件 | `pi.tool("write")` 或 `pi.fs.write()` | `write` |
| 执行命令 | `pi.exec()` 或 `pi.tool("bash")` | `exec` |
| HTTP 请求 | `pi.http()` | `http` |
| 会话元数据 | `pi.session.*` | `session` |
| UI 提示 | `pi.ui.*` | `ui` |

每个连接器调用都会：
1. **策略检查**——对照扩展已授予能力进行检查
2. **日志记录**——在结构化审计账本（`pi.ext.log.v1`）中记录
3. **作用域限定**——在适用时进行路径限制、主机白名单
4. **超时强制**——支持取消

### 2.3 被阻止的内容

以下内容因构造而被阻止（非策略——它们在运行时中不存在）：

| 被阻止能力 | 原因 |
|---|---|
| 原始文件系统访问 | 无 `fs` 模块；仅通过连接器中介的读写 |
| 原始网络套接字 | 无 `net`/`tls`/`dgram` 模块 |
| 原生插件 | QuickJS 无 `dlopen` 或 `.node` 加载 |
| Worker 线程 | QuickJS 按设计为单线程 |
| `vm` 模块（代码求值） | 未提供；`eval()` 仅对已打包代码有效 |
| `inspector`/`repl` | 未提供 |
| `process.binding()` | 未提供 |

### 2.4 安全证据

**30 项负向测试**验证恶意扩展被正确拒绝：

```bash
cargo test --test ext_conformance_negative --features ext-conformance -- --nocapture
```

这些测试覆盖：禁用 API 使用、能力拒绝、路径遍历尝试、超大载荷与格式错误的注册载荷。

**护栏测试**验证能力强制：

```bash
cargo test --test ext_conformance_guard --features ext-conformance -- --nocapture
```

---

## 3. 兼容性论证

### 3.1 怀疑者的担忧

> “QuickJS 只是一个 JS 引擎。没有 Node API，任何真实的东西都无法工作。”

这是狭义上正确的担忧。仅靠 QuickJS 无法运行导入 `node:fs`、`node:path` 或 `node:child_process` 的扩展。

### 3.2 答案：Node API 垫片

Pi 为扩展实际使用的 Node API 提供针对性垫片。这些并非完整的 Node 实现——而是 Pi 能力门控连接器之上的薄包装：

| Node API | PiJS 垫片 | 覆盖率 |
|---|---|---|
| `node:fs` | `readFileSync`、`writeFileSync`、`existsSync`、`readdirSync`、`statSync`、`mkdirSync`、`realpathSync`、promises API | 足以覆盖 95%+ 扩展 |
| `node:path` | `join`、`resolve`、`dirname`、`basename`、`extname`、`sep`、`delimiter` | 完整 |
| `node:os` | `platform`、`homedir`、`tmpdir`、`hostname`、`type`、`arch`、`EOL` | 完整 |
| `node:crypto` | `randomBytes`、`createHash`、`randomUUID` | 常用子集 |
| `node:url` | `URL`、`parse`、`fileURLToPath`、`pathToFileURL` | 完整 |
| `node:child_process` | `spawn`、`exec`、`execSync` | 经由 `exec` 能力 |
| `node:readline` | 基础 `createInterface` | 足够 |
| `node:module` | `createRequire` 存根 | 足够 |
| `node:util` | `format`、`inherits`、`types`、`stripVTControlCharacters` | 常用子集 |

16+ 个 npm 包存根覆盖常见第三方依赖（`node-pty`、`chokidar`、`jsdom`、`turndown`、`@opentelemetry/*` 等）。

### 3.3 存在性证明：txiki.js

我们并非首个在无需 Node/Bun 的前提下基于 QuickJS 构建可用 JS 运行时的团队。[txiki.js](https://github.com/saghul/txiki.js)（QuickJS-ng + libuv）证明了围绕 QuickJS 的非 Node 事件循环 + OS 包装层是可行且实用的。PiJS 通过使 OS 层改为能力门控而非环境授权，走得更远。

### 3.4 一致性证据

当前已认证一致性状态：
- 必过扩展门：`123/123` 通过，`0` 失败，`0` 跳过。
- 信息性 stretch 集：`98/101` 通过，`3` 失败，`0` 跳过。
- 全清单健康度：`221/224` 通过（`98.7%`），`0` 回归，相对 2026-02-07 基线 `34` 项修复。

源制品：
- `tests/ext_conformance/reports/gate/must_pass_gate_verdict.json`（`generated_at=2026-05-01T03:20:54.460Z`）
- `tests/ext_conformance/reports/health_delta/health_delta_report.json`（`generated_at=2026-05-01T04:10:28.479Z`）

历史性的 `224/224` 与 `187/223` 数据仅作为更早的归档上下文保留；不覆盖上述当前必过/全清单划分。

**复现：**

```bash
cargo test --test ext_conformance_generated --features ext-conformance -- conformance_must_pass_gate --nocapture --exact
cargo test --test ext_conformance_generated --features ext-conformance -- conformance_health_delta --nocapture --exact
```

---

## 4. 确定性论证

### 4.1 为何确定性重要

非确定性扩展运行时会导致：
- 不稳定测试（不同运行产生不同结果）
- 不可复现缺陷（扩展在一台机器上工作，在另一台上失败）
- 安全审计缺口（无法证明实际发生了什么）

### 4.2 PiJS 事件循环：形式化状态机

PiJS 事件循环被规范为确定性状态机（EXTENSIONS.md §1A.4.5）：

- **每 tick 一个宏任务**（无交错）
- 每个宏任务后**微任务排空至不动点**
- 通过单调 `seq` 计数器实现**全序**（确定性平局打破）
- 计时器按 `(deadline_ms, seq)` 排序——在相等截止时间下稳定
- **无重入**——宿主调用完成入队宏任务，永不同步重入 JS

### 4.3 LabRuntime：确定性测试

`asupersync` 运行时提供 `LabRuntime`，可完全控制调度、时间与 IO。测试可以：

- 确定性地推进时间
- 控制任务调度顺序
- 在精确点注入宿主调用完成
- 断言精确执行轨迹

```bash
# 运行确定性扩展测试
cargo test ext_lab --features ext-conformance -- --nocapture
```

### 4.4 确定性证据

给定相同输入（制品字节、事件序列、宿主调用结果、时钟），PiJS 产生相同输出。这通过以下方式验证：

- 黄金夹具对比（16 个代表性扩展）
- 差异对等分流样本（TS vs Rust）：`22` 匹配，`0` 不匹配，`3` 跳过（`25` 总计）
- 基于属性的测试（13 个 proptest 套件，每套 512 用例）

---

## 5. 性能论证

### 5.1 为何 PiJS 更快

Node/Bun 支付了 PiJS 所避免的启动成本：

| 阶段 | Node.js | PiJS |
|---|---|---|
| 运行时初始化 | 200-500ms | 0ms（内嵌） |
| 模块加载（require/import） | 100-300ms | <1ms（虚拟模块） |
| JIT 预热 | 50-100ms | 0ms（解释器） |
| V8 堆分配 | 50-100MB | <5MB（QuickJS） |

对于扩展（小代码、短执行），V8 的 JIT 优势无关紧要。扩展注册工具并响应事件——它们不运行紧密计算循环。

### 5.2 实测性能

在 103 个安全扩展上基准测试，每项 10 次迭代（debug 构建）：

| 指标 | 值 |
|---|---|
| 冷加载 P50 | 77ms |
| 冷加载 P95 | 106ms |
| 冷加载 P99 | 134ms |
| 热加载 P50 | 333us |
| 热加载 P95 | 734us |
| 热加载 P99 | 926us |
| 事件分发 P99 | 616us |
| 最快冷加载 | 67ms（trigger-compact） |
| 最慢冷加载 | 126ms（hjanuschka-plan-mode） |

Release 构建快 5-10 倍（预期冷加载约 5-10ms）。

### 5.3 性能预算

通过 `tests/perf_budgets.rs` 在 CI 中强制：

| 预算 | 阈值 | 状态 |
|---|---|---|
| 冷加载 P95 | < 200ms | PASS（106ms） |
| 热加载 P95 | < 100ms | PASS（734us） |
| 事件分发 P99 | < 5ms | PASS（616us） |

### 5.4 复现

```bash
# PR 模式（10 个多样扩展，快速）
PI_BENCH_MODE=pr cargo test --test ext_bench_harness \
  --features ext-conformance -- --nocapture

# 全量语料（103 个扩展，全面）
PI_BENCH_MODE=nightly PI_BENCH_MAX=103 PI_BENCH_ITERATIONS=10 \
  cargo test --test ext_bench_harness --features ext-conformance -- --nocapture
```

---

## 6. 对比表

| 属性 | Node.js | Bun | PiJS（QuickJS） |
|---|---|---|---|
| **安全模型** | 环境授权 | 环境授权 | 能力门控 |
| **审计日志** | 手动 | 手动 | 按宿主调用内置 |
| **启动时间** | 200-500ms | 50-100ms | <1ms（热） |
| **内存基线** | 50-100MB | 30-50MB | <5MB |
| **确定性** | 非确定 | 非确定 | 确定（形式化） |
| **测试框架** | 手动搭建 | 手动搭建 | LabRuntime（内置） |
| **原生插件** | 支持（有风险） | 支持（有风险） | 已阻止（安全） |
| **WebAssembly** | 内置 | 内置 | 经由 wasmtime 桥接* |
| **兼容性** | 100% Node API | ~98% Node API | `123/123` 已认证必过扩展；`221/224` 全清单健康度（测试框架范围） |
| **依赖** | node 二进制（80MB+） | bun 二进制（60MB+） | 内嵌（0 字节） |

*PiWasm 桥接在 `wasm-host` 之后对需要 WebAssembly 的扩展可用。它仅链接显式宿主导入与有界 Emscripten 兼容存根；不支持的导入在实例化期间以 fail-closed 方式失败。当前语料中没有扩展将其作为运行时依赖。

---

## 7. 复现步骤

所有证据均可从零开始重新生成：

```bash
# 1. 一致性（全部 224 个当前供应扩展）
cargo test --test ext_conformance_generated --features ext-conformance -- \
  conformance_sharded_matrix --nocapture --exact

# 2. 性能（103 个安全扩展）
PI_BENCH_MODE=nightly PI_BENCH_MAX=103 PI_BENCH_ITERATIONS=10 \
  cargo test --test ext_bench_harness --features ext-conformance -- --nocapture

# 3. 安全（30 项负向测试）
cargo test --test ext_conformance_negative --features ext-conformance -- --nocapture

# 4. 确定性（基于属性）
cargo test extensions_property --features ext-conformance -- --nocapture

# 5. 预算合规
cargo test --test perf_budgets --features ext-conformance -- --nocapture
```

**生成制品：**

| 制品 | 位置 |
|---|---|
| 一致性分片报告 | `tests/ext_conformance/reports/sharded/shard_0_report.json` |
| 场景一致性报告 | `tests/ext_conformance/reports/scenario_conformance.json` |
| 对等分流报告 | `tests/ext_conformance/reports/parity/triage.json` |
| 性能基线 | `tests/perf/reports/ext_bench_baseline.json` |
| 性能报告 | `tests/perf/reports/BASELINE_REPORT.md` |
| 预算摘要 | `tests/perf/reports/budget_summary.json` |
| 扩展目录 | `docs/extension-catalog.json` |

---

## 8. 参考

1. **QuickJS** — Fabrice Bellard. https://bellard.org/quickjs/
   - 字节码与版本耦合，执行前不做安全检查。
   - 作业队列（`JS_ExecutePendingJob`）必须由嵌入方驱动。

2. **txiki.js** — Saghul. https://github.com/saghul/txiki.js
   - 存在性证明：无需 Node 的 QuickJS + libuv 事件循环 + OS 包装。

3. **wasmtime** — Bytecode Alliance. https://wasmtime.dev/
   - 用于 WASM 扩展的组件模型（Tier A 运行时）。

4. **EXTENSIONS.md** — Pi Agent Rust 扩展系统架构。
   - §1A.4: PiJS 运行时契约（事件循环状态机）。
   - §2A: Extc 兼容性契约（重写规则、禁用 API）。
   - §3.2A: 统一能力模型。
