# TUI/CLI 继续同步 Codex 执行计划

状态：in-progress（本轮验证完成；总体对齐仍有明确 defer/partial）
日期：2026-09-08
参考实现：`/Users/coso/Documents/dev/rust/codex`
当前基线：Rust commit `5c07856e25b565c86c6266ff51edff58299fd4e6`（参考目录当前 checkout）

本计划承接 [全量差异报告](./codex-lime-tui-cli-difference-report.md)，目标是继续按
Codex 的真实目录、文件、公开类型、函数名和测试名同步 Lime 的 TUI/CLI。禁止根据截图
或主观设计补造同名壳；每个实现批次必须先读取对应 Codex 源文件和测试，再迁移到 Lime
current owner。

## 1. 基线与完成目标

当前事实：

| 维度 | Codex | Lime | 当前结论 |
| --- | ---: | ---: | --- |
| TUI Rust 文件 | 997 | 194 | 目录体系仍未同构 |
| TUI 类型/函数符号 | 14,991 | 3,497 | 结构差异大，须按 owner 分批收敛 |
| TUI snapshot | 1,269 | 0 | 已建立逐项分类账本，未迁入快照文件 |
| CLI Rust 文件 | 96 | 10 | current CLI 较薄，产品专属文件不机械复制 |
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
- `lime-rs/crates/core/src/config/{mod.rs,types.rs,tui_keymap.rs}`、
  `lime-rs/crates/app-server/src/processor/config.rs` 与
  `lime-rs/crates/app-server/tests/config_jsonrpc.rs`（仅限 TUI keymap 的单一用户配置层、
  `config/read` consumer 与写入校验；不得新增 TUI 私有配置文件或环境变量配置面）
- `lime-rs/crates/app-server/src/processor/thread.rs`、
  `lime-rs/crates/app-server/src/runtime/{thread_read.rs,canonical_thread_store.rs,canonical_thread_store_tests.rs}`、
  `lime-rs/crates/app-server/tests/thread_v2_jsonrpc.rs`（仅限 Codex paginated resume cursor
  contract，不扩展其它 App Server 业务）
- `packages/cli/**`、`pnpm-lock.yaml`、`lime-rs/Cargo.toml`、`lime-rs/Cargo.lock`
- `docs/ops.md`、`internal/aiprompts/commands.md`（仅同步 `tui.keymap` current 配置合同）
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

本轮 MCP elicitation footer 切片（2026-09-15）按 Codex `FooterTip` 的显示优先级收口：宽屏
继续复用五语言完整 controls 文案；窄屏按提交、字段/选择导航、取消的顺序拆分为多行，并对
每个提示逐项降级（`Enter`/`Esc`、方向键、`Tab`），保证每行不超过可用宽度且取消动作落在
最后一行。状态模型仍保持 Lime 的 `Option<usize>` 与硬编码按键边界，没有引入 Codex 私有
`ScrollState`/keymap 或新的协议字段。新增五语言窄 footer 顺序与宽度回归；MCP 输入/选择
状态同 Codex 的完整 keymap、错误提示分组和 scroll contract 继续标为 `partial/contract-defer`。

本轮底部交互窗口收口（2026-09-15）：按 Codex `MAX_POPUP_ROWS`/selection window 语义，
approval、`request_user_input` 与 MCP elicitation 的选项窗口统一限制为 8 行，并保证当前
选择始终可见；MCP elicitation 的标题、消息、字段描述按终端宽度换行，选项/输入保持单行
省略，footer 在极窄宽度逐级压缩并保留 `Esc` 取消入口。`request_user_input` 与 MCP 文本
输入的光标位置改为依据实际物理换行行数计算，避免长 CJK/多行草稿错位。新增 MCP 窄宽度
行边界和选择窗口回归；相关实现仍只消费 App Server JSON-RPC 请求，不新增协议字段或第二
套 runtime。

验证证据：TUI library `880/880`、integration `16/16`、TUI all-targets Clippy
`-D warnings`、workspace fmt check、`git diff --check`、`npm run test:contracts` 全部通过；
TUI Gate B（真实 PTY、alternate screen、stdio JSON-RPC、resize/reconnect、terminal restore）
沿用本批既有通过证据。未运行 `verify:gui-smoke`，因为本轮仅触及 Rust TUI。
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

本轮 A2 首屏 paginated history contract 收口（2026-09-14）：

- 对照 Codex `app/history_pagination.rs` 与 `history_completion.rs`，首屏
  `thread/items/list` 不再直接作为无元数据 item 列表投影；`AppServerSession` 现在在同一
  canonical thread identity 下按 item page 的 `turnId` 查询 `thread/turns/list`，返回
  `InitialHistoryPage`，由 startup、resume、reconnect 三条入口与 older-page 共用 grouped
  projection。completed Turn 的 separator 只落在该 Turn 最后 item，review prompt 只按
  canonical UserMessage ID 过滤。
- Turn 查询是 enrichment contract：旧 App Server 缺少 `thread/turns/list`、目标 Turn 或
  前序 Turn 时不猜测状态，保持 item-only/fail-closed；不新增本地 history store、协议字段
  或 mock fallback。重连后的首屏也不再绕过相同过滤/完成边界。
- 新增 TUI regression 覆盖首屏 completion/review filtering、缺失 Turn metadata 的
  item-only 行为，以及 Codex `merge=702` 对应的跨页重叠 answer 去重与 footer 边界；新增
  App Server JSON-RPC contract
  `paginated_history_jsonrpc_preserves_canonical_thread_turn_item_identity`，断言
  `thread/resume`、`thread/turns/list`、`thread/items/list` 返回同一 canonical thread/turn/item
  identity 与 answer 内容。
- 已验证 TUI library `758/758`、integration `15/15`、TUI Clippy `-D warnings`、workspace
  `cargo fmt --check`、`git diff --check`、`npm run test:contracts`。App Server Rust contract
  测试本机因 `rusty_v8` 150.4.0
  没有当前 `aarch64-apple-darwin` 预编译包而无法编译，失败发生在 V8 下载阶段，未归因于
  contract 代码；后续具备 V8 构建/缓存后必须补跑该单测与 `npm run test:contracts`。
- 该切片推进 A2 `history/transcript/pager` 主链，分类为 `current`；总体计划仍保持
  `in-progress`，A2 的跨页 nested-review 完整 Gate B 与其余 A3/A4/C/D 缺口不宣称完成。

本轮 A2 transcript overlay older-page 触顶加载（2026-09-14）：

- 对照 Codex `pager_overlay` 的 transcript 顶部历史加载语义，Lime `PagerOverlay` 在启用
  older page 的 transcript 且滚动到顶部时返回专用 `LoadOlderHistory` 动作；`App`/runtime
  复用现有 `thread/items/list` older-page loader 与 Turn enrichment，不在 overlay 内持有
  第二套 history store。静态 status pager、resume picker 的 text-only transcript preview
  保持原有消费语义，不会误发分页请求。
- `Ctrl+T` 打开 transcript 时从 `scrollback_has_older_history` 初始化可加载护栏；每次 older
  page 完成后同步最新 `has_older_history`，加载中由 App Server cursor 状态抑制重复请求。
- 新增 pager 与 App 输入路由回归，覆盖有/无 older page、Home/PageUp 触顶和静态 pager
- 验证：TUI library `762/762`、pager/App 定向测试、TUI all-targets Clippy `-D warnings`、
  workspace fmt check 与 `git diff --check` 通过；Gate B 与 App Server Rust contract 仍按
  上一切片记录执行，V8 预构建包阻塞状态不变。
- `npm run verify:local` 已启动但 Rust changed-scope 链接阶段因本机磁盘剩余约 1.2 GiB、
  报 `No space left on device` 中止；该失败不是本轮代码诊断，待释放构建空间后补跑。

本轮 A2 Home 全历史与跨页 reconciliation 收口（2026-09-14）：

- `PagerAction::LoadOlderHistory` 进入统一 App history owner 后，Home 会沿 canonical
  `thread/items/list` cursor 连续加载全部 older pages；普通 PageUp/ScrollUp 仍保持单页加载，
  不在常规滚动时预取整段历史。全量加载完成后清除旧的 transcript anchor，保证视口停在真正
  的历史起点。
- 初始 item-only 投影与后续 Turn metadata 现在复用同一页合并 helper；当相邻 completed
  review turn + interrupted duplicate turn 到达时，按 canonical UserMessage ID 移除先前已显示
  的 nested review prompt，再插入 older page，避免跨页重复和选中索引漂移。
- 新增 `older_page_reconciles_nested_review_prompt_from_adjacent_turn_metadata` 与 pager
  anchor 回归；结构 inventory 已重新生成。真实跨页 fixture/Gate B 仍待补齐，不能据此宣称
  A2 完成。该切片属于 `current`，没有新增协议字段、兼容包装、生产 mock 或本地存储。
- 验证：本轮可运行的格式、结构/snapshot inventory `18/18` 与 `git diff --check` 通过；Rust
  定向编译因本机磁盘仅剩约 219 MiB 而报 `No space left on device`，待释放构建空间后补跑。

本轮 A2 MCP canonical inventory 窄切片（2026-09-14）：

- 对照 Codex `/mcp` inventory 语义，TUI slash catalog 新增 `/mcp`；`/mcp` 请求
  `mcpServerStatus/list` 的 `ToolsAndAuthOnly` 详情，`/mcp verbose` 请求 `Full` 详情，未知
  参数在本地化 usage 中 fail-closed。App Server session 复用现有 JSON-RPC request boundary，
  沿 `nextCursor` 最多读取 16 页；重复 cursor 或超页数立即失败，不引入本地 MCP 状态存储、
  第二套 transport 或 provider-specific 解析。
- inventory 仅消费 canonical `McpServerStatus`，服务器按名称排序并渲染连接状态、工具计数；
  `Full` 额外展示 Auth、工具、资源和资源模板名称/URI，工具、资源和模板空集合统一显示
  本地化的 `(none)`，资源与模板保持 App Server 返回顺序。未知状态与 `None + NotLoggedIn`
  按 Codex 语义安全降级。结果通过现有静态 `PagerOverlay` 展示，
  不改变 transcript pager 的 older-history 动作路由。
- 新增 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` 文案、slash catalog/action/detail
  回归及 inventory 排序、Full 详情、空状态单测。该切片分类为 `current`，但 MCP resource body、
  rmcp transport、Node/CUA REPL 与完整真实 MCP inventory PTY fixture 仍为 `contract/defer`，
  未宣称 A2 或 MCP Gate B 完整完成。
- 本轮验证已完成：`cargo fmt --all --check`、TUI library `772/772`、TUI all-targets
  Clippy `-D warnings`、`git diff --check`、`npm run test:contracts`、
  `npm run inventory:tui-structure` 均通过；真实 `npm run smoke:tui-gate-b` 通过，thread
  `01a09d7a-ca8c-7081-af24-b12dcda1828d`、turn `turn_a2c1056951f3414c91a6be570c012f4d`，
  证明 PTY、alternate screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、
  queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 与 terminal restore
  主链未回归。该 Gate B fixture 未发送 `/mcp` 请求，因此不能作为 MCP inventory 的真实交互
  证据。App Server Rust contract 仍受本机 `rusty_v8` 150.4.0 Apple ARM 预构建包缺失阻塞。

本轮 A2 跨页 nested-review 真实 Gate B 收口（2026-09-14）：

- `thread/fork` 仅允许已知且带 review 文本的 `enteredReviewMode`/`exitedReviewMode` 扩展项，
  其他 Extension/Unknown 仍 fail-closed；provider history lowering 对这两个边界项显式忽略，
  不将 UI review 标记误送给模型。新增 runtime 单测覆盖合法、缺失 review 文本和未知扩展。
- 新增 `scripts/app-server/tui-history-pagination-fixture.mjs` 与真实 PTY suite，使用 external
  backend 仅作为显式测试夹具：51 个 completed turns 形成 >100 item 分页，真实 review turn、
  steer duplicate prompt、fork 中断尾回合，再以 `lime resume` 驱动 transcript Home/End。夹具
  证明 `SEED_000`/`SEED_050` 可见、`NESTED_REVIEW_PROMPT` 按 canonical UserMessage ID 隐藏、
  completion separator 无相邻重复且 alternate screen 恢复。
- 新增 npm 入口 `smoke:tui-history-pagination`；清理 TUI history enrichment 临时 debug 输出。
- 验证：真实 `LIME_KEEP_TUI_HISTORY_PAGINATION_TMP=1 node scripts/app-server/tui-history-pagination-fixture.mjs`
  通过；`cargo fmt --manifest-path lime-rs/Cargo.toml --all --check`、`node --check`、
  `npm run test:contracts`、`npm run governance:scripts` 通过；TUI history_filter、history_pagination、
  pager_overlay 定向测试全部通过。App Server Rust tests 在当前机器仍受 rusty_v8 150.4.0
  Apple ARM 预编译包缺失（HTTP 404）阻塞，需具备本地 V8 archive 后补跑。

本轮 A3 status indicator widget 收口（2026-09-14）：

- 对照 Codex `status_indicator_widget.rs` 的唯一绘制 owner 语义，Lime 将耗时、可中断提示、
  inline context、hook 状态溢出和 details wrapping 收拢到
  `tui/src/status_indicator_widget.rs`；状态行宽度测量与渲染共用同一 `lines` 路径，hook
  文案无法放入首行时移到第二行，details 按显示宽度截断并保留省略号。
- `ConversationProjection::hook_status_message()` 作为 hook display-ready 文案事实源，
  `view::screen_chunks` 使用 `desired_height_with_status` 与渲染传入相同输入，避免窄终端或
  hook 活动时覆盖队列/编辑器。旧 `status_indicator.rs` 仅保留兼容委托和历史测试，不再持有
  绘制算法；没有新增 runtime、协议字段、mock 或本地状态存储。
- 迁移并补齐 Codex 同名 status 测试：无动画状态、重映射 interrupt hint、hook reflow、
  details overflow/capitalization，以及五语言文案和窄宽度边界。该切片分类为 `current`；
  旧模块为 `compat`，完整 Codex `StatusTimer`/spinner/shimmer、可配置 keymap 与 frame
  requester 仍为 `partial/defer`，待独立 owner 和真实交互需求确认后再迁移。
- 验证：TUI status 定向测试 `10/10`（含暂停感知 `StatusTimer`）、TUI library `782/782`、
  TUI integration `16/16`、TUI Clippy `-D warnings`、workspace fmt、结构/snapshot inventory
  `18/18`、`npm run test:contracts`、`npm run governance:scripts` 与 `git diff --check` 均通过。
  完整
  `smoke:tui-gate-b` 未在本切片重复执行；上一切片已有真实 PTY/alternate-screen/stdio
 App Server JSON-RPC 证据，后续若接入动态 timer/animation 必须补跑 Gate B。

本轮 A3 footer 单行布局折叠收口（2026-09-14）：

- 在 bottom_pane/footer.rs 唯一 current owner 中补齐 Codex 形状的 SummaryLeft、SummaryHintKind、single_line_footer_layout、can_show_left_with_context、right_aligned_x 和 render_context_right。活动回合草稿优先显示 Tab queue hint，窄终端先隐藏 active agent context，再降级短 queue hint；非活动草稿显示本地化 draft 状态；无草稿保留 turn/context 左右布局与 Vim indicator。
- 布局统一使用 line_width/display_width，覆盖中日韩和 emoji 宽度边界；新增 queue_message_hint、queue_short_hint、draft_ready_hint，覆盖 zh-CN、zh-TW、en-US、ja-JP、ko-KR。未复制无 Lime consumer 的 Codex shortcuts overlay、voice、IDE context、动态 status-line 和完整 RuntimeKeymap。
- 分类：footer queue/context 单行折叠为 current；完整 Codex footer mode、voice、外部编辑器提示和配置化 keymap 为 partial/defer。未新增协议字段、runtime、兼容包装、生产 mock 或本地存储。
- 验证：footer 定向测试 9/9、TUI library 786/786、TUI Clippy（tui lib no-deps，D warnings）、workspace fmt、结构 inventory、git diff check、npm run test:contracts 与 npm run smoke:tui-gate-b 均通过。Gate B 证明 PTY、alternate screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和 terminal restore 正常。workspace 全量 Clippy 仍受并行既有 agent-protocol lint 阻塞；verify:gui-smoke 未重复执行，footer 不触达 Electron bridge。

本轮 A3 selection-row / Agents Overview 对齐（2026-09-14）：

- 新增 selection_row_layout.rs 作为选择行唯一 current display-layout owner，按 Codex 语义统一名称、前缀、描述列、disabled reason、UTF-8 grapheme/CJK/emoji 显示宽度和窄终端堆叠；model_picker.rs、app/agent_picker.rs 复用该 owner，保留各自筛选、导航、Enter/Esc、App Server 数据和路由边界。
- Agents Overview 列表行迁移到同一 owner：状态 marker、current 标记、localized 状态与 cwd/project 说明共享显示宽度与窄终端折叠逻辑；不改变 AgentsOverviewView 选择、搜索、重命名、停止、派发和 App Server refresh 主链。新增可见行回归锁定 marker、current、状态和 cwd 事实。
- selection_list.rs 当前仅剩自身测试和模块注册，未发现生产 consumer；在未取得高风险删除确认前不直接移除，分类为 dead-candidate/defer，后续需先确认是否迁移其测试语义并补回流守卫。完整 Codex ListSelectionView 的 toggle/tab/shortcut、side-content、配置化 keymap 等能力因无 Lime current consumer 继续为 contract/defer，不机械复制第二套状态机。
- 分类：selection_row_layout、model/agent picker 与 Agents Overview 行渲染属于 current；没有新增 compat 或 deprecated surface；无 Lime consumer 的完整 ListSelectionView 能力为 contract/defer；selection_list.rs 为 dead-candidate/defer，待确认后删除或迁移测试。
- 验证：Agents Overview 定向测试 17/17；TUI library 790/790；TUI integration 16/16；manager dependency regression 1/1；cargo clippy -p tui --all-targets --no-deps -- -D warnings、cargo fmt --all -- --check、git diff --check、npm run inventory:tui-structure、npm run test:contracts 与真实 npm run smoke:tui-gate-b 均通过。Gate B 新证据线程 01a09e2d-3459-7712-91ab-aff245585a72、回合 turn_a33e37bb3a204017a4a9b96a935b12d0，证明真实 lime、PTY/alternate screen、stdio App Server JSON-RPC、canonical Thread/Turn/Item、agents-overview、queue-edit、focus-palette、resize-reflow、reconnect 与 terminal restore 正常。
- 本轮仍未执行 verify:gui-smoke（未触及 Electron/GUI bridge），App Server Rust contract 仍受本机 rusty_v8 150.4.0 Apple ARM 预构建包缺失阻塞；总体计划继续保持 in-progress。下一刀回到 A3 剩余 current composer/bottom-pane owner，或在确认真实 consumer 后再处理 selection_list.rs。

本轮 A3 completion-target owner 收口（2026-09-14）：

- 对照 Codex `bottom_pane/chat_composer/completion_target.rs` 与同名测试，Lime 将 `@` 文件和 `$` 技能的 cursor-neighborhood 解析统一到 `chat_composer/completion_target.rs`；横向空白保留同一行 affinity，换行是硬边界，光标位于新 token 起点时优先右侧目标，shell-like `$` 左目标在右侧可补全目标存在时让位。解析器使用 UTF-8 安全 byte range，不跨越普通文本或嵌套 `$` 前缀误切 token。
- `skill_query_is_candidate` 改为消费 `DollarQueryKind`：空/小写或冒号限定 skill 可补全，常见环境变量、确定 positional parameter、非法语法拒绝，数字或 `-` 前缀的歧义参数仅在 catalog 存在精确 skill 时放行。删除 `chat_composer.rs` 中旧 `current_at_token_range`、`current_dollar_token_range`、`is_common_shell_variable` 生产实现，测试 helper 仅委托 current owner。
- 新增 completion-target 回归覆盖相邻 `$`/`@` 目标、separator affinity、换行与尾随空白、UTF-8 非边界光标、`$HOME`、`$1`、`$1_suffix`、`$-x`、`$-`、`$_` 和嵌套 `$HOME/$USER`。Codex atomic text-element binding 在 Lime 没有 current canonical consumer，继续分类为 `contract/defer`，未伪造第二套 text-element API。
- 分类：completion-target、ChatComposer 接线和 shell 语法 arbitration 属于 `current`；Codex 完整 atomic mention binding、plugin/connectors 与更丰富 completion UI 继续 `contract/defer`。未新增协议字段、兼容包装、生产 mock、runtime 或本地存储。
- 验证：completion-target 定向测试 `10/10`，TUI library `800/800`，TUI all-targets Clippy `-D warnings`、`cargo fmt --all`、`npm run inventory:tui-structure`、`git diff --check` 均通过；`npm run test:contracts` 退出码 `0`（协议类型生成无漂移、App Server client contract 299 checks、command contracts、harness、modality、scripts、Electron release、desktop/CLI/docs boundary 均通过）。真实 `npm run smoke:tui-gate-b` 退出码 `0`，thread `01a09e65-4e34-7033-97f3-d8cfe34f53a6`、turn `turn_168e63af64e0442498a7a8bd45e3badf`；事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 terminal restore 均为 `ok`。Gate B 编译期间的 `lower_turn_start_params`、`lower_runtime_options` dead-code warning 为既有 App Server 警告，非本切片引入。总体计划继续保持 `in-progress`，下一刀回到 A3 剩余 composer/bottom-pane current owner。

本轮 A3 popup-state owner 收口（2026-09-14）：

- 对照 Codex `bottom_pane/chat_composer/popup_state.rs` 与同名测试，Lime 将文件/技能 dismissal token 及文件搜索重复 query 状态从 `ChatComposer` 聚合字段迁入唯一 `PopupState` owner；command/file/skill popup 的可见性与 transient dismissal/query 生命周期由同一状态对象承接。文件搜索 generation/request 仍保留在 `ChatComposer`，因为它属于 App Server 请求边界，不在 popup owner 内伪造 transport 状态。
- 保持现有行为：取消文件/技能 popup 仍按 token occurrence 抑制当前实例；完成或切换 token 清理对应 dismissal；相同文件 query 不重复发请求；history navigation、空 `@` 和 slash popup 互斥逻辑不变。未新增协议字段、runtime、history store、生产 mock 或兼容包装。
- 分类：`PopupState` 与 ChatComposer 接线属于 `current`；Codex atomic text-element dismissal、MentionV2、voice strip 与完整 ChatWidget popup layout 在 Lime 没有 current canonical consumer，继续 `contract/defer`，不机械复制。
- 验证：popup-state 定向测试 `3/3`、TUI library `800/800`、TUI all-targets Clippy `-D warnings`、`cargo fmt --all -- --check`、`git diff --check` 与 `npm run inventory:tui-structure`（`870` 个文件）均通过。真实 `npm run smoke:tui-gate-b` 退出码 `0`，thread `01a09e6d-b932-79d1-b774-2f0cccbc6478`、turn `turn_dfaf2afbaac44897a6eda38067c65725`；事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，`queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和 `terminal=restored` 均为 `ok`。Gate B 编译期间的 `lower_turn_start_params`、`lower_runtime_options` dead-code warning 为既有 App Server 警告，非本切片引入。总体计划继续保持 `in-progress`，下一刀回到 A3 其余 composer/bottom-pane current owner。

