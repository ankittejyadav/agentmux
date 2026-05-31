# Handoff 014: Adapter-Specific Waiting Patterns

## Goal

Make waiting detection agent-aware for `codex`, `claude`, `gemini`, and `agy`, while keeping the current generic detector as a fallback.

This should reduce false positives and catch more native approval prompts without changing how the user approves anything.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/013-waiting-approval-detection.md`
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

Handoff 013 added:

- `BackgroundSession.waiting_hint`
- generic text detection in `src/waiting.rs`
- background PTY rolling-buffer detection
- `waiting(<hint>)` in `status`
- waiting hint clearing when input is sent to a focused background session

Remaining problem:

- generic substring matching is useful but rough
- each harness phrases approvals and confirmations differently
- we need more precise per-agent patterns before adding popup/status-line UI

## Scope

Add agent-aware waiting detection.

Do not add automatic approval.

Do not add panes, a full TUI, popup UI, async runtime, config parsing, or inline `//` commands in this handoff.

Do not change the public command surface.

Avoid introducing a large adapter trait system. Keep this as simple data and pure functions.

## Desired API

Keep the current generic API if tests depend on it:

```rust
pub fn detect_waiting_hint(text: &str) -> Option<&'static str>
```

Add an agent-aware API:

```rust
pub fn detect_waiting_hint_for_agent(agent: &str, text: &str) -> Option<&'static str>
```

Behavior:

- check patterns for the named agent first
- if no agent-specific match is found, fall back to `detect_waiting_hint(text)`
- unknown agents should use only the generic fallback

## Suggested Pattern Shape

Keep pattern data static and easy to maintain.

One acceptable shape:

```rust
struct WaitingPattern {
    hint: &'static str,
    needle: &'static str,
}
```

Then:

```rust
const CODEX_PATTERNS: &[WaitingPattern] = &[...];
const CLAUDE_PATTERNS: &[WaitingPattern] = &[...];
const GEMINI_PATTERNS: &[WaitingPattern] = &[...];
const AGY_PATTERNS: &[WaitingPattern] = &[...];
```

Matching can stay case-insensitive substring matching for now.

## Initial Agent-Specific Patterns

Use conservative patterns. The exact strings can be refined later.

Codex:

```text
approval:
  "allow command"
  "allow codex"
  "approve command"
  "requires approval"

permission:
  "sandbox"
  "escalated permissions"
  "run outside the sandbox"
```

Claude:

```text
permission:
  "do you want to allow"
  "allow this command"
  "permission to use"

confirmation:
  "continue?"
  "proceed?"
```

Gemini:

```text
confirmation:
  "do you want to continue"
  "continue?"
  "proceed?"

input:
  "select an option"
  "press enter"
```

Antigravity / `agy`:

```text
approval:
  "approval required"
  "requires approval"

permission:
  "permission required"
  "allow"

input:
  "waiting for input"
  "press enter"
```

If any of these are too broad in tests, keep the tests conservative and document the risk in the deliverable.

## PTY Integration

In the background PTY output thread, replace the generic call:

```rust
crate::waiting::detect_waiting_hint(&rolling_buf)
```

with agent-aware detection:

```rust
crate::waiting::detect_waiting_hint_for_agent(&agent_name, &rolling_buf)
```

Important: clone the background `agent` string into the thread before moving it, so session registration still works cleanly.

Keep the rolling buffer cap at about 4096 characters.

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

Do not reintroduce the foreground switch deadlock. In `run_pty_command`, keep using:

```rust
let mut child_killer = child.clone_killer();
```

Do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

## Tests

Keep all existing tests passing.

Add focused tests for pure logic:

- Codex-specific approval pattern returns `approval`
- Codex-specific sandbox/escalation pattern returns `permission`
- Claude-specific allow pattern returns `permission`
- Gemini-specific continue/select prompt returns the expected hint
- `agy` approval or permission prompt returns the expected hint
- unknown agent falls back to generic detection
- normal progress output still returns `None`

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
3. `Ctrl-G`, then `status`.
4. If Gemini asks for confirmation/input, confirm `status` shows `waiting(<hint>)`.
5. `Ctrl-G`, then `focus <session-id>`.
6. Type a response to the native prompt.
7. Confirm the waiting hint clears after input is sent.

## Deliverable

Report:

- files changed
- exact agent-aware API added
- exact per-agent patterns implemented
- confirmation that generic fallback still works
- confirmation that native approval is still manual
- verification commands run
- tests added
- limitations noticed

## Expected Limitations

- these patterns are still heuristic
- prompt text from native harnesses may change over time
- terminal escape sequences can still split or obscure prompts
- config-driven pattern overrides can come later
