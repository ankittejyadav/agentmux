# Architectural Decision Record: Configured Agent Launches, Inline Prefix Commands, and Log Management (V1 Consolidation)

* **Date**: 2026-05-31

## 1. Context & Problem Statement
As `agentmux` approached its V1 release, several user experience and system administrative gaps needed to be resolved:
1. Native agent binary locations and default arguments were hardcoded, preventing users from customizing paths or using alternative CLI models.
2. Relying solely on `Ctrl-G` for wrapper command dispatch added keyboard overhead, whereas developers wanted inline command convenience (e.g., `//status`).
3. Running commands needed simple lifecycle controls like terminating all background runs (`stop-all`) and pruning old execution runs (`cleanup-runs`) to prevent directory bloat.
4. Transcript inspection lacked a way to quickly check progress without exiting the active harness session.

## 2. Options Considered

### Option 1: Hardcoded Bindings and Minimal Run Controls
* **Description**: Keep agent launching hardcoded to command names. Require the `Ctrl-G` escape sequence for all multiplexer actions. Delegate run directory management and process termination to external scripts.
* **Pros**:
  * Simpler implementation with zero external dependencies (no YAML/TOML parsing needed in Rust).
* **Cons**:
  * Low usability. Users cannot redirect execution to specific binary paths or custom wrapper scripts.
  * Friction in typing command escapes repeatedly.

### Option 2: Declarative Configuration, Inline Prefix Interception, and Integrated Session Controls (Chosen)
* **Description**: Introduce `serde` and `toml` to parse `.agentmux/config.toml` for command and argument overrides. Implement an inline prefix state machine (`process_input_byte`) in the PTY input reader thread to intercept commands starting with `//` (or custom prefix) at the beginning of a line. Add `stop-all` for process termination, `cleanup-runs` CLI commands with safety guards, and `tail <run-id> [lines]` leveraging a memory-bounded line ring buffer.
* **Pros**:
  * High extensibility: Users can customize their environments without compiling code.
  * Fluid UX: Inline commands are fast to type, while native slash commands (e.g., `/skills`) pass through seamlessly.
  * Safety: Run directory cleanup is fenced to protect config files, plans, and active runs.
* **Cons**:
  * Adds external parser dependencies (`toml` and `serde`).
  * Inline parsing state machine adds complexity to the standard PTY input piping loop.

## 3. Chosen Decision & Rationale
We chose **Option 2**.

This consolidation completes the V1 feature set. Incorporating a real TOML parser ensures robust configuration management. Separating the stable agent key (used for wait-heuristics, runs, and metadata) from the custom execution path allows binary customization while retaining all system adapters.

The inline command interceptor works by checking character matching at the absolute start of a line. If the configured prefix (default `//`) is fully matched, we enter inline command mode. If the user typing diverges from the prefix, the buffered characters are immediately flushed to the PTY, ensuring single-slash commands (`/skills`, `/goals`) and normal text are sent transparently without delay or interruption.

The `tail` command reads the PTY transcript file lossily to prevent crashes from invalid UTF-8 and uses a ring buffer to cap memory consumption. Finally, `cleanup-runs` reads active in-memory session states to prevent deleting currently executing directories, protecting state integrity.

## 4. Rejected Alternatives
* **Option 1**: Rejected because customizable execution paths and simple run pruning are core requirements of a robust, standalone terminal harness.
