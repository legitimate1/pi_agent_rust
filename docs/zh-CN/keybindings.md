# 快捷键

Pi 在交互模式下支持可配置的快捷键。

## 配置

用户快捷键从以下位置加载：
`~/.pi/agent/keybindings.json`

### 格式

配置为一个 JSON 对象，将**动作 ID**（驼峰命名）映射到**按键字符串**（或字符串数组）。

```json
{
  "cursorUp": ["up", "ctrl+p"],
  "cursorDown": ["down", "ctrl+n"],
  "submit": "enter",
  "newLine": ["shift+enter", "ctrl+enter"]
}
```

使用空数组可完全移除默认绑定：

```json
{
  "cursorUp": []
}
```

### 按键语法

按键以 `modifier+key` 形式指定。

- **修饰键**：`ctrl`、`alt`、`shift`（以及如 `ctrl+shift` 的组合）。
- **按键**：
  - 字母：`a`、`b`、`c`……
  - 数字：`1`、`2`……
  - 功能键：`f1`–`f20`
  - 特殊键：`enter`、`escape`、`tab`、`space`、`backspace`、`delete`、`insert`、`clear`、`home`、`end`、`pageup`、`pagedown`、`up`、`down`、`left`、`right`
  - 符号：单字符按键如 `` ` ``、`-`、`=`、`[`、`]`、`\`、`;`、`'`、`,`、`.`、`/` 及其 shifted 变体（`!`、`@`、`#`、`$`、`%`、`^`、`&`、`*`、`(`、`)`、`_`、`+`、`{`、`}`、`|`、`:`、`"`、`<`、`>`、`?`）

**同义词**：
- `return` -> `enter`
- `esc` -> `escape`

## 动作与默认值

### 光标移动

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `cursorUp` | `up` | 光标上移 |
| `cursorDown` | `down` | 光标下移 |
| `cursorLeft` | `left`、`ctrl+b` | 光标左移 |
| `cursorRight` | `right`、`ctrl+f` | 光标右移 |
| `cursorWordLeft` | `alt+left`、`ctrl+left`、`alt+b` | 光标按词左移 |
| `cursorWordRight` | `alt+right`、`ctrl+right`、`alt+f` | 光标按词右移 |
| `cursorLineStart` | `home`、`ctrl+a` | 移至行首 |
| `cursorLineEnd` | `end`、`ctrl+e` | 移至行尾 |
| `jumpForward` | `ctrl+]` | 向前跳转到字符 |
| `jumpBackward` | `ctrl+alt+]` | 向后跳转到字符 |
| `pageUp` | `pageup` | 按页上滚 |
| `pageDown` | `pagedown` | 按页下滚 |

### 删除

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `deleteCharBackward` | `backspace` | 向后删除字符 |
| `deleteCharForward` | `delete`、`ctrl+d` | 向前删除字符 |
| `deleteWordBackward` | `ctrl+w`、`alt+backspace` | 向后删除单词 |
| `deleteWordForward` | `alt+d`、`alt+delete` | 向前删除单词 |
| `deleteToLineStart` | `ctrl+u` | 删除至行首 |
| `deleteToLineEnd` | `ctrl+k` | 删除至行尾 |

### 文本输入

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `newLine` | `shift+enter`、`ctrl+enter` | 插入新行 |
| `submit` | `enter` | 提交输入 |
| `tab` | `tab` | Tab / 自动补全 |

### 应用

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `interrupt` | `escape` | 取消 / 中止 |
| `clear` | `ctrl+c` | 清空编辑器（或取消选择） |
| `exit` | `ctrl+d` | 退出（编辑器为空时） |
| `suspend` | `ctrl+z` | 挂起到后台 |
| `externalEditor` | `ctrl+g` | 在外部编辑器中打开 |

### 剪贴板与 Kill Ring

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `copy` | `ctrl+c` | 复制选区 |
| `pasteImage` | `ctrl+v` | 从剪贴板粘贴图片 |
| `yank` | `ctrl+y` | 粘贴最近删除的文本 |
| `yankPop` | `alt+y` | 循环已删除文本 |
| `undo` | `ctrl+-` | 撤销上次编辑 |

### 模型与思考

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `selectModel` | `ctrl+l` | 打开模型选择器 |
| `cycleModelForward` | `ctrl+p` | 切换到下一个模型 |
| `cycleModelBackward` | `ctrl+shift+p` | 切换到上一个模型 |
| `cycleThinkingLevel` | `shift+tab` | 切换思考级别 |

### 显示与工具

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `expandTools` | `ctrl+o` | 折叠/展开工具输出 |
| `toggleThinking` | `ctrl+t` | 折叠/展开思考块 |

### 会话

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `newSession` | - | 开始新会话 |
| `tree` | - | 打开会话树导航器 |
| `fork` | - | 复刻当前会话 |

### 消息队列

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `followUp` | `alt+enter` | 排队后续消息 |
| `dequeue` | `alt+up` | 将已排队消息恢复到编辑器 |

### 选择（列表/选择器）

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `selectUp` | `up` | 选择上移 |
| `selectDown` | `down` | 选择下移 |
| `selectPageUp` | `pageup` | 列表中上翻页 |
| `selectPageDown` | `pagedown` | 列表中下翻页 |
| `selectConfirm` | `enter` | 确认选择 |
| `selectCancel` | `escape`、`ctrl+c` | 取消选择 |

### 会话选择器

| 动作 ID | 默认按键 | 描述 |
|-----------|--------------|-------------|
| `toggleSessionPath` | `ctrl+p` | 切换路径显示 |
| `toggleSessionSort` | `ctrl+s` | 切换排序模式 |
| `toggleSessionNamedFilter` | `ctrl+n` | 切换仅命名过滤 |
| `renameSession` | `ctrl+r` | 重命名会话 |
| `deleteSession` | `ctrl+d` | 删除会话 |
| `deleteSessionNoninvasive` | `ctrl+backspace` | 查询为空时删除会话 |

## 依赖上下文的冲突

部分按键被有意绑定到多个动作，并根据 UI 状态解析：

- `ctrl+c` 可表示**复制**（有选区时）、**清空**（编辑器中）或**中止**（运行时）。
- `ctrl+d` 在编辑器中为 **DeleteCharForward**，编辑器为空时为**退出**，在会话选择器内为 **DeleteSession**。
- `ctrl+p` 在编辑器中循环切换模型，但在选择器中切换会话路径显示。
