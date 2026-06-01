# AgentMux Portfolio

## Project Summary

AgentMux is a CLI-first terminal multiplexer for coordinating multiple AI coding agent harnesses from one project-local workflow. It currently targets Codex CLI, Claude Code, Gemini CLI, and Antigravity CLI through the stable agent keys `codex`, `claude`, `gemini`, and `agy`.

The core idea is harness-preserving orchestration. Instead of replacing each native CLI with a new UI, protocol, or agent runtime, AgentMux wraps the existing interactive terminal process and keeps it working normally. Native slash commands, approval prompts, model controls, permissions, local config, and terminal behavior continue to belong to the original harness.

AgentMux adds a thin coordination layer around those native tools: project plans, task handoffs, foreground and background sessions, same-terminal status, focus and detach, waiting detection, transcripts, result files, and safe run cleanup.

## Problem

AI coding agents are becoming powerful, but each agent usually lives inside its own harness. A user might want to use one high-reasoning agent for architecture decisions, another for implementation, another for verification, and another for long-running edits. In practice, this creates friction:

- Plans must be copied manually between tools.
- Each harness has different commands, approval flows, logs, and state folders.
- Running multiple agents means juggling terminals, editor panels, or GUI windows.
- Background agents can silently stall on approval prompts.
- Results are scattered across chat history, terminal scrollback, and tool-specific folders.

AgentMux solves this by making the terminal session itself the orchestration surface.

## Project Description

AgentMux runs native agent CLIs inside managed PTY sessions. The active harness still behaves like the real harness. For example, typing `/skills`, `/goals`, or `/model` goes directly to the native CLI. AgentMux only intercepts its own wrapper commands through `Ctrl-G` command mode or a configurable line-start prefix such as `//`.

A typical workflow looks like this:

```sh
cd my-project
agentmux init
agentmux plan auth
agentmux codex
```

Inside the Codex CLI session, the user can discuss architecture and write plan files under `.agentmux/plans/auth/`. Once the plan is ready, the user can delegate the work:

```text
//send auth gemini --bg
```

AgentMux starts Gemini CLI in a background PTY, injects a compact handoff prompt pointing at the plan folder, records its transcript, asks it to write a `result.md`, and keeps the current foreground harness alive. If the background session appears to be waiting for approval, AgentMux prints a same-terminal notification:

```text
agentmux: auth-gemini-1777598200 waiting(permission); Ctrl-G focus auth-gemini-1777598200
```

The user can then focus the background session, respond to the native approval prompt exactly as they would in the original CLI, and detach back to the original foreground session.

## Unique Selling Criteria

AgentMux is designed around a specific gap: lightweight orchestration for existing CLI harnesses without forcing agents into a new platform.

Its unique angle is the combination of:

- Native harness preservation: existing slash commands, permissions, approvals, and terminal behavior remain intact.
- CLI-first orchestration: the main interface is the terminal, not an editor plugin or dashboard.
- Protocol-light integration: AgentMux does not require ACP, MCP, JSON-RPC, plugins, webhooks, or agent SDK changes.
- Same-window delegation: users can hand off a plan, monitor background work, inspect results, and answer approvals without switching terminal sessions.
- File-based project memory: plans, transcripts, metadata, and results live in `.agentmux/` inside the project.
- Manual approval safety: AgentMux detects likely waiting states but does not auto-approve commands or bypass native permission flows.

Most adjacent tools focus on custom agent runtimes, editor-integrated panels, chat UIs, or full terminal dashboards. AgentMux instead treats the native CLI harness as the unit of orchestration and adds only the minimum coordination layer needed to make multi-agent work practical.

## Architecture Decisions

### 1. CLI-First, Not GUI-First

AgentMux starts from the terminal because the target tools are already terminal-native. This keeps the first version lightweight and avoids rebuilding editor features, chat panels, or a full dashboard.

Decision:
Use a Rust CLI binary that launches and manages agent CLIs from the current project folder.

Why:
- Works on projects regardless of editor.
- Matches the existing workflow of Codex CLI, Claude Code, Gemini CLI, and Antigravity CLI.
- Keeps native harnesses usable exactly as-is.

Tradeoff:
- No rich visual panes or graphical run browser in V1.

### 2. PTY Wrapping Instead Of API Integration

AgentMux uses PTYs to run interactive CLI harnesses. This lets each agent behave like it is running directly in a terminal.

Decision:
Use `portable-pty` for process spawning, terminal IO, resizing, foreground sessions, and background sessions.

Why:
- Preserves native input/output behavior.
- Supports interactive prompts and terminal UI behavior.
- Avoids requiring each agent provider to expose a compatible API.

Tradeoff:
- PTY output is text/ANSI oriented, so structured state must be inferred or recorded separately.

### 3. Native Commands Pass Through By Default

AgentMux does not own single-slash commands. Native commands such as `/skills`, `/goals`, and `/model` are forwarded unchanged to the active harness.

Decision:
Use `Ctrl-G` and a configurable line-start prefix such as `//` for AgentMux commands.

Why:
- Avoids collisions with existing harness command surfaces.
- Keeps the wrapper minimal.
- Makes AgentMux feel like a control layer rather than a replacement shell.

Tradeoff:
- Inline prefix commands only trigger at the absolute start of a terminal line.

### 4. Project-Local State

AgentMux stores its own state under `.agentmux/`.

Decision:
Use this folder structure:

