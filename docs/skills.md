# Skills

Skills provide specialized instructions and capabilities to the agent. They are defined using the [Agent Skills](https://github.com/Start-Agent-Skills) format.

## Locations

Skills are loaded from:

1. **Global**: `~/.pi/agent/skills/*/SKILL.md` or `~/.pi/agent/skills/*.md`
2. **Project**: `.pi/skills/*/SKILL.md` or `.pi/skills/*.md`
3. **Packages**: Installed packages.

## File Format

A skill is defined in a `SKILL.md` file with YAML frontmatter.

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

### Frontmatter Fields

| Field | Description |
|-------|-------------|
| `name` | Skill ID (must match directory name if in a subdir; mismatches emit a warning). |
| `description` | **Required.** Short description used for selection; empty descriptions are skipped. |
| `disable-model-invocation` | If `true`, the skill is not shown to the model in the system prompt and the model is explicitly prohibited from loading or using it automatically. Users can still invoke it via `/skill:name`. |

If `name` is omitted, the parent directory name is used.

## Usage

### Auto-Discovery (Model-Invoked)

By default, Pi includes all enabled skills in the system prompt. The model can decide to "activate" a skill by reading its definition file using the `read` tool.

**Hard constraint**: Skills with `disable-model-invocation: true` are excluded from the `<available_skills>` list in the system prompt, and the model is explicitly instructed **not** to load or use skills not present in that list. This prevents the model from automatically invoking skills that are intended for manual use only.

### Explicit Invocation

You can explicitly invoke a skill using the slash command:

```bash
/skill:sql-expert "Optimize this query..."
```

This effectively wraps your prompt with the skill's instructions.

## Configuration

To disable the `/skill:` slash commands, set `enable_skill_commands` to `false` in `settings.json`.
