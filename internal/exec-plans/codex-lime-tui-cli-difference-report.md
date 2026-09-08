# Codex 与 Lime TUI/CLI 全量差异报告

基线：Codex checkout `/Users/coso/Documents/dev/rust/codex`，Rust commit
`cac96cd7b1756ab42e8925d938817a2ac10ebb6e`。本报告只记录对照结果，不把 Codex
私有产品能力误判成 Lime current 需求。逐项路径、符号、哈希、测试名和分类以以下
JSON 为最终事实源：

- [TUI 目录/符号账本](./tui-structure-inventory.json)
- [TUI 802 个 snapshot 账本](./tui-codex-snapshot-inventory.json)
- [CLI 目录/符号账本](./cli-structure-inventory.json)
- [CLI 433 个测试账本](./cli-codex-test-inventory.json)

## 1. 总量差异

| 维度 | Codex | Lime | 差异 | 结论 |
| --- | ---: | ---: | ---: | --- |
| TUI Rust 源文件 | 579 | 91 | 缺 506，Lime 独有 18 | 目录体系未同构 |
| TUI Rust 类型/函数符号 | 12,000 | 1,491 | 缺 9,878，Lime 独有 791 | 大量行为仍聚合在 Lime owner |
| TUI snapshot | 802 | 0 | 802 未复制为 insta 文件 | 账本已分类，未机械迁移 |
| TUI 测试标记 | 4,145 | 372 | 缺 3,773（以扫描口径） | Cargo 实际 TUI 测试 390/390 |
| CLI Rust 源文件 | 82 | 11 | 缺 71 | 产品专属文件被排除，current 仍较薄 |
| execpolicy 源文件 | 17 | 17 | 0 | 目录同构 |
| CLI npm 文件 | 7 | 7 | 3 个文件名不同 | launcher 已改为 Lime 命名 |
| Codex CLI 测试 | 433 | - | 50 covered，88 partial，64 deferred，231 excluded | `missing=0` 不等于行为全覆盖 |

## 2. TUI 目录与文件差异

### 2.1 Codex 缺失于 Lime 的文件族

`filesMissingInLime=506`，完整路径清单在 TUI 结构账本。按一级 owner 计数如下：

| owner | 缺失数 | 主要差异 |
| --- | ---: | --- |
| 根级文件 | 112 | startup、config、terminal probe、history、streaming 等大量 owner |
| `app/` | 69 | history UI/pagination、startup、resize/reflow、safety buffering、thread state/routing、完整 app tests |
| `bottom_pane/` | 75 | footer、popup、selection、mentions、skills、hooks、textarea/vim、unified exec |
| `chatwidget/` | 110 | Codex 主交互状态机、输入流、权限、工具生命周期、replay、streaming、settings |
| `history_cell/` | 20 | approval、exec、MCP、patch、plan、request_user_input、session 等历史项渲染 |
| `render/` | 7 | renderable、line utils、streaming highlight |
| `streaming/` | 10 | chunking、fence、controller、commit tick、table holdback、render |
| `status/` | 10 | account/card/format/rate limits/remote/thread usage |
| `tui/` | 12 | history tail、input boundary、job control、keyboard modes、scrollback、Windows console |
| 其它目录 | 81 | `onboarding`、`pets`、`terminal_probe`、`ide_context`、`notifications`、`keymap_setup`、`public_widgets` 等 |

重点缺失的 Codex 同名顶层 owner：

`chatwidget.rs`、`history_cell/`、`render/`、`streaming/`、`terminal_probe.rs`、
`cwd_prompt.rs`、`session_start.rs`、`session_resume.rs`、`app_command.rs`、
`app_event.rs`、`app_event_sender.rs`、`app_server_connection.rs`、
`app_server_approval_conversions.rs`、`backend_banners.rs`、`dynamic_tools.rs`、
`dynamic_tools_mcp.rs`、`file_search.rs`、`hooks_rpc.rs`、`ide_context.rs`、
`terminal_title.rs`、`tooltips.rs`、`ui_consts.rs`、`workspace_command.rs`。