```text
.agentmux/
  config.toml
  plans/
    <plan>/
      plan.md
      tasks.md
      acceptance.md
      constraints.md
  runs/
    <run-id>/
      meta.txt
      transcript.ansi
      result.md
```

Why:
- Keeps orchestration artifacts portable with the repo.
- Avoids mutating native harness folders such as `.codex`, `.gemini-cli`, or `.antigravity-ide`.
- Gives agents a stable file target for plans and final results.

Tradeoff:
- AgentMux does not attempt to make native harness state folders interoperable.

### 5. Stable Agent Keys With Configurable Launch Commands

AgentMux uses stable logical names for agents while allowing each command path and default argument list to be configured.

Decision:
Use `.agentmux/config.toml`:

```toml
command_prefix = "//"

[agents.codex]
command = "codex"
args = []

[agents.claude]
command = "claude"
args = []

[agents.gemini]
command = "gemini"
args = []

[agents.agy]
command = "agy"
args = []
```

Why:
- Metadata and waiting detection can use stable keys.
- Users can customize binary paths and default args per machine.
- Direct launches, foreground switches, and background sends share the same launch config.

Tradeoff:
- V1 intentionally limits agent keys to the four supported harnesses.

### 6. Background Sessions With Focus And Detach

AgentMux supports background agent work without hiding interactive prompts forever.

Decision:
Background sessions keep their PTY writer and control handles. Output is logged to transcripts by default. `focus <session-id>` routes keyboard input and output to that background session. `detach` restores input/output to the original foreground harness.

Why:
- Users can keep working while another agent runs.
- Native approvals remain manual and visible.
- No separate terminal is required for each agent.

Tradeoff:
- V1 does not include full split-pane rendering or a persistent status bar.

### 7. Waiting Detection Without Auto-Approval

AgentMux scans background PTY output for likely approval, permission, confirmation, or input prompts.

Decision:
Use lightweight text pattern matching with generic fallback patterns and agent-specific patterns for `codex`, `claude`, `gemini`, and `agy`.

Why:
- Detects common stalled states quickly.
- Avoids bypassing native safety systems.
- Keeps the implementation simple and testable.

Tradeoff:
- Text matching can produce false positives or miss prompts if CLIs change wording.

### 8. Transcripts And Result Files

Each run records a raw transcript and a human-readable result file.

Decision:
Use `transcript.ansi` as the raw terminal record and `result.md` as the expected completion report.

Why:
- The transcript preserves what happened.
- The result file gives the user a stable summary to inspect from AgentMux.
- Handoff prompts can tell agents exactly where to write their final report.

Tradeoff:
- Agents may ignore or partially follow the requested `result.md` format.

## Current Feature Set

- Initialize project state with `agentmux init`.
- Create plan folders with `agentmux plan <name>`.
- Launch native harnesses with `agentmux codex`, `agentmux claude`, `agentmux gemini`, and `agentmux agy`.
- Preserve native slash commands and approval prompts.
- Run wrapper commands through `Ctrl-G`.
- Run inline wrapper commands with `//` at line start.
- Dispatch plans to foreground or background agents.
- Focus and detach background sessions.
- Stop one session or all background sessions.
- Track waiting states and print same-terminal notifications.
- List runs and inspect `result.md`.
- Tail raw run transcripts.
- Clean up old run directories safely.

## Command Examples

```sh
agentmux init
agentmux plan auth
agentmux codex
```

```text
//status
//send auth gemini --bg
//runs
//tail auth-gemini-1777598200 80
//focus auth-gemini-1777598200
//detach
//stop auth-gemini-1777598200
//stop-all
```

```sh
agentmux cleanup-runs --dry-run
agentmux cleanup-runs --older-than 7
```

## Tech Stack

- Rust: core CLI application and process orchestration.
- portable-pty: cross-platform PTY management for interactive native CLI sessions.
- crossterm: terminal raw mode and terminal sizing support.
- serde: typed config deserialization.
- toml: `.agentmux/config.toml` parsing.
- Standard library concurrency primitives: session tracking, shared input handles, process lifecycle management.
- Filesystem-based state: plans, metadata, transcripts, and results.
- Native CLI tools: Codex CLI, Claude Code, Gemini CLI, and Antigravity CLI.

## Pros

- Preserves native harness functionality instead of replacing it.
- Lets users orchestrate multiple agents from one terminal window.
- Keeps approval flows manual and visible.
- Does not require agent vendors to support a shared protocol.
- Uses simple, inspectable project-local files.
- Records transcripts and final results for later review.
- Lightweight enough to work alongside existing editor or GUI workflows.

## Cons And Limitations

- V1 supports only four configured agent keys.
- Waiting detection is heuristic and based on terminal text.
- No full TUI, split-pane view, fuzzy finder, or graphical dashboard yet.
- Background sessions can still stall if an agent uses an unexpected prompt format.
- `result.md` quality depends on whether the delegated agent follows instructions.
- Shell aliases are not supported directly; users should configure executable paths in `.agentmux/config.toml`.
- Windows compile verification has not been completed locally if the Windows Rust target is not installed.

## Future Improvements

- Richer visual status line or optional terminal dashboard.
- More robust prompt adapters per harness.
- Live transcript follow mode.
- Result validation and summary extraction.
- User-defined agent keys beyond the initial four.
- Optional integrations with editor extensions or external orchestration protocols.
- Cross-platform release builds and installer packaging.

## Open Source License

AgentMux is released under the MIT License. See `LICENSE` in the repository root.
