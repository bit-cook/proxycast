# 小部件系统 (Widgets)

右侧小部件栏组件，移植自 Waveterm 的 widgets 系统。

## 功能概述

- 右侧垂直工具栏，固定宽度 48px
- 响应式显示模式（normal/compact/supercompact）
- 设置浮动菜单
- 小部件配置持久化
- 分块布局支持（水平排列多个面板）
- 文件浏览器右键菜单（借鉴 Waveterm）

## 文件索引

| 文件 | 说明 |
|------|------|
| `types.ts` | 类型定义 |
| `constants.ts` | 常量配置 |
| `context.ts` | React Context 定义 |
| `WidgetContext.tsx` | Provider 组件和状态持久化 |
| `useWidgets.ts` | 小部件相关 Hooks |
| `Widget.tsx` | 单个小部件按钮组件 |
| `WidgetsSidebar.tsx` | 主容器组件 |
| `SettingsFloatingMenu.tsx` | 设置浮动菜单 |
| `SysinfoView.tsx` | 系统信息监控视图（CPU/内存图表） |
| `FileBrowserView.tsx` | 文件浏览器视图 |
| `FileContextMenu.tsx` | 文件浏览器右键菜单组件 |
| `EntryManagerOverlay.tsx` | 文件/文件夹名称编辑对话框 |
| `WebView.tsx` | 内嵌浏览器视图（使用 Tauri 原生 webview） |
| `index.ts` | 模块导出 |

## 右键菜单功能

借鉴 Waveterm 的文件浏览器右键菜单，支持以下功能：

| 功能 | 说明 |
|------|------|
| 新建文件 | 在当前目录创建新文件 |
| 新建文件夹 | 在当前目录创建新文件夹 |
| 重命名 | 重命名选中的文件或文件夹 |
| 复制文件名 | 复制文件名到剪贴板 |
| 复制完整路径 | 复制完整路径到剪贴板 |
| 复制文件名 (Shell 引用) | 复制 Shell 转义后的文件名 |
| 复制完整路径 (Shell 引用) | 复制 Shell 转义后的完整路径 |
| 在 Finder 中显示 | 在系统文件管理器中显示 |
| 使用默认应用打开 | 使用系统默认应用打开文件 |
| 在新终端中打开 | 在新终端中打开目录 |
| 删除 | 删除文件或文件夹 |

## 使用方式

```tsx
import { WidgetProvider, WidgetsSidebar, WidgetType } from "@/components/terminal/widgets";

function App() {
  const handleWidgetClick = (type: WidgetType) => {
    // 处理小部件点击
  };

  return (
    <WidgetProvider>
      <div className="flex">
        <main>{/* 主内容 */}</main>
        <WidgetsSidebar onWidgetClick={handleWidgetClick} />
      </div>
    </WidgetProvider>
  );
}
```

## 显示模式

| 模式 | 触发条件 | 显示内容 |
|------|----------|----------|
| normal | 高度充足 | 图标 + 标签 |
| compact | 高度有限 | 仅图标 |
| supercompact | 高度非常有限 | 2列网格，仅图标 |

## 分块布局

在终端页面，点击右侧小部件图标可以添加附加面板：
- 点击 Terminal 图标：添加新的终端面板（支持多个）
- 点击 Files 图标：添加文件浏览器面板
- 点击 Web 图标：添加内嵌浏览器面板
- 点击 Sysinfo 图标：添加系统监控面板

所有面板水平排列，对齐 Waveterm 的 TileLayout 风格。

## 依赖

- `@floating-ui/react` - 浮动菜单定位
- `lucide-react` - 图标
- `styled-components` - 样式
- `@tauri-apps/plugin-shell` - 外部浏览器打开
- `@/lib/webview-api` - Tauri webview 管理 API
