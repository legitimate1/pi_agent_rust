# 发布 pi_agent_rust（Releasing pi_agent_rust）

本仓库交付以下内容：
- 一个 crates.io 包：`pi_agent_rust`（Cargo `[package].name`）
- 一个库 crate：`pi`（Cargo `[lib].name`）
- 一个二进制文件：`pi`（Cargo `[[bin]].name`）

Cargo 源包还保留了内部的 `pi_legacy_capture` 一致性工具，因为集成测试通过 `CARGO_BIN_EXE_pi_legacy_capture` 执行它。该工具由非默认的 `internal-legacy-capture` 特性门控，不是受支持的发布产物。因此，普通的 `cargo install pi_agent_rust --locked` 仅安装 `pi`；覆盖该工具的仓库门禁会显式启用其内部特性。

## 版本与标签（Versioning + tags，源的真实性）

**源的真实性（Source of truth）：** `Cargo.toml` `[package].version`。

- **标签格式（Tag format）：** `vX.Y.Z`（SemVer）。示例：`v0.2.0`。
- **预发布版本（Pre-releases）：** `vX.Y.Z-rc.1`（或类似）。示例：`v0.2.0-rc.1`。
- **耦合（Coupling）：** `pi_agent_rust`（crate）、`pi`（库）和 `pi`（二进制）均由同一包构建，因此共享同一个版本号。
- **兄弟仓库（Sibling repos）：** `asupersync`、`rich_rust`、`charmed_rust`、`sqlmodel_rust` 在各自仓库中独立版本化。

### 发布到 crates.io（Publishing to crates.io）

`.github/workflows/publish.yml` 是一个手动触发的、非权威的诊断流程。它验证带注释的标签、精确的根包标识、干净的冻结检出、发布门禁以及 `cargo publish --dry-run --locked`。它没有仓库密钥，从不发布任何内容。

稳定的 crates.io 发布恰好有两条授权通道：自动化的 `.github/workflows/release.yml` 通道和下文所述的已审计手动 DSR 通道。没有其他工作流或临时的操作员命令是授权的发布方。在自动化通道中，`release.yml` 首先创建或安全地完成一个已验证的 GitHub 草稿，在无密钥的情况下构建并检查精确的 `.crate`，然后将该 crate 和一个与源绑定的校验和收据传递给一个全新的、受审核门控的运行器。该全新运行器将受审核门控的密钥捕获到一个非导出的 shell 变量中，从步骤环境中移除它，并在无密钥的情况下重复最终的 crate、提供方和有效 Cargo 配置证明。然后它将令牌交给唯一一个执行 `cargo publish --locked --no-verify --registry crates-io` 的进程链。在该交接之前，包和 dry-run 证明已经完成；`--no-verify` 可防止 Cargo 在构建脚本可能继承发布凭证时重新构建包。其自定义 Cargo 凭证提供方为规范的 crates.io 读取请求提供令牌而不写入发布收据，并且仅当 Cargo 出示精确的、已验证的 crate 名称/版本/SHA-256 时才为发布请求提供令牌。随后该工作流要求 crates.io 报告精确的、未被撤回（non-yanked）的版本，然后才将 GitHub 发布设为公开。预发布版本完全跳过 crates.io。

发布和发布工作流在 `Cargo.lock` 下从 crates.io 解析兄弟项目 crate；它们不会基于任意的兄弟仓库检出进行构建。因此，按目标的构建清单记录的是所选的已锁定 crate 版本、仓库源和校验和，而不是无关的仓库 HEAD。

### 发布 GitHub Releases 二进制产物（Publishing GitHub Releases binaries）

`.github/workflows/release.yml` 在推送匹配 `v*` 的标签时触发，并将：
- 运行完整的冻结 SHA 格式/检查/clippy/测试和发布证据门禁
- 为 Linux/macOS/Windows 构建 `pi`，并拒绝任何原始可执行文件大小大于或等于 22 MiB（23,068,672 字节）的原生二进制文件
- 将平台归档、按目标的构建清单和 `SHA256SUMS` 附加到已验证的草稿，在安全重跑时保留匹配的产物且仅添加缺失的产物
- 当解析后的 SemVer 包含预发布组件时（例如 `-rc.1`），将 GitHub Release 标记为预发布
- 对于稳定版本，在将已验证的 GitHub 草稿设为公开之前发布/协调精确的 crate；仅当精确的、未被撤回的 crate 已存在时，才接受已公开的精确发布

发布说明仅从精确的 `## [vX.Y.Z] ...` 变更日志标题中提取。请确保为你要切割的标签存在该精确标题。

### 自动化通道所需的 GitHub 治理（Required GitHub governance for the automated lane）

工作流 YAML 无法使标签引用不可变，也无法将自动创建的环境变为受保护环境。在启用自动化通道之前，所有者必须在仓库设置中配置以下全部内容：

- 一个名为 `release` 的环境，至少有一名必需的审核者并启用自审防护；在其中存储 `CARGO_REGISTRY_TOKEN` 并禁用管理员绕过
- 一个覆盖字面量 `refs/tags/v*` 的活跃标签规则集，禁止更新和删除且无绕过参与者
- 仅在检查完上述控制后，将仓库变量 `RELEASE_GOVERNANCE_ACK` 精确设置为 `release-env-reviewers+immutable-v-tags-v1`

该工作流会查询可观测的环境和活跃规则集形态，当任一缺失、不可读、未激活或格式错误时则失效关闭（fail closed）。GitHub 通常会对只读调用者隐去规则集的 `bypass_actors`；缺失被视为未证明且失效关闭，而不是与空列表混淆。环境 API 也不会独立证明管理员绕过设置。因此，除非其工作流身份能够读取到显式的空绕过列表且所有者已提供上述精确的审计确认，否则自动化通道必须保持禁用。不要仅仅为了使此门禁变绿就为标签触发的工作流添加宽泛的管理员令牌。手动无 Actions 通道同样需要服务端标签不可变性；本地 Git 引用检查只是纵深防御，不能替代它。

**当前状态（Current state）：** 规则集 `20418963` 于 2026-08-04 创建并回读为对 `refs/tags/v*` 生效，禁止更新和删除且无绕过参与者。手动通道在打标签之前和发布之前仍必须重新运行下文精确的治理检查；缺失或变更的控制是硬性阻断。自动化通道保持禁用的原因是上述受保护的 `release` 环境和确认尚未配置。重复的本地引用比较永远不能替代服务端规则。

## 分发兼容性策略（Distribution compatibility strategy，DROPIN-146）

目标：保持打包和调用的人体工学足够兼容，以实现从上游 Pi 的无摩擦迁移。

### 支持的分发路径（Supported distribution paths）

- **安装器路径（Installer path，`install.sh`）**：面向最终用户的默认通道；安装 GitHub 发布二进制文件、验证校验和并管理迁移状态。
- **发布产物路径（Release artifact path，GitHub Releases）**：按操作系统/架构直接下载二进制文件，并通过 `SHA256SUMS` 验证。
- **源码路径（Source path，`cargo build --release --locked`）**：面向受限/离线环境的确定性回退方案。

### 可执行文件兼容性路径（Executable compatibility path）

- 规范命令为 `pi`。
- 如果已存在 TypeScript 版 `pi`，安装器支持原地迁移并将旧命令保留为 `legacy-pi`。
- 如果拒绝迁移（`--keep-existing-pi`），Rust 版 Pi 将安装为 `pi-rust`，以便两个 CLI 均可调用。
- 通过 `install.sh --version vX.Y.Z` 支持固定版本 rollout。

### 代表性验证矩阵（Representative validation matrix）

在为发布候选声明分发对等完成之前运行此矩阵：

1. 全新 Linux/macOS 安装（无既有 `pi`）：
   - `curl .../install.sh | bash`
   - `command -v pi && pi --version && pi --help >/dev/null`
2. 已存在 TypeScript `pi` 的迁移主机：
   - `install.sh --adopt`（或交互式 adopt 路径）
   - `pi --version` 返回 Rust 构建
   - `legacy-pi --version` 仍解析为保留的 TypeScript CLI
3. 保留现有路径：
   - `install.sh --keep-existing-pi`
   - `pi` 保持为 TypeScript CLI，`pi-rust --version` 解析为 Rust 构建
4. 固定企业/CI rollout：
   - `install.sh --version vX.Y.Z`
   - 二进制校验和验证对照发布的 `SHA256SUMS` 通过

## 性能与体积产物策略（Perf-vs-size artifact policy，bd-3ar8v.5.5）

发布操作必须将基准证据与交付产物区分开。

- **交付/分发产物（Shipping/distribution artifacts）**：使用 Cargo `release` 配置构建，并通过 `release.yml` + 安装器流程发布（`pi` 二进制 + `SHA256SUMS`）。
- **基准证据产物（Benchmark evidence artifacts）**：由 PERF-3X 通道（`scripts/perf/orchestrate.sh`、`scripts/bench_extension_workloads.sh`）使用基准配置标签（通常为 `perf`）生成，并带有运行级溯源（`correlation_id`、构建/配置元数据、分配器/PGO 元数据）。

策略约束：

1. 性能和认证声明必须引用基准证据产物，而非仅发布二进制文件。
2. 发布二进制文件仍是部署目标，可用于验证体积/启动/安装行为。
3. 任何声称性能提升的发布说明都应包含来自基准产物包的、关联链路的证据引用。
4. 如果配置标签/溯源缺失或矛盾，则在重新生成之前将性能声明视为无效。

## 集群规模声明就绪报告（Swarm-scale claim readiness report，bd-2zcs5.27）

在发布面向内容的文案中使用集群规模、drop-in、扩展、全量套件或性能证据之前，生成只读的就绪报告：

```bash
python3 scripts/report_swarm_claim_readiness.py --self-test
python3 scripts/report_swarm_claim_readiness.py --json
```

该报告发出模式（schema）`pi.swarm.claim_readiness_report.v1`，并按 `perf`、`full_suite`、`dropin`、`extension` 和 `activity_ledger` 对产物分组。其稳定的顶层机器字段为 `overall_status`、`overall_ready`、`blocking_issue_count` 和 `blocking_count`；`overall_ready` 是 `overall_status == "ready"` 的布尔别名，`blocking_count` 是 `blocking_issue_count` 的精确别名，便于操作员使用 jq。报告区分 `release_facing` 产物与 `historical_snapshot` 或 `release_policy` 记录，因此旧的规划快照保持可见而不会自动授权当前声明。

```bash
python3 scripts/report_swarm_claim_readiness.py --json \
  | jq '{overall_status, overall_ready, blocking_issue_count, blocking_count}'
```

同一 JSON 还包含模式为 `pi.swarm.stale_claim_report.v1` 的 `stale_claims`。此部分仅作报告：它从不重新打开、重新分配或编辑 Beads。它使用 `--stale-claim-after-hours` 对来自 `.beads/issues.jsonl` 的 `in_progress` Beads 进行分类，并可将来自 `--stale-claim-activity-jsonl` 行的更新鲜协调证据视为在 `--stale-claim-activity-fresh-hours` 内的活跃所有者证据。每一项都会列出 bead id、受托人、最后更新、证据来源、分类和精确的推荐操作员操作，以便操作员在确认后向所有者发送消息或手动重新打开。

该 JSON 还包含模式为 `pi.swarm.hostcall_queue_readiness.v1` 的 `hostcall_queue_telemetry`。它从 `tests/perf/reports/stress_triage.json` 和 `docs/evidence/ext-stress-reactor-queue-coverage.json` 读取宿主调用队列证据，然后报告以下稳定计数器：`s3fifo_fallback_transitions`、`s3fifo_fairness_rejected_total`、`s3fifo_lane_overflow_rejected_total`、`queue_overflow_rejected_total`、`safe_reclamation_fallback_transitions`、`bravo_transitions_total` 和 `bravo_rollbacks_total`。缺失的 S3-FIFO 或 BRAVO 遥测会在 `missing_required_fields` 中列出，而不是视为零；非零的回退、公平性拒绝、通道溢出或 BRAVO 回滚总数会使该部分变为 `fallback_heavy`，以便操作员知道在进一步分流之前不应将该运行呈现为无争用干净的结果。

仅当发布路径必须因陈旧或不受支持的证据而失败时才使用门禁模式：

```bash
python3 scripts/report_swarm_claim_readiness.py --gate
```

门禁模式仅对面向发布的阻断因素以非零退出：缺失产物、陈旧的生成时间戳、无数据的预算摘要、失败的裁决字段、模式漂移，或在作为同一声明使用的产物之间溯源不匹配。非门禁模式始终以 0 退出，适用于交接说明、操作员仪表板和陈旧证据分流。

当报告阻断时：
- 当声明仍打算面向发布时，重新生成所列的精确产物路径。
- 当报告为一个类别识别出多个溯源值时，按运行拆分声明。
- 当唯一可用证据是历史快照时，弱化或移除面向发布的内容。
- 不要使用 `docs/parity-certification.json` 来覆盖 `docs/evidence/dropin-certification-verdict.json` 或报告的 drop-in 阻断。

## 何时称为 1.0？（When do we call it 1.0?）

我们将在以下条件满足时称为 `1.0.0`：
- CI 在 Linux/macOS/Windows 上为绿（`.github/workflows/ci.yml`）
- 所需的执行面已达到对等稳定（交互式 + print + JSON 模式 + RPC + SDK 契约）且一致性证据为绿
- 扩展运行时表面和安全策略已足够稳定，我们可以承诺在没有有意的 SemVer 升级的情况下不破坏用户
- Drop-in 认证产物对干净的发布源提交报告 `CERTIFIED`，且最终发布引用等于它或仅包含已列入允许清单的、仅证据的后代提交，然后才使用严格替换声明

在此之前，`0.x` 发布仍可能改变行为以改进正确性/对等性，发布消息不得声称严格的 drop-in 替换。

## 发布切版（Cutting a release，patch/minor）

1) **选择版本（Pick version）**（SemVer）：
   - patch：缺陷修复 / 内部重构
   - minor：新的面向用户功能
2) **更新版本** 在 `Cargo.toml`（`[package].version`）中。
3) **在本地运行质量门禁（Run quality gates locally）**：
   - `cargo fmt --check`
   - `cargo check --locked --all-targets --features internal-legacy-capture`
   - `cargo clippy --locked --all-targets --features internal-legacy-capture -- -D warnings`
   - `cargo test --locked --all-targets --features internal-legacy-capture`
4) **更新变更日志（Update changelog）**：
   - `br changelog --since-tag vX.Y.Z`（若无既有标签则使用 `--since YYYY-MM-DD`）
   - 将输出粘贴到 `CHANGELOG.md` 的新版本标题下
5) **提交（Commit）**（`git commit`）。
6) **按所选通道打标签（Tag according to the selected lane）**：
   - 自动化：同步 `main` 和遗留的 `master`，在它们的共同 tip 上创建带注释的标签，然后推送它
   - 手动/无 Actions：不要在此预先创建或推送标签；下文失效关闭（fail-closed）通道仅在最终源码冻结后在本地创建它，将其用于保留的原始构建，且仅在打包通过后才推送它
7) **恰好完成一条发布通道（Complete exactly one publication lane）**：
   - 自动化：`Release (GitHub binaries)` 在其外部治理门禁通过后完成有序的草稿 → 精确的稳定 crate → 公开发布流程
   - 手动/无 Actions：遵循下文每一个失效关闭步骤；不要分派、重跑或以其他方式调用工作流
   - 可选的 `Publish validation (no publication)` 仅作诊断，永远不能作为已发布的证据

## 手动 DSR 通道（无 GitHub Actions）（Manual DSR lane (no GitHub Actions)）

当发布有意从操作员主机构建和发布时使用此通道。它不查询、分派、重跑、取消或以其他方式将 GitHub Actions 工作流用作执行或证据。冻结的 Windows 构建分支使用 DSR 主机 `wsurf`，映射到 SSH 主机 `oldsurface`；`wlap` 仅是构建后的 Windows 执行冒烟主机。保持每一个已推送的发布准备、源码和证据提交都带有 `[skip actions]` 标记；标签最终引用的提交必须包含该标记。使用带该标记的带注释标签作为额外的可审计信号。

从精确的 `[skip actions]` 源提交的全新、私有克隆来操作该通道，绝不要从共享的开发检出操作。从该精确的本地提交创建克隆且不使用硬链接，保持其 `main` 分支位于源提交，将其拉取 URL 重新指向规范的 GitHub 仓库，并在下文显式的分支推送检查点之前禁用其推送 URL。不要将已忽略或未跟踪的文件复制到其中。在整个失效快速（fail-fast）会话期间固定 `RUSTUP_TOOLCHAIN`，绕过 RCH Cargo 包装器，并将 rustup 所选的实际 Cargo/Rust 编译器目录置于 `PATH` 最前。记录原始 Cargo/Rust 入口点和所选的实际二进制文件，包括解析路径、SHA-256 摘要和详细版本。在创建私有克隆和状态目录的最小引导之后，将本手册或已审计的、仓库自有的门禁、测试和脚本命令面显式调用的每一个预先存在的控制器可执行文件记录为精确的 `(label, SHA-256, requested path, resolved path)` 元组。在每个主要边界重新解析并重新哈希该清单，并在每次远程变更之前立即重新验证；仅写入一次的收据不是执行绑定。

此操作员工具收据**不**声称完整的传递性进程闭包。Cargo/Rust 入口点和 rustup 所选的 `cargo`/`rustc` 二进制文件在上文单独绑定，但由 Cargo、rustc、原生链接器驱动、过程宏、依赖构建脚本或 OS 加载器在内部选择的后代不在 `operator-tools.tsv` 范围内；在隔离测试目录内生成的夹具可执行文件和在远程构建/冒烟主机上执行的命令也不在范围内。该通道不对这些被排除的后代作字节同一性声明。不得在没有针对精确发布源的全新 exec-trace/allowlist 证明的情况下，将其排除描述为完整的构建工具闭包。Shell 内建命令（包括 `pwd` 和控制器的 `kill`）改为绑定到已验证的运行中控制器 Bash；它们故意不在普通 PATH 工具行中。`path-kill` 行单独绑定通过 Rust 子进程 PATH 查找触达的外部可执行文件。每一个普通的收据所列 PATH 工具必须解析为文件，绝不能是别名、函数或内建，且 Bash 命令哈希已禁用。

在 E2E 运行期间，使克隆的 `.git` 元数据不可写，以使测试无法移动 HEAD 或更改索引，并将 Cargo 目标和临时输出保持在克隆之外。工作区保持可写，因为普通测试合法地产生已忽略的报告，但运行器会在运行前后对每个已跟踪字节和模式进行哈希，拒绝未列入允许清单的已忽略/未跟踪输入，并在任何净源码变更时失败。在运行器退出后立即恢复 `.git` 的所有者写权限，然后要求恢复和运行器均已通过。保留的 DSR 配置是路径固定的，因此仅通过已记录的 bubblewrap 绑定调用该子进程，该绑定将此克隆呈现在规范路径而不改变共享检出。通过发布保留私有克隆，以便每个保留的绝对证据路径保持可解析。

在打开失效快速会话之前，将每一个发布源变更冻结在一个或多个以 `[skip actions]` 结尾的提交中，并使检出完全干净。将该通道作为一个失效快速 Bash 会话（`set -euo pipefail`）运行，从当前操作员 shell 使用 `exec /bin/bash --noprofile --norc -p` 启动。此处使用特权 Bash 模式仅作为进程分派加固：它拒绝导入的 shell 函数和启动文件；它不是授权提升。该块在信任任何 PATH 查找之前验证该精确的干净 shell 契约。不要孤立地复制后续的发布命令。首先将所有操作员状态绑定到预期的稳定版本、检出之外的新鲜目录、固定的已审计冒烟主机（`trj`、`mmini` 和 `wlap`）以及已审计控制器的 ARM64 sysroot。Linux AMD64 在 `trj` 上原生执行。Linux ARM64 在该 x86_64 主机上显式通过 `qemu-aarch64` 执行；那是目标运行时仿真，不是硬件原生 ARM64 声明。`mmini` 必须同时支持原生 ARM64 执行和 Rosetta x86_64 执行，`wlap` 必须报告 x86_64 Windows 运行时。在运行此块之前替换显式的操作员提供值：

