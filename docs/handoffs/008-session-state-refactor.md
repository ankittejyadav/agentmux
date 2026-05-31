# Handoff 008: Session State Refactor

## Goal

Refactor background session state out of `commands.rs` and keep command parsing separate from process/session management.

This is a cleanup and hardening step before adding interactive focus.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/007-stop-background-session.md`
- `src/main.rs`
- `src/commands.rs`
- `src/pty.rs`

## Current State

Implemented command-mode features:

```text
help
status
send <plan> <agent>
send <plan> <agent> --bg
stop <session-id>
```

Known code issue:

- `commands.rs` now mixes command parsing, background session storage, status formatting, and `ChildKiller` process handles.
- `pty.rs` owns process spawning and also implements stop behavior directly inside command-mode handling.

Before adding focus, separate these responsibilities.

## Scope

Add a new module:

```text
src/sessions.rs
```

Move background session types and global state there:

```rust
BackgroundSession
BackgroundStatus
get_background_sessions
```

Also move session operations there:

```rust
format_status(active_agent: &str) -> String
register_background_session(...)
mark_background_exited(...)
stop_background_session(session_id: &str) -> String
```

Exact function names can differ, but the boundary should be clear:

- `commands.rs`: parse user command into an enum
- `sessions.rs`: own background session state and status text
- `pty.rs`: launch PTYs and call session operations

Do not implement new user-facing features in this handoff.

## Critical Regression Guard

Do not reintroduce the foreground switch deadlock.

In `run_pty_command`, do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

Use:

```rust
let mut child_killer = child.clone_killer();
```

The wait thread may own `child` and call `child.wait()`.

The main thread should use `child_killer.kill()` on switch.

## Command Behavior Must Stay The Same

These must keep working:

```text
help
status
send <plan> <agent>
send <plan> <agent> --bg
stop <session-id>
```

No changes to output text unless necessary for the refactor.

## Suggested Parser Shape

`commands.rs` should not read or write global session state.

Suggested enum:

```rust
pub enum WrapperCommandResult {
    Help,
    Status,
    Print(String),
    Send { plan: String, agent: String, background: bool },
    Stop { session_id: String },
}
```

Then `pty.rs` or a small dispatcher can map:

```text
Help   -> print help text
Status -> sessions::format_status(active_agent)
Stop   -> sessions::stop_background_session(session_id)
```

If you prefer to keep `Help` as `Print`, that is acceptable. The important part is moving session state out of `commands.rs`.

## Tests

Keep existing tests and add/update tests so command parsing remains pure.

Unit tests should not depend on `.agentmux/plans/...` existing unless testing plan validation explicitly.

If plan validation stays in `commands.rs`, keep those tests. If plan validation moves, test it there.

Run all tests.

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

Manual smoke check:

```sh
cargo run -- codex
```

Inside Codex:

1. `Ctrl-G`, then `status`.
2. `Ctrl-G`, then `send auth gemini --bg`.
3. `Ctrl-G`, then `status`.
4. `Ctrl-G`, then `stop <session-id>`.
5. `Ctrl-G`, then `status`.

Verify behavior matches before the refactor.

## Deliverable

Report:

- files changed
- what moved into `sessions.rs`
- verification commands run
- manual smoke behavior tested
- confirmation that foreground switch still uses `clone_killer()`
- limitations noticed
