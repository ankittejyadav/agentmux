# Handoff 013: Waiting And Approval Detection

## Goal

Make background sessions visibly report when they appear to be waiting for approval, confirmation, permission, or user input.

Keep this lightweight. Detection should help the user know when to `focus <session-id>` from the same terminal. It should not replace native harness approval flows.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/012-foreground-output-gating.md`
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

Handoff 012 added foreground transcripts and stdout gating so focused background output is no longer overwritten by active foreground output.

Remaining problem:

- background sessions can still stall when a native harness asks for approval or user input
- the user has to inspect transcripts manually or guess that a session is stuck

## Scope

Add generic waiting detection for background PTY output.

Do not add automatic approval.

Do not add panes, a full TUI, popup UI, async runtime, config parsing, or inline `//` commands in this handoff.

Do not change the public command surface.

## Desired Behavior

When a background session emits text that looks like an approval or input prompt, `agentmux status` should show it.

Example:

```text
active: codex
mode: single-session pty
foreground transcript: .agentmux/runs/foreground-codex-1777598300/transcript.ansi
background:
  auth-gemini-1777598200 running waiting(approval) background .agentmux/runs/auth-gemini-1777598200/transcript.ansi
```

The user can then run:

```text
focus auth-gemini-1777598200
```

and answer the native harness prompt directly.

## Session State

Prefer keeping process status and waiting state separate.

Suggested addition to `BackgroundSession`:

```rust
pub waiting_hint: Arc<Mutex<Option<String>>>
```

Rationale:

- a waiting process is still running
- existing `stop`, `focus`, and lifecycle checks already depend on `BackgroundStatus::Running`
- this avoids turning `Waiting` into a pseudo-process status

Update the custom `Debug` implementation so it does not expose internal lock details.

## Detection

Add a pure helper for detection. A small new module is acceptable, for example:

```rust
src/waiting.rs
```

Suggested API:

```rust
pub fn detect_waiting_hint(text: &str) -> Option<&'static str>
```

Start with generic, conservative pattern groups:

```text
approval:
  "approval"
  "approve"
  "requires approval"

permission:
  "permission"
  "allow"
  "do you want to"

confirmation:
  "proceed?"
  "continue?"
  "confirm"

input:
  "press enter"
  "select an option"
  "waiting for input"
```

Implementation guidance:

- lowercase before matching
- maintain a small rolling text buffer in the background output thread so prompts split across PTY chunks can still be detected
- cap the rolling buffer to a small size, such as the last 4096 characters
- false positives are acceptable if they are rare and easy to clear by focusing/responding

## Session APIs

Add small functions in `sessions.rs`:

```rust
pub fn set_background_waiting_hint(session_id: &str, hint: &str)
pub fn clear_background_waiting_hint(session_id: &str)
```

When setting a hint:

- update in-memory session state
- update the run `meta.txt` with a `waiting: <hint>` line

When clearing a hint:

- clear in-memory session state
- update `meta.txt` to remove the waiting line or set `waiting: none`

Clear waiting state when:

- the session exits
- the session is stopped
- keyboard input is sent to that focused background session

Do not clear the hint merely because the user focuses the session; focusing may be only for inspection.

## Status Output

Update `format_status` so waiting sessions are obvious.

Suggested background line format:

```text
<session-id> <status> waiting(<hint>) <output-mode> <transcript-path>
```

For non-waiting sessions, keep the current layout:

```text
<session-id> <status> <output-mode> <transcript-path>
```

## PTY Integration

In the background PTY output thread:

- write output to transcript exactly as today
- preserve existing output mirroring behavior for focused sessions
- append decoded output to a rolling detection buffer
- call the waiting detector on that buffer
- if a hint is detected, call `set_background_waiting_hint`

In the foreground/input routing thread:

- when input is routed to a focused background session, call `clear_background_waiting_hint` for that focused session
- preserve native slash command pass-through

## Critical Regression Guards

Do not reintroduce the foreground switch deadlock.

In `run_pty_command`, keep using:

```rust
let mut child_killer = child.clone_killer();
```

Do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

Native slash commands must still pass through unchanged to whichever session is currently receiving keyboard input.

Foreground stdout gating from handoff 012 must remain intact.

## Tests

Keep all existing tests passing.

Add focused tests for pure logic:

- detector returns `approval` for approval prompts
- detector returns `permission` for permission prompts
- detector returns `confirmation` for proceed/continue prompts
- detector returns `input` for press-enter/input prompts
- detector returns `None` for normal progress output
- `format_status` includes `waiting(<hint>)` only when a session has a waiting hint
- stopping or marking a session exited clears waiting state

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
2. Wait for the background session to produce output.
3. `Ctrl-G`, then `status`.
4. If Gemini or another harness asks for approval/input, confirm `status` shows `waiting(<hint>)`.
5. `Ctrl-G`, then `focus <session-id>`.
6. Type a response to the native prompt.
7. `Ctrl-G`, then `status`.
8. Confirm the waiting hint is cleared after input is routed to the focused session.

## Deliverable

Report:

- files changed
- waiting state field added
- exact detection patterns implemented
- exact `status` format for waiting sessions
- confirmation that native approval is not auto-handled
- verification commands run
- tests added
- limitations noticed

## Expected Limitations

- generic text matching will not catch every harness prompt
- some prompts may be detected late if the harness draws using complex terminal control sequences
- rare false positives are acceptable in this step
- adapter-specific prompt detection can come later
