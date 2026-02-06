/**
 * @file Skill 执行 Hook
 * @description 提供 Skill 执行功能，监听 Tauri 事件并管理执行状态
 *
 * 功能：
 * - 执行 Skill 并返回结果
 * - 监听执行进度事件（step_start, step_complete, step_error, complete）
 * - 管理执行状态（isExecuting, currentStep, progress, error）
 * - 提供事件回调
 *
 * @module hooks/useSkillExecution
 * @requirements 6.4, 7.1
 */

import { useState, useEffect, useCallback, useRef } from "react";
import { safeListen } from "@/lib/dev-bridge";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  skillExecutionApi,
  SKILL_EVENTS,
  type SkillExecutionResult,
  type StepStartPayload,
  type StepCompletePayload,
  type StepErrorPayload,
  type ExecutionCompletePayload,
} from "@/lib/api/skill-execution";

// ============================================================================
// 类型定义
// ============================================================================

/**
 * useSkillExecution Hook 选项
 */
export interface UseSkillExecutionOptions {
  /** 步骤开始回调 */
  onStepStart?: (stepId: string, stepName: string, total: number) => void;
  /** 步骤完成回调 */
  onStepComplete?: (stepId: string, output: string) => void;
  /** 步骤错误回调 */
  onStepError?: (stepId: string, error: string, willRetry: boolean) => void;
  /** 执行完成回调 */
  onComplete?: (success: boolean, output?: string) => void;
}

/**
 * useSkillExecution Hook 返回值
 */
export interface UseSkillExecutionReturn {
  /** 执行 Skill */
  execute: (
    skillName: string,
    input: string,
    provider?: string,
  ) => Promise<SkillExecutionResult>;
  /** 是否正在执行 */
  isExecuting: boolean;
  /** 当前步骤名称 */
  currentStep: string | null;
  /** 执行进度（0-100） */
  progress: number;
  /** 错误信息 */
  error: string | null;
  /** 当前执行 ID */
  executionId: string | null;
  /** 总步骤数 */
  totalSteps: number;
  /** 当前步骤序号 */
  currentStepIndex: number;
}

// ============================================================================
// Hook 实现
// ============================================================================

/**
 * Skill 执行 Hook
 *
 * 提供 Skill 执行功能，监听 Tauri 事件并管理执行状态。
 *
 * @param options - Hook 选项，包含事件回调
 * @returns Hook 返回值，包含执行函数和状态
 *
 * @example
 * ```tsx
 * function SkillRunner() {
 *   const {
 *     execute,
 *     isExecuting,
 *     currentStep,
 *     progress,
 *     error,
 *   } = useSkillExecution({
 *     onStepStart: (stepId, stepName, total) => {
 *       console.log(`开始步骤 ${stepName} (${stepId}/${total})`);
 *     },
 *     onComplete: (success, output) => {
 *       if (success) {
 *         console.log('执行成功:', output);
 *       }
 *     },
 *   });
 *
 *   const handleExecute = async () => {
 *     const result = await execute('my-skill', 'user input');
 *     console.log('结果:', result);
 *   };
 *
 *   return (
 *     <div>
 *       <button onClick={handleExecute} disabled={isExecuting}>
 *         执行
 *       </button>
 *       {isExecuting && (
 *         <div>
 *           <p>当前步骤: {currentStep}</p>
 *           <progress value={progress} max={100} />
 *         </div>
 *       )}
 *       {error && <p className="error">{error}</p>}
 *     </div>
 *   );
 * }
 * ```
 */
