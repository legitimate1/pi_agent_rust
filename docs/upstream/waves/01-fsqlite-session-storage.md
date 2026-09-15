# 波次 01：fsqlite session storage

## 分析目的

确认上游在第一个小窗口内对 session storage 做了什么变化，判断这些变化是否构成独立功能闭包，以及当前 `custom` 是否应该采纳。

本波次只做语义分析，不执行 merge、不移植源码、不修改 Cargo 配置。

## 分析边界

```text
上游起点 S：226a876425a856f657b2a5d7c7ac6f0ca1ad25f1
上游终点 T：8f12352e174ea06c7d8d66cde15768cecfccccf3

T 的标题：chore(beads): close bd-oc1wu (fsqlite cutover landed)
```

`S..T` 包含 16 个提交、69 个变更文件，净变化为 `+3412 / -1119`。本窗口中的功能核心是：

```text
432c90cc583e9924dd4b0a71a9a309736b214814
feat(session): cut session storage over to fsqlite 0.3.4 (bd-oc1wu)
```

其余同时间段提交包含工具、扩展、压缩、测试门禁和文档等变化，不能因为处在同一个 Git 区间就自动归入本波次。

## 一句话结论

上游完成的是**SQLite session 后端替换闭包**，不是新的 session 产品能力：用 `fsqlite 0.3.4` 替换 `sqlmodel-sqlite`/`sqlmodel-core`，同时引入专用 SQLite 线程、严格多进程打开、完整侧车文件管理和类型化错误处理。当前 `custom` 已经具备 SQLite session 能力，但仍使用 `sqlmodel-*` 和 `asupersync 0.3.9`；因此本波次不直接 merge，暂时冻结为独立的后端迁移议题。

## 上游最终变化

### 后端和依赖替换

上游终点的 `Cargo.toml` 使用：

```text
fsqlite = { version = "0.3.4", default-features = false, features = ["native"] }
```

并移除 session 路径使用的：

```text
sqlmodel-sqlite
sqlmodel-core
```

同时上游终点使用 `asupersync 0.4.4` 和 `sha2 0.11`。这两个版本不能直接视为本波次可以顺带接受的依赖，因为当前 `custom` 仍使用 `asupersync 0.3.9`、`sha2 0.10`，且其他模块还依赖旧 API。

### SQLite 连接模型

`src/session_sqlite.rs` 新增了一个同步外观：

```text
fsqlite::Connection（异步、!Send）
        ↓
专用 16 MiB 栈线程
        ↓
SqliteConnection 同步方法
```

上游这样处理的原因是：

- fsqlite 的 engine future 较深，默认线程栈可能不足；
- connection future 不能跨线程发送；
- 在专用线程中使用普通 `block_on`，避免和当前 runtime 互相阻塞。

写连接使用：

```text
open_strict_multi_process
PRAGMA busy_timeout = 5000
```

只读连接使用：

```text
open_schema_only
```

这意味着后端替换不仅是类型名替换，还改变了连接打开、线程调度、只读行为和多进程协作边界。

### SQLite 侧车文件

上游把需要检查、统计、保护权限和删除的侧车文件从传统的 WAL/SHM 扩展为完整集合：

```text
-wal
-shm
-journal
-fsqlite-ns-gate
-fsqlite-ns-use
-wal-cert
-wal-cert-head
```

这些路径会影响：

- 读写前置权限检查；
- 私有权限设置；
- session 文件大小和修改时间统计；
- 删除 session 时的清理；
- 只读数据库在缺失侧车文件时的行为。

### 错误和诊断

上游把 SQLite 错误从 `sqlmodel_core::Error` 切换为 `fsqlite::FrankenError`，并将部分字符串匹配改为类型匹配，例如：

```text
Busy
BusyRecovery
BusySnapshot
LockFailed
MultiProcessContractViolation
DatabaseCorrupt
WalCorrupt
NotADatabase
NoSuchTable
```

因此受影响的不只有 `session_sqlite.rs`，还包括：

```text
src/error.rs
src/error_hints.rs
src/doctor.rs
相关错误和 session 测试
```

### Session index 和 picker

`src/session_index.rs` 和 `src/session_picker.rs` 同步切换到 fsqlite 的连接、行和参数类型，并通过专用 SQLite 线程运行连接操作。

这同时带来：

- 行读取从按列名读取转为按位置读取；
- session index 的数据库连接生命周期变化；
- index lock 超时和连接关闭行为变化；
- picker 对完整 SQLite 侧车集合的发现、统计和删除行为变化。

### 测试和发布耦合

上游补充或调整了：

- SQLite 只读打开测试；
- 侧车缺失、权限和符号链接测试；
- 多进程 writer/reader 测试；
- fsqlite 错误类型和诊断测试；
- session index 与 picker 测试。

同时该提交还把 release size budget 从 22 MiB 调到 26 MiB，因为 fsqlite engine 会增加二进制体积。这是发布预算变化，不属于 session 语义闭包，应从本波次的核心采纳范围中分离出来。

## 功能闭包

本波次的真正功能闭包是：

