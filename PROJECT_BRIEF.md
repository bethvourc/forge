# Forge

## Overview

Forge is a terminal-native, AI-powered DevOps command center built in Rust.

It combines two systems into one cohesive product:

1. An intelligent terminal wrapper that supports both direct shell execution and higher-level AI-assisted operational workflows
2. A live DevOps dashboard that provides real-time visibility into services, logs, tests, processes, git state, and system health

Forge is designed to feel like a serious developer tool, not a demo utility. It should be stable, composable, operationally safe, and suitable for daily use by engineers working across local development environments, self-hosted systems, and eventually remote environments.

The primary product form is a cross-platform terminal-native application distributed as a single compiled binary.

## Vision

Forge should become the central operating surface for modern software development inside the terminal.

Instead of splitting work across a shell, log viewer, task runner, test output, Docker commands, git commands, and an external AI assistant, Forge should unify these workflows into a single system that can:

- execute commands
- monitor system state
- understand what is happening in the repository and runtime environment
- explain failures
- recommend actions
- perform approved actions safely
- keep the user in control

Forge should reduce operational friction without hiding complexity from the user. It should improve awareness, speed, and confidence while preserving the precision and trust expected from terminal-native workflows.

## Product Principles

### 1. Terminal-native first

Forge is first and foremost a terminal application. It should feel natural to users who live in the command line.

### 2. Production-grade by design

The system should be structured from the beginning for maintainability, reliability, observability, extensibility, and safe operation.

### 3. Human-in-control automation

Forge can recommend and perform actions, but the user must remain in control. Risky and destructive actions must be clearly surfaced and gated.

### 4. Operational visibility is a first-class feature

The product is not just a command runner. It must continuously expose what is happening across the environment.

### 5. AI is an augmentation layer, not a gimmick

The AI system should be tightly grounded in observable system context, repository state, command output, logs, and structured runtime data. Forge should avoid vague assistant behavior and favor precise, context-aware assistance.

### 6. Extensible architecture

The codebase should support future additions such as remote targets, deployment integrations, incident workflows, plugin systems, and richer AI orchestration without requiring a rewrite.

## Core User Experience

A user launches Forge inside a project directory.

Forge detects and loads project context, initializes the UI, begins observing the environment, and presents a structured interface with:

- a top status bar showing repo, branch, environment, health, and mode
- a command and AI interaction pane
- a live operational dashboard
- an event stream showing actions, transitions, and notable system events

From that interface, the user can:

- run shell commands
- view command history and output
- inspect services and process health
- stream and filter logs
- monitor test state
- inspect git status and diffs
- ask AI-grounded questions about failures or behavior
- request operational help through slash commands
- approve safe automation flows

Forge should feel like a high-trust operator cockpit.

## Target Users

### Primary users

- software engineers
- platform engineers
- DevOps engineers
- founders and solo builders
- developer tooling enthusiasts
- power users who work heavily in the terminal

### Secondary users

- SRE-style operators working locally
- teams managing multi-service development environments
- engineers debugging distributed local setups
- AI-assisted coding users who want a more operational interface

## Primary Use Cases

### Local service operations

Users can start, stop, restart, inspect, and monitor local services from one surface.

### Failure diagnosis

Forge can correlate logs, failing commands, test results, process state, and repository changes to help explain what is broken and why.

### AI-assisted command workflows

Users can issue high-level operational commands and receive grounded, inspectable action plans.

### Developer environment visibility

Forge can reveal service status, port usage, test failures, container health, and event history continuously.

### Safe execution workflows

Forge can propose sequences of actions and require approval before impactful operations.

## Scope

Forge should support an end-to-end system with the following major capabilities:

- terminal UI shell and dashboard
- command execution and process tracking
- background job management
- streaming stdout and stderr
- system monitoring
- service discovery
- process inspection
- log ingestion and filtering
- git context awareness
- test result awareness
- event timeline
- slash command system
- AI integration layer
- action approval and safety model
- configuration system
- cross-platform packaging and distribution
- structured logging and diagnostics
- extensibility for future plugins and integrations

## Functional Requirements

### Command execution

- run shell commands interactively
- support foreground and background execution
- stream output in real time
- capture exit codes, duration, and metadata
- maintain command history
- allow command rerun and replay

### Dashboard visibility

- show active services and statuses
- display process metadata
- surface ports and resource consumption
- show recent logs and log severity markers
- show recent tests and failing states
- show repo status, diffs, and branch context
- show event stream and action history

### AI interaction

- support slash commands and natural-language requests
- ground all AI actions in current project and runtime context
- summarize command failures, logs, and operational states
- propose actionable next steps
- generate patches or command sequences when appropriate
- require approval for risky actions

### Safety and approvals

- define clear action classes such as passive, safe, risky, and destructive
- block or require approval on risky operations
- surface exactly what action is being proposed
- log approvals and user decisions

### Configuration

- support project-local config
- support user-global config
- support environment-specific behavior
- allow customization of panels, commands, providers, and rules

### Observability

- structured internal logging
- optional debug mode
- crash-safe error handling
- diagnostics export for troubleshooting

## Non-Goals

The following are explicitly not the initial focus:

- full cloud-hosted SaaS control plane
- enterprise RBAC and multi-user collaboration
- complete Kubernetes platform replacement
- full desktop-first UX
- replacing standard shell behavior at the OS level
- becoming a general-purpose IDE

These may become future extensions, but Forge should remain focused on being a world-class terminal-native operational interface.

## Technical Direction

- Language: Rust
- UI: Ratatui
- Async runtime: Tokio
- Distribution: single native binary
- Primary mode: local terminal-native application
- Future expansion: optional desktop wrapper or remote agent model

## Quality Bar

Forge should be engineered with the expectation that it can become a serious product.

This means:

- coherent architecture
- strong module boundaries
- testable core logic
- deterministic behavior where possible
- graceful failure modes
- explicit state transitions
- safe handling of command execution
- thoughtful UX for high-signal terminal operation
- code quality suitable for long-term maintenance

## Success Criteria

Forge is successful when a user can open it inside a real project and confidently use it as a daily operational surface for:

- running commands
- monitoring services
- tracking logs
- understanding failures
- navigating project state
- taking AI-assisted actions safely

A successful implementation should feel integrated, stable, and trustworthy, not experimental.
