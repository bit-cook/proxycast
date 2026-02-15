/**
 * @file MaterialPreviewDialog.tsx
 * @description 素材预览对话框组件
 * @module components/projects/dialogs/MaterialPreviewDialog
 * @requirements 7.5
 */

import { useState, useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  FileIcon,
  ImageIcon,
  FileTextIcon,
  DatabaseIcon,
  LinkIcon,
  Music2Icon,
  VideoIcon,
  ExternalLinkIcon,
  PaletteIcon,
  LayoutIcon,
} from "lucide-react";
import type { Material, MaterialType } from "@/types/material";
import { MaterialTypeLabels } from "@/types/material";

export interface MaterialPreviewDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  material: Material | null;
}

const MaterialTypeIcons: Record<MaterialType, typeof FileIcon> = {
  document: FileIcon,
  image: ImageIcon,
  audio: Music2Icon,
  video: VideoIcon,
  text: FileTextIcon,
  data: DatabaseIcon,
  link: LinkIcon,
  icon: FileIcon,
  color: PaletteIcon,
  layout: LayoutIcon,
};

/**
 * 素材预览对话框
 *
 * 支持预览图片、文本、链接等类型的素材。
 */
export function MaterialPreviewDialog({
  open,
  onOpenChange,
  material,
}: MaterialPreviewDialogProps) {
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [textContent, setTextContent] = useState<string | null>(null);

  useEffect(() => {
    if (!material || !open) {
      setImageSrc(null);
      setTextContent(null);
      return;
    }

    // 处理图片类型
    if (
      (material.type === "image" ||
        material.type === "audio" ||
        material.type === "video") &&
      material.filePath
    ) {
      const src = convertFileSrc(material.filePath);
      setImageSrc(src);
    }

    // 处理文本内容
    if (material.content) {
      setTextContent(material.content);
    }
  }, [material, open]);

  if (!material) return null;

  const Icon = MaterialTypeIcons[material.type as MaterialType] || FileIcon;

  const renderPreview = () => {
    switch (material.type as MaterialType) {
      case "image":
        return imageSrc ? (
          <div className="flex items-center justify-center p-4 bg-muted/30 rounded-lg">
            <img
              src={imageSrc}
              alt={material.name}
              className="max-w-full max-h-[400px] object-contain rounded"
            />
          </div>
        ) : (
          <div className="flex items-center justify-center h-48 bg-muted/30 rounded-lg">
            <p className="text-muted-foreground">无法加载图片</p>
          </div>
        );

      case "text":
        return textContent ? (
          <ScrollArea className="h-[300px] rounded-lg border p-4 bg-muted/30">
            <pre className="whitespace-pre-wrap text-sm">{textContent}</pre>
          </ScrollArea>
        ) : (
          <div className="flex items-center justify-center h-48 bg-muted/30 rounded-lg">
            <p className="text-muted-foreground">无文本内容</p>
          </div>
        );

      case "audio":
        return imageSrc ? (
          <div className="p-4 bg-muted/30 rounded-lg">
            <audio controls className="w-full" src={imageSrc}>
              当前浏览器不支持音频预览
            </audio>
          </div>
        ) : (
          <div className="flex items-center justify-center h-48 bg-muted/30 rounded-lg">
            <p className="text-muted-foreground">无法加载音频</p>
          </div>
        );

      case "video":
        return imageSrc ? (
          <div className="p-4 bg-muted/30 rounded-lg">
            <video controls className="max-h-[420px] w-full rounded" src={imageSrc}>
              当前浏览器不支持视频预览
            </video>
          </div>
        ) : (
          <div className="flex items-center justify-center h-48 bg-muted/30 rounded-lg">
            <p className="text-muted-foreground">无法加载视频</p>
          </div>
        );

      case "link":
        return (
          <div className="p-4 bg-muted/30 rounded-lg">
            <a
              href={material.content || "#"}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-primary hover:underline"
            >
              <ExternalLinkIcon className="h-4 w-4" />
              {material.content || "无链接"}
            </a>
          </div>
        );

      case "document":
      case "data":
      default:
        return (
          <div className="flex flex-col items-center justify-center h-48 bg-muted/30 rounded-lg">
            <Icon className="h-16 w-16 text-muted-foreground mb-4" />
            <p className="text-muted-foreground">
              {material.filePath ? "文件预览暂不支持" : "无文件"}
            </p>
            {material.filePath && (
              <p className="text-xs text-muted-foreground mt-2 max-w-full truncate px-4">
                {material.filePath}
              </p>
            )}
          </div>
        );
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Icon className="h-5 w-5" />
            {material.name}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* 元信息 */}
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="outline">
              {MaterialTypeLabels[material.type as MaterialType] ||
                material.type}
            </Badge>
            {material.fileSize && (
              <Badge variant="secondary">
                {(material.fileSize / 1024).toFixed(1)} KB
              </Badge>
            )}
            {material.mimeType && (
              <Badge variant="secondary">{material.mimeType}</Badge>
            )}
          </div>

          {/* 预览区域 */}
          {renderPreview()}

          {/* 描述 */}
          {material.description && (
            <div className="space-y-1">
              <p className="text-sm font-medium">描述</p>
              <p className="text-sm text-muted-foreground">
                {material.description}
              </p>
            </div>
          )}

          {/* 标签 */}
          {material.tags.length > 0 && (
            <div className="space-y-1">
              <p className="text-sm font-medium">标签</p>
              <div className="flex flex-wrap gap-1">
                {material.tags.map((tag) => (
                  <Badge key={tag} variant="secondary">
                    {tag}
                  </Badge>
                ))}
              </div>
            </div>
          )}

          {/* 创建时间 */}
          <div className="text-xs text-muted-foreground">
            创建于 {new Date(material.createdAt * 1000).toLocaleString()}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default MaterialPreviewDialog;
