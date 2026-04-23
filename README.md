# Forge

Forge is a terminal-native DevOps command center built in Rust.

It combines a command execution surface, an operational dashboard, structured runtime state, AI-assisted diagnosis, and safety-gated actions into one terminal application. Forge is intended for engineers who want a single local control surface for running commands, watching services, reading logs, inspecting Git and test state, and asking context-aware AI questions without giving up operator control.

Forge is designed as a serious developer tool: terminal-native first, strongly typed, observable, safe by default, and packaged as a single native binary.

---

## What Forge Is For

Forge is intended to be launched inside a project directory and used as a daily operational workspace.

The product goal is to let a user:

- run shell commands
- stream stdout and stderr
- track foreground commands and background jobs
- inspect service and process health
- watch logs and important events
- inspect repository state
- understand test failures
- ask AI-grounded operational questions
- review and approve proposed actions before execution
- keep a visible record of commands, approvals, and outcomes

Forge should feel like an operator cockpit for local development and DevOps workflows, not a generic chatbot or a thin command wrapper.

---

## Current Status

Forge currently has a working foundation:

- Ratatui terminal UI with status bar, command pane, dashboard, event stream, and modals
- async runtime loop built on Tokio and Crossterm events
- managed shell command execution
- streaming command output into structured logs
- command, job, process, service, Git, log, test, timeline, approval, and AI domain models
- slash command parser and command suggestions
- safety classification and approval routing
- AI provider abstraction with mock, OpenAI, disabled, and unconfigured providers
- TOML configuration loading from defaults, global config, project config, environment, and CLI flags
- structured tracing to `.forge/logs/forge.log`
- foundation tests for parsing, state transitions, safety routing, AI proposal handling, and rendering

Forge is not yet complete end to end. The remaining work is tracked in `docs/IMPLEMENTATION_PLAN.md`.

Major remaining areas include:

- PTY-backed interactive shell support
- persistent shell sessions and cwd handling
- durable command history and approval audit history
- real service discovery, port inspection, and health checks
- process CPU/memory monitoring
- file and service log ingestion
- test result adapters
- expanded Git context and diff summaries
- richer AI workflows and patch proposals behind approvals
- scrollback, inspectors, and complete keyboard workflows
- release packaging and install documentation

---

## Technology

Forge uses the following core stack:

- Language: Rust
- UI: Ratatui
- Terminal events: Crossterm
- Async runtime: Tokio
- Serialization: Serde
- Configuration: TOML
- Logging/telemetry: tracing
- Packaging target: single native binary

---

## Running Forge

From the repository root:

```sh
cargo run
```

Show CLI options:

```sh
cargo run -- --help
```

Run Forge in a specific working directory:

```sh
cargo run -- --cwd /path/to/project
```

Run with debug logging:

```sh
cargo run -- --debug
```

Forge writes internal logs to:

```text
.forge/logs/forge.log
```

---

## Basic Usage

Launch Forge inside a project:

```sh
cargo run -- --cwd /path/to/project
```

Inside Forge, type commands into the command pane.

Examples:

```text
pwd
git status
cargo test
/bg cargo run
/ai summarize the latest failure
/diagnose why tests are failing
/apply 1
/clear
/quit
```

Plain `clear` is treated as a Forge built-in clear action. It should not spawn the OS `clear` command or create a successful command record.

---

## Slash Commands

Currently supported slash commands include:

```text
/help
/ai <prompt>
/diagnose <prompt>
/apply <n>
/bg <cmd>
/cancel <id>
/approve
/deny
/tab next
/tab prev
/clear
/quit
/exit
/next-tab
/prev-tab
/run-proposal <n>
```

Plain shell commands are also supported. A trailing `&` marks a command as background work.

---

## Safety Model

Forge classifies actions before execution.

Current safety classes:

- Passive: read-only inspection
- Safe: low-risk command execution
- Caution: visible inline review before execution when configured
- Risky: modal approval required
- Destructive: explicit confirmation required by default

AI-proposed commands are not trusted blindly. When a user applies an AI proposal, Forge reclassifies the command locally at runtime and routes it through the same approval pipeline as user-entered commands.

---

## AI Configuration

AI is enabled by default in configuration, but no provider is selected by default.

Use the mock provider for local testing:

```sh
cargo run -- --ai-provider mock
```

Use OpenAI:

```sh
OPENAI_API_KEY=... cargo run -- --ai-provider openai
```

Optionally choose a model:

```sh
OPENAI_API_KEY=... cargo run -- --ai-provider openai --ai-model gpt-5-mini
```

AI requests are grounded in Forge's current state: project context, Git snapshot, recent commands, services, tests, logs, and timeline entries.

---

## Configuration

Forge loads configuration from:

1. built-in defaults
2. global config at the platform config directory under `forge/config.toml`
3. project config at `.forge/config.toml`
4. environment variables
5. CLI flags

Example project config:

```toml
[ui]
tick_rate_ms = 120
main_split_pct = 40
event_stream_height = 8

[commands]
history_limit = 200
scrollback_limit = 2000

[safety]
caution_requires_review = true
destructive_requires_confirmation = true

[ai]
enabled = true
provider = "mock"
model = "gpt-5-mini"
request_timeout_secs = 30

[observability]
log_level = "info"
log_to_file = true
```

Environment overrides include:

```text
FORGE_TICK_RATE_MS
FORGE_LOG_LEVEL
FORGE_SHELL
FORGE_AI_ENABLED
FORGE_AI_PROVIDER
FORGE_AI_MODEL
FORGE_AI_TIMEOUT_SECS
FORGE_EVENT_STREAM_HEIGHT
FORGE_MAIN_SPLIT_PCT
```

---

## Development

Run the full local verification set:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Useful focused checks:

```sh
cargo test parses_plain_clear_as_forge_builtin
cargo test foundation
cargo run -- --help
```

---

## Repository Documents

Important project documents:

- `AGENTS.md`: repository instructions and engineering principles for coding agents
- `PROJECT_BRIEF.md`: product goals, intended users, use cases, and requirements
- `ARCHITECTURE.md`: system architecture and subsystem boundaries
- `docs/IMPLEMENTATION_PLAN.md`: remaining implementation work required for end-to-end completion
- `docs/README.md`: documentation layout

---

## Project Direction

Forge should remain:

- terminal-native
- production-grade
- modular
- strongly typed
- observable
- safe by default
- extensible without large rewrites
- packaged as a single installable binary

The core standard for every change is whether it moves Forge closer to being a polished, trustworthy, production-grade terminal-native DevOps system.
