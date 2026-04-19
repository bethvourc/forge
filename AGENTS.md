# AGENTS.md

## Project Identity

This repository contains the implementation of **Forge**, a production-grade, terminal-native DevOps command center built in Rust.

Forge combines:
- an intelligent terminal wrapper for command execution and AI-assisted workflows
- a live operational dashboard for services, logs, system health, git state, and task visibility
- a safe action framework for approvals, command classification, and controlled automation

Forge is not a prototype, demo, or MVP. It is being designed and implemented as a complete, end-to-end, production-ready system with a clean architecture, strong reliability guarantees, extensibility, and a polished user experience.

---

## Product Goal

Forge should feel like a serious developer tool that users can install, trust, and use daily.

It must provide:
- a high-performance terminal-native interface
- responsive multi-pane workflows
- reliable command execution
- streaming logs and event output
- operational visibility across local development services
- structured AI-assisted diagnosis and action flows
- safe approval boundaries for risky or destructive actions
- clear architecture for future expansion into remote environments, deployments, and team workflows

Every implementation decision should support long-term maintainability, correctness, safety, and user trust.

---

## Technology Direction

The stack is fixed unless explicitly changed by the user:

- **Language:** Rust
- **UI:** Ratatui
- **Terminal/event handling:** Crossterm or equivalent Rust-native terminal stack
- **Async runtime:** Tokio
- **Serialization:** Serde
- **Logging/telemetry:** tracing
- **Configuration:** TOML-based configuration
- **Distribution:** single cross-platform binary
- **Primary form factor:** terminal-native application

Do not rewrite the project in another language.
Do not introduce Python, Node, or Go as runtime dependencies for core product functionality unless explicitly approved by the user.

---

## Product Constraints

Forge must be designed as:

- terminal-native first
- production-grade
- modular
- strongly typed
- observable
- safe by default
- extensible without large rewrites
- cleanly packaged as a single installable binary

Avoid building this as:
- a proof of concept
- a throwaway prototype
- a one-file demo
- a mock UI without real architecture
- a hardcoded toy app

All code should move the repository toward a real, shippable system.

---

## Core Product Areas

Forge should support these major areas as first-class concerns:

1. **Terminal Command Layer**
   - command execution
   - streaming stdout/stderr
   - background jobs
   - exit status tracking
   - shell/session awareness
   - command history and replay

2. **Operational Dashboard**
   - service/process visibility
   - health status
   - log streams
   - test/build state
   - git/repo state
   - system resource indicators
   - event feed

3. **AI Workflow Layer**
   - slash commands
   - diagnosis flows
   - summarization flows
   - codebase-aware assistance
   - repo-context reasoning
   - operational action suggestions

4. **Safety and Approval Layer**
   - risk classification
   - approval prompts
   - action review
   - destructive command gating
   - auditability of AI-suggested actions

5. **Configuration and State**
   - user config
   - workspace/project config
   - persisted preferences
   - command history
   - panel/layout state where appropriate

6. **Observability**
   - structured logs
   - traceable internal events
   - actionable error messages
   - debug-friendly architecture

---

## Non-Negotiable Engineering Principles

### 1. Architecture before expansion
Do not rush into feature sprawl.
Prefer a small number of well-defined modules with clear boundaries over fast but tangled implementation.

### 2. Real implementations over mock behavior
Do not fake production features with placeholders unless explicitly marked and isolated.
If a subsystem is introduced, it should have a believable architecture and real integration path.

### 3. Strong typing and explicit state
Prefer explicit domain models, enums, and typed state transitions over loose dynamic patterns.

### 4. Safety by default
Anything that can alter the system, delete data, kill processes, or perform risky actions must pass through a safety/approval mechanism.

### 5. Responsive UX
The UI should remain responsive under streaming logs, command execution, and background refreshes.
Never block the interface with avoidable synchronous work.

### 6. Clear error handling
Avoid `unwrap` and fragile assumptions in production paths.
Errors should be propagated, classified, and surfaced meaningfully.

### 7. Extensibility
Design modules so future additions such as Docker, Kubernetes, CI integrations, remote agents, and deployment workflows can be added cleanly.

### 8. Production-quality code
Code should be organized, documented where needed, tested where practical, and suitable for long-term maintenance.

---

## Expected Architecture Style

Prefer a modular architecture with clear separation between:

- `app` or application orchestration
- `domain` models and core state
- `ui` rendering and layout
- `actions` / command handling
- `services` / integrations
- `runtime` / async event loop
- `config`
- `safety`
- `observability`

Keep business logic out of rendering code.
Keep rendering code out of core state transitions.
Keep external integration logic isolated behind interfaces or service modules.

---

## UI Expectations

Forge should feel polished, focused, and operationally useful.

The intended terminal layout includes:
- top status bar
- command / AI interaction pane
- operational dashboard pane
- bottom event stream

The UI should support:
- keyboard-first workflows
- clear visual hierarchy
- actionable status visibility
- readable log presentation
- consistent panel behavior
- graceful resizing
- stable redraw behavior
- low visual clutter

Do not treat the UI as a temporary shell around the backend.
The terminal experience is the product.

---

## AI Interaction Expectations

AI-assisted workflows should be integrated as a serious product feature, not as a gimmick.

AI features should:
- operate with repository and runtime context
- produce structured, understandable outputs
- explain reasoning at the user-facing level when useful
- suggest actions before taking them
- respect approval boundaries
- avoid pretending certainty when uncertain
- surface confidence, assumptions, and next actions clearly

Do not design AI features as unrestricted autonomous behavior.
Forge should be operator-centered, with the user in control.

---

## Safety Policy for Actions

All actions should be classified before execution.

Suggested action classes:
- **Safe**: read-only inspection, summaries, diagnostics
- **Caution**: service restarts, non-destructive local changes, test reruns
- **Risky**: file modifications, dependency changes, git resets, container rebuilds
- **Destructive**: deletes, force operations, irreversible changes, production-impacting actions

Requirements:
- Safe actions may run directly
- Caution actions should be visible and reviewable
- Risky actions require explicit user approval
- Destructive actions must always require explicit confirmation and clear explanation

Any implementation involving automation must preserve auditability of what was proposed, approved, and executed.

---

## Performance Expectations

Forge should be efficient and stable enough for real daily use.

Priorities:
- fast startup
- low-latency input handling
- smooth panel refresh
- non-blocking command/log streaming
- bounded memory usage
- controlled background polling
- predictable behavior under long-running sessions

Do not introduce wasteful polling or poorly bounded background tasks without justification.

---

## Code Quality Expectations

When generating or modifying code:

- prefer explicit, readable implementations
- keep files reasonably scoped
- avoid excessive abstraction too early
- avoid over-coupling modules
- add comments where they clarify intent, not obvious mechanics
- write code that compiles cleanly
- maintain idiomatic Rust style
- use tests for important domain logic where practical
- use feature flags or clear module boundaries for future integrations if needed

Do not leave the repository in a broken or half-integrated state.

---

## Planning Expectations

When asked to implement features:

1. first inspect the current architecture
2. explain how the feature fits into the system
3. propose the relevant file/module changes
4. implement in coherent slices
5. keep the system compiling as changes are introduced

When the requested change is large, prefer staged implementation with solid foundations rather than rushed monolithic patches.

Do not describe the system as an MVP, prototype, or quick demo.
Describe it as a production-grade terminal-native platform.

---

## Communication Rules for the Coding Agent

When assisting in this repository:

- be direct
- be technically concrete
- explain tradeoffs clearly
- do not oversell incomplete work
- do not claim something is production-ready unless the implementation actually supports that claim
- identify gaps honestly
- preserve architectural consistency
- prioritize correctness and maintainability over flashy shortcuts

If something is not yet complete, say so clearly and define what remains.

---

## Initial Build Priorities

In early implementation, prioritize a robust foundation for:

- application state model
- event loop/runtime model
- pane layout system
- command execution engine
- event stream model
- log ingestion/rendering model
- service status model
- safety/approval pipeline
- configuration loading
- structured observability

These should be implemented in a way that supports the full end-to-end system, not just a temporary first pass.

---

## What to Avoid

Do not:
- collapse the architecture into a single giant file
- hardcode fake service state for core flows
- add unnecessary framework churn
- optimize for speed at the expense of structure
- introduce weakly defined modules
- bypass approval logic for risky actions
- implement “AI magic” without state/control boundaries
- frame the product as a toy or experiment

---

## Final Standard

Every contribution should answer this question:

**Does this move Forge closer to being a polished, trustworthy, production-grade terminal-native DevOps system?**

If not, rethink the implementation.