### 2.2 Lime 独有文件

这些文件在 Codex 无同名 owner，属于结构差异或待收敛 owner：

`bottom_pane/render.rs`、`command_popup.rs`、`entry.rs`、`highlight.rs`、`locale.rs`、
`markdown/local_links.rs`、`markdown/table.rs`、`model_picker.rs`、
`pending_input_preview.rs`、`projection.rs`、`projection_tests.rs`、`reconnect.rs`、
`runtime.rs`、`runtime_pty_tests.rs`、`settings.rs`、`status_indicator.rs`、
`view.rs`、`viewport.rs`。

其中 `projection.rs`、`runtime.rs`、`view.rs` 是 Lime 聚合 owner；`reconnect.rs` 是
顶层委托壳，真实 current owner 已在 `app/reconnect.rs`，且当前未发现消费者；
`viewport.rs` 已有 Codex-shaped state/test，但尚未完全接入 runtime resize/reflow 主路径。

### 2.3 Codex 集成测试目录缺失

Lime 没有 `crates/tui/tests/` 集成测试目录。Codex 的以下 11 个测试文件没有同构副本：

`tests/all.rs`、`tests/test_backend.rs`、`tests/manager_dependency_regression.rs`、
`tests/suite/focus_palette.rs`、`tests/suite/reconnect.rs`、
`tests/suite/resize_reflow.rs`、`tests/suite/status_indicator.rs`、
`tests/suite/vt100_history.rs`、`tests/suite/vt100_live_commit.rs`、
`tests/suite/mod.rs`、`tests/fixtures/oss-story.jsonl`。

因此当前差异是：Lime 主要依赖 inline unit tests、`projection_tests.rs`、
`runtime_pty_tests.rs` 和 Gate B；Codex 还有真实 `vt100` history/live-commit、focus、
resize/reflow、reconnect 集成套件和独立测试 backend/support。

## 3. TUI 符号差异

结构扫描结果为 Codex `12,000` 个符号、Lime `1,491` 个符号：

- `symbolNamesMissingInLime=9,878`：完整名称、路径和类型在
  `tui-structure-inventory.json.comparisons.symbolNamesMissingInLime`。
- `symbolNamesOnlyInLime=791`：完整名称、路径和类型在
  `tui-structure-inventory.json.comparisons.symbolNamesOnlyInLime`。
- 缺失符号主要集中于 `ChatWidget`/输入流、`HistoryCell`、`Renderable`、
  streaming controller、permission/approval、MCP、startup/history、terminal probe、
  account/status 和 Codex 私有产品面。
- Lime 独有符号主要来自 `Projection`、`Runtime`、`View`、`ModelCatalog`、
  `CollaborationModes`、resume/archive 聚合 owner，以及当前 App Server canonical
  projection 的类型。

不能把符号同名率当作语义覆盖率：Codex 私有 runtime 状态必须通过 App Server JSON-RPC
canonical contract 重建，不能直接把私有类型搬进 Lime。

## 4. TUI snapshot 差异

Codex snapshot 基线为 802 个，Lime 当前 snapshot 文件为 0；账本已逐项保存路径和 SHA-256。

| Codex 模块 | snapshot 数 | 分类 |
| --- | ---: | --- |
| `app` | 56 | contract/dead |
| `bottom_pane` | 244 | merge/dead |
| `chatwidget` | 278 | merge/dead |
| `history_cell` | 56 | merge/dead |
| `status` | 23 | merge/dead |
| `resume_picker` | 18 | contract |
| `markdown_render` | 20 | direct |
| `diff_render` | 23 | direct |
| `pager_overlay` | 9 | merge |
| `status_indicator_widget` | 8 | merge |
| `cwd_prompt` | 5 | contract |
| `onboarding` | 4 | dead |
| `model_migration` | 4 | dead |
| `inline_visualization` | 4 | defer |
| `custom_terminal` | 3 | defer |
| `debug_config` | 3 | dead |
| 其它模块 | 44 | 按账本逐项分类 |

