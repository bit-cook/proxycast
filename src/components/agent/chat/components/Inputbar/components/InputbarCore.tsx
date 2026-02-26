import React from "react";
import {
  Container,
  InputBarContainer,
  StyledTextarea,
  BottomBar,
  LeftSection,
  RightSection,
  SendButton,
  DragHandle,
  ImagePreviewContainer,
  ImagePreviewItem,
  ImagePreviewImg,
  ImageRemoveButton,
  ToolButton,
} from "../styles";
import { InputbarTools } from "./InputbarTools";
import { ArrowUp, Square, X, Languages } from "lucide-react";
import { BaseComposer } from "@/components/input-kit";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { MessageImage } from "../../../types";

interface InputbarCoreProps {
  text: string;
  setText: (text: string) => void;
  onSend: () => void;
  /** 停止生成回调 */
  onStop?: () => void;
  isLoading?: boolean;
  disabled?: boolean;
  activeTools: Record<string, boolean>;
  executionStrategy?: "react" | "code_orchestrated" | "auto";
  showExecutionStrategy?: boolean;
  onToolClick: (tool: string) => void;
  pendingImages?: MessageImage[];
  onRemoveImage?: (index: number) => void;
  onPaste?: (e: React.ClipboardEvent) => void;
  isFullscreen?: boolean;
  /** 画布是否打开 */
  isCanvasOpen?: boolean;
  /** Textarea ref（用于 CharacterMention） */
  textareaRef?: React.RefObject<HTMLTextAreaElement>;
  /** 输入框底栏左侧扩展区域 */
  leftExtra?: React.ReactNode;
  /** 输入框底栏右侧扩展区域 */
  rightExtra?: React.ReactNode;
  /** 输入框内部顶部扩展区域（textarea 上方） */
  topExtra?: React.ReactNode;
}

export const InputbarCore: React.FC<InputbarCoreProps> = ({
  text,
  setText,
  onSend,
  onStop,
  isLoading = false,
  disabled = false,
  activeTools,
  executionStrategy,
  showExecutionStrategy = false,
  onToolClick,
  pendingImages = [],
  onRemoveImage,
  onPaste,
  isFullscreen = false,
  isCanvasOpen = false,
  textareaRef: externalTextareaRef,
  leftExtra,
  rightExtra,
  topExtra,
}) => {
  return (
    <BaseComposer
      text={text}
      setText={setText}
      onSend={onSend}
      onStop={onStop}
      isLoading={isLoading}
      disabled={disabled}
      onPaste={onPaste}
      isFullscreen={isFullscreen}
      fillHeightWhenFullscreen
      hasAdditionalContent={pendingImages.length > 0}
      maxAutoHeight={300}
      textareaRef={externalTextareaRef}
      onEscape={() => onToolClick("fullscreen")}
      placeholder={
        isFullscreen
          ? "全屏编辑模式，按 ESC 退出，Enter 发送"
          : "在这里输入消息, 按 Enter 发送"
      }
    >
      {({ textareaProps, textareaRef, isPrimaryDisabled, onPrimaryAction }) => (
        <Container className={isFullscreen ? "flex-1 flex flex-col" : ""}>
          <InputBarContainer
            className={isFullscreen ? "flex-1 flex flex-col" : ""}
          >
            {!isFullscreen && <DragHandle />}

            {pendingImages.length > 0 && (
              <ImagePreviewContainer>
                {pendingImages.map((img, index) => (
                  <ImagePreviewItem key={index}>
                    <ImagePreviewImg
                      src={`data:${img.mediaType};base64,${img.data}`}
                      alt={`预览 ${index + 1}`}
                    />
                    <ImageRemoveButton onClick={() => onRemoveImage?.(index)}>
                      <X size={12} />
                    </ImageRemoveButton>
                  </ImagePreviewItem>
                ))}
              </ImagePreviewContainer>
            )}

            {topExtra}

            <StyledTextarea
              ref={textareaRef}
              {...textareaProps}
              className={isFullscreen ? "flex-1 resize-none" : ""}
            />

            <BottomBar>
              <LeftSection>
                {leftExtra && (
                  <div className="flex items-center gap-2 mr-2">{leftExtra}</div>
                )}
                <InputbarTools
                  onToolClick={onToolClick}
                  activeTools={activeTools}
                  executionStrategy={executionStrategy}
                  showExecutionStrategy={showExecutionStrategy}
                  isCanvasOpen={isCanvasOpen}
                />
              </LeftSection>

              <RightSection>
                {rightExtra}
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <ToolButton onClick={() => onToolClick("translate")}>
                        <Languages size={18} />
                      </ToolButton>
                    </TooltipTrigger>
                    <TooltipContent side="top">翻译</TooltipContent>
                  </Tooltip>
                </TooltipProvider>
                <SendButton
                  onClick={onPrimaryAction}
                  disabled={isPrimaryDisabled}
                  $isStop={isLoading}
                >
                  {isLoading ? (
                    <Square size={16} fill="currentColor" />
                  ) : (
                    <ArrowUp size={20} strokeWidth={3} />
                  )}
                </SendButton>
              </RightSection>
            </BottomBar>
          </InputBarContainer>
        </Container>
      )}
    </BaseComposer>
  );
};
