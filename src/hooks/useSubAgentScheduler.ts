/**
 * SubAgent 调度器 Hook
 *
 * 提供 SubAgent 调度功能的 React 集成
 */

import { useState, useEffect, useCallback } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

/**
 * SubAgent 任务定义
 */
export interface SubAgentTask {
  id: string;
  taskType: string;
  prompt: string;
  description?: string;
  priority?: number;
  dependencies?: string[];
  timeout?: number;
  model?: string;
  returnSummary?: boolean;
  allowedTools?: string[];
  deniedTools?: string[];
  maxTokens?: number;
}

/**
 * SubAgent 执行结果
 */
export interface SubAgentResult {
  taskId: string;
  success: boolean;
  output?: string;
  summary?: string;
  error?: string;
  durationMs: number;
  retries: number;
}

/**
 * 调度进度
 */
export interface SchedulerProgress {
  total: number;
  completed: number;
  failed: number;
  running: number;
  pending: number;
  skipped: number;
  cancelled: boolean;
  currentTasks: string[];
  percentage: number;
}

/**
 * 调度事件
 */
export type SchedulerEvent =
  | { type: "started"; totalTasks: number }
  | { type: "taskStarted"; taskId: string; taskType: string }
  | { type: "taskCompleted"; taskId: string; durationMs: number }
  | { type: "taskFailed"; taskId: string; error: string }
  | { type: "taskRetry"; taskId: string; retryCount: number }
  | { type: "taskSkipped"; taskId: string; reason: string }
  | { type: "progress"; progress: SchedulerProgress }
  | { type: "completed"; success: boolean; durationMs: number }
  | { type: "cancelled" };

/**
 * 调度执行结果
 */
export interface SchedulerExecutionResult {
  success: boolean;
  results: SubAgentResult[];
  totalDurationMs: number;
  successfulCount: number;
  failedCount: number;
  skippedCount: number;
  mergedSummary?: string;
  totalTokenUsage: {
    inputTokens: number;
    outputTokens: number;
    totalTokens: number;
  };
}

/**
 * 调度器配置
 */
export interface SchedulerConfig {
  maxConcurrency?: number;
  defaultTimeoutMs?: number;
  retryOnFailure?: boolean;
  stopOnFirstError?: boolean;
  maxRetries?: number;
  retryDelayMs?: number;
  autoSummarize?: boolean;
  summaryMaxTokens?: number;
  defaultModel?: string;
}

/**
 * Hook 状态
 */
interface UseSubAgentSchedulerState {
  isRunning: boolean;
  progress: SchedulerProgress | null;
  events: SchedulerEvent[];
  result: SchedulerExecutionResult | null;
  error: string | null;
}

/**
 * Hook 返回值
 */
interface UseSubAgentSchedulerReturn extends UseSubAgentSchedulerState {
  execute: (
    tasks: SubAgentTask[],
    config?: SchedulerConfig,
  ) => Promise<SchedulerExecutionResult>;
  cancel: () => Promise<void>;
  clearEvents: () => void;
}

/**
 * SubAgent 调度器 Hook
 */
export function useSubAgentScheduler(): UseSubAgentSchedulerReturn {
  const [state, setState] = useState<UseSubAgentSchedulerState>({
    isRunning: false,
    progress: null,
    events: [],
    result: null,
    error: null,
  });

  // 监听调度事件
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setupListener = async () => {
      unlisten = await listen<SchedulerEvent>(
        "subagent-scheduler-event",
        (event) => {
          const schedulerEvent = event.payload;

          setState((prev) => ({
            ...prev,
            events: [...prev.events, schedulerEvent],
          }));

          // 更新进度
          if (schedulerEvent.type === "progress") {
            setState((prev) => ({
              ...prev,
              progress: schedulerEvent.progress,
            }));
          }

          // 更新运行状态
          if (schedulerEvent.type === "started") {
            setState((prev) => ({
              ...prev,
              isRunning: true,
              error: null,
            }));
          }

          if (
            schedulerEvent.type === "completed" ||
            schedulerEvent.type === "cancelled"
          ) {
            setState((prev) => ({
              ...prev,
              isRunning: false,
            }));
          }
        },
      );
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // 执行任务
  const execute = useCallback(
    async (
      tasks: SubAgentTask[],
      config?: SchedulerConfig,
    ): Promise<SchedulerExecutionResult> => {
      setState((prev) => ({
        ...prev,
        isRunning: true,
        error: null,
        events: [],
        progress: null,
        result: null,
      }));

      try {
        const result = await invoke<SchedulerExecutionResult>(
          "execute_subagent_tasks",
          {
            tasks,
            config,
          },
        );

        setState((prev) => ({
          ...prev,
          isRunning: false,
          result,
        }));

        return result;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setState((prev) => ({
          ...prev,
          isRunning: false,
          error: errorMessage,
        }));
        throw err;
      }
    },
    [],
  );

  // 取消执行
  const cancel = useCallback(async () => {
    try {
      await invoke("cancel_subagent_tasks");
    } catch (err) {
      console.error("取消 SubAgent 任务失败:", err);
    }
  }, []);

  // 清除事件
  const clearEvents = useCallback(() => {
    setState((prev) => ({
      ...prev,
      events: [],
    }));
  }, []);

  return {
    ...state,
    execute,
    cancel,
    clearEvents,
  };
}

export default useSubAgentScheduler;
