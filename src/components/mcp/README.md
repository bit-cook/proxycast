# mcp

<!-- 一旦我所属的文件夹有所变化，请更新我 -->

## 架构说明

MCP (Model Context Protocol) 服务器管理组件集合。
提供完整的 MCP 前端管理界面，包括配置管理、运行时控制、工具/提示词/资源浏览与调用。

## 文件索引

- `McpPanel.tsx` - MCP 管理面板（主入口，整合所有子组件，Tab 切换布局）
- `McpPage.tsx` - MCP 配置管理页面（服务器增删改查、导入/同步）
- `McpServerList.tsx` - 服务器运行状态列表（启动/停止控制、状态指示）
- `McpToolsBrowser.tsx` - 工具浏览器（按服务器分组、搜索、Schema 展示）
- `McpToolCaller.tsx` - 工具调用组件（参数表单/JSON 模式、结果展示）
- `McpPromptsBrowser.tsx` - 提示词浏览器（参数输入、内容获取）
- `McpResourcesBrowser.tsx` - 资源浏览器（资源列表、内容预览）
- `index.ts` - 模块导出

## 依赖关系

- Hooks: `useMcp`（运行时状态）、`useMcpServers`（配置管理）
- API: `src/lib/api/mcp.ts`（Tauri 命令封装）
- 后端: `src-tauri/src/commands/mcp_cmd.rs`、`src-tauri/src/mcp/`

## 更新提醒

任何文件变更后，请更新此文档和相关的上级文档。
