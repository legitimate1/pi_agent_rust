## 未经过滤的原始扩展候选清单

这是一份**以源优先、未经过滤**的候选清单，用于扩展采样。它优先保证**功能覆盖的广度**而非流行度，并将由下游采样 Beads 进一步细化。

### 来源（候选来源）

1. **pi‑mono 示例扩展**（本地仓库快照；上游提交快照供参考）  
   `legacy_pi_mono_code/pi-mono/packages/coding-agent/examples/extensions/README.md`  
   Upstream snapshot: https://upd.dev/badlogic/pi-mono/src/commit/c6fc084534d0091e6243bdcf929249e48c36c9e9/packages/coding-agent/examples/extensions/README.md  
   Repo: https://github.com/badlogic/pi-mono  

2. **pi‑mono 本地 `.pi/extensions`**（仓库中的种子扩展）  
   `legacy_pi_mono_code/pi-mono/.pi/extensions/`  

3. **Pi 官方站点**（文档与软件包）  
   https://buildwithpi.ai/  
   https://buildwithpi.ai/packages  

4. **badlogic GitHub gists（扩展）**  
   https://gist.github.com/badlogic  
   https://gist.github.com/badlogic/679b221a1749353a5be3f3134c120685  
   https://gist.github.com/badlogic/30aef35d686483ffce22cc2aad99f3ff  
   https://gist.github.com/badlogic/587bcbc5d1d2b4d1cf30a1d0756275b9  
   https://gist.github.com/badlogic/8273f2bff572272e1036887e0744c3c8  

5. **社区 GitHub gists**  
   https://gist.github.com/nicobailon/ee8a65353b9103ad5d149e7eeb452b10  
   https://gist.github.com/aadishv/7615082df075519d6efd9de793aa860a  

6. **集成了 Pi 扩展的社区 npm 软件包**  
   https://www.npmjs.com/package/agentsbox  

7. **Claude Code 插件目录 / 市场（精选清单）**  
   https://www.claudedirectory.org/  
   https://www.claudeindex.com/

> 注：此处未枚举 npm “pi-package” 关键词结果及 buildwithpi 软件包列表；来源清单提供了搜索位置。

---

## npm Registry 扫描（bd-2p71）— 增量轮次（2026-02-06）

已执行的 npm registry 搜索查询（每条 `size=100`）：
- `buildwithpi`
- `pi-coding-agent`
- `"pi agent" extension`
- `pi extensionapi`
- `@mariozechner/pi-coding-agent`
- `pi-extension`
- `pi-agent-extension`
- `@oh-my-pi`

轮次概要：
- 8 次查询共捕获 700 条原始搜索记录。
- 观察到 341 个唯一软件包名称。
- 过滤后识别出 37 个高信号的 Pi 作用域/软件包名称候选。
- 本轮次向 `docs/extension-candidate-pool.json` 新增 9 个 npm 候选。

新增的 npm 候选：

| Package | Latest | Weekly Downloads | Monthly Downloads | Repository |
|---|---:|---:|---:|---|
| `@oh-my-pi/anthropic-websearch` | `1.3.3710` | 15 | 276 | https://github.com/can1357/oh-my-pi.git |
| `@oh-my-pi/basics` | `1.3.3710` | 5 | 54 | https://github.com/can1357/oh-my-pi.git |
| `@oh-my-pi/exa` | `1.3.3710` | 19 | 273 | https://github.com/can1357/oh-my-pi.git |
| `@oh-my-pi/lsp` | `1.3.3710` | 11 | 136 | https://github.com/can1357/oh-my-pi.git |
| `@oh-my-pi/pi-git-tool` | `6.8.5` | 165 | 7557 | https://github.com/can1357/oh-my-pi.git |
| `@oh-my-pi/subagents` | `1.3.3710` | 12 | 195 | https://github.com/can1357/oh-my-pi.git |
| `@tmustier/pi-arcade` | `0.1.5` | 141 | 578 | https://github.com/tmustier/pi-extensions.git |
| `@qualisero/pi-agent-scip` | `0.3.0` | 17 | 196 | https://github.com/qualisero/pi-agent-scip.git |
| `pi-interview` | `0.4.5` | 175 | 868 | https://github.com/nicobailon/pi-interview-tool.git |

## GitHub / 社区扫描（bd‑3jxt）— 首轮（2026‑02‑05）

这是通过 GitHub 主题页面（`claude-code-plugin`、`claude-code-plugins`）以及 Pi 官方源和社区精选清单发现的扩展生态的**高信号、非穷尽**快照。“Updated” 反映 GitHub 仓库的 `updated_at` 字段（UTC）。Release 标签为存在时的最新 GitHub release。**分类/备注为推断得出**，源自仓库名称/描述，需在后续工作中验证。

| Repo | Category | Stars / Forks | Updated (UTC) | License | Latest Release | Notes |
|---|---|---:|---|---|---|---|
| `badlogic/pi-mono` | Official repo | 6,977 / 717 | 2026‑02‑05 | MIT | v0.51.6 | — |
| `wshobson/agents` | Community repo | 27,847 / 3,068 | 2026‑02‑05 | MIT | none | — |
| `timescale/pg-aiguide` | Community repo | 1,501 / 77 | 2026‑02‑05 | Apache‑2.0 | v0.3.0 | — |
| `jeremylongshore/claude-code-plugins-plus-skills` | Community repo | 1,285 / 155 | 2026‑02‑05 | NOASSERTION | v4.14.0 | — |
| `kenryu42/claude-code-safety-net` | Community repo | 972 / 42 | 2026‑02‑05 | MIT | v0.7.1 | — |
| `gmickel/gmickel-claude-marketplace` | Community repo | 501 / 33 | 2026‑02‑05 | MIT | flow-next‑v0.20.19 | — |
| `ccplugins/awesome-claude-code-plugins` | Curated list | 440 / 65 | 2026‑02‑05 | Apache‑2.0 | none | — |
| `fcakyon/claude-codex-settings` | Community repo | 401 / 39 | 2026‑02‑05 | Apache‑2.0 | v2.1.0 | — |
| `quemsah/awesome-claude-plugins` | Curated list | 89 / 4 | 2026‑02‑05 | NONE | none | — |
| `vincenthopf/My-Claude-Code` | Curated list | 127 / 3 | 2026‑02‑02 | NOASSERTION | none | — |
| `steipete/claude-code-mcp` | Community repo | 1,073 / 128 | 2026‑02‑05 | MIT | v1.10.2 | MCP server |
| `siteboon/claudecodeui` | Community repo | 6,018 / 787 | 2026‑02‑05 | GPL‑3.0 | v1.16.3 | UI wrapper |
| `disler/claude-code-hooks-mastery` | Community repo | 2,534 / 509 | 2026‑02‑05 | NONE | none | Hooks |
| `hesreallyhim/awesome-claude-code` | Curated list | 22,903 / 1,319 | 2026‑02‑05 | NOASSERTION | none | — |
| `ComposioHQ/awesome-claude-skills` | Curated list | 30,633 / 2,921 | 2026‑02‑05 | NONE | none | — |

### 主题扫描：`pi-agent`、`pi-coding-agent`、`pi-extension`（长尾）

| Repo | Category | Stars / Forks | Updated (UTC) | License | Latest Release | Notes |
|---|---|---:|---|---|---|---|
| `Piebald-AI/splitrail` | Community repo | 100 / 10 | 2026‑02‑05 | MIT | v3.3.1 | — |
| `qualisero/awesome-pi-agent` | Curated list | 49 / 5 | 2026‑02‑05 | MIT | none | — |
| `tmustier/pi-extensions` | Community repo | 35 / 4 | 2026‑02‑05 | MIT | pi-skill-creator/v0.2.0 | — |
| `nicobailon/pi-web-access` | Community repo | 34 / 1 | 2026‑02‑05 | MIT | v0.7.2 | — |
| `tmustier/pi-nes` | Community repo | 13 / 1 | 2026‑02‑03 | MIT | v0.2.36 | — |
| `ben-vargas/pi-packages` | Community repo | 7 / 1 | 2026‑02‑05 | MIT | none | — |
| `Graffioh/pi-super-curl` | Community repo | 3 / 0 | 2026‑02‑05 | MIT | none | — |
| `imsus/pi-extension-minimax-coding-plan-mcp` | Community repo | 0 / 0 | 2026‑01‑29 | MIT | v1.0.0 | — |

### 仓库搜索日志（bd‑kgmr）— 展开视图

