# 安全审查：scheduler.rs — IcyCompass

## 审查日期
2026-04-17

## 受检文件
`/data/projects/pi_agent_rust/src/scheduler.rs` — PiJS 运行时的确定性事件循环调度器

## 导入/依赖
- 被以下模块使用：`extension_dispatcher.rs`、`extensions_js.rs`、`hostcall_amac.rs`、`extensions.rs`
- 扩展运行时调度的核心组件

## 安全缺陷狩猎结果

针对以下模式进行了系统性安全审查：

### ✅ 1. 缺失的 saturating_add
**状态**：良好 — 已正确使用 `saturating_add()`
- **位置**：第 38 行 — `Self(self.0.saturating_add(1))`
- **分析**：序列计数器中正确的溢出防护

### ✅ 2. 对哈希/MAC 的裸 == 比较
**状态**：良好 — 未发现哈希比较
- **分析**：该调度器中无密码学操作

### ✅ 3. Fail-open 的 > 而非 >=
**状态**：良好 — 边界检查正确
- **位置**：第 471 行 — `if entry.deadline_ms > now`（定时器触发逻辑正确）
- **位置**：第 865 行 — `if self.queue.len() >= self.capacity`（容量检查正确）

### ✅ 4. 无界的 Vec::push
**状态**：良好 — 无无界增长
- **分析**：所有 push 操作均受调度器设计约束
- **位置**：定时器的 BinaryHeap::push()、任务的 VecDeque::push_back() — 均会定期处理

### ✅ 5. 用 let _ = 隐藏 Result
**状态**：良好 — 仅在测试代码中发现且已显式处理错误
- **分析**：测试代码使用带 `.expect()` 调用的 `let _ =`，属可接受用法

### ✅ 6. 在可失败操作上使用 .unwrap()
**状态**：良好 — 仅在合理场景下使用
- **位置**：第 475 行 — `.expect("peeked")` 安全（刚已验证条目存在）
- **分析**：其余 unwrap 均位于测试代码中

### ✅ 7. 路径遍历
**状态**：不适用 — 调度器中无文件路径操作

### ✅ 8. 未设防的 NaN/Inf 算术
**状态**：不适用 — 无浮点算术

## 交叉审查：最近提交（b3974d21）

### 文件：package_manager.rs
**变更**：当 settings.json 缺失时保留 lockfile 的修复

### 安全性分析
- **路径操作**：使用带硬编码路径的安全 `join()`，如 "keep-me"、".pi"
- **错误处理**：仅在测试代码中正确使用 `.expect()`
- **输入校验**：已具备广泛的路径遍历防护
- **通道使用**：`let _ = tx.send()` 模式可接受（错误在接收端处理）

### ✅ 在该提交中未发现安全问题

## 总体评估

` scheduler.rs` 文件与最近的 `package_manager.rs` 提交均体现出优秀的安全实践：

- 正确的算术溢出防护
- 恰当的错误处理模式
- 未使用不安全代码
- 良好的边界条件处理
- 强大的路径遍历防护（位于 package_manager 中）

**无需安全修复。**

## 构建验证
运行 `rch exec -- cargo check --all-targets && clippy && test` 以验证编译完整性。
