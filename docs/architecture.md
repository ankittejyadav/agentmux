# Architecture

## Core Idea

`agentmux` is a thin terminal multiplexer for native agent CLIs.

It should launch each agent as a real interactive process inside its own PTY. The agent should believe it is running in a normal terminal.

```text
agentmux
  |- codex PTY
  |- claude PTY
  |- gemini PTY
  `- agy PTY
```

## Layers

```text
src/
  main.rs
  cli/
  commands/
  config/
  adapters/
  pty/
  tui/
  sessions/
  plans/
```

The first pass can keep these in fewer files and split them as the code grows.

## Responsibilities

### CLI

Parse top-level invocations:

```sh
agentmux init
agentmux plan auth
agentmux codex
agentmux claude
agentmux gemini
agentmux agy
```

### Commands

Handle wrapper actions:

```text
plan
send
status
focus
stop
```

### Config

Project config lives at `.agentmux/config.toml`.

For now, config should keep stable agent keys while allowing each key to map to an executable command and default args:

```toml
[agents.codex]
command = "codex"
args = []
```

The stable key is still used for metadata, run ids, status, and waiting detection. Only the process launch command and default args come from config.

### Adapters

Represent each native harness:

```text
codex
claude
gemini
agy
```

Each adapter should define:

- display name
- command name
- default args
- approval/waiting patterns
- initial handoff prompt format

### PTY

Launch native CLIs directly, not through another agent's shell mode.

This avoids problems like `!agy .` failing inside Codex because the nested process does not receive the same shell, TTY, PATH, or interactive behavior.

### TUI

Long-term view:

```text
main pane: active harness
status line: codex idle | claude waiting | gemini running
modal: approval or wrapper command input
```

### Plans

Plans are file-based so they survive context window limits and can be handed to any agent.

```text
.agentmux/plans/auth/
  plan.md
  tasks.md
  acceptance.md
  constraints.md
```

### Runs

Each delegated execution should get a run folder:

```text
.agentmux/runs/auth-claude-001/
  transcript.ansi
  events.json
  result.md
```

The startup prompt for delegated work should include the exact `result.md` path for that run. Transcripts remain the raw terminal record; `result.md` is the concise human-readable completion summary with files changed, verification run, and blockers.

Run inspection can stay file-based for now: list `.agentmux/runs/`, parse simple `meta.txt` key/value lines, print `result.md` on demand, and expose bounded transcript tail output from command mode. A richer run browser can wait until a real TUI exists.

### Sessions

Foreground and background sessions should converge on one managed session shape before `focus` is implemented.

A managed session needs enough state to support later non-destructive switching:

- session id
- agent name
- plan name, if any
- PTY input writer
- child killer
- transcript path
- status
- output routing mode

Background output should continue writing to `transcript.ansi`. A routing flag can later decide whether the same bytes are also mirrored to the foreground terminal.

The first focus implementation can be a pragmatic MVP:

- foreground keyboard input is redirected to a selected background session
- focused background output is mirrored to stdout
- `detach` restores keyboard input to the original foreground harness

The original foreground harness may still print to stdout while another session is focused. Full visual isolation can come later from a managed foreground session or a real TUI.

Terminal size should come from the real terminal when a PTY is opened. Live resize support can be implemented by polling the terminal size and calling `MasterPty::resize` on the foreground PTY and retained background PTY controls when dimensions change. This avoids a separate crossterm event reader competing with raw stdin forwarding.

To reduce output interleaving before adding a full TUI, foreground output can always be written to a foreground transcript and mirrored to stdout only while the foreground harness is visually active. When a background session is focused, foreground output should continue being captured but should not be mirrored to stdout. `detach` should resume foreground mirroring.

Waiting detection should start as lightweight transcript/output scanning for background sessions. A background process that appears to be waiting for approval, permission, confirmation, or input is still a running process, so waiting state should be tracked separately from process status. The first response path is manual: show the hint in `status`, then let the user `focus <session-id>` and answer the native prompt directly.

After the generic detector works, detection should become agent-aware with small per-agent pattern groups layered over the generic fallback. Keep this as simple data and pure functions for now. Configured launch commands can be added without introducing a large adapter trait system; prompt-template customization can wait.

Before a real TUI exists, waiting visibility should be surfaced with a small same-terminal notification written to stderr when a background session first enters or changes waiting state. Notifications should be deduped; `status` remains the authoritative current state. The response path stays manual: `focus <session-id>`, answer the native prompt, then detach.

## Non-Goals For V1

- no shared memory system
- no mutation of native harness config folders
- no universal normalized permission system
- no GUI
- no full ACP/A2A implementation
