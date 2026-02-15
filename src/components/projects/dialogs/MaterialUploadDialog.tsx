/**
 * @file MaterialUploadDialog.tsx
 * @description 素材上传对话框组件
 * @module components/projects/dialogs/MaterialUploadDialog
 * @requirements 7.1, 7.2
 */

import React, { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  UploadIcon,
  FileIcon,
  Loader2Icon,
  XIcon,
  FolderOpenIcon,
} from "lucide-react";
import type { MaterialType, UploadMaterialRequest } from "@/types/material";

export interface MaterialUploadDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  onUpload: (data: UploadMaterialRequest) => Promise<void>;
}

const TYPE_OPTIONS: { value: MaterialType; label: string }[] = [
  { value: "document", label: "文档" },
  { value: "image", label: "图片" },
  { value: "audio", label: "语音" },
  { value: "video", label: "视频" },
  { value: "text", label: "文本" },
  { value: "data", label: "数据" },
  { value: "link", label: "链接" },
];

const IMAGE_EXTENSIONS = new Set([
  "jpg",
  "jpeg",
  "png",
  "gif",
  "webp",
  "svg",
  "bmp",
]);
const AUDIO_EXTENSIONS = new Set(["mp3", "wav", "aac", "m4a", "ogg", "flac"]);
const VIDEO_EXTENSIONS = new Set(["mp4", "mov", "avi", "mkv", "webm", "flv"]);
const DATA_EXTENSIONS = new Set(["csv", "json", "xml", "xlsx", "xls"]);
const TEXT_EXTENSIONS = new Set(["txt", "md"]);

const extractFileNameFromPath = (filePath: string): string => {
  const normalized = filePath.replace(/\\/g, "/");
  const name = normalized.split("/").pop();
  return name && name.trim() ? name : "未命名文件";
};

const inferMaterialTypeFromPath = (filePath: string): MaterialType => {
  const extension = filePath.split(".").pop()?.toLowerCase();
  if (!extension) {
    return "document";
  }
  if (IMAGE_EXTENSIONS.has(extension)) {
    return "image";
  }
  if (AUDIO_EXTENSIONS.has(extension)) {
    return "audio";
  }
  if (VIDEO_EXTENSIONS.has(extension)) {
    return "video";
  }
  if (DATA_EXTENSIONS.has(extension)) {
    return "data";
  }
  if (TEXT_EXTENSIONS.has(extension)) {
    return "text";
  }
  return "document";
};

export function MaterialUploadDialog({
  open,
  onOpenChange,
  projectId,
  onUpload,
}: MaterialUploadDialogProps) {
  const [uploading, setUploading] = useState(false);
  const [name, setName] = useState("");
  const [type, setType] = useState<MaterialType>("document");
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState("");
  const [content, setContent] = useState("");
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);

  const resetForm = () => {
    setName("");
    setType("document");
    setDescription("");
    setTags("");
    setContent("");
    setSelectedFilePath(null);
  };

  const handleFileSelect = async () => {
    const selected = await openDialog({
      title: "选择素材文件",
      directory: false,
      multiple: false,
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }

    setSelectedFilePath(selected);
    if (!name.trim()) {
      setName(extractFileNameFromPath(selected));
    }
    setType(inferMaterialTypeFromPath(selected));
  };

  const handleUpload = async () => {
    if (!name.trim()) return;
    const requiresFile = !["text", "link"].includes(type);
    if (requiresFile && !selectedFilePath) {
      return;
    }

    setUploading(true);
    try {
      await onUpload(
        {
          projectId,
          name: name.trim(),
          type,
          description: description.trim() || undefined,
          tags: tags
            ? tags
                .split(/[,，、]/)
                .map((t) => t.trim())
                .filter(Boolean)
            : [],
          content:
            type === "text" || type === "link" ? content.trim() : undefined,
          filePath: selectedFilePath ?? undefined,
        },
      );
      resetForm();
      onOpenChange(false);
    } finally {
      setUploading(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        if (!v) resetForm();
        onOpenChange(v);
      }}
    >
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>上传素材</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 文件选择区域 */}
          <div
            className="border-2 border-dashed rounded-lg p-6 text-center cursor-pointer hover:border-primary/50 transition-colors"
            onClick={() => {
              void handleFileSelect();
            }}
          >
            {selectedFilePath ? (
              <div className="flex items-center justify-center gap-2">
                <FileIcon className="h-8 w-8 text-primary" />
                <div className="text-left">
                  <p className="font-medium">
                    {extractFileNameFromPath(selectedFilePath)}
                  </p>
                  <p className="text-xs text-muted-foreground truncate max-w-[280px]">
                    {selectedFilePath}
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={(e) => {
                    e.stopPropagation();
                    setSelectedFilePath(null);
                  }}
                >
                  <XIcon className="h-4 w-4" />
                </Button>
              </div>
            ) : (
              <>
                <UploadIcon className="h-10 w-10 mx-auto mb-2 text-muted-foreground" />
                <p className="text-sm text-muted-foreground">
                  点击选择本地文件
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  支持图片、文档、音视频、数据文件
                </p>
              </>
            )}
          </div>

          <div className="flex justify-end">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                void handleFileSelect();
              }}
            >
              <FolderOpenIcon className="h-4 w-4 mr-1" />
              重新选择文件
            </Button>
          </div>

          {/* 素材名称 */}
          <div className="space-y-2">
            <Label htmlFor="material-name">素材名称 *</Label>
            <Input
              id="material-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="输入素材名称"
            />
          </div>

          {/* 素材类型 */}
          <div className="space-y-2">
            <Label>素材类型</Label>
            <Select
              value={type}
              onValueChange={(v) => setType(v as MaterialType)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TYPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* 文本/链接内容 */}
          {(type === "text" || type === "link") && (
            <div className="space-y-2">
              <Label htmlFor="material-content">
                {type === "link" ? "链接地址" : "文本内容"}
              </Label>
              <Textarea
                id="material-content"
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder={type === "link" ? "https://..." : "输入文本内容"}
                rows={3}
              />
            </div>
          )}

          {/* 描述 */}
          <div className="space-y-2">
            <Label htmlFor="material-desc">描述</Label>
            <Input
              id="material-desc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="简要描述素材内容"
            />
          </div>

          {/* 标签 */}
          <div className="space-y-2">
            <Label htmlFor="material-tags">标签</Label>
            <Input
              id="material-tags"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="用逗号分隔，例如：参考、数据、图片"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button
            onClick={handleUpload}
            disabled={
              uploading ||
              !name.trim() ||
              (!selectedFilePath && !["text", "link"].includes(type))
            }
          >
            {uploading ? (
              <Loader2Icon className="h-4 w-4 mr-1 animate-spin" />
            ) : (
              <UploadIcon className="h-4 w-4 mr-1" />
            )}
            {uploading ? "上传中..." : "上传"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default MaterialUploadDialog;