```json
{
  "executed_at": "2026-02-05T17:29:10Z",
  "queries": [
    {
      "query": "topic:pi-agent",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": [
        {"repo": "Piebald-AI/splitrail", "stars": 100, "forks": 10, "updated_at": "2026-02-05T12:17:59Z", "license": "mit", "url": "https://github.com/Piebald-AI/splitrail"},
        {"repo": "qualisero/awesome-pi-agent", "stars": 49, "forks": 5, "updated_at": "2026-02-05T10:28:25Z", "license": "mit", "url": "https://github.com/qualisero/awesome-pi-agent"},
        {"repo": "qualisero/rhubarb-pi", "stars": 2, "forks": 0, "updated_at": "2026-01-25T22:30:56Z", "license": "mit", "url": "https://github.com/qualisero/rhubarb-pi"},
        {"repo": "Dwsy/ace-tool-skill", "stars": 0, "forks": 0, "updated_at": "2026-01-23T01:23:44Z", "license": "mit", "url": "https://github.com/Dwsy/ace-tool-skill"},
        {"repo": "Dwsy/knowledge-builder-extension", "stars": 0, "forks": 0, "updated_at": "2026-01-07T14:04:16Z", "license": "", "url": "https://github.com/Dwsy/knowledge-builder-extension"}
      ]
    },
    {
      "query": "topic:pi-extension",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": [
        {"repo": "ben-vargas/pi-packages", "stars": 7, "forks": 1, "updated_at": "2026-02-05T07:04:12Z", "license": "mit", "url": "https://github.com/ben-vargas/pi-packages"},
        {"repo": "Graffioh/pi-super-curl", "stars": 3, "forks": 0, "updated_at": "2026-02-05T09:31:40Z", "license": "mit", "url": "https://github.com/Graffioh/pi-super-curl"},
        {"repo": "default-anton/pi-moonshot", "stars": 1, "forks": 0, "updated_at": "2026-01-27T19:28:10Z", "license": "", "url": "https://github.com/default-anton/pi-moonshot"},
        {"repo": "default-anton/pi-subdir-context", "stars": 1, "forks": 0, "updated_at": "2026-01-29T20:13:06Z", "license": "mit", "url": "https://github.com/default-anton/pi-subdir-context"},
        {"repo": "imsus/pi-extension-minimax-coding-plan-mcp", "stars": 0, "forks": 0, "updated_at": "2026-01-29T14:54:38Z", "license": "mit", "url": "https://github.com/imsus/pi-extension-minimax-coding-plan-mcp"},
        {"repo": "juanibiapina/pi-gob", "stars": 0, "forks": 0, "updated_at": "2026-02-04T14:54:47Z", "license": "mit", "url": "https://github.com/juanibiapina/pi-gob"},
        {"repo": "gturkoglu/pi-dynsys", "stars": 0, "forks": 0, "updated_at": "2026-02-04T23:13:42Z", "license": "mit", "url": "https://github.com/gturkoglu/pi-dynsys"},
        {"repo": "juanibiapina/pi-files", "stars": 0, "forks": 0, "updated_at": "2026-02-04T07:50:39Z", "license": "mit", "url": "https://github.com/juanibiapina/pi-files"}
      ]
    },
    {
      "query": "topic:pi-coding-agent",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": [
        {"repo": "tmustier/pi-extensions", "stars": 35, "forks": 4, "updated_at": "2026-02-05T13:32:49Z", "license": "mit", "url": "https://github.com/tmustier/pi-extensions"},
        {"repo": "nicobailon/pi-web-access", "stars": 34, "forks": 1, "updated_at": "2026-02-05T16:46:52Z", "license": "mit", "url": "https://github.com/nicobailon/pi-web-access"},
        {"repo": "tmustier/pi-nes", "stars": 13, "forks": 1, "updated_at": "2026-02-03T22:50:24Z", "license": "mit", "url": "https://github.com/tmustier/pi-nes"},
        {"repo": "mxyhi/ok-skills", "stars": 3, "forks": 0, "updated_at": "2026-02-04T04:09:08Z", "license": "apache-2.0", "url": "https://github.com/mxyhi/ok-skills"},
        {"repo": "gturkoglu/pi-codex-apply-patch", "stars": 2, "forks": 0, "updated_at": "2026-02-02T05:27:32Z", "license": "mit", "url": "https://github.com/gturkoglu/pi-codex-apply-patch"},
        {"repo": "otahontas/pi-coding-agent-catppuccin", "stars": 1, "forks": 0, "updated_at": "2026-02-03T22:57:39Z", "license": "", "url": "https://github.com/otahontas/pi-coding-agent-catppuccin"},
        {"repo": "zenobi-us/pi-rose-pine", "stars": 1, "forks": 1, "updated_at": "2026-02-03T04:24:53Z", "license": "mit", "url": "https://github.com/zenobi-us/pi-rose-pine"},
        {"repo": "imsus/pi-extension-minimax-coding-plan-mcp", "stars": 0, "forks": 0, "updated_at": "2026-01-29T14:54:38Z", "license": "mit", "url": "https://github.com/imsus/pi-extension-minimax-coding-plan-mcp"},
        {"repo": "gturkoglu/pi-dynsys", "stars": 0, "forks": 0, "updated_at": "2026-02-04T23:13:42Z", "license": "mit", "url": "https://github.com/gturkoglu/pi-dynsys"}
      ]
    },
    {
      "query": "buildwithpi extension",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": []
    },
    {
      "query": "\"pi-mono\" extension",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": []
    },
    {
      "query": "\"pi agent\" extension language:TypeScript",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": [
        {"repo": "yulqen/conductor-pi", "stars": 2, "forks": 0, "updated_at": "2026-02-04T04:19:55Z", "license": "other", "url": "https://github.com/yulqen/conductor-pi"},
        {"repo": "rytswd/pi-agent-extensions", "stars": 2, "forks": 0, "updated_at": "2026-02-05T13:32:15Z", "license": "mit", "url": "https://github.com/rytswd/pi-agent-extensions"},
        {"repo": "lebonbruce/pi-hippocampus", "stars": 3, "forks": 1, "updated_at": "2026-02-03T11:52:06Z", "license": "mit", "url": "https://github.com/lebonbruce/pi-hippocampus"},
        {"repo": "byteowlz/pi-agent-extensions", "stars": 0, "forks": 0, "updated_at": "2026-01-30T08:26:00Z", "license": "", "url": "https://github.com/byteowlz/pi-agent-extensions"},
        {"repo": "charles-cooper/pi-extensions", "stars": 0, "forks": 0, "updated_at": "2026-01-28T14:54:33Z", "license": "mit", "url": "https://github.com/charles-cooper/pi-extensions"},
        {"repo": "Willyfrog/pi-agent-extensions", "stars": 0, "forks": 0, "updated_at": "2026-01-15T23:53:28Z", "license": "mit", "url": "https://github.com/Willyfrog/pi-agent-extensions"},
        {"repo": "Itsnotaka/dot-pi", "stars": 0, "forks": 0, "updated_at": "2026-02-05T07:22:43Z", "license": "", "url": "https://github.com/Itsnotaka/dot-pi"},
        {"repo": "LEUNGUU/pi-agent-config", "stars": 0, "forks": 0, "updated_at": "2026-01-20T07:17:03Z", "license": "", "url": "https://github.com/LEUNGUU/pi-agent-config"}
      ]
    },
    {
      "query": "\"pi agent\" extension language:JavaScript",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": [
        {"repo": "Volantk/pi-agent-skills-extensions", "stars": 0, "forks": 0, "updated_at": "2026-02-04T09:12:25Z", "license": "", "url": "https://github.com/Volantk/pi-agent-skills-extensions"}
      ]
    },
    {
      "query": "\"Pi Agent\" extension",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": []
    },
    {
      "query": "\"pi coding agent\" extension",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": [
        {"repo": "nicobailon/pi-interactive-shell", "stars": 109, "forks": 5, "updated_at": "2026-02-04T21:56:07Z", "license": "", "url": "https://github.com/nicobailon/pi-interactive-shell"},
        {"repo": "nicobailon/pi-model-switch", "stars": 9, "forks": 0, "updated_at": "2026-02-02T00:42:43Z", "license": "", "url": "https://github.com/nicobailon/pi-model-switch"},
        {"repo": "toorusr/ai-extensions", "stars": 0, "forks": 0, "updated_at": "2026-01-26T20:46:47Z", "license": "", "url": "https://github.com/toorusr/ai-extensions"},
        {"repo": "ferologics/pi-extensions", "stars": 1, "forks": 0, "updated_at": "2026-01-25T14:41:05Z", "license": "", "url": "https://github.com/ferologics/pi-extensions"},
        {"repo": "assagman/pi-extensions", "stars": 1, "forks": 0, "updated_at": "2026-01-30T20:47:06Z", "license": "mit", "url": "https://github.com/assagman/pi-extensions"},
        {"repo": "zenobi-us/pi-zk", "stars": 0, "forks": 0, "updated_at": "2026-01-31T14:32:02Z", "license": "mit", "url": "https://github.com/zenobi-us/pi-zk"},
        {"repo": "Istar-Eldritch/ai-tools", "stars": 0, "forks": 1, "updated_at": "2026-02-05T14:42:44Z", "license": "", "url": "https://github.com/Istar-Eldritch/ai-tools"},
        {"repo": "carsonfarmer/pi-extensions", "stars": 0, "forks": 0, "updated_at": "2026-02-05T05:27:40Z", "license": "", "url": "https://github.com/carsonfarmer/pi-extensions"},
        {"repo": "gturkoglu/pi-codex-apply-patch", "stars": 2, "forks": 0, "updated_at": "2026-02-02T05:27:32Z", "license": "mit", "url": "https://github.com/gturkoglu/pi-codex-apply-patch"},
        {"repo": "Istar-Eldritch/pi-wakatime", "stars": 0, "forks": 0, "updated_at": "2026-01-23T19:22:59Z", "license": "mit", "url": "https://github.com/Istar-Eldritch/pi-wakatime"}
      ]
    },
    {
      "query": "\"pi-coding-agent\" extension",
      "executed_at": "2026-02-05T17:29:10Z",
      "limit": 30,
      "results": [
        {"repo": "nicobailon/pi-interactive-shell", "stars": 109, "forks": 5, "updated_at": "2026-02-04T21:56:07Z", "license": "", "url": "https://github.com/nicobailon/pi-interactive-shell"},
        {"repo": "nicobailon/pi-model-switch", "stars": 9, "forks": 0, "updated_at": "2026-02-02T00:42:43Z", "license": "", "url": "https://github.com/nicobailon/pi-model-switch"},
        {"repo": "ferologics/pi-extensions", "stars": 1, "forks": 0, "updated_at": "2026-01-25T14:41:05Z", "license": "", "url": "https://github.com/ferologics/pi-extensions"},
        {"repo": "assagman/pi-extensions", "stars": 1, "forks": 0, "updated_at": "2026-01-30T20:47:06Z", "license": "mit", "url": "https://github.com/assagman/pi-extensions"},
        {"repo": "zenobi-us/pi-zk", "stars": 0, "forks": 0, "updated_at": "2026-01-31T14:32:02Z", "license": "mit", "url": "https://github.com/zenobi-us/pi-zk"},
        {"repo": "Istar-Eldritch/ai-tools", "stars": 0, "forks": 1, "updated_at": "2026-02-05T14:42:44Z", "license": "", "url": "https://github.com/Istar-Eldritch/ai-tools"},
        {"repo": "carsonfarmer/pi-extensions", "stars": 0, "forks": 0, "updated_at": "2026-02-05T05:27:40Z", "license": "", "url": "https://github.com/carsonfarmer/pi-extensions"},
        {"repo": "Istar-Eldritch/pi-wakatime", "stars": 0, "forks": 0, "updated_at": "2026-01-23T19:22:59Z", "license": "mit", "url": "https://github.com/Istar-Eldritch/pi-wakatime"},
        {"repo": "gturkoglu/pi-codex-apply-patch", "stars": 2, "forks": 0, "updated_at": "2026-02-02T05:27:32Z", "license": "mit", "url": "https://github.com/gturkoglu/pi-codex-apply-patch"}
      ]
    }
  ]
}
```

