/**
 * 三阶段工作流管理器
 *
 * 基于 planning-with-files 的核心机制，实现：
 * - Pre-Action → Action → Post-Action 三阶段工作流
 * - 自动化上下文工程和错误学习
 * - 2-Action 规则和 3次错误协议
 */

import { ContextMemoryAPI } from "../api/contextMemory";
import { ToolHooksAPI } from "../api/toolHooks";

export interface WorkflowPhase {
  number: number;
  name: string;
  status: "pending" | "in_progress" | "complete";
  tasks: string[];
  notes?: string;
}

export interface WorkflowConfig {
  sessionId: string;
  projectName: string;
  goal: string;
  phases: WorkflowPhase[];
}

export interface ActionContext {
  sessionId: string;
  actionType: string;
  actionDescription: string;
  toolName?: string;
  toolParameters?: Record<string, string>;
  messageCount: number;
}

/**
 * 三阶段工作流管理器
 */
export class ThreeStageWorkflowManager {
  private sessionId: string;
  private visualOperationCount: number = 0;
  private errorAttempts: Map<string, number> = new Map();

  constructor(sessionId: string) {
    this.sessionId = sessionId;
  }

  /**
   * 初始化工作流
   */
  async initializeWorkflow(config: WorkflowConfig): Promise<void> {
    // 触发会话开始钩子
    await ToolHooksAPI.triggerSessionStart(this.sessionId, {
      project_name: config.projectName,
      goal: config.goal,
    });

    // 创建任务计划
    const taskPlanContent = this.generateTaskPlanContent(config);
    await ContextMemoryAPI.saveTaskPlan(
      this.sessionId,
      `任务计划: ${config.projectName}`,
      taskPlanContent,
      5,
    );

    // 创建初始发现记录
    await ContextMemoryAPI.saveFinding(
      this.sessionId,
      "工作流初始化",
      `三阶段工作流已初始化\n项目: ${config.projectName}\n目标: ${config.goal}`,
      ["初始化", "工作流"],
      3,
    );

    // 记录初始进度
    await ContextMemoryAPI.logProgress(
      this.sessionId,
      "工作流启动",
      `三阶段工作流已启动，共 ${config.phases.length} 个阶段`,
    );
  }

  /**
   * Pre-Action 阶段：执行操作前的上下文刷新
   */
  async preAction(context: ActionContext): Promise<string> {
    // 触发 Pre-Tool-Use 钩子
    await ToolHooksAPI.triggerPreToolUse(
      context.sessionId,
      context.toolName || context.actionType,
      context.toolParameters || {},
      context.actionDescription,
      context.messageCount,
    );

    // 获取当前记忆上下文
    const memoryContext = await ContextMemoryAPI.getMemoryContext(
      context.sessionId,
    );

    // 检查是否应该避免该操作（3次错误协议）
    const shouldAvoid = await ContextMemoryAPI.shouldAvoidOperation(
      context.sessionId,
      context.actionDescription,
    );

    if (shouldAvoid) {
      const warning = `⚠️ 3次错误协议警告: 该操作已失败3次，建议更换方法\n操作: ${context.actionDescription}`;

      await ContextMemoryAPI.recordError({
        session_id: context.sessionId,
        error_description: `重复失败操作: ${context.actionDescription}`,
        attempted_solution: "触发3次错误协议，建议更换方法",
      });

      return `${warning}\n\n当前上下文:\n${memoryContext}`;
    }

    // 记录上下文刷新
    await ContextMemoryAPI.logProgress(
      context.sessionId,
      "Pre-Action 上下文刷新",
      `准备执行: ${context.actionDescription}`,
    );

    return `🔄 Pre-Action 上下文刷新完成\n\n准备执行: ${context.actionDescription}\n\n当前记忆上下文:\n${memoryContext}`;
  }

