# Lime v1.142.0 发布执行计划

状态：进行中
日期：2026-09-08
目标版本：`1.142.0`
目标 tag：`v1.142.0`

## 主目标

发布 v1.141.0 之后当前工作树中的 TUI/CLI Codex 对齐、App Server 执行进程与有界输出、
认证 WebSocket transport foundation、Electron 安全凭证存储、Cloud App Server 配置解析、
DeepSWE harness 环境注入及配套治理/测试改动；完成版本事实源、双语单页 release notes、
质量门禁、release commit、tag、main 推送和远端复核。

## Release Candidate

- `release metadata`：`package.json`、`packages/cli/package.json`、`lime-rs/Cargo.toml`、
  `lime-rs/Cargo.lock`、`RELEASE_NOTES.md`、`RELEASE_NOTES.en.md`、本计划及
  `internal/exec-plans/README.md`。
- `candidate changes`：当前工作树全部已跟踪的产品、文档、测试、schema、依赖、脚本、
  Electron、App Server、Runtime、tool-runtime、CLI/TUI 与执行计划改动，以及除本地二进制外
  全部未跟踪源码、测试、脚本和文档文件。
- `excluded changes`：未跟踪的本机 Mach-O 构建产物 `rust_out`；不删除、不覆盖，保留在工作树。
  `.lime/`、`dist/`、`dist-electron/` 和 `lime-rs/target/` 为被忽略的本地生成目录，不进入提交。

## 架构确认

本轮仍遵循唯一业务主链：

```text
Electron Desktop Host / CLI-TUI Host
  -> App Server JSON-RPC
  -> RuntimeCore
  -> Thread / Turn / Item projection
```

Remote WebSocket 仅实现同一 App Server session 的认证 transport foundation；Electron 凭证
存储只由 Desktop Host `safeStorage` owner 承担，不能读取 token 或创建第二套 runtime、会话状态、
工具 registry 或持久化。执行进程输出继续由 `tool-runtime` 统一拥有，App Server 只负责生命周期与
canonical projection。相关架构边界已在 `internal/aiprompts/architecture.md` 中同步；本轮没有
新增平行业务后端。

## 退出条件

- 根应用、CLI npm 包、Rust workspace 与 Cargo.lock 统一为 `1.142.0`；双语 release notes
  只保留 v1.142.0；目标 tag 在写操作前不存在。
- `npm run verify:app-version`、`npm run typecheck`、`npm run test:contracts`、受影响包/Rust
  定向测试、CLI/TUI Gate B、Agent Runtime current fixture、治理扫描和 `git diff --check` 通过。
- `npm run verify:gui-smoke` 若环境允许则通过；无法执行时在收尾报告说明限制。
- staged 内容覆盖全部 candidate changes 与 release metadata，仅排除 `rust_out` 及被忽略生成目录；
  完成 `Release v1.142.0` commit、`v1.142.0` tag，并推送 `origin/main` 与远端 tag 后复核。

## 验证记录

已完成：

- `npm run verify:app-version`：通过（改版本前基线 `1.141.0`）。
- `npm run typecheck`：首次被新增 remote `ws` 声明未纳入根级编译阻断；补充
  `packages/app-server-client/tsconfig.json` 的 `.d.ts` include 与 `remote.ts` reference 后通过。
- `npm run test:contracts`：通过（协议类型、App Server client、命令、harness、modality、脚本、
  Electron release、CLI、docs boundary）。
- `npm --prefix "packages/app-server-client" test`：11 个测试文件、139 项通过。
- `cargo test --manifest-path "lime-rs/Cargo.toml" -p tui`：390/390 通过。
- `npm run smoke:cli-gate-b`、`npm run smoke:tui-gate-b`：真实 CLI/TUI、stdio App Server、
  canonical projection、队列编辑与终端恢复通过。
- `npm run smoke:agent-runtime-current-fixture`：通过，真实 Electron/preload/IPC/App Server/
  Runtime/read model、Claw、Coding Workbench、Skills、MCP、媒体、审批、恢复等 current fixture
  全部通过，`liveProviderUsed=false`。
- `npm run verify:gui-smoke`：通过；真实 Electron/preload/IPC、App Server `appserver.v0`、Claw
  workbench、响应式布局与 memory settings smoke 通过，app-server 版本为 `1.142.0`。
- `npm run governance:legacy-report`：通过；结构/脚本/legacy 边界无违规。
- `npx vitest run scripts/app-server/cli-gate-b.test.mjs scripts/app-server/tui-gate-b.test.mjs
  scripts/app-server/tui-structure-inventory.test.mjs scripts/app-server/terminal-gate-binaries.test.mjs
  scripts/harness/deepswe-adapter.test.mjs`：5 个测试文件、46 项通过。
- `git diff --check`：通过。

环境限制：直接运行 `cargo test` 触发 `rusty_v8 v150.4.0` Darwin/aarch64 预构建 archive 404，
导致包含 V8 的 app-server/cli/lime-agent workspace 编译未完成；仓库 runner 已用于此前 current
fixture 和可运行的定向测试，不能把该本机 archive 失败误报为产品测试通过。Windows restricted
execution 真机矩阵、签名/公证和完整 Forge 产物由对应平台 runner 提供，本轮未在本机执行。

## 收尾分类

- `current`：TUI/CLI Codex-shaped owner、App Server execution process 与 command event mirror、
  bounded output、authenticated remote WebSocket foundation、Electron secure credential store、
  Cloud App Server parser、current fixture 与治理守卫。
- `compat`：无新增；既有显式兼容导出不扩展。
- `deprecated`：无新增。
- `dead / deleted`：旧全局输出 FIFO、tail-only capture、未校验 sidecar data-dir、token URL/query
  传递、旧聚合 TUI 状态和禁止恢复的平行 runtime 入口。

当前完成度：版本文件与候选整理 95%；待用户确认后执行 release commit、tag、推送和远端复核。
