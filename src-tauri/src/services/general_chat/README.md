# general_chat

<!-- 一旦我所属的文件夹有所变化，请更新我 -->

## 架构说明

通用对话服务模块，提供 AI 对话功能的核心后端服务。

主要功能：
- 会话管理（创建、删除、重命名、切换）
- 消息存储和检索
- 会话标题自动生成
- 内容块解析（代码块、文件等）

## 文件索引

- `mod.rs` - 模块入口，导出公共类型和服务
- `types.rs` - 核心数据类型定义（ChatSession、ChatMessage、ContentBlock 等）
- `session_service.rs` - 会话管理服务（创建会话、消息、验证、标题生成）

## 数据结构

### ChatSession
对话会话，包含 id、name、created_at、updated_at、metadata

### ChatMessage
对话消息，包含 id、session_id、role、content、blocks、status、created_at、metadata

### MessageRole
消息角色枚举：User、Assistant、System

### ContentBlock
内容块，支持 text、code、image、file 类型

## 更新提醒

任何文件变更后，请更新此文档和相关的上级文档。
