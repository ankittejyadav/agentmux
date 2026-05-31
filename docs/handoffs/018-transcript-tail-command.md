# Handoff 018: Transcript Tail Command

## Goal

Add a lightweight command to inspect the end of a run transcript from `Ctrl-G` command mode.

This is a scroll-control step, not a full pager. Keep it small and file-based.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/017-run-inspection-commands.md`
- `src/main.rs`
- `src/commands.rs`
- `src/runs.rs`
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
runs
result <run-id>
```

Handoff 017 added:

- `runs` to list recent run folders
- `result <run-id>` to print a run's `result.md`
- `src/runs.rs` for file-based run inspection

Missing piece:

- no command exists to inspect transcript output without leaving the harness

## Scope

Add one wrapper command:

```text
tail <run-id> [lines]
```

Do not add a full pager.

Do not add live following.

Do not strip or parse ANSI in this handoff.

Do not add top-level shell commands yet.

## Desired Behavior

Inside any active harness:

```text
Ctrl-G, then tail auth-gemini-1777598200
```

Print the last 80 lines of:

```text
.agentmux/runs/auth-gemini-1777598200/transcript.ansi
```

With explicit line count:

```text
Ctrl-G, then tail auth-gemini-1777598200 40
```

Print the last 40 lines.

If the run folder is missing:

```text
run not found: <run-id>
```

If the transcript file is missing:

```text
transcript not found: <run-id>
```

If the line count is invalid:

```text
usage: tail <run-id> [lines]
```

## Output Format

Use a short header so the user knows what is being shown:

```text
transcript tail: <run-id> last <n> lines
<raw transcript lines>
```

Keep output on stderr through the existing `write_stderr` path.

Use raw transcript text. It is acceptable if ANSI escape sequences are printed.

## Limits

Default line count:

```text
80
```

Maximum line count:

```text
500
```

If the user requests more than 500 lines, cap to 500.

If the transcript is huge, avoid reading it into unbounded memory if the implementation can stay simple. A line ring buffer is ideal:

- read lines from the transcript
- keep only the last `n` lines in memory
- join and return them

If Rust line reading has trouble with non-UTF-8 transcript bytes, use lossy UTF-8 conversion rather than failing the whole command.

## Parser Changes

In `commands.rs`, add a parser variant:

```rust
Tail { run_id: String, lines: usize }
```

Parse:

```text
tail <run-id>
tail <run-id> <lines>
```

Default lines to 80.

Reject zero or non-numeric counts with:

```text
usage: tail <run-id> [lines]
```

Update help text to include:

```text
agentmux commands: help, status, send, stop, focus, detach, runs, result, tail
```

## Run Inspection Module

In `runs.rs`, add:

```rust
pub fn read_run_transcript_tail(run_id: &str, lines: usize) -> String
```

Expected behavior:

- look for `.agentmux/runs/<run-id>/`
- look for `.agentmux/runs/<run-id>/transcript.ansi`
- return explicit missing-run or missing-transcript messages
- return a header plus the last `lines.min(500)` lines

It is fine to add a private helper that accepts a runs directory path so tests can use isolated temp fixtures.

## PTY Integration

In `pty.rs`, handle the new parser variant inside `handle_command_mode`:

- `Tail { run_id, lines }` calls `runs::read_run_transcript_tail(&run_id, lines)`
- write the returned text to stderr

Do not route this command into the active native harness.

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

- parser recognizes `tail <run-id>` and defaults to 80 lines
- parser recognizes `tail <run-id> 40`
- parser rejects missing run id
- parser rejects zero line count
- parser rejects non-numeric line count
- `read_run_transcript_tail` reports missing run
- `read_run_transcript_tail` reports missing transcript
- `read_run_transcript_tail` returns only the requested last lines for a fixture transcript
- requested line count above 500 is capped

Avoid tests that require real Codex, Gemini, Claude, or Antigravity processes.

Use isolated temp fixture directories for run inspection tests, not the real `.agentmux/runs` directory.

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
2. Pick a run id with a transcript.
3. `Ctrl-G`, then `tail <run-id>`.
4. Confirm the last transcript lines print.
5. `Ctrl-G`, then `tail <run-id> 20`.
6. Confirm fewer lines print.
7. Confirm native slash commands still work after inspection.

## Deliverable

Report:

- files changed
- exact `tail <run-id> [lines]` output format
- default and max line counts
- parser variant added
- whether transcript bytes are read lossily or strictly as UTF-8
- verification commands run
- tests added
- limitations noticed

## Expected Limitations

- Transcript output is raw and may contain ANSI escape codes
- No live following exists yet
- No interactive pager exists yet
