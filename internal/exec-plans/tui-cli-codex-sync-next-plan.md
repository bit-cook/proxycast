# TUI/CLI 继续同步 Codex 执行计划

状态：in-progress（本轮验证完成；总体对齐仍有明确 defer/partial）
日期：2026-09-08
参考实现：`/Users/coso/Documents/dev/rust/codex`
当前基线：Rust commit `c4017a87aacc7558002b7cb510025e967c1d765e`（参考目录当前 checkout）

本计划承接 [全量差异报告](./codex-lime-tui-cli-difference-report.md)，目标是继续按
Codex 的真实目录、文件、公开类型、函数名和测试名同步 Lime 的 TUI/CLI。禁止根据截图
或主观设计补造同名壳；每个实现批次必须先读取对应 Codex 源文件和测试，再迁移到 Lime
current owner。

## 1. 基线与完成目标

当前事实：

| 维度 | Codex | Lime | 当前结论 |
| --- | ---: | ---: | --- |
| TUI Rust 文件 | 704 | 138 | 目录体系仍未同构 |
| TUI 类型/函数符号 | 13,120 | 2,188 | 结构差异大，须按 owner 分批收敛 |
| TUI snapshot | 991 | 0 | 已建立逐项分类账本，未迁入快照文件 |
| CLI Rust 文件 | 82 | 11 | current CLI 较薄，产品专属文件不机械复制 |
| execpolicy 文件 | 17 | 17 | 已同构 |
| CLI 测试 | 467 | - | 50 covered、89 partial、81 Cloud deferred、247 excluded |

完成目标不是让文件数量形式上相等，而是：

1. Codex current TUI 行为都有明确 Lime owner、测试和分类；可迁移的目录/符号/测试使用
   Codex 原名，不能继续由 Lime 聚合文件隐式承载。
2. TUI 的 direct/merge 场景具备稳定的 Lime snapshot 或等价 TestBackend/VT100 断言；
   contract 场景具备真实 App Server JSON-RPC evidence。
3. CLI 的 89 个 partial 要么补齐 current contract 和真实 Gate B，要么记录明确的
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
- [ ] 对 `markdown_render`、`diff_render`、OSC 8 和宽字符边界补窄终端、UTF-8 grapheme、
  表格、重排回归；`wrapping` 已完成 Codex range/projection 与 URL-aware 行为回归。
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
已通过 `cargo test -p tui`、TUI Clippy、结构 inventory 与 snapshot inventory 守卫。

本轮 wrapping 收口：`lime-rs/crates/tui/src/wrapping.rs` 已迁入 Codex 同名
`ProjectedText`、`project_halfwidth_sound_marks`、`source_offset`、`break_projected_words`、
`wrap_projected_ranges`、`borrowed_slice_range`、`map_owned_wrapped_line_to_range`、
`word_wrap_flattened_line`、`MixedUrlWord`、`mixed_url_wrap_ranges` 和完整 URL 识别 helper；
`bottom_pane/textarea/wrapping_tests.rs` 与 wrapping inline tests 共保留 Codex 测试名 61 个，
覆盖 trailing-space/sentinel、owned penalty、CRLF、缩进 source mapping、halfwidth sound mark、
URL-only 与 mixed URL/prose。相关过滤测试 61/61、TUI Clippy 和 fmt 均通过。

本轮 textarea hyperlink 收口：按 Codex `bottom_pane/textarea/hyperlinks.rs` 建立同名
`HyperlinkCache` owner，并把 URL 扫描、换行行偏移缓存和可见行 OSC 8 重映射接入
`TextArea` 的 wrap cache。缓存随文本替换/插入/删除失效，滚动只标记当前可见行；不把
OSC 8 控制序列写入 canonical draft，也不复制第二套 hyperlink parser。新增同名测试覆盖
跨行 URL、滚动后的目的地保留、emoji grapheme、超长 URL fail-closed、文本变更失效和
重绘复用；`bottom_pane/textarea/hyperlinks.rs`、`hyperlinks_tests.rs` 已纳入结构 inventory。

相关验证：textarea hyperlink 定向测试 12/12、TUI library 586/586、TUI Clippy、workspace
fmt check、结构 inventory 14/14 和 `git diff --check` 通过。

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
- [x] 对齐 Codex `/export` 的 destination/selection popup 与 filename prompt：无参数命令先
  进入复制到剪贴板/保存到文件选择，保存路径使用线程 ID 默认文件名并复用 canonical
  transcript、clipboard 和 noclobber 写入；直接带路径的 `/export <path>` 继续走现有 current
  写入路径。
- [x] 建立 Codex-shaped `app/history_pagination.rs` 的 cursor/loading/去重状态，并接入
  App Server `thread/items/list` contract；legacy thread 继续由 `thread/read` hydrate，
  paginated thread 使用 `thread/resume(excludeTurns=true)` 后按 `nextCursor` 加载 older
  items。TUI 不持有第二套 history store。
- [x] 对齐 Codex `paginated_resume_backwards_cursors`：canonical ThreadStore 的 turn/item
  page 始终从首行生成 inclusive `backwardsCursor`，metadata-only `thread/resume` 返回稳定
  turn/item head cursor，重复 resume 保持相同 cursor，cursor 可重新读取最新 canonical
  Turn/Item。TUI `advancing_cursor` 同步为 Codex 的重复 cursor 截止语义。
- [x] 对齐 transcript pager 的 bounded reflow：手动滚动在前置历史和终端宽度变化后保持
  逻辑行锚点，底部 pinned 状态继续跟随最新尾部；实现位于 `pager_overlay.rs`，不引入
  第二套 transcript/history store。
- [ ] 迁移 `merge=702` 中与 history/transcript/pager 相关场景；跨 runtime 的 136 个
  `contract` 场景必须绑定 App Server fixture 和 canonical identity。

当前剩余：分页 persisted-history hydration 的 TUI current loader 已落地并完成本地投影回归；
older-page 的跨页 nested-review reconciliation 已接入已有 `thread/turns/list` contract，
仍需补充完整 history/transcript contract 证据；不得用本地 DB、缺失的 Turn status 字段或伪造字段补齐。review prompt filtering
已在 TUI 唯一 `history_filter.rs` owner 中按 Codex canonical Turn/ThreadItem 规则落地：完整
Thread hydration 隐藏 review 区间内的 UserMessage，并隐藏前一 completed review turn 后、
当前未完成 interrupted turn 中完全重复的双 prompt；实时 ItemStarted/ItemCompleted 与
TurnCompleted canonical repair 继承 review 边界状态，普通用户消息保持可见。分页
`thread/items/list` 在有 Turn metadata 时按 canonical UserMessage ID 做跨页过滤；旧 App Server
缺少 `thread/turns/list` 或目标页缺少可判定的前序 Turn 时保持 item-only、fail-closed。MCP 摘要已在唯一 projection.rs owner 中对齐可由 v2 canonical 字段
证明的内容类型计数、structured content、截断/可取回标记、错误与耗时，并覆盖未知块
fail-closed 回归及五语言 detail 文案；Codex 专用 rmcp 媒体解码、资源正文、Node/CUA REPL
与相邻 computer activity 仍因 Lime canonical 不保留原始 wire 内容而标为 contract/defer。
Web Search action detail 已在唯一 projection.rs owner 中对齐：typed Search/OpenPage/FindInPage
字段按 Codex 语义展示，多 query 使用首项省略标记，未知、字符串或 malformed action 均回退
canonical query；新增 projection 与 entry-boundary 回归，不扩展 v2 协议。
export destination/selection popup 与 filename prompt 已完成 current 迁移。历史分页已具备
current 最小路径，public App Server JSON-RPC 已覆盖 resume/head cursor/turn-item 重读；真实
stdio fixture 与 PTY/alternate-screen Gate B 已通过，剩余是 history/transcript contract 场景扩大。

