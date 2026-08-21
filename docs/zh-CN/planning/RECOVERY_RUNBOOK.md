# 恢复手册

## 症状
- 检测到数据库损坏
- JSONL 解析失败
- 版本标记不匹配

## 步骤
1. 获取锁
2. 校验可信源
3. 重建目标存储
4. 更新版本标记
5. 校验计数/哈希
6. 释放锁

## 命令（规划中）
- `pi sessions reindex`（从 JSONL 重建 SQLite 索引）
- `pi sessions export-jsonl`（导出已索引元数据以供检查）
