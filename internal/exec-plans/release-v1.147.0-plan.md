# Lime v1.147.0 发布执行计划

状态：183 文件发布提交和 main/tag 推送完成；远端打包与分发进行中
日期：2026-09-30
目标：将全部当前未提交/未跟踪候选发布为 `v1.147.0`，完成 commit、tag、main/tag 推送与远端 Actions/Release/npm 复核。

## 范围与授权

- 起始基线：`a123afaf538005e5ac1d9ad9956ea4c262355ea8`；`main`、`origin/main` 与 `v1.146.0` 一致。
- 用户已回复“确认”，并进一步要求“继续”“全部都要递交”。commit、tag、推送授权有效；当前候选包含全部 tracked/untracked 文件。
- `release metadata`：根 npm/CLI package、Rust workspace/Cargo lock、双语单页 release notes、本计划与执行计划导航。
- `candidate changes`：TUI 任务中心、popup/composer/keymap/clipboard/history/status、CLI 权限与 Plugin、MCP OAuth logout 和登录关联、App Server protocol/schema/generated client、stdio/PTY fixtures、inventory、文档及 npm OIDC release workflow/guard。
- 用户“全部”要求覆盖此前排除的 `internal/exec-plans/release-v1.142.1-plan.md`，本次按历史记录纳入；它不替代本计划作为当前发版 owner。无人工排除的 tracked/untracked 文件。
- 本轮写集：上述 release metadata，以及 CLI surface Gate B 中过时 logout 断言与对应守卫、PTY suggestions 格式修复、popup 键盘路由中的输入缓冲同步，以及审批详情 PTY 的状态等待修复。已有产品源码修改由原写入进程收口，保留其修改。

## 版本事实源

- 版本 `1.147.0`；tag `v1.147.0`；前版 `v1.146.0`。
- 同步 `package.json`、`packages/cli/package.json`、`lime-rs/Cargo.toml` 和 Cargo lock 的 36 个 workspace 包版本；不升级依赖。
- Forge 从根 package 读取版本；pnpm lock 无独立发布版本；App Server 产物 manifest 由 current 打包链生成。
- 中文 primary、英文 companion release notes 均采用当前版本单页策略。

## 验证证据

- [x] `npm run verify:app-version`：`1.147.0`。
- [x] `npm run typecheck`：最终全部候选收口时重跑通过。
- [x] `npm run test:contracts`：299 checks 和全部边界/治理通过；后续新增切片仅在 TUI presentation/fixture，无协议/client 修改。
- [x] CLI all-targets：library `8/8`、binary `51/51`、integration `2/2`；protocol library `133/133`、schema fixture `1/1`。
- [x] 最终 TUI all-targets：library `1182/1182`、integration `18/18`、manager `1/1`。
- [x] lime-core `config::` `122/122`、lime-mcp `oauth::` `9/9`、App Server `mcp_oauth_` 公共 JSON-RPC `3/3`。
- [x] App Server client `139/139`、CLI npm `9/9`、发布/Gate/MCP fixture 定向 `72/72`；后续 TUI/CLI fixture 守卫按变更重跑。
- [x] 真实 CLI Gate B：thread `01a0f2a1-f625-7681-8c92-61a711f42c25`、turn `turn_d39fda17da364fa282d883a34e04aec5`，stdio/read model/JSONL/stdin/error/completion 通过。
- [x] 真实 CLI surface Gate B：MCP、features、Plugin、debug、execpolicy、queue、sandbox；OAuth logout 覆盖未知服务、stdio 拒绝与两次无凭据幂等。
- [x] 真实 Electron `verify:gui-smoke`：App Server `1.147.0`，壳加载/重载、三种窗口尺寸、设置页通过；summary `standalone-shell-01-20260930135938-87372/shell-01-electron-smoke/summary.json`。
- [x] 后续 TUI PTY Gate B：thread `01a0f2c1-621c-78e0-8883-cb17c2bf6e39`、turn `turn_df8c6a55eb1047449c6dc1919b2211b1`，canonical 事件链、queue-edit、agents-overview、sticky-prompt、main-find、focus-palette、resize/reflow、reconnect、terminal restore 通过。
- [x] 新增 popup suggestions 的真实 PTY complete 场景：thread `01a0f2da-fdb0-7ab1-a385-7fc2f18b5310`、turn `turn_119fe14e70834002a85dca8b9e96a823`。
- [x] 新增审批只读详情 PTY approval 场景：thread `01a0f2de-501a-7a00-ae59-306a1d6cc148`；打开/关闭详情不提前决定 protected request。
- [x] popup 输入缓冲修复回归 `2/2`；最终 TUI/CLI fixture 守卫 `25/25`。
- [x] 最终完整 TUI PTY Gate B：thread `01a0f2df-7413-7be1-9157-fcb9b2aa8e7d`、turn `turn_5f90cbe039074ccc862a38d09dee5f9a`；complete/approval/user-input/interrupt/failure/queue-edit/agents-overview、focus-palette/resize-reflow/reconnect 和 terminal restore 通过。
- [x] workspace fmt 与 `git diff --check`；新增 PTY suggestions 文件格式已修复。
- [x] git 写操作和全部候选递交授权。
- [x] `Release v1.147.0` commit `1cbc94e5f7f00b58f0580fafab1b202fdf4e9d0c`，183 文件；轻量 tag `v1.147.0` 与 main 已分别推送，远端两引用一致；提交 hook `182` 通过、`0` 失败。
- [x] Release run `36735313925` 已启动，GitHub draft 已创建，三个 Electron 平台构建中；Docs run `36735269856` 成功。
- [ ] GitHub Release 公开资产、updater 和 npm registry 最终复核。
- [ ] Quality run `36735269701`：Frontend Full 在批次 `56/119` 因既有 fixture 未登记失败，Rust/GUI/Windows job 尚在运行，不能声明全量 CI 通过。