本轮历史完成边界补充（2026-09-13）：新增 `app/history_completion.rs`，按 Codex
`group_completed_turn_items` 语义把跨页 item 分组，并仅为包含 completed turn 最后 item 的
分组生成 completion metadata；`Failed`、`Interrupted`、`InProgress` 不生成边界。完成元数据
由 `ConversationProjection` 独立保存，完整 Thread hydrate、实时 `TurnCompleted` 和旧页加载
均可接入现有 `FinalMessageSeparator`；transcript/pager 渲染时插入分隔线，导出仍只消费
canonical `TranscriptEntry`；耗时标签已覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR`。
旧 App Server 无法提供 `thread/turns/list` 时保持 item-only
投影并 fail-closed。Codex runtime metrics、完成时间本地化和跨页 nested-review 状态仍为
`partial/contract/defer`，没有伪造字段或第二套 history store。

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
  Lime 的图片/队列 canonical contract，不复制 Codex 私有 provider/auth。当前
  `attachment_state` 已覆盖本地图片、远程 `UserInput::Image`、上下选择、删除、统一编号、
  提交、队列和无损队列编辑；其余 composer 子状态仍待拆分。未选中的远程图片不会被空
  Backspace 隐式删除，必须先通过上下键选中后再删除。
- [ ] 按 Codex `bottom_pane` 补齐 action banner、footer、selection、file search、skills、
  hooks、MCP elicitation 和 unified exec 的可用子集；没有 App Server consumer 的动态工具
  继续 reject/fail closed。
- [ ] 将 Lime `command_popup.rs`、`pending_input_preview.rs`、`status_indicator.rs`、
  `settings.rs` 迁移到对应 Codex owner，迁移后删除重复聚合实现。

本轮 A3 composer/TextArea 对齐切片（2026-09-13）已建立 `textarea/vim.rs` 作为 Lime TextArea
唯一 Vim owner，并以 `vim_commands_tests.rs` 覆盖 Insert/Normal/Replace、grapheme 移动删除、
operator pending、替换和模式指示器。`ChatComposer` 通过 `/vim` 本地命令接入开关，App 继续
把输入交给同一 composer/TextArea；活动回合中 Insert/Replace/operator pending 的 Escape
优先退出编辑状态，Normal 模式 Escape 才产生中断，Normal 模式 Up/Down 不再劫持历史导航。
footer 复用 composer 的模式指示器，并在窄终端通过统一截断保持边界；五语言 `/vim` 描述和
启停状态文案已补齐。`SlashCommand::ALL`、popup、App 本地命令和 canonical projection status
均已同步，未引入第二套 runtime、history store 或生产 mock。

本切片的 Codex 对齐范围是 current 最小核心，不等同于完整 Vim 同构：
`RuntimeKeymap`/配置化 toggle、search、text object、find/till、dot repeat、完整 replace
recovery、search highlight/overlay 和完整 ChatWidget/footer 行为仍为 `partial/defer`；visual-wrap
preferred-column 已在后续切片收口为 `current`。后续只有
在读取对应 Codex owner 与测试后才继续迁移，不以当前硬编码按键集合冒充完成。

本轮 VimHistory 切片（2026-09-13）已建立 `bottom_pane/chat_composer/vim_history.rs` 唯一
owner：以 Lime composer 的文本、游标、本地图片、远程图片 URL 和选择状态组成快照，按
Codex 的 pending transaction 语义把 Normal 命令、Insert/Replace 会话、普通粘贴和附件变更
合并为一个可撤销编辑；undo 使用 `u`，redo 使用 `Ctrl-R`，历史限制为 64 步和 1 MiB，查询
输入、移动、模式切换和空历史操作不会污染历史。`vim_history_tests.rs` 覆盖文本、Unicode
Insert、附件、远程图片删除和空历史；完整 RuntimeKeymap、dot repeat、text object、find/till
与更完整附件原子事务仍保持 `partial/defer`。

本轮反向历史搜索 Enter 语义（2026-09-13）按 Codex `history_search` 对齐：命中预览后 Enter
只接受预览并退出搜索，保留文本作为可继续编辑的草稿，不直接创建 turn；接受动作同时清空
旧 Vim 撤销栈，使后续 `u` 不会回退到搜索前草稿。新增
`ctrl_r_search_accepts_a_match_with_enter` 与
`accepted_reverse_history_preview_starts_a_fresh_vim_edit_history` 回归；可配置 keymap、
异步 persistent history、跨后端历史查询与完整 ChatWidget/footer 行为仍保持
`partial/defer`。
无匹配时 Enter 保持搜索会话和原草稿，允许继续编辑查询；新增
`history_search_no_match_enter_keeps_search_open_for_query_edits` 回归。
本轮 history-search 可见契约补充（2026-09-13）：Lime `ChatComposer` 复用 Codex 的单一
历史搜索预览状态，新增大小写无关且 UTF-8 安全的 `match_ranges`，仅在命中预览期间把
查询匹配范围作为 render-only 反色粗体样式传给 `TextArea`；Enter 接受后搜索状态清除，
高亮随之消失。footer 搜索提示新增查询末尾光标定位，使用显示宽度计算并在窄终端内钳位，
不改变 canonical draft 或历史存储。新增 `history_search_highlights_preview_until_accepted`、
`history_search_preview_highlights_matches_until_accepted`、
`history_search_footer_cursor_tracks_query_and_clamps_to_narrow_width` 与 Unicode/大小写
匹配范围回归。
本轮文件搜索对齐（2026-09-13）：Lime `ChatComposer` 已建立唯一 `@` 文件补全 owner，
通过 `AppServerSession::fuzzy_file_search_request` 调用真实 `fuzzyFileSearch` JSON-RPC，
使用绝对 cwd、App Server roots 和稳定 cancellation token；runtime 以 generation/query
过滤旧结果并对相同 query 去重。文件 popup 对齐 Codex 的最多 8 条结果、循环上下选择、
`Ctrl-P`/`Ctrl-N` 导航、loading/no-match、Esc dismissal、Enter/Tab 补全和分隔空格处理；
空 `@` 只显示 idle 提示，不发搜索请求，命令 popup 与文件 popup 保持互斥。新增 stale
结果、UTF-8 token 边界、嵌入/重复 `@`、空 query、补全和窄终端渲染回归；未引入本地
filesystem/mock fallback，真实 App Server/PTY Gate B 与 contracts 均通过。
本轮最终验证：TUI library 706/706、history-search 相关回归 12/12、文件搜索与 token 边界回归、
TUI Clippy `-D warnings`、cargo fmt check、`git diff --check`、`npm run test:contracts`、
结构/snapshot inventory 17/17 与真实 `npm run smoke:tui-gate-b` 均通过；Gate B 证明真实 PTY、
alternate screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、queue-edit、
agents-overview、focus-palette、resize-reflow、reconnect 和 terminal restore。App Server 仍有既有
`lower_turn_start_params`/`lower_runtime_options` dead_code 警告，不影响本轮门禁。

本轮 A3 slash popup 光标语义对齐（2026-09-13）：

- `bottom_pane/chat_composer/slash_input.rs` 新增 Codex-shaped `command_popup_filter_text`，按
  首行命令名与 UTF-8 安全光标边界计算 popup 过滤值；命令带参数时，光标在命令名内仍可恢复
  popup，进入参数区则保持关闭，不改变提交解析或 App Server contract。
- `ChatComposer::sync_command_popup` 与 Esc dismissal 共用该过滤 owner，避免整段文本的空格
  判断造成 popup 丢失；无新增 popup 状态、协议字段、兼容包装或本地 fallback。
- 回归覆盖参数后回到命令前缀、参数区关闭、Esc dismissal 作用域和非边界 UTF-8 光标；该切片
  分类为 `current`，A3 完整 ChatWidget/RuntimeKeymap 仍为 `partial/defer`。
- 已通过 slash-input 4/4、composer 26/26、TUI library 733/733、Clippy `-D warnings`、
  workspace fmt check、结构/snapshot inventory 18/18、`npm run test:contracts`、
  `npm run smoke:tui-gate-b` 与 `git diff --check`。Gate B 证明真实 PTY、alternate screen、
  stdio App Server JSON-RPC、canonical Thread/Turn/Item、queue-edit、agents-overview、
  focus-palette、resize-reflow、reconnect 和 terminal restore；编译期间仍有 App Server 既有
  `lower_turn_start_params`/`lower_runtime_options` dead_code 警告，非本切片引入。

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
  的 Codex 同名 helper 已有 Lime 测试，并在首个无待处理请求的键盘/粘贴事件后释放 boundary，
  避免后续普通会话请求继续被误标为 startup request。`app/startup_prompts.rs` 已承接 Codex 同名的
  `SkillLoadWarningState`、`StartupTooltipOverride`、model migration/availability NUX 纯
  逻辑，并由真实 `skills/list`、`model/list` JSON-RPC 启动请求消费。`cwd_prompt.rs` 已
  承接 Codex 同名 `CwdPromptAction`、`CwdSelection`、`CwdPromptOutcome` 状态机。
  完整 `working_directory`/`/cd` trust/config、fork/replace 和 resume-cwd persistence
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
- [x] 收口 startup protected-input handoff：首个无待处理启动请求的键盘/粘贴事件释放
  `App` boundary；可见 BottomPane 请求仍优先接收解决键，后台线程请求继续只进入其线程
  缓冲区。新增 `startup_boundary_ends_on_the_first_safe_user_input`、
  `first_safe_user_input_releases_boundary_before_reaching_composer` 与
  `startup_boundary_waits_for_visible_request_before_releasing` 回归测试。
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
- [ ] 对 991 个 snapshot 保持逐项账本：`direct` 迁移、`merge` 合并、`contract` 绑定真实
  protocol、`defer` 保留退出条件、`dead` 禁止复制。

当前证据：Codex-shaped 集成目录已建立，VT100 history/live commit、status indicator、
focus palette、resize/reflow 和 reconnect 共 15 个测试通过；TUI library 全量测试在本轮
wrapping owner 迁移后重新验证，terminal
probe 定向测试、TUI integration Clippy、fmt、inventory、治理报告和真实
`smoke:tui-gate-b`（包含 `reconnect=ok`）均通过。`fixtures/oss-story.jsonl` 仍是明确
`defer`，原因是上游当前没有 Rust consumer。

退出条件：TUI 集成目录、snapshot、TestBackend、VT100 history/live commit、status 和
PTY Gate B 同时通过；账本无未分类条目，且测试失败不会回退到 mock backend。

## 6. 阶段 C：CLI 89 个 partial 收口

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
- [x] 将 `packages/cli/bin/lime.js` 的 `codexPackageRoot`、`findCodexExecutable`、
  `isPnpmOwnedCodexInstall` 和 `isVitePlusOwnedCodexInstall` 改为 Lime 语义，并同步
  npm package test 和 structure inventory；launcher current owner 不再暴露 Codex 残留名称。
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

退出条件：89 个 partial 全部变成 `covered`、有记录的 `defer` 或 `excluded`；CLI Gate B、
npm staging/launcher、JSON/JSONL/stdin/exit/signal/completion 和结构 inventory 通过。

## 7. 阶段 D：Cloud transport foundation

仅在阶段 A-C 的本地 current contract 稳定后推进：

- [ ] 完成 authenticated WebSocket transport 的 tenant identity、protocol version、TLS、
  credential lifecycle 和 token redaction contract。
- [ ] 补跨网络 disconnect/reconnect/resume、tenant isolation、rate limit、audit 和
  credential leak negative tests；TUI/CLI 继续复用同一 session facade。
- [ ] 远程 queue、remote plugin catalog、Cloud managed permission profile、remote sandbox
  和 exec-server 测试仍对应 CLI ledger 的 81 个 `cloud-deferred` 条目。
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
- 991 个 snapshot 全部有 `direct/merge/contract/defer/dead` 分类，direct/merge/contract
  的可交付子集均有对应测试和证据，dead 不进入 current。
- TUI Codex-shaped integration suite 全绿；631 个 library、15 个 integration 和 1 个
  manager regression 测试不回退。
- CLI ledger 不再有 `missing/pending`；partial 逐项收口或有可验证 defer/excluded 记录。
- Desktop 与 CLI/TUI 仍消费同一个 App Server JSON-RPC/RuntimeCore/read model；Cloud 仍是
  authenticated transport 扩展点。
- `lime-cli`、`terminal-ui`、旧 runtime、旧 launcher 和第二套状态机无 current 引用，治理
  扫描通过。

本轮收尾验证（2026-09-12）：

- TUI library 586 tests、CLI all-targets 52 tests、TUI integration 15 tests 全部通过。
- TUI clippy `-D warnings`、fmt、diff-check、governance legacy report 全部通过。
- 当前仍未完成的工作以本计划 A3、C1/C2、D 及各阶段 `[ ]` 条目为准；不能标记为 complete。
- 当前分类：TUI/CLI current owner 与测试门禁可用；Codex 私有产品能力为 excluded/dead；
  尚无 Lime current contract 的历史、Cloud、MCP OAuth、marketplace、完整 Vim 和部分
  startup/working-directory 能力为 defer/partial；顶层 `reconnect.rs` 仍是 compat 委托壳。
- 本轮补充：`packages/cli/bin/lime.js` 的 package-root、可执行文件查找和包管理器归属
  helper 已改为 `lime*` 命名；npm launcher 8 个测试通过，CLI structure inventory 已刷新。
- 参考目录更新至 Codex `c4017a87aacc7558002b7cb510025e967c1d765e` 后，CLI 测试账本刷新为
  467 条：`missing/pending=0`，当前 `covered=50`、`partial=89`、`cloud-deferred=81`、
  `product-specific/excluded=247`。新增 exec-server auth/MCP OAuth 测试归 Cloud deferred，
  worktree/update 测试归 product-specific，sandbox TTY 保留 current contract partial。

本轮补充验证（2026-09-13）：

- startup protected-input boundary 已按 Codex 生命周期收口：首个无待处理启动请求的键盘/粘贴
  事件释放 boundary；可见 BottomPane 请求仍优先接收解决键，后台线程请求不阻塞主线程。
- TUI library 597/597、integration 15/15、manager regression 1/1、TUI Gate B（runtime、
  queue-edit、agents-overview、focus-palette、resize-reflow、reconnect、terminal restore）
  全部通过；结构 inventory 15/15、snapshot inventory 2/2、governance legacy report、
  test:contracts 和 `git diff --check` 通过。
- 本次新增与调整属于 `current`；没有新增 `compat` 或 `deprecated` surface。Codex `/cd`
  trust/config、fork/replace、后台 terminal 与 resume-cwd persistence 仍为
  `contract/defer`，不在 TUI 本地伪造。
- 本次补充：`/export` 无参数 destination/selection popup 与 filename prompt 已迁入
  `app/transcript_export.rs::ExportPicker` current owner；TestBackend 覆盖目的地选择、默认
  文件名、保存动作和渲染，TUI library 更新为 597/597。

本轮 A3 Vim composer 补充验证（2026-09-13）：

- TUI library 全量 629/629、TUI integration 15/15、manager dependency regression 1/1
  通过；其中新增 App `/vim` 切换、活动回合 Escape 边界、Normal 模式历史隔离和窄终端
  footer 指示器回归均通过。
- `npx vitest run scripts/app-server/tui-structure-inventory.test.mjs scripts/app-server/tui-snapshot-inventory.test.mjs`
  通过（17/17）；结构清单已登记 `textarea/vim.rs` 与 `vim_commands_tests.rs`。
- `npm run smoke:tui-gate-b` 通过：真实 `lime`、PTY、alternate screen、stdio App Server
  JSON-RPC、canonical Thread/Turn/Item projection、queue-edit、agents-overview、
  focus-palette、resize-reflow、reconnect 和 terminal restore 均有证据。
- 本轮改动分类为 `current`；没有新增 `compat`/`deprecated`。完整 Vim 行为与 Codex 私有
  keymap/search/history 能力继续保持 `partial/defer`，本计划总体仍为 `in-progress`。

本轮 A2 transcript overlay 补充验证（2026-09-13）：

- `pager_overlay.rs` 的 transcript overlay 现在按 Codex 语义保留手动滚动锚点：前置历史时
  按新增逻辑行偏移，终端宽度变化时按重排后的逻辑行首映射；底部 pinned 状态继续跟随
  最新尾部，静态 status pager 不共享该缓存。
- 新增前置历史与窄终端重排 TestBackend 回归；pager overlay 定向测试 8/8，TUI library
  631/631、integration 15/15、manager regression 1/1、fmt、TUI clippy、结构/快照
  inventory 17/17 和 `git diff --check` 均通过。
- `npm run smoke:tui-gate-b` 重新通过：真实 PTY、alternate screen、stdio App Server
  JSON-RPC、canonical Thread/Turn/Item projection、queue-edit、agents-overview、
  focus-palette、resize-reflow、reconnect 与 terminal restore 均有证据。
- 本轮改动属于 `current`，没有新增 `compat`/`deprecated`。review prompt filtering 已完成
  current 迁移并由纯过滤器、完整 hydration、实时边界与 canonical repair 回归覆盖；分页跨页
  nested-review reconciliation、MCP/file-activity details 与剩余 history/transcript contract
  场景仍为 `defer`/未完成。

本轮 A2 persisted-history hydration 补充（2026-09-13）：

- `thread_transcript.rs` 现在先通过只读 `thread/read(includeTurns=false)` 获取 canonical Thread
  metadata，避免 resume picker 预览触发 `thread/resume` 的队列唤醒或归档拒绝。Legacy 线程
  继续通过 `thread/read(includeTurns=true)` 获取完整 Turn/Item；Paginated 线程则从
  `thread/items/list(cursor=None)` 按 descending 页面读取全部 Item，按服务端时间顺序重组后
  复用 `ConversationProjection`，resume picker preview 与完整 transcript 共用同一 loader。
- 分页后续 nextCursor 通过唯一 cursor 集合 fail-closed；新增多页顺序与重复 cursor 回归，不创建
  本地 history store，不伪造 Turn status。分页页面缺失 Turn status 时，跨页 nested-review 过滤继续
  明确 defer。归档线程保留只读 `thread/read` 路径。
- TUI library 643/643、TUI Clippy `-D warnings`、workspace fmt、`git diff --check` 已通过；
  结构/snapshot inventory 17/17 与真实 `npm run smoke:tui-gate-b` 均通过。Gate B 线程为
  `01a097e1-afaf-7012-b734-f52b83ba6213`，回合为 `turn_38d084cfd5c7457bb3949b0c330ec14d`。

本轮 A3 Ctrl-C composer 语义补充（2026-09-13）：

- 对照 Codex `ChatComposer::clear_for_ctrl_c`、`BottomPane::on_ctrl_c` 与
  `ChatWidget::on_ctrl_c`，Lime `ChatComposer` 新增纯文本草稿清理 owner：Ctrl-C 会清空草稿、
  写入本地文本历史并重置历史导航；空闲状态可立即通过 Up 恢复，活动回合仍返回
  `AppAction::Interrupt`，不改变 RuntimeCore/App Server 中断主链。
- 含本地或远程图片的草稿暂不清空，保持 fail-closed，避免当前 Lime 纯文本历史伪造或
  丢失 canonical 附件；该富历史能力继续分类为 `partial/defer`，没有新增 compat/deprecated
  surface，也没有复制 Codex rollout/history DB。
- App connected input 回归覆盖 idle plain-text draft、active-turn draft 和 attachment draft；
  ChatComposer 回归覆盖 Ctrl-C 清空、Up 恢复和附件保留。
- 本轮改动仍保持 `TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical Thread/Turn/Item`
  单主链；断开态 Ctrl-C 的现有退出语义未混入本刀。

本轮 A3 MCP elicitation approval 补充（2026-09-13）：

- `bottom_pane/mcp_server_elicitation.rs` 现在按 Codex 的 message-only form 语义处理合法空对象
  schema：普通空 schema 进入 Allow / Deny / Cancel 选择面，`_meta.persist` 暴露 session / always
  选项，接受动作发送带空对象 `content` 的 typed v2 response；所有 action/content/meta 组合继续
  通过 App Server `McpServerElicitationRequestResponse::validate`。
- `_meta.codex_approval_kind=tool_suggestion` 仍 fail closed，因为 Lime 没有当前 tool
  suggestion enable/install consumer；不伪造 Codex 私有插件/connector side effect，也不新增
  compat/fallback。MCP 表单字段（string、boolean、single-select）保持原有 current owner。
- 五语言 approval option 文案已补齐；BottomPane 回归覆盖空 schema accept、decline、持久化
  accept，以及 tool-suggestion 负向守卫。
- 验证：MCP elicitation 定向测试 9/9，BottomPane unsupported-request 回归 1/1；完整 TUI
  library 669/669、integration 15/15、manager regression 1/1，TUI Clippy、workspace fmt、
  结构/snapshot inventory 和 `git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 仍覆盖
  PTY/alternate-screen/stdio/App Server 主链；当前 Gate B 未包含真实 MCP server elicitation，
  该跨层 fixture 保持下一步 contract 任务，不以单元测试冒充。

