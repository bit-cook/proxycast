# 生产运维与上线就绪清单

本文档用于生产环境部署、运行、备份与恢复的最小操作规范，避免上线后缺少可执行流程。

## 部署前检查

- 确认 API Key 已更换（禁止使用默认值 `proxy_cast`）。
- 确认监听地址：
  - 本机使用 `127.0.0.1`/`localhost`。
- 当前版本仅支持本地监听，不支持对外服务。
- 若需要 HTTPS，请使用反向代理终止 TLS；当前服务端未启用内置 TLS。
- 确认磁盘权限可写：`~/.lime/`、`~/.lime/request_logs/`、应用数据目录（macOS: `~/Library/Application Support/lime/`，Linux: `~/.local/share/lime/`，Windows: `%APPDATA%\\lime\\`）。

## 配置路径与加载顺序

- YAML 配置（优先）：
  - macOS: `~/Library/Application Support/lime/config.yaml`
  - Linux: `~/.config/lime/config.yaml`
  - Windows: `%APPDATA%\\lime\\config.yaml`
- JSON 配置（兼容）：macOS `~/Library/Application Support/lime/config.json`，Linux `~/.config/lime/config.json`，Windows `%APPDATA%\\lime\\config.json`
- 旧版遗留路径：`~/.lime/config.json`（检测到会提示手动迁移）
- 两者都不存在时使用默认配置。
  - 首次启动会自动生成强随机 API Key 并写入配置。

### TUI 按键配置

TUI 只从上述 Lime 用户配置读取 `tui.keymap` 与 `tui.right_click_paste`。启动时会经 App Server
`config/read` 生成不可变 runtime snapshot；修改配置后需重启当前 TUI 进程。不要创建 TUI 专用
配置文件或按键环境变量。`right_click_paste` 支持 `auto`（默认，遵循 SSH/WSL/VS Code 安全护栏）、
`on` 和 `off`；中键 PRIMARY 仅在本地 X11 可用时启用。

```yaml
tui:
  right_click_paste: auto
  keymap:
    global:
      find_transcript: ctrl-x f
    pager:
      page_down: [page-down, space, ctrl-f]
      find: [f3, /]
    agents:
      resume: []
```

每个 action 可使用单个按键、按优先级排列的数组、最多两段且以空格分隔的 chord，或用空数组
显式解除绑定。当前 context/action 为：

- `global`：`open_agents`、`open_transcript`、`find_transcript`
- `pager`：`scroll_up`、`scroll_down`、`page_up`、`page_down`、`half_page_up`、
  `half_page_down`、`jump_top`、`jump_bottom`、`close`、`close_transcript`、`find`
- `agents`：`resume`、`search`、`new_task`、`rename`、`stop`、`toggle_grouping`

Agent Center 当前默认键位与 Codex 一致：`o` 恢复、`f` 搜索、`n` 新建、`r` 改名、`x` 停止、
`g` 切换分组；`Tab/Shift+Tab` 切换状态标签，`PageDown/Ctrl+F`、`PageUp/Ctrl+B` 按可见行
翻页，`?` 查看已接线快捷键。metadata 输入时可打印字符只用于编辑，不触发任务动作；
自定义 bindings 优先，显式空数组不会回退默认键位，footer/help 使用同一配置 snapshot。

修饰键使用 `ctrl-`、`alt-`、`shift-`；支持 ASCII 字符、`f1` 至 `f24` 及常见命名键。
未知字段、非法键名、过长 chord、同 context 重复绑定、single/chord prefix 冲突和普通可打印字符
chord prefix 都会被拒绝。composer/editor/Vim 尚未接入该配置面。

## 数据与日志位置

- SQLite 数据库：`~/.lime/lime.db`
- 日志目录：`~/.lime/logs/`
- 请求日志目录：`~/.lime/request_logs/`
- 数据库备份目录：`~/.lime/backups/`
- 旧凭证池副本目录与旧 OAuth/Token 目录不属于 current 常规备份面；启动期会清理 Lime 管理的旧凭证池副本。如需法务或人工排障留存，先离线加密归档，再启动新版本。

## 备份与恢复

### 备份

1. 可使用管理端点触发备份（需配置管理密钥）：
   - `POST /v0/management/backup`
2. 或手动备份（建议停服后执行）：
   - 复制以下路径：
   - 配置文件（macOS: `~/Library/Application Support/lime/config.yaml`，Linux: `~/.config/lime/config.yaml`，Windows: `%APPDATA%\\lime\\config.yaml`）
   - 配置备份文件：`config.yaml.backup`
   - `~/.lime/lime.db`
   - `~/.lime/logs/`、`~/.lime/request_logs/`（如需保留日志）
3. 将备份文件存入受控存储（加密磁盘或安全存储）。

### 自动备份

- 服务运行期间每 24 小时自动创建数据库备份到 `~/.lime/backups/`。
- 备份默认保留 7 天，过期文件会被清理。

### 恢复

1. 停止 Lime 服务。
2. 使用管理端点恢复（建议停服后执行，执行时会锁定数据库并短暂阻塞请求）：
   - `POST /v0/management/restore`，请求体：`{"backup_path": "/path/to/lime_YYYYMMDD_HHMMSS.db"}`
3. 或手动恢复上述文件到原路径。
4. 启动服务并检查 `/health` 与 `/ready`。

## 升级与回滚

- 升级前执行备份流程。
- 升级后若出现异常：
  - 恢复备份文件。
  - 回滚到上一个稳定版本的安装包。

## 运行与排障

- 健康检查：`GET /health`
- 就绪检查：`GET /ready`
- 常见问题排查：
  - 端口占用：修改配置端口或释放占用端口。
  - 配置解析失败：检查 YAML/JSON 语法，确认缩进正确。
  - 数据库初始化失败：检查 `~/.lime/` 权限与磁盘空间。

## 安全基线

- 禁止默认 API key。
- 当前版本未实现内置 TLS，远程管理必须保持关闭且仅本地访问。

## 管理 API 基线

- 管理 API 启用后会对失败认证进行短期限制，避免暴力尝试。
- 建议仅在内网使用，并配合独立强密钥。
