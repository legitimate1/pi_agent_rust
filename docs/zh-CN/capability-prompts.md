# 能力提示 (UX 规格)

本文档定义扩展能力提示面向用户的体验：用户看到什么、可以做什么决策、决策如何持久化，以及流程在交互式 TUI 与无头 RPC 模式下如何工作。

本规格旨在与以下内容保持一致：
- `EXTENSIONS.md`（能力分类 + 策略模式 + 宿主调用 ABI）
- `src/permissions.rs`（当前持久化形态；需为作用域扩展）
- `docs/rpc.md` (JSONL RPC 请求/响应/事件帧)

## 目标

- 在不让用户不堪重负的前提下提供知情同意。
- 避免提示轰炸：在安全且有意义时批量请求。
- 使决策可审计（结构化日志、稳定的关联 id）。
- 确保无头模式下的确定性行为（超时、默认值）。

## 非目标

- 实现能力策略引擎本身（本文为 UX + 交互规格）。
- 允许扩展绘制任意 UI（UI 由核心拥有）。

## 术语

- **能力**: 策略键，如 `read`、`write`、`exec`、`http`、`session`、`ui`、`tool`。该键必须与核心为宿主调用派生的能力一致（见 `EXTENSIONS.md` §3.2）。
- **作用域**: 被访问的具体资源（路径/主机/命令等）。
- **宿主调用**: 扩展到核心的特权请求（`host_call` / `host_result`）。
- **提示**: 向用户询问能力决策的可见提问。
- **决策**: 允许/拒绝结果及其持久化语义。

## 涵盖的宿主调用类型

本规格涵盖以下连接器方法的能力提示：

| 方法 | 典型能力 | 向用户展示的作用域 |
|------|----------|---------------------|
| `tool` | 由工具名派生 (`read`/`write`/`exec`/`tool`) | 工具名 + 关键参数 (路径/模式等) |
| `exec` | `exec` | 命令 + 参数（按需脱敏） |
| `http` | `http` | 协议 + 主机 + 方法 + 路径（查询默认脱敏） |
| `session` | `session` | 操作名（如 `get_messages`、`fork`、`compact`） |
| `ui` | `ui` | 操作名（如 `confirm`、`input`、`select`、`editor`） |

## 风险分级量表

风险以简单、一致的指示器展示。分级是确定性的。

### 按能力的基础风险

| 能力 | 风险 | 原因 |
|------|------|------|
| `exec` | 高 | 任意进程执行 |
| `write` | 高 | 文件系统变更 / 数据丢失 |
| `http` | 中 | 数据外泄 + 网络副作用 |
| `read` | 中 | 密钥泄露（取决于作用域） |
| `tool` | 中 | 未知/自定义工具（在 strict/prompt 模式下强制提示/拒绝） |
| `session` | 低 | 仅 Pi 内部元数据 |
| `ui` | 低 | 仅用户交互（但在无头模式下可能被拒绝） |

### 作用域升级因子（可选、确定性）

作用域只能提升风险，绝不会降低。

- `read` 若路径匹配常见密钥模式则升级为高: `.env`、`**/*secret*`、`**/*token*`、`**/.ssh/**`、`**/*credentials*`。
- `http` 若主机不在扩展声明/推断的主机作用域内则升级为高。
- `exec` 始终为高。

若作用域为动态/未知（非字面量），则将作用域显示为 `"<dynamic>"` 并对该能力升级至更高风险等级。

## 提示内容规格

每条提示必须包含：

- **扩展身份**:
  - 显示名称
  - `extension_id`
  - 版本（若已知）
  - 来源（npm/GitHub/本地路径，若已知）
- **请求的能力**:
  - 能力键（如 `http`）
  - 人类可读描述（如 "Network access"）
- **作用域摘要**（默认脱敏）:
  - 路径 / 主机 / 命令 / 操作
  - 展示简短预览及“详情”开关