下一执行批次：**A2 remaining history/transcript contract**，优先补齐 MCP/file-activity details、
剩余 history/transcript contract 与具备 Turn status contract 后的分页跨页 nested-review reconciliation；
startup protected-input、
reconnect、resize 和 working-directory 的可用 current 子集已具备真实 PTY/App Server 证据。
不并行扩张 Cloud，也不复制 Codex 产品专属模块。

本轮 Gate B 诊断与收口（2026-09-13）：

- `queue-edit` 单场景曾在等待 `interrupted` 时超时；只读 PTY 输出确认 App Server 已发送
  `thread/status/changed` 与 `turn/completed`，且 Ratatui 已把头部从 `interrupting` 更新为
  `interrupted`。失败根因是测试 helper 仅剥离 ANSI 后串接增量写入，无法从差分片段（例如
  只写入 `ed `）重建当前屏幕文本，不是客户端通知反序列化或 projection 路由缺陷。
- `runtime_pty_tests.rs` 的状态等待改为复用现有 `vt100::Parser` 屏幕投影
  `wait_for_screen_marker`；移除临时子进程/接收诊断。增量历史文本标记仍保留原 helper，避免
  改变其它场景的等待语义。
- 验证：`LIME_TUI_GATE_B_SCENARIOS=queue-edit npm run smoke:tui-gate-b`、
  `npm run smoke:tui-gate-b`、`cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui`
  （653 library、15 integration、1 manager regression）、
  `cargo clippy --locked --manifest-path lime-rs/Cargo.toml -p tui --no-deps -- -D warnings`、
  `cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check`、结构/快照 inventory
  （17/17）及 `git diff --check` 均通过。
