<p align="center">
  <img src="pi_agent_rust_illustration.webp" alt="Pi Agent Rust" width="600"/>
</p>

<h1 align="center">pi_agent_rust</h1>

<p align="center">
  <strong>High-performance AI coding agent CLI written in Rust</strong>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> •
  <a href="#features">Features</a> •
  <a href="#commands">Commands</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#extensions--security">Extensions &amp; Security</a> •
  <a href="#troubleshooting">Troubleshooting</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-2024%20edition-orange?logo=rust" alt="Rust 2024">
  <img src="https://img.shields.io/badge/license-MIT%20%2B%20Rider-blue" alt="License: MIT + Rider">
  <img src="https://img.shields.io/badge/unsafe-forbidden-brightgreen" alt="No Unsafe Code">
</p>

```bash
# Install latest release
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | bash
```

---

## The Problem

Existing AI coding assistants are **slow to start** (Node/Python runtimes add 500ms+), **memory hungry**, **unreliable** (broken streaming, corrupted sessions), and **hard to extend**.

## The Solution

**pi_agent_rust** is a from-scratch Rust port of [Pi Agent](https://github.com/badlogic/pi) by Mario Zechner (made with his blessing). Single static binary, instant startup (<100ms), stable streaming, and 9 built-in tools.

It builds on two purpose-built Rust libraries:

- **[asupersync](https://github.com/Dicklesworthstone/asupersync)** — structured concurrency async runtime with built-in HTTP/TLS/SQLite
- **[rich_rust](https://github.com/Dicklesworthstone/rich_rust)** — Rust port of [Rich](https://github.com/Textualize/rich), terminal output with markup syntax

```bash
# Start a session
pi "Help me refactor this function to use async/await"

# Continue a previous session
pi --continue

# Single-shot mode (ephemeral by default; opt-in session with --session-dir/--session)
pi -p "What does this error mean?" < error.log
```

## Quick Start

### 1. Install

```bash
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/pi_agent_rust/main/install.sh?$(date +%s)" | bash
```

If you already have the original TypeScript `pi` installed, the installer asks whether to make Rust Pi canonical as `pi` and preserves the old command as `legacy-pi`.

### 2. Configure API Key

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### 3. Run

```bash
pi                                   # Interactive mode
pi "Explain this codebase structure" # With an initial message
pi @src/main.rs "What does this do?" # Read files as context
```

### Using a local model

`ollama`, `llamacpp`, `mistralrs`, and `lmstudio` are built-in **local** providers — no API key needed, work out of the box against their default ports:

```bash
pi --provider ollama    --model llama3        -p "hi"
pi --provider llamacpp  --model <gguf-repo-id> -p "hi"
pi --provider mistralrs --model default        -p "hi"
```

Point any other OpenAI-compatible server via `~/.pi/agent/models.json` (see [docs/models.md](docs/models.md) for the full schema).

## Features

- **Streaming responses** with extended thinking (levels: `off`, `minimal`, `low`, `medium`, `high`, `xhigh`)

| Tool                               | Description                                                                                        |
| ---------------------------------- | -------------------------------------------------------------------------------------------------- |
| `read`                             | Read file contents (text + images), head/tail/info/diff, encoding auto-detect                      |
| `write` / `edit` / `hashline_edit` | Create, surgical replace (LINE#HASH anchors for precision), optional post-edit syntax verification |
| `bash` / `pwsh`                    | Execute shell commands with timeout + process-tree cleanup                                         |
| `grep` / `find` / `ls`             | Search content / discover files / list directories (respects `.gitignore`)                         |

All tools: automatic truncation (2000 lines / 1MB), detailed metadata, process-group cleanup (no orphaned processes).

- **Session management** — JSONL tree sessions with branching, `--continue`, `--resume`, `--no-session`, automatic compaction for long conversations, SQLite session index + V2 sidecar for fast resume at scale; print mode persists sessions on demand via `--session-dir`/`--session` (success and failure alike, for unattended task diagnostics)
- **Autocomplete** — `@` file references and `/` slash commands with fuzzy scoring; background re-index every 30s
- **Skills & prompt templates** — `SKILL.md` under `~/.pi/agent/skills/` or `.pi/skills/` invoked via `/skill:name`; templates via `/<name>` with positional args; share as packages (`pi install npm:@org/pi-packages`)
- **Credentials** — API keys, OAuth, AWS credential chains, bearer tokens stored in `~/.pi/agent/auth.json`; `pi config` shows per-provider status

### Three Execution Modes

| Mode            | Invocation      | Use Case                                                                                                                              |
| --------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| **Interactive** | `pi` (default)  | Full TUI: streaming, tools, session branching, autocomplete, model selector (`Ctrl+L`), `/tree` branch navigator                      |
| **Print**       | `pi -p "..."`   | Single response to stdout, scriptable                                                                                                 |
| **RPC**         | `pi --mode rpc` | Line-delimited JSON protocol over stdin/stdout for IDE integrations (`prompt`, `steer`, `follow_up`, `abort`, `get_state`, `compact`) |

## Commands

```bash
pi [OPTIONS] [MESSAGE]...
```

Key options: `-c/--continue` · `-r/--resume` · `--session <PATH>` · `--no-session` · `-p/--print` · `--mode text|json|rpc` · `--provider <NAME>` · `--model <MODEL>` · `--thinking <LEVEL>` · `--tools <LIST>` · `--extension-policy safe|balanced|permissive` · `--list-models` · `--list-providers` · `--export <PATH>`

Subcommands: `install` / `remove` / `update` / `list` (packages) · `config` · `update-index` / `search` / `info` (extension catalog) · `doctor` (environment diagnostics) · `migrate` (JSONL → V2 session store) · `swarm-progress` (read-only SLO evaluation)

## Configuration

Settings live in `~/.pi/agent/settings.json` (project-level `.pi/settings.json` overrides global):

```json
{
  "default_provider": "anthropic",
  "default_model": "claude-opus-4-5",
  "default_thinking_level": "medium",
  "compaction": {
    "enabled": true,
    "reserve_tokens": 8192,
    "keep_recent_tokens": 20000
  },
  "retry": {
    "enabled": true,
    "max_retries": 3,
    "base_delay_ms": 1000,
    "max_delay_ms": 30000
  },
  "shell_path": "/bin/bash",
  "shell_command_prefix": "set -e"
}
```

**Precedence** (first match wins): CLI flags → environment variables → project settings → global settings → built-in defaults.

**Resources** (skills/templates/themes/extensions) resolve in order: CLI paths → project dir (`.pi/...`) → global dir (`~/.pi/agent/...`) → installed packages.

**Environment variables**: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GOOGLE_API_KEY`, `AZURE_OPENAI_API_KEY`, `COHERE_API_KEY`, plus OpenAI-compatible keys (Groq, OpenRouter, Mistral, DeepSeek, Together, xAI, Perplexity, Moonshot, DashScope, Cerebras, DeepInfra, Fireworks) and `PI_CONFIG_PATH`, `PI_CODING_AGENT_DIR`, `PI_PACKAGE_DIR`, `PI_SESSIONS_DIR`.

## Extensions & Security

Pi supports two extension runtime families with capability-gated host connectors:

- **JS/TS** entrypoints run in an embedded QuickJS runtime — no Node/Bun required (Node API shims for `fs`, `path`, `os`, `crypto`, etc.)
- **`*.native.json`** descriptors run in the native-rust descriptor runtime

Security is first-class: every hostcall (`tool`/`exec`/`http`/`session`/`ui`/`env`/`log`) is checked against the active capability policy and audited. `exec` calls get command-level mediation that blocks dangerous shell patterns (recursive delete, disk/device writes, reverse shell) before spawn. Extensions have an explicit trust lifecycle (`pending` → `acknowledged` → `trusted` → `killed`) with kill-switch audit logs.

```bash
pi --extension-policy safe|balanced|permissive   # Capability profile
pi --explain-extension-policy                     # Inspect effective decisions
```

See [docs/planning/EXTENSIONS.md](docs/planning/EXTENSIONS.md) for the full architecture and [docs/extension-catalog.json](docs/extension-catalog.json) for the extension catalog.

## Troubleshooting

| Symptom               | Fix                                                                             |
| --------------------- | ------------------------------------------------------------------------------- |
| `fd not found`        | Install `fd` (`apt install fd-find` / `brew install fd`); may be named `fdfind` |
| `API key not set`     | `export ANTHROPIC_API_KEY=...` or `pi --api-key ...`                            |
| Session corrupted     | Start fresh: `pi --no-session`, or delete the JSONL session file                |
| Streaming hangs       | Check network; `curl -N https://api.anthropic.com/v1/messages`                  |
| Tool output truncated | Intentional (2000 lines / 1MB); ask for specific ranges with offset/limit       |

Full guide: [docs/troubleshooting.md](docs/troubleshooting.md)

## FAQ

**Q: What's the relationship to the original Pi Agent?**
A: An authorized Rust port of [Pi Agent](https://github.com/badlogic/pi), built on asupersync + rich_rust. Same UX, idiomatic Rust, drastically faster startup.

**Q: Which providers are supported?**
A: 12 native provider modules (Anthropic, OpenAI Chat/Responses/Codex, Gemini, Cohere, Azure OpenAI, Bedrock, Vertex AI, GitHub Copilot, GitLab Duo, Cursor, model catalog fetch) + many OpenAI-compatible presets + local models (ollama/llamacpp/mistralrs/lmstudio). Run `pi --list-providers` for canonical IDs.

**Q: How do sessions work?**
A: JSONL v3 files with parent references for branching, compaction metadata, a SQLite session index sidecar for fast resume, and an optional V2 segmented-log sidecar for large histories. SQLite-backed storage is available via the default-enabled `sqlite-sessions` feature.

**Q: Why is unsafe forbidden?**
A: Memory safety is non-negotiable for a tool that executes arbitrary commands. The performance cost is negligible here.

**Q: Can I add a custom provider?**
A: Yes. Add a `models.json` entry (model ID, base URL, API type — usually `openai-completions`) in `~/.pi/agent/` or `.pi/`; works with vLLM, Ollama, LiteLLM, etc.

## Development

```bash
cargo build --release                    # Build (binary at target/release/pi)
./scripts/cargo_headroom.sh test --all-targets  # Full tests with disk preflight
./scripts/e2e/run_all.sh --profile ci    # Unified verification runner
```

- Rust 2024 **nightly** required (`rust-toolchain.toml`)
- `#![forbid(unsafe_code)]` project-wide; release profile: `opt-level=3` + thin LTO + `panic=abort` + `strip`
- Default features: `sqlite-sessions` + `tui`; `--features full` adds image-resize/jemalloc/clipboard/wasm-host/syntax-highlighting
- Releases are tag-driven (`vX.Y.Z` must match `Cargo.toml` version); publish order: `asupersync` → `rich_rust` → `charmed-*` → `pi_agent_rust`

For contributors and maintainers: development workflow, full command reference, architecture notes, and debugging playbooks live in `AGENTS.md` and `docs/context/` (`features.md`, `architecture.md`, `commands.md`, `conventions.md`, `design-decisions.md`, `debugging.md`). Extended docs index: [docs/](docs/) — session, tree, TUI, RPC, SDK, settings, models, providers, extension architecture, security, and swarm operations runbooks.

## License

MIT License (with OpenAI/Anthropic Rider). See [LICENSE](LICENSE) for details.