- **风险指示器**:
  - `LOW`、`MEDIUM`、`HIGH` 标签
  - 简短、具体的理由（"exec can run arbitrary commands"）
- **批量摘要**（若为批量）:
  - 待处理请求数量
  - 最多 3 个代表性作用域列表，随后 `+N more`
- **决策选项**（见下文）

### 脱敏规则（提示 UI）

提示 UI 默认不得显示可能的密钥。

- 对于 `exec`: 不显示完整环境，并对参数中的令牌做脱敏。
- 对于 `http`: 显示主机 + 路径；默认隐藏查询字符串，除非用户展开详情。
- 对于 `read`/`write`: 显示规范化的相对路径；不内联文件内容。

## 决策选项

提示提供一小组固定的决策：

1. **Allow Once（仅允许一次）**
   - 仅适用于本次宿主调用（单个 `call_id`）。
2. **Allow For Session（在本次会话内允许）**
   - 仅适用于当前 Pi 会话（会话结束时清除）。
3. **Allow Always（始终允许）**
   - 持久化决策（见持久化模型）。
4. **Deny（拒绝）**
   - 拒绝本次宿主调用。
5. **Deny Always（始终拒绝）**
   - 持久化的拒绝决策（见持久化模型）。

说明：
- 在 `extensions.policy.mode=strict` 下，通常不显示“提示” UI；没有允许规则的请求将被拒绝。若用户显式触发“授权”流程（未来），UI 仍可能被使用。
- 在 `extensions.policy.mode=permissive` 下，默认应抑制提示并记录决策；可能存在“始终提示”的调试覆盖。

## 批量/分组规则

批量在保留知情同意的同时减少骚扰。

### 提示键

请求按以下键批量：

- `extension_id`
- `capability`
- `risk_level`
- `scope_group`（方法特定的规范化；示例见下）

绝不跨以下维度批量：

- 不同能力
- 不同风险等级
- 不同扩展

### 作用域分组（确定性）

- `read`/`write`（工具或 fs）：按规范化路径中的顶级目录分组（如 `src/**`、`tests/**`）。若路径为 `"<dynamic>"`，则自成一组。
- `http`: 按主机分组（如 `api.github.com`）。若主机未知/动态，则自成一组。
- `exec`: 不跨不同命令二进制（首个 argv 标记）分组。
- `session`/`ui`: 按操作名分组。

### 批量窗口

- 收集请求的时间窗口较短：自首个入队请求起 `250ms`，随后一次性提示批量。
- 当提示已显示时，与同一提示键匹配的额外请求将追加到批次（计数实时更新），但 UI 应避免滚动；仅显示计数 + 简短代表列表。

### 批量算法（伪代码）

```text
on_hostcall_request(req):
  cap = derive_capability(req.method, req.params)
  decision = lookup_cached_or_persisted(ext_id, cap, scope)
  if decision == ALLOW: dispatch
  if decision == DENY: return denied

  if policy.mode == strict: return denied
  if policy.mode == permissive: dispatch (log policy.decision=allow_permissive)

  // prompt 模式:
  key = PromptKey(ext_id, cap, risk(cap, scope), scope_group(scope))
  enqueue pending[key].push(req)
  if no prompt scheduled for key:
    schedule after 250ms: show_prompt(key)

show_prompt(key):
  prompt_id = new_id()
  display prompt for pending[key]
  wait for decision (TUI or RPC), timeout 30s -> deny
  apply decision to pending[key] requests
  clear pending[key]
```

## 持久化模型

### 目标键

持久化决策按键为：

`(extension_id, capability, scope_pattern, version_range?) -> decision`

其中：

- 对于无作用域的能力（exec/session/ui），`scope_pattern` 可选。
- `version_range` 为可选的 semver 约束（扩展变更时重新提示）。

### 作用域模式语义

- 路径：相对于项目根/cwd 的类 glob 模式（与 `EXTENSIONS.md` 能力清单语义相同）。
- 主机：精确主机或后缀匹配（`api.github.com`、`*.github.com`）。
- Exec：可选的命令前缀 glob（`git *`、`rg *`）。

