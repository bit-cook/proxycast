import React from "react";
import { InputbarCore } from "./components/InputbarCore";
import { toast } from "sonner";
import { useState, useCallback, useRef } from "react";
import styled from "styled-components";
import type { MessageImage } from "../../types";
import { TaskFileList, type TaskFile } from "../TaskFiles";
import { FolderOpen, ChevronUp } from "lucide-react";

// 任务文件触发器区域（在输入框上方，与输入框对齐）
const TaskFilesArea = styled.div`
  display: flex;
  justify-content: flex-end;
  padding: 0 18px 8px 18px;
  width: 100%;
  max-width: 900px;
  margin: 0 auto;
`;

// 按钮和面板的包装容器
const TaskFilesWrapper = styled.div`
  position: relative;
`;

// 任务文件按钮
const TaskFilesButton = styled.button<{
  $expanded?: boolean;
  $hasFiles?: boolean;
}>`
  display: ${(props) => (props.$hasFiles ? "flex" : "none")};
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: hsl(var(--background));
  border: 1px solid hsl(var(--border));
  border-radius: 8px;
  font-size: 13px;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    border-color: hsl(var(--primary) / 0.5);
    color: hsl(var(--foreground));
  }

  ${(props) =>
    props.$expanded &&
    `
    border-color: hsl(var(--primary));
    color: hsl(var(--foreground));
    background: hsl(var(--primary) / 0.05);
  `}
`;

const FileCount = styled.span`
  font-weight: 500;
`;

const ChevronIcon = styled.span<{ $expanded?: boolean }>`
  display: flex;
  align-items: center;
  transform: ${(props) =>
    props.$expanded ? "rotate(0deg)" : "rotate(180deg)"};
  transition: transform 0.2s;
`;

interface InputbarProps {
  input: string;
  setInput: (value: string) => void;
  onSend: (
    images?: MessageImage[],
    webSearch?: boolean,
    thinking?: boolean,
  ) => void;
  /** 停止生成回调 */
  onStop?: () => void;
  isLoading: boolean;
  disabled?: boolean;
  onClearMessages?: () => void;
  /** 切换画布显示 */
  onToggleCanvas?: () => void;
  /** 画布是否打开 */
  isCanvasOpen?: boolean;
  /** 任务文件列表 */
  taskFiles?: TaskFile[];
  /** 选中的文件 ID */
  selectedFileId?: string;
  /** 任务文件面板是否展开 */
  taskFilesExpanded?: boolean;
  /** 切换任务文件面板 */
  onToggleTaskFiles?: () => void;
  /** 文件点击回调 */
  onTaskFileClick?: (file: TaskFile) => void;
}

