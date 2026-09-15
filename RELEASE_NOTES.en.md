## Lime v1.145.0

Simplified Chinese release notes are the primary version.

### New Features

- Continued Codex-shaped TUI owners for paginated history, resume/transcript browsing, export, MCP
  inventory, user-input and approval flows, model/agent/resume pickers, and slash command popups.
- Added Vim editing sessions, undo/redo, reverse-history acceptance, image attachment queue editing,
  and unified multi-agent, tool-lifecycle, and reasoning-status projection in the composer.

### Fixes

- Prevented late streaming deltas from reopening completed items or creating transcript rows after a
  terminal turn.
- Fixed cross-page completion separators, review-prompt deduplication, cursor rereads, and transcript
  scroll anchoring.
- Fixed narrow-terminal wrapping and overflow for CJK/emoji text, option windows, footers, cursors,
  pickers, and composer layout.
- Fixed CLI npm platform-package resolution and naming, V8/Linux dependency handling, and npm token
  propagation in the release job.

### Improvements and Refactoring

- Split TUI input, submission, interrupt, turn/tool lifecycle, and history-cell behavior into
  Codex-shaped single owners while preserving the App Server JSON-RPC and canonical
  Thread/Turn/Item projection chain.
- Consolidated localized MCP, approval, user-input, status/footer, and streaming copy with
  fail-closed narrow-screen behavior across five locales.

### Testing and Quality

- Added TestBackend/VT100 regressions for TUI pagination, overlays, composer, pickers, MCP, and
  projection, and refreshed the TUI structure inventory and generated protocol types.
- Passed targeted and all-targets TUI tests, Clippy `-D warnings`, Cargo fmt, protocol contract tests,
  real PTY/alternate-screen Gate B, and `git diff --check`.

### Documentation

- Updated the TUI/CLI Codex alignment execution plan and structure inventory.

### Other

- Kept the App Server user-input schema and generated TypeScript types synchronized; cross-platform
  CLI artifacts continue to publish through GitHub Actions.

**Full changes**: `v1.144.1` -> `v1.145.0`
