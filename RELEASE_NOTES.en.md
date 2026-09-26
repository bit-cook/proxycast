## Lime v1.146.0

Simplified Chinese release notes are the primary version.

### New Features

- Added main-transcript Find search, a sticky prompt header, dual reading modes, follow control, and unseen-activity feedback to the TUI.
- Added mouse and keyboard transcript selection, continuous edge scrolling, link opening, clipboard feedback, and a shared copy experience across the main view and pager.
- Completed raw scrollback, compact reasoning/activity presentation, paginated history loading, and cross-page view-state preservation.
- Added a runtime keymap owner that keeps default shortcut semantics consistent across the transcript, pager, composer, and footer.

### Fixes

- Fixed narrow-terminal geometry and selection boundaries for CJK, emoji, long URLs, Markdown links, ANSI/OSC 8 hyperlinks, and diffs.
- Fixed stale history cursors, retry surfaces, older-page insertion, completion separators, sticky-header anchoring, and main-view scroll restoration.
- Fixed state leaks across streaming terminal states, reasoning activity, MCP/approval/user-input flows, composer mouse editing, and terminal restoration.
- Fixed narrow-screen overflow in TUI footers, pickers, overlays, textareas, resume transcripts, and user-input prompts.

### Improvements and Refactoring

- Split TUI interaction, transcript view, history-cell, pager, keymap, selection, footer, and activity presentation into Codex-shaped current owners while preserving the App Server JSON-RPC and canonical Thread/Turn/Item projection chain.
- Consolidated transcript search, copy, selection, reading position, raw/rich mode, and follow control as session-local presentation state without a second history store or runtime.
- Clarified the Desktop versus CLI/TUI product boundary, platform payloads, installation, and shared App Server chain in the CLI/TUI documentation.

### Testing and Quality

- Expanded TUI library and all-targets regression coverage to `1052/1052`, with integration `18/18` and manager regression `1/1`.
- Passed TUI `--all-targets --no-deps` Clippy `-D warnings`, workspace fmt, locked Cargo metadata, structure/snapshot/Gate guards `22/22`, script governance, and `git diff --check`.
- Real TUI Gate B covers complete, approval, user-input, interrupt, failure, queue-edit, agents-overview, F3 Find, focus palette, resize/reflow, reconnect, and terminal restoration.
- Retained real App Server JSON-RPC, RuntimeCore, PTY, alternate-screen, canonical read-model, and terminal-visible evidence; changes that did not touch Electron/App Server protocol do not claim GUI Gate B.

### Documentation

- Synchronized the Chinese and English README documentation for Desktop and CLI/TUI entry points, installation, and platform support.
- Updated the TUI/CLI Codex-alignment execution plan and structure inventory records.

### Other

- Kept the App Server configuration contract as the single source of truth for TUI keymap settings; no TUI-private config file, environment-variable configuration surface, or compat dual track was added.
- CLI cross-platform artifacts continue to build and publish through the existing GitHub Actions release chain.

**Full changes**: `v1.145.0` -> `v1.146.0`
