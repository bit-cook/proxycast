# Release v0.74.0

## 📊 变更统计

- **110 个文件变更**
- **新增 18,997 行代码**
- **删除 1,918 行代码**
- **净增 17,079 行代码**

## 🎉 重大功能

### 1. Chrome Bridge - 浏览器自动化集成

实现了完整的 Chrome 浏览器自动化控制系统，AI 可以在对话中直接操作浏览器。

#### 核心特性

- **零配置自动连接**：打开 Chrome Profile 时自动加载扩展并配置连接
- **双通道架构**：Observer 通道（页面监控）+ Control 通道（命令控制）
- **AI 原生集成**：作为 MCP 工具（`mcp__proxycast-browser__`）集成到 Aster Agent
- **多 Profile 支持**：可同时管理多个独立的 Chrome Profile

#### 新增文件

- `extensions/proxycast-chrome/` - Chrome 扩展完整实现（1,555 行）
- `src-tauri/crates/server/src/chrome_bridge.rs` - 服务器端桥接（1,139 行）
- `src-tauri/crates/server/src/handlers/chrome_bridge_ws.rs` - WebSocket 处理（260 行）
- `src/components/settings-v2/system/chrome-relay/` - Chrome 中继设置页面（745 行）
- `scripts/test-chrome-bridge.mjs` - 测试脚本

#### 支持的操作

- 导航：打开 URL、刷新、前进、后退
- 页面读取：获取内容（Markdown）、标题、URL
- 元素交互：点击、输入文本、滚动
- 表单操作：批量填写表单字段
- 标签页管理：列表、切换

### 2. 小说创作工作流（Novel Theme）

全新的小说创作主题，提供完整的创作、发布、管理流程。

#### 核心功能

- **小说流程工作台**：可视化创作流程管理（1,319 行）
- **小说设置向导**：完整的作品配置系统（1,223 行）
- **发布管理**：多平台发布支持（506 行）
- **AI 辅助创作**：集成 AI 生成和优化

#### 新增文件

- `src-tauri/src/services/novel_service.rs` - 小说服务核心（2,407 行）
- `src-tauri/src/commands/novel_cmd.rs` - Tauri 命令（111 行）
- `src/components/projects/tabs/novel-flow/` - 流程工作台
- `src/components/projects/tabs/novel-settings/` - 设置向导
- `src/lib/novel-flow/` - 流程引擎
- `src/lib/novel-settings/` - 设置类型定义（418 行）
- `src/lib/api/novel.ts` - API 接口（238 行）

### 3. 主题系统（Theme System）

可扩展的主题系统，支持不同类型项目的专属工作流。

#### 核心架构

- **主题注册表**：动态加载和管理主题
- **面板渲染器**：自定义 UI 组件
- **API 集成**：主题专属 API 接口
- **工作台重构**：模块化设计，支持主题扩展

#### 新增文件

- `src/features/themes/` - 主题系统核心
  - `registry.ts` - 主题注册表
  - `types.ts` - 类型定义（59 行）
  - `novel/` - 小说主题
  - `video/` - 视频主题
  - `shared/` - 共享组件
- `src-tauri/src/theme/` - 服务器端主题支持
  - `novel/command.rs` - 小说主题命令（93 行）
  - `video/command.rs` - 视频主题命令（84 行）

### 4. 工作台重构（Workbench Refactor）

将 1,767 行的单体组件重构为模块化架构。

#### 改进

- **Hooks 拆分**：6 个专用 hooks
  - `useWorkbenchController` - 控制器（426 行）
  - `useCreationDialogs` - 创建对话框（426 行）
  - `useWorkbenchNavigation` - 导航（198 行）
  - `useWorkbenchProjectData` - 项目数据（170 行）
  - `useWorkbenchQuickActions` - 快捷操作（97 行）
  - `useWorkbenchPanelRenderer` - 面板渲染（41 行）

