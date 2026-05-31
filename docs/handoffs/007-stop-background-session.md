# Handoff 007: Stop Background Session

Status: complete. A foreground switch deadlock regression was fixed again by restoring `clone_killer()` in `run_pty_command`.

## Goal

Implement command-mode support for stopping a running background session:

```text
stop <session-id>
```

This gives the user basic process control before adding interactive focus/panes.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/006-background-send.md`
- `src/main.rs`
- `src/pty.rs`
- `src/commands.rs`

## Current State

Implemented command-mode features:

```text
help
status
send <plan> <agent>
send <plan> <agent> --bg
```

Background sessions:

- are launched in a background PTY
- write `.agentmux/runs/<run-id>/transcript.ansi`
- write `.agentmux/runs/<run-id>/meta.txt`
- show up in `status`
- cannot currently be stopped

## Scope

Add:

```text
stop <session-id>
```

Do not implement:

- focus switching
- panes
- interactive stdin for background sessions
- approval detection
- inline `//`
- full TUI

## Required Behavior

### Stop Existing Session

If the session exists and is running:

```text
stopped <session-id>
```

The background process should be terminated.

Update in-memory status to:

```text
stopped
```

Update `.agentmux/runs/<run-id>/meta.txt` with:

```text
status: stopped
```

### Stop Unknown Session

If the session id does not exist:

```text
session not found: <session-id>
```

### Stop Finished Session

If the session has already exited or failed:

```text
session not running: <session-id>
```

Do not treat this as a hard error.

## Implementation Guidance

The current background session state may need to store a process killer handle.

`portable-pty` exposes:

```rust
child.clone_killer()
```

Use that rather than wrapping the child in a mutex while another thread waits on it.

Suggested status enum addition:

```rust
Stopped
```

Suggested session field:

```rust
killer: Option<Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>>
```

If storing the trait object in `commands.rs` is awkward, move background session state into a small `sessions.rs` module. Keep the public command parser simple and testable.

## Command Parsing

Update parser support for:

```text
stop <session-id>
```

Suggested enum:

```rust
pub enum WrapperCommandResult {
    Print(String),
    Send { plan: String, agent: String, background: bool },
    Stop { session_id: String },
}
```

Unit-test:

- `stop`
- `stop auth-agy-123`
- unknown command still works

## Status Output

`status` should show stopped sessions as:

```text
auth-agy-123 stopped .agentmux/runs/auth-agy-123/transcript.ansi
```

## Verification

Run:

```sh
cargo fmt
cargo build
cargo test
cargo run -- init
cargo run -- plan auth
cargo run -- version
cargo run -- gemini --version
```

Manual interactive check:

```sh
cargo run -- codex
```

Inside Codex:

1. Press `Ctrl-G`.
2. Type `send auth gemini --bg` and press Enter.
3. Press `Ctrl-G`.
4. Type `status` and copy the background session id.
5. Press `Ctrl-G`.
6. Type `stop <session-id>`.
7. Press `Ctrl-G`.
8. Type `status`.
9. Verify the session shows `stopped`.
10. Verify `.agentmux/runs/<session-id>/meta.txt` says `status: stopped`.

If Gemini exits too quickly for a stop test, use `agy` or another target that stays open long enough.

## Deliverable

Report:

- files changed
- verification commands run
- manual stop behavior tested
- session id used
- meta file status after stop
- limitations noticed
