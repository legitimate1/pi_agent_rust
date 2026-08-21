# 分支保护与合并策略

## 目的

仅当质量门（一并性、clippy、测试、覆盖率）在合并工作流中无法被绕过时，它们才能真正保护代码库。本文规定 `main` 分支所需的分支保护规则，并说明每个 CI 门如何映射为必需的状态检查。

## 必需的状态检查

以下检查必须通过，PR 才能合并到 `main`：

| CI 任务 | 工作流 | 必需 | 阻塞合并 |
|---------|--------|------|----------|
| `rust (ubuntu-latest)` | `ci.yml` | 是 | 是 |
| `rust (macos-latest)` | `ci.yml` | 是 | 是 |
| `rust (windows-latest)` | `ci.yml` | 是 | 是 |
| `conformance (fast-official)` | `conformance.yml` | 是 | 是 |
| `conformance (fast-generated)` | `conformance.yml` | 是 | 是 |
| `conformance (fast-negative)` | `conformance.yml` | 是 | 是 |
| `conformance (fast-capability-matrix)` | `conformance.yml` | 是 | 是 |

## 每个门禁的强制内容

### CI 流水线 (`ci.yml`)

按平台 (Linux、macOS、Windows)：

1. **无模拟依赖守护** — 在 `Cargo.toml`/`Cargo.lock` 中拦截 `mockall`、`mockito`、`wiremock`。
2. **无模拟代码守护** — 在测试代码中拦截 `Mock*/Fake*/Stub*` 标识符（允许清单: `MockHttp{Server,Request,Response}`）。
3. **追溯矩阵守护** — 校验 `docs/traceability_matrix.json` 的一致性。
4. **套件分类守护** — 每个 `tests/*.rs` 必须出现在 `tests/suite_classification.toml` 中。
5. **VCR 泄漏守护** — 单元套件文件不得引用 VCR 基础设施。
6. **PR 完成定义证据守护** — 对于功能面 PR，若 PR 描述未包含已勾选的单元/端到端/扩展证据链接与复现命令则拦截合并（仅 Linux PR 通道）。
7. **`cargo fmt --check`** — 格式合规。
8. **`cargo clippy -D warnings`** — 零 clippy 警告。
9. **`cargo doc --no-deps`** — 文档可干净构建。
10. **`cargo test --all-targets`** — 全部测试通过。
11. **统一验证运行器** — `scripts/e2e/run_all.sh --profile ci`（仅 Linux）。
12. **CI 门晋升** — 强制执行一致性阈值（仅 Linux）。
13. **一致性回归门** — 不允许通过率回归（仅 Linux）。
14. **覆盖率门** — 行覆盖率 >= 50%（仅 Linux）。

### 一致性流水线 (`conformance.yml`)

在 PR 上运行四项快速一致性检查：

1. **fast-official** — 官方扩展抽样（最多 5 个）。
2. **fast-generated** — 生成的 tier 1-2 场景。
3. **fast-negative** — 负向策略测试。
4. **fast-capability-matrix** — 能力拒绝矩阵。

## GitHub 分支保护设置

### `main` 的推荐配置

```
Settings → Branches → Branch protection rules → main
```

| 设置 | 取值 | 原因 |
|------|------|------|
| Require a pull request before merging | 已启用 | 禁止直接推送到 main |
| Required approvals | 1 | 最低评审门槛 |
| Dismiss stale pull request approvals | 已启用 | force-push 后重新评审 |
| Require status checks to pass before merging | 已启用 | CI 门禁为强制 |
| Require branches to be up to date before merging | 已启用 | 防止陈旧合并 |
| Required status checks | 见 [必需的状态检查](#必需的状态检查) | 全部所列检查 |
| Require conversation resolution before merging | 已启用 | 不允许未解决的讨论 |
| Require signed commits | 推荐 | 提交溯源 |
| Include administrators | 已启用 | 管理员也不可绕过 |
| Allow force pushes | 已禁用 | 防止历史重写 |
| Allow deletions | 已禁用 | 防止分支删除 |

### 通过 GitHub CLI 应用

```bash
# 设置必需的状态检查（按需调整仓库 owner/name）：
gh api repos/{owner}/{repo}/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["rust (ubuntu-latest)","rust (macos-latest)","rust (windows-latest)","conformance (fast-official)","conformance (fast-generated)","conformance (fast-negative)","conformance (fast-capability-matrix)"]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true}' \
  --field restrictions=null \
  --field allow_force_pushes=false \
  --field allow_deletions=false
```

## 校验脚本

运行 `scripts/check_branch_protection.sh` 以校验分支保护是否配置正确。该脚本检查：

1. 是否存在必需的状态检查。
2. 是否启用 `strict` 模式（分支需保持最新）。
3. 是否启用管理员强制。
4. 是否禁用 force push。
5. 是否禁用删除。
6. 是否要求 PR 评审。

## 存量功能分支的迁移指引

当此 DoD 门禁上线时，在上线前创建的存量开放功能分支可能缺少必需的 PR 证据区块。合并前请按以下步骤迁移这些分支：

1. 基于最新的 `main` 变基。
2. 将 PR 描述替换为 `.github/pull_request_template.md`。
3. 添加指向单元、端到端与扩展证据产物的直接链接。
4. 添加用于校验的精确复现命令及最近一次失败路径的命令。
5. 重新运行 CI 并确认 DoD 证据守护通过。

## 发布工作流集成

发布工作流 (`release.yml`) 在版本标签 (`v*`) 上触发。由于发布基于 `main` 创建，分支保护规则确保只有通过全部 CI 门禁的代码才能发布。

`scripts/release_gate.sh` 脚本提供了额外的本地或 CI 预发布检查，用于在创建发布标签前校验一致性证据包是否满足最低阈值。

### 发布前检查清单

1. `main` 上的全部 CI 检查通过。
2. `scripts/release_gate.sh --report` 返回 `verdict: pass`。
3. 一致性通过率 >= 80%（可通过 `RELEASE_GATE_MIN_PASS_RATE` 配置）。
4. 一致性失败数 <= 36（可通过 `RELEASE_GATE_MAX_FAIL_COUNT` 配置）。
5. 标签遵循 semver: `vMAJOR.MINOR.PATCH[-prerelease]`。

## 绕过防护

### 不可被绕过的内容

- 状态检查：对包括管理员在内的所有用户均为必需。
- PR 要求：直接推送到 `main` 被拦截。
- 格式与 lint：`cargo fmt --check` 与 `cargo clippy -D warnings`。

### 应急流程

在真正的紧急情况下（如安全补丁），仓库管理员可临时禁用分支保护。必须做到：

1. 在 GitHub issue 中记录理由。
2. 紧急合并后立即重新启用。
3. 在下一次团队同步中复盘。

## 监控

### CI 健康看板

每周跟踪以下指标：

- **不稳定率**: 瞬时失败 / 总运行次数（目标: < 5%）。
- **CI 平均时长**: `ci` 工作流的平均墙钟时间。
- **覆盖率趋势**: 随时间的行覆盖率（下限: 50%）。
- **一致性通过率**: 扩展一致性趋势。

### 告警

- CI 不稳定率超过 5% → 按目标排查不稳定用例分流预算。
- 覆盖率跌破 50% → `cargo llvm-cov` 门禁将拦截合并。
- 一致性通过率下降 → `CI_GATE_PROMOTION_MODE=strict` 将拦截合并。
