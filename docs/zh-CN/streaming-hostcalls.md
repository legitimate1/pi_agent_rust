# 流式宿主调用结果

从 Rust 向扩展 JavaScript 增量投递宿主调用结果的协议扩展。

## 状态

**草案** — bd-2tl1.1

## 动机

当前的宿主调用协议使用简单的请求/响应模型：

```
Extension JS                         Rust Host
     │                                   │
     │── HostcallRequest { call_id } ──►│
     │                                   │  （完全处理）
     │◄── HostcallOutcome::Success ─────│
     │                                   │
```

对于长时间运行的操作（带流式 stdout 的 exec、大型 HTTP 下载、文件监听），扩展必须等待整个结果完成后才能收到任何数据。流式宿主调用允许增量投递：

```
Extension JS                         Rust Host
     │                                   │
     │── HostcallRequest { stream } ──►│
     │                                   │
     │◄── StreamChunk { seq=0 } ───────│  ← 首个分片
     │◄── StreamChunk { seq=1 } ───────│
     │◄── StreamChunk { seq=2, final } │  ← 末个分片
     │                                   │
```

## 线路格式

### 新增 `HostcallOutcome` 变体

```rust
pub enum HostcallOutcome {
    // 已有变体 —— 保持不变。
    Success(serde_json::Value),
    Error { code: String, message: String },

    // 新增：增量分片投递。
    StreamChunk {
        /// 每个 (call_id) 单调递增。从 0 开始。
        sequence: u64,
        /// 任意 JSON 载荷（stdout 行、HTTP body 字节等）。
        chunk: serde_json::Value,
        /// 最后一个分片为 `true`。此后流完成。
        is_final: bool,
    },
}
```

`call_id` 不在 `StreamChunk` 内重复 —— 它已由外层的 `MacrotaskKind::HostcallComplete { call_id, outcome }` 携带。

### 新增 `MacrotaskKind` 变体

无需新增变体。每个 `StreamChunk` 作为普通的 `HostcallComplete` 宏任务投递：

```rust
MacrotaskKind::HostcallComplete {
    call_id: "hc-42".into(),
    outcome: HostcallOutcome::StreamChunk {
        sequence: 0,
        chunk: json!("first line of stdout\n"),
        is_final: false,
    },
}
```

这复用了现有的调度器队列与确定性排序，无需对 `Macrotask` 结构体或 `tick()` 调度循环做任何改动。

## 流生命周期

### 正常路径

```
seq  call_id  outcome
───  ───────  ─────────────────────────────────────
 0   hc-42    StreamChunk { sequence: 0, chunk: "line 1\n", is_final: false }
 1   hc-42    StreamChunk { sequence: 1, chunk: "line 2\n", is_final: false }
 2   hc-42    StreamChunk { sequence: 2, chunk: "done\n",   is_final: true  }
```

`is_final: true` 之后，不再为 `hc-42` 入队进一步的分片。扩展的异步迭代器在下一次拉取时产出 `{ done: true }`。

### 流中出错

若在已投递一个或多个分片后发生错误，宿主改为入队最终的 `HostcallOutcome::Error`，而非另一个 `StreamChunk`：

```
seq  call_id  outcome
───  ───────  ─────────────────────────────────────
 0   hc-42    StreamChunk { sequence: 0, chunk: "partial", is_final: false }
 1   hc-42    Error { code: "EXEC_FAILED", message: "exit code 1" }
```

JS 桥接将其转换为从迭代器 `next()` 调用中抛出的异常。流被隐式关闭。

### 流中取消

扩展可通过调用 `pi.cancelStream(call_id)`（或丢弃异步迭代器）取消流。宿主：

1. 停止产生分片（终止子进程 / 中止 HTTP 请求）。
2. 入队最终的 `StreamChunk`，其中 `is_final: true` 且分片为空（`json!(null)`），以便 JS 侧可确定性地清理。
3. 不再为此 `call_id` 入队进一步的宏任务。

若宿主已入队尚未被消费的分片，它们仍保留在队列中并正常投递。最终哨兵分片始终是最后入队的项。

### 零分片流

在产生任何数据前就完成的流式宿主调用会发送单个分片：

