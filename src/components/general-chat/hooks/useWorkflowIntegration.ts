/**
 * 工作流集成 Hook
 *
 * 为通用对话功能集成三阶段工作流，实现自动化上下文管理
 */

import { useCallback, useEffect, useRef } from "react";
import { useThreeStageWorkflow } from "../../../hooks/useThreeStageWorkflow";
import { ToolHooksAPI } from "../../../lib/api/toolHooks";
import type { Message } from "../types";

export interface WorkflowIntegrationOptions {
  sessionId: string;
  enableAutoWorkflow?: boolean;
  workflowThreshold?: number; // 消息数量阈值，超过后自动启用工作流
}

export interface WorkflowIntegrationState {
  isWorkflowActive: boolean;
  isWorkflowInitialized: boolean;
  messageCount: number;
  lastActionType: string | null;
  visualOperationCount: number;
}

export interface WorkflowIntegrationActions {
  initializeWorkflow: (projectName: string, goal: string) => Promise<void>;
  handlePreMessage: (
    content: string,
    messageType: "user" | "assistant",
  ) => Promise<string | null>;
  handlePostMessage: (
    message: Message,
    isError?: boolean,
  ) => Promise<string | null>;
  handleToolUse: (
    toolName: string,
    toolParams: any,
    toolResult: string,
    error?: string,
  ) => Promise<void>;
  finalizeSession: () => Promise<string | null>;
  toggleWorkflow: () => void;
}

/**
 * 工作流集成 Hook
 */
