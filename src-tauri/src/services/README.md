# services

<!-- 一旦我所属的文件夹有所变化，请更新我 -->

## 架构说明

业务服务层，封装核心业务逻辑。
提供凭证池管理、Token 缓存、MCP 同步等功能。

## 文件索引

- `mod.rs` - 模块入口
- `provider_pool_service.rs` - Provider 凭证池服务（多凭证轮询）
- `token_cache_service.rs` - Token 缓存服务
- `mcp_service.rs` - MCP 服务器管理
- `mcp_sync.rs` - MCP 配置同步
- `prompt_service.rs` - Prompt 管理服务
- `prompt_sync.rs` - Prompt 同步
- `skill_service.rs` - 技能管理服务
- `usage_service.rs` - 使用量统计服务
- `backup_service.rs` - 备份服务
- `live_sync.rs` - 实时同步服务
- `switch.rs` - 开关服务
- `sysinfo_service.rs` - 系统信息服务（CPU/内存监控）
- `file_browser_service.rs` - 文件浏览器服务（目录列表、文件预览）
- `general_chat/` - 通用对话服务模块（会话管理、消息存储）
- `api_key_provider_service.rs` - API Key Provider 服务
- `kiro_event_service.rs` - Kiro 事件服务
- `machine_id_service.rs` - 机器 ID 服务
- `model_registry_service.rs` - 模型注册表服务
- `update_check_service.rs` - 自动更新检查服务（每日检查、系统通知）
- `update_window.rs` - 更新提醒独立窗口管理

## 更新提醒

任何文件变更后，请更新此文档和相关的上级文档。
