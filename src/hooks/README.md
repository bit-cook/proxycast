# Hooks 目录

全局共享的 React Hooks。

## 文件索引

| 文件 | 说明 |
|------|------|
| `useUnifiedChat.ts` | 统一对话 Hook，支持 Agent/General/Creator 三种模式 |
| `useSkillExecution.ts` | Skill 执行 Hook，监听 Tauri 事件并管理执行状态 |

## useUnifiedChat

统一的对话逻辑 Hook，替代原有分散的 `useAgentChat` 和 `useChat`。

### 使用示例

```typescript
import { useUnifiedChat } from "@/hooks/useUnifiedChat";

// Agent 模式
const agentChat = useUnifiedChat({
  mode: "agent",
  providerType: "claude",
  model: "claude-sonnet-4-20250514",
});

// 内容创作模式
const creatorChat = useUnifiedChat({
  mode: "creator",
  systemPrompt: "你是一位专业的内容创作助手...",
  onCanvasUpdate: (path, content) => {
    // 更新画布内容
  },
});

// 通用对话模式
const generalChat = useUnifiedChat({
  mode: "general",
});
```

### 返回值

- `session` - 当前会话
- `messages` - 消息列表
- `isLoading` - 加载状态
- `isSending` - 发送状态
- `error` - 错误信息
- `createSession()` - 创建会话
- `loadSession()` - 加载会话
- `sendMessage()` - 发送消息
- `stopGeneration()` - 停止生成
- `configureProvider()` - 配置 Provider

## 相关文档

- 架构设计：`docs/prd/chat-architecture-redesign.md`
- 类型定义：`src/types/chat.ts`
- API 封装：`src/lib/api/unified-chat.ts`

## useSkillExecution

Skill 执行 Hook，提供 Skill 执行功能，监听 Tauri 事件并管理执行状态。

### 使用示例

```typescript
import { useSkillExecution } from "@/hooks/useSkillExecution";

function SkillRunner() {
  const {
    execute,
    isExecuting,
    currentStep,
    progress,
    error,
  } = useSkillExecution({
    onStepStart: (stepId, stepName, total) => {
      console.log(`开始步骤 ${stepName} (${stepId}/${total})`);
    },
    onComplete: (success, output) => {
      if (success) {
        console.log('执行成功:', output);
      }
    },
  });

  const handleExecute = async () => {
    const result = await execute('my-skill', 'user input');
    console.log('结果:', result);
  };

  return (
    <div>
      <button onClick={handleExecute} disabled={isExecuting}>
        执行
      </button>
      {isExecuting && (
        <div>
          <p>当前步骤: {currentStep}</p>
          <progress value={progress} max={100} />
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </div>
  );
}
```

### 返回值

- `execute(skillName, input, provider?)` - 执行 Skill
- `isExecuting` - 是否正在执行
- `currentStep` - 当前步骤名称
- `progress` - 执行进度（0-100）
- `error` - 错误信息
- `executionId` - 当前执行 ID
- `totalSteps` - 总步骤数
- `currentStepIndex` - 当前步骤序号

### 事件回调

- `onStepStart(stepId, stepName, total)` - 步骤开始
- `onStepComplete(stepId, output)` - 步骤完成
- `onStepError(stepId, error, willRetry)` - 步骤错误
- `onComplete(success, output?)` - 执行完成

### 相关文档

- API 封装：`src/lib/api/skill-execution.ts`
- 设计文档：`.kiro/specs/skills-integration/design.md`
