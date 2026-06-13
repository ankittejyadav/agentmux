# Architectural Decision Record: Robust Command Prefix Parsing and Terminal Title Updates

* **Date**: 2026-05-31

## 1. Context & Problem Statement
During testing of the `agentmux` multiplexer wrapper, two usability issues were identified:
1. **Redundant Command Prefix Errors**: Users entering command mode via the `Ctrl-G` escape sequence often instinctively typed commands with the configured prefix (e.g., `//status` instead of `status`). Since the prefix was only stripped in the inline line-start detection path, this caused the command parser to fail with an "unknown command: //status" error.
2. **Lack of Session Title Visibility**: There was no visual indicator in the terminal window/tab title showing which native agent harness (e.g. `gemini`, `claude`, etc.) was currently active, causing navigation confusion when running multiple terminal instances.

## 2. Options Considered

### Option 1: Enforce Strict Command Syntax & Keep Static Window Title
* **Description**: Keep the command parser strict (only accepting command names without prefix in command mode) and leave the terminal title unchanged.
* **Pros**:
  * Zero code changes.
  * No external terminal title manipulation which could theoretically interfere with some non-standard terminal emulators.
* **Cons**:
  * High friction: Users must remember to omit the prefix in `Ctrl-G` mode but include it for inline commands.
  * Poor situational awareness when multiplexing multiple agent sessions.

### Option 2: Preemptive Prefix Stripping and ANSI OSC Title Manipulation (Chosen)
* **Description**: Modify the central command runner `run_wrapper_command` to dynamically load the configured prefix and strip it from the input string if present before parsing. Additionally, emit the standard ANSI OSC escape sequence (`\x1b]0;<title>\x07`) to set the terminal tab/window title to `agentmux: <agent>` upon session initialization.
* **Pros**:
  * Intuitive UX: Both `//status` and `status` work perfectly in command mode.
  * Dynamic terminal tab titles improve navigation across multiple workspace sessions.
* **Cons**:
  * Requires parsing/loading config prefix at command dispatch time.

## 3. Chosen Decision & Rationale
We chose **Option 2**.

By stripping the prefix in `run_wrapper_command`, we align command mode input with user muscle memory. Standardizing window titles via ANSI OSC `\x1b]0;...\x07` is highly portable across modern Unix and macOS terminal emulators, providing immediate visual feedback on the active multiplexed agent without adding graphical UI complexity.

## 4. Rejected Alternatives
* **Option 1**: Rejected because strict syntax checks lead to unnecessary user errors, and static window titles degrade multiplexer usability.
