# 开发命令

## 构建

```bash
cargo check              # 快速检查编译错误
cargo build              # Debug 构建（LLD 链接器 + sccache 缓存）
cargo build --release    # 发布构建（LTO + size opt）
```

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
opt-level = "z"     # 按体积优化
lto = true
codegen-units = 1
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
