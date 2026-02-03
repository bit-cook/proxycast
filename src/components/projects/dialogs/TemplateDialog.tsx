/**
 * @file TemplateDialog.tsx
 * @description 排版模板编辑对话框组件
 * @module components/projects/dialogs/TemplateDialog
 * @requirements 8.1, 8.2, 8.3, 8.4
 */

import { useState, useEffect } from "react";
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
import { SaveIcon, Loader2Icon } from "lucide-react";
import type {
  Template,
  Platform,
  EmojiUsage,
  CreateTemplateRequest,
} from "@/types/template";

export interface TemplateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  template: Template | null;
  onSave: (data: CreateTemplateRequest) => Promise<void>;
}

const PLATFORM_OPTIONS: { value: Platform; label: string }[] = [
  { value: "xiaohongshu", label: "小红书" },
  { value: "wechat", label: "微信公众号" },
  { value: "zhihu", label: "知乎" },
  { value: "weibo", label: "微博" },
  { value: "douyin", label: "抖音" },
  { value: "markdown", label: "Markdown" },
];

const EMOJI_OPTIONS: { value: EmojiUsage; label: string }[] = [
  { value: "heavy", label: "大量使用" },
  { value: "moderate", label: "适度使用" },
  { value: "minimal", label: "少量使用" },
];

export function TemplateDialog({
  open,
  onOpenChange,
  projectId,
  template,
  onSave,
}: TemplateDialogProps) {
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("");
  const [platform, setPlatform] = useState<Platform>("xiaohongshu");
  const [emojiUsage, setEmojiUsage] = useState<EmojiUsage>("moderate");
  const [titleStyle, setTitleStyle] = useState("");
  const [paragraphStyle, setParagraphStyle] = useState("");
  const [endingStyle, setEndingStyle] = useState("");
  const [hashtagRules, setHashtagRules] = useState("");

  const isEditing = !!template?.id;

  useEffect(() => {
    if (template) {
      setName(template.name || "");
      setPlatform(template.platform || "xiaohongshu");
      setEmojiUsage(template.emojiUsage || "moderate");
      setTitleStyle(template.titleStyle || "");
      setParagraphStyle(template.paragraphStyle || "");
      setEndingStyle(template.endingStyle || "");
      setHashtagRules(template.hashtagRules || "");
    } else {
      setName("");
      setPlatform("xiaohongshu");
      setEmojiUsage("moderate");
      setTitleStyle("");
      setParagraphStyle("");
      setEndingStyle("");
      setHashtagRules("");
    }
  }, [template, open]);

  const handleSave = async () => {
    if (!name.trim()) return;
    setSaving(true);
    try {
      await onSave({
        projectId,
        name: name.trim(),
        platform,
        emojiUsage,
        titleStyle: titleStyle.trim() || undefined,
        paragraphStyle: paragraphStyle.trim() || undefined,
        endingStyle: endingStyle.trim() || undefined,
        hashtagRules: hashtagRules.trim() || undefined,
      });
      onOpenChange(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{isEditing ? "编辑模板" : "创建模板"}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="template-name">模板名称 *</Label>
            <Input
              id="template-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：小红书清新风"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>目标平台</Label>
              <Select
                value={platform}
                onValueChange={(v) => setPlatform(v as Platform)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {PLATFORM_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>Emoji 使用</Label>
              <Select
                value={emojiUsage}
                onValueChange={(v) => setEmojiUsage(v as EmojiUsage)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {EMOJI_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="template-title">标题风格</Label>
            <Textarea
              id="template-title"
              value={titleStyle}
              onChange={(e) => setTitleStyle(e.target.value)}
              placeholder="例如：使用数字开头，加入 emoji，控制在 20 字以内"
              rows={2}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="template-paragraph">段落风格</Label>
            <Textarea
              id="template-paragraph"
              value={paragraphStyle}
              onChange={(e) => setParagraphStyle(e.target.value)}
              placeholder="例如：每段 2-3 句话，使用短句，多用分点"
              rows={2}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="template-ending">结尾风格</Label>
            <Input
              id="template-ending"
              value={endingStyle}
              onChange={(e) => setEndingStyle(e.target.value)}
              placeholder="例如：引导互动，提问式结尾"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="template-hashtag">话题标签规则</Label>
            <Input
              id="template-hashtag"
              value={hashtagRules}
              onChange={(e) => setHashtagRules(e.target.value)}
              placeholder="例如：5-10 个标签，包含热门话题"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={saving || !name.trim()}>
            {saving ? (
              <Loader2Icon className="h-4 w-4 mr-1 animate-spin" />
            ) : (
              <SaveIcon className="h-4 w-4 mr-1" />
            )}
            {saving ? "保存中..." : "保存"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default TemplateDialog;
