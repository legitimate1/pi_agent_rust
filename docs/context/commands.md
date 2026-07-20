# 开发命令

## 构建

```bash
cargo check              # 快速检查编译错误
cargo build              # Debug 构建（LLD 链接器 + sccache 缓存）
cargo build --release    # 发布构建（opt-level=3 + thin LTO + panic=abort）
```

> ⚠️ **构建注意事项**：`cargo build --release` 耗时 5-10 分钟，请在**新开的终端窗口**中执行构建，不要占用当前交互终端。使用 `Start-Process` 或手动开新窗口。

### 部署

构建完成后，用脚本一键停进程+覆盖（脚本还会自动更新 cargo-sweep 时间戳，为下次清理做准备）：

```powershell
.\scripts\deploy-release.ps1
```

### target/ 磁盘空间管理

Cargo 设计上永不删除旧编译产物，每次构建（`cargo build`/`cargo test`）生成新的 hash 文件，日积月累会导致 `target/` 膨胀到数百 GB。使用 `cargo-sweep` 管理：

```bash
# 安装
cargo install cargo-sweep

# 标记当前构建时间戳（部署时自动执行）
cargo sweep --stamp

# 清理所有比标记时间戳旧的产物（回收几十~几百 GB）
cargo sweep --file

# 或按天数清理
cargo sweep --time 30
```

部署脚本 `deploy-release.ps1` 已在末尾自动执行 `cargo sweep --stamp`，每次部署后跑一次 `cargo sweep --file` 即可清理旧产物。

### 构建优化配置（`.cargo/config.toml`）

- **LLD 链接器**：`lld-link.exe` 替代 MSVC `link.exe`，链接速度快 3-5x
- **sccache**：编译器缓存，`cargo clean` / 切换分支后大幅加速冷启动
- **Dev profile**：`debug = "line-tables-only"`，减少 debug 信息量以加速编译
- **Defender 排除**：项目 `target/` 已加入 Windows Defender 排除列表



## 测试

```bash
cargo test               # 全部测试
cargo test -- --nocapture  # 带输出
cargo test conformance   # 一致性测试
cargo test sse::tests    # 特定模块测试
```

## 代码质量

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Release 构建

默认 profile：
```toml
opt-level = 3       # 速度优化
lto = "thin"        # 薄 LTO，兼顾编译速度与代码质量
panic = "abort"
strip = true
```

## CLI 用法

```bash
pi [OPTIONS] [ARGS]...
pi --print "message"         # 非交互模式
pi --model <MODEL> "..."     # 指定模型
pi --thinking <LEVEL>        # 思考级别
pi --no-tools                # 禁用全部内置工具
pi --tools read,write,edit   # 仅启用指定工具
pi --provider <PROVIDER>     # 指定 Provider
```

## 工具配置

### 运行时禁用工具

在 `settings.json` 中设置 `disabledTools` 数组，启动时自动过滤：

```json
{
  "disabledTools": ["bash"]
}
```

支持 `camelCase`（`disabledTools`）和 `snake_case`（`disabled_tools`）两种格式。

### `--tools` CLI 参数

覆盖启用的工具列表（逗号分隔）：
```bash
pi --tools read,write,edit,grep,find,ls,hashline_edit,pwsh
```

## 版本迁移注意事项

- `asupersync` 和 `rich_rust` 为外部依赖（sibling 项目）
- 从 TypeScript 迁移时参考 `docs/sdk.md` 的迁移映射表
