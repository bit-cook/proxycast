## ProxyCast v0.72.0

发布日期：2026-02-26

### ✨ 新功能

#### 渠道管理重构
- 重写渠道设置页面：移除旧的「AI 模型提供商」和「消息通知渠道」双 tab 布局，改为 Telegram / Discord / 飞书 三个 Bot 渠道 tab，每个 tab 内联表单配置
- 新增后端 ChannelsConfig 类型：在 Rust 配置层新增 `ChannelsConfig`、`TelegramBotConfig`、`DiscordBotConfig`、`FeishuBotConfig` 结构体，支持 YAML 序列化/反序列化
- Telegram Bot 配置：支持 Enable 开关、Bot Token（密码输入+显示切换）、允许的用户 ID 列表、默认模型选择
- Discord Bot 配置：支持 Enable 开关、Bot Token、允许的服务器 ID 列表、默认模型选择
- 飞书 Bot 配置：支持 Enable 开关、App ID、App Secret、Verification Token（可选）、Encrypt Key（可选）、默认模型选择
- 默认模型选择器：复用现有 Provider Pool 数据，下拉列出所有已配置 Provider 的模型
- 脏状态检测：修改表单后底部固定栏显示「未保存的更改」提示，支持保存和取消操作

#### Agent Chat 改进
- ChatSidebar 精简（减少约 300 行冗余代码）
- CharacterMention 角色提及组件功能增强
- Inputbar 新增 SkillBadge 组件和相关 hooks
- 新增 Agent Chat 集成测试

#### 内容创作增强
- 新增 `content-creator/canvas/shared/` 共享组件目录
- Document、Music、Novel、Poster、Script、Video 画布均有功能增强
- 视频工作区 PromptInput、VideoCanvas、VideoWorkspace 组件优化

### 🔧 优化与重构

#### 设置页面迁移
- 删除旧版 `src/components/settings/` 下 13 个组件（AboutSection、ConnectionsSettings、DeveloperSettings、ExperimentalSettings、ExtensionsSettings、ExternalToolsSettings、GeneralSettings、LanguageSelector、ProxySettings、SettingsPage、UpdateNotification 等）
- settings-v2 布局和导航结构优化

#### 其他改进
- 通用聊天 ChatPanel 和 CompactModelSelector 组件优化
- 图像生成 ImageGenPage 功能增强
- input-kit ModelSelector 组件改进
- Smart Input ChatInput 和 SmartInputWindow 优化
- 终端 AI TerminalAIInput 和 TerminalAIPanel 改进
- 工具页面、工作台、记忆管理、插件系统、资源管理页面更新
- 外观设置页优化

### 📦 技术细节
- 62 个文件变更，+1551 行，-3217 行（净减少 1666 行代码）
- Rust 后端新增渠道配置类型，前端 TypeScript 类型同步更新
- 旧版设置页面完全迁移至 settings-v2 架构
