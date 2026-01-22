/**
 * @file useSession.ts
 * @description 会话管理 Hook
 * @module components/general-chat/hooks/useSession
 *
 * 封装会话加载、切换、自动标题生成等逻辑
 *
 * @requirements 1.2, 1.5
 */

import { useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useGeneralChatStore } from "../store/useGeneralChatStore";
import type { Session } from "../types";

/**
 * 后端会话数据结构
 */
interface BackendSession {
  id: string;
  name: string;
  created_at: number;
  updated_at: number;
  metadata?: Record<string, unknown>;
}

/**
 * 后端会话详情数据结构
 */
interface BackendSessionDetail {
  session: BackendSession;
  messages: unknown[];
  message_count: number;
}

/**
 * 转换后端会话为前端格式
 */
const convertSession = (backend: BackendSession): Session => ({
  id: backend.id,
  name: backend.name,
  createdAt: backend.created_at,
  updatedAt: backend.updated_at,
  messageCount: 0,
});

/**
 * useSession Hook 配置
 */
interface UseSessionOptions {
  /** 自动加载会话列表 */
  autoLoad?: boolean;
  /** 会话切换回调 */
  onSessionChange?: (sessionId: string | null) => void;
}

/**
 * 会话管理 Hook
 */
export const useSession = (options: UseSessionOptions = {}) => {
  const { autoLoad = true, onSessionChange } = options;

  const {
    sessions,
    currentSessionId,
    setSessions,
    selectSession,
    createSession: createNewSession,
    deleteSession: _removeSession,
    updateSession,
  } = useGeneralChatStore();

  /**
   * 加载会话列表
   */
  const loadSessions = useCallback(async () => {
    try {
      const backendSessions = await invoke<BackendSession[]>(
        "general_chat_list_sessions",
      );
      const frontendSessions = backendSessions.map(convertSession);
      setSessions(frontendSessions);
    } catch (error) {
      console.error("加载会话列表失败:", error);
    }
  }, [setSessions]);

  /**
   * 创建新会话
   */
  const createSession = useCallback(
    async (name?: string): Promise<string | null> => {
      try {
        const _session = await invoke<BackendSession>(
          "general_chat_create_session",
          {
            name: name || undefined,
            metadata: undefined,
          },
        );
        // 使用 store 的 createSession 方法，它会自动添加会话并设置为当前会话
        const sessionId = await createNewSession();
        onSessionChange?.(sessionId);
        return sessionId;
      } catch (error) {
        console.error("创建会话失败:", error);
        return null;
      }
    },
    [createNewSession, onSessionChange],
  );

  /**
   * 切换会话
   */
  const switchSession = useCallback(
    async (sessionId: string) => {
      try {
        // 加载会话详情
        const detail = await invoke<BackendSessionDetail>(
          "general_chat_get_session",
          {
            sessionId,
            messageLimit: 50,
          },
        );

        // 更新会话消息数量
        updateSession(sessionId, { messageCount: detail.message_count });

        // 切换当前会话
        selectSession(sessionId);
        onSessionChange?.(sessionId);
      } catch (error) {
        console.error("切换会话失败:", error);
      }
    },
    [selectSession, updateSession, onSessionChange],
  );

  /**
   * 删除会话
   */
  const deleteSession = useCallback(
    async (sessionId: string) => {
      try {
        await invoke("general_chat_delete_session", { sessionId });
        // 使用 store 的 deleteSession 方法，它会自动处理会话切换逻辑
        await useGeneralChatStore.getState().deleteSession(sessionId);

        // 获取新的当前会话 ID 并触发回调
        const newCurrentId = useGeneralChatStore.getState().currentSessionId;
        onSessionChange?.(newCurrentId);
      } catch (error) {
        console.error("删除会话失败:", error);
      }
    },
    [onSessionChange],
  );

  /**
   * 重命名会话
   */
  const renameSession = useCallback(
    async (sessionId: string, name: string) => {
      try {
        await invoke("general_chat_rename_session", { sessionId, name });
        updateSession(sessionId, { name });
      } catch (error) {
        console.error("重命名会话失败:", error);
      }
    },
    [updateSession],
  );

  /**
   * 自动生成会话标题
   * 基于第一条用户消息生成
   */
  const generateTitle = useCallback(
    async (sessionId: string, firstMessage: string) => {
      // 简单实现：截取前 20 个字符作为标题
      const title =
        firstMessage.slice(0, 20) + (firstMessage.length > 20 ? "..." : "");
      await renameSession(sessionId, title);
    },
    [renameSession],
  );

  // 自动加载会话列表
  useEffect(() => {
    if (autoLoad) {
      loadSessions();
    }
  }, [autoLoad, loadSessions]);

  return {
    sessions,
    currentSessionId,
    loadSessions,
    createSession,
    switchSession,
    deleteSession,
    renameSession,
    generateTitle,
  };
};

export default useSession;
