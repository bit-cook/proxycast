# TUI/CLI 继续同步 Codex 执行计划

状态：planned
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
| TUI Rust 文件 | 579 | 91 | 目录体系仍未同构 |
| TUI 类型/函数符号 | 12,000 | 1,491 | 结构差异大，须按 owner 分批收敛 |
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
- `packages/cli/**`、`pnpm-lock.yaml`、`lime-rs/Cargo.toml`、`lime-rs/Cargo.lock`
- `scripts/app-server/{tui,cli}-*.mjs` 及对应测试、TUI/CLI inventory 生成器和账本
- `.gitignore`、`internal/exec-plans/README.md`、本计划和差异报告

### 明确避让

`electron/**`、无关 `src/**`、Codex Desktop/Goose 计划热区、服务端外部仓库、
`lime-rs/crates/agent/**` 和已有未跟踪产物。若必须改变 App Server protocol，先单独登记
contract 写集、消费者、schema、锁文件和测试，不在 TUI 批次夹写。

## 4. 阶段 A：TUI Codex owner 同构

### A0 迁移准备

- [ ] 为每个批次建立 `upstream-path -> lime-owner -> classification -> test` 映射，来源必须
  是 Codex 当前 checkout，不得只依据文件名猜测。
- [ ] 复核 `tui-structure-inventory.json` 和 `tui-codex-snapshot-inventory.json` 的 hash、
  source commit 与分类；新增 upstream 文件必须先分类再写代码。
- [ ] 维护唯一 owner 表，确认 `projection.rs`、`runtime.rs`、`view.rs` 等 Lime 聚合 owner
  的拆分边界；不创建第二个 Thread/Turn/Item 模型。

### A1 纯终端 direct owner

对应 Codex：`render/`、`streaming/`、`markdown_render/`、`diff_render`、`insert_history`、
`terminal_hyperlinks`、`terminal_palette`、`table_detect`、`wrapping`。

- [ ] 按 Codex 文件名补齐 `tui/src/render/` 的 `mod.rs`、`line_utils.rs`、`renderable.rs`、
  `highlight.rs` 和 streaming highlight；把 Lime `highlight.rs`/`view.rs` 中对应纯算法迁入
  唯一 owner。
- [ ] 按 Codex 真实测试名迁移 48 个 `direct` snapshot 场景，使用 `insta` 或等价
  `TestBackend/Buffer` 断言；不引入 Codex runtime 状态。
- [ ] 对 `markdown_render`、`diff_render`、`wrapping`、OSC 8 和宽字符边界补窄终端、UTF-8
  grapheme、URL、表格、重排回归。
- [ ] 删除迁移后的 Lime-only 重复纯渲染函数，更新 inventory 并增加禁止重复 owner 的结构守卫。

退出条件：48 个 direct snapshot 全部有 Lime owner 和稳定断言；`cargo test -p tui`、
TUI Clippy、结构 inventory 和 TUI Gate B 通过。

### A2 transcript/history owner

对应 Codex：`history_cell/`、`exec_cell/`、`app/history_ui.rs`、`app/history_pagination.rs`、
`app/transcript_export.rs`、`pager_overlay/`。

- [ ] 建立 `history_cell` 的 canonical entry adapter，只消费 App Server `Thread/Turn/Item`
  projection；按 Codex 文件拆分 approvals、exec、MCP、patches、plans、messages、notices、
  session 和 request_user_input。
- [ ] 建立 `exec_cell/{mod,model,live_output,render}.rs`，把 command live output 与 Lime
  `entry.rs` 的重复渲染收回同一 owner。
- [ ] 将 `Ctrl+T` transcript overlay、resume transcript、pager 和 export 统一到
  `history_cell`/`pager_overlay` 的渲染链；不创建 rollout/history DB。
- [ ] 迁移 `merge=579` 中与 history/transcript/pager 相关场景；跨 runtime 的 80 个
  `contract` 场景必须绑定 App Server fixture 和 canonical identity。

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

- [ ] 把 `viewport.rs` 接入 runtime resize/reflow 主路径，使用实际 Ratatui rendered rows，
  覆盖宽度变化、底部对齐、手动滚动和 alternate-screen round trip。
- [ ] 迁移 Codex startup/session/working-directory 语义到 App Server contract；没有 Lime
  current contract 的字段明确标为 `contract/defer`，不能在 TUI 本地合成。
- [ ] 补齐 `tests/suite/reconnect.rs`、`resize_reflow.rs`、`status_indicator.rs`、
  `focus_palette.rs` 的 Lime 版本；顶层 `tui/src/reconnect.rs` 在零消费者后删除。
- [ ] 把 terminal lifecycle 继续固定在 `Tui`/`Terminal`/`EventBroker`/`FrameRequester`，
  不恢复 `TerminalGuard` 或 runtime EOF 重启。

退出条件：TUI resize/reflow、focus、reconnect 和 terminal restore 通过真实 PTY/VT100；
旧 reconnect 壳和旧输入恢复路径不存在 current 引用。

## 5. 阶段 B：TUI 测试体系同构

- [ ] 新增 Codex-shaped `lime-rs/crates/tui/tests/`：`all.rs`、`test_backend.rs`、
  `manager_dependency_regression.rs`、`suite/mod.rs`、`suite/focus_palette.rs`、
  `suite/reconnect.rs`、`suite/resize_reflow.rs`、`suite/status_indicator.rs`、
  `suite/vt100_history.rs`、`suite/vt100_live_commit.rs`、`fixtures/oss-story.jsonl`。
- [ ] 从 Codex 测试复制测试结构和测试名，仅替换 Lime App Server fixture、canonical
  projection 和五语言文案；不得把 Codex 私有 backend/state DB 带入测试。
- [ ] 引入 `insta`、`serial_test`、`assert_matches` 等依赖前先确认只用于 TUI test target，
  同步 Cargo.lock 和最小结构守卫。
- [ ] 对 802 个 snapshot 保持逐项账本：`direct` 迁移、`merge` 合并、`contract` 绑定真实
  protocol、`defer` 保留退出条件、`dead` 禁止复制。

退出条件：TUI 集成目录、snapshot、TestBackend、VT100 history/live commit 和 PTY Gate B
同时通过；账本无未分类条目，且测试失败不会回退到 mock backend。

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

下一执行批次：**A0 + A1（纯终端 direct owner 与 48 个 direct snapshot）**。完成后再进入
A2 transcript/history，不并行扩张 Cloud 或复制 Codex 产品专属模块。
