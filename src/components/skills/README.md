# Skills 组件

<!-- 一旦我所属的文件夹有所变化，请更新我 -->

## 架构说明

Skills 组件模块提供 Skill 管理和执行的 UI 界面，包括：
- Skill 列表展示和管理
- Skill 仓库管理
- Skill 执行对话框
- Workflow 执行进度展示

## 文件索引

| 文件 | 说明 |
|------|------|
| `index.ts` | 模块导出入口 |
| `SkillsPage.tsx` | Skills 主页面，展示 Skill 列表 |
| `SkillCard.tsx` | Skill 卡片组件，展示单个 Skill 信息和操作 |
| `SkillCard.test.ts` | SkillCard 组件测试 |
| `RepoManagerPanel.tsx` | Skill 仓库管理面板 |
| `SkillExecutionDialog.tsx` | Skill 执行对话框，显示详情、输入表单和进度 |
| `WorkflowProgress.tsx` | Workflow 进度展示组件 |

## 组件依赖关系

```
SkillsPage
├── SkillCard (Skill 列表项)
├── RepoManagerPanel (仓库管理)
└── SkillExecutionDialog (执行对话框)
    └── WorkflowProgress (执行进度)
```

## 更新提醒

任何文件变更后，请更新此文档和相关的上级文档。
