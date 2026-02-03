/**
 * @file PersonaDialog.tsx
 * @description 人设编辑对话框组件
 * @module components/projects/dialogs/PersonaDialog
 * @requirements 6.1, 6.2, 6.3, 6.4
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
import { SaveIcon, Loader2Icon } from "lucide-react";
import type { Persona, CreatePersonaRequest } from "@/types/persona";

export interface PersonaDialogProps {
  /** 是否打开 */
  open: boolean;
  /** 关闭回调 */
  onOpenChange: (open: boolean) => void;
  /** 项目 ID */
  projectId: string;
  /** 编辑的人设（新建时为 null） */
  persona: Persona | null;
  /** 保存回调 */
  onSave: (data: CreatePersonaRequest) => Promise<void>;
}

/**
 * 人设编辑对话框
 */
export function PersonaDialog({
  open,
  onOpenChange,
  projectId,
  persona,
  onSave,
}: PersonaDialogProps) {
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [style, setStyle] = useState("");
  const [tone, setTone] = useState("");
  const [targetAudience, setTargetAudience] = useState("");
  const [forbiddenWords, setForbiddenWords] = useState("");
  const [preferredWords, setPreferredWords] = useState("");

  const isEditing = !!persona?.id;

  // 同步编辑数据
  useEffect(() => {
    if (persona) {
      setName(persona.name || "");
      setDescription(persona.description || "");
      setStyle(persona.style || "");
      setTone(persona.tone || "");
      setTargetAudience(persona.targetAudience || "");
      setForbiddenWords(persona.forbiddenWords?.join("、") || "");
      setPreferredWords(persona.preferredWords?.join("、") || "");
    } else {
      // 重置表单
      setName("");
      setDescription("");
      setStyle("");
      setTone("");
      setTargetAudience("");
      setForbiddenWords("");
      setPreferredWords("");
    }
  }, [persona, open]);

  const handleSave = async () => {
    if (!name.trim()) return;
    setSaving(true);
    try {
      await onSave({
        projectId,
        name: name.trim(),
        description: description.trim() || undefined,
        style: style.trim() || "专业、清晰",
        tone: tone.trim() || undefined,
        targetAudience: targetAudience.trim() || undefined,
        forbiddenWords: forbiddenWords
          ? forbiddenWords
              .split(/[,，、]/)
              .map((w) => w.trim())
              .filter(Boolean)
          : [],
        preferredWords: preferredWords
          ? preferredWords
              .split(/[,，、]/)
              .map((w) => w.trim())
              .filter(Boolean)
          : [],
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
          <DialogTitle>{isEditing ? "编辑人设" : "创建人设"}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 名称 */}
          <div className="space-y-2">
            <Label htmlFor="persona-name">人设名称 *</Label>
            <Input
              id="persona-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：科技博主小王"
            />
          </div>

          {/* 描述 */}
          <div className="space-y-2">
            <Label htmlFor="persona-desc">描述</Label>
            <Textarea
              id="persona-desc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="简要描述这个人设的特点"
              rows={2}
            />
          </div>

          {/* 写作风格 */}
          <div className="space-y-2">
            <Label htmlFor="persona-style">写作风格 *</Label>
            <Input
              id="persona-style"
              value={style}
              onChange={(e) => setStyle(e.target.value)}
              placeholder="例如：轻松幽默、专业严谨、温暖亲切"
            />
          </div>

          {/* 语气 */}
          <div className="space-y-2">
            <Label htmlFor="persona-tone">语气</Label>
            <Input
              id="persona-tone"
              value={tone}
              onChange={(e) => setTone(e.target.value)}
              placeholder="例如：友好、正式、活泼"
            />
          </div>

          {/* 目标受众 */}
          <div className="space-y-2">
            <Label htmlFor="persona-audience">目标受众</Label>
            <Input
              id="persona-audience"
              value={targetAudience}
              onChange={(e) => setTargetAudience(e.target.value)}
              placeholder="例如：年轻女性、科技爱好者、职场新人"
            />
          </div>

          {/* 禁用词 */}
          <div className="space-y-2">
            <Label htmlFor="persona-forbidden">禁用词</Label>
            <Input
              id="persona-forbidden"
              value={forbiddenWords}
              onChange={(e) => setForbiddenWords(e.target.value)}
              placeholder="用逗号分隔，例如：绝对、一定、必须"
            />
            <p className="text-xs text-muted-foreground">
              AI 创作时会避免使用这些词
            </p>
          </div>

          {/* 偏好词 */}
          <div className="space-y-2">
            <Label htmlFor="persona-preferred">偏好词</Label>
            <Input
              id="persona-preferred"
              value={preferredWords}
              onChange={(e) => setPreferredWords(e.target.value)}
              placeholder="用逗号分隔，例如：宝子、姐妹、干货"
            />
            <p className="text-xs text-muted-foreground">
              AI 创作时会优先使用这些词
            </p>
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

export default PersonaDialog;
