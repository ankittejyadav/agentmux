# Handoff 005: Single-Session Plan Send

Status: complete. A deadlock in the switch path was fixed by using `portable-pty`'s `clone_killer()` instead of locking the child handle while waiting.

## Goal

Implement the first usable handoff workflow: `send <plan> <agent>` inside `Ctrl-G` command mode.

This step should let a user start in one harness, open command mode, and hand a saved plan to another harness without copy-pasting the task.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/004-command-mode.md`
- `src/main.rs`
- `src/pty.rs`
- `src/commands.rs`

## Current State

Implemented:

```sh
agentmux init
agentmux plan <name>
agentmux codex
agentmux claude
agentmux gemini
agentmux agy
```

Inside a running PTY session, `Ctrl-G` opens command mode.

Current command mode supports:

```text
help
status
```

## Scope

Add command-mode support for:

```text
send <plan> <agent>
```

For this step, keep it single-session:

- stop the current native harness
- launch the target native harness in the same `agentmux` process
- inject a handoff prompt into the target PTY

Do not implement:

- concurrent background sessions
- panes
- `focus`
- `stop`
- approval detection
- inline `//`
- full TUI

## Agent Names

Support these target agents:

```text
codex
claude
gemini
agy
```

Reject unknown agents with:

```text
unknown agent: <agent>
```

## Plan Validation

Plan folders live at:

```text
.agentmux/plans/<plan>/
```

`send auth agy` should require this folder to exist:

```text
.agentmux/plans/auth/
```

If missing, print:

```text
plan not found: auth
```

## Handoff Prompt

When launching the target agent, inject this prompt after the child starts:

```text
Read .agentmux/plans/<plan>/ and implement the task described there. Follow acceptance.md and constraints.md. When done, summarize files changed, verification run, and any blockers.
```

Send Enter after the prompt.

Keep the prompt simple and stable.

## Process Model

Recommended approach:

Refactor `run_pty_command` so it can return a command-mode action:

```rust
enum SessionAction {
    Exit(i32),
    SwitchAgent { agent: String, prompt: String },
}
```

Then `main.rs` can loop:

```text
current_agent = codex
optional_startup_prompt = none
run session
if SwitchAgent, run target agent with startup prompt
if Exit, exit process
```

Keep the implementation simple. A full session manager comes later.

## Command Parsing

Update `commands.rs` so command parsing is testable.

Suggested shape:

```rust
pub enum WrapperCommandResult {
    Print(String),
    Send { plan: String, agent: String },
}
```

Unit-test:

- `help`
- `status`
- unknown command
- valid `send auth agy`
- missing send arguments
- unknown agent

## Native Behavior Invariant

All normal input must still pass through to the native harness unchanged.

Do not intercept `/`.

Do not implement inline `//`.

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

Inside the Codex session:

1. Press `Ctrl-G`.
2. Type `send auth agy` and press Enter.
3. Verify the Codex session exits or is replaced.
4. Verify `agy` starts in the same terminal.
5. Verify the handoff prompt is sent automatically.

If `agy` is not convenient, use:

```text
send auth gemini
```

Also verify `/skills` or another native slash command still reaches the active harness before running `send`.

## Deliverable

Report:

- files changed
- verification commands run
- manual send behavior tested
- whether native slash commands still pass through
- limitations noticed