```
StreamChunk { sequence: 0, chunk: json!(null), is_final: true }
```

这在语义上等价于 `Success(Value::Null)`，但保留了流式契约，使 JS 侧始终使用相同的代码路径。

## 背压模型

### 问题

若 Rust 产生分片的速度快于 JS 消费速度（例如进程以每秒 10,000 行写入 stdout，而扩展每行都做异步工作），无界缓冲将耗尽内存。

### 机制：有界通道

每个流式宿主调用在 Rust 生产者与调度器入队点之间创建一个有界通道：

```
                    ┌─────────────────────┐
Rust producer ──────►│  bounded channel     │──────► Scheduler queue
  (exec/http)        │  capacity = 16       │        (macrotask FIFO)
                    └─────────────────────┘
```

**容量**：16 个分片（可通过 `buffer_size` 选项按流配置，默认 16）。这是可在生产者与调度器之间缓冲的分片数，*并非*宏任务队列中的总分片数。

**生产者阻塞**：当通道已满时，Rust 生产者任务挂起（`channel.send().await`），直到消费者至少腾出一个槽位。这自然将生产者限速至与 JS 消费速度匹配。

**消费者节奏**：JS 侧通过异步迭代器上的 `next()` 调用消费分片。每次 `next()` 调用：

1. 当下一个 `StreamChunk` 宏任务经 `tick()` 投递时 resolve。
2. 处理后，有界通道中的槽位被释放（分片已从通道 → 调度器队列 → JS 投递）。

### 停滞检测

若 JS 消费者 **30 秒**内未调用 `next()`（停滞超时），宿主将该流视为已废弃：

1. 取消生产者（终止子进程、 abort HTTP）。
2. 入队最终哨兵分片（`is_final: true`，`chunk: null`）。
3. 记录警告：`"Stream stalled: JS consumer did not pull for 30s"`。

停滞超时从有界通道变满之时（即生产者被阻塞）开始计时。若通道从未填满，则不会发生停滞。

**停滞超时**可按流通过 `stall_timeout_ms` 选项配置（默认：30,000 ms）。值为 0 时禁用停滞检测。

### 流程图

```
                                             JS tick loop
                                             ┌──────────┐
Rust producer                                │ tick()    │
┌──────────┐     bounded channel (cap=16)    │          │
│ exec     │──►  [c0][c1][c2]...[c15]  ──►  │ deliver_ │
│ stdout   │     ▲                           │ hostcall │
│          │     │ blocks when full           │ _complete│
└──────────┘     │                           │          │
                 │                           │ next()   │
                 └── slot freed when ────────┘ pulls    │
                     chunk delivered               chunk │
                     to JS                              │
                                             └──────────┘
```

## 调度器集成

### 排序

流分片使用现有的基于 `Seq` 的排序。每个分片获得一个独立的 `Macrotask`，其 `seq` 在入队时由 `Scheduler::next_seq()` 分配。这保证：

1. **单流内排序**：同一 `call_id` 的分片按 `sequence` 顺序（0, 1, 2, ...）入队，因此 `seq` 值递增。由于宏任务队列为 FIFO，它们按序投递。

2. **跨流交错**：当多个流同时活跃时，它们的分片按全局 `seq` 顺序交错。当生产者以相似速率让出时，这是自然的轮询。

**示例** —— 两个并发流：

```
Global seq   call_id   sequence   is_final
─────────    ───────   ────────   ────────
    14       hc-42     0          false
    15       hc-99     0          false
    16       hc-42     1          false
    17       hc-99     1          true       ← hc-99 完成
    18       hc-42     2          true       ← hc-42 完成
```

每次 `tick()` 弹出一个宏任务（行为不变）。流分片与非流宿主调用完成在同一队列中共存，无特殊优先级。

### 不保证重排

调度器**不会**重排分片。若分片乱序入队（单生产者单流下不应发生），调度器按入队顺序投递。`sequence` 字段允许 JS 侧在需要时检测空隙，但在正常运行下不会出现空隙。

### 确定性

在 `DeterministicClock` 下，流分片投递完全确定，因为：
- 生产者按固定顺序入队（由任务调度决定）。
- FIFO 队列保持插入顺序。
- `tick()` 一次弹出一个。

## 按需启用机制