```json
{
  "executed_at": "2026-02-05T19:25:07Z",
  "queries": [
    {
      "query": "topic:claude-code",
      "executed_at": "2026-02-05T19:25:07Z",
      "limit": 30,
      "result_count": 30,
      "top_results": [
        {"repo": "affaan-m/everything-claude-code", "stars": 40494, "forks": 5015, "updated_at": "2026-02-05T19:24:48Z", "license": "mit", "url": "https://github.com/affaan-m/everything-claude-code"},
        {"repo": "CherryHQ/cherry-studio", "stars": 39337, "forks": 3618, "updated_at": "2026-02-05T18:26:57Z", "license": "agpl-3.0", "url": "https://github.com/CherryHQ/cherry-studio"},
        {"repo": "ComposioHQ/awesome-claude-skills", "stars": 30683, "forks": 2930, "updated_at": "2026-02-05T19:23:24Z", "license": "", "url": "https://github.com/ComposioHQ/awesome-claude-skills"},
        {"repo": "code-yeongyu/oh-my-opencode", "stars": 28473, "forks": 2092, "updated_at": "2026-02-05T19:18:45Z", "license": "other", "url": "https://github.com/code-yeongyu/oh-my-opencode"},
        {"repo": "nextlevelbuilder/ui-ux-pro-max-skill", "stars": 28146, "forks": 2828, "updated_at": "2026-02-05T19:23:14Z", "license": "mit", "url": "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill"},
        {"repo": "wshobson/agents", "stars": 27858, "forks": 3070, "updated_at": "2026-02-05T18:57:40Z", "license": "mit", "url": "https://github.com/wshobson/agents"},
        {"repo": "thedotmack/claude-mem", "stars": 23578, "forks": 1565, "updated_at": "2026-02-05T19:24:25Z", "license": "other", "url": "https://github.com/thedotmack/claude-mem"},
        {"repo": "hesreallyhim/awesome-claude-code", "stars": 22907, "forks": 1319, "updated_at": "2026-02-05T18:31:40Z", "license": "other", "url": "https://github.com/hesreallyhim/awesome-claude-code"},
        {"repo": "winfunc/opcode", "stars": 20418, "forks": 1590, "updated_at": "2026-02-05T17:22:01Z", "license": "agpl-3.0", "url": "https://github.com/winfunc/opcode"},
        {"repo": "oraios/serena", "stars": 19750, "forks": 1332, "updated_at": "2026-02-05T19:12:54Z", "license": "mit", "url": "https://github.com/oraios/serena"}
      ]
    },
    {
      "query": "topic:claude-code-plugin",
      "executed_at": "2026-02-05T19:25:07Z",
      "limit": 30,
      "result_count": 30,
      "top_results": [
        {"repo": "wshobson/agents", "stars": 27858, "forks": 3070, "updated_at": "2026-02-05T18:57:40Z", "license": "mit", "url": "https://github.com/wshobson/agents"},
        {"repo": "thedotmack/claude-mem", "stars": 23578, "forks": 1565, "updated_at": "2026-02-05T19:24:25Z", "license": "other", "url": "https://github.com/thedotmack/claude-mem"},
        {"repo": "timescale/pg-aiguide", "stars": 1501, "forks": 77, "updated_at": "2026-02-05T09:38:11Z", "license": "apache-2.0", "url": "https://github.com/timescale/pg-aiguide"},
        {"repo": "kenryu42/claude-code-safety-net", "stars": 972, "forks": 42, "updated_at": "2026-02-05T17:01:06Z", "license": "mit", "url": "https://github.com/kenryu42/claude-code-safety-net"},
        {"repo": "gmickel/gmickel-claude-marketplace", "stars": 501, "forks": 33, "updated_at": "2026-02-05T11:25:26Z", "license": "mit", "url": "https://github.com/gmickel/gmickel-claude-marketplace"},
        {"repo": "zscole/adversarial-spec", "stars": 473, "forks": 41, "updated_at": "2026-02-03T22:23:19Z", "license": "mit", "url": "https://github.com/zscole/adversarial-spec"},
        {"repo": "ccplugins/awesome-claude-code-plugins", "stars": 440, "forks": 65, "updated_at": "2026-02-05T10:00:36Z", "license": "apache-2.0", "url": "https://github.com/ccplugins/awesome-claude-code-plugins"},
        {"repo": "fcakyon/claude-codex-settings", "stars": 401, "forks": 39, "updated_at": "2026-02-05T16:00:46Z", "license": "apache-2.0", "url": "https://github.com/fcakyon/claude-codex-settings"},
        {"repo": "keskinonur/claude-code-ios-dev-guide", "stars": 293, "forks": 34, "updated_at": "2026-02-05T15:20:39Z", "license": "", "url": "https://github.com/keskinonur/claude-code-ios-dev-guide"},
        {"repo": "jarrodwatts/claude-stt", "stars": 290, "forks": 27, "updated_at": "2026-02-05T17:36:16Z", "license": "mit", "url": "https://github.com/jarrodwatts/claude-stt"}
      ]
    },
    {
      "query": "topic:claude-code-plugins",
      "executed_at": "2026-02-05T19:25:07Z",
      "limit": 30,
      "result_count": 30,
      "top_results": [
        {"repo": "wshobson/agents", "stars": 27858, "forks": 3070, "updated_at": "2026-02-05T18:57:40Z", "license": "mit", "url": "https://github.com/wshobson/agents"},
        {"repo": "timescale/pg-aiguide", "stars": 1501, "forks": 77, "updated_at": "2026-02-05T09:38:11Z", "license": "apache-2.0", "url": "https://github.com/timescale/pg-aiguide"},
        {"repo": "jeremylongshore/claude-code-plugins-plus-skills", "stars": 1288, "forks": 156, "updated_at": "2026-02-05T19:09:00Z", "license": "other", "url": "https://github.com/jeremylongshore/claude-code-plugins-plus-skills"},
        {"repo": "malob/nix-config", "stars": 450, "forks": 35, "updated_at": "2026-02-04T19:39:30Z", "license": "mit", "url": "https://github.com/malob/nix-config"},
        {"repo": "quemsah/awesome-claude-plugins", "stars": 89, "forks": 4, "updated_at": "2026-02-05T08:39:03Z", "license": "", "url": "https://github.com/quemsah/awesome-claude-plugins"},
        {"repo": "NikiforovAll/claude-code-rules", "stars": 80, "forks": 13, "updated_at": "2026-02-02T08:26:17Z", "license": "apache-2.0", "url": "https://github.com/NikiforovAll/claude-code-rules"},
        {"repo": "PCIRCLE-AI/claude-code-buddy", "stars": 56, "forks": 12, "updated_at": "2026-02-05T16:35:48Z", "license": "agpl-3.0", "url": "https://github.com/PCIRCLE-AI/claude-code-buddy"},
        {"repo": "wakatime/claude-code-wakatime", "stars": 48, "forks": 11, "updated_at": "2026-02-02T03:33:00Z", "license": "bsd-3-clause", "url": "https://github.com/wakatime/claude-code-wakatime"},
        {"repo": "secondsky/claude-skills", "stars": 42, "forks": 1, "updated_at": "2026-02-05T14:30:30Z", "license": "", "url": "https://github.com/secondsky/claude-skills"},
        {"repo": "Securiteru/codex-openai-proxy", "stars": 66, "forks": 5, "updated_at": "2026-02-03T18:49:10Z", "license": "mit", "url": "https://github.com/Securiteru/codex-openai-proxy"}
      ]
    },
    {
      "query": "\"claude code\" extension",
      "executed_at": "2026-02-05T19:25:07Z",
      "limit": 30,
      "result_count": 30,
      "top_results": [
        {"repo": "Securiteru/codex-openai-proxy", "stars": 66, "forks": 5, "updated_at": "2026-02-03T18:49:10Z", "license": "mit", "url": "https://github.com/Securiteru/codex-openai-proxy"},
        {"repo": "ntanner-ctrl/claude-bootstrap", "stars": 54, "forks": 5, "updated_at": "2026-02-01T06:24:48Z", "license": "", "url": "https://github.com/ntanner-ctrl/claude-bootstrap"},
        {"repo": "jimmy927/claude-code-extension-patcher", "stars": 15, "forks": 4, "updated_at": "2025-12-15T23:30:59Z", "license": "", "url": "https://github.com/jimmy927/claude-code-extension-patcher"},
        {"repo": "ruimgbarros/data-journalism-marketplace", "stars": 14, "forks": 0, "updated_at": "2026-01-29T22:50:20Z", "license": "mit", "url": "https://github.com/ruimgbarros/data-journalism-marketplace"},
        {"repo": "aegntic/cldcde", "stars": 9, "forks": 0, "updated_at": "2026-01-29T16:46:33Z", "license": "mit", "url": "https://github.com/aegntic/cldcde"},
        {"repo": "yuji0809/cc-recommender", "stars": 8, "forks": 0, "updated_at": "2026-02-04T12:44:59Z", "license": "mit", "url": "https://github.com/yuji0809/cc-recommender"},
        {"repo": "0x1NotMe/claude-workspace-tools", "stars": 8, "forks": 2, "updated_at": "2026-01-17T18:10:59Z", "license": "", "url": "https://github.com/0x1NotMe/claude-workspace-tools"},
        {"repo": "zpaper-com/ClaudeKit", "stars": 6, "forks": 4, "updated_at": "2025-12-21T04:21:06Z", "license": "", "url": "https://github.com/zpaper-com/ClaudeKit"},
        {"repo": "walidboulanouar/ay-claude-templates", "stars": 5, "forks": 1, "updated_at": "2026-02-02T14:06:57Z", "license": "mit", "url": "https://github.com/walidboulanouar/ay-claude-templates"},
        {"repo": "Autopsias/slashagents", "stars": 4, "forks": 2, "updated_at": "2026-01-16T20:42:36Z", "license": "mit", "url": "https://github.com/Autopsias/slashagents"}
      ]
    },
    {
      "query": "\"claude-code\" extension",
      "executed_at": "2026-02-05T19:25:07Z",
      "limit": 30,
      "result_count": 30,
      "top_results": [
        {"repo": "Securiteru/codex-openai-proxy", "stars": 66, "forks": 5, "updated_at": "2026-02-03T18:49:10Z", "license": "mit", "url": "https://github.com/Securiteru/codex-openai-proxy"},
        {"repo": "ntanner-ctrl/claude-bootstrap", "stars": 54, "forks": 5, "updated_at": "2026-02-01T06:24:48Z", "license": "", "url": "https://github.com/ntanner-ctrl/claude-bootstrap"},
        {"repo": "jimmy927/claude-code-extension-patcher", "stars": 15, "forks": 4, "updated_at": "2025-12-15T23:30:59Z", "license": "", "url": "https://github.com/jimmy927/claude-code-extension-patcher"},
        {"repo": "ruimgbarros/data-journalism-marketplace", "stars": 14, "forks": 0, "updated_at": "2026-01-29T22:50:20Z", "license": "mit", "url": "https://github.com/ruimgbarros/data-journalism-marketplace"},
        {"repo": "aegntic/cldcde", "stars": 9, "forks": 0, "updated_at": "2026-01-29T16:46:33Z", "license": "mit", "url": "https://github.com/aegntic/cldcde"},
        {"repo": "yuji0809/cc-recommender", "stars": 8, "forks": 0, "updated_at": "2026-02-04T12:44:59Z", "license": "mit", "url": "https://github.com/yuji0809/cc-recommender"},
        {"repo": "0x1NotMe/claude-workspace-tools", "stars": 8, "forks": 2, "updated_at": "2026-01-17T18:10:59Z", "license": "", "url": "https://github.com/0x1NotMe/claude-workspace-tools"},
        {"repo": "zpaper-com/ClaudeKit", "stars": 6, "forks": 4, "updated_at": "2025-12-21T04:21:06Z", "license": "", "url": "https://github.com/zpaper-com/ClaudeKit"},
        {"repo": "walidboulanouar/ay-claude-templates", "stars": 5, "forks": 1, "updated_at": "2026-02-02T14:06:57Z", "license": "mit", "url": "https://github.com/walidboulanouar/ay-claude-templates"},
        {"repo": "Autopsias/slashagents", "stars": 4, "forks": 2, "updated_at": "2026-01-16T20:42:36Z", "license": "mit", "url": "https://github.com/Autopsias/slashagents"}
      ]
    },
    {
      "query": "\"claude\" \"mcp\" extension",
      "executed_at": "2026-02-05T19:25:07Z",
      "limit": 30,
      "result_count": 2,
      "top_results": [
        {"repo": "k3d3/firefox_mcpbridge", "stars": 4, "forks": 0, "updated_at": "2025-08-31T14:44:29Z", "license": "", "url": "https://github.com/k3d3/firefox_mcpbridge"},
        {"repo": "k3d3/mcpbridge", "stars": 2, "forks": 1, "updated_at": "2025-08-25T05:18:42Z", "license": "", "url": "https://github.com/k3d3/mcpbridge"}
      ]
    }
  ]
}
```

