# 热门 Pi 扩展 — 评分与入选标准 (bd-29ko)

本文档定义 **“热门（popular）”** 对 Pi 扩展的含义，以及如何判定某个扩展是否应纳入**扩充兼容性语料库（expanded compatibility corpus）**（见 `bd-po15` / `bd-d7gn`）。

目标：让筛选过程**机械化、可复现**——两个人对相同输入应用本评分细则，应收敛到相同的分级结果，同时显式纳入**兼容性**与**可靠性风险**的考量。

非目标：
- 本文**不是**分层一致性抽样（即已固定的 16 个抽样，见 `docs/extension-sample.json`）。
- 本文不规定*如何*抓取数据；本文规定清单（inventory）必须产出的**字段与评分逻辑**。

---

## 0) 定义

- **候选（Candidate）**：一个扩展制品（单文件、目录或包）及其溯源元数据。
- **语料库（Corpus）**：我们旨在**无需修改**即可支持的分级集合。
  - **Tier-0**：官方基线（pi-mono 示例）；始终纳入覆盖范围。
  - **Tier-1**：必过（must-pass），在 CI 中始终运行（**目标规模：>= 200**）。
  - **Tier-2**：扩展覆盖；在 CI 中按计划或按需任务运行的额外长尾覆盖。
- **信号（Signals）**：客观度量，如 stars/downloads/官方收录等。
- **覆盖度（Coverage）**：该扩展为证明新增了多少*新的*扩展表层覆盖（运行时层级、交互模型、宿主调用组合）。

---

## 1) 硬性门禁（进入 Tier-1/Tier-2 的必过项）

若候选未通过任一门禁，则为**已排除（Excluded）**（或仅作为“仅元数据”保留用于研究）。

### 1.1 溯源可固定（Provenance is pin-able）

候选必须具有稳定、可复现的引用：

- Git 仓库：commit SHA（优先）或不可变 tag + 仓库 URL
- Gist：gist 修订版 SHA / commit SHA + URL
- npm：包名 + 精确版本

不允许浮动引用（例如 “main”、“latest”、未固定的 URL）。

### 1.2 许可证 / 再分发已知

我们必须能够回答：“能否将其收录到 `tests/ext_conformance/artifacts/**` 中？”

允许的结果：

- **OK**：MIT/Apache-2.0/BSD 等或已获明确授权
- **Restricted**：可再分发但有约束（需记录）
- **Exclude**：未知 / 专有 / 不清晰

Tier-1/Tier-2 要求为 **OK** 或 **Restricted** 且有具体方案。

### 1.3 未修改兼容性（规范性）

Tier-1/Tier-2 扩展必须**无需手动修改源码**即可运行。

允许：

- 确定性编译/转译（TS → JS）
- 确定性打包（多文件 → 单制品）
- 确定性导入/引用重写（例如 Node 内置模块 → `pi:node/*`）
- 运行时中由 Pi 提供的垫片/连接器（非针对单个扩展的 hack）

不允许：

- “仅为 Pi”而编辑扩展逻辑
- 运行时中针对单个扩展的特例（“if extension_id == X …”）
- 以不可审计的方式对产出包进行事后补丁

完整契约见 `docs/ext-compat.md`（“Unmodified compatibility”）。

### 1.4 确定性与可复现性

Tier-1/Tier-2 候选必须至少有一个可确定性运行的场景：

- 无真实 OAuth 登录、无真实 API 密钥、无不稳定的网络依赖
- 若扩展重度依赖网络，则必须具备离线/错误路径场景或可通过 VCR 录制回放

---

## 2) 评分细则（基础分 0–100 + 风险扣分）

我们计算**基础分（0–100）**，再减去**可靠性风险扣分（0–15）**。

**基础分 = 热门度 + 采用度 + 覆盖度 + 活跃度 + 兼容性。**  
**最终得分 = 基础分 – 风险扣分（下限为 0）。**

