# Handoff 004: Wrapper Command Mode

Status: complete.

## Goal

Implement Step 4 of `agentmux`: a hotkey-activated wrapper command mode inside the single-session PTY wrapper.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/003-pty-portability.md`
- `src/main.rs`
- `src/pty.rs`

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

The four agent commands now launch through a single-session PTY wrapper.

Raw mode uses `crossterm`, and `libc` has been removed.

## Scope

Add a wrapper command prompt that is activated by a hotkey while a native harness is running.

Default hotkey:

```text
Ctrl-G
```

Do not implement:

- inline `//`
- multi-session
- panes
- background agents
- approval detection
- plan handoff
- full TUI

## Required Behavior

### Pass-Through By Default

All normal input must continue going to the native harness unchanged.

This is the core invariant:

```text
/skills -> active native harness
/goals  -> active native harness
//...   -> still goes to native harness for now unless command mode is explicitly open
```

Do not intercept `/`.

### Command Mode

When the user presses `Ctrl-G`, do not forward that byte to the child PTY.

Instead show a small prompt in the same terminal:

```text
agentmux>
```

Read one command line from stdin, then execute it and return to the native harness session.

Use terminal-safe line endings:

```text
\r\n
```

Handle at least:

- Enter: submit command
- empty command: return to harness
- Backspace/Delete: edit current command
- Ctrl-C or Esc: cancel command mode and return to harness

### Initial Commands

Implement only:

```text
help
status
```

`help` prints:

```text
agentmux commands: help, status
```

`status` prints a minimal single-session status:

```text
active: codex
mode: single-session pty
```

Use the actual active adapter name.

Unknown commands should print:

```text
unknown agentmux command: <name>
```

Then return to the native harness.

## Suggested Implementation Shape

Keep this small.

Possible structure:

```text
src/main.rs
src/pty.rs
src/commands.rs
```

`commands.rs` can expose a pure helper:

```rust
pub fn run_wrapper_command(input: &str, active_agent: &str) -> String
```

This makes command behavior easy to unit test without a PTY.

`pty.rs` should own hotkey detection in the stdin-to-PTY bridge.

Recommended constants:

```rust
const CTRL_G: u8 = 0x07;
const ESC: u8 = 0x1b;
```

## Output Stream

Prefer writing wrapper prompt/output to stderr so child stdout stays as native as possible.

This is acceptable for now:

```rust
let mut ui = io::stderr();
```

Do not add a TUI dependency for this step.

## Constraints

- Preserve `init`, `plan`, and all native launch behavior.
- Keep native slash commands untouched.
- Do not implement `//` yet.
- Do not parse approval prompts yet.
- Do not add ratatui or a full-screen UI.
- Keep code readable.

## Verification

Run:

```sh
cargo fmt
cargo build
cargo test
cargo run -- version
cargo run -- gemini --version
```

Manual interactive check:

```sh
cargo run -- codex
```

Inside the Codex session:

1. Type a native slash command like `/skills` or `/help` and verify Codex receives it.
2. Press `Ctrl-G`.
3. Type `status` and press Enter.
4. Verify `agentmux` prints the active session status.
5. Continue typing in Codex and verify the session still works.

If Codex is not convenient, use `cargo run -- gemini` or `cargo run -- agy`.

## Deliverable

Report:

- files changed
- verification commands run
- manual command-mode behavior tested
- whether native slash commands still pass through
- limitations noticed
