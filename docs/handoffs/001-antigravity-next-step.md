# Handoff 001: Direct Native Launch

Status: complete.

## Goal

Implement Step 2 of `agentmux`: direct native launch for the four target agent CLIs.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/command-surface.md`

## Scope

Implement:

```sh
agentmux codex
agentmux claude
agentmux gemini
agentmux agy
```

Do not implement PTYs or TUI yet. This step should only launch the native CLI directly as a foreground child process.

## Expected Behavior

### Native Launch

Each command should launch the matching native CLI:

```text
agentmux codex   -> codex
agentmux claude  -> claude
agentmux gemini  -> gemini
agentmux agy     -> agy
```

The child process should inherit stdin, stdout, stderr, environment, and current working directory.

If the command is missing, print a clear error like:

```text
agentmux: could not launch `agy`; is it installed and on PATH?
```

## Constraints

- Use Rust.
- Keep the implementation std-only for now.
- Do not add dependencies yet.
- Keep code simple and readable.
- Preserve current `init` and `plan` behavior.
- Do not touch native harness config folders like `.codex`, `.gemini`, `.claude`, or `.antigravity-ide`.

## Verification

Run:

```sh
cargo build
cargo run -- version
cargo run -- codex
```

If Codex is installed, `cargo run -- codex` should open the normal Codex CLI in the current terminal.

Also verify existing local state commands still work:

```sh
cargo run -- init
cargo run -- plan auth
```
