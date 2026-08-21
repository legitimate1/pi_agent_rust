# 技能

技能为智能体提供专门的指令与能力。它们使用 [Agent Skills](https://github.com/Start-Agent-Skills) 格式定义。

## 位置

技能从以下位置加载：

1. **全局**：`~/.pi/agent/skills/*/SKILL.md` 或 `~/.pi/agent/skills/*.md`
2. **项目**：`.pi/skills/*/SKILL.md` 或 `.pi/skills/*.md`
3. **包**：已安装的包。

## 文件格式

技能在带有 YAML frontmatter 的 `SKILL.md` 文件中定义。

```markdown
---
name: "sql-expert"
description: "Expert at writing and optimizing SQL queries"
disable-model-invocation: false
---
You are an expert SQL developer. When writing queries:
1. Always prefer CTEs over subqueries.
2. Use uppercase for keywords.
3. Check for index usage.
```

### Frontmatter 字段

| 字段 | 说明 |
|------|------|
| `name` | 技能 ID（若在子目录中必须与目录名匹配；不匹配会发出警告）。 |
| `description` | **必填。** 用于选择的简短描述；空描述会被跳过。 |
| `disable-model-invocation` | 若为 `true`，则不会在系统提示中向模型展示该技能。 |

若省略 `name`，则使用父目录名。

## 使用

### 自动发现

默认情况下，Pi 会在系统提示中包含所有已启用的技能。模型可通过 `read` 工具读取其定义文件来“激活”某个技能。

### 显式调用

你可以使用斜杠命令显式调用技能：

```bash
/skill:sql-expert "Optimize this query..."
```

这实际上会用该技能的指令包裹你的提示。

## 配置

要禁用 `/skill:` 斜杠命令，请在 `settings.json` 中将 `enable_skill_commands` 设为 `false`。
