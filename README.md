# agentmux

`agentmux` is a CLI-first terminal multiplexer for agent CLIs.

It keeps Codex CLI, Claude Code, Gemini CLI, and Antigravity CLI working as their native tools while adding a thin shared layer for plans, handoffs, sessions, status, same-window approval visibility, and run log management.

## Installation

Install `agentmux` from the local repository directory:

```sh
cargo install --path .
```

## Initialization

Initialize a new `agentmux` project state and create a new plan:

```sh
agentmux init
agentmux plan auth
```

## Configuration

Configure native agent executable command paths and default arguments in `.agentmux/config.toml`:

```toml
command_prefix = "//"

[agents.codex]
command = "/opt/homebrew/bin/codex"
args = ["--model", "gpt-5"]
```

## Launching Agents

Launch any of the supported native CLIs directly:

```sh
agentmux codex
agentmux claude
agentmux gemini
agentmux agy
```

## Usage and Wrapper Commands

Native commands (e.g. `/skills`, `/goals`, `/model`) pass directly to the active native CLI unchanged.

To run `agentmux` wrapper commands, either prefix the command at the beginning of a line with the configured `command_prefix` (default `//`), or press `Ctrl-G` to open the command prompt.

Available wrapper commands:
* `status` or `//status`: show active foreground and background sessions.
* `send <plan> <agent> [--bg]` or `//send <plan> <agent> [--bg]`: dispatch plans.
* `focus <session-id>` or `//focus <session-id>`: focus a background session.
* `detach` or `//detach`: return to the active foreground harness.
* `stop <session-id>` or `//stop <session-id>`: terminate a background session.
* `stop-all` or `//stop-all`: terminate all background sessions.
* `runs` or `//runs`: list recent run folders.
* `result <run-id>` or `//result <run-id>`: view result markdown.
* `tail <run-id> [lines]` or `//tail <run-id> [lines]`: tail run transcripts (maximum 500 lines).

## Run Directory Cleanup

Clean up aged run log directories from `.agentmux/runs/`:

```sh
agentmux cleanup-runs --dry-run
agentmux cleanup-runs --older-than 7
```

## Documentation

* [Manual V1 Smoke Checklist](docs/v1-smoke-test.md)
* Detailed decisions and roadmap are archived in `docs/`
# AgentMux
