# Forge Architecture

## Document Purpose

This document defines the system architecture for Forge.

Forge is a terminal-native, production-grade AI DevOps command center built in Rust. It combines an intelligent terminal wrapper, a live operational dashboard, structured observability, grounded AI assistance, and a safety-first action system into one cohesive product.

This architecture is intended to guide implementation decisions across the codebase and establish clear system boundaries from the beginning. Forge should not be built as a demo, toy app, or loosely connected collection of utilities. It should be built as a serious developer product with maintainable internals, strong module separation, safe execution paths, and a high-trust user experience.

---

## 1. Architectural Vision

Forge should function as a unified control surface for software development and operational workflows inside the terminal.

Instead of forcing users to switch across:
- a shell
- Docker commands
- log viewers
- system monitors
- git commands
- test runners
- AI chat tools
- deployment interfaces

Forge should provide a single terminal-native runtime that can:
- execute commands
- observe the environment
- understand repository and runtime context
- explain failures
- recommend actions
- coordinate workflows
- safely execute approved actions
- preserve a visible record of what happened

Forge should feel like an operator cockpit rather than a command wrapper.

### Architectural outcomes we want
- stable and responsive terminal UX
- explicit concurrency model
- predictable state transitions
- clear separation between rendering, orchestration, and infrastructure
- safety-gated operational actions
- grounded AI behavior
- long-term extensibility without rewriting the core

---

## 2. System Goals

### Primary goals
- Build a terminal-native application that can be used continuously in real engineering workflows
- Combine command execution and dashboard visibility in one unified runtime
- Make AI assistance grounded, explainable, and operationally safe
- Support concurrent observation of services, logs, jobs, and repository state
- Preserve user trust through explicit approvals and visible execution history
- Provide a strong foundation for future remote environments, plugin systems, and richer workflows

### Secondary goals
- Cross-platform compatibility
- Single-binary distribution
- Low-friction setup and install
- Rich diagnostics and internal observability
- Clear extension points for future integrations

---

## 3. Non-Goals

The following are intentionally out of scope for the initial architecture, even though the system should remain extensible enough to support them later:

- Full cloud-hosted multi-user control plane
- Enterprise collaboration features
- Full replacement for Kubernetes dashboards
- Full replacement for IDEs
- OS-level shell replacement
- Browser-first application as the primary delivery method

These may become future expansions, but the architecture should prioritize a world-class terminal-native local runtime first.

---

## 4. Architectural Principles

### 4.1 Terminal-native first
The terminal is not just a rendering target. It is the primary product environment. Every architectural decision should preserve the speed, precision, and ergonomics expected by terminal-native users.

### 4.2 Production-grade from the start
The system should be designed for reliability, maintainability, observability, and safe behavior. We are not building a prototype architecture that will later need replacement.

### 4.3 Event-driven core
Forge is fundamentally a stateful, concurrent system. The architecture should center around typed events, predictable state transitions, and decoupled background producers.

### 4.4 Human-in-control automation
The system may propose or coordinate actions, but the user remains the decision-maker for impactful operations.

### 4.5 AI as a grounded reasoning layer
AI is not a free-floating assistant. It should be grounded in repository state, command history, logs, process data, tests, and structured system context.

### 4.6 Explicit boundaries
Rendering, orchestration, domain modeling, infrastructure access, AI orchestration, and safety logic must remain clearly separated.

### 4.7 Progressive extensibility
We should be able to add new providers, panels, environments, workflows, and integrations without destabilizing the core.

---

## 5. High-Level System Overview

Forge is composed of the following primary architectural layers:

1. **Presentation Layer**
2. **Application Layer**
3. **Domain Layer**
4. **Infrastructure Layer**
5. **Integration Layer**
6. **Persistence and Diagnostics Layer**

These layers cooperate through a typed event system and a centralized application state model.

### 5.1 Presentation Layer
Responsible for:
- terminal lifecycle
- rendering
- keyboard interaction
- layout
- panel management
- modal overlays
- visual state reflection

### 5.2 Application Layer
Responsible for:
- runtime orchestration
- event routing
- action dispatch
- state transitions
- background task supervision
- workflow coordination

### 5.3 Domain Layer
Responsible for:
- core entities
- business rules
- typed actions
- event definitions
- service models
- command/job lifecycle concepts
- approval and policy types
- AI request and response abstractions

