## Lime v1.143.0

Simplified Chinese release notes are the primary version.

### New Features

- Added Codex-shaped `history_cell`, `exec_cell`, `render`, history replay, and ANSI terminal owners for TUI transcript, command output, diff rendering, and wide-character handling.
- Added canonical Thread/Turn/Item history pagination and metadata-only resume cursors. TUI now loads older history through App Server `thread/items/list` and provides transcript pager and `/export` support.
- Expanded TUI startup, reconnect, resize/reflow, terminal palette/focus, and session recovery paths with real PTY/VT100 Gate B scenarios and stable integration suites.
- Aligned the CLI working-directory flags with Codex using `--cd` and `-C`, and extended CLI/TUI structure inventory and current-fixture guards.

### Fixes

- Fixed canonical command completion projection so canceled commands consistently report `canceled`, with regression coverage for raw lifecycle and redacted interaction events.
- Fixed TUI history hydration, disconnected editing, terminal restoration, Unicode cursor handling, narrow-terminal wrapping, queue recovery, and multi-agent projection boundaries.
- Fixed Windows CI CLI npm packaging by invoking `npm.cmd` through the Windows command interpreter, and added ALSA development dependencies required by the Linux CLI runner.
- Corrected the Electron Cloud credential test to ensure renderer-visible metadata never contains the session secret.

### Improvements and Refactoring

- Split TUI aggregate logic into unique `app/`, `history_cell/`, `exec_cell/`, `render/`, and `terminal_probe/` owners while keeping the App Server canonical projection as the sole session source of truth.
- Expanded script governance, structure inventory, CLI/TUI Gate B, and five-language terminal regression coverage while preserving real stdio/PTY validation.

### Testing and Quality

- Version consistency, TypeScript typecheck, protocol contracts, script governance, Rust related (App Server 1780/1780, TUI 510/510), CLI Gate B, TUI Gate B, and the real Electron GUI smoke all passed.
- TUI Gate B covered default turn, approval, user input, interrupt, failure, queue editing, multi-agent, focus/palette, resize/reflow, reconnect, and terminal restoration; the four real PTY resize/reflow cases passed 80/80 across 20 consecutive rounds.
- Windows hardware, signing/notarization, and complete Forge artifacts remain platform-runner evidence; any local Rust behavior-test limitation from the `rusty_v8` Darwin/aarch64 prebuilt archive will be recorded at closeout.

### Documentation

- Updated architecture, quality, CLI/TUI execution plans, and structure inventory to document paginated resume cursors, history pagination, and the current TUI Gate B boundary.

### Other

- Bumped the root app, CLI npm package, Rust workspace, and Cargo.lock versions to `1.143.0`.
- This candidate includes all currently modified and untracked product, documentation, test, and script changes; the untracked local binary `rust_out` and the old `internal/exec-plans/release-v1.142.1-plan.md` are excluded and untouched.

**Full changes**: `v1.142.0` -> `v1.143.0`
