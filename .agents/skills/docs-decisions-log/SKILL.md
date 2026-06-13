---
name: decision-log
description: Summarizes key architectural decisions, options, alternatives, and rationales from a chat thread and persists them in the docs/decisions/ folder.
---

# Decision Log Generator Skill

This skill outlines how the agent should capture context from a thread where architectural or design decisions were made, and persist them as a Markdown document in the codebase's durable documentation under `docs/decisions/`.

## Workflow Protocol

When the user asks to run the decision log check or log a decision:

### 1. Structure the Log Document
Create a new file under `docs/decisions/` with the filename format:
`YYYY-MM-DD-<short-topic-slug>.md`

> [!IMPORTANT]
> The `YYYY-MM-DD` prefix and document date MUST be the date the decisions were actually discussed or implemented in the chat thread, NOT today's date if logging post-facto.

Use the following template for the contents:
```markdown
# Architectural Decision Record: <Topic Title>

* **Date**: YYYY-MM-DD

## 1. Context & Problem Statement
Describe the problem that needed solving, why it occurred, and the impact of the issue.

## 2. Options Considered
Detail all potential solutions that were discussed or analyzed during the thread.

### Option 1: <Name>
* **Description**: ...
* **Pros**: ...
* **Cons**: ...

### Option 2: <Name>
* **Description**: ...
* **Pros**: ...
* **Cons**: ...

## 3. Chosen Decision & Rationale
State which option was chosen and provide the clear technical rationale behind the decision.

## 4. Rejected Alternatives
Specify which options were rejected and the reasons why they were not chosen.
```

### 2. Verify and Commit
Stage the generated file and commit it:
```powershell
git add docs/decisions/
git commit -m "docs: add decision log for <topic-slug>"
git push
```