export const Inputbar: React.FC<InputbarProps> = ({
  input,
  setInput,
  onSend,
  onStop,
  isLoading,
  disabled,
  onClearMessages,
  onToggleCanvas,
  isCanvasOpen = false,
  taskFiles = [],
  selectedFileId,
  taskFilesExpanded = false,
  onToggleTaskFiles,
  onTaskFileClick,
}) => {
  const [activeTools, setActiveTools] = useState<Record<string, boolean>>({});
  const [pendingImages, setPendingImages] = useState<MessageImage[]>([]);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleToolClick = useCallback(
    (tool: string) => {
      switch (tool) {
        case "thinking":
        case "web_search":
          setActiveTools((prev) => {
            const newState = { ...prev, [tool]: !prev[tool] };
            toast.info(
              `${tool === "thinking" ? "深度思考" : "联网搜索"}${newState[tool] ? "已开启" : "已关闭"}`,
            );
            return newState;
          });
          break;
        case "clear":
          setInput("");
          setPendingImages([]);
          toast.success("已清除输入");
          break;
        case "new_topic":
          onClearMessages?.();
          setInput("");
          setPendingImages([]);
          break;
        case "attach":
          fileInputRef.current?.click();
          break;
        case "quick_action":
          toast.info("快捷指令功能开发中...");
          break;
        case "translate":
          toast.info("翻译功能开发中...");
          break;
        case "fullscreen":
          setIsFullscreen((prev) => !prev);
          toast.info(isFullscreen ? "已退出全屏" : "已进入全屏编辑");
          break;
        case "canvas":
          onToggleCanvas?.();
          break;
        default:
          break;
      }
    },
    [setInput, onClearMessages, isFullscreen, onToggleCanvas],
  );

  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (!files || files.length === 0) return;

      Array.from(files).forEach((file) => {
        if (file.type.startsWith("image/")) {
          const reader = new FileReader();
          reader.onload = (event) => {
            const base64 = event.target?.result as string;
            const base64Data = base64.split(",")[1];
            setPendingImages((prev) => [
              ...prev,
              {
                data: base64Data,
                mediaType: file.type,
              },
            ]);
            toast.success(`已添加图片: ${file.name}`);
          };
          reader.readAsDataURL(file);
        } else {
          toast.info(`暂不支持该文件类型: ${file.type}`);
        }
      });

      e.target.value = "";
    },
    [],
  );

  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items;
    if (!items) return;

    for (const item of items) {
      if (item.type.startsWith("image/")) {
        e.preventDefault();
        const file = item.getAsFile();
        if (file) {
          const reader = new FileReader();
          reader.onload = (event) => {
            const base64 = event.target?.result as string;
            const base64Data = base64.split(",")[1];
            setPendingImages((prev) => [
              ...prev,
              {
                data: base64Data,
                mediaType: item.type,
              },
            ]);
            toast.success("已粘贴图片");
          };
          reader.readAsDataURL(file);
        }
        break;
      }
    }
  }, []);

  // 文件拖拽处理
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const files = e.dataTransfer.files;
    if (!files || files.length === 0) return;

    Array.from(files).forEach((file) => {
      if (file.type.startsWith("image/")) {
        const reader = new FileReader();
        reader.onload = (event) => {
          const base64 = event.target?.result as string;
          const base64Data = base64.split(",")[1];
          setPendingImages((prev) => [
            ...prev,
            {
              data: base64Data,
              mediaType: file.type,
            },
          ]);
          toast.success(`已添加图片: ${file.name}`);
        };
        reader.readAsDataURL(file);
      } else {
        toast.info(`暂不支持该文件类型: ${file.type}`);
      }
    });
  }, []);

  const handleRemoveImage = useCallback((index: number) => {
    setPendingImages((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handleSend = useCallback(() => {
    if (!input.trim() && pendingImages.length === 0) return;
    const webSearch = activeTools["web_search"] || false;
    const thinking = activeTools["thinking"] || false;
    onSend(
      pendingImages.length > 0 ? pendingImages : undefined,
      webSearch,
      thinking,
    );
    setPendingImages([]);
  }, [input, pendingImages, onSend, activeTools]);

  const handleToggleTaskFiles = useCallback(() => {
    onToggleTaskFiles?.();
  }, [onToggleTaskFiles]);

  return (
    <div
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      className={
        isFullscreen ? "fixed inset-0 z-50 bg-background p-4 flex flex-col" : ""
      }
    >
      {/* 任务文件区域 - 在输入框上方 */}
      {taskFiles.length > 0 && (
        <TaskFilesArea>
          {/* 按钮和面板的包装容器 */}
          <TaskFilesWrapper>
            {/* 任务文件面板 */}
            <TaskFileList
              files={taskFiles}
              selectedFileId={selectedFileId}
              onFileClick={onTaskFileClick}
              expanded={taskFilesExpanded}
              onExpandedChange={(expanded) => {
                if (expanded !== taskFilesExpanded) {
                  onToggleTaskFiles?.();
                }
              }}
            />
            {/* 任务文件按钮 */}
            <TaskFilesButton
              $hasFiles={taskFiles.length > 0}
              $expanded={taskFilesExpanded}
              onClick={handleToggleTaskFiles}
              data-task-files-trigger
            >
              <FolderOpen size={14} />
              任务文件
              <FileCount>({taskFiles.length})</FileCount>
              <ChevronIcon $expanded={taskFilesExpanded}>
                <ChevronUp size={14} />
              </ChevronIcon>
            </TaskFilesButton>
          </TaskFilesWrapper>
        </TaskFilesArea>
      )}
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        multiple
        style={{ display: "none" }}
        onChange={handleFileSelect}
      />
      <InputbarCore
        text={input}
        setText={setInput}
        onSend={handleSend}
        onStop={onStop}
        isLoading={isLoading}
        disabled={disabled}
        onToolClick={handleToolClick}
        activeTools={activeTools}
        pendingImages={pendingImages}
        onRemoveImage={handleRemoveImage}
        onPaste={handlePaste}
        isFullscreen={isFullscreen}
        isCanvasOpen={isCanvasOpen}
      />
    </div>
  );
};
