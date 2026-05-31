# Handoff 010: Focus And Detach MVP

## Goal

Implement a minimal usable version of:

```text
focus <session-id>
detach
```

This should let the user interact with a running background session from the same terminal, then return keyboard input to the original foreground harness.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/009-session-io-refactor.md`
- `src/main.rs`
- `src/commands.rs`
- `src/sessions.rs`
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

Handoff 009 added:

- `SessionOutputMode::{BackgroundLog, Foreground}`
- `SessionInput = Arc<Mutex<Box<dyn std::io::Write + Send>>>`
- `BackgroundSession.input`
- `BackgroundSession.output_mode`
- background output routing that always writes transcripts and can mirror to stdout when `Foreground`

No command currently changes `output_mode`.

## Scope

Add two wrapper commands inside `Ctrl-G` command mode:

```text
focus <session-id>
detach
```

`focus <session-id>` should:

- validate that the session exists
- validate that the session is still running
- get the retained `SessionInput` writer for that background session
- set that session's `output_mode` to `Foreground`
- route future normal keyboard input bytes to that session input
- keep `Ctrl-G` intercepted by `agentmux`
- print a short confirmation to stderr

`detach` should:

- return normal keyboard input bytes to the original foreground harness
- set the focused background session's `output_mode` back to `BackgroundLog`
- print a short confirmation to stderr
- be harmless if no background session is focused

Do not add panes, a full TUI, async runtime, config parsing, or inline `//` commands in this handoff.

## Important Limitation For This MVP

This is not full visual isolation.

The original foreground harness may still write to stdout while a background session is focused. That is acceptable for this handoff as long as:

- the foreground process is not killed
- keyboard input is routed correctly
- focused background output is visible
- `detach` restores keyboard input to the foreground harness

Full isolation can come later by turning the foreground harness into a managed session or by adding a real TUI.

## Suggested Command Parser Shape

Add variants in `commands.rs`:

```rust
pub enum WrapperCommandResult {
    Help,
    Status,
    Print(String),
    Send { plan: String, agent: String, background: bool },
    Stop { session_id: String },
    Focus { session_id: String },
    Detach,
}
```

Parser rules:

```text
focus              -> usage: focus <session-id>
focus <session-id> -> Focus { session_id }
detach             -> Detach
```

Update parser unit tests.

## Suggested Session APIs

Add small APIs in `sessions.rs`.

Suggested names:

```rust
pub fn focus_background_session(session_id: &str) -> Result<SessionInput, String>
pub fn detach_background_session(session_id: &str) -> Result<(), String>
```

`focus_background_session` should:

- find the session
- ensure `BackgroundStatus::Running`
- ensure `input` exists
- set `output_mode` to `SessionOutputMode::Foreground`
- return a clone of `SessionInput`

`detach_background_session` should:

- find the session
- set `output_mode` to `SessionOutputMode::BackgroundLog`
- return an error only if the session id is unknown

Do not hold the global sessions lock while doing blocking PTY writes.

## Suggested PTY Input Shape

In `run_pty_command`, wrap the foreground PTY writer in the same `SessionInput` shape:

```rust
let foreground_input: crate::sessions::SessionInput =
    Arc::new(Mutex::new(Box::new(pty_writer)));

let mut active_input = Arc::clone(&foreground_input);
let mut focused_session_id: Option<String> = None;
```

Normal input bytes should write to `active_input`.

When `Ctrl-G` opens command mode:

- `focus <session-id>` updates `active_input`
- `detach` restores `active_input` to `foreground_input`
- `send <plan> <agent>` and process exit behavior remain unchanged

Keep command-mode UI on stderr.

## Critical Regression Guards

Do not reintroduce the foreground switch deadlock.

In `run_pty_command`, do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

Keep using:

```rust
let mut child_killer = child.clone_killer();
```

The wait thread may own `child` and call `child.wait()`.

The main thread should use `child_killer.kill()` on foreground switch.

Native slash commands must still pass through unchanged to whichever session is currently receiving keyboard input.

## Command Behavior Must Stay The Same

These must keep working:

```text
help
status
send <plan> <agent>
send <plan> <agent> --bg
stop <session-id>
```

`stop <session-id>` should still work for a focused session. If the stopped session is currently focused, restore input to the original foreground harness.

## Tests

Keep all existing tests passing.

Add tests for:

- `focus` missing args
- `focus <session-id>` parser result
- `detach` parser result
- focusing unknown session returns an error
- detaching unknown session returns an error
- debug/status behavior still does not expose raw writer internals

Avoid tests that require real Codex, Gemini, Claude, or Antigravity processes.

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

1. `Ctrl-G`, then `send auth gemini --bg`.
2. `Ctrl-G`, then `status`.
3. Copy the session id from `status`.
4. `Ctrl-G`, then `focus <session-id>`.
5. Type a harmless Gemini command such as `/help` or `hello`.
6. Confirm output appears in the same terminal and still writes to `.agentmux/runs/<session-id>/transcript.ansi`.
7. `Ctrl-G`, then `detach`.
8. Type a harmless Codex command or text and confirm input goes back to Codex.
9. `Ctrl-G`, then `stop <session-id>`.
10. `Ctrl-G`, then `status`.

## Deliverable

Report:

- files changed
- parser variants added
- session APIs added
- how input routing works
- how detach restores foreground input
- verification commands run
- manual smoke behavior tested
- confirmation that foreground switch still uses `clone_killer()`
- limitations noticed

## Next Handoff After This

After this is complete, the next handoff should harden focus behavior:

- better focused status display
- handling stopped/exited focused sessions
- reducing foreground output interleaving
- terminal resize support