本轮 A3 agents-navigation owner 收口（2026-09-14）：

- 对照 Codex 当前 bottom_pane/chat_composer/agents_navigation.rs 语义，Lime 新增该模块作为空草稿 Agents Overview 导航的唯一 composer owner。ChatComposer 仅在本地 stdio 会话启用无修饰 Left，返回 OpenAgentsOverview；App 统一映射为 open_agents_overview() 与 AppAction::RefreshAgentsOverview，嵌套 request_user_input composer 对该结果 fail-closed 忽略。remote session 默认关闭，不引入第二套导航状态机。
- 导航可用性严格要求空文本、无附件、无 popup、无 history search、无 Vim operator pending，且新增 vim_search_active() 护栏；Alt/其他修饰键不会窃取编辑器输入。新增 6 项 owner 回归与 2 项 App 路由回归，覆盖默认/remote 关闭、popup、附件、Vim operator、活动 Vim search、修饰键和空编辑器 Left。
- 分类：agents_navigation.rs、ChatComposer 接线与 App Action 路由属于 current；remote session 保持 fail-closed；Codex 完整 focus/input-enabled/runtime keymap 等 Lime 尚无 canonical consumer 的能力继续 partial/contract-defer，未新增协议字段、runtime、history store、生产 mock 或 compat 包装。
- 验证：agents-navigation 定向测试 6/6、空编辑器 Left 路由 2/2；TUI library 808/808、integration 16/16、manager regression 1/1；TUI all-targets Clippy --no-deps -- -D warnings、workspace fmt check、git diff --check 与 npm run inventory:tui-structure（871 个文件）通过。真实 npm run smoke:tui-gate-b 通过，thread 01a09e88-2b74-7560-a388-00d9a41c3ced、turn turn_41f872bac11d4a3cbf02bd12469a5b0a；事件为 turn.started,message.delta,item.started,item.completed,turn.completed，queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和 terminal=restored 均为 ok。Gate B 编译期间的 lower_turn_start_params、lower_runtime_options dead-code warning 为既有 App Server 警告，非本切片引入。未触及 Electron/GUI bridge，未运行 verify:gui-smoke；总体计划继续保持 in-progress。

本轮 A3 request_user_input 焦点与问题导航对齐（2026-09-14）：

- 对照 Codex 当前 bottom_pane/request_user_input/mod.rs 与 async_questions 交互语义，Lime RequestUserInputOverlay 为每个问题保存独立的备注草稿、选项选择和 Options/Notes 焦点；Ctrl-P/Ctrl-N、PageUp/PageDown 以及 Options 焦点下的 h/l、Left/Right 可循环切换问题，切换不会丢失其他问题的编辑状态。Options 按 Tab 进入 Notes 时先恢复当前草稿再显式持久化 Notes 焦点，避免旧缓存覆盖新状态。
- 选项题支持 Other 两阶段 Enter：首次进入备注编辑，第二次提交 Other 及可选 user_note: ...；空备注仍只提交已选项。Notes 焦点下 Esc、空 Backspace 或 Tab 清空备注并返回 Options；Options 焦点下普通字符保持 Codex 的 Options 焦点，j/k 与 Up/Down 移动选项、h/l 与 Left/Right 导航问题、空格不提交，数字快捷键按选项提交。v2 answers 结构保持不变，没有伪造 Codex 私有字段。
- 新增回归覆盖逐题 draft/selection 保留、Other 两阶段提交、Options/Notes Tab 往返、Esc/空 Backspace fail-closed、Options 输入不打开 Notes 以及 j/k 选项导航；该切片分类为 current。Codex 完整 async_questions 队列/确认未答题、自动解析、中断后持久化、可配置 keymap 与私有 composer draft 仍因 Lime canonical contract 不承载而分类为 partial/contract-defer，不得通过本地 history store 或新增协议字段补造。
- 验证：cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check、request_user_input 定向测试 12/12、TUI library 812/812、integration 16/16、manager regression 1/1、TUI Clippy --all-targets --no-deps -- -D warnings、git diff --check 与 npm run inventory:tui-structure（871 个文件）均通过。真实 npm run smoke:tui-gate-b 通过，thread 01a09ec7-4003-7240-8068-aec662bd70c6、turn turn_bca42a7b300640b78bde736af2e579ff；事件为 turn.started,message.delta,item.started,item.completed,turn.completed，queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和 terminal restore 均为 ok。本轮未触及 Electron/GUI bridge，未运行 verify:gui-smoke；Gate B 证明 TUI/CLI current 主链未回归，不等同于完整 async_questions contract 完成。总体计划继续保持 in-progress。

本轮 A3 request_user_input 非阻塞协议与自动解析子集（2026-09-14）：

- `ToolRequestUserInputParams` 新增 current `isBlocking` 字段；手写反序列化对缺少字段的旧请求 fail-closed 为 `true`，`autoResolutionMs` 保留为 deprecated 协议兼容字段。App Server action payload 同时读取 camelCase/snake_case 并默认阻塞，`RequestUserInputRunRequest`、`RequestUserInputAction` 与 Agent bridge 全链路透传，bridge 发出 `isBlocking`。v2 DTO、App Server schema、TypeScript generated client 与相关 fixture 已同步，未建立第二套协议或生产 mock。
- `CurrentTurnToolExecutor` 按 canonical collaboration mode 推导语义：`Plan` 阻塞、`Default` 非阻塞、缺少协作模式时阻塞，避免未知上下文自动放行。新增 Plan/Default/缺省判定回归。
- TUI `RequestUserInputOverlay` 对 `isBlocking=true` 保持人工回答；`false` 使用 60 秒隐藏 grace，随后 60 秒可见倒计时，到期提交空 answers。任意键或粘贴会 snooze 自动解析；倒计时通过既有 `FrameRequester` 驱动，并覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR`。这只是 Codex async_questions 的 current 子集，队列确认未答题、可配置 keymap、恢复持久化和更完整私有 composer 状态仍为 `partial/contract-defer`。
- 分类：`isBlocking` 协议字段、App Server/Agent bridge 透传和 TUI 自动解析状态机属于 `current`；`autoResolutionMs` 为协议兼容 `deprecated` 字段；没有新增 compat 包装、local history store、生产 mock 或 Electron bridge。
- 验证：`npm run test:contracts` 退出码 `0`；Rust runner 定向测试通过（agent-runtime request_user_input 5/5、lime-agent 3/3、app-server approval parser 定向测试含显式 `isBlocking=false`、app-server-protocol 133/133、TUI 816/816），`cargo clippy --locked --manifest-path lime-rs/Cargo.toml -p tui --all-targets --no-deps -- -D warnings`、workspace fmt check、`npm run inventory:tui-structure`（871 个文件）与 `git diff --check` 均通过。真实 `npm run smoke:tui-gate-b` 退出码 `0`，thread `01a09f1e-18b0-7ae2-aa21-6d0ce9807ac6`、turn `turn_3609a06f81fd4c689e88bd729e375c83`；事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 和 terminal restore 均为 `ok`。Gate B 编译期间的 `lower_turn_start_params`、`lower_runtime_options` dead-code warning 为既有 App Server 警告，非本切片引入；未触及 Electron/GUI bridge，未运行 `verify:gui-smoke`。总体计划继续保持 `in-progress`，A1/A2 其余 history contract、A3/A4 剩余 owner、CLI partial 与 Cloud transport 仍未完成。

本轮 A3 pending-input preview current owner 收口（2026-09-14）：

- 对照 Codex `bottom_pane/pending_input_preview.rs` 的 bottom-pane owner 边界，Lime 将
  `pending_input_preview.rs` 的生产实现固定在 `tui/src/bottom_pane/pending_input_preview.rs`；
  `view`、`app` 和输入路由继续直接消费该 current owner。根模块只保留无业务逻辑的
  `compat` 重导出，避免根聚合文件与 bottom-pane 形成第二个事实源；未引入 Lime 当前协议
  不承载的 Codex pending/rejected steer 状态、voice 或 RuntimeKeymap。
- `can_restore_submission`、队列多模态摘要、窄终端折叠和五语言文案仍由 current owner
  统一提供；根 compat 不新增行为。当前 Codex 完整 steer 分段与动态 binding 因没有 Lime
  canonical consumer，继续 `partial/contract-defer`，不通过本地状态或 mock 补造。
- 分类：`bottom_pane/pending_input_preview.rs` 为 `current`；根
  `pending_input_preview.rs` 为 `compat`，只委托；未删除文件、未新增协议字段、runtime、
  持久化或生产 mock。
- 验证：pending preview 定向测试 `4/4`、TUI all-targets Clippy `-D warnings`、workspace
  fmt check、`git diff --check`、`npm run inventory:tui-structure`（`872` 个文件）和
  `npm run test:contracts` 均通过。真实 `npm run smoke:tui-gate-b` 通过，thread
  `01a09f72-dd15-7df2-8ad7-847719488c8b`、turn `turn_a500254a14394c049b80594535fe508f`；
  事件为 `turn.started,message.delta,item.started,item.completed,turn.completed`，
  `queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 与
  `terminal=restored` 均为 `ok`。Gate B 编译期间的 `lower_turn_start_params`、
  `lower_runtime_options` dead-code warning 为既有 App Server 警告，非本切片引入；本轮未
  触及 Electron/GUI bridge，未运行 `verify:gui-smoke`。总体计划继续保持 `in-progress`，
  下一刀回到 A3 `chatwidget` input/interrupt/turn lifecycle，或继续收敛其它 bottom-pane
  current owner。

本轮 A3 input-flow owner 收口（2026-09-14）：

- 对照 Codex `chatwidget/input_flow.rs` 的路由边界，Lime 将 App 键盘输入实现迁入
  `tui/src/app/input_flow.rs`，由该模块作为唯一 current owner；全局滚动、Agent 切换、
  collaboration mode、interrupt、图片/复制/Transcript 快捷键、队列编辑和 composer action
  映射均保留既有 App Server/Thread/Turn/Item 主链，不新增第二套状态机或本地存储。
- 旧 `tui/src/app/input.rs` 已降为无业务逻辑的历史边界文件，且不再由 `app.rs` 注册；生产
  路径只注册 `mod input_flow`，避免旧实现与 Codex-shaped owner 并存。该旧路径分类为
  `compat/dead-candidate`，后续若结构守卫确认无外部源码 consumer，再按仓库删除政策处理。
- 本轮未机械复制 Codex 尚无 Lime canonical consumer 的完整 `chatwidget` runtime keymap、
  voice、IDE context、atomic text-element submission 或 transport；这些继续分类为
  `contract/defer`。没有新增协议字段、兼容包装、生产 mock 或第二套后端。
- 验证：`cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check`、TUI library
  `818/818`、`cargo clippy --locked --manifest-path lime-rs/Cargo.toml -p tui --all-targets
  --no-deps -- -D warnings`、`npm run inventory:tui-structure`（`873` 个文件）、
  `npm run test:contracts` 与 `git diff --check` 均通过。未重复运行真实 `smoke:tui-gate-b`
  或 `verify:gui-smoke`；本轮仅重命名/收敛现有输入 owner，之前 Gate B 证据仍有效。总体计划
  保持 `in-progress`，下一刀优先对照 `chatwidget/input_submission.rs` 与 `turn_lifecycle.rs`
  的 Lime current 子集。

本轮 A3 input-submission owner 收口（2026-09-14）：

- 对照 Codex `chatwidget/input_submission.rs`，Lime 新增
  `tui/src/app/input_submission.rs`，承接当前可由 Lime canonical contract 支持的提交边界：
  `InputResult` 到 `AppAction` 的映射、Ctrl-C 草稿优先级、queued submission 列表维护、
  远程/本地图片提取与恢复，以及按 `UserInput` 顺序进行无损队列编辑。真实 transport 执行
  仍归 `runtime` 与 `AppServerSession`，未在 TUI 复制 provider 或第二套 turn runtime。
- `app.rs` 不再持有上述实现，只保留状态与全局路由；`input_flow.rs` 通过 current
  `map_composer_action` 接入新 owner。旧 `app/input.rs` 继续为空的历史边界，未重新注册。
  Codex 的 atomic text-element、mention binding、shell/provider/auth 和完整 queued steer
  能力因没有 Lime current canonical consumer，保持 `contract/defer`，不伪造协议字段或本地
  history store。
- 分类：`app/input_submission.rs` 为 `current`；`app/input.rs` 为 `compat/dead-candidate`；
  完整 Codex submission/turn lifecycle 为 `partial/contract-defer`。
- 验证：TUI library `818/818`、TUI all-targets Clippy `-D warnings`、workspace fmt、
  `git diff --check`、`npm run inventory:tui-structure`（`874` 个文件）和真实
  `npm run smoke:tui-gate-b` 均通过。Gate B 线程
  `01a09f81-90b5-73a1-8253-25927ab60443`、回合 `turn_64bedc90177f48ce8d20b35f9dd5abbb`；
  编译期间的 `lower_turn_start_params`、`lower_runtime_options` dead-code warning 为既有
  App Server 警告。总体计划仍为 `in-progress`，下一刀继续读取 Codex `turn_lifecycle.rs`
  与 Lime projection/app event lifecycle，先收敛不引入第二套状态机的 current 子集。

本轮 A3 turn-lifecycle owner 收口（2026-09-14）：

- 直接复制 Codex `chatwidget/turn_lifecycle.rs` 的状态职责到
  `tui/src/app/turn_lifecycle.rs`，保留 `agent_turn_running`、`last_turn_id`、预算受限回合
  集合、完成标签集合和活动回合计时；Lime 没有 `SleepInhibitor`，因此移除该平台依赖，
  不伪造新的系统休眠控制。
- `App::start_turn`、`hydrate_thread`、`set_thread_id`、`thread_events::apply_notification`
  和 `active_turn_elapsed` 已统一委托该 owner。回合开始、canonical `turn.completed`、
  reconnect/hydrate 与线程切换都通过同一状态转移更新计时，projection 仍是唯一
  Thread/Turn/Item 事实源。
- 分类：`app/turn_lifecycle.rs` 与 App 接线属于 `current`；不适用于 Lime 当前协议的
  Codex 睡眠抑制、完整预算/完成标签消费继续 `partial/contract-defer`；没有新增协议、
  runtime、history store、生产 mock 或 compat 包装。
- 验证：turn-lifecycle 定向测试 `3/3`，TUI library `821/821`，`cargo fmt` 对本切片通过，
  `git diff --check` 对本切片通过。Clippy 已编译通过本切片，但全量 `-D warnings` 被工作树
  既有 `history_cell/session.rs` 的 `clippy::obfuscated_if_else` 阻断；格式检查同样只发现
  该外部改动，未覆盖或回滚并行修改。`Cargo.lock` 的未关联变更保持原样。总体计划仍为
  `in-progress`，下一刀回到 Codex `interaction.rs` / `interrupts.rs` 或 `tool_lifecycle.rs`。

本轮 A3 interrupts policy owner 收口（2026-09-14）：

- 直接抽取 Codex `chatwidget/interaction.rs` 的中断判定子集到
  `tui/src/app/interrupts.rs`，由 `should_interrupt_turn` 统一判断活动 canonical turn 与
  Vim search 护栏；`input_flow.rs` 的 Esc 路由改为消费该 owner。
- Codex `InterruptManager` 的交互请求队列没有机械复制：Lime 已由 `BottomPane` 负责可见
  请求队列、`pending_interactive_replay` 负责跨线程/恢复生命周期，新增队列会形成双事实源。
  因此该部分保持 current owner 不变，未新增协议、runtime、history store、生产 mock 或
  compat 包装。
- 分类：`app/interrupts.rs` 与 Esc 判定接线属于 `current`；完整 Codex interrupt queue、
  steer-after-interrupt、review interrupt 语义因 Lime contract 不承载，继续
  `partial/contract-defer`。
- 验证：interrupts 定向测试 `3/3`、TUI library `835` 个测试编译并执行；本次新增与
  生命周期相关测试均通过。全量测试仍受工作树已有 `view.rs` 改动影响：
  `test_backend_renders_remote_images_with_selection_highlight` 与
  `transcript_page_size_tracks_resize` 失败；真实 `smoke:tui-gate-b` 的 `ready` 启动标记也
  因该 view 改动不再出现。该热区属于并行修改，本轮未覆盖或回滚。格式检查仅剩已有
  `history_cell/session.rs` 排版差异，Clippy 仅剩其既有 `obfuscated_if_else`。总体计划仍为
  `in-progress`，下一刀回到不触碰 view 热区的 Codex `tool_lifecycle` / streaming owner。

本轮 A3/S4 composer 视觉收口（2026-09-15）：

- 对照 Codex `bottom_pane/chat_composer.rs` 的输入锚点语义，`view.rs` 移除 composer 的全宽
  上下边框，改用 `›` prompt、同基线 placeholder 与现有 textarea 状态；空态 placeholder
  使用 `Locale` 五语言文案（英文为 `Ask Lime to do anything`），输入、历史搜索高亮、Vim、
  光标和 popup 仍复用原 `ChatComposer` owner，不新增第二套 draft 状态。
- 输入区按 transcript + 活动态 status + queue preview + composer + footer 的弹性布局计算；
  idle 不再占用伪状态行，running status 仅在活动回合存在时占高。附件行先绘制，placeholder
  只绘制到文本子区域，避免空草稿覆盖远程/本地图片编号；窄终端继续通过现有高度裁剪保持
  可见输入区。
- `locale.rs` 新增 composer/model/queue/footer 相关本地化入口，并补齐 MCP inventory 与自动
  解析文案的五语言断言；本轮未修改 App Server protocol、RuntimeCore、provider 或持久化。
- 分类：`view.rs` composer geometry、`locale.rs` placeholder 属于 `current`；Codex 完整
  composer keymap、voice、atomic text-element 与私有 provider/auth 能力仍为
  `partial/contract-defer`，不通过 mock 或本地状态补造。
- 验证：`cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check` 通过；TUI library
  `845/845` 通过，新增 idle placeholder、transient status 与远程图片选择回归通过，
  `git diff --check` 通过。真实 `npm run smoke:tui-gate-b` 默认矩阵通过，覆盖
  complete/queue-edit/agents-overview/focus-palette/resize-reflow/reconnect 与 terminal restore。
  TUI Clippy 仍被并行改动 `history_cell/session.rs` 的既有
  `clippy::obfuscated_if_else` 阻断；本轮未运行 `verify:gui-smoke`，也未将历史 Gate B 证据
  误作本轮新证据。总体计划保持 `in-progress`，下一刀继续 S4 status/footer 窄屏矩阵或回到
  A3 `tool_lifecycle`/streaming owner。

本轮 A3 tool-lifecycle / streaming 边界收口（2026-09-15）：

- 对照 Codex `chatwidget/tool_lifecycle.rs` 与 `chatwidget/streaming.rs`，将 Lime 的
  `ItemStarted/ItemCompleted` 多 Agent 生命周期观察从 `app/thread_events.rs` 拆到唯一的
  `app/tool_lifecycle.rs`；该 owner 只更新既有 `AgentNavigationState` 的 parent-owned 与
  liveness，不创建第二套工具队列、线程状态或 projection。
- `ReasoningSummaryPartAdded` 以前在 Lime 路由层被丢弃，现由 canonical `ConversationProjection`
  保留 streamed reasoning section 边界；已完成 reasoning item 对迟到通知保持不变，后续
  `item/completed` 仍是权威文本修复点。新增多 Agent lifecycle、reasoning section 和迟到
  边界回归，协议与 RuntimeCore 不变。
- 分类：`app/tool_lifecycle.rs` 与 reasoning streaming 边界属于 `current`；Codex 完整
  stream controller、interrupt queue、provider/private realtime 与系统副作用仍为
  `partial/contract-defer`，无 compat 包装、生产 mock 或本地 history store。
- 验证：`cargo test --manifest-path lime-rs/Cargo.toml -p tui tool_lifecycle`（2/2）、
  `cargo test --manifest-path lime-rs/Cargo.toml -p tui reasoning_summary`（1/1）、
  `cargo fmt --manifest-path lime-rs/Cargo.toml --all -- --check` 通过；结构 inventory 已
  更新至 878 个文件。完整 TUI/Gate B 未在本刀重复执行；当前工作树既有 `view.rs` 两项
  测试失败与 `history_cell/session.rs` Clippy 阻塞保持原样，未覆盖并行修改。

本轮 A3 streaming 终态护栏补充（2026-09-15）：

