# TUI/CLI 继续同步 Codex 执行计划

状态：in-progress
日期：2026-09-08
参考实现：`/Users/coso/Documents/dev/rust/codex`
当前基线：Rust commit `cac96cd7b1756ab42e8925d938817a2ac10ebb6e`

本计划承接 [全量差异报告](./codex-lime-tui-cli-difference-report.md)，目标是继续按
Codex 的真实目录、文件、公开类型、函数名和测试名同步 Lime 的 TUI/CLI。禁止根据截图
或主观设计补造同名壳；每个实现批次必须先读取对应 Codex 源文件和测试，再迁移到 Lime
current owner。

## 1. 基线与完成目标

当前事实：

| 维度 | Codex | Lime | 当前结论 |
| --- | ---: | ---: | --- |
| TUI Rust 文件 | 579 | 120 | 目录体系仍未同构 |
| TUI 类型/函数符号 | 12,000 | 1,863 | 结构差异大，须按 owner 分批收敛 |
| TUI snapshot | 802 | 0 | 已建立逐项分类账本，未迁入快照文件 |
| CLI Rust 文件 | 82 | 11 | current CLI 较薄，产品专属文件不机械复制 |
| execpolicy 文件 | 17 | 17 | 已同构 |
| CLI 测试 | 433 | - | 50 covered、88 partial、64 Cloud deferred、231 excluded |

完成目标不是让文件数量形式上相等，而是：

1. Codex current TUI 行为都有明确 Lime owner、测试和分类；可迁移的目录/符号/测试使用
   Codex 原名，不能继续由 Lime 聚合文件隐式承载。
2. TUI 的 direct/merge 场景具备稳定的 Lime snapshot 或等价 TestBackend/VT100 断言；
   contract 场景具备真实 App Server JSON-RPC evidence。
3. CLI 的 88 个 partial 要么补齐 current contract 和真实 Gate B，要么记录明确的
   `defer`/`excluded` 理由，不保留 `missing/pending`。
4. Desktop、CLI/TUI 继续共享 App Server、RuntimeCore、Thread/Turn/Item projection；
   Cloud 只增加 authenticated transport，不复制 runtime、状态机、工具 registry 或存储。

## 2. 不变量与命名规则

- Codex 路径、模块名、`pub struct/enum/type/fn` 和测试名是基线；迁移前保存上游路径、
  source commit 和内容 hash。
- `current` 只能落在 `lime-rs/crates/**`、`packages/cli` 或 App Server current owner；
  `compat` 只能委托；`deprecated` 只能迁出；`dead` 必须删除并补回流守卫。
- TUI 业务链固定为：`TUI Host -> app-server-client -> App Server JSON-RPC -> RuntimeCore -> canonical projection`。
- 不复制 Codex 私有 auth、account、rollout/history DB、daemon、provider manager、
  marketplace、doctor、pets、theme、update、ChatGPT 状态或平台私有 sandbox internals。
- 不恢复 `lime-cli`、`terminal-ui`、`TerminalGuard`、`TuiTerminal`、旧 bounded EOF
  restart、旧 crossterm registry 输入路径；这些只能出现在 negative guard/history evidence。
- 生产路径不使用 mock；fixture 只能用于测试，且必须通过真实 stdio/PTY/JSON-RPC 边界。
- 新增代码不得使用 `codex*` 品牌前缀；只有与 Codex 对齐的领域 owner 名称可以保留
  `ChatWidget`、`HistoryCell`、`Renderable` 等语义名。

## 3. 写集与避让范围

### 本计划允许的写集

- `lime-rs/crates/tui/**`
- `lime-rs/crates/cli/**`
- `lime-rs/crates/app-server-client/**`、`lime-rs/crates/app-server-protocol/**`（仅为已确认的
  TUI/CLI contract 缺口）
- `lime-rs/crates/app-server/src/processor/thread.rs`、
  `lime-rs/crates/app-server/src/runtime/{thread_read.rs,canonical_thread_store.rs,canonical_thread_store_tests.rs}`、
  `lime-rs/crates/app-server/tests/thread_v2_jsonrpc.rs`（仅限 Codex paginated resume cursor
  contract，不扩展其它 App Server 业务）
