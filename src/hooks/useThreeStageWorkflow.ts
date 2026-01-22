/**
 * 三阶段工作流 React Hook
 *
 * 提供在 React 组件中使用三阶段工作流的便捷接口
 */

import { useState, useCallback, useRef, useEffect } from "react";
import ThreeStageWorkflowManager, {
  type WorkflowConfig,
  type ActionContext,
} from "../lib/workflow/threeStageWorkflow";

export interface UseThreeStageWorkflowOptions {
  sessionId: string;
  autoInitialize?: boolean;
  defaultConfig?: Partial<WorkflowConfig>;
}

export interface WorkflowState {
  isInitialized: boolean;
  isLoading: boolean;
  error: string | null;
  currentPhase: number;
  visualOperationCount: number;
  errorAttempts: Record<string, number>;
}

export interface WorkflowActions {
  initializeWorkflow: (config: WorkflowConfig) => Promise<void>;
  preAction: (context: ActionContext) => Promise<string>;
  executeAction: (context: ActionContext, result: string) => Promise<void>;
  postAction: (
    context: ActionContext,
    result: string,
    error?: string,
  ) => Promise<string>;
  updatePhaseStatus: (
    phaseNumber: number,
    status: "pending" | "in_progress" | "complete",
    notes?: string,
  ) => Promise<void>;
  recordFinding: (
    title: string,
    content: string,
    tags?: string[],
  ) => Promise<void>;
  recordDecision: (decision: string, rationale: string) => Promise<void>;
  checkCompletion: () => Promise<{ isComplete: boolean; summary: string }>;
  finalizeWorkflow: () => Promise<string>;
  getSessionStats: () => Promise<any>;
  reset: () => void;
}

/**
 * 三阶段工作流 Hook
 */
