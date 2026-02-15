/**
 * Aster Agent Chat Hook
 *
 * 基于 Aster 框架的聊天 hook
 * 接口与 useAgentChat 保持一致，便于切换
 */

import { useState, useEffect, useRef, useCallback } from "react";
import { toast } from "sonner";
import { safeListen } from "@/lib/dev-bridge";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  initAsterAgent,
  sendAsterMessageStream,
  createAsterSession,
  listAsterSessions,
  getAsterSession,
  renameAsterSession,
  deleteAsterSession,
  stopAsterSession,
  confirmAsterAction,
  submitAsterElicitationResponse,
  parseStreamEvent,
  type StreamEvent,
  type AsterSessionInfo,
} from "@/lib/api/agent";
import { Message, MessageImage, ContentPart } from "../types";

/** 话题信息 */
export interface Topic {
  id: string;
  title: string;
  createdAt: Date;
  messagesCount: number;
}

/** 权限确认请求 */
export interface ActionRequired {
  requestId: string;
  actionType: "tool_confirmation" | "ask_user" | "elicitation";
  toolName?: string;
  arguments?: Record<string, unknown>;
  prompt?: string;
  requestedSchema?: Record<string, unknown>;
  timestamp: Date;
}

/** 确认响应 */
export interface ConfirmResponse {
  requestId: string;
  confirmed: boolean;
  response?: string;
  actionType?: ActionRequired["actionType"];
  userData?: unknown;
}

/** Hook 配置选项 */
interface UseAsterAgentChatOptions {
  systemPrompt?: string;
  onWriteFile?: (content: string, fileName: string) => void;
  workspaceId: string;
}

// 音效相关（复用）
let toolcallAudio: HTMLAudioElement | null = null;
let typewriterAudio: HTMLAudioElement | null = null;
let lastTypewriterTime = 0;
const TYPEWRITER_INTERVAL = 120;

const initAudio = () => {
  if (!toolcallAudio) {
    toolcallAudio = new Audio("/sounds/tool-call.mp3");
    toolcallAudio.volume = 1;
    toolcallAudio.load();
  }
  if (!typewriterAudio) {
    typewriterAudio = new Audio("/sounds/typing.mp3");
    typewriterAudio.volume = 0.6;
    typewriterAudio.load();
  }
};

const getSoundEnabled = (): boolean => {
  return localStorage.getItem("proxycast_sound_enabled") === "true";
};

const playToolcallSound = () => {
  if (!getSoundEnabled()) return;
  initAudio();
  if (toolcallAudio) {
    toolcallAudio.currentTime = 0;
    toolcallAudio.play().catch(console.error);
  }
};

const playTypewriterSound = () => {
  if (!getSoundEnabled()) return;
  const now = Date.now();
  if (now - lastTypewriterTime < TYPEWRITER_INTERVAL) return;
  initAudio();
  if (typewriterAudio) {
    typewriterAudio.currentTime = 0;
    typewriterAudio.play().catch(console.error);
    lastTypewriterTime = now;
  }
};

// 持久化 helpers
const loadPersisted = <T>(key: string, defaultValue: T): T => {
  try {
    const stored = localStorage.getItem(key);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (e) {
    console.error(e);
  }
  return defaultValue;
};

const savePersisted = (key: string, value: unknown) => {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch (e) {
    console.error(e);
  }
};

const loadTransient = <T>(key: string, defaultValue: T): T => {
  try {
    const stored = sessionStorage.getItem(key);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (key.startsWith("aster_messages") && Array.isArray(parsed)) {
        return parsed.map((msg: any) => ({
          ...msg,
          timestamp: new Date(msg.timestamp),
        })) as unknown as T;
      }
      return parsed;
    }
  } catch (e) {
    console.error(e);
  }
  return defaultValue;
};

const saveTransient = (key: string, value: unknown) => {
  try {
    sessionStorage.setItem(key, JSON.stringify(value));
  } catch (e) {
    console.error(e);
  }
};