- `packages/cli/**`、`pnpm-lock.yaml`、`lime-rs/Cargo.toml`、`lime-rs/Cargo.lock`
- `scripts/app-server/{tui,cli}-*.mjs` 及对应测试、TUI/CLI inventory 生成器和账本
- `.gitignore`、`internal/exec-plans/README.md`、本计划和差异报告

### 明确避让

`electron/**`、无关 `src/**`、Codex Desktop/Goose 计划热区、服务端外部仓库、
`lime-rs/crates/agent/**` 和已有未跟踪产物。若必须改变 App Server protocol，先单独登记
contract 写集、消费者、schema、锁文件和测试，不在 TUI 批次夹写。

## 4. 阶段 A：TUI Codex owner 同构

### A0 迁移准备

- [x] 为每个批次建立 `upstream-path -> lime-owner -> classification -> test` 映射，来源必须
  是 Codex 当前 checkout，不得只依据文件名猜测。
- [x] 复核 `tui-structure-inventory.json` 和 `tui-codex-snapshot-inventory.json` 的 hash、
  source commit 与分类；新增 upstream 文件必须先分类再写代码。
- [x] 维护唯一 owner 表，确认 `projection.rs`、`runtime.rs`、`view.rs` 等 Lime 聚合 owner
  的拆分边界；不创建第二个 Thread/Turn/Item 模型。

### A1 纯终端 direct owner

对应 Codex：`render/`、`streaming/`、`markdown_render/`、`diff_render`、`insert_history`、
`terminal_hyperlinks`、`terminal_palette`、`table_detect`、`wrapping`。

- [x] 按 Codex 文件名补齐 `tui/src/render/` 的 `mod.rs`、`line_utils.rs`、`renderable.rs`、
  `highlight.rs` 和 streaming highlight；把 Lime `highlight.rs`/`view.rs` 中对应纯算法迁入
  唯一 owner。当前 `render/highlight.rs` 已承接固定 ANSI 主题的 Lime 算法，
  `render/highlight_streaming.rs` 已复制 Codex 的增量状态机；未引入 Codex 私有主题配置。
- [x] 按 Codex 真实测试名迁移 48 个 `direct` snapshot 场景，使用 `insta` 或等价
  `TestBackend/Buffer` 断言；不引入 Codex runtime 状态。
- [ ] 对 `markdown_render`、`diff_render`、`wrapping`、OSC 8 和宽字符边界补窄终端、UTF-8
  grapheme、URL、表格、重排回归。
- [ ] 删除迁移后的 Lime-only 重复纯渲染函数，更新 inventory 并增加禁止重复 owner 的结构守卫。

当前进度：`render`/`highlight`/`renderable`/增量高亮和 `insert_history` owner 已完成；
48 个 direct snapshot 已全部按 Codex 测试名补齐 Lime 等价断言：
`diff_render` 23、`markdown_render` 20、`render` 1、`terminal_hyperlinks` 2、
`insert_history` 2。`insert_history` 使用真实 VT100 parser，覆盖 Zellij raw terminal
软换行、overflow replay、viewport 边界和 history-row bookkeeping，没有复制 Codex 私有
`custom_terminal` 或 runtime state DB。
`history_cell/{mod,base,messages,exec,patches,plans,approvals,mcp,notices,request_user_input,separators,session}.rs`
现已建立，`TranscriptHistoryCell` 只适配 Lime canonical `TranscriptEntry`；
`exec_cell/{mod,model,live_output,render}.rs` 现已承接 command output head/tail、UTF-8
行截断和 omission marker，`entry.rs` 不再持有 bounded output 算法。已新增 Codex-shaped
`HistoryCell`、`TranscriptHistoryCell`、`CommandOutput`、`LiveCommandOutput`、
`OutputLines`、`OutputLinesParams` 和 `output_lines` 符号，并由结构守卫锁定目录/符号。
`app/history_ui.rs` 现已承接 Codex-shaped `render_transcript_content_lines`，主 transcript
与 pager overlay 复用同一 canonical projection 投影；未伪造尚无 Lime consumer 的
`history_pagination` 或完整导出 runtime。
已通过 `cargo test -p tui`（510 个测试）、TUI Clippy、结构 inventory 与 snapshot
inventory 守卫。