## 修复与限制

初次并发 TUI 编译遇到尚未落盘模块，后续 popup 生命周期与两项 selection geometry/style 回归曾失败；原写入进程收口后，最终 `1182/1182` 全目标回归通过，旧失败结果不冒充通过。CLI surface fixture 原先仍期待 logout 协议缺失，本轮迁移到已实现 current 合同并以真实 CLI 复跑通过。无凭据 logout 不冒充 live OAuth 凭据删除证据。

真实 PTY 发现 `/status` 最后一个字符仍留在 paste-burst 缓冲中，弹窗提前消费 Enter 后该字符重新写回草稿。本轮在 popup 路由前同步 due input，并让活跃 paste 内的 Enter/Tab 保持草稿文本；补 owner 回归且真实 complete 通过。审批详情 PTY 原先只等待页面间共用提示，修为同时观察审批选项恢复及详情标题消失，不增加固定等待或跳过断言。

Rust 构建使用仓库 rusty-v8 artifact resolver 的已校验缓存，未修改系统环境变量或依赖。App Server 保留既有两处 dead-code warning；本轮不宣称全量 lint/Vitest/Cargo/Clippy 矩阵通过。跨平台 Forge、Windows 真机、签名/公证与 npm 分发由 GitHub runner 验证。

## 发布后的定向修复

远端 Frontend Full 的唯一已知失败为 `scripts/app-server/tui-history-pagination-fixture.mjs` 未登记在 ExternalBackend 测试夹具允许清单。该 fixture、测试与 support 在 `v1.146.0..v1.147.0` diff 为空，是既有守卫漏登记。检查确认 fixture 使用隔离临时 app data、受控 backend 和真实 CLI/App Server，不属于生产默认入口。

本轮补 `src/lib/governance/appServerRuntimeBoundary.testSupport.ts` 的精确允许路径，保留全目录扫描与生产 Runtime 默认断言；失败守卫定向 `2/2` 和 `npm run typecheck` 通过。作为 main 的独立测试修复提交，保持发布 tag 原提交，不宣称旧 tag 的 Quality run 已通过。推送后并发会话开始的下一项 request-user-input 改造属于发布后工作，保留其工作树，不覆盖、不并入本次标签。

第一项测试守卫修复提交 `faa8be408f3d78a23bcb35b91f30afb8e737c26e` 已推送 main，Quality run `36737461083`。该 run 的 Frontend Full 又在 batch `49/119` 发现既有 Coding roadmap 文案守卫漂移：README 已将下一刀改为 current-SHA Windows large-output evidence，旧断言仍要求先前的 unelevated runner 原文。本轮让 README 和 implementation 的断言分别守住当前证据缺口与既有 unelevated runner，不改路线图事实。

Rust Full 失败是既有 `paginated_history_jsonrpc_preserves_canonical_thread_turn_item_identity` 仍给出非规范 item ID `answer-item` 并要求原样返回；实际返回 `item_answer-item`，正文与 turn identity 均正确。迁移 fixture 输入和断言到规范 `item_answer`，保持公共 JSON-RPC 的 identity/text 检查，并补实际响应诊断；整个 `thread_v2_jsonrpc` target `19/19` 通过，没有 production/runtime 修改。

