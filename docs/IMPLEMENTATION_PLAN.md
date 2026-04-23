# Forge End-to-End Implementation Plan

## Document Purpose

This document defines the remaining implementation work required to complete Forge as a production-grade, terminal-native DevOps command center.

It is intended for coding agents and contributors who need enough context to implement meaningful slices without rediscovering the product direction. It should be read alongside:

- `AGENTS.md`
- `PROJECT_BRIEF.md`
- `ARCHITECTURE.md`
- `README.md`

The plan is organized as implementation tracks. Each track includes the product outcome, expected module ownership, concrete implementation tasks, acceptance criteria, and validation commands. Agents should complete work in coherent slices that compile and preserve the architectural boundaries described in `ARCHITECTURE.md`.

---

## 1. Current Implementation Baseline

Forge already has a meaningful foundation:

- Rust crate with Ratatui, Crossterm, Tokio, Serde, tracing, TOML configuration, and Clap.
- Modular source layout across `app`, `domain`, `runtime`, `infra`, `ui`, `ai`, `safety`, `config`, `observability`, and `shared`.
- Typed application state in `domain`.
- Central application store and effect dispatch in `app`.
- Async runtime loop that handles terminal input, ticks, supervisor events, rendering, and shutdown.
- Managed shell command execution through the configured shell.
- Line-oriented stdout/stderr streaming into typed log events.
- Command/job/process state transitions for managed commands.
- Background command parsing through `/bg <cmd>` and trailing `&`.
- Basic service records for background managed commands.
- Slash command parsing and command suggestions.
- Safety classifier and approval routing for caution, risky, and destructive commands.
- AI provider abstraction with `mock`, `openai`, disabled, and unconfigured providers.
- AI context bundling from project, git, command, service, test, log, and timeline state.
- OpenAI Responses API request construction and structured JSON response parsing.
- Ratatui UI with status bar, command/AI pane, dashboard tabs, modals, and event stream.
- Config loading from defaults, global config, project config, environment variables, and CLI overrides.
- Structured file logging and panic hook terminal restoration.
- Foundation test coverage for parsing, store transitions, safety routing, AI proposal reclassification, and UI rendering.

This baseline should be preserved and extended. Do not collapse the architecture into a single file or bypass the state/effect/runtime model.

---

## 2. Completion Definition

Forge should be considered end-to-end complete when a user can launch it inside a real project and use it continuously to:

- run interactive and non-interactive shell commands
- maintain shell/session awareness
- run foreground and background jobs
- stream and inspect command output and logs
- discover and monitor local services
- inspect process, port, and resource state
- track Git state, diffs, and repository changes
- track test runs and failures
- ask AI-grounded questions using current runtime and repository context
- review and approve proposed operational actions
- persist command history, approvals, and relevant session state
- diagnose failures with observable evidence
- exit cleanly without corrupting terminal state

Forge should be considered production-ready only when the implementation has:

- reliable PTY-backed shell execution
- bounded async queues and supervised background tasks
- robust cancellation and process cleanup
- durable state where users expect continuity
- tested safety and approval boundaries
- tested AI provider failure handling
- tested terminal rendering and resizing behavior
- release packaging for supported platforms
- clear user documentation and troubleshooting guidance

---

## 3. Implementation Rules for Agents

### 3.1 Work in vertical slices

Each implementation slice should connect domain types, app store transitions, runtime effects, infrastructure behavior, UI visibility, and tests where applicable. Avoid adding unused domain structures without a real integration path.

### 3.2 Keep module boundaries intact

- Domain models belong in `src/domain`.
- State transitions and effects belong in `src/app`.
- Async task supervision belongs in `src/runtime`.
- External system access belongs in `src/infra`.
- Provider-specific AI code belongs in `src/ai/providers`.
- Rendering belongs in `src/ui`.
- Configuration schema and loading belong in `src/config`.
- Safety rules and approval policy belong in `src/safety`.

### 3.3 Preserve safety by default

Any command, workflow step, AI proposal, or service action that can modify files, kill processes, restart services, install dependencies, change Git state, or delete data must pass through the safety classifier and approval pipeline.

### 3.4 Validate every slice

At minimum, run:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