```json
{
  "executed_at": "2026-02-05T19:31:10Z",
  "queries": [
    {
      "query": "\"pi extension\" in:name,description language:TypeScript",
      "executed_at": "2026-02-05T19:31:10Z",
      "limit": 50,
      "result_count": 50,
      "top_results": [
        {"repo": "microsoft/azure-pipelines-extensions", "stars": 299, "forks": 426, "updated_at": "2026-01-27T14:30:57Z", "license": "mit", "url": "https://github.com/microsoft/azure-pipelines-extensions"},
        {"repo": "tony2001/pinba_extension", "stars": 86, "forks": 24, "updated_at": "2025-08-29T13:25:10Z", "license": "lgpl-2.1", "url": "https://github.com/tony2001/pinba_extension"},
        {"repo": "hackup/Pi1541io", "stars": 96, "forks": 20, "updated_at": "2025-12-26T05:49:21Z", "license": "cc-by-sa-4.0", "url": "https://github.com/hackup/Pi1541io"},
        {"repo": "hashicorp/azure-pipelines-extension-terraform", "stars": 63, "forks": 23, "updated_at": "2025-01-13T21:32:26Z", "license": "mpl-2.0", "url": "https://github.com/hashicorp/azure-pipelines-extension-terraform"},
        {"repo": "chrdavis/PIFShellExtensions", "stars": 35, "forks": 8, "updated_at": "2026-02-04T23:14:11Z", "license": "mit", "url": "https://github.com/chrdavis/PIFShellExtensions"},
        {"repo": "microsoft/powerbi-azure-pipelines-extensions", "stars": 41, "forks": 13, "updated_at": "2025-10-30T23:48:32Z", "license": "mit", "url": "https://github.com/microsoft/powerbi-azure-pipelines-extensions"},
        {"repo": "leognon/ClonePilotExtension", "stars": 98, "forks": 7, "updated_at": "2025-10-27T12:52:50Z", "license": "", "url": "https://github.com/leognon/ClonePilotExtension"},
        {"repo": "winstonpuckett/WinstonPuckett.PipeExtensions", "stars": 40, "forks": 4, "updated_at": "2025-09-05T03:51:55Z", "license": "mit", "url": "https://github.com/winstonpuckett/WinstonPuckett.PipeExtensions"},
        {"repo": "code-philia/CoEdPilot-extension", "stars": 21, "forks": 9, "updated_at": "2025-07-01T04:28:58Z", "license": "", "url": "https://github.com/code-philia/CoEdPilot-extension"},
        {"repo": "mnholtz/pixiebrix-extension", "stars": 0, "forks": 24, "updated_at": "2021-06-24T00:54:10Z", "license": "gpl-3.0", "url": "https://github.com/mnholtz/pixiebrix-extension"}
      ]
    },
    {
      "query": "\"pi extension\" in:readme language:TypeScript",
      "executed_at": "2026-02-05T19:31:10Z",
      "limit": 50,
      "result_count": 50,
      "top_results": [
        {"repo": "tmustier/pi-extensions", "stars": 36, "forks": 4, "updated_at": "2026-02-05T18:22:58Z", "license": "mit", "url": "https://github.com/tmustier/pi-extensions"},
        {"repo": "mitsuhiko/agent-stuff", "stars": 911, "forks": 50, "updated_at": "2026-02-05T19:04:23Z", "license": "apache-2.0", "url": "https://github.com/mitsuhiko/agent-stuff"},
        {"repo": "mactkg/vscode-sonic-pi", "stars": 26, "forks": 10, "updated_at": "2023-03-10T09:00:13Z", "license": "mit", "url": "https://github.com/mactkg/vscode-sonic-pi"},
        {"repo": "qualisero/awesome-pi-agent", "stars": 49, "forks": 5, "updated_at": "2026-02-05T10:28:25Z", "license": "mit", "url": "https://github.com/qualisero/awesome-pi-agent"},
        {"repo": "voocel/openclaw-mini", "stars": 280, "forks": 16, "updated_at": "2026-02-05T13:06:43Z", "license": "mit", "url": "https://github.com/voocel/openclaw-mini"},
        {"repo": "aliou/pi-extensions", "stars": 21, "forks": 2, "updated_at": "2026-02-05T19:02:58Z", "license": "", "url": "https://github.com/aliou/pi-extensions"},
        {"repo": "meesokim/spc1000", "stars": 8, "forks": 6, "updated_at": "2026-01-18T12:18:39Z", "license": "", "url": "https://github.com/meesokim/spc1000"},
        {"repo": "nicobailon/pi-annotate", "stars": 35, "forks": 2, "updated_at": "2026-02-02T20:51:21Z", "license": "mit", "url": "https://github.com/nicobailon/pi-annotate"},
        {"repo": "yongsukki/clickpirc", "stars": 2, "forks": 9, "updated_at": "2025-04-02T12:45:37Z", "license": "mit", "url": "https://github.com/yongsukki/clickpirc"},
        {"repo": "nat-n/socket_control", "stars": 15, "forks": 5, "updated_at": "2024-10-20T11:59:19Z", "license": "", "url": "https://github.com/nat-n/socket_control"}
      ]
    },
    {
      "query": "\"pi agent\" \"extension\" in:readme language:TypeScript",
      "executed_at": "2026-02-05T19:31:10Z",
      "limit": 50,
      "result_count": 50,
      "top_results": [
        {"repo": "openclaw/openclaw", "stars": 167158, "forks": 26567, "updated_at": "2026-02-05T19:30:43Z", "license": "mit", "url": "https://github.com/openclaw/openclaw"},
        {"repo": "qualisero/awesome-pi-agent", "stars": 49, "forks": 5, "updated_at": "2026-02-05T10:28:25Z", "license": "mit", "url": "https://github.com/qualisero/awesome-pi-agent"},
        {"repo": "nicobailon/pi-rewind-hook", "stars": 36, "forks": 3, "updated_at": "2026-02-04T15:10:39Z", "license": "", "url": "https://github.com/nicobailon/pi-rewind-hook"},
        {"repo": "tmustier/pi-extensions", "stars": 36, "forks": 4, "updated_at": "2026-02-05T18:22:58Z", "license": "mit", "url": "https://github.com/tmustier/pi-extensions"},
        {"repo": "Piebald-AI/splitrail", "stars": 100, "forks": 10, "updated_at": "2026-02-05T19:04:58Z", "license": "mit", "url": "https://github.com/Piebald-AI/splitrail"},
        {"repo": "nicobailon/pi-interview-tool", "stars": 73, "forks": 7, "updated_at": "2026-02-05T17:36:33Z", "license": "", "url": "https://github.com/nicobailon/pi-interview-tool"},
        {"repo": "dannote/dot-pi", "stars": 10, "forks": 3, "updated_at": "2026-02-04T19:37:14Z", "license": "mit", "url": "https://github.com/dannote/dot-pi"},
        {"repo": "melihmucuk/leash", "stars": 37, "forks": 6, "updated_at": "2026-01-28T08:37:17Z", "license": "mit", "url": "https://github.com/melihmucuk/leash"},
{"repo": "Dicklesworthstone/pi_agent_rust", "stars": 15, "forks": 4, "updated_at": "2026-02-05T19:28:47Z", "license": "mit", "url": "https://github.com/Dicklesworthstone/pi_agent_rust"},
        {"repo": "nicobailon/mcp-to-pi-tools", "stars": 13, "forks": 2, "updated_at": "2026-02-02T18:56:42Z", "license": "", "url": "https://github.com/nicobailon/mcp-to-pi-tools"}
      ]
    },
    {
      "query": "\"pi coding agent\" in:name,description language:TypeScript",
      "executed_at": "2026-02-05T19:31:10Z",
      "limit": 50,
      "result_count": 50,
      "top_results": [
        {"repo": "badlogic/pi-skills", "stars": 338, "forks": 35, "updated_at": "2026-02-05T18:25:26Z", "license": "mit", "url": "https://github.com/badlogic/pi-skills"},
        {"repo": "hjanuschka/shitty-extensions", "stars": 41, "forks": 5, "updated_at": "2026-02-05T03:14:02Z", "license": "", "url": "https://github.com/hjanuschka/shitty-extensions"},
        {"repo": "dnouri/pi-coding-agent", "stars": 31, "forks": 6, "updated_at": "2026-02-04T21:54:26Z", "license": "gpl-3.0", "url": "https://github.com/dnouri/pi-coding-agent"},
        {"repo": "nicobailon/pi-mcp-adapter", "stars": 45, "forks": 2, "updated_at": "2026-02-05T12:19:28Z", "license": "mit", "url": "https://github.com/nicobailon/pi-mcp-adapter"},
        {"repo": "nicobailon/pi-review-loop", "stars": 20, "forks": 3, "updated_at": "2026-02-02T19:01:21Z", "license": "mit", "url": "https://github.com/nicobailon/pi-review-loop"},
        {"repo": "qualisero/awesome-pi-agent", "stars": 49, "forks": 5, "updated_at": "2026-02-05T10:28:25Z", "license": "mit", "url": "https://github.com/qualisero/awesome-pi-agent"},
        {"repo": "nicobailon/pi-powerline-footer", "stars": 14, "forks": 2, "updated_at": "2026-02-02T21:01:50Z", "license": "", "url": "https://github.com/nicobailon/pi-powerline-footer"},
        {"repo": "dannote/dot-pi", "stars": 10, "forks": 3, "updated_at": "2026-02-04T19:37:14Z", "license": "mit", "url": "https://github.com/dannote/dot-pi"},
        {"repo": "nicobailon/pi-interactive-shell", "stars": 109, "forks": 5, "updated_at": "2026-02-04T21:56:07Z", "license": "", "url": "https://github.com/nicobailon/pi-interactive-shell"},
        {"repo": "nicobailon/pi-web-access", "stars": 34, "forks": 1, "updated_at": "2026-02-05T16:46:52Z", "license": "mit", "url": "https://github.com/nicobailon/pi-web-access"}
      ]
    },
    {
      "query": "\"pi-extensions\" in:name,description",
      "executed_at": "2026-02-05T19:31:10Z",
      "limit": 50,
      "result_count": 50,
      "top_results": [
        {"repo": "microsoft/azure-pipelines-extensions", "stars": 299, "forks": 426, "updated_at": "2026-01-27T14:30:57Z", "license": "mit", "url": "https://github.com/microsoft/azure-pipelines-extensions"},
        {"repo": "hackup/Pi1541io", "stars": 96, "forks": 20, "updated_at": "2025-12-26T05:49:21Z", "license": "cc-by-sa-4.0", "url": "https://github.com/hackup/Pi1541io"},
        {"repo": "tmustier/pi-extensions", "stars": 36, "forks": 4, "updated_at": "2026-02-05T18:22:58Z", "license": "mit", "url": "https://github.com/tmustier/pi-extensions"},
        {"repo": "aliou/pi-extensions", "stars": 21, "forks": 2, "updated_at": "2026-02-05T19:02:58Z", "license": "", "url": "https://github.com/aliou/pi-extensions"},
        {"repo": "chrdavis/PIFShellExtensions", "stars": 35, "forks": 8, "updated_at": "2026-02-04T23:14:11Z", "license": "mit", "url": "https://github.com/chrdavis/PIFShellExtensions"},
        {"repo": "nicobailon/pi-subagents", "stars": 87, "forks": 5, "updated_at": "2026-02-05T17:13:03Z", "license": "", "url": "https://github.com/nicobailon/pi-subagents"},
        {"repo": "microsoft/powerbi-azure-pipelines-extensions", "stars": 41, "forks": 13, "updated_at": "2025-10-30T23:48:32Z", "license": "mit", "url": "https://github.com/microsoft/powerbi-azure-pipelines-extensions"},
        {"repo": "meesokim/spc1000", "stars": 8, "forks": 6, "updated_at": "2026-01-18T12:18:39Z", "license": "", "url": "https://github.com/meesokim/spc1000"},
        {"repo": "winstonpuckett/WinstonPuckett.PipeExtensions", "stars": 40, "forks": 4, "updated_at": "2025-09-05T03:51:55Z", "license": "mit", "url": "https://github.com/winstonpuckett/WinstonPuckett.PipeExtensions"},
        {"repo": "asottile-archive/tox-pip-extensions", "stars": 36, "forks": 5, "updated_at": "2025-11-17T18:36:36Z", "license": "mit", "url": "https://github.com/asottile-archive/tox-pip-extensions"}
      ]
    }
  ]
}
```