const DEFAULT_AGENT_PROVIDER = "claude";
const DEFAULT_AGENT_MODEL = "claude-sonnet-4-5";
const GLOBAL_PROVIDER_PREF_KEY = "agent_pref_provider_global";
const GLOBAL_MODEL_PREF_KEY = "agent_pref_model_global";
const GLOBAL_MIGRATED_PREF_KEY = "agent_pref_migrated_global";

const loadPersistedString = (key: string): string | null => {
  try {
    const stored = localStorage.getItem(key);
    if (stored === null) {
      return null;
    }

    try {
      const parsed = JSON.parse(stored);
      return typeof parsed === "string" ? parsed : stored;
    } catch {
      return stored;
    }
  } catch (e) {
    console.error(e);
    return null;
  }
};

interface AgentPreferences {
  providerType: string;
  model: string;
}

interface AgentPreferenceKeys {
  providerKey: string;
  modelKey: string;
  migratedKey: string;
}

const getAgentPreferenceKeys = (
  workspaceId?: string | null,
): AgentPreferenceKeys => {
  const resolvedWorkspaceId = workspaceId?.trim();
  if (!resolvedWorkspaceId) {
    return {
      providerKey: GLOBAL_PROVIDER_PREF_KEY,
      modelKey: GLOBAL_MODEL_PREF_KEY,
      migratedKey: GLOBAL_MIGRATED_PREF_KEY,
    };
  }

  return {
    providerKey: `agent_pref_provider_${resolvedWorkspaceId}`,
    modelKey: `agent_pref_model_${resolvedWorkspaceId}`,
    migratedKey: `agent_pref_migrated_${resolvedWorkspaceId}`,
  };
};

const resolveWorkspaceAgentPreferences = (
  workspaceId?: string | null,
): AgentPreferences => {
  const { providerKey, modelKey, migratedKey } =
    getAgentPreferenceKeys(workspaceId);

  const scopedProvider = loadPersistedString(providerKey);
  const scopedModel = loadPersistedString(modelKey);
  if (scopedProvider || scopedModel) {
    return {
      providerType: scopedProvider || DEFAULT_AGENT_PROVIDER,
      model: scopedModel || DEFAULT_AGENT_MODEL,
    };
  }

  const migrated = loadPersisted<boolean>(migratedKey, false);
  if (!migrated) {
    const legacyProvider =
      loadPersistedString("agent_pref_provider") ||
      loadPersistedString(GLOBAL_PROVIDER_PREF_KEY);
    const legacyModel =
      loadPersistedString("agent_pref_model") ||
      loadPersistedString(GLOBAL_MODEL_PREF_KEY);

    if (legacyProvider) {
      savePersisted(providerKey, legacyProvider);
    }
    if (legacyModel) {
      savePersisted(modelKey, legacyModel);
    }

    savePersisted(migratedKey, true);

    return {
      providerType: legacyProvider || DEFAULT_AGENT_PROVIDER,
      model: legacyModel || DEFAULT_AGENT_MODEL,
    };
  }

  return {
    providerType: DEFAULT_AGENT_PROVIDER,
    model: DEFAULT_AGENT_MODEL,
  };
};

/**
 * 将前端 Provider 类型映射到 Aster Provider 名称
 */
const mapProviderName = (providerType: string): string => {
  const mapping: Record<string, string> = {
    // OpenAI 兼容
    openai: "openai",
    "gpt-4": "openai",
    "gpt-4o": "openai",
    // Anthropic
    claude: "anthropic",
    anthropic: "anthropic",
    // Google
    google: "google",
    gemini: "google",
    // DeepSeek（OpenAI 兼容）
    deepseek: "deepseek",
    "deepseek-reasoner": "deepseek",
    // Ollama
    ollama: "ollama",
    // OpenRouter
    openrouter: "openrouter",
    // 其他（OpenAI 兼容）
    groq: "openai",
    mistral: "openai",
  };
  return mapping[providerType.toLowerCase()] || providerType;
};

