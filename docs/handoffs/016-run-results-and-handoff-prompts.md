# Handoff 016: Run Results And Handoff Prompts

## Goal

Make delegated agent runs easier to inspect by giving each run a dedicated `result.md` file and telling the target agent the exact path to write.

Keep this lightweight. Do not add a TUI, panes, config parsing, inline `//` commands, auto-approval, or a new command surface in this handoff.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/015-waiting-notifications.md`
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

Every foreground and background run already gets:

```text
.agentmux/runs/<run-id>/meta.txt
.agentmux/runs/<run-id>/transcript.ansi
```

The current handoff prompt asks the target agent to summarize when done, but it does not provide a stable file path for that summary.

## Scope

Add a run-local result file:

```text
.agentmux/runs/<run-id>/result.md
```

Tell delegated agents to write their final completion summary there.

Do not try to parse the result file yet.

Do not add a `result`, `runs`, `open`, or `tail` command yet.

## Desired Behavior

For background send:

```text
Ctrl-G, then send auth gemini --bg
```

`agentmux` should create:

```text
.agentmux/runs/auth-gemini-<timestamp>/meta.txt
.agentmux/runs/auth-gemini-<timestamp>/transcript.ansi
.agentmux/runs/auth-gemini-<timestamp>/result.md
```

The injected prompt should include:

```text
Write your final result to .agentmux/runs/auth-gemini-<timestamp>/result.md.
```

For foreground switch:

```text
Ctrl-G, then send auth agy
```

The switched foreground agent should also receive a prompt with the exact foreground run result path:

```text
.agentmux/runs/foreground-agy-<timestamp>/result.md
```

## Result File Template

Create `result.md` when the run directory is created.

Suggested initial content:

```markdown
# Result: <run-id>

Status: running

## Summary

## Files Changed

## Verification

## Blockers
```

Keep this ASCII.

Do not overwrite an existing result file if it already exists.

## Prompt Requirements

The target agent prompt should remain compact and file-based.

It should still tell the target agent:

- read `.agentmux/plans/<plan>/`
- follow `acceptance.md` and `constraints.md`
- implement the task described there

It should now also tell the target agent:

- write final result to the exact run result path
- include files changed
- include verification run
- include blockers, or `None`

Example prompt shape:

```text
Read .agentmux/plans/auth/ and implement the task described there. Follow acceptance.md and constraints.md. When done, write your final result to .agentmux/runs/auth-gemini-1777598200/result.md with summary, files changed, verification run, and blockers.
```

It is acceptable if foreground and background prompts use the same helper plus a small difference in run id.

## Metadata

Add the result path to `meta.txt` for new runs:

```text
result: .agentmux/runs/<run-id>/result.md
```

Preserve existing metadata fields:

- `plan`
- `agent`
- `role` for foreground runs if present
- `status`
- `waiting` when present

Do not remove existing status or waiting updates.

Important: status/waiting updates currently rewrite `meta.txt`. After adding `result: ...`, make sure later calls such as `mark_background_exited`, `stop_background_session`, `set_background_waiting_hint`, and `clear_background_waiting_hint` preserve or rewrite the `result` line instead of dropping it.

It is acceptable to add a `result_path` field to `BackgroundSession`, or to reconstruct the path from the run id. Prefer the smaller change that keeps the code readable.

## Suggested Helpers

Add small pure helpers where they fit best, likely in `sessions.rs`:

```rust
pub fn format_result_template(run_id: &str) -> String
pub fn format_result_instruction(result_path: &str) -> String
```

If a different naming/location fits the code better, that is fine. Keep the behavior unit-tested.

## PTY Integration

Background runs:

1. Generate `run_id`.
2. Create run directory.
3. Create `result.md` placeholder.
4. Write `meta.txt` including `result: ...`.
5. Inject prompt including exact `result.md` path.

Foreground switched runs:

1. Create foreground run directory as today.
2. Create `result.md` placeholder.
3. Write foreground `meta.txt` including `result: ...`.
4. If `startup_prompt` is present, append or replace with a prompt that includes exact `result.md` path before injecting into the child PTY.

Direct launches like `agentmux codex` may create a result placeholder because they already create foreground run directories, but they should not receive a handoff prompt unless `startup_prompt` is present.

## Critical Regression Guards

Native slash commands must still pass through unchanged.

Waiting notifications from handoff 015 must still work.

Foreground output gating must remain intact.

Focused background sessions must remain interactive.

Do not reintroduce the foreground switch deadlock. In `run_pty_command`, keep using:

```rust
let mut child_killer = child.clone_killer();
```

Do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

## Tests

Keep all existing tests passing.

Add focused tests for pure logic:

- `format_result_template` includes the run id and expected headings
- the handoff prompt still includes the plan directory
- the handoff prompt includes the exact result path
- metadata rewrites preserve `result: ...` after waiting, stop, and exit updates if those helpers are touched
- foreground metadata formatting includes `result: ...` if you update the helper signature
- existing waiting notification tests still pass

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
2. Confirm `.agentmux/runs/auth-gemini-<timestamp>/result.md` exists.
3. Confirm `meta.txt` contains `result: .agentmux/runs/auth-gemini-<timestamp>/result.md`.
4. Confirm the transcript shows the injected prompt with the exact result path.
5. `Ctrl-G`, then `send auth agy`.
6. Confirm the switched foreground run has a `result.md` file and prompt includes its exact path.

## Deliverable

Report:

- files changed
- exact `result.md` path format for background runs
- exact `result.md` path format for foreground switched runs
- prompt text or prompt helper behavior
- whether direct native launches create result placeholders
- metadata fields after the change
- verification commands run
- tests added
- limitations noticed

## Expected Limitations

- Agents may ignore the instruction to write `result.md`
- `agentmux` will not parse or validate `result.md` yet
- No command exists yet to show recent runs or open result files
