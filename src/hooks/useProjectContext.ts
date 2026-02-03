/**
 * @file useProjectContext.ts
 * @description 项目上下文 Hook，提供项目上下文加载和 system_prompt 构建功能
 * @module hooks/useProjectContext
 * @requirements 10.1, 10.2, 10.3
 */

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectContext } from "@/types/context";

/** Hook 返回类型 */
export interface UseProjectContextReturn {
  /** 项目上下文 */
  context: ProjectContext | null;
  /** System Prompt */
  systemPrompt: string | null;
  /** 加载状态 */
  loading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 刷新上下文 */
  refresh: () => Promise<void>;
  /** 构建 System Prompt */
  buildSystemPrompt: () => Promise<string>;
}

/**
 * 项目上下文 Hook
 *
 * @param projectId - 项目 ID
 */
export function useProjectContext(
  projectId: string | null,
): UseProjectContextReturn {
  const [context, setContext] = useState<ProjectContext | null>(null);
  const [systemPrompt, setSystemPrompt] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  /** 刷新项目上下文 */
  const refresh = useCallback(async () => {
    if (!projectId) {
      setContext(null);
      setSystemPrompt(null);
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const [ctx, prompt] = await Promise.all([
        invoke<ProjectContext>("get_project_context", { projectId }),
        invoke<string>("build_project_system_prompt", { projectId }),
      ]);

      setContext(ctx);
      setSystemPrompt(prompt);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  /** 构建 System Prompt */
  const buildSystemPrompt = useCallback(async (): Promise<string> => {
    if (!projectId) {
      return "";
    }

    const prompt = await invoke<string>("build_project_system_prompt", {
      projectId,
    });
    setSystemPrompt(prompt);
    return prompt;
  }, [projectId]);

  // 初始加载
  useEffect(() => {
    refresh();
  }, [refresh]);

  return {
    context,
    systemPrompt,
    loading,
    error,
    refresh,
    buildSystemPrompt,
  };
}

export default useProjectContext;
