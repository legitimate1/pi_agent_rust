# 包管理

Pi 支持安装提供扩展、技能、提示模板与主题的包。

## 源

Pi 支持三种包源类型：

1. **npm**：`npm:package-name` 或 `npm:@org/package`（可选 `@version`）
2. **git**：`git:host/owner/repo` 或仅 `https://github.com/owner/repo`（可选 `@ref`）
3. **local**：目录路径（例如 `../my-package`）

## 命令

### 安装

全局安装包（用户作用域）：
```bash
pi install npm:pi-skills
pi install git:github.com/someuser/my-tools
```

为当前项目本地安装：
```bash
pi install --local npm:@org/project-utils
```

这会将包添加到你的 `settings.json`（全局或项目）并安装它。

### 移除

移除包：
```bash
pi remove npm:pi-skills
pi remove --local npm:@org/project-utils
```

### 更新

更新所有包（或指定包）：
```bash
pi update
pi update npm:pi-skills
```

带固定版本的包（例如 `npm:pkg@1.2.3` 或 `git:repo@v1`）会被跳过，除非命令参数显式更改版本。

### 列表

列出已安装包：
```bash
pi list
```

## 资源发现

安装包时，Pi 在包根目录内的以下位置查找资源：

1. **清单**：若 `package.json` 含 `pi` 段，则使用其中定义的路径。
   ```json
   "pi": {
     "extensions": ["dist/extension.js"],
     "skills": ["skills/"],
     "prompts": ["prompts/"],
     "themes": ["themes/"]
   }
   ```

2. **约定**：若不存在清单条目，Pi 会查找标准目录：
   - `extensions/`（单文件扩展则为 `index.ts`/`index.js`）
   - `skills/`
   - `prompts/`
   - `themes/`

### 扩展清单回退策略（确定性、Fail-Closed）

对于扩展包根，解析是有意确定性的：

- 若 `package.json` 包含 `pi.extensions`，则仅考虑其中列出的条目。
- 若 `pi.extensions` 键以空数组（`[]`）形式存在，Pi 将从该包加载**零个**扩展。
- 若 `pi.extensions` 条目存在但均未解析到现有文件，Pi 将从该包加载**零个**扩展。
- 在以上两种情况下，Pi **不会**隐式回退到 `index.ts`、`index.js` 或目录级扩展加载。
- 约定回退（`extensions/`、`index.ts`、`index.js`）仅在 `pi.extensions` 键缺失时适用。

## 配置

你可以在 `settings.json` 中手动配置包：

```json
{
  "packages": [
    "npm:pi-skills",
    {
      "source": "git:github.com/org/repo",
      "skills": ["relevant-skill"],
      "extensions": [] 
    }
  ]
}
```

对象形式允许过滤加载哪些资源（白名单）。若省略某字段，则加载该类型的所有资源。若某字段存在但为空，则该类型**零个**资源被启用。

### 过滤模式

过滤值可以是字符串或数组。它们接受相对于包根的类 glob 模式，并带可选前缀：

- `pattern` → 包含匹配项（若存在任何包含模式）
- `!pattern` → 排除匹配项
- `+pattern` → **强制包含**（精确路径匹配）
- `-pattern` → **强制排除**（精确路径匹配）

示例：

```json
{
  "packages": [
    {
      "source": "git:github.com/org/repo",
      "extensions": ["dist/*.js", "!dist/experimental.js"],
      "skills": []
    }
  ]
}
```

## 确定性锁文件与溯源验证

Pi 在成功安装/更新验证后写入确定性包锁文件：

- 项目作用域：`.pi/packages.lock.json`
- 用户作用域：`~/.pi/agent/packages.lock.json`

锁条目按确定性排序并包含：

- `identity`（稳定包标识，例如 `npm:name`、`git:host/path`、`local:/abs/path`）
- `source` 与 `source_kind`
- 已解析溯源（npm 版本、git 提交/ref/来源或已解析本地路径）
- `digest_sha256`（确定性内容摘要）
- `trust_state`

### Fail-Closed 验证

默认情况下，当受信任溯源或摘要不匹配时，安装/更新为 fail-closed：

- 固定的 npm 安装必须匹配固定版本元数据
- 固定的 git 安装必须匹配固定的 ref/commit 解析
- 受信任摘要/溯源不匹配会阻止安装/更新

对于未固定的 `pi update`，允许溯源/摘要轮转并作为受信任更新重新记录。

### 信任转换审计制品

Pi 将信任状态转换作为 JSONL 审计事件追加：

- 项目作用域：`.pi/package-trust-audit.jsonl`
- 用户作用域：`~/.pi/agent/package-trust-audit.jsonl`

每个事件记录动作（`install`、`update`、`remove`）、作用域、标识、源、`from_state`、`to_state`、确定性原因码及可选修复指引。
