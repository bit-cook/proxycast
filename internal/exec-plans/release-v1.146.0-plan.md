# Lime v1.146.0 发布执行计划

状态：准备发布
日期：2026-09-26
目标：将当前 TUI/CLI Codex 对齐候选发布为 `v1.146.0`，同步版本事实源、双语 release notes，并在门禁通过且获得明确确认后完成 commit、tag 和推送。

## Release Candidate

- `release metadata`：`package.json`、`packages/cli/package.json`、`lime-rs/Cargo.toml`、`lime-rs/Cargo.lock`、`RELEASE_NOTES.md`、`RELEASE_NOTES.en.md`、本执行计划。
- `candidate changes`：当前工作树中 88 个已跟踪改动和 30 个候选未跟踪文件，范围为 TUI/CLI Codex 对齐、配置合同、历史分页、transcript presentation、interaction/keymap、Gate B fixture/test、结构 inventory 与 Desktop/CLI/TUI 文档；另有 1 个未跟踪执行计划明确排除。
- `excluded changes`：`internal/exec-plans/release-v1.142.1-plan.md`。该文件在上一轮并发 TUI 工作中已被明确排除；本轮不修改或删除。

## 版本与说明

- 目标版本：`1.146.0`；目标 tag：`v1.146.0`。
- Release notes 采用当前版本单页策略，只保留 `v1.146.0` 内容。
- 版本事实源必须通过 `npm run verify:app-version`；Cargo lock 中所有 Lime workspace 包版本必须与 workspace 版本一致。

## 验证与退出条件

- [x] `git diff --check`
- [x] 版本事实源和双语 release notes 已更新
- [x] `npm run verify:app-version`
- [x] `npm run typecheck`
- [x] TUI `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui --lib`：`1057/1057`
- [x] lime-core 配置相关测试：`149/149`
- [x] App Server `config_jsonrpc`：`1/1`（使用仓库校验的 rusty-v8 缓存资产）
- [x] `npm run test:contracts`：299 checks 与全部边界治理通过
- [x] `npm run verify:gui-smoke`：真实 Electron smoke 通过，App Server version `1.146.0`
- [ ] `npm run test:rust:related -- lime-rs/crates/tui lime-rs/crates/core lime-rs/crates/app-server`：初次 related 扩展集在 `lime_mcp` 单测无输出挂起，已终止自启动进程；由上述窄集替代验证
- [ ] `npm run governance:file-size`：仓库既有 170 处体量基线违规，未在发版候选中临时拆分
- [ ] GitHub Actions、Release 资产和 npm registry 复核
- [ ] 获得 git 写操作确认后创建 commit、tag，推送 `main` 和 `v1.146.0`，并复核远端引用

## 当前限制

直接 Cargo 构建 App Server 时，本机 Darwin/aarch64 的 `rusty_v8 v150.4.0` 官方 Deno archive 返回 HTTP 404；使用仓库 `scripts/lib/rusty-v8-artifacts.mjs` 解析出的本地已校验 archive/binding 后，App Server 配置测试和 Electron smoke 均通过。全量 related 入口另因 `lime_mcp` 单测长时间无输出而中止，不将其记为通过。

## 架构确认

本候选保持唯一业务主链：`CLI/TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical Thread/Turn/Item -> terminal projection`。未新增 runtime、第二套 history store、平行协议或 Electron/GUI 边界；不需要修改 `internal/aiprompts/architecture.md`。
