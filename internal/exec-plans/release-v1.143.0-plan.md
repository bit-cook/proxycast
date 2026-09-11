# Lime v1.143.0 发布执行计划

状态：verified-awaiting-publish
日期：2026-09-11
目标：将当前已确认的 TUI/CLI、App Server、脚本、文档和测试候选发布为 `v1.143.0`。

## Release Candidate

纳入：当前工作树全部已修改文件，以及未跟踪的 TUI/CLI 源码、测试、`ansi-escape` crate 和脚本。

排除：未跟踪本机二进制 `rust_out`、旧的 `internal/exec-plans/release-v1.142.1-plan.md`。两者保持原样，不删除。

## 发版事实源

- `package.json`
- `packages/cli/package.json`
- `lime-rs/Cargo.toml`
- `lime-rs/Cargo.lock`
- `RELEASE_NOTES.md`（中文 primary）
- `RELEASE_NOTES.en.md`（英文 companion）

目标版本：`1.143.0`；目标 tag：`v1.143.0`。

## 主链与风险

本候选覆盖 `CLI/TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical Thread/Turn/Item -> terminal projection`，并触及 Electron credential 测试、Windows/Linux CLI 发布 runner。风险集中在 Rust/TUI 构建、PTY/stdio Gate B、版本一致性和跨平台打包。

## 验证清单

- [x] `npm run verify:app-version`：通过（`1.143.0`）。
- [x] `npm run typecheck`：通过。
- [x] `npm run test:contracts`：通过。
- [x] `npm run governance:scripts`：通过。
- [x] `npm run test:rust:related -- lime-rs/crates/tui lime-rs/crates/cli lime-rs/crates/app-server`：通过；App Server `1780/1780`、TUI `510/510`。
- [x] `npm run verify:gui-smoke`：通过；真实 Electron/preload/App Server、3 个视口和版本 `1.143.0` 均通过。
- [x] `npm run smoke:agent-runtime-current-fixture`：通过。
- [x] `npm run smoke:cli-gate-b`：通过。
- [x] `npm run smoke:cli-surface-gate-b`：通过。
- [x] `npm --prefix packages/cli test`：通过（8/8）。
- [x] `npm run smoke:tui-gate-b`：通过；默认场景、queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和终端恢复均通过。
- [x] resize/reflow 专项重复验证：4 个真实 PTY 用例连续 20 轮，`80/80` 通过；修复长输入尾标记同步后未再出现退出超时。
- [x] `cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check`、`git diff --check`：通过。

平台 runner 证据：Windows 真机、签名/公证、完整 Forge 产物；本机 Rust 行为测试若被 `rusty_v8` Darwin/aarch64 预构建 archive 阻塞，需在收尾标明。

本轮真实 TUI Gate B 证据覆盖：`lime` CLI、stdio App Server、RuntimeCore/canonical Thread/Turn/Item、真实 PTY 键盘输入、窗口 resize、VT100 可见投影和 alternate-screen 恢复；测试 fixture 未使用生产 mock fallback。App Server 构建仍输出两个既有 `dead_code` warning（`lower_turn_start_params`、`lower_runtime_options`），不影响通过结论。

## Git 收口

验证通过后，先汇总 staged 文件与验证结果，再请求一次明确危险操作确认；确认后连续执行 `git add`、`git commit -m "Release v1.143.0"`、`git tag v1.143.0`、`git push origin main`、`git push origin v1.143.0`，并复核本地与远端 tag。

## 收尾记录

完成度：95%；代码、版本事实源、发布说明和本地验证已完成，剩余仅为经确认后的 Git commit、tag、main 与 tag 推送及远端复核。Windows 真机、签名/公证和完整 Forge 产物仍属于平台 runner 后续证据。
