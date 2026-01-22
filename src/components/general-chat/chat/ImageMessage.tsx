/**
 * @file ImageMessage.tsx
 * @description 图片消息渲染组件
 * @module components/general-chat/chat/ImageMessage
 *
 * 用于在消息中显示图片内容，支持点击放大预览
 */

import React, { useState, useCallback } from "react";
import { X, ZoomIn, Download } from "lucide-react";
import type { ContentBlock } from "../types";

interface ImageMessageProps {
  /** 图片内容块 */
  block: ContentBlock;
  /** 点击回调 */
  onClick?: () => void;
}

/**
 * 图片预览模态框
 */
interface ImagePreviewModalProps {
  /** 图片 URL */
  src: string;
  /** 图片文件名 */
  filename?: string;
  /** 是否显示 */
  isOpen: boolean;
  /** 关闭回调 */
  onClose: () => void;
}

const ImagePreviewModal: React.FC<ImagePreviewModalProps> = ({
  src,
  filename,
  isOpen,
  onClose,
}) => {
  const handleDownload = useCallback(() => {
    const link = document.createElement("a");
    link.href = src;
    link.download = filename || "image.png";
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  }, [src, filename]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
      {/* 背景遮罩 */}
      <div
        className="absolute inset-0"
        onClick={onClose}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
        }}
      />

      {/* 图片容器 */}
      <div className="relative max-w-[90vw] max-h-[90vh] bg-white rounded-lg shadow-2xl">
        {/* 工具栏 */}
        <div className="absolute top-0 left-0 right-0 z-10 flex items-center justify-between p-4 bg-gradient-to-b from-black/50 to-transparent">
          <div className="text-white text-sm font-medium">
            {filename || "图片预览"}
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleDownload}
              className="p-2 text-white hover:bg-white/20 rounded-lg transition-colors"
              title="下载图片"
            >
              <Download size={18} />
            </button>
            <button
              type="button"
              onClick={onClose}
              className="p-2 text-white hover:bg-white/20 rounded-lg transition-colors"
              title="关闭"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* 图片 */}
        <img
          src={src}
          alt={filename || "图片"}
          className="max-w-full max-h-[90vh] object-contain rounded-lg"
          style={{ minWidth: "300px", minHeight: "200px" }}
        />
      </div>
    </div>
  );
};

/**
 * 图片消息组件
 */
export const ImageMessage: React.FC<ImageMessageProps> = ({
  block,
  onClick,
}) => {
  const [isPreviewOpen, setIsPreviewOpen] = useState(false);

  const handleImageClick = useCallback(() => {
    setIsPreviewOpen(true);
    onClick?.();
  }, [onClick]);

  const handleClosePreview = useCallback(() => {
    setIsPreviewOpen(false);
  }, []);

  // 处理键盘事件
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        handleImageClick();
      }
    },
    [handleImageClick],
  );

  // 确保是图片类型
  if (block.type !== "image") {
    return null;
  }

  return (
    <>
      {/* 图片缩略图 */}
      <div className="relative group max-w-sm">
        <div
          className="relative overflow-hidden rounded-lg border border-ink-200 bg-surface cursor-pointer transition-all duration-200 hover:shadow-md hover:border-accent/30"
          onClick={handleImageClick}
          onKeyDown={handleKeyDown}
          role="button"
          tabIndex={0}
          aria-label="点击查看大图"
        >
          <img
            src={block.content}
            alt={block.filename || "图片"}
            className="w-full h-auto max-h-64 object-cover transition-transform duration-200 group-hover:scale-105"
            loading="lazy"
          />

          {/* 悬浮遮罩 */}
          <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors duration-200 flex items-center justify-center">
            <div className="opacity-0 group-hover:opacity-100 transition-opacity duration-200 bg-white/90 rounded-full p-2">
              <ZoomIn size={20} className="text-ink-700" />
            </div>
          </div>
        </div>

        {/* 文件名标签 */}
        {block.filename && (
          <div className="mt-2 text-xs text-ink-500 truncate">
            {block.filename}
          </div>
        )}
      </div>

      {/* 预览模态框 */}
      <ImagePreviewModal
        src={block.content}
        filename={block.filename}
        isOpen={isPreviewOpen}
        onClose={handleClosePreview}
      />
    </>
  );
};

export default ImageMessage;
