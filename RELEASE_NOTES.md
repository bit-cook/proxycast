# Release v0.74.0

## 🎉 重大功能

### Chrome Bridge - 浏览器自动化集成

实现了完整的 Chrome 浏览器自动化控制系统，AI 可以在对话中直接操作浏览器。

#### 核心特性

- **零配置自动连接**：打开 Chrome Profile 时自动加载扩展并配置连接
- **双通道架构**：Observer 通道（页面监控）+ Control 通道（命令控制）
- **AI 原生集成**：作为 MCP 工具（`mcp__proxycast-browser__`）集成到 Aster Agent，支持自然语言控制
- **多 Profile 支持**：可同时管理多个独立的 Chrome Profile

#### 支持的操作

- **导航**：打开 URL、刷新、前进、后退
- **页面读取**：获取页面内容（Markdown 格式）、标题、URL
- **元素交互**：点击、输入文本、滚动
- **表单操作**：批量填写表单字段
- **标签页管理**：获取标签页列表、切换标签页

#### 使用示例

用户：帮我在 Google 上搜索 "Rust"
AI 自动执行：打开 Google → 输入搜索词 → 点击搜索 → 读取结果 → 总结

## 🔄 重构

- 移除 `claude-in-chrome` 命名，统一使用 `mcp__proxycast-browser__` 前缀
- 模块重命名：`orchestrator.rs` → `model_orchestrator.rs`，`installer.rs` → `plugin_installer.rs`
- 使用 `#[derive(Default)]` 替代手动 impl
- 使用 `.is_multiple_of()` 替代 `% n == 0`
- 重命名 `from_str` 方法为 `parse_str` 避免与 std trait 冲突

## 🐛 Bug 修复

- WebSocket 路由修复：Axum 路径参数语法从 `/Proxycast_Key={key}` 改为 `/:key`
- Chrome 扩展源路径修复：开发模式下正确定位项目根目录
- Chrome 扩展存储清理：删除旧配置缓存确保自动配置生效
- 扩展重复注入防护：使用 IIFE 包装 content_script.js
- 剪贴板权限：在 manifest.json 中添加 `clipboardRead` 权限
- 前端 `cn` 函数缺失导入修复

## 🔧 代码质量改进

- 修复 33+ Clippy 警告
- 所有 259 个测试通过
- Cargo fmt 格式化通过
- ESLint 无警告

## 📝 文档

新增：
- `CHROME_BRIDGE_AI_USAGE.md` - AI 使用指南
- `CHROME_BRIDGE_QUICKSTART.md` - 快速参考
- `CHROME_BRIDGE_USAGE.md` - API 文档