For terminal, PTY, rendering, and AI work, add focused tests or documented manual smoke checks as described in the relevant track.

---

## 4. Track A: Shell and Command Execution

## Outcome

Forge should behave like a trustworthy terminal-native command surface. Users should be able to run normal shell commands, interactive commands, foreground tasks, background jobs, and command replays without losing context or corrupting terminal state.

## Current state

Managed non-interactive shell execution exists. Commands are spawned through the configured shell, stdout/stderr are streamed line-by-line, and lifecycle events update command, job, process, log, and timeline state.

PTY execution and persistent shell sessions are not implemented yet.

## Primary modules

- `src/domain/command.rs`
- `src/domain/job.rs`
- `src/app/store.rs`
- `src/app/effects.rs`
- `src/runtime/engine.rs`
- `src/runtime/supervisor.rs`
- `src/infra/shell`
- `src/ui/views/command_pane.rs`
- `src/config/schema.rs`
- `tests/`

## Implementation tasks

### A1. Add a PTY execution backend

Implement a Rust-native PTY backend for interactive commands.

Requirements:

- Add a PTY-capable executor behind a shell execution abstraction.
- Support Unix-like systems first with a clean path for Windows support.
- Preserve terminal raw mode and alternate screen behavior.
- Stream PTY output into the existing event pipeline.
- Support resize propagation from Forge terminal resize events into active PTY sessions.
- Ensure PTY child processes are cleaned up on cancellation and Forge shutdown.

Expected shape:

- Introduce a shell executor trait or enum-backed backend in `src/infra/shell`.
- Keep `ExecutionMode::Managed` for piped non-interactive commands.
- Implement `ExecutionMode::Pty` for interactive commands and persistent shell sessions.
- Add typed errors for PTY setup, read/write, resize, and shutdown failures.

Acceptance criteria:

- Interactive commands such as `less`, `top`, and shell prompts can run without corrupting Forge.
- Terminal resize events are reflected in PTY-backed commands.
- Cancelling a PTY command terminates the full process group where supported.
- Forge restores the terminal after PTY failures and panics.

Validation:

```sh
cargo test shell
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Manual smoke checks:

- Run `pwd`.
- Run `clear` and confirm it triggers Forge's built-in clear behavior, not an OS command record.
- Run an interactive command through the PTY path.
- Cancel a long-running PTY command.
- Resize the terminal while output is streaming.

### A2. Implement persistent shell/session state

Users should not lose shell context between commands.

Requirements:

- Track session cwd, shell, environment overlays, and last exit status.
- Support `cd` as a state-changing shell built-in for the Forge session.
- Reflect current cwd in status or command pane metadata.
- Run commands from the session cwd unless explicitly overridden.
- Keep command provenance visible.

Acceptance criteria:

- `cd some/path` changes the Forge session cwd.
- A later `pwd` reflects the updated cwd.
- Failed `cd` surfaces a user-visible diagnostic and does not change state.
- AI context includes current cwd and recent command status.

### A3. Improve command history and replay

Requirements:

- Persist command history across Forge launches.
- Support rerun of recent commands through slash command and UI affordance.
- Preserve command metadata: cwd, shell, provenance, background flag, safety class, timestamps, exit status.
- Avoid re-running risky/destructive commands without approval.

Suggested commands:

- `/history`
- `/rerun <command-id>`
- `/rerun-last`

Acceptance criteria:

- History survives restart.
- Replayed commands go through safety classification again.
- UI shows replay provenance distinctly from original user input.

### A4. Harden cancellation and process cleanup

Requirements:

- Track child process ids and process groups.
- Cancel foreground and background commands reliably.
- Escalate from graceful termination to force kill after a timeout.
- Mark commands, jobs, services, and processes consistently after cancellation.
- Clean up cancellation registry entries on all exit paths.

Acceptance criteria:

- Cancelling `sleep 30` marks the command cancelled and removes it from active state.
- Cancelling a background service marks service health as stopped.
- Forge shutdown cancels or detaches processes according to explicit policy.

---

## 5. Track B: Runtime, Events, and Supervision

## Outcome

Forge should have a predictable async runtime that can supervise long-running background work without blocking the UI or leaking tasks.

## Current state

The runtime loop uses a bounded channel for app events and dispatches effects to a supervisor. Git and project scans run in background tasks. Commands and AI requests are spawned asynchronously.

## Primary modules

- `src/app/effects.rs`
- `src/app/store.rs`
- `src/runtime/engine.rs`
- `src/runtime/supervisor.rs`
- `src/domain/timeline.rs`
- `src/domain/diagnostics.rs`

## Implementation tasks

### B1. Introduce a typed task registry

Requirements:

- Track supervised tasks by id, kind, status, start time, and cancellation handle.
- Distinguish command tasks, AI requests, project scans, Git refreshes, service monitors, log tails, and health checks.
- Surface task failures into diagnostics and timeline state.
- Ensure repeated refreshes do not spawn unbounded overlapping work.

Acceptance criteria:

- The dashboard can show active background tasks.
- Refresh tasks are deduplicated or coalesced where appropriate.
- Task failures are visible and do not crash the UI loop.

### B2. Add backpressure and event coalescing

Requirements:

- Prevent high-volume logs from starving UI input.
- Coalesce repetitive process/resource updates.
- Preserve important command lifecycle events.
- Drop or summarize low-value high-volume events only under explicit policy.

Acceptance criteria:

- A command producing thousands of lines does not freeze input.
- Log buffers stay bounded.
- Timeline remains readable under high output volume.

### B3. Define shutdown semantics

Requirements:

- Implement graceful shutdown sequence for UI, command tasks, AI tasks, log tails, monitors, and observability guard.
- Decide and document whether background commands are cancelled or detached by default.
- Provide config for background shutdown policy.

Acceptance criteria:

- Ctrl-C and `/quit` perform the same clean shutdown path.
- Terminal state is restored after normal shutdown and task failures.
- Long-running tasks do not keep Forge hanging indefinitely.

---

## 6. Track C: Safety, Approvals, and Auditability

## Outcome

Forge should make user-impacting actions reviewable, auditable, and safe by default.

## Current state

Command classification exists. Caution commands route to inline review. Risky and destructive commands route to modal confirmation. AI proposals are reclassified at runtime instead of trusting model-provided safety hints.

## Primary modules

- `src/safety`
- `src/domain/approval.rs`
- `src/domain/command.rs`
- `src/app/store.rs`
- `src/ui/views/modal.rs`
- `src/ui/views/command_pane.rs`
- `src/config/schema.rs`

## Implementation tasks

### C1. Replace pattern-only classification with structured command analysis

Requirements:

- Parse command strings into structured command features where practical.
- Identify shell operators, redirects, pipes, subshells, globs, variable expansion, destructive verbs, filesystem targets, package managers, Git state changes, service/process operations, and network operations.
- Return classification plus reasons.
- Keep conservative defaults for commands that cannot be understood.

Acceptance criteria:

- `rm -rf`, `git reset --hard`, `git clean`, redirection, dependency installs, `kill`, service restarts, and Docker prune are classified with explicit reasons.
- AI proposal approval detail shows runtime classification reasons.
- Unit tests cover common safe, caution, risky, and destructive cases.

### C2. Add approval policy configuration

Requirements:

- Add config for class thresholds, remembered approvals, command allowlists, command blocklists, and per-workspace policy.
- Prevent config from disabling destructive confirmation unless explicitly allowed by a hard-coded development override.
- Include policy source in approval detail.

Acceptance criteria:

- Project config can require approval for all non-passive commands.
- Blocklisted commands cannot execute and surface a clear diagnostic.
- Destructive commands remain gated by default.

### C3. Persist audit history

Requirements:

- Persist proposed action, classification, approval decision, timestamp, user note if provided, command id, provenance, and execution outcome.
- Include AI request id and model/provider for AI-proposed actions.
- Provide a UI and slash command path to inspect recent approvals.

Suggested commands:

- `/approvals`
- `/audit`

Acceptance criteria:

- Approval history survives restart.
- AI-suggested commands can be traced from prompt to proposal to approval to execution.

---

## 7. Track D: Services, Processes, Ports, and Health

## Outcome

Forge should provide real operational visibility across local services and processes, not just command lifecycle records.

## Current state

Domain models exist for services and processes. Background managed commands create service records. There is no general process monitor, port inspector, health evaluator, or configured service registry yet.

## Primary modules

- `src/domain/service.rs`
- `src/domain/process.rs`
- `src/infra/monitor`
- `src/runtime/supervisor.rs`
- `src/app/effects.rs`
- `src/app/store.rs`
- `src/ui/views/dashboard.rs`
- `src/config/schema.rs`

## Implementation tasks

### D1. Implement process snapshots

Requirements:

- Collect local process snapshots at bounded intervals.
- Include pid, parent pid where available, command/name, status, CPU, memory, observed timestamp.
- Associate managed command processes with Forge command/job ids.
- Avoid platform-specific assumptions leaking into domain models.

Acceptance criteria:

- Processes dashboard shows real CPU and memory for tracked commands.
- Process monitor failures produce diagnostics, not panics.
- Polling interval is configurable and bounded.

### D2. Implement port discovery

Requirements:

- Detect listening ports and owning pids where available.
- Associate ports with managed services and configured services.
- Support partial data when OS permissions restrict inspection.

Acceptance criteria:

- A background web server shows its listening port in the service dashboard.
- Missing permission or unsupported platform is surfaced as degraded observability, not failure.

### D3. Add configured services

Requirements:

- Extend TOML config with service definitions.
- Support service name, command, cwd, env, health check, log sources, tags, restart policy, and expected ports.
- Add start, stop, restart, and inspect actions through safety pipeline.

Suggested config shape:

```toml
[[services]]
name = "api"
command = "cargo run"
cwd = "."
tags = ["rust", "local"]
ports = [8080]