```bash
set -euo pipefail
set +x
umask 077
[[ -n "${BASH_VERSION:-}" && "$-" == *p* ]]
builtin hash -r
builtin set +h
[[ "$-" != *h* ]]
builtin shopt -u expand_aliases
if builtin shopt -q expand_aliases; then
  exit 1
fi
(( ${#BASH_ALIASES[@]} == 0 ))
builtin unalias -a
while IFS= builtin read -r -d '' release_env_entry; do
  case "${release_env_entry%%=*}" in
    BASH_FUNC_*)
      builtin printf 'refusing exported shell function environment\n' >&2
      exit 1
      ;;
  esac
done < "/proc/$$/environ"
builtin unset release_env_entry
builtin unset BASH_ENV ENV CDPATH GLOBIGNORE
[[ ! -v BASH_ENV && ! -v ENV ]]
release_tool_names=(
  realpath sha256sum bash git python3 rustup cargo rustc gh jq ssh bwrap yq
  uuidgen curl scp tar file dirname awk grep wc stat id mktemp date sort cmp comm
  sed find chmod head tail tee tr cat mkdir env uname df nproc sysctl ubs br
  rg timeout base64 flock mv od basename sleep cp paste am bv cut dd fd mkfifo
  pgrep ps rch rm sh tmux touch which install rmdir xz yes ls seq whoami
)
release_path_descendant_tool_names=(kill)
for release_tool in \
    "${release_tool_names[@]}" "${release_path_descendant_tool_names[@]}"; do
  if builtin declare -F "$release_tool" >/dev/null; then
    builtin printf 'controller function shadows tool: %s\n' "$release_tool" >&2
    exit 1
  fi
done
export RUSTUP_TOOLCHAIN="nightly-2026-07-05"
export RCH_CARGO_WRAPPER_BYPASS=1
test "$RUSTUP_TOOLCHAIN" = nightly-2026-07-05
test "$RCH_CARGO_WRAPPER_BYPASS" = 1
# Capture the crates.io credential into one non-exported shell variable before
# starting any subprocess. The release shell keeps it unavailable to git,
# rustup, Cargo gates, tests, evidence generators, DSR, and packaging until the
# single checksum-gated publication process in step 8.
if [[ -n "${CARGO_REGISTRY_TOKEN:-}" &&
      -n "${CARGO_REGISTRIES_CRATES_IO_TOKEN:-}" ]]; then
  [[ "$CARGO_REGISTRY_TOKEN" == "$CARGO_REGISTRIES_CRATES_IO_TOKEN" ]]
fi
release_crates_io_token="${CARGO_REGISTRY_TOKEN:-${CARGO_REGISTRIES_CRATES_IO_TOKEN:-}}"
[[ -n "$release_crates_io_token" ]]
(( ${#release_crates_io_token} <= 4096 ))
case "$release_crates_io_token" in *$'\n'*|*$'\r'*) exit 1 ;; esac
builtin export -n release_crates_io_token
[[ -z "${PI_CRATES_IO_RELEASE_TOKEN:-}" ]]
builtin unset CARGO_REGISTRY_TOKEN CARGO_REGISTRIES_CRATES_IO_TOKEN \
  PI_CRATES_IO_RELEASE_TOKEN
release_cargo_entrypoint="$(builtin type -P -- cargo)"
release_rustc_entrypoint="$(builtin type -P -- rustc)"
release_rustup_entrypoint="$(builtin type -P -- rustup)"
test -n "$release_cargo_entrypoint"
test -n "$release_rustc_entrypoint"
test -n "$release_rustup_entrypoint"
release_cargo_actual="$(realpath -e -- \
  "$(rustup which --toolchain "$RUSTUP_TOOLCHAIN" cargo)")"
release_rustc_actual="$(realpath -e -- \
  "$(rustup which --toolchain "$RUSTUP_TOOLCHAIN" rustc)")"
test -f "$release_cargo_actual" && test ! -L "$release_cargo_actual"
test -f "$release_rustc_actual" && test ! -L "$release_rustc_actual"
release_rust_bin="$(dirname -- "$release_cargo_actual")"
test "$(dirname -- "$release_rustc_actual")" = "$release_rust_bin"
export PATH="$release_rust_bin:$PATH"
test "$(realpath -e -- "$(builtin type -P -- cargo)")" = "$release_cargo_actual"
test "$(realpath -e -- "$(builtin type -P -- rustc)")" = "$release_rustc_actual"
case "$(cargo --version)" in
  'cargo 1.98.0-nightly ('*) ;;
  *) printf 'unexpected pinned Cargo version\n' >&2; exit 1 ;;
esac
case "$(rustc --version)" in
  'rustc 1.98.0-nightly ('*) ;;
  *) printf 'unexpected pinned rustc version\n' >&2; exit 1 ;;
esac
export RELEASE_VERSION="X.Y.Z"
export LINUX_AMD64_SMOKE_HOST="trj"
export LINUX_ARM64_SMOKE_HOST="trj"
export LINUX_ARM64_QEMU_SYSROOT="/operator/supplied/aarch64/sysroot"
export DARWIN_SMOKE_HOST="mmini"
export WINDOWS_AMD64_SMOKE_HOST="wlap"
[[ "$RELEASE_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
export RELEASE_TAG="v${RELEASE_VERSION}"
test "$RELEASE_TAG" != "vX.Y.Z"
release_source_checkout="$(builtin pwd -P)"
test "$release_source_checkout" = /data/projects/pi_agent_rust
test -z "$(git status --porcelain=v2 --untracked-files=all)"
source_commit="$(git rev-parse 'HEAD^{commit}')"
case "$(git show -s --format=%s "$source_commit")" in
  *'[skip actions]') ;;
  *) printf 'release-source HEAD lacks [skip actions]\n' >&2; exit 1 ;;
esac
release_clone_id="$(uuidgen | tr '[:upper:]' '[:lower:]')"
[[ "$release_clone_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
export MANUAL_RELEASE_ROOT="/data/tmp/pi_agent_rust-v${RELEASE_VERSION}-release-$release_clone_id"
export MANUAL_RELEASE_STATE_DIR="$MANUAL_RELEASE_ROOT/state"
release_checkout="$MANUAL_RELEASE_ROOT/checkout"
case "$MANUAL_RELEASE_ROOT" in
  /data/tmp/pi_agent_rust-v"$RELEASE_VERSION"-release-"$release_clone_id") ;;
  *) exit 1 ;;
esac
case "$MANUAL_RELEASE_ROOT" in
  "$release_source_checkout"|"$release_source_checkout"/*) exit 1 ;;
esac
test ! -e "$MANUAL_RELEASE_ROOT" && test ! -L "$MANUAL_RELEASE_ROOT"
mkdir -m 700 "$MANUAL_RELEASE_ROOT"
test "$(realpath -e -- "$MANUAL_RELEASE_ROOT")" = "$MANUAL_RELEASE_ROOT"
test "$(stat -c '%a:%u' "$MANUAL_RELEASE_ROOT")" = "700:$(id -u)"
test ! -e "$release_checkout" && test ! -L "$release_checkout"
git clone --no-local --no-hardlinks --single-branch --branch main \
  "$release_source_checkout" "$release_checkout"
test "$(git -C "$release_checkout" rev-parse 'HEAD^{commit}')" = "$source_commit"
test -z "$(git -C "$release_checkout" status --porcelain=v2 --untracked-files=all)"
release_remote_url="https://github.com/Dicklesworthstone/pi_agent_rust.git"
git -C "$release_checkout" remote set-url origin "$release_remote_url"
git -C "$release_checkout" remote set-url --push origin \
  no-push://pi-agent-rust-v0.2.0-release-guard
test "$(git -C "$release_checkout" remote get-url origin)" = "$release_remote_url"
test "$(git -C "$release_checkout" remote get-url --push origin)" = \
  no-push://pi-agent-rust-v0.2.0-release-guard
cd "$release_checkout"
test "$(builtin pwd -P)" = "$release_checkout"

assert_origin_push_disabled() {
  local -a release_fetch_urls release_push_urls
  mapfile -t release_fetch_urls < <(git remote get-url --all origin)
  test "${#release_fetch_urls[@]}" -eq 1 || return
  test "${release_fetch_urls[0]}" = "$release_remote_url" || return
  mapfile -t release_push_urls < <(git remote get-url --push --all origin)
  test "${#release_push_urls[@]}" -eq 1 || return
  test "${release_push_urls[0]}" = \
    no-push://pi-agent-rust-v0.2.0-release-guard
}
origin_push_guarded() {
  local push_status=0 guard_status=0
  # Push to the reviewed URL explicitly; never make the configured `origin`
  # push URL live, even transiently.  An interrupted controller therefore
  # leaves the persistent no-push guard intact.
  { assert_origin_push_disabled &&
    git push --atomic "$release_remote_url" "$@"; } || push_status=$?
  assert_origin_push_disabled || guard_status=$?
  test "$guard_status" -eq 0 || return "$guard_status"
  return "$push_status"
}
assert_origin_push_disabled
test "$LINUX_AMD64_SMOKE_HOST" = trj
test "$LINUX_ARM64_SMOKE_HOST" = trj
test "$LINUX_ARM64_QEMU_SYSROOT" != "/operator/supplied/aarch64/sysroot"
[[ "$LINUX_ARM64_QEMU_SYSROOT" =~ ^/[A-Za-z0-9._/-]+$ ]]
case "$LINUX_ARM64_QEMU_SYSROOT" in *'/../'*|*'/..'|*'//'*) exit 1 ;; esac
test "$DARWIN_SMOKE_HOST" = mmini
test "$WINDOWS_AMD64_SMOKE_HOST" = wlap
test -z "${PI_CRATES_IO_RELEASE_TOKEN:-}"
test ! -e "$MANUAL_RELEASE_STATE_DIR"
mkdir -m 700 "$MANUAL_RELEASE_STATE_DIR"
release_rust_tool_receipt="$MANUAL_RELEASE_STATE_DIR/operator-rust-tools.txt"
test ! -e "$release_rust_tool_receipt"
record_release_rust_tool() {
  local label="$1"
  local entrypoint="$2"
  local resolved
  [[ "$label" =~ ^(cargo|rustc)-(entrypoint|actual)$ ]]
  resolved="$(realpath -e -- "$entrypoint")"
  test -f "$resolved" && test ! -L "$resolved"
  printf '[%s]\nentrypoint=%s\nresolved=%s\nsha256=%s\n' \
    "$label" "$entrypoint" "$resolved" \
    "$(sha256sum -- "$resolved" | awk '{print $1}')"
  "$entrypoint" --version --verbose
}
(set -C; {
  record_release_rust_tool cargo-entrypoint "$release_cargo_entrypoint"
  record_release_rust_tool rustc-entrypoint "$release_rustc_entrypoint"
  record_release_rust_tool cargo-actual "$release_cargo_actual"
  record_release_rust_tool rustc-actual "$release_rustc_actual"
} > "$release_rust_tool_receipt")
test "$(grep -Ec '^\[(cargo|rustc)-(entrypoint|actual)\]$' \
  "$release_rust_tool_receipt")" = 4
test "$(grep -Fxc 'release: 1.98.0-nightly' \
  "$release_rust_tool_receipt")" = 4
release_tool_receipt="$MANUAL_RELEASE_STATE_DIR/operator-tools.tsv"
test ! -e "$release_tool_receipt"
release_requested_tool_labels=(
  bin-sh usr-bin-node home-bun home-bun-node bin-bash bin-echo
)
release_requested_tool_paths=(
  /bin/sh /usr/bin/node /home/ubuntu/.bun/bin/bun /home/ubuntu/.bun/bin/node
  /bin/bash /bin/echo
)
test "${#release_requested_tool_labels[@]}" -eq \
  "${#release_requested_tool_paths[@]}"
record_operator_tool() {
  local release_tool="$1"
  local requested_path="$2"
  local resolved_path digest_line digest
  [[ "$release_tool" =~ ^[a-zA-Z0-9._-]+$ ]]
  test -n "$requested_path"
  [[ "$requested_path" == /* ]]
  [[ "$requested_path" != *$'\t'* && "$requested_path" != *$'\n'* ]]
  resolved_path="$(realpath -e -- "$requested_path")"
  test -f "$resolved_path" && test ! -L "$resolved_path"
  [[ "$resolved_path" != *$'\t'* && "$resolved_path" != *$'\n'* ]]
  digest_line="$(sha256sum -- "$resolved_path")"
  digest="${digest_line%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]]
  printf '%s\t%s\t%s\t%s\n' \
    "$release_tool" "$digest" "$requested_path" "$resolved_path"
}
(set -C; {
  for release_tool in "${release_tool_names[@]}"; do
    test "$(builtin type -t -- "$release_tool")" = file
    release_tool_requested_path="$(builtin type -P -- "$release_tool")"
    record_operator_tool "$release_tool" "$release_tool_requested_path"
  done
  for release_tool in "${release_path_descendant_tool_names[@]}"; do
    release_tool_requested_path="$(builtin type -P -- "$release_tool")"
    record_operator_tool "path-$release_tool" "$release_tool_requested_path"
  done
  for ((release_tool_index=0;
        release_tool_index<${#release_requested_tool_labels[@]};
        release_tool_index++)); do
    record_operator_tool \
      "${release_requested_tool_labels[$release_tool_index]}" \
      "${release_requested_tool_paths[$release_tool_index]}"
  done
} > "$release_tool_receipt")

verify_operator_tools() {
  local release_tool expected_digest expected_requested_path expected_resolved_path
  local expected_tool recognized release_tool_index actual_requested_path
  local actual_resolved_path actual_digest_line actual_digest expected_count
  local verified_count=0
  local -A seen_tools=()
  test -f "$release_tool_receipt" && test ! -L "$release_tool_receipt"
  [[ "$-" != *h* ]]
  if builtin shopt -q expand_aliases; then
    return 1
  fi
  (( ${#BASH_ALIASES[@]} == 0 ))
  while IFS=$'\t' read -r release_tool expected_digest \
      expected_requested_path expected_resolved_path; do
    [[ "$release_tool" =~ ^[a-zA-Z0-9._-]+$ ]]
    [[ "$expected_digest" =~ ^[0-9a-f]{64}$ ]]
    test -n "$expected_requested_path" && test -n "$expected_resolved_path"
    [[ "$expected_requested_path" == /* && "$expected_resolved_path" == /* ]]
    recognized=false
    actual_requested_path=""
    for expected_tool in "${release_tool_names[@]}"; do
      if test "$release_tool" = "$expected_tool"; then
        recognized=true
        test "$(builtin type -t -- "$release_tool")" = file
        actual_requested_path="$(builtin type -P -- "$release_tool")"
        break
      fi
    done
    if test "$recognized" = false; then
      for expected_tool in "${release_path_descendant_tool_names[@]}"; do
        if test "$release_tool" = "path-$expected_tool"; then
          recognized=true
          actual_requested_path="$(builtin type -P -- "$expected_tool")"
          break
        fi
      done
    fi
    if test "$recognized" = false; then
      for ((release_tool_index=0;
            release_tool_index<${#release_requested_tool_labels[@]};
            release_tool_index++)); do
        if test "$release_tool" = \
            "${release_requested_tool_labels[$release_tool_index]}"; then
          recognized=true
          actual_requested_path="${release_requested_tool_paths[$release_tool_index]}"
          break
        fi
      done
    fi
    test "$recognized" = true
    test -z "${seen_tools[$release_tool]+present}"
    seen_tools["$release_tool"]=1
    test "$actual_requested_path" = "$expected_requested_path"
    actual_resolved_path="$(realpath -e -- "$actual_requested_path")"
    test "$actual_resolved_path" = "$expected_resolved_path"
    test -f "$actual_resolved_path" && test ! -L "$actual_resolved_path"
    actual_digest_line="$(sha256sum -- "$actual_resolved_path")"
    actual_digest="${actual_digest_line%% *}"
    test "$actual_digest" = "$expected_digest"
    verified_count=$((verified_count + 1))
  done < "$release_tool_receipt"
  expected_count=$((${#release_tool_names[@]} + \
    ${#release_path_descendant_tool_names[@]} + \
    ${#release_requested_tool_labels[@]}))
  test "$verified_count" -eq "$expected_count"
  for expected_tool in "${release_tool_names[@]}"; do
    test "${seen_tools[$expected_tool]+present}" = present
  done
  for expected_tool in "${release_path_descendant_tool_names[@]}"; do
    test "${seen_tools[path-$expected_tool]+present}" = present
  done
  for expected_tool in "${release_requested_tool_labels[@]}"; do
    test "${seen_tools[$expected_tool]+present}" = present
  done
}

operator_tool_path() {
  local release_tool="$1"
  local match_count resolved_path
  [[ "$release_tool" =~ ^[a-zA-Z0-9._-]+$ ]]
  match_count="$(awk -F '\t' -v tool="$release_tool" '
    $1 == tool { count += 1 }
    END { print count + 0 }
  ' "$release_tool_receipt")"
  test "$match_count" -eq 1
  resolved_path="$(awk -F '\t' -v tool="$release_tool" \
    '$1 == tool { print $4 }' "$release_tool_receipt")"
  test -n "$resolved_path"
  printf '%s\n' "$resolved_path"
}
verify_operator_tools
release_bash_path="$(operator_tool_path bash)"
release_realpath_path="$(operator_tool_path realpath)"
release_bwrap_path="$(operator_tool_path bwrap)"
release_git_path="$(operator_tool_path git)"
release_sha256sum_path="$(operator_tool_path sha256sum)"
release_controller_bash="$("$release_realpath_path" -e -- "/proc/$$/exe")"
test "$release_controller_bash" = "$release_bash_path"
release_cargo_parent="$MANUAL_RELEASE_STATE_DIR/controller-cargo"
test ! -e "$release_cargo_parent" && test ! -L "$release_cargo_parent"
mkdir -m 700 "$release_cargo_parent"
test -d "$release_cargo_parent" && test ! -L "$release_cargo_parent"
test "$(stat -c '%a:%u' "$release_cargo_parent")" = "700:$(id -u)"
RELEASE_CARGO_WORK_DIR="$(mktemp -d \
  "$release_cargo_parent/work-v${RELEASE_VERSION}-XXXXXXXX")"
export RELEASE_CARGO_WORK_DIR
export CARGO_TARGET_DIR="$RELEASE_CARGO_WORK_DIR/target"
export TMPDIR="$RELEASE_CARGO_WORK_DIR/tmp"
export RELEASE_BUILD_HOME="$RELEASE_CARGO_WORK_DIR/home"
export RELEASE_BUILD_CARGO_HOME="$RELEASE_CARGO_WORK_DIR/cargo-home"
[[ "$CARGO_TARGET_DIR" == /* && "$TMPDIR" == /* &&
   "$RELEASE_BUILD_HOME" == /* && "$RELEASE_BUILD_CARGO_HOME" == /* ]]
test ! -e "$CARGO_TARGET_DIR" && test ! -e "$TMPDIR"
test ! -e "$RELEASE_BUILD_HOME" && test ! -e "$RELEASE_BUILD_CARGO_HOME"
mkdir -m 700 "$CARGO_TARGET_DIR" "$TMPDIR" \
  "$RELEASE_BUILD_HOME" "$RELEASE_BUILD_CARGO_HOME"
(set -C; printf \
  'cargo_target_dir=%s\ntmpdir=%s\nbuild_home=%s\nbuild_cargo_home=%s\n' \
  "$CARGO_TARGET_DIR" "$TMPDIR" "$RELEASE_BUILD_HOME" \
  "$RELEASE_BUILD_CARGO_HOME" \
  > "$MANUAL_RELEASE_STATE_DIR/local-build-paths.txt")
release_build_env() {
  env -i \
    PATH="$PATH" \
    HOME="$RELEASE_BUILD_HOME" \
    CARGO_HOME="$RELEASE_BUILD_CARGO_HOME" \
    CARGO_TARGET_DIR="$CARGO_TARGET_DIR" \
    TMPDIR="$TMPDIR" \
    XDG_CACHE_HOME="$RELEASE_BUILD_HOME/.cache" \
    XDG_CONFIG_HOME="$RELEASE_BUILD_HOME/.config" \
    XDG_DATA_HOME="$RELEASE_BUILD_HOME/.local/share" \
    RUSTUP_TOOLCHAIN="$RUSTUP_TOOLCHAIN" \
    RCH_CARGO_WRAPPER_BYPASS="$RCH_CARGO_WRAPPER_BYPASS" \
    GIT_CONFIG_GLOBAL=/dev/null \
    GIT_CONFIG_NOSYSTEM=1 \
    LANG=C.UTF-8 LC_ALL=C.UTF-8 TZ=UTC TERM=dumb NO_COLOR=1 \
    RUST_BACKTRACE=1 CARGO_TERM_COLOR=never \
    USER="${USER:-release}" LOGNAME="${LOGNAME:-${USER:-release}}" \
    "$@"
}
release_build_env cargo --version >/dev/null
RELEASE_REPOSITORY="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
export RELEASE_REPOSITORY
test "$RELEASE_REPOSITORY" = "Dicklesworthstone/pi_agent_rust"
test -z "$(git status --porcelain=v2 --untracked-files=all)"

# This lane intentionally has no GitHub Actions dependency. Every build,
# conformance run, test, platform smoke, package check, and publication
# reconciliation is executed directly by the operator and retained below.
# Do not query, dispatch, rerun, cancel, or otherwise use Actions as evidence.
MANUAL_RELEASE_RUN_ID="manual-${RELEASE_TAG}-$(uuidgen | tr '[:upper:]' '[:lower:]')"
export MANUAL_RELEASE_RUN_ID
[[ "$MANUAL_RELEASE_RUN_ID" =~ ^manual-v[0-9]+\.[0-9]+\.[0-9]+-[0-9a-f-]{36}$ ]]
```

在步骤 1 之前，再次证明活跃的不可变标签规则集。该规则必须以 `refs/tags/v*`（或所有引用）为目标，无排除项，禁止更新和删除，并暴露空的绕过参与者列表。该命令必须针对线上仓库通过；若上文记录的控制已消失或变更则停止：

```bash
set -euo pipefail
verify_operator_tools
ruleset_inventory="$MANUAL_RELEASE_STATE_DIR/tag-ruleset-inventory.json"
ruleset_details="$MANUAL_RELEASE_STATE_DIR/tag-ruleset-details.json"
test ! -e "$ruleset_inventory" && test ! -e "$ruleset_details"
gh api --paginate \
  -H 'Accept: application/vnd.github+json' \
  "/repos/${RELEASE_REPOSITORY}/rulesets?includes_parents=true&targets=tag&per_page=100" \
  | jq -s 'add' > "$ruleset_inventory"
jq -e 'type == "array" and length <= 100 and
  all(.[]; (.id | type) == "number")' "$ruleset_inventory" >/dev/null
while IFS= read -r ruleset_id; do
  gh api \
    -H 'Accept: application/vnd.github+json' \
    "/repos/${RELEASE_REPOSITORY}/rulesets/${ruleset_id}?includes_parents=true"
done < <(jq -r '.[].id' "$ruleset_inventory") | jq -s '.' > "$ruleset_details"
jq -e 'any(.[];
  .target == "tag" and .enforcement == "active" and
  ((.conditions.ref_name.include | index("refs/tags/v*")) != null or
   (.conditions.ref_name.include | index("~ALL")) != null) and
  .conditions.ref_name.exclude == [] and
  ([.rules[].type] | index("update")) != null and
  ([.rules[].type] | index("deletion")) != null and
  (.bypass_actors | type) == "array" and .bypass_actors == []
)' "$ruleset_details" >/dev/null
sha256sum "$ruleset_inventory" "$ruleset_details" \
  > "$MANUAL_RELEASE_STATE_DIR/tag-governance.sha256"
```

如果 API 省略 `bypass_actors`、返回超过 100 个标签规则集摘要、形态发生变化或无法使用操作员凭证读取，则停止。缺乏证明不等于空绕过列表的证明。

1. 运行已锁定的仓库门禁，包括内部捕获目标：

   ```bash
   set -euo pipefail
   verify_operator_tools
   release_build_env cargo fmt --check
   release_build_env cargo check --locked --all-targets --features internal-legacy-capture
   release_build_env cargo clippy --locked --all-targets --features internal-legacy-capture -- -D warnings
   release_build_env cargo test --locked --all-targets --features internal-legacy-capture
   ```

2. 在生成已跟踪证据之前绑定已洁净的发布源。除非精确的 HEAD 主题带有必需的 `[skip actions]` 标记，否则失败；此步骤故意不执行空提交或隐式提交：

   ```bash
   set -euo pipefail
   verify_operator_tools
   source_commit="$(git rev-parse 'HEAD^{commit}')"
   source_subject="$(git show -s --format=%s "$source_commit")"
   case "$source_subject" in
     *'[skip actions]') ;;
     *) printf 'release-source HEAD lacks [skip actions]: %s\n' "$source_subject" >&2; exit 1 ;;
   esac
   test -z "$(git status --porcelain=v2 --untracked-files=all)"
   git diff --quiet "$source_commit" --
   git diff --cached --quiet "$source_commit" --
   ```

   从该绑定的源生成保留的 CI 配置 E2E 证据。预先声明精确的带时间戳产物目录，使私有克隆的 Git 元数据不可写，并将 Cargo 输出保持在克隆之外。生产者独立捕获并重新捕获精确的源提交/树/索引/标志/原始字节，拒绝已忽略源输入中位于已批准生成输出根之外的输入，对诊断进行脱敏，并按 SHA-256 和字节数绑定每一个保留的诊断。将完整运行保留在 Git 中，以便发布门禁可以将契约、结果记录和诊断字节绑定到发布 HEAD：

   ```bash
   set -euo pipefail
   verify_operator_tools
   release_checkout="$(builtin pwd -P)"
   e2e_timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
   e2e_artifact_dir="$release_checkout/tests/e2e_results/$e2e_timestamp"
   test ! -e "$e2e_artifact_dir"
   mkdir -m 700 "$e2e_artifact_dir"
   find -P "$release_checkout/.git" -xdev ! -type l \
     -exec chmod a-w -- {} +
   e2e_status=0
   release_build_env \
     E2E_ARTIFACT_DIR="$e2e_artifact_dir" \
     VERIFY_CARGO_RUNNER=local \
     ./scripts/e2e/run_all.sh --profile ci --skip-lint || e2e_status=$?
   git_metadata_restore_status=0
   find -P "$release_checkout/.git" -xdev ! -type l \
     -exec chmod u+w -- {} + || git_metadata_restore_status=$?
   test "$git_metadata_restore_status" -eq 0
   test "$e2e_status" -eq 0
   test "$(jq -r .source_commit "$e2e_artifact_dir/evidence_contract.json")" = \
     "$source_commit"
   test "$(jq -r .source_commit "$e2e_artifact_dir/environment.json")" = \
     "$source_commit"
   test "$(jq -r .source_commit "$e2e_artifact_dir/summary.json")" = \
     "$source_commit"
   e2e_source_snapshot="$(jq -r .source_snapshot \
     "$e2e_artifact_dir/evidence_contract.json")"
   [[ "$e2e_source_snapshot" =~ ^sha256:[0-9a-f]{64}$ ]]
   test "$(jq -r .source_snapshot "$e2e_artifact_dir/environment.json")" = \
     "$e2e_source_snapshot"
   test "$(jq -r .source_snapshot "$e2e_artifact_dir/summary.json")" = \
     "$e2e_source_snapshot"
   # Keep the ignored run un-staged until the conformance generator has taken
   # its clean-HEAD source snapshot. The evidence commit below stages both
   # families together only after every producer has finished.
   test -z "$(git status --porcelain=v2 --untracked-files=all)"
   ```

