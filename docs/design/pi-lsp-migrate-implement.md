# IMPLEMENT.md — pi-lsp QuickJS 扩展迁移

## 1. 实现边界

**目标**：将 `@narumitw/pi-lsp`（narumiruna-pi-lsp/extensions/pi-lsp）从 Node.js TypeScript 扩展迁移为 pi_agent_rust 的 QuickJS 扩展。

**In scope**：
- 编译 TS 源代码为 JS（QuickJS 运行时不支持 TS 语法）
- 修复与 pi_agent_rust QuickJS 运行时不兼容的 3 个 API 使用点
- 将扩展部署到 pi_agent_rust 可加载的位置
- 注册到资源管理器

**Out of scope**：
- 不改 LSP 协议逻辑、不重构、不改适配器配置
- 不改 pi_agent_rust 的扩展运行时（`extensions_js.rs`）
- 不改原有 Node.js 兼容性（原地编译，不破坏上游）

**Assumptions**：
- tsc（TypeScript 编译器）在目标机器可用
- QuickJS 运行时支持 ES2020 标准（不支持 `#` 私有字段）
- `node:child_process.spawn` 的 `longLived: true` 路径满足 LspClient 的 stdin/stdout 双向通信需求

**Design delta**：
- `lsp-client.ts`：`#` 私有字段改为 `private` 关键字（TypeScript 编译时擦除）
- `lsp-client.ts`：`spawn()` 调用添加 `longLived: true`
- `pi-lsp.ts`：移除 `defineTool` 导入，直接构建对象传给 `pi.registerTool()`

---

## 2. 文件变更清单

| ID | 路径 | 操作 | 用途 | 主要改动 |
|:--:|:-----|:----|:-----|:---------|
| C1 | `pi-lsp/extensions/pi-lsp/tsconfig.json` | 修改 | 启用 JS 输出到 `dist/` | 加 `outDir: "dist"`, `noEmit: false`, `declaration: false` |
| C2 | `pi-lsp/extensions/pi-lsp/package.json` | 修改 | 更新扩展入口 | `pi.extensions` 指向 `dist/index.js` |
| C3 | `pi-lsp/extensions/pi-lsp/src/lsp-client.ts` | 修改 | QuickJS 兼容 | `#` 私有字段 → `private`；`spawn()` 加 `longLived: true` |
| C4 | `pi-lsp/extensions/pi-lsp/src/pi-lsp.ts` | 修改 | QuickJS 兼容 | 移除 `defineTool` 导入，直接传对象给 `registerTool` |
| C5 | `pi-lsp/extensions/pi-lsp/.gitignore` | 新建 | 忽略编译产物 | `dist/` |
| C6 | ~ | 新建 | 编译产物 | `tsc -p tsconfig.json` 生成 `dist/` 目录 |

---

## 3. 依赖关系

Dependencies:
- C1 → C2: 编译配置先改好，package.json 入口才能指向 dist/
- C3 → C6: 源码改好后再编译
- C4 → C6: 源码改好后再编译
- C5: 独立，任何时候都可做
- C6: 依赖 C3, C4 完成

并行分组建议：
- Phase 1：C1, C5, C4（可并行 — 配置和 pi-lsp.ts 修改无依赖）
- Phase 2：C3（需要理解 LspClient 结构，独立进行）
- Phase 3：C6（依赖 C3+C4 完成）
- Phase 4：C2（package.json 入口修改，需要 C6 产物路径确定后做）

---

## 4. 验证计划

自动化检查：
- `tsc -p tsconfig.json` 编译无错误
- 生成的 `dist/` 包含所有 JS 文件
- `cargo check`（在 pi_agent_rust 项目中无影响）

人工检查：
- 确认 dist/index.js 可被 pi_agent_rust 加载（扩展入口有效）
- 确认 `longLived: true` 已写入编译后的 lsp-client.js