export function useWorkflowIntegration(
  options: WorkflowIntegrationOptions,
): [WorkflowIntegrationState, WorkflowIntegrationActions] {
  const {
    sessionId,
    enableAutoWorkflow = true,
    workflowThreshold = 5,
  } = options;

  const [workflowState, workflowActions] = useThreeStageWorkflow({
    sessionId,
    autoInitialize: false,
  });

  const messageCountRef = useRef(0);
  const visualOperationCountRef = useRef(0);
  const isWorkflowActiveRef = useRef(false);

  const initializeWorkflow = useCallback(
    async (projectName: string, goal: string) => {
      try {
        await workflowActions.initializeWorkflow({
          sessionId,
          projectName,
          goal,
          phases: [
            {
              number: 1,
              name: "需求理解",
              status: "in_progress",
              tasks: [
                "理解用户需求和目标",
                "识别关键约束条件",
                "记录重要信息到 findings.md",
              ],
            },
            {
              number: 2,
              name: "方案制定",
              status: "pending",
              tasks: ["分析可行的解决方案", "制定执行计划", "记录关键决策"],
            },
            {
              number: 3,
              name: "执行实施",
              status: "pending",
              tasks: ["按计划执行任务", "监控执行进度", "记录执行结果"],
            },
            {
              number: 4,
              name: "验证完善",
              status: "pending",
              tasks: ["验证结果质量", "完善不足之处", "总结经验教训"],
            },
          ],
        });

        isWorkflowActiveRef.current = true;

        // 触发会话开始钩子
        await ToolHooksAPI.triggerSessionStart(sessionId, {
          project_name: projectName,
          goal,
          auto_initialized: enableAutoWorkflow.toString(),
        });
      } catch (error) {
        console.error("初始化工作流失败:", error);
        throw error;
      }
    },
    [sessionId, workflowActions, enableAutoWorkflow],
  );

  // 自动启用工作流检查
  useEffect(() => {
    if (
      enableAutoWorkflow &&
      messageCountRef.current >= workflowThreshold &&
      !workflowState.isInitialized &&
      !isWorkflowActiveRef.current
    ) {
      // 自动初始化工作流
      initializeWorkflow("通用对话任务", "协助用户完成复杂任务");
    }
  }, [
    enableAutoWorkflow,
    workflowThreshold,
    workflowState.isInitialized,
    initializeWorkflow,
  ]);

  const handlePreMessage = useCallback(
    async (
      content: string,
      messageType: "user" | "assistant",
    ): Promise<string | null> => {
      if (!workflowState.isInitialized || !isWorkflowActiveRef.current) {
        return null;
      }

      messageCountRef.current++;

      try {
        // 只对用户消息执行 Pre-Action
        if (messageType === "user") {
          const context = {
            sessionId,
            actionType: "user_message",
            actionDescription: `用户消息: ${content.substring(0, 100)}${content.length > 100 ? "..." : ""}`,
            messageCount: messageCountRef.current,
          };

          const preActionResult = await workflowActions.preAction(context);
          return preActionResult;
        }

        return null;
      } catch (error) {
        console.error("Pre-Message 处理失败:", error);
        return null;
      }
    },
    [workflowState.isInitialized, workflowActions, sessionId],
  );

  const handlePostMessage = useCallback(
    async (
      message: Message,
      isError: boolean = false,
    ): Promise<string | null> => {
      if (!workflowState.isInitialized || !isWorkflowActiveRef.current) {
        return null;
      }

      try {
        const context = {
          sessionId,
          actionType:
            message.role === "user" ? "user_message" : "assistant_response",
          actionDescription: `${message.role} 消息处理`,
          messageCount: messageCountRef.current,
        };

        const postActionResult = await workflowActions.postAction(
          context,
          message.content,
          isError ? "消息处理出现错误" : undefined,
        );

        // 检测视觉操作
        if (message.images && message.images.length > 0) {
          visualOperationCountRef.current++;

          // 应用 2-Action 规则
          if (visualOperationCountRef.current >= 2) {
            await workflowActions.recordFinding(
              "视觉内容分析",
              `处理了 ${message.images.length} 个图片，内容: ${message.content.substring(0, 200)}`,
              ["视觉操作", "2-Action规则"],
            );
            visualOperationCountRef.current = 0;
          }
        }

        return postActionResult;
      } catch (error) {
        console.error("Post-Message 处理失败:", error);
        return null;
      }
    },
    [workflowState.isInitialized, workflowActions, sessionId],
  );

  const handleToolUse = useCallback(
    async (
      toolName: string,
      toolParams: any,
      toolResult: string,
      error?: string,
    ) => {
      if (!workflowState.isInitialized || !isWorkflowActiveRef.current) {
        return;
      }

      try {
        // 触发工具使用钩子
        if (error) {
          await ToolHooksAPI.triggerPostToolUse(
            sessionId,
            toolName,
            toolResult,
            toolParams,
            `工具 ${toolName} 执行失败`,
            messageCountRef.current,
            error,
          );
        } else {
          await ToolHooksAPI.triggerPostToolUse(
            sessionId,
            toolName,
            toolResult,
            toolParams,
            `工具 ${toolName} 执行成功`,
            messageCountRef.current,
          );
        }

        // 记录工具使用进度
        await workflowActions.recordFinding(
          `工具使用: ${toolName}`,
          `工具: ${toolName}\n参数: ${JSON.stringify(toolParams, null, 2)}\n结果: ${toolResult.substring(0, 300)}${toolResult.length > 300 ? "..." : ""}${error ? `\n错误: ${error}` : ""}`,
          ["工具使用", toolName, error ? "错误" : "成功"],
        );
      } catch (err) {
        console.error("工具使用处理失败:", err);
      }
    },
    [workflowState.isInitialized, workflowActions, sessionId],
  );

  const finalizeSession = useCallback(async (): Promise<string | null> => {
    if (!workflowState.isInitialized || !isWorkflowActiveRef.current) {
      return null;
    }

    try {
      // 触发会话停止钩子
      await ToolHooksAPI.triggerStop(sessionId, messageCountRef.current, {
        total_messages: messageCountRef.current.toString(),
        visual_operations: visualOperationCountRef.current.toString(),
      });

      // 检查完成状态
      const { isComplete, summary } = await workflowActions.checkCompletion();

      // 结束工作流
      const finalMessage = await workflowActions.finalizeWorkflow();

      isWorkflowActiveRef.current = false;
      messageCountRef.current = 0;
      visualOperationCountRef.current = 0;

      return `${finalMessage}\n\n完成状态: ${isComplete ? "✅ 已完成" : "⏳ 未完成"}\n\n${summary}`;
    } catch (error) {
      console.error("结束会话处理失败:", error);
      return null;
    }
  }, [workflowState.isInitialized, workflowActions, sessionId]);

  const toggleWorkflow = useCallback(() => {
    isWorkflowActiveRef.current = !isWorkflowActiveRef.current;
  }, []);

  const state: WorkflowIntegrationState = {
    isWorkflowActive: isWorkflowActiveRef.current,
    isWorkflowInitialized: workflowState.isInitialized,
    messageCount: messageCountRef.current,
    lastActionType: null, // 可以根据需要实现
    visualOperationCount: visualOperationCountRef.current,
  };

  const actions: WorkflowIntegrationActions = {
    initializeWorkflow,
    handlePreMessage,
    handlePostMessage,
    handleToolUse,
    finalizeSession,
    toggleWorkflow,
  };

  return [state, actions];
}

export default useWorkflowIntegration;