3. 显式生成与源绑定的的一致性证据。不要复用历史性的 `CERTIFIED` 裁决：除非规范的完整认证流水线已针对此精确源提交成功重跑，否则重新生成一个诚实的 `NOT_CERTIFIED` 裁决并附带显式阻断原因。这是一个失效关闭的发布声明，不是豁免。提交生成的证据，然后运行强制的手动发布门禁。普通的测试运行是只读的，不会刷新这些已跟踪产物。v0.2.0 通道既不作严格的 drop-in 声明也不作定量/全局性能声明：其有效的性能摘要显式为 `blocked`/`NO_DATA` 且 performance-claims-NOT-authorized。预检和质量仍为必需：

   ```bash
   set -euo pipefail
   verify_operator_tools
   export CI_RUN_ID="$MANUAL_RELEASE_RUN_ID"
   export CI_CORRELATION_ID="${CI_RUN_ID}-conformance"
   release_build_env CI_RUN_ID="$CI_RUN_ID" CI_CORRELATION_ID="$CI_CORRELATION_ID" \
     cargo test --locked --test ext_conformance_diff \
     --features ext-conformance load_time_benchmark_official -- \
     --ignored --exact --nocapture
   release_build_env CI_RUN_ID="$CI_RUN_ID" CI_CORRELATION_ID="$CI_CORRELATION_ID" \
     cargo test --locked --test ext_conformance_scenarios \
     --features ext-conformance scenario_conformance_suite -- \
     --exact --nocapture
   release_build_env CI_RUN_ID="$CI_RUN_ID" CI_CORRELATION_ID="$CI_CORRELATION_ID" \
     cargo test --locked --test ext_conformance_scenarios \
     --features ext-conformance parity_runner -- --exact --nocapture
   release_build_env CI_RUN_ID="$CI_RUN_ID" CI_CORRELATION_ID="$CI_CORRELATION_ID" \
     cargo test --locked --test extensions_policy_negative \
     negative_conformance_report -- --exact --nocapture
   release_build_env CI_RUN_ID="$CI_RUN_ID" CI_CORRELATION_ID="$CI_CORRELATION_ID" \
     PI_GENERATE_CONFORMANCE_REPORT=1 \
     cargo test --locked --test conformance_report \
     generate_conformance_report -- --exact --nocapture
   release_build_env RELEASE_TAG="$RELEASE_TAG" python3 - <<'PY'
   import json
   import os
   import re
   import subprocess
   from datetime import datetime, timezone
   from pathlib import Path

   commit = subprocess.run(
       ["git", "rev-parse", "HEAD^{commit}"],
       check=True,
       capture_output=True,
       text=True,
   ).stdout.strip()
   if re.fullmatch(r"[0-9a-f]{40}", commit) is None:
       raise SystemExit("release source is not bound to a full SHA-1 commit")
   tag = os.environ["RELEASE_TAG"]
   path = Path("docs/evidence/dropin-certification-verdict.json")
   if path.is_symlink() or not path.is_file():
       raise SystemExit("drop-in verdict must remain a regular tracked file")
   payload = {
       "schema": "pi.dropin.certification_verdict.v1",
       "git_commit": commit,
       "generated_at_utc": datetime.now(timezone.utc).replace(microsecond=0)
           .isoformat().replace("+00:00", "Z"),
       "overall_verdict": "NOT_CERTIFIED",
       "hard_gate_results": [],
       "blocking_reasons": [
           f"{tag} is not strict-drop-in certified: the canonical full-certification "
           f"pipeline was not regenerated and proven against source commit {commit}."
       ],
       "evidence_index": [],
       "source": {
           "generator": "manual-release-fail-closed",
           "certification_lane_artifact": "tests/full_suite_gate/certification_verdict.json",
           "lane_verdict": "not-run-for-this-source",
       },
   }
   path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
   PY
   git add \
     docs/evidence/dropin-certification-verdict.json \
     tests/ext_conformance/reports/CONFORMANCE_REPORT.md \
     tests/ext_conformance/reports/conformance_summary.json
   # Raw producer outputs are normally ignored. This release deliberately
   # retains only the exact decision inputs and referenced per-extension logs,
   # together with the complete E2E run, so the gate can authenticate them at
   # HEAD instead of trusting summary counters.
   git add -f -- \
     "$e2e_artifact_dir" \
     tests/ext_conformance/reports/conformance_events.jsonl \
     tests/ext_conformance/reports/load_time_benchmark.json \
     tests/ext_conformance/reports/scenario_conformance.json \
     tests/ext_conformance/reports/smoke_triage.json \
     tests/ext_conformance/reports/extensions/*.jsonl \
     tests/ext_conformance/reports/parity/parity_events.jsonl \
     tests/ext_conformance/reports/parity/extensions/*.jsonl \
     tests/ext_conformance/reports/negative/negative_events.jsonl \
     tests/ext_conformance/reports/negative/triage.json
   release_build_env ubs --staged --only=rust .
   release_build_env ./scripts/reconcile_beads_ledger.sh
   git commit -m "Record ${RELEASE_TAG} release evidence [skip actions]"
   evidence_commit="$(git rev-parse 'HEAD^{commit}')"
   evidence_subject="$(git show -s --format=%s "$evidence_commit")"
   case "$evidence_subject" in
     *'[skip actions]') ;;
     *) printf 'release-evidence HEAD lacks [skip actions]: %s\n' \
          "$evidence_subject" >&2; exit 1 ;;
   esac

   release_gate_report="$MANUAL_RELEASE_STATE_DIR/release-gate-report.json"
   test ! -e "$release_gate_report"
   (
     set -C
     release_build_env \
       RELEASE_GATE_REQUIRE_PREFLIGHT=1 \
       RELEASE_GATE_REQUIRE_QUALITY=1 \
       RELEASE_GATE_REQUIRE_DROPIN_CERTIFIED=0 \
       RELEASE_GATE_REQUIRE_PERFORMANCE_CLAIM_READY=0 \
       RELEASE_GATE_CARGO_RUNNER=local \
       ./scripts/release_gate.sh --no-rch --report > "$release_gate_report"
   )
   jq -e '
     .schema == "pi.release_gate.v1" and .verdict == "pass" and
     .thresholds.require_performance_claim_ready == 0 and
     .counts.fail == 0 and .counts.total == (.checks | length) and
     any(.checks[];
       .name == "performance_claim_readiness" and .status == "warn" and
       (.detail | contains("performance claims are NOT authorized"))) and
     all(.checks[]; .status != "fail")
   ' "$release_gate_report" >/dev/null
   ```

   仅当显式模式为 `0` 且 v0.2.0 发布文案未作定量或全局性能声明时，性能警告才是可接受的。结构、模式、计数、状态或就绪性矛盾在任一模式下仍为硬性失败。未来若发布作此类声明则必须将标志设为 `1`；届时门禁还要求新鲜的、与源绑定的血缘以及固定的规范预算清单摘要。全局授权还要求每个已声明预算都有数据且已声明预算零失败；非 CI 的 `NO_DATA` 和 `FAIL` 结果分别推导出 `budget_data_missing` 和 `budget_failed` 阻断。门禁证明精确的严格性能测试被列出一次、运行一次、未被忽略，并新鲜地重新计算和深度比较已检入的定义、结果、失败、计数和就绪性。

   该门禁在入口要求仓库洁净，并在每次可执行检查后重新验证精确的 HEAD、规范源树摘要、索引、索引标志、符号链接拓扑、未跟踪路径和原始工作区字节。仅在通过后才推送，然后同步遗留兼容引用：

   ```bash
   set -euo pipefail
   verify_operator_tools
   branch_source_subject="$(git show -s --format=%s HEAD)"
   case "$branch_source_subject" in
     *'[skip actions]') ;;
     *) printf 'branch-push HEAD lacks [skip actions]: %s\n' \
          "$branch_source_subject" >&2; exit 1 ;;
   esac
   origin_push_guarded \
     refs/heads/main:refs/heads/main \
     refs/heads/main:refs/heads/master
   branch_source_commit="$(git rev-parse 'HEAD^{commit}')"
   test "$(git ls-remote origin refs/heads/main | awk 'NR == 1 {print $1}')" = \
     "$branch_source_commit"
   test "$(git ls-remote origin refs/heads/master | awk 'NR == 1 {print $1}')" = \
     "$branch_source_commit"
   assert_origin_push_disabled
   ```

4. 从该最终的干净证据提交构建并检查精确的 Cargo 源包。在运行 dry-run 之前，在检出之外记录其 SHA-256 和字节大小，然后证明 dry-run 复现了相同字节。此证明不得早于最终源/证据提交：

   ```bash
   set -euo pipefail
   verify_operator_tools
   release_build_env cargo package --locked
   crate_path="${CARGO_TARGET_DIR:-target}/package/pi_agent_rust-${RELEASE_VERSION}.crate"
   test -f "$crate_path" && test ! -L "$crate_path"
   source_commit="$(git rev-parse 'HEAD^{commit}')"
   test "$(tar -xOf "$crate_path" \
     "pi_agent_rust-${RELEASE_VERSION}/.cargo_vcs_info.json" \
     | jq -er --arg commit "$source_commit" \
       'select(.git.sha1 == $commit and (.git.dirty // false) == false) | .git.sha1')" \
     = "$source_commit"
   package_sha256="$(sha256sum "$crate_path" | awk '{print $1}')"
   package_size="$(wc -c < "$crate_path" | tr -d '[:space:]')"
   proof_file="$MANUAL_RELEASE_STATE_DIR/pi_agent_rust-${RELEASE_VERSION}-crate.txt"
   test ! -e "$proof_file"
   umask 077
   (set -C; printf 'source_commit=%s\npackage_sha256=%s\npackage_size=%s\n' \
     "$source_commit" "$package_sha256" "$package_size" > "$proof_file")

   release_build_env cargo publish --dry-run --locked
   test -f "$crate_path" && test ! -L "$crate_path"
   test "$(tar -xOf "$crate_path" \
     "pi_agent_rust-${RELEASE_VERSION}/.cargo_vcs_info.json" \
     | jq -er --arg commit "$source_commit" \
       'select(.git.sha1 == $commit and (.git.dirty // false) == false) | .git.sha1')" \
     = "$source_commit"
   dry_run_sha256="$(sha256sum "$crate_path" | awk '{print $1}')"
   dry_run_size="$(wc -c < "$crate_path" | tr -d '[:space:]')"
   test "$dry_run_sha256" = "$package_sha256"
   test "$dry_run_size" = "$package_size"
   printf 'dry_run_sha256=%s\ndry_run_size=%s\n' \
     "$dry_run_sha256" "$dry_run_size" >> "$proof_file"
   test -z "$(git status --porcelain=v2 --untracked-files=all)"
   printf 'release_tag=%s\n' "$RELEASE_TAG" >> "$proof_file"
   ```

如果检出为脏状态、包元数据未绑定到最终提交、任一相等性检查失败，或收据未存储在检出目录之外，则停止。

5. 在本地附注标签（annotated tag）下冻结干净的源代码，然后仅使用已审计的私有保存包装器（preservation wrapper）来执行五个原生构建分支（raw build legs）。这个保留的 v0.2.0 通道（lane）有意比普通的 DSR 更窄：启动器只接受一组精确的参数向量，拒绝 `--no-sync` 以及所有 resume/release/fallback/cleanup 覆盖项，将已冻结的源代码快照到已配置构建主机上的全新单次运行路径中，以标记为 `native` 的 DSR 构建模式运行，且仅产出原生可执行文件。该 DSR 标签描述的是通道本身，而非硬件原生执行的证明。不要直接调用私有的 `dsr` 入口点，不要用标准的 `dsr build` 替代，也不要将 `--only-native` 视为每个目标都在匹配的 CPU 硬件上运行的证明：已审计配置中的 Linux ARM64 分支是在其配置的 Linux 主机上进行的跨目标（cross-target）构建。

   保留的通道及其审计均为发布输入。下面固定的哈希值仅适用于 v0.2.0。如果路径缺失、任何哈希或模式不同，或正在切割更高版本，请停止并执行新的保存通道审计；切勿静默回退到其他 DSR 调用。在创建本地标签之前，下文所有的保留输入检查以及精确环境 Windows MSVC 链接预检（preflight）都必须通过。如果该预检失败，请修复并重新审计一条新的保存通道，替换本手册中所有固定的包装器/审计/清单哈希，并从干净的源代码重新开始。不要通过注入环境构建变量来修复：真实的 DSR 子进程会故意剥离它们。

   ```bash
   set -euo pipefail
   verify_operator_tools
   test "$RELEASE_VERSION" = "0.2.0"
   test "$RELEASE_TAG" = "v0.2.0"
   source_commit="$(awk -F= '$1 == "source_commit" {print $2}' "$proof_file")"
   [[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
   test "$(builtin pwd -P)" = "$release_checkout"
   test "$(git rev-parse 'HEAD^{commit}')" = "$source_commit"
   test "$(git rev-parse 'main^{commit}')" = "$source_commit"
   tag_source_subject="$(git show -s --format=%s "$source_commit")"
   case "$tag_source_subject" in
     *'[skip actions]') ;;
     *) printf 'tag source lacks [skip actions]: %s\n' \
          "$tag_source_subject" >&2; exit 1 ;;
   esac
   test -z "$(git status --porcelain=v2 --untracked-files=all)"

   # The audited preserved DSR lane is intentionally pinned to the canonical
   # project path. Prove that a child-only bubblewrap mount presents this exact
   # private clone there without modifying, moving, or fast-forwarding the
   # shared checkout outside the namespace.
   test "$release_source_checkout" = /data/projects/pi_agent_rust
   test "$release_checkout" != "$release_source_checkout"
   bwrap_source_receipt="$MANUAL_RELEASE_STATE_DIR/bwrap-source-preflight.txt"
   test ! -e "$bwrap_source_receipt"
   (
     set -C
     "$release_bwrap_path" \
       --die-with-parent --new-session --bind / / --dev-bind /dev /dev \
       --bind "$release_checkout" /data/projects/pi_agent_rust \
       --chdir /data/projects/pi_agent_rust \
       "$release_bash_path" --noprofile --norc -c '
         set -euo pipefail
         git_path="$1"
         expected_commit="$2"
         test "$(builtin pwd -P)" = /data/projects/pi_agent_rust
         test "$("$git_path" rev-parse "HEAD^{commit}")" = "$expected_commit"
         test "$("$git_path" rev-parse "main^{commit}")" = "$expected_commit"
         test -z "$("$git_path" status --porcelain=v2 --untracked-files=all)"
         printf "source_commit=%s\n" "$expected_commit"
       ' bash "$release_git_path" "$source_commit" > "$bwrap_source_receipt"
   )
   test "$(cat "$bwrap_source_receipt")" = "source_commit=$source_commit"

   git fetch --no-tags origin \
     refs/heads/main:refs/remotes/origin/main \
     refs/heads/master:refs/remotes/origin/master
   test "$(git rev-parse 'origin/main^{commit}')" = "$source_commit"
   test "$(git rev-parse 'origin/master^{commit}')" = "$source_commit"
   test -z "$(git tag --list "$RELEASE_TAG")"
   test -z "$(git ls-remote --tags origin \
     "refs/tags/$RELEASE_TAG" "refs/tags/$RELEASE_TAG^{}")"
   export PRESERVED_DSR_LANE="/data/tmp/dsr-preserve-pi-v0.2.0-d33f69b8-9756-4181-9de8-8b30671a9976"
   export PRESERVED_DSR_WRAPPER="$PRESERVED_DSR_LANE/preserved-pi-build"
   export PRESERVED_DSR_AUDIT="$PRESERVED_DSR_LANE/PRESERVATION_LANE_AUDIT.md"
   expected_preserved_wrapper_sha256=\
7c1c3528229f89eadea62d72eb692b4a5f089e037e008c153544c35701f93f75
   expected_preserved_audit_sha256=\
308b9ce092b34bac3224a91390452721475a9cb96a9ba9b4a164fcc2666662dc
   expected_preservation_manifest_sha256=\
d040d967dbf63644a29d72068aa6ac35e5ff74a7e168cb5eda08a46ff828f32b

   verify_preserved_dsr_inputs() {
     test -x "$PRESERVED_DSR_WRAPPER" && test ! -L "$PRESERVED_DSR_WRAPPER"
     test -f "$PRESERVED_DSR_AUDIT" && test ! -L "$PRESERVED_DSR_AUDIT"
     test -f "$PRESERVED_DSR_LANE/preservation-manifest.sha256"
     test ! -L "$PRESERVED_DSR_LANE/preservation-manifest.sha256"
     test "$(stat -c '%a' "$PRESERVED_DSR_WRAPPER")" = 700
     test "$(stat -c '%a' "$PRESERVED_DSR_AUDIT")" = 400
     test "$(stat -c '%a' \
       "$PRESERVED_DSR_LANE/preservation-manifest.sha256")" = 400
     test "$(sha256sum "$PRESERVED_DSR_WRAPPER" | awk '{print $1}')" = \
       "$expected_preserved_wrapper_sha256"
     test "$(sha256sum "$PRESERVED_DSR_AUDIT" | awk '{print $1}')" = \
       "$expected_preserved_audit_sha256"
     test "$(sha256sum \
       "$PRESERVED_DSR_LANE/preservation-manifest.sha256" | awk '{print $1}')" = \
       "$expected_preservation_manifest_sha256"
     (cd "$PRESERVED_DSR_LANE" && \
       sha256sum --check --strict --status preservation-manifest.sha256)
   }
   verify_operator_tools
   verify_preserved_dsr_inputs
   preserved_inputs="$MANUAL_RELEASE_STATE_DIR/preserved-lane-inputs.sha256"
   test ! -e "$preserved_inputs"
   (set -C; sha256sum \
     "$PRESERVED_DSR_WRAPPER" \
     "$PRESERVED_DSR_AUDIT" \
     "$PRESERVED_DSR_LANE/preservation-manifest.sha256" \
     > "$preserved_inputs")
   test "$(wc -l < "$preserved_inputs" | tr -d '[:space:]')" = 3
   sha256sum --check --strict --status "$preserved_inputs"

   windows_preflight_ps1="$MANUAL_RELEASE_STATE_DIR/windows-dsr-msvc-link-preflight.ps1"
   windows_preflight_receipt="$MANUAL_RELEASE_STATE_DIR/windows-dsr-msvc-link-preflight.json"
   windows_preflight_stderr="$MANUAL_RELEASE_STATE_DIR/windows-dsr-msvc-link-preflight.stderr"
   test ! -e "$windows_preflight_ps1"
   test ! -e "$windows_preflight_receipt"
   test ! -e "$windows_preflight_stderr"

   windows_dsr_ssh_host="$(yq -er '
     .hosts.wsurf |
     select(.enabled == true and .platform == "windows/amd64" and
            .connection == "ssh") |
     .ssh_host
   ' "$PRESERVED_DSR_LANE/preserve-config/hosts.yaml")"
   test "$windows_dsr_ssh_host" = oldsurface
   test "$(yq -er '.cross_compile."windows/amd64".host' \
     "$PRESERVED_DSR_LANE/preserve-config/repos.d/pi.yaml")" = wsurf
   test "$(yq -er '.cross_compile."windows/amd64".env.CARGO_BUILD_TARGET' \
     "$PRESERVED_DSR_LANE/preserve-config/repos.d/pi.yaml")" = \
     x86_64-pc-windows-msvc

   python3 - "$windows_preflight_ps1" <<'PY'
   from pathlib import Path
   import sys

   script = r'''$ErrorActionPreference = 'Stop'
   $Marker = 'pi-dsr-msvc-link-preflight-ok'
   $TempRoot = Join-Path $env:LOCALAPPDATA 'Temp'
   $TempItem = Get-Item -LiteralPath $TempRoot -Force
   if (-not $TempItem.PSIsContainer -or
       (($TempItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
       throw 'Windows temporary root is not a plain directory'
   }

   $Scratch = Join-Path $TempRoot (
       'pi-dsr-msvc-link-preflight-' + [Guid]::NewGuid().ToString('D')
   )
   if (Test-Path -LiteralPath $Scratch) {
       throw 'Fresh preflight path unexpectedly exists'
   }
   New-Item -ItemType Directory -Path $Scratch | Out-Null
   $ScratchItem = Get-Item -LiteralPath $Scratch -Force
   if (-not $ScratchItem.PSIsContainer -or
       (($ScratchItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
       throw 'Preflight scratch path is not a plain directory'
   }

   $CargoHome = Join-Path $Scratch 'cargo-home'
   $CargoTarget = Join-Path $Scratch 'cargo-target'
   New-Item -ItemType Directory -Path $CargoHome, $CargoTarget | Out-Null

   $Utf8 = [Text.UTF8Encoding]::new($false)
   $Source = Join-Path $Scratch 'main.rs'
   $Binary = Join-Path $Scratch 'pi-dsr-msvc-link-preflight.exe'
   $CompileStdoutPath = Join-Path $Scratch 'compile.stdout'
   $CompileStderrPath = Join-Path $Scratch 'compile.stderr'
   $RunStdoutPath = Join-Path $Scratch 'run.stdout'
   $RunStderrPath = Join-Path $Scratch 'run.stderr'
   $RemoteReceipt = Join-Path $Scratch 'receipt.json'

   [IO.File]::WriteAllText(
       $Source,
       'fn main() { println!("pi-dsr-msvc-link-preflight-ok"); }' +
           [Environment]::NewLine,
       $Utf8
   )

   $Build = [Diagnostics.ProcessStartInfo]::new()
   $Build.UseShellExecute = $false
   $Build.CreateNoWindow = $true
   $Build.RedirectStandardOutput = $true
   $Build.RedirectStandardError = $true

   $Keys = @($Build.EnvironmentVariables.Keys)
   foreach ($Key in $Keys) {
       if (($Key -match '^(CARGO_|RUST|XWIN_)') -or
           ($Key -match '^(CC|CXX|CPP|AR|RANLIB|LD|NM|OBJCOPY|STRIP|CFLAGS|CXXFLAGS|CPPFLAGS|LDFLAGS|BINDGEN_EXTRA_CLANG_ARGS|SDKROOT|MACOSX_DEPLOYMENT_TARGET|IPHONEOS_DEPLOYMENT_TARGET|INCLUDE|LIB|LIBPATH)(_|$)') -or
           ($Key -match '_(CC|CXX|AR|RANLIB|CFLAGS|CXXFLAGS|LDFLAGS)$')) {
           [void]$Build.EnvironmentVariables.Remove($Key)
       }
   }

   $Build.EnvironmentVariables['CARGO_BUILD_TARGET'] =
       'x86_64-pc-windows-msvc'
   $Build.EnvironmentVariables['CARGO_TERM_COLOR'] = 'always'
   $Build.EnvironmentVariables['RUST_BACKTRACE'] = '1'
   $Build.EnvironmentVariables['RCH_DISABLED'] = '1'
   $Build.EnvironmentVariables['CARGO_HOME'] = $CargoHome
   $Build.EnvironmentVariables['CARGO_TARGET_DIR'] = $CargoTarget

   $Build.FileName = $env:ComSpec
   $Build.WorkingDirectory = $Scratch
   $Build.Arguments = '/d /s /c ' +
       'where.exe link.exe > link-resolution.txt 2>&1 & ' +
       'where.exe cl.exe > cl-resolution.txt 2>&1 & ' +
       'rustc --target x86_64-pc-windows-msvc --edition 2024 ' +
       '--crate-name pi_dsr_msvc_link_preflight main.rs ' +
       '-o pi-dsr-msvc-link-preflight.exe'

   $Process = [Diagnostics.Process]::Start($Build)
   $StdoutTask = $Process.StandardOutput.ReadToEndAsync()
   $StderrTask = $Process.StandardError.ReadToEndAsync()
   $Process.WaitForExit()
   $CompileStdout = $StdoutTask.Result
   $CompileStderr = $StderrTask.Result
   [IO.File]::WriteAllText($CompileStdoutPath, $CompileStdout, $Utf8)
   [IO.File]::WriteAllText($CompileStderrPath, $CompileStderr, $Utf8)

   if ($Process.ExitCode -ne 0) {
       throw "MSVC link preflight failed with exit $($Process.ExitCode); retained at $Scratch"
   }

   $BinaryItem = Get-Item -LiteralPath $Binary -Force
   if ($BinaryItem.PSIsContainer -or $BinaryItem.Length -le 0 -or
       (($BinaryItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
       throw 'Preflight did not produce a plain, nonempty executable'
   }

   $Run = [Diagnostics.ProcessStartInfo]::new()
   $Run.UseShellExecute = $false
   $Run.CreateNoWindow = $true
   $Run.RedirectStandardOutput = $true
   $Run.RedirectStandardError = $true
   $Run.FileName = $Binary
   $Run.WorkingDirectory = $Scratch

   $RunProcess = [Diagnostics.Process]::Start($Run)
   $RunStdoutTask = $RunProcess.StandardOutput.ReadToEndAsync()
   $RunStderrTask = $RunProcess.StandardError.ReadToEndAsync()
   $RunProcess.WaitForExit()
   $RunStdout = $RunStdoutTask.Result
   $RunStderr = $RunStderrTask.Result
   [IO.File]::WriteAllText($RunStdoutPath, $RunStdout, $Utf8)
   [IO.File]::WriteAllText($RunStderrPath, $RunStderr, $Utf8)

   if ($RunProcess.ExitCode -ne 0 -or $RunStdout.Trim() -cne $Marker) {
       throw "Linked executable smoke failed; retained at $Scratch"
   }

   $Payload = [ordered]@{
       schema = 'pi.release.windows_msvc_link_preflight.v1'
       status = 'success'
       host = $env:COMPUTERNAME
       target = 'x86_64-pc-windows-msvc'
       compile_exit = $Process.ExitCode
       run_exit = $RunProcess.ExitCode
       run_stdout = $RunStdout.Trim()
       sha256 = (Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant()
       link_resolution = (
           Get-Content -LiteralPath (Join-Path $Scratch 'link-resolution.txt') -Raw
       ).Trim()
       cl_resolution = (
           Get-Content -LiteralPath (Join-Path $Scratch 'cl-resolution.txt') -Raw
       ).Trim()
       retained_path = $Scratch
   }
   $Json = $Payload | ConvertTo-Json -Compress
   [IO.File]::WriteAllText($RemoteReceipt, $Json + [Environment]::NewLine, $Utf8)
   [Console]::Out.WriteLine($Json)
   '''
   path = Path(sys.argv[1])
   with path.open("x", encoding="utf-8", newline="\n") as stream:
       stream.write(script)
   PY

   # Keep EncodedCommand below Windows' command-line limit. The tiny bootstrap
   # reads the audited script on stdin, parses it as one script block, and runs
   # it; encoding the full script is long enough to be truncated by OpenSSH.
   windows_preflight_bootstrap="$(python3 - <<'PY'
   import base64

   payload = (
       "$source = [Console]::In.ReadToEnd()\n"
       "$block = [ScriptBlock]::Create($source)\n"
       "& $block\n"
   )
   print(base64.b64encode(payload.encode("utf-16le")).decode("ascii"))
   PY
   )"

   set +e
   (
     set -C
     ssh -o BatchMode=yes -o ConnectTimeout=15 \
       "$windows_dsr_ssh_host" \
       powershell.exe -NoLogo -NoProfile -NonInteractive \
         -EncodedCommand "$windows_preflight_bootstrap" \
       < "$windows_preflight_ps1" \
       > "$windows_preflight_receipt" \
       2> "$windows_preflight_stderr"
   )
   windows_preflight_status=$?
   set -e
   unset windows_preflight_bootstrap
   test "$windows_preflight_status" -eq 0

   jq -e '
     .schema == "pi.release.windows_msvc_link_preflight.v1" and
     .status == "success" and
     .target == "x86_64-pc-windows-msvc" and
     .compile_exit == 0 and .run_exit == 0 and
     .run_stdout == "pi-dsr-msvc-link-preflight-ok" and
     (.sha256 | test("^[0-9a-f]{64}$")) and
     (.retained_path | type == "string" and length > 0)
   ' "$windows_preflight_receipt" >/dev/null

   printf 'windows_dsr_preflight_script_sha256=%s\nwindows_dsr_preflight_receipt_sha256=%s\n' \
     "$(sha256sum "$windows_preflight_ps1" | awk '{print $1}')" \
     "$(sha256sum "$windows_preflight_receipt" | awk '{print $1}')" \
     >> "$proof_file"

   git tag -a "$RELEASE_TAG" \
     -m "$RELEASE_TAG manual DSR release [skip actions]" "$source_commit"
   test "$(git cat-file -t "refs/tags/$RELEASE_TAG")" = tag
   test "$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")" = "$source_commit"
   test "$(git tag --list --format='%(contents:subject)' "$RELEASE_TAG")" = \
     "$RELEASE_TAG manual DSR release [skip actions]"

   DSR_BUILD_RUN_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
   export DSR_BUILD_RUN_ID
   [[ "$DSR_BUILD_RUN_ID" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
   export PRESERVED_DSR_STATE_DIR="$MANUAL_RELEASE_ROOT/dsr-state-$DSR_BUILD_RUN_ID"
   export RAW_RELEASE_DIR="$MANUAL_RELEASE_ROOT/raw-assets-$DSR_BUILD_RUN_ID"
   case "$PRESERVED_DSR_STATE_DIR:$RAW_RELEASE_DIR" in
     "$MANUAL_RELEASE_ROOT"/*:"$MANUAL_RELEASE_ROOT"/*) ;;
     *) exit 1 ;;
   esac
   build_receipt="$MANUAL_RELEASE_STATE_DIR/preserved-build-$DSR_BUILD_RUN_ID.json"
   test ! -e "$PRESERVED_DSR_STATE_DIR" && test ! -L "$PRESERVED_DSR_STATE_DIR"
   test ! -e "$RAW_RELEASE_DIR" && test ! -L "$RAW_RELEASE_DIR"
   test ! -e "$build_receipt"
   # Re-resolve the operator toolchain and rehash the preserved lane at the
   # last possible point. Inside the namespace the whole lane is mounted
   # read-only, and the mounted bytes are checked again before exec.
   verify_operator_tools
   verify_preserved_dsr_inputs
   sha256sum --check --strict --status "$preserved_inputs"
   (
     set -C
     "$release_bwrap_path" \
       --die-with-parent --new-session --bind / / --dev-bind /dev /dev \
       --ro-bind "$PRESERVED_DSR_LANE" "$PRESERVED_DSR_LANE" \
       --bind "$release_checkout" /data/projects/pi_agent_rust \
       --chdir /data/projects/pi_agent_rust \
       "$release_bash_path" --noprofile --norc -c '
         set -euo pipefail
         sha256sum_path="$1"
         preserved_receipt="$2"
         preserved_wrapper="$3"
         shift 3
         "$sha256sum_path" --check --strict --status "$preserved_receipt"
         exec "$preserved_wrapper" "$@"
       ' bash "$release_sha256sum_path" "$preserved_inputs" \
       "$PRESERVED_DSR_WRAPPER" \
       --run-id "$DSR_BUILD_RUN_ID" \
       --state-dir "$PRESERVED_DSR_STATE_DIR" \
       --output-dir "$RAW_RELEASE_DIR" -- \
       build pi --version 0.2.0 \
       --targets linux/amd64,linux/arm64,darwin/amd64,darwin/arm64,windows/amd64 \
       --only-native --jobs 1 > "$build_receipt"
   )
   # These values come from the receipt that was revalidated inside the
   # read-only execution mount. They, rather than duplicated literals, feed
   # every retained/public packaging receipt below.
   test "$(grep -Fc "$PRESERVED_DSR_WRAPPER" "$preserved_inputs")" = 1
   test "$(grep -Fc "$PRESERVED_DSR_AUDIT" "$preserved_inputs")" = 1
   test "$(grep -Fc \
     "$PRESERVED_DSR_LANE/preservation-manifest.sha256" \
     "$preserved_inputs")" = 1
   preserved_wrapper_sha256="$(awk -v path="$PRESERVED_DSR_WRAPPER" \
     '$2 == path {print $1}' "$preserved_inputs")"
   preserved_audit_sha256="$(awk -v path="$PRESERVED_DSR_AUDIT" \
     '$2 == path {print $1}' "$preserved_inputs")"
   preservation_manifest_sha256="$(awk \
     -v path="$PRESERVED_DSR_LANE/preservation-manifest.sha256" \
     '$2 == path {print $1}' "$preserved_inputs")"
   test "$preserved_wrapper_sha256" = "$expected_preserved_wrapper_sha256"
   test "$preserved_audit_sha256" = "$expected_preserved_audit_sha256"
   test "$preservation_manifest_sha256" = \
     "$expected_preservation_manifest_sha256"
   printf 'preserved_wrapper_sha256=%s\npreserved_audit_sha256=%s\npreservation_manifest_sha256=%s\n' \
     "$preserved_wrapper_sha256" "$preserved_audit_sha256" \
     "$preservation_manifest_sha256" >> "$proof_file"

   raw_manifest="$RAW_RELEASE_DIR/pi-v0.2.0-manifest.json"
   jq -e \
     --arg output "$RAW_RELEASE_DIR" \
     --arg manifest "$raw_manifest" '
     .command == "build" and .status == "success" and .exit_code == 0 and
     .details.tool == "pi" and .details.version == "0.2.0" and
     .details.total == 5 and .details.success == 5 and .details.failed == 0 and
     .details.output_dir == $output and .details.manifest == $manifest and
     .details.targets == [
       "linux/amd64", "linux/arm64", "darwin/amd64", "darwin/arm64",
       "windows/amd64"
     ]
   ' "$build_receipt" >/dev/null

   RAW_EXPECTED=(
     pi_linux_amd64
     pi_linux_arm64
     pi_darwin_amd64
     pi_darwin_arm64
     pi_windows_amd64.exe
     pi-v0.2.0-manifest.json
   )
   expected_raw="$(printf '%s\n' "${RAW_EXPECTED[@]}" | LC_ALL=C sort)"
   actual_raw="$(find "$RAW_RELEASE_DIR" -mindepth 1 -maxdepth 1 \
     -printf '%f\n' | LC_ALL=C sort)"
   test "$actual_raw" = "$expected_raw"
   for raw_name in "${RAW_EXPECTED[@]}"; do
     test -f "$RAW_RELEASE_DIR/$raw_name"
     test ! -L "$RAW_RELEASE_DIR/$raw_name"
     test -s "$RAW_RELEASE_DIR/$raw_name"
   done

   jq -e \
     --arg tag "$RELEASE_TAG" \
     --arg commit "$source_commit" \
     --arg run "$DSR_BUILD_RUN_ID" '
     .schema_version == "1.0.0" and .tool == "pi" and .version == $tag and
     .run_id == $run and .source.git_sha == $commit and
     .source.git_ref == $tag and (.source.dependencies | type) == "array" and
     .status == "success" and
     .summary == {total: 5, success: 5, failed: 0} and
     (.build_environments | length) == 5 and
     all(.build_environments[];
       .method == "native" and (.host | type) == "string" and
       (.host | length) > 0 and (.build_influence_env | type) == "object" and
       (.cargo_isolation | type) == "object") and
     ([.build_environments[].target] | sort) == [
       "darwin/amd64", "darwin/arm64", "linux/amd64", "linux/arm64",
       "windows/amd64"
     ] and
     (.artifacts | length) == 5 and
     ([.artifacts[] | {target, name}] | sort_by(.target)) == ([
       {target: "linux/amd64", name: "pi_linux_amd64"},
       {target: "linux/arm64", name: "pi_linux_arm64"},
       {target: "darwin/amd64", name: "pi_darwin_amd64"},
       {target: "darwin/arm64", name: "pi_darwin_arm64"},
       {target: "windows/amd64", name: "pi_windows_amd64.exe"}
     ] | sort_by(.target)) and
     all(.artifacts[];
       (.sha256 | test("^[0-9a-f]{64}$")) and
       (.size_bytes | type) == "number" and .size_bytes > 0 and
       .size_bytes < 23068672 and .archive_format == "binary" and
       .signed == false and .signature_file == "")
   ' "$raw_manifest" >/dev/null
   while IFS=$'\t' read -r raw_name expected_sha expected_size; do
     raw_path="$RAW_RELEASE_DIR/$raw_name"
     test "$(sha256sum "$raw_path" | awk '{print $1}')" = "$expected_sha"
     test "$(wc -c < "$raw_path" | tr -d '[:space:]')" = "$expected_size"
   done < <(jq -r '.artifacts[] | [.name, .sha256, .size_bytes] | @tsv' \
     "$raw_manifest")
   ```

   操作员保留的聚合清单证明了源代码/标签绑定、精确的 5/5 目标集合、DSR 记录的 `method = native` 通道标签、原生字节摘要/大小、构建影响环境收据、单次运行隔离的源代码根目录，以及可执行文件格式/架构检查。它**不**包含 `rustc -Vv` 编译器身份信息，也不能证明每个二进制文件已在其目标操作系统上成功执行。不要在公开清单中编造这两项声明。特别是，DSR 的 `native` 方法值不能证明硬件原生执行：这条已审计的通道在配置的 x86_64 主机上交叉构建 Linux ARM64。在注册表发布或公开 GitHub 发布之前，步骤 7 中的全部五个目标运行时冒烟测试均为强制性要求。

