## Lime v1.142.0

Simplified Chinese release notes are the primary version.

### New Features

- Continued Codex-shaped current owners across TUI/CLI: multi-agent navigation and overview, model catalog, collaboration modes, paginated resume sessions and transcript pager, composer textarea, history search, viewport handling, and five-language terminal copy.
- Added unified App Server execution-process lifecycle and command-event mirroring for canonical Thread/Turn/Item command-start, live-output, completion, and test-run projection.
- Added an authenticated WebSocket foundation to `@limecloud/app-server-client` for the same JSON-RPC session, with `ws/wss`, Bearer token, tenant headers, identity/protocol handshake, ping/pong, and message limits.
- Added Electron `safeStorage`-backed Cloud session credential storage with set/status/delete IPC; status exposes metadata only and never reads back tokens.

### Fixes

- Unified bounded output capture across pipe, PTY, Windows restricted runner, remote Environment, and unified exec with stable head/tail retention, UTF-8 boundaries, and omission accounting.
- Fixed App Server process-owner connection isolation, duplicate active `processId` rejection, disconnect cleanup, stdin/resize/terminate lifecycle, and per-process output replay.
- Fixed TUI terminal restoration, disconnected editing, Unicode cursor behavior, narrow-terminal wrapping, queue recovery, history hydration, approval/input requests, and multi-agent projection boundaries.
- Hardened sidecar `dataDir` and Cloud App Server endpoint validation against `undefined`/relative paths, URL credentials, query/fragment leakage, and insecure public WebSocket token connections.

### Improvements and Refactoring

- Moved TUI aggregate state into unique `app/`, `chat_composer/`, `textarea/`, and `viewport` owners while retaining the App Server canonical projection as the sole session source of truth.
- Reworked App Server and tool-runtime execution output around per-process bounded queues and a shared framer, removing global FIFO, tail-only capture, and lossy per-chunk UTF-8 semantics.
- Added isolated Docker CLI configuration and active Docker-context endpoint injection to the DeepSWE harness.
- Kept Cloud transport foundation-only: credentials, tenant isolation, reconnect, rate limits, audit, and a production endpoint remain Cloud-service prerequisites.

### Testing and Quality

- `tui` Rust unit tests: 390/390 passed; `@limecloud/app-server-client`: 11 test files and 139 tests passed.
- CLI Gate B, TUI Gate B, Agent Runtime current Electron fixtures, App Server client contracts, structure inventory, harness, legacy governance, and script-boundary checks passed.
- Preserved five-language copy, generated protocol, Electron Forge, version-consistency, real PTY/stdio, and canonical Thread/Turn/Item identity gates.

### Documentation

- Updated architecture, command, quality, roadmap, and TUI/CLI execution-plan documentation for execution processes, remote transport, credential storage, and the single Product Surface -> App Server JSON-RPC -> RuntimeCore path.

### Other

- Bumped the root app, CLI npm package, Rust workspace, and Cargo.lock versions to `1.142.0`.
- The local `rusty_v8 v150.4.0` Darwin/aarch64 prebuilt archive returned 404, so the V8-dependent workspace full build was not completed locally; Windows hardware matrix, signing/notarization, and full Forge artifacts remain platform-runner evidence.
- Excluded the untracked local Mach-O `rust_out` and ignored local build directories from the release candidate; the existing v1.141.0 tag was not deleted or overwritten.

**Full changes**: `v1.141.0` -> `v1.142.0`
