# Handoff 012: Foreground Output Gating

## Goal

Reduce terminal output interleaving without adding a full TUI.

When a background session is focused, the original foreground harness should keep running and its output should still be captured, but it should not keep writing over the focused background session on stdout.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/011-focus-hardening-resize.md`
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

Handoff 011 added:

- dynamic initial PTY sizing
- resize polling
- background `SessionControl`
- clearer background output mode in `status`
- automatic detach when a focused background session is no longer running

Remaining problem:

- the foreground output thread still mirrors foreground harness output directly to stdout even while a background session is focused
- focused background output also mirrors to stdout
- the two streams can visually interleave

## Scope

Add foreground output gating and foreground transcript capture.

Do not add panes, a full TUI, async runtime, config parsing, or inline `//` commands in this handoff.

Do not change the public command surface.

## Desired Behavior

Normal foreground mode:

```text
foreground PTY output -> foreground transcript
foreground PTY output -> stdout
```

Focused background mode:

```text
foreground PTY output -> foreground transcript
foreground PTY output -X stdout
focused background PTY output -> background transcript
focused background PTY output -> stdout
```

After `detach`:

```text
foreground PTY output -> foreground transcript
foreground PTY output -> stdout
background PTY output -> background transcript only
```

Command-mode UI should still write to stderr and remain visible.

## Foreground Transcript

Create a foreground run directory when `run_pty_command` starts.

Suggested path:

```text
.agentmux/runs/foreground-<agent>-<timestamp>/
  meta.txt
  transcript.ansi
```

Suggested `meta.txt`:

```text
agent: codex
role: foreground
status: running
```

When the foreground harness exits, update `meta.txt` to:

```text
agent: codex
role: foreground
status: exited(<code>)
```

If creating the transcript fails, keep the native harness running and fall back to stdout-only behavior. Transcript failure should not prevent launching Codex, Claude, Gemini, or Antigravity.

## Output Gating

Add a small shared flag in `pty.rs`.

Suggested shape:

```rust
let foreground_output_visible = Arc::new(AtomicBool::new(true));
```

The foreground output thread should:

- always write PTY bytes to the foreground transcript when available
- write PTY bytes to stdout only when `foreground_output_visible` is true

Set `foreground_output_visible` to `false` when `focus <session-id>` succeeds.

Set `foreground_output_visible` to `true` when:

- `detach` succeeds
- the focused session is stopped
- the focused session exits and the lifecycle check auto-detaches
- `send <plan> <agent>` foreground switch is triggered
- the foreground command is about to return

Keep this flag local to `run_pty_command`; do not put it in global session state.

## Status Output

When command mode handles `status`, include the foreground transcript path if available.

Example:

```text
active: codex
mode: single-session pty
foreground transcript: .agentmux/runs/foreground-codex-1777598300/transcript.ansi
focused: auth-gemini-1777598200
background:
  auth-gemini-1777598200 running foreground .agentmux/runs/auth-gemini-1777598200/transcript.ansi
```

If there is no foreground transcript because transcript creation failed, omit the line.

## Implementation Notes

Keep the current input routing model:

- `active_input` controls where keyboard bytes go
- `foreground_output_visible` controls whether foreground output appears on stdout
- background session `SessionOutputMode` controls whether background output appears on stdout

These are separate concerns. Do not merge them into one global state object in this handoff.

Avoid holding locks while writing to stdout or transcript files when practical.

## Critical Regression Guards

Do not reintroduce the foreground switch deadlock.

In `run_pty_command`, do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

Keep using:

```rust
let mut child_killer = child.clone_killer();
```

The wait thread may own `child` and call `child.wait()`.

The main thread should use `child_killer.kill()` on foreground switch.

Native slash commands must still pass through unchanged to whichever session is currently receiving keyboard input.

## Command Behavior Must Stay The Same

These must keep working:

```text
help
status
send <plan> <agent>
send <plan> <agent> --bg
focus <session-id>
detach
stop <session-id>
```

Do not change command names or add new required arguments.

## Tests

Keep all existing tests passing.

Add focused tests if practical for pure helpers:

- foreground run id/path helper produces a stable path shape
- foreground meta text formats `running` and `exited(<code>)`
- status text appends foreground transcript only when a path is present

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
2. `Ctrl-G`, then `status`.
3. Confirm status shows a foreground transcript path.
4. Copy the background session id.
5. `Ctrl-G`, then `focus <session-id>`.
6. Confirm focused background output appears.
7. While focused, confirm foreground output does not continuously overwrite the focused background output.
8. `Ctrl-G`, then `detach`.
9. Confirm foreground output resumes.
10. Confirm foreground transcript exists and contains foreground output.
11. Confirm background transcript still exists and contains background output.
12. `Ctrl-G`, then `stop <session-id>`.

## Deliverable

Report:

- files changed
- foreground transcript path format
- output gating flag added
- when foreground stdout mirroring is disabled/enabled
- status output before/after
- verification commands run
- manual smoke behavior tested
- confirmation that foreground switch still uses `clone_killer()`
- limitations noticed

## Next Handoff After This

After this is complete, the next handoff should address one of these:

- waiting/approval detection from background transcripts
- config file support for agent command paths and aliases
- foreground session handle cleanup on app exit