退出条件：48 个 direct snapshot 全部有 Lime owner 和稳定断言；`cargo test -p tui`、
TUI Clippy、结构 inventory 和 TUI Gate B 通过。

### A2 transcript/history owner

对应 Codex：`history_cell/`、`exec_cell/`、`app/history_ui.rs`、`app/history_pagination.rs`、
`app/transcript_export.rs`、`pager_overlay/`。

- [x] 建立 `history_cell` 的 canonical entry adapter，只消费 App Server `Thread/Turn/Item`
  projection；按 Codex 文件拆分 approvals、exec、MCP、patches、plans、messages、notices、
  session 和 request_user_input。
- [x] 建立 `exec_cell/{mod,model,live_output,render}.rs`，把 command live output 与 Lime
  `entry.rs` 的重复渲染收回同一 owner。
- [x] 将 `Ctrl+T` transcript overlay、resume transcript、pager 和 export 统一到
  `history_cell`/`pager_overlay` 的渲染链；不创建 rollout/history DB。当前 `/export` 支持
  剪贴板和显式路径 noclobber 写入，主 transcript、pager 与 resume preview 共享
  `app/history_ui.rs` 的 canonical projection。
- [x] 建立 Codex-shaped `app/history_pagination.rs` 的 cursor/loading/去重状态，并接入
  App Server `thread/items/list` contract；legacy thread 继续由 `thread/read` hydrate，
  paginated thread 使用 `thread/resume(excludeTurns=true)` 后按 `nextCursor` 加载 older
  items。TUI 不持有第二套 history store。
- [x] 对齐 Codex `paginated_resume_backwards_cursors`：canonical ThreadStore 的 turn/item
  page 始终从首行生成 inclusive `backwardsCursor`，metadata-only `thread/resume` 返回稳定
  turn/item head cursor，重复 resume 保持相同 cursor，cursor 可重新读取最新 canonical
  Turn/Item。TUI `advancing_cursor` 同步为 Codex 的重复 cursor 截止语义。
- [ ] 迁移 `merge=579` 中与 history/transcript/pager 相关场景；跨 runtime 的 80 个
  `contract` 场景必须绑定 App Server fixture 和 canonical identity。

当前剩余：Codex 的 export destination/selection popup、完整 persisted-history hydration、
review prompt filtering、MCP/file-activity details 和 overlay 专用 bounded reflow 尚未完全
迁移；这些保留为下一刀 `defer`，不得用本地 DB 或伪造字段补齐。历史分页已具备 current
最小路径，public App Server JSON-RPC 已覆盖 resume/head cursor/turn-item 重读；真实 stdio
fixture 与 PTY/alternate-screen Gate B 已通过，剩余是 history/transcript contract 场景扩大。

聚合文件退出约束：`processor/thread.rs`、`runtime/thread_read.rs` 与
`runtime/canonical_thread_store.rs` 均已超过 1000 行。本批只允许完成上述 Codex cursor
合同的最小补丁；下一次继续修改这些热区前，必须先依据 Codex 当前 checkout 确认对应
子模块边界并迁出本次触达的 resume/page 逻辑，保留同名函数与公共测试，禁止继续堆叠。

架构图确认：本批保持 `TUI -> app-server-client -> App Server JSON-RPC -> RuntimeCore ->
ThreadStore -> canonical Thread/Turn/Item projection` 单主链；Cloud 仍只允许 transport 扩展。
责任开发者确认：root，2026-09-10。

退出条件：Message/Reasoning/Command/Patch/MCP/Plan/Multi-Agent/approval/request_user_input
各有稳定 cell owner；历史分页、overlay、export、live output 具备 VT100/PTY 证据。

### A3 composer/chatwidget/bottom_pane owner

对应 Codex：`chatwidget/`、`bottom_pane/`、`public_widgets/composer_input.rs`、
`keymap/`。

- [ ] 逐文件对照 Codex `chatwidget/{constructor,input_flow,input_submission,interaction,
  interrupts,tool_lifecycle,turn_lifecycle,streaming,transcript,settings}.rs`，把 Lime
  `app`、`bottom_pane/chat_composer` 和 `projection` 中相应逻辑迁入 Codex-shaped owner。