export function useSkillExecution(
  options: UseSkillExecutionOptions = {},
): UseSkillExecutionReturn {
  const { onStepStart, onStepComplete, onStepError, onComplete } = options;

  // 状态
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentStep, setCurrentStep] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [executionId, setExecutionId] = useState<string | null>(null);
  const [totalSteps, setTotalSteps] = useState(0);
  const [currentStepIndex, setCurrentStepIndex] = useState(0);

  // 使用 ref 存储回调，避免重新订阅事件
  const callbacksRef = useRef({
    onStepStart,
    onStepComplete,
    onStepError,
    onComplete,
  });

  // 更新回调引用
  useEffect(() => {
    callbacksRef.current = {
      onStepStart,
      onStepComplete,
      onStepError,
      onComplete,
    };
  }, [onStepStart, onStepComplete, onStepError, onComplete]);

  // 当前执行 ID 的 ref（用于事件过滤）
  const currentExecutionIdRef = useRef<string | null>(null);

  // 监听 Tauri 事件
  useEffect(() => {
    const unlistenFns: UnlistenFn[] = [];

    const setupListeners = async () => {
      // 步骤开始事件
      const unlistenStepStart = await safeListen<StepStartPayload>(
        SKILL_EVENTS.STEP_START,
        (event) => {
          const payload = event.payload;
          // 只处理当前执行的事件
          if (
            currentExecutionIdRef.current &&
            payload.execution_id !== currentExecutionIdRef.current
          ) {
            return;
          }

          setCurrentStep(payload.step_name);
          setCurrentStepIndex(payload.current_step);
          setTotalSteps(payload.total_steps);

          // 计算进度（基于步骤）
          const stepProgress =
            payload.total_steps > 0
              ? ((payload.current_step - 1) / payload.total_steps) * 100
              : 0;
          setProgress(stepProgress);

          callbacksRef.current.onStepStart?.(
            payload.step_id,
            payload.step_name,
            payload.total_steps,
          );
        },
      );
      unlistenFns.push(unlistenStepStart);

      // 步骤完成事件
      const unlistenStepComplete = await safeListen<StepCompletePayload>(
        SKILL_EVENTS.STEP_COMPLETE,
        (event) => {
          const payload = event.payload;
          if (
            currentExecutionIdRef.current &&
            payload.execution_id !== currentExecutionIdRef.current
          ) {
            return;
          }

          callbacksRef.current.onStepComplete?.(
            payload.step_id,
            payload.output,
          );
        },
      );
      unlistenFns.push(unlistenStepComplete);

      // 步骤错误事件
      const unlistenStepError = await safeListen<StepErrorPayload>(
        SKILL_EVENTS.STEP_ERROR,
        (event) => {
          const payload = event.payload;
          if (
            currentExecutionIdRef.current &&
            payload.execution_id !== currentExecutionIdRef.current
          ) {
            return;
          }

          // 如果不会重试，设置错误状态
          if (!payload.will_retry) {
            setError(payload.error);
          }

          callbacksRef.current.onStepError?.(
            payload.step_id,
            payload.error,
            payload.will_retry,
          );
        },
      );
      unlistenFns.push(unlistenStepError);

      // 执行完成事件
      const unlistenComplete = await safeListen<ExecutionCompletePayload>(
        SKILL_EVENTS.COMPLETE,
        (event) => {
          const payload = event.payload;
          if (
            currentExecutionIdRef.current &&
            payload.execution_id !== currentExecutionIdRef.current
          ) {
            return;
          }

          // 更新状态
          setIsExecuting(false);
          setProgress((prev) => (payload.success ? 100 : prev));
          currentExecutionIdRef.current = null;

          if (!payload.success && payload.error) {
            setError(payload.error);
          }

          callbacksRef.current.onComplete?.(payload.success, payload.output);
        },
      );
      unlistenFns.push(unlistenComplete);
    };

    setupListeners();

    // 清理函数
    return () => {
      unlistenFns.forEach((unlisten) => unlisten());
    };
  }, []);

  // 执行 Skill
  const execute = useCallback(
    async (
      skillName: string,
      input: string,
      provider?: string,
    ): Promise<SkillExecutionResult> => {
      // 重置状态
      setIsExecuting(true);
      setCurrentStep(null);
      setProgress(0);
      setError(null);
      setTotalSteps(0);
      setCurrentStepIndex(0);

      // 生成执行 ID（用于事件过滤）
      const execId = crypto.randomUUID();
      setExecutionId(execId);
      currentExecutionIdRef.current = execId;

      try {
        const result = await skillExecutionApi.executeSkill({
          skillName,
          userInput: input,
          providerOverride: provider,
          executionId: execId,
        });

        // 如果执行失败，设置错误
        if (!result.success && result.error) {
          setError(result.error);
        }

        return result;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(errorMessage);
        setIsExecuting(false);
        currentExecutionIdRef.current = null;

        return {
          success: false,
          error: errorMessage,
          steps_completed: [],
        };
      } finally {
        // 注意：不在这里设置 isExecuting = false
        // 因为 complete 事件会处理这个
        // 但如果发生异常，需要在 catch 中处理
      }
    },
    [],
  );

  return {
    execute,
    isExecuting,
    currentStep,
    progress,
    error,
    executionId,
    totalSteps,
    currentStepIndex,
  };
}

export default useSkillExecution;