[services.health]
kind = "http"
url = "http://localhost:8080/health"
timeout_ms = 1000
```

Acceptance criteria:

- Configured services appear before they are started.
- Starting a service creates command, job, process, service, log, and timeline records.
- Restart and stop actions require appropriate safety review.

### D4. Implement health checks

Requirements:

- Support basic TCP, HTTP, process-alive, and command health checks.
- Record health reasons and last check timestamp.
- Distinguish unknown, starting, healthy, degraded, unhealthy, and stopped.
- Avoid aggressive polling.

Acceptance criteria:

- Service cards show health status with reason.
- Health checks continue while UI remains responsive.
- Health failures can be included in AI diagnosis context.

---

## 8. Track E: Log Ingestion, Filtering, and Search

## Outcome

Forge should treat logs as structured operational data that can be filtered, searched, associated with services/jobs, and passed to AI workflows.

## Current state

Command stdout/stderr is normalized into log entries with inferred severity. Logs are stored in bounded global and per-source buffers. There is no file tailing, container log ingestion, search UI, or structured parser pipeline yet.

## Primary modules

- `src/domain/log.rs`
- `src/infra/logs`
- `src/runtime/supervisor.rs`
- `src/app/store.rs`
- `src/ui/views/dashboard.rs`
- `src/ai/context.rs`
- `src/config/schema.rs`

## Implementation tasks

### E1. Add log source configuration

Requirements:

- Support file log sources in project config.
- Associate file logs with services where configured.
- Tail files from the end by default with configurable initial read size.
- Handle rotation and truncation.

Acceptance criteria:

- Configured log files stream into the Logs dashboard.
- Log tail tasks are supervised and cancellable.
- Missing files produce diagnostics and can retry if configured.

### E2. Implement log filtering and search

Requirements:

- Add UI state for text filters, severity filters, source filters, and service filters.
- Add slash commands and keyboard flow for search.
- Preserve bounded memory behavior.

Suggested commands:

- `/logs`
- `/logs service <name>`
- `/logs grep <text>`
- `/logs level <warn|error|info>`

Acceptance criteria:

- Logs dashboard reflects active filters.
- Clearing filters is discoverable.
- AI context receives filtered and recent high-signal logs.

### E3. Add structured log parsing

Requirements:

- Detect JSON log lines and extract fields.
- Preserve raw text.
- Infer severity from common keys such as `level`, `severity`, `msg`, `message`, and `error`.
- Support future parser plugins by keeping parser logic isolated.

Acceptance criteria:

- JSON logs display with severity and message correctly.
- Unknown formats remain visible as raw logs.

---

## 9. Track F: Git, Project Context, and Test Awareness

## Outcome

Forge should understand the repository and test state well enough to support diagnosis, safe action suggestions, and operational awareness.

## Current state

Project scan detects root, some common files, stack hints, and config paths. Git snapshot reads branch, short head, dirty flag, and changed file names. Test domain models exist but no adapters populate test state.

## Primary modules

- `src/domain/project.rs`
- `src/domain/git.rs`
- `src/domain/test.rs`
- `src/infra/project`
- `src/infra/git`
- `src/app/store.rs`
- `src/ai/context.rs`
- `src/ui/views/dashboard.rs`

## Implementation tasks

### F1. Expand Git context

Requirements:

- Track staged, unstaged, untracked, conflicted, and ignored states separately.
- Track ahead/behind upstream counts where available.
- Track recent commits with hash, author, date, and subject.
- Provide bounded diff stats and optional detailed diff excerpts for AI context.
- Avoid expensive Git commands on every tick.

Acceptance criteria:

- Git dashboard shows branch, head, upstream relation, changed files grouped by state, and recent commits.
- AI context can cite changed files and diff stats.
- Git refresh remains bounded and does not block the UI.

### F2. Improve project scanning

Requirements:

- Detect Rust, Node, Python, Docker Compose, Make, Just, and common CI files.
- Discover likely commands from `Cargo.toml`, `package.json`, `Makefile`, `justfile`, and project config.
- Detect candidate services from Docker Compose and config files.
- Keep scanning bounded and cache expensive results.

Acceptance criteria:

- Project dashboard or AI context includes stack hints and likely commands.
- Discovered service hints can be promoted into service records only through clear policy.

### F3. Implement test adapters

Requirements:

- Start with Rust `cargo test`.
- Detect when a command is a test run.
- Parse pass/fail summary and failed test names where practical.
- Track duration and source command id.
- Keep parser code outside UI rendering.

Acceptance criteria:

- Running `cargo test` updates `TestState`.
- Failed tests appear in dashboard and AI context.
- Test parsing failures do not mark the command itself failed unless the process failed.

### F4. Add test workflow commands

Suggested commands:

- `/test`
- `/test last`
- `/test failed`
- `/diagnose tests`

Acceptance criteria:

- Test slash commands create command/job records and go through safety policy.
- AI diagnosis can reference recent test failures.

---

## 10. Track G: AI Workflows and Provider Reliability

## Outcome

Forge's AI features should be grounded, inspectable, provider-aware, and safe. AI should assist the operator without becoming unrestricted automation.

## Current state

AI request state, context bundling, mock provider, OpenAI provider, structured response parsing, and AI proposal application exist. Runtime reclassifies proposal commands before execution.

## Primary modules

- `src/domain/ai.rs`
- `src/ai/context.rs`
- `src/ai/provider.rs`
- `src/ai/providers`
- `src/app/store.rs`
- `src/commands/mod.rs`
- `src/ui/views/command_pane.rs`
- `src/config/schema.rs`

## Implementation tasks

### G1. Harden provider configuration and diagnostics

Requirements:

- Validate provider configuration at startup.
- Clearly show provider status: disabled, unconfigured, ready, missing API key, failed.
- Add provider timeout, retry, and cancellation behavior.
- Avoid exposing secrets in logs, diagnostics, or UI.

Acceptance criteria:

- Missing API key produces an actionable UI diagnostic.
- Provider HTTP errors show status and truncated safe body.
- AI request cancellation is possible.

### G2. Improve context budgeting

Requirements:

- Add context budgets for commands, logs, diffs, tests, services, and timeline.
- Prioritize failed commands, error logs, unhealthy services, failed tests, and dirty Git files.
- Include citations that map back to local state.

Acceptance criteria:

- AI context remains bounded under long sessions.
- Diagnosis requests prioritize failures over generic recent events.

### G3. Add structured workflow proposals

Requirements:

- Extend AI proposals beyond single commands to typed plans.
- Each plan step must have a safety class, rationale, command/action payload, and dependencies.
- Require approval at step or plan level depending on risk.
- Preserve full audit trail.

Suggested domain model:

- `AiPlan`
- `AiPlanStep`
- `WorkflowRun`
- `WorkflowStepStatus`

Acceptance criteria:

- AI can propose a multi-step diagnosis plan without executing it automatically.
- Applying a plan schedules steps through the safety pipeline.
- Failed steps stop the plan unless policy says otherwise.

### G4. Support patch proposals safely

Requirements:

- AI may suggest file patches as proposed artifacts, not direct writes.
- Patches must be reviewable before application.
- Applying a patch is risky and requires approval.
- Patch application must preserve user changes and surface conflicts.

Acceptance criteria:

- AI patch proposals are visible and can be approved or denied.
- Patch application uses a safe, reviewable pathway.
- Tests can be rerun after approved patch application.

---

## 11. Track H: Terminal UI and Interaction Completeness

## Outcome

Forge should feel polished and efficient for daily terminal use. UI should remain stable under resizing, streaming output, approvals, background jobs, and long sessions.

## Current state

The UI renders a top status bar, command/AI pane, dashboard pane, bottom event stream, help modal, error modal, and approval modal. Dashboard tabs include Services, Processes, Logs, Git, and Tests.

## Primary modules

- `src/ui`
- `src/ui/views`
- `src/domain/app.rs`
- `src/app/store.rs`
- `tests/foundation.rs`

## Implementation tasks

### H1. Add scrollback and selection

Requirements:

- Support scrolling command output, logs, event stream, and dashboard lists.
- Track scroll state per pane.
- Keep input editing ergonomic while other panes are focused.
- Add keyboard hints in help modal.

Acceptance criteria:

- Users can inspect old output without losing new output.
- Scroll state behaves predictably when new events arrive.

### H2. Add detail inspectors

Requirements:

- Inspect command details, job details, service details, log entry details, Git file details, test failures, and approval audit records.
- Keep inspector rendering separate from state transition logic.

Acceptance criteria:

- Selecting an item opens a focused detail view.
- Detail views include ids needed for slash command references.

### H3. Improve command palette and slash command UX

Requirements:

- Add command categories.
- Show command usage and risk where relevant.
- Complete ids from recent commands/services/tests where possible.
- Ensure built-ins such as `clear` do not accidentally run as shell commands when Forge owns the behavior.

Acceptance criteria:

- Slash commands are discoverable.
- Built-ins and shell commands have unambiguous behavior.

### H4. Add render regression tests

Requirements:

- Use Ratatui `TestBackend` for stable layout tests.
- Cover narrow, normal, and wide terminal sizes.
- Cover empty state, active command state, approval modal, failed command, background service, logs, and AI response.

Acceptance criteria:

- UI tests catch text overlap, missing modal content, and broken status/dashboard rendering.

---

## 12. Track I: Configuration, Persistence, and Local State

## Outcome

Forge should support durable user preferences, workspace configuration, command history, approval audit history, and session continuity without requiring external services.

## Current state

Config defaults, partial config merging, global/project config discovery, environment overrides, and CLI overrides exist. Runtime state is mostly in memory.

## Primary modules

- `src/config`
- `src/domain`
- `src/app/store.rs`
- `src/infra`
- `src/observability`

## Implementation tasks

### I1. Define local state layout

Requirements:

- Store user-level state under platform config/data directories.
- Store workspace-level Forge state under `.forge/`.
- Separate logs, history, audit, cache, and session files.
- Avoid writing secrets.

Suggested layout:

```text
.forge/
  config.toml
  logs/
    forge.log
  state/
    session.toml
    command-history.jsonl
    approvals.jsonl
    ai-history.jsonl
  cache/
    project-scan.json
    git-snapshot.json
