# Architectural Decision Record: Waiting Approval Detection, Real-time Notifications, and Run Inspection

* **Date**: 2026-05-31

## 1. Context & Problem Statement
Delegated background agent runs (using harnesses like Codex, Gemini, Claude, or Antigravity) can stall when a native harness asks for user approval, permission, confirmation, or other inputs (e.g., tool execution confirmation). Without visibility, users had to inspect transcripts manually or guess that a session was stuck. We needed a lightweight way to detect when a background process is waiting, notify the user in real-time, and allow quick inspection of runs and completed result files from the command mode.

## 2. Options Considered

### Option 1: Heuristic String Matching with Agent-Aware Patterns, Stderr Notifications, and Command-Mode Inspection (Chosen)
* **Description**: Scan the PTY output in a rolling buffer (up to 4096 characters) using static string patterns. Use per-agent patterns for `codex`, `claude`, `gemini`, and `agy` with a generic fallback. Change `set_background_waiting_hint` to return a boolean indicating whether the state changed to deduplicate notifications. Write alerts to `stderr` when a non-focused session changes state. Add `runs` and `result <run-id>` command wrappers to print run metadata and template summaries.
* **Pros**:
  * Extremely lightweight, pure data matching with no heavy dependencies.
  * Stderr alerts keep transcripts clean and do not interleave with focused streams.
  * Wrapper commands integrate cleanly into the existing `Ctrl-G` interface.
* **Cons**:
  * Substring heuristics may cause rare false positives.
  * ANSI escape sequences or split PTY chunks can occasionally break pattern detection.

### Option 2: OS-level Syscall and Stdin Block Inspection
* **Description**: Inspect parent/child process states at the operating system level, checking if background processes are blocked on standard input reads.
* **Pros**:
  * Higher theoretical accuracy.
  * Does not rely on heuristic text matching.
* **Cons**:
  * Platform-specific, fragile, and highly complex.
  * Process status block states do not necessarily differentiate standard terminal reads from other socket/file I/O waits.

### Option 3: Overlay status bar and Interactive Modal UI
* **Description**: Embed a dashboard status line or interactive modal dialog in the terminal output to present live notifications.
* **Pros**:
  * Clean, polished, and structured presentation.
* **Cons**:
  * Requires a complete alternate screen TUI framework.
  * Violates the project design principle of a thin, transparent command wrapper.

## 3. Chosen Decision & Rationale
We chose **Option 1**.
This implementation provides a non-invasive, fast, and transparent mechanism to track and alert on background stalls.
Keeping the `waiting_hint` state in `BackgroundSession` separate from `status` preserves existing execution guards. 
By designing `set_background_waiting_hint` to return a state-change boolean, we emit a single distinct `stderr` notification line only when the state changes, preventing terminal spam.
The `runs` and `result <run-id>` commands leverage existing metadata files (`meta.txt` and `result.md`) to reconstruct run lists and result summaries dynamically, without adding persistence complexity. Switched agents receive their specific `result.md` instruction dynamically, ensuring they record completion results reliably.

## 4. Rejected Alternatives
* **Option 2**: Rejected because tracing OS blocking states or hooking native harness internals is fragile, invasive, and engine-dependent.
* **Option 3**: Rejected to maintain the lightweight command-line interface design without adopting heavy TUI rendering runtimes.
