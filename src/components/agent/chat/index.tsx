/**
 * AI Agent 聊天页面
 *
 * 包含聊天区域和侧边栏（话题/技能列表）
 * 支持内容创作模式下的布局过渡和步骤引导
 * 当主题为 general 时，使用 GeneralChat 组件实现
 */

import { useState, useCallback, useMemo, useEffect, useRef } from "react";
import styled from "styled-components";
import { useAgentChat } from "./hooks/useAgentChat";
import { useSessionFiles } from "./hooks/useSessionFiles";
import { ChatNavbar } from "./components/ChatNavbar";
import { ChatSidebar } from "./components/ChatSidebar";
import { ChatSettings } from "./components/ChatSettings";
import { MessageList } from "./components/MessageList";
import { Inputbar } from "./components/Inputbar";
import { EmptyState, CreationMode } from "./components/EmptyState";
import { type TaskFile } from "./components/TaskFiles";
import { LayoutTransition } from "@/components/content-creator/core/LayoutTransition/LayoutTransition";
import { StepProgress } from "@/components/content-creator/core/StepGuide/StepProgress";
import { useWorkflow } from "@/components/content-creator/hooks/useWorkflow";
import { CanvasFactory } from "@/components/content-creator/canvas/CanvasFactory";
import {
  createInitialCanvasState,
  type CanvasStateUnion,
} from "@/components/content-creator/canvas/canvasUtils";
import { createInitialDocumentState } from "@/components/content-creator/canvas/document";
import { CanvasPanel as GeneralCanvasPanel } from "@/components/general-chat/canvas";
import {
  type CanvasState as GeneralCanvasState,
  DEFAULT_CANVAS_STATE,
} from "@/components/general-chat/types";
import { createInitialMusicState } from "@/components/content-creator/canvas/music/types";
import { parseLyrics } from "@/components/content-creator/canvas/music/utils/lyricsParser";
import {
  generateContentCreationPrompt,
  isContentCreationTheme,
} from "@/components/content-creator/utils/systemPrompt";

import type { MessageImage } from "./types";
import type { ThemeType, LayoutMode } from "@/components/content-creator/types";
import type { A2UIFormData } from "@/components/content-creator/a2ui/types";

// 文件名到步骤索引的映射（静态常量，移到组件外部避免重复创建）
const FILE_TO_STEP_MAP: Record<string, number> = {
  "brief.md": 0, // 明确需求
  "specification.md": 1, // 调研收集
  "research.md": 1, // 调研收集（备选文件名）
  "outline.md": 2, // 生成大纲
  "draft.md": 3, // 撰写内容
  "article.md": 4, // 润色优化
};

const PageContainer = styled.div`
  display: flex;
  height: 100%;
  width: 100%;
  background-color: hsl(var(--background));
`;

const MainArea = styled.div`
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
`;

const ChatContainer = styled.div`
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  height: 100%;
`;

const ChatContent = styled.div`
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 0 16px;
  overflow: hidden;
  height: 100%;
`;

// 主题到 ThemeType 的映射
const THEME_MAP: Record<string, ThemeType> = {
  general: "general",
  knowledge: "knowledge",
  planning: "planning",
  social: "social-media",
  image: "poster",
  office: "document",
  video: "video",
  music: "music",
};

