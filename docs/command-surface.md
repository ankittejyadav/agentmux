# Command Surface

Keep commands minimal.

## Native Commands

Single slash commands always belong to the active native harness:

```text
/skills
/goals
/model
```

`agentmux` must pass these through unchanged.

## Wrapper Commands

Target command set:

```text
help
status
send <plan> <agent>
focus <session-id>
detach
stop <session-id>
stop-all
runs
result <run-id>
tail <run-id> [lines]
```

Current command mode implements:

```text
help
status
send <plan> <agent>
send <plan> <agent> --bg
focus <session-id>
detach
stop <session-id>
stop-all
runs
result <run-id>
tail <run-id> [lines]
```

Top-level utility commands:

```text
agentmux cleanup-runs --dry-run
agentmux cleanup-runs --older-than <days>
```

Inline wrapper form:

```text
//send auth claude
//status
//focus <session-id>
//detach
//stop-all
//runs
//tail <run-id>
```

The prefix is configurable. `Ctrl-G` command mode remains the stable fallback.

## Agent Names

Use short names:

```text
codex
claude
gemini
agy
```

These names stay stable while each agent's executable command and default args come from `.agentmux/config.toml`.

## Example Flow

```sh
cd project
agentmux codex
```

Native Codex session:

```text
Discuss architecture normally.
Use /skills and /goals normally.
```

Wrapper command:

```text
Ctrl-G, then status
Ctrl-G, then send auth agy
Ctrl-G, then send auth gemini --bg
Ctrl-G, then focus <session-id>
Ctrl-G, then detach
Ctrl-G, then stop <session-id>
Ctrl-G, then runs
Ctrl-G, then result <run-id>
Ctrl-G, then tail <run-id>
```

For the current manual workflow, the reasoning agent writes the plan files and the user opens Antigravity manually with the handoff doc.