  /**
   * Action 阶段：执行实际操作
   */
  async executeAction(
    context: ActionContext,
    actionResult: string,
  ): Promise<void> {
    // 记录操作执行
    await ContextMemoryAPI.logProgress(
      context.sessionId,
      `执行操作: ${context.actionType}`,
      `操作描述: ${context.actionDescription}\n结果: ${actionResult.substring(0, 200)}${actionResult.length > 200 ? "..." : ""}`,
    );

    // 如果是视觉操作，增加计数
    if (this.isVisualOperation(context.actionType)) {
      this.visualOperationCount++;
    }
  }

  /**
   * Post-Action 阶段：操作后的状态更新
   */
  async postAction(
    context: ActionContext,
    actionResult: string,
    error?: string,
  ): Promise<string> {
    let message = "📝 Post-Action 状态更新:\n\n";

    // 处理错误情况
    if (error) {
      const errorKey = context.actionDescription;
      const attemptCount = (this.errorAttempts.get(errorKey) || 0) + 1;
      this.errorAttempts.set(errorKey, attemptCount);

      const { shouldAvoid } = await ContextMemoryAPI.recordErrorWithCheck(
        context.sessionId,
        error,
        `尝试次数: ${attemptCount}`,
        context.actionDescription,
      );

      message += `🚨 错误记录 (第${attemptCount}次尝试): ${error}\n`;

      if (shouldAvoid) {
        message += `⚠️ 已达到3次错误限制，建议更换方法\n`;
      }
    }

    // 触发 Post-Tool-Use 钩子
    await ToolHooksAPI.triggerPostToolUse(
      context.sessionId,
      context.toolName || context.actionType,
      actionResult,
      context.toolParameters || {},
      context.actionDescription,
      context.messageCount,
      error,
    );

    // 应用 2-Action 规则
    if (this.visualOperationCount >= 2) {
      await this.apply2ActionRule(actionResult);
      message += `🎯 2-Action 规则已应用 (视觉操作计数: ${this.visualOperationCount})\n`;
      this.visualOperationCount = 0; // 重置计数
    }

    // 提醒更新状态
    message += `\n💡 提醒:\n`;
    message += `- 如果完成了某个阶段，请更新任务计划状态\n`;
    message += `- 有新发现请记录到 findings.md\n`;
    message += `- 重要进展请更新 progress.md\n`;

    return message;
  }

  /**
   * 应用 2-Action 规则
   */
  private async apply2ActionRule(actionResult: string): Promise<void> {
    const timestamp = new Date().toLocaleTimeString();
    const finding = `2-Action 规则触发 (${timestamp})\n\n最近操作结果:\n${actionResult.substring(0, 500)}${actionResult.length > 500 ? "..." : ""}`;

    await ContextMemoryAPI.apply2ActionRule(this.sessionId, finding);
  }

  /**
   * 更新阶段状态
   */
  async updatePhaseStatus(
    phaseNumber: number,
    status: "pending" | "in_progress" | "complete",
    notes?: string,
  ): Promise<void> {
    const statusText = {
      pending: "待开始",
      in_progress: "进行中",
      complete: "已完成",
    }[status];

    await ContextMemoryAPI.saveTaskPlan(
      this.sessionId,
      `阶段 ${phaseNumber} 状态更新`,
      `阶段 ${phaseNumber} 状态已更新为: ${statusText}${notes ? `\n备注: ${notes}` : ""}`,
      4,
    );

    await ContextMemoryAPI.logProgress(
      this.sessionId,
      `阶段 ${phaseNumber} 状态更新`,
      `状态: ${statusText}${notes ? `\n备注: ${notes}` : ""}`,
    );
  }

  /**
   * 记录重要发现
   */
  async recordFinding(
    title: string,
    content: string,
    tags: string[] = [],
  ): Promise<void> {
    await ContextMemoryAPI.saveFinding(
      this.sessionId,
      title,
      content,
      ["发现", ...tags],
      4,
    );
  }

  /**
   * 记录决策
   */
  async recordDecision(decision: string, rationale: string): Promise<void> {
    await ContextMemoryAPI.saveFinding(
      this.sessionId,
      `决策: ${decision}`,
      `决策内容: ${decision}\n\n决策理由:\n${rationale}`,
      ["决策", "重要"],
      5,
    );
  }