6. 在独立的控制器端阶段打包五个保留的原生二进制文件。此阶段读取已冻结的源代码对象和保留的聚合清单，但从不运行 DSR 或 Cargo。它使用带标签提交的时间戳作为 `SOURCE_DATE_EPOCH`，固定的归档成员排序/所有权/模式、USTAR+xz、ZIP deflate 级别 9，以及稳定的按键排序 JSON 序列化。对于固定的源代码、原生二进制文件、聚合清单以及 Python/压缩运行时，其输出字节是确定性的。

   公开的按目标（per-target）模式特意使用 `pi.release.dsr_build_manifest.v1`，而非自动化通道的 `pi.release.build_manifest.v1`：后者需要此保留构建收据未记录的编译器身份信息。每个手动清单改为将其原生制品以及构建环境和聚合 DSR 清单的不透明摘要承诺，绑定到精确的源代码对象、锁定的注册表依赖溯源（provenance）、最终归档和已归档的二进制文件。聚合清单、环境收据、保存通道审计和打包收据仍作为操作员保留的证据保存在 `MANUAL_RELEASE_STATE_DIR` 下；它们不是发布资产，公开清单不得暗示这些摘要承诺是可公开解析的。

   ```bash
   set -euo pipefail
   verify_operator_tools
   test "$(git rev-parse 'HEAD^{commit}')" = "$source_commit"
   test "$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")" = "$source_commit"
   test -z "$(git status --porcelain=v2 --untracked-files=all)"
   test -f "$raw_manifest" && test ! -L "$raw_manifest"
   verify_operator_tools
   verify_preserved_dsr_inputs
   sha256sum --check --strict --status "$preserved_inputs"
   export RELEASE_ARTIFACT_DIR="$MANUAL_RELEASE_STATE_DIR/artifacts"
   packaging_receipt="$MANUAL_RELEASE_STATE_DIR/deterministic-packaging.json"
   test ! -e "$RELEASE_ARTIFACT_DIR" && test ! -L "$RELEASE_ARTIFACT_DIR"
   test ! -e "$packaging_receipt"
   mkdir -m 700 "$RELEASE_ARTIFACT_DIR"
   (
     set -C
     RELEASE_ROOT="$(git rev-parse --show-toplevel)" \
     SOURCE_COMMIT="$source_commit" \
     RELEASE_TAG="$RELEASE_TAG" \
     RELEASE_VERSION="$RELEASE_VERSION" \
     RAW_RELEASE_DIR="$RAW_RELEASE_DIR" \
     RAW_MANIFEST="$raw_manifest" \
     DSR_BUILD_RUN_ID="$DSR_BUILD_RUN_ID" \
     RELEASE_ARTIFACT_DIR="$RELEASE_ARTIFACT_DIR" \
     PRESERVED_WRAPPER_SHA256="$preserved_wrapper_sha256" \
     PRESERVED_AUDIT_SHA256="$preserved_audit_sha256" \
     PRESERVATION_MANIFEST_SHA256="$preservation_manifest_sha256" \
     python3 - > "$packaging_receipt" <<'PY'
   import hashlib
   import io
   import json
   import os
   import re
   import stat
   import struct
   import subprocess
   import tarfile
   import tomllib
   import zipfile
   from datetime import datetime, timezone
   from pathlib import Path

   def fail(message):
       raise SystemExit(message)

   def strict_object(pairs):
       result = {}
       for key, value in pairs:
           if key in result:
               fail(f"duplicate JSON key: {key!r}")
           result[key] = value
       return result

   def strict_json(path):
       try:
           return json.loads(
               path.read_text(encoding="utf-8"), object_pairs_hook=strict_object
           )
       except (OSError, UnicodeError, json.JSONDecodeError) as error:
           fail(f"invalid JSON {path}: {error}")

   def git(root, *arguments):
       process = subprocess.run(
           ["git", "-C", str(root), *arguments],
           check=False,
           capture_output=True,
           text=True,
       )
       if process.returncode != 0:
           fail(f"git {' '.join(arguments)} failed: {process.stderr.strip()}")
       return process.stdout.strip()

   def sha256_bytes(data):
       return hashlib.sha256(data).hexdigest()

   def digest(path):
       data = path.read_bytes()
       return {"name": path.name, "sha256": sha256_bytes(data), "size": len(data)}

   def exclusive_write(path, data, mode):
       with path.open("xb") as output:
           output.write(data)
       path.chmod(mode)

   def validate_binary(data, triple):
       if triple.endswith("linux-gnu"):
           if len(data) < 20 or data[:5] != b"\x7fELF\x02" or data[5] != 1:
               fail(f"{triple} is not a 64-bit little-endian ELF image")
           machine = 0x3E if triple.startswith("x86_64") else 0xB7
           if struct.unpack_from("<H", data, 18)[0] != machine:
               fail(f"{triple} ELF machine mismatch")
       elif triple.endswith("apple-darwin"):
           if len(data) < 8 or data[:4] != b"\xcf\xfa\xed\xfe":
               fail(f"{triple} is not a little-endian Mach-O 64 image")
           cpu = 0x01000007 if triple.startswith("x86_64") else 0x0100000C
           if struct.unpack_from("<I", data, 4)[0] != cpu:
               fail(f"{triple} Mach-O CPU mismatch")
       elif triple == "x86_64-pc-windows-msvc":
           if len(data) < 64 or data[:2] != b"MZ":
               fail("Windows binary has no DOS/PE header")
           offset = struct.unpack_from("<I", data, 0x3C)[0]
           if offset + 6 > len(data) or data[offset:offset + 4] != b"PE\0\0":
               fail("Windows binary has an invalid PE header")
           if struct.unpack_from("<H", data, offset + 4)[0] != 0x8664:
               fail("Windows binary is not x86_64")
       else:
           fail(f"unsupported target triple: {triple}")

   def verify_archive(path, archive_root, binary_name, binary_bytes, license_bytes,
                      readme_bytes, source_epoch, zip_timestamp):
       expected = {
           f"{archive_root}/{binary_name}": (binary_bytes, 0o755),
           f"{archive_root}/LICENSE": (license_bytes, 0o644),
           f"{archive_root}/README.md": (readme_bytes, 0o644),
       }
       if path.suffix == ".zip":
           with zipfile.ZipFile(path) as archive:
               infos = archive.infolist()
               names = [info.filename.rstrip("/") for info in infos]
               if len(names) != len(set(names)) or set(names) != set(expected):
                   fail(f"ZIP inventory differs: {path}")
               for info, name in zip(infos, names, strict=True):
                   mode = info.external_attr >> 16
                   if info.is_dir() or info.flag_bits & 0x1 or stat.S_ISLNK(mode):
                       fail(f"ZIP contains an unsafe entry: {info.filename!r}")
                   if info.date_time != zip_timestamp or mode & 0o777 != expected[name][1]:
                       fail(f"ZIP member metadata differs: {info.filename!r}")
                   if archive.read(info) != expected[name][0]:
                       fail(f"ZIP member bytes differ: {info.filename!r}")
           return
       with tarfile.open(path, mode="r:xz") as archive:
           members = archive.getmembers()
           names = [member.name.rstrip("/") for member in members]
           expected_names = {archive_root, *expected}
           if len(names) != len(set(names)) or set(names) != expected_names:
               fail(f"tar inventory differs: {path}")
           for member, name in zip(members, names, strict=True):
               if name == archive_root:
                   if not member.isdir() or member.mode != 0o755:
                       fail(f"archive root is not a directory: {path}")
               elif not member.isreg() or member.issym() or member.islnk():
                   fail(f"tar contains an unsafe entry: {member.name!r}")
               else:
                   extracted = archive.extractfile(member)
                   if extracted is None or extracted.read() != expected[name][0]:
                       fail(f"tar member bytes differ: {member.name!r}")
                   if member.mode != expected[name][1]:
                       fail(f"tar member mode differs: {member.name!r}")
               if member.uid != 0 or member.gid != 0 \
                       or member.uname != "" or member.gname != "" \
                       or member.mtime != source_epoch:
                   fail(f"tar member metadata differs: {member.name!r}")

   root = Path(os.environ["RELEASE_ROOT"])
   commit = os.environ["SOURCE_COMMIT"]
   tag = os.environ["RELEASE_TAG"]
   version = os.environ["RELEASE_VERSION"]
   run_id = os.environ["DSR_BUILD_RUN_ID"]
   raw_dir = Path(os.environ["RAW_RELEASE_DIR"])
   raw_manifest_path = Path(os.environ["RAW_MANIFEST"])
   output_dir = Path(os.environ["RELEASE_ARTIFACT_DIR"])
   preservation_lane = {
       "wrapper_sha256": os.environ["PRESERVED_WRAPPER_SHA256"],
       "audit_sha256": os.environ["PRESERVED_AUDIT_SHA256"],
       "manifest_sha256": os.environ["PRESERVATION_MANIFEST_SHA256"],
   }
   if any(re.fullmatch(r"[0-9a-f]{64}", value) is None
          for value in preservation_lane.values()):
       fail("preservation-lane execution receipt contains an invalid digest")
   if re.fullmatch(r"[0-9a-f]{40}", commit) is None:
       fail("source commit is not a full SHA-1")
   if re.fullmatch(
       r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-"
       r"[89ab][0-9a-f]{3}-[0-9a-f]{12}",
       run_id,
   ) is None:
       fail("DSR run ID has an unexpected shape")
   if git(root, "rev-parse", "HEAD^{commit}") != commit:
       fail("HEAD differs from frozen source")
   if git(root, "rev-parse", f"refs/tags/{tag}^{{commit}}") != commit:
       fail("annotated tag differs from frozen source")
   if git(root, "cat-file", "-t", f"refs/tags/{tag}") != "tag":
       fail("release tag is not annotated")
   if git(root, "status", "--porcelain=v2", "--untracked-files=all"):
       fail("release checkout is dirty")
   if not output_dir.is_dir() or output_dir.is_symlink() or any(output_dir.iterdir()):
       fail("public artifact directory must be a fresh empty plain directory")

   support_paths = {
       "cargo_toml": "Cargo.toml",
       "cargo_lock": "Cargo.lock",
       "rust_toolchain": "rust-toolchain.toml",
       "license": "LICENSE",
       "readme": "README.md",
       "install": "install.sh",
       "dropin_verdict": "docs/evidence/dropin-certification-verdict.json",
       "models_generated_ts":
           "legacy_pi_mono_code/pi-mono/packages/ai/src/models.generated.ts",
   }
   source_blobs = {}
   for label, relative in support_paths.items():
       path = root / relative
       if path.is_symlink() or not path.is_file():
           fail(f"frozen source input is missing/non-regular: {relative}")
       blob = git(root, "rev-parse", f"{commit}:{relative}")
       tree_fields = git(root, "ls-tree", commit, "--", relative).split(maxsplit=3)
       expected_mode = "100755" if relative == "install.sh" else "100644"
       if len(tree_fields) != 4 or tree_fields[0] != expected_mode \
               or tree_fields[1] != "blob" or tree_fields[2] != blob \
               or tree_fields[3] != relative:
           fail(f"frozen source mode/type differs: {relative}")
       if git(root, "hash-object", "--no-filters", "--", relative) != blob:
           fail(f"worktree bytes differ from frozen blob: {relative}")
       source_blobs[label] = blob

   cargo = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
   if cargo["package"]["version"] != version or tag != f"v{version}":
       fail("Cargo version, release version, and tag differ")
   lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
   registry = "registry+https://github.com/rust-lang/crates.io-index"
   selected = []
   for package in lock["package"]:
       name = package["name"]
       if not (
           name in {"asupersync", "rich_rust"}
           or name.startswith("charmed-")
           or name.startswith("sqlmodel-")
       ):
           continue
       checksum = package.get("checksum")
       if package.get("source") != registry or not isinstance(checksum, str) \
               or re.fullmatch(r"[0-9a-f]{64}", checksum) is None:
           fail(f"invalid locked registry provenance for {name}")
       selected.append({
           "name": name,
           "version": package["version"],
           "source": registry,
           "checksum": checksum,
       })
   selected.sort(key=lambda item: (item["name"], item["version"]))
   identities = [(item["name"], item["version"]) for item in selected]
   required = {"asupersync", "rich_rust", "sqlmodel-core", "sqlmodel-sqlite"}
   if len(identities) != len(set(identities)) \
           or not required.issubset({name for name, _ in identities}):
       fail("locked release dependency selection is duplicate or incomplete")

   specs = {
       "linux/amd64": {
           "raw": "pi_linux_amd64", "asset": "pi-linux-amd64",
           "triple": "x86_64-unknown-linux-gnu", "runner_os": "Linux",
           "format": "tar.xz", "binary": "pi",
       },
       "linux/arm64": {
           "raw": "pi_linux_arm64", "asset": "pi-linux-arm64",
           "triple": "aarch64-unknown-linux-gnu", "runner_os": "Linux",
           "format": "tar.xz", "binary": "pi",
       },
       "darwin/amd64": {
           "raw": "pi_darwin_amd64", "asset": "pi-darwin-amd64",
           "triple": "x86_64-apple-darwin", "runner_os": "macOS",
           "format": "tar.xz", "binary": "pi",
       },
       "darwin/arm64": {
           "raw": "pi_darwin_arm64", "asset": "pi-darwin-arm64",
           "triple": "aarch64-apple-darwin", "runner_os": "macOS",
           "format": "tar.xz", "binary": "pi",
       },
       "windows/amd64": {
           "raw": "pi_windows_amd64.exe", "asset": "pi-windows-amd64",
           "triple": "x86_64-pc-windows-msvc", "runner_os": "Windows",
           "format": "zip", "binary": "pi.exe",
       },
   }
   expected_raw = {item["raw"] for item in specs.values()} | {
       f"pi-{tag}-manifest.json"
   }
   raw_entries = list(raw_dir.iterdir()) if raw_dir.is_dir() and not raw_dir.is_symlink() else []
   if len(raw_entries) != len(expected_raw) \
           or {entry.name for entry in raw_entries} != expected_raw:
       fail("raw DSR inventory is not exactly five binaries plus one manifest")
   if any(entry.is_symlink() or not entry.is_file() or entry.stat().st_size == 0
          for entry in raw_entries):
       fail("raw DSR inventory contains an invalid entry")

   if raw_manifest_path != raw_dir / f"pi-{tag}-manifest.json":
       fail("aggregate manifest path is outside the exact raw inventory")
   raw_manifest_bytes = raw_manifest_path.read_bytes()
   raw_manifest = strict_json(raw_manifest_path)
   expected_manifest_keys = {
       "schema_version", "tool", "version", "run_id", "source", "built_at",
       "duration_ms", "status", "summary", "build_environments", "artifacts",
   }
   if not isinstance(raw_manifest, dict) or set(raw_manifest) != expected_manifest_keys:
       fail("aggregate DSR manifest schema changed")
   if raw_manifest.get("schema_version") != "1.0.0" \
           or raw_manifest.get("tool") != "pi" \
           or raw_manifest.get("version") != tag \
           or raw_manifest.get("run_id") != run_id \
           or raw_manifest.get("status") != "success" \
           or raw_manifest.get("summary") != {"total": 5, "success": 5, "failed": 0} \
           or raw_manifest.get("source", {}).get("git_sha") != commit \
           or raw_manifest.get("source", {}).get("git_ref") != tag:
       fail("aggregate DSR manifest is not bound to this exact successful run")
   artifacts = raw_manifest.get("artifacts")
   environments = raw_manifest.get("build_environments")
   if not isinstance(artifacts, list) or len(artifacts) != 5 \
           or not isinstance(environments, list) or len(environments) != 5:
       fail("aggregate DSR manifest does not contain exact five-target receipts")
   artifacts_by_target = {item.get("target"): item for item in artifacts}
   environments_by_target = {item.get("target"): item for item in environments}
   if set(artifacts_by_target) != set(specs) or set(environments_by_target) != set(specs):
       fail("aggregate DSR manifest target set differs")
   if len(artifacts_by_target) != len(artifacts) \
           or len(environments_by_target) != len(environments):
       fail("aggregate DSR manifest contains duplicate targets")

   source_epoch = int(git(root, "show", "-s", "--format=%ct", commit))
   zip_time = datetime.fromtimestamp(source_epoch, tz=timezone.utc)
   if not 1980 <= zip_time.year <= 2107:
       fail("commit timestamp cannot be represented safely in ZIP")
   zip_timestamp = (
       zip_time.year, zip_time.month, zip_time.day,
       zip_time.hour, zip_time.minute, zip_time.second - zip_time.second % 2,
   )
   license_bytes = (root / "LICENSE").read_bytes()
   readme_bytes = (root / "README.md").read_bytes()
   aggregate_sha = sha256_bytes(raw_manifest_bytes)
   generated = []

   def tar_info(name, mode, size=0, directory=False):
       info = tarfile.TarInfo(name=name)
       info.type = tarfile.DIRTYPE if directory else tarfile.REGTYPE
       info.mode = mode
       info.uid = 0
       info.gid = 0
       info.uname = ""
       info.gname = ""
       info.mtime = source_epoch
       info.size = size
       return info

   def zip_info(name, mode):
       info = zipfile.ZipInfo(filename=name, date_time=zip_timestamp)
       info.create_system = 3
       info.compress_type = zipfile.ZIP_DEFLATED
       info.external_attr = (stat.S_IFREG | mode) << 16
       return info

   for dsr_target, spec in specs.items():
       raw_path = raw_dir / spec["raw"]
       raw_bytes = raw_path.read_bytes()
       raw_receipt = artifacts_by_target[dsr_target]
       environment = environments_by_target[dsr_target]
       if raw_receipt != {
           "name": spec["raw"],
           "target": dsr_target,
           "sha256": sha256_bytes(raw_bytes),
           "size_bytes": len(raw_bytes),
           "archive_format": "binary",
           "signed": False,
           "signature_file": "",
       }:
           fail(f"aggregate raw receipt differs for {dsr_target}")
       if len(raw_bytes) >= 22 * 1024 * 1024:
           fail(f"raw binary violates <22 MiB budget: {dsr_target}")
       if environment.get("target") != dsr_target \
               or environment.get("method") != "native" \
               or not isinstance(environment.get("host"), str) \
               or not environment["host"]:
           fail(f"invalid DSR build-environment receipt: {dsr_target}")
       validate_binary(raw_bytes, spec["triple"])

       archive_root = f"pi-{version}-{spec['triple']}"
       suffix = ".zip" if spec["format"] == "zip" else ".tar.xz"
       archive_path = output_dir / f"{spec['asset']}{suffix}"
       if archive_path.exists() or archive_path.is_symlink():
           fail(f"refusing to clobber {archive_path}")
       members = [
           (f"{archive_root}/{spec['binary']}", raw_bytes, 0o755),
           (f"{archive_root}/LICENSE", license_bytes, 0o644),
           (f"{archive_root}/README.md", readme_bytes, 0o644),
       ]
       with archive_path.open("xb") as output:
           if spec["format"] == "zip":
               with zipfile.ZipFile(
                   output, mode="w", compression=zipfile.ZIP_DEFLATED,
                   compresslevel=9, strict_timestamps=True,
               ) as archive:
                   for name, data, mode in members:
                       archive.writestr(
                           zip_info(name, mode), data,
                           compress_type=zipfile.ZIP_DEFLATED, compresslevel=9,
                       )
           else:
               with tarfile.open(
                   fileobj=output, mode="w:xz", format=tarfile.USTAR_FORMAT,
                   preset=9,
               ) as archive:
                   archive.addfile(tar_info(archive_root, 0o755, directory=True))
                   for name, data, mode in members:
                       archive.addfile(tar_info(name, mode, len(data)), io.BytesIO(data))
       archive_path.chmod(0o600)
       verify_archive(
           archive_path, archive_root, spec["binary"], raw_bytes,
           license_bytes, readme_bytes, source_epoch, zip_timestamp,
       )

       environment_bytes = json.dumps(
           environment, sort_keys=True, separators=(",", ":"), ensure_ascii=False
       ).encode("utf-8")
       manifest = {
           "schema": "pi.release.dsr_build_manifest.v1",
           "tag": tag,
           "version": version,
           "target": spec["triple"],
           "dsr_target": dsr_target,
           "asset": spec["asset"],
           "runner_os": spec["runner_os"],
           "pi_agent_rust": commit,
           "source_blobs": source_blobs,
           "selected_locked_registry_packages": selected,
           "raw_build": {
               "run_id": run_id,
               "operator_retained_aggregate_manifest": {
                   "name": raw_manifest_path.name,
                   "schema_version": "1.0.0",
                   "sha256": aggregate_sha,
               },
               "raw_binary": {
                   "name": spec["raw"],
                   "sha256": sha256_bytes(raw_bytes),
                   "size": len(raw_bytes),
               },
               "build_environment": {
                   "host": environment["host"],
                   "dsr_method_label": environment["method"],
                   "hardware_native_build_proven": False,
                   "operator_retained_receipt_sha256": sha256_bytes(environment_bytes),
               },
               "preservation_lane": preservation_lane,
           },
           "packaging": {
               "source_date_epoch": source_epoch,
               "archive_root": archive_root,
               "format": spec["format"],
               "metadata_policy": "fixed-order-uid0-gid0-source-epoch-v1",
           },
           "archive": digest(archive_path),
           "binary": {
               "name": spec["binary"],
               "sha256": sha256_bytes(raw_bytes),
               "size": len(raw_bytes),
           },
       }
       manifest_path = output_dir / f"build-manifest-{spec['asset']}.json"
       manifest_bytes = (
           json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
       ).encode("utf-8")
       exclusive_write(manifest_path, manifest_bytes, 0o600)
       generated.extend([archive_path.name, manifest_path.name])

   install_path = output_dir / "install.sh"
   exclusive_write(install_path, (root / "install.sh").read_bytes(), 0o700)
   generated.append(install_path.name)
   if len(generated) != 11 or len(set(generated)) != 11:
       fail("packaging stage did not create exactly eleven pre-checksum assets")
   checksum_path = output_dir / "SHA256SUMS"
   checksum_lines = []
   for name in sorted(generated):
       checksum_lines.append(f"{digest(output_dir / name)['sha256']}  {name}\n")
   exclusive_write(checksum_path, "".join(checksum_lines).encode("utf-8"), 0o600)
   if len(checksum_lines) != 11:
       fail("SHA256SUMS must contain exactly eleven lines")

   expected_public = set(generated) | {"SHA256SUMS"}
   public_entries = list(output_dir.iterdir())
   if len(public_entries) != 12 \
           or {entry.name for entry in public_entries} != expected_public:
       fail("public release inventory is not exactly twelve assets")
   if any(entry.is_symlink() or not entry.is_file() or entry.stat().st_size == 0
          for entry in public_entries):
       fail("public release inventory contains an invalid entry")
   receipt = {
       "schema": "pi.release.deterministic_packaging_receipt.v1",
       "tag": tag,
       "source_commit": commit,
       "source_date_epoch": source_epoch,
       "raw_manifest_sha256": aggregate_sha,
       "preservation_lane": preservation_lane,
       "assets": [digest(output_dir / name) for name in sorted(expected_public)],
   }
   print(json.dumps(receipt, indent=2, sort_keys=True))
   PY
   )

   EXPECTED_ASSETS=(
     pi-linux-amd64.tar.xz
     pi-linux-arm64.tar.xz
     pi-darwin-amd64.tar.xz
     pi-darwin-arm64.tar.xz
     pi-windows-amd64.zip
     install.sh
     SHA256SUMS
     build-manifest-pi-linux-amd64.json
     build-manifest-pi-linux-arm64.json
     build-manifest-pi-darwin-amd64.json
     build-manifest-pi-darwin-arm64.json
     build-manifest-pi-windows-amd64.json
   )
   expected_assets="$(printf '%s\n' "${EXPECTED_ASSETS[@]}" | LC_ALL=C sort)"
   actual_assets="$(find "$RELEASE_ARTIFACT_DIR" -mindepth 1 -maxdepth 1 \
     -printf '%f\n' | LC_ALL=C sort)"
   test "$actual_assets" = "$expected_assets"
   test "$(printf '%s\n' "$actual_assets" | wc -l | tr -d '[:space:]')" = 12
   for asset in "${EXPECTED_ASSETS[@]}"; do
     test -f "$RELEASE_ARTIFACT_DIR/$asset"
     test ! -L "$RELEASE_ARTIFACT_DIR/$asset"
     test -s "$RELEASE_ARTIFACT_DIR/$asset"
   done

   aggregate_sha256="$(sha256sum "$raw_manifest" | awk '{print $1}')"
   (
     cd "$RELEASE_ARTIFACT_DIR"
     test "$(wc -l < SHA256SUMS | tr -d '[:space:]')" = 11
     checksum_names="$(sed -E 's/^[0-9a-f]{64}  //' SHA256SUMS)"
     expected_checksum_names="$(printf '%s\n' "${EXPECTED_ASSETS[@]}" \
       | grep -v '^SHA256SUMS$' | LC_ALL=C sort)"
     test "$checksum_names" = "$expected_checksum_names"
     sha256sum --check --strict SHA256SUMS
     set -- build-manifest-pi-*.json
     test "$#" = 5
     for manifest in "$@"; do
       jq -e \
         --arg tag "$RELEASE_TAG" \
         --arg version "$RELEASE_VERSION" \
         --arg commit "$source_commit" \
         --arg run "$DSR_BUILD_RUN_ID" \
         --arg aggregate "$aggregate_sha256" \
         --arg wrapper "$preserved_wrapper_sha256" \
         --arg audit "$preserved_audit_sha256" \
         --arg preservation_manifest "$preservation_manifest_sha256" '
         .schema == "pi.release.dsr_build_manifest.v1" and
         .tag == $tag and .version == $version and
         .pi_agent_rust == $commit and .raw_build.run_id == $run and
         .raw_build.operator_retained_aggregate_manifest.sha256 == $aggregate and
         .raw_build.operator_retained_aggregate_manifest.schema_version == "1.0.0" and
         .raw_build.build_environment.dsr_method_label == "native" and
         .raw_build.build_environment.hardware_native_build_proven == false and
         (.raw_build.build_environment.operator_retained_receipt_sha256 |
           test("^[0-9a-f]{64}$")) and
         .raw_build.preservation_lane == {
           wrapper_sha256: $wrapper,
           audit_sha256: $audit,
           manifest_sha256: $preservation_manifest
         } and
         (has("rustc") | not) and
         (.archive.sha256 | test("^[0-9a-f]{64}$")) and
         (.archive.size | type) == "number" and .archive.size > 0 and
         (.binary.sha256 | test("^[0-9a-f]{64}$")) and
         (.binary.size | type) == "number" and
         .binary.size > 0 and .binary.size < 23068672
       ' "$manifest" >/dev/null
     done
   )
   jq -e \
     --arg tag "$RELEASE_TAG" \
     --arg commit "$source_commit" \
     --arg aggregate "$aggregate_sha256" \
     --arg wrapper "$preserved_wrapper_sha256" \
     --arg audit "$preserved_audit_sha256" \
     --arg preservation_manifest "$preservation_manifest_sha256" '
     .schema == "pi.release.deterministic_packaging_receipt.v1" and
     .tag == $tag and .source_commit == $commit and
     .raw_manifest_sha256 == $aggregate and
     .preservation_lane == {
       wrapper_sha256: $wrapper,
       audit_sha256: $audit,
       manifest_sha256: $preservation_manifest
     } and
     (.assets | length) == 12 and
     ([.assets[].name] | length) == ([.assets[].name] | unique | length)
   ' "$packaging_receipt" >/dev/null
   receipt_assets="$(jq -r '.assets[].name' "$packaging_receipt" | LC_ALL=C sort)"
   test "$receipt_assets" = "$expected_assets"
   while IFS=$'\t' read -r asset expected_sha expected_size; do
     test "$(sha256sum "$RELEASE_ARTIFACT_DIR/$asset" | awk '{print $1}')" = \
       "$expected_sha"
     test "$(wc -c < "$RELEASE_ARTIFACT_DIR/$asset" | tr -d '[:space:]')" = \
       "$expected_size"
   done < <(jq -r '.assets[] | [.name, .sha256, .size] | @tsv' \
     "$packaging_receipt")
   printf 'raw_manifest_sha256=%s\npackaging_receipt_sha256=%s\n' \
     "$aggregate_sha256" \
     "$(sha256sum "$packaging_receipt" | awk '{print $1}')" >> "$proof_file"
   ```

   现在定义精确的远端标签协调器（Define the exact remote-tag reconciler），但暂不调用。本地
   附注标签（annotated tag）可回退；受保护的远端标签不可回退。其首次
   调用被刻意推迟到全部五个目标运行时冒烟测试（target-runtime smoke）
   通过之后。仅当附注标签对象 ID 与剥离后提交（peeled commit）均与保留的本地对象完全一致时，
   重试方可采纳远端标签。任何其他状态均
   闭合失败（fail closed）；该函数永不移动或删除标签：

   ```bash
   set -euo pipefail
   verify_operator_tools
   immutable_ruleset_id="$(jq -er 'first(.[] |
     select(.target == "tag" and .enforcement == "active" and
       ((.conditions.ref_name.include | index("refs/tags/v*")) != null or
        (.conditions.ref_name.include | index("~ALL")) != null) and
       .conditions.ref_name.exclude == [] and
       ([.rules[].type] | index("update")) != null and
       ([.rules[].type] | index("deletion")) != null and
       (.bypass_actors | type) == "array" and .bypass_actors == [])) | .id' \
     "$ruleset_details")"

   reconcile_exact_remote_tag() {
     local attempt_id="$1"
     local attempt_dir="$2"
     local pretag_ruleset local_tag_object remote_refs
     local remote_tag_object remote_tag_commit push_status=0
     [[ "$attempt_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     test -d "$attempt_dir" && test ! -L "$attempt_dir"
     verify_operator_tools
     test "$(git rev-parse 'HEAD^{commit}')" = "$source_commit"
     test "$(git cat-file -t "refs/tags/$RELEASE_TAG")" = tag
     test "$(git rev-parse "refs/tags/$RELEASE_TAG^{commit}")" = "$source_commit"
     test -z "$(git status --porcelain=v2 --untracked-files=all)"
     local_tag_object="$(git rev-parse "refs/tags/$RELEASE_TAG")"
     [[ "$local_tag_object" =~ ^[0-9a-f]{40}$ ]]

     pretag_ruleset="$attempt_dir/pre-tag-ruleset.json"
     test ! -e "$pretag_ruleset"
     gh api -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/rulesets/${immutable_ruleset_id}?includes_parents=true" \
       > "$pretag_ruleset"
     jq -e '
       .target == "tag" and .enforcement == "active" and
       ((.conditions.ref_name.include | index("refs/tags/v*")) != null or
        (.conditions.ref_name.include | index("~ALL")) != null) and
       .conditions.ref_name.exclude == [] and
       ([.rules[].type] | index("update")) != null and
       ([.rules[].type] | index("deletion")) != null and
       (.bypass_actors | type) == "array" and .bypass_actors == []
     ' "$pretag_ruleset" >/dev/null
     git fetch --no-tags origin \
       refs/heads/main:refs/remotes/origin/main \
       refs/heads/master:refs/remotes/origin/master
     test "$(git rev-parse 'origin/main^{commit}')" = "$source_commit"
     test "$(git rev-parse 'origin/master^{commit}')" = "$source_commit"

     remote_refs="$(git ls-remote --tags origin \
       "refs/tags/$RELEASE_TAG" "refs/tags/$RELEASE_TAG^{}")"
     if test -z "$remote_refs"; then
       set +e
       origin_push_guarded "refs/tags/$RELEASE_TAG:refs/tags/$RELEASE_TAG"
       push_status=$?
       set -e
     fi
     assert_origin_push_disabled
     remote_tag_object="$(git ls-remote --tags origin \
       "refs/tags/$RELEASE_TAG" | awk 'NR == 1 {print $1}')"
     remote_tag_commit="$(git ls-remote --tags origin \
       "refs/tags/$RELEASE_TAG^{}" | awk 'NR == 1 {print $1}')"
     test "$remote_tag_object" = "$local_tag_object"
     test "$remote_tag_commit" = "$source_commit"
     (set -C; printf \
       'attempt_id=%s\npush_exit=%s\ntag_object=%s\ntag_commit=%s\n' \
       "$attempt_id" "$push_status" "$remote_tag_object" \
       "$remote_tag_commit" > "$attempt_dir/remote-tag-reconciliation.txt")
   }
   ```