```

Acceptance criteria:

- State directories are created only when needed.
- Files are written atomically where corruption matters.

### I2. Persist history and audit logs

Requirements:

- Persist command history.
- Persist approval decisions.
- Persist AI request/response summaries without secrets.
- Provide retention limits and compaction policy.

Acceptance criteria:

- Restarting Forge restores recent command history.
- Audit logs can be inspected and exported.

### I3. Version configuration schema

Requirements:

- Add schema version to config.
- Validate unknown or unsupported fields where helpful.
- Provide clear errors for malformed config.
- Document config examples.

Acceptance criteria:

- Invalid config gives file path and field context where possible.
- Config docs match actual schema.

---

## 13. Track J: Observability, Diagnostics, and Troubleshooting

## Outcome

Forge should be easy to debug when it fails and should surface actionable errors to users.

## Current state

Tracing to `.forge/logs/forge.log` exists. Diagnostics state exists. Panic hook restores terminal state.

## Primary modules

- `src/observability`
- `src/domain/diagnostics.rs`
- `src/app/store.rs`
- `src/ui/views/dashboard.rs`
- `src/shared/error.rs`

## Implementation tasks

### J1. Add diagnostics export

Requirements:

- Export config summary, runtime state summary, recent diagnostics, recent timeline, active tasks, recent command failures, provider status, and log path.
- Exclude secrets and large raw logs by default.

Suggested command:

- `/diagnostics export`

Acceptance criteria:

- Export file can be attached to bug reports.
- Export does not include API keys or environment secrets.

### J2. Improve error taxonomy

Requirements:

- Add structured error kinds for shell, PTY, config, Git, AI, monitoring, logs, persistence, and rendering.
- Surface recovery suggestions where possible.

Acceptance criteria:

- Common failures produce actionable messages.
- UI and logs include enough context to debug.

### J3. Add health reporting for Forge itself

Requirements:

- Track internal queue depth, active task count, dropped/coalesced event count, last render time, and diagnostic count.
- Show internal health in debug mode.

Acceptance criteria:

- Debug mode can reveal runtime pressure without external tools.

---

## 14. Track K: Packaging, Distribution, and Release Readiness

## Outcome

Forge should build and ship as a single cross-platform binary with documented installation and upgrade paths.

## Current state

The crate builds as a Rust binary. Release packaging is not yet implemented.

## Primary modules and files

- `Cargo.toml`
- `Cargo.lock`
- `.github/`
- `README.md`
- release scripts or workflows

## Implementation tasks

### K1. Define supported platforms

Requirements:

- Decide initial target triples.
- Document terminal compatibility expectations.
- Document unsupported or degraded features per platform.

Suggested initial platforms:

- macOS arm64
- macOS x86_64
- Linux x86_64
- Linux arm64

Windows support should remain architecturally planned but may require separate PTY and process inspection work.

### K2. Add release builds

Requirements:

- Add CI workflow for check, test, clippy, fmt, and release artifacts.
- Build optimized binaries.
- Generate checksums.
- Attach release artifacts.

Acceptance criteria:

- A tagged release produces downloadable binaries and checksums.

### K3. Document installation

Requirements:

- Document `cargo install --path .` for local development.
- Document binary install once release artifacts exist.
- Document config locations and first-run behavior.

Acceptance criteria:

- A new user can install, launch, configure AI, and run basic workflows from README instructions.

---

## 15. Track L: End-to-End Testing Strategy

## Outcome

Forge should have automated confidence across domain logic, runtime behavior, terminal rendering, shell execution, safety, AI providers, and integration flows.

## Current state

Unit and foundation integration tests exist. There are no PTY integration tests, process monitor tests, persistence tests, or full terminal-flow tests yet.

## Test categories

### L1. Domain and parser tests

Cover:

- slash commands
- shell built-ins
- safety classifier
- approval policy
- command/session state
- service health transitions
- Git/test/log parsers

### L2. Store transition tests

Cover:

- command lifecycle events
- background service lifecycle
- approval accept/deny
- AI request success/failure
- AI proposal application
- cancellation
- persistence restore

### L3. Runtime and infrastructure tests

Cover:

- managed command execution
- PTY execution where available
- cancellation
- project scan
- Git snapshot
- log tailing
- process/port monitor behavior

### L4. UI render tests

Cover:

- empty state
- active command
- logs dashboard
- services dashboard
- approval modal
- AI response
- error modal
- narrow terminal
- resize behavior

### L5. End-to-end smoke scenarios

Automate or document:

1. Launch Forge in a Rust repo.
2. Run `pwd`.
3. Run `clear` and confirm no shell command record is created.
4. Run `cargo test` and approve caution review.
5. Start a background service with `/bg <cmd>`.
6. Observe process, service, logs, and port.
7. Ask `/diagnose <symptom>` with mock provider.
8. Apply an AI proposal and verify safety routing.
9. Cancel a running command.
10. Quit and restart, verifying persisted history and audit records.

---

## 16. Suggested Implementation Order

### Phase 1: Command Surface Reliability

1. Harden shell built-ins such as `clear`, `cd`, `exit`, and history behavior.
2. Add persistent session cwd and command history.
3. Add managed command integration tests.
4. Improve cancellation and process cleanup.

Why first:

The command surface is the center of Forge. Every other subsystem depends on reliable command lifecycle state.

### Phase 2: PTY and Interactive Shell

1. Add PTY backend.
2. Add resize propagation.
3. Add PTY cancellation and cleanup.
4. Add manual and automated smoke coverage.

Why second:

Without PTY support Forge cannot meet the terminal-native product bar.

### Phase 3: Services, Processes, and Logs

1. Implement process snapshots.
2. Implement port discovery.
3. Add configured services.
4. Add health checks.
5. Add file log tailing.

Why third:

Operational visibility depends on real producers feeding service, process, and log state.

### Phase 4: Git, Tests, and Diagnosis Context

1. Expand Git status and diff metadata.
2. Improve project scan.
3. Add Rust test adapter.
4. Feed failures into AI context.

Why fourth:

AI diagnosis should be grounded in reliable local facts before workflow automation expands.

### Phase 5: AI Workflows and Audit

1. Harden provider diagnostics.
2. Add context budgeting and prioritization.
3. Add structured plan proposals.
4. Add patch proposals behind approval.
5. Persist audit history.

Why fifth:

AI actions should build on mature safety, command, and context foundations.

### Phase 6: Persistence, UI Completion, and Release

1. Persist session, history, approvals, and AI summaries.
2. Add scrollback, inspectors, and full keyboard UX.
3. Add diagnostics export.
4. Add release workflows and installation docs.
5. Complete end-to-end smoke testing.

Why sixth:

These steps turn the integrated system into something users can rely on daily.

---

## 17. Production Readiness Checklist

Forge is not production-ready until all of the following are true:

- `cargo fmt --check` passes.
- `cargo test` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- PTY command execution works for supported platforms.
- Managed and PTY command cancellation are reliable.
- Shell session cwd and history persist correctly.
- Risky and destructive actions require approval by default.
- Approval audit history persists.
- Background services can be started, stopped, restarted, and inspected.
- Process CPU/memory and port state are visible where supported.
- Logs can be ingested from commands and configured files.
- Git state includes staged, unstaged, untracked, and diff summary data.
- Rust test runs update `TestState`.
- AI provider status is explicit and failure modes are tested.
- AI proposals route through runtime safety classification.
- UI supports scrolling, resize, detail inspection, and modal flows.
- Terminal state restores after normal exit, Ctrl-C, command failure, and panic.
- Local state files avoid secrets and handle corruption gracefully.
- Diagnostics export is available.
- Release artifacts are produced for supported platforms.
- README documents installation, usage, config, AI setup, safety behavior, and troubleshooting.

---

## 18. Agent Handoff Template

When assigning a slice to an agent, use this shape:

```text
Implement <track/task id>: <task name>.

Context:
- Read AGENTS.md, ARCHITECTURE.md, PROJECT_BRIEF.md, README.md, and docs/IMPLEMENTATION_PLAN.md.
- Preserve module boundaries.
- Route risky/destructive actions through safety approvals.

Scope:
- Owned files/modules:
  - <paths>
- Do not edit:
  - <paths if needed>

Requirements:
- <specific behavior>
- <specific UI/state/runtime integration>
- <specific persistence/config requirements>

Acceptance criteria:
- <observable behavior>
- <tests>
- <manual smoke checks if needed>

Validation:
- cargo fmt --check
- cargo test
- cargo clippy --all-targets --all-features -- -D warnings
```

Agents should update this implementation plan when completing major tracks so future work reflects the actual system state.