  /**
   * 检查任务完成状态
   */
  async checkCompletion(): Promise<{ isComplete: boolean; summary: string }> {
    const stats = await ContextMemoryAPI.getMemoryStats(this.sessionId);
    const memories = await ContextMemoryAPI.getSessionMemories(this.sessionId);

    // 简单的完成度检查逻辑
    const taskPlanMemories = memories.filter(
      (m) => m.file_type === "task_plan",
    );
    const hasCompletedPhases = taskPlanMemories.some(
      (m) => m.content.includes("已完成") || m.content.includes("complete"),
    );

    const summary =
      `📊 任务完成状态检查:\n\n` +
      `- 活跃记忆: ${stats.active_memories} 个\n` +
      `- 未解决错误: ${stats.unresolved_errors} 个\n` +
      `- 已解决错误: ${stats.resolved_errors} 个\n` +
      `- 是否有已完成阶段: ${hasCompletedPhases ? "是" : "否"}\n\n` +
      `${stats.unresolved_errors > 0 ? "⚠️ 仍有未解决的错误需要处理" : "✅ 无未解决错误"}`;

    return {
      isComplete: hasCompletedPhases && stats.unresolved_errors === 0,
      summary,
    };
  }

  /**
   * 结束工作流
   */
  async finalizeWorkflow(): Promise<string> {
    const { isComplete, summary } = await this.checkCompletion();

    // 触发停止钩子
    await ToolHooksAPI.triggerStop(this.sessionId, 0, {
      workflow_complete: isComplete.toString(),
    });

    // 保存会话摘要
    await ContextMemoryAPI.saveFinding(
      this.sessionId,
      "工作流会话摘要",
      `三阶段工作流已结束\n\n${summary}`,
      ["摘要", "会话结束"],
      5,
    );

    return `🎉 三阶段工作流已结束\n\n${summary}`;
  }

  /**
   * 生成任务计划内容
   */
  private generateTaskPlanContent(config: WorkflowConfig): string {
    let content = `# 任务计划: ${config.projectName}\n\n`;
    content += `## 目标\n${config.goal}\n\n`;
    content += `## 当前阶段\n阶段 1\n\n`;
    content += `## 阶段列表\n\n`;

    config.phases.forEach((phase) => {
      content += `### 阶段 ${phase.number}: ${phase.name}\n`;
      phase.tasks.forEach((task) => {
        content += `- [ ] ${task}\n`;
      });
      content += `- **状态**: ${phase.status}\n\n`;
    });

    content += `## 关键问题\n`;
    content += `1. [需要回答的重要问题]\n`;
    content += `2. [另一个关键问题]\n\n`;

    content += `## 已做决策\n`;
    content += `| 决策 | 理由 |\n`;
    content += `|------|------|\n`;
    content += `|      |      |\n\n`;

    content += `## 遇到的错误\n`;
    content += `| 错误 | 尝试次数 | 解决方案 |\n`;
    content += `|------|----------|----------|\n`;
    content += `|      | 1        |          |\n\n`;

    content += `## 注意事项\n`;
    content += `- **2-Action 规则**: 每2次视觉操作后立即保存发现\n`;
    content += `- **3次错误协议**: 永不重复相同的失败操作\n`;
    content += `- **上下文刷新**: 重要决策前重新阅读计划文件\n`;

    return content;
  }

  /**
   * 判断是否为视觉操作
   */
  private isVisualOperation(actionType: string): boolean {
    const visualActions = [
      "view",
      "read",
      "browse",
      "search",
      "screenshot",
      "image",
    ];
    return visualActions.some((action) =>
      actionType.toLowerCase().includes(action),
    );
  }

  /**
   * 获取会话统计
   */
  async getSessionStats(): Promise<{
    memoryStats: any;
    visualOperationCount: number;
    errorAttempts: Record<string, number>;
  }> {
    const memoryStats = await ContextMemoryAPI.getMemoryStats(this.sessionId);

    return {
      memoryStats,
      visualOperationCount: this.visualOperationCount,
      errorAttempts: Object.fromEntries(this.errorAttempts),
    };
  }
}

export default ThreeStageWorkflowManager;