分类总计：`direct=48`、`merge=579`、`contract=80`、`defer=25`、`dead=70`。

- `direct`：`diff_render`、`markdown_render`、`insert_history`、`render`、
  `terminal_hyperlinks`、`terminal_palette`、`table_detect`、`wrapping` 等纯终端算法。
- `merge`：与 Lime 交互相关、但必须合入 `app/composer/entry/view/picker` current owner。
- `contract`：跨 App Server、Thread/Turn/Item、历史恢复、multi-agent 或持久化状态。
- `defer`：`custom_terminal`、`git_action_directives`、`inline_visualization`、
  `keymap_setup`、`startup_hooks_review`。
- `dead`：Codex account、onboarding、migration、update、marketplace、Luna reserve、
  ChatGPT plan/account、产品专属 debug/config 等。

## 5. TUI 能力差异裁决

### 已对齐或已有 current owner

App Server 事件/请求分离、ThreadEventStore、replay filter、agent picker、resume/archive、
queue edit、approval、request_user_input、interrupt、external editor、model catalog、
多 agent overview、terminal restore、clipboard、markdown/diff/hyperlink/wrapping、
status indicator、真实 PTY Gate B 均已有 current owner 和回归测试。

### 部分对齐

1. Codex `chatwidget` 的完整状态机被 Lime `app` + `bottom_pane/chat_composer` + `projection`
   分散承接，目录和类型没有一一对应。
2. Codex `history_cell` 的历史项渲染被 Lime `entry.rs`/`thread_transcript.rs` 承接，
   但 approval、MCP、patch、plan、hook、exec 的完整 cell owner 和 snapshot 套件未同构。
3. Codex `render`/`streaming` 被 Lime `view.rs`、`entry.rs`、`markdown.rs`、`highlight.rs`
   分散承接，没有独立 `render` 和 `streaming` owner。
4. Codex `status`、`backend_banners`、`rate_limit_refresh`、`safety_buffering`、
   `misalignment_policy`、`history_ui`、`working_directory`、`session_start/resume` 仍有
   行为或测试缺口。
5. `viewport.rs` 已建立，但 resize/reflow 主路径还没有完全接入。
6. `reconnect.rs` 顶层壳仍在，应该在确认无引用后删除并保留负向守卫。

### 明确暂缓

`custom_terminal`、`git_action_directives`、`inline_visualization`、`keymap_setup`、
`startup_hooks_review`；另外包括 Codex 完整 vt100 snapshot suite、Windows terminal replay、
以及非当前 Desktop parity 优先级的产品能力。

### 明确禁止复制

Codex account/onboarding/update/marketplace/pets/theme、Luna reserve、ChatGPT 专属状态、
Codex daemon/state DB、Codex rollout/history DB、Codex misalignment 专属协议枚举、
第二套 runtime、`TerminalGuard`、`TuiTerminal`、旧 `lime-cli`、`terminal-ui`、旧
bounded-EOF restart 和旧 crossterm registry 输入路径，均为 `dead/deleted/forbidden-to-restore`。

## 6. TUI Cargo/依赖差异

Codex TUI 是独立产品 crate：

- package `codex-tui`，`autobins=false`。
- binary `codex-tui`，另有 `md-events` binary。
- lib name `codex_tui`。
- Codex 私有依赖覆盖 config/login/rollout/state/model-provider/sandboxing/plugin/
  connectors/history/feedback/file-search/cloud-config/otel/terminal-detection 等 owner。
- 测试依赖包括 `insta`、`wiremock`、`serial_test`、`assert_matches`、`vt100`、Codex test support。

Lime TUI 当前为：

