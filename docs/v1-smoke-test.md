# Manual V1 Smoke Checklist

This document details the manual smoke checklist for verifying agentmux V1 capabilities against installed native agent CLIs.

## Checklist

### 1. Initialization and Setup
- [ ] Run `cargo run -- init` to initialize project state.
- [ ] Run `cargo run -- plan smoke` to verify starter document structure creation.
- [ ] Run `cargo run -- gemini --version` to check custom agent CLI execution through configured command launcher.

### 2. Native CLI Launches
Launch each available CLI to verify direct launches:
- [ ] `cargo run -- codex`
- [ ] `cargo run -- claude`
- [ ] `cargo run -- gemini`
- [ ] `cargo run -- agy`

### 3. Native Pass-through
Inside any launched native CLI:
- [ ] Type `/skills` and confirm it passes directly to the native agent CLI.
- [ ] Type `/goals` and confirm it passes directly to the native agent CLI.
- [ ] Type `/model` and confirm it passes directly to the native agent CLI.

### 4. Wrapper Commands and Prefix Interception
- [ ] Press `Ctrl-G` followed by `status` to print active session status.
- [ ] Type `//status` at the beginning of a line to verify prefix interception.
- [ ] Type `//send smoke gemini --bg` to start a background session.
- [ ] Type `//runs` to view the recent runs list.
- [ ] Type `//tail <run-id>` to verify raw transcript tailing.
- [ ] Type `//focus <session-id>` to focus the background session.
- [ ] Manually answer a prompt if the native agent asks for confirmation.
- [ ] Press `Ctrl-G`, then type `detach` to return to the active foreground harness.
- [ ] Type `//stop <session-id>` to stop a background session.
- [ ] Type `//stop-all` to gracefully terminate all remaining background sessions.

### 5. Run Log Cleanup
- [ ] Run `cargo run -- cleanup-runs --dry-run` to dry-run delete aged run directories.

## Results Table

| Check | Result | Notes |
| --- | --- | --- |
| `init` and `plan` | Pass | |
| `gemini --version` | Pass | |
| `codex` direct launch | Skipped | CLI not locally installed |
| `claude` direct launch | Skipped | CLI not locally installed |
| `gemini` direct launch | Pass | |
| `agy` direct launch | Skipped | CLI not locally installed |
| Native pass-through | Pass | Checked with gemini CLI |
| `Ctrl-G` status | Pass | |
| `//status` prefix | Pass | |
| `//send` background | Pass | |
| `//runs` list | Pass | |
| `//tail` command | Pass | |
| `//focus` session | Pass | |
| `//detach` action | Pass | |
| `//stop` session | Pass | |
| `//stop-all` command | Pass | |
| `cleanup-runs --dry-run` | Pass | |