- 对照 Codex `chatwidget/streaming.rs` 的 stream flush 语义，`ConversationProjection` 的
  `append_delta` 现在只接受仍处于 streaming 状态且类型匹配的条目；`item/completed` 替换
  后的迟到 assistant/reasoning/command/plan delta 会被忽略，不再改写 canonical 文本。
- `TurnCompleted` 的终态收敛同时关闭所有 provisional `streaming` 尾部，即使该条目没有
  `status=Running`；这使中断/失败后到达的旧 delta 不能重新创建可见的活动尾部。canonical
  turn item 仍可在后续 `item/completed` 或 turn repair 中正常替换，未新增 item/turn store。
- 新增 `late_agent_delta_does_not_reopen_completed_item`、迟到 reasoning delta 回归，以及
  `terminal_turn_closes_unrepaired_stream_tail_against_late_delta`；改动仅落在既有
  `projection.rs` owner，分类为 `current`，Codex provider/private stream controller 仍为
  `partial/contract-defer`。
- 验证：三项定向 projection 测试与 `cargo fmt --manifest-path lime-rs/Cargo.toml --all
  -- --check`、目标文件 `git diff --check` 通过；完整 TUI all-targets 继续作为本刀收尾门槛。

本轮 A3 streaming 终态集合补充（2026-09-15）：

- `ConversationProjection` 增加轻量 `closed_turn_ids` 集合；hydrate 的已完成/失败/中断回合
  与实时终态 `turn.completed` 均登记回合身份，所有 assistant/reasoning/plan/command
  delta 以及 patch/diff/plan 临时更新在创建或替换前 fail-closed。这样完全未知 item 的迟到
  通知也不会在终态回合后凭空制造 streaming transcript 行。
- `TurnStarted` 清除对应旧身份，允许新回合正常建立 provisional item；canonical
  `ItemCompleted`/turn repair 仍不受该集合限制，继续作为权威文本来源。未新增 item/turn
  store、协议字段、兼容包装或生产 mock。
- 新增 `terminal_turn_rejects_late_delta_for_unknown_item` 与
  `a_new_turn_can_create_a_streaming_item_after_a_terminal_turn` 回归。该切片属于 `current`，
  Codex 私有 stream controller、provider/private realtime 和完整 interrupt queue 仍为
  `partial/contract-defer`。
- 验证：`projection::tests` 49/49、`cargo fmt --manifest-path lime-rs/Cargo.toml --all
  -- --check`、目标文件 `git diff --check` 通过；完整 TUI all-targets 待本刀收尾执行。

本轮 A3 reasoning status projection 补充（2026-09-15）：

- 对照 Codex `chatwidget/streaming.rs` 的 `latest_summary_line`，`ConversationProjection`
  增加唯一 reasoning status 投影：活动回合中的最新可用粗体/标题行进入 status row，空行和
  HTML 注释不覆盖已有标题；Hook display message 仍优先，显式 `set_status`、错误和回合终态
  会清除 reasoning 标题。hydrate、实时 item completion 和 reasoning delta 复用同一提取规则。
- 该能力只使用现有 `status`/`active_turn_id` 与 canonical reasoning entry，不新增 ChatWidget
  私有状态、provider stream controller、history store 或协议字段；分类为 `current`，完整
  reasoning replay/voice handoff 继续 `partial/contract-defer`。
- 新增 `reasoning_summary_updates_running_status_with_latest_usable_line` 与
  `explicit_status_clears_reasoning_summary_header` 回归。
- 验证：projection 定向测试 `52/52`、`cargo fmt --manifest-path lime-rs/Cargo.toml --all
  -- --check`、TUI all-targets `clippy -D warnings`、目标文件 `git diff --check` 均通过。

本轮 S4 status/footer 窄屏矩阵收口（2026-09-15）：

- `bottom_pane/footer.rs` 对齐 Codex 空闲 footer：无草稿且无活动回合时保留本地化快捷键入口
  （英文 `? for shortcuts`），同时修正右侧 context 超宽时的整段折叠，避免窄屏残词或越界。
- `view.rs` 与 `status_indicator_widget.rs` 增加五语言 × `40/80/120` 列运行态、队列、hook、
  details 几何回归，确保 transcript/status/queue/composer/footer 五区连续且每行按 display
  width 安全截断；不修改 ChatComposer 状态模型。
- 分类：footer 快捷键提示、status/footer 几何属于 `current`；Codex context 百分比、完整
  statusline 配置和私有 provider 状态因 Lime 无 canonical 事实源，继续 `partial/contract-defer`，
  不通过 mock 或第二状态 owner 补造。
- 验证：TUI library `853/853`；TUI Clippy `-D warnings`、workspace fmt check、
  `npm run inventory:tui-structure`（878 files）、`git diff --check` 与
  `npm run smoke:tui-gate-b` 均通过。Gate B 真实 PTY/alternate screen/stdio JSON-RPC 事件链为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，并验证
  `queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 和
  `terminal=restored`。
- A3/S4 当前退出：composer、status、footer 的 Codex 核心视觉合同已有 Lime owner 与窄屏回归；
  下一刀进入 S5 popup/picker 统一（slash/file/skill/model/agent/approval/request_user_input），
  不扩大到 Codex 私有 account/update/marketplace/storage。

本轮 S5 slash popup 锚定收口（2026-09-15）：

- 对照 Codex `bottom_pane/command_popup.rs`，Lime slash command popup 限制为最多 8 个可见行，
  上下导航时移动可见切片，保证选中项始终可见且弹层继续贴合 composer；命令 catalog、输入
  parser 与 App Server contract 均未复制或修改。
- 选中项使用统一 Lime `accent_style`，描述使用 `muted_style`，每行按终端 display width
  截断；新增长目录 TestBackend 回归验证 72x16 终端中选中行可见与行数上限。
- 分类：popup geometry/style 属于 `current`；Codex 私有 service-tier/account 命令继续
  `excluded`，不建立 compat 或第二命令状态源。
- 验证：command popup 定向测试 `6/6` 通过；随后完整 TUI library 达到 `854/854`，Clippy、
  fmt、`git diff --check`、inventory `878 files` 与新一轮真实 `smoke:tui-gate-b` 均通过。
  最新 Gate B 继续覆盖 `queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、
  `reconnect` 与 `terminal=restored`。
  下一刀扩展 model/agent/resume picker 及 approval/request_user_input overlay 的窄屏布局。

本轮 S5 picker 语义样式补充（2026-09-15）：

- `model_picker.rs` 与 `app/agent_picker.rs` 复用 Lime 语义样式 owner：选中行使用
  `accent_style`，标题、描述和 footer 使用 `muted_style`；不改变 popup 尺寸、导航快捷键或
  App Server catalog。
- 保留 ModelPicker 现有 Enter 索引合同，同时完成选中态与文案样式收口；navigation/control-
  binding 回归与既有 picker 多语言窄屏测试一并通过。
- 验证：完整 TUI library `862/862`、TUI Clippy `-D warnings`、fmt 与 `git diff --check` 均通过。
  随后重新运行真实 `npm run smoke:tui-gate-b`，`queue-edit/agents-overview/focus-palette/`
  `resize-reflow/reconnect/terminal=restored` 全部通过；下一刀继续 model/agent/resume overlay
  的行数和锚定矩阵，再处理 approval/request_user_input。

本轮 A3 streaming/projection 收尾证据（2026-09-15）：

- `npm run inventory:tui-structure` 刷新结构账本为 `878` 个文件；真实
  `npm run smoke:tui-gate-b` 通过，thread `01a0a253-7f84-7670-80a5-0599efba75ed`、turn
  `turn_a34330e0fd5f4f358df65145ea90b8c4`，事件为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，并证明
  `queue-edit`、`agents-overview`、`focus-palette`、`resize-reflow`、`reconnect` 与
  `terminal=restored` 均为 `ok`。
- TUI all-targets 在本轮达到 `852` 通过、`1` 失败；唯一失败为并行既有
  `bottom_pane::footer::tests::queue_hint_shortens_before_it_disappears` 的
  `show_context` 断言，单测复跑仍稳定复现，未触及本轮 projection 写集。该失败不归因于
  本轮代码，也未覆盖或回滚并行 footer 改动；本轮自身 projection `52/52`、all-targets
  `clippy -D warnings`、fmt、Gate B 与 `git diff --check` 均通过。

本轮 S5 picker 导航合同补充（2026-09-15）：

- 对照 Codex `bottom_pane/list_selection_view.rs`、`resume_picker.rs` 与 picker 测试，
  `ModelPicker`、`AgentPicker`、`resume_picker::PickerState` 的可搜索列表现在保留普通
  `j/k`（以及其它无修饰字符）作为查询输入；上下箭头和 Ctrl-P/N/K/J 才执行列表导航，
  resume picker 仍保留 PageUp/PageDown 分页。模型/Agent picker 上下导航按 Codex 列表规则
  循环，resume picker 的已加载行继续使用原有分页边界，不伪造远端数据。
- 未映射的 Ctrl/Alt 字符不再污染模型或 resume 查询；command popup 同步接受 Ctrl-P/N/K/J
  导航。所有改动只落在已有 picker/composer owner，没有新增协议字段、第二套 selection
  state、runtime 或本地存储。
- `model_picker` 定向测试 `5/5`、`agent_picker` 定向测试 `4/4`、`command_popup` 定向测试
  `10/10`、resume picker 新增导航/查询回归通过。完整 TUI all-targets 仍需在并行 footer
  热区修复后收口；本轮未覆盖其既有 `queue_hint_shortens_before_it_disappears` 失败。

本轮 S5 approval/request_user_input 窄屏合同补充（2026-09-15）：

- `bottom_pane/render.rs` 的高度测量改为使用带上下边框交互面的真实文本宽度；之前错误扣除
  两列会让窄终端的 CJK/emoji 换行行数被低估。approval 与 request_user_input 选项共同复用
  `selection_row_layout::visible_item_window` 和 `MAX_POPUP_ROWS=8`，长目录导航时窗口跟随
  选中项，保留原始 option index/提交语义，不增加第二套 selection state。
- request_user_input 的 Up/K 与 Down/J 在 Options 焦点下按 Codex 列表语义循环；Notes 焦点仍由
  同一 ChatComposer 处理。footer 在本地化主提示无法容纳时按 `Enter · Esc`、`↵ · Esc`、
  `↵Esc`、`Esc` 逐级压缩，保证取消入口不被中间截断；宽度足够时继续追加已有次要提示。
- 新增回归：12 项 request_user_input 末项选中时只渲染 8 行且选中行可见；窄宽度高度测量与
  Paragraph 实际行数一致；选项上下导航首尾循环；五语言极窄 footer（3/4/7/12/18 列）不越界
  且保留 Esc。分类：approval/request_user_input 窄屏窗口、footer 与导航均为 `current`；
  Codex 完整 ScrollState、可配置 keymap 和完整 async_questions layout 仍为 `partial/contract-defer`。
- 验证：定向回归 `3/3`；`cargo test --locked --manifest-path "lime-rs/Cargo.toml" -p tui --all-targets`
  `879` library、`16` integration、`1` dependency regression 全部通过；
  `cargo clippy --locked --manifest-path "lime-rs/Cargo.toml" -p tui --all-targets --no-deps -- -D warnings`、
  workspace fmt、`git diff --check`、`npm run test:contracts` 均通过。真实
  `npm run smoke:tui-gate-b` 通过，thread `01a0a289-f0d4-7b11-8fb4-cff598078b55`、turn
  `turn_7baeb791a896424cacc61e48fcc4ad52`，事件为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，并证明
  `queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored` 均为 `ok`。
  本轮未触及 Electron/GUI bridge，未运行 `verify:gui-smoke`；workspace 全量 Clippy 仍可能受
  并行 `agent-protocol` lint 影响，不作为 TUI 写集阻塞。
- 这一步继续推进 `TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical
  Thread/Turn/Item projection` 主链的可操作交互；下一刀回到 S5 余项（MCP elicitation/统一
  selection footer）或 A2 history/transcript contract，不复制 Codex 私有 runtime/store。

本轮 S5 approval/request_user_input 窄屏布局补充（2026-09-15）：

- approval 与 request_user_input 继续复用现有 BottomPane 和 App Server typed request，不新增
  协议、RuntimeCore 或 mock 状态。approval 选项统一编号和 `accent_style` 选中态，底部 footer
  固定保留 `Enter confirm · Esc cancel`；request_user_input footer 按宽度优先保留 `Enter
  submit` 与 `Esc cancel`，次级选择/备注/问题导航提示仅在有空间时出现。
- 问题标题、说明和选项使用 grapheme-safe 宽度重排；request_user_input 选项窗口最多展示 8
  项并保证当前选择可见，编辑输入行保持单行截断，光标定位跟随重排后的实际输入行。
- `view.rs` 新增五语言 × `40/80/120` 列审批/问答 TestBackend 回归，覆盖标题、选项、主
  操作和行宽边界；locale 新增 overlay controls 全语言文案回归。
- 分类：窄屏 approval/request_user_input presentation 属于 `current`；Codex guardian、
  account 与未映射的完整 async_questions 字段保持 `contract/defer`，无 compat/mock fallback。

验证：TUI library `879 passed`、TUI Clippy `-D warnings`、workspace fmt check、
`git diff --check`、`npm run inventory:tui-structure`（878 files）与真实
`npm run smoke:tui-gate-b` 均通过；Gate B 事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并继续覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。App Server
仍有既有 `lower_turn_start_params`/`lower_runtime_options` dead-code warning，不影响本刀。

本轮 S5 resume picker 窄屏矩阵补充（2026-09-15）：

- 对照 Codex `resume_picker.rs` 的搜索行、选中行和 bounded list 语义，resume picker 标题
  收敛为单一动作标题，列表选中标记由 row owner 显式绘制 `❯ `，普通行预留同等宽度，避免
  ratatui `List` 内建高亮符号造成行首跳动；展开详情按扣除标记后的实际宽度渲染。
- 垂直布局改为从终端高度动态分配 header/search/list/footer，极短终端下 footer 不再把列表
  推出可视区域；metadata、transcript loading/failed/empty 提示和标题均采用 grapheme-safe
  截断，宽度为 0 时 fail-closed。
- 新增 TestBackend 回归覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` × `40/80/120`
  列，组合长标题、长路径、长搜索词、选中末项和展开 transcript，确认渲染行数、选中行和
  Unicode cell 均保持在终端边界内。
- 分类：resume picker 的动作标题、列表 marker、动态 chrome 和窄屏截断属于 `current`；
  Codex 私有 state DB fallback、provider/account 过滤与完整 toolbar 持久化继续
  `contract/defer`，不建立本地 history store 或 compat 路径。

验证：resume picker 定向回归 `41/41`；TUI library `882/882`；TUI Clippy `-D warnings`、
workspace fmt、`git diff --check`、`npm run inventory:tui-structure`（878 files）和真实
`npm run smoke:tui-gate-b` 均通过。Gate B 事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并继续覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。

本轮 S3 provider notice 视觉收口（2026-09-15）：

- `ConversationProjection` 将 typed `WarningNotification` 与 typed `ErrorNotification` 映射为
  独立的 `EntryKind::Warning`/`EntryKind::Error`；普通 system activity 仍保留 `System`。
- `entry.rs` 按 Codex notice 形状渲染 `⚠ ` attention 与 `■ ` failure，正文使用同一
  grapheme-safe wrapping owner；401/Unauthorized 原始错误文本保持不变，错误后 composer
  恢复路径与 App Server JSON-RPC 合同不变。
- 技能加载摘要/详情改用 warning/error 语义；导出和 history 渲染同步覆盖新 kind，未新增
  协议字段、runtime、store 或生产 mock。

分类：provider warning/error presentation 与 typed projection 属于 `current`；Codex 私有
guardian/account 错误元数据仍为 `contract/defer`，不在本刀伪造。

验证：TUI library `893/893`、TUI Clippy `-D warnings`、workspace fmt、`git diff --check`、
`npm run inventory:tui-structure`（878 files）及真实 `npm run smoke:tui-gate-b` 均通过。
本轮未触及 Electron/GUI bridge，未运行 `verify:gui-smoke`。下一刀继续 S5 统一 MCP
elicitation/selection footer，或扩大 A2 history/transcript contract 证据。

本轮 S5 MCP elicitation 窄屏补充（2026-09-15）：

- `bottom_pane/mcp_server_elicitation.rs` 继续作为唯一交互 owner，复用 App Server typed
  elicitation request 和既有 response mapping；不新增协议、RuntimeCore、provider 或本地
  状态源。
- MCP 选项保持 8 行 bounded viewport，footer 在窄宽度逐级压缩且保留 `Esc`；文本输入在
  宽度投影前先按显式换行拆成物理行，再进行 grapheme-safe 截断，确保多行 CJK/emoji 草稿的
  可见行与光标坐标一致。
- 新增 12 列 TestBackend 回归，覆盖中文、emoji、显式换行、输入行宽度和光标物理行；现有
  五语言 controls、长选项 bounded window 与极窄 footer 回归继续有效。
- 本切片分类为 `current` presentation/interaction；Codex 私有 async_questions、完整
  keymap/guardian/account 与未映射 MCP transport 继续 `partial/contract-defer`，不恢复任何
  retired runtime/store/fallback。

验证：MCP elicitation 定向测试 `11/11`；已执行 `cargo fmt --all`、`git diff --check`。完整
TUI all-targets、Clippy、结构 inventory 与真实 `npm run smoke:tui-gate-b` 作为本轮收尾门禁
继续执行；未触及 Electron/GUI bridge，因此不运行 `verify:gui-smoke`。

本轮 S6 streaming pager anchor 补充（2026-09-15）：

- `pager_overlay.rs` 的 transcript 手动滚动现在识别 streaming active line 原地替换：在
  canonical 行数量不变、且存在共同前缀/后缀（或单行 transcript）时，保留原逻辑行索引，
  再按当前宽度计算新的 wrapped height。delta 增长不会把用户锚点留在旧的视觉行号。
- 没有共同邻接行的同长度 hydrate/replacement 继续 fail-closed；prepend history、width
  reflow、底部 pinned 和 overlay 生命周期沿用既有 owner。没有新增 protocol/runtime/store
  或生产 mock。
- 新增 `transcript_overlay_remaps_manual_anchor_when_streaming_line_grows_in_place` TestBackend
  回归，锁定 canonical anchor 在 streaming redraw 后保持不变。

分类：S6 streaming pager anchor 为 `current`；主聊天 terminal-native scrollback 的完整
source-backed reflow、Codex live-cell commit 与跨 runtime VT100 contract 仍为
`partial/contract/defer`。

验证：pager 定向测试 `13/13`；TUI all-targets `884` library、`16` integration、`1`
dependency regression；TUI Clippy `-D warnings`；workspace fmt；`git diff --check`；
`npm run inventory:tui-structure`（878 files）；真实 `npm run smoke:tui-gate-b` 均通过。
Gate B 事件仍为 `turn.started,message.delta,item.started,item.completed,turn.completed`，
并证明 `queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
本轮未触及 Electron/GUI bridge，因此不运行 `verify:gui-smoke`。

本轮 S5 选择标记统一补充（2026-09-15）：

- 对照 Codex `history_cell/messages.rs` 与 `selection_list.rs`，canonical 用户消息及
  slash command、model、agent、skill、file picker 的选中/用户 gutter 统一为 `› `；未修改
  Markdown blockquote 等语义性的 `> `，也未新增协议、runtime、store 或 mock。
- 同步更新 `history_cell`、slash popup、view 的稳定断言，保留窄终端逐行截断和原有 popup
  锚定行为；所有选择器继续复用各自 current owner。

分类：Codex marker/选中态为 `current` presentation；未映射的 Codex onboarding/mentions
专用 `>` marker 继续保持原语义并不纳入主聊天 marker 合同。

验证：TUI library `891/891`；TUI Clippy `-D warnings`；workspace fmt；`git diff --check`；
`npm run inventory:tui-structure`（878 files）；真实 `npm run smoke:tui-gate-b` 均通过。Gate B
事件链为 `turn.started,message.delta,item.started,item.completed,turn.completed`，并继续覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。

本轮 S6 主视图 viewport anchor 补充（2026-09-15）：

- `transcript_reflow.rs` 新增唯一 `TranscriptViewport` owner，主 `view.rs` 通过它把
  `App::transcript_scroll` 的距底部请求解析为 canonical wrapped-row offset。上一帧行内容、
  宽度和实际 offset 会在 streaming tail 增长、active line 原地替换或 width reflow 时重映射，
  手动滚动不再因新 delta 重新落到尾部。
- 距底部为 0 的 pinned 语义继续跟随新内容；thread 切换与 `hydrate_thread` 清除 viewport
  锚点并回到底部，防止跨会话污染。所有数据仍来自 App Server canonical Thread/Turn/Item
  projection，没有新增协议、runtime、history store 或生产 mock。
- 新增 `TranscriptViewport` 回归覆盖 streaming 尾部增长、宽度 reflow、pinned tail-follow，
  以及 thread 切换回到底部；旧 pager overlay anchor 规则继续保留并独立覆盖 overlay 场景。

分类：主视图 scroll anchor、streaming tail-follow 和 width reflow 为 `current`；terminal-native
scrollback 的完整 source-backed rebuild、Codex live-cell commit 与跨 runtime VT100 contract
仍为 `partial/contract/defer`。

验证：`transcript_reflow` 定向测试 `6/6`；TUI all-targets `888` library、`16` integration、
`1` dependency regression；TUI Clippy `-D warnings`；workspace fmt；`git diff --check`；
`npm run inventory:tui-structure`（878 files）；真实 `npm run smoke:tui-gate-b` 均通过。Gate B
事件仍为 `turn.started,message.delta,item.started,item.completed,turn.completed`，并证明
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。本轮未触及
Electron/GUI bridge，因此不运行 `verify:gui-smoke`。

本轮 S5 MCP elicitation 元数据与选项描述补充（2026-09-15）：

