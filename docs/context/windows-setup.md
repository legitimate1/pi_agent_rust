# Windows 开发环境搭建（权威）

> 按需查阅：新机/重装后首次搭建，或 `cargo test` 报 MSVC/linker/sccache 相关错误时查。日常开发的高频命令见 `AGENTS.md`「工具链/静态检查/测试」。

## 前置要求

- Windows 10/11 64-bit
- Rust `nightly-2026-07-05`（由 `rust-toolchain.toml` 固定，`rustup` 自动安装）
- 磁盘余量：`target/` 冷编译需 5–10 GB；全量 `cargo test --all-targets` 峰值约 30 GB debug 产物（见 `AGENTS.md` 门禁说明）

## 一键搭建（新机照做）

> 下列步骤均为**免管理员**（scoop 免提权）；`BuildTools` 安装本身需管理员确认一次。

### 1. 安装 BuildTools（烤箱 / MSVC `cl.exe`）

本项目依赖 `ring` / `libsqlite3-sys` / `rquickjs-sys` / `tree-sitter` 等含 C/C++ 的 crate，必须由 MSVC 编译。

- 方式 A（推荐）：安装 [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/)，勾选 **C++ build tools**（或 **Desktop development with C++**），确保包含 **MSVC v14.x + Windows 10/11 SDK**
- 方式 B（命令行）：`winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`

安装后应存在 `VC\Auxiliary\Build\vcvars64.bat`，常见路径（按优先级探测）：

```
C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat
C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat
C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat
```

验证：`where cl.exe` 仅在已加载 `vcvars64` 的 shell 中可见，属正常（见下文「MSVC 自动注入」）。

### 2. 安装 Rust 工具链

```pwsh
rustup show          # 触发 rust-toolchain.toml 的 nightly-2026-07-05 自动安装
cargo --version      # 预期 1.98.0-nightly
rustc --version
```

### 3. 安装 sccache + LLVM（含 lld-link） via scoop

```pwsh
# 安装 scoop（若未装）：https://scoop.sh
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
irm get.scoop.sh | iex

scoop install sccache llvm
sccache --version    # 预期 0.17.x
lld-link --version   # 预期 LLD 2x.x
where.exe fd         # 若缺失，见下步
where.exe cygpath    # Git for Windows 自带，通常在 C:\Program Files\Git\usr\bin
```

> 说明：项目 `.cargo/config.toml` 已配置 `build.rustc-wrapper = "sccache"` 与 `target.x86_64-pc-windows-msvc.linker = "lld-link.exe"`，scoop 安装后即生效；`cargo` 执行时若 `sccache` 不在 `PATH` 会自动回退为直连 `rustc`（仅慢，不报错）。

### 4. 补齐 PATH 依赖（fd / cygpath）

- `fd`：本项目 `C:\Users\m\.pi\agent\bin\fd.exe`（fd 10.4.2）或 `scoop install fd` / `cargo install fd-find`，需在用户 `PATH` 可见
- `cygpath`：随 Git for Windows 安装，位于 `C:\Program Files\Git\usr\bin\cygpath.exe`，需在用户 `PATH` 可见（`e2e_tools` / `tools_conformance` 的 bash cwd 用例依赖 `cygpath -w` 做 Windows 路径转换）

```pwsh
where.exe fd       # 预期命中 fd.exe
where.exe cygpath  # 预期命中 cygpath.exe
```

### 5. 启用 PowerShell 自动注入（MSVC + PATH 热补）

本仓库在 `pwsh` / `powershell` 的 `$PROFILE` 中提供 `cargo` 包装函数：首次在新 shell 中执行 `cargo` 时自动探测并加载 `vcvars64.bat`（约 1–1.5s，随后缓存），并热补 `fd` / `cygpath` / `sccache` / `lld-link` 到会话 `PATH`。

```pwsh
# 查看/编辑当前用户的 profile（按需自适配：若本机 vcvars64 / scoop 路径与文档不同，直接改此文件）
notepad $PROFILE
# 或
code $PROFILE
```

行为契约：

- 在 `pwsh` 中执行 `cargo test / check / build / clippy` 自动注入 MSVC；`cargo.exe` 可绕过包装（显式不注入）
- 在 `bash` / `git bash` 中**禁止直接执行 `cargo`**（该环境不读取 PowerShell `$PROFILE`，必定缺失 MSVC；见 `AGENTS.md`「工具链 — Windows 构建约束」）
- 快捷：`ct` = `cargo test`、`cc` = `cargo check`、`ccl` = `cargo clippy --lib -- -D warnings`、`cca` = `cargo clippy --all-targets -- -D warnings`、`vsenv` = 手动预热 MSVC

新机若 `$PROFILE` 尚未创建，可从本仓库示例复制并按需改路径：

```pwsh
# 若无则新建（-Force 会创建父目录）
New-Item -ItemType File -Force -Path $PROFILE
# 将本机已验证的 profile 内容写入（示例路径见仓库内同名 profile 模板/当前机器的 $PROFILE）
```

## 验证

```pwsh
# 1) 工具链
rustup show
sccache --version; lld-link --version; where.exe fd; where.exe cygpath

# 2) 编译冒烟（应在 pwsh 中执行，首跑会显示 [vsenv] loading）
cargo test --lib -- dispatcher_tool_find_discovers_files -- --nocapture
# 预期：[vsenv] MSVC ready ... + ok. 1 passed（二次执行不再显示 loading）

# 3) 轻量门禁
cargo clippy --lib -- -D warnings; cargo fmt --check
```

## 故障排查

- **现象：`build-script-build exit 1` / `cl.exe not found` / `linker not found`**
  - 原因：当前 shell 未注入 MSVC
  - 处理：改用 `pwsh` 执行 `cargo`（自动注入）；或显式 `cmd /c "vcvars64.bat >nul && cargo ..."`；bash 下必现

- **现象：`fd is not available` / `where fd` 无结果**
  - 原因：`fd` 不在用户 PATH
  - 处理：`scoop install fd` 或将 `C:\Users\m\.pi\agent\bin` 加入用户 PATH 并重开 shell

- **现象：`cygpath` 无结果 / `bash_cwd_*` 失败**
  - 原因：`Git\usr\bin` 不在 PATH
  - 处理：将 `C:\Program Files\Git\usr\bin` 加入用户 PATH

- **现象：`lld-link --version` 无结果**
  - 原因：`llvm` 未安装或 PATH 未含
  - 处理：`scoop install llvm`；确认 `C:\Users\m\scoop\apps\llvm\current\bin` 在 PATH

- **现象：`cargo test --all-targets` 磁盘告警**
  - 原因：`target/` 膨胀
  - 处理：`cargo sweep --file --time 30` 或 `cargo sweep --file`（见 `commands.md`「target/ 磁盘空间管理」）

- **现象：profile 未生效**
  - 原因：当前 shell 不读 `$PROFILE`
  - 处理：确认执行的是 `pwsh`（非 `bash`/`cmd`）；重开窗口；`Test-Path $PROFILE` 为 False 时需新建

## 自适配清单（换机必做）

> 本机差异仅两处，按实改 `$PROFILE` 即可，其余沿用。

1. **MSVC 路径**：`$PROFILE` 的 `$script:PiVsBatch` 指向本机实际的 `vcvars64.bat`（Community/BuildTools/Enterprise 路径不同，见上文探测列表）
2. **工具路径**：`$script:PiExtraPaths` 列出的 `fd` / `Git\usr\bin` / `scoop\shims` / `scoop\apps\llvm\current\bin` 按本机实际安装位置调整；若工具经 `winget`/`choco` 等其他方式安装，替换为对应目录即可
