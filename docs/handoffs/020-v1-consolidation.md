# Handoff 020: V1 Consolidation

## Goal

Finish a usable personal V1 in one consolidated handoff.

This handoff combines:

- inline `//` wrapper commands
- real interactive smoke testing across the four native CLIs
- basic session cleanup and safety commands
- install and usage documentation

Keep the implementation lightweight. Do not add a full TUI, panes, fuzzy finder, database, GUI, or auto-approval system.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`
- `docs/handoffs/019-configured-agent-launches.md`
- `src/config.rs`
- `src/commands.rs`
- `src/pty.rs`
- `src/sessions.rs`
- `src/runs.rs`
- `src/waiting.rs`
- `src/main.rs`

## Current State

Completed:

- `Ctrl-G` command mode
- direct native launches for `codex`, `claude`, `gemini`, `agy`
- PTY wrapping and resize handling
- `send <plan> <agent>`
- `send <plan> <agent> --bg`
- `focus <session-id>`
- `detach`
- `stop <session-id>`
- waiting detection and same-terminal waiting notifications
- foreground/background transcripts
- `result.md` handoff instructions
- `runs`
- `result <run-id>`
- `tail <run-id> [lines]`
- `.agentmux/config.toml` configured executable paths and default args

Still missing for personal V1:

- inline `//` command convenience
- documented manual smoke test against real native CLIs
- one-shot background cleanup controls
- practical install and usage docs

## Phase 1: Inline Prefix Commands

Add the configured inline wrapper prefix as a convenience path.

`Ctrl-G` remains the stable fallback.

`.agentmux/config.toml` already contains:

```toml
command_prefix = "//"
```

### Config API

Extend `src/config.rs`:

```rust
pub fn load_command_prefix() -> Result<String, String>
```

Behavior:

- read `.agentmux/config.toml` when present
- fall back to `DEFAULT_CONFIG` when missing
- parse `command_prefix`
- reject empty prefix
- reject non-ASCII prefix for now
- return clear errors:

```text
command_prefix is empty
command_prefix must be ASCII
invalid .agentmux/config.toml: <parser-error>
```

### Inline Behavior

Support inline wrapper commands typed at the beginning of a terminal input line:

```text
//status
//send auth gemini --bg
//focus auth-gemini-1777598200
//detach
//runs
//result <run-id>
//tail <run-id> 40
```

Only detect the prefix at line-start.

Initial state is line-start.

After forwarding normal non-newline input to the active native harness, line-start becomes false.

After forwarding `\n` or `\r`, line-start becomes true.

When line-start is true:

- if typed bytes match the configured prefix, do not forward the prefix
- collect the rest of the line as an `agentmux` wrapper command
- execute through the same command dispatcher used by `Ctrl-G`
- write output to stderr through the existing `write_stderr` path

If bytes do not complete the prefix:

- forward all buffered bytes to the active native harness in order
- continue normal pass-through

Native single-slash commands must still pass through unchanged:

```text
/skills
/goals
/model
```

Typing this should execute wrapper status:

```text
//status
```

Typing this should pass through to the native harness:

```text
/skills
```

Typing this should pass through to the native harness because the prefix is not at line-start:

```text
hello //status
```

### Refactor Requirement

Avoid duplicating the large command match in `handle_command_mode`.

Preferred shape:

```rust
fn execute_wrapper_command(...) -> Option<SessionAction>
```

Use it from both:

- `Ctrl-G` command mode
- inline prefix command mode

Keep all existing wrapper command behavior identical.

## Phase 2: Session Cleanup And Safety

Add minimal process-control commands for personal V1.

### Command: `stop-all`

Add wrapper command:

```text
stop-all
```

Behavior:

- stop all currently running background sessions
- leave the active foreground harness alone
- update each stopped session's in-memory status
- update each stopped session's `meta.txt`
- clear waiting hints for stopped sessions
- print a concise summary:

```text
stopped <n> background session(s)
```

If none are running:

```text
no running background sessions
```

### Command: `cleanup-runs`

Add top-level CLI command, not wrapper command:

```sh
agentmux cleanup-runs --dry-run
agentmux cleanup-runs --older-than 7
```

Behavior:

- inspect `.agentmux/runs/`
- `--dry-run` prints the run folders that would be deleted
- `--older-than <days>` deletes run folders older than that many days
- reject missing or invalid args with:

```text
usage: agentmux cleanup-runs --dry-run|--older-than <days>
```

Safety rules:

- never delete `.agentmux/config.toml`
- never delete `.agentmux/plans/`
- never delete native harness folders
- never delete running sessions from in-memory state
- refuse `--older-than 0`

This does not need to be sophisticated. Use filesystem modified time.

## Phase 3: Manual V1 Smoke Test Doc

Create:

```text
docs/v1-smoke-test.md
```

Document a manual smoke checklist for the real native CLIs.

The doc should cover:

- `cargo run -- init`
- `cargo run -- plan smoke`
- configured launch check with `cargo run -- gemini --version`
- direct launch for each installed native CLI:
  - `cargo run -- codex`
  - `cargo run -- claude`
  - `cargo run -- gemini`
  - `cargo run -- agy`
- native slash command pass-through:
  - `/skills`
  - `/goals`
  - `/model`
- `Ctrl-G`, then `status`
- `//status`
- `send smoke <agent> --bg`
- `runs`
- `tail <run-id>`
- `focus <session-id>`
- answer a native prompt manually if one appears
- `detach`
- `stop <session-id>`
- `stop-all`
- `cleanup-runs --dry-run`

The doc should include a small result table:

```text
| Check | Result | Notes |
| --- | --- | --- |
| codex direct launch | pass/fail/skipped | |
```

If a native CLI is not installed, the expected result is `skipped`, not a Rust failure.

## Phase 4: Install And Usage Docs

Update `README.md` with practical usage sections:

- install from repo:

```sh
cargo install --path .
```

- initialize a project:

```sh
agentmux init
agentmux plan auth
```

- launch:

```sh
agentmux codex
```

- configure native commands:

```toml
[agents.codex]
command = "/opt/homebrew/bin/codex"
args = ["--model", "gpt-5"]
```

- wrapper commands:

```text
Ctrl-G, then status
//status
//send auth gemini --bg
//runs
//tail <run-id>
```

- cleanup:

```sh
agentmux cleanup-runs --dry-run
agentmux cleanup-runs --older-than 7
```

Keep the README short. Link deeper details to existing docs.

## Tests

Add focused unit tests.

Config prefix tests:

- default prefix is `//`
- custom prefix loads
- empty prefix errors
- non-ASCII prefix errors
- invalid TOML errors

Inline prefix helper tests if a helper is introduced:

- `//status` is detected as wrapper command at line-start
- `/skills` is passed through as native input
- `hello //status` is passed through as native input
- prefix mismatch forwards buffered bytes in order

Command parser/session tests:

- `stop-all` parser variant
- stopping none returns `no running background sessions`
- stopped sessions clear waiting hints

Cleanup tests:

- dry-run lists old fixture runs without deleting them
- `--older-than 0` is rejected
- invalid args print usage
- cleanup only touches isolated temp run fixtures in tests

Do not add tests that require native `codex`, `claude`, `gemini`, or `agy` binaries to exist.

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
cargo run -- cleanup-runs --dry-run
```

Manual smoke testing is documented in `docs/v1-smoke-test.md`. Run as much of it as the local machine supports and report skipped CLIs clearly.

## Critical Regression Guards

Native slash commands must still pass through unchanged.

`Ctrl-G` command mode must still work.

Inline prefix commands must only trigger at line-start.

Focused background sessions must remain interactive.

Waiting notifications must still work.

Foreground output gating must remain intact.

Configured agent executable paths/default args must continue to work.

Do not alter `.codex`, `.gemini-cli`, `.antigravity-ide`, or any native harness state folders.

Do not auto-approve native prompts.

Do not reintroduce the foreground switch deadlock. In `run_pty_command`, keep using:

```rust
let mut child_killer = child.clone_killer();
```

Do not wrap the foreground child in `Arc<Mutex<child>>` while another thread calls `wait()`.

## Expected Response

When complete, report:

- files changed
- inline prefix behavior implemented
- `stop-all` behavior
- `cleanup-runs` behavior
- install/usage docs added
- manual smoke doc path
- confirmation that `/skills`, `/goals`, and `/model` still pass through
- confirmation that `Ctrl-G` still works
- verification commands run
- manual smoke checks run or skipped
- any limitations noticed
