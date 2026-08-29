# Hub 最小移植执行计划 — 2026-08-29

> 输入：`probe-report-2026-08-29.md` (§6) + `fork-merge-sop.md`。决策：**不 `merge upstream/main`**，单文件 `git show` 移植 hub 闭包。

## 1. 目标 / 非目标

- 目标：让 `custom` 可用上游 hub 史诗 (`bd-cv653`) 的最小闭环 — `hub roster/jobs` 调度，不追全量 1411 commits。
- 非目标：不升 `Cargo.toml` 大版本 (`asupersync 0.3.9 / rust 1.85 / digest 0.10` 保持 `--ours`)，不碰 `src/extensions/*` 重构域，不改 `tools/*` 拆分结构，不搬 `tiktoken/pprof/ftui` 可选依赖。

## 2. 移植闭包 (8+4 文件, 约 9k 行)

| 类 | 文件 | 来源 | 动作 |
|---|---|---|---|
| 必选 4 | `src/hub.rs` `src/jobs.rs` `src/agent_hub.rs` `src/subagents.rs` | `upstream/main` | `git show >` 全量覆盖 |
| 注入点 4 | `src/app.rs` `src/cli.rs` `src/config.rs` `src/rpc.rs` | `custom` 为主 | 植入 hub 钩子 (Tier1 已定位, 仅 `hub roster/jobs` 最小钩子) |
| 可选 | `src/checkpoint.rs` `src/handoff.rs` `src/github.rs` `src/btw.rs` | 按需 | 若 hub 编译依赖则一并搬, 否则冻结 |
| 适配层 1 | `src/tools/mod.rs` 或新建 `src/tools/shim.rs` | 新写 | `Tool::execute 4→5 参` shim: `fn adapt(abort: Option<AbortSignal>)` 不改 20+ impl |
| Cargo | `Cargo.toml` | `custom --ours` | 仅 `cargo add portable-pty fsqlite globset` 3 个 crate |

冻结：`tests/*` `docs/*` `.beads/*` `src/extensions/*` `benches/*` `examples/*` 全 `--ours` / `rm --cached`。

## 3. 执行步骤 (worktree 隔离, 3 步)

```pwsh
# 0. 基线校验 (主仓, <30s)
cargo check --lib 2>&1 | Select-String "error\["
# 1. 起隔离 worktree
git worktree add --detach /tmp/hub-port custom
Push-Location /tmp/hub-port
# 2. 搬必选 4 (全量覆盖, 不解冲突)
foreach($f in @("src/hub.rs","src/jobs.rs","src/agent_hub.rs","src/subagents.rs")){
  git show upstream/main:$f > $f; git add $f
}
# 可选按需同法; 注入点 4 手工打最小钩子 (参考 probe §3.1 Tier1, 丢弃上游大段只留 hub 初始化)
# 3. 适配层 + Cargo + 验证
cargo add portable-pty --optional 2>$null; cargo add fsqlite globset
cargo check --lib  # 目标 0 error, 替代 --all-targets 省 30GB
```

破裂治法：`>1500 行` 大文件若丢 `}` 则 `git show upstream/main:src/xxx > src/xxx` 重盖 (probe §3.1 教训)。

## 4. 验证门禁

- `cargo check --lib` 0 error / `grep -r "<<<<<<<" src/` 0
- `cargo clippy --lib -- -D warnings` + `cargo fmt --check` (日常门禁, 不跑 `--all-targets`)
- 单测：`cargo test --lib` 或 `cargo test --test <单文件>` 针对性跑
- 上游 hub 冒烟：`hub roster / jobs` 命令可列空表即通

## 5. 落地 / 回退

- 过门禁 → `git commit -m "port: hub minimal closure (hub/jobs/agent_hub/subagents + shim)"` → 主仓 `git merge --ff-only` → `git push origin custom` → `git worktree remove /tmp/hub-port --force`
- 失败回退：`git worktree remove --force` 即弃, `custom` 零改动 (`ae3f4b78` 基线, 见 probe §1)

## 6. 预算

- Token：`5-8k` (vs 全量合流 30k)；改动面 `8-12` 文件 vs `40+`；规避 `hostcall/touched_files/asupersync` 重构区正面撞。
- 时间：`hub 4` 覆盖 <5min, 注入点+shim <30min, 不触发 `my-check.yml` 云端全量。

> 下一步：执行 §3 步骤 0→1，已就绪可直接 `worktree` 开干。