- [ ] 按 Codex 目录补齐 composer 的 `attachment_state`、`draft_state`、`history_search`、
  `popup_state`、`slash_input`、`vim_history`、`vim_search`、`footer_state` 和测试；保留
  Lime 的图片/队列 canonical contract，不复制 Codex 私有 provider/auth。
- [ ] 按 Codex `bottom_pane` 补齐 action banner、footer、selection、file search、skills、
  hooks、MCP elicitation 和 unified exec 的可用子集；没有 App Server consumer 的动态工具
  和 MCP elicitation 继续 reject/fail closed。
- [ ] 将 Lime `command_popup.rs`、`pending_input_preview.rs`、`status_indicator.rs`、
  `settings.rs` 迁移到对应 Codex owner，迁移后删除重复聚合实现。

退出条件：composer key/event/input/interrupt/queue/attachment/history 逐函数有 Codex 来源；
所有当前可用 popup、approval、request_user_input、status、queue 场景有测试和五语言文案。

### A4 app/startup/reconnect/resize owner

对应 Codex：`app/{startup,startup_prompts,working_directory,session_picker,reconnect,
resize_reflow,thread_routing,thread_session_state,thread_title}.rs`、`tui/*`。

- [x] 把 `viewport.rs` 接入 runtime resize/reflow 主路径，使用实际 TUI resize event 和
  Ratatui viewport，覆盖宽度变化、底部对齐、composer 草稿、alternate-screen round trip
  和重复缩放；Codex 的 tmux-specific smoke 在 Lime 中等价为 portable-pty `MasterPty::resize`。
- [ ] 迁移 Codex startup/session/working-directory 语义到 App Server contract；没有 Lime
  current contract 的字段明确标为 `contract/defer`，不能在 TUI 本地合成。
  当前 startup 初始化已迁入 `app/startup.rs::initialize_session`，统一承接
  `thread/start`、`thread/resume`、canonical history hydrate、permission/collaboration
  catalog、model catalog、skills/list、prompt history 与 queued submissions；启动保护 gate
  的 Codex 同名 helper 已有 Lime 测试。`app/startup_prompts.rs` 已承接 Codex 同名的
  `SkillLoadWarningState`、`StartupTooltipOverride`、model migration/availability NUX 纯
  逻辑，并由真实 `skills/list`、`model/list` JSON-RPC 启动请求消费。`cwd_prompt.rs` 已
  承接 Codex 同名 `CwdPromptAction`、`CwdSelection`、`CwdPromptOutcome` 状态机。
  剩余 protected-input handoff、`working_directory`/`/cd` 和 resume-cwd persistence
  继续按 contract/defer 处理。
  当前已对齐可由 Lime canonical session 直接承接的 `/pwd` 与 Codex `/cwd` 别名，使用
  `App.cwd -> ConversationProjection`，并补齐五语言文案和同名回归测试；Codex `/cd` 的
  trust/config、后台 terminal、fork/replace 与 resume-cwd preference 仍为 `contract/defer`，
  不在 TUI 本地伪造。
- [x] 补齐 `tests/suite/status_indicator.rs` 的 Lime 版本；它消费独立的
  `ansi-escape::{ansi_escape, ansi_escape_line}` current owner。
- [x] 补齐 `tests/suite/reconnect.rs` 的 Lime 版本；测试通过真实 PTY 和 loopback
  `RemoteTransport` 断线/重连，验证草稿保留、精确两次 `thread/resume`、恢复后的通知路由
  和 alternate-screen 恢复。顶层 `tui/src/reconnect.rs` 仍只作为历史委托壳，待零消费者后
  删除。
- [x] 补齐 `tests/suite/resize_reflow.rs` 的 Lime 版本，保留 Codex 四个测试名并通过真实
  PTY window-size signal、VT100 屏幕投影和终端退出恢复验证；Gate B 以单线程顺序执行，
  不依赖 tmux、live provider 或 mock backend。