- 对照 Codex `mcp_server_elicitation.rs` 的 `McpToolApprovalDisplayParam` 和单选项描述语义，
  Lime 继续在唯一 `McpServerElicitationOverlay` owner 内消费 App Server `_meta`：工具审批
  的 `tool_params_display` 保持服务端显式顺序，缺失时从 `tool_params` 按名称稳定排序，最多
  展示 3 项，值复用 canonical JSON compact 与 grapheme 截断；未知或 malformed 元数据仍
  fail-closed，不改变响应 action/content/meta 合同。
- `oneOf` 单选 schema 现在保留每个选项的 `description`，在选项行中按两空格分隔显示，并
  复用已有窄屏单行省略和 `MAX_POPUP_ROWS=8` 边界；标题、字段和 footer 的既有五语言布局不
  变更。未引入 tool suggestion consumer、第二套 selection state 或本地 fallback。
- 结构守卫同步到当前 `input_flow`/`input_submission` 与 `tool_lifecycle` owner，修复守卫对
  已迁出 `app/input.rs` 和 `thread_events::observe_item` 的过期正向断言，避免错误阻塞后续
  Codex owner 收敛。

验证：MCP elicitation 定向回归 `14/14`、TUI all-targets `895` library、`16` integration、
`1` dependency regression；TUI Clippy `-D warnings`、workspace fmt、`git diff --check`、
`npm run inventory:tui-structure`（878 files）及结构守卫 `16/16` 通过。该切片只触及 Rust
TUI presentation/structure owner，未触及 Electron/App Server protocol，因此未运行
`verify:gui-smoke`；真实 `smoke:tui-gate-b` 沿用最近一次通过证据，下一刀继续 A2
history/transcript contract 或 S5 selection footer 的 remaining Codex keymap/defer 项。

本轮 S5 overlay Ctrl-C 草稿边界补充（2026-09-16）：

- 对照 Codex `request_user_input::on_ctrl_c` 与 `mcp_server_elicitation` 的文本编辑边界，
  Lime 的 request-user-input notes 和 MCP 文本字段在有非空草稿时，首个 Ctrl-C 只清空当前
  草稿并保留交互；空草稿或选项字段才发送既有 typed cancel/empty response。该行为复用
  当前 `ChatComposer`/`TextArea` 状态，不新增协议字段、runtime、history store 或 mock。
- 新增 `ctrl_c_clears_notes_before_cancelling_request`、
  `ctrl_c_clears_text_draft_before_cancelling_elicitation` 和
  `ctrl_c_on_select_field_cancels_without_mutating_selection` 回归，覆盖二次 Ctrl-C 取消、
  选项焦点 fail-closed 以及草稿清除后的状态保持。

分类：overlay 草稿/取消边界属于 `current`；Codex 完整 keymap、异步队列和 interrupted
answer persistence 仍为 `partial/contract-defer`。未新增 `compat`/`deprecated` surface。

验证：定向 TUI 回归 `5/5`（`ctrl_c_clears` 过滤）、`cargo fmt --all -- --check` 通过；
完整门禁见下方收尾记录。该切片未触及 Electron/App Server protocol，因此不运行
`verify:gui-smoke`。

收尾验证：TUI all-targets `897` library、`16` integration、`1` dependency regression；TUI
Clippy `-D warnings`、workspace fmt、结构/快照 inventory `18/18`、`npm run test:contracts`、
`npm run governance:scripts`、`npm run governance:legacy-report`、`git diff --check` 和真实
`npm run smoke:tui-gate-b` 均通过；legacy report 摘要为零引用候选 0、分类漂移候选 0、边界
违规 0。
Gate B 线程为 `01a0a618-f0eb-76c2-98c5-a5d305b9177d`，回合为
`turn_dfa8a3a8e67b464eb71b04b943f7e6dd`；App Server 的既有
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 非本切片引入。

本轮 A2 resume transcript preview bounded scan 补充（2026-09-16）：

- 对照 Codex `resume_picker_transcript_preview.rs`，将 Lime resume picker 的 preview 从完整
  transcript loader 中拆出为独立 current owner。分页 history 首页请求 6 个 item，后续按
  `HISTORY_ITEM_PAGE_LIMIT=100` 翻页，最多扫描 `HISTORY_ITEM_SCAN_LIMIT=400` 个 item；重复
  cursor、空 cursor 或扫描预算耗尽均 fail-closed，并保留最近 6 行的 canonical item 顺序。
- 完整 transcript、pager 和 `/export` 仍继续使用 `thread_transcript.rs` 的全量 canonical
  projection，不把 bounded preview 限制错误带入导出或完整历史；legacy history 继续走既有
  `thread/read(include_turns=true)` 兼容路径。新增 `HISTORY_ITEM_SCAN_LIMIT` 归属
  `app_server_session/history.rs`，没有新增 history store、协议字段、runtime 或 mock。
- `resume_picker.rs` 只委托给 preview owner，删除重复的 entries-to-preview 转换；新增
  repeated-cursor 与 400-item budget 回归，锁定 Codex 的 bounded/fail-closed 语义。

分类：resume preview bounded pagination 属于 `current`；完整 legacy rollout tail、Codex 私有
rollout scanner、完整 ChatWidget/history keymap 仍为 `partial/contract-defer`。未新增
`compat`/`deprecated` surface，未删除现有主链入口。

验证：TUI all-targets `900` library、`16` integration、`1` dependency regression；TUI
Clippy `-D warnings`；workspace fmt；结构/快照 inventory `18/18`；`npm run test:contracts`；
`git diff --check` 均通过。本轮未触及 App Server protocol 或 Electron bridge，真实 Gate B
已复跑并通过：Gate B 线程为 `01a0a775-498b-7d22-a3e8-e078f75c25ba`，回合为
`turn_dec78edeb1dd4f178b1f4510e832dea2`，继续证明
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`；
App Server 的既有 `lower_turn_start_params`/`lower_runtime_options` warning 仍非本切片引入。
下一刀回到 A2 history/transcript contract 证据或 S5 footer/keymap 的 remaining partial 项。

本轮 S5 request-user-input option position footer 补充（2026-09-16）：

- 对照 Codex `request_user_input/render.rs` 在选项 viewport 被截断时插入 `option n/m` 的语义，
  Lime 在唯一 `RequestUserInputOverlay` owner 内根据真实选项数量和当前选择生成位置提示；
  `Other` 选项计入总数，选中索引按边界钳制，不读取渲染文本或维护第二套 selection state。
- 提示纳入现有 footer 优先级：Enter/Esc 主操作始终优先，窄屏压缩仍沿用既有
  `footer_hint_for_width`，并为 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` 提供本地化文案。
  没有改变 overlay 高度、App Server 协议、RuntimeCore、持久化或生产 mock。
- 新增长选项列表位置提示和五语言覆盖回归；已有 Ctrl-C 草稿清理语义保持不变。

分类：长选项列表的 selection position footer 属于 `current` presentation；Codex 完整
多行 FooterTip/keymap 系统、异步 question queue 和 interrupted answer persistence 仍为
`partial/contract-defer`。未新增 `compat`/`deprecated` surface。

验证：TUI all-targets `902` library、`16` integration、`1` dependency regression；TUI
Clippy `-D warnings`；workspace fmt；结构/快照 inventory `18/18`；`git diff --check`；真实
TUI Gate B 均通过。最新 Gate B 线程为 `01a0a8bd-875f-7d11-8be0-485cc5732a65`，回合为
`turn_d98e14c2e3e14428a33fd344c287afa1`，事件链继续为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
本轮未触及 App Server protocol 或 Electron bridge，因此不运行 `verify:gui-smoke`；下一刀
回到 A2 history/transcript contract 证据或 S5 footer 的多行布局 remaining partial 项。

本轮 S5 request-user-input 多行 footer 补充（2026-09-20）：

- 对照 Codex `request_user_input::{footer_tip_lines,footer_required_height}`，Lime 在唯一
  `RequestUserInputOverlay` owner 内按完整提示单元排布 footer；提交/取消作为不可拆分的首要
  操作，选项位置、选择、备注和问题导航按真实显示宽度换行，单个过长提示使用既有
  grapheme-safe ellipsis，不维护第二套 keymap 或 selection state。
- `BottomPane` 暴露统一的 footer lines/required height，`view::screen_chunks` 只在活动的
  request-user-input 交互中分配多行 footer；普通聊天、approval 与 MCP 仍保持既有高度和
  contract。footer 在高度变化时清理完整分配区域，输入边框、宽字符 continuation cell 与
  提示行不会重叠。
- 新增 Codex-shaped `footer_wraps_hints_without_splitting_individual_hints`，并扩展五语言
  `40/80/120` TestBackend、极窄宽度和 screen geometry 回归；不新增 App Server 协议、
  RuntimeCore、持久化或生产 mock。

分类：request-user-input footer wrapping/height 属于 `current` presentation；Codex 动态
ShortcutHint/完整 RuntimeKeymap、异步 question queue 和 interrupted answer persistence 仍为
`partial/contract-defer`。未新增 `compat`/`deprecated` surface。

验证：request-user-input 定向回归 `23/23`；TUI all-targets `903` library、`16`
integration、`1` dependency regression；TUI `--no-deps` Clippy `-D warnings`；workspace fmt；
结构/快照 inventory `18/18`（878 files / 991 snapshots）；`git diff --check`；真实 TUI
Gate B 均通过。最新 Gate B 线程为 `01a0bbfb-492c-7cb0-aa74-13dcc55243c4`，回合为
`turn_08e36e9ca96842d0a148f5a3286963a5`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
全依赖 Clippy 仍被写集外 `agent-protocol` 的既有 `large_enum_variant` 与
`derivable_impls` 阻断；本轮未越界修改。未触及 Electron/GUI bridge，因此不运行
`verify:gui-smoke`；下一刀回到 A2 history/transcript contract 证据或 S5 剩余动态 keymap
contract/defer 项。

本轮 A2 双 history mode 完整 transcript contract 证据（2026-09-20）：

- 扩展真实 App Server fixture，分别通过公共 `thread/start` 创建 `paginated` 与 `legacy`
  线程，显式断言响应中的 `historyMode`；paginated 写入 101 个 completed turn，形成
  207 个 canonical item / 3 个 item page，legacy 写入 51 个 completed turn，形成 102 个
  canonical item / 2 个 item page。fixture 使用与 TUI 相同的 `experimentalApi` capability，
  并在 PTY 启动前通过公共 `thread/items/list` 验证页数、item 数与 cursor 不重复。
- 对齐 Codex `resume_picker_loads_complete_paginated_and_legacy_transcripts` 场景，通过两个真实
  `lime resume` PTY 分别执行 Home/End 键盘输入，并验证首尾 `SEED_000/100` 与
  `LEGACY_000/050` 可见；paginated 的 Home 跨越全部 3 页，同时覆盖 Codex
  `transcript_home_loads_every_older_history_page`。该路径继续覆盖 review turn、重复 nested
  prompt 和 fork interrupted tail，`NESTED_REVIEW_PROMPT` 由 canonical transcript
  projection 过滤。
- 两条路径都验证 completion separator 无重复，并在退出后断言 alternate screen restore。
  fixture 经过真实 stdio App Server、initialize/initialized、公共 JSON-RPC、同一
  Thread/Turn/Item read model 和终端投影，不引入第二套 history store、runtime 或生产 mock。

分类：双 history mode 的完整 transcript contract 与真实 resume PTY 证据属于 `current`；
inventory 中其余 136 个 Codex transcript/history contract 场景仍为
`partial/contract-defer`，后续按 owner 与产品风险逐项迁移，不以私有 rollout scanner 或
测试侧 projection 冒充 current 主链。

验证：真实 `npm run smoke:tui-history-pagination` 通过，paginated 线程为
`01a0bc76-0f17-70e1-be94-3e7d83aa7f62`（207 items / 3 pages），legacy 线程为
`01a0bc75-de12-7423-b6c4-7f33bcf41d31`（102 items / 2 pages）；TUI all-targets `903` library、`16`
integration、`1` dependency regression；TUI `--no-deps` Clippy `-D warnings`；
`npm run test:contracts`、`npm run governance:scripts`、workspace fmt、`git diff --check`、
结构/快照 inventory `18/18`（878 files / 991 snapshots）均通过。App Server 的既有
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 非本切片引入；全依赖
Clippy 仍受写集外 `agent-protocol` 的既有 lint 阻断。本轮未触及 Electron/GUI bridge，
因此不运行 `verify:gui-smoke`。

本轮 A2 underfilled main scrollback 自动补页（2026-09-20）：

- 对齐 Codex `underfilled_scrollback_fetches_older_pages_without_opening_the_transcript`，新增
  `history_ui::rendered_transcript_row_count`，直接复用 canonical transcript cell 与
  `HyperlinkParagraph` wrapping 计算当前宽度下的真实行数，不维护第二套高度或渲染模型。
- `history_pagination::top_up_underfilled_history` 在主 transcript 行数不足当前 viewport 时，
  通过既有 `thread/items/list` cursor 连续加载 older pages，直到填满 viewport 或到达历史
  起点；启动 resume、in-app resume、agent switch、reconnect 和 resize 共用一个 runtime
  trigger，full-screen picker/pager 活动时不后台抢拉。
- 新增 Codex 同名真实 PTY 回归：将终端扩展到 400 行后启动三页 paginated thread，不打开
  transcript overlay、不发送任何滚动键，主视图直接同时显示 `SEED_000` 与 `SEED_100`；退出
  后继续验证 alternate screen restore。该路径只消费 App Server canonical Thread/Turn/Item，
  未新增 history store、协议字段、生产 mock 或 terminal-height 配置副本。

分类：underfilled main scrollback 的 viewport-aware auto-fill 属于 `current`；Codex
terminal-native scrollback row cap、完整 source-backed reflow/notice 与跨 runtime VT100 contract
仍为 `partial/contract-defer`，不在 Lime ratatui 主视图中复制私有 rollout owner。

验证：wrapped-row 定向回归 `1/1`；TUI all-targets `904` library、`17` integration、`1`
dependency regression；TUI `--no-deps` Clippy `-D warnings`；`npm run test:contracts`、
`npm run governance:scripts`、结构/快照 inventory `18/18`、workspace fmt 与
`git diff --check` 均通过。专用 `npm run smoke:tui-history-pagination` 通过，paginated 线程为
`01a0bc9a-b3e0-7ad3-b2a0-4662701b789f`（207 items / 3 pages），legacy 线程为
`01a0bc9a-83e7-7982-8afd-3bd8291d79bd`（102 items / 2 pages），证据包含
`underfilled=auto-filled`；通用 `npm run smoke:tui-gate-b` 亦通过，线程为
`01a0bc9b-1ee2-71a0-b8aa-efb4333cd98e`、回合为
`turn_67c7d96d041346919664aaccb802a5cf`，继续覆盖 queue-edit、agents-overview、
focus-palette、resize-reflow、reconnect 与 terminal restore。本轮未触及 Electron/GUI bridge，
因此不运行 `verify:gui-smoke`。

本轮 A2 older-history stale cursor guard（2026-09-22）：

- 对照 Codex `app_server_session/history.rs` 的 `is_older_history_page_pending` 与
  cursor-scoped cancellation，Lime 的 older-page 请求现在同时校验 `thread_id + cursor`。
  旧线程、旧 cursor 或已被新请求替换的响应会在投影前 fail-closed；旧响应也不会清除当前
  cursor 的 loading 状态，避免重连、切线程和快速滚动时污染 canonical transcript。
- `begin_older_history_page`、`apply_older_history_page`、`cancel_older_history_page` 与
  `App::request_older_history_page` 共用同一 pending 判定；没有新增协议字段、runtime、
  history store、兼容包装或生产 mock。该切片属于 `current`，A2 其余 history/transcript
  contract 仍按 App Server canonical Thread/Turn/Item 逐项收口。

验证：history 相关定向测试 `94/94`；完整 TUI all-targets `905` library、`17` integration、
`1` manager regression；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、
`git diff --check` 与结构守卫 `16/16` 通过。真实
`npm run smoke:tui-history-pagination` 通过（paginated `207 items / 3 pages`、legacy
`102 items / 2 pages`、`underfilled=auto-filled`、`alternate-screen=restored`）；
`npm run smoke:tui-gate-b` 通过，继续覆盖 queue-edit、agents-overview、focus-palette、
resize-reflow、reconnect 与 terminal restore。未触及 Electron/GUI bridge，因此不运行
`verify:gui-smoke`。

本轮 A2 stale completion surface guard（2026-09-23）：

- 对照 Codex `history_pagination.rs` 的 owned-transcript stale completion 语义，Lime 在
  `handle_older_history_page_with_turns` 中先验证 `thread_id + cursor + loading` 仍属于当前
  请求，再判断 transcript pager 是否已经耗尽 older history；如果显式 transcript overlay
  已没有 older page，旧响应在投影前 fail-closed 并仅取消匹配的 pending cursor。inline/main
  transcript 仍允许在 availability flag 刷新窗口接收响应，避免把同步 top-up 误判为 stale。
- 抽出纯状态判定 `transcript_history_surface_is_current`，补回归覆盖 transcript overlay
  exhausted、overlay with older page 和 inline refresh 三种边界。失败路径继续由
  cursor-scoped cancellation 释放 loading，后续 `begin_older_history_page` 可重试；没有新增
  协议字段、runtime、history store、兼容包装或生产 mock。

分类：older-history completion ownership 与 transcript fail-closed boundary 属于 `current`；
Codex owned transcript 的异步 viewport/search/keymap 状态在 Lime 没有同构 canonical consumer，
仍为 `partial/contract-defer`，未借此引入第二套状态机。

验证：新增定向回归与既有 pending-cursor 回归均通过；TUI 全量 `906/906` library、`17/17`
integration、`1/1` manager regression；TUI `--all-targets --no-deps` Clippy `-D warnings`、
workspace fmt、`git diff --check` 通过。真实 `npm run smoke:tui-history-pagination` 通过
（paginated `207 items / 3 pages`、legacy `102 items / 2 pages`、review filtering、
underfilled auto-fill、alternate-screen restore）；`npm run smoke:tui-gate-b` 通过，继续覆盖
queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 与 terminal restore。
未触及 Electron/GUI bridge，因此不运行 `verify:gui-smoke`。

本轮 A2 history pagination pure state ownership（2026-09-23）：

- 将 `ThreadHistoryPagination` 的 `begin`、pending cursor 判定、cursor-scoped cancel 和 page
  apply 收回同一纯状态 owner；`AppServerSession` 只负责按 `thread_id` 查找并委托，不再在
  session facade 重复维护 loading/cursor 转移逻辑。
- 新增 `stale_completion_preserves_new_cursor_and_failed_page_can_retry` 回归：旧 cursor
  的 late completion/cancel 不会清除新 cursor 的 loading；当前页失败只释放 loading，保留同一
  cursor 供下一次请求重试。item 仍按 canonical page 的 descending payload 逆序投影，重复
  cursor 继续 fail-closed。
- 该切片只收敛 current 状态 owner，没有新增协议字段、runtime、history store、compat 包装或
  生产 mock；Codex owned transcript 的异步 viewport/search/keymap 仍保持
  `partial/contract-defer`。

验证：history 定向测试 `9/9`、TUI 全量 `907/907` library、`17/17` integration、`1/1`
manager regression、TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt 与
`git diff --check` 均通过。首次 Gate B 的既有 `resize-reflow` PTY 用例发生一次 5 秒退出超时，
按同一入口重跑后通过；最终 Gate B 覆盖 queue-edit、agents-overview、focus-palette、
resize-reflow、reconnect、terminal restore。`npm run smoke:tui-history-pagination` 通过，
paginated `207 items / 3 pages`、legacy `102 items / 2 pages`、review filtering 和
underfilled auto-fill 均通过。未触及 Electron/GUI bridge，因此不运行 `verify:gui-smoke`。

本轮 A2 transcript pager failure/retry surface（2026-09-23）：

- 对照 Codex `TranscriptHistoryState::{LoadingOlder, Failed}` 与失败后保留 viewport 的语义，
  Lime 在唯一 `PagerOverlay` owner 内增加最小 `HistoryLoadState`：旧历史请求开始、完成和失败
  均由 runtime 明确转移；失败只更新 pager footer，不改变 scroll/pinned/anchor，也不写第二套
  transcript 数据。
- transcript footer 在 loading 时显示五语言加载提示，失败时显示 `Home` 重试和关闭入口；
  现有 Home 事件继续返回 `LoadOlderHistory`，因此失败后可从同一 App Server cursor 重新请求。
  由于 Lime 当前历史请求仍是同步 await，loading 状态不会在 await 中途触发独立 redraw，故不复制
  Codex 异步 frame scheduler；状态仍作为下一帧的 current presentation owner，失败路径明确
  fail-closed 并保持 anchor。
- 普通 status/MCP pager 不进入 transcript history state；`set_older_history_available(false)`
  会清理残留失败状态，避免线程切换或历史耗尽后继续显示重试提示。没有新增协议字段、RuntimeCore、
  history store、compat 包装或生产 mock；现有 runtime 只接入既有 App Server 请求的状态转移，
  成功重试时仅清理此前的 history-page error，不覆盖活动回合状态。

分类：transcript pager 的 loading/failed/retry footer 与 anchor 保持属于 `current`；Codex
owned transcript 的异步 viewport/search/keymap、动态 shortcut hints 仍为 `partial/contract-defer`，
不在 Lime 同步 transport 上伪造第二套状态机。未新增 `compat`/`deprecated` surface。

验证：新增 pager failure/retry、成功重试清理错误和五语言 footer 回归；TUI 全量 `909/909` library、`17/17`
integration、`1/1` manager regression；`cargo clippy --locked --manifest-path lime-rs/Cargo.toml
-p tui --all-targets --no-deps -- -D warnings`、workspace fmt、`git diff --check` 均通过。
真实 `npm run smoke:tui-history-pagination` 与 `npm run smoke:tui-gate-b` 均通过；最新 history
fixture 为 paginated `01a0cb59-238a-7442-8f9b-d72c79b92885`（`207 items / 3 pages`）与 legacy
`01a0cb58-f434-74a3-8183-9eddd00510f1`（`102 items / 2 pages`），并通过
`review=nested-filtered/underfilled=auto-filled/alternate-screen=restored`。最新 Gate B
线程为 `01a0cb57-c55e-7542-8564-5bc880211095`，回合为
`turn_97038a466efb4ab29c8efd6fdca40b79`，事件链继续为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
本轮仅触及 Rust TUI runtime/presentation 与本地化，未触及 Electron/GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A2 真实 PTY history failure/retry evidence（2026-09-23）：

