# 同步策略

## 真实来源
- 主源：JSONL
- 原因：与遗留行为保持一致，并保留人类可读/对 Git 友好的会话。SQLite 是用于快速查找与搜索的派生索引。

## 同步触发
- 会话保存时：`AgentSession::persist_new_messages` 更新 SQLite 索引
- 手动重建：`SessionIndex::reindex_all()`（已规划的 CLI 命令）
- 定时器/节流：无后台定时器；索引在会话保存时增量更新

## 版本控制
- 数据库标记：`meta.last_sync_epoch_ms`，按会话的 `sessions.last_mtime_ms` + `last_size_bytes`
- JSONL 标记：文件 `mtime` + `size`（文件系统元数据）

## 并发
- 锁文件路径：`~/.pi/agent/session-index.lock`
- 忙等待超时：5 秒（SQLite busy timeout）

## 失败处理
- 数据库被锁定：通过 busy timeout 重试，然后抛出清晰错误并保持以 JSONL 为权威
- JSONL 解析错误：跳过对该文件的索引，报告错误，允许手动修复
- Git 提交错误：不适用（无自动 git 操作）

## 已规划的 CLI 助手
- `pi sessions reindex` — 从 JSONL 重建 SQLite 索引
- `pi sessions export-jsonl` — 导出索引元数据以便检查