### 5.4 Infrastructure Layer
Responsible for:
- shell execution
- process and resource inspection
- filesystem interaction
- git interaction
- log streaming
- project inspection
- system metrics collection
- config loading
- local storage

### 5.5 Integration Layer
Responsible for:
- AI provider connectivity
- external service adapters
- container tooling adapters
- future deployment adapters
- plugin-style extension points

### 5.6 Persistence and Diagnostics Layer
Responsible for:
- command history persistence
- local session state
- structured internal logging
- diagnostics export
- crash-safe state snapshots where needed

---

## 6. Runtime Model

Forge is a long-running, event-driven, asynchronous application.

The runtime consists of:
- a main UI loop
- a centralized app state store
- async task workers
- an event bus
- task supervisors
- input and command dispatch
- rendering updates triggered from state changes

### 6.1 Event-driven architecture
Background subsystems emit typed events such as:
- command started
- output chunk received
- command exited
- log line received
- process observed
- service health changed
- git state updated
- tests changed
- approval requested
- approval granted
- approval denied
- AI request started
- AI response received
- system error raised

The application layer consumes these events, updates state, emits derived events where appropriate, and drives UI updates.

### 6.2 Why event-driven
This model is important because Forge must:
- observe many concurrent sources
- remain responsive under streaming output
- support composable workflows
- make state changes explicit
- enable traceability and diagnostics
- scale to future integrations without fragile coupling

---

## 7. Core Architectural Subsystems

## 7.1 Terminal Runtime and UI System

### Responsibilities
- initialize terminal state safely
- enter and exit alternate screen mode
- handle raw input mode
- render the full application layout
- manage focus and keyboard navigation
- render modals and overlays
- support resizing
- maintain consistent user experience over long sessions

### UI composition
Forge should have a structured layout composed of:
- top status bar
- left command and AI pane
- right dashboard pane
- bottom event stream
- optional overlays and inspector views

### UI requirements
- keyboard-first interactions
- deterministic navigation
- readable dense layouts
- responsive redraw behavior
- minimal flicker under heavy updates
- support for split panes, detail drawers, and focused inspection modes

### UI constraints
Rendering must not own operational logic. UI components should consume state and dispatch typed UI actions only.

---

## 7.2 Application State Store

Forge should maintain a centralized, typed application state store.

### Responsibilities
- hold all user-visible state
- provide deterministic updates
- support rendering as a pure function of current state
- coordinate cross-subsystem visibility

### State domains
Suggested top-level domains:
- app metadata
- session state
- UI state
- input state
- project context
- command history
- running jobs
- process snapshots
- service registry
- log buffers
- test state
- git state
- AI state
- approval queue
- event timeline
- config state
- diagnostics state
- error notifications

### State design expectations
- state should be strongly typed
- transitions should be explicit
- no hidden state should live only inside rendering widgets
- ephemeral UI state and durable operational state should remain distinguishable

---

## 7.3 Event Bus

The event bus is the central nervous system of Forge.

### Responsibilities
- deliver events between producers and consumers
- decouple background workers from the app core
- preserve consistent event handling semantics
- support bounded queues and backpressure-aware behavior

### Producer examples
- shell executor
- log collectors
- process monitor
- service discovery
- resource monitor
- git watcher
- filesystem watcher
- AI adapter
- approval subsystem
- project scanner

### Consumer examples
- state reducer
- event timeline recorder
- UI refresh coordinator
- diagnostics logger
- workflow engine
- notification subsystem

### Design notes
- use typed event enums and payload structs
- avoid generic stringly-typed event systems
- prefer bounded channels where possible
- define clear ownership and lifetimes for emitted data

---

## 7.4 Command Execution Engine

The command execution engine is responsible for shell-based operations.

### Responsibilities
- run interactive and non-interactive commands
- stream stdout and stderr
- track command metadata
- support background jobs
- support cancellation
- capture environment, cwd, start time, duration, and exit status
- classify commands before execution
- integrate with approval policy

### Command model
Each command should be represented by a domain object containing:
- command id
- source type
- raw input
- normalized execution request
- working directory
- environment metadata
- safety classification
- execution status
- output buffers or handles
- timestamps
- exit status
- linked event ids

### Supported command modes
- foreground command
- background job
- managed workflow step
- AI-proposed command
- user-rerun command

