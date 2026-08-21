# AzureCanyon 评审发现

## 评审轮次：`src/config.rs`

- 追踪了经由 `Config::patch_settings_with_roots`、CLI 配置命令及测试的设置加载与补丁流程。
- 发现设置补丁写入虽使用原子重命名，但未在 Pi 进程间串行化 read/merge/write。
- 已在 `f3120138` 中修复：新增进程互斥锁、建议锁文件、父目录 fsync，以及并发更新回归测试。

## 交叉评审：`75f0f6d9`

- 针对压缩/权限加固提交，评审了溢出、过期边界与持久化回归。
- 发现 `PermissionStore` 存在相同的 read/merge/write 丢失更新竞态：两个并发打开的存储可能相互覆盖 allow/deny 决策。
- 已在 `d2d864af` 中修复：新增进程互斥锁、建议锁文件、加锁下重载事务、父目录 fsync，以及并发记录回归测试。

## 验证

- `rch exec -- ... cargo fmt --check` 通过。
- `rch exec -- ... cargo check --all-targets` 通过。
- `rch exec -- ... cargo clippy --all-targets -- -D warnings` 通过。
- 聚焦配置回归通过：`cargo test --lib config::tests::patch_settings_serializes_concurrent_updates`。
- 全量 `cargo test` 与更广的 `cargo test --lib` 因共享磁盘耗尽而中止/失败；观测到的失败为 `StorageFull` / `database or disk is full`，而非被审代码中的断言失败。
