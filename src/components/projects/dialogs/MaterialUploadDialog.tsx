/**
 * @file MaterialUploadDialog.tsx
 * @description 素材上传对话框组件
 * @module components/projects/dialogs/MaterialUploadDialog
 * @requirements 7.1, 7.2
 */

import React, { useState, useRef } from "react";
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
import { UploadIcon, FileIcon, Loader2Icon, XIcon } from "lucide-react";
import type { MaterialType, UploadMaterialRequest } from "@/types/material";

export interface MaterialUploadDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  onUpload: (data: UploadMaterialRequest, file?: File) => Promise<void>;
}

const TYPE_OPTIONS: { value: MaterialType; label: string }[] = [
  { value: "document", label: "文档" },
  { value: "image", label: "图片" },
  { value: "text", label: "文本" },
  { value: "data", label: "数据" },
  { value: "link", label: "链接" },
];

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
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const resetForm = () => {
    setName("");
    setType("document");
    setDescription("");
    setTags("");
    setContent("");
    setSelectedFile(null);
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setSelectedFile(file);
      if (!name) setName(file.name);
      // 根据文件类型自动设置素材类型
      if (file.type.startsWith("image/")) setType("image");
      else if (file.type.includes("json") || file.type.includes("csv"))
        setType("data");
      else setType("document");
    }
  };

  const handleUpload = async () => {
    if (!name.trim()) return;
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
        },
        selectedFile || undefined,
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
            onClick={() => fileInputRef.current?.click()}
          >
            <input
              ref={fileInputRef}
              type="file"
              className="hidden"
              onChange={handleFileSelect}
              accept="image/*,.pdf,.doc,.docx,.txt,.md,.json,.csv"
            />
            {selectedFile ? (
              <div className="flex items-center justify-center gap-2">
                <FileIcon className="h-8 w-8 text-primary" />
                <div className="text-left">
                  <p className="font-medium">{selectedFile.name}</p>
                  <p className="text-xs text-muted-foreground">
                    {(selectedFile.size / 1024).toFixed(1)} KB
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={(e) => {
                    e.stopPropagation();
                    setSelectedFile(null);
                  }}
                >
                  <XIcon className="h-4 w-4" />
                </Button>
              </div>
            ) : (
              <>
                <UploadIcon className="h-10 w-10 mx-auto mb-2 text-muted-foreground" />
                <p className="text-sm text-muted-foreground">
                  点击或拖拽文件到此处
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  支持图片、文档、数据文件
                </p>
              </>
            )}
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
          <Button onClick={handleUpload} disabled={uploading || !name.trim()}>
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