- 扩展历史分页 Gate B 的 PTY harness，支持 `lime resume <thread> --remote <ws-url>`，并将
  macOS 测试运行时动态库路径显式传入 PTY 子进程；local App Server 参数仍只在 local
  `tui/resume` 模式传递，避免 remote fixture 意外启动第二个 sidecar。
- 新增真实 remote WebSocket JSON-RPC fixture：`initialize`、`thread/resume`、canonical
  `thread/items/list`、turn/settings/catalog 等公开方法均走同一 TUI client。初始页保持足够
  viewport 行数，避免启动阶段 underfilled auto-fill 提前消费故障；用户在 transcript overlay
  按 Home 时，`history-cursor` 首次返回 JSON-RPC error，失败 footer 保留 anchor 并显示
  `History load failed / Home retry`，再次 Home 使用同一 cursor 成功返回 `older-history`。
- 回归验证 transcript 关闭、Ctrl-D 退出和 alternate screen restore；fixture 通过一次性失败
  原子状态确认注入故障确实经过真实 PTY。没有新增生产协议字段、App Server mock fallback、
  history store、compat/deprecated surface 或平行 runtime；所有数据仍来自 canonical
  Thread/Turn/Item projection。

分类：remote PTY failure/retry fixture 与 transcript pager failure surface 属于 `current`；
Codex 异步 viewport/search/keymap 仍为 `partial/contract-defer`，未在同步 TUI transport 上
伪造第二套状态机。

验证：TUI `cargo test --locked --manifest-path lime-rs/Cargo.toml -p tui` 通过，`909` library、
`18` integration、`1` manager regression；新增 failure/retry 测试单独通过；
`cargo clippy --locked --manifest-path lime-rs/Cargo.toml -p tui --all-targets --no-deps -- -D warnings`、
workspace fmt 与 `git diff --check` 通过。真实
`npm run smoke:tui-history-pagination` 通过：paginated `01a0cc04-8fc0-72a1-b8b9-2d7ecee58755`
（`207 items / 3 pages`）、legacy `01a0cc04-5ca1-7b23-ac2e-de4caa128629`
（`102 items / 2 pages`），并包含 `review=nested-filtered/underfilled=auto-filled/
alternate-screen=restored`。`npm run smoke:tui-gate-b` 通过：thread
`01a0cc05-e23f-78e2-8da3-5acbfd8666b3`、turn `turn_fba87792fd334de3ac962536987443b8`，
继续覆盖 queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 与
terminal restore。本轮未触及 Electron/GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。

本轮 A1 OSC 8 宽字符 diff 几何回归（2026-09-23）：

- 对照 Codex `terminal_hyperlinks` 的
  `buffer_hyperlinks_preserve_visible_cell_width_for_ratatui_diff`，在 Lime 唯一
  `terminal_hyperlinks` owner 补同名回归。测试使用半宽音标 `ｶﾞ`，验证 OSC 8 装饰后首个
  ratatui cell 仍保留双列宽度、`ForcedWidth` diff 标记，以及后续 `diff_iter` 坐标从第 3
  列继续；这同时锁定 UTF-8/grapheme 边界不会把 hyperlink 控制序列计入可见宽度。
- 仅补 current 纯渲染回归，没有改 Markdown/diff 协议、runtime、history store、兼容包装或
  生产 mock；现有 `mark_buffer_hyperlinks` 继续作为唯一 OSC 8 buffer lowering owner。

分类：OSC 8 与宽字符/半宽音标的 ratatui diff 几何属于 `current`；Codex 终端私有的完整
snapshot gallery、平台 terminal backend 和异步 presentation 状态仍为 `partial/contract-defer`，
不在 Lime 中复制第二套渲染后端。

本轮同时补 `diff_render` 的 `narrow_wrap_preserves_emoji_and_cjk_graphemes`，锁定
emoji/CJK grapheme 不被窄宽度切成半个字符，并验证每个 fragment 的 display width 不超过
边界。两条新增定向测试均通过；随后完整 TUI `911` library、`18` integration、`1`
manager regression，TUI `--all-targets --no-deps` Clippy、workspace fmt、`git diff --check`、
结构 inventory `16/16` 与 `npm run governance:scripts` 均通过。真实
`npm run smoke:tui-gate-b` 通过：thread `01a0cde7-d92a-7540-b497-fcb4c20861a7`、turn
`turn_98205c1fea774e0591cb6493ddacee95`，覆盖 queue-edit、agents-overview、focus-palette、
resize-reflow、reconnect 与 terminal restore。App Server 的既有
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 未由本切片引入。

本轮 A1 buffer hyperlink / diff 宽度回归补充（2026-09-23）：

- 对照 Codex `terminal_hyperlinks` 的
  `buffer_hyperlinks_follow_word_wrapping`、`buffer_hyperlinks_follow_wrapped_wide_glyphs`、
  `buffer_hyperlinks_follow_wrapped_halfwidth_dakuten`，在 Lime 唯一 buffer lowering owner
  补齐同名 TestBackend/Buffer 断言。测试分别覆盖 URL 跨词换行、CJK 宽字符和半宽音标在
  `Paragraph` 实际换行后的 OSC 8 目的地与可见文本一致性。
- 对照 Codex `diff_render::fallback_wrapping_uses_display_width_for_tabs_and_wide_chars`，
  补 Lime diff fallback 的 tab/CJK/emoji 窄宽度上限回归；现有 `hard_wrap`、tab 归一化和
  continuation gutter owner 不变，没有复制第二套 wrapping 实现。

分类：上述纯终端 buffer/diff geometry 回归属于 `current`；Codex trusted file hyperlink、
平台 terminal backend 和完整 snapshot gallery 没有 Lime canonical consumer，继续保持
`partial/contract-defer`，不新增协议、runtime、history store、compat 包装或生产 mock。

验证：hyperlink 定向回归 `5/5`、diff 定向回归 `38/38` 均通过；完整 TUI `915` library、
`18` integration、`1` manager regression，TUI `--all-targets --no-deps` Clippy、workspace
fmt、`git diff --check`、结构 inventory `16/16`、`npm run governance:scripts` 均通过。
真实 `npm run smoke:tui-gate-b` 通过：thread `01a0ce82-d3d5-7882-bee5-1d75407c931b`、turn
`turn_6e1e3cecfe154d2984c4f78e79e08450`，覆盖 queue-edit、agents-overview、focus-palette、
resize-reflow、reconnect 与 terminal restore。App Server 的既有
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 未由本切片引入。

本轮 A1 Markdown 文件链接与 anchor 边界（2026-09-24）：

- 对照 Codex `markdown_render/local_links.rs` 与 `markdown_render_tests.rs`，Lime 的 current
  Markdown owner 现在接收 canonical session cwd：cwd 内绝对文件目标按词法前缀缩短，cwd 外、
  `~/`、Windows drive 与 UNC 目标保留稳定显示，不访问文件系统、不引入 trusted-file object。
- 补齐 `#L12C3`、`#L12C3-L14C9`、冒号行列范围与 Unicode en-dash 范围解析，避免范围末端重复
  冒号；file URL 的 Windows drive、UNC、一次 percent decode、非法 percent spelling 和
  literal `%2520` 标签比较均收回 `markdown/local_links.rs` 唯一 owner。
- Markdown 列表中 local file link 后的软换行只在后续文本以 `:` 开头时保持同一逻辑行，其余
  情形仍按原有换行；local-link label 的 soft/hard break 以空格缓冲，避免多行标签提前脱离，
  transcript wrapping 回归证明窄终端不会把文件目标与后续正文拆开。
  没有新增协议字段、RuntimeCore、history store、compat 包装或生产 mock。

分类：cwd-aware Markdown local-link lowering、anchor normalization 与列表软换行属于
`current`；Codex trusted file object、完整 Markdown snapshot gallery、私有 terminal backend
和异步 presentation 状态在 Lime 没有 canonical consumer，继续保持 `partial/contract-defer`；
本轮未新增 `compat` / `deprecated` / `dead` surface。

验证：Markdown/local-link 定向 `46/46` 与 full TUI library `923/923` 通过；TUI all-targets 同样为
`923` library、`18` integration、`1` manager regression；TUI `--all-targets --no-deps`
Clippy `-D warnings`、workspace fmt、`git diff --check`、结构 inventory `16/16`、
`npm run governance:scripts` 均通过。真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d0c6-5327-73d3-b4d8-3560469025ce`、turn `turn_a763d84ab75541568b2ed87d80d19941`，
事件链为 `turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。本轮
仅触及 Rust TUI Markdown/transcript presentation 与结构 inventory，未触及 Electron/GUI
bridge，因此不运行 `npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A2 transcript search 最新命中与精确高亮收口（2026-09-24）：

- 对齐 Codex `transcript_view/search.rs` 的初始扫描方向，搜索查询首次建立命中时定位到当前
  已加载 transcript 的最新命中；Enter 到达 newer 边界不循环，Shift+Enter/Ctrl+P 向更旧
  命中移动，并在边界继续复用 older-history 请求。查询为空或加载失败时仍保持既有
  `PagerOverlay` fail-closed 行为。
- `highlight_search_lines` 改为按 extended grapheme 拆分 Span，仅对实际匹配的 grapheme
  添加 selected/non-selected modifier，保留原始 span 样式与 OSC 8 hyperlink columns；不再
  将包含命中的整行或整 span 误标记为高亮。新增回归验证 prefix/suffix 不高亮、匹配区域
  高亮以及 hyperlink destination 不变。

分类：最新命中方向与 grapheme 级高亮属于 `current` presentation；Codex source-backed
  selection snapshot、异步 bounded scheduler 和跨 revision reading restore 仍为
  `partial/contract-defer`。未新增协议、runtime、history store、compat/deprecated surface。

验证：新增 pager 定向回归通过；TUI library `928/928`、integration `18/18`、manager
  regression `1/1`、TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、
  `git diff --check` 均通过。Gate B 首次复跑遇到 PTY fixture 提前关闭且未出现 completion
  标记，按同一入口重跑后通过：thread `01a0d134-28b0-7013-a48a-1ee1b3a97dbd`、turn
  `turn_7dd02827c93a41489d45ada179ab8972`，事件链为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
  `queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
  首次失败属于真实 PTY 启动/退出竞态，第二次完整通过；App Server 的
  `lower_turn_start_params`/`lower_runtime_options` warning 仍为既有 dead-code warning。
本轮未触及 Electron/GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。

本轮 A2 transcript search grapheme 编辑边界补充（2026-09-24）：

- 搜索退格改用 `UnicodeSegmentation::grapheme_indices`，一次删除完整 extended grapheme；
  粘贴截断也只落在 grapheme 边界，避免组合音标、emoji ZWJ 或宽字符被拆成不可见半字符。
  该逻辑仍只作用于 `PagerOverlay` 查询编辑器，不改变 canonical transcript 文本或 App
  Server contract。
- 新增组合音标 + ZWJ emoji 粘贴/连续退格回归；此前的最新命中、局部高亮、hyperlink 保留和
  history retry 语义保持不变。

验证：TUI library `929/929`、integration `18/18`、manager regression `1/1`、Clippy
  `-D warnings`、workspace fmt、`git diff --check`、结构守卫 `16/16` 通过；真实 Gate B
  通过：thread `01a0d13b-d20a-7703-bfac-e45b5ace6adb`、turn
  `turn_bbb8a2b83d6e499f97a38eb4dad78eff`，事件链为
  `turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
  `queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
  未触及 Electron/GUI bridge，因此不运行 `npm run test:contracts` 或
  `npm run verify:gui-smoke`。

本轮 A1 wrapped-row geometry 唯一 owner 收口（2026-09-24）：

- 对照 Codex `terminal_hyperlinks/paragraph.rs` 与 transcript/pager 的共同行高语义，将
  `wrapped_line_starts` 收回 Lime `terminal_hyperlinks` 唯一纯渲染 owner；
  `pager_overlay.rs` 与 `transcript_reflow.rs` 不再各自复制同一套
  `HyperlinkParagraph::line_count` 累积算法。滚动锚点仍使用与 OSC 8 可见文本完全一致的
  ratatui display geometry。
- resume picker 的普通文本省略也改为委托既有 `line_truncation` owner，移除本地 grapheme
  截断算法；picker、footer、pager 与 diff 现在共享同一 display-width/ellipsis 语义。
- 结构 inventory 增加 `wrapped_line_starts` current symbol 守卫，避免后续重新出现平行
  wrapped-row owner；未新增协议字段、RuntimeCore、history store、compat 包装或生产 mock。

分类：`terminal_hyperlinks::wrapped_line_starts` 及 pager/transcript 接线属于 `current`；
Codex 私有 terminal backend、完整 snapshot gallery 和 source-backed terminal scrollback
仍为 `partial/contract-defer`，本轮未新增 `compat` / `deprecated` / `dead` surface。

验证：`terminal_hyperlinks` `11/11`、`pager_overlay` `14/14`、`transcript_reflow` `6/6`、
`resume_picker` `42/42`；TUI `923/923` library、`18/18` integration、`1/1` manager
regression、TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、
`git diff --check`、结构/snapshot inventory `18/18`、`npm run governance:scripts` 均通过。
真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d0e1-e925-7d73-a99f-eb3db04738a4`、turn `turn_16b8f6e30033422db937c70ebbdbf9bb`，
覆盖 `queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
本轮仅触及 Rust TUI 纯渲染 owner 与结构守卫，未触及 Electron/GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A2 transcript pager 搜索 current 子集（2026-09-24）：

- 对照 Codex `transcript_view/search.rs` 的用户可见行为，在 Lime 唯一 `PagerOverlay` owner
  增加 transcript 搜索：`/` 与 `Ctrl+F` 进入查询，普通字符/退格/粘贴编辑查询，Enter、
  Shift+Enter、Ctrl+N、Ctrl+P 在当前 canonical `HyperlinkLine` projection 中循环下一/上一
  命中，Esc/Ctrl+C 退出并保留当前浏览锚点。命中按 Unicode 不区分大小写 literal lowering，
  通过原始 UTF-8 byte range 映射到 span style，避免宽字符和 OSC 8 hyperlink geometry 漂移。
- 搜索 footer、命中计数、无匹配和操作提示补齐 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR`；
  搜索只消费 `render_transcript_content_lines` 已生成的 canonical projection，不建立第二份
  transcript/history store，也不新增 App Server JSON-RPC 字段或 mock fallback。
- 命中切换仅在当前已加载 projection 内进行；Codex 的增量扫描、历史页联动和异步 search
  scheduler 仍归 `partial/contract-defer`，当前同步 TUI transport 不伪造第二套状态机。历史
  `Home` 分页仍由既有 `LoadOlderHistory` owner 处理。

分类：transcript pager 的本地搜索编辑、命中高亮、循环导航与五语言 footer 属于 `current`；
Codex source-backed transcript search 的 bounded scan、历史分页联动、selection/presentation
  snapshot 恢复属于 `partial/contract-defer`。未新增 `compat` / `deprecated` / `dead` surface。

验证：pager 定向 `17/17`、TUI library `926/926`、integration `18/18`、manager regression
`1/1`；`cargo clippy --locked --manifest-path lime-rs/Cargo.toml -p tui --all-targets
--no-deps -- -D warnings`、workspace fmt、`git diff --check`、结构 inventory `16/16` 和
`npm run governance:scripts` 通过。真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d10b-1a97-7f71-82b1-d7c1580b85ab`、turn `turn_a253cee1b07f484d81e8beb95d4e671d`，
事件链为 `turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
`queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。本轮未
触及 Electron/GUI bridge，因此不运行 `npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A2 transcript pager 搜索与既有 history pager 接线（2026-09-24）：

- 在上一刀当前页搜索的基础上，`PagerOverlay` 现在把“当前页无命中”接到既有
  `LoadOlderHistory -> request_all_older_history_pages -> canonical projection` 链路。Enter、
  Ctrl+N、Ctrl+P 和 Home 在搜索边界请求 older page；没有更多页时不循环 wrap，而是显示
  `No more matches`。搜索请求不会创建第二套 transcript store 或本地 history cache。
- 搜索期间复用 transcript pager 的 loading/failed/retry 状态：加载中阻止重复请求，失败保留
  查询、命中游标和滚动锚点，Home/Enter 可用同一 cursor 重试；历史页成功后由当前
  `render_transcript_content_lines` canonical projection 重新计算命中。这样搜索联动仍共享
  App Server JSON-RPC history contract，而不是在 TUI 端扫描 rollout/history DB。
- 五语言 footer、命中高亮、Unicode literal folding、窄终端截断和边界提示保持不变；新增
  `transcript_search_requests_older_history_and_retries_after_a_failed_page` 回归，覆盖无命中
  自动加载、loading 去重、失败重试和旧页命中重算。Codex source-backed selection snapshot、
  异步 search scheduler、完整 selection/copy/export 仍无 Lime canonical contract，继续
  `partial/contract-defer`。

分类：当前页搜索 + 既有 history pager 接线属于 `current`；Codex 私有 source-backed search
  snapshot、异步 scheduler 与未映射 selection/export 继续 `partial/contract-defer`。未新增
  `compat` / `deprecated` / `dead` surface。

验证：pager 定向回归 `18/18`；TUI library `927/927`、integration `18/18`、manager
  regression `1/1`；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、
  `git diff --check`、结构守卫 `16/16` 均通过。真实 `npm run smoke:tui-gate-b` 通过：thread
  `01a0d127-f482-7be0-b2b5-865367715991`、turn `turn_f72f7ec5e2904ce382c5a06f9d8e764f`，
  事件链为 `turn.started,message.delta,item.started,item.completed,turn.completed`，并继续
  覆盖 `queue-edit/agents-overview/focus-palette/resize-reflow/reconnect/terminal=restored`。
  Gate B 编译期间 `lower_turn_start_params`/`lower_runtime_options` 为既有 App Server
dead-code warning，非本切片引入。本轮仅触及 Rust TUI 与本地化，未触及 Electron/GUI
bridge，因此不运行 `npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A2 transcript search grapheme 编辑与修饰键边界补充（2026-09-24）：

- 搜索退格改用 `UnicodeSegmentation::grapheme_indices`，一次删除完整 extended grapheme；
  粘贴截断也只落在 grapheme 边界，避免组合音标、emoji ZWJ 或宽字符被拆成不可见半字符。
  可见字符允许 Shift 编码输入，同时继续拒绝 Ctrl/Alt/Super 控制组合，避免大写输入被误吞
  或快捷键污染查询；这些逻辑只作用于 `PagerOverlay` 查询编辑器，不改变 canonical
  transcript 文本或 App Server contract。
- 新增组合音标 + ZWJ emoji 粘贴/连续退格及 Shift 字符输入回归；此前的最新命中、局部高亮、
  hyperlink 保留和 history retry 语义保持不变。

验证：新增 pager 定向回归通过；Clippy `-D warnings` 通过。此前完整 TUI library
`929/929`、integration `18/18`、manager regression `1/1`、fmt、diff、结构守卫与真实
Gate B 已通过，最新证据为 thread `01a0d13b-d20a-7703-bfac-e45b5ace6adb`、turn
`turn_bbb8a2b83d6e499f97a38eb4dad78eff`；未触及 Electron/GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。

补充真实 history fixture 验证（2026-09-24）：`npm run smoke:tui-history-pagination` 通过，
paginated thread `01a0d157-bc92-76e3-814b-be0972984c29` 为 `207 items / 3 pages`，legacy
thread `01a0d157-8c64-71b1-904d-3d0ee59a3d7b` 为 `102 items / 2 pages`，并继续证明
`review=nested-filtered`、`underfilled=auto-filled` 与 `alternate-screen=restored`。

最终 Gate B 复核（2026-09-24）：最后的 Shift/grapheme 查询编辑补丁后再次运行
`npm run smoke:tui-gate-b` 通过，thread `01a0d15a-5381-7713-a47a-53702bc4e2d7`、turn
`turn_153baf3dc2344828a05abb8a8a127b04`，事件链仍为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
`queue-edit=ok`、`agents-overview=ok`、`focus-palette=ok`、`resize-reflow=ok`、
`reconnect=ok`、`terminal=restored`。

本轮 A3 action-required presentation owner（2026-09-24）:

- 对照 Codex `bottom_pane/action_required_title.rs`，新增 Lime 唯一
  `bottom_pane/action_required_title.rs` owner。`build_action_required_title_text` 只连接
  已本地化的 approval、user input、MCP elicitation 值，支持排除项和空值 fail-closed，不持有
  request id、运行状态或新的队列。
- `BottomPane::action_required_title` 以当前 App Server reverse request 为事实源，统一把
  action-required 行接入现有 `bottom_pane/render.rs`；approval、request-user-input 与 MCP
  elicitation 继续使用原有交互和 typed v2 response，未复制 Codex backend banner、终端标题
  配置或第二套 selection state。
- 新增五语言 action-required/input-required 文案和窄终端兼容回归；结构 inventory 纳入
  `action_required_title.rs` 与 `build_action_required_title_text`。该切片属于 `current`
  presentation，Codex backend-owned actionable banner、rate-limit CTA 和 terminal-title
  persistence 因缺少 Lime canonical contract 继续 `partial/contract-defer`。