export function useThreeStageWorkflow(
  options: UseThreeStageWorkflowOptions,
): [WorkflowState, WorkflowActions] {
  const { sessionId, autoInitialize = false, defaultConfig } = options;

  const [state, setState] = useState<WorkflowState>({
    isInitialized: false,
    isLoading: false,
    error: null,
    currentPhase: 1,
    visualOperationCount: 0,
    errorAttempts: {},
  });

  const workflowManagerRef = useRef<ThreeStageWorkflowManager | null>(null);

  // 初始化工作流管理器
  useEffect(() => {
    if (sessionId && !workflowManagerRef.current) {
      workflowManagerRef.current = new ThreeStageWorkflowManager(sessionId);
    }
  }, [sessionId]);

  const updateStats = useCallback(async () => {
    if (!workflowManagerRef.current) return;

    try {
      const stats = await workflowManagerRef.current.getSessionStats();
      setState((prev) => ({
        ...prev,
        visualOperationCount: stats.visualOperationCount,
        errorAttempts: stats.errorAttempts,
      }));
    } catch (error) {
      console.warn("更新统计信息失败:", error);
    }
  }, []);

  const setLoading = useCallback((loading: boolean) => {
    setState((prev) => ({ ...prev, isLoading: loading }));
  }, []);

  const setError = useCallback((error: string | null) => {
    setState((prev) => ({ ...prev, error }));
  }, []);

  const initializeWorkflow = useCallback(
    async (config: WorkflowConfig) => {
      if (!workflowManagerRef.current) return;

      setLoading(true);
      setError(null);

      try {
        await workflowManagerRef.current.initializeWorkflow(config);
        setState((prev) => ({
          ...prev,
          isInitialized: true,
          currentPhase: 1,
        }));
        await updateStats();
      } catch (error) {
        setError(error instanceof Error ? error.message : "初始化工作流失败");
      } finally {
        setLoading(false);
      }
    },
    [setLoading, setError, updateStats],
  );

  // 自动初始化
  useEffect(() => {
    if (
      autoInitialize &&
      defaultConfig &&
      !state.isInitialized &&
      workflowManagerRef.current
    ) {
      const config: WorkflowConfig = {
        sessionId,
        projectName: defaultConfig.projectName || "新项目",
        goal: defaultConfig.goal || "待定义目标",
        phases: defaultConfig.phases || [
          {
            number: 1,
            name: "需求分析",
            status: "in_progress",
            tasks: ["理解用户需求", "识别约束条件", "记录发现"],
          },
          {
            number: 2,
            name: "方案设计",
            status: "pending",
            tasks: ["制定技术方案", "创建项目结构", "记录关键决策"],
          },
          {
            number: 3,
            name: "实施执行",
            status: "pending",
            tasks: ["按计划执行", "增量测试", "记录进展"],
          },
        ],
      };

      initializeWorkflow(config);
    }
  }, [
    autoInitialize,
    defaultConfig,
    state.isInitialized,
    sessionId,
    initializeWorkflow,
  ]);

  const preAction = useCallback(
    async (context: ActionContext): Promise<string> => {
      if (!workflowManagerRef.current) throw new Error("工作流未初始化");

      setLoading(true);
      setError(null);

      try {
        const result = await workflowManagerRef.current.preAction(context);
        await updateStats();
        return result;
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : "Pre-Action 执行失败";
        setError(errorMessage);
        throw error;
      } finally {
        setLoading(false);
      }
    },
    [setLoading, setError, updateStats],
  );

  const executeAction = useCallback(
    async (context: ActionContext, result: string) => {
      if (!workflowManagerRef.current) throw new Error("工作流未初始化");

      try {
        await workflowManagerRef.current.executeAction(context, result);
        await updateStats();
      } catch (error) {
        setError(error instanceof Error ? error.message : "Action 执行失败");
        throw error;
      }
    },
    [setError, updateStats],
  );

  const postAction = useCallback(
    async (
      context: ActionContext,
      result: string,
      error?: string,
    ): Promise<string> => {
      if (!workflowManagerRef.current) throw new Error("工作流未初始化");

      setLoading(true);

      try {
        const message = await workflowManagerRef.current.postAction(
          context,
          result,
          error,
        );
        await updateStats();
        return message;
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : "Post-Action 执行失败";
        setError(errorMessage);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [setLoading, setError, updateStats],
  );

  const updatePhaseStatus = useCallback(
    async (
      phaseNumber: number,
      status: "pending" | "in_progress" | "complete",
      notes?: string,
    ) => {
      if (!workflowManagerRef.current) throw new Error("工作流未初始化");

      try {
        await workflowManagerRef.current.updatePhaseStatus(
          phaseNumber,
          status,
          notes,
        );
        setState((prev) => ({ ...prev, currentPhase: phaseNumber }));
        await updateStats();
      } catch (error) {
        setError(error instanceof Error ? error.message : "更新阶段状态失败");
        throw error;
      }
    },
    [setError, updateStats],
  );

  const recordFinding = useCallback(
    async (title: string, content: string, tags: string[] = []) => {
      if (!workflowManagerRef.current) throw new Error("工作流未初始化");

      try {
        await workflowManagerRef.current.recordFinding(title, content, tags);
        await updateStats();
      } catch (error) {
        setError(error instanceof Error ? error.message : "记录发现失败");
        throw error;
      }
    },
    [setError, updateStats],
  );

  const recordDecision = useCallback(
    async (decision: string, rationale: string) => {
      if (!workflowManagerRef.current) throw new Error("工作流未初始化");

      try {
        await workflowManagerRef.current.recordDecision(decision, rationale);
        await updateStats();
      } catch (error) {
        setError(error instanceof Error ? error.message : "记录决策失败");
        throw error;
      }
    },
    [setError, updateStats],
  );

  const checkCompletion = useCallback(async () => {
    if (!workflowManagerRef.current) throw new Error("工作流未初始化");

    try {
      const result = await workflowManagerRef.current.checkCompletion();
      await updateStats();
      return result;
    } catch (error) {
      setError(error instanceof Error ? error.message : "检查完成状态失败");
      throw error;
    }
  }, [setError, updateStats]);

  const finalizeWorkflow = useCallback(async (): Promise<string> => {
    if (!workflowManagerRef.current) throw new Error("工作流未初始化");

    setLoading(true);

    try {
      const result = await workflowManagerRef.current.finalizeWorkflow();
      setState((prev) => ({ ...prev, isInitialized: false }));
      return result;
    } catch (error) {
      setError(error instanceof Error ? error.message : "结束工作流失败");
      throw error;
    } finally {
      setLoading(false);
    }
  }, [setLoading, setError]);

  const getSessionStats = useCallback(async () => {
    if (!workflowManagerRef.current) throw new Error("工作流未初始化");

    try {
      return await workflowManagerRef.current.getSessionStats();
    } catch (error) {
      setError(error instanceof Error ? error.message : "获取会话统计失败");
      throw error;
    }
  }, [setError]);

  const reset = useCallback(() => {
    setState({
      isInitialized: false,
      isLoading: false,
      error: null,
      currentPhase: 1,
      visualOperationCount: 0,
      errorAttempts: {},
    });
    workflowManagerRef.current = sessionId
      ? new ThreeStageWorkflowManager(sessionId)
      : null;
  }, [sessionId]);

  const actions: WorkflowActions = {
    initializeWorkflow,
    preAction,
    executeAction,
    postAction,
    updatePhaseStatus,
    recordFinding,
    recordDecision,
    checkCompletion,
    finalizeWorkflow,
    getSessionStats,
    reset,
  };

  return [state, actions];
}

export default useThreeStageWorkflow;
