# V1 Roadmap

## Step 0: Repo And Docs

Status: complete.

Deliverables:

- Rust project exists
- decisions saved in docs
- minimal CLI builds
- first manual handoff doc exists

## Step 1: Local Project State

Status: complete.

Commands:

```sh
agentmux init
agentmux plan <name>
```

Behavior:

- create `.agentmux/`
- create default config
- create plan folder
- write empty `plan.md`, `tasks.md`, `acceptance.md`, `constraints.md`

## Step 2: Direct Native Launch

Status: complete.

Commands:

```sh
agentmux codex
agentmux claude
agentmux gemini
agentmux agy
```

Behavior:

- launch the selected native CLI directly
- no TUI yet
- prove commands and PATH resolution work

## Step 3: PTY Wrapper

Status: complete.

Behavior:

- run one native harness inside a PTY
- pass input/output through
- preserve native slash commands and approval prompts

## Step 3.5: PTY Portability Hardening

Status: complete, but Windows compile has not been checked because the Windows Rust target is not installed locally.

Behavior:

- remove Unix-only raw mode from core PTY code
- keep Windows builds possible
- avoid manual PATH checks that miss `.exe`, `.cmd`, and `PATHEXT`
- preserve current macOS behavior

## Step 4: Wrapper Command Mode

Status: complete.

Behavior:

- add a hotkey for wrapper commands
- implement `status`
- avoid inline `//` until PTY pass-through is stable

## Step 4.5: Single-Session Plan Send

Status: complete.

Behavior:

- implement `send <plan> <agent>` inside command mode
- exit current harness and replace it with the target agent for now
- inject a compact handoff prompt that points at `.agentmux/plans/<plan>/`
- avoid multi-session until handoff UX is proven

## Step 5A: Background Send MVP

Status: complete.

Behavior:

- implement `send <plan> <agent> --bg`
- keep the current active session running
- launch the target agent in a background PTY
- capture background output to a run transcript
- extend `status` to list background sessions

## Step 5B: Background Stop

Status: complete.

Behavior:

- implement `stop <session-id>`
- terminate a running background session
- update in-memory status and `meta.txt`
- keep foreground harness active

## Step 5C: Session State Refactor

Status: complete.

Behavior:

- move background session state out of command parsing
- keep parser unit tests pure and deterministic
- centralize status/stop/session metadata logic
- preserve existing command behavior

## Step 5D: Session IO Refactor

Status: complete.

Behavior:

- make background sessions keep their PTY input writer instead of dropping it after startup prompt injection
- add output routing state so a session can later move between transcript-only background logging and foreground display
- keep `send <plan> <agent> --bg`, `status`, and `stop <session-id>` behavior unchanged
- do not add `focus` yet; prepare the session model first

## Step 5E: Focus And Detach MVP

Status: complete.

Behavior:

- implement `focus <session-id>` in command mode
- route future keyboard input to the selected running background session
- mirror the focused background session output to the foreground terminal
- implement `detach` so input returns to the original foreground harness and background output returns to transcript-only logging
- keep the current foreground harness process alive while focused elsewhere

## Step 5F: Focus Hardening And Resize

Status: complete.

Behavior:

- automatically detach if the focused background session exits or is stopped
- show focused/background output mode more clearly in `status`
- add terminal-size detection so PTYs start with the real terminal dimensions instead of fixed `80x24`
- add lightweight resize polling and call `MasterPty::resize` for managed PTYs when the terminal size changes
- keep the no-TUI approach for now

## Step 5G: Foreground Output Gating

Status: complete.

Behavior:

- write foreground harness output to a transcript
- when a background session is focused, stop mirroring foreground output to stdout
- keep focused background output visible
- restore foreground stdout mirroring on `detach`
- keep command mode visible through stderr

## Step 5H: Waiting Detection MVP

Status: complete.

Behavior:

- scan background PTY output for generic approval, permission, confirmation, and input-waiting text
- keep detection lightweight and transcript-based
- show likely waiting sessions in `status`
- keep native approval flows native; the user responds by focusing the session
- do not auto-approve anything
- do not add a full TUI or popup yet

## Step 5I: Adapter-Specific Waiting Patterns

Status: complete.

Behavior:

- make waiting detection agent-aware while keeping generic fallback patterns
- add small per-agent pattern groups for `codex`, `claude`, `gemini`, and `agy`
- keep matching pure and unit-tested
- avoid a large adapter trait system until command/path config is needed
- keep native approvals native; no auto-approval

## Step 5J: Waiting Notifications MVP

Status: complete.

Behavior:

- notify the user in the same terminal when a background session first enters a waiting state
- dedupe repeated waiting detections so the terminal is not spammed
- keep `status` as the source of truth for current waiting sessions
- keep approval manual through `focus <session-id>`
- do not add a full TUI, panes, popup UI, or auto-approval yet

## Step 5: Multi-Session

Status: baseline complete; richer visual management can come later.

Behavior:

- start background agent sessions
- track running/waiting/done states
- focus sessions from one terminal

## Step 6: Run Results And Handoff Prompts

Status: complete.

Command:

```text
send <plan> <agent>
```

Behavior:

- launch target agent
- inject a compact prompt pointing at `.agentmux/plans/<plan>/`
- ask the agent to write `.agentmux/runs/<run>/result.md`
- create a result file placeholder next to `meta.txt` and `transcript.ansi`
- include the exact result path in foreground and background handoff prompts
- keep transcripts as the raw record and `result.md` as the human-readable completion summary

## Step 6A: Run Inspection Commands

Status: complete.

Commands:

```text
runs
result <run-id>
```

Behavior:

- list recent run folders from `.agentmux/runs/`
- show run id, status, agent, result path, and transcript path when available
- print a run's `result.md` content in command mode
- keep output lightweight on stderr
- avoid a full TUI, fuzzy finder, or external pager for now

## Step 6B: Transcript Tail Command

Status: complete.

Command:

```text
tail <run-id> [lines]
```

Behavior:

- print the last lines of a run transcript from command mode
- default to a small line count such as 80
- cap requested line counts to avoid flooding the terminal
- keep transcript output raw for now
- avoid a full pager or live transcript following

## Step 6C: Configured Agent Launch Commands

Status: complete.

Behavior:

- read `.agentmux/config.toml`
- keep the four stable agent names: `codex`, `claude`, `gemini`, `agy`
- allow each agent key to map to a command path and default args
- use configured launch settings for direct launches, foreground switches, and background sends
- keep native harness stdin/stdout/stderr behavior unchanged
- fail with clear errors when config is invalid or an agent entry is missing

## Step 7: Same-Window Approval Visibility

Status: partial. Waiting detection, status hints, notifications, focus, and manual approval response are implemented. Full status-line or popup UI is deferred.

Behavior:

- adapter-specific waiting pattern detection
- status line shows blocked sessions
- popup can focus session or route simple approval input

## Step 8: V1 Consolidation

Status: complete.

Behavior:

- support configured prefix like `//`
- preserve `Ctrl-G` as fallback
- add basic session cleanup and run cleanup controls
- document manual V1 smoke testing
- document install and practical usage