- package `tui`，只有 lib `tui`，没有 `tui` binary 和 `md-events`。
- 依赖收敛到 `app-server-client`、`app-server-protocol`、`agent-protocol`、
  `tool/runtime` 现有 workspace owner；不复制 Codex 私有 crate。
- 已对齐 crossterm fork 和 feature：
  `https://github.com/openai-oss-forks/crossterm@45fecb9508105988f42fe6ff0441783ed3717f92`，
  `bracketed-paste` + `event-stream`。
- 仍缺 Codex 的独立 binary、`insta` snapshot 测试基础设施、完整 test backend/support。

## 7. CLI Rust 目录与符号差异

### 7.1 文件

Codex `codex-rs/cli` 为 82 个文件，Lime `lime-rs/crates/cli` 为 11 个文件。
Lime current 文件为：

`debug_sandbox.rs`、`exit_status.rs`、`lib.rs`、`main.rs`、`mcp_cmd.rs`、`plugin_cmd.rs`、
`queue_cmd.rs`、`sandbox_setup.rs`、`wsl_paths.rs`、`Cargo.toml`、`tests/execpolicy.rs`。

Codex 缺失文件完整路径在 `cli-structure-inventory.json.comparisons.rustFilesMissingInLime`，
其中包括：

- 产品/诊断：`src/doctor/**`、`src/login.rs`、`src/marketplace_cmd.rs`、`src/app_cmd.rs`、
  `src/bin/logs_client.rs`、`src/state_db_recovery.rs`、`src/migrate_rollouts.rs`。
- Cloud/远程：`src/cloud_config.rs`、`src/remote_control_cmd.rs`、
  `src/exec_server_telemetry*.rs`、`src/debug_sandbox/cloud_config*.rs`。
- 平台私有：`src/desktop_app/**`、`src/debug_sandbox/pid_tracker.rs`、
  `src/debug_sandbox/seatbelt.rs`。
- 测试：`tests/app_server.rs`、`tests/cloud_*.rs`、`tests/debug_*.rs`、`tests/features.rs`、
  `tests/mcp_*.rs`、`tests/plugin_cli.rs`、`tests/queue.rs`、`tests/sandbox_*.rs`、
  `tests/marketplace_*.rs`、`tests/login.rs`、`tests/update.rs` 等。

### 7.2 符号

- CLI Rust 缺失 Codex 符号 `774` 个，Lime 独有 `65` 个；完整路径/名称在结构账本。
- `execpolicy` 为 17/17 文件、95/95 符号，同构完成。
- Lime current 命名已按 Codex 形状收敛：`MultitoolCli`、`Subcommand`、`CompletionCommand`、
  `TuiCli`、`ExecCli`、`ResumeCommand`、`McpCli`、`PluginCli`、`FeaturesCli`、
  `QueueCommand`、`DebugCommand`、`SandboxStateArgs`、`SandboxSetupCommand`、
  `handle_exit_status` 等。
- 当前仍有 Lime 独有命名/产品扩展：`SkillsCli`、`ThreadCommand`、`ConnectionArgs`、
  `QueueSubcommand`、`FeatureListArgs`、`PluginIdArgs` 等，需要继续确认是否应成为
  current contract 或删除。

## 8. CLI 测试差异

Codex CLI 账本共 433 个测试，来自 53 个源文件。当前分类：

| 分类/状态 | 数量 | 含义 |
| --- | ---: | --- |
| `direct / covered` | 9 | 纯 parser、WSL、execpolicy 等直接覆盖 |
| `contract / covered` | 41 | 通过 App Server JSON-RPC/current owner 覆盖 |
| `contract / partial` | 88 | 有 current owner，但行为或 Gate B 未完全同构 |
| `cloud-deferred / deferred` | 64 | authenticated remote transport 基础完成前暂缓 |
| `product-specific / excluded` | 231 | Codex account/doctor/marketplace/updater/desktop/state 私有能力 |
| `missing / pending` | 0 | 每个测试都已明确绑定到分类；不代表行为全部实现 |