### Important constraints
- executor logic must remain independent of the UI
- output handling must support streaming and chunking
- command cancellation must be explicit and safe
- command provenance must be visible

---

## 7.5 Job and Workflow Orchestrator

Forge needs a structured notion of jobs beyond raw commands.

### Responsibilities
- manage background jobs
- coordinate multi-step workflows
- track dependencies between steps
- surface job state to the UI
- support retries, cancellation, and status inspection

### Job types
- shell job
- test job
- log stream
- health check job
- AI orchestration task
- project scan
- future deploy workflow step

### Workflow model
A workflow is a sequence of typed actions with:
- explicit stages
- status transitions
- approval boundaries
- recorded outcomes

This is necessary because Forge should eventually support operational flows like:
- inspect logs
- diagnose failure
- propose fix
- patch file
- rerun tests
- restart service
- verify health

without collapsing everything into ad hoc chains of shell commands.

---

## 7.6 Observability Engine

This subsystem powers the live operational dashboard.

### Responsibilities
- observe running processes
- build service snapshots
- collect resource metrics
- map ports to listeners
- ingest logs
- evaluate health
- detect state changes
- emit operational events

### Components
- process monitor
- resource monitor
- port inspector
- service registry
- log collection system
- health evaluator

### Observability sources
- local process table
- command-managed processes
- container runtime data
- filesystem logs
- process output streams
- user-defined service definitions
- project-specific configuration

### Health model
Service health should be a typed concept such as:
- unknown
- starting
- healthy
- degraded
- unhealthy
- stopped

Each health status should be backed by reasons where possible.

### Design notes
- observability should be modular
- sources may be partial or unavailable
- dashboard views should reflect confidence and freshness where relevant
- resource collection should avoid excessive system overhead

---

## 7.7 Log Ingestion and Processing System

Logs are first-class data in Forge.

### Responsibilities
- collect logs from files, processes, and containers
- buffer and index recent output
- classify severity where possible
- support search and filtering
- associate logs with services, jobs, and commands
- feed AI diagnosis workflows with structured log context

### Log sources
- stdout/stderr of managed commands
- tailed files
- Docker logs
- future provider integrations

### Log model
Each log event should ideally include:
- timestamp
- source
- service or job identity
- stream type
- severity if known
- raw message
- parsed fields if available

### Design notes
- logs should not be treated as a monolithic text blob
- bounded buffering is important
- log views should support recency-focused navigation
- high-volume streams must not break UI responsiveness

---

## 7.8 Service Discovery and Registry

Forge should maintain a runtime service registry.

### Responsibilities
- represent the services relevant to the current session
- associate services with ports, jobs, health checks, logs, and metadata
- update service status as observations change
- support user-defined and auto-detected services

### Service sources
- project config
- command launches
- Docker Compose inspection
- process pattern matching
- user declarations
- future orchestrator integrations

### Service model
A service should include:
- service id
- display name
- type
- source
- pid or runtime handle where applicable
- port mappings
- health status
- related logs
- tags
- environment association
- last observed timestamp

---

## 7.9 Project Context Engine

Forge must understand the project it is operating in.

### Responsibilities
- detect repository root
- inspect git state
- identify project shape
- infer stack characteristics
- discover common commands
- identify likely services
- identify known config files
- provide structured context to the AI subsystem

### Context sources
- filesystem layout
- git metadata
- lockfiles
- docker compose files
- package manifests
- workspace files
- user config
- test config
- scripts and task definitions

### Project context outputs
- repository metadata
- branch information
- modified file summaries
- likely runtime stack
- command suggestions
- detected service hints
- config hints for observability and workflows

---

## 7.10 Git Context Subsystem

Git awareness should be built in, not added as an afterthought.

### Responsibilities
- read branch and status
- identify changed files
- summarize staged vs unstaged changes
- inspect recent commits
- provide diff metadata for AI context
- track repo cleanliness and working state

### Design notes
- git state should be refreshed incrementally
- expensive operations should be bounded or cached
- git context should be visible in the status bar and dashboard
- AI workflows should use git state as grounding when relevant

---

## 7.11 Test Awareness Subsystem

Tests are another first-class operational signal.

### Responsibilities
- ingest test run outcomes
- track recent failures
- correlate tests with jobs and files
- expose test summaries to dashboard and AI workflows
- support future adapters for multiple ecosystems

