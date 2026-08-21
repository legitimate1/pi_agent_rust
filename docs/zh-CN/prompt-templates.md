# 提示模板

提示模板允许你定义带参数的可复用提示。

## 位置

模板为从以下位置加载的 Markdown 文件：

1. **全局**：`~/.pi/agent/prompts/*.md`
2. **项目**：`.pi/prompts/*.md`
3. **包**：已安装包也可提供模板。

## 文件格式

模板为 Markdown 文件，可选含 YAML frontmatter。

```markdown
---
description: "Review code for security issues"
---
Review the following code for security vulnerabilities. Focus on XSS and SQL injection.

Code:
$1
```

若省略 description，则使用文件的第一行。文件名（不含扩展名）成为命令名（例如 `review.md` -> `/review`）。

## 调用

使用 `/` 加名称调用模板：

```bash
/review src/main.rs
```

参数按空格分割；单引号或双引号可将参数保持在一起：

```bash
/review "src/main.rs src/lib.rs" --strict
```

## 展开语法

模板支持类 bash 变量展开：

| 变量 | 描述 |
|----------|-------------|
| `$1`、`$2`…… | 位置参数 |
| `$@`、`$ARGUMENTS` | 以空格连接的所有参数 |
| `${@:N}` | 从索引 N 开始的参数（1 起始） |
| `${@:N:L}` | 从 N 开始的长度为 L 的参数切片 |

缺失的位置参数展开为空字符串。

### 示例：提交信息

`commit.md`：
```markdown
Write a commit message for the following changes.
Context: $1
Diff:
${@:2}
```

用法：
```bash
/commit "Refactor auth" src/auth.rs src/main.rs
```

展开为：
```
Write a commit message for the following changes.
Context: Refactor auth
Diff:
src/auth.rs src/main.rs
```