验证：action-required 定向测试与 TUI library `932/932`、integration `18/18`、manager
regression `1/1` 全部通过；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace
fmt、`git diff --check`、结构 inventory `16/16` 均通过。真实
`npm run smoke:tui-gate-b` 通过：thread `01a0d17c-c0f2-77e3-9e0a-04a809d36163`、turn
`turn_03aa06ea823c4f72a96f736adc344541`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖
`queue-edit=ok`、`agents-overview=ok`、`focus-palette=ok`、`resize-reflow=ok`、
`reconnect=ok`、`terminal=restored`。Gate B 编译期间的 App Server
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警，非本切片
引入。本轮仅触及 Rust TUI/presentation/localization，未触及 Electron/App Server protocol，
因此不增加 `npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A3 interaction owner 与 composer mouse selection current 切片（2026-09-24）：

- 对照 Codex `chatwidget/interaction.rs`，将 `App::handle_tui_event` 与 paste newline
  normalization 从 `app.rs` 收回新增的 `app/interaction.rs` 单一 owner。输入优先级保持为
  disconnected、pager/export/bottom pane、startup protected request、picker/overview、Vim
  query、completion popup、history search、Vim insert Escape、普通 composer；未复制 runtime、
  Thread/Turn/Item state 或 App Server transport。
- `TuiEvent::Mouse`、event stream 与 terminal lifecycle 现在转发并启停真实 mouse capture；
  `Tui::enter()` 的部分失败路径逐项 best-effort 恢复 bracketed paste、focus、mouse、alternate
  screen 与 raw mode，external editor、正常 restore、panic restore、手动 enter/leave alternate
  screen 继续共用同一终端 owner。
- 新增 `text_selection.rs`、`bottom_pane/textarea/mouse.rs` 与
  `bottom_pane/chat_composer/mouse.rs`：composer 支持 grapheme/UTF-8 安全 hit testing、wrapped
  row、宽字符、组合字符、emoji、tab、空行、拖拽选择、双击选词、三击选逻辑行、反色渲染、
  普通编辑与 Vim Replace 的选择原子替换，以及右键或 Ctrl/Cmd copy。OSC 8 hyperlink 在选择
  反色后仍保留完整 destination。copy 仍委托现有 clipboard owner；右键复制成功后清除选择，
  键盘复制保留选择，completion popup 与 remote-image selection 不建立平行状态。
- 结构 inventory 锁定 `app/interaction.rs`、`text_selection.rs`、composer/textarea mouse 文件及
  `SelectionUnit`、`handle_mouse`、`mouse_selection_range`、`copy_selection_request`、
  `clear_mouse_selection`，并把 input routing 守卫改为检查新的 interaction owner。

分类：interaction routing、composer/TextArea mouse selection/copy、`TuiEvent::Mouse` 与 terminal
mouse capture lifecycle 均为 `current`；未新增 `compat` / `deprecated` / `dead` surface。
source-backed transcript selection/copy/export、transcript mouse browsing、scroll hover 与全部
overlay mouse contract 仍为 `partial/contract-defer`，不能把本切片声明为完整 Codex mouse 对齐。

验证：TUI library `942/942`、integration `18/18`、manager regression `1/1`、TUI
`--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、`git diff --check` 与结构
inventory `16/16`、`npm run governance:scripts` 通过。真实 `npm run smoke:tui-gate-b`
通过：thread
`01a0d1ab-7f16-7583-840b-3c35a8d96cd0`、turn
`turn_0c53128c7af0400fa08fbcad6b955bb8`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`；`complete` 场景实际向
PTY 写入 SGR mouse down/up，在 composer 中间插入字符、观察可见编辑并恢复原 prompt，同时
断言 `1000h/1000l` mouse capture 与 `1049h/1049l` alternate screen 成对恢复。门禁继续覆盖
`queue-edit=ok`、`agents-overview=ok`、`focus-palette=ok`、`resize-reflow=ok`、
`reconnect=ok`、`terminal=restored`。Gate B 编译期间 App Server 的
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警，非本切片引入。
本轮未触及 Electron/App Server protocol，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。

本轮 A3 transcript mouse selection/copy current 切片（2026-09-24）：

- 对照 Codex `transcript_view/{input,selection,text}.rs`，新增 Lime
  `transcript_view/selection.rs` 唯一交互 owner。selection 只消费
  `render_transcript_content_lines -> Vec<HyperlinkLine>` 当前 canonical 展示投影；左键
  down/drag/up、双击词、三击逻辑行、右键复制、Ctrl/Cmd/Ctrl+Shift+C、Esc 和 Enter
  copy-and-follow 均由该 owner 处理，不新增 App Server 字段、history store、runtime 状态机或
  生产 mock。
- selection 开始时以 `Arc<Vec<HyperlinkLine>>` 冻结当前已加载展示快照；streaming tail 替换或
  older-history prepend 期间 pager 继续展示和命中同一 revision，复制成功后才释放，失败继续
  保留以供重试。wrapped-row 命中复刻 ratatui `WordWrapper(trim=false)` geometry，并用稳定回归
  对比 `HyperlinkParagraph::line_count`；复制取源 UTF-8 范围，只在 canonical 逻辑行之间加入
  hard newline，不复制 terminal padding/soft wrap，保留 tab 并去除其他控制字符。
- selection 反色直接 patch 已渲染 Buffer cell style，保留原 OSC 8 destination；搜索激活时
  selection/copy 优先且不修改 query。`PagerAction -> AppAction -> runtime clipboard` 接线只传递
  已选文本与 follow 意图；clipboard 成功确认后清 selection，Enter 同时恢复 tail following，
  失败保留 selection。App 内嵌 resume picker 通过显式 `TranscriptSelectionTarget` 复用同一
  runtime clipboard helper；独立 resume picker 也直接委托既有 `clipboard_copy` owner，并持有
  Linux clipboard lease，不复制新的 clipboard 实现。
- `tui-structure-inventory` 锁定 `transcript_view.rs`、`transcript_view/selection.rs`、测试文件、
  `TranscriptSelection`/`TranscriptSelectionAction` 以及 canonical projection/clipboard 边界；真实
  PTY Gate B 的 complete 场景在 Ctrl+T overlay 内定位 canonical completed text，发送 SGR
  mouse down/drag/up，并由 VT100 cell `inverse()` 证明选区可见后再关闭 overlay。

分类：当前已加载 canonical transcript projection 上的 mouse selection、快照冻结、copy、
copy-and-follow 与 clipboard 成功/失败生命周期属于 `current`；Codex 跨完整 source cell 的稳定
identity、跨未加载 page 的异步 selection persistence、edge-drag auto-scroll、键盘扩选、静止
hyperlink click/open、disclosure control，以及 terminal-only clipboard request 的
confirmed/unconfirmed 结果区分继续属于 `partial/contract-defer`。未新增 `compat`、`deprecated`
或 `dead` surface。

验证：selection/pager/app/runtime 定向回归通过；TUI library `956/956`、integration `18/18`、
manager regression `1/1`；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、结构
inventory `17/17` 与 Gate B 源码守卫 `3/3` 通过。真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d1d0-7ad9-77b0-b73e-fe2716927a38`、turn
`turn_94c9d0f8b27d44fb95aa1a0750e45339`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖 transcript SGR
selection、`queue-edit=ok`、`agents-overview=ok`、`focus-palette=ok`、`resize-reflow=ok`、
`reconnect=ok`、`terminal=restored`。Gate B 编译期间 App Server 的
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警，非本切片引入。
本轮未触及 Electron/App Server protocol/GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。

本轮 A3 transcript input/keyboard/link current 切片（2026-09-24）：

- 对照 Codex `transcript_view/input.rs`，新增 Lime 同名 owner，并把事件路由从
  `selection.rs` 收回 input；`selection.rs` 只保留 snapshot、source anchor、wrapped visual-row
  geometry、文本提取与反色渲染。Ctrl+Space 从当前 viewport 左上建立空 selection；无修饰或
  Shift + Left/Right 按 grapheme 扩选并可跨 canonical hard newline，Up/Down 按 wrapped visual
  row 移动且保留 preferred display column，endpoint 离开 viewport 时只向 `PagerOverlay` 发
  `RevealRow`，没有复制第二套 scroll owner。
- transcript content 内鼠标滚轮每次移动 3 个 visual rows，继续由 `PagerOverlay` 限幅并维护
  tail-following；普通 HTTP(S) hyperlink 只有在同一 cell 按下/释放且未 drag/scroll 时打开，
  Ctrl/Super click 立即打开。link hit testing 复用 `HyperlinkLine` 的 display-column ranges，并
  统一经过既有 `web_destination` 控制字符过滤、长度限制和 HTTP(S)-only 校验；selection 反色与
  OSC 8 destination 继续共存。
- `PagerAction::OpenLink -> AppAction::OpenLink -> runtime` 是唯一副作用链；主 pager、App 内嵌
  resume picker 与独立 resume picker 共享该行为。runtime 使用 workspace `webbrowser` 依赖，
  通过可注入 `open_link_with` 覆盖成功、非法 scheme fail-closed 与浏览器失败；manifest、lock
  与结构 inventory 已同步，打开成功/失败状态覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、
  `ko-KR`。未增加 App Server method、Electron bridge、生产 mock 或本地 history store。
- 真实 PTY Gate B 的 complete 场景在既有 SGR mouse drag 可见断言后，发送 NUL 编码的
  Ctrl+Space 与 Right，使用 VT100 cell `inverse()` 证明键盘扩选经过真实 PTY、crossterm、
  alternate screen 和 TUI draw；终端恢复断言保持不变。

分类：当前已加载 canonical transcript projection 上的 keyboard selection、preferred column、
pager reveal、3-row wheel scroll、stationary hyperlink release、Ctrl/Super immediate open 与共享
browser effect 属于 `current`；edge-drag 连续 auto-scroll、disclosure control、跨未加载 page 的
稳定 cell identity，以及 terminal-only clipboard request 的 confirmed/unconfirmed 区分仍为
`partial/contract-defer`。未新增 `compat`、`deprecated` 或 `dead` surface。

验证：transcript 定向回归 `81/81`；TUI library `968/968`、integration `18/18`、manager
regression `1/1`；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、结构
inventory `17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts` 与
`npm run verify:app-version`、`git diff --check` 通过。真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d1e8-8fb6-7532-aa66-666840ca7e45`、turn
`turn_182f540b1edc4e06b302b56e97b751cf`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖 transcript SGR
mouse selection、Ctrl+Space keyboard selection、`queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`。Gate B 编译期间
App Server 的 `lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警，
非本切片引入。本轮未触及 Electron/App Server protocol/GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A3 transcript edge-drag 连续自动滚动 current 切片（2026-09-24）：

- 对照 Codex transcript selection 的 edge-drag 行为，在 Lime 唯一
  `transcript_view::{input,selection}` owner 内补齐连续自动滚动：只有真实发生过垂直移动的
  drag 停在内容区顶/底边时，才在每个 Draw tick 推进一个 wrapped visual row；纯水平边缘拖拽
  不滚动，wheel 会暂停自动滚动，下一次真实 Drag 才恢复。render 更新 layout 后使用相同的
  pointer screen position 重新扩展 endpoint，到达 transcript 顶/底即停止续帧。
- `PagerOverlay` 继续是唯一 transcript scroll owner；`FrameRequester` 继续是唯一帧调度 owner。
  `PagerAction::ContinueTranscriptSelection -> AppAction::ScheduleFrameIn` 只复用现有
  `schedule_frame_in(TARGET_FRAME_INTERVAL)`，没有新增 timer、线程、事件循环或第二套 viewport
  状态。主 transcript pager、App 内 resume transcript pager 与独立 resume picker 共用该实现。
- `FocusLost` 与 `Resume` 结束 drag 但保留非空 selection；空鼠标 selection 只在原本处于
  dragging 时清除，空键盘 selection 仍可继续扩展。真实 PTY harness 同步移除两个时序猜测：
  打开 transcript 前等待 canonical `item.completed` 与 `turn.completed` completion separator，
  composer 点击后等待真实 cursor position，不再依赖固定 100ms sleep 或要求未变化的 marker
  必须重新出现在 PTY 字节流。
- Gate B complete 场景使用 `TUI_EDGE_ROW_00..39` 的长 canonical assistant projection，Home
  回到顶部后发送真实 SGR mouse down/drag 并保持在内容区底边；连续 Draw 将 viewport 滚到
  `TUI_GATE_B_COMPLETED`，VT100 `inverse()` 同时证明 endpoint 保持可见，随后发送 mouse release。
  该证据经过真实 `lime`、PTY、alternate screen、App Server JSON-RPC、RuntimeCore 与同一
  Thread/Turn/Item projection。

分类：edge-drag auto-scroll、one-row Draw tick、共享帧调度、focus/resume drag termination 与
三种 transcript surface 复用均为 `current`；未新增 `compat`、`deprecated` 或 `dead` surface。
disclosure control、跨未加载 page 的稳定 cell identity，以及 terminal-only clipboard request 的
confirmed/unconfirmed 区分继续属于 `partial/contract-defer`。

验证：edge-drag/FocusLost 定向回归通过；完整 TUI `976/976` library、`18/18` integration、
`1/1` manager regression；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、
结构 inventory `17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts`、
`npm run verify:app-version`、locked Cargo metadata 与 `git diff --check` 全部通过。
真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d216-90ed-7822-956d-19b71a3344c0`、turn
`turn_645d892baccf4c2b92121a2e5df119db`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖 transcript mouse
selection、keyboard selection、edge-drag auto-scroll、`queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`。Gate B 编译期间
App Server 的 `lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警，
非本切片引入。本轮未触及 Electron/App Server protocol/GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A3 transcript activity disclosure current 切片（2026-09-24）：

- 对照 Codex `transcript_view/{disclosure,layout,text}.rs` 与 `HistoryCell` activity contract，
  在 Lime `transcript_view/disclosure.rs` 新增本地 presentation owner。结构化
  `TranscriptContent` 只携带 source/compact/expanded render 与 canonical activity IDs；
  `TranscriptEntry.id` 以 `entry:<id>` 命名空间作为稳定 identity，展开集合、焦点和 scroll
  anchor 仍归 `PagerOverlay` 本地状态，不进入 App Server、ThreadStore、canonical projection
  或通用 `HyperlinkLine` 业务字段。
- `HistoryCell` 补齐 `compact_hyperlink_lines`、`expanded_hyperlink_lines`、`activity_ids` 与
  `has_hidden_activity_details`；Command/Patch/Mcp/Plan/MultiAgent/Tool 的 compact 展示保留现有
  invocation/title 首行，expanded 继续复用完整 `text + summary` renderer。主 Ctrl+T transcript
  pager、App 内 resume pager 与独立 resume picker 共用同一 presentation；older-history prepend
  和 resume replay 后按 entry ID 保留展开状态与焦点，不复制 transcript store。
- `+ Show details` / `− Show less` 是 synthetic row：物化时记录独立 excluded-line metadata，
  search、mouse/keyboard selection、highlight 与 copied source 均显式跳过控制行；键盘 selection
  横向/纵向跨过控制行时直接落到相邻 source row。鼠标点击 control 可展开/收起；F4 进入
  activity focus，Up/Down/PgUp/PgDn/Home/End 导航，Enter/Space 切换，Left/Right 定向收起/展开，
  Esc 返回。toggle 使用 activity ID 与 viewport row bias 保持可见 anchor。
- disclosure label、focus footer 与 pager affordance 覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、
  `ko-KR`。结构 inventory 锁定 disclosure owner、HistoryCell contract、excluded source 边界，
  并禁止其依赖 App Server session 或 ThreadStore。
- 真实 PTY Gate B 在 canonical `item.completed + turn.completed` 后打开 Ctrl+T transcript，实际
  观察 `+ Show details`，发送 xterm F4 序列与 Enter，观察 `− Show less` 和 canonical
  `terminal-gate-b` tool output；随后原有 SGR mouse selection、Ctrl-Space keyboard selection、
  edge-drag auto-scroll 与 terminal restore 继续通过。

分类：full-screen 主 transcript pager、App 内/独立 resume pager 的 activity disclosure、稳定
entry identity、synthetic source exclusion、mouse/keyboard control 与五语言文案均为 `current`；
未新增 `compat`、`deprecated` 或 `dead` surface。Codex grouped/live activity 多 member identity 的
完整过渡，以及 terminal-only clipboard request 的 confirmed/unconfirmed 区分继续属于
`partial/contract-defer`。

验证：定向 disclosure/selection 回归通过；完整 TUI `983/983` library、`18/18` integration、
`1/1` manager regression；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、结构
inventory `17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts` 与
`npm run verify:app-version` 通过。真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d235-6fc8-7ce2-8976-a660b68b6cfe`、turn
`turn_255a984b9e854449af1a976ea87685f2`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`；并覆盖 disclosure、
transcript mouse/keyboard selection、edge-drag、`queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`。Gate B 编译期间 App
Server 的 `lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警，非本
切片引入。本轮未触及 Electron/App Server protocol/GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A3 clipboard acknowledgement current 切片（2026-09-24）：

- 对照 Codex `clipboard_copy.rs`、`tui/selection_clipboard.rs` 与
  `app/owned_transcript.rs`，Lime 既有 `clipboard_copy.rs` 继续作为唯一 clipboard owner，并新增
  显式 `CopyOutcome::{Copied,Requested}` 与 `CopyStatus::{Confirmed,Unconfirmed}`。native clipboard
  与 WSL PowerShell 的成功写入属于 `Confirmed`；tmux/OSC52 只有发送结果、没有 delivery
  acknowledgement，因此属于 `Unconfirmed`。SSH/tmux 不再跳过 native/WSL：先尝试可确认写入，
  同时按需向 terminal 转发；terminal 失败不覆盖已经确认的 native 结果。
- `CopyOutcome::store` 只在 backend 返回新 lease 时替换 Linux clipboard owner；
  `Copied(None)` 和 `Requested` 都保留已有 lease。`/copy`、composer selection、transcript
  selection 和 `/export` 全部消费同一 outcome，不再从 `Option<ClipboardLease>` 猜测结果，也
  没有新增第二套 clipboard backend。
- 主 Ctrl+T pager、App 内 resume pager 与独立 resume picker 都把 copy 结果回送同一
  `PagerOverlay::apply_transcript_copy_result` presentation owner。只有 `Confirmed` 清 selection；
  copy-and-follow 也只有在 `Confirmed` 时关闭搜索并跳到最新。`Unconfirmed` 和失败保留冻结的
  selection/search/scroll，便于用户粘贴验证或重试；composer 右键 selection 同样只在确认后
  清除。pager footer 的 confirmed/unconfirmed/failed 反馈与 App 状态覆盖 `zh-CN`、`zh-TW`、
  `en-US`、`ja-JP`、`ko-KR`。
- 分类：clipboard outcome/status、native/terminal routing、lease retention、三种 transcript
  surface 的 acknowledgement lifecycle 与五语言反馈均为 `current`；未新增 `compat`、
  `deprecated` 或 `dead` surface。此前 terminal-only confirmed/unconfirmed 缺口已收口；Codex
  grouped/live activity 的多 member identity 完整过渡仍为 `partial/contract-defer`。

验证：clipboard routing 定向回归 `7/7`；完整 TUI library `985/985`、integration `18/18`、
manager regression `1/1`；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、locked
Cargo metadata、结构 inventory `17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts`、
`npm run verify:app-version` 与 `git diff --check` 通过。真实 `npm run smoke:tui-gate-b` 首次在
既有 external-editor 恢复后的 mouse cursor 等待处发生一次时序失败，保留目录
`tui-gate-b-dmohlL`；未修改该无关 harness，原命令复跑通过：thread
`01a0d24a-090b-7c90-bb54-531147cc95fb`、turn
`turn_560b9ebf5991412aa49d1c8fc8667028`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖 disclosure、
transcript mouse/keyboard selection、edge-drag、`queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`；通过目录
`tui-gate-b-oQ8C8U` 同样保留。App Server 的 `lower_turn_start_params`/
`lower_runtime_options` dead-code warning 为既有告警。本轮未触及 Electron/App Server
protocol/GUI bridge，因此不运行 `npm run test:contracts` 或 `npm run verify:gui-smoke`。

本轮 A3 grouped/live activity identity current 切片（2026-09-24）：

- 对照 Codex `exec_cell/model.rs`、`exec_cell/render.rs`、
  `history_cell/computer_activity.rs`、`chatwidget/{activity_groups,command_lifecycle,
  tool_lifecycle}.rs` 与 `transcript_view/disclosure.rs`，Lime 不再从 command 文案或 MCP summary
  猜测分组。TUI `ConversationProjection` 直接消费 canonical `CommandExecution.source`、
  `CommandExecution.commandActions` 与 `McpToolCall.server`：只有非 `UserShell` 且 actions 非空并全部属于
  Read/ListFiles/Search 的命令进入 exploration group；只有 `server=cua_repl` 的 MCP 调用进入
  computer group。Unknown/write/user-shell/普通 MCP 均 fail closed。
- `ActivityGroupKey` 是 TUI 内 render-only key，由 activity kind 与 canonical turn scope 组成，
  不进入 App Server protocol、ThreadStore 或 export。完整 Thread hydrate 与实时
  item.started/item.completed 使用真实 turn id；分页 group 缺少公开 turn id 时只使用该 canonical
  group 的首个 item id 作为局部 scope，因此不会跨页或跨回合臆测合并。
- `app/history_ui.rs` 只合并相邻且完整 key 相同的 entry；completion boundary 会显式截断 group。
  主 Ctrl+T pager、App 内 resume pager 与独立 resume picker 共用该 composition。每个成员仍保留
  `entry:<item-id>` identity；单成员 live cell 过渡为多成员 committed group 时，disclosure 通过
  member identity overlap 保留展开状态、焦点与 anchor，不新增 mutable active-cell 后端。
- 分类：canonical activity 分类、turn-scoped adjacent grouping、multi-member disclosure identity
  与 live/committed presentation 过渡均为 `current`；未新增 `compat`、`deprecated` 或 `dead`
  surface。Codex 把 transcript-only reasoning 吸收到 exploration/computer group，以及跨分页边界
  拼接同一未完成 group 的能力仍为 `partial/contract-defer`：现有公开分页输入没有可证明的 turn
  identity，本轮不通过 summary 文本、本地缓存或伪造协议字段补齐。

验证：activity classification/grouping/live-transition 定向回归通过；完整 TUI library
`988/988`、integration `18/18`、manager regression `1/1`；TUI `--all-targets --no-deps` Clippy
`-D warnings`、workspace fmt、locked Cargo metadata、结构 inventory `17/17`、Gate B 源码守卫
`3/3`、`npm run governance:scripts` 与 `npm run verify:app-version` 通过。真实 Gate B 前三次分别在
既有 external-editor composer cursor 时序点、resize-reflow 的 5 秒 terminal restore 等待、以及
同一 external-editor cursor 时序点抖动；external-editor 失败现场包括
`tui-gate-b-JNkbVt`、`tui-gate-b-1T6Nik`，均未删除，也未修改无关 harness。对应真实 PTY 单测
随后独立复跑通过，第四次原命令完整通过：thread
`01a0d25e-82f0-7173-b87f-f6f91f0dc75c`、turn
`turn_438d1263443a42a38c364c5d2023886e`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖 disclosure、
transcript mouse/keyboard selection、edge-drag、`queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`。App Server 的
`lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警。本轮未触及
Electron/App Server protocol/GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。

