# Workspace 组件

Workspace 相关的 React 组件。

## 文件索引

| 文件 | 说明 |
|------|------|
| `index.ts` | 组件导出 |
| `WorkbenchPage.tsx` | 主题工作台页面（项目管理 / 项目详情 / 作业） |
| `WorkbenchPage.test.tsx` | Workbench 左侧栏模式行为测试 |
| `WorkspaceSelector.tsx` | Workspace 选择器下拉组件 |
| `utils/creationIntentPrompt.ts` | 新建文稿创作意图构建与校验 |
| `utils/creationIntentPrompt.test.ts` | 创作意图构建与校验测试 |

## 组件

### WorkbenchPage

主题工作台主页面，按 `workspaceMode` 分三种模式：

- `project-management`：项目管理态
- `project-detail`：项目详情态
- `workspace`：三栏作业态（对话/画布）

左侧栏默认规则（按模式切换时生效）：

- `project-management` / `project-detail`：默认展开
- `workspace`：默认折叠

用户仍可通过按钮或 `Cmd/Ctrl + B` 手动切换左侧栏展开状态。

### WorkspaceSelector

Workspace 选择器组件，用于切换和管理工作目录。

```tsx
import { WorkspaceSelector } from '@/components/workspace';

<WorkspaceSelector
  onSelect={(workspace) => console.log('选中:', workspace)}
  onAddClick={() => openAddDialog()}
/>
```

## 相关 Hook

- `useWorkspace` - Workspace 管理 Hook

## 测试

```bash
# 验证 Workbench 左侧栏模式行为
npx vitest --run "src/components/workspace/WorkbenchPage.test.tsx"

# 验证 workspace 组件相关测试
npx vitest --run "src/components/workspace/WorkbenchPage.test.tsx" "src/components/workspace/utils/creationIntentPrompt.test.ts"
```