扩大治理目录验证：`47` files 通过、`440` tests 通过；唯一失败是本机外部 Codex checkout 的可选 `codexModelResponsesPolicyOrigin` 源码形状已经变化，不属于本次 Lime/tag 改动，保留失败记录、不修改外部仓库或跳过断言。两个 CI 失败守卫已通过。最后 `npm run typecheck`、owned Rust fmt 与 diff check 通过。

## 架构与分类

主链保持 `Desktop Host / CLI-TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical Thread/Turn/Item -> GUI/terminal projection`。MCP OAuth 扩展仍归既有 protocol/App Server/MCP credential owner；新增 TUI helper 是 presentation 内部分工，没有平行 runtime/history store 或新的 public boundary。本次无重大架构变更。

- current：CLI/TUI、MCP OAuth、protocol/schema/client 与 Forge/GitHub/npm 发布链。
- compat / deprecated：本轮无新增。
- dead / deleted：不恢复已退役 runtime/发布路径，本轮不删除文件。
- 历史记录：v1.142.1 发布计划按用户“全部”要求纳入，不升级为 current。

版本同步和窄 fixture 修复保持 KISS/DRY；不引入兼容包装或新依赖。

## 候选提交文件清单

全部 `183` 个文件，release metadata `8` 个，candidate changes `175` 个。

### release metadata

- `RELEASE_NOTES.en.md`
- `RELEASE_NOTES.md`
- `internal/exec-plans/README.md`
- `internal/exec-plans/release-v1.147.0-plan.md`
- `lime-rs/Cargo.lock`
- `lime-rs/Cargo.toml`
- `package.json`
- `packages/cli/package.json`

### candidate changes

