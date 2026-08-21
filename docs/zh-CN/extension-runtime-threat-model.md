# 扩展运行时威胁模型

状态：Active（有效）  
主 Bead：`bd-k5q5.4.6`  
最后更新：2026-08-06

本文档对针对扩展运行时的真实滥用路径进行建模，并将每项威胁映射到代码层面的控制措施与可执行测试。

## 1. 系统范围

范围内组件：
- `PiJsRuntime` 宿主桥接（`src/extensions_js.rs`）
- 宿主调用能力策略（`src/extensions.rs`、`src/extensions/protocol.rs`、`src/extensions/extension_manager_impl.rs`、`src/config.rs`、`src/extension_dispatcher.rs`）
- JS 兼容垫片（`node:fs`、`node:child_process`、`node:http` 等）
- 扩展事件分发与注册面

范围外组件：
- 外部提供方账户被入侵
- 进程边界之外的宿主操作系统/内核漏洞
- 运行时策略/UI 控制范围之外的社会工程学

## 2. 资产

关键资产：
- 项目工作区文件完整性
- 环境/会话状态中的敏感材料
- 命令执行边界（`exec`、`child_process`）
- 扩展事件流与工具调用完整性
- 可复现的一致性与审计证据制品

## 3. 信任边界

边界：
1. 扩展 JS 代码（不受信任）-> 宿主调用（受信任宿主边界）
2. 虚拟文件系统（`__pi_vfs`）-> 宿主文件系统回退
3. 能力策略决策引擎 -> 执行分发
4. 用户提示/UI 决策通道 -> 运行时允许/拒绝效果

风险最高的边界是 (2)，因为路径访问错误可能暴露宿主文件。

## 4. 攻击者模型

攻击者类型：
- 恶意扩展作者
- 被入侵的第三方扩展包
- 含缺陷路径/进程逻辑的良性扩展被精心构造的输入所滥用

攻击者目标：
- 读取工作区外的敏感文件
- 提升至任意命令执行
- 绕过拒绝/提示策略关卡
- 通过宽松的宿主调用外泄敏感信息

## 5. 威胁目录（面向 STRIDE）

### T1：宿主文件系统读取逃逸
- 向量：`node:fs` 读取路径如 `/etc/hostname` 或经遍历归一化的绝对路径。
- 影响：敏感宿主数据泄露。
- 控制：
  - 宿主读取回退现已在 `src/extensions_js.rs` 中限制于运行时 cwd 根目录。
  - 规范路径边界检查阻止对根目录外的读取。
- 证据：
  - `tests/security_fs_escape.rs::host_read_fallback_denies_outside_workspace`
  - `tests/security_fs_escape.rs::read_file_traversal_with_dot_dot`
  - `tests/extensions_fs_shim.rs::fs_stat_host_fallback`

### T2：符号链接/路径遍历逃逸
- 向量：`..` 片段、符号链接间接跳转或精心构造的路径归一化。
- 影响：工作区外的读写或策略绕过。
- 控制：
  - 在 VFS 与工具路径解析中进行路径归一化与根目录检查。
  - 对解析后超出根目录的宿主回退路径拒绝访问。
- 证据：
  - `tests/security_fs_escape.rs` 归一化/写入限制套件
  - `src/extensions/tests/core.rs::fs_connector_denies_path_traversal_outside_cwd`
  - `src/extensions/tests/core.rs::fs_connector_denies_symlink_escape`

### T3：通过方法/能力不匹配绕过能力策略
- 向量：伪造与方法语义不同的 `capability`。
- 影响：通过低权限标签调用高权限行为。
- 控制：
  - `required_capability_for_host_call(...)` 权威映射。
  - 分发器对无效/不匹配的能力请求予以拒绝。
- 证据：
  - `tests/extensions_policy_negative.rs` 能力映射测试
  - `src/extensions/tests/core.rs::required_capability_for_host_call_maps_tools_and_fs_ops`
  - `src/extensions/tests/runtime_parity.rs` 下的对等/适配器测试

### T4：危险宿主调用被意外启用
- 向量：含糊的默认值或未知的配置名称。
- 影响：意外暴露 `exec`/`env`。
- 控制：
  - 默认配置已改为 `permissive`；严格模式仍为显式可选加入。
  - 未知配置令牌以 fail-closed 方式回退至 `safe`。
  - 危险能力的显式可选加入路径（`allowDangerous`、配置覆盖）。
- 证据：
  - `tests/capability_policy_scoped.rs` 配置解析测试
  - `tests/e2e_cli.rs` 解释/迁移护栏测试

### T5：提示疲劳或非交互式拒绝歧义
- 向量：重复的需提示决策导致操作员失误。
- 影响：授予过宽的持久权限。
- 控制：
  - 策略决策日志包含原因/修复元数据。
  - 当提示管理器/UI 通道不可用时回退为拒绝。
- 证据：
  - `tests/extensions_policy_negative.rs`
  - `src/extensions/tests/core.rs` 下的提示/拒绝测试
  - `src/extensions/tests/shared_dispatch.rs::shared_dispatch_prompt_without_manager_fails_closed`

## 6. 滥用用例测试矩阵

| 滥用用例 | 预期结果 | 测试证据 |
|---|---|---|
| 从扩展读取 `/etc/hostname` | 拒绝（`outside extension root`） | `tests/security_fs_escape.rs::host_read_fallback_denies_outside_workspace` |
| 通过宿主回退读取工作区文件 | 允许 | `tests/security_fs_escape.rs::host_read_fallback_allows_workspace_file` |
| 遍历读取 `/fake/../etc/hostname` | 拒绝 | `tests/security_fs_escape.rs::read_file_traversal_with_dot_dot` |
| 通过宿主回退对根目录外进行 Stat/exists | 拒绝/为 false | `tests/extensions_fs_shim.rs::fs_stat_host_fallback` |
| 在安全默认值下 `exec` 被拒绝 | 拒绝 | `tests/extensions_policy_negative.rs::exec_tool_denied_by_default_policy` |
| 默认配置解析为 permissive 模式 | 允许大多数 | `tests/capability_policy_scoped.rs::default_config_resolves_to_permissive` |

## 7. 遗留缺口与负责人

| 缺口 | 风险 | 负责人 | 跟踪 |
|---|---|---|---|
| 危险能力的操作员发布指引已在 `README.md` + `EXTENSIONS.md` 中发布 | 已缓解 | 能力策略 UX 负责人 | `bd-k5q5.4.7`（已完成） |
| 跨所有扩展类别的端到端滥用语料 | 高 | 一致性活动负责人 | `bd-k5q5.2` / `bd-k5q5.2.4` |
| 从每个安全测试到需求的完整追溯映射 | 中 | 验证治理负责人 | `bd-k5q5.7.12` |

## 8. 验证命令

运行针对性的滥用/安全检查：

```bash
cargo test --test security_fs_escape -- --nocapture
cargo test --test extensions_fs_shim fs_stat_host_fallback -- --nocapture
cargo test --test extensions_policy_negative -- --nocapture
cargo test --test capability_policy_scoped -- --nocapture
```

运行质量门：

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```