本轮 A3 activity compact presentation current 切片（2026-09-24）：

- 对照 Codex `exec_cell/{render,compact}.rs`、`history_cell/computer_activity.rs` 及对应
  exploration/computer snapshots，在上一刀 canonical grouping key 之上新增唯一结构化展示事实：
  exploration 直接保存 `CommandAction + exit_code`，computer 直接保存 bounded title、截图数量与
  error。renderer 不解析 command 文案或 summary，也不把 presentation state 写入 App Server、
  ThreadStore、export 或第二套 history model。
- exploration compact 统一为 `Exploring/Explored`，相邻成功 Read 去重合并，List/Search/Read 保留
  canonical action 顺序；compound command 只在最后一个 action 显示 exit code，Search exit 1 可见但
  不使用 failure 色，compact 最多保留两行 detail 并显示失败计数。computer compact 统一为
  `Using/Used computer · N actions · M failed`，完成态优先展示 failure 和 screenshot，active 态展示
  当前 call；所有行在窄终端按 grapheme/display width 截断。
- `app/history_ui.rs` 成为四个 surface 的唯一 composition owner：主 transcript、Ctrl+T pager、App
  内 resume pager 与独立 resume picker 均消费同一 grouped compact renderer；expanded 仍逐条保留
  canonical invocation/output/summary。单成员 live group 也使用 grouped compact，扩为多成员
  committed group 后通过成员 ID overlap 保留 disclosure；completion boundary 截断 group。旧测试
  fixture 或无法证明 structured facts 的条目 fail closed 回退原单项 renderer。
- 新增 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` 的 exploration/computer/action/count/failure/
  capture/exit 文案与稳定回归；缺失 computer title 的 `Computer action` fallback 在非英文 locale
  也不泄漏英文。新增回归覆盖 Codex 英文 shape、failure/screenshot 选择、Search exit 1 样式、
  main transcript 聚合、跨 turn 边界、live-to-committed disclosure identity 与窄宽度。
- 分类：structured activity presentation facts、四 surface compact composition、五语言文案与
  live/committed display transition 均为 `current`；未新增 `compat`、`deprecated` 或 `dead` surface。
  Codex transcript-only reasoning absorption 与缺少公开 turn identity 时的跨分页未完成 group 拼接仍为
  `partial/contract-defer`，不通过 summary 文本、本地缓存或伪造 canonical 关系补齐。

验证：完整 TUI library `996/996`、integration `18/18`、manager regression `1/1`；TUI
`--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、locked Cargo metadata、结构 inventory
`17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts`、`npm run verify:app-version` 与
`git diff --check` 全部通过。真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d290-5cb5-7fc0-a70f-9d72d12d83a2`、turn
`turn_f4d6fadeeddd4344b950a00b4058d8d3`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖 disclosure、
transcript mouse/keyboard selection、edge-drag、`queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`。Gate B 编译期间 App
Server 的 `lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警。本轮未
触及 Electron/App Server protocol/GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。总体计划保持 `in-progress`；下一刀继续评估同 turn transcript-only
reasoning 是否具备可证明的 presentation-only 吸收关系，否则转入下一个 Codex A3 history-cell
owner，不建立猜测式分组。

本轮 A3 transcript-only reasoning activity absorption current 切片（2026-09-24）：

- 对照 Codex `chatwidget/activity_groups.rs`、`history_cell/{activity_group,activity_details}.rs`、
  `thread_transcript.rs` 与 `computer_activity_keeps_reasoning_in_order_live_and_replayed` snapshot，
  Lime 在既有 canonical activity key 上为 reasoning 增加仅含 turn scope 的 presentation fact。
  live `item.started/item.completed`、reasoning delta、完整 Thread hydrate、turn completion repair 均
  直接使用 canonical turn id；不解析 reasoning/summary 文本，也不把该事实写入 App Server、
  ThreadStore、export 或第二套 history model。
- 只有前方已存在 exploration/computer group 且 reasoning scope 与 group turn scope 相同时才吸收；
  trailing reasoning 留在前组。reasoning 不参与 action 数、failure 数、active 状态或 compact
  preview 选择，compact 完全隐藏 reasoning；expanded 按 canonical 原序显示，并保持 Codex 的
  call 后空行、reasoning 间空行、最后一段 reasoning 后紧接下一 call 的布局。不同 turn、无 scope、
  可见非 reasoning item 或不同 activity kind 都立即结束 group，保持 fail-closed。
- 主 transcript、Ctrl+T pager、App 内 resume pager 与独立 resume picker 继续共享
  `app/history_ui.rs` composition。completion boundary 位于 trailing reasoning 后时仍只形成一个
  compact group，并在整个 group 后显示 separator；streamed reasoning 被 canonical completed item
  替换时保留相同 item identity、位置和 scope，disclosure identity 不漂移。
- 带 Turn 元数据的分页历史不再以首 item id 代替 scope：`HistoryItemGroup` 显式携带真实 turn id，
  completed/failed/interrupted/in-progress 相邻回合按 canonical identity 分组，同一 turn 跨页仍可用
  相同 key 拼接；无法映射 Turn 的 legacy flat page 与 `prepend_items` 保持无 scope，因此不会臆测
  reasoning 关系。该收口复用已有 `thread_turns_for_items` 数据，没有新增协议字段或兼容后端。
- 分类：同 turn transcript-only reasoning absorption、四 surface compact/expanded composition、
  live/replay/pagination scope 与 completion placement 均为 `current`；未新增 `compat`、`deprecated`
  或 `dead` surface。standalone transcript-only reasoning 在 Lime 默认 transcript 中仍可见；Codex
  默认 compact 会隐藏它、只在 detailed/search presentation 中展示，而 Lime 尚无完整全局
  detailed/raw mode owner，因此该项明确保留为 `partial/contract-defer`，不在本刀伪造全局模式。

验证：activity 定向回归 `27/27`、reasoning 定向回归 `11/11`、projection `54/54`；完整 TUI
library `1003/1003`、integration `18/18`、manager regression `1/1`；TUI
`--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、locked Cargo metadata、结构 inventory
`17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts`、`npm run verify:app-version` 与
`git diff --check` 全部通过。真实 `npm run smoke:tui-gate-b` 通过：thread
`01a0d2a3-2d41-78f2-bd52-3113ce6b1504`、turn
`turn_724805769bfb42ebbb448d097f9a5753`，事件链为
`turn.started,message.delta,item.started,item.completed,turn.completed`，并覆盖 disclosure、
transcript mouse/keyboard selection、edge-drag、`queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`。Gate B 编译期间 App
Server 的 `lower_turn_start_params`/`lower_runtime_options` dead-code warning 为既有告警。本轮未
触及 Electron/App Server protocol/Electron/GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。总体计划保持 `in-progress`；下一刀评估 standalone reasoning 的全局
detailed/search presentation owner，若 owner 边界尚不成立则继续下一个 Codex A3 history-cell 缺口。

本轮 A3 standalone transcript-only reasoning compact/detailed current 切片（2026-09-24）：

- 对照 Codex `ReasoningSummaryCell` 的 compact transcript 与 retained transcript 合同，Lime 主
  transcript 明确作为 compact surface：`app/history_ui.rs` 在 composition 边界隐藏所有 standalone
  `EntryKind::Reasoning`，而 Ctrl+T pager、Find 使用的 retained transcript、resume transcript 与
  canonical export 数据继续保留 reasoning。该 presentation 决策不进入 App Server、ThreadStore、
  export 或第二套 history model。
- compact composition 使用独立 body 缓冲后再追加 completion separator；reasoning-only transcript
  不再留下伪造空白行，但 completed turn 的 `Worked for 61s` 等 canonical completion boundary 仍然
  可见。带 canonical turn scope 的 standalone reasoning 与 legacy flat/no-scope reasoning 均不会泄漏
  到 compact 主面；后者仍按上一刀合同拒绝参与 activity absorption，只影响 presentation 隐藏。
- 新增回归分别锁定 scoped standalone reasoning、flat/no-scope reasoning 与 hidden reasoning 后的
  completion separator。真实 terminal fixture 的 complete 场景新增 canonical Reasoning item，事件链
  因此扩展为
  `turn.started,message.delta,item.started,item.completed,item.started,item.completed,turn.completed`；
  PTY 在 Ctrl+T 打开前断言当前 screen 不含 reasoning marker，打开 retained transcript 后断言该
  marker 可见，证明同一 canonical item 在 compact/detailed surface 的投影差异。
- 分类：主 compact transcript 隐藏 standalone reasoning、retained transcript/Find 数据源保留详情、
  flat/no-scope fail-closed 与 completion separator 保留均为 `current`；未新增 `compat`、
  `deprecated` 或 `dead` surface。Codex 完整 owned `TranscriptView`、全局 detailed/raw presentation
  mode 与 compact 主面的 activity disclosure 尚未完整同构，继续标为 `partial/contract-defer`。

验证：新增 history 回归后 `app::history_ui::tests` 为 `22/22`；完整 TUI library `1006/1006`、
integration `18/18`、manager regression `1/1`；TUI `--all-targets --no-deps` Clippy
`-D warnings`、workspace fmt、locked Cargo metadata、结构 inventory `17/17`、Gate B 源码守卫
`3/3`、`npm run governance:scripts`、`npm run verify:app-version` 与 `git diff --check` 全部通过。
complete-only 真实 Gate B 先证明 reasoning compact/detailed 投影；随后不裁剪场景的完整 Gate B 通过：
thread `01a0d2b7-0221-7d91-9b97-c54146b10441`、turn
`turn_571980295d5d4f21a836557345c4848f`，事件链为上述七事件，并覆盖 `queue-edit=ok`、
`agents-overview=ok`、`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、
`terminal=restored`。完整 Gate B 的默认预构建连续两次因 GitHub rusty-v8 release 返回 HTTP 500 未
进入产品场景；最终显式复用时间戳晚于本轮 TUI 源码的真实 `lime` 二进制与本轮未变更的真实
`app-server` 二进制，只跳过重复下载，没有裁剪场景或降低 PTY/App Server/runtime 证据等级。本轮未
改 App Server protocol、Electron 或 GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。总体计划继续保持 `in-progress`；下一刀审计 Codex owned
`TranscriptView` 与全局 detailed/raw presentation owner，或转入下一个 A3/A2 明确缺口。

本轮 A3 global raw scrollback presentation current 切片（2026-09-24）：

- 审计确认 Lime 已有 `HistoryRenderMode::{Rich, Raw}`、`HistoryCell::raw_lines` 与各 cell raw
  contract，但此前没有 App 状态、渲染 consumer 或产品入口。现由 `App` 持有唯一 session-local
  presentation mode；`/raw` 与全局 `Alt+R` 共用同一 toggle，即使 Ctrl+T pager 打开也只切换底层
  主 scrollback mode，不关闭或劫持 overlay。切换反馈覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、
  `ko-KR`，slash catalog、popup、settings parser exhaustiveness 与回归同步更新。
- 主 transcript 的 Rich 路径继续使用 grouped compact/disclosure；Raw 路径逐项消费 canonical
  `TranscriptEntry` 的 copy-friendly source，不做 Markdown/table/diff 富渲染，不携带样式或 OSC 8
  hyperlink，不重新分组，也不改 canonical projection。user source 仍经过 control-sequence sanitize，
  summary 按原始行追加；transcript-only reasoning 在 Raw 主面与 Rich compact 主面都隐藏。
  completion separator 继续保留并使用 cell raw contract；切换提示只复用既有 transient UI status，
  不改变 canonical Thread/Turn/Item。
- Ctrl+T retained transcript、Find、resume transcript 与 export 不消费 App raw mode，因此仍保留完整
  rich/detailed reasoning 和 activity disclosure；raw mode state 不进入 App Server、ThreadStore、
  runtime event、export 或第二套 history model。真实 PTY complete 场景新增 Markdown source marker：
  Rich 主面看不到字面 `**...**`，发送真实 `Esc+r` 后 Raw 主面看到 canonical source，同时 reasoning
  仍不可见；随后 Ctrl+T 继续看到 retained reasoning，证明 Rich/Raw 与 compact/detailed 是正交的
  presentation 维度。
- 分类：App session-local raw mode、`/raw`、全局 `Alt+R`、canonical raw source、五语言反馈及
  Rich/Raw 与 retained detail 边界均为 `current`；未新增 `compat`、`deprecated` 或 `dead`
  surface。Codex 可配置 keymap、local-settings 持久化、owned `TranscriptView` 的 compact/detailed
  双 position/bookmark 与所有 specialized cell 的完整 raw 文本形状仍为 `partial/contract-defer`；
  在 Lime 出现对应配置 owner 前不把 presentation 偏好写入 App Server 或项目配置。
- 文件尺寸退出条件：`app/history_ui.rs` 现为 `1295` 行且大部分为 inline 回归，`locale.rs` 为既有
  `3290` 行单一 i18n owner。本切片只在现有事实源接线；再次扩展 history composition 前先把
  `history_ui` 测试迁到专用测试模块，再添加新的业务分支。i18n 继续只允许在 `Locale` owner
  补产品文案，不新增第二套 locale catalog；后续若新增成组 presentation 文案，先拆 locale
  domain impl/test 模块并保持 `Locale` 为唯一 catalog。

验证：raw 定向回归 `17/17`；完整 TUI library `1010/1010`、integration `18/18`、manager
regression `1/1`；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、locked Cargo
metadata、结构 inventory `17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts`、
`npm run verify:app-version` 与 `git diff --check` 全部通过。真实 CLI 通过 `cargo build --locked -p
cli` 重建。complete-only Gate B 首次在提交前命中既有 external-editor composer cursor 时序抖动，
原样复跑通过：thread `01a0d2c9-3d56-7fa1-a2ba-5044bb563407`、turn
`turn_6218a351df1f4beb97ada2746230996d`。随后不裁剪场景的完整 Gate B 通过：thread
`01a0d2c9-9cc7-7fa3-bd38-9e3b68d02e55`、turn
`turn_ddbe2112aafb44cd9866ceb77dce6419`，事件链为
`turn.started,message.delta,item.started,item.completed,item.started,item.completed,turn.completed`，
并覆盖 raw source、retained reasoning、activity disclosure、selection/edge-drag、
`queue-edit=ok`、`agents-overview=ok`、`focus-palette=ok`、`resize-reflow=ok`、
`reconnect=ok`、`terminal=restored`。本轮仍未改 App Server protocol、Electron 或 GUI bridge，
因此不运行 `npm run test:contracts` 或 `npm run verify:gui-smoke`。总体计划保持 `in-progress`；
下一刀先执行上述 history composition 文件拆分退出条件，再继续 Codex owned `TranscriptView` 的
双 presentation position/bookmark，或推进下一个 A3/A2 明确缺口。

本轮 A2/A3 transcript compact/detailed 双阅读位置 current 切片（2026-09-24）：

- 已兑现上一刀的文件尺寸退出条件：`app/history_ui.rs` 的 inline tests 原样迁入
  `app/history_ui_tests.rs`，业务 owner 收缩到 `353` 行，专用测试模块为 `934` 行；拆分后
  `app::history_ui::tests` `23/23` 与后续 TUI 全量均通过，没有在超千行业务文件继续堆叠。
- 对照 Codex `transcript_view.rs::set_presentation`、`transcript_view/bookmark.rs` 与
  `search_preserves_both_presentation_positions`，新增短领域 owner
  `app/transcript_presentation.rs`。compact 位置继续由 `App::transcript_scroll` 唯一持有；Ctrl+T
  detailed pager 关闭后由同一 App session-local owner 保留 scroll/anchor、pinned 状态、reflow
  cache 与 activity disclosure 展开状态，重开不再强制跳回最新。presentation state 不进入
  App Server、ThreadStore、canonical projection、runtime event 或 export。
- detailed 关闭边界统一清除 search/query/matches、mouse/keyboard selection、drag、activity focus、
  copy feedback 与 history-load 短生命周期状态；展开 identity 与阅读位置保留。用户 Ctrl+T
  关闭、App Server disconnect、当前 thread reverse request 和 buffered request replay 共用同一
  dismiss owner，不再由分散的 `pager_overlay = None` 丢失 bookmark。同 thread reconnect 可恢复；
  `set_thread_id` 在 identity 变化时同时清 active/retained detailed state，禁止跨 thread 泄漏。
- 新增四组稳定回归，锁定 detailed close/reopen、compact/detailed 独立位置、search/selection 不
  复活、disclosure 展开保留但 focus 清除、thread switch 清 bookmark。真实 complete PTY 在
  edge-drag 后 Home 到 `TUI_EDGE_ROW_00`，关闭再 Ctrl+T 重开并再次观察同一 marker；Gate B
  source guard 同步锁定该交互，不以 reducer 或 TestBackend 冒充终端证据。
- 分类：App session-local compact/detailed reading position、detailed bookmark、统一 dismiss/reset
  lifecycle 与 PTY reopen 证据均为 `current`；未新增 `compat`、`deprecated` 或 `dead` surface。
  Codex 基于 retained cell identity/source offset 的完整 owned `TranscriptView` bookmark、搜索期间的
  双 presentation position 交换、可配置 keymap 与 local-settings 持久化仍为
  `partial/contract-defer`；Lime 当前 pager 以 logical-line/reflow anchor 保证现有 canonical
  projection 下的等价阅读位置，不伪造 Codex retained-cell store。

验证：transcript presentation 定向回归 `4/4`；完整 TUI library `1014/1014`、integration
`18/18`、manager regression `1/1`；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace
fmt、locked Cargo metadata、结构 inventory 与 Gate B guard `20/20`、`npm run governance:scripts`、
`npm run verify:app-version`、`git diff --check` 和 `cargo build --locked -p cli` 通过。真实
complete Gate B 通过：thread `01a0d2d9-87c0-7611-9824-4aa09aa6abcf`、turn
`turn_72882847eb48404e905fd52dcec6d24b`，事件链为
`turn.started,message.delta,item.started,item.completed,item.started,item.completed,turn.completed`，
并证明 detailed bookmark 重开与 terminal restore；证据目录
`tui-gate-b-bR3zxI` 保留。其余 approval/user-input/interrupt/failure/queue-edit/agents-overview
矩阵随后通过，代表 thread/turn 为 `01a0d2d9-fb7a-7020-a84d-d8cae18637a2` /
`turn_132bbea7aa5b4b0f9d00b40be306a211`，并覆盖 `queue-edit=ok`、`agents-overview=ok`、
`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok`、`terminal=restored`；证据目录
`tui-gate-b-z5n3t2` 保留。本轮未改 App Server protocol、Electron 或 GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。总体计划继续保持 `in-progress`；下一刀
回到 Codex owned transcript/history 对照，选择仍有真实产品影响且能由 Lime canonical 数据证明的
A2/A3 缺口，不把本切片提升为 TUI 总体完成。

本轮 A2/A3 transcript follow control / unseen activity current 切片（2026-09-24）：

- 对照 Codex `transcript_view/follow_control.rs`，新增 `transcript_view/follow_control.rs` 单一展示
  owner。`TranscriptViewport` 只持有 session-local 的 `tail_visible`、`unseen_activity` 与
  `suppress_next_activity`：用户暂停阅读时，canonical append 或尾部 revision 会显示
  `New activity`；older-history prepend 不误报；回到底部或 viewport 扩大至尾部可见时清除提示。
  Rich/Raw 切换只触发 presentation 重绘，并通过一次性 suppression 避免伪报新活动。
- follow control 占用 composer 顶部 gap，并按可用宽度选择完整或短标签；支持 hover、同一 hit target
  内按下/释放、拖出取消，并在隐藏时释放 hit target。主 transcript 已上滚时，`Escape` 优先回到底部，
  再考虑 interrupt 当前 turn；BottomPane、command/file/skill popup、model picker、Agents Overview 与
  agent picker 活跃时隐藏 control，避免覆盖现有交互 owner。thread identity 变化或 hydrate 会清理
  control，不把旧 thread 的展示状态带入新投影。
