# connect

<!-- 一旦我所属的文件夹有所变化，请更新我 -->

## 架构说明

ProxyCast Connect 前端组件，实现中转商 API Key 添加确认流程。
使用 shadcn/ui 组件库和 TailwindCSS 进行样式管理。

## 功能概述

1. **确认弹窗** - 显示中转商信息和脱敏 API Key
2. **品牌展示** - 展示中转商 Logo、名称、描述
3. **警告提示** - 未验证中转商警告
4. **错误提示** - Deep Link 解析、Registry 加载、API Key 存储错误提示

## 文件索引

- `index.ts` - 组件导出入口
- `ConnectConfirmDialog.tsx` - 确认添加 API Key 弹窗
- `ConnectConfirmDialog.test.tsx` - Provider Display 属性测试 (Property 7)
- `ConnectErrorToast.tsx` - 错误提示 Toast 组件和工具函数

## 相关需求

- Requirements 3.x - 连接确认弹窗
- Requirements 6.x - 中转商品牌展示
- Requirements 7.x - 错误处理

## 更新提醒

任何文件变更后，请更新此文档和相关的上级文档。
