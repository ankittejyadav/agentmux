# Handoff 019: Configured Agent Launches

## Goal

Make `.agentmux/config.toml` the source of truth for how each native agent CLI is launched.

This keeps the public agent names stable while letting the user rename binaries, use absolute paths, or add default args without changing Rust code.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/018-transcript-tail-command.md`
- `src/main.rs`
- `src/commands.rs`
- `src/pty.rs`
- `src/sessions.rs`
- `src/runs.rs`
- `src/waiting.rs`
- `Cargo.toml`

## Current State

`agentmux init` already creates:

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

But the runtime still launches the hardcoded agent name directly. For example, `agentmux codex` always spawns `codex`, and `send auth agy --bg` always spawns `agy`.

## Scope

Add config-based launch resolution for the four existing agent keys:

```text
codex
claude
gemini
agy
```

Do not add aliases yet.

Do not add a config editing command yet.

Do not change native slash-command pass-through.

Do not change wrapper command mode.

## Dependency

Use a real TOML parser instead of hand-rolled string parsing.

Add dependencies:

```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

If the local Cargo registry needs to fetch these dependencies, that is acceptable for this handoff.

## New Module

Create `src/config.rs`.

Move the default config string out of `main.rs` and into:

```rust
pub const DEFAULT_CONFIG: &str = r#"..."#;
```

`main.rs` should use `crate::config::DEFAULT_CONFIG` when writing `.agentmux/config.toml`.

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLaunch {
    pub key: String,
    pub command: String,
    pub args: Vec<String>,
}

pub fn load_agent_launch(agent_key: &str) -> Result<AgentLaunch, String>
```

It is fine to add private helpers such as:

```rust
fn load_agent_launch_from_path(path: &std::path::Path, agent_key: &str) -> Result<AgentLaunch, String>
fn parse_agent_launch(config_text: &str, agent_key: &str) -> Result<AgentLaunch, String>
```

The test-facing helper can stay private inside `#[cfg(test)]` tests if preferred.

## Config Loading Behavior

`load_agent_launch(agent_key)` should:

- read `.agentmux/config.toml` from the current working directory when present
- fall back to `DEFAULT_CONFIG` when the file is missing
- parse `[agents.<agent_key>]`
- return the configured `command`
- return the configured `args`
- reject an empty command
- return clear string errors for invalid config, missing agent entries, or invalid field types

Expected error text can be concise, for example:

```text
invalid .agentmux/config.toml: <parser-error>
agent config missing: <agent>
agent command is empty: <agent>
```

## Launch Behavior

Use the configured command and default args in all launch paths:

```text
agentmux codex <extra args>
agentmux claude <extra args>
agentmux gemini <extra args>
agentmux agy <extra args>
send <plan> <agent>
send <plan> <agent> --bg
```

Argument order:

```text
configured args first, user/runtime args second
```

Example:

```toml
[agents.codex]
command = "/opt/homebrew/bin/codex"
args = ["--model", "gpt-5"]
```

Then:

```sh
agentmux codex --dangerously-bypass-approvals-and-sandbox
```

should spawn:

```text
/opt/homebrew/bin/codex --model gpt-5 --dangerously-bypass-approvals-and-sandbox
```

Do not invoke a shell for this. Pass the configured command directly to `portable_pty::CommandBuilder`.

Shell aliases are still not supported; configured command paths are the supported solution.

## Preserve Agent Keys

Keep using the stable agent key for:

- `send` validation
- run IDs
- `meta.txt` agent field
- foreground run IDs
- waiting pattern detection
- status output

Only the executable command should come from config.

For example, if `[agents.codex].command = "/tmp/my-codex-wrapper"`, the foreground run should still look like:

```text
.agentmux/runs/foreground-codex-<timestamp>/
```

and waiting detection should still use `codex`.

## PTY Changes

In `src/pty.rs`, treat the current `command` argument as the agent key.

Suggested internal naming:

```rust
pub fn run_pty_command(
    agent_key: &str,
    args: Vec<String>,
    startup_prompt: Option<String>,
) -> Result<SessionAction, String>
```

Inside it:

- call `crate::config::load_agent_launch(agent_key)?`
- build `CommandBuilder::new(&launch.command)`
- pass `launch.args` first
- pass runtime `args` after
- keep `agent_key` for metadata, status, waiting detection, foreground run ID, and error context

For background sessions, `spawn_background_pty(plan, agent_key)` should also call `load_agent_launch(agent_key)` and spawn `launch.command` with `launch.args`.

It is acceptable to change `spawn_background_pty` from `io::Result<()>` to `Result<(), String>` so config errors flow cleanly into command mode.

## Spawn Error Message

If the configured executable is not found, return a clean message that includes both the agent key and command path:

```text
could not launch agent `<agent>` command `<command>`; is it installed and configured correctly?
```

Other spawn errors can keep the existing style, but include the command path if possible.

## Tests

Add focused unit tests for `config.rs`:

- default config loads `codex` as command `codex` with empty args
- custom config loads an absolute command and args
- missing agent returns `agent config missing: <agent>`
- empty command returns `agent command is empty: <agent>`
- invalid TOML returns an `invalid .agentmux/config.toml` error

Add or update PTY-adjacent tests only if a pure helper is introduced for argument merging. Do not add tests that require native `codex`, `claude`, `gemini`, or `agy` binaries to exist.

## Verification

Run:

```sh
cargo fmt -- --check
cargo build
cargo test
cargo run -- init
cargo run -- plan auth
cargo run -- version
cargo run -- gemini --version
```

If `gemini` is not installed on the current machine, report that clearly instead of treating it as a Rust failure.

## Critical Regression Guards

Native slash commands must still pass through unchanged.

`Ctrl-G` command mode must still work.

`send`, `send --bg`, `focus`, `detach`, `stop`, `runs`, `result`, and `tail` must keep their current command syntax.

Do not alter `.codex`, `.gemini-cli`, `.antigravity-ide`, or any native harness state folders.

Do not reintroduce the foreground switch deadlock. In `run_pty_command`, keep using:

```rust
let mut child_killer = child.clone_killer();
```

Do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

## Expected Response

When complete, report:

- files changed
- exact config API added
- how direct launch uses configured command and args
- how foreground switch and background send use configured command and args
- confirmation that agent keys remain stable in metadata and waiting detection
- verification commands run
- any limitations noticed
