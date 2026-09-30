## Lime v1.147.0

Simplified Chinese release notes are the primary version.

### New Features

- The TUI task center now supports project/status grouping, status filters, search, pagination, task creation, renaming, and switching.
- Added dynamic shortcut help and selection tabs, with consistent hints across the task center, transcript, pager, and composer.
- Added a borderless model picker with current-model indicators, page navigation, and narrow-terminal wrapping.
- Unified search matching and selected-row scrolling across file, Skill, and command popups, with Unicode highlighting and long-path presentation.
- Approval panels offer full details through Ctrl-A, preserve actionable choices in narrow terminals, and return to the pending approval when details are closed.
- MCP OAuth login notifications can identify the thread and login attempt. Added App Server-backed `lime mcp logout` with idempotent per-server credential removal.
- Expanded CLI sandbox, approval, and permission argument inheritance, plus local Plugin marketplace discovery and available-plugin listing.

### Fixes

- Fixed reading-position, selection, and state restoration across history pagination, terminal reflow, and thread switching.
- Fixed multiline Markdown quote paste, paste bursts, composer layout, and narrow-terminal status rendering.
- Fixed buffered characters reappearing in the draft after a command popup consumes Enter, while preserving newlines and tabs inside pasted text.
- Prevented late clipboard results from updating a new draft or an old session, and clarified availability rules for remote terminals and X11 PRIMARY.
- Removed token injection from npm Trusted Publishing while retaining the existing OIDC and provenance publishing contract.

### Improvements and Refactoring

- Moved clipboard reads and writes to session-level background workers with timeouts, cancellation, and late-result handling to keep slow desktop clipboard services from blocking terminal input.
- Consolidated borderless session headers, `/status`, footers, task rows, and activity summaries while retaining canonical state and complete raw values.
- Split task-center, status-formatting, clipboard, selection-style, and transcript-reading bookmark modules while preserving the single App Server chain.

### Testing and Quality

- Expanded regression coverage for MCP OAuth protocol/schema/client contracts, CLI permissions and plugins, TUI interactions, and clipboard behavior.
- Updated real CLI/TUI stdio/PTY Gate B fixtures, MCP fixtures, structure inventory, and npm OIDC publishing guards.

### Documentation

- Updated CLI permissions, plugin management, TUI keymap configuration, operations guidance, and App Server command contracts.
- Updated the TUI/CLI Codex-alignment execution plan and structure records.

### Other

- Desktop and CLI/TUI continue to share App Server JSON-RPC, RuntimeCore, and Thread/Turn/Item persistence without a parallel runtime or history store.
- Local validation results are recorded in the release execution plan; GitHub Actions validates platform artifacts, signing, notarization, and npm distribution.

**Full changes**: `v1.146.0` -> `v1.147.0`
