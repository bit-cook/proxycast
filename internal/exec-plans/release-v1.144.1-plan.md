# Lime v1.144.1 发布执行计划

状态：准备发布
日期：2026-09-14
目标：修复 `v1.144.0` Windows CLI npm 构建失败，并在不移动既有 `v1.144.0` tag 的前提下发布 `v1.144.1`。

## Release Candidate

- `release metadata`：`package.json`、`packages/cli/package.json`、`lime-rs/Cargo.toml`、`lime-rs/Cargo.lock`、`RELEASE_NOTES.md`、`RELEASE_NOTES.en.md`。
- `candidate changes`：`lime-rs/crates/tui/Cargo.toml` 的 Windows `windows-sys` feature 修复。
- `excluded changes`：工作树中并发进行的 TUI/A2 改动和未跟踪的旧 `release-v1.142.1-plan.md`，均保持原样。

## 根因与修复

Windows CLI 构建引用 `windows_sys::Win32::Storage::FileSystem::WriteFile`，但 `windows-sys 0.52` 还要求启用 `Win32_System_IO` feature。候选只增加该 feature，不改变运行时协议或 CLI/TUI 主链。

## 验证与退出条件

- [x] `cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check`
- [x] `cargo test --manifest-path lime-rs/Cargo.toml -p tui`
- [ ] `npm run verify:app-version`
- [ ] `npm run typecheck`
- [ ] CLI npm 定向测试与 Gate B
- [ ] GitHub Actions 四平台 CLI 构建、Release 资产和 npm Trusted Publishing
- [ ] 远端 `main`、`v1.144.1` tag、GitHub Release 与 npm registry 复核

## 发布约束

不使用聊天中暴露的 npm token；GitHub Actions 使用 npm OIDC Trusted Publishing。不得覆盖或移动 `v1.144.0` tag。