### 已覆盖 current CLI 行为

parser command tree、sandbox parser、sandbox command lowering、sandbox setup parser、
execpolicy check、debug models/clear memories、features list/enable/disable、基本 plugin
add/list/read/search/enable/disable/remove、MCP list/add/get/remove/start/stop、queue add/list、
thread archive/delete/unarchive/fork/resume、非交互 exec、TUI/exec 入口、WSL path 算法和
退出码处理。

### 88 个 partial 的能力族

1. permission options：Codex `approve-for-me`、`not-so-yolo` 别名和 root/exec/resume 合并
   优先级未逐字复制；Lime 以 `permissionProfile/list` + `turn/start` 为 current owner。
2. plugin control plane：基本 CRUD 已覆盖，marketplace cache、远程 catalog refresh、
   未配置 repo-local marketplace 等 Codex 边界未同构。
3. MCP mutation：OAuth registration/logout 尚无 `mcpServer/oauth/*` App Server contract，
   当前必须 fail closed，禁止 CLI 直接读 credential store。
4. queue：本地 queue 行为已覆盖，远程 queue 证据随 Cloud deferred。
5. app-server entrypoint：CLI 转发参数已保留，transport/daemon/proxy/signed/capability
   语义仍由 sibling App Server owner 承担。
6. thread lifecycle：canonical resume/archive/delete/unarchive/fork 已接入，但 Codex 私有
   daemon/state/rollout storage 不复制。
7. sandbox：sandbox-state replay、managed network、named profile 全矩阵仍不完整。

### 64 个 Cloud deferred 能力族

remote-control、exec-server、remote API-key auth、Cloud managed permission profile、
Cloud MCP config/OAuth、remote plugin catalog、remote queue、remote working directory、
parent lifetime/telemetry、远程 sandbox 和所有 authenticated transport 相关测试。
完整测试名在 `cli-codex-test-inventory.json` 中 `status=deferred` 的条目。

### 231 个 product-specific 排除能力族

Codex `doctor` 全部诊断面、account/login、marketplace add/remove/upgrade、updater/update、
desktop launcher、macOS PID tracker/seatbelt denial internals、state DB recovery、rollout
migration、logs client、Codex 内部 support client 和已退役 command alias。完整测试名在账本中
`status=excluded` 的条目；这些不能恢复成 Lime compat/current。

## 9. npm/launcher 差异

### 文件

Codex `codex-cli`：`.gitignore`、`bin/codex.js`、`package.json`、`scripts/README.md`、
`scripts/build_npm_package.py`、`scripts/init_firewall.sh`、`scripts/run_in_container.sh`。

Lime `packages/cli`：`.gitignore`、`README.md`、`bin/lime.js`、`package.json`、
`scripts/README.md`、`scripts/build_npm_package.py`、`tests/npm-package.test.mjs`。

Lime 缺少 `init_firewall.sh`、`run_in_container.sh`；两者属于 Cloud/容器专属，不应伪造
成生产入口。Lime 多了 README 和 npm package test。

### manifest

| 字段 | Codex | Lime | 状态 |
| --- | --- | --- | --- |
| package | `@openai/codex` | `@limecloud/lime` | 品牌差异，按产品要求为 Lime |
| bin | `codex` | `lime` | 已收敛到 Lime |
| version | `0.0.0-dev` | `1.141.0` | 发布策略差异 |
| Node | `>=16` | `>=18` | 运行时基线差异 |
| pnpm | `10.34.5` | `9.15.9` | workspace 基线差异 |
| license | Apache-2.0 | MIT | 产品法律文本差异 |
| scripts | 无 | `build:npm`、`test` | Lime 增加构建/测试入口 |

### 平台包

Codex 支持 `x86_64-unknown-linux-musl`、`aarch64-unknown-linux-musl`、
`x86_64-apple-darwin`、`aarch64-apple-darwin`、`x86_64-pc-windows-msvc`、
`aarch64-pc-windows-msvc`，并在 Linux 分支同时覆盖 Android。

