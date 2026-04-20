---
name: terminal-ui-premium
description: Design and implement premium terminal-native interfaces for Ratatui or other TUI/CLI products. Use this skill when improving terminal UX, command panes, dashboards, onboarding, keyboard navigation, focus hierarchy, empty states, or visual polish for CLI-native tools.
---

This skill guides premium terminal UX work for production-grade CLI and TUI products.

Use it when the user wants the interface to feel calmer, more native, easier to navigate, and more trustworthy without turning the terminal into a fake GUI.

## Workflow

1. Inspect the existing render tree first.
   Identify where layout, theme tokens, focus state, and empty states are defined before changing visuals.

2. Read [references/benchmark-cli-ux.md](references/benchmark-cli-ux.md) when choosing a direction.
   It distills patterns from Ghostty, Warp, GitHub CLI, and Lazygit.

3. Pick one dominant direction and stay consistent.
   For Forge, prefer calm operational minimalism: dark surfaces, quiet chrome, explicit focus, strong keyboard discoverability, and compact status language.

4. Apply hierarchy in this order:
   - primary work surface first
   - active context second
   - navigation affordances third
   - chrome last

5. Keep onboarding progressive.
   Empty states should teach one next action, not explain the whole product.

6. Preserve terminal ergonomics.
   Do not depend on color alone, avoid noisy animation, keep labels short, and make all important state readable as plain text.

## Design Rules

- Use spacing, contrast, and density before adding more borders.
- Focus should be obvious at a glance.
- A status badge should communicate state in 1 to 2 words.
- Keep one clear input anchor. If input is always available, it should feel pinned and dependable.
- Group command + context + outcome tightly so the user can scan history quickly.
- Prefer contextual shortcut rails over giant help paragraphs.
- Empty views should always answer: what appears here, and what should I do next?
- Accessibility matters in terminals too: stable text, readable contrast, and redundant cues beat flashy redraws.

## Ratatui Guidance

- Put palette decisions in `theme.rs`, not inline in views.
- Use fixed-height header bands for identity, mode, and status.
- Use subtle surface changes for nested regions instead of stacking heavy borders everywhere.
- Keep pane titles short and product-like.
- Align tabular content and truncate aggressively to avoid ragged walls of text.
- When adding onboarding hints, keep them tied to the currently focused pane.

## Delivery Standard

Ship the visual system and the behavior together:
- updated theme tokens
- improved focus states
- clearer empty states
- tighter navigation hints
- verified compile/test pass