- **面板组件化**：
  - `WorkbenchLeftSidebar` - 左侧边栏（331 行）
  - `WorkbenchMainContent` - 主内容区（231 行）
  - `WorkbenchRightRail` - 右侧栏（301 行）

- **Shell 组件**：
  - `WorkspaceShell` - 工作区外壳
  - `WorkspaceTopbar` - 顶部栏

### 5. Web 搜索增强

- **搜索提示服务**：`web_search_prompt_service.rs`（86 行）
- **搜索设置页面**：`settings-v2/system/web-search/`（156 行）
- 支持自定义搜索引擎和提示词

## 🔄 重构

### 代码质量

- 移除 `claude-in-chrome` 命名，统一使用 `mcp__proxycast-browser__` 前缀
- 模块重命名：`orchestrator.rs` → `model_orchestrator.rs`
- 模块重命名：`installer.rs` → `plugin_installer.rs`
- 使用 `#[derive(Default)]` 替代手动 impl（6 处）
- 使用 `.is_multiple_of()` 替代 `% n == 0`（2 处）
- 重命名 `from_str` 方法为 `parse_str` 避免与 std trait 冲突

### 架构改进

- 工作台组件从 1,767 行拆分为多个模块
- 主题系统支持动态扩展
- 配置系统增强：支持更多自定义选项

## 🐛 Bug 修复

### Chrome Bridge

- WebSocket 路由修复：Axum 路径参数语法从 `/Proxycast_Key={key}` 改为 `/:key`
- Chrome 扩展源路径修复：开发模式下正确定位项目根目录
- Chrome 扩展存储清理：删除旧配置缓存确保自动配置生效
- 扩展重复注入防护：使用 IIFE 包装 content_script.js
- 剪贴板权限：在 manifest.json 中添加 `clipboardRead` 权限

### 前端

- 修复 `cn` 函数缺失导入（WorkbenchPage.tsx）
- 修复 React Hooks 依赖警告
- 修复设置页面布局问题

### 后端

- 修复配置类型定义
- 修复数据库 schema
- 修复 Cargo 依赖

## 🔧 代码质量改进

- 修复 33+ Clippy 警告
- 所有 259 个测试通过
- Cargo fmt 格式化通过
- ESLint 无警告
- AI 代码验证平均分 98/100

## 📝 文档

### Chrome Bridge

- `CHROME_BRIDGE_AI_USAGE.md` - AI 使用指南
- `CHROME_BRIDGE_QUICKSTART.md` - 快速参考
- `CHROME_BRIDGE_USAGE.md` - API 文档
- `CHROME_BRIDGE_FIX.md` - 问题修复记录
- `IMPLEMENTATION_PLAN.md` - 实现计划

### 其他

- `extensions/proxycast-chrome/README.md` - 扩展说明
- 更新服务文档：`src-tauri/src/services/README.md`

## 🚀 升级说明

### 版本号

已自动同步到：
- `package.json`: 0.74.0
- `src-tauri/Cargo.toml`: 0.74.0
- `src-tauri/tauri.conf.json`: 0.74.0

### 新功能使用

#### Chrome Bridge

1. 在 ProxyCast 设置页面打开 Chrome Profile
2. 扩展会自动加载并连接
3. 在 AI 对话中自然描述需求即可

示例：
```
用户：帮我查一下今天的天气
AI：[自动打开天气网站并读取内容] 今天晴天，20-25°C...
```

#### 小说创作

1. 创建新项目，选择"小说"类型
2. 使用小说流程工作台管理创作流程
3. 配置作品设置并发布

#### 主题系统

开发者可以创建自定义主题：
1. 在 `src/features/themes/` 创建主题目录
2. 实现主题接口
3. 在 `registry.ts` 注册主题

## 🙏 致谢

感谢所有贡献者和用户的支持！

---

**完整变更日志**：https://github.com/aiclientproxy/proxycast/compare/v0.73.0...v0.74.0
