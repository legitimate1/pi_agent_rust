# Cross-Platform CI Matrix — WINDOWS

> Generated: 2026-08-07T11:41:06Z
> OS: windows / x86_64
> Required checks: 5/5 passed

## Check Results

| Check | Policy | Status | Tag |
|-------|--------|--------|-----|
| Cargo check compiles | required | PASS | - |
| Test infrastructure functional | required | PASS | - |
| Temp directory writable | required | PASS | - |
| Git CLI available | required | PASS | - |
| Conformance artifacts present | informational | PASS | - |
| E2E TUI test support (tmux) | informational | UNSUPPORTED | platform-unsupported |
| POSIX file permission support | informational | UNSUPPORTED | platform-unsupported |
| Extension test artifacts present | informational | PASS | - |
| Evidence bundle index present | informational | PASS | - |
| Suite classification file present and valid | required | PASS | - |

## Merge Policy

| Platform | Role |
|----------|------|
| Linux | **Required** — all required checks must pass |
| macOS | Informational — failures logged, not blocking |
| Windows | Informational — failures logged, not blocking |

## Platform-Specific Issues

- **E2E TUI test support (tmux)** (unsupported): tmux not available on Windows
- **POSIX file permission support** (unsupported): POSIX permissions not available on Windows

