# 插件管理组件

插件系统的前端 UI 组件，提供插件安装、卸载和管理功能。

## 文件索引

| 文件 | 说明 |
|------|------|
| `PluginsPage.tsx` | 插件中心页面，独立的导航栏入口 |
| `PluginManager.tsx` | 插件管理主组件，显示插件列表和状态 |
| `PluginInstallDialog.tsx` | 插件安装对话框，支持本地文件和 URL 安装 |
| `PluginUninstallDialog.tsx` | 插件卸载确认对话框 |
| `PluginUIRenderer.tsx` | 插件 UI 渲染器，根据 pluginId 渲染对应的插件 UI |
| `PluginItemContextMenu.tsx` | 插件项右键菜单，支持启用/禁用、打开目录、卸载等操作 |
| `index.ts` | 模块导出 |

## 功能说明

### PluginManager
- 显示插件系统状态概览
- 列出已加载的插件和已安装的插件包
- 提供安装/卸载入口
- 支持启用/禁用插件

### PluginInstallDialog
- 支持从本地文件安装（.zip, .tar.gz）
- 支持从 URL 下载安装（GitHub Releases 等）
- 显示安装进度（下载、验证、解压、安装、注册）
- 显示安装结果

### PluginUninstallDialog
- 显示插件信息确认
- 调用后端卸载命令
- 刷新插件列表

### PluginUIRenderer
- 根据 pluginId 渲染对应的插件 UI 组件
- 支持内置插件组件映射 (machine-id-tool -> MachineIdTool)
- 显示友好的错误提示（插件未找到、加载失败）
- 导出 Page 类型定义，支持动态插件路由

### PluginItemContextMenu
- 为已安装插件列表提供右键菜单
- 支持启用/禁用插件
- 支持打开插件目录
- 支持卸载插件（带确认对话框）

## 相关需求

- 需求 1.1: 本地文件安装
- 需求 2.1: URL 安装
- 需求 3.1-3.4: 安装进度显示
- 需求 3.2: 插件 UI 渲染
- 需求 4.1-4.3: 卸载功能
- 需求 6.1: 插件列表显示
