# Architectural Decision Record: Session State Isolation

* **Date**: 2026-06-13

## 1. Context & Problem Statement
In previous iterations, the command parsing module `src/commands.rs` had grown to mix command parsing, background session storage, status formatting, and process control logic. Similarly, `src/pty.rs` owned process spawning and command-mode logic, but interacted with process termination behavior directly inside command-mode handling. This lack of modularity made it difficult to introduce future interactive state transitions (like `focus` and `detach`) without risking regression, state corruption, or code duplication.

## 2. Options Considered

### Option 1: Keep State Inline in Commands & PTY Modules
* **Description**: Keep in-memory tracking of sessions inside `commands.rs` or directly in `pty.rs` where processes are spawned/managed.
* **Pros**:
  * Fewer files/modules in the codebase.
  * Direct access to variables without moving ownership or thread coordination.
* **Cons**:
  * Violates single responsibility principle.
  * Tightly couples CLI command parser to session management and process lifecycles.
  * Hard to test parser logic independently without mock/stub session objects.

### Option 2: Dedicated Session Management Module (Chosen)
* **Description**: Extract background session metadata, global list storage, status text generation, and shutdown actions into a dedicated `src/sessions.rs` module.
* **Pros**:
  * Clear architectural boundaries: parsing command enums (`commands.rs`), managing active sessions (`sessions.rs`), and controlling process execution and raw terminal control (`pty.rs`).
  * Pure command-parsing unit tests become completely decoupled from global system state.
  * Extensible structure for adding future session features (e.g. session focus, detaching, and persistence).
* **Cons**:
  * Requires global synchronization primitives (e.g., `OnceLock` and `Mutex`) for thread-safe access to session states from different terminal reader threads.

## 3. Chosen Decision & Rationale
We chose **Option 2** (Dedicated Session Management Module). 
By isolating session state into `src/sessions.rs`, we created a clean interface for registering, querying, formatting, and killing background sessions. Thread safety was achieved using a global `OnceLock<Mutex<Vec<BackgroundSession>>>` containing all active background tasks. This allows the command parser (`commands.rs`) to remain pure and easy to test, and keeps `pty.rs` focused on process spawning and raw mode lifecycle guards.

## 4. Rejected Alternatives
* **Option 1**: Rejected because keeping parsing coupled to process management would make introducing multi-session features like foreground-to-background focus swaps highly error-prone and complex to maintain.
