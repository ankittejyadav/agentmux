# Handoff 006: Background Send MVP

Status: complete. A foreground switch deadlock regression was fixed by restoring use of `portable-pty`'s `clone_killer()`.

## Goal

Implement the smallest useful multi-session step: `send <plan> <agent> --bg`.

This should let the user keep working in the current harness while another agent starts in the background from a saved plan.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/005-single-session-send.md`
- `src/main.rs`
- `src/pty.rs`
- `src/commands.rs`

## Current State

Implemented:

```text
Ctrl-G -> command mode
help
status
send <plan> <agent>
```

Current `send <plan> <agent>` is intentionally destructive: it terminates the active harness and switches to the target agent.

## Scope

Add:

```text
send <plan> <agent> --bg
```

Behavior:

- keep the active harness running
- launch the target agent in a background PTY
- inject the same handoff prompt used by foreground `send`
- capture background output to a transcript file
- extend `status` to show background sessions

Do not implement:

- panes
- focus switching
- stopping sessions
- approval detection
- inline `//`
- full TUI

## Run Metadata

Create a run folder for each background session:

```text
.agentmux/runs/<run-id>/
  transcript.ansi
  meta.txt
```

Use a simple run id:

```text
<plan>-<agent>-<unix-timestamp>
```

If timestamp handling is inconvenient, use a monotonic counter for this process.

`meta.txt` should include:

```text
plan: auth
agent: agy
status: running
```

When the process exits, update the in-memory status. Updating `meta.txt` on exit is nice but not required in this step.

## Command Parsing

Update command parsing so these are different:

```text
send auth agy       -> foreground switch
send auth agy --bg  -> background session
```

Suggested enum:

```rust
pub enum WrapperCommandResult {
    Print(String),
    Send { plan: String, agent: String, background: bool },
}
```

Unit-test:

- foreground send
- background send
- `--bg` with missing args
- unknown agent
- missing plan

## Session State

Keep it in memory for now.

Suggested status:

```rust
struct BackgroundSession {
    id: String,
    plan: String,
    agent: String,
    status: BackgroundStatus,
    transcript_path: PathBuf,
}
```

Possible statuses:

```text
running
exited(<code>)
failed
```

Keep the data structure simple. Persistence can come later.

## Status Output

`status` should still show the active foreground agent:

```text
active: codex
mode: single-session pty
```

If background sessions exist, append:

```text
background:
  auth-agy-123 running .agentmux/runs/auth-agy-123/transcript.ansi
```

Use `\n` in command output; `pty.rs` can keep converting to `\r\n` for rendering.

## Background PTY

Background session output should not print to the main terminal.

It should write to:

```text
.agentmux/runs/<run-id>/transcript.ansi
```

No input is sent to the background session after the startup prompt in this step.

If the background agent needs approval, it may stall. Approval detection comes later.

## Handoff Prompt

Use the existing prompt:

```text
Read .agentmux/plans/<plan>/ and implement the task described there. Follow acceptance.md and constraints.md. When done, summarize files changed, verification run, and any blockers.
```

Send Enter after the prompt.

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
3. Verify Codex remains active.
4. Press `Ctrl-G`.
5. Type `status`.
6. Verify the background Gemini session is listed.
7. Verify a transcript exists under `.agentmux/runs/`.

If Gemini is not convenient, use `agy`.

## Deliverable

Report:

- files changed
- verification commands run
- manual background send behavior tested
- transcript path created
- limitations noticed