- 本轮没有新增 current/compat/deprecated surface；Gate B 已恢复为可交付状态。总体计划仍为
  `in-progress`，下一刀继续 A2 remaining history/transcript contract。

本轮 A2 Web Search history detail（2026-09-13）：

- `projection.rs` 复用 `agent_protocol::response_item::WebSearchAction` 的 typed 反序列化，
  把 v2 `WebSearchItem.action` 的 Search/OpenPage/FindInPage detail 转成稳定的 canonical
  transcript text；`Other`、字符串、未知类型和 malformed 字段 fail-closed 回退 `query`，
  不把任意 action JSON 原文带入终端。
- 新增多 query、URL/pattern、空 detail、malformed/unknown fallback 以及五语言 entry-boundary
  文案回归；长文本继续由现有 transcript/pager wrapping owner 处理。该切片属于 `current`，
  没有新增 `compat`/`deprecated`，也不改变 App Server protocol/schema。
- 定向验证：`cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui --lib projection::tests`
  21/21 通过；完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为
  library 656/656、integration 15/15、manager regression 1/1；TUI Clippy、fmt、结构/
  snapshot inventory 17/17、`git diff --check` 与 `npm run smoke:tui-gate-b` 均通过。
  Gate B 线程为 `01a0985c-9647-7382-ab8a-08d1f94c13a8`，回合为
  `turn_9b13a4b9a8fa4fc4916901dc143c832f`。

本轮 A2 Web Search lifecycle（2026-09-13）：

- 对照 Codex `history_cell/search.rs` 的 `Searching the web`/`Searched the web` 语义，
  Lime 在唯一 `projection.rs` owner 中仅使用实时 JSON-RPC 生命周期证明状态：
  `ItemStarted(WebSearchItem)` 显示 `searching the web`，`ItemCompleted(WebSearchItem)` 显示
  `searched the web for ...`；无 query/action 时分别显示不带 detail 的状态文案。
- persisted `ThreadItem::WebSearch` 仍走 status-agnostic historical projection，显示稳定的
  `web search: ...`；因为 v2 `WebSearchItem` 不含 status 字段，历史 hydration、分页和导出
  不猜测 running/completed，也不扩展 App Server schema。五语言 entry-boundary detail 前缀
  已补齐，未知 action 继续 fail-closed 回退 canonical query。
- 新增实时 started/completed 替换、历史 status-agnostic 回归，以及 completed turn canonical
  repair 不降级已完成 WebSearch 文案的回归；该切片属于 `current`，没有新增
  `compat`/`deprecated` surface，也没有第二套 history store 或 wire parser。由于 v2
  `WebSearchItem` 不含 status 字段，未伪造历史实时状态；五语言 detail 仍在 entry boundary
  统一本地化。
- 验证：`projection::tests` 27/27；完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui`
  为 680 library、15 integration、1 manager regression；TUI Clippy `-D warnings`、workspace fmt、
  结构/snapshot inventory 17/17 与 `git diff --check` 通过。真实 Gate B
  已通过：thread `01a0993f-e24b-7bf3-8b00-9e3d85e460b5`、turn
  `turn_f81ba7a78ea1422e9e5ec4267b1428f9`；事件为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，queue-edit、
  agents-overview、focus-palette、resize-reflow、reconnect 和 terminal restore 均为 `ok`。
  Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options`
  dead-code 警告，非本切片引入。

本轮 A2 persisted nested-review hydration（2026-09-13）：

- `thread_transcript.rs` 在 paginated transcript loader 中增加只读
  `thread/turns/list(itemsView=notLoaded)` 分页；turn 与 item 页面均按 descending cursor
  重新组装为临时 canonical `Thread`，再复用 `ConversationProjection::hydrate_thread` 和
  `history_filter::hidden_user_message_ids`。因此跨 item page 的“已完成 review turn + 未完成
  interrupted 子 turn”重复 prompt 现在按 Codex 规则隐藏，且不引入本地 history store。
- turn/item 关联缺失、分页 cursor 重复或旧 App Server 不提供 turn list 时，loader 保留 flat
  item projection，明确 fail-closed，不猜测 Turn status，也不隐藏可能是用户输入的内容。
- 新增跨页 nested-review 和未知 turn metadata 回归；TUI library 658/658、integration
  15/15、manager regression 1/1、TUI Clippy、fmt、结构/snapshot inventory 17/17 与
  `git diff --check` 通过。App Server 分页 contract 定向测试因本机 `rusty_v8 v150.4.0`
  预构建包不存在（下载返回 HTTP 404）未能启动，待工具链缓存/版本恢复后重跑。
- 本切片属于 `current`，没有新增 `compat`/`deprecated`；主 TUI transcript overlay 的
  older-page runtime reconciliation 仍需沿用同一 turn metadata contract，MCP 原始媒体与
  file-activity 富内容继续按 canonical 字段可用性逐项收口。

本轮 A2 MCP canonical summary 补充（2026-09-13）：

- `projection.rs` 继续作为唯一 MCP 投影 owner：对 App Server 已限界的 canonical 结果增加文本
  output preview（首行、最多 160 字符、最多 4 条）、structured content compact JSON、内容类型
  计数、`truncated`/`output available` 标记、错误和耗时；未知 content block 只计为 `unknown`，
  不解码媒体或把任意 wire body 带入 TUI。现有 MCP invocation 标题仍显示紧凑参数 JSON。