流式按宿主调用按需启用。扩展通过在宿主调用载荷中设置 `stream: true` 来请求流式：

```javascript
// 非流式（已有行为，保持不变）
const result = await pi.exec("ls -la");
// result 为完整输出字符串

// 流式（新增）
const stream = await pi.exec("tail -f /var/log/syslog", { stream: true });
for await (const chunk of stream) {
  console.log("got:", chunk);
}
```

### 支持流式的宿主调用类型

| Kind   | 流式支持 | 分片载荷 |
|--------|----------|----------|
| `Exec` | 是 | `string`（stdout/stderr 行） |
| `Http` | 是 | `string`（body 分片） |
| `Tool` | 否 | — |
| `Session` | 否 | — |
| `Events` | 否 | — |
| `Ui`   | 否 | — |

非流式类型会忽略 `stream: true` 标志并返回正常的 `Success`/`Error` 结果。

### Rust 分发中的检测

```rust
fn dispatch_hostcall_allowed(
    &self,
    request: &HostcallRequest,
    // ...
) -> Result<()> {
    let wants_stream = request.payload
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match request.kind {
        HostcallKind::Exec if wants_stream => {
            self.dispatch_exec_streaming(request).await
        }
        HostcallKind::Exec => {
            self.dispatch_exec(request).await  // 已有路径
        }
        // ...
    }
}
```

## JS 桥接

### `deliver_hostcall_completion` 扩展

`extensions_js.rs` 中现有的 `deliver_hostcall_completion` 函数被扩展以处理新变体：

```rust
fn deliver_hostcall_completion(
    ctx: &Ctx<'_>,
    call_id: &str,
    outcome: &HostcallOutcome,
) -> rquickjs::Result<()> {
    let global = ctx.globals();
    let complete_fn: Function<'_> = global.get("__pi_complete_hostcall")?;

    let js_outcome = match outcome {
        HostcallOutcome::Success(value) => { /* 已有 */ }
        HostcallOutcome::Error { code, message } => { /* 已有 */ }
        HostcallOutcome::StreamChunk { sequence, chunk, is_final } => {
            let obj = Object::new(ctx.clone())?;
            obj.set("stream", true)?;
            obj.set("sequence", *sequence)?;
            obj.set("chunk", json_to_js(ctx, chunk)?)?;
            obj.set("isFinal", *is_final)?;
            obj
        }
    };

    complete_fn.call::<_, ()>((call_id, js_outcome))?;
    Ok(())
}
```

### JS 侧 `__pi_complete_hostcall`

JS 侧处理器检查 `outcome.stream`：

```javascript
function __pi_complete_hostcall(call_id, outcome) {
  const pending = __pi_pending_hostcalls.get(call_id);
  if (!pending) return;

  if (outcome.stream) {
    // 将分片推入流的内部缓冲区。
    // 异步迭代器的 next() 从此缓冲区拉取。
    pending.pushChunk(outcome.chunk, outcome.isFinal);
    if (outcome.isFinal) {
      __pi_pending_hostcalls.delete(call_id);
    }
    return;
  }

  // 非流式：已有的 resolve/reject 逻辑。
  __pi_pending_hostcalls.delete(call_id);
  if (outcome.ok) {
    pending.resolve(outcome.value);
  } else {
    pending.reject(new Error(`${outcome.code}: ${outcome.message}`));
  }
}
```

### 异步迭代器实现

每个流式宿主调用返回一个实现异步迭代器协议的对象：

```javascript
class HostcallStream {
  constructor(callId) {
    this.callId = callId;
    this.buffer = [];       // 已收到但尚未拉取的分片
    this.waitResolve = null; // 等待中 next() 的 resolve 函数
    this.done = false;
    this.error = null;
  }

  pushChunk(chunk, isFinal) {
    if (isFinal) {
      this.done = true;
    }
    if (this.waitResolve) {
      // 消费者正在等待 —— 立即投递。
      const resolve = this.waitResolve;
      this.waitResolve = null;
      resolve({ value: chunk, done: isFinal && chunk === null });
    } else {
      // 缓冲待后续拉取。
      this.buffer.push({ chunk, isFinal });
    }
  }

  async next() {
    if (this.buffer.length > 0) {
      const { chunk, isFinal } = this.buffer.shift();
      return { value: chunk, done: isFinal && chunk === null };
    }
    if (this.done) {
      return { value: undefined, done: true };
    }
    // 等待下一个分片投递。
    return new Promise(resolve => {
      this.waitResolve = resolve;
    });
  }

  [Symbol.asyncIterator]() { return this; }
}
```

