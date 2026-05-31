# Handoff 009: Session IO Refactor

## Goal

Prepare background sessions for future `focus <session-id>` support without adding the `focus` command yet.

The important change is that a background PTY session must keep its input writer and output routing state alive after startup prompt injection.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/008-session-state-refactor.md`
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

`commands.rs` is now a pure parser.

`sessions.rs` owns background session state, status formatting, registration, exit marking, and stop behavior.

`pty.rs` still owns PTY spawning. In `spawn_background_pty`, the PTY writer is used only to inject the startup prompt and then drops when the function returns. That is fine for the current fire-and-log background mode, but it prevents future interactive focus because there is no saved input handle to write user keystrokes into the background PTY.

## Scope

Add session IO fields and output routing primitives, but do not add new user-facing commands.

Suggested additions in `sessions.rs`:

```rust
pub enum SessionOutputMode {
    BackgroundLog,
    Foreground,
}

pub struct BackgroundSession {
    // existing fields...
    pub input: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    pub output_mode: Arc<Mutex<SessionOutputMode>>,
}
```

Exact names can differ. The important behavior is:

- background session input writer is retained in session state
- output mode is shared with the background reader thread
- default output mode is transcript-only background logging
- existing `status` and `stop` APIs still work

If adding `Write` to `sessions.rs` feels too leaky, create a small type alias there:

```rust
pub type SessionInput = Arc<Mutex<Box<dyn Write + Send>>>;
```

## Output Routing Behavior

For this handoff, background output must still always be written to:

```text
.agentmux/runs/<session-id>/transcript.ansi
```

The background output thread may check `SessionOutputMode`, but default behavior must remain unchanged:

```text
BackgroundLog -> write transcript only
Foreground    -> write transcript and mirror to stdout
```

No command should switch to `Foreground` yet. This is only preparation for the next handoff.

## Critical Regression Guards

Do not reintroduce the foreground switch deadlock.

In `run_pty_command`, do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

Keep using:

```rust
let mut child_killer = child.clone_killer();
```

The wait thread may own `child` and call `child.wait()`.

The main thread should use `child_killer.kill()` on foreground switch.

Also avoid adding a full TUI, panes, async runtime, or `focus` command in this handoff.

## Command Behavior Must Stay The Same

These must keep working:

```text
help
status
send <plan> <agent>
send <plan> <agent> --bg
stop <session-id>
```

Native slash commands must still pass through unchanged to the foreground harness.

## Suggested Implementation Shape

In `spawn_background_pty`:

1. Take the PTY writer as today.
2. Wrap it in `Arc<Mutex<Box<dyn Write + Send>>>`.
3. Use the retained writer to inject the startup prompt.
4. Store a clone of that writer in `BackgroundSession`.
5. Create a shared `SessionOutputMode` set to `BackgroundLog`.
6. Give a clone of that output mode to the output thread.
7. Keep transcript writing exactly as it works today.

Do not hold the global sessions lock while doing blocking process IO.

## Tests

Keep existing parser tests passing.

Add focused tests if practical for:

- status formatting still works with the new session fields
- stop still works when input/output fields are present but unused
- `BackgroundSession` debug output does not try to print the raw writer

Do not write brittle tests that require actual Codex, Gemini, Claude, or Antigravity processes.

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
4. Confirm the run has `.agentmux/runs/<session-id>/transcript.ansi`.
5. `Ctrl-G`, then `stop <session-id>`.
6. `Ctrl-G`, then `status`.

## Deliverable

Report:

- files changed
- exact session IO fields added
- confirmation that the background PTY writer is retained after startup prompt injection
- confirmation that background output still writes transcripts
- verification commands run
- manual smoke behavior tested
- confirmation that foreground switch still uses `clone_killer()`
- limitations noticed

## Next Handoff After This

After this is complete, the next handoff should add:

```text
focus <session-id>
```

That command can use the retained input writer and output routing mode from this refactor.