后续工作：
- 通过 LICENSE 文件或 SPDX 元数据解析 `NOASSERTION`/`NONE` 许可证条目。
- 将覆盖范围扩展到其他高信号主题页面（例如 `claude-code-mcp`、`claude-code-hooks`）。
- 若干广撒网查询返回零结果；计划通过代码搜索 + 精选清单进行扩展以达到目标覆盖率。
- `topic:claude-code*` 查询噪声很大（许多非 Pi 仓库）；接受前需要进行代码签名验证（bd‑3l39）。
- 基于关键词的 `pi extension` / `pi-extensions` 查询**噪声非常大**（Azure Pipelines、树莓派等）；仅用作广度覆盖，接受候选前需要签名验证。

---

## 发现手册（可重复查询）(bd‑19rf)

目标：提供一份确定性的**发现渠道 + 可直接复制粘贴的查询**清单，使后续智能体能够重复在线研究并收敛到相同的候选集。

### A) 官方 Pi 源（基线）

- `pi-mono` 示例/扩展清单（本地快照）：  
  `legacy_pi_mono_code/pi-mono/packages/coding-agent/examples/extensions/README.md`
- `pi-mono` 种子扩展（本地快照）：  
  `legacy_pi_mono_code/pi-mono/.pi/extensions/`
- buildwithpi 包 + 文档：  
  https://buildwithpi.ai/  
  https://buildwithpi.ai/packages
- `badlogic` gists（扩展）：  
  https://gist.github.com/badlogic

### B) GitHub 仓库发现（基于关键词的“广撒网”）

通过 GitHub UI 搜索或 `gh search repos` 执行。记录**日期/时间**、确切查询语句以及已审查的候选仓库数量。

建议的查询（可调整语言过滤器以降低噪声）：

- `"buildwithpi" extension`
- `"pi-mono" extension`
- `"pi agent" extension language:TypeScript`
- `"pi agent" extension language:JavaScript`
- `"Pi Agent" extension`

`gh` 示例：

```bash
gh search repos '"buildwithpi" extension' --limit 200
gh search repos '"pi-mono" extension' --limit 200
gh search repos '"pi agent" extension language:TypeScript' --limit 200
gh search repos '"pi agent" extension language:JavaScript' --limit 200
```

### C) GitHub 代码发现（基于签名的“定位真实入口点”）

目标：找到包含真实 Pi 扩展注册代码的仓库，而非仅提及的仓库。

建议的代码搜索模式（在 GitHub Code Search 中或通过 `gh search code` 分别执行）：

- `registerTool(`（工具）
- `registerCommand(`（斜杠命令）
- `registerProvider(`（自定义提供方）
- `resources_discover` / `resourcesDiscover`（动态资源钩子）
- `tool_call` / `tool_result` / `turn_start` / `turn_end`（生命周期事件）

`gh` 示例：

```bash
gh search code 'registerTool(' --limit 200
gh search code 'registerCommand(' --limit 200
gh search code 'registerProvider(' --limit 200
gh search code 'resources_discover' --limit 200
```

验证启发式（推荐）：对于每个命中结果，确认仓库具有扩展入口点（例如，导出接收 Pi 上下文对象的默认函数的文件，或明显的扩展包布局）。

### C1) GitHub 代码搜索日志（bd‑3l39）—— 初轮（2026‑02‑05）

通过 `gh search code` 执行（除非另有说明，limit=100）。结果数量：

| Query | Result count |
|---|---:|
| `@mariozechner/pi-coding-agent` | 100 |
| `@mariozechner/pi-ai` | 100 |
| `registerTool(` | 100 |
| `registerCommand(` | 100 |
| `registerProvider(` | 100 |
| `ExtensionAPI` | 100 |
| `registerFlag(` | 100 |
| `registerShortcut(` | 100 |
| `registerMessageRenderer(` | 100 |
| `.pi/agent/extensions` | 9 |
| `"pi-extensions" "ExtensionAPI"` | 0 |
| `pi.registerTool(` | rate-limited (403) |
| `pi.registerCommand(` | rate-limited (403) |
| `ExtensionAPI registerTool(` | rate-limited (403) |

验证通过（51 个唯一入口点；已观测到 export‑default + 注册/事件钩子）：

| Repo | Entrypoint | Evidence |
|---|---|---|
| `openclaw/openclaw` | `.pi/extensions/redraws.ts` | `export default` + `registerCommand(...)` |
| `mitsuhiko/agent-stuff` | `pi-extensions/loop.ts` | `export default` + `registerTool(...)` |
| `joelazar/dotfiles` | `dot_pi/agent/extensions/qna.ts` | `export default` + `registerCommand(...)` |
| `w-winter/dot314` | `extensions/mac-system-theme.ts` | `export default` + `pi.on(...)` |
| `davidgasquez/dotfiles` | `agents/pi/extensions/branch-term.ts` | `export default` + `registerFlag(...)` |
| `pasky/pi-amplike` | `extensions/handoff.ts` | `export default` + `registerCommand(...)` |
| `mikeyobrien/rho` | `extensions/vault.ts` | `export default` + `pi.on(...)` |
| `mikeyobrien/rho` | `extensions/brain.ts` | `export default` + `pi.on(...)` |
| `hjanuschka/shitty-extensions` | `extensions/flicker-corp.ts` | `export default` + `registerCommand(...)` |
| `hjanuschka/shitty-extensions` | `extensions/status-widget.ts` | `export default` + `pi.on(...)` |
| `hjanuschka/shitty-extensions` | `extensions/memory-mode.ts` | `export default` + `registerCommand(...)` |
| `hjanuschka/shitty-extensions` | `extensions/plan-mode.ts` | `export default` + `registerFlag(...)` |
| `hjanuschka/shitty-extensions` | `extensions/speedreading.ts` | `export default` + `registerCommand(...)` |
| `Mic92/dotfiles` | `home/.pi/agent/extensions/direnv.ts` | `export default` + `pi.on(...)` |
| `Mic92/dotfiles` | `home/.pi/agent/extensions/custom-footer.ts` | `export default` + `pi.on(...)` |
| `leiserfg/nix-config` | `home/leiserfg/pi-extensions/fzf.ts` | `export default` + `registerShortcut(...)` |
| `leiserfg/nix-config` | `home/leiserfg/pi-extensions/notify.ts` | `export default` + `pi.on(...)` |
| `zenobi-us/dotfiles` | `devtools/files/pi/agent/extensions/lsp/lsp.ts` | `export default` + `pi.on(...)` |
| `nexxeln/dots` | `config/pi/agent/extensions/review.ts` | `export default` + `registerCommand(...)` |
| `richardgill/nix` | `out-of-store-config/ai-agents/pi/extensions/process-info.ts` | `export default` + `pi.on(...)` |
| `default-anton/dotfiles` | `pi/agent/extensions/inject-context.impl.mjs` | `export default` + `pi.on(...)` |
| `Dicklesworthstone/pi_agent_rust` | `tests/ext_conformance/artifacts/community/prateekmedia-lsp/lsp.ts` | `export default` + `registerMessageRenderer(...)` |
| `Dicklesworthstone/pi_agent_rust` | `tests/ext_conformance/artifacts/npm/lsp-pi/lsp.ts` | `export default` + `registerMessageRenderer(...)` |
| `Dicklesworthstone/pi_agent_rust` | `tests/ext_conformance/artifacts/npm/pi-mermaid/index.ts` | `export default` + `registerMessageRenderer(...)` |
| `Dwsy/agent` | `extensions/ralph/index.ts` | `export default` + `registerFlag(...)` |
| `Graffioh/dotfiles` | `pi/agent/extensions/pi-web-search/index.ts` | `export default` + `pi.on(...)` |
| `badlogic/pi-mono` | `packages/coding-agent/examples/extensions/message-renderer.ts` | `export default` + `registerMessageRenderer(...)` |
| `hjanuschka/pi-qmd` | `extensions/qmd.ts` | `export default` + `registerTool(...)` |
| `hjanuschka/shitty-extensions` | `extensions/oracle.ts` | `export default` + `registerCommand(...)` |
| `kcosr/pi-extensions` | `skill-picker/index.ts` | `export default` + `registerMessageRenderer(...)` |
| `mitsuhiko/agent-stuff` | `pi-extensions/control.ts` | `export default` + `registerFlag(...)` |
| `mrndstvndv/nixdots` | `modules/pi/package/extensions/lsp/lsp.ts` | `export default` + `registerMessageRenderer(...)` |
| `nicobailon/pi-skill-palette` | `index.ts` | `export default` + `registerMessageRenderer(...)` |
| `prateekmedia/pi-hooks` | `lsp/lsp.ts` | `export default` + `registerMessageRenderer(...)` |
| `w-winter/dot314` | `extensions/oracle.ts` | `export default` + `registerCommand(...)` |
| `w-winter/dot314` | `extensions/skill-palette/index.ts` | `export default` + `registerMessageRenderer(...)` |
| `deybhayden/dotfiles` | `.pi/agent/extensions/answer.ts` | `export default` + `registerCommand(...)` |
| `deybhayden/dotfiles` | `.pi/agent/extensions/github.ts` | `export default` + `registerTool(...)` |
| `deybhayden/dotfiles` | `.pi/agent/extensions/uv.ts` | `export default` + `pi.on(...)` |
| `joshuadavidthomas/agentkit` | `runtimes/pi/extensions/notify.ts` | `export default` + `pi.on(...)` |
| `l-lin/dotfiles` | `home-manager/modules/share/ai/pi/.pi/agent/extensions/handoff.ts` | `export default` + `registerCommand(...)` |
| `leiserfg/nix-config` | `home/leiserfg/pi-extensions/loop.ts` | `export default` + `registerTool(...)` |
| `mikeyobrien/rho` | `extensions/rho.ts` | `export default` + `pi.on(...)` |
| `nicobailon/pi-coordination` | `scout.ts` | `export default` + `registerTool(...)` |
| `pasky/pi-amplike` | `extensions/session-query.ts` | `export default` + `registerTool(...)` |
| `tmustier/pi-extensions` | `arcade/tetris.ts` | `export default` + `registerCommand(...)` |
| `tmustier/pi-extensions` | `tab-status/tab-status.ts` | `export default` + `pi.on(...)` |
| `vrslev/dotfiles` | `home/.pi/agent/extensions/todo.ts` | `export default` + `pi.on(...)` |
| `zanieb/pi-plugins` | `extensions/rename.ts` | `export default` + `registerCommand(...)` |
| `tmustier/pi-extensions` | `arcade/mario-not/mario-not.ts` | `export default` + `registerCommand(...)` |
| `tmustier/pi-extensions` | `arcade/picman.ts` | `export default` + `registerCommand(...)` |

