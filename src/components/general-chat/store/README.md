# store

<!-- 一旦我所属的文件夹有所变化，请更新我 -->

## 架构说明

通用对话功能的 Zustand 状态管理模块。
使用 persist 中间件持久化 UI 布局状态和当前会话 ID。

### 状态切片

- **会话状态** - sessions、currentSessionId
- **消息状态** - messages (sessionId -> Message[])
- **流式状态** - isStreaming、currentMessageId、partialContent
- **UI 状态** - sidebarCollapsed、sidebarWidth、canvasCollapsed、canvasWidth
- **画布状态** - isOpen、contentType、content、language、filename、isEditing

## 文件索引

- `index.ts` - 模块导出入口
- `useGeneralChatStore.ts` - 主 Store 实现
  - 会话操作：createSession、selectSession、deleteSession、renameSession
  - 消息操作：sendMessage、stopGeneration、appendStreamingContent、finalizeMessage
  - UI 操作：toggleSidebar、setSidebarWidth、toggleCanvas、setCanvasWidth
  - 画布操作：openCanvas、closeCanvas、updateCanvasContent
  - 选择器 Hooks：useCurrentSession、useCurrentMessages、useStreamingState 等

## 使用示例

```typescript
import { useGeneralChatStore, useCurrentMessages, useIsStreaming } from './store';

// 使用完整 Store
const { createSession, sendMessage } = useGeneralChatStore();

// 使用选择器 Hooks（性能优化）
const messages = useCurrentMessages();
const isStreaming = useIsStreaming();
```

## 更新提醒

任何文件变更后，请更新此文档和相关的上级文档。