| Sub-score | Weight | What it measures |
|---|---:|---|
| Popularity | 30 | “How broadly visible is it?” |
| Adoption | 15 | “Do real users install/use it?” |
| Coverage | 20 | “How much new surface does it cover for our proof?” |
| Activity | 15 | “Is it maintained enough to matter today?” |
| Compatibility | 20 | “How close is it to unmodified compatibility?” |

清单应存储用于计算得分的**全部输入**及简短理由说明。

### 2.1 热门度 Popularity (0–30)

将以下分项相加（上限 30）。缺失的指标在源数据中保持显式的 `null`，计分时按 **0** 计算，并记录在 `missingSignals` 中，以使未知数据可审计，**不会**静默伪装为已知的零值。

1) **官方 / 第一方可见性 (0–10)**（取**最大值**，不累加）
- 已收录于 `buildwithpi.ai/packages`（或官方文档）：+10
- 作为 pi-mono 示例扩展发布：+8
- 由 badlogic 编写的、被官方文档引用的 gist：+6

2) **GitHub stars (0–10)**（仓库或 gist 镜像，若适用）
- ≥ 5,000：+10
- ≥ 2,000：+9
- ≥ 1,000：+8
- ≥ 500：+6
- ≥ 200：+4
- ≥ 50：+2
- 其他：+0

3) **市场可见性 (0–6)**（OpenClaw / ClawHub）
- 排名 ≤ 10：+6
- 排名 ≤ 50：+4
- 排名 ≤ 100：+2
- 其他：+0  
**精选徽章（Featured badge）：** +2（市场可见性总分上限为 6）

4) **社区引用 (0–4)**（统计不同来源数量）
- ≥ 10 个不同引用：+4
- ≥ 5：+3
- ≥ 2：+2
- 其他：+0

“引用”示例：其他仓库的链接、博客文章、精选清单、带链接的 Discord 片段。（清单必须收录 URL。）

### 2.2 采用度 Adoption (0–15)

针对来源类型选择最合适的可用信号；存储原始数值。缺失的指标在源数据中保持 `null`，计分时按 **0** 计算，并记录在 `missingSignals` 中。

1) **npm 下载量 (0–8)**（若已发布到 npm）
- ≥ 50k / 月：+8
- ≥ 10k / 月：+6
- ≥ 2k / 月：+4
- ≥ 500 / 月：+2
- 其他：+0

2) **市场安装量 (0–5)**（OpenClaw / ClawHub）
- ≥ 10k / 月：+5
- ≥ 2k / 月：+4
- ≥ 500 / 月：+2
- ≥ 100 / 月：+1
- 其他：+0

3) **Fork / 衍生使用 (0–2)**（GitHub）
- forks ≥ 500：+2
- ≥ 200：+1
- ≥ 50：+1
- 其他：+0

### 2.3 覆盖度 Coverage (0–20)

覆盖度旨在最大化证明价值，而非“热门度”。按标签计分：

1) **运行时层级 (0–6)**
- pkg-with-deps **或** provider-ext：+6
- multi-file：+4
- legacy-js（单文件）：+2

2) **交互模型广度 (0–8)**
- provider：+3
- ui_integration：+2
- event_hook：+2
- slash_command：+1
- tool_only：+1

（上限 8；标签定义见 `docs/EXTENSION_SAMPLING_MATRIX.md`。）

3) **宿主调用组合 (0–6)**（使用的能力）
- 使用 `exec`：+2
- 使用 `http`：+2
- 使用 `read`/`write`/`edit`：+1
- 使用 `ui`：+1
- 使用会话变更 API：+1

### 2.4 活跃度 / 新近度 Activity / Recency (0–15)

针对来源类型使用最相关的日期（仓库最后提交、npm 发布、gist 更新）。

- 更新于 ≤ 30 天内：+15
- ≤ 90 天：+12
- ≤ 180 天：+9
- ≤ 365 天：+6
- ≤ 730 天：+3
- 其他：+0

### 2.5 兼容性 Compatibility (0–20)

兼容性是**正向得分**（而不仅是门禁），以便在筛选时优先选择已接近未修改一致性的扩展。

建议评分（选择最匹配的档位，再根据细微差别 ±2 调整）：