### 存储

当前实现 (`src/permissions.rs`) 持久化 `(extension_id, capability)`。为满足本规格，需扩展为包含 `scope_pattern`，并将无作用域条目视为该能力的通配。

## 交互式 TUI 线框图

ASCII 模拟（模态覆盖层）：

```text
┌──────────────────────────────────────────────────────────────────────┐
│ Extension Permission Request                                          │
├──────────────────────────────────────────────────────────────────────┤
│ Extension:  Auto Commit on Exit  (ext.auto_commit)  v1.2.0           │
│ Source:     npm:@user/pi-auto-commit                                 │
│                                                                      │
│ Requested:  EXEC  (High Risk)                                        │
│ Reason:     exec can run arbitrary commands on your machine          │
│                                                                      │
│ Scope:      git commit -am "<redacted>"                              │
│ Batch:      3 pending exec requests (showing 1/3)                    │
│            - git status                                              │
│            - git commit ...                                          │
│            - +1 more                                                 │
│                                                                      │
│ [A] Allow once   [S] Allow for session   [Y] Allow always            │
│ [D] Deny         [N] Deny always         [?] Details / help          │
└──────────────────────────────────────────────────────────────────────┘
```

交互规则：
- 默认焦点位于权限最小的正向选项（`Allow once`）。
- `?` 展开详情面板（完整规范化参数、脱敏说明、链接）。
- 可选显示可见倒计时；即使不显示也会强制执行超时。

## 无头 / RPC 模式

在 RPC 模式下，提示作为服务端发送事件投递，必须由显式的客户端命令应答。

### 事件: `capability_prompt`

```json
{
  "type": "capability_prompt",
  "data": {
    "promptId": "perm-123",
    "callIds": ["host-7", "host-8"],
    "extension": {
      "id": "ext.auto_commit",
      "name": "Auto Commit on Exit",
      "version": "1.2.0",
      "source": { "type": "npm", "ref": "@user/pi-auto-commit" }
    },
    "capability": "exec",
    "method": "exec",
    "risk": "high",
    "scopes": [
      { "kind": "command", "summary": "git status" },
      { "kind": "command", "summary": "git commit ..." }
    ],
    "options": ["allow_once", "allow_session", "allow_always", "deny_once", "deny_always"],
    "timeoutMs": 30000
  }
}
```

### 命令: `capability_decision`

客户端响应：

```json
{
  "id": "req-55",
  "type": "capability_decision",
  "promptId": "perm-123",
  "decision": "allow_once"
}
```

服务端以标准响应信封回复 (`docs/rpc.md`)：

```json
{
  "id": "req-55",
  "type": "response",
  "command": "capability_decision",
  "success": true,
  "data": null,
  "error": null
}
```

### 超时行为

- 若在 `timeoutMs`（默认 30s）内未收到决策，Pi 必须：
  - 拒绝所有待处理的 `callIds`，
  - 以 `decision="deny_timeout"` 发出 `policy.decision` 日志。

## 日志要求

每次提示解决必须发出结构化日志事件（`pi.ext.log.v1`）：

- `event`: `policy.decision`
- `data` 字段：
  - `prompt_id`
  - `call_ids[]`
  - `extension_id`
  - `capability`
  - `decision` (`allow_once`、`allow_always`、`deny_always` 等)
  - `policy_mode` (`strict`、`prompt`、`permissive`)
  - `risk`
  - `scope_hashes[]` (规范化作用域摘要的 sha256；绝不含原始密钥)

## 待决问题（显式）

- exec 命令的精确作用域模式语言（glob vs 前缀 vs 正则）。
- 当作用域严格位于 `src/**` 内时 `read` 是否应默认为低（当前：中）。
- 在交互式模式下 `ui` 能力是否应自动允许（当前：与其他能力一样可提示）。