备注 / 下一轮：
- 9 个查询触及 100 结果上限；仍有未审查的候选剩余。
- 3 个查询被 GitHub Search API 限流（见表格）；限流重置后重新运行。
- 4 条记录为已纳入的制品或官方示例（pi_agent_rust 制品 x3 + pi‑mono message‑renderer），为完整性而纳入。
- 当前已验证数量：**51 / 50** 目标。下一轮应验证队列清单中剩余的候选，并为 `registerFlag(`、`registerShortcut(`、`registerMessageRenderer(` 以及带 TS/JS 语言过滤的 `pi.registerTool(` 补充代码搜索查询。

### D) npm 发现（分发层）—— 已研究 2026‑02‑06（bd‑kcj6）

目标：找到发布 Pi 扩展或与 Pi Agent 集成的 npm 包。

**状态：已研究。** 已识别出具有基础流行度 + 新近度信号的高信号 npm 包。

#### 已执行的查询

已通过以下关键词查询 npm registry 搜索端点：
- `pi agent extension`
- `pi-agent`
- `pi-coding-agent`
- `@oh-my-pi`
- 加上针对上述发现的特定包名的后续轮次

等效的 CLI 查询（若本地已安装 `npm`）：

```bash
npm search "pi agent extension" --json | jq '.[0:50] | map({name,version,description})'
npm search buildwithpi --json | jq '.[0:50] | map({name,version,description})'
npm search pi-mono --json | jq '.[0:50] | map({name,version,description})'
```

下载量统计来自 `api.npmjs.org` 的**上周**和**上月**点计数。

#### 精选高信号集合

| Package | Version | Published | License | DL (wk) | DL (mo) | Repo |
|---|---:|---:|---|---:|---:|---|
| @mariozechner/pi-coding-agent | 0.52.6 | 2026-02-05 | MIT | 1,366,239 | 2,142,135 | git+https://github.com/badlogic/pi-mono.git |
| @mariozechner/pi | 0.52.6 | 2026-02-05 | MIT | 2,254 | 7,777 | git+https://github.com/badlogic/pi-mono.git |
| @mariozechner/pi-agent-core | 0.52.6 | 2026-02-05 | MIT | 1,366,745 | 2,147,688 | git+https://github.com/badlogic/pi-mono.git |
| @vaclav-synacek/pi-coding-agent-termux | 0.51.1-2 | 2026-02-03 | MIT | 718 | 2,663 | git+https://github.com/VaclavSynacek/pi-coding-agent-termux.git |
| @oh-my-pi/pi-coding-agent | 11.5.0 | 2026-02-06 | MIT | 4,656 | 17,932 | git+https://github.com/can1357/oh-my-pi.git |
| @oh-my-pi/pi-agent-core | 11.5.0 | 2026-02-06 | MIT | 4,736 | 16,855 | git+https://github.com/can1357/oh-my-pi.git |
| pi-interactive-shell | 0.7.1 | 2026-02-03 | MIT | 650 | 2,638 | git+https://github.com/nicobailon/pi-interactive-shell.git |
| pi-web-access | 0.7.3 | 2026-02-06 | MIT | 761 | 908 | git+https://github.com/nicobailon/pi-web-access.git |
| pi-mcp-adapter | 2.1.2 | 2026-02-03 | MIT | 754 | 1,638 | git+https://github.com/nicobailon/pi-mcp-adapter.git |
| pi-powerline-footer | 0.2.22 | 2026-02-01 | MIT | 609 | 2,130 | git+https://github.com/nicobailon/pi-powerline-footer.git |
| pi-review-loop | 0.4.2 | 2026-02-02 | MIT | 402 | 887 | git+https://github.com/nicobailon/pi-review-loop.git |
| pi-messenger | 0.10.0 | 2026-02-06 | MIT | 425 | 502 | git+https://github.com/nicobailon/pi-messenger.git |
| pi-notify | 1.0.3 | 2026-02-03 | MIT | 525 | 629 | git+https://github.com/ferologics/pi-notify.git |
| @verioussmith/pi-openrouter | 1.1.0 | 2026-01-30 | MIT | 462 | 462 | git+https://github.com/verioussmith/pi-openrouter-extension.git |
| agentsbox | 0.1.3 | 2026-01-25 | MIT | 34 | 805 | git+https://github.com/assagman/agentsbox.git |

#### 其他值得关注的命中（需深入审查）

| Package | Version | Published | License | DL (wk) | DL (mo) | Repo |
|---|---:|---:|---|---:|---:|---|
| pi-package-test | 0.1.6 | 2026-01-25 | MIT | 44 | 682 | git+https://github.com/badlogic/pi-package-test.git |
| shitty-extensions | 1.0.9 | 2026-02-02 | MIT | 318 | 1,094 | git+https://github.com/hjanuschka/shitty-extensions.git |
| pi-acp | 0.0.14 | 2026-01-14 | MIT | 56 | 228 | git+https://github.com/svkozak/pi-acp.git |
| pi-subdir-context | 1.0.1 | 2026-01-29 | MIT | 31 | 181 | git+https://github.com/default-anton/pi-subdir-context.git |
| pi-screenshots-picker | 1.1.9 | 2026-02-04 | MIT | 2,455 | 2,455 | git+https://github.com/Graffioh/pi-screenshots-picker.git |
| @qualisero/pi-agent-scip | 0.3.0 | 2026-01-10 | Apache-2.0 | 17 | 196 | git+https://github.com/qualisero/pi-agent-scip.git |
| @imsus/pi-extension-minimax-coding-plan-mcp | 1.0.1 | 2026-01-29 | MIT | 38 | 165 | git+https://github.com/imsus/pi-extension-minimax-coding-plan-mcp.git |
| mitsupi | 1.1.1 | 2026-01-30 |  | 305 | 825 | git+https://github.com/mitsuhiko/agent-stuff.git |
| pi-extensions | 0.1.21 | 2026-02-05 | MIT | 426 | 1,213 |  |

#### 初筛短名单（初轮）

- **纳入（分发 / 参考）：** `@mariozechner/*` 包（官方 pi-mono 分发）。
- **纳入（社区分发）：** `@oh-my-pi/*`（替代分发；对真实世界兼容性测试有用）。
- **高信号“扩展包”候选：** `pi-web-access`、`pi-mcp-adapter`、`pi-interactive-shell`、`pi-review-loop`、`pi-powerline-footer`、`pi-messenger`、`pi-notify`。
- **提供方风格的扩展候选：** `@verioussmith/pi-openrouter`。
- **集成候选：** `agentsbox`（安装/桥接 Pi 扩展；本文档前文已引用）。
- **待调查 / 除非找到仓库链接否则可能排除：** `pi-extensions`（registry 元数据中无关联的仓库/主页）。

### E) 集市生态（OpenClaw / ClawHub）—— 已研究 2026‑02‑06（bd‑2m6d）

**状态：已研究。** 已识别权威来源。已评估兼容性。见下文。

#### 权威来源

| Resource | URL | Type |
|----------|-----|------|
| OpenClaw GitHub org | https://github.com/openclaw | Organization |
| OpenClaw main repo | https://github.com/openclaw/openclaw | Source (167k stars) |
| ClawHub registry repo | https://github.com/openclaw/clawhub | Registry source |
| ClawHub website | https://clawhub.ai/ | SPA (TanStack Start + Convex) |
| ClawHub docs | https://docs.openclaw.ai/tools/clawhub | Documentation |
| Skills archive | https://github.com/openclaw/skills | All ClawHub skills backup |
| Awesome list | https://github.com/VoltAgent/awesome-openclaw-skills | 1,715+ curated skills |

#### API 端点（机器可读）

