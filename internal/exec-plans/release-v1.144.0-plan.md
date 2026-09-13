# Lime v1.144.0 发布执行计划

状态：准备发布
日期：2026-09-13
目标：将当前 CLI/TUI Codex 对齐候选发布为 `v1.144.0`，由 GitHub Actions 完成跨平台构建、GitHub Release 和 npm Trusted Publishing。

## Release Candidate

- `release metadata`：`package.json`、`packages/cli/package.json`、`lime-rs/Cargo.toml`、`lime-rs/Cargo.lock`、`RELEASE_NOTES.md`、`RELEASE_NOTES.en.md`、本计划和 `internal/exec-plans/README.md`。
- `candidate changes`：当前工作树已有的 CLI/TUI 源码、测试、inventory、脚本和文档改动，以及本轮版本同步。
- `excluded changes`：未跟踪本机二进制 `rust_out`、旧的 `internal/exec-plans/release-v1.142.1-plan.md`；两者保持原样，不删除。

## 发布事实源

版本统一为 `1.144.0`，目标 tag 为 `v1.144.0`。GitHub Actions 的 `publish_cli_assets` 在 macOS arm64/x64、Windows x64、Linux x64 构建并打包 `lime`、`app-server`、`code-mode-host` 及平台动态库；`publish_cli_npm` 按平台包优先、根包最后顺序发布，并使用 npm OIDC Trusted Publishing。

## 主链与风险

本候选覆盖 `CLI/TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical Thread/Turn/Item -> terminal projection`。风险集中在 Rust/TUI 构建、PTY/stdio Gate B、跨平台 npm 载荷、版本一致性和 GitHub Actions 发布权限。

## 已执行验证

- [x] `cargo build --manifest-path lime-rs/Cargo.toml -p cli`。
- [x] `cargo test --manifest-path lime-rs/Cargo.toml -p cli --all-targets`：52/52。
- [x] `cargo test --manifest-path lime-rs/Cargo.toml -p tui --all-targets`：749 单元测试 + 15 集成测试 + 1 PTY 测试（总计 765 个通过测试目标；另有 0 测试目标）。
- [x] `npm --prefix packages/cli test`：9/9。
- [x] `npm run verify:app-version`、`npm run typecheck`、`npm run test:contracts`。
- [x] `smoke:cli-gate-b`、`smoke:cli-surface-gate-b`。
- [x] `smoke:cli-npm-gate-b`：macOS arm64 launcher、CLI Gate B、TUI Gate B。
- [x] `cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check`、`git diff --check`。
- [x] App Server/其他相关 crate 全目标 Rust 测试：使用本机 `rusty_v8` artifact resolver 完成，App Server `--all-targets -- --list` 列出 1939 个测试，执行全部通过；其中 `agent-runtime` 211、`app-server` 库 1781、`app-server` 主程序 27、`execpolicy` 34、`tool-runtime` 379，以及全部 App Server 集成测试目标均通过；远程 Environment cold-resume 回归已通过。

## GitHub 与 npm 发布

不直接使用聊天中暴露的 npm token。完成危险操作确认后，创建 `Release v1.144.0` commit、`v1.144.0` tag 并推送 `origin/main` 与 tag；随后由 GitHub Actions 完成四平台 CLI 构建、测试、npm tarball staging、provenance 和 npm 发布。

## 交付门槛

只有 GitHub Actions 的跨平台矩阵、Release 资产完整性、npm Trusted Publishing 和远端 tag 复核均成功后，才标记为已发布。Windows 真机、签名/公证、完整 Forge 产物和非 macOS npm 包属于平台 runner 证据。
