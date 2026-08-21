# 旧版 pi-mono 扩展运行器（已固定）

本文档固定了用于扩展参照捕获的**精确旧版环境**，并列出了复现运行的**命令**。

---

## 1) 仓库固定

**源仓库：** `https://github.com/badlogic/pi-mono`  
**本地供应快照：** `legacy_pi_mono_code/pi-mono/`  
**提交：** `df5b0f76c026b35fdd7f0fb78cb0dbaaf939c1b5`

验证：
```bash
cat legacy_pi_mono_code/pi-mono/PINNED_COMMIT
test -f legacy_pi_mono_code/pi-mono/package-lock.json
test -x legacy_pi_mono_code/pi-mono/pi-test.sh
test -f legacy_pi_mono_code/pi-mono/packages/coding-agent/src/cli.ts
```

该快照以源文件的形式供应，而非嵌套 Git 检出，因此 `git -C legacy_pi_mono_code/pi-mono rev-parse HEAD` 解析的是上层 Rust 仓库，并非有效的固定检查。

---

## 2) 运行时与依赖固定

**Node 引擎要求：** `>=20.0.0`（来自 `package.json`）。

**依赖锁：** `package-lock.json`（为可复现性请使用 `npm ci`）。

**工作区包：** `packages/*` 加上位于 `packages/coding-agent/examples/extensions/*` 的扩展示例。

安装/构建（从仓库根目录）：
```bash
cd legacy_pi_mono_code/pi-mono
npm ci
npm run build
```

> `npm run check` 需要先执行 `npm run build`。

---

## 3) 运行旧版 CLI（从源码）

便捷包装器：
```bash
./pi-test.sh
```

其执行：
```bash
npx tsx packages/coding-agent/src/cli.ts
```

**无环境变量模式**（清空 API 密钥以获得确定性测试）：
```bash
./pi-test.sh --no-env
```

---

## 4) 扩展加载（示例 + 本地）

**通过 CLI 加载单个扩展：**
```bash
./pi-test.sh --extension packages/coding-agent/examples/extensions/permission-gate.ts
```

**通过复制到扩展目录自动发现：**
```bash
cp packages/coding-agent/examples/extensions/permission-gate.ts ~/.pi/agent/extensions/
./pi-test.sh
```

**仓库本地扩展**（已存在）：
```
legacy_pi_mono_code/pi-mono/.pi/extensions/
```

---

## 5) Pi 包（npm 或 git）

安装带有扩展/技能/提示模板/主题的包：
```bash
./pi-test.sh install npm:@foo/pi-tools
./pi-test.sh install npm:@foo/pi-tools@1.2.3
./pi-test.sh install git:github.com/user/repo
./pi-test.sh install git:github.com/user/repo@v1
```

包安装到：
```
~/.pi/agent/git/   (git)
~/.pi/agent/npm/   (npm)
```

对于项目本地安装：
```bash
./pi-test.sh install -l npm:@foo/pi-tools
```

---

## 6) 基准捕获工作流（建议）

1. **确保固定：**检出精确提交并运行 `npm ci`。  
2. **选择扩展：**来自 `examples/extensions/` 或 `.pi/extensions/`。  
3. **以确定性环境运行：**除非需要 API 密钥，否则优先使用 `--no-env`。  
4. **记录输出：**捕获 stdout/stderr 以及所有会话 JSONL 输出。

示例捕获命令：
```bash
./pi-test.sh --no-env --extension packages/coding-agent/examples/extensions/permission-gate.ts
```

---

## 7) 备注

- 示例扩展列表及描述位于：  
  `packages/coding-agent/examples/extensions/README.md`
- 扩展文档见：  
  `packages/coding-agent/docs/extensions.md`
