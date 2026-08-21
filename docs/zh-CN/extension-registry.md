# 扩展注册表与本地索引

本文档定义了 Pi 的 *离线优先* 扩展发现注册表以及面向用户的发现命令（`pi search`、`pi info`）和刷新命令（`pi update-index`）所使用的本地磁盘索引。

关键目标：
- 无中心服务器：数据来自公共后端（npm + GitHub）以及随 Pi 二进制文件一起交付的精选种子数据。
- 离线优先：发现功能必须在首次运行时无需网络访问即可工作。
- 故障开放：网络故障绝不会破坏搜索或安装；Pi 始终回退到种子/缓存数据。

## 概念

### 索引

**索引** 是包含扩展描述符列表及元数据的单一 JSON 文件，适用于本地（客户端）搜索。

默认位置：
- `~/.pi/agent/extension-index.json`
- 可通过 `PI_EXTENSION_INDEX_PATH` 覆盖

### 种子索引（内置）

Pi 在编译时将 **种子索引** 嵌入二进制文件。它提供：
- 首次运行时即时返回结果（无需“请等待”）
- 有用的离线体验
- 刷新失败或缓存损坏时的稳定回退

种子索引随每个 Pi 版本一起更新。

### 缓存（用户机器）

Pi 从远程源刷新后会将缓存的索引写入磁盘。当缓存有效时，Pi 优先使用缓存，但在出错时会透明地回退到种子索引。

## 数据源

Pi 将多个源合并为单一索引：

1. **npm 注册表搜索**
   - 查找带有 `pi-extension` / `pi-agent-extension` 等关键词的包。
   - 填充：包名、版本、描述、仓库 URL、最后发布时间。

2. **GitHub 搜索**
   - 按主题（例如 `topic:pi-extension`）和/或查询词搜索仓库。
   - 填充：仓库名称、描述、star 数、最后更新时间、仓库 URL。

3. **精选清单（由 Pi 维护）**
   - 已知可用扩展的静态列表（高信号、已测试、已固定版本）。
   - 这是随二进制文件一起交付的种子索引的主要内容。

## 模式：`pi.ext.index.v1`

`extension-index.json` 使用带版本的模式，以便未来的变更明确且可迁移。

示例：

```json
{
  "schema": "pi.ext.index.v1",
  "version": 1,
  "generatedAt": "2026-02-06T08:00:00Z",
  "lastRefreshedAt": "2026-02-06T08:00:00Z",
  "entries": [
    {
      "id": "npm/checkpoint-pi",
      "name": "checkpoint-pi",
      "description": "Checkpoint and restore your Pi sessions",
      "tags": ["npm", "extension"],
      "license": "MIT",
      "source": {
        "type": "npm",
        "package": "checkpoint-pi",
        "version": "1.2.3",
        "url": "https://www.npmjs.com/package/checkpoint-pi"
      },
      "installSource": "npm:checkpoint-pi@1.2.3"
    }
  ]
}
```

字段说明：
- `id`：索引内全局唯一标识符（稳定键）。
- `name`：主要显示标识符（通常为 npm 包名或仓库名）。
- `installSource`：与 Pi 包管理器兼容的可选字符串（例如 `npm:pkg@ver`、`git:https://github.com/org/repo@ref`）。如果缺失，该条目可被发现但无法通过 id 直接解析安装。

## 刷新策略

### 何时刷新

- 当缓存缺失或超过 24 小时未更新时自动刷新（在 store API 中可用；命令层面的装配可选择主动或惰性刷新行为）。
- 通过 `pi update-index` 手动刷新。

### 失败语义（关键）

刷新是 *尽力而为*：
- 网络错误绝不能导致发现命令失败。
- 如果刷新失败，Pi 将继续使用已缓存的索引（如果存在）或种子索引。

### 损坏处理

如果缓存文件无法解析：
- 警告（非致命）。
- 回退到种子索引。
- 在下次成功刷新时覆盖缓存。

## 搜索算法（客户端）

搜索在本地缓存数据上计算：
- 按空白字符对用户查询进行分词。
- 加权评分：
  - 名称匹配：权重最高
  - 标签匹配：中等
  - 描述匹配：较低
- 平局决胜：
  - 优先包含 `installSource` 的条目
  - 优先更高质量的信号（未来：一致性层级、star 数、下载量、时效性）

目标是在不引入重量级模糊匹配依赖的情况下实现“足够好”的相关性。

## 按 ID 安装解析

为便于使用，Pi 应支持：
- 对于 `installSource` 存在且匹配唯一的条目，支持 `pi install <id-or-name>`。

解析规则：
1. 对 `name` 的精确匹配（大小写不敏感）
2. 对 `id` 的精确匹配（大小写不敏感）
3. 提供方特定的别名（例如 `npm/<name>`）

如果多个条目匹配，Pi 应拒绝猜测，并指示用户传入显式的 `npm:` 或 `git:` 源字符串。

## 当前运行时装配

- `src/extension_index.rs` 实现了本地模式、内置种子加载、缓存陈旧检查、搜索评分、id/name 安装源解析，以及针对 npm + GitHub 的远程刷新适配器。
- `src/config.rs` 通过 `PI_EXTENSION_INDEX_PATH` 覆盖提供 `Config::extension_index_path()`。
- `pi install`、`pi remove` 和 `pi update <source>` 现在在委托给包管理器操作之前，会通过本地索引解析简写的 id/name 别名。
- `pi update-index` 执行尽力而为的远程刷新，并将合并后的缓存写入本地 extension-index 路径。