## 边界情况

### 1. 流中取消

**触发**：扩展丢弃异步迭代器（例如 `for await` 中的 `break`）或调用 `pi.cancelStream(callId)`。

**序列**：
1. JS 调用原生函数 `__pi_cancel_stream(call_id)`。
2. Rust 收到取消信号，终止子进程 / 中止 HTTP。
3. Rust 排空有界通道（丢弃已缓冲的分片）。
4. Rust 入队 `StreamChunk { sequence: N, chunk: null, is_final: true }`。
5. JS 迭代器在下一次拉取时产出 `{ done: true }`。

**不变量**：即使在取消时也始终投递恰好一个最终分片。

### 2. 流中出错

**触发**：子进程以非零退出、HTTP 连接断开、超时。

**序列**：
1. 生产者检测到错误。
2. 生产者为该 `call_id` 入队 `HostcallOutcome::Error { code, message }`（而非 `StreamChunk`）。
3. JS 桥接将其转换为从 `next()` 抛出的异常。
4. 不再入队进一步的分片。

**注意**：已在通道或宏任务队列中缓冲的分片会在错误之前投递。错误始终是此 `call_id` 的最后一项。

### 3. 背压停滞

**触发**：JS 消费者在生产者有数据时停止调用 `next()`。

**序列**：
1. 生产者填满有界通道（16 个分片）。
2. 生产者在 `channel.send().await` 上阻塞。
3. 停滞计时器启动（默认 30s）。
4. 30s 内无消费者进展后：
   - 取消生产者。
   - 入队最终哨兵分片。
   - 记录警告。

**恢复**：扩展可通过处理最终分片并检查哨兵值（`null`）来捕获停滞。

### 4. 流期间扩展卸载

**触发**：扩展在流活跃时被卸载（例如 `ExtensionRegion` 被丢弃）。

**序列**：
1. `ExtensionRegion::drop()` 按预算发起清理。
2. 此扩展的所有活跃流被取消（与流中取消相同）。
3. 有界通道被丢弃，从而解除生产者阻塞。
4. 生产者检测到通道已关闭并停止。

### 5. 多个并发流

来自同一扩展或不同扩展的多个流无干扰共存：

- 每个流拥有独立的有界通道。
- 每个流拥有独立的 `sequence` 计数器（从 0 开始）。
- 调度器按全局 `seq` 顺序交错来自所有流的分片。
- 背压按流隔离（一个慢消费者不会阻塞其他）。

### 6. 带 `DeterministicClock` 的流

在确定性测试下：
- 生产者同步入队所有分片（无真实 I/O）。
- 宏任务队列以已知顺序包含所有分片。
- `tick()` 一次投递一个，允许在每个分片后做断言。

## 配置

| 参数 | 默认值 | 作用域 | 说明 |
|------|--------|--------|------|
| `stream` | `false` | 按调用 | 为此次宿主调用启用流式 |
| `buffer_size` | `16` | 按调用 | 有界通道容量（分片数） |
| `stall_timeout_ms` | `30000` | 按调用 | 自动取消前的最大空闲时间（0 = 禁用） |

它们在宿主调用载荷中传递：

```javascript
const stream = await pi.exec("make build", {
  stream: true,
  buffer_size: 32,       // 为突发输出使用更大缓冲
  stall_timeout_ms: 0,   // 禁用停滞检测
});
```

## 向后兼容

- 非流式宿主调用完全保持不变。
- 不支持流式的宿主调用类型会忽略 `stream: true` 标志并返回正常的 `Success`/`Error` 结果。
- 未使用流式的扩展看不到任何行为差异。
- `HostcallOutcome::StreamChunk` 变体是增量添加 —— 对 `Success`/`Error` 的已有 match 分支继续工作（Rust 会要求新增分支，但那是编译时检查，而非运行时破坏）。