### Test model
- test run id
- runner type
- status
- pass/fail counts
- failed test identifiers
- duration
- source command
- associated files if known

### Design notes
This subsystem should not rely only on naive parsing of terminal output in the UI layer. Test awareness belongs in the domain/application model.

---

## 7.12 AI Orchestration Layer

This layer is responsible for grounded AI-assisted operational behavior.

### Responsibilities
- accept slash commands and AI requests
- gather structured context
- assemble prompts or tool requests
- normalize results into domain-safe outputs
- return explanations, summaries, plans, or proposed actions
- coordinate approval requirements before execution

### Inputs to AI
Potential inputs include:
- project context
- git state
- recent commands
- recent command failures
- service health
- relevant logs
- test failures
- recent event timeline
- user query
- active safety mode

### Outputs from AI
Allowed outputs should be normalized into typed forms such as:
- explanation
- diagnosis
- recommendation list
- proposed command sequence
- proposed patch request
- summary
- open questions
- structured workflow plan

### Required constraints
- raw model text must not directly mutate state
- model output must be validated and classified
- all proposed actions must go through internal policy checks
- AI should be visible as advisor and workflow generator, not hidden executor

---

## 7.13 Safety and Approval Engine

Safety is a core system, not an optional check.

### Responsibilities
- classify actions
- define policy rules
- enforce approvals
- generate user-visible approval requests
- record user decisions
- reject policy-violating actions
- surface mode and action risk clearly in the UI

### Action classes
- passive
- safe
- risky
- destructive

### Sample classifications
Passive:
- read logs
- inspect git status
- list processes
- summarize failures

Safe:
- rerun tests
- run formatting
- run lint
- collect diagnostics

Risky:
- write file changes
- restart services
- modify environment config
- run codegen that changes tracked files

Destructive:
- delete files
- kill broad process groups
- force reset git state
- prune resources
- destructive cleanup

### Safety policy requirements
- risky actions require explicit approval
- destructive actions always require explicit approval
- the user must see what will happen before it happens
- execution history must record proposal and decision
- ambiguous actions should be escalated to stricter treatment

---

## 7.14 Command Parsing and Intent Resolution

Forge should support both raw shell commands and structured slash commands.

### Responsibilities
- distinguish shell input from internal commands
- parse slash commands into typed intents
- validate arguments
- route requests to the correct subsystem
- preserve raw input history

### Input types
- raw shell command
- slash command
- AI free-form request
- keyboard shortcut-triggered action

### Slash command examples
- `/explain`
- `/summarize-logs`
- `/diagnose`
- `/review-changes`
- `/fix-tests`
- `/restart-service`
- `/project-map`

### Design notes
Intent resolution should remain explicit and typed. Do not overload one free-form input path with too many hidden behaviors.

---

## 7.15 Configuration System

Forge requires layered configuration.

### Responsibilities
- load and validate configuration
- merge multiple config sources
- expose runtime configuration
- support user customization
- preserve versioned schema evolution where possible

### Config layers
- built-in defaults
- global user config
- project-local config
- environment variables
- CLI flags

### Config areas
- shell settings
- panel preferences
- service definitions
- log sources
- AI provider settings
- safety policies
- history limits
- diagnostics settings
- theme and rendering settings

### Validation requirements
- malformed config should degrade gracefully
- useful errors should be surfaced to users
- invalid config should not crash the application without explanation

---

## 7.16 Persistence Layer

Forge should persist only the state that improves continuity and usability.

### Candidate persisted data
- command history
- recent sessions
- panel preferences
- cached project profiles
- AI conversation/session metadata if needed
- diagnostics settings
- local workflow templates

### Persistence constraints
- persistence must be explicit
- data should be versioned where appropriate
- corrupted state should be recoverable
- do not mix domain state carelessly with transient UI data

---

## 7.17 Diagnostics and Internal Observability

Forge itself must be diagnosable.

### Responsibilities
- structured internal logs
- subsystem-level tracing
- error classification
- event timeline export
- optional debug overlays
- crash diagnostics

### Design goals
- help users debug Forge itself
- help maintainers understand subsystem behavior
- preserve trust when failures occur
- support development and production troubleshooting

---

## 8. Detailed Data Flow