7. 准备冻结的 GitHub release 正文并定义精确的草稿/产物协调器 (Prepare the frozen GitHub release body and define an exact draft/asset
   协调器 reconciler)，但暂不调用。**不要**运行
   `dsr release`：其面向恢复的传输层（recovery-oriented transport）可能会采纳已有的 release
   并可能移除其上传状态文件。此通道（lane）保留所有收据（receipt），
   且仅允许在仍在运行的 fail-fast 会话（session）内通过将经认证的远端清单（authenticated remote inventory）与精确保留的标签、
   正文和产物字节进行协调来重试。它永不
   删除或替换 release、产物、状态目录或收据。额外的、
   重复的、大小不同的或字节不匹配的远端产物即为硬性停止（hard stop）。

   release 正文也是冻结的发布输入（frozen publication input）。它必须包含
   在打标签的源码处从 `CHANGELOG.md` 提取的精确的 `v0.2.0` 章节，
   陈述实时的 `NOT_CERTIFIED` 结论，明确禁止严格的 drop-in 表述，
   并准确描述 `SHA256SUMS`：它覆盖其他十一个
   可下载产物，而非其自身。历史性的 `CERTIFIED` 结果绝不能
   被复制到当前正文中。

   ```bash
   set -euo pipefail
   verify_operator_tools
   expected_source_commit="$(awk -F= '$1 == "source_commit" {print $2}' "$proof_file")"
   expected_crate_sha256="$(awk -F= '$1 == "package_sha256" {print $2}' "$proof_file")"
   expected_crate_size="$(awk -F= '$1 == "package_size" {print $2}' "$proof_file")"
   [[ "$expected_source_commit" =~ ^[0-9a-f]{40}$ ]]
   [[ "$expected_crate_sha256" =~ ^[0-9a-f]{64}$ ]]
   [[ "$expected_crate_size" =~ ^[0-9]+$ ]]
   test "$(git rev-parse 'HEAD^{commit}')" = "$expected_source_commit"
   test -z "$(git status --porcelain=v2 --untracked-files=all)"

   verdict_source="$(jq -er '
     select(.schema == "pi.dropin.certification_verdict.v1" and
            .overall_verdict == "NOT_CERTIFIED" and
            (.git_commit | test("^[0-9a-f]{40}$")) and
            (.blocking_reasons | type) == "array" and
            (.blocking_reasons | length) > 0) | .git_commit
   ' docs/evidence/dropin-certification-verdict.json)"
   git merge-base --is-ancestor "$verdict_source" "$expected_source_commit"

   frozen_changelog="$MANUAL_RELEASE_STATE_DIR/CHANGELOG.frozen.md"
   changelog_section="$MANUAL_RELEASE_STATE_DIR/CHANGELOG-${RELEASE_TAG}.md"
   release_body="$MANUAL_RELEASE_STATE_DIR/RELEASE_BODY.md"
   test ! -e "$frozen_changelog" && test ! -e "$changelog_section"
   test ! -e "$release_body"
   (set -C; git show "${expected_source_commit}:CHANGELOG.md" > "$frozen_changelog")
   FROZEN_CHANGELOG="$frozen_changelog" \
     CHANGELOG_SECTION="$changelog_section" \
     RELEASE_TAG="$RELEASE_TAG" python3 - <<'PY'
   import os
   from pathlib import Path

   source = Path(os.environ["FROZEN_CHANGELOG"])
   output = Path(os.environ["CHANGELOG_SECTION"])
   lines = source.read_text(encoding="utf-8").splitlines(keepends=True)
   prefix = f"## [{os.environ['RELEASE_TAG']}]"
   starts = [index for index, line in enumerate(lines) if line.startswith(prefix)]
   if len(starts) != 1:
       raise SystemExit(f"expected exactly one changelog section for {prefix}")
   start = starts[0]
   end = next(
       (index for index in range(start + 1, len(lines)) if lines[index].startswith("## ")),
       len(lines),
   )
   section = "".join(lines[start:end]).rstrip() + "\n"
   if not section.startswith(prefix) or len(section.splitlines()) < 2:
       raise SystemExit("release changelog section is empty or malformed")
   with output.open("x", encoding="utf-8", newline="") as handle:
       handle.write(section)
   PY
   (set -C; {
     printf '%s\n' \
       "# ${RELEASE_TAG}" \
       "" \
       "Manual DSR release of pi_agent_rust ${RELEASE_VERSION}." \
       "" \
       "### Drop-in certification status" \
       "" \
       "**NOT_CERTIFIED** — This release is not certified as a strict drop-in replacement and must not be described as one." \
       "" \
       "Evidence: https://github.com/Dicklesworthstone/pi_agent_rust/blob/${RELEASE_TAG}/docs/evidence/dropin-certification-verdict.json" \
       "" \
       "### Changelog" \
       ""
     cat "$changelog_section"
     printf '%s\n' \
       "" \
       "SHA256SUMS covers each of the other eleven downloadable assets; as the checksum index, it does not checksum itself."
   } > "$release_body")
   grep -Fx '**NOT_CERTIFIED** — This release is not certified as a strict drop-in replacement and must not be described as one.' \
     "$release_body" >/dev/null
   RELEASE_BODY="$release_body" CHANGELOG_SECTION="$changelog_section" \
     python3 - <<'PY'
   import os
   from pathlib import Path

   body = Path(os.environ["RELEASE_BODY"]).read_bytes()
   section = Path(os.environ["CHANGELOG_SECTION"]).read_bytes()
   if body.count(section) != 1:
       raise SystemExit("release body does not contain the exact changelog section once")
   PY
   sha256sum "$release_body" > "$MANUAL_RELEASE_STATE_DIR/release-body.sha256"

   release_identity_receipt="$MANUAL_RELEASE_STATE_DIR/github-release-identity.json"

   reconcile_exact_github_draft() {
     local attempt_id="$1"
     local attempt_dir="$2"
     local create_status=0
     local precreate_inventory draft_payload draft_created create_response
     local postcreate_inventory release_id_receipt expected_upload_template
     local release_upload_url expected_assets remote_assets
     local release_id created_target_commitish
     local asset asset_path upload_response asset_size asset_count
     local upload_status upload_attempt metadata_after_asset asset_id
     local downloaded_asset upload_receipts
     local -a EXPECTED_ASSETS
     [[ "$attempt_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     test -d "$attempt_dir" && test ! -L "$attempt_dir"
     verify_operator_tools

     # The tag endpoint intentionally hides drafts. The authenticated paginated
     # inventory is authoritative. Zero matches permits creation; one match is
     # adopted only after every immutable field and byte is proved below.
     precreate_inventory="$attempt_dir/github-releases-before-reconcile.json"
     test ! -e "$precreate_inventory"
     gh api --paginate -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/releases?per_page=100" \
       | jq -s 'add' > "$precreate_inventory"
     jq -e --arg tag "$RELEASE_TAG" '
       type == "array" and
       all(.[];
         (.id | type) == "number" and .id > 0 and
         (.tag_name | type) == "string") and
       ([.[].id] | length) == ([.[].id] | unique | length) and
       ([.[] | select(.tag_name == $tag)] | length) <= 1
     ' "$precreate_inventory" >/dev/null

     draft_payload="$attempt_dir/github-draft-create-payload.json"
     draft_created="$attempt_dir/github-draft-reconciled.json"
     test ! -e "$draft_payload" && test ! -e "$draft_created"
     jq -n \
       --arg tag "$RELEASE_TAG" \
       --arg commit "$expected_source_commit" \
       --arg title "$RELEASE_TAG" \
       --rawfile body "$release_body" \
       '{tag_name: $tag, target_commitish: $commit, name: $title,
         body: $body, draft: true, prerelease: false}' \
       > "$draft_payload"
     if test "$(jq --arg tag "$RELEASE_TAG" \
       '[.[] | select(.tag_name == $tag)] | length' \
       "$precreate_inventory")" = 0; then
       create_response="$attempt_dir/github-draft-create-response.json"
       test ! -e "$create_response"
       set +e
       (set -C; gh api --method POST \
         -H 'Accept: application/vnd.github+json' \
         "/repos/${RELEASE_REPOSITORY}/releases" \
         --input "$draft_payload" > "$create_response")
       create_status=$?
       set -e
     fi

     # A lost POST response is not interpreted from its exit status. Refetch the
     # authenticated inventory and accept exactly one matching tag or fail.
     postcreate_inventory="$attempt_dir/github-releases-after-reconcile.json"
     test ! -e "$postcreate_inventory"
     gh api --paginate -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/releases?per_page=100" \
       | jq -s 'add' > "$postcreate_inventory"
     release_id="$(jq -er --arg tag "$RELEASE_TAG" '
       select(type == "array") |
       [.[] | select(.tag_name == $tag)] |
       select(length == 1) | .[0].id |
       select(type == "number" and . > 0)
     ' "$postcreate_inventory")"
     gh api -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/releases/${release_id}" > "$draft_created"
     created_target_commitish="$(jq -er '
       .target_commitish | select(type == "string" and length > 0)
     ' "$draft_created")"

     # GitHub documents target_commitish as unused when tag_name already
     # exists. The protected annotated tag object and peeled commit are the
     # commit authority; this receipt freezes the API metadata value and ID so
     # later retries can detect metadata substitution without misusing it as
     # tag-target proof.
     test ! -L "$release_identity_receipt"
     if test ! -e "$release_identity_receipt"; then
       (set -C; jq -n \
         --argjson id "$release_id" \
         --arg tag "$RELEASE_TAG" \
         --arg target_commitish "$created_target_commitish" \
         '{schema: "pi.release.github_identity.v1", id: $id,
           tag: $tag, target_commitish: $target_commitish}' \
         > "$release_identity_receipt")
     fi
     test -f "$release_identity_receipt" && test ! -L "$release_identity_receipt"
     jq -e \
       --argjson id "$release_id" \
       --arg tag "$RELEASE_TAG" \
       --arg target_commitish "$created_target_commitish" '
       .schema == "pi.release.github_identity.v1" and
       .id == $id and .tag == $tag and
       .target_commitish == $target_commitish
     ' "$release_identity_receipt" >/dev/null
     release_id_receipt="$attempt_dir/github-release-id.txt"
     test ! -e "$release_id_receipt"
     (set -C; printf \
       'attempt_id=%s\ncreate_exit=%s\nrelease_id=%s\ntag=%s\ntarget_commitish=%s\n' \
       "$attempt_id" "$create_status" "$release_id" "$RELEASE_TAG" \
       "$created_target_commitish" \
       > "$release_id_receipt")
   expected_upload_template="https://uploads.github.com/repos/${RELEASE_REPOSITORY}/releases/${release_id}/assets{?name,label}"
   jq -e \
     --argjson id "$release_id" \
     --arg tag "$RELEASE_TAG" \
     --arg target_commitish "$created_target_commitish" \
     --arg upload "$expected_upload_template" \
     --rawfile body "$release_body" \
     '.id == $id and .draft == true and .prerelease == false and
       .tag_name == $tag and .target_commitish == $target_commitish and
       .name == $tag and .body == $body and
       .upload_url == $upload and
       (.assets | type) == "array" and
       ([.assets[].id] | length) == ([.assets[].id] | unique | length) and
       ([.assets[].name] | length) == ([.assets[].name] | unique | length)' \
     "$draft_created" >/dev/null
   jq -e \
     --argjson id "$release_id" \
     --arg tag "$RELEASE_TAG" \
     --arg target_commitish "$created_target_commitish" '
     type == "array" and
     all(.[];
       (.id | type) == "number" and .id > 0 and
       (.tag_name | type) == "string") and
     ([.[].id] | length) == ([.[].id] | unique | length) and
     ([.[] | select(.tag_name == $tag)] | length) == 1 and
     ([.[] | select(.tag_name == $tag and .id == $id and
       .draft == true and .prerelease == false and
       .target_commitish == $target_commitish)] | length) == 1
   ' "$postcreate_inventory" >/dev/null
   release_upload_url="$(jq -er '
     .upload_url | sub("\\{\\?name,label\\}$"; "") |
     select(startswith("https://uploads.github.com/"))
   ' "$draft_created")"

   EXPECTED_ASSETS=(
     pi-linux-amd64.tar.xz
     pi-linux-arm64.tar.xz
     pi-darwin-amd64.tar.xz
     pi-darwin-arm64.tar.xz
     pi-windows-amd64.zip
     install.sh
     SHA256SUMS
     build-manifest-pi-linux-amd64.json
     build-manifest-pi-linux-arm64.json
     build-manifest-pi-darwin-amd64.json
     build-manifest-pi-darwin-arm64.json
     build-manifest-pi-windows-amd64.json
   )
   expected_assets="$(printf '%s\n' "${EXPECTED_ASSETS[@]}" | LC_ALL=C sort)"
   remote_assets="$(jq -r '.assets[].name' "$draft_created" | LC_ALL=C sort)"
   test -z "$remote_assets" || \
     test "$(comm -23 \
       <(printf '%s\n' "$remote_assets") \
       <(printf '%s\n' "$expected_assets"))" = ""
   upload_receipts="$attempt_dir/github-upload-reconciled"
   test ! -e "$upload_receipts" && test ! -L "$upload_receipts"
   mkdir -m 700 "$upload_receipts"
   for asset in "${EXPECTED_ASSETS[@]}"; do
     [[ "$asset" =~ ^[A-Za-z0-9._-]+$ ]]
     asset_path="$RELEASE_ARTIFACT_DIR/$asset"
     upload_response="$upload_receipts/${asset}.json"
     test -f "$asset_path" && test ! -L "$asset_path" && test -s "$asset_path"
     test ! -e "$upload_response"
     asset_size="$(wc -c < "$asset_path" | tr -d '[:space:]')"
     asset_count="$(jq --arg name "$asset" \
       '[.assets[] | select(.name == $name)] | length' "$draft_created")"
     test "$asset_count" = 0 || test "$asset_count" = 1
     upload_status=0
     if test "$asset_count" = 0; then
       upload_attempt="$attempt_dir/github-upload-attempt-${asset}.json"
       test ! -e "$upload_attempt"
       set +e
       (set -C; gh api --method POST \
         -H 'Accept: application/vnd.github+json' \
         -H 'Content-Type: application/octet-stream' \
         --input "$asset_path" \
         "${release_upload_url}?name=${asset}" \
         > "$upload_attempt")
       upload_status=$?
       set -e
     fi
     metadata_after_asset="$attempt_dir/github-release-after-${asset}.json"
     test ! -e "$metadata_after_asset"
     gh api -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/releases/${release_id}" \
       > "$metadata_after_asset"
     asset_id="$(jq -er \
       --arg name "$asset" --argjson size "$asset_size" '
       [.assets[] | select(
         .name == $name and .size == $size and .state == "uploaded" and
         (.id | type) == "number" and .id > 0)] |
       select(length == 1) | .[0].id
     ' "$metadata_after_asset")"
     downloaded_asset="$attempt_dir/github-asset-${asset}"
     test ! -e "$downloaded_asset" && test ! -L "$downloaded_asset"
     (set -C; gh api -H 'Accept: application/octet-stream' \
       "/repos/${RELEASE_REPOSITORY}/releases/assets/${asset_id}" \
       > "$downloaded_asset")
     cmp "$asset_path" "$downloaded_asset"
     (set -C; jq -e \
       --arg name "$asset" --argjson id "$asset_id" \
       --argjson size "$asset_size" '
       first(.assets[] | select(
         .id == $id and .name == $name and .size == $size and
         .state == "uploaded"))
     ' "$metadata_after_asset" > "$upload_response")
     (set -C; printf 'upload_exit=%s\nasset_id=%s\n' \
       "$upload_status" "$asset_id" \
       > "$attempt_dir/github-upload-${asset}.txt")
     draft_created="$metadata_after_asset"
   done
   test "$(jq -r '.assets[].name' "$draft_created" | LC_ALL=C sort)" = \
     "$expected_assets"
   (set -C; printf \
     'release_id=%s\nrelease_target_commitish=%s\nrelease_body_sha256=%s\n' \
     "$release_id" "$created_target_commitish" \
     "$(sha256sum "$release_body" | awk '{print $1}')" \
     > "$attempt_dir/github-draft-reconciliation.txt")
   }
   ```

   定义一个验证器（Define one verifier）并在公开前与公开后各立即使用一次。它
   绑定数据库 ID、草稿/公开状态、附注标签对象与
   剥离后提交（peeled commit）、冻结的 API 元数据、标题、正文、预发布（prerelease）
   标志、精确的 12 名称清单以及每一个已下载字节：

   ```bash
   set -euo pipefail
   verify_operator_tools
   EXPECTED_ASSETS=(
     pi-linux-amd64.tar.xz
     pi-linux-arm64.tar.xz
     pi-darwin-amd64.tar.xz
     pi-darwin-arm64.tar.xz
     pi-windows-amd64.zip
     install.sh
     SHA256SUMS
     build-manifest-pi-linux-amd64.json
     build-manifest-pi-linux-arm64.json
     build-manifest-pi-darwin-amd64.json
     build-manifest-pi-darwin-arm64.json
     build-manifest-pi-windows-amd64.json
   )
   expected_assets="$(printf '%s\n' "${EXPECTED_ASSETS[@]}" | LC_ALL=C sort)"
   local_assets="$(find "$RELEASE_ARTIFACT_DIR" -mindepth 1 -maxdepth 1 \
     -printf '%f\n' | LC_ALL=C sort)"
   test "$local_assets" = "$expected_assets"
   test "$(printf '%s\n' "$local_assets" | wc -l | tr -d '[:space:]')" = 12
   for asset in "${EXPECTED_ASSETS[@]}"; do
     test -f "$RELEASE_ARTIFACT_DIR/$asset"
     test ! -L "$RELEASE_ARTIFACT_DIR/$asset"
     test -s "$RELEASE_ARTIFACT_DIR/$asset"
   done

   verify_exact_release() {
     local expected_draft="$1"
     local label="$2"
     local inventory="$MANUAL_RELEASE_STATE_DIR/github-releases-${label}.json"
     local metadata="$MANUAL_RELEASE_STATE_DIR/github-release-${label}.json"
     local download_dir="$MANUAL_RELEASE_STATE_DIR/github-assets-${label}"
     local expected_release_id recorded_target_commitish remote_assets
     local remote_tag_object remote_tag_commit local_tag_object
     test "$expected_draft" = true || test "$expected_draft" = false
     [[ "$label" =~ ^[A-Za-z0-9._-]+$ ]]
     verify_operator_tools
     test -f "$release_identity_receipt" && test ! -L "$release_identity_receipt"
     expected_release_id="$(jq -er '
       select(.schema == "pi.release.github_identity.v1") |
       .id | select(type == "number" and . > 0)
     ' "$release_identity_receipt")"
     recorded_target_commitish="$(jq -er '
       select(.schema == "pi.release.github_identity.v1") |
       .target_commitish | select(type == "string" and length > 0)
     ' "$release_identity_receipt")"
     jq -e --arg tag "$RELEASE_TAG" '
       .schema == "pi.release.github_identity.v1" and .tag == $tag
     ' "$release_identity_receipt" >/dev/null
     test ! -e "$inventory" && test ! -e "$metadata"
     test ! -e "$download_dir"
     gh api --paginate -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/releases?per_page=100" \
       | jq -s 'add' > "$inventory"
     jq -e \
       --argjson id "$expected_release_id" \
       --argjson draft "$expected_draft" \
       --arg tag "$RELEASE_TAG" \
       --arg target_commitish "$recorded_target_commitish" '
       type == "array" and
       all(.[];
         (.id | type) == "number" and .id > 0 and
         (.tag_name | type) == "string") and
       ([.[].id] | length) == ([.[].id] | unique | length) and
       ([.[] | select(.tag_name == $tag)] | length) == 1 and
       ([.[] | select(.tag_name == $tag and .id == $id and
         .draft == $draft and .prerelease == false and
         .target_commitish == $target_commitish)] | length) == 1
     ' "$inventory" >/dev/null
     gh api -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/releases/${expected_release_id}" \
       > "$metadata"
     jq -e \
       --argjson id "$expected_release_id" \
       --argjson draft "$expected_draft" \
       --arg tag "$RELEASE_TAG" \
       --arg target_commitish "$recorded_target_commitish" \
       --rawfile body "$release_body" \
       '.id == $id and .draft == $draft and .prerelease == false and
        .tag_name == $tag and .target_commitish == $target_commitish and
        .name == $tag and .body == $body and
        (.assets | type) == "array" and (.assets | length) == 12 and
        ([.assets[].name] | length) == ([.assets[].name] | unique | length) and
        ([.assets[].id] | length) == ([.assets[].id] | unique | length) and
        all(.assets[];
          (.id | type) == "number" and .id > 0 and
          (.name | type) == "string" and .name != "" and
          .state == "uploaded" and
          (.size | type) == "number" and .size > 0)' \
       "$metadata" >/dev/null
     remote_assets="$(jq -r '.assets[].name' "$metadata" | LC_ALL=C sort)"
     test "$remote_assets" = "$expected_assets"
     mkdir -m 700 "$download_dir"
     for asset in "${EXPECTED_ASSETS[@]}"; do
       local local_asset recorded_asset_id recorded_asset_size downloaded_asset
       local_asset="$RELEASE_ARTIFACT_DIR/$asset"
       downloaded_asset="$download_dir/$asset"
       recorded_asset_size="$(wc -c < "$local_asset" | tr -d '[:space:]')"
       recorded_asset_id="$(jq -er \
         --arg name "$asset" \
         --argjson size "$recorded_asset_size" '
         [.assets[] | select(
           .name == $name and .state == "uploaded" and .size == $size and
           (.id | type) == "number" and .id > 0)] |
         select(length == 1) | .[0].id
       ' "$metadata")"
       test ! -e "$downloaded_asset" && test ! -L "$downloaded_asset"
       (set -C; gh api \
         -H 'Accept: application/octet-stream' \
         "/repos/${RELEASE_REPOSITORY}/releases/assets/${recorded_asset_id}" \
         > "$downloaded_asset")
       test -f "$downloaded_asset" && test ! -L "$downloaded_asset"
       test "$(wc -c < "$downloaded_asset" | tr -d '[:space:]')" = \
         "$recorded_asset_size"
       cmp "$local_asset" "$downloaded_asset"
     done
     local_tag_object="$(git rev-parse "refs/tags/$RELEASE_TAG")"
     remote_tag_object="$(git ls-remote --tags origin \
       "refs/tags/$RELEASE_TAG" | awk 'NR == 1 {print $1}')"
     remote_tag_commit="$(git ls-remote --tags origin \
       "refs/tags/$RELEASE_TAG^{}" | awk 'NR == 1 {print $1}')"
     [[ "$remote_tag_object" =~ ^[0-9a-f]{40}$ ]]
     test "$remote_tag_object" = "$local_tag_object"
     test "$remote_tag_object" != "$remote_tag_commit"
     test "$remote_tag_commit" = "$expected_source_commit"
   }

   reconcile_exact_github_publication() {
     local attempt_id="$1"
     local attempt_dir="$2"
     local release_id current_metadata current_draft
     local public_payload public_response patch_attempted=false patch_status=0
     [[ "$attempt_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     test -d "$attempt_dir" && test ! -L "$attempt_dir"
     verify_operator_tools
     test -f "$release_identity_receipt" && test ! -L "$release_identity_receipt"
     release_id="$(jq -er '
       select(.schema == "pi.release.github_identity.v1") |
       .id | select(type == "number" and . > 0)
     ' "$release_identity_receipt")"
     current_metadata="$attempt_dir/github-release-before-publication.json"
     test ! -e "$current_metadata"
     gh api -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/releases/${release_id}" \
       > "$current_metadata"
     current_draft="$(jq -er '
       .draft | select(type == "boolean")
     ' "$current_metadata")"
     verify_exact_release "$current_draft" "before-public-${attempt_id}"

     if test "$current_draft" = true; then
       public_payload="$attempt_dir/github-public-payload.json"
       public_response="$attempt_dir/github-public-response.json"
       test ! -e "$public_payload" && test ! -e "$public_response"
       jq -n \
         --arg tag "$RELEASE_TAG" \
         --arg title "$RELEASE_TAG" \
         --rawfile body "$release_body" \
         '{tag_name: $tag, name: $title,
           body: $body, draft: false, prerelease: false}' \
         > "$public_payload"
       patch_attempted=true
       set +e
       (set -C; gh api --method PATCH \
         -H 'Accept: application/vnd.github+json' \
         "/repos/${RELEASE_REPOSITORY}/releases/${release_id}" \
         --input "$public_payload" > "$public_response")
       patch_status=$?
       set -e
     fi

     # The authenticated inventory and downloaded bytes, not the PATCH process
     # status or response body, are authoritative after an ambiguous network
     # result. A retry sees the exact public state and skips the PATCH.
     verify_exact_release false "after-public-${attempt_id}"
     (set -C; printf \
       'attempt_id=%s\nrelease_id=%s\npatch_attempted=%s\npatch_exit=%s\n' \
       "$attempt_id" "$release_id" "$patch_attempted" "$patch_status" \
       > "$attempt_dir/github-publication-reconciliation.txt")
   }

   ```

   在跨越不可变的远端边界之前，在目标运行时上执行精确的五个已保留的原始二进制文件（retained raw
   binaries）。归档检查、文件格式检查、
   交叉编译成功以及仅执行控制器的 Linux 二进制文件
   均不能替代。Linux AMD64 原生运行。经审计的 Linux ARM64 分支
   在已配置的 x86_64 主机上通过 `qemu-aarch64` 加选定的 ARM64 sysroot 显式运行，
   并标记为 `qemu-emulated`；它不被表述
   为硬件原生（hardware-native）。macOS x86_64 分支显式在 Rosetta 下运行，
   而 macOS ARM64 分支原生运行。每次尝试都会获得全新的 UUID、
   远端目录和本地证据目录。失败的尝试会完整保留
   且永不复用。控制器在此 shell 中最多进行三次尝试，
   仅在某一次尝试产生恰好五个成功收据后才提升证据（proof）；
   这些命令刻意不执行任何清理。

   ```bash
   set -euo pipefail
   verify_operator_tools

   run_target_runtime_smoke_attempt() (
     set -euo pipefail
     local attempt_id="$1"
     local attempt_dir="$2"
     [[ "$attempt_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     test -d "$attempt_dir" && test ! -L "$attempt_dir"
     case "$attempt_dir" in
       "$MANUAL_RELEASE_STATE_DIR"/target-smoke-"$attempt_id") ;;
       *) exit 2 ;;
     esac
     verify_operator_tools

     smoke_unix_raw_binary() {
     local label="$1"
     local host="$2"
     local raw_name="$3"
     # OpenSSH does not reliably preserve an empty final argument.  Use an
     # explicit sentinel so the remote positional contract always has five
     # arguments and remains checkable under `set -u`.
     local qemu_sysroot="${4:-not-applicable}"
     local raw_path="$RAW_RELEASE_DIR/$raw_name"
     local expected_sha remote_dir receipt receipt_label
     [[ "$label" =~ ^(linux|darwin)-(amd64|arm64)$ ]]
     test -n "$host"
     test -f "$raw_path" && test ! -L "$raw_path" && test -s "$raw_path"
     expected_sha="$(jq -er --arg name "$raw_name" '
       first(.artifacts[] | select(.name == $name) | .sha256) |
       select(test("^[0-9a-f]{64}$"))
     ' "$raw_manifest")"
     test "$(sha256sum "$raw_path" | awk '{print $1}')" = "$expected_sha"
     remote_dir="pi-agent-rust-${RELEASE_TAG}-${DSR_BUILD_RUN_ID}-${attempt_id}-${label}"
     [[ "$remote_dir" =~ ^[A-Za-z0-9._-]+$ ]]
     receipt_label="$label"
     if test "$label" = linux-arm64; then
       receipt_label="linux-arm64-qemu-emulated"
       test "$qemu_sysroot" = "$LINUX_ARM64_QEMU_SYSROOT"
     else
       test "$qemu_sysroot" = not-applicable
     fi
     receipt="$attempt_dir/smoke-${receipt_label}.txt"
     test ! -e "$receipt"

     ssh "$host" sh -s -- "$remote_dir" <<'REMOTE'
   set -eu
   remote_dir="$1"
   case "$remote_dir" in
     *[!A-Za-z0-9._-]*|'') exit 2 ;;
   esac
   test ! -e "$HOME/$remote_dir"
   mkdir -m 700 "$HOME/$remote_dir"
   REMOTE
     scp -- "$raw_path" "${host}:${remote_dir}/pi"
     (set -C; ssh "$host" sh -s -- \
       "$label" "$remote_dir" "$RELEASE_VERSION" "$expected_sha" \
       "$qemu_sysroot" \
       > "$receipt" 2>&1 <<'REMOTE'
   set -eu
   label="$1"
   remote_dir="$2"
   expected_version="$3"
   expected_sha="$4"
   qemu_sysroot="$5"
   binary="$HOME/$remote_dir/pi"
   test -f "$binary" && test ! -L "$binary" && test -s "$binary"
   chmod 700 "$binary"
   host_arch="$(uname -m)"
   qemu_version="not-applicable"
   emulated_uname="not-applicable"

   if test "$label" = linux-arm64; then
     test "$qemu_sysroot" != not-applicable
   else
     test "$qemu_sysroot" = not-applicable
   fi

   case "$label" in
     linux-amd64)
       test "$(uname -s)" = Linux
       runtime_arch="$host_arch"
       test "$runtime_arch" = x86_64
       execution_mode="native"
       actual_sha="$(sha256sum "$binary" | awk '{print $1}')"
       version_output="$("$binary" --version)"
       "$binary" --help >/dev/null
       ;;
     linux-arm64)
       test "$(uname -s)" = Linux
       test "$host_arch" = x86_64
       case "$qemu_sysroot" in
         /*) ;;
         *) exit 3 ;;
       esac
       case "$qemu_sysroot" in *'/../'*|*'/..'|*'//'*) exit 3 ;; esac
       test -d "$qemu_sysroot"
       command -v qemu-aarch64 >/dev/null
       command -v file >/dev/null
       file -b "$binary" | grep -Eq 'ARM aarch64|aarch64'
       qemu_version="$(qemu-aarch64 --version | head -n 1)"
       test -n "$qemu_version"
       runtime_arch="aarch64"
       execution_mode="qemu-emulated"
       actual_sha="$(sha256sum "$binary" | awk '{print $1}')"
       version_output="$(qemu-aarch64 -L "$qemu_sysroot" "$binary" --version)"
       qemu-aarch64 -L "$qemu_sysroot" "$binary" --help >/dev/null
       if test -f "$qemu_sysroot/bin/uname"; then
         emulated_uname="$(qemu-aarch64 -L "$qemu_sysroot" \
           "$qemu_sysroot/bin/uname" -m)"
         case "$emulated_uname" in aarch64|arm64) ;; *) exit 3 ;; esac
       else
         emulated_uname="unavailable-in-selected-sysroot"
       fi
       ;;
     darwin-amd64)
       test "$(uname -s)" = Darwin
       runtime_arch="$(arch -x86_64 uname -m)"
       test "$runtime_arch" = x86_64
       execution_mode="rosetta-translated"
       actual_sha="$(shasum -a 256 "$binary" | awk '{print $1}')"
       version_output="$(arch -x86_64 "$binary" --version)"
       arch -x86_64 "$binary" --help >/dev/null
       ;;
     darwin-arm64)
       test "$(uname -s)" = Darwin
       runtime_arch="$(arch -arm64 uname -m)"
       test "$runtime_arch" = arm64
       execution_mode="native"
       actual_sha="$(shasum -a 256 "$binary" | awk '{print $1}')"
       version_output="$(arch -arm64 "$binary" --version)"
       arch -arm64 "$binary" --help >/dev/null
       ;;
     *) exit 4 ;;
   esac

   test "$actual_sha" = "$expected_sha"
   case "$version_output" in
     "pi $expected_version ("*) ;;
     *) printf 'unexpected version output: %s\n' "$version_output" >&2; exit 5 ;;
   esac
   receipt_label="$label"
   if test "$execution_mode" = qemu-emulated; then
     receipt_label="${label}-qemu-emulated"
   fi
   printf 'status=success\nlabel=%s\nos=%s\nhost_arch=%s\nruntime_arch=%s\nexecution_mode=%s\nsha256=%s\nversion=%s\nqemu_version=%s\nemulated_uname=%s\n' \
     "$receipt_label" "$(uname -s)" "$host_arch" "$runtime_arch" \
     "$execution_mode" "$actual_sha" "$version_output" "$qemu_version" \
     "$emulated_uname"
   REMOTE
     )
     grep -Fx 'status=success' "$receipt" >/dev/null
     grep -Fx "label=$receipt_label" "$receipt" >/dev/null
     grep -Fx "sha256=$expected_sha" "$receipt" >/dev/null
   }

   smoke_unix_raw_binary \
     linux-amd64 "$LINUX_AMD64_SMOKE_HOST" pi_linux_amd64
   smoke_unix_raw_binary \
     linux-arm64 "$LINUX_ARM64_SMOKE_HOST" pi_linux_arm64 \
     "$LINUX_ARM64_QEMU_SYSROOT"
   smoke_unix_raw_binary \
     darwin-amd64 "$DARWIN_SMOKE_HOST" pi_darwin_amd64
   smoke_unix_raw_binary \
     darwin-arm64 "$DARWIN_SMOKE_HOST" pi_darwin_arm64

   windows_raw="$RAW_RELEASE_DIR/pi_windows_amd64.exe"
   windows_expected_sha="$(jq -er '
     first(.artifacts[] | select(.name == "pi_windows_amd64.exe") | .sha256) |
     select(test("^[0-9a-f]{64}$"))
   ' "$raw_manifest")"
   test -f "$windows_raw" && test ! -L "$windows_raw" && test -s "$windows_raw"
   test "$(sha256sum "$windows_raw" | awk '{print $1}')" = "$windows_expected_sha"
   windows_remote_dir="pi-agent-rust-${RELEASE_TAG}-${DSR_BUILD_RUN_ID}-${attempt_id}-windows-amd64"
   [[ "$windows_remote_dir" =~ ^[A-Za-z0-9._-]+$ ]]
   windows_setup_ps="$attempt_dir/windows-smoke-setup.ps1"
   windows_smoke_ps="$attempt_dir/windows-smoke-run.ps1"
   windows_setup_receipt="$attempt_dir/windows-smoke-setup.txt"
   windows_receipt="$attempt_dir/smoke-windows-amd64.txt"
   test ! -e "$windows_setup_ps" && test ! -e "$windows_smoke_ps"
   test ! -e "$windows_setup_receipt" && test ! -e "$windows_receipt"
   WINDOWS_REMOTE_DIR="$windows_remote_dir" \
     WINDOWS_EXPECTED_SHA="$windows_expected_sha" \
     RELEASE_VERSION="$RELEASE_VERSION" \
     WINDOWS_SETUP_PS="$windows_setup_ps" \
     WINDOWS_SMOKE_PS="$windows_smoke_ps" python3 - <<'PY'
   import os
   import re
   from pathlib import Path

   remote_dir = os.environ["WINDOWS_REMOTE_DIR"]
   expected_sha = os.environ["WINDOWS_EXPECTED_SHA"]
   version = os.environ["RELEASE_VERSION"]
   if re.fullmatch(r"[A-Za-z0-9._-]+", remote_dir) is None:
       raise SystemExit("unsafe Windows smoke directory")
   if re.fullmatch(r"[0-9a-f]{64}", expected_sha) is None:
       raise SystemExit("invalid Windows smoke digest")
   if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) is None:
       raise SystemExit("invalid Windows smoke version")
   setup = f"""$ErrorActionPreference = 'Stop'
   $RemoteDir = Join-Path $HOME '{remote_dir}'
   if (Test-Path -LiteralPath $RemoteDir) {{ throw 'remote smoke directory already exists' }}
   New-Item -ItemType Directory -Path $RemoteDir -ErrorAction Stop | Out-Null
   Write-Output 'status=ready'
   """
   smoke = f"""$ErrorActionPreference = 'Stop'
   $RemoteDir = Join-Path $HOME '{remote_dir}'
   $Binary = Join-Path $RemoteDir 'pi.exe'
   if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {{ throw 'binary missing' }}
   $Item = Get-Item -LiteralPath $Binary -Force
   if (($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $Item.Length -le 0) {{
       throw 'binary is empty or a reparse point'
   }}
   $Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
   if ($Arch -ne 'X64') {{ throw "expected Windows X64 runtime, got $Arch" }}
   $ActualSha = (Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant()
   if ($ActualSha -ne '{expected_sha}') {{ throw 'Windows smoke digest mismatch' }}
   $VersionOutput = ((& $Binary --version 2>&1) -join "`n").Trim()
   if ($LASTEXITCODE -ne 0) {{ throw 'pi --version failed' }}
   if (-not $VersionOutput.StartsWith('pi {version} (')) {{ throw "unexpected version: $VersionOutput" }}
   & $Binary --help *> $null
   if ($LASTEXITCODE -ne 0) {{ throw 'pi --help failed' }}
   Write-Output 'status=success'
   Write-Output 'label=windows-amd64'
   Write-Output "os=$([System.Runtime.InteropServices.RuntimeInformation]::OSDescription)"
   Write-Output "arch=$Arch"
   Write-Output "sha256=$ActualSha"
   Write-Output "version=$VersionOutput"
   """
   with Path(os.environ["WINDOWS_SETUP_PS"]).open(
       "x", encoding="utf-8", newline="\n"
   ) as handle:
       handle.write(setup)
   with Path(os.environ["WINDOWS_SMOKE_PS"]).open(
       "x", encoding="utf-8", newline="\n"
   ) as handle:
       handle.write(smoke)
   PY
   (set -C; ssh "$WINDOWS_AMD64_SMOKE_HOST" \
     powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass \
       -Command - < "$windows_setup_ps" > "$windows_setup_receipt" 2>&1)
   test "$(tr -d '\r' < "$windows_setup_receipt" |
     grep -Fxc 'status=ready')" = 1
   scp -- "$windows_raw" \
     "${WINDOWS_AMD64_SMOKE_HOST}:${windows_remote_dir}/pi.exe"
   (set -C; ssh "$WINDOWS_AMD64_SMOKE_HOST" \
     powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass \
       -Command - < "$windows_smoke_ps" > "$windows_receipt" 2>&1)
   test "$(tr -d '\r' < "$windows_receipt" |
     grep -Fxc 'status=success')" = 1
   test "$(tr -d '\r' < "$windows_receipt" |
     grep -Fxc 'label=windows-amd64')" = 1
   test "$(tr -d '\r' < "$windows_receipt" |
     grep -Fxc "sha256=$windows_expected_sha")" = 1

   SMOKE_RECEIPTS=(
     "$attempt_dir/smoke-linux-amd64.txt"
     "$attempt_dir/smoke-linux-arm64-qemu-emulated.txt"
     "$attempt_dir/smoke-darwin-amd64.txt"
     "$attempt_dir/smoke-darwin-arm64.txt"
     "$attempt_dir/smoke-windows-amd64.txt"
   )
   test "${#SMOKE_RECEIPTS[@]}" = 5
   for smoke_receipt in "${SMOKE_RECEIPTS[@]}"; do
     test -f "$smoke_receipt" && test ! -L "$smoke_receipt" \
       && test -s "$smoke_receipt"
     test "$(tr -d '\r' < "$smoke_receipt" |
       grep -Fxc 'status=success')" = 1
   done
   (set -C; sha256sum "${SMOKE_RECEIPTS[@]}" \
     > "$attempt_dir/target-runtime-smokes.sha256")
   test "$(wc -l < "$attempt_dir/target-runtime-smokes.sha256" | tr -d '[:space:]')" = 5
   sha256sum --check --strict "$attempt_dir/target-runtime-smokes.sha256"
   (set -C; printf \
     'attempt_id=%s\nproof_sha256=%s\nstate=exact\n' \
     "$attempt_id" \
     "$(sha256sum "$attempt_dir/target-runtime-smokes.sha256" | awk '{print $1}')" \
     > "$attempt_dir/target-runtime-smokes-success.txt")
   )

   smoke_attempt_limit=3
   smoke_attempt_index=0
   successful_smoke_attempt_id=
   successful_smoke_attempt_dir=
   smoke_attempt_status=1
   while test "$smoke_attempt_index" -lt "$smoke_attempt_limit"; do
     smoke_attempt_index=$((smoke_attempt_index + 1))
     SMOKE_ATTEMPT_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
     [[ "$SMOKE_ATTEMPT_ID" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     smoke_attempt_dir="$MANUAL_RELEASE_STATE_DIR/target-smoke-$SMOKE_ATTEMPT_ID"
     test ! -e "$smoke_attempt_dir" && test ! -L "$smoke_attempt_dir"
     mkdir -m 700 "$smoke_attempt_dir"

     run_target_runtime_smoke_attempt \
       "$SMOKE_ATTEMPT_ID" "$smoke_attempt_dir" &
     smoke_attempt_pid=$!
     set +e
     wait "$smoke_attempt_pid"
     smoke_attempt_status=$?
     set -e
     if test "$smoke_attempt_status" -eq 0; then
       attempt_smoke_proof="$smoke_attempt_dir/target-runtime-smokes.sha256"
       test -f "$attempt_smoke_proof" && test ! -L "$attempt_smoke_proof"
       test "$(wc -l < "$attempt_smoke_proof" | tr -d '[:space:]')" = 5
       sha256sum --check --strict "$attempt_smoke_proof"
       attempt_smoke_success="$smoke_attempt_dir/target-runtime-smokes-success.txt"
       test -f "$attempt_smoke_success" && test ! -L "$attempt_smoke_success" \
         && test -s "$attempt_smoke_success"
       test "$(grep -Fxc "attempt_id=$SMOKE_ATTEMPT_ID" \
         "$attempt_smoke_success")" = 1
       test "$(grep -Fxc 'state=exact' "$attempt_smoke_success")" = 1
       test "$(grep -Fxc \
         "proof_sha256=$(sha256sum "$attempt_smoke_proof" | awk '{print $1}')" \
         "$attempt_smoke_success")" = 1
       successful_smoke_attempt_id="$SMOKE_ATTEMPT_ID"
       successful_smoke_attempt_dir="$smoke_attempt_dir"
       break
     fi

     (set -C; printf \
       'attempt_id=%s\nattempt_index=%s\nsmoke_exit=%s\nstate=unresolved\n' \
       "$SMOKE_ATTEMPT_ID" "$smoke_attempt_index" "$smoke_attempt_status" \
       > "$smoke_attempt_dir/target-runtime-smokes-unresolved.txt")
   done

   test "$smoke_attempt_status" -eq 0
   test -n "$successful_smoke_attempt_id"
   test -d "$successful_smoke_attempt_dir" \
     && test ! -L "$successful_smoke_attempt_dir"
   successful_attempt_smoke_proof="$successful_smoke_attempt_dir/target-runtime-smokes.sha256"
   canonical_smoke_proof="$MANUAL_RELEASE_STATE_DIR/target-runtime-smokes.sha256"
   test ! -e "$canonical_smoke_proof" && test ! -L "$canonical_smoke_proof"
   (set -C; cat "$successful_attempt_smoke_proof" > "$canonical_smoke_proof")
   cmp "$successful_attempt_smoke_proof" "$canonical_smoke_proof"
   test "$(wc -l < "$canonical_smoke_proof" | tr -d '[:space:]')" = 5
   sha256sum --check --strict "$canonical_smoke_proof"
   smoke_success_receipt="$MANUAL_RELEASE_STATE_DIR/target-runtime-smoke-success.txt"
   test ! -e "$smoke_success_receipt" && test ! -L "$smoke_success_receipt"
   (set -C; printf \
     'attempt_id=%s\nattempt_dir=%s\nproof_sha256=%s\nstate=exact\n' \
     "$successful_smoke_attempt_id" "$successful_smoke_attempt_dir" \
     "$(sha256sum "$canonical_smoke_proof" | awk '{print $1}')" \
     > "$smoke_success_receipt")

   reconcile_post_boundary_attempt() (
     set -euo pipefail
     local attempt_id="$1"
     local attempt_dir="$2"
     local smoke_proof="$MANUAL_RELEASE_STATE_DIR/target-runtime-smokes.sha256"
     local smoke_receipt="$MANUAL_RELEASE_STATE_DIR/target-runtime-smoke-success.txt"
     [[ "$attempt_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     test -d "$attempt_dir" && test ! -L "$attempt_dir"
     case "$attempt_dir" in
       "$MANUAL_RELEASE_STATE_DIR"/post-boundary-"$attempt_id") ;;
       *) exit 2 ;;
     esac
     verify_operator_tools
     test -f "$smoke_receipt" && test ! -L "$smoke_receipt" \
       && test -s "$smoke_receipt"
     test "$(grep -Fxc 'state=exact' "$smoke_receipt")" = 1
     test "$(grep -Fxc \
       "proof_sha256=$(sha256sum "$smoke_proof" | awk '{print $1}')" \
       "$smoke_receipt")" = 1
     sha256sum --check --strict "$smoke_proof"
     reconcile_exact_remote_tag "$attempt_id" "$attempt_dir"
     reconcile_exact_github_draft "$attempt_id" "$attempt_dir"
     verify_exact_release true "draft-${attempt_id}"
     test -f "$attempt_dir/remote-tag-reconciliation.txt" \
       && test ! -L "$attempt_dir/remote-tag-reconciliation.txt"
     test -f "$attempt_dir/github-draft-reconciliation.txt" \
       && test ! -L "$attempt_dir/github-draft-reconciliation.txt"
     (set -C; printf 'attempt_id=%s\nstate=exact\n' "$attempt_id" \
       > "$attempt_dir/post-boundary-reconciliation.txt")
   )

   # This is the first irreversible remote mutation. Every reversible package,
   # archive, and target-runtime check is complete. Each retry receives a fresh
   # append-only attempt directory, then adopts only byte-identical state.
   POST_BOUNDARY_ATTEMPT_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
   export POST_BOUNDARY_ATTEMPT_ID
   [[ "$POST_BOUNDARY_ATTEMPT_ID" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
   post_boundary_attempt_dir="$MANUAL_RELEASE_STATE_DIR/post-boundary-$POST_BOUNDARY_ATTEMPT_ID"
   test ! -e "$post_boundary_attempt_dir" && test ! -L "$post_boundary_attempt_dir"
   mkdir -m 700 "$post_boundary_attempt_dir"
   post_boundary_reconcile_status=0
   set +e
   (
     set -euo pipefail
     reconcile_post_boundary_attempt \
       "$POST_BOUNDARY_ATTEMPT_ID" "$post_boundary_attempt_dir"
   )
   post_boundary_reconcile_status=$?
   set -e
   if test "$post_boundary_reconcile_status" -eq 0; then
     post_boundary_success_receipt="$post_boundary_attempt_dir/post-boundary-reconciliation.txt"
     test -f "$post_boundary_success_receipt" \
       && test ! -L "$post_boundary_success_receipt" \
       && test -s "$post_boundary_success_receipt"
     test "$(grep -Fxc "attempt_id=$POST_BOUNDARY_ATTEMPT_ID" \
       "$post_boundary_success_receipt")" = 1
     test "$(grep -Fxc 'state=exact' "$post_boundary_success_receipt")" = 1
   else
     (set -C; printf \
       'attempt_id=%s\nreconciliation_exit=%s\nstate=unresolved\n' \
       "$POST_BOUNDARY_ATTEMPT_ID" "$post_boundary_reconcile_status" \
       > "$post_boundary_attempt_dir/post-boundary-unresolved.txt")
     printf '%s\n' \
       'tag/draft reconciliation is unresolved; retained all attempt state' >&2
   fi
   ```

如果在此边界之后网络/API 命令失败或其响应丢失，而控制性的 fail-fast shell 仍存活，请保留相同的私有检出、`MANUAL_RELEASE_STATE_DIR`、本地标签、包证明、原始二进制、制品、smoke 回执和发布正文。不要重新执行步骤 1–7、创建不同的正文或删除部分远端状态。在同一个 shell 中，选择新的 UUID/attempt 目录并仅重新运行以 `POST_BOUNDARY_ATTEMPT_ID=...` 开头的边界块。前台子 shell 保留 fail-fast 行为，而父进程捕获其状态且不将 reconciler 置于 Bash 条件中。标签 reconciler 要求精确的 annotated 对象和 peeled commit；草稿 reconciler 要求精确的标签/目标/标题/正文/状态，并且仅在下载并比较字节后才采纳资产。这是唯一授权的重试路径。如果控制 shell 终止，则停止：本 runbook 不提供独立的 post-boundary 恢复引导。

8. 在精确打标签提交上的干净发布者检出中，从冻结的发布工作流中具现并保留 checksum-gated Cargo 凭据提供方（credential provider）。不要替换为 `cargo:token`。下方 v0.2.0 已审查的工作流和提取的提供方哈希是有意的 fail-closed 锁定；此手动通道能够发布之前，任何后续工作流变更都需要显式审查和文档更新。

   ```bash
   set -euo pipefail
   verify_operator_tools
   test "$post_boundary_reconcile_status" -eq 0
   post_boundary_success_receipt="$post_boundary_attempt_dir/post-boundary-reconciliation.txt"
   test -f "$post_boundary_success_receipt" \
     && test ! -L "$post_boundary_success_receipt" \
     && test -s "$post_boundary_success_receipt"
   test "$(grep -Fxc "attempt_id=$POST_BOUNDARY_ATTEMPT_ID" \
     "$post_boundary_success_receipt")" = 1
   test "$(grep -Fxc 'state=exact' "$post_boundary_success_receipt")" = 1
   test "$(git rev-parse 'HEAD^{commit}')" = "$expected_source_commit"
   test -z "$(git status --porcelain=v2 --untracked-files=all)"
   smoke_proof="$MANUAL_RELEASE_STATE_DIR/target-runtime-smokes.sha256"
   test -f "$smoke_proof" && test ! -L "$smoke_proof"
   test "$(wc -l < "$smoke_proof" | tr -d '[:space:]')" = 5
   sha256sum --check --strict "$smoke_proof"
   frozen_workflow="$MANUAL_RELEASE_STATE_DIR/frozen-release-workflow.yml"
   provider="$MANUAL_RELEASE_STATE_DIR/pi-crates-credential-provider.py"
   provider_proof="$MANUAL_RELEASE_STATE_DIR/credential-provider.sha256"
   test ! -e "$frozen_workflow" && test ! -e "$provider" && test ! -e "$provider_proof"
   (set -C; git show \
     "$expected_source_commit:.github/workflows/release.yml" > "$frozen_workflow")
   test "$(sha256sum "$frozen_workflow" | awk '{print $1}')" = \
     df6b169fd80b34fb219154bb4255cf574b6a5130a504c60a1b192607aac3f2fd
   FROZEN_WORKFLOW="$frozen_workflow" PROVIDER_PATH="$provider" python3 - <<'PY'
   import os
   from pathlib import Path

   workflow = Path(os.environ["FROZEN_WORKFLOW"]).read_text(encoding="utf-8")
   start = "          source = r'''"
   end = "          '''\n          Path(os.environ[\"PROVIDER_PATH\"]).write_text(source, encoding=\"utf-8\")"
   if workflow.count(start) != 1 or workflow.count(end) != 1:
       raise SystemExit("frozen workflow does not contain one auditable provider source block")
   raw = workflow.split(start, 1)[1].split(end, 1)[0]
   lines = raw.splitlines(keepends=True)
   if not lines or lines[0] != "#!/usr/bin/env python3\n":
       raise SystemExit("credential provider block has an unexpected header")
   source = lines[0]
   for line in lines[1:]:
       if line.startswith("          "):
           source += line[10:]
       elif line == "\n":
           source += line
       else:
           raise SystemExit("credential provider block has unexpected YAML indentation")
   compile(source, os.environ["PROVIDER_PATH"], "exec")
   with Path(os.environ["PROVIDER_PATH"]).open("x", encoding="utf-8") as output:
       output.write(source)
   PY
   chmod 700 "$provider"
   test -f "$provider" && test ! -L "$provider"
   provider_sha256="$(sha256sum "$provider" | awk '{print $1}')"
   test "$provider_sha256" = \
     3aee4bc78904238aecba0ee6f973caae69027efaf28d5b1d649ddf9ef4aaf903
   (set -C; sha256sum "$frozen_workflow" "$provider" > "$provider_proof")
   ```

在读取任何真实令牌之前，对允许和拒绝行为进行对抗式自测。Cargo 对规范注册表（canonical-registry）的精确读取请求在没有发布回执的情况下被允许。成功的精确发布请求必须创建精确的回执；错误的校验和、注册表、身份或额外字段必须被拒绝且不创建回执。

   ```bash
   set -euo pipefail
   verify_operator_tools
   PROVIDER_PATH="$provider" \
   SELF_TEST_DIR="$MANUAL_RELEASE_STATE_DIR" \
   PACKAGE_VERSION="$RELEASE_VERSION" \
   CRATE_SHA256="$expected_crate_sha256" python3 - <<'PY'
   import json
   import os
   import subprocess
   from pathlib import Path

   provider = os.environ["PROVIDER_PATH"]
   root = Path(os.environ["SELF_TEST_DIR"])
   official = {"index-url": "sparse+https://index.crates.io/", "name": "crates-io"}
   publish = {
       "v": 1, "kind": "get", "operation": "publish",
       "name": "pi_agent_rust", "vers": os.environ["PACKAGE_VERSION"],
       "cksum": os.environ["CRATE_SHA256"], "registry": official, "args": [],
   }

   def invoke(label, request):
       receipt = root / f"provider-self-test-{label}.json"
       if receipt.exists():
           raise SystemExit(f"self-test path already exists: {receipt}")
       env = {
           **os.environ,
           "PI_CRATES_IO_RELEASE_TOKEN": "self-test-token",
           "PI_EXPECTED_CRATE_NAME": "pi_agent_rust",
           "PI_EXPECTED_CRATE_VERSION": os.environ["PACKAGE_VERSION"],
           "PI_EXPECTED_CRATE_SHA256": os.environ["CRATE_SHA256"],
           "PI_CREDENTIAL_RECEIPT": str(receipt),
       }
       process = subprocess.run(
           [provider, "--cargo-plugin"],
           input=json.dumps(request, separators=(",", ":")) + "\n",
           capture_output=True, text=True, env=env, timeout=10, check=False,
       )
       lines = process.stdout.splitlines()
       if process.returncode != 0 or len(lines) != 2 or json.loads(lines[0]) != {"v": [1]}:
           raise SystemExit(f"credential-provider protocol failure: {label}")
       return json.loads(lines[1]), receipt

   read = {"v": 1, "kind": "get", "operation": "read", "registry": official, "args": []}
   response, receipt = invoke("read", read)
   if response.get("Ok", {}).get("token") != "self-test-token" or receipt.exists():
       raise SystemExit("read allow self-test failed")
   response, receipt = invoke("exact-publish", publish)
   if response.get("Ok") != {
       "kind": "get", "token": "self-test-token", "cache": "never",
       "operation_independent": False,
   } or not receipt.is_file():
       raise SystemExit("exact publish allow self-test failed")
   expected_receipt = {
       "schema": "pi.release.cargo_credential_receipt.v1",
       "name": "pi_agent_rust", "version": os.environ["PACKAGE_VERSION"],
       "crate_sha256": os.environ["CRATE_SHA256"],
       "registry_name": "crates-io", "registry_index_url": official["index-url"],
   }
   if json.loads(receipt.read_text(encoding="utf-8")) != expected_receipt:
       raise SystemExit("exact publish receipt differs")
   denials = {
       "wrong-checksum": {**publish, "cksum": "0" * 64},
       "wrong-name": {**publish, "name": "other"},
       "wrong-version": {**publish, "vers": "999.0.0"},
       "wrong-registry": {**publish, "registry": {**official, "name": "other"}},
       "extra-field": {**publish, "unexpected": True},
   }
   for label, request in denials.items():
       response, receipt = invoke(label, request)
       if "Err" not in response or receipt.exists():
           raise SystemExit(f"credential-provider deny self-test failed: {label}")
   PY
   test "$(sha256sum "$provider" | awk '{print $1}')" = "$provider_sha256"
   ```

在此隔离的发布者路径上重新创建包并匹配源证明，然后再将捕获的令牌暴露给任何子进程。随后以命令行优先级将两项 Cargo 凭据设置强制指向已审查的提供方。该提供方为 Cargo 对 crates.io 的规范读取请求提供令牌而不创建发布回执。对于发布请求，仅当 Cargo 自身出示精确的 crate 名称、版本、注册表和 SHA-256 时，它才提供令牌并写入回执。所有构建和 dry-run 都在真实令牌进入任何子进程环境之前发生。因此真实发布使用 `--no-verify`：Cargo 默认的发布验证会构建已打包的源，而任何包或依赖的构建脚本否则都会继承令牌。checksum-gated 提供方仍能证明 Cargo 正在上传已在无密钥情况下构建并验证过的精确 crate 字节。

   ```bash
   set -euo pipefail
   verify_operator_tools
   sha256sum --check --strict \
     "$MANUAL_RELEASE_STATE_DIR/target-runtime-smokes.sha256"
   verify_exact_release true immediately-before-crates-publication
   manifest_abs="$(realpath Cargo.toml)"
   publisher_home="$MANUAL_RELEASE_STATE_DIR/publisher-home"
   publisher_cargo_home="$MANUAL_RELEASE_STATE_DIR/publisher-cargo-home"
   publisher_cwd="$MANUAL_RELEASE_STATE_DIR/publisher-cwd"
   publisher_target_dir="$MANUAL_RELEASE_STATE_DIR/publisher-target"
   publisher_tmp_dir="$MANUAL_RELEASE_STATE_DIR/publisher-tmp"
   test ! -e "$publisher_home" && test ! -L "$publisher_home"
   test ! -e "$publisher_cargo_home" && test ! -L "$publisher_cargo_home"
   test ! -e "$publisher_cwd" && test ! -L "$publisher_cwd"
   test ! -e "$publisher_target_dir" && test ! -L "$publisher_target_dir"
   test ! -e "$publisher_tmp_dir" && test ! -L "$publisher_tmp_dir"
   mkdir -m 700 \
     "$publisher_home" "$publisher_cargo_home" "$publisher_cwd" \
     "$publisher_target_dir" "$publisher_tmp_dir"
   (set -C; printf \
     'publisher_home=%s\npublisher_cargo_home=%s\npublisher_cwd=%s\npublisher_target_dir=%s\npublisher_tmp_dir=%s\n' \
     "$publisher_home" "$publisher_cargo_home" "$publisher_cwd" \
     "$publisher_target_dir" "$publisher_tmp_dir" \
     > "$MANUAL_RELEASE_STATE_DIR/publisher-paths.txt")
   publisher_env() {
     env -i \
       PATH="$PATH" \
       HOME="$publisher_home" \
       CARGO_HOME="$publisher_cargo_home" \
       CARGO_TARGET_DIR="$publisher_target_dir" \
       TMPDIR="$publisher_tmp_dir" \
       XDG_CACHE_HOME="$publisher_home/.cache" \
       XDG_CONFIG_HOME="$publisher_home/.config" \
       XDG_DATA_HOME="$publisher_home/.local/share" \
       RUSTUP_TOOLCHAIN="$RUSTUP_TOOLCHAIN" \
       RCH_CARGO_WRAPPER_BYPASS="$RCH_CARGO_WRAPPER_BYPASS" \
       GIT_CONFIG_GLOBAL=/dev/null \
       GIT_CONFIG_NOSYSTEM=1 \
       LANG=C.UTF-8 LC_ALL=C.UTF-8 TZ=UTC TERM=dumb NO_COLOR=1 \
       RUST_BACKTRACE=1 CARGO_TERM_COLOR=never \
       USER="${USER:-release}" LOGNAME="${LOGNAME:-${USER:-release}}" \
       "$@"
   }
   publisher_env cargo --version >/dev/null
   (
     cd "$publisher_cwd"
     publisher_env cargo publish --manifest-path "$manifest_abs" \
       --dry-run --locked --registry crates-io
   )
   publisher_crate="$publisher_target_dir/package/pi_agent_rust-${RELEASE_VERSION}.crate"
   test -f "$publisher_crate" && test ! -L "$publisher_crate"
   test "$(sha256sum "$publisher_crate" | awk '{print $1}')" = "$expected_crate_sha256"
   test "$(wc -c < "$publisher_crate" | tr -d '[:space:]')" = "$expected_crate_size"
   test -z "$(git status --porcelain=v2 --untracked-files=all)"
   test "$(sha256sum "$provider" | awk '{print $1}')" = "$provider_sha256"

   registry_credential_config="$(publisher_env PROVIDER_PATH="$provider" python3 - <<'PY'
   import json, os
   print("registry.credential-provider=" + json.dumps(os.environ["PROVIDER_PATH"]))
   PY
   )"
   named_credential_config="$(publisher_env PROVIDER_PATH="$provider" python3 - <<'PY'
   import json, os
   print("registries.crates-io.credential-provider=" + json.dumps(os.environ["PROVIDER_PATH"]))
   PY
   )"
   actual_registry_provider="$(
     cd "$publisher_cwd"
     publisher_env cargo -Z unstable-options config get registry.credential-provider \
         --format=json-value \
         --config 'registry.credential-provider="/bin/false"' \
         --config "$registry_credential_config" \
         --config 'registries.crates-io.credential-provider="/bin/false"' \
         --config "$named_credential_config"
   )"
   actual_named_provider="$(
     cd "$publisher_cwd"
     publisher_env cargo -Z unstable-options config get registries.crates-io.credential-provider \
         --format=json-value \
         --config 'registry.credential-provider="/bin/false"' \
         --config "$registry_credential_config" \
         --config 'registries.crates-io.credential-provider="/bin/false"' \
         --config "$named_credential_config"
   )"
   test "$(jq -er '.' <<<"$actual_registry_provider")" = "$provider"
   test "$(jq -er '.' <<<"$actual_named_provider")" = "$provider"

   publish_exact_crate_with_scoped_token() {
     local credential_receipt="$1"
     local controller_token="$release_crates_io_token"
     [[ -n "$controller_token" ]]
     (( ${#controller_token} <= 4096 ))
     case "$controller_token" in *$'\n'*|*$'\r'*) return 2 ;; esac
     builtin export -n controller_token

     # Keep the real token out of argv and the controller environment. The
     # left side is a Bash builtin writing to an anonymous pipe. The clean
     # publisher child reads exactly one validated line, exports it only for
     # Cargo's final process image, replaces stdin with /dev/null so Cargo
     # cannot consume credential bytes, and then execs the no-verify upload.
     builtin printf '%s\n' "$controller_token" |
       publisher_env \
         PI_EXPECTED_CRATE_NAME=pi_agent_rust \
         PI_EXPECTED_CRATE_VERSION="$RELEASE_VERSION" \
         PI_EXPECTED_CRATE_SHA256="$expected_crate_sha256" \
         PI_CREDENTIAL_RECEIPT="$credential_receipt" \
         "$release_bash_path" --noprofile --norc -c '
           set -euo pipefail
           [[ -z "${PI_CRATES_IO_RELEASE_TOKEN:-}" ]]
           IFS= read -r scoped_release_token
           [[ -n "$scoped_release_token" ]]
           (( ${#scoped_release_token} <= 4096 ))
           case "$scoped_release_token" in *$'"'"'\n'"'"'*|*$'"'"'\r'"'"'*) exit 2 ;; esac
           export PI_CRATES_IO_RELEASE_TOKEN="$scoped_release_token"
           unset scoped_release_token
           exec 0</dev/null
           cd "$1"
           shift
           exec cargo publish --manifest-path "$1" --locked --no-verify \
             --registry crates-io \
             --config "$2" \
             --config "$3"
         ' bash "$publisher_cwd" "$manifest_abs" \
           "$registry_credential_config" "$named_credential_config"
   }

   precrate_ruleset="$MANUAL_RELEASE_STATE_DIR/pre-crates-publication-ruleset.json"
   test ! -e "$precrate_ruleset"
   gh api -H 'Accept: application/vnd.github+json' \
     "/repos/${RELEASE_REPOSITORY}/rulesets/${immutable_ruleset_id}?includes_parents=true" \
     > "$precrate_ruleset"
   jq -e '
     .target == "tag" and .enforcement == "active" and
     ((.conditions.ref_name.include | index("refs/tags/v*")) != null or
      (.conditions.ref_name.include | index("~ALL")) != null) and
     .conditions.ref_name.exclude == [] and
     ([.rules[].type] | index("update")) != null and
     ([.rules[].type] | index("deletion")) != null and
     (.bypass_actors | type) == "array" and .bypass_actors == []
   ' "$precrate_ruleset" >/dev/null

   record_exact_crates_state() {
     local output="$1"
     local max_attempts="$2"
     test ! -e "$output" && test ! -L "$output"
     [[ "$max_attempts" =~ ^[1-9][0-9]*$ ]]
     OUTPUT="$output" MAX_ATTEMPTS="$max_attempts" \
       PACKAGE_VERSION="$RELEASE_VERSION" \
       CRATE_SHA256="$expected_crate_sha256" python3 - <<'PY'
   import json
   import os
   import re
   import time
   import urllib.error
   import urllib.parse
   import urllib.request
   from pathlib import Path

   MAX_RESPONSE_BYTES = 1024 * 1024

   def strict_object(pairs):
       result = {}
       for key, value in pairs:
           if key in result:
               raise SystemExit(f"duplicate crates.io response key: {key!r}")
           result[key] = value
       return result

   endpoint = (
       "https://crates.io/api/v1/crates/pi_agent_rust/"
       + urllib.parse.quote(os.environ["PACKAGE_VERSION"], safe="")
   )
   max_attempts = int(os.environ["MAX_ATTEMPTS"])
   state = "absent"
   for attempt in range(1, max_attempts + 1):
       request = urllib.request.Request(
           endpoint,
           headers={
               "Accept": "application/json",
               "User-Agent": "pi-agent-rust-manual-release",
           },
       )
       try:
           with urllib.request.urlopen(request, timeout=30) as response:
               body = response.read(MAX_RESPONSE_BYTES + 1)
       except urllib.error.HTTPError as exc:
           if exc.code != 404:
               raise
       else:
           if len(body) > MAX_RESPONSE_BYTES:
               raise SystemExit("crates.io response exceeds 1 MiB")
           payload = json.loads(body, object_pairs_hook=strict_object)
           version = payload.get("version") if isinstance(payload, dict) else None
           if not isinstance(version, dict) \
               or version.get("crate") != "pi_agent_rust" \
               or version.get("num") != os.environ["PACKAGE_VERSION"] \
               or version.get("yanked") is not False \
               or version.get("checksum") != os.environ["CRATE_SHA256"] \
               or re.fullmatch(r"[0-9a-f]{64}", version.get("checksum", "")) is None:
               raise SystemExit(
                   "existing crates.io version identity/checksum/yank state differs"
               )
           state = "exact"
           break
       if attempt != max_attempts:
           time.sleep(5)
   receipt = {
       "schema": "pi.release.crates_reconciliation.v1",
       "state": state,
       "attempts": attempt,
       "name": "pi_agent_rust",
       "version": os.environ["PACKAGE_VERSION"],
       "expected_checksum": os.environ["CRATE_SHA256"],
   }
   with Path(os.environ["OUTPUT"]).open("x", encoding="utf-8") as handle:
       json.dump(receipt, handle, indent=2, sort_keys=True)
       handle.write("\n")
   PY
   }

   reconcile_exact_crates_publication() {
     local attempt_id="$1"
     local attempt_dir="$2"
     local before_state after_state actual_receipt
     local cargo_status=not-run receipt_sha256=not-applicable post_attempts=1
     [[ "$attempt_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     test -d "$attempt_dir" && test ! -L "$attempt_dir"
     case "$attempt_dir" in
       "$MANUAL_RELEASE_STATE_DIR"/crates-"$attempt_id") ;;
       *) exit 2 ;;
     esac
     verify_operator_tools
     verify_exact_release true "before-crates-${attempt_id}"
     sha256sum --check --strict \
       "$MANUAL_RELEASE_STATE_DIR/target-runtime-smokes.sha256"
     test -f "$publisher_crate" && test ! -L "$publisher_crate"
     test "$(sha256sum "$publisher_crate" | awk '{print $1}')" = \
       "$expected_crate_sha256"
     test "$(wc -c < "$publisher_crate" | tr -d '[:space:]')" = \
       "$expected_crate_size"
     test "$(sha256sum "$provider" | awk '{print $1}')" = "$provider_sha256"

     before_state="$attempt_dir/crates-before.json"
     record_exact_crates_state "$before_state" 1
     if ! jq -e '.state == "exact"' "$before_state" >/dev/null; then
       jq -e '.state == "absent"' "$before_state" >/dev/null
       actual_receipt="$attempt_dir/pi-crates-credential-receipt.json"
       test ! -e "$actual_receipt"
       test -z "${PI_CRATES_IO_RELEASE_TOKEN:-}"
       test -n "${release_crates_io_token:-}"
       set +e
       (
         set -euo pipefail
         publish_exact_crate_with_scoped_token "$actual_receipt"
       )
       cargo_status=$?
       set -e
       if test -e "$actual_receipt"; then
         test -f "$actual_receipt" && test ! -L "$actual_receipt"
         jq -e \
           --arg version "$RELEASE_VERSION" \
           --arg sha "$expected_crate_sha256" '
           .schema == "pi.release.cargo_credential_receipt.v1" and
           .name == "pi_agent_rust" and .version == $version and
           .crate_sha256 == $sha and .registry_name == "crates-io" and
           (.registry_index_url == "sparse+https://index.crates.io/" or
            .registry_index_url == "https://github.com/rust-lang/crates.io-index")
         ' "$actual_receipt" >/dev/null
         receipt_sha256="$(sha256sum "$actual_receipt" | awk '{print $1}')"
       else
         test "$cargo_status" -ne 0
       fi
       post_attempts=60
     fi

     # Cargo's exit status can be ambiguous. The authoritative registry read is
     # the authority. A retry always performs that read before it can expose the
     # token or issue another publish request.
     after_state="$attempt_dir/crates-after.json"
     record_exact_crates_state "$after_state" "$post_attempts"
     jq -e '.state == "exact"' "$after_state" >/dev/null
     (set -C; printf \
       'attempt_id=%s\ncargo_publish_exit=%s\ncredential_receipt_sha256=%s\nregistry_state=exact\n' \
       "$attempt_id" "$cargo_status" "$receipt_sha256" \
       > "$attempt_dir/crates-publication-reconciliation.txt")
   }

   CRATES_ATTEMPT_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
   export CRATES_ATTEMPT_ID
   [[ "$CRATES_ATTEMPT_ID" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
   crates_attempt_dir="$MANUAL_RELEASE_STATE_DIR/crates-$CRATES_ATTEMPT_ID"
   test ! -e "$crates_attempt_dir" && test ! -L "$crates_attempt_dir"
   mkdir -m 700 "$crates_attempt_dir"
   crates_reconcile_status=0
   set +e
   (
     set -euo pipefail
     reconcile_exact_crates_publication \
       "$CRATES_ATTEMPT_ID" "$crates_attempt_dir"
   )
   crates_reconcile_status=$?
   set -e
   if test "$crates_reconcile_status" -eq 0; then
     successful_crates_receipt="$crates_attempt_dir/crates-publication-reconciliation.txt"
     test -f "$successful_crates_receipt" \
       && test ! -L "$successful_crates_receipt" \
       && test -s "$successful_crates_receipt"
     test "$(grep -Fxc "attempt_id=$CRATES_ATTEMPT_ID" \
       "$successful_crates_receipt")" = 1
     test "$(grep -Fxc 'registry_state=exact' \
       "$successful_crates_receipt")" = 1
     unset release_crates_io_token
   else
     (set -C; printf \
       'attempt_id=%s\nreconciliation_exit=%s\nregistry_state=unresolved\n' \
       "$CRATES_ATTEMPT_ID" "$crates_reconcile_status" \
       > "$crates_attempt_dir/crates-publication-unresolved.txt")
     printf '%s\n' \
       'crates.io publication is unresolved; retained state and token for reconciliation' \
       >&2
   fi
   ```

如果 Cargo 或 crates.io 查询出现不明确的失败，请保留精确的发布者 crate、提供方、草稿发布和状态目录。不要从 Cargo 的退出码推断发布，也不要盲目地重新运行 `cargo publish`。前台子 shell 保持 fail-fast，而父进程捕获其状态，并在 reconciliation 未解决时有意保持 shell 存活且令牌保留在未导出的变量中。选择全新的 `CRATES_ATTEMPT_ID`/attempt 目录并再次运行相同的块。reconciler 在读取令牌之前会采纳已存在的精确未撤回（non-yanked）校验和；冲突的注册表身份或校验和是永久性停止条件。除非 `crates_reconcile_status=0` 且成功的 attempt 回执存在，否则不要继续到步骤 9。如果控制 shell 在任何不可变远端变更后终止，则停止此通道：保留的回执是诊断证据，而非独立的恢复引导，且不得通过从本文档复制孤立命令来重建发布。

9. 最后再将 GitHub 设为公开。在发布 reconciliation 之前，立即重新检查不可变标签规则、标签对象/peeled 目标、精确的发布 ID/状态/标题/正文/预发布、全部 12 个名称和字节、保留的运行时回执以及 crates.io 校验和。如果保留的发布仍为草稿，则按记录的数据库 ID 进行 PATCH；如果较早的 PATCH 响应已丢失但精确的发布已公开，则直接采纳而不再次发送 PATCH。两种情况下，均重复精确验证器。

   ```bash
   set -euo pipefail
   verify_operator_tools
   test "$crates_reconcile_status" -eq 0
   successful_crates_receipt="$crates_attempt_dir/crates-publication-reconciliation.txt"
   test -f "$successful_crates_receipt" && test ! -L "$successful_crates_receipt" \
     && test -s "$successful_crates_receipt"
   test "$(grep -Fxc 'registry_state=exact' "$successful_crates_receipt")" = 1

   reconcile_final_publication_attempt() (
     set -euo pipefail
     local attempt_id="$1"
     local attempt_dir="$2"
     local crates_receipt="$3"
     local prepublic_ruleset registry_checksum post_registry_checksum
     local github_receipt github_receipt_sha256
     [[ "$attempt_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
     test -d "$attempt_dir" && test ! -L "$attempt_dir"
     case "$attempt_dir" in
       "$MANUAL_RELEASE_STATE_DIR"/publication-"$attempt_id") ;;
       *) exit 2 ;;
     esac
     verify_operator_tools
     test -f "$crates_receipt" && test ! -L "$crates_receipt" \
       && test -s "$crates_receipt"
     test "$(grep -Fxc 'registry_state=exact' "$crates_receipt")" = 1
     sha256sum --check --strict \
       "$MANUAL_RELEASE_STATE_DIR/target-runtime-smokes.sha256"
     prepublic_ruleset="$attempt_dir/pre-public-ruleset.json"
     test ! -e "$prepublic_ruleset"
     gh api -H 'Accept: application/vnd.github+json' \
       "/repos/${RELEASE_REPOSITORY}/rulesets/${immutable_ruleset_id}?includes_parents=true" \
       > "$prepublic_ruleset"
     jq -e '
       .target == "tag" and .enforcement == "active" and
       ((.conditions.ref_name.include | index("refs/tags/v*")) != null or
        (.conditions.ref_name.include | index("~ALL")) != null) and
       .conditions.ref_name.exclude == [] and
       ([.rules[].type] | index("update")) != null and
       ([.rules[].type] | index("deletion")) != null and
       (.bypass_actors | type) == "array" and .bypass_actors == []
     ' "$prepublic_ruleset" >/dev/null

     registry_checksum="$(curl -fsS -A 'pi-agent-rust-manual-release' \
       "https://crates.io/api/v1/crates/pi_agent_rust/${RELEASE_VERSION}" \
       | jq -er --arg version "$RELEASE_VERSION" '
         select(.version.crate == "pi_agent_rust" and
                .version.num == $version and .version.yanked == false and
                (.version.checksum | test("^[0-9a-f]{64}$"))) |
         .version.checksum')"
     test "$registry_checksum" = "$expected_crate_sha256"
     reconcile_exact_github_publication "$attempt_id" "$attempt_dir"
     post_registry_checksum="$(curl -fsS -A 'pi-agent-rust-manual-release' \
       "https://crates.io/api/v1/crates/pi_agent_rust/${RELEASE_VERSION}" \
       | jq -er --arg version "$RELEASE_VERSION" '
         select(.version.crate == "pi_agent_rust" and
                .version.num == $version and .version.yanked == false and
                (.version.checksum | test("^[0-9a-f]{64}$"))) |
         .version.checksum')"
     test "$post_registry_checksum" = "$expected_crate_sha256"
     github_receipt="$attempt_dir/github-publication-reconciliation.txt"
     test -f "$github_receipt" && test ! -L "$github_receipt" \
       && test -s "$github_receipt"
     test "$(grep -Fxc "attempt_id=$attempt_id" "$github_receipt")" = 1
     github_receipt_sha256="$(sha256sum "$github_receipt" | awk '{print $1}')"
     (set -C; printf \
       'attempt_id=%s\ngithub_receipt_sha256=%s\nregistry_checksum=%s\nstate=exact\n' \
       "$attempt_id" "$github_receipt_sha256" "$post_registry_checksum" \
       > "$attempt_dir/publication-attempt-success.txt")
   )

   PUBLICATION_ATTEMPT_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
   export PUBLICATION_ATTEMPT_ID
   [[ "$PUBLICATION_ATTEMPT_ID" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]
   publication_attempt_dir="$MANUAL_RELEASE_STATE_DIR/publication-$PUBLICATION_ATTEMPT_ID"
   test ! -e "$publication_attempt_dir" && test ! -L "$publication_attempt_dir"
   mkdir -m 700 "$publication_attempt_dir"
   publication_reconcile_status=0
   set +e
   (
     set -euo pipefail
     reconcile_final_publication_attempt \
       "$PUBLICATION_ATTEMPT_ID" "$publication_attempt_dir" \
       "$successful_crates_receipt"
   )
   publication_reconcile_status=$?
   set -e
   if test "$publication_reconcile_status" -eq 0; then
     publication_success_receipt="$publication_attempt_dir/publication-attempt-success.txt"
     test -f "$publication_success_receipt" \
       && test ! -L "$publication_success_receipt" \
       && test -s "$publication_success_receipt"
     test "$(grep -Fxc "attempt_id=$PUBLICATION_ATTEMPT_ID" \
       "$publication_success_receipt")" = 1
     test "$(grep -Fxc 'state=exact' "$publication_success_receipt")" = 1
   else
     (set -C; printf \
       'attempt_id=%s\nreconciliation_exit=%s\nstate=unresolved\n' \
       "$PUBLICATION_ATTEMPT_ID" "$publication_reconcile_status" \
       > "$publication_attempt_dir/publication-attempt-unresolved.txt")
     printf '%s\n' \
       'GitHub publication is unresolved; retained all attempt state' >&2
   fi
   ```

所有提供方代码、其冻结工作流源、哈希、自测回执、发布回执、发布元数据快照和已下载资产都保留在 `MANUAL_RELEASE_STATE_DIR` 下。手动通道无法使 crates.io 查询与 GitHub PATCH 原子化，因此不可变的服务端标签规则是硬性前置条件。任何缺失字段、不可读的 bypass 列表、变更的哈希、重复/额外资产、元数据漂移、不匹配的字节或意外的公开状态均为停止条件。

如果 PATCH 或其验证查询出现不明确的失败，请保留相同的发布身份回执、正文、制品、标签、crates.io 证明和状态目录。选择全新的 `PUBLICATION_ATTEMPT_ID`/attempt 目录并仅重新运行此步骤。前台子 shell 保持 fail-fast，而其父进程捕获状态并保留未解决的回执。reconciler 首先验证当前状态，仅对精确保留的草稿发送 PATCH，并且仅在完整元数据、远端标签、清单和字节检查通过后才采纳已公开的发布。它从不删除、替换或移动远端状态。

10. 从隔离的 home 验证现已公开的安装器路径，并确认 crates.io 仍提供精确的未撤回版本/校验和。这是安装器和发布验证，而非首次二进制 smoke：全部五个精确二进制已在步骤 7 中执行，其保留的回执哈希仍必须通过。此手动通道既不从 GitHub Actions 读取也不向其写入。

    ```bash
    set -euo pipefail
    verify_operator_tools
    test "$publication_reconcile_status" -eq 0
    publication_success_receipt="$publication_attempt_dir/publication-attempt-success.txt"
    test -f "$publication_success_receipt" \
      && test ! -L "$publication_success_receipt" \
      && test -s "$publication_success_receipt"
    test "$(grep -Fxc "attempt_id=$PUBLICATION_ATTEMPT_ID" \
      "$publication_success_receipt")" = 1
    test "$(grep -Fxc 'state=exact' "$publication_success_receipt")" = 1
    sha256sum --check --strict "$MANUAL_RELEASE_STATE_DIR/target-runtime-smokes.sha256"

    public_download_dir="$MANUAL_RELEASE_STATE_DIR/github-assets-after-public-${PUBLICATION_ATTEMPT_ID}"
    public_installer="$public_download_dir/install.sh"
    test -f "$public_installer" && test ! -L "$public_installer" \
      && test -s "$public_installer"
    cmp "$RELEASE_ARTIFACT_DIR/install.sh" "$public_installer"
    installer_root="$MANUAL_RELEASE_STATE_DIR/post-public-installer-linux-amd64"
    installer_receipt="$MANUAL_RELEASE_STATE_DIR/post-public-installer-linux-amd64.txt"
    test ! -e "$installer_root" && test ! -L "$installer_root"
    test ! -e "$installer_receipt"
    mkdir -m 700 \
      "$installer_root" "$installer_root/home" "$installer_root/state" \
      "$installer_root/bin" "$installer_root/tmp"
    installer_lock="$installer_root/install.lock.d"
    test ! -e "$installer_lock" && test ! -L "$installer_lock"
    (set -C; \
      HOME="$installer_root/home" \
      XDG_STATE_HOME="$installer_root/state" \
      TMPDIR="$installer_root/tmp" \
      PI_INSTALLER_RETAIN_TEMP=1 \
      PI_INSTALLER_LOCK_DIR="$installer_lock" \
      AGENT_SKILLS_ENABLED=0 \
      bash "$public_installer" \
        --yes --version "$RELEASE_TAG" --dest "$installer_root/bin" \
        --verify --no-gum --no-completions --no-agent-skills \
        > "$installer_receipt" 2>&1)
    test -d "$installer_lock" && test ! -L "$installer_lock"
    test -f "$installer_lock/pid" && test ! -L "$installer_lock/pid"
    test "$(find "$installer_root/tmp" -mindepth 1 -maxdepth 1 -type d |
      wc -l | tr -d '[:space:]')" -ge 1
    grep -F 'Retaining installer temporary directory:' \
      "$installer_receipt" >/dev/null
    grep -F "Retaining installer lock directory: $installer_lock" \
      "$installer_receipt" >/dev/null
    installer_state="$installer_root/state/pi-agent-rust/install-state.env"
    test -f "$installer_state" && test ! -L "$installer_state"
    (
      set -euo pipefail
      # This state file was produced by the exact downloaded installer whose
      # bytes were compared above; source it only inside the isolated subshell.
      # shellcheck disable=SC1090
      source "$installer_state"
      test "$PIAR_INSTALL_VERSION" = "$RELEASE_TAG"
      test "$PIAR_INSTALL_SOURCE" = release
      case "$PIAR_CHECKSUM_STATUS" in "verified (SHA256SUMS)") ;; *) exit 1 ;; esac
      test -f "$PIAR_INSTALL_BIN" && test ! -L "$PIAR_INSTALL_BIN"
      installed_sha="$(sha256sum "$PIAR_INSTALL_BIN" | awk '{print $1}')"
      linux_release_sha="$(jq -er '
        first(.artifacts[] | select(.name == "pi_linux_amd64") | .sha256) |
        select(test("^[0-9a-f]{64}$"))
      ' "$raw_manifest")"
      test "$installed_sha" = "$linux_release_sha"
      installed_version="$("$PIAR_INSTALL_BIN" --version)"
      case "$installed_version" in "pi $RELEASE_VERSION ("*) ;; *) exit 1 ;; esac
      printf 'post_public_installer_status=success\nsha256=%s\nversion=%s\n' \
        "$installed_sha" "$installed_version"
    ) >> "$installer_receipt"
    grep -Fx 'post_public_installer_status=success' "$installer_receipt" >/dev/null

    curl -fsS -A 'pi-agent-rust-manual-release' \
      "https://crates.io/api/v1/crates/pi_agent_rust/${RELEASE_VERSION}" \
      | jq -e \
        --arg version "$RELEASE_VERSION" \
        --arg checksum "$expected_crate_sha256" '
        .version.crate == "pi_agent_rust" and
        .version.num == $version and .version.yanked == false and
        .version.checksum == $checksum
      ' >/dev/null
    sha256sum "$installer_receipt" "$installer_state" \
      > "$MANUAL_RELEASE_STATE_DIR/post-public-installer.sha256"
    ```

## Pre-release flow (rc) / 预发布流程 (rc)

使用带注释的预发布标签来演练已配置的自动化发布通道，而无需发布到 crates.io：
- `git tag -a vX.Y.Z-rc.1 -m "vX.Y.Z-rc.1 release" && git push origin vX.Y.Z-rc.1`

`release.yml` 在其治理和制品门禁通过后跳过 crates.io 并仅发布 GitHub 预发布。`publish.yml` 不在标签推送时运行；它是可选的手动 dry-run 诊断。对于无 Actions 的 DSR 通道，请保持打标签提交的消息标记为 `[skip actions]` 且不要调度任一工作流。

## Merge-Gate DoD Policy / 合并门禁 DoD 策略

功能层面的拉取请求在合并前必须满足 Definition-of-Done 证据清单：
- 单元证据链接
- 端到端(e2e)证据链接
- 扩展证据链接
- 用于通过/失败验证路径的复现命令

CI 通过 `.github/workflows/ci.yml` 并使用 `.github/pull_request_template.md` 作为规范清单格式来强制执行。

### Migration Guidance for Existing Feature Branches / 现有功能分支的迁移指南

对于在此门禁引入之前打开的分支：
1. 变基到最新的 `main`。
2. 将 PR 正文替换为 `.github/pull_request_template.md`。
3. 回填指向当前证据制品的链接。
4. 包含用于验证最新失败路径修复的精确重跑命令。
5. 重新运行 CI，且仅在 DoD 证据守卫通过后合并。

## Pre-release checklist / 预发布清单

- 所选发布通道拥有各自完整的证明：
  - 自动化通道：CI 在 `main` 上为绿（Linux/macOS/Windows），且受保护的自动化发布治理门禁已满足
  - 手动/无 Actions 通道：上述每个 fail-fast 手动门禁均为绿，且未查询、调度、重跑、取消任何工作流或将其用作证据
- 本地门禁均为绿：
  - `cargo fmt --check`
  - `cargo check --locked --all-targets --features internal-legacy-capture`
  - `cargo clippy --locked --all-targets --features internal-legacy-capture -- -D warnings`
  - `cargo test --locked --all-targets --features internal-legacy-capture`
- 自上一个标签以来合并的功能 PR 满足 DoD 证据清单（单元 + 端到端 + 扩展 + 复现命令）。
- `CHANGELOG.md` 已针对你要打标签的版本更新。
- 如果此发布对性能敏感，已运行基准测试（参见[基准指南](planning/BENCHMARKS.md)）。
- 分发兼容性矩阵（上文）对所有必需路径均通过。

## Post-release checklist / 发布后清单

- GitHub Release 已存在并包含各平台的预期制品。
- `SHA256SUMS` 与下载的制品匹配。
- Crates.io 发布成功（如已配置）且版本与标签匹配。
- Smoke 测试安装路径（下载二进制并运行 `pi --version`）。
