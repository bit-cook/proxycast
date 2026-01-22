import React, { useState, useRef, useEffect } from "react";
import { User, Copy, Edit2, Trash2, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import {
  MessageListContainer,
  MessageWrapper,
  AvatarColumn,
  ContentColumn,
  MessageHeader,
  AvatarCircle,
  SenderName,
  TimeStamp,
  MessageBubble,
  MessageActions,
} from "../styles";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { StreamingRenderer } from "./StreamingRenderer";
import { TokenUsageDisplay } from "./TokenUsageDisplay";
import { Message } from "../types";
import type { A2UIFormData } from "@/components/content-creator/a2ui/types";
import type { ConfirmResponse } from "../types";
import logoImg from "/logo.png";

interface MessageListProps {
  messages: Message[];
  onDeleteMessage?: (id: string) => void;
  onEditMessage?: (id: string, content: string) => void;
  /** A2UI 表单提交回调 */
  onA2UISubmit?: (formData: A2UIFormData, messageId: string) => void;
  /** 文件写入回调 */
  onWriteFile?: (content: string, fileName: string) => void;
  /** 文件点击回调 */
  onFileClick?: (fileName: string, content: string) => void;
  /** 权限确认响应回调 */
  onPermissionResponse?: (response: ConfirmResponse) => void;
}

export const MessageList: React.FC<MessageListProps> = ({
  messages,
  onDeleteMessage,
  onEditMessage,
  onA2UISubmit,
  onWriteFile,
  onFileClick,
  onPermissionResponse,
}) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editContent, setEditContent] = useState("");
  const [isUserScrolling, setIsUserScrolling] = useState(false);
  const [shouldAutoScroll, setShouldAutoScroll] = useState(true);

  // 检测用户是否在手动滚动
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    let scrollTimeout: ReturnType<typeof setTimeout>;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = container;
      const isAtBottom = scrollHeight - scrollTop - clientHeight < 50; // 50px 容差

      setIsUserScrolling(true);
      setShouldAutoScroll(isAtBottom);

      // 清除之前的定时器
      clearTimeout(scrollTimeout);

      // 500ms 后认为用户停止滚动
      scrollTimeout = setTimeout(() => {
        setIsUserScrolling(false);
      }, 500);
    };

    container.addEventListener("scroll", handleScroll, { passive: true });

    return () => {
      container.removeEventListener("scroll", handleScroll);
      clearTimeout(scrollTimeout);
    };
  }, []);

  // 智能自动滚动：只在用户没有手动滚动且在底部时才自动滚动
  useEffect(() => {
    if (shouldAutoScroll && !isUserScrolling && scrollRef.current) {
      scrollRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, shouldAutoScroll, isUserScrolling]);

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  const handleCopy = async (content: string, id: string) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedId(id);
      toast.success("已复制到剪贴板");
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      toast.error("复制失败");
    }
  };

  const handleEdit = (msg: Message) => {
    setEditingId(msg.id);
    setEditContent(msg.content);
  };

  const handleSaveEdit = (id: string) => {
    if (onEditMessage && editContent.trim()) {
      onEditMessage(id, editContent);
    }
    setEditingId(null);
    setEditContent("");
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setEditContent("");
  };

  const handleDelete = (id: string) => {
    if (onDeleteMessage) {
      onDeleteMessage(id);
      toast.success("消息已删除");
    }
  };

  return (
    <MessageListContainer ref={containerRef}>
      <div className="py-8 flex flex-col">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground opacity-50">
            <img
              src={logoImg}
              alt="ProxyCast"
              className="w-12 h-12 mb-4 opacity-20"
            />
            <p className="text-lg font-medium">开始一段新的对话吧</p>
          </div>
        )}

        {messages.map((msg) => (
          <MessageWrapper key={msg.id} $isUser={msg.role === "user"}>
            <AvatarColumn>
              {msg.role === "user" ? (
                <AvatarCircle $isUser={true}>
                  <User size={20} />
                </AvatarCircle>
              ) : (
                <img
                  src={logoImg}
                  alt="ProxyCast"
                  style={{
                    width: 45,
                    height: 45,
                    minWidth: 45,
                    minHeight: 45,
                    borderRadius: 8,
                    display: "block",
                  }}
                />
              )}
            </AvatarColumn>

            <ContentColumn>
              <MessageHeader>
                <SenderName>
                  {msg.role === "user" ? "用户" : "ProxyCast"}
                </SenderName>
                <TimeStamp>{formatTime(msg.timestamp)}</TimeStamp>
              </MessageHeader>

              <MessageBubble $isUser={msg.role === "user"}>
                {editingId === msg.id ? (
                  <div className="flex flex-col gap-2">
                    <textarea
                      value={editContent}
                      onChange={(e) => setEditContent(e.target.value)}
                      className="w-full min-h-[100px] p-2 rounded border border-border bg-background resize-none focus:outline-none focus:ring-1 focus:ring-primary"
                      autoFocus
                    />
                    <div className="flex gap-2 justify-end">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={handleCancelEdit}
                      >
                        取消
                      </Button>
                      <Button size="sm" onClick={() => handleSaveEdit(msg.id)}>
                        保存
                      </Button>
                    </div>
                  </div>
                ) : msg.role === "assistant" ? (
                  /* 使用 StreamingRenderer 渲染 assistant 消息 - Requirements: 9.3, 9.4 */
                  <StreamingRenderer
                    content={msg.content}
                    isStreaming={msg.isThinking}
                    toolCalls={msg.toolCalls}
                    showCursor={msg.isThinking && !msg.content}
                    thinkingContent={msg.thinkingContent}
                    contentParts={msg.contentParts}
                    actionRequests={msg.actionRequests}
                    onA2UISubmit={
                      onA2UISubmit
                        ? (formData) => onA2UISubmit(formData, msg.id)
                        : undefined
                    }
                    onWriteFile={onWriteFile}
                    onFileClick={onFileClick}
                    onPermissionResponse={onPermissionResponse}
                  />
                ) : (
                  <MarkdownRenderer
                    content={msg.content}
                    onA2UISubmit={
                      onA2UISubmit
                        ? (formData) => onA2UISubmit(formData, msg.id)
                        : undefined
                    }
                  />
                )}

                {msg.images && msg.images.length > 0 && (
                  <div className="flex flex-wrap gap-2 mt-3">
                    {msg.images.map((img, i) => (
                      <img
                        key={i}
                        src={`data:${img.mediaType};base64,${img.data}`}
                        className="max-w-xs rounded-lg border border-border"
                        alt="attachment"
                      />
                    ))}
                  </div>
                )}

                {/* Token 使用量显示 - Requirements: 9.5 */}
                {msg.role === "assistant" && !msg.isThinking && msg.usage && (
                  <TokenUsageDisplay usage={msg.usage} />
                )}

                {editingId !== msg.id && (
                  <MessageActions className="message-actions">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-muted-foreground hover:text-foreground"
                      onClick={() => handleCopy(msg.content, msg.id)}
                    >
                      {copiedId === msg.id ? (
                        <Check size={12} className="text-green-500" />
                      ) : (
                        <Copy size={12} />
                      )}
                    </Button>
                    {msg.role === "user" && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-muted-foreground hover:text-foreground"
                        onClick={() => handleEdit(msg)}
                      >
                        <Edit2 size={12} />
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-muted-foreground hover:text-destructive"
                      onClick={() => handleDelete(msg.id)}
                    >
                      <Trash2 size={12} />
                    </Button>
                  </MessageActions>
                )}
              </MessageBubble>
            </ContentColumn>
          </MessageWrapper>
        ))}
        <div ref={scrollRef} />
      </div>
    </MessageListContainer>
  );
};