- [x] 补齐 `tests/suite/focus_palette.rs` 的 Lime 版本，并绑定真实 PTY、启动期 OSC 10/11
  palette probe、FocusGained 输入恢复和 alternate-screen restore；Gate B 通过
  `suite::focus_palette::focus_gained_with_unanswered_palette_queries_preserves_immediate_input`
  执行，不使用 mock backend 或空 `#[ignore]`。
- [ ] 把 terminal lifecycle 继续固定在 `Tui`/`Terminal`/`EventBroker`/`FrameRequester`，
  不恢复 `TerminalGuard` 或 runtime EOF 重启。

退出条件：TUI resize/reflow、focus、reconnect 和 terminal restore 通过真实 PTY/VT100；
旧 reconnect 壳和旧输入恢复路径不存在 current 引用。

## 5. 阶段 B：TUI 测试体系同构

- [x] 新增 Codex-shaped `lime-rs/crates/tui/tests/` 第一批：`all.rs`、`test_backend.rs`、
  `manager_dependency_regression.rs`、`suite/mod.rs`、`suite/vt100_history.rs`、
  `suite/vt100_live_commit.rs`、`suite/status_indicator.rs`。VT100 测试直接调用 Lime
  current `insert_history_lines`/`RowBuilder`，status 测试消费新增的 Codex-shaped
  `ansi-escape::{ansi_escape, ansi_escape_line}` owner；manager regression 扫描 Lime
  `src`，不复制 Codex 私有 `custom_terminal` 或 manager/runtime 状态。
- [x] 迁移 `suite/reconnect.rs`：使用 Lime 当前 `--remote` + `RemoteTransport` 的公开
  App Server JSON-RPC WebSocket 合同，未复制 Codex 私有 daemon 控制 socket、auth、history
  DB 或 runtime。`fixtures/oss-story.jsonl` 在 Codex 当前 `tui/tests` 没有 Rust consumer，
  继续保持 `defer`，不得为填文件差异而引入未消费 fixture。
- [ ] 从 Codex 测试复制测试结构和测试名，仅替换 Lime App Server fixture、canonical
  projection 和五语言文案；不得把 Codex 私有 backend/state DB 带入测试。
- [ ] 引入 `insta`、`serial_test`、`assert_matches` 等依赖前先确认只用于 TUI test target，
  同步 Cargo.lock 和最小结构守卫。
- [ ] 对 802 个 snapshot 保持逐项账本：`direct` 迁移、`merge` 合并、`contract` 绑定真实
  protocol、`defer` 保留退出条件、`dead` 禁止复制。

当前证据：Codex-shaped 集成目录已建立，VT100 history/live commit、status indicator、
focus palette、resize/reflow 和 reconnect 共 15 个测试通过；TUI library 510/510、terminal
probe 定向测试、TUI integration Clippy、fmt、inventory、治理报告和真实
`smoke:tui-gate-b`（包含 `reconnect=ok`）均通过。`fixtures/oss-story.jsonl` 仍是明确
`defer`，原因是上游当前没有 Rust consumer。

退出条件：TUI 集成目录、snapshot、TestBackend、VT100 history/live commit、status 和
PTY Gate B 同时通过；账本无未分类条目，且测试失败不会回退到 mock backend。

## 6. 阶段 C：CLI 88 个 partial 收口

### C1 CLI current contract

- [ ] `permission options`：按 Codex `approve-for-me`、`not-so-yolo` 和 root/exec/resume
  precedence 测试逐项决定是否能映射 Lime `permissionProfile/list` + `turn/start`；不能
  映射的别名不添加假兼容。
- [ ] `plugin`：补 marketplace cache、repo-local marketplace、catalog refresh、JSON
  output 和 enable/disable 边界；所有 mutation 继续走 `plugin/*` App Server owner。
- [ ] `mcp`：先补 `mcpServer/oauth/*` protocol、schema、client、handler 和 fixture，再
  实现 login/logout；协议未完成前继续 fail closed，禁止 CLI 直接读 credential store。
- [ ] `queue`：补本地 queue 的完整命令/错误矩阵；远程 queue 只在 Cloud transport evidence
  完成后启用，不添加本地 fallback。