export function AgentChatPage({
  onNavigate: _onNavigate,
}: {
  onNavigate?: (page: string) => void;
}) {
  const [showSidebar, setShowSidebar] = useState(true);
  const [showSettings, setShowSettings] = useState(false);
  const [input, setInput] = useState("");

  // 内容创作相关状态
  const [activeTheme, setActiveTheme] = useState<string>("general");
  const [creationMode, setCreationMode] = useState<CreationMode>("guided");
  const [layoutMode, setLayoutMode] = useState<LayoutMode>("chat");

  // 画布状态（支持多种画布类型）
  const [canvasState, setCanvasState] = useState<CanvasStateUnion | null>(null);

  // General 主题专用画布状态
  const [generalCanvasState, setGeneralCanvasState] =
    useState<GeneralCanvasState>(DEFAULT_CANVAS_STATE);

  // 任务文件状态
  const [taskFiles, setTaskFiles] = useState<TaskFile[]>([]);
  const [taskFilesExpanded, setTaskFilesExpanded] = useState(false);
  const [selectedFileId, setSelectedFileId] = useState<string | undefined>();

  // 用于追踪已处理的消息 ID，避免重复处理
  const processedMessageIds = useRef<Set<string>>(new Set());

  // 文件写入回调 ref（用于传递给 useAgentChat）
  const handleWriteFileRef =
    useRef<(content: string, fileName: string) => void>();

  // 工作流状态（仅在内容创作模式下使用）
  const mappedTheme = THEME_MAP[activeTheme] || "general";
  const { steps, currentStepIndex, goToStep, completeStep } = useWorkflow(
    mappedTheme,
    creationMode,
  );

  // 判断是否为内容创作模式
  const isContentCreationMode = isContentCreationTheme(activeTheme);

  // 生成系统提示词（仅在内容创作模式下，根据创作模式生成不同提示词）
  const systemPrompt = useMemo(() => {
    if (isContentCreationMode) {
      return generateContentCreationPrompt(mappedTheme, creationMode);
    }
    return undefined;
  }, [isContentCreationMode, mappedTheme, creationMode]);

  // 使用 Agent Chat Hook（传递系统提示词）
  const {
    processStatus,
    providerType,
    setProviderType,
    model,
    setModel,
    messages,
    isSending,
    sendMessage,
    stopSending,
    clearMessages,
    deleteMessage,
    editMessage,
    handlePermissionResponse,
    topics,
    sessionId,
    switchTopic: originalSwitchTopic,
    deleteTopic,
  } = useAgentChat({
    systemPrompt,
    onWriteFile: (content, fileName) => {
      // 使用 ref 调用最新的 handleWriteFile
      handleWriteFileRef.current?.(content, fileName);
    },
  });

  // 会话文件持久化 hook
  const {
    saveFile: saveSessionFile,
    files: sessionFiles,
    readFile: readSessionFile,
    meta: sessionMeta,
  } = useSessionFiles({
    sessionId,
    theme: mappedTheme,
    creationMode,
    autoInit: true,
  });

  // 追踪已恢复元数据和文件的会话 ID
  const restoredMetaSessionId = useRef<string | null>(null);
  const restoredFilesSessionId = useRef<string | null>(null);

  // 当 sessionMeta 加载完成时，恢复主题和创建模式
  useEffect(() => {
    if (!sessionId || !sessionMeta) {
      return;
    }

    // 检查 sessionMeta 是否属于当前 sessionId
    if (sessionMeta.sessionId !== sessionId) {
      return;
    }

    // 避免重复恢复
    if (restoredMetaSessionId.current === sessionId) {
      return;
    }

    console.log("[AgentChatPage] 恢复会话元数据:", sessionId, sessionMeta);

    // 从会话元数据恢复主题
    if (sessionMeta.theme) {
      const themeEntry = Object.entries(THEME_MAP).find(
        ([_, v]) => v === sessionMeta.theme,
      );
      if (themeEntry) {
        console.log("[AgentChatPage] 恢复主题:", themeEntry[0]);
        setActiveTheme(themeEntry[0]);
      }
    }

    // 从会话元数据恢复创建模式
    if (sessionMeta.creationMode) {
      console.log("[AgentChatPage] 恢复创建模式:", sessionMeta.creationMode);
      setCreationMode(sessionMeta.creationMode as CreationMode);
    }

    restoredMetaSessionId.current = sessionId;
  }, [sessionId, sessionMeta]);

  // 当 sessionFiles 加载完成时，恢复文件到 taskFiles
  useEffect(() => {
    if (!sessionId || sessionFiles.length === 0) {
      return;
    }

    // 避免重复恢复
    if (restoredFilesSessionId.current === sessionId) {
      return;
    }

    // 如果当前已有 taskFiles，说明是本次会话新生成的文件，不需要从持久化恢复
    if (taskFiles.length > 0) {
      restoredFilesSessionId.current = sessionId;
      return;
    }

    console.log(
      "[AgentChatPage] 开始恢复文件:",
      sessionId,
      sessionFiles.length,
      "个文件",
    );

    // 恢复文件到 taskFiles
    const restoreFiles = async () => {
      const restoredFiles: TaskFile[] = [];

      for (const file of sessionFiles) {
        try {
          const content = await readSessionFile(file.name);
          if (content) {
            restoredFiles.push({
              id: crypto.randomUUID(),
              name: file.name,
              type: file.fileType === "document" ? "document" : "document",
              content,
              version: 1,
              createdAt: file.createdAt,
              updatedAt: file.updatedAt,
            });
          }
        } catch (err) {
          console.error("[AgentChatPage] 恢复文件失败:", file.name, err);
        }
      }

      if (restoredFiles.length > 0) {
        console.log(
          "[AgentChatPage] 从持久化存储恢复",
          restoredFiles.length,
          "个文件",
        );
        setTaskFiles(restoredFiles);
      }
      restoredFilesSessionId.current = sessionId;
    };

    restoreFiles();
  }, [sessionId, sessionFiles, readSessionFile, taskFiles.length]);

  // 包装 switchTopic，在切换话题时重置相关状态
  const switchTopic = useCallback(
    async (topicId: string) => {
      // 先重置本地状态
      setLayoutMode("chat");
      setCanvasState(null);
      setGeneralCanvasState(DEFAULT_CANVAS_STATE);
      setTaskFiles([]);
      setSelectedFileId(undefined);
      processedMessageIds.current.clear();
      // 清空已恢复的会话 ID，以便新话题能触发恢复
      restoredMetaSessionId.current = null;
      restoredFilesSessionId.current = null;

      // 然后调用原始的 switchTopic
      await originalSwitchTopic(topicId);
    },
    [originalSwitchTopic],
  );

  /**
   * 从 AI 响应中提取文档内容
   * 支持多种格式：
   * 1. <document>...</document> 标签
   * 2. ```markdown ... ``` 代码块
   * 3. 以 # 开头的 Markdown 内容（整个响应）
   */
  const extractDocumentContent = useCallback(
    (content: string): string | null => {
      // 1. 检查 <document> 标签
      const documentMatch = content.match(/<document>([\s\S]*?)<\/document>/);
      if (documentMatch) {
        return documentMatch[1].trim();
      }

      // 2. 检查 markdown 代码块
      const markdownMatch = content.match(/```(?:markdown|md)\n([\s\S]*?)```/);
      if (markdownMatch) {
        return markdownMatch[1].trim();
      }

      // 3. 如果整个内容以 # 开头且长度超过 200 字符，认为是文档
      if (content.trim().startsWith("#") && content.length > 200) {
        return content.trim();
      }

      return null;
    },
    [],
  );

  // 监听 AI 消息变化，自动提取文档内容
  useEffect(() => {
    if (!isContentCreationMode) return;

    // 找到最新的 assistant 消息
    const lastAssistantMsg = [...messages]
      .reverse()
      .find(
        (msg) => msg.role === "assistant" && !msg.isThinking && msg.content,
      );

    if (!lastAssistantMsg) return;

    // 检查是否已处理过
    if (processedMessageIds.current.has(lastAssistantMsg.id)) return;

    // 提取文档内容
    const docContent = extractDocumentContent(lastAssistantMsg.content);
    if (docContent) {
      // 标记为已处理
      processedMessageIds.current.add(lastAssistantMsg.id);

      // 更新画布内容（仅文档类型画布支持流式更新）
      setCanvasState((prev) => {
        // 如果是海报主题，不自动更新画布
        if (mappedTheme === "poster") {
          return prev;
        }

        if (!prev || prev.type !== "document") {
          return createInitialDocumentState(docContent);
        }
        // 添加新版本
        const newVersion = {
          id: crypto.randomUUID(),
          content: docContent,
          createdAt: Date.now(),
          description: `AI 生成 - 版本 ${prev.versions.length + 1}`,
        };
        return {
          ...prev,
          content: docContent,
          versions: [...prev.versions, newVersion],
          currentVersionId: newVersion.id,
        };
      });

      // 自动打开画布
      setLayoutMode("chat-canvas");
    }
  }, [messages, isContentCreationMode, extractDocumentContent, mappedTheme]);

  const handleSend = useCallback(
    async (
      images?: MessageImage[],
      webSearch?: boolean,
      thinking?: boolean,
    ) => {
      if (!input.trim() && (!images || images.length === 0)) return;
      const text = input;
      setInput("");
      await sendMessage(text, images || [], webSearch, thinking);
    },
    [input, sendMessage],
  );

  const handleClearMessages = useCallback(() => {
    clearMessages();
    setInput("");
    // 重置布局模式
    setLayoutMode("chat");
    // 恢复侧边栏显示
    setShowSidebar(true);
    // 清理画布和文件状态
    setCanvasState(null);
    setGeneralCanvasState(DEFAULT_CANVAS_STATE);
    setTaskFiles([]);
    setSelectedFileId(undefined);
    processedMessageIds.current.clear();
  }, [clearMessages]);

  // 当开始对话时自动折叠侧边栏
  const hasMessages = messages.length > 0;
  useEffect(() => {
    if (hasMessages) {
      setShowSidebar(false);
    }
  }, [hasMessages]);

  // 当有文件时默认在画布中显示最后一个文件
  useEffect(() => {
    if (taskFiles.length > 0) {
      const lastFile = taskFiles[taskFiles.length - 1];
      // 设置选中的文件
      setSelectedFileId(lastFile.id);
      // 如果文件有内容，在画布中显示
      if (lastFile.content) {
        setCanvasState((prev) => {
          if (mappedTheme === "music") {
            const sections = parseLyrics(lastFile.content!);
            if (!prev || prev.type !== "music") {
              const musicState = createInitialMusicState();
              musicState.sections = sections;
              const titleMatch = lastFile.content!.match(/^#\s*(.+)$/m);
              if (titleMatch) {
                musicState.spec.title = titleMatch[1].trim();
              }
              return musicState;
            }
            return { ...prev, sections };
          }
          if (!prev || prev.type !== "document") {
            return createInitialDocumentState(lastFile.content!);
          }
          return { ...prev, content: lastFile.content! };
        });
        setLayoutMode("chat-canvas");
      }
    }
  }, [taskFiles, mappedTheme]);

  const handleToggleSidebar = () => {
    setShowSidebar(!showSidebar);
  };

  // 切换画布显示
  const handleToggleCanvas = useCallback(() => {
    // General 主题使用专门的画布
    if (activeTheme === "general") {
      setGeneralCanvasState((prev) => ({
        ...prev,
        isOpen: !prev.isOpen,
        contentType:
          prev.contentType === "empty" ? "markdown" : prev.contentType,
        content: prev.content || "# 新文档\n\n在这里开始编写内容...",
      }));
      setLayoutMode((prev) => (prev === "chat" ? "chat-canvas" : "chat"));
      return;
    }

    setLayoutMode((prev) => {
      if (prev === "chat") {
        // 打开画布时，如果没有画布状态则创建初始状态
        if (!canvasState) {
          const initialState =
            createInitialCanvasState(
              mappedTheme,
              "# 新文档\n\n在这里开始编写内容...",
            ) ||
            createInitialDocumentState("# 新文档\n\n在这里开始编写内容...");
          setCanvasState(initialState);
        }
        return "chat-canvas";
      }
      return "chat";
    });
  }, [canvasState, mappedTheme, activeTheme]);

  // 关闭画布
  const handleCloseCanvas = useCallback(() => {
    setLayoutMode("chat");
    // General 主题关闭画布状态
    if (activeTheme === "general") {
      setGeneralCanvasState((prev) => ({ ...prev, isOpen: false }));
    }
  }, [activeTheme]);

  // 处理文件写入 - 同名文件更新内容，不同名文件独立保存
  const handleWriteFile = useCallback(
    (content: string, fileName: string) => {
      console.log(
        "[AgentChatPage] 收到文件写入:",
        fileName,
        content.length,
        "字符",
      );

      // General 主题使用专门的画布处理
      if (activeTheme === "general") {
        const ext = fileName.split(".").pop()?.toLowerCase() || "";
        const isCode = [
          "js",
          "ts",
          "tsx",
          "jsx",
          "py",
          "rs",
          "go",
          "java",
          "c",
          "cpp",
          "h",
          "css",
          "scss",
          "json",
          "yaml",
          "yml",
          "toml",
          "xml",
          "html",
          "sql",
          "sh",
          "bash",
        ].includes(ext);
        const isMd = ["md", "markdown"].includes(ext);

        console.log(
          "[AgentChatPage] General 主题文件写入:",
          fileName,
          "类型:",
          isCode ? "code" : isMd ? "markdown" : "file",
        );

        setGeneralCanvasState({
          isOpen: true,
          contentType: isCode ? "code" : isMd ? "markdown" : "file",
          content,
          language: isCode ? ext : undefined,
          filename: fileName,
          isEditing: false,
        });
        setLayoutMode("chat-canvas");
        return;
      }

      const now = Date.now();

      // 持久化文件到会话目录
      saveSessionFile(fileName, content).catch((err) => {
        console.error("[AgentChatPage] 持久化文件失败:", err);
      });

      // 根据文件名推进工作流步骤
      const stepIndex = FILE_TO_STEP_MAP[fileName];
      if (
        stepIndex !== undefined &&
        stepIndex === currentStepIndex &&
        isContentCreationMode
      ) {
        console.log(
          "[AgentChatPage] 推进工作流步骤:",
          stepIndex,
          "->",
          stepIndex + 1,
        );
        completeStep({
          aiOutput: { fileName, preview: content.slice(0, 100) },
        });
      }

      // 更新或创建文件
      setTaskFiles((prev) => {
        // 查找同名文件
        const existingIndex = prev.findIndex((f) => f.name === fileName);

        if (existingIndex >= 0) {
          // 同名文件存在 - 直接更新内容（不创建新版本）
          const existing = prev[existingIndex];

          // 如果内容完全相同，跳过
          if (existing.content === content) {
            console.log("[AgentChatPage] 文件内容相同，跳过:", fileName);
            setSelectedFileId(existing.id);
            return prev;
          }

          // 更新文件内容
          console.log("[AgentChatPage] 更新文件:", fileName);
          const updated = [...prev];
          updated[existingIndex] = {
            ...existing,
            content,
            updatedAt: now,
          };
          setSelectedFileId(existing.id);
          return updated;
        }

        // 新文件 - 添加到列表
        console.log("[AgentChatPage] 创建新文件:", fileName);
        const newFile: TaskFile = {
          id: crypto.randomUUID(),
          name: fileName,
          type: "document",
          content,
          version: 1,
          createdAt: now,
          updatedAt: now,
        };
        setSelectedFileId(newFile.id);
        return [...prev, newFile];
      });

      // 更新画布内容
      setCanvasState((prev) => {
        console.log("[AgentChatPage] 更新画布状态:", {
          prevType: prev?.type,
          mappedTheme,
          contentLength: content.length,
        });

        // 海报主题不自动更新画布
        if (mappedTheme === "poster") {
          return prev;
        }

        // 音乐主题：解析歌词并更新 sections
        if (mappedTheme === "music") {
          const sections = parseLyrics(content);
          if (!prev || prev.type !== "music") {
            const musicState = createInitialMusicState();
            musicState.sections = sections;
            // 尝试从内容中提取歌曲名称
            const titleMatch = content.match(/^#\s*(.+)$/m);
            if (titleMatch) {
              musicState.spec.title = titleMatch[1].trim();
            }
            console.log("[AgentChatPage] 创建新音乐状态");
            return musicState;
          }
          // 更新现有音乐状态的 sections
          return {
            ...prev,
            sections,
          };
        }

        // 文档类型画布
        if (!prev || prev.type !== "document") {
          console.log("[AgentChatPage] 创建新文档状态");
          return createInitialDocumentState(content);
        }
        console.log("[AgentChatPage] 更新现有文档状态");
        return {
          ...prev,
          content,
        };
      });

      // 自动打开画布显示流式内容
      setLayoutMode("chat-canvas");
    },
    [
      activeTheme, // 添加 activeTheme 依赖
      currentStepIndex,
      isContentCreationMode,
      completeStep,
      mappedTheme,
      saveSessionFile,
    ],
  );

  // 更新 ref，供 useAgentChat 使用
  useEffect(() => {
    handleWriteFileRef.current = handleWriteFile;
  }, [handleWriteFile]);

  // 处理文件点击 - 在画布中显示文件内容
  const handleFileClick = useCallback(
    (fileName: string, content: string) => {
      console.log("[AgentChatPage] 文件点击:", fileName, "主题:", activeTheme);

      // General 主题使用专门的画布
      if (activeTheme === "general") {
        const ext = fileName.split(".").pop()?.toLowerCase() || "";
        const isCode = [
          "js",
          "ts",
          "tsx",
          "jsx",
          "py",
          "rs",
          "go",
          "java",
          "c",
          "cpp",
          "h",
          "css",
          "scss",
          "json",
          "yaml",
          "yml",
          "toml",
          "xml",
          "html",
          "sql",
          "sh",
          "bash",
        ].includes(ext);
        const isMd = ["md", "markdown"].includes(ext);

        setGeneralCanvasState({
          isOpen: true,
          contentType: isCode ? "code" : isMd ? "markdown" : "file",
          content,
          language: isCode ? ext : undefined,
          filename: fileName,
          isEditing: false,
        });
        setLayoutMode("chat-canvas");
        return;
      }

      // 查找或创建任务文件
      setTaskFiles((prev) => {
        const existingFile = prev.find((f) => f.name === fileName);
        if (existingFile) {
          setSelectedFileId(existingFile.id);
          return prev;
        }
        // 如果文件不存在，添加到列表
        const newFile: TaskFile = {
          id: crypto.randomUUID(),
          name: fileName,
          type: "document",
          content,
          version: 1,
          createdAt: Date.now(),
          updatedAt: Date.now(),
        };
        setSelectedFileId(newFile.id);
        return [...prev, newFile];
      });

      // 更新画布内容
      setCanvasState((prev) => {
        // 音乐主题：解析歌词并更新 sections
        if (mappedTheme === "music") {
          const sections = parseLyrics(content);
          if (!prev || prev.type !== "music") {
            const musicState = createInitialMusicState();
            musicState.sections = sections;
            const titleMatch = content.match(/^#\s*(.+)$/m);
            if (titleMatch) {
              musicState.spec.title = titleMatch[1].trim();
            }
            return musicState;
          }
          return { ...prev, sections };
        }

        // 文档类型画布
        if (!prev || prev.type !== "document") {
          return createInitialDocumentState(content);
        }
        return {
          ...prev,
          content,
        };
      });

      // 打开画布
      setLayoutMode("chat-canvas");
    },
    [activeTheme, mappedTheme],
  );

  // 在画布中打开代码块（General 主题专用）- 通过 onFileClick 触发
  // 此函数保留以备将来扩展使用

  // 处理任务文件点击 - 在画布中显示文件内容
  const handleTaskFileClick = useCallback(
    (file: TaskFile) => {
      if (file.type === "document" && file.content) {
        setSelectedFileId(file.id);
        setCanvasState((prev) => {
          // 音乐主题：解析歌词并更新 sections
          if (mappedTheme === "music") {
            const sections = parseLyrics(file.content!);
            if (!prev || prev.type !== "music") {
              const musicState = createInitialMusicState();
              musicState.sections = sections;
              const titleMatch = file.content!.match(/^#\s*(.+)$/m);
              if (titleMatch) {
                musicState.spec.title = titleMatch[1].trim();
              }
              return musicState;
            }
            return { ...prev, sections };
          }

          // 文档类型画布
          if (!prev || prev.type !== "document") {
            return createInitialDocumentState(file.content!);
          }
          return {
            ...prev,
            content: file.content!,
          };
        });
        // 只打开画布，不关闭文件列表（让用户自己关闭）
        setLayoutMode("chat-canvas");
      }
    },
    [mappedTheme],
  );

  // A2UI 表单提交处理
  const handleA2UISubmit = useCallback(
    async (formData: A2UIFormData, _messageId: string) => {
      console.log("[AgentChatPage] A2UI 表单提交:", formData);

      // 将表单数据格式化为用户消息
      const formattedData = Object.entries(formData)
        .map(([key, value]) => {
          if (Array.isArray(value)) {
            return `- ${key}: ${value.join(", ")}`;
          }
          return `- ${key}: ${value}`;
        })
        .join("\n");

      const userMessage = `我的选择：\n${formattedData}`;

      // 发送用户消息
      await sendMessage(userMessage, [], false, false);
    },
    [sendMessage],
  );

  // 聊天区域内容
  const chatContent = (
    <ChatContainer>
      {/* 步骤进度条 - 仅在内容创作模式且有消息时显示 */}
      {isContentCreationMode && hasMessages && steps.length > 0 && (
        <StepProgress
          steps={steps}
          currentIndex={currentStepIndex}
          onStepClick={goToStep}
        />
      )}

      {hasMessages ? (
        <ChatContent>
          <MessageList
            messages={messages}
            onDeleteMessage={deleteMessage}
            onEditMessage={editMessage}
            onA2UISubmit={handleA2UISubmit}
            onWriteFile={handleWriteFile}
            onFileClick={handleFileClick}
            onPermissionResponse={handlePermissionResponse}
          />
        </ChatContent>
      ) : (
        <EmptyState
          input={input}
          setInput={setInput}
          onSend={(text) => {
            setInput(text);
            setTimeout(() => handleSend([], false, false), 0);
          }}
          creationMode={creationMode}
          onCreationModeChange={setCreationMode}
          activeTheme={activeTheme}
          onThemeChange={setActiveTheme}
        />
      )}

      {hasMessages && (
        <>
          <Inputbar
            input={input}
            setInput={setInput}
            onSend={handleSend}
            onStop={stopSending}
            isLoading={isSending}
            disabled={!processStatus.running && false}
            onClearMessages={handleClearMessages}
            onToggleCanvas={handleToggleCanvas}
            isCanvasOpen={layoutMode === "chat-canvas"}
            taskFiles={taskFiles}
            selectedFileId={selectedFileId}
            taskFilesExpanded={taskFilesExpanded}
            onToggleTaskFiles={() => setTaskFilesExpanded(!taskFilesExpanded)}
            onTaskFileClick={handleTaskFileClick}
          />
        </>
      )}
    </ChatContainer>
  );

  // 画布区域内容
  const canvasContent = useMemo(() => {
    // General 主题使用专门的预览画布
    if (activeTheme === "general") {
      if (generalCanvasState.isOpen) {
        return (
          <GeneralCanvasPanel
            state={generalCanvasState}
            onClose={handleCloseCanvas}
            onContentChange={(content) =>
              setGeneralCanvasState((prev) => ({ ...prev, content }))
            }
          />
        );
      }
      return null;
    }

    // 其他主题使用 CanvasFactory
    if (canvasState) {
      return (
        <CanvasFactory
          theme={mappedTheme}
          state={canvasState}
          onStateChange={setCanvasState}
          onClose={handleCloseCanvas}
          isStreaming={isSending}
        />
      );
    }
    return null;
  }, [
    activeTheme,
    generalCanvasState,
    canvasState,
    mappedTheme,
    handleCloseCanvas,
    isSending,
  ]);

  // ========== 渲染逻辑 ==========

  // 所有主题统一使用 useAgentChat 的状态和渲染逻辑
  // General 主题与其他主题的区别仅在于不显示步骤进度条
  return (
    <PageContainer>
      {showSidebar && (
        <ChatSidebar
          onNewChat={handleClearMessages}
          topics={topics}
          currentTopicId={sessionId}
          onSwitchTopic={switchTopic}
          onDeleteTopic={deleteTopic}
        />
      )}

      <MainArea>
        <ChatNavbar
          providerType={providerType}
          setProviderType={setProviderType}
          model={model}
          setModel={setModel}
          isRunning={isSending}
          onToggleHistory={handleToggleSidebar}
          onToggleFullscreen={() => {}}
          onToggleSettings={() => setShowSettings(!showSettings)}
        />

        {/* 使用布局过渡组件 */}
        <LayoutTransition
          mode={layoutMode}
          chatContent={chatContent}
          canvasContent={canvasContent}
        />
      </MainArea>

      {showSettings && <ChatSettings onClose={() => setShowSettings(false)} />}
    </PageContainer>
  );
}