ClawHub v1 REST API（由 Convex 驱动）：

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/skills` | GET | List skills (supports `sort=installs,trending,stars,newest,name`; max 200 items) |
| `/api/v1/skills` | POST | Publish skill (multipart) |
| `/api/v1/skills/{slug}` | GET | Fetch skill metadata + version info |
| `/api/v1/stars/{slug}` | POST/DELETE | Idempotent star management |

搜索使用 OpenAI 嵌入（`text-embedding-3-small`）+ Convex 向量搜索。
CLI：`clawhub search "query"` / `clawhub install <slug>` / `clawhub sync`

**适用速率限制。** 认证操作需要 GitHub OAuth。
v1 端点的 OpenAPI 规范可用。

#### 规模

- ClawHub registry 中**总计 3,000+ 技能**（截至 2026-02-02）
- awesome-openclaw-skills 清单中**精选 1,715+**（按质量过滤）
- **30+ 分类**：Web/Frontend (46)、Coding Agents/IDEs (55)、DevOps/Cloud (144)、AI/LLMs (159)、Search/Research (148)、CLI Utils (88)、Marketing/Sales (94)、Productivity (93)、Communication (58)、Smart Home/IoT (50)，另有 15+ 更多
- **技能归档仓库**（`openclaw/skills`）：Python 43.9%、JS 25.6%、Shell 11.6%、TS 11.2%

#### 关系：Pi ↔ OpenClaw

据 Armin Ronacher 的分析（https://lucumr.pocoo.org/2026/1/31/pi/）：
> "What's under the hood of OpenClaw is a little coding agent called Pi."

OpenClaw 通过 RPC 模式将 Pi 用作其底层编码智能体运行时。Pi 的扩展系统（`ExtensionAPI`、`registerTool`、`registerProvider` 等）是 OpenClaw 智能体用于代码执行的基础。

OpenClaw 在此基础上通过自有的**插件架构**（4 种类型：channels、tools、providers、memory）进行扩展，将 Pi 智能体封装在 Gateway WebSocket 控制平面中。OpenClaw 插件在 `package.json` 清单中使用 `openclaw.extensions`，并从 `extensions/*` 工作区目录中发现。

#### 兼容性评估

**ClawHub 技能（SKILL.md 文本包）与 Pi 技能：**

| Aspect | Pi Skills | ClawHub Skills | Compatible? |
|--------|-----------|----------------|-------------|
| File format | SKILL.md (YAML frontmatter + markdown) | SKILL.md (metadata + markdown) | PARTIAL |
| Frontmatter | YAML: `name`, `description`, `disable-model-invocation` | Table/YAML: `metadata.clawdbot.secrets`, `nix.plugin` | NEEDS NORMALIZATION |
| Body content | Markdown instructions/prompts | Markdown instructions/prompts | YES (direct) |
| Load path | `~/.pi/agent/skills/*/SKILL.md` | `~/.openclaw/skills/*/SKILL.md` | TRIVIAL REMAP |
| Invocation | `/skill:name` | Automatic (agent discovers and loads) | COMPATIBLE |

**SKILL.md 兼容性结论：**
- ClawHub 技能的 markdown 正文可直接作为 Pi 技能内容使用
- Frontmatter 元数据不同：ClawHub 使用带 secrets/config 的 `metadata.clawdbot` 命名空间；Pi 使用带 `name`/`description`/`disable-model-invocation` 的扁平 YAML
- 对 ClawHub 特有元数据进行剥离并映射 `name`/`description` 字段的规范化器，可使约 90% 的技能直接兼容

**OpenClaw 代码扩展与 Pi 代码扩展：**

| Aspect | Pi Extensions | OpenClaw Plugins | Compatible? |
|--------|--------------|------------------|-------------|
| API import | `import { ExtensionAPI } from "@mariozechner/pi-coding-agent"` | `openclaw.extensions` manifest in package.json | NO (different APIs) |
| Registration | `pi.registerTool()`, `pi.registerProvider()`, etc. | Gateway plugin lifecycle (discovery/validation/loading/init/runtime) | NO |
| Runtime | QuickJS/WASM in pi_agent_rust | Node.js process in OpenClaw Gateway | NO |
| Tool calls | Pi tool registry | OpenClaw Gateway tool routing | STRUCTURAL OVERLAP |

**代码扩展兼容性结论：**
- **OpenClaw 使用与 Pi 的 `ExtensionAPI` 不同的扩展 API**
- OpenClaw 的 4 类插件系统（channels/tools/providers/memory）在结构上与 Pi 的扩展类型有重叠，但注册机制不同
- 若无大量桥接工作，兼容层不可行
- **建议：对于“openclaw”层级，仅关注 SKILL.md 技能**

#### 候选分类

对于主目录流水线：

- **“真正的 Pi 扩展”候选**：基于文本的 SKILL.md 包中包含指令/提示的 ClawHub 技能（3,000+ 中的估计 2,500+）
- **“non-extension” 分组**：OpenClaw 专属插件（渠道集成、Gateway 工具、内存后端），使用 `openclaw.extensions` 清单——与 Pi 协议不兼容
- **已排除**：被标记为恶意的技能（Koi Security 检出 341 个）、存在凭据泄露的技能（Snyk/Evo 扫描器检出 283 个）

#### 安全注意事项

ClawHub 市场已出现重大安全事件（截至 2026 年 2 月）：
- **341 个恶意技能**由 Koi Security 识别（“ClawHavoc” 活动，通过 Atomic Stealer 针对 macOS）
- **283 个存在凭据泄露的技能**（占注册表的 7.1%，数据来自 Snyk/Evo Agent Security Analyzer）
- ClawHub 允许任何注册满 1 周的 GitHub 账户发布技能
- **建议**：执行严格的溯源校验；仅纳入来自精选 awesome 清单或具有已验证发布者历史的技能

#### 可复现查询

```bash
# 1. Enumerate ClawHub skills via REST API (trending, paginated)
curl -s "https://clawhub.ai/api/v1/skills?sort=installs&limit=200" > openclaw_trending_$(date +%Y%m%d).json

# 2. Clone the skills archive (all versions of all published skills)
git clone --depth 1 https://github.com/openclaw/skills.git openclaw_skills_archive/

# 3. Clone the curated awesome list
git clone --depth 1 https://github.com/VoltAgent/awesome-openclaw-skills.git

# 4. Enumerate via clawhub CLI (requires npm install)
npx clawhub@latest search --json "" > clawhub_search_all.json
```

#### 下游数据

本清单将供给：
- **bd-28ov**（校验与去重候选）：已就绪待分类的 openclaw 层级候选
- **bd-hhzv**（构建候选扩展清单）：以 openclaw 市场作为发现来源
- **bd-250p**（许可证与策略筛查）：安全发现要求对 openclaw 层级进行额外审查

### F) 精选清单与交叉引用挖掘（提及）

目标：发现被其他扩展作者引用的“隐藏”扩展。

建议的查询：

- GitHub 仓库搜索：`awesome "pi agent"` / `awesome buildwithpi` / `awesome pi-mono`
- 在已发现仓库中进行 GitHub 代码搜索：`pi extension`、`buildwithpi`、`pi-mono`、`registerTool(`、`registerCommand(`
- 在 `pi-mono` 与 buildwithpi 仓库中搜索 Issues/PR，关键词为 “extension”、“packages”、“marketplace”

### 噪声说明（实用过滤器）

- 查询 `pi extension` 通常过于宽泛；请添加锚点（`buildwithpi`、`pi-mono`、`registerTool(`）。
- 优先使用特征搜索（`registerTool(` / `registerCommand(` / `registerProvider(`）以减少误报。
- 当 GitHub 搜索结果噪声较大时，按语言过滤（优先 TS/JS），并按最近更新时间筛选。

---

## 候选元数据字段

- **Name/Path**：扩展名称或目录。
- **Source**：来源（examples、gist、npm、git）。
- **Type**：类型，file、package directory、gist、npm package。
- **Interaction Model**：交互模型，tool、斜杠命令、event hook、提供方、仅 UI 或混合。
- **Capabilities (likely)**：能力（推测），`read` / `write` / `exec` / `http` / `env`（根据描述近似推断）。
- **I/O Pattern**：I/O 模式，FS‑heavy、network‑heavy、CPU‑heavy 或 UI‑centric。
- **Last update**：来自来源清单（如可获取）；否则为 TBD。
- **Popularity score**：0‑100 分（见下方评分细则）。
- **Popularity evidence**：支撑分数的链接/指标（stars、下载量、文档提及）。
- **Compatibility status**：兼容性状态，`unmodified` / `modified` / `blocked`（见下方要求）。
- **Compatibility notes**：当状态非 `unmodified` 时的简短原因。
- **Notes**：纳入理由简述。

> Capabilities 为**基于描述推断**。后续可通过静态扫描进一步细化。

---

## 选择评分与覆盖目标（bd‑3o8d）

本评分细则定义了**如何对 Tier‑1/Tier‑2 语料的候选进行评分与分层**。
它在流行度的基础上扩展了**活跃度、兼容性与可靠性风险**。完整细节
见 `docs/EXTENSION_POPULARITY_CRITERIA.md`；此处为面向选择的摘要。

### 选择分数（基础 0–100 + 风险扣分）

**Base score = Popularity (30) + Adoption (15) + Coverage (20) + Activity (15) + Compatibility (20).**  
**Final score = Base score – Risk penalty (0–15).**

| Dimension | Points | How to Score |
|---|---:|---|
| **Popularity** | 0‑30 | Visibility: stars/forks, buildwithpi listings, npm downloads, curated mentions. |
| **Adoption** | 0‑15 | Evidence of real usage: docs/examples, references in multiple repos. |
| **Coverage** | 0‑20 | Unique surface area: interaction tags + capability diversity. |
| **Activity** | 0‑15 | Recency: ≤30d=15, ≤90d=12, ≤180d=9, ≤365d=6, ≤730d=3. |
| **Compatibility** | 0‑20 | Unmodified readiness: 20 (clean), 15 (needs generic shims), 10 (depends on incomplete generic runtime), 0 (blocked). |
| **Risk penalty** | 0‑15 | Subtract for high‑risk: OAuth‑heavy, native deps, non‑determinism, unclear license. |

分层（依据 `docs/EXTENSION_POPULARITY_CRITERIA.md`）：
- **Tier‑1**：通过门禁且 **final score ≥ 70**
- **Tier‑2**：通过门禁且 **final score ≥ 50**
- **已排除**：未通过任一门禁或 final score < 50  
官方 pi‑mono 示例**始终纳入**。

### 证据来源（非穷举）

- buildwithpi 包清单及安装次数（如已公开）
- GitHub stars/forks 及仓库活跃度
- Gist stars/forks 及最后更新时间
- npm 下载统计（周/月）
- 官方文档、示例或社区帖子中的提及

### Unmodified 兼容性要求

**Unmodified** 指扩展通过通用 `extc` 流水线运行，且**无需针对单个扩展的源码修改**、**无需特例运行时垫片**。可接受的转换为：

- 确定性打包/压缩/TS→JS 编译
- 通用导入重写（例如 `node:*` → `pi:node/*`）
- Pi 提供的通用 polyfills/垫片（例如 `pi:node/fs`、`process.env`、`Buffer`）
- 通过清单或环境变量进行配置
- 确定性测试桩（VCR/network stubs），**无需**修改扩展源码

**不允许**（将导致候选被归为 `modified` 或 `blocked`）：

- 编辑扩展源码以移除/替换 API
- 针对单个扩展的兼容性补丁或定制垫片
- Node/Bun 运行时依赖或原生 addon
- 无法通过通用重写处理的动态 `require`/`eval` 模式

**状态定义**

- `unmodified`：可通过通用流水线加载、注册并至少执行一个场景
- `modified`：需要针对单个扩展的修改或定制垫片
- `blocked`：依赖无法安全垫片的不受支持/不安全 API

### 覆盖目标（Tier‑1 必过语料）

覆盖目标以 `EXTENSIONS.md`（§1C.5）为准。摘要如下：

- **Tier‑0 基线**：官方 pi‑mono 示例（必过）。
- **Tier‑1 必过**：**≥ 200** 个 unmodified 扩展，按来源层级与行为分组进行分层。
- **Tier‑2 扩展**：为独特 API 覆盖面而选择的长尾增量（非按流行度）。

**Tier‑1 按来源层级的最低数量（初始框架）：**
`official-pi-mono` 60, `npm-registry` 50, `community` 50, `third-party-github` 20,
`agents-mikeastock` all available.

**行为/能力配额（最低要求）：**
包含所有注册了提供方的扩展及 exec‑heavy 扩展；≥80 个 event hooks；≥60 个工具注册；≥25 个斜杠命令；≥15 个 overlay‑heavy UI；≥40 个 UI 集成；≥25 个 network‑heavy；≥50 个 FS‑heavy；≥50 个 session/UI‑heavy 合计。

### 机器可消费的选择输出（必需）

选择输出必须为**机器可消费**，以便采集与一致性可在无需手工衔接的情况下运行。每个入选候选应包含：

- 稳定 ID + 固定来源（repo SHA / npm 版本 / gist 修订）
- Tier（`tier‑0|tier‑1|tier‑2`）+ 分数拆解（基础分 + 风险扣分）
- 兼容性状态（`unmodified|required_shims|blocked`）+ 理由
- 覆盖标签（运行时层级、交互标签、能力）

---

## A) pi‑mono 示例扩展（本地快照）

**生命周期与安全**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `permission-gate.ts` | pi‑mono examples | file | event hook + UI | exec? | UI‑centric | Confirm dangerous bash commands. |
| `protected-paths.ts` | pi‑mono examples | file | event hook | write | FS‑heavy | Blocks writes to protected paths. |
| `confirm-destructive.ts` | pi‑mono examples | file | command + UI | env? | UI‑centric | Confirms destructive session actions. |
| `dirty-repo-guard.ts` | pi‑mono examples | file | event hook | exec | FS‑heavy | Prevents changes when git dirty. |
| `sandbox/` | pi‑mono examples | dir | tool hook + runtime | exec | FS/OS | OS‑level sandboxing. |

**自定义工具**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `todo.ts` | pi‑mono examples | file | tool + command + UI | write | FS‑heavy | Todo tool + `/todos` with persistence. |
| `hello.ts` | pi‑mono examples | file | tool | none | UI‑centric | Minimal custom tool example. |
| `question.ts` | pi‑mono examples | file | tool + UI | env? | UI‑centric | `ctx.ui.select()` example. |
| `questionnaire.ts` | pi‑mono examples | file | tool + UI | env? | UI‑centric | Multi‑question UI flow. |
| `tool-override.ts` | pi‑mono examples | file | tool override | read/write | FS‑heavy | Wrap built‑ins for logging/ACL. |
| `truncated-tool.ts` | pi‑mono examples | file | tool | exec | FS‑heavy | Wrap ripgrep with truncation. |
| `antigravity-image-gen.ts` | pi‑mono examples | file | tool | http/write | network‑heavy | Image generation via HTTP. |
| `ssh.ts` | pi‑mono examples | file | tool | exec/http | network‑heavy | Delegate tools over SSH. |
| `subagent/` | pi‑mono examples | dir | tool + process | exec | CPU/FS | Delegates tasks to subagents. |

**命令与 UI**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `preset.ts` | pi‑mono examples | file | command | env | UI‑centric | Model/tool preset switching. |
| `plan-mode/` | pi‑mono examples | dir | command + UI | read | UI‑centric | Plan mode workflow. |
| `tools.ts` | pi‑mono examples | file | command + UI | env | UI‑centric | `/tools` enable/disable. |
| `handoff.ts` | pi‑mono examples | file | command | write | FS‑heavy | Handoff to new session. |
| `qna.ts` | pi‑mono examples | file | command + UI | env | UI‑centric | Extracts questions into editor. |
| `status-line.ts` | pi‑mono examples | file | UI | env | UI‑centric | Status updates. |
| `widget-placement.ts` | pi‑mono examples | file | UI | env | UI‑centric | Widget placement demo. |
| `model-status.ts` | pi‑mono examples | file | event hook + UI | env | UI‑centric | Model change status bar. |
| `snake.ts` | pi‑mono examples | file | UI | env | CPU/UI | Game w/ keyboard input. |
| `space-invaders.ts` | pi‑mono examples | file | UI | env | CPU/UI | Game w/ custom UI. |
| `send-user-message.ts` | pi‑mono examples | file | command | env | UI‑centric | Send user messages from extension. |
| `timed-confirm.ts` | pi‑mono examples | file | UI | env | UI‑centric | Abortable confirm/select dialogs. |
| `rpc-demo.ts` | pi‑mono examples | file | UI + RPC | env | UI‑centric | Exercises RPC UI methods. |
| `modal-editor.ts` | pi‑mono examples | file | UI | env | UI‑centric | Custom modal editor. |
| `rainbow-editor.ts` | pi‑mono examples | file | UI | env | UI‑centric | Animated editor content. |
| `notify.ts` | pi‑mono examples | file | UI | exec | OS‑heavy | Desktop notifications via OSC. |
| `titlebar-spinner.ts` | pi‑mono examples | file | UI | env | UI‑centric | Titlebar spinner animation. |
| `summarize.ts` | pi‑mono examples | file | command + tool | http | network‑heavy | Summarize with model call. |
| `custom-footer.ts` | pi‑mono examples | file | UI | env | UI‑centric | Footer customization. |
| `custom-header.ts` | pi‑mono examples | file | UI | env | UI‑centric | Header customization. |
| `overlay-test.ts` | pi‑mono examples | file | UI | env | UI‑centric | Overlay compositing tests. |
| `overlay-qa-tests.ts` | pi‑mono examples | file | UI | env | UI‑centric | Overlay QA suite. |
| `doom-overlay/` | pi‑mono examples | dir | UI | exec? | CPU/UI | Doom overlay @ 35 FPS. |
| `shutdown-command.ts` | pi‑mono examples | file | command | env | UI‑centric | `/quit` via `ctx.shutdown()`. |
| `interactive-shell.ts` | pi‑mono examples | file | event hook | exec | OS‑heavy | Interactive commands. |
| `inline-bash.ts` | pi‑mono examples | file | input transform | exec | OS‑heavy | `!{command}` expansion. |
| `bash-spawn-hook.ts` | pi‑mono examples | file | event hook | exec | OS‑heavy | Spawn hook for bash. |
| `input-transform.ts` | pi‑mono examples | file | event hook | env | UI‑centric | Input transformation. |
| `system-prompt-header.ts` | pi‑mono examples | file | prompt | env | UI‑centric | System prompt header. |

**Git 集成**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `git-checkpoint.ts` | pi‑mono examples | file | event hook | exec | FS‑heavy | Git stash checkpoints. |
| `auto-commit-on-exit.ts` | pi‑mono examples | file | lifecycle hook | exec | FS‑heavy | Auto‑commit on exit. |

**系统提示与压缩**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `pirate.ts` | pi‑mono examples | file | prompt | env | UI‑centric | `systemPromptAppend`. |
| `claude-rules.ts` | pi‑mono examples | file | prompt | read | FS‑heavy | Read `.claude/rules/`. |
| `custom-compaction.ts` | pi‑mono examples | file | compaction hook | env | UI‑centric | Custom compaction. |
| `trigger-compact.ts` | pi‑mono examples | file | command | env | UI‑centric | Trigger compaction on size. |

**系统集成 / 资源 / 消息**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `mac-system-theme.ts` | pi‑mono examples | file | system integration | env | OS‑heavy | Sync theme with macOS. |
| `dynamic-resources/` | pi‑mono examples | dir | resource hook | read | FS‑heavy | `resources_discover`. |
| `message-renderer.ts` | pi‑mono examples | file | UI | env | UI‑centric | Custom message renderer. |
| `event-bus.ts` | pi‑mono examples | file | event hook | env | UI‑centric | Inter‑extension events. |
| `session-name.ts` | pi‑mono examples | file | session hook | env | UI‑centric | Set session name. |
| `bookmark.ts` | pi‑mono examples | file | session hook | env | UI‑centric | Bookmark entries. |

**自定义提供方**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `custom-provider-anthropic/` | pi‑mono examples | dir | provider | http | network‑heavy | Custom provider w/ OAuth. |
| `custom-provider-gitlab-duo/` | pi‑mono examples | dir | provider | http | network‑heavy | Provider via proxy. |
| `custom-provider-qwen-cli/` | pi‑mono examples | dir | provider | exec/http | network‑heavy | Qwen CLI provider. |

**外部依赖**

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `with-deps/` | pi‑mono examples | dir | mixed | read/write | FS‑heavy | Package.json + deps. |
| `file-trigger.ts` | pi‑mono examples | file | event hook | read | FS‑heavy | Watches trigger file. |

---

## B) GitHub Gists（badlogic）

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `diff.ts` | https://gist.github.com/badlogic/679b221a1749353a5be3f3134c120685 | gist | command + UI | exec | FS‑heavy | `/diff` command w/ UI; last active 2026‑01‑23. |
| `review-extension-v3.ts` | https://gist.github.com/badlogic/30aef35d686483ffce22cc2aad99f3ff | gist | command + session ops | write | FS‑heavy | `/review` branch‑from‑root; created 2026‑01‑16; other versions exist (v2/v1/corrected). |

---

## B2) 社区 GitHub Gists

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `terminal-title.ts` | https://gist.github.com/nicobailon/ee8a65353b9103ad5d149e7eeb452b10 | gist | event hook + UI | env | UI‑centric | Terminal tab title/status extension; created 2026‑01‑15. |
| `claude-style.ts` | https://gist.github.com/aadishv/7615082df075519d6efd9de793aa860a | gist | UI | env | UI‑centric | Claude‑style UI tweaks; created 2026‑01‑25. |

---

## C) 仓库本地 `.pi/extensions`（遗留 pi-mono）

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `.pi/extensions/diff.ts` | pi‑mono `.pi` | file | command + UI | exec | FS‑heavy | Local diff UI extension. |
| `.pi/extensions/files.ts` | pi‑mono `.pi` | file | command + UI | read | FS‑heavy | File browser helper. |
| `.pi/extensions/prompt-url-widget.ts` | pi‑mono `.pi` | file | UI | http | network‑heavy | URL preview widget. |
| `.pi/extensions/redraws.ts` | pi‑mono `.pi` | file | UI | env | UI‑centric | UI redraw debugging. |

---

## D) 社区 / npm / Git 包

| Name/Path | Source | Type | Interaction Model | Capabilities (likely) | I/O Pattern | Notes |
|---|---|---|---|---|---|---|
| `agentsbox` | npm (agentsbox) | npm pkg | tool + MCP bridge | exec/http | network‑heavy | Installs a pi extension via `agentsbox setup pi`. |
| `pi-doom` | buildwithpi example | git pkg | UI overlay | exec | CPU/UI | Example git package install for pi (from official docs). |

---

## E) 扩展研究（bd-hhzv，2026-02-07）

### 研究流水线

四条研究流已通过确定性校验流水线（`src/extension_validation.rs`）执行并合并：

| Source | Queries | Raw Candidates | Validated Extensions |
|--------|---------|----------------|---------------------|
| GitHub Code Search (bd-3l39) | 10 | 479 repos, 290 inspected | 189 true extensions |
| GitHub Repo Search (bd-kgmr) | 12 | 136 repos | 7 new (beyond code search) |
| npm Registry Scan (bd-2p71) | 10 | 134 packages, 68 triaged | 33 validated packages |
| Curated Lists Sweep (bd-3gly) | 10 sources | 45 candidates | 22 new candidates |

### 去重与分类结果（bd-28ov）

- **输入候选总数**：498（来自所有来源及现有池）
- **去重后**：346
- **真实扩展**：331
- **仅提及**：1
- **未知**：14
- **分类覆盖率**：96.0%（超过 95% 阈值）
- **已合并来源**：38 个跨来源候选

### Tier 分布

| Tier | Count |
|------|-------|
| third-party-github | 200 |
| official-pi-mono | 60 |
| npm-registry | 53 |
| extensions (curated) | 14 |
| skills (curated) | 2 |
| community | 1 |
| agents-mikeastock | 1 |

### 最常用的注册 API

| API | Count |
|-----|-------|
| ExtensionAPI_import | 282 |
| export_default | 272 |
| registerCommand | 102 |
| registerTool | 83 |
| registerShortcut | 21 |
| registerProvider | 17 |
| registerFlag | 14 |
| registerMessageRenderer | 3 |

### 关键数据文件

- `docs/extension-validated-dedup.json` — 完整已校验且去重后的清单（346 个候选）
- `docs/extension-individual-enumeration.json` — 331 个真实扩展及其能力
- `docs/extension-code-search-inventory.json` — 来自 GitHub 代码搜索的 189 个已校验项
- `docs/extension-repo-search-summary.json` — 来自 GitHub 仓库搜索的 7 个新增项
- `docs/extension-npm-scan-summary.json` — 33 个已校验的 npm 包
- `docs/extension-curated-list-summary.json` — 来自精选清单的 22 个新增候选

### 流水线二进制

```bash
cargo run --bin ext_validate_dedup -- \
  --code-search docs/extension-code-search-inventory.json \
  --repo-search docs/extension-repo-search-summary.json \
  --npm-scan docs/extension-npm-scan-summary.json \
  --curated-list docs/extension-curated-list-summary.json \
  --candidate-pool docs/extension-candidate-pool.json \
  --out docs/extension-validated-dedup.json \
  --log-out /tmp/validation-decisions.jsonl
```

---

## F) 说明与后续步骤

1. **静态能力扫描**：解析每个候选以提取精确的宿主调用使用情况。
2. **丰富元数据**：在存在的地方补充 package.json 名称/版本。
3. **抽样矩阵**：以本清单作为 `bd-22h` 分层选择的输入。
4. **许可证筛查**：将已校验清单输入 bd-250p（许可证与策略筛查）。
5. **评分与选择**：输入 bd-34io（分类并评分候选；挑选分层语料）。

