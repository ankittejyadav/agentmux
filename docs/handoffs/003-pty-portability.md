# Handoff 003: PTY Portability Hardening

Status: complete. Windows compile was not checked because the Windows Rust target is not installed locally.

## Goal

Make the current single-session PTY wrapper cross-platform-ready before adding wrapper command mode.

## Context

Read these files first:

- `README.md`
- `docs/decisions.md`
- `docs/architecture.md`
- `docs/v1-roadmap.md`
- `docs/handoffs/002-pty-wrapper.md`
- `src/main.rs`
- `src/pty.rs`

## Current State

Step 3 is implemented and passes local macOS checks.

Known issues:

- `src/pty.rs` imports `std::os::unix::io::AsRawFd`, so the project will not compile on Windows.
- raw mode uses `libc::termios`, which is Unix-specific.
- manual PATH lookup checks only `dir.join(command).exists()`, which misses Windows executable resolution such as `.exe`, `.cmd`, `.bat`, and `PATHEXT`.

These need to be fixed before Step 4.

## Scope

Keep the current behavior:

```sh
agentmux codex
agentmux claude
agentmux gemini
agentmux agy
```

Do not implement:

- wrapper command mode
- inline `//`
- TUI
- background sessions
- approval detection
- plan handoff

## Required Changes

### Raw Mode

Replace Unix-only raw mode with a cross-platform approach.

Preferred option:

```toml
crossterm = "0.29"
```

Use:

```rust
crossterm::terminal::enable_raw_mode()
crossterm::terminal::disable_raw_mode()
```

Then remove the direct `libc` dependency unless it is still needed elsewhere.

If using `cfg(unix)` instead, provide a Windows-safe no-op fallback so the project still compiles on Windows.

### PATH Handling

Remove the manual pre-check for whether the command exists on PATH.

Let `portable-pty` attempt to spawn the command. If spawn fails, map the error to the existing clean message when the failure appears to be a missing executable:

```text
agentmux: could not launch `agy`; is it installed and on PATH?
```

This avoids breaking Windows command shims like `codex.cmd` or `gemini.cmd`.

### Thread Lifecycle

Keep the current blocking stream bridge if it works, but avoid adding complexity.

It is acceptable for output/input threads not to be perfect yet. This step is about portability and preserving native behavior.

## Verification

Run locally:

```sh
cargo fmt
cargo build
cargo test
cargo run -- version
cargo run -- codex --help
cargo run -- gemini --version
```

Also run:

```sh
rustup target list --installed
```

If a Windows Rust target is installed, run:

```sh
cargo check --target x86_64-pc-windows-msvc
```

If no Windows target is installed, do not install it unless asked. Just report that Windows cross-check was skipped because the target is not installed.

## Deliverable

Report:

- files changed
- dependencies added or removed
- whether `libc` was removed
- verification commands run
- whether Windows compile was actually checked
- any remaining portability risks