- **20** — 未修改，通过静态扫描，无禁用 API，无需特定于扩展的垫片。
- **15** — 未修改，但需要**通用**垫片/重写（Node 核心、`pi:*` 垫片）。
- **10** — 未修改但依赖**尚未完整的通用运行时能力**（例如提供方钩子尚未完全接线）；仍可通过运行时工作落地。
- **0** — 需要针对单个扩展的编辑或未通过兼容性门禁（已阻断）。

### 2.6 可靠性风险扣分 Reliability Risk Penalty (0–15)

风险为**扣分项**（从基础分中减去）。它刻画“该扩展在 CI 中出现不稳定、非确定性或高支持成本的可能性”。

建议扣分区间：

- **0** — 确定性、依赖极少、无网络或完全可通过 VCR 录制回放。
- **5** — 依赖适中或有网络使用，但可通过 mock/VCR 复现。
- **10** — 高风险：OAuth 流程、重度 UI 时序敏感、依赖树庞大。
- **15** — 关键风险：原生二进制、非确定性副作用、许可证不清晰。

### 2.7 计算示例（市场信号）

假设 `as_of = 2026-02-01` 用于活跃度计分。

**示例 A — “OpenClaw 精选工具”（pkg-with-deps）**
- 信号：GitHub stars **1,200**，市场排名 **8**，精选 **true**，引用 **6**
- 热门度 = 0（官方）+ 8（stars）+ 6（市场）+ 3（引用） = **17**
- 采用度 = 6（npm 12k/月）+ 5（市场安装量 15k/月）+ 1（forks 220） = **12**
- 覆盖度 = 6（运行时 pkg-with-deps）+ 5（tool + event + UI）+ 6（exec+http+fs+ui） = **17**
- 活跃度 = **15**（更新于 2026-01-15）
- 兼容性 = **15**（需要通用垫片）
- 基础分 = 17 + 12 + 17 + 15 + 15 = **76**
- 风险扣分 = **5**（中等，网络密集）
- **最终得分 = 71 → Tier-1**

**示例 B — “小众 GitHub 脚本”（legacy-js）**
- 信号：GitHub stars **120**，引用 **1**，无市场数据
- 热门度 = 0（官方）+ 2（stars）+ 0（市场）+ 0（引用） = **2**
- 采用度 = **0**（无 npm/市场安装量，forks 12）
- 覆盖度 = 2（运行时 legacy-js）+ 1（tool_only）+ 1（fs） = **4**
- 活跃度 = **0**（更新于 2023-01-01）
- 兼容性 = **20**（干净的未修改）
- 基础分 = 2 + 0 + 4 + 0 + 20 = **26**
- 风险扣分 = **0**
- **最终得分 = 26 → 已排除**

**示例 C — “官方 pi-mono 示例”**
- 信号：pi-mono 示例 **true**，GitHub stars **7,000**，引用 **12**
- 热门度 = 8（官方）+ 10（stars）+ 0（市场）+ 4（引用） = **22**
- 采用度 = **2**（forks 720）
- 覆盖度 = 2（运行时 legacy-js）+ 4（event + UI）+ 4（exec+ui+session） = **10**
- 活跃度 = **15**（更新于 2026-01-20）
- 兼容性 = **20**（干净的未修改）
- 基础分 = 22 + 2 + 10 + 15 + 20 = **69**
- 风险扣分 = **0**
- **最终得分 = 69 → 按分数为 Tier-2，但因官方身份归为 Tier-0 基线**

---

## 3) 分级规则

分级同时依据门禁与分数阈值：

| Tier | Requirements |
|---|---|
| Tier-1 | Pass all gates + **final score** ≥ 70 |
| Tier-2 | Pass all gates + **final score** ≥ 50 |
| Excluded | Fails a gate OR final score < 50 |

平分时的优先规则（当选择固定大小集合时）：

1) 优先选择 **覆盖度**得分更高的（证明价值更高）
2) 其次选择 **热门度**更高的
3) 再按更新时间更新者优先

### 3.1 语料库规模策略（规范性）

