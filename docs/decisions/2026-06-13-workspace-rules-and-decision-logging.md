# Architectural Decision Record: Workspace Rules and Architectural Decision Logging

* **Date**: 2026-06-13

## 1. Context & Problem Statement
During development and pair-programming with agentic AI assistants, two major process friction points were identified:
1. Routine git and workspace commands frequently trigger security/permission boundaries, resulting in repetitive manual prompts that disrupt the workflow.
2. Architectural design choices, trade-offs, and decisions are often lost in chat history transcripts, making long-term maintenance and handoffs difficult.

## 2. Options Considered

### Option 1: Interactive Prompts and Ad-Hoc Documentation
* **Description**: Require the user to manually approve every command execution and rely on manual/ad-hoc updates to the roadmap or architecture files.
* **Pros**:
  * No upfront configuration or rule setup required.
  * Maximum manual control over agent actions.
* **Cons**:
  * Severe flow disruption and developer fatigue due to constant approval prompting.
  * Lack of a central, standardized history of why specific system design decisions were made.

### Option 2: Pre-Approved Workspace Rules and Structured Decision Logging (Chosen)
* **Description**: Configure automated rules (e.g., `GEMINI.md`, `.claude/settings.json`) to allow git command prefixes without interactive prompts. Establish the `docs-decisions-log` skill to enforce documenting architectural decisions in a standardized ADR (Architectural Decision Record) format under `docs/decisions/`.
* **Pros**:
  * Significantly reduces developer friction and command approval fatigue.
  * Standardizes how technical decisions are preserved, making future context retrieval easy.
* **Cons**:
  * Requires setting up and maintaining configuration rules per workspace.
  * Adds small overhead to write and commit ADRs.

## 3. Chosen Decision & Rationale
We chose **Option 2**.

Automating common permissions (like `git` operations) allows agents to run formatting, diff, status, and commit workflows uninterrupted while preserving safety. 
Implementing the structured `decision-log` protocol ensures that every architectural fork in the road is documented with its context, considered options, pros/cons, and rationales, establishing a durable, searchable history of the codebase's architecture.

## 4. Rejected Alternatives
* **Option 1**: Rejected because manual permission prompting for basic commands slows down iteration, and ad-hoc documentation fails to capture alternative options and rejected trade-offs.
