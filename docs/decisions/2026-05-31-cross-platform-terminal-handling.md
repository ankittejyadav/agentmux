# Architectural Decision Record: Cross-Platform Terminal Handling

* **Date**: 2026-05-31

## 1. Context & Problem Statement
Early versions of `agentmux` relied on UNIX-specific `libc` calls to configure raw mode on the user's terminal. This approach broke cross-platform compatibility (e.g. compiling on Windows or platforms without a standard `libc`). Additionally, entering raw mode unconditionally caused command execution to hang or panic when run in non-interactive/piped environments (such as testing harnesses or scripts) because stdin was not a true interactive terminal.

## 2. Options Considered

### Option 1: Continue Using Platform-Specific Libc Calls
* **Description**: Wrap raw mode implementation in `#[cfg(unix)]` blocks, and leave Windows without raw terminal support.
* **Pros**:
  * No external terminal-handling dependencies.
* **Cons**:
  * Windows environments fail to build or run.
  * TTY check is manually done through raw file descriptor checks, leading to platform-specific code.

### Option 2: Transition to Crossterm and Implement IsTerminal Detection (Chosen)
* **Description**: Use the `crossterm` crate for cross-platform raw mode enabling/disabling, wrapped in an RAII `RawModeGuard` struct. Use `std::io::IsTerminal` to verify stdin is an active terminal before enabling raw mode.
* **Pros**:
  * Complete portability: builds and runs out-of-the-box on Windows and Unix alike.
  * Safe RAII pattern guarantees terminal is restored to normal cooking mode on exit or panic.
  * Non-interactive execution (e.g. piped tests, `cargo run -- --help`) works smoothly without hangs.
* **Cons**:
  * Adds an additional dependency (`crossterm`).

## 3. Chosen Decision & Rationale
We chose **Option 2**. Transitioning to `crossterm` provides robust, cross-platform terminal raw mode control, and wrapping it in a guard prevents terminal corruption in case of unexpected crashes. Incorporating `std::io::IsTerminal` checks ensures that non-interactive usage does not freeze during raw mode setup.

## 4. Rejected Alternatives
* **Option 1**: Rejected because cross-platform compatibility (specifically keeping Windows and non-standard POSIX environments working) is a key portability requirement for the `agentmux` client.