- **Tier-0 基线**：保留全部官方 pi-mono 扩展在覆盖范围内。
- **Tier-1 必过语料库**：**>= 200** 个扩展。
- **Tier-2 扩展语料库**：在 Tier-1 选定之后，所有其他符合条件的扩展。

若因门禁失败（许可证、确定性场景、固定溯源、未修改兼容性）导致 Tier-1 低于 200，应输出机器可读的缺口报告，包含：`required`、`selected`、`missing` 以及按门禁分类的原因统计。

### 3.2 可执行评分契约

规范性实现：

- `examples/ext_popularity_snapshot.rs`
- `src/extension_scoring.rs`
- `examples/ext_score_candidates.rs`

热门度快照运行示例：

```bash
cargo run --example ext_popularity_snapshot -- \
  --input docs/extension-candidate-pool.json \
  --out docs/extension-candidate-pool.json \
  --log-jsonl tests/e2e_results/ext-popularity-snapshot.jsonl
```

说明：

- GitHub 指标在存在 `GITHUB_TOKEN` 时使用该令牌，否则若可用则回退到 `gh auth token` 获取的 GitHub CLI 认证。
- 若两种令牌来源均不可用，则跳过 GitHub 查询，仍会刷新 npm 信号。
- 使用 `--dry-run` 可在不写入文件的情况下预览汇总统计。

运行示例：

```bash
cargo run --example ext_score_candidates -- \
  --input docs/extension-candidate-pool.json \
  --out docs/extension-priority.json \
  --summary-out docs/extension-priority-summary.json \
  --as-of 2026-02-06T00:00:00Z \
  --generated-at 2026-02-06T00:00:00Z
```

预期报告模式：`pi.ext.scoring.v1`，包含确定性排序以及显式的 `gates`、`missingSignals` 和按准则细分的得分明细。

---

## 4) 清单字段（面向 bd-1o8j / bd-hhzv / bd-34io 的模式指引）

候选清单应可表示为具有以下字段的 JSON 对象：

```json
{
  "id": "stable-id",
  "name": "display name",
  "source": {
    "kind": "repo|gist|npm|pi-mono|buildwithpi",
    "url": "https://…",
    "repo": "owner/name",
    "commit": "sha-or-tag",
    "path": "path/inside/repo",
    "npm": { "name": "pkg", "version": "1.2.3" }
  },
  "license": { "spdx": "MIT", "redistribution": "ok|restricted|exclude", "notes": "" },
  "tags": {
    "runtime": "legacy-js|multi-file|pkg-with-deps|provider-ext",
    "interaction": ["tool_only","slash_command","event_hook","ui_integration","provider"],
    "capabilities": ["read","write","edit","exec","http","ui","session"]
  },
  "signals": {
    "official_listing": false,
    "pi_mono_example": false,
    "badlogic_gist": false,
    "github_stars": 0,
    "github_forks": 0,
    "npm_downloads_month": 0,
    "references": ["https://…"],
    "marketplace": {
      "rank": 0,
      "installs_month": 0,
      "featured": false
    }
  },
  "recency": { "updated_at": "2026-01-31T00:00:00Z" },
  "compat": {
    "status": "unmodified|requires_shims|runtime_gap|blocked",
    "unmodified_required": true,
    "blocked_reasons": [],
    "required_shims": ["pi:node/fs", "pi:node/path"]
  },
  "gates": {
    "provenance_pinned": true,
    "deterministic": true
  },
  "score": {
    "popularity": 0,
    "adoption": 0,
    "coverage": 0,
    "activity": 0,
    "compatibility": 0,
    "risk_penalty": 0,
    "base_total": 0,
    "final_total": 0,
    "tier": "tier-0|tier-1|tier-2|excluded",
    "rationale": "1-3 sentences explaining the score.",
    "risk_notes": "Optional: why the risk penalty was applied."
  }
}
```

说明：

- `required_shims` 为描述性字段（扩展看似需要的垫片），而非针对单个扩展的 hack 清单。
- `blocked_reasons` 必须客观且可操作（例如“许可证未知”、“需要 Node C++  addon”）。