- `Locale::detail` 已补齐 `output:` 的 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` 文案；
  projection 回归同步覆盖 output 与 structured content 的完整摘要文本。新增回归锁定多行文本
  只取首行、超过 160 字符追加省略号，以及最多保留 4 条文本 preview。
- 本切片属于 `current`，没有新增 `compat`/`deprecated`。Codex 专用的逐媒体/资源正文渲染、
  更丰富的多行宽度截断和 export-only structured result 仍受 Lime canonical 字段边界限制，
  继续分类为 `contract/defer`，不在 TUI 伪造第二套 wire 解析或历史存储。
- 验证：`cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui`（659 library、15
  integration、1 manager regression）通过；`projection::tests` 22/22、TUI Clippy
  `-D warnings`、workspace fmt check、结构/快照 inventory 17/17、`git diff --check` 和
  `npm run smoke:tui-gate-b` 均通过。Gate B 线程为
  `01a09886-44d2-7532-9380-94f489c13a46`，回合为 `turn_9c171a0a164343cc809dd60470b8708c`。
- 本切片未修改 App Server protocol/schema；App Server contract 全量测试未在本刀重复执行，
  既有环境阻塞仍是 `rusty_v8 v150.4.0` 预构建包下载返回 HTTP 404，待工具链缓存恢复后补跑。

本轮 A2 用户图片 history/export 对齐（2026-09-13）：

- `projection.rs` 继续作为 live `item/completed` 与 persisted Thread hydration 的唯一用户输入
  投影 owner：`UserInput::Image` 和 `UserInput::LocalImage` 只保留图片数量与原始顺序，形成内部
  `image: N` 摘要；远程 URL、data URL 和本地路径均不进入 `TranscriptEntry.text`。文本、skill
  与 mention 仍进入用户消息正文，不扩展 App Server v2 protocol/schema。
- `entry.rs` 在终端边界把内部图片摘要渲染为独立编号行；`Locale::numbered_image_label` 覆盖
  zh-CN `[图片 #N]`、zh-TW `[圖片 #N]`、en-US `[Image #N]`、ja-JP `[画像 #N]` 与
  ko-KR `[이미지 #N]`。图片行不复用普通 activity 的 `- detail` 前缀。
- `app/transcript_export.rs` 按同一 canonical 摘要把图片编号写入 Markdown 用户消息正文；
  纯图片输入仍保留 `## User` 段，且不导出内部 `image: N` token、图片 URI、data URL 或
  本地路径。resume picker 与 Codex 一致保持 text-only preview，不在缺少 canonical 文本时
  伪造图片预览。
- 本切片属于 `current`，没有新增 `compat` 或 `deprecated` surface。MCP 原始媒体/资源正文、
  相邻 `ComputerActivityCell` 聚合和更丰富的 file-activity detail 仍受 canonical producer/consumer
字段限制，继续分类为 `contract/defer`；不得在 TUI 增加第二套 wire parser、history store 或
  rollout DB。
- 验证：`projection::tests` 23/23、`entry::tests` 6/6、
  `app::transcript_export::tests` 8/8；完整 TUI 为 library 672/672、integration 15/15、
  manager regression 1/1。TUI Clippy `-D warnings`、workspace fmt check、结构/snapshot
  inventory 17/17 与 `git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 通过，线程为
  `01a098cf-3dc6-7f71-b061-c8058f5c9f23`，回合为
  `turn_1b9f9bc4af1a4241bb4166c6a712bb46`；`queue-edit`、`agents-overview`、
  `focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间
  仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，
  非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 MCP result history-cell owner 对齐（2026-09-13）：

- MCP canonical result 摘要从 `projection.rs` 下沉到独立的 `history_cell/mcp_result.rs` owner；投影层只复用该 owner 的 invocation、summary 与 bounded text 事实。摘要仅消费 App Server v2 已限界的 `content`、`structured_content`、`meta` 与 error/duration：文本 preview 取首行、最多 160 字符、最多 4 条，补充内容类型计数、`truncated`/`output available` 标记和紧凑 structured JSON。
- image/audio/resource/未知 content block 不解码、不保留正文或 URI，仅计入类型事实；未知 wire payload、原始媒体/资源正文和更丰富的 export-only 结构继续按 canonical 边界 `contract/defer`，不新增第二套 parser、history store、protocol/schema 或 mock fallback。
- `history_cell/mcp_result.rs` 新增 2 个单元回归，覆盖 bounded text、结构化事实、媒体/资源/未知块 fail-closed 与 preview 截断；结构 inventory 期望同步锁定该 owner。该切片属于 `current`，没有新增 `compat`/`deprecated` surface。
- 验证：MCP owner 定向测试 2/2；`cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为 682 library、15 integration、1 manager regression；TUI Clippy `--lib --no-deps -- -D warnings`、workspace fmt check、结构/snapshot inventory 17/17 与 `git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 通过，thread `01a09963-2f05-7093-9daf-aab827f906f5`、turn `turn_e8d86225a587403bb5c873775f8fdf87`；事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 FileChange history/export 对齐（2026-09-13）：

- ThreadItem::FileChange 继续只消费 App Server v2 canonical status/changes/diff；终端正文保留逐文件路径、rename 目标与 diff，摘要保留 files/add/delete/update 计数，不新增协议字段或本地 patch/history store。
- Markdown export 现在保留仅有摘要的 FileChange activity，并从 canonical EntryStatus 与 files: N 生成 Codex 风格 file changes: <status> · N changes 头部；已有逐文件 diff、ANSI 清理与图片导出语义保持不变。persisted/live transcript 共享同一 projection，不再因正文为空丢弃合法的零变更或失败 patch 条目。
- 本切片属于 current，没有新增 compat/deprecated。MCP 媒体/资源逐项 export 仍需先确认 canonical content producer 的边界，未在 TUI 伪造原始 wire 解码。
- 验证：app::transcript_export::tests 10/10、projection::tests 23/23；完整 TUI library 674/674、integration 15/15、manager regression 1/1；TUI Clippy -D warnings、workspace fmt check、结构/snapshot inventory 17/17、git diff --check 均通过。真实 npm run smoke:tui-gate-b 通过，线程为 01a098e4-f271-7b13-8491-1cc4fbe8e27e，回合为 turn_133013e8f71348748d10bfbb7e951da6；queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和 terminal restore 均为 ok。App Server 既有 lower_turn_start_params、lower_runtime_options dead-code 警告非本切片引入。

本轮 A2 persisted export canonical loader + persisted/live merge（2026-09-13）：

- `runtime.rs::export_transcript_with` 改为异步复用现有 `thread_transcript::load_session_transcript_with_handle`，优先读取 App Server persisted Thread/Turn/Item；加载失败、缺少 thread id 或请求句柄不可用时回退当前 live projection，不新增 history store、wire parser 或 mock fallback。
- `merge_export_entries` 保持 persisted 顺序；同 ID 由 live 最新 entry 覆盖，persisted 中尚未落盘的新 live entry 追加到末尾，避免进行中或尚未持久化内容在 `/export` 中丢失。Markdown export 继续复用既有 canonical renderer、路径 noclobber 与 clipboard 边界。
- 新增 `export_merge_keeps_persisted_order_and_latest_live_entries` 回归，锁定同 ID 覆盖、顺序保持和新条目追加；该切片属于 `current`，没有新增 `compat`/`deprecated` surface。
- 验证：真实 `npm run smoke:tui-gate-b` 通过，线程为 `01a098ff-ad6b-7be3-a654-7e2895c032ab`，回合为 `turn_28627348942544eda6752a11456169d8`；`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。`cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui`（675 library、15 integration、1 manager regression）、TUI Clippy `-D warnings`、workspace fmt check、结构/snapshot inventory 17/17 与 `git diff --check` 均通过；Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告。总体计划继续保持 `in-progress`。

本轮 A2 DynamicToolCall history/transcript bounded summary（2026-09-13）：

- `projection.rs` 继续作为唯一 DynamicToolCall canonical consumer：复用 v2 `DynamicToolCallOutputContentItem`，对 `InputText` 提取首行、最多 160 字符、最多 4 条 `output:` preview；`InputImage`/`InputAudio` 仅保留既有 `content items` 计数，不把 image/audio URL 或媒体正文写入 `TranscriptEntry.summary`。
- Markdown export 继续复用同一 `TranscriptEntry.summary`，因此 persisted/live export 可见 bounded 文本事实，同时保持 Codex/Lime 的媒体 fail-closed 边界；没有扩展 App Server schema、增加 wire parser、history store 或 mock fallback。
- 新增 projection 回归覆盖多行首行选择、160 字符省略、最多 4 条 preview、媒体 URL 不泄露；本切片属于 `current`，没有新增 `compat`/`deprecated` surface。
- 验证：`projection::tests` 24/24；完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为 676 library、15 integration、1 manager regression；TUI Clippy `-D warnings`、workspace fmt check、结构/snapshot inventory 17/17 与 `git diff --check` 通过。真实 `npm run smoke:tui-gate-b` 通过，线程为 `01a0990b-85c2-77e1-bf33-4954a9c00cb6`，回合为 `turn_f834ac084d614cfca0ecfdc17c2cf76a`；`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 ImageView history/display 路径对齐（2026-09-13）：

- `diff_render::display_path_for` 提取为共享路径显示 owner；`entry.rs` 在终端渲染边界识别 `view image: ...`，把当前 cwd 下的绝对路径归一化为相对路径后再交给 Locale 处理，其他 Tool/System 详情保持原样。canonical `TranscriptEntry`、App Server v2 schema、persisted loader 与 Markdown export 均不变。
- 新增 `image_view_paths_are_relative_to_the_current_working_directory` 回归，锁定 `/workspace/assets/result.png` 在 `/workspace` cwd 下显示为 `assets/result.png`，不暴露冗长绝对路径；该切片属于 `current`，没有新增 `compat`/`deprecated` surface。
- 验证：`entry::tests` 7/7、完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为 677 library、15 integration、1 manager regression；TUI Clippy `-D warnings`、workspace fmt check、结构/snapshot inventory 17/17 与 `git diff --check` 通过。真实 `npm run smoke:tui-gate-b` 通过，线程为 `01a09915-8b45-7ce1-9826-a8f81fd1b65b`，回合为 `turn_0f9f3097bc2946e5ad090eacb6ae4766`；`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 Sleep history visibility 对齐（2026-09-13）：

- `projection.rs` 将 v2 `ThreadItem::Sleep` 按 Codex `thread_transcript`/`agent_status_feed` 语义视为内部 runtime control item，直接 fail-closed，不进入 `TranscriptEntry`、transcript overlay、persisted export 或 agent status feed；同时移除唯一无消费者的 `sleep:` locale 前缀。
- 新增 projection 回归锁定 `SleepItem` 不产生 transcript entry；该切片属于 `current`，没有新增 `compat`/`deprecated` surface，不改变 App Server v2 schema 或 runtime 行为。
- 验证：`projection::tests` 24/24、`entry::tests` 7/7；完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为 677 library、15 integration、1 manager regression；TUI Clippy `-D warnings`、workspace fmt check、结构/snapshot inventory 17/17 与 `git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 通过，线程为 `01a0991f-3b10-7e61-80df-768a0c03d28f`，回合为 `turn_8df4cbc170b342a6936f49557afae52d`；`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 ImageGeneration history/export 边界对齐（2026-09-13）：

- `projection.rs` 按 Codex history cell 语义不再把 `ImageGenerationItem.result`（通常为媒体 URL 或 data payload）写入 `TranscriptEntry.summary`；仅保留 canonical `saved_path` 与 `revised_prompt`，状态继续由 `EntryStatus` 映射。这样终端与 Markdown export 不会泄露媒体地址或原始结果正文。
- 新增 projection 回归锁定结果 URL 不进入 summary；该切片属于 `current`，没有新增 `compat`/`deprecated` surface，不改变 App Server v2 schema、运行时生成或保存路径语义。
- 验证：`projection::tests` 24/24、`app::transcript_export::tests` 10/10；完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为 677 library、15 integration、1 manager regression；TUI Clippy `-D warnings`、workspace fmt check、结构/snapshot inventory 17/17 与 `git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 通过，线程为 `01a09923-658c-7181-96a6-de31df43c83c`，回合为 `turn_713b3a2adc204192856841eca424e79a`；`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 Web Search history-cell owner 收敛（2026-09-13）：

- 新增 `history_cell/search.rs` 作为 Codex-shaped Web Search action detail owner；`projection.rs` 不再持有本地解析实现，只复用该 owner。对 v2 `WebSearchAction` typed 解析 `Search`、`OpenPage`、`FindInPage` 和 `Other`，多 query 仅保留首项并以 `...` 标记；未知 action、malformed payload、字符串或不支持字段均 fail-closed 回退 canonical query。
- Started/Completed/Historical 生命周期文案、入口边界多语言映射和 persisted status-agnostic projection 保持不变；未扩展 App Server protocol/schema，也未新增兼容包装或第二套 wire parser。未知 action 字段继续按 `contract/defer` 处理。该切片属于 `current`，没有新增 `compat` 或 `deprecated` surface。
- 新增 Web Search owner 定向回归 2/2，覆盖 typed action detail 与 malformed/unknown fallback；结构 inventory 同步锁定 `history_cell/search.rs`。
- 验证：`projection::tests` 27/27；完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为 684 library、15 integration、1 manager regression；TUI Clippy `--lib --no-deps -- -D warnings`、workspace fmt check、结构/snapshot inventory 17/17 与 `git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 通过，thread `01a09971-fca9-7d82-b67d-7af821174202`、turn `turn_e40bb6dbd06b44cdb6a9c22f6259ee93`；事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 Hook lifecycle transcript/status 对齐（2026-09-13）：

- `projection.rs` 接入 App Server v2 `hook/started` 与 `hook/completed` 通知，维护按 turn 关联的 active Hook 运行状态；单个 Hook 显示 status message 或 `running hook`，多个同消息 Hook 复用该消息，不同消息 fail-closed 为 `running hooks`。Hook completion 会清理对应 active 状态，并将失败、阻断、停止及带用户可见输出的结果投影为 bounded system transcript entry。
- 新增 `history_cell/hook.rs` 作为唯一 Hook display-fact owner：Context-only 成功 Hook 静默，不把 model-facing Context 写入 transcript；非 Context 输出最多 4 条，每条取首行并限制 160 字符。未复制 Codex 私有 HookCell 定时状态机、history DB 或 rollout DB。
- `Locale::status`/`Locale::detail` 已覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` 的 `running hook(s)`、`hook completed/failed/blocked/stopped` 与 `hook output:` 文案；结构 inventory 锁定 Lime `history_cell/hook.rs`，同时保留 Codex `history_cell/hook_cell.rs` 作为上游基线。
- 本切片属于 `current`，没有新增 `compat`/`deprecated`，未修改 App Server protocol/schema。当前 Hook summary 只由实时通知提供，canonical `ThreadItem` 尚不携带 `HookRunSummary`；因此 persisted history/export 不伪造 Hook completion，继续标记为 `contract/defer`，待 canonical producer 提供持久化字段后再收口。
- 验证：Hook owner 定向测试 2/2；`projection::tests` 41/41；Locale 定向测试 21/21；`cargo fmt --manifest-path lime-rs/Cargo.toml --all` 通过；结构 inventory 已刷新为 Codex/Lime 合计 859 个 Rust 文件。完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 通过（691 library、15 integration、1 manager regression）；TUI Clippy `--lib --no-deps -- -D warnings`、结构/snapshot Vitest 17/17、`git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 通过，thread `01a0998c-08a0-7fc0-8228-2b09c975422e`、turn `turn_90b335d8fc1443c6934f358cb964e895`；事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 MessagePhase final-answer 语义修复（2026-09-13）：

- `ConversationProjection` 在唯一 TUI canonical projection owner 内维护 assistant item 的 `MessagePhase` 索引。`Commentary` 仍进入 transcript 供用户查看，但不再被 `final_answer()` 误选；`FinalAnswer` 作为最终回答，`phase=None` 继续保留 legacy 行为。hydrate、prepend、实时 `ItemStarted`/`ItemCompleted` 与 `TurnCompleted` canonical repair 均同步 phase，hydrate 时清理索引避免跨线程残留。
- 不扩展 `TranscriptEntry`、App Server protocol/schema 或新增过滤投影；resume preview、transcript overlay 与 export 继续复用同一 canonical transcript。该切片属于 `current`，没有新增 `compat`/`deprecated` surface。
- 新增回归覆盖 Commentary 可见但非最终回答、FinalAnswer 覆盖旧 Commentary、legacy 无 phase、实时流式 phase 修复和 persisted hydrate。
- 验证：`projection::tests` 37/37；完整 `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 通过（696 library、15 integration、1 manager regression）；TUI Clippy `--lib --no-deps -- -D warnings`、`cargo fmt --check`、结构/snapshot Vitest 17/17、`git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 通过，thread `01a099ab-da21-75a3-ab59-ddbf93c5edb4`、turn `turn_1d5d76ce99284a60b01c91a004af0a34`；事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有 `lower_turn_start_params`、`lower_runtime_options` dead-code 警告，非本切片引入。总体计划继续保持 `in-progress`。

本轮 A3 ComposerDraft 共享快照对齐（2026-09-13）：

- `bottom_pane/chat_composer/draft_state.rs` 建立 Lime 唯一 `ComposerDraft` owner，统一保存文本、UTF-8 安全游标以及本地/远程图片附件状态；快照字节预算与 Vim undo/redo 共用同一实现，避免历史搜索、历史导航和 Vim 各自维护重复快照。
- `history_search.rs` 的 `HistorySearchState` 改为保存完整 `ComposerDraft`。搜索取消、断线编辑取消预览和无匹配恢复均通过同一 `restore_draft`，因此会保留原始游标与附件；历史导航的 unsent draft 也改为同一快照。未引入 Codex 私有 text elements、mention、paste 或 rollout/history DB 字段，缺失能力继续是 `partial/defer`。
- `ChatComposer` 只保留一个 capture/restore 边界；Vim history 复用该边界并继续把文本/附件编辑作为单一事务。footer 与 command popup 在恢复后同步，避免取消搜索残留 HistorySearch 状态。
- 本切片属于 `current`，没有新增 `compat`/`deprecated` surface，也未修改 App Server protocol/schema；生产链仍为 `TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical Thread/Turn/Item`。
- 新增回归覆盖历史搜索取消后游标、附件和 footer 草稿状态保持不变；既有 Vim 文本、Unicode、附件、远程图片、历史搜索和断线恢复测试继续通过。
- 验证：composer 定向测试 43/43；完整 `cargo test --locked --manifest-path "lime-rs/Cargo.toml" -p tui --lib` 697/697；TUI Clippy `--lib --no-deps -- -D warnings`、`cargo fmt --manifest-path "lime-rs/Cargo.toml" --all -- --check` 与 `git diff --check` 通过。总体计划继续保持 `in-progress`。

本轮 A3 Skills `$` mention popup 对齐（2026-09-13）：

- `bottom_pane/chat_composer/skill_popup.rs` 建立 Codex-shaped current owner：消费 App Server
  `skills/list` 返回的 enabled `SkillMetadata`，支持大小写无关的子序列筛选、稳定排序、最多
  8 行、Up/Down 与 Ctrl-P/Ctrl-N 循环选择、Enter/Tab 补全、Esc dismissal 和窄终端截断。
  Popup 只负责展示与选择，技能路径仍由 canonical `SkillMetadata.path` 提供，不读取本地
  skill 目录或创建第二份缓存。
- `app/startup.rs` 将真实 `skills/list` 响应注入 `ChatComposer`；`ChatComposer` 维护唯一
  `$` token range/dismissal 状态，并与现有 slash/`@` file popup 互斥。补全写回 `$name` 和
  分隔空格，重复 token dismissal 按 occurrence 区分；`view.rs`/App event routing 同步接入。
- runtime 提交路径按已加载 catalog 为精确 `$name` 追加 `UserInput::Skill { name, path }`，
  保留原始 prompt 文本和图片顺序；queue edit/preview 接受 Skill part 并恢复为可编辑 `$name`
  前缀。未匹配的 shell 变量、未知 `$token` 和空 skills catalog 不会制造 Skill input。
- 本切片属于 `current`，无新增 `compat`/`deprecated`。Codex 私有 connectors/plugins、完整
  atomic text-element binding、skills 管理 UI 与 marketplace 仍无 Lime current consumer，继续
  分类为 `contract/defer` 或 `excluded`，没有伪造协议字段、local history store 或 mock fallback。
- 验证：skill popup/composer 定向测试 4/4，submission lowering 定向测试 1/1；完整 TUI
  library 708/708、TUI Clippy `--lib --no-deps -- -D warnings`、workspace fmt check、
  TUI structure/snapshot inventory 18/18 与 `git diff --check` 通过。App Server protocol/schema
  未修改，Gate B 需在下一轮真实启动后补跑；总体计划继续保持 `in-progress`。

本轮 A3 Skills `$` current 收口与 catalog refresh（2026-09-13）：

- 复核并补齐 `skill_popup` 的 Ctrl-P/Ctrl-N、shell `$HOME`/`$1` fail-closed 与 UTF-8 token
  边界回归；skill popup/composer 定向测试实际为 6/6。完整 TUI library 测试为 716/716，
  没有引入 provider 侧重复 Skill 文本注入；`UserInput::Skill` 继续由 App Server/runtime
  的结构化输入 owner 消费，prompt 文本仅作为用户可见原文保留。
- 对齐 Codex `SkillsChanged` 行为：`app_server_events.rs` 收到真实 App Server
  `skills/changed` 通知后调用 `skills/list(forceReload=true)`，通过 `startup::apply_skills_list_response`
  原子更新 composer enabled catalog 与 skill load warning；启动和运行时刷新共用同一投影
  helper，没有本地 skills store、mock fallback 或第二套 runtime。新增回归锁定 disabled skill
  不进入 `$` 补全目录。
- 该切片属于 `current`，未修改 App Server protocol/schema，也未新增 `compat`/`deprecated`。
  Codex 私有 atomic text-element binding、skills 管理 UI、marketplace 与 plugin mention
  继续分类为 `contract/defer` 或 `excluded`。
- 验证：`cargo test --locked --manifest-path "lime-rs/Cargo.toml" -p tui --lib` 716/716；
  `cargo clippy --locked --manifest-path "lime-rs/Cargo.toml" -p tui --lib --no-deps -- -D warnings`；
  `npm run inventory:tui-structure`、结构/snapshot Vitest 18/18、`npm run test:contracts`、
  `npm run smoke:tui-gate-b` 与 `git diff --check` 均通过。Gate B 证明真实 PTY、alternate
  screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、queue-edit、agents-overview、
  focus-palette、resize-reflow、reconnect 和 terminal restore；编译期间仍有既有
  `lower_turn_start_params`/`lower_runtime_options` dead-code 警告，非本切片引入。总体计划
  继续保持 `in-progress`。

本轮 A2 CUA MCP display facts（2026-09-13）：

- 新增 `history_cell/computer_activity.rs` 作为 CUA-backed MCP 的唯一 display-fact owner。
  对 canonical `server == "cua_repl"` 的 `McpToolCall`，保留参数中的有界 `title`、截图块数量
  和首行失败诊断；原始图片/音频/资源正文、provider 手册和跨 item 相邻调用均不进入
  `TranscriptEntry`，不新增协议字段、wire parser 或 history store。
- `projection.rs` 继续保持每个 canonical MCP item 一个 entry，并在既有 MCP result summary
  后追加 CUA facts；调用 ID、server/tool/arguments 和状态仍由原有 projection 保留，待协议
  具备稳定分组/Turn 边界后再评估 Codex `ComputerActivityCell` 的相邻聚合。
- `Locale::detail` 补齐 computer action/error/screenshot 的 `zh-CN`、`zh-TW`、`en-US`、
  `ja-JP`、`ko-KR` 映射。该切片属于 `current`，没有新增 `compat`/`deprecated` surface。
- 定向回归覆盖截图事实和 provider 错误首行截断；完整 TUI 为 `718` library、`15` integration、
  `1` manager regression，Clippy `-D warnings`、workspace fmt check、结构/snapshot inventory
  `18/18`、`npm run test:contracts`、真实 `npm run smoke:tui-gate-b` 与 `git diff --check` 均通过。
  Gate B 证明真实 PTY、alternate screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、
  queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和 terminal restore；编译
  期间仍有 App Server 既有 `lower_turn_start_params`/`lower_runtime_options` dead-code 警告，
  非本切片引入。总体计划继续保持 `in-progress`。

本轮 A2 MCP startup diagnostics（2026-09-13）：

- `app/startup_prompts.rs` 新增 `McpStartupWarningState`，消费 App Server v2
  `McpServerStatusUpdated` canonical notification。Failed/Cancelled 按服务器名去重并保留可选
  错误，Ready 只清除对应服务器；Starting 不制造虚假失败状态。
- App header 与 status pager 通过 `App::status_value()` 复用该状态；active turn、Hook status
  和显式命令状态优先，MCP startup 诊断只在 ready/空闲状态显示。该诊断不写入
  `TranscriptEntry`、不新增 history store，也不改变 protocol/schema。
- `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` 的 status 前缀和 server/error 动态尾部均有回归；
  App-scoped notification 的失败、Ready 清理和状态优先级均有测试。该切片属于 `current`，
  没有新增 `compat`/`deprecated` surface。
- 验证已完成：`cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 为
  `721 library + 15 integration + 1 manager regression`，TUI Clippy `--lib --no-deps -- -D warnings`、
  workspace `cargo fmt --check`、`npm run inventory:tui-structure`、结构/snapshot Vitest
  `18/18`、`npm run test:contracts`、`npm run smoke:tui-gate-b` 和 `git diff --check` 均通过。
  Gate B 真实线程为 `01a09a70-d2fc-7601-80b9-423c40c4abc3`，回合为
  `turn_494f9631329e489a9394d5dacf733fa0`，事件为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，并证明
  `queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 与
  terminal restore 均为 `ok`。Gate B 编译期间仍有 App Server 既有
  `lower_turn_start_params`/`lower_runtime_options` dead-code warning，非本切片引入。

本轮 A3 ChatComposer 历史导航与 visual-wrap 边界补充（2026-09-13）：

- 对照 Codex `ChatComposerHistory::should_handle_navigation`，Lime 增加最近召回文本状态：
  空草稿可开始历史，非空草稿仅在精确匹配最近召回文本且光标位于边界时消费 Up/Down。
- 单行长文本在已知 visual-wrap 宽度下，仅首/尾 visual row 进入历史；中间行交给 TextArea，
  并补齐插入模式箭头键到同一 visual vertical owner。
- 旧的任意非空草稿历史替换回归改为 Codex 语义，并新增
  `wrapped_single_line_vertical_navigation_stays_in_editor_until_visual_boundary`。
- TUI library `749/749`、TUI Clippy、相关 rustfmt 与 `git diff --check` 通过；workspace fmt
  仍被其他既有 agent-runtime/tool-runtime debug 输出格式差异阻断。此前 integration、contracts
  与 TUI Gate B 证据保持有效；本切片未触达协议、Electron 或生产 mock。
  总体计划继续保持 `in-progress`，A2 history/transcript contract、A3 剩余 owner、CLI
  partial 与 Cloud transport 仍未完成。

本轮 A2 CUA title boundedness 修复（2026-09-13）：

- `history_cell/computer_activity.rs` 的 CUA action title 现在复用 canonical `compact_text`
  规则，仅保留首行、最多 160 个字符并追加省略号；避免 provider 手册或超长多行 title
  直接进入 `TranscriptEntry.summary`。截图计数、错误首行和媒体 fail-closed 边界保持不变。
- 新增长标题/多行 title 回归；定向 CUA 测试 `3/3`、TUI Clippy、workspace fmt 和
  `git diff --check` 通过。该修复属于既有 `current` owner，没有新增 `compat`/`deprecated`，
  也未修改 App Server protocol/schema。

本轮 A2 history completion boundaries（2026-09-13）：

- 新增 `app/history_completion.rs` 与 Codex 同名的三个边界回归，覆盖跨页 turn、多个完成
  turn 顺序以及失败/中断/运行中 turn 不产生 completion boundary。
- `ConversationProjection` 独立保存 completion metadata；完整 Thread hydrate、实时
  `TurnCompleted` 和旧页加载使用同一 `FinalMessageSeparator` 渲染链，旧页按 item 页的
  `turn_id` 有界查找 Turn（最多 16 页，重复 cursor 截止）。分隔线耗时文案覆盖五种产品
  locale，导出仍只读取 canonical `TranscriptEntry`。
- 验证已完成：TUI library `729`、integration `15`、manager regression `1`；TUI Clippy
  `--all-targets --no-deps -D warnings`、workspace fmt、`git diff --check`、结构/snapshot
  Vitest `18/18`、`npm run test:contracts` 与真实 `npm run smoke:tui-gate-b` 均通过。Gate B
  线程为 `01a09a92-9968-75f0-8f85-aff9ff46b4be`，回合为
  `turn_8220938727854d1f8248108f7c9316a6`；PTY、alternate screen、stdio JSON-RPC、canonical
  projection、queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和
  terminal restore 均为 `ok`。Codex runtime metrics、完成时间本地化和跨页 nested-review
  仍保持 `partial/contract/defer`，总体计划继续 `in-progress`。

本轮 A2 older-page nested-review reconciliation（2026-09-13）：

- 对照 Codex `app/history_pagination.rs` 的 Turn 顺序要求，`thread_turns_for_items` 保持
  App Server `sortDirection=desc` 的读取方向，并在返回前统一为时间正序；当目标页最早
  Turn 位于当前 Turn 页尾部时继续请求一页，确保至少带一个更早 Turn 作为 nested-review
  前序上下文，最多 16 页且重复 cursor 立即 fail-closed。
- older-page projection 复用唯一 `history_filter::hidden_user_message_ids`，将完整 Turn
  metadata 得出的 canonical UserMessage ID 传给 grouped projection；页面内显式
  Entered/Exited 边界和跨页重复 prompt 均只按 ID 过滤，不按文本猜测。完成 separator 仍以
  原始 group 最后 item 为边界，隐藏用户消息不会前移分隔线；缺少 Turn metadata 时保持
  item-only 投影。
- 新增 canonical-id 过滤、前序 Turn 上下文、正序恢复和 grouped completion 边界回归。
  本轮已通过 TUI library `740/740`、integration `15/15`、manager regression `1/1`、
  TUI Clippy `--all-targets --no-deps -D warnings`、workspace `cargo fmt --check`、
  `git diff --check`、TUI structure/snapshot inventory `18/18` 与 `npm run test:contracts`。
  真实 `npm run smoke:tui-gate-b` 通过，
  最终 thread 为 `01a09ab2-a01d-7990-9082-feeb2a6cb1d9`、turn 为
  `turn_015404412e8d4dedb6ca324d8985ff5c`，PTY/alternate screen、stdio App Server
  JSON-RPC、canonical Thread/Turn/Item、queue-edit、agents-overview、focus-palette、
  resize-reflow、reconnect 与 terminal restore 均为 `ok`。该切片属于 `current`，未新增
  `compat`/`deprecated`、协议字段、第二套 history store 或 mock fallback；总体计划继续
  保持 `in-progress`。

本轮 A3 TextArea 编辑按键边界对齐（2026-09-13）：

- 对照 Codex `bottom_pane/textarea.rs::input_with_keymap`，将插入模式编辑快捷键的判定收回
  `keymap.rs::is_editor_key_event`，避免 ChatComposer 的控制键分支吞掉编辑动作。补齐
  `Ctrl-H`/退格、`Ctrl-M`/换行、`Ctrl-P`/上移、`Ctrl-N`/下移，以及终端可能上报的 C0
  `^A/^B/^E/^F/^H/^J/^M/^N/^P/^U` 别名；新增 `key_hint::is_altgr` 的 Windows 判定，
  Alt-Gr 字符不会误触发编辑动作，Ctrl-C 和 Ctrl-R 仍由原有 composer 边界优先处理。
- `TextArea` 新增 UTF-8/grapheme 安全的逻辑行垂直移动，按终端显示宽度选择目标列；多行
  `Up/Down` 进入同一编辑 owner，单行 Up/Down 仍保留历史导航。未复制 Codex 私有配置
  schema，完整 `RuntimeKeymap` 配置化与 visual-wrap preferred-column 仍为 `partial/defer`。
- 新增 keymap、TextArea、ChatComposer 三组 Codex-shaped 回归，覆盖控制键、C0 输入、宽字符
  边界和多行 composer 导航；本切片属于 `current`，无新增 `compat`/`deprecated`、协议字段、
  runtime 分支或本地存储。
- 验证已完成：TUI library `745/745`、integration `15/15`、manager regression `1/1`，定向
  keymap/TextArea/ChatComposer 测试通过；TUI Clippy `--lib --no-deps -- -D warnings`、workspace
  `cargo fmt --check`、`git diff --check`、structure/snapshot inventory `18/18`、
  `npm run test:contracts` 和真实 `npm run smoke:tui-gate-b` 均通过。Gate B 线程为
  `01a09ac5-7037-7ab2-972f-18769f54b259`，回合为 `turn_1073b2655f42487d9994bdca2972096a`，
  PTY、alternate screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、queue-edit、
  agents-overview、focus-palette、resize-reflow、reconnect 与 terminal restore 均为 `ok`；编译
  期间仍有 App Server 既有 `lower_turn_start_params`/`lower_runtime_options` dead-code warning，
  非本切片引入。总体计划保持 `in-progress`。

本轮 A3 visual-wrap preferred-column 补充（2026-09-13）：

- `TextArea` 新增与 Codex 同语义的 `preferred_col`，有 wrapping cache 时 `Up/Down` 按 visual
  row 导航，短行、尾部 sentinel row、制表符和宽字符均在 grapheme 边界上钳位；缩放后保留
  已保存的显示列，水平移动、编辑、模式切换和内容替换会清除旧列。无 wrapping cache 时
  继续使用逻辑行 fallback，不改变首次渲染前的输入行为。
- Vim Normal 的 `j/k` 复用同一 TextArea visual owner，避免 Vim 与 Insert 两套垂直导航语义
  分叉；未引入配置 schema、协议字段、第二套 runtime 或本地存储。
- 新增 Codex-shaped 回归：`vertical_navigation_preserves_preferred_column_across_short_wrapped_rows`、
  `vertical_navigation_preserves_destination_tab_columns`、
  `vertical_navigation_clamps_saved_column_after_resize`。本切片分类为 `current`；完整
  `RuntimeKeymap` 配置化、Vim search/text-object/find/till/dot-repeat 和 ChatWidget owner
  迁移继续保持 `partial/defer`。
- 本轮变更后的最低验证已完成：TUI library `748/748`、TUI integration `15/15`、manager
  regression `1/1`、TUI Clippy `-D warnings`、workspace fmt check、`git diff --check`、
  structure/snapshot inventory `18/18`、`npm run test:contracts` 和真实
  `npm run smoke:tui-gate-b` 均通过。Gate B 线程为 `01a09ae4-25b4-7ba3-9668-a5c1fe65032f`，
  回合为 `turn_53ed4b41815a4a96929ca31cce2d0990`；PTY、alternate screen、stdio App Server
  JSON-RPC、canonical Thread/Turn/Item、queue-edit、agents-overview、focus-palette、
  resize-reflow、reconnect 与 terminal restore 均为 `ok`。编译期间仍有 App Server 既有
  `lower_turn_start_params`/`lower_runtime_options` dead-code warning，非本切片引入。

本轮 A3 composer 历史边界清理补充（2026-09-13）：

- 对照 Codex `ChatComposerHistory` 的 reset/restore 语义，断线态 Enter/Tab 现在也会清理
  `history_index`、`last_history_text` 和保存草稿，避免重连后把旧召回文本误判为可继续历史导航。
- 图片附件变更和文件/skill popup 补全统一视为外部草稿编辑，清理历史导航状态；Vim 开启时
  popup 替换也进入同一 pending transaction。历史浏览期间 popup 同步直接退出并取消待处理
  文件搜索，避免 slash/@/$ 历史文本抢走后续 Up/Down 输入。
- 纯文本历史遇到 attachment-only 草稿时 fail closed，不用文本条目覆盖图片草稿；新增
  `attachment_only_draft_does_not_enter_text_history` 回归，等附件历史条目 contract 完整后
  再迁移 Codex 的富历史恢复语义。
- 新增 `history_navigation_clears_completion_popups_for_recalled_text`、
  `attachment_edit_exits_history_navigation` 和
  `disconnected_submit_keys_clear_history_recall_state` 回归。该切片只修改 TUI current
  composer owner，无协议、schema、兼容包装或生产 mock 变化。
- 验证已完成：TUI library `754/754`、integration `15/15`、TUI all-targets Clippy
  `-D warnings`、`npm run test:contracts` 和 `git diff --check` 通过。真实 TUI Gate B 的
  `complete`、`approval`、`user-input`、`interrupt` 单场景均通过，证明 PTY、alternate
  screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、请求交互和 terminal
  restore 主链未回归；默认多场景连续 runner 及 `failure` 单场景偶发在首次 `ready` 前
  关闭 PTY，单独 composer/runtime 测试仍稳定通过，暂按既有 Gate B fixture 启动竞态记录，
  不把它归因于本轮代码。
- 附件取出路径补充后，`complete` 场景再次通过：thread
  `01a09b4c-8549-7ff2-aac3-b1e5dba6904c`、turn
  `turn_42554e2130f141cdb58a0be4ce67384a`，事件为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，并再次证明
  `focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 为 `ok`。
