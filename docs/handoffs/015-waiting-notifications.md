# Handoff 015: Waiting Notifications

## Goal

Notify the user in the same terminal when a background session appears to be waiting for approval, permission, confirmation, or input.

Keep this as a lightweight notification layer. Do not add a full TUI or automatic approvals.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/014-adapter-specific-waiting-patterns.md`
- `src/main.rs`
- `src/commands.rs`
- `src/sessions.rs`
- `src/pty.rs`
- `src/waiting.rs`

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

Handoff 014 added:

- `detect_waiting_hint_for_agent(agent, text)`
- per-agent waiting patterns for `codex`, `claude`, `gemini`, and `agy`
- generic fallback detection
- background PTY detection wired to the agent-aware API

Remaining problem:

- waiting states appear in `status`
- but the user must manually check `status` to know a background session is stuck

## Scope

Add live same-terminal waiting notifications.

Do not add automatic approval.

Do not add panes, a full TUI, popup UI, async runtime, config parsing, or inline `//` commands in this handoff.

Do not change the public command surface.

## Desired Behavior

When a background session first enters a waiting state, print a concise notification to stderr:

```text
agentmux: auth-gemini-1777598200 waiting(confirmation); Ctrl-G focus auth-gemini-1777598200
```

If the same session keeps emitting matching text for the same hint, do not repeat the notification.

If the hint changes, print a new notification:

```text
agentmux: auth-gemini-1777598200 waiting(permission); Ctrl-G focus auth-gemini-1777598200
```

`status` should remain the source of truth for all waiting sessions.

## Session API

Change `set_background_waiting_hint` so the caller can tell whether the visible waiting state changed.

Suggested signature:

```rust
pub fn set_background_waiting_hint(session_id: &str, hint: &str) -> bool
```

Return value:

- `true` when the session existed and its hint changed from `None` or a different hint
- `false` when the session was not found or the hint was already the same

This function should still:

- update in-memory session state
- update the run `meta.txt` with `waiting: <hint>`

Keep `clear_background_waiting_hint(session_id)` behavior unchanged unless a small return value is useful for tests.

## Notification Formatting

Add a pure helper so notification text is unit-tested.

Suggested API:

```rust
pub fn format_waiting_notification(session_id: &str, hint: &str) -> String
```

Suggested output:

```text
agentmux: <session-id> waiting(<hint>); Ctrl-G focus <session-id>
```

Use stderr for live notifications so stdout transcripts and normal process output are not contaminated.

Use `\r\n` line endings when writing to the terminal, consistent with existing command-mode stderr output.

Do not add a terminal bell in this handoff unless it is trivial and easy to disable later. Text notification is enough for now.

## PTY Integration

In the background PTY output thread:

1. Detect waiting hints as today with `detect_waiting_hint_for_agent`.
2. Call `set_background_waiting_hint`.
3. If it returns `true`, write the notification to stderr.

Avoid spamming notifications while the same hint is repeatedly detected from the rolling buffer.

If the session is currently focused and already mirroring output to stdout, it is acceptable to skip the notification because the prompt is already visible. Simpler implementation can notify either way if it remains deduped.

## Status Behavior

Do not change the `status` command format introduced in handoff 013.

Waiting sessions should still look like:

```text
<session-id> <status> waiting(<hint>) <output-mode> <transcript-path>
```

## Critical Regression Guards

Native slash commands must still pass through unchanged to whichever session is receiving keyboard input.

Foreground stdout gating from handoff 012 must remain intact.

Waiting state must remain separate from process status.

Native approval remains manual. The user responds by running:

```text
focus <session-id>
```

Do not reintroduce the foreground switch deadlock. In `run_pty_command`, keep using:

```rust
let mut child_killer = child.clone_killer();
```

Do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

## Tests

Keep all existing tests passing.

Add focused tests for pure/session logic:

- `format_waiting_notification` returns the exact expected string
- `set_background_waiting_hint` returns `true` for a new hint
- `set_background_waiting_hint` returns `false` for the same hint repeated
- `set_background_waiting_hint` returns `true` when the hint changes
- `status` output remains unchanged except for existing `waiting(<hint>)`

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
2. Wait for Gemini output.
3. If Gemini asks for confirmation/input, confirm a one-line `agentmux:` notification appears.
4. Confirm repeated output does not spam repeated identical notifications.
5. `Ctrl-G`, then `status`.
6. Confirm `status` still shows `waiting(<hint>)`.
7. `Ctrl-G`, then `focus <session-id>`.
8. Type a response to the native prompt.
9. Confirm the waiting hint clears after input is sent.

## Deliverable

Report:

- files changed
- exact notification format
- exact return behavior of `set_background_waiting_hint`
- whether focused sessions emit notifications or skip them
- confirmation that native approval is still manual
- verification commands run
- tests added
- limitations noticed

## Expected Limitations

- stderr notifications may visually interleave with active foreground harness output
- notifications are heuristic because waiting detection is heuristic
- a richer status line or modal can come later with a real TUI
