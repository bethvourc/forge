# Premium CLI/TUI Benchmarks

These notes are distilled from official product docs and repos. Re-read the linked sources when making larger design calls.

## Ghostty

Sources:
- https://ghostty.org/docs/about
- https://ghostty.org/docs/features
- https://ghostty.org/docs/config/keybind/reference

What to borrow:
- Native feel over decorative chrome. Ghostty explicitly aims to look, feel, and behave like a purpose-built native app on each platform.
- Fast + feature-rich + native is the standard, not a tradeoff.
- Platform conventions matter. Familiar shortcuts and window behaviors reduce learning cost.
- Useful secondary surfaces such as quick terminal, tabs, splits, and command palette should feel immediate, not bulky.

Implication for Forge:
- The UI should feel operational and settled by default.
- Focus states, keyboard hints, and pane titles should feel native and unsurprising.
- Styling should support the workflow, not advertise itself.

## Warp

Sources:
- https://docs.warp.dev/terminal/blocks
- https://docs.warp.dev/terminal/appearance/input-position

What to borrow:
- Commands and output are easier to scan when treated as a single unit.
- A pinned input anchor reduces disorientation.
- Navigation improves when the UI makes history feel structured instead of like an endless text dump.
- Input position and command palette are discoverable, not hidden power-user features.

Implication for Forge:
- Keep the command pane visually anchored.
- Tie recent command, AI state, and approvals closely to the current editor context.
- Make slash actions feel like a real palette, not a loose autocomplete list.

## GitHub CLI

Sources:
- https://cli.github.com/manual/gh_help_formatting
- https://github.blog/engineering/user-experience/building-a-more-accessible-github-cli/

What to borrow:
- Text output can still feel polished when alignment, truncation, hyperlinks, and relative time are handled deliberately.
- Accessibility in terminals requires stable text, high contrast, and structure that assistive tech can infer from plain text.
- Avoid relying on redraw-heavy spinners when contextual status text can do the job better.
- Color should be customizable and should never be the only carrier of meaning.

Implication for Forge:
- Use labels plus color for states like dirty, queued, failed, and approved.
- Prefer static textual status summaries over ornamental motion.
- Keep tables, badges, and event lines readable in plain text.

## Lazygit

Source:
- https://github.com/jesseduffield/lazygit

What to borrow:
- Terminal tools feel premium when they collapse painful workflows into obvious, keyboard-first actions.
- Dense UIs can still feel approachable when every pane has a clear purpose and the next action is obvious.
- Filtering, contextual actions, compare flows, undo, and history all increase trust because the user stays in control.

Implication for Forge:
- Every pane should make the next move legible.
- Empty states should point to one concrete action.
- Dense operational data is fine if hierarchy stays crisp.

## Shared Principles

- Calm default state, strong active state.
- One dominant work surface, not five competing ones.
- Progressive disclosure: teach only the next action.
- Keyboard hints belong near the place they matter.
- Dense information is acceptable when structure is explicit.
- Premium terminal UX is mostly restraint, clarity, and confidence.
