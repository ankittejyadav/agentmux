# Handoff 017: Run Inspection Commands

## Goal

Make completed and in-progress runs inspectable from the same `Ctrl-G` command mode.

Add lightweight commands to list recent runs and print a run's `result.md` content. Do not add a full TUI, panes, fuzzy finder, external pager, config parsing, inline `//` commands, or result validation in this handoff.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/016-run-results-and-handoff-prompts.md`
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

Handoff 016 added run result files:

```text
.agentmux/runs/<run-id>/meta.txt
.agentmux/runs/<run-id>/transcript.ansi
.agentmux/runs/<run-id>/result.md
```

The missing piece is quick same-window inspection. Today the user has to leave the harness or manually open files.

## Scope

Add two wrapper commands:

```text
runs
result <run-id>
```

Keep these as command-mode operations written to stderr through the existing `write_stderr` path.

Do not change native slash-command pass-through.

Do not add top-level shell commands yet.

## Desired Behavior

Inside any active harness:

```text
Ctrl-G, then runs
```

Print a compact list of recent run folders:

```text
runs:
  auth-gemini-1777598200 running gemini result=.agentmux/runs/auth-gemini-1777598200/result.md transcript=.agentmux/runs/auth-gemini-1777598200/transcript.ansi
  foreground-codex-1777598300 exited(0) codex result=.agentmux/runs/foreground-codex-1777598300/result.md transcript=.agentmux/runs/foreground-codex-1777598300/transcript.ansi
```

Inside any active harness:

```text
Ctrl-G, then result auth-gemini-1777598200
```

Print the contents of:

```text
.agentmux/runs/auth-gemini-1777598200/result.md
```

If the result file is missing:

```text
result not found: <run-id>
```

If the run folder is missing:

```text
run not found: <run-id>
```

If no runs exist:

```text
no runs found
```

## Parser Changes

In `commands.rs`, add parser variants:

```rust
Runs
Result { run_id: String }
```

Parse:

```text
runs
result <run-id>
```

Error:

```text
usage: result <run-id>
```

Update help text to include:

```text
agentmux commands: help, status, send, stop, focus, detach, runs, result
```

## Run Inspection Module

Add a small module if it keeps the code clean:

```rust
src/runs.rs
```

Suggested public functions:

```rust
pub fn format_runs_list() -> String
pub fn read_run_result(run_id: &str) -> String
```

These functions can read `.agentmux/runs/` directly.

Keep parsing simple:

- read each run folder in `.agentmux/runs/`
- read `meta.txt` if present
- parse `key: value` lines into agent/status/result where available
- infer transcript path as `.agentmux/runs/<run-id>/transcript.ansi` if the file exists
- sort newest first by directory modified time when available
- cap the displayed list at 20 runs

If metadata is incomplete, display `unknown` for missing agent/status fields.

## Result Output

For `result <run-id>`, print raw markdown content to stderr.

Do not strip headings.

Do not parse markdown.

Do not truncate unless the file is very large. If adding a cap, use a simple cap like 64 KiB and append:

```text
[truncated]
```

## PTY Integration

In `pty.rs`, handle new command parser variants inside `handle_command_mode`:

- `Runs` calls `runs::format_runs_list()` and writes the returned string to stderr
- `Result { run_id }` calls `runs::read_run_result(&run_id)` and writes the returned string to stderr

Do not route these commands into the active native harness.

## Critical Regression Guards

Native slash commands must still pass through unchanged.

Waiting notifications must still work.

Foreground output gating must remain intact.

Focused background sessions must remain interactive.

Run inspection must not require any native agent command to be installed.

Do not reintroduce the foreground switch deadlock. In `run_pty_command`, keep using:

```rust
let mut child_killer = child.clone_killer();
```

Do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

## Tests

Keep all existing tests passing.

Add focused tests:

- parser recognizes `runs`
- parser recognizes `result <run-id>`
- parser returns `usage: result <run-id>` when missing id
- `format_runs_list` returns `no runs found` when `.agentmux/runs/` is absent or empty
- `format_runs_list` includes run id, status, agent, result path, and transcript path for a fixture run
- `read_run_result` returns result markdown for a fixture run
- `read_run_result` reports missing run
- `read_run_result` reports missing result file

Avoid tests that require real Codex, Gemini, Claude, or Antigravity processes.

Use unique fixture run ids in tests so global or local state from other tests does not matter.

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

1. `Ctrl-G`, then `runs`.
2. Confirm recent run ids appear.
3. `Ctrl-G`, then `result <run-id>`.
4. Confirm the run's `result.md` content prints.
5. Confirm native slash commands still work after inspection.

## Deliverable

Report:

- files changed
- exact `runs` output format
- exact `result <run-id>` behavior
- parser variants added
- whether a new `runs.rs` module was added
- verification commands run
- tests added
- limitations noticed

## Expected Limitations

- Result files are displayed raw, not parsed
- Long result files may be noisy in the terminal
- No interactive run picker exists yet
- No command exists yet to tail transcripts
