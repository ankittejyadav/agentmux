# Handoff 011: Focus Hardening And Resize

## Goal

Harden the new `focus <session-id>` / `detach` behavior and replace the fixed `80x24` PTY size with real terminal sizing plus lightweight resize polling.

Do not add a full TUI, panes, async runtime, config parsing, or inline `//` commands in this handoff.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/010-focus-detach-mvp.md`
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
focus <session-id>
detach
stop <session-id>
```

Handoff 010 added input routing:

- foreground PTY writer is wrapped as `SessionInput`
- `active_input` points either at the foreground writer or a focused background writer
- focused background output is mirrored to stdout by setting `SessionOutputMode::Foreground`
- `detach` restores input to the foreground writer

Known limitations:

- if a focused background session exits naturally, `active_input` can still point at a stale writer until the user detaches manually
- `status` does not clearly show which background session is foreground-routed by output mode
- all PTYs still open with static `80x24`
- there is no resize propagation to child harnesses
- foreground output can still visually interleave with focused background output; this handoff should make state clearer, not build a full pane system

## Scope

Implement three improvements:

1. Focus lifecycle hardening.
2. Better status display for focused/output-routed sessions.
3. Real terminal size and resize propagation.

Do not change the public command surface.

## Focus Lifecycle Hardening

Add a small session status API in `sessions.rs`.

Suggested names:

```rust
pub fn background_session_status(session_id: &str) -> Option<BackgroundStatus>
pub fn is_background_session_running(session_id: &str) -> bool
```

Then update the stdin forwarding loop in `pty.rs`:

- if `focused_session_id` is set, check whether that session is still running before writing normal input
- if it is not running, restore `active_input` to the foreground writer
- set `focused_session_id` to `None`
- write a short stderr message like `focused session ended; detached`

Also make `mark_background_exited` reset the session's `output_mode` to `BackgroundLog`.

`stop <session-id>` already detaches if the stopped session is focused. Keep that behavior.

## Better Status Display

Update `sessions::format_status(active_agent)` so each background line includes the output mode.

Example:

```text
background:
  auth-gemini-1777598200 running foreground .agentmux/runs/auth-gemini-1777598200/transcript.ansi
  docs-agy-1777598210 running background .agentmux/runs/docs-agy-1777598210/transcript.ansi
```

Use simple words:

```text
foreground
background
```

Do not print raw writer/control internals.

## Terminal Size

Replace fixed `80x24` PTY startup sizing with the current terminal dimensions.

Suggested helper in `pty.rs`:

```rust
fn current_pty_size() -> PtySize {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}
```

Use this helper for both foreground and background `openpty`.

Keep the `80x24` fallback for non-interactive or unsupported terminals.

## Resize Propagation

`portable-pty` exposes `MasterPty::resize(PtySize)`.

Add a retained PTY control handle for sessions so resize can be applied after startup.

Suggested type in `sessions.rs`:

```rust
pub type SessionControl = Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>;
```

Add to `BackgroundSession`:

```rust
pub control: Option<SessionControl>
```

In `spawn_background_pty`:

- clone reader and take writer as today
- retain `pair.master` inside `SessionControl`
- store `control` in `BackgroundSession`
- keep transcript logging and input retention unchanged

Add a resize API:

```rust
pub fn resize_background_sessions(size: PtySize)
```

It should:

- resize only running sessions
- ignore resize errors
- avoid holding the global sessions lock while doing blocking or slow operations if practical

For foreground:

- retain the foreground `pair.master` in a local `SessionControl`
- use it for resize polling

Add a lightweight polling thread in `run_pty_command`:

- poll `current_pty_size()` every 500ms
- if size changed, call `resize` on the foreground control
- call `sessions::resize_background_sessions(size)`
- stop the polling thread when the foreground `run_pty_command` returns

Use `Arc<AtomicBool>` or equivalent to stop the polling thread cleanly.

Do not use `crossterm::event::read` for resize in this handoff because it can compete with the raw stdin forwarding loop.

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
focus <session-id>
detach
stop <session-id>
```

Do not change command names or add new required arguments.

## Tests

Keep all existing tests passing.

Add or update tests for:

- `format_status` includes output mode words
- `mark_background_exited` changes a focused/output-routed session back to `BackgroundLog`
- `is_background_session_running` or equivalent returns false for stopped/exited/unknown sessions
- `BackgroundSession` debug output does not expose raw input/control internals

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
3. Confirm background rows show `background`.
4. Copy the session id.
5. `Ctrl-G`, then `focus <session-id>`.
6. `Ctrl-G`, then `status`.
7. Confirm that row shows `foreground` and status still shows the focused id.
8. Resize the terminal window.
9. Confirm the active harness output remains usable after resize.
10. `Ctrl-G`, then `detach`.
11. `Ctrl-G`, then `stop <session-id>`.
12. `Ctrl-G`, then `status`.

If practical, also test a focused session that exits naturally and confirm the next normal keypress detaches back to the foreground harness instead of writing indefinitely to the stale background writer.

## Deliverable

Report:

- files changed
- focus lifecycle APIs added
- resize/control fields added
- how resize polling is stopped
- status output before/after
- verification commands run
- manual smoke behavior tested
- confirmation that foreground switch still uses `clone_killer()`
- limitations noticed

## Next Handoff After This

After this is complete, the next handoff should address one of these:

- foreground transcript/buffering to reduce output interleaving
- waiting/approval detection from background transcripts
- config file support for agent command paths and aliases