- [ ] `app-server entrypoint`：复制 Codex 参数和错误测试到 Lime sibling App Server owner；
  transport/daemon/proxy/capability 只在有 current handler 时暴露。
- [ ] `sandbox`：补 sandbox-state replay、managed network、named profile 矩阵；缺少
  App Server/tool-runtime owner 时保持 fail closed，不在 CLI 自建策略。

### C2 CLI 结构与命名

- [ ] 保持 `MultitoolCli`、`Subcommand`、`TuiCli`、`ExecCli`、`ResumeCommand`、
  `McpCli`、`PluginCli`、`FeaturesCli`、`QueueCommand`、`DebugCommand`、
  `SandboxSetupCommand`、`handle_exit_status` 等 Codex-shaped owner。
- [ ] 将 `packages/cli/bin/lime.js` 的 `codexPackageRoot`、`findCodexExecutable`、
  `isPnpmOwnedCodexInstall` 改为 Lime 语义，并同步 npm package test 和 structure inventory。
- [ ] 扩展 Linux ARM64/musl、Windows ARM64、Android 目标前必须先有可复现构建产物、
  native payload、launcher smoke 和 CI evidence；没有产物的 optional package 不得发布。
- [ ] 保持 `init_firewall.sh`、`run_in_container.sh` 为 Codex Cloud/容器参考，不复制为
  Lime 伪生产入口。

## 2026-09-11 CLI 工作目录参数命名收敛

- `lime-rs/crates/cli/src/main.rs::ConnectionArgs::cwd` 已按 Codex
  `SharedCliOptions::cwd` 对齐为 `--cd`/`-C`；`exec`、`resume`、默认 TUI、Thread/Skills/
  Plugin/Queue 等共享连接参数统一消费该定义。
- 真实 TUI PTY fixtures（runtime、focus palette、reconnect）和 CLI Gate B fixtures 已统一
  使用 `--cd`；旧 `--cwd` 仅保留在 CLI 负向解析回归，确保 legacy 命名不能回流。
- 退出证据：`cargo test --manifest-path lime-rs/Cargo.toml -p cli --bin lime`（42）、
  `cargo test --manifest-path lime-rs/Cargo.toml -p tui --lib`（507）、
  `node scripts/app-server/cli-gate-b.mjs`、
  `LIME_TUI_GATE_B_SCENARIOS=complete node scripts/app-server/tui-gate-b.mjs`、
  CLI Gate B Vitest（4 tests）、fmt 和 `git diff --check` 均通过。
- 分类：CLI/TUI 参数与 fixtures 为 `current`；`--cwd` 仅为 `dead` 负向 guard；没有新增
  `compat` 或 `deprecated` 入口。与主链关系：参数只影响 Host 启动配置，业务仍走同一
  `CLI/TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical Thread/Turn/Item`。

## 2026-09-11 session_resume 纯路径语义

- 新增 `lime-rs/crates/tui/src/session_resume.rs`，按 Codex 同名 owner 承接无副作用的
  `cwds_differ` 路径比较：优先解析现存路径的 symlink/canonical path，路径不存在时执行
  lexical `.`/`..` 归一化。
- `app/startup.rs` 与 `app/session_lifecycle.rs` 统一通过该 helper 判断是否需要更新本地
  cwd；最终 cwd 仍以 App Server `thread/start`/`thread/resume` 的 `response.cwd` 为事实源。
- Codex `effective_resume_cwd_mode`、rollout/state DB 读取、trust/config 重建、后台 terminal
  阻塞检查和 `/cd` fork/replace 仍没有 Lime current contract，继续分类为 `contract/defer`，
  没有复制私有持久化或在 TUI 本地伪造配置。
- 验证：`cargo test --manifest-path lime-rs/Cargo.toml -p tui --lib session_resume`（3）、
  `cargo clippy --manifest-path lime-rs/Cargo.toml -p tui --lib --no-deps -- -D warnings`、
  `cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check` 通过。
- 同批修复 `tests/suite/focus_palette.rs` 的 PTY 退出时序：等待 Ctrl-U redraw 后再发送
  Ctrl-D，避免 focus palette 与 resize 串行 Gate B 下的偶发退出竞态；隔离测试与完整
  `LIME_TUI_GATE_B_SCENARIOS=complete node scripts/app-server/tui-gate-b.mjs` 均通过。

