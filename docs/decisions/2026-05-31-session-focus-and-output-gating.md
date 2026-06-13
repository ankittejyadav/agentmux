# Architectural Decision Record: Session Focus, Dynamic Input Routing, and Output Gating

* **Date**: 2026-05-31

## 1. Context & Problem Statement
To support interactive session switching (e.g. `focus <session-id>` and `detach`), the agentmux tool needed a way to dynamically route user keyboard input to different background process inputs, mirror process output to stdout on focus, and prevent stdout from the original foreground process from interleaving and corrupting the visual state of the focused process, all without using a full Terminal User Interface (TUI) or external alternate screen runtimes.

## 2. Options Considered

### Option 1: Inline Terminal Multiplexing with Full TUI (e.g., Ratatui, Panes)
* **Description**: Implement a full TUI with screen panels or alternate screen buffers to manage visual isolation.
* **Pros**:
  * Complete visual separation of input/output streams.
  * Robust tabbed or tiled layouts for multiple active sessions.
* **Cons**:
  * High complexity and large external dependencies.
  * Overkill for a lightweight command-line harness.
  * Requires major structural changes to the terminal interaction model.

### Option 2: Dynamic Input Routing, Output Gating, and Foreground Transcript Capturing (Chosen)
* **Description**: Wrap both foreground and background PTY writers in a shared `SessionInput` container (`Arc<Mutex<Box<dyn Write + Send>>>`). Use a thread-safe atomic visibility flag (`foreground_output_visible`) and session output modes (`SessionOutputMode`) to dynamically gate who prints to stdout. Capture the gated foreground output in a background transcript directory (`.agentmux/runs/foreground-.../`).
* **Pros**:
  * Extremely lightweight, requiring no external TUI dependencies.
  * Preserves terminal raw mode behaviour.
  * Fully records foreground activity during background session focus.
* **Cons**:
  * Does not prevent visual output interleaving from processes spawned outside of the PTY harness.
  * Creates directory and file overhead for every foreground command execution.

### Option 3: Standard Input Multiplexing via PTY Reader Interception Only
* **Description**: Swap input streams but let output streams print to stdout concurrently without gating.
* **Pros**:
  * Very simple to implement.
  * Minimal state tracking.
* **Cons**:
  * Complete visual interleaving of background and foreground stdout makes interactive tasks unusable.

## 3. Chosen Decision & Rationale
We chose **Option 2**.
By wrapping the PTY writers, we enabled clean input routing swaps via an `active_input` variable in the raw terminal reader loop.
To prevent visual corruption during focused sessions, we gated the foreground stdout writes using a local `foreground_output_visible` atomic boolean, while redirecting all foreground stdout output to a persistent file (`foreground-.../transcript.ansi`).
When a session is focused, its output mode is changed to `Foreground`, triggering stdout mirroring from the background reader thread.
Terminal resizing is propagated to both foreground and active background PTY sessions via a 500ms polling thread that queries the window dimensions and invokes `MasterPty::resize`.
Finally, to handle natural exits, the input reader thread checks `is_background_session_running()` and automatically detaches when a focused session exits.

## 4. Rejected Alternatives
* **Option 1**: Rejected due to high implementation complexity and because the tool aims to remain a lightweight command wrapper.
* **Option 3**: Rejected because visual interleaving makes interactive tasks unreadable.
