# Architectural Decision Record: Deadlock-Free PTY Termination

* **Date**: 2026-05-31

## 1. Context & Problem Statement
When running background agent sessions, a dedicated background thread spawns the child process and blocks on `child.wait()`. If the main user control loop attempts to terminate/stop that session, it needs to access the process handle to send a kill signal. If the process handle is shared via standard shared mutability (like `Arc<Mutex<Child>>`), a deadlock can occur because the blocking wait thread holds the lock/ownership of the child process.

## 2. Options Considered

### Option 1: Shared Arc<Mutex<Child>> Ownership
* **Description**: Wrap the child process in `Arc<Mutex<Child>>` and share it between the reader/wait thread and the main control thread.
* **Pros**:
  * Gives full control over the child handle to both threads.
* **Cons**:
  * High risk of deadlock: the thread waiting on the child will block, potentially holding the mutex and preventing the main thread from acquiring it to call `kill()`.

### Option 2: Cloneable Process Killer Handles (Chosen)
* **Description**: Leverage `portable_pty::ChildKiller` via `child.clone_killer()`. The background thread retains sole ownership of the `child` handle and calls `wait()`, while the main thread stores the cloneable `Box<dyn ChildKiller + Send + Sync>` in the session metadata.
* **Pros**:
  * Zero-deadlock design: no shared locks/mutexes are held during blocking operations.
  * The main thread can call `kill()` asynchronously at any time.
* **Cons**:
  * `ChildKiller` only supports process termination; other process info must be tracked separately.

## 3. Chosen Decision & Rationale
We chose **Option 2**. Cloned `ChildKiller` handles completely eliminate lock contention and deadlocks between the command-handling loop and the PTY reader threads. It allows the main thread to immediately stop background processes without blocking or waiting.

## 4. Rejected Alternatives
* **Option 1**: Rejected because blocking wait loops must never hold locks needed by the user-interactive interface, as it leads to random hangs during session switching/stopping.
