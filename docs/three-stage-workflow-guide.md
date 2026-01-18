# 三阶段工作流使用指南

基于 planning-with-files 的核心理念，ProxyCast 和 aster-rust 现已集成完整的三阶段工作流系统，解决 AI Agent 的上下文丢失、目标漂移、错误重复问题。

## 核心理念

```
Context Window = RAM (易失性，有限)
Filesystem = Disk (持久性，无限)

→ 重要信息都写入磁盘存储
```

## 三阶段工作流

### 1. Pre-Action 阶段
- **目的**: 执行前的上下文刷新和检查
- **功能**: 
  - 读取任务计划和历史记忆
  - 检查 3次错误协议
  - 刷新目标和上下文

### 2. Action 阶段  
- **目的**: 执行实际操作
- **功能**:
  - 记录操作过程
  - 跟踪视觉操作计数
  - 监控执行状态

### 3. Post-Action 阶段
- **目的**: 操作后的状态更新和学习
- **功能**:
  - 应用 2-Action 规则
  - 记录错误和解决方案
  - 更新进度和发现

## 核心文件系统

### 三文件模式

1. **task_plan.md** - 任务计划和阶段跟踪
2. **findings.md** - 研究发现和重要信息  
3. **progress.md** - 会话进度日志

### 自动化规则

- **2-Action 规则**: 每2次视觉操作后立即保存发现
- **3次错误协议**: 永不重复相同的失败操作
- **上下文刷新**: 重要决策前重新阅读计划文件

## ProxyCast 使用方法

### 1. 基础集成

```typescript
import { useWorkflowIntegration } from './hooks/useWorkflowIntegration';

function ChatComponent({ sessionId }: { sessionId: string }) {
  const [workflowState, workflowActions] = useWorkflowIntegration({
    sessionId,
    enableAutoWorkflow: true,
    workflowThreshold: 5, // 5条消息后自动启用
  });

  // 初始化工作流
  const handleInitWorkflow = async () => {
    await workflowActions.initializeWorkflow(
      '数据分析项目',
      '帮助用户分析销售数据并生成报告'
    );
  };

  // 处理消息发送
  const handleSendMessage = async (content: string) => {
    // Pre-Action: 获取上下文
    const preActionInfo = await workflowActions.handlePreMessage(content, 'user');
    
    // Action: 发送消息
    const response = await sendMessageToAPI(content);
    
    // Post-Action: 更新状态
    const postActionInfo = await workflowActions.handlePostMessage(response);
    
    return { response, preActionInfo, postActionInfo };
  };

  return (
    <div>
      <WorkflowStatusPanel
        sessionId={sessionId}
        isWorkflowActive={workflowState.isWorkflowActive}
        isWorkflowInitialized={workflowState.isWorkflowInitialized}
        messageCount={workflowState.messageCount}
        visualOperationCount={workflowState.visualOperationCount}
        onInitializeWorkflow={handleInitWorkflow}
        onFinalizeWorkflow={workflowActions.finalizeSession}
      />
      {/* 其他聊天组件 */}
    </div>
  );
}
```

### 2. 手动记录重要信息

```typescript
// 记录重要发现
await workflowActions.recordFinding(
  '关键数据洞察',
  '发现销售数据中存在明显的季节性趋势，Q4销量比Q1高出40%',
  ['数据分析', '季节性', '重要']
);

// 记录决策
await workflowActions.recordDecision(
  '使用时间序列分析',
  '考虑到数据的季节性特征，决定采用ARIMA模型进行预测分析'
);

// 更新阶段状态
await workflowActions.updatePhaseStatus(2, 'complete', '数据清洗和初步分析已完成');
```

### 3. 工具使用集成

```typescript
// 工具使用前后自动记录
const handleToolUse = async (toolName: string, params: any) => {
  try {
    const result = await executeTool(toolName, params);
    
    // 自动记录成功的工具使用
    await workflowActions.handleToolUse(toolName, params, result);
    
    return result;
  } catch (error) {
    // 自动记录失败的工具使用
    await workflowActions.handleToolUse(toolName, params, '', error.message);
    throw error;
  }
};
```

## aster-rust 使用方法

### 1. 基础工具集成

```rust
use aster::tools::{ThreeStageWorkflowTool, WorkflowIntegratedTool, ToolHookManager};

// 创建带钩子的工具
let hook_manager = Arc::new(ToolHookManager::new(true));
hook_manager.register_default_hooks().await;

let workflow_tool = WorkflowIntegratedTool::default()
    .with_hook_manager(hook_manager.clone());

// 注册到工具注册表
registry.register(Box::new(workflow_tool));
```

### 2. 三阶段工作流工具

