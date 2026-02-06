# MCP 模块

MCP（Model Context Protocol）集成模块，提供 MCP 协议的客户端实现。

## 模块结构

| 文件 | 说明 |
|------|------|
| `mod.rs` | 模块导出和文档 |
| `types.rs` | MCP 数据类型定义（配置、工具、提示词、资源、错误） |
| `client.rs` | MCP 客户端实现（rmcp ClientHandler） |
| `manager.rs` | MCP 客户端管理器（连接池、缓存、生命周期） |
| `tool_converter.rs` | 工具格式转换器（OpenAI/Anthropic/Gemini） |

## 功能概览

### 服务器生命周期管理
- 启动/停止 MCP 服务器进程
- stdio 传输连接
- 状态监控和事件通知

### 工具管理
- 工具发现和缓存
- 工具调用路由
- 名称冲突解决（服务器前缀）

### 格式转换
- MCP → OpenAI function calling
- MCP → Anthropic tool use
- MCP → Gemini function declaration

## 依赖

- `rmcp`: Rust MCP SDK
- `tokio`: 异步运行时
- `serde`: 序列化/反序列化
- `thiserror`: 错误类型定义

## 相关文档

- 设计文档: `.kiro/specs/mcp-integration/design.md`
- 需求文档: `.kiro/specs/mcp-integration/requirements.md`