## 8.1 Startup flow
1. Process starts
2. CLI args parsed
3. Config loaded and validated
4. Project root detected
5. App state initialized
6. Event bus initialized
7. Background workers start
8. Initial project scan runs
9. Terminal UI enters active mode
10. Initial render occurs
11. Incremental state hydration updates dashboard and context views

## 8.2 Shell command flow
1. User enters raw command
2. Parser classifies command input
3. Safety classifier evaluates command
4. If approval required, approval request is emitted
5. On approval, executor spawns process
6. Output events stream into bus
7. State store updates command and job records
8. Related dashboard components update
9. Event stream records lifecycle
10. Exit status and timing recorded
11. Optional downstream workflows triggered

## 8.3 Slash command AI flow
1. User enters slash command
2. Intent resolver maps request to AI or internal subsystem
3. Context builder gathers relevant state
4. AI request created
5. AI response returned
6. Response normalized into explanation or proposed actions
7. Safety engine classifies proposed actions
8. If approval needed, user is prompted
9. Approved actions run through executor or workflow engine
10. Results recorded in timeline and state

## 8.4 Log ingestion flow
1. Source registered
2. Collector streams log entries
3. Parser normalizes metadata if possible
4. Relevant log events emitted
5. Log buffers updated
6. Service associations refreshed
7. UI log views update
8. AI-relevant summaries optionally refreshed

## 8.5 Service health flow
1. Process monitor or source emits observation
2. Service registry updates runtime association
3. Health evaluator computes current status
4. Health change event emitted if changed
5. Dashboard updates
6. Event timeline records state transition

---

## 9. State Model

Forge should use a central `AppState` with clear substate partitions.

### Suggested top-level structure

```text
AppState
├── app
├── ui
├── session
├── config
├── project
├── commands
├── jobs
├── services
├── processes
├── logs
├── tests
├── git
├── ai
├── approvals
├── timeline
├── diagnostics
└── notifications
```

### State design rules

- each domain should have clear ownership
- avoid duplicate truth for the same concept
- expensive derived views should be computed explicitly
- state transitions should be traceable to events or actions
- reducers/transition handlers should be testable independently

---

## 10. Concurrency Model

Forge must handle multiple concurrent activities safely.

### Concurrent concerns

- UI input
- terminal rendering
- command execution
- log streaming
- process observation
- git refresh
- AI requests
- workflow management
- persistence writes
- diagnostics logging

### Concurrency strategy

- Tokio runtime
- supervised async tasks
- bounded channels
- explicit cancellation tokens
- lifecycle-aware worker management
- minimal shared mutable state outside central store boundaries

### Rules

- background tasks must be cancellable
- workers should fail loudly into diagnostics, not silently disappear
- avoid detached tasks with no ownership or supervision
- protect UI responsiveness above noncritical background refreshes

---

## 11. Error Handling Strategy

Forge must degrade gracefully.

### Principles

- use typed errors at subsystem boundaries
- attach context to infrastructure failures
- avoid panics in steady-state operation
- surface recoverable errors to users appropriately
- record system errors in diagnostics and timeline

### Categories

- configuration errors
- execution errors
- permission errors
- environment discovery errors
- provider integration errors
- parsing and normalization errors
- rendering and terminal errors
- internal invariant violations

### User-facing behavior

- visible but non-noisy errors
- actionable messaging where possible
- safe fallback behavior
- explicit notification when capabilities are unavailable

---

## 12. Security and Trust Model

Forge is a local tool, but trust still matters deeply.

### Security principles

- never execute hidden destructive behavior
- never trust model output as executable intent
- preserve user visibility into what is proposed and run
- sanitize and normalize command requests
- make provenance visible for actions and patches
- avoid dangerous defaults

### Trust model

User trust depends on:

- predictable behavior
- visible approvals
- clear status
- accurate state representation
- reversible or inspectable actions where possible
- explicit history of what happened

---

## 13. Extensibility Model

Forge should be modular enough to support future additions.

### Future extension areas

- remote host support
- SSH target management
- Docker Compose deep integration
- Kubernetes awareness
- deploy providers
- plugin system
- custom slash command packs
- multi-project workspaces
- desktop wrapper
- richer observability providers

### Extensibility strategy

- define provider traits where appropriate
- avoid hardcoding assumptions into the app core
- preserve typed adapters for integrations
- keep domain concepts generic enough to support multiple implementations

---

## 14. Packaging and Distribution Architecture