- `.github/workflows/release.yml`
- `docs/ops.md`
- `internal/aiprompts/commands.md`
- `internal/exec-plans/release-v1.142.1-plan.md`
- `internal/exec-plans/tui-cli-codex-sync-next-plan.md`
- `internal/exec-plans/tui-structure-inventory.json`
- `lime-rs/crates/app-server-protocol/schema/json/app_server_protocol.schemas.json`
- `lime-rs/crates/app-server-protocol/schema/json/manifest.json`
- `lime-rs/crates/app-server-protocol/schema/json/v0/AppServerClientRequest.json`
- `lime-rs/crates/app-server-protocol/schema/json/v0/AppServerRequestMethod.json`
- `lime-rs/crates/app-server-protocol/schema/json/v0/McpServerOauthLoginParams.json`
- `lime-rs/crates/app-server-protocol/schema/json/v0/McpServerOauthLoginResponse.json`
- `lime-rs/crates/app-server-protocol/schema/json/v0/McpServerOauthLogoutParams.json`
- `lime-rs/crates/app-server-protocol/schema/json/v0/McpServerOauthLogoutResponse.json`
- `lime-rs/crates/app-server-protocol/schema/json/v2/McpServerOauthLoginCompletedNotification.json`
- `lime-rs/crates/app-server-protocol/schema/json/v2/ServerNotification.json`
- `lime-rs/crates/app-server-protocol/src/protocol/v0/catalog.rs`
- `lime-rs/crates/app-server-protocol/src/protocol/v0/client_request.rs`
- `lime-rs/crates/app-server-protocol/src/protocol/v0/mcp.rs`
- `lime-rs/crates/app-server-protocol/src/protocol/v0/method_names.rs`
- `lime-rs/crates/app-server-protocol/src/protocol/v0/schema_types.rs`
- `lime-rs/crates/app-server-protocol/src/protocol/v0/tests/catalog.rs`
- `lime-rs/crates/app-server-protocol/src/protocol/v2/mcp.rs`
- `lime-rs/crates/app-server-protocol/src/schema_export/registry.rs`
- `lime-rs/crates/app-server/src/local_data_source/impls/mcp.rs`
- `lime-rs/crates/app-server/src/local_data_source/mcp.rs`
- `lime-rs/crates/app-server/src/processor/dispatch.rs`
- `lime-rs/crates/app-server/src/processor/mcp.rs`
- `lime-rs/crates/app-server/src/processor/tests/mcp.rs`
- `lime-rs/crates/app-server/src/runtime/app_data/mcp.rs`
- `lime-rs/crates/app-server/src/runtime/mcp.rs`
- `lime-rs/crates/cli/src/main.rs`
- `lime-rs/crates/cli/src/mcp_cmd.rs`
- `lime-rs/crates/cli/src/plugin_cmd.rs`
- `lime-rs/crates/core/src/config/mod.rs`
- `lime-rs/crates/core/src/config/tui_keymap.rs`
- `lime-rs/crates/mcp/src/manager.rs`
- `lime-rs/crates/mcp/src/oauth.rs`
- `lime-rs/crates/mcp/src/oauth_tests.rs`
- `lime-rs/crates/tui/src/app.rs`
- `lime-rs/crates/tui/src/app/agent_center/hints.rs`
- `lime-rs/crates/tui/src/app/agent_center/input.rs`
- `lime-rs/crates/tui/src/app/agent_center/mod.rs`
- `lime-rs/crates/tui/src/app/agent_center/navigation.rs`
- `lime-rs/crates/tui/src/app/agent_center/render.rs`
- `lime-rs/crates/tui/src/app/agent_center/rows.rs`
- `lime-rs/crates/tui/src/app/agent_center_tests.rs`
- `lime-rs/crates/tui/src/app/agents_overview.rs`
- `lime-rs/crates/tui/src/app/agents_overview_grouping.rs`
- `lime-rs/crates/tui/src/app/agents_overview_render.rs`
- `lime-rs/crates/tui/src/app/agents_overview_tests.rs`
- `lime-rs/crates/tui/src/app/agents_overview_threads.rs`
- `lime-rs/crates/tui/src/app/agents_overview_view.rs`
- `lime-rs/crates/tui/src/app/app_server_events.rs`
- `lime-rs/crates/tui/src/app/event_dispatch.rs`
- `lime-rs/crates/tui/src/app/history_ui.rs`
- `lime-rs/crates/tui/src/app/history_ui_tests.rs`
- `lime-rs/crates/tui/src/app/input_flow.rs`
- `lime-rs/crates/tui/src/app/input_submission.rs`
- `lime-rs/crates/tui/src/app/interaction.rs`
- `lime-rs/crates/tui/src/app/mcp_login.rs`
- `lime-rs/crates/tui/src/app/reconnect.rs`
- `lime-rs/crates/tui/src/app/right_click_paste.rs`
- `lime-rs/crates/tui/src/app/startup.rs`
- `lime-rs/crates/tui/src/app/tests.rs`
- `lime-rs/crates/tui/src/app/thread_events.rs`
- `lime-rs/crates/tui/src/app/thread_settings.rs`
- `lime-rs/crates/tui/src/app/transcript_presentation.rs`
- `lime-rs/crates/tui/src/app/transcript_presentation_tests.rs`
- `lime-rs/crates/tui/src/app_server_session.rs`
- `lime-rs/crates/tui/src/bottom_pane/approval_render.rs`
- `lime-rs/crates/tui/src/bottom_pane/approval_render_tests.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/agents_navigation.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/draft_state.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/file_search_popup.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/footer_state.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/footer_state_tests.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/layout.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/mouse.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/paste_input.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/skill_popup.rs`
- `lime-rs/crates/tui/src/bottom_pane/chat_composer/vim_search.rs`
- `lime-rs/crates/tui/src/bottom_pane/command_popup.rs`
- `lime-rs/crates/tui/src/bottom_pane/footer.rs`
- `lime-rs/crates/tui/src/bottom_pane/mod.rs`
- `lime-rs/crates/tui/src/bottom_pane/paste_burst.rs`
- `lime-rs/crates/tui/src/bottom_pane/picker_rows.rs`
- `lime-rs/crates/tui/src/bottom_pane/render.rs`
- `lime-rs/crates/tui/src/bottom_pane/scroll_state.rs`
- `lime-rs/crates/tui/src/bottom_pane/selection_popup_common.rs`
- `lime-rs/crates/tui/src/bottom_pane/selection_popup_common_tests.rs`
- `lime-rs/crates/tui/src/bottom_pane/selection_row_layout.rs`
- `lime-rs/crates/tui/src/bottom_pane/selection_tabs.rs`
- `lime-rs/crates/tui/src/bottom_pane/shortcut_overlay.rs`
- `lime-rs/crates/tui/src/bottom_pane/shortcut_overlay_tests.rs`
- `lime-rs/crates/tui/src/clipboard_copy.rs`
- `lime-rs/crates/tui/src/clipboard_copy/worker.rs`
- `lime-rs/crates/tui/src/clipboard_paste.rs`
- `lime-rs/crates/tui/src/clipboard_paste/worker.rs`
- `lime-rs/crates/tui/src/fuzzy_match.rs`
- `lime-rs/crates/tui/src/history_cell/activity_preview.rs`
- `lime-rs/crates/tui/src/history_cell/mod.rs`
- `lime-rs/crates/tui/src/history_cell/separators.rs`
- `lime-rs/crates/tui/src/history_cell/session.rs`
- `lime-rs/crates/tui/src/keymap.rs`
- `lime-rs/crates/tui/src/keymap/hints.rs`
- `lime-rs/crates/tui/src/keymap/tests.rs`
- `lime-rs/crates/tui/src/lib.rs`
- `lime-rs/crates/tui/src/local_settings.rs`
- `lime-rs/crates/tui/src/locale.rs`
- `lime-rs/crates/tui/src/locale/agents.rs`
- `lime-rs/crates/tui/src/locale/pickers.rs`
- `lime-rs/crates/tui/src/locale/shortcuts.rs`
- `lime-rs/crates/tui/src/markdown.rs`
- `lime-rs/crates/tui/src/markdown_render.rs`
- `lime-rs/crates/tui/src/model_picker.rs`
- `lime-rs/crates/tui/src/model_picker/render.rs`
- `lime-rs/crates/tui/src/model_picker/render_tests.rs`
- `lime-rs/crates/tui/src/pager_overlay.rs`
- `lime-rs/crates/tui/src/pager_overlay/disclosure_tests.rs`
- `lime-rs/crates/tui/src/projection.rs`
- `lime-rs/crates/tui/src/resume_picker.rs`
- `lime-rs/crates/tui/src/runtime.rs`
- `lime-rs/crates/tui/src/runtime_pty_tests.rs`
- `lime-rs/crates/tui/src/runtime_pty_tests/approval.rs`
- `lime-rs/crates/tui/src/runtime_pty_tests/suggestions.rs`
- `lime-rs/crates/tui/src/shortcut_help.rs`
- `lime-rs/crates/tui/src/status/format.rs`
- `lime-rs/crates/tui/src/status/mod.rs`
- `lime-rs/crates/tui/src/status_indicator_widget.rs`
- `lime-rs/crates/tui/src/status_indicator_widget/summary_shimmer.rs`
- `lime-rs/crates/tui/src/style.rs`
- `lime-rs/crates/tui/src/style/selection.rs`
- `lime-rs/crates/tui/src/terminal_palette.rs`
- `lime-rs/crates/tui/src/transcript_view.rs`
- `lime-rs/crates/tui/src/transcript_view/bookmark.rs`
- `lime-rs/crates/tui/src/transcript_view/bookmark_tests.rs`
- `lime-rs/crates/tui/src/transcript_view/disclosure.rs`
- `lime-rs/crates/tui/src/transcript_view/prompt_header.rs`
- `lime-rs/crates/tui/src/transcript_view/prompt_header_tests.rs`
- `lime-rs/crates/tui/src/transcript_view/selection.rs`
- `lime-rs/crates/tui/src/transcript_view/selection_tests.rs`
- `lime-rs/crates/tui/src/tui.rs`
- `lime-rs/crates/tui/src/view.rs`
- `lime-rs/crates/tui/src/view/tests.rs`
- `lime-rs/crates/tui/src/view/tests/composer.rs`
- `lime-rs/crates/tui/src/view/tests/interaction.rs`
- `lime-rs/crates/tui/src/view/tests/navigation.rs`
- `lime-rs/crates/tui/src/view/tests/presentation.rs`
- `lime-rs/crates/tui/src/view/tests/suggestions.rs`
- `lime-rs/crates/tui/tests/suite/reconnect.rs`
- `packages/app-server-client/src/connection-methods.ts`
- `packages/app-server-client/src/generated/protocol-types.ts`
- `packages/app-server-client/src/protocol.ts`
- `packages/app-server-client/src/request-client-methods.ts`
- `packages/app-server-client/src/request-client.ts`
- `packages/app-server-client/src/server-notifications.ts`
- `packages/app-server-client/tests/client.test.mjs`
- `packages/app-server-client/tests/direct-notifications.test.mjs`
- `packages/cli/README.md`
- `scripts/README.md`
- `scripts/app-server/cli-gate-b.mjs`
- `scripts/app-server/cli-surface-gate-b.mjs`
- `scripts/app-server/cli-surface-gate-b.test.mjs`
- `scripts/app-server/terminal-gate-fixture.mjs`
- `scripts/app-server/tui-gate-b.mjs`
- `scripts/app-server/tui-gate-b.test.mjs`
- `scripts/app-server/tui-structure-inventory.test.mjs`
- `scripts/electron/lib/release-workflow-candidate-guard.mjs`
- `scripts/electron/release-workflow-guard.test.mjs`
- `scripts/mcp/current-smoke.test.mjs`
- `scripts/mcp/lib/contract-guards.mjs`
- `scripts/mcp/lib/current-smoke-core.mjs`
- `scripts/mcp/oauth-fixture-smoke.mjs`
