# Handoff 002: PTY Wrapper

Status: complete on macOS. Follow-up portability hardening is tracked in `docs/handoffs/003-pty-portability.md`.

## Goal

Implement Step 3 of `agentmux`: run one native harness inside a PTY while preserving native interactive behavior.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `src/main.rs`

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

The four agent commands currently use `std::process::Command` as direct foreground child processes.

## Scope

Add a single-session PTY wrapper for the four agent launch commands:

```sh
agentmux codex
agentmux claude
agentmux gemini
agentmux agy
```

The command should still launch the same native CLI, but through a PTY instead of direct stdio.

Do not implement multi-session, panes, background agents, wrapper command mode, inline `//`, plan handoff, or approval popup UI yet.

## Dependency Decision

Add the smallest dependency needed for cross-platform PTYs:

```toml
portable-pty = "0.8"
```

Avoid adding TUI dependencies in this step.

## Expected Behavior

### Native-Like Interaction

The wrapped process should behave like a normal terminal session:

- stdin goes to the child PTY
- child PTY output is printed to stdout
- Ctrl-C and normal terminal input should go through as naturally as possible
- `/skills`, `/goals`, `/model`, and other native slash commands must reach the child harness unchanged

### Agent Selection

Keep the same mapping:

```text
agentmux codex   -> codex
agentmux claude  -> claude
agentmux gemini  -> gemini
agentmux agy     -> agy
```

Pass any trailing arguments through to the native CLI:

```sh
agentmux codex --help
agentmux gemini --version
```

### Terminal Size

Start with the current terminal size if easy. If not, use a reasonable default like 80x24 and leave a clear TODO.

Resize handling can come later.

### Exit Code

Propagate the child process exit code when available.

If the executable is missing, keep the current clean error:

```text
agentmux: could not launch `agy`; is it installed and on PATH?
```

## Suggested Implementation Shape

Keep it simple:

```text
src/main.rs
src/pty.rs
```

`main.rs` should route agent commands to a helper like:

```rust
run_pty_command(command: &str, args: Vec<String>) -> Result<i32, String>
```

`pty.rs` should own the PTY-specific logic.

The initial PTY implementation can use blocking threads:

- one thread copies PTY output to stdout
- main thread copies stdin to PTY input
- wait for child process and return its status

## Constraints

- Preserve `init` and `plan`.
- Keep native harness config folders untouched.
- Do not implement `//` yet.
- Do not parse approval prompts yet.
- Do not add a TUI yet.
- Prefer readable code over abstraction.

## Verification

Run:

```sh
cargo fmt
cargo build
cargo test
cargo run -- version
cargo run -- codex --help
cargo run -- gemini --version
```

Manual interactive check:

```sh
cargo run -- codex
```

Inside Codex, verify native input still works. In particular, `/skills` or another native slash command should be handled by Codex, not `agentmux`.

If `claude` is not installed on PATH, note that in the result instead of treating it as a code failure.

## Deliverable

Report:

- files changed
- dependencies added
- verification commands run
- which native CLIs were actually tested on this machine
- any interactive limitations noticed