Forge is distributed primarily as a single native binary.

### Requirements

- cross-platform builds
- simple installation
- low runtime dependency burden
- deterministic behavior across supported environments

### Packaging goals

- GitHub Releases
- Homebrew support
- future package manager support
- optional auto-update considerations later

### Deployment posture

This is a **local terminal-native product** first, not a hosted service.

---

## 15. Proposed Source Layout

```text
forge/
  src/
    main.rs

    app/
      mod.rs
      runtime.rs
      bootstrap.rs
      state.rs
      events.rs
      actions.rs
      reducers.rs
      supervisors.rs

    ui/
      mod.rs
      terminal.rs
      layout.rs
      focus.rs
      input.rs
      theme.rs
      renderer.rs
      views/
        mod.rs
        status_bar.rs
        command_pane.rs
        dashboard.rs
        event_stream.rs
        modal.rs
        inspectors.rs

    domain/
      mod.rs
      app.rs
      command.rs
      job.rs
      service.rs
      process.rs
      log.rs
      git.rs
      test.rs
      ai.rs
      approval.rs
      project.rs
      timeline.rs
      config.rs
      diagnostics.rs

    commands/
      mod.rs
      parser.rs
      intent.rs
      slash.rs
      registry.rs

    workflows/
      mod.rs
      engine.rs
      definitions.rs
      planner.rs

    safety/
      mod.rs
      classifier.rs
      policy.rs
      approvals.rs

    ai/
      mod.rs
      orchestrator.rs
      context.rs
      requests.rs
      responses.rs
      normalization.rs
      providers/
        mod.rs
        openai.rs

    infra/
      mod.rs

      shell/
        mod.rs
        executor.rs
        process.rs
        streams.rs

      monitor/
        mod.rs
        processes.rs
        resources.rs
        ports.rs
        health.rs

      logs/
        mod.rs
        collector.rs
        parser.rs
        sources.rs

      git/
        mod.rs
        repository.rs

      project/
        mod.rs
        scanner.rs
        detectors.rs

      config/
        mod.rs
        loader.rs
        schema.rs
        merge.rs

      persistence/
        mod.rs
        history.rs
        sessions.rs
        store.rs

      diagnostics/
        mod.rs
        logging.rs
        export.rs

      containers/
        mod.rs
        docker.rs

    shared/
      mod.rs
      error.rs
      ids.rs
      time.rs
      utils.rs

  tests/
    integration/

  docs/
    architecture/
    flows/
    decisions/

  assets/
```

---

## 16. Architectural Decision Guidance

When implementing Forge, use these heuristics:

### Add a new subsystem when:

- a concern has its own lifecycle
- it owns distinct state
- it touches external systems
- it needs test coverage independent of rendering

### Keep logic out of UI when:

- it changes business state
- it talks to external systems
- it manages background tasks
- it interprets AI results
- it classifies safety

### Prefer typed domain models when:

- data is user-visible
- logic depends on status transitions
- event history matters
- safety depends on classification

---

## 17. Initial Build Priorities

The architecture should support the following early implementation sequence:

1. application runtime and terminal shell
2. app state and event system
3. layout and panel rendering
4. command execution engine
5. event timeline and command history
6. process and service monitoring
7. log ingestion
8. project and git context
9. test awareness
10. safety and approval engine
11. AI orchestration
12. persistence and diagnostics hardening
13. distribution and operational polish

The order may shift slightly, but the architecture should preserve these dependencies.

---

## 18. Quality Attributes

Forge should optimize for:

### Reliability

The tool should remain stable during long sessions and under streaming output.

### Responsiveness

UI responsiveness should remain strong even under background activity.

### Safety

High-impact actions must always be explicit and inspectable.

### Clarity

The system should be understandable both to users and maintainers.

### Extensibility

New capabilities should not require rewiring the whole application.

### Traceability

Important actions and state changes should be visible and diagnosable.

---

## 19. Summary

Forge should be built as a modular, event-driven Rust application with:

- a reactive terminal UI
- a centralized typed app state
- concurrent background observers
- a clear command and workflow engine
- first-class observability
- grounded AI orchestration
- a robust safety and approval system
- strong diagnostics and persistence foundations

This architecture is intended to support a real product from the start: one that is trustworthy, maintainable, terminal-native, and capable of becoming a serious daily-use developer tool.
