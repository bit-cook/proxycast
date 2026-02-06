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
  stopAsterSession,
  confirmAsterAction,
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
  actionType: string;
  toolName?: string;
  arguments?: Record<string, unknown>;
  question?: string;
  timestamp: Date;
}

/** 确认响应 */
export interface ConfirmResponse {
  requestId: string;
  confirmed: boolean;
  response?: string;
}

/** Hook 配置选项 */
interface UseAsterAgentChatOptions {
  systemPrompt?: string;
  onWriteFile?: (content: string, fileName: string) => void;
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

export function useAsterAgentChat(options: UseAsterAgentChatOptions = {}) {
  const { onWriteFile } = options;

  // 状态
  const [isInitialized, setIsInitialized] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [topics, setTopics] = useState<Topic[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [pendingActions, setPendingActions] = useState<ActionRequired[]>([]);

  // Provider/Model（本地状态）
  const [providerType, setProviderType] = useState(
    () => localStorage.getItem("agent_pref_provider") || "claude",
  );
  const [model, setModel] = useState(
    () => localStorage.getItem("agent_pref_model") || "claude-sonnet-4-5",
  );

  // Refs
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const currentAssistantMsgIdRef = useRef<string | null>(null);

  // 持久化 provider/model
  useEffect(() => {
    localStorage.setItem("agent_pref_provider", providerType);
  }, [providerType]);

  useEffect(() => {
    localStorage.setItem("agent_pref_model", model);
  }, [model]);

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
      const newSessionId = await createAsterSession();
      setSessionId(newSessionId);
      return newSessionId;
    } catch (error) {
      console.error("[AsterChat] 创建会话失败:", error);
      toast.error("创建会话失败");
      return null;
    }
  }, [sessionId]);

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
    ) => {
      // 用户消息
      const userMsg: Message = {
        id: crypto.randomUUID(),
        role: "user",
        content,
        images: images.length > 0 ? images : undefined,
        timestamp: new Date(),
      };

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

      setMessages((prev) => [...prev, userMsg, assistantMsg]);
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
                const actionData: ActionRequired = {
                  requestId: rawEvent.request_id as string,
                  actionType: rawEvent.action_type as string,
                  toolName: (rawEvent.data as Record<string, unknown>)
                    ?.tool_name as string,
                  arguments: (rawEvent.data as Record<string, unknown>)
                    ?.arguments as Record<string, unknown>,
                  question: (rawEvent.data as Record<string, unknown>)
                    ?.question as string,
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

        await sendAsterMessageStream(
          content,
          activeSessionId,
          eventName,
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
    [ensureSession, onWriteFile, providerType, model],
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
  const confirmAction = useCallback(async (response: ConfirmResponse) => {
    try {
      await confirmAsterAction(
        response.requestId,
        response.confirmed,
        response.response,
      );
      // 移除已处理的请求
      setPendingActions((prev) =>
        prev.filter((a) => a.requestId !== response.requestId),
      );
    } catch (error) {
      console.error("[AsterChat] 确认失败:", error);
      toast.error("确认操作失败");
    }
  }, []);

  // 清空消息
  const clearMessages = useCallback(() => {
    setMessages([]);
    setSessionId(null);
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
      if (topicId === sessionId) return;

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
        setSessionId(topicId);
        toast.error("加载对话历史失败");
      }
    },
    [sessionId],
  );

  // 删除话题
  const deleteTopic = useCallback(
    async (topicId: string) => {
      // TODO: 实现后端删除
      setTopics((prev) => prev.filter((t) => t.id !== topicId));
      if (topicId === sessionId) {
        setSessionId(null);
        setMessages([]);
      }
      toast.success("话题已删除");
    },
    [sessionId],
  );

  // 兼容接口
  const handleStartProcess = useCallback(async () => {
    // Aster 不需要单独启动进程
  }, []);

  const handleStopProcess = useCallback(async () => {
    setSessionId(null);
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

    topics,
    sessionId,
    switchTopic,
    deleteTopic,
    loadTopics,

    // Aster 特有功能
    pendingActions,
    confirmAction,
  };
}
