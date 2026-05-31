# Decisions

## Product Shape

`agentmux` is a wrapper around native agent CLIs, not a replacement harness.

The native harness keeps ownership of:

- slash commands like `/skills`, `/goals`, `/model`
- native approval flows
- native config and memory
- native logs and private state folders
- model selection and built-in tools

`agentmux` owns only:

- PTY sessions
- panes/status/modals
- wrapper commands
- plan and handoff files
- run metadata
- process lifecycle

## Initial Agents

V1 should include all four target tools:

- Codex CLI
- Claude Code
- Gemini CLI
- Antigravity CLI

Each starts as a small adapter that knows how to launch the native command and detect common waiting states.

## Command Prefix

Native `/...` commands must pass through unchanged.

Wrapper commands may use `//...`, but the reliable design is:

- normal input goes directly to the active native harness
- a wrapper hotkey opens an `agentmux` command line
- inline `//` can be added later as convenience

The command prefix should be config-driven:

```toml
command_prefix = "//"
```

## State

Do not make `.codex`, `.claude`, `.gemini`, `.antigravity`, or `.antigravity-ide` interoperable.

Use a neutral local folder:

```text
.agentmux/
  config.toml
  plans/
  runs/
  sessions.json
```

Native state remains native.

## Permission Model

Do not replace native approval systems.

`agentmux` should detect when a background agent appears to be waiting for approval, show a same-terminal popup/status notification, and route the user's answer back into that native PTY.

If detection is uncertain, the fallback is to focus the waiting session.

## Tech Stack

Use Rust.

Planned libraries:

- `ratatui` and `crossterm` for the terminal UI
- `portable-pty` for cross-platform PTYs
- `vt100` or `termwiz` for terminal output parsing
- `serde` with `toml`/`json` for config and metadata
- `tokio` once concurrent PTY handling is needed

Step 0 intentionally avoids dependencies so the project builds immediately.

## Portability Rule

`agentmux` should stay cross-platform.

Unix-specific APIs are allowed only behind `cfg(unix)` with a working non-Unix fallback. Avoid code paths that make Windows builds fail, especially inside core PTY/session code.

Do not manually implement PATH lookup unless it handles Windows `PATHEXT` and command shims. Prefer letting process spawning report missing executables, then map that error into a clean user-facing message.
