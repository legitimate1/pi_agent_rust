# 翻译术语表 / Translation Glossary

> 人与 Agent 共用。翻译前必读此表，遇到表中词必须用定译，保留英文类原样输出。
> 格式：无序列表，`英文 → 中文（类别）`，双语锚点需保留英文原文。

---

## 使用规则

- 保留英文：原文照抄，不译不改大小写（`Beads`/`VCR`/`JSON` 等）
- 固定译：全篇统一用此中文，不可换同义词（`provider → 提供方` 就不能写成 `提供商`）
- 双语锚点：英文锚点行保留原文，中文写在同行括号或紧邻下一行（供门禁 `contains`/`starts_with` 匹配）

---

## 1. 专有与保留英文（不译）

- `Beads` → `Beads`（保留英文，专有 issue/任务系统，首字母大写）
- `bead id` / `bd-xxxx` → `bead id` / `bd-xxxx`（保留英文，珠/任务编号）
- `VCR` → `VCR`（保留英文，录制回放框架）
- `VCR cassette` → `VCR cassette`（保留英文，或写作 `VCR 磁带(cassette)`，`VCR` 必须保留）
- `JSON` / `JSONL` / `TOML` / `YAML` / `Markdown` → 保留英文
- `schema: pi.*.v1` → 保留英文（如 `pi.swarm.progress_slo.closeout_gate.v1`）
- `Cargo` / `cargo test` / `clippy` / `rustfmt` → 保留英文（工具链命令）
- `npm` / `GitHub` / `OpenAI` → 保留英文（品牌/生态名）
- `QuickJS` / `rquickjs` / `asupersync` / `rich_rust` → 保留英文（库名）
- `TUI` / `RPC` / `CLI` / `API` → 保留英文（缩写）
- `F_OK` / `R_OK` / `W_OK` → 保留英文（常量）

---

## 2. 运行时与架构（固定译）

- `provider` → `提供方`（固定译，不用 `提供商/供应商/提供者`）
- `native provider` → `原生提供方`
- `provider preset` / `OpenAI-compatible preset` → `提供方预设` / `OpenAI 兼容预设`
- `extension` → `扩展`（固定译，不用 `插件/拓展`）
- `extension root` → `扩展根`（固定时 `extension root` 路径锚点保留英文，见双语锚点）
- `capability` → `能力`（如 `capability policy` → `能力策略`）
- `hostcall` → `宿主调用`（`extension → host` 的特权调用）
- `tool` → `工具`（如 `ToolRegistry` → `工具注册表`）
- `session` → `会话`
- `skill` → `技能`（`~/.pi/agent/skills/` 下的 `SKILL.md`）
- `prompt template` → `提示模板`
- `theme` → `主题`
- `agent` → `智能体`（项目语境下的 coding agent；泛指时可 `Agent`）
- `swarm` → `集群`（多智能体协作语境）
- `model` → `模型`
- `streaming` → `流式`
- `thinking level` → `思考级别`
- `auth` / `API key` → `认证` / `API 密钥`

---

## 3. 契约与门禁（固定译）

- `contract` → `契约`（`docs/contracts/*.json`）
- `evidence` → `证据`（`docs/evidence/*.json`）
- `traceability matrix` → `追溯矩阵`（`docs/traceability_matrix.json`）
- `closeout gate` → `收口闸门`
- `quality gate` → `质量门`
- `conformance` → `一致性`
- `source boundary` → `源边界`
- `claim boundary` → `声明边界`
- `stale` → `陈旧`（如 `stale matrix entries` → `陈旧的矩阵条目`）
- `waiver` → `豁免`
- `audit` → `审计`
- `provenance` → `溯源`（`extension provenance` → `扩展溯源`）
- `catalog` / `master catalog` → `目录` / `主目录`
- `checksum` → `校验和`
- `suite classification` → `套件分类`（`tests/suite_classification.toml`）
- `coverage matrix` → `覆盖率矩阵`（`docs/TEST_COVERAGE_MATRIX.md`）

---

## 4. 测试与验证（固定译）

- `VCR cassette` → `VCR 磁带`（`VCR` 保留，`cassette` 可译 `磁带`）
- `golden corpus` / `golden cassette` → `黄金语料` / `黄金磁带`
- `fixture` → `夹具`
- `e2e` → `端到端`（首次出现可 `端到端(e2e)`）
- `unit test` → `单元测试`
- `backwards compatibility lock` / `provider lock` → `向后兼容锁` / `提供方锁`
- `no-mock` → `无模拟`
- `allowlist` → `允许清单`
- `flake triage` → `不稳定用例分流`

---

## 5. 双语锚点（必须保留英文原文，中文写在同行括号或紧邻行）

> 这些行被 `tests/*_contract.rs` 用 `contains`/`starts_with` 精确匹配，全翻中文即红。

- `Provider-count rule: Pi has 12 native provider implementation modules, counted as the Rust files under \`src/providers/\` excluding \`mod.rs\`: ...` → 保留英文整行，中文写在同行括号或下一行（`README.md`、`docs/providers.md:15`）
- `native provider implementation modules` → 锚点短语保留英文（`traceability_staleness.rs:284` 匹配 `contains`）
- `docs/contracts/validation-broker-closeout-gate-contract.json` → 路径字符串保留英文
- `docs/evidence/validation-broker-closeout-gate.json` → 路径字符串保留英文
- `docs/contracts/swarm-progress-slo-closeout-gate-contract.json` / `docs/evidence/swarm-progress-slo-closeout-gate.json` → 路径字符串保留英文
- `docs/swarm-operations-runbook.md#validation-broker-operator-workflow` → 路径+锚点保留英文
- `### Validation Broker Operator Workflow` → 标题保留英文
- `docs/contracts/dropin-certification-contract.json` / `docs/evidence/dropin-certification-verdict.json` → 路径保留英文
- `docs/traceability_matrix.json` / `docs/TEST_COVERAGE_MATRIX.md` → 路径保留英文
- `docs/providers.md` / `docs/qa-runbook.md` / `docs/testing-policy.md` / `docs/ci-operator-runbook.md` → 路径保留英文（见清单 GATE 组）
- `| \`src/foo.rs\` |` 表格行 → 管道与反引号路径保留，中文写在同行 description 列

---

## 6. 常见易混辨析

- `provider`（提供方，模型接入） vs `extension provider`（扩展提供的提供方，需区分）
- `Beads`（专有系统，不译） vs `bead`（普通英文 `珠`，但本项目一律保留 `Beads`）
- `closeout gate`（收口闸门） vs `gate`（门禁，泛指 CI gate）
- `traceability`（追溯，需求→测试的追踪） vs `provenance`（溯源，制品来源的追踪）
- `cassette`（磁带，VCR 语境） vs `fixture`（夹具，通用测试数据）
- `capability`（能力，策略维度） vs `permission`（权限，不用）
- `hostcall`（宿主调用，扩展调宿主） vs `tool call`（工具调用，模型调工具）

---

## 7. 维护约定

- 新增术语先查此表，未收录的按 5 类归类后追加，保持无序列表格式
- 改固定译需全量 `grep` 旧译并替换，避免一篇多译
- 双语锚点改动前先跑 `cargo test --test traceability_staleness -- --nocapture` 与对应 `*_contract` 测试验证