```text
fsqlite 依赖
    ↓
session_sqlite 连接和事务适配
    ↓
session.rs 的 SQLite save/load/append 接入
    ↓
session_index 的 SQLite index 接入
    ↓
session_picker 的侧车发现和删除
    ↓
错误分类、权限、只读、多进程和恢复测试
```

因此不能只移植一个 `src/session_sqlite.rs` 文件。至少需要同时核对依赖、错误映射、session index、picker、doctor 和相关测试。

## 当前 custom 的对应实现

当前 `custom` 已经拥有 SQLite session backend：

- `Cargo.toml` 的 `sqlite-sessions` feature 默认启用；
- `Cargo.toml` 使用 `sqlmodel-sqlite 0.2.2` 和 `sqlmodel-core 0.2.2`；
- `src/session_sqlite.rs` 使用 `sqlmodel_core` 和 `sqlmodel_sqlite`；
- `src/session_index.rs` 使用 `sqlmodel_core::Value` 和 `sqlmodel_sqlite::SqliteConnection`；
- `src/session.rs` 已有 `SessionStoreKind::Sqlite`、SQLite 打开、保存和追加路径；
- `src/session_picker.rs` 已处理 SQLite 的 `-wal` 和 `-shm` 侧车文件。

也就是说，当前两边的关系不是：

```text
custom 没有 SQLite
upstream 新增 SQLite
```

而是：

```text
custom：已有 SQLite session，使用 sqlmodel/libsqlite3 路径
upstream：把已有 SQLite session 改为 fsqlite/纯 Rust 路径
```

## 语义差异

### 可以视为互补的部分

上游提供了当前 custom 尚未具备的后端属性：

- 不再依赖 `libsqlite3-sys` 的 C 实现路径；
- 对 fsqlite 的多进程和侧车语义有明确处理；
- 错误可以按 fsqlite 类型分类，而不是依赖错误字符串；
- 只读打开和专用线程行为有专门测试。

### 不能视为简单替换的部分

以下变化会穿透 session 模块边界：

- `fsqlite` 的异步连接必须适配到当前 custom 的 runtime 和线程模型；
- `fsqlite::FrankenError` 会改变 `Error::Sqlite` 及错误提示接口；
- 侧车集合变化会影响当前 custom 的文件触达、统计、删除和权限逻辑；
- `open_schema_only` 的只读语义需要重新核对 custom 的路径权限约束；
- 上游的 `asupersync 0.4.4` 不能与当前 `custom` 的 0.3.9 一起顺手升级；
- fsqlite 会增加发布二进制体积，需要重新判断项目的 size budget。

## 为什么不直接 merge

当前不直接 merge 的原因不是 Git 冲突数量，而是语义成本：

1. 这是已有后端的替换，不是必须立即获得的新产品功能。
2. 直接采纳会连带 `fsqlite` 依赖树、`asupersync` API、错误类型和侧车契约。
3. 当前 custom 的 session、index、picker、error 和权限逻辑已经有自己的定制边界。
4. 上游提交还混入 release size budget 和其他同日的修复，直接按提交合入会失去范围归因。
5. 即使 Git 冲突解决，仍必须重新验证旧 SQLite 文件、WAL/SHM、只读、并发、权限和删除行为。

## 处理结论

```text
不直接 merge。
不把 fsqlite 视为普通上游功能波次。
暂时冻结，必要时另立“SQLite 后端迁移”波次。
```

如果未来决定采用 fsqlite，下一波不应从 `git merge 432c90cc...` 开始，而应先建立独立迁移设计，至少拆成：

```text
1. fsqlite 与当前 Cargo/runtime 的兼容性
2. session_sqlite 连接和线程适配
3. session index / picker 的侧车契约
4. Error::Sqlite 和错误提示迁移
5. 旧 SQLite 文件兼容与只读行为
6. 多进程、权限、删除和恢复验证
7. 发布体积预算重新评估
```

## 未纳入本波次的变化

`S..T` 内以下内容不纳入 fsqlite session storage 波次：

- 工具和搜索后端变化；
- extension、compact、provider 和 RPC 的同日变化；
- `.beads`、WAL certification、tracker 等上游内部数据；
- 单纯格式、Clippy、文档和 README 更新；
- 与 fsqlite 无直接因果关系的测试修复；
- release workflow 和 size budget 的独立治理变化。

它们应在相应的主题波次中重新分析，不能因为处在同一个 16-commit 窗口就一起处理。

## 证据

- 上游功能核心提交：`432c90cc583e9924dd4b0a71a9a309736b214814`
- 上游窗口终点：`8f12352e174ea06c7d8d66cde15768cecfccccf3`
- 当前 custom：`d657691303b8352179b4e67f176f69e11fbc0207`
- 上游直接影响的核心路径：`Cargo.toml`、`Cargo.lock`、`src/session.rs`、`src/session_index.rs`、`src/session_picker.rs`、`src/session_sqlite.rs`、`src/error.rs`、`src/error_hints.rs`、`src/doctor.rs` 及相关 session 测试。
- 当前 custom 的旧后端依据：`Cargo.toml` 中的 `sqlmodel-sqlite`/`sqlmodel-core`，以及 `src/session_sqlite.rs`、`src/session_index.rs` 的对应导入。