```rust
// 初始化工作流
let init_params = serde_json::json!({
    "action": "init_workflow",
    "project_name": "Rust项目重构"
});

let result = three_stage_tool.execute(init_params, &context).await?;

// 记录发现
let finding_params = serde_json::json!({
    "action": "add_finding",
    "finding": "发现代码中存在大量重复逻辑，需要提取公共模块"
});

let result = three_stage_tool.execute(finding_params, &context).await?;

// 应用 2-Action 规则
let rule_params = serde_json::json!({
    "action": "apply_2action_rule",
    "finding": "通过代码审查发现了性能瓶颈"
});

let result = three_stage_tool.execute(rule_params, &context).await?;
```

### 3. 自定义钩子

```rust
use aster::tools::hooks::{ToolHook, HookContext, HookTrigger};

#[derive(Debug)]
struct CustomWorkflowHook {
    name: String,
}

#[async_trait]
impl ToolHook for CustomWorkflowHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "自定义工作流钩子"
    }

    async fn execute(&self, context: &HookContext) -> Result<()> {
        // 自定义钩子逻辑
        tracing::info!("执行自定义工作流钩子: {}", context.tool_name);
        Ok(())
    }
}

// 注册自定义钩子
hook_manager.register_hook(
    HookTrigger::PreExecution,
    Box::new(CustomWorkflowHook {
        name: "custom_workflow_hook".to_string(),
    })
).await;
```

## 最佳实践

### 1. 工作流初始化时机
- **自动模式**: 消息数量达到阈值时自动初始化
- **手动模式**: 用户明确表达复杂任务意图时初始化
- **智能模式**: 结合消息内容分析和用户行为模式

### 2. 记忆管理策略
- **及时记录**: 重要发现立即保存，不要依赖记忆
- **分类标记**: 使用标签系统便于后续检索
- **定期清理**: 自动归档过期记忆，保持系统性能

### 3. 错误处理原则
- **详细记录**: 记录错误的完整上下文和尝试的解决方案
- **避免重复**: 严格执行 3次错误协议
- **学习改进**: 从错误中提取经验，更新工作流程

### 4. 阶段管理技巧
- **明确划分**: 每个阶段有清晰的目标和完成标准
- **及时更新**: 阶段状态变化时立即更新
- **灵活调整**: 根据实际情况调整阶段计划

## 故障排除

### 常见问题

1. **工作流未自动初始化**
   - 检查 `enableAutoWorkflow` 设置
   - 确认消息数量是否达到阈值
   - 查看控制台错误信息

2. **钩子未触发**
   - 验证钩子管理器是否正确初始化
   - 检查钩子条件是否匹配
   - 确认钩子是否已启用

3. **记忆保存失败**
   - 检查会话ID是否有效
   - 验证存储权限
   - 查看后端服务状态

4. **性能问题**
   - 定期清理过期记忆
   - 限制单次记录的内容长度
   - 优化钩子执行频率

### 调试技巧

```typescript
// 启用详细日志
const [workflowState, workflowActions] = useThreeStageWorkflow({
  sessionId,
  debugMode: true, // 启用调试模式
});

// 获取详细统计信息
const stats = await workflowActions.getSessionStats();
console.log('工作流统计:', stats);

// 检查记忆状态
const memoryStats = await ContextMemoryAPI.getMemoryStats(sessionId);
console.log('记忆统计:', memoryStats);
```

## 扩展开发

### 自定义钩子规则

```typescript
// 创建自定义钩子规则
const customRule = ToolHooksAPI.createCustomRule(
  'code-review-reminder',
  '代码审查提醒',
  '检测到代码相关操作时提醒进行代码审查',
  'post_tool_use',
  [
    { message_contains: '代码' },
    { tool_name_contains: 'edit' },
  ],
  [
    {
      save_finding: {
        title: '代码审查提醒',
        content: '建议对修改的代码进行审查',
        tags: ['代码审查', '提醒'],
        priority: 3,
      },
    },
  ],
  50 // 优先级
);

await ToolHooksAPI.addHookRule(customRule);
```

### 自定义记忆类型

```typescript
// 扩展记忆文件类型
type ExtendedMemoryFileType = MemoryFileType | 'code_review' | 'test_results';

// 保存自定义类型记忆
await ContextMemoryAPI.saveMemoryEntry({
  session_id: sessionId,
  file_type: 'code_review' as any,
  title: '代码审查结果',
  content: '审查发现3个潜在问题...',
  tags: ['代码审查', '质量'],
  priority: 4,
});
```

## 总结

三阶段工作流系统为 ProxyCast 和 aster-rust 提供了强大的上下文管理和自动化能力：

- **解决核心问题**: 上下文丢失、目标漂移、错误重复
- **自动化工程**: Pre-Action → Action → Post-Action 流程
- **持久化记忆**: 基于文件系统的可靠存储
- **智能学习**: 错误跟踪和经验积累
- **灵活扩展**: 支持自定义钩子和记忆类型

通过正确使用这个系统，可以显著提升 AI Agent 的工作效率和可靠性，让复杂任务的执行更加有序和可控。