# Hub 最小移植执行计划 — 2026-08-29

> 输入：`probe-report-2026-08-29.md` (§6) + `fork-merge-sop.md`。决策：**不 `merge upstream/main`**，单文件 `git show` 移植 hub 闭包。
> **状态：✅ 已完成 2026-08-29 — A1 全闭包落 custom `4eeb3bf7`，`cargo check --lib` 44→0 / `clippy --lib -D warnings` 33→0 / `cargo fmt` pass，已 `push origin/custom`。**
> **实际闭包：`hub/jobs/agent_hub/subagents(8965行) + secrets(437)+worktree_iso(657) + Cargo 5增量 + tools 105行`，替代原计划 5-8k，实耗 8-10k，一次性拿齐 subagent 集群。**

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

## 7. 落地证据 (2026-08-29 17:00)

- 提交：`4eeb3bf7 port: hub full closure A1 (...)` (rebase 过 `b55139dd` 的同名 bump，已 `push origin/custom`)
- 隔离验证：`hub-port` (detached `ae3f4b78`) `cargo check --lib` 44→0 (33.92s) → `cargo clippy --lib -D warnings` 33→0 (0.62s) → `cargo fmt --check` pass → `cargo fmt` 收口 → 合回 `custom` 复验 `check 1m42s / clippy 36.59s` 双绿
- 改动：`+11258 -78`，14 files — `src/hub.rs(1134) + jobs.rs(5354) + agent_hub.rs(585) + subagents.rs(2515) + secrets.rs(437) + worktree_iso.rs(657)` + `Cargo.toml/lock + lib.rs + permissions.rs + session.rs + tools/mod.rs`
- Cargo 增量：`fs4 0.13→1.1`，`+portable-pty 0.9 / rustix 1.1.4 / win32job 2.0.3(windows) / jsonschema→[dependencies]`，`lib.rs +#![feature(windows_by_handle)]`
- 适配：`permissions.rs` upstream 回灌、`session.rs lock_exclusive→FileExt::lock`、`hub.rs String+&String→format!`、`subagents.rs execute 4→5 _abort`、`tools/mod.rs +attach/terminate 105行`
- 清理：`git worktree remove /tmp/hub-port --force` + `rm /tmp/up_tools.rs`，`hub-port` 已删，`custom` 零残留
- 下次：该闭包 3 个月内无需再动；如需 `hub roster/jobs` 注入点 (`app/cli/config/rpc`) 另起小移植，不再 `merge upstream/main`