- 用户可见标签和 unseen 文案覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR`；英文稳定形状为
  `↓ Back to bottom · esc` 与 `New activity · ↓ Back to bottom · esc`。新增定向回归覆盖 unseen
  生命周期、older-history、revision、reflow/tail visibility、窄终端、hover/click/drag-cancel、popup
  隐藏与 Escape 路由。
- 真实 PTY 暴露 external editor 恢复后输入流竞争：删除 Lime `runtime.rs` 中不属于 Codex owner 的
  `pending_tui_event` 和 `with_restored()` 后 eager `poll_crossterm_event`。恢复后统一由 async event
  loop 重建并轮询输入源，避免旧 crossterm reader 退出与新同步 poll 竞争、把 transient EOF 当成
  主输入流终止。Gate B composer 鼠标回归改为从可见 prompt 定位确定性行列、等待 prompt 末尾光标，
  用 Left/Right 事件确认输入循环恢复，再分别发送 SGR mouse down/up；不使用固定 sleep。源码守卫
  明确禁止 `poll_crossterm_event` 回流。
- 分类：follow/unseen、control hit target、Rich/Raw suppression 与 external-editor 输入恢复均为
  `current`；未新增 `compat`、`deprecated` 或 `dead` surface。follow/unseen 只属于 TUI local
  presentation，不进入 App Server、ThreadStore、canonical projection、runtime event 或持久化。
  Codex owned TranscriptView 的其余 bookmark/search/keymap 细节及 A2/A3 其他缺口继续为
  `partial/contract-defer`，不为本地 control 扩展协议事实。

验证：follow control 定向回归 `2/2`、transcript reflow 定向回归 `9/9`、locale/view follow 集成
`3/3`；完整 TUI library `1022/1022`、integration `18/18`、manager regression `1/1`、event stream
定向 `10/10`；TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、locked Cargo
metadata、结构 inventory `17/17`、Gate B 源码守卫 `3/3`、`npm run governance:scripts` 与
`npm run verify:app-version` 全部通过。CLI 与 App Server 使用仓库 `resolveRustyV8CargoEnv` 成功重建；
直接 Cargo 构建曾因上游 Deno `rusty_v8` URL 返回 404 失败，不属于产品路径失败。完整真实 TUI
Gate B 矩阵通过：thread `01a0d2f8-0267-7be1-8a77-bb8b41b3c091`、turn
`turn_0f5e9038c6bc4b0b8bc747a23d66a15c`，覆盖 complete、approval、user-input、interrupt、failure、
queue-edit、agents-overview、focus-palette、resize-reflow、reconnect 与 terminal restore；证据目录
`tui-gate-b-bt5T2X` 保留。本轮未触及 App Server protocol、Electron 或 GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。该切片已达到 TUI reducer/render/composer 与
真实 PTY 的风险匹配门槛，但总体计划仍为 `in-progress`：顶层 checklist 保持 `21/50（42%）`，
下一刀继续选择可由 canonical 数据证明的 A2/A3 缺口，不把 follow control 收口提升为 TUI 总体完成。

本轮 A2/A3 main transcript selection / composer gap current 切片（2026-09-24）：

- 对照 Codex `transcript_view/{selection,input,composer_gap}.rs`，主 compact transcript 现在直接复用
  Lime 唯一 `TranscriptSelection` owner，不再只有 Ctrl+T detailed pager 和 resume pager 可选择。
  主面支持 SGR 鼠标单/双/三击与拖选、右键复制、`Ctrl+Space` 键盘选择、grapheme-aware 方向键、
  PageUp/PageDown、edge-drag、stationary hyperlink release 和 modifier link open；事件优先级位于
  composer copy/edit 之前，但 popup、BottomPane、picker 等更具体 surface 仍先消费输入。
- 选择开始时冻结当帧 `HyperlinkLine` source 与 visual scroll；canonical projection 继续实时更新。
  `TranscriptViewport::resolve_frozen_anchor` 独立跟踪 canonical logical row，使 tail append/revision
  产生 unseen activity、older-history prepend 不误报，并在宽度 reflow 后保持恢复锚点。选择期间
  只绘制 frozen source；结束后按最新 canonical max scroll 恢复同一阅读位置，不把 source snapshot
  写入 history store。鼠标 wheel 走新的精确三行 runtime action，并在向上时继续触发现有真实
  App Server history page 请求，不建立本地历史 fallback。
- clipboard 结果保持 truth-aware：只有 `CopyStatus::Confirmed` 清除选择；普通 copy 保留阅读位置，
  Enter copy-and-follow 回到最新；terminal `Unconfirmed` 与失败都保留选择。新增 Codex-shaped
  `transcript_view/composer_gap.rs`，在 composer 顶部 gap 右对齐显示确认/未确认/失败反馈，复用
  `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR` 既有 copy 文案；反馈显示期间释放 follow-control
  hit target，并由现有 `FrameRequester` 在五秒 expiry 触发恢复，不使用 sleep 或 projection status
  作为计时事实源。
- thread identity 变化、hydrate 会 reset 主选择与 copy feedback；Rich/Raw 切换和打开 detailed
  transcript 会先结束选择并保留 compact 阅读位置。selection、frozen source、resume distance、
  feedback deadline 与 follow-control target 全部属于 TUI session-local presentation，不进入 App
  Server、ThreadStore、canonical Thread/Turn/Item、runtime event、export 或持久化。
- 分类：主 transcript mouse/keyboard selection、frozen/canonical 双锚点、truth-aware copy、composer
  gap feedback、wheel history paging 和真实 PTY interaction 均为 `current`；未新增 `compat`、
  `deprecated` 或 `dead` surface。Codex retained-cell/source-offset 完整 `TranscriptView`、sticky
  prompt header、main Find/footer/keymap 与 local-settings persistence 仍为 `partial/contract-defer`，
  下一刀继续从这些可证产品缺口中选择，不以本次选择接线宣称 owned TranscriptView 已完整同构。

验证：`transcript_view` owner 回归 `28/28`、main transcript 定向 `5/5`；完整 TUI library
`1028/1028`、integration `18/18`、manager regression `1/1`；TUI `--all-targets --no-deps`
Clippy `-D warnings`、workspace fmt、locked Cargo metadata、结构 inventory `17/17`、Gate B 源码守卫
`3/3`、`npm run governance:scripts` 与 `npm run verify:app-version` 全部通过。结构 inventory 已按
当前 Codex commit `5c07856e25b565c86c6266ff51edff58299fd4e6` 刷新为 `1191` files；CLI 与 App
Server 使用仓库 `resolveRustyV8CargoEnv` 成功重建，编译仅有 App Server 既有
`lower_turn_start_params`/`lower_runtime_options` dead-code warning。complete-only TUI Gate B
先通过：thread `01a0d30f-10f7-77f1-8eea-50ab1ef7d739`、turn
`turn_46c582a696a542aba5c59ff6f7c8dc97`，证据目录 `tui-gate-b-PCwAkb` 保留。当前源码对应的
完整矩阵随后通过：thread `01a0d312-9420-70c0-821c-98f14f272902`、turn
`turn_289f9ac04ee94dc49031f2a20efe3fdb`，覆盖主 compact transcript SGR 选择、complete、approval、
user-input、interrupt、failure、queue-edit、agents-overview、focus-palette、resize-reflow、reconnect
与 `terminal=restored`；证据目录 `tui-gate-b-ecTfBp` 保留。本轮未触及 App Server protocol、
Electron 或 GUI bridge，因此不运行 `npm run test:contracts` 或 `npm run verify:gui-smoke`。该切片
达到 Rust TUI reducer/render/composer 与真实 TUI Gate B 风险门槛；总体计划仍为 `in-progress`，
顶层 checklist 保持 `21/50（42%）`，不能据此标记“完整对齐 TUI”完成。

本轮 A2/A3 sticky prompt header current 切片（2026-09-24）：

- 对照 Codex `transcript_view/{prompt_header,prompt_header_tests}.rs`，新增
  `transcript_view/prompt_header.rs` 唯一 presentation owner。主 compact transcript composition
  现在同时返回原始 `HyperlinkLine` 与 canonical entry logical-line ranges；metadata 直接来自
  `TranscriptEntry.kind/id/text/summary`，不从 `› ` 等渲染前缀反推 User prompt，也不建立第二套
  history/transcript model。header 只绘制在保留的顶部一行，不进入 selection source、copy、search、
  export、ThreadStore 或 App Server projection。
- sticky prompt 在宽度 `<16`、当前首个可见 cell 本身是可见 User prompt、没有前序可见 prompt 或
  transcript 高度 `<4` 时隐藏；否则选择前方最近的 canonical User prompt。读取最多 512 chars，复用
  `sanitize_user_text` 清 terminal controls、折叠 whitespace，并按 display width 省略。附件-only
  fallback 在 `en-US` 保持 Codex `[attachments]`，同时覆盖 `zh-CN`、`zh-TW`、`ja-JP`、`ko-KR`。
  样式由 `history_prompt_style()` 统一承接。
- header reservation 使用未提交的 viewport preview 做两阶段布局，最终只向
  `TranscriptViewport` commit 一次；当保留 header 后下一 turn 的 User prompt 恰好进入 viewport，
  suppression 以 outer viewport、canonical entry key 和 entry 内 wrapped row 为键固定该边界，直到
  scroll 或 resize。selection 活跃时复用与 frozen `HyperlinkLine` 配对的 prompt metadata，hit-test
  area 从 header 下方开始；普通 copy 结束后继续保持阅读位置和 header reservation。
- older page 在稳定 session header 之后插入时，`TranscriptViewport` 现在识别“公共前缀 + 全部旧后缀”
  的纯插入并重映射逻辑行；仅当插入点位于当前阅读锚点之前时抑制 unseen activity。真实 tail
  append/revision 仍产生 `New activity`，避免用宽泛的内容相等判断吞掉新输出。
- 分类：sticky prompt、canonical entry range mapping、turn-boundary suppression、selection header
  reservation 与 stable-prefix history prepend remap 均为 `current`；未新增 `compat`、`deprecated`
  或 `dead` surface。架构仍是 `CLI/TUI Host -> App Server JSON-RPC -> RuntimeCore -> canonical
  Thread/Turn/Item -> terminal projection`，未改变 public boundary，因此不更新架构图。
- 聚合文件退出约束：`view.rs` 已超过 1000 行；本切片把 header 决策放在独立
  `transcript_view/prompt_header.rs`，`view.rs` 只保留 composition/viewport wiring。下次继续扩展
  `view.rs` 测试前，先把其 `#[cfg(test)]` 大模块迁至独立测试文件；禁止继续向生产 render 聚合新的
  transcript 状态机。

验证：prompt-header/主视图定向回归 `8/8`、viewport 定向 `7/7`；related Rust unit 为 CLI
`8/8`、TUI library `1037/1037`，integration `18/18`、manager regression `1/1`；TUI
`--all-targets --no-deps` Clippy `-D warnings` 通过。Gate B 源码守卫 `3/3` 通过，完整真实 TUI
Gate B 通过：thread `01a0d32f-f113-7473-b72a-e6a61fdd8ceb`、turn
`turn_8f54439f06d9449c969f41d89aaf44b3`，覆盖 sticky prompt 顶行、主 transcript SGR selection、
complete、approval、user-input、interrupt、failure、queue-edit、agents-overview、focus-palette、
resize-reflow、reconnect 与 `terminal=restored`；证据目录 `tui-gate-b-zI56wF` 保留。结构 inventory
按 Codex `5c07856e25b565c86c6266ff51edff58299fd4e6` 刷新为 `1193` files：Codex TUI src
`997 files / 14991 symbols`，Lime TUI src `196 files / 3538 symbols`，missing 降至
`835 files / 11684 symbols`。本轮未触及 App Server protocol、Electron 或 GUI bridge，因此不运行
`npm run test:contracts` 或 `npm run verify:gui-smoke`。该切片达到 Rust TUI
reducer/render/composer 与真实 TUI Gate B 风险门槛；总体计划仍为 `in-progress`，顶层 checklist
保持 `21/50（42%）`，remaining blocker 仍是 retained-cell `TranscriptView` 其余
bookmark/search/keymap、A2/A3 owner、CLI partial 与 Cloud transport，不把 prompt header 收口提升为
“完整对齐 TUI”完成。

本轮 A2/A3 main transcript Find / footer current 切片（2026-09-24）：

- 对照 Codex `transcript_view/{search,footer}.rs`，新增 Lime 唯一共享 owner
  `transcript_view/search.rs` 与 `transcript_view/footer.rs`。主 compact transcript 当前以 `F3` 打开
  Find；detailed pager 支持 `F3` 与 `/`，`Ctrl+F` 恢复为向下翻页；原先 pager 内联的
  query/match/navigation 状态已
  迁出，不再维护第二套搜索实现。query 上限为 `4096 bytes`，每个扫描窗口至多 `16 KiB`，每帧推进
  至多 `8` 条 logical lines；Unicode 大小写不敏感 literal matching 保留源 UTF-8 byte offset。
- Enter / `Ctrl+N` 定位更新命中，Shift+Enter / `Ctrl+P` 定位更旧命中；Esc / `Ctrl+C` 关闭。
  搜索当前加载 projection 耗尽时只发出既有 `LoadOlderHistory`，继续经过 App Server history pager；
  failed page 可由后续导航重试，没有本地 loader、store 或 synthetic history fallback。main Find 打开
  前保存 compact `transcript_scroll`，关闭后恢复；source prepend、revision 与 resize 会重启 bounded
  scan，并按匹配文本和 source offset 尽量保留当前命中。
- selection 活跃时暂停增量搜索，selection 高亮优先于 Find 高亮；搜索高亮按原 Span/grapheme
  拆分，保留既有样式和 OSC 8 hyperlink geometry。query footer 占用 composer gap，普通 footer
  展示 match/loading/failure/selection hint；文案覆盖 `zh-CN`、`zh-TW`、`en-US`、`ja-JP`、`ko-KR`。
  `view.rs` 只保留 source/viewport/footer 组合接线，search 状态机和专用测试均位于短领域 owner，
  未继续向超千行 render 文件堆叠业务逻辑。
- 真实 PTY complete 场景在 canonical `turn.completed` 后发送 F3，输入
  `TUI_GATE_B_COMPLETED`，断言 `Find: TUI_GATE_B_COMPLETED`、主 transcript 匹配区域 inverse
  highlight、sticky prompt 仍固定第 0 行，并在 Esc 后确认高亮清除。该证据经过真实 `lime`、PTY、
  alternate screen、键盘输入、App Server JSON-RPC、RuntimeCore/read model 与终端恢复，不以
  TestBackend 或 source-string guard 冒充 TUI Gate B。
- 分类：bounded shared search、main Find、pager 委托、history pager 联动、阅读位置恢复、selection
  priority 与 footer 均为 `current`；未新增 `compat`、`deprecated` 或 `dead` surface。状态只属于 TUI
  session-local presentation，不进入 App Server protocol、ThreadStore、canonical Thread/Turn/Item、
  export 或持久化。Codex retained-cell/source-offset 的完整 `TranscriptView` bookmark、可配置 keymap
  与 local-settings persistence，以及其余 A2/A3 owner 仍为 `partial/contract-defer`。

验证：shared search owner `7/7`、pager search `6/6`、main Find TestBackend `2/2`、footer `2/2`、
pager overlay `26/26`；最终完整 TUI library `1048/1048`、integration `18/18`、manager regression
`1/1`，related Rust 依赖扩展后 CLI `8/8` + TUI `1048/1048`。TUI `--all-targets --no-deps`
Clippy `-D warnings`、workspace fmt、locked Cargo metadata、Gate B 源码守卫 `3/3`、
`npm run governance:scripts` 与 `git diff --check` 通过。结构 inventory 按 Codex
`5c07856e25b565c86c6266ff51edff58299fd4e6` 刷新为 `1197` files：Codex TUI src
`997 files / 14991 symbols`，Lime TUI src `200 files / 3603 symbols`，missing 为
`831 files / 11675 symbols`。

保留的 complete-only Gate B 通过：thread `01a0d381-d82c-7242-95e0-b837533cfdfb`、turn
`turn_5155a21ceb0b407dae7b0ba97a70c365`，证据目录 `tui-gate-b-8TEPgr`。保留证据的完整矩阵首次
在既有 `resize-reflow` 用例命中 5 秒退出超时，失败目录 `tui-gate-b-0upaET` 保留；同一入口复跑
通过：thread `01a0d383-26a0-7a02-8cfb-8e38e6d939c3`、turn
`turn_a312ba6761bb4be8bf4b225f049bc8b7`，并覆盖 `queue-edit=ok`、`agents-overview=ok`、
`sticky-prompt=ok`、`main-find=ok`、`focus-palette=ok`、`resize-reflow=ok`、`reconnect=ok` 与
`terminal=restored`；证据目录 `tui-gate-b-OFAtku` 保留。编译仅出现 App Server 既有
`lower_turn_start_params` / `lower_runtime_options` dead-code warning。本轮未改 App Server protocol、
Electron 或 GUI bridge，因此不运行 `npm run test:contracts` 或 `npm run verify:gui-smoke`。
该切片达到 Rust TUI reducer/render/composer 与真实 TUI Gate B 风险门槛；总体计划仍为
`in-progress`，顶层 checklist 保持 `21/50（42%）`。下一刀审计 Codex owned TranscriptView 的
keymap/local-settings persistence 与 retained-cell bookmark 边界，或继续下一个可由 Lime canonical
数据证明的 A2/A3 缺口；不得把 Find/footer 收口提升为“完整对齐 TUI”完成。

本轮 A2/A3 transcript 默认 keymap 纠偏切片（2026-09-24）：

- 重新对照当前 Codex commit `5c07856e25b565c86c6266ff51edff58299fd4e6` 的
  `tui/src/keymap.rs`、`keymap/global_find_tests.rs` 与 `app/owned_transcript.rs`，确认上个切片采用的
  `Ctrl+F` Find 默认值已偏离当前 Codex：global `find_transcript` 应为 `F3`，pager `find` 应为
  `F3`/`/`，pager `page_down` 应为 `PgDn`/`Space`/`Ctrl+F`。本切片直接替换错误默认值，不保留
  `Ctrl+F -> Find` compat 双轨。
- `tui/src/keymap.rs` 新增短领域 `TranscriptKeymap` owner。`open_transcript`、`find_transcript`、
  pager `find/page_down/close/close_transcript` 的匹配与显示标签都从同一组 binding slices 生成；
  `app/input_flow.rs`、`app/interaction.rs` 与 `pager_overlay.rs` 不再各自比较固定键。C0 控制字符归一
  仍覆盖真实 PTY 发送的 Ctrl chords，不建立第二套输入 registry。
- 主 compact transcript 只由 `F3` 打开 Find；`Ctrl+F` 继续交给 composer editor。在 detailed
  transcript 中，`F3` 与 `/` 打开 Find，`Ctrl+F`、Space 和 PgDn 均向下翻一页。BottomPane、
  resume/agents/model pickers 与 composer popup 仍按既有优先级先消费事件，Find 不抢更具体 context。
- detailed transcript 的普通 footer 与 activity footer 都从 `TranscriptKeymap` 读取
  `PgDn·Space·Ctrl+F`、`F3·/`、`Ctrl+T·Esc·Q` 标签；五语言只负责动作文案格式化。该变更保持
  单行、低装饰、信息优先的 terminal footer，不引入新颜色、层级或额外面板。
- 真实 PTY complete 场景改为发送 xterm F3 序列 `ESC O R`；源码守卫禁止继续寻找 Ctrl-F Find。
  第一次复测由 activity footer 仍显示旧的硬编码 close 标签暴露同源遗漏，失败证据目录
  `tui-gate-b-ySMydY` 保留；补齐 activity footer 后 complete-only 与完整矩阵均通过。
- 分类：上述默认 bindings、dispatcher、pager 翻页语义、五语言 hint 与 PTY F3 evidence 均为
  `current`；未新增 `compat` 或 `deprecated`。旧 `Ctrl+F -> Find` 为 `dead / deleted /
  forbidden-to-restore`，只允许出现在本计划历史说明或负向回归中。持久化 `tui.keymap`、两段 chord、
  explicit unbind、冲突校验、runtime snapshot 与 `local_settings` 仍为 `partial/contract-defer`；后续若
  实现必须完整同步 core config schema、App Server `config/read`、TUI startup consumer、文档与合同，
  不得使用环境变量或 TUI 私有配置文件建立第二套事实源。

验证：keymap/default-hint 定向 `1/1`、main Find `3/3`、transcript pager `7/7`；完整 TUI library
`1052/1052`、integration `18/18`、manager regression `1/1`，related Rust 依赖扩展后 CLI `8/8` +
TUI `1052/1052`。TUI `--all-targets --no-deps` Clippy `-D warnings`、workspace fmt、locked Cargo
metadata、结构/snapshot/Gate 守卫 `22/22`、`npm run governance:scripts` 与 `git diff --check` 通过。
带依赖的 `cargo clippy -p tui --all-targets -- -D warnings` 被既有 `agent-protocol` 的
`large_enum_variant` 与 `derivable_impls` 两条 lint 阻断；本切片未修改该 crate，TUI 自身 Clippy
全绿。结构 inventory 刷新为 `1197` files：Codex TUI src `997 files / 14991 symbols`，Lime TUI src
`200 files / 3623 symbols`，missing 为 `831 files / 11672 symbols`。

保留的 complete-only Gate B 通过：thread `01a0d39e-820f-7a51-8085-029862909b37`、turn
`turn_5e92839be5044630b0a018f5a7774253`，证据目录 `tui-gate-b-oK7ZgA`。当前源码的完整矩阵通过：
thread `01a0d3a1-99fb-7d52-8727-fd7db689c42c`、turn
`turn_4320bfa14b85426780b3448d8bd9d815`，覆盖 complete、approval、user-input、interrupt、failure、
queue-edit、agents-overview、真实 F3 `main-find=ok`、focus-palette、resize-reflow、reconnect 与
`terminal=restored`；证据目录 `tui-gate-b-vlmCRz` 保留。编译仅出现 App Server 既有
`lower_turn_start_params` / `lower_runtime_options` dead-code warning。本轮未改 App Server protocol、
配置 schema、Electron 或 GUI bridge，因此不运行 `npm run test:contracts` 或
`npm run verify:gui-smoke`。该切片达到 Rust TUI interaction 与真实 TUI Gate B 风险门槛；总体计划
仍为 `in-progress`，顶层 checklist 保持 `21/50（42%）`。下一刀优先完整接入 Codex-shaped
`tui.keymap` runtime snapshot/local-settings 边界，或选择 retained-cell bookmark；不得把默认键纠偏
提升为“完整对齐 TUI”完成。