export function useAsterAgentChat(options: UseAsterAgentChatOptions) {
  const { onWriteFile, workspaceId } = options;

  const getRequiredWorkspaceId = useCallback((): string => {
    const resolvedWorkspaceId = workspaceId?.trim();
    if (!resolvedWorkspaceId) {
      throw new Error("缺少项目工作区，请先选择项目后再使用 Agent");
    }
    return resolvedWorkspaceId;
  }, [workspaceId]);

  const getScopedKey = useCallback(
    (key: string): string => {
      const resolvedWorkspaceId = workspaceId?.trim();
      return resolvedWorkspaceId
        ? `${key}_${resolvedWorkspaceId}`
        : `${key}_global`;
    },
    [workspaceId],
  );

  const getScopedSessionKey = useCallback(
    () => getScopedKey("aster_curr_sessionId"),
    [getScopedKey],
  );
  const getScopedMessagesKey = useCallback(
    () => getScopedKey("aster_messages"),
    [getScopedKey],
  );
  const getScopedPersistedSessionKey = useCallback(
    () => getScopedKey("aster_last_sessionId"),
    [getScopedKey],
  );

  // 状态
  const [isInitialized, setIsInitialized] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(() => {
    if (!workspaceId?.trim()) {
      return null;
    }

    const scopedSessionId = loadTransient<string | null>(
      `aster_curr_sessionId_${workspaceId.trim()}`,
      null,
    );
    if (scopedSessionId) {
      return scopedSessionId;
    }

    return loadPersisted<string | null>(
      `aster_last_sessionId_${workspaceId.trim()}`,
      null,
    );
  });
  const [messages, setMessages] = useState<Message[]>(() =>
    workspaceId?.trim()
      ? loadTransient<Message[]>(`aster_messages_${workspaceId.trim()}`, [])
      : [],
  );
  const [topics, setTopics] = useState<Topic[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [pendingActions, setPendingActions] = useState<ActionRequired[]>([]);

  const initialPreferencesRef = useRef<AgentPreferences>(
    resolveWorkspaceAgentPreferences(workspaceId),
  );

  // Provider/Model（按工作区保存）
  const [providerType, setProviderType] = useState(
    () => initialPreferencesRef.current.providerType,
  );
  const [model, setModel] = useState(() => initialPreferencesRef.current.model);

  // Refs
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const currentAssistantMsgIdRef = useRef<string | null>(null);
  const restoredWorkspaceRef = useRef<string | null>(null);
  const hydratedSessionRef = useRef<string | null>(null);
  const skipAutoRestoreRef = useRef(false);
  const scopedProviderPrefKeyRef = useRef<string>(
    getAgentPreferenceKeys(workspaceId).providerKey,
  );
  const scopedModelPrefKeyRef = useRef<string>(
    getAgentPreferenceKeys(workspaceId).modelKey,
  );

  // workspace 变化时恢复 Provider/Model 偏好
  useEffect(() => {
    const { providerKey, modelKey } = getAgentPreferenceKeys(workspaceId);

    scopedProviderPrefKeyRef.current = providerKey;
    scopedModelPrefKeyRef.current = modelKey;

    const scopedPreferences = resolveWorkspaceAgentPreferences(workspaceId);
    setProviderType(scopedPreferences.providerType);
    setModel(scopedPreferences.model);

    savePersisted(providerKey, scopedPreferences.providerType);
    savePersisted(modelKey, scopedPreferences.model);
  }, [workspaceId]);

  // 持久化 provider/model（仅写当前工作区）
  useEffect(() => {
    savePersisted(scopedProviderPrefKeyRef.current, providerType);
  }, [providerType]);

  useEffect(() => {
    savePersisted(scopedModelPrefKeyRef.current, model);
  }, [model]);

  useEffect(() => {
    const resolvedWorkspaceId = workspaceId?.trim();
    if (!resolvedWorkspaceId) {
      return;
    }

    const scopedSessionKey = getScopedSessionKey();
    const scopedPersistedSessionKey = getScopedPersistedSessionKey();

    saveTransient(scopedSessionKey, sessionId);
    savePersisted(scopedPersistedSessionKey, sessionId);

    if (sessionId) {
      savePersisted(
        `agent_session_workspace_${sessionId}`,
        resolvedWorkspaceId,
      );
    }
  }, [
    getScopedPersistedSessionKey,
    getScopedSessionKey,
    sessionId,
    workspaceId,
  ]);

  useEffect(() => {
    if (!workspaceId?.trim()) {
      return;
    }
    saveTransient(getScopedMessagesKey(), messages);
  }, [getScopedMessagesKey, messages, workspaceId]);

  // workspace 变化时恢复对应会话状态
  useEffect(() => {
    if (!workspaceId?.trim()) {
      setSessionId(null);
      setMessages([]);
      setPendingActions([]);
      restoredWorkspaceRef.current = null;
      hydratedSessionRef.current = null;
      skipAutoRestoreRef.current = false;
      return;
    }

    const scopedSessionId =
      loadTransient<string | null>(getScopedSessionKey(), null) ??
      loadPersisted<string | null>(getScopedPersistedSessionKey(), null);

    const scopedMessages = loadTransient<Message[]>(getScopedMessagesKey(), []);

    setSessionId(scopedSessionId);
    setMessages(scopedMessages);
    setPendingActions([]);
    restoredWorkspaceRef.current = null;
    hydratedSessionRef.current = null;
    skipAutoRestoreRef.current = false;
  }, [
    getScopedMessagesKey,
    getScopedPersistedSessionKey,
    getScopedSessionKey,
    workspaceId,
  ]);

  // 初始化 Aster Agent
  useEffect(() => {
    const init = async () => {
      try {
        await initAsterAgent();
        setIsInitialized(true);
        console.log("[AsterChat] Agent 初始化成功");
        // 初始化后加载话题列表
        const sessions = await listAsterSessions();
        const topicList: Topic[] = sessions.map((s: AsterSessionInfo) => ({
          id: s.id,
          title:
            s.name ||
            `话题 ${new Date(s.created_at * 1000).toLocaleDateString("zh-CN")}`,
          createdAt: new Date(s.created_at * 1000),
          messagesCount: s.messages_count ?? 0,
        }));
        setTopics(topicList);
      } catch (err) {
        console.error("[AsterChat] 初始化失败:", err);
      }
    };
    init();
  }, []);

  useEffect(() => {
    if (!isInitialized) {
      return;
    }

    if (!workspaceId?.trim()) {
      setTopics([]);
      return;
    }

    listAsterSessions()
      .then((sessions) => {
        const topicList: Topic[] = sessions.map((s: AsterSessionInfo) => ({
          id: s.id,
          title:
            s.name ||
            `话题 ${new Date(s.created_at * 1000).toLocaleDateString("zh-CN")}`,
          createdAt: new Date(s.created_at * 1000),
          messagesCount: s.messages_count ?? 0,
        }));
        setTopics(topicList);
      })
      .catch((error) => {
        console.error("[AsterChat] 加载话题失败:", error);
      });
  }, [isInitialized, workspaceId]);

  // 加载话题列表
  const loadTopics = useCallback(async () => {
    try {
      const sessions = await listAsterSessions();
      const topicList: Topic[] = sessions.map((s: AsterSessionInfo) => ({
        id: s.id,
        title:
          s.name ||
          `话题 ${new Date(s.created_at * 1000).toLocaleDateString("zh-CN")}`,
        createdAt: new Date(s.created_at * 1000),
        messagesCount: s.messages_count ?? 0,
      }));
      setTopics(topicList);
    } catch (error) {
      console.error("[AsterChat] 加载话题失败:", error);
    }
  }, []);

  // 确保有会话
  const ensureSession = useCallback(async (): Promise<string | null> => {
    if (sessionId) return sessionId;

    try {
      const resolvedWorkspaceId = getRequiredWorkspaceId();
      const newSessionId = await createAsterSession(resolvedWorkspaceId);
      setSessionId(newSessionId);
      skipAutoRestoreRef.current = false;
      return newSessionId;
    } catch (error) {
      console.error("[AsterChat] 创建会话失败:", error);
      toast.error(`创建会话失败: ${error}`);
      return null;
    }
  }, [getRequiredWorkspaceId, sessionId]);

  // 辅助函数：追加文本到 contentParts
  const appendTextToParts = (
    parts: ContentPart[],
    text: string,
  ): ContentPart[] => {
    const newParts = [...parts];
    const lastPart = newParts[newParts.length - 1];

    if (lastPart && lastPart.type === "text") {
      newParts[newParts.length - 1] = {
        type: "text",
        text: lastPart.text + text,
      };
    } else {
      newParts.push({ type: "text", text });
    }
    return newParts;
  };

  // 发送消息
  const sendMessage = useCallback(
    async (
      content: string,
      images: MessageImage[],
      _webSearch?: boolean,
      _thinking?: boolean,
      skipUserMessage = false,
    ) => {
      // 助手消息占位符
      const assistantMsgId = crypto.randomUUID();
      const assistantMsg: Message = {
        id: assistantMsgId,
        role: "assistant",
        content: "",
        timestamp: new Date(),
        isThinking: true,
        thinkingContent: "思考中...",
        contentParts: [],
      };

      if (skipUserMessage) {
        setMessages((prev) => [...prev, assistantMsg]);
      } else {
        // 用户消息
        const userMsg: Message = {
          id: crypto.randomUUID(),
          role: "user",
          content,
          images: images.length > 0 ? images : undefined,
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, userMsg, assistantMsg]);
      }
      setIsSending(true);
      currentAssistantMsgIdRef.current = assistantMsgId;

      let accumulatedContent = "";
      let unlisten: UnlistenFn | null = null;

      try {
        const activeSessionId = await ensureSession();
        if (!activeSessionId) throw new Error("无法创建会话");

        const eventName = `aster_stream_${assistantMsgId}`;

        // 设置事件监听
        unlisten = await safeListen<StreamEvent>(eventName, (event) => {
          const data = parseStreamEvent(event.payload);
          if (!data) return;

          switch (data.type) {
            case "text_delta":
              accumulatedContent += data.text;
              playTypewriterSound();
              setMessages((prev) =>
                prev.map((msg) =>
                  msg.id === assistantMsgId
                    ? {
                        ...msg,
                        content: accumulatedContent,
                        thinkingContent: undefined,
                        contentParts: appendTextToParts(
                          msg.contentParts || [],
                          data.text,
                        ),
                      }
                    : msg,
                ),
              );
              break;

            case "tool_start": {
              playToolcallSound();
              const newToolCall = {
                id: data.tool_id,
                name: data.tool_name,
                arguments: data.arguments,
                status: "running" as const,
                startTime: new Date(),
              };

              // 检查是否是写入文件工具
              const toolName = data.tool_name.toLowerCase();
              if (toolName.includes("write") || toolName.includes("create")) {
                try {
                  const args = JSON.parse(data.arguments || "{}");
                  const filePath = args.path || args.file_path || args.filePath;
                  const fileContent = args.content || args.text || "";
                  if (filePath && fileContent && onWriteFile) {
                    onWriteFile(fileContent, filePath);
                  }
                } catch (e) {
                  console.warn("[AsterChat] 解析工具参数失败:", e);
                }
              }

              setMessages((prev) =>
                prev.map((msg) => {
                  if (msg.id !== assistantMsgId) return msg;
                  if (msg.toolCalls?.find((tc) => tc.id === data.tool_id))
                    return msg;
                  return {
                    ...msg,
                    toolCalls: [...(msg.toolCalls || []), newToolCall],
                    contentParts: [
                      ...(msg.contentParts || []),
                      { type: "tool_use" as const, toolCall: newToolCall },
                    ],
                  };
                }),
              );
              break;
            }

            case "tool_end":
              setMessages((prev) =>
                prev.map((msg) => {
                  if (msg.id !== assistantMsgId) return msg;
                  const updatedToolCalls = (msg.toolCalls || []).map((tc) =>
                    tc.id === data.tool_id
                      ? {
                          ...tc,
                          status: data.result.success
                            ? ("completed" as const)
                            : ("failed" as const),
                          result: data.result,
                          endTime: new Date(),
                        }
                      : tc,
                  );
                  const updatedContentParts = (msg.contentParts || []).map(
                    (part) => {
                      if (
                        part.type === "tool_use" &&
                        part.toolCall.id === data.tool_id
                      ) {
                        return {
                          ...part,
                          toolCall: {
                            ...part.toolCall,
                            status: data.result.success
                              ? ("completed" as const)
                              : ("failed" as const),
                            result: data.result,
                            endTime: new Date(),
                          },
                        };
                      }
                      return part;
                    },
                  );
                  return {
                    ...msg,
                    toolCalls: updatedToolCalls,
                    contentParts: updatedContentParts,
                  };
                }),
              );
              break;

            case "final_done":
              setMessages((prev) =>
                prev.map((msg) =>
                  msg.id === assistantMsgId
                    ? {
                        ...msg,
                        isThinking: false,
                        content: accumulatedContent || "(无响应)",
                      }
                    : msg,
                ),
              );
              setIsSending(false);
              unlistenRef.current = null;
              currentAssistantMsgIdRef.current = null;
              if (unlisten) {
                unlisten();
                unlisten = null;
              }
              break;

            case "error":
              toast.error(`响应错误: ${data.message}`);
              setMessages((prev) =>
                prev.map((msg) =>
                  msg.id === assistantMsgId
                    ? {
                        ...msg,
                        isThinking: false,
                        content: accumulatedContent || `错误: ${data.message}`,
                      }
                    : msg,
                ),
              );
              setIsSending(false);
              if (unlisten) {
                unlisten();
                unlisten = null;
              }
              break;

            // 处理权限确认请求
            default: {
              // 检查是否是 action_required 事件（通过原始 payload）
              const rawEvent = event.payload as unknown as Record<
                string,
                unknown
              >;
              if (rawEvent.type === "action_required") {
                const actionPayload =
                  (rawEvent.data as Record<string, unknown> | undefined) || {};
                const actionData: ActionRequired = {
                  requestId: rawEvent.request_id as string,
                  actionType:
                    rawEvent.action_type as ActionRequired["actionType"],
                  toolName: actionPayload.tool_name as string | undefined,
                  arguments: actionPayload.arguments as
                    | Record<string, unknown>
                    | undefined,
                  prompt:
                    (actionPayload.prompt as string | undefined) ||
                    (actionPayload.message as string | undefined),
                  requestedSchema: actionPayload.requested_schema as
                    | Record<string, unknown>
                    | undefined,
                  timestamp: new Date(),
                };
                setPendingActions((prev) => [...prev, actionData]);
              }
              break;
            }
          }
        });

        unlistenRef.current = unlisten;

        // 发送请求
        const imagesToSend =
          images.length > 0
            ? images.map((img) => ({
                data: img.data,
                media_type: img.mediaType,
              }))
            : undefined;

        // 构建 Provider 配置
        const providerConfig = {
          provider_name: mapProviderName(providerType),
          model_name: model,
        };

        const resolvedWorkspaceId = getRequiredWorkspaceId();

        await sendAsterMessageStream(
          content,
          activeSessionId,
          eventName,
          resolvedWorkspaceId,
          imagesToSend,
          providerConfig,
        );
      } catch (error) {
        console.error("[AsterChat] 发送失败:", error);
        toast.error(`发送失败: ${error}`);
        setMessages((prev) => prev.filter((msg) => msg.id !== assistantMsgId));
        setIsSending(false);
        if (unlisten) unlisten();
      }
    },
    [ensureSession, getRequiredWorkspaceId, onWriteFile, providerType, model],
  );

  // 停止发送
  const stopSending = useCallback(async () => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }

    if (sessionId) {
      try {
        await stopAsterSession(sessionId);
      } catch (e) {
        console.error("[AsterChat] 停止失败:", e);
      }
    }

    if (currentAssistantMsgIdRef.current) {
      setMessages((prev) =>
        prev.map((msg) =>
          msg.id === currentAssistantMsgIdRef.current
            ? { ...msg, isThinking: false, content: msg.content || "(已停止)" }
            : msg,
        ),
      );
      currentAssistantMsgIdRef.current = null;
    }

    setIsSending(false);
    toast.info("已停止生成");
  }, [sessionId]);

  // 确认权限请求
  const confirmAction = useCallback(
    async (response: ConfirmResponse) => {
      try {
        const actionType =
          response.actionType ||
          pendingActions.find((item) => item.requestId === response.requestId)
            ?.actionType;

        if (actionType === "elicitation" || actionType === "ask_user") {
          if (!sessionId) {
            throw new Error("缺少会话 ID，无法提交 elicitation 响应");
          }

          let userData: unknown;
          if (!response.confirmed) {
            userData = "";
          } else if (response.userData !== undefined) {
            userData = response.userData;
          } else if (response.response !== undefined) {
            const rawResponse = response.response.trim();
            if (!rawResponse) {
              userData = "";
            } else {
              try {
                userData = JSON.parse(rawResponse);
              } catch {
                userData = rawResponse;
              }
            }
          } else {
            userData = "";
          }

          await submitAsterElicitationResponse(
            sessionId,
            response.requestId,
            userData,
          );
        } else {
          await confirmAsterAction(
            response.requestId,
            response.confirmed,
            response.response,
          );
        }

        // 移除已处理的请求
        setPendingActions((prev) =>
          prev.filter((a) => a.requestId !== response.requestId),
        );
      } catch (error) {
        console.error("[AsterChat] 确认失败:", error);
        toast.error("确认操作失败");
      }
    },
    [pendingActions, sessionId],
  );

  // 兼容 Native 接口：权限响应处理
  const handlePermissionResponse = useCallback(
    async (response: ConfirmResponse) => {
      await confirmAction(response);
    },
    [confirmAction],
  );

  // 兼容 Native 接口：触发 AI 引导（仅生成助手消息，不注入用户气泡）
  const triggerAIGuide = useCallback(async () => {
    await sendMessage("", [], false, false, true);
  }, [sendMessage]);

  // 清空消息
  const clearMessages = useCallback(() => {
    setMessages([]);
    setSessionId(null);
    setPendingActions([]);
    restoredWorkspaceRef.current = null;
    hydratedSessionRef.current = null;
    skipAutoRestoreRef.current = true;
    toast.success("新话题已创建");
  }, []);

  // 删除消息
  const deleteMessage = useCallback((id: string) => {
    setMessages((prev) => prev.filter((msg) => msg.id !== id));
  }, []);

  // 编辑消息
  const editMessage = useCallback((id: string, newContent: string) => {
    setMessages((prev) =>
      prev.map((msg) =>
        msg.id === id ? { ...msg, content: newContent } : msg,
      ),
    );
  }, []);

  // 切换话题
  const switchTopic = useCallback(
    async (topicId: string) => {
      if (topicId === sessionId && messages.length > 0) return;

      skipAutoRestoreRef.current = false;
      try {
        const detail = await getAsterSession(topicId);
        const loadedMessages: Message[] = detail.messages.map((msg, index) => {
          // 从 TauriMessageContent 数组中提取文本和 contentParts
          const contentParts: ContentPart[] = [];
          const textParts: string[] = [];

          for (const part of msg.content) {
            if (part.type === "text" && part.text) {
              textParts.push(part.text);
              contentParts.push({ type: "text", text: part.text });
            } else if (part.type === "thinking" && part.text) {
              contentParts.push({ type: "thinking", text: part.text });
            }
          }

          return {
            id: `${topicId}-${index}`,
            role: msg.role as "user" | "assistant",
            content: textParts.join("\n"),
            contentParts: contentParts.length > 0 ? contentParts : undefined,
            timestamp: new Date(msg.timestamp * 1000),
            isThinking: false,
          };
        });

        setMessages(loadedMessages);
        setSessionId(topicId);
        toast.info("已切换话题");
      } catch (error) {
        console.error("[AsterChat] 切换话题失败:", error);
        setMessages([]);
        setSessionId(null);
        saveTransient(getScopedSessionKey(), null);
        savePersisted(getScopedPersistedSessionKey(), null);
        toast.error("加载对话历史失败");
      }
    },
    [
      getScopedPersistedSessionKey,
      getScopedSessionKey,
      messages.length,
      sessionId,
    ],
  );

  // 自动恢复当前 workspace 最近会话
  useEffect(() => {
    const resolvedWorkspaceId = workspaceId?.trim();
    if (!resolvedWorkspaceId) return;
    if (!isInitialized) return;
    if (skipAutoRestoreRef.current) return;
    if (sessionId) return;
    if (topics.length === 0) return;
    if (restoredWorkspaceRef.current === resolvedWorkspaceId) return;

    restoredWorkspaceRef.current = resolvedWorkspaceId;

    const scopedCandidate =
      loadTransient<string | null>(getScopedSessionKey(), null) ||
      loadPersisted<string | null>(getScopedPersistedSessionKey(), null);
    const mappedFallbackCandidate =
      topics.find(
        (topic) =>
          loadPersisted<string | null>(
            `agent_session_workspace_${topic.id}`,
            null,
          ) === resolvedWorkspaceId,
      )?.id || null;

    const targetSessionId = scopedCandidate || mappedFallbackCandidate;
    if (!targetSessionId) {
      return;
    }

    switchTopic(targetSessionId).catch((error) => {
      console.warn("[AsterChat] 自动恢复会话失败:", error);
      saveTransient(getScopedSessionKey(), null);
      savePersisted(getScopedPersistedSessionKey(), null);
    });
  }, [
    getScopedPersistedSessionKey,
    getScopedSessionKey,
    isInitialized,
    sessionId,
    switchTopic,
    topics,
    workspaceId,
  ]);

  useEffect(() => {
    if (sessionId) {
      skipAutoRestoreRef.current = false;
    }
  }, [sessionId]);

  // 有 sessionId 但消息为空时，主动回填历史
  useEffect(() => {
    if (!sessionId) return;

    if (messages.length > 0) {
      hydratedSessionRef.current = sessionId;
      return;
    }

    if (hydratedSessionRef.current === sessionId) {
      return;
    }

    hydratedSessionRef.current = sessionId;

    switchTopic(sessionId).catch((error) => {
      console.warn("[AsterChat] 会话水合失败:", error);
      hydratedSessionRef.current = null;
    });
  }, [messages.length, sessionId, switchTopic]);

  // 删除话题
  const deleteTopic = useCallback(
    async (topicId: string) => {
      try {
        await deleteAsterSession(topicId);
        await loadTopics();

        if (topicId === sessionId) {
          setSessionId(null);
          setMessages([]);
          setPendingActions([]);
          hydratedSessionRef.current = null;
          restoredWorkspaceRef.current = null;
          saveTransient(getScopedSessionKey(), null);
          savePersisted(getScopedPersistedSessionKey(), null);
        }

        toast.success("话题已删除");
      } catch (error) {
        console.error("[AsterChat] 删除话题失败:", error);
        toast.error("删除话题失败");
      }
    },
    [
      getScopedPersistedSessionKey,
      getScopedSessionKey,
      loadTopics,
      sessionId,
    ],
  );

  // 重命名话题（持久化）
  const renameTopic = useCallback(async (topicId: string, newTitle: string) => {
    const normalizedTitle = newTitle.trim();
    if (!normalizedTitle) {
      return;
    }

    try {
      await renameAsterSession(topicId, normalizedTitle);
      await loadTopics();
      toast.success("话题已重命名");
    } catch (error) {
      console.error("[AsterChat] 重命名话题失败:", error);
      toast.error("重命名失败");
    }
  }, [loadTopics]);

  // 兼容接口
  const handleStartProcess = useCallback(async () => {
    // Aster 不需要单独启动进程
  }, []);

  const handleStopProcess = useCallback(async () => {
    setSessionId(null);
    setMessages([]);
    setPendingActions([]);
    restoredWorkspaceRef.current = null;
    hydratedSessionRef.current = null;
  }, []);

  return {
    // 兼容 useAgentChat 接口
    processStatus: { running: isInitialized },
    handleStartProcess,
    handleStopProcess,

    providerType,
    setProviderType,
    model,
    setModel,
    providerConfig: {}, // 简化版本
    isConfigLoading: false,

    messages,
    isSending,
    sendMessage,
    stopSending,
    clearMessages,
    deleteMessage,
    editMessage,
    handlePermissionResponse,
    triggerAIGuide,

    topics,
    sessionId,
    switchTopic,
    deleteTopic,
    renameTopic,
    loadTopics,

    // Aster 特有功能
    pendingActions,
    confirmAction,
  };
}
