# Skills 集成 E2E 测试指南

本文档指导如何手动进行 Skills 集成功能的端到端测试。

## 架构说明

ProxyCast 的 Skills 集成基于 aster-rust 框架的 `SkillTool`：

```
用户消息 → AI Agent → SkillTool → global_registry → 执行 Skill
                                        ↑
                                        |
                    load_proxycast_skills() 加载 ~/.proxycast/skills/
```

关键组件：
- `AsterAgentState::load_proxycast_skills()` - 启动时加载 Skills
- `AsterAgentState::reload_proxycast_skills()` - 安装/卸载后刷新
- `aster::skills::global_registry()` - 全局 Skill 注册表
- `aster::skills::SkillTool` - AI 调用 Skills 的工具

## 前置条件

1. ProxyCast 应用已构建并可运行
2. 至少配置了一个可用的 AI Provider（如 OpenAI API Key）
3. 终端可以访问 `~/.proxycast/skills/` 目录

## 测试场景

### 场景 1：Skills 自动加载

**目的**：验证 Agent 初始化时能正确加载 Skills

**步骤**：

1. 创建测试 Skill：
```bash
mkdir -p ~/.proxycast/skills/test-greeting
cat > ~/.proxycast/skills/test-greeting/SKILL.md << 'EOF'
---
name: test-greeting
description: 一个简单的问候技能，用于测试 Skills 集成
---

# 问候技能

当用户请求问候时，使用以下格式回复：

"你好！我是 ProxyCast 助手，很高兴为你服务！"

请始终使用中文回复。
EOF
```

2. 启动 ProxyCast 应用：
```bash
cd proxycast && npm run tauri dev
```

3. 打开开发者工具（Cmd+Option+I），查看控制台日志

4. **预期结果**：
   - 日志中应显示 `[AsterAgent] 成功加载 1 个 ProxyCast Skills 到 global_registry`
   - 日志中应显示 `[AsterAgent] 已注册 Skill: user:test-greeting`

### 场景 2：AI 自动调用 Skill

**目的**：验证 AI 能根据用户意图自动调用 Skill

**步骤**：

1. 确保测试 Skill 已创建（见场景 1）

2. 在 ProxyCast 聊天界面发送消息：
   ```
   请用问候技能跟我打个招呼
   ```

3. **预期结果**：
   - AI 应该识别到 `test-greeting` Skill
   - AI 应该调用 Skill 并返回问候语
   - 响应中应包含 "你好！我是 ProxyCast 助手"

### 场景 3：通过斜杠命令调用 Skill

**目的**：验证用户可以通过 `/skill-name` 显式调用 Skill

**步骤**：

1. 在聊天界面发送：
   ```
   /test-greeting
   ```

2. **预期结果**：
   - AI 应该直接执行 `test-greeting` Skill
   - 返回问候语

### 场景 4：安装新 Skill 后动态刷新

**目的**：验证安装新 Skill 后 AI 能立即发现

**步骤**：

1. 在 ProxyCast 运行时，创建新 Skill：
```bash
mkdir -p ~/.proxycast/skills/test-calculator
cat > ~/.proxycast/skills/test-calculator/SKILL.md << 'EOF'
---
name: test-calculator
description: 一个简单的计算器技能
---

# 计算器技能

当用户请求计算时，执行数学运算并返回结果。

支持：加法、减法、乘法、除法
EOF
```

2. 在 ProxyCast Skills 页面点击刷新（或重新进入页面）

3. 发送消息：
   ```
   请用计算器技能帮我算 123 + 456
   ```

4. **预期结果**：
   - AI 应该能发现新安装的 `test-calculator` Skill
   - AI 应该调用该 Skill 并返回计算结果

### 场景 5：卸载 Skill 后不再可用

**目的**：验证卸载 Skill 后 AI 不再能调用

**步骤**：

1. 删除测试 Skill：
```bash
rm -rf ~/.proxycast/skills/test-greeting
```

2. 在 ProxyCast Skills 页面点击刷新

3. 发送消息：
   ```
   /test-greeting
   ```

4. **预期结果**：
   - AI 应该提示找不到该 Skill
   - 或者 AI 应该说明该 Skill 不可用

## 清理测试数据

测试完成后，清理测试 Skills：

```bash
rm -rf ~/.proxycast/skills/test-greeting
rm -rf ~/.proxycast/skills/test-calculator
```

## 常见问题排查

### Skills 没有被加载

1. 检查目录是否存在：`ls -la ~/.proxycast/skills/`
2. 检查 SKILL.md 文件格式是否正确
3. 查看应用日志中是否有错误信息

### AI 没有调用 Skill

1. 确认 Skill 已被加载（查看启动日志）
2. 尝试使用更明确的指令，如 "使用 xxx 技能"
3. 检查 Skill 的 `description` 是否清晰描述了用途

### 动态刷新不生效

1. 确认调用了 `reload_proxycast_skills()`
2. 检查日志中是否有刷新相关的输出
3. 尝试重启应用

## 自动化测试（未来计划）

后续可以使用 Playwright 或 Tauri 的测试框架实现自动化 E2E 测试：

```typescript
// 示例：Playwright E2E 测试
test('AI should auto-invoke skill based on intent', async ({ page }) => {
  // 1. 创建测试 Skill
  await createTestSkill('test-greeting');
  
  // 2. 启动应用
  await launchProxyCast();
  
  // 3. 发送消息
  await page.fill('[data-testid="chat-input"]', '请用问候技能跟我打招呼');
  await page.click('[data-testid="send-button"]');
  
  // 4. 验证响应
  await expect(page.locator('[data-testid="chat-message"]'))
    .toContainText('你好！我是 ProxyCast 助手');
});
```
