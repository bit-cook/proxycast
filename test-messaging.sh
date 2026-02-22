#!/bin/bash

# ProxyCast 闪退修复验证脚本

echo "🧪 ProxyCast 闪退修复验证测试"
echo "================================"
echo ""

# 检查当前版本
CURRENT_VERSION=$(grep '"version"' package.json | head -1 | cut -d '"' -f 4)
echo "📦 当前版本: $CURRENT_VERSION"
echo ""

# 检查最近的修复提交
echo "📝 最近的修复提交:"
git log --oneline -1
echo ""

# 编译检查
echo "🔧 编译检查..."
echo "正在编译 Rust 代码..."
cd src-tauri
cargo build 2>&1 | tail -5
cd ..
echo ""

# 运行测试
echo "🧪 运行测试..."
npm run test 2>&1 | tail -10
echo ""

# Lint 检查
echo "🔍 Lint 检查..."
npm run lint 2>&1 | tail -5
echo ""

echo "✅ 修复验证完成！"
echo ""
echo "📋 测试清单："
echo "  1. 启动应用（应该能看到详细错误信息而非直接崩溃）"
echo "  2. 创建新对话"
echo "  3. 发送第一条消息（例如：'你好'）"
echo "  4. 检查是否仍然崩溃"
echo ""
echo "如果问题仍然存在，请查看："
echo "  - 浏览器开发者工具 Console 标签页"
echo "  - 应用日志文件"
echo "  - test-crash-fix.md 中的详细说明"