Lime 当前只有 `x86_64-unknown-linux-gnu`、`x86_64-apple-darwin`、
`aarch64-apple-darwin`、`x86_64-pc-windows-msvc`；缺 Linux ARM64、Linux musl、Windows ARM64、
Android/Linux musl 分支。

### launcher 命名和实现

Lime launcher 源码仍残留 Codex-shaped 变量/函数名：`codexPackageRoot`、
`findCodexExecutable`、`isPnpmOwnedCodexInstall`；这是当前未完成的命名差异，不能把
Codex 名字留在 Lime current owner 中。应改为 Lime 语义（例如 package root 和
`findLimeExecutable`），并同步 npm 测试和结构账本。Lime 额外设置
`DYLD_LIBRARY_PATH`/`LD_LIBRARY_PATH` 以加载 bundled native libraries，这是运行时适配，
不是 Codex 同构差异。

## 10. 架构与 Cloud 预留

唯一业务主链必须保持：

`Desktop Host 或 CLI/TUI Host -> App Server JSON-RPC -> RuntimeCore -> Thread/Turn/Item projection`。

Cloud 只能作为 `app-server-client` 的 authenticated transport，继续消费同一 protocol、
runtime 和 canonical read model；不能复制 runtime、工具 registry、状态机或持久化。

因此下列“差异”是架构约束，不是待机械复制项：Codex 私有 crate 依赖、Codex daemon/state DB、
Codex rollout/history DB、hosted `--code-mode-host` URL、bubblewrap network proxy、
account/onboarding/update/marketplace/doctor/pets/theme，以及 Codex 产品内部 telemetry。

## 11. 运行证据差异

已验证：

- `cargo test --locked -p tui -p cli`：TUI 390/390，CLI 50/50。
- `cargo clippy --locked -p tui --no-deps -- -D warnings`、`cargo check --locked -p tui`。
- `cargo metadata --locked`、`cargo fmt --package tui -- --check`、`git diff --check`。
- TUI/CLI inventory Vitest、`npm run test:contracts`、`npm run governance:legacy-report`。
- TUI Gate B 七场景通过；`complete` 场景连续 5 次通过，包含 queue edit、agents overview、
  external editor、approval/request_user_input、interrupt、terminal restored。

仍缺：

- Codex `insta` snapshot 全量迁移或等价 contract snapshot。
- Codex `tests/suite` 的 focus/reconnect/resize/vt100 集成套件。
- Windows 原生 TUI/CLI/packaged 运行证据和完整 terminal probe replay。
- npm Linux musl/ARM64、Windows ARM64 平台包 Gate B。
- MCP OAuth App Server contract、Cloud authenticated transport 真实 Gate B。
- runtime resize/reflow 对 `viewport.rs` 的完整接线。

## 12. 最终结论与优先级

当前状态不是“Codex TUI/CLI 已完全复制”，而是“Codex-shaped Lime current host 已可运行，
且所有已知差异已进入账本”。最重要的剩余差异按优先级为：

1. 补齐 Lime `chatwidget/history_cell/render/streaming` 的 current owner 拆分和测试基础设施，
   同时把可迁移的 direct/merge snapshot 转为 Lime owner 的稳定 contract snapshot。
2. 接通 `viewport` resize/reflow，并补 Codex 形状的 reconnect/focus/vt100 集成套件。
3. 收敛 CLI partial：MCP OAuth contract、plugin edge cases、sandbox profile/network 矩阵、
   app-server entrypoint transport tests。
4. 完成 npm 平台目标与 packaged Gate B；Cloud 只在 authenticated transport contract 完整后推进。
5. 清理无消费者的顶层 `tui/src/reconnect.rs`，并继续用结构/治理守卫阻止
   `lime-cli`、`terminal-ui`、旧 runtime 和旧 launcher 命名回流。