退出条件：88 个 partial 全部变成 `covered`、有记录的 `defer` 或 `excluded`；CLI Gate B、
npm staging/launcher、JSON/JSONL/stdin/exit/signal/completion 和结构 inventory 通过。

## 7. 阶段 D：Cloud transport foundation

仅在阶段 A-C 的本地 current contract 稳定后推进：

- [ ] 完成 authenticated WebSocket transport 的 tenant identity、protocol version、TLS、
  credential lifecycle 和 token redaction contract。
- [ ] 补跨网络 disconnect/reconnect/resume、tenant isolation、rate limit、audit 和
  credential leak negative tests；TUI/CLI 继续复用同一 session facade。
- [ ] 远程 queue、remote plugin catalog、Cloud managed permission profile、remote sandbox
  和 exec-server 测试仍对应 CLI ledger 的 64 个 `cloud-deferred` 条目。
- [ ] 未取得 LimeCore 服务端合同、租户隔离证明和真实远端 Gate B 前，不接入默认 CLI/TUI、
  Electron sidecar 或 npm production package，不创建假 Cloud endpoint。

退出条件：安全评审、服务端合同、真实 authenticated remote Gate B 和审计 evidence 全部具备。

## 8. 治理与删除

- [ ] 每个批次刷新 TUI/CLI structure、snapshot、test inventory，并把新增差异分类为
  `current/compat/deprecated/dead`。
- [ ] 零消费者后删除顶层 `tui/src/reconnect.rs`；旧路径只保留 retired guard/negative test。
- [ ] 对 `lime-cli`、`terminal-ui`、`TerminalGuard`、`TuiTerminal`、旧 runtime、旧
  crossterm registry、旧 launcher 命名增加结构负向测试。
- [ ] 若发现跨命令组 legacy policy/mock residual，登记 `tech-debt-tracker.md` 的 `CCD-012`，
  不在本计划中新增兼容层。

## 9. 验证门禁

每个阶段按最小边界验证，阶段收尾再扩大：

```bash
cargo test --locked -p tui -p cli
cargo clippy --locked -p tui --no-deps -- -D warnings
cargo clippy --locked -p cli --no-deps -- -D warnings
npx vitest run scripts/app-server/tui-structure-inventory.test.mjs
npx vitest run scripts/app-server/tui-snapshot-inventory.test.mjs
npm run test:contracts
npm run smoke:cli-gate-b
npm run smoke:tui-gate-b
npm run governance:legacy-report
npm --prefix packages/cli test
git diff --check
```

涉及协议/锁文件时追加 `cargo metadata --locked`、相关 App Server crate 测试和
`npm run test:rust:related -- <paths...>`；涉及真实 TUI 生命周期时必须追加 PTY、alternate
screen、键盘输入和终端恢复证据；涉及 Cloud 时必须追加服务端合同和 authenticated remote Gate B。

## 10. 完成定义

- TUI structure inventory 中所有 Codex current 能力均有 Lime current owner；聚合 Lime-only
  owner 已拆除或有明确保留理由。
- 802 个 snapshot 全部有 `direct/merge/contract/defer/dead` 分类，direct/merge/contract
  的可交付子集均有对应测试和证据，dead 不进入 current。
- TUI Codex-shaped integration suite 全绿；390 个现有 TUI 测试不回退。
- CLI ledger 不再有 `missing/pending`；partial 逐项收口或有可验证 defer/excluded 记录。
- Desktop 与 CLI/TUI 仍消费同一个 App Server JSON-RPC/RuntimeCore/read model；Cloud 仍是
  authenticated transport 扩展点。
- `lime-cli`、`terminal-ui`、旧 runtime、旧 launcher 和第二套状态机无 current 引用，治理
  扫描通过。

下一执行批次：**A4 startup/session/working-directory 的 Lime App Server contract**，先收口
Codex startup protected-input/session-start 语义，再
回到 A2 remaining history/transcript contract 场景；reconnect 的当前 transport contract
已具备 PTY Gate B 证据。不并行扩张 Cloud，也不复制 Codex 产品专属模块。
